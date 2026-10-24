//! Shared state for rendering one request: the site, the main query, the
//! theme, and what blocks collect while rendering (CSS rules, script modules,
//! ID counters).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use rustpress_core::{QueryResult, Site};
use rustpress_db::Post;
use serde_json::{Map, Value};

use crate::registry::Registry;

/// What the renderer needs from the active theme.
pub trait ThemeAccess: Send + Sync {
    /// Merged `settings` of core, theme and user (`wp_get_global_settings()`).
    fn global_settings(&self) -> &Value;
    /// Merged `styles` (`wp_get_global_styles()`).
    fn global_styles(&self) -> &Value;
    /// Block markup of a template part (`header`, `footer`, ...), if it exists.
    fn template_part(&self, slug: &str) -> Option<TemplatePart>;
    /// Block markup of a registered pattern.
    fn pattern(&self, slug: &str) -> Option<String>;
    /// `get_template_directory_uri()`.
    fn template_directory_uri(&self) -> String;
    /// `get_stylesheet_directory_uri()`.
    fn stylesheet_directory_uri(&self) -> String;
    /// A block style variation defined in theme.json (`styles.blocks.<name>.variations.<slug>`).
    fn style_variation(&self, _block_name: &str, _variation: &str) -> Option<Value> {
        None
    }
    /// CSS for a style variation instance, scoped to `scope` (e.g. `.is-style-x--2`).
    fn variation_css(&self, _block_name: &str, _variation: &Value, _scope: &str) -> String {
        String::new()
    }
    /// The `wp_calculate_image_sizes` filter of the theme.
    fn filter_calculated_sizes(&self, _site: &Site, _query: &QueryResult, sizes: Option<String>, _width: u64) -> Option<String> {
        sizes
    }
    /// The `wp_get_attachment_image_attributes` filter of the theme, for the `sizes` attribute:
    /// the replacement, if the theme sets one.
    fn filter_attachment_sizes(&self, _site: &Site, _query: &QueryResult, _current: Option<&str>) -> Option<String> {
        None
    }
    /// The `excerpt_more` filter of the theme as a template: `%1$s` is the permalink, `%2$s` the title.
    fn excerpt_more(&self) -> Option<String> {
        None
    }
    /// A size registered with `add_image_size()`: width and height.
    fn image_size(&self, _name: &str) -> Option<(u64, u64)> {
        None
    }
    /// Translation lookup for the theme's text domain; returns the text unchanged by default.
    fn translate(&self, text: &str, _context: Option<&str>) -> String {
        text.to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct TemplatePart {
    pub slug: String,
    pub content: String,
    /// `header`, `footer` or `uncategorized`.
    pub area: String,
}

/// One CSS rule collected from block supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssRule {
    pub selector: String,
    pub rules_group: Option<String>,
    pub declarations: Vec<(String, String)>,
}

/// `WP_Style_Engine_CSS_Rules_Store`: rules in insertion order, merged by selector.
#[derive(Debug, Clone, Default)]
pub struct CssStore {
    rules: Vec<CssRule>,
}

impl CssStore {
    pub fn add(&mut self, selector: &str, declarations: &[(String, String)], rules_group: Option<&str>) {
        // The style engine drops properties `safecss_filter_attr()` does not know (e.g. `box-sizing`).
        let declarations: Vec<(String, String)> = declarations.iter().filter(|(property, _)| crate::safe_css::is_safe_property(property)).cloned().collect();
        let declarations = declarations.as_slice();
        if selector.is_empty() || declarations.is_empty() {
            return;
        }
        let existing = self.rules.iter_mut().find(|r| r.selector == selector && r.rules_group.as_deref() == rules_group);
        let rule = match existing {
            Some(rule) => rule,
            None => {
                self.rules.push(CssRule { selector: selector.to_owned(), rules_group: rules_group.map(str::to_owned), declarations: Vec::new() });
                self.rules.last_mut().expect("just pushed")
            }
        };
        for (property, value) in declarations {
            match rule.declarations.iter_mut().find(|(p, _)| p == property) {
                Some(slot) => slot.1 = value.clone(),
                None => rule.declarations.push((property.clone(), value.clone())),
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn rules(&self) -> &[CssRule] {
        &self.rules
    }

    /// `.a{x:y;}.b{...}` with rules inside the same `@media` group wrapped together.
    pub fn compile(&self) -> String {
        let mut out = String::new();
        let mut groups: Vec<&str> = Vec::new();
        for rule in self.rules.iter().filter(|r| r.rules_group.is_none()) {
            out.push_str(&rule_css(rule));
        }
        for rule in &self.rules {
            if let Some(group) = &rule.rules_group
                && !groups.contains(&group.as_str()) {
                    groups.push(group);
                }
        }
        for group in groups {
            out.push_str(group);
            out.push('{');
            for rule in self.rules.iter().filter(|r| r.rules_group.as_deref() == Some(group)) {
                out.push_str(&rule_css(rule));
            }
            out.push('}');
        }
        out
    }
}

fn rule_css(rule: &CssRule) -> String {
    let body = crate::style_engine::compile_declarations(&rule.declarations);
    if body.is_empty() { String::new() } else { format!("{}{{{}}}", rule.selector, body) }
}

/// Things collected while rendering.
#[derive(Debug, Default)]
pub struct State {
    /// Layout and element rules (`core-block-supports-inline-css`).
    pub block_supports_css: CssStore,
    /// Names of the block types that were rendered, in first-use order.
    pub used_blocks: Vec<String>,
    /// `wp_unique_prefixed_id` counters.
    pub id_counters: HashMap<String, u64>,
    /// Counter behind wp_unique_id().
    pub unique_counter: u64,
    /// Patterns being rendered right now (recursion guard).
    pub active_patterns: HashSet<String>,
    /// Script modules that must be loaded on the page.
    pub script_modules: BTreeSet<String>,
    /// Classic scripts to load (`comment-reply`).
    pub scripts: Vec<String>,
    /// Extra inline styles keyed by handle (e.g. per-block custom CSS).
    pub inline_styles: Vec<(String, String)>,
    /// static $has_submenus: once any navigation has submenus, later ones count as interactive.
    pub nav_has_submenus: bool,
    /// Navigation names seen so far, to number duplicates (Menu 2).
    pub seen_menu_names: HashMap<String, u64>,
    /// Names of the blocks that went through `render_block`, in first-render order (unlike `used_blocks`,
    /// this leaves out blocks whose markup other code produces, like the button of the password form).
    pub rendered_blocks: Vec<String>,
    /// Template-part areas rendered (for `<header>` / `<footer>` tag choice).
    pub rendered_parts: Vec<String>,
    /// What the image loading logic remembers between images.
    pub media: crate::images::MediaState,
}

impl State {
    pub fn note_block(&mut self, name: &str) {
        if !self.used_blocks.iter().any(|b| b == name) {
            self.used_blocks.push(name.to_owned());
        }
    }

    /// `wp_unique_id( $prefix )`: one counter shared by all prefixes.
    pub fn unique_id(&mut self, prefix: &str) -> String {
        self.unique_counter += 1;
        format!("{prefix}{}", self.unique_counter)
    }

    /// wp_unique_prefixed_id(  ): a counter per prefix.
    pub fn unique_prefixed_id(&mut self, prefix: &str) -> String {
        let counter = self.id_counters.entry(prefix.to_owned()).or_insert(0);
        *counter += 1;
        format!("{prefix}{counter}")
    }
}

/// `WP_Block::$context` plus the current post (`global $post`).
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub values: Map<String, Value>,
    pub post: Option<Arc<Post>>,
}

impl Context {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn with(&self, key: &str, value: Value) -> Self {
        let mut next = self.clone();
        next.values.insert(key.to_owned(), value);
        next
    }

    pub fn post_id(&self) -> Option<u64> {
        self.values.get("postId").and_then(Value::as_u64)
    }
}

/// Everything a block render needs. One per request.
pub struct Env<'a> {
    pub site: &'a Site,
    pub query: &'a QueryResult,
    pub theme: &'a dyn ThemeAccess,
    pub registry: &'a Registry,
    pub state: Mutex<State>,
}

impl<'a> Env<'a> {
    pub fn new(site: &'a Site, query: &'a QueryResult, theme: &'a dyn ThemeAccess, registry: &'a Registry) -> Self {
        Self { site, query, theme, registry, state: Mutex::new(State::default()) }
    }

