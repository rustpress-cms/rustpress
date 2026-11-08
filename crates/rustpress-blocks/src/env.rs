//! Shared state for rendering one request: the site, the main query, the
//! theme, and what blocks collect while rendering (CSS rules, script modules,
//! ID counters).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use rustpress_core::{QueryResult, Site};
use rustpress_db::Post;
use serde_json::{Map, Value};

use crate::registry::Registry;

/// What the renderer needs from the active theme.
pub trait ThemeAccess: Send + Sync {
    /// Merged `settings` of core, theme and user (`wp_get_global_settings()`).
    fn global_settings(&self) -> &Value;
    /// `wp_is_block_theme()`.
    fn is_block_theme(&self) -> bool {
        true
    }
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
    /// `$content_width` as a classic theme sets it on `after_setup_theme`; 0 when it does not.
    fn content_width(&self) -> u64 {
        0
    }
    /// The `render_block_{$name}` filters of the theme's `functions.php`, after core's `render_block` filters.
    fn render_block_filter(&self, _name: &str, content: String) -> String {
        content
    }
    /// The `area_tag` of a template part area the theme adds (`default_wp_template_part_areas`).
    fn template_part_area_tag(&self, _area: &str) -> Option<&'static str> {
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

    /// `.a{x:y;}.b{...}` in the order the rules were added; a rule of a group (`@container ...`) is wrapped on its own.
    pub fn compile(&self) -> String {
        let mut out = String::new();
        for rule in &self.rules {
            let css = rule_css(rule);
            match &rule.rules_group {
                Some(group) if !css.is_empty() => out.push_str(&format!("{group}{{{css}}}")),
                Some(_) => {}
                None => out.push_str(&css),
            }
        }
        out
    }
}

fn rule_css(rule: &CssRule) -> String {
    let body = crate::style_engine::compile_declarations(&rule.declarations);
    if body.is_empty() { String::new() } else { format!("{}{{{}}}", rule.selector, body) }
}

/// Script modules in the order they were enqueued.
#[derive(Debug, Clone, Default)]
pub struct ModuleSet(Vec<String>);

impl ModuleSet {
    /// `wp_enqueue_script_module()`: a module is queued once.
    pub fn insert(&mut self, id: String) -> bool {
        if self.0.contains(&id) {
            return false;
        }
        self.0.push(id);
        true
    }

    pub fn contains(&self, id: &str) -> bool {
        self.0.iter().any(|m| m == id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }
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
    /// How many blocks are being rendered inside each other right now.
    pub render_depth: u32,
    /// Comments read during this render, by ID, and the listed comments of each post (every comment block asked
    /// the database again before).
    pub comments: HashMap<u64, rustpress_db::Comment>,
    pub listed_comments: HashMap<u64, Vec<rustpress_db::Comment>>,
    /// The loops of custom query blocks, by query, page and current post.
    pub(crate) query_loops: HashMap<String, crate::dynamic::query::LoopData>,
    /// Counter behind wp_unique_id().
    pub unique_counter: u64,
    /// `do_blocks()` took `wpautop` off the `the_content` filter and `_restore_wpautop_hook()` put it back
    /// behind `shortcode_unautop`: from then on the paragraphs are made after the shortcodes were unwrapped.
    pub wpautop_late: bool,
    /// `set_query_var( 'cpage' )` of `comments_template()`: the page of comments it showed.
    pub cpage_override: u64,
    /// Instances of `[audio]` and `[video]`, and the assets they pull in.
    pub audio_instances: u64,
    pub video_instances: u64,
    pub mediaelement: bool,
    /// `$content_width` as the page of a classic theme changed it on `template_redirect`; 0 when it did not run.
    pub content_width: u64,
    /// `wp_playlist_scripts()` ran.
    pub playlist: bool,
    /// `static $instance` of `wp_playlist_shortcode()`.
    pub playlist_instances: u64,
    pub vimeo: bool,
    /// `the_custom_header_markup()` enqueued `wp-custom-header` (the header video).
    pub custom_header_script: bool,
    /// Duotone filters used on the page.
    pub duotone: crate::duotone::DuotoneState,
    /// `static $instance` of `gallery_shortcode()`.
    pub gallery_instances: u64,
    /// `WP_Widget_Calendar::$instance`.
    pub calendars: u32,
    /// `static $first_dropdown` of `WP_Widget_Categories::widget()` is spent.
    pub categories_dropdown: bool,
    /// `static $_used_ids` of `_nav_menu_item_id_use_once()`.
    pub nav_menu_item_ids: HashSet<u64>,
    /// Patterns being rendered right now (recursion guard).
    pub active_patterns: HashSet<String>,
    /// Script modules that must be loaded on the page.
    pub script_modules: ModuleSet,
    /// `render_block_core_navigation_submenu()` turned `color` on for the block type.
    pub submenu_colors: bool,
    /// `block_core_image_render_lightbox()` is on `render_block_core/image`.
    pub image_lightbox: bool,
    /// `block_core_image_print_lightbox_overlay()` is on `wp_footer`.
    pub lightbox_overlay: bool,
    /// The last value of [`State::uniqid`].
    pub last_uniqid: u128,
    /// State and derived state closures of the Interactivity API.
    pub interactivity: crate::interactivity::Interactivity,
    /// A root interactive block is rendering: its directives are processed once it is done.
    pub interactive_root: bool,
    /// `static $tab_counters` of the tab panel block: panels rendered so far per tabs ID.
    pub tab_counters: HashMap<String, u64>,
    /// The posts `the_post` ran for, in order (the main loop of a singular page and every loop item).
    pub looped_posts: Vec<u64>,
    /// Classic scripts to load (`comment-reply`).
    pub scripts: Vec<String>,
    /// Extra inline styles keyed by handle (e.g. per-block custom CSS).
    pub inline_styles: Vec<(String, String)>,
    /// static $has_submenus: once any navigation has submenus, later ones count as interactive.
    pub nav_has_submenus: bool,
    /// Navigation names seen so far, to number duplicates (Menu 2).
    pub seen_menu_names: HashMap<String, u64>,
    /// The published pages of the page list, read once per render (navigation and page list both ask).
    pub(crate) published_pages: Option<Vec<rustpress_db::Post>>,
    /// Names of the blocks that went through `render_block`, in first-render order (unlike `used_blocks`,
    /// this leaves out blocks whose markup other code produces, like the button of the password form).
    pub rendered_blocks: Vec<String>,
    /// Template-part areas rendered (for `<header>` / `<footer>` tag choice).
    pub rendered_parts: Vec<String>,
    /// What the image loading logic remembers between images.
    pub media: crate::images::MediaState,
    /// What the native plugins remember during the request.
    pub plugins: rustpress_core::native::PluginState,
    /// Content is rendered outside the loop (`in_the_loop()` is false, as in the REST API).
    pub outside_loop: bool,
    /// A `core/query` block with `inherit` off was rendered.
    pub custom_query_loop: bool,
    /// The last rendered `core/query-pagination-next` and `core/query-pagination-previous`.
    pub pagination_next: Option<String>,
    pub pagination_previous: Option<String>,
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

    /// `uniqid()`: seconds and microseconds in hexadecimal, different on every call.
    pub fn uniqid(&mut self) -> String {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_micros());
        self.last_uniqid = now.max(self.last_uniqid + 1);
        format!("{:08x}{:05x}", self.last_uniqid / 1_000_000, self.last_uniqid % 1_000_000)
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

/// What a plugin may ask of the rendering environment.
impl rustpress_core::native::PageServices for Env<'_> {
    fn excerpt<'a>(&'a self, post: &'a rustpress_db::Post) -> futures::future::BoxFuture<'a, rustpress_core::Result<String>> {
        Box::pin(async move {
            let post = std::sync::Arc::new(post.clone());
            crate::excerpt::get_the_excerpt(self, &post).await.map_err(|e| match e {
                crate::Error::Core(e) => e,
                crate::Error::Db(e) => rustpress_core::Error::Db(e),
            })
        })
    }

    fn wp_root(&self) -> &std::path::Path {
        // The blocks live in `wp-includes/blocks`.
        let blocks = self.registry.blocks_dir();
        blocks.parent().and_then(std::path::Path::parent).unwrap_or(blocks)
    }

    fn content_filters<'a>(&'a self, post: Option<&'a rustpress_db::Post>, text: &'a str, context: &'a str) -> futures::future::BoxFuture<'a, rustpress_core::Result<String>> {
        Box::pin(async move {
            let post = post.map(|p| std::sync::Arc::new(p.clone()));
            crate::content::content_filters(self, post.as_ref(), text, context).await.map_err(|e| match e {
                crate::Error::Core(e) => e,
                crate::Error::Db(e) => rustpress_core::Error::Db(e),
            })
        })
    }

    fn avatar(&self, email: &str, size: u64) -> String {
        crate::content::get_avatar(self, email, size, "", "", "")
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