    /// Runs `f` with the shared state. The lock is never held across `.await`.
    pub fn with_state<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.state.lock().expect("render state lock"))
    }

    pub fn take_state(&self) -> State {
        std::mem::take(&mut *self.state.lock().expect("render state lock"))
    }

    /// `home_url( $path )`.
    pub fn home_url(&self, path: &str) -> String {
        let home = self.site.settings.home.trim_end_matches('/');
        if path.is_empty() { home.to_owned() } else { format!("{home}/{}", path.trim_start_matches('/')) }
    }

    /// `global $post` for the root context, derived from the main query.
    pub fn root_context(&self) -> Context {
        let mut ctx = Context::default();
        if self.query.flags.is_singular()
            && let Some(post) = self.query.queried_post() {
                ctx.values.insert("postId".into(), Value::from(post.id));
                ctx.values.insert("postType".into(), Value::from(post.post_type.clone()));
                ctx.post = Some(Arc::new(post.clone()));
            }
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_rules_merge_by_selector_and_group() {
        let mut store = CssStore::default();
        store.add(".a", &[("color".into(), "1".into())], None);
        store.add(".b", &[("margin".into(), "2".into())], None);
        store.add(".a", &[("color".into(), "3".into()), ("gap".into(), "4".into())], None);
        store.add(".a", &[("padding".into(), "5".into())], Some("@media (min-width:600px)"));
        assert_eq!(store.compile(), ".a{color:3;gap:4;}.b{margin:2;}@media (min-width:600px){.a{padding:5;}}");
    }

    #[test]
    fn empty_rules_are_ignored() {
        let mut store = CssStore::default();
        store.add("", &[("x".into(), "1".into())], None);
        store.add(".a", &[], None);
        assert!(store.is_empty());
        assert_eq!(store.compile(), "");
    }

    #[test]
    fn unique_ids_count_per_prefix() {
        let mut state = State::default();
        assert_eq!(state.unique_prefixed_id("wp-elements-"), "wp-elements-1");
        assert_eq!(state.unique_prefixed_id("wp-elements-"), "wp-elements-2");
        assert_eq!(state.unique_prefixed_id("modal-"), "modal-1");
    }
}
