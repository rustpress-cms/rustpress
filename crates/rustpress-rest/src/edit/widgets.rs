//! `wp/v2/sidebars`, `wp/v2/widgets` and `wp/v2/widget-types` (`WP_REST_Sidebars_Controller`,
//! `WP_REST_Widgets_Controller`, `WP_REST_Widget_Types_Controller`): reading. Like WordPress, every read first runs
//! `retrieve_widgets()`, which maps the widgets of sidebars the theme does not have and stores the result.

use base64::Engine;
use rustpress_phpser::{Key, Value as Php};
use rustpress_theme::classic::widgets::{self as theme_widgets, RegisteredSidebar};
use serde_json::{Map, Value, json};

use super::Args;
use crate::error::RestError;
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

/// The core widgets in the order `wp_widgets_init()` registers them: id base, name, description, class name.
pub(crate) const WIDGET_TYPES: [(&str, &str, &str, &str); 18] = [
    ("pages", "Pages", "A list of your site&#8217;s Pages.", "widget_pages"),
    ("calendar", "Calendar", "A calendar of your site’s posts.", "widget_calendar"),
    ("archives", "Archives", "A monthly archive of your site&#8217;s Posts.", "widget_archive"),
    ("media_audio", "Audio", "Displays an audio player.", "widget_media_audio"),
    ("media_image", "Image", "Displays an image.", "widget_media_image"),
    ("media_gallery", "Gallery", "Displays an image gallery.", "widget_media_gallery"),
    ("media_video", "Video", "Displays a video from the media library or from YouTube, Vimeo, or another provider.", "widget_media_video"),
    ("meta", "Meta", "Login, RSS, &amp; WordPress.org links.", "widget_meta"),
    ("search", "Search", "A search form for your site.", "widget_search"),
    ("text", "Text", "Arbitrary text.", "widget_text"),
    ("categories", "Categories", "A list or dropdown of categories.", "widget_categories"),
    ("recent-posts", "Recent Posts", "Your site&#8217;s most recent Posts.", "widget_recent_entries"),
    ("recent-comments", "Recent Comments", "Your site&#8217;s most recent comments.", "widget_recent_comments"),
    ("rss", "RSS", "Entries from any RSS or Atom feed.", "widget_rss"),
    ("tag_cloud", "Tag Cloud", "A cloud of your most used tags.", "widget_tag_cloud"),
    ("nav_menu", "Navigation Menu", "Add a navigation menu to your sidebar.", "widget_nav_menu"),
    ("custom_html", "Custom HTML", "Arbitrary HTML code.", "widget_custom_html"),
    ("block", "Block", "A widget containing a block.", "widget_block"),
];

/// A PHP list of widget ids with the keys it keeps after `unset()`.
type Ids = Vec<(i64, String)>;

/// Sidebar id -> widget ids, in the order of the PHP array; `None` stands for a value that is not an array.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Sidebars(Vec<(String, Option<Ids>)>);

impl Sidebars {
    fn from_php(value: Option<&Php>) -> Self {
        let Some(entries) = value.and_then(Php::as_array) else { return Self::default() };
        Self(
            entries
                .iter()
                .map(|(key, list)| {
                    let name = match key {
                        Key::Int(n) => n.to_string(),
                        Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
                    };
                    let ids = list.as_array().map(|items| {
                        items
                            .iter()
                            .filter_map(|(k, id)| {
                                let Key::Int(k) = k else { return None };
                                Some((*k, id.as_str()?.to_owned()))
                            })
                            .collect()
                    });
                    (name, ids)
                })
                .collect(),
        )
    }

    fn to_php(&self) -> Php {
        Php::Array(
            self.0
                .iter()
                .map(|(name, ids)| {
                    let list = ids.as_ref().map_or_else(Vec::new, |ids| ids.iter().map(|(k, id)| (Key::Int(*k), Php::Str(id.as_bytes().to_vec()))).collect());
                    (Key::Str(name.as_bytes().to_vec()), Php::Array(list))
                })
                .collect(),
        )
    }

    fn get(&self, name: &str) -> Option<&Option<Ids>> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, ids)| ids)
    }

    fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// `$array[ $name ] = $ids`: replaces in place or appends.
    fn set(&mut self, name: &str, ids: Option<Ids>) {
        match self.0.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = ids,
            None => self.0.push((name.to_owned(), ids)),
        }
    }

    fn remove(&mut self, name: &str) {
        self.0.retain(|(n, _)| n != name);
    }

    /// The ids of a sidebar; a value that is not an array has none.
    fn ids(&self, name: &str) -> Vec<String> {
        self.get(name).and_then(Option::as_ref).map(|ids| ids.iter().map(|(_, id)| id.clone()).collect()).unwrap_or_default()
    }

    /// `wp_find_widgets_sidebar()`.
    fn find(&self, widget_id: &str) -> Option<&str> {
        self.0.iter().find(|(_, ids)| ids.as_ref().is_some_and(|ids| ids.iter().any(|(_, id)| id == widget_id))).map(|(name, _)| name.as_str())
    }
}

/// `array_merge()` of lists: the keys start again at 0.
fn merge(a: Option<&Ids>, b: Option<&Ids>) -> Ids {
    a.into_iter().chain(b).flatten().enumerate().map(|(i, (_, id))| (i as i64, id.clone())).collect()
}

/// `_wp_remove_unregistered_widgets()`: `array_intersect()` keeps the keys.
fn remove_unregistered(sidebars: &Sidebars, registered: &[String]) -> Sidebars {
    Sidebars(sidebars.0.iter().map(|(name, ids)| (name.clone(), ids.as_ref().map(|ids| ids.iter().filter(|(_, id)| registered.contains(id)).cloned().collect()))).collect())
}

fn non_empty(ids: &Option<Ids>) -> bool {
    ids.as_ref().is_some_and(|ids| !ids.is_empty())
}

/// `stripos( $haystack, $needle ) !== false`.
fn ci_contains(haystack: &str, needle: &str) -> bool {
    haystack.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
}

/// `wp_map_sidebars_widgets()`.
fn map_sidebars_widgets(existing: &Sidebars, registered: &[String], previous: Option<&Sidebars>, widget_ids: &[String]) -> Sidebars {
    let mut new = Sidebars(vec![("wp_inactive_widgets".to_owned(), Some(Vec::new()))]);
    if existing.0.is_empty() {
        return new;
    }
    let mut existing = existing.clone();
    for (sidebar, widgets) in existing.0.clone() {
        if sidebar == "wp_inactive_widgets" || sidebar.starts_with("orphaned_widgets") {
            let merged = merge(new.get("wp_inactive_widgets").and_then(Option::as_ref), widgets.as_ref());
            new.set("wp_inactive_widgets", Some(merged));
            existing.remove(&sidebar);
        }
    }
    // If old and new theme have just one sidebar, map it and we're done.
    if existing.0.len() == 1 && registered.len() == 1 {
        let (_, ids) = existing.0.pop().unwrap_or_default();
        new.set(&registered[0], ids);
        return new;
    }
    // Map locations with the same slug.
    let existing_names: Vec<String> = existing.0.iter().map(|(n, _)| n.clone()).collect();
    for sidebar in registered {
        if existing_names.contains(sidebar) {
            let ids = existing.get(sidebar).cloned().flatten();
            new.set(sidebar, ids);
            existing.remove(sidebar);
        } else if !new.has(sidebar) {
            new.set(sidebar, Some(Vec::new()));
        }
    }
    // If there are more sidebars, try to map them by phrases from the same group.
    if !existing.0.is_empty() {
        const GROUPS: [&[&str]; 4] = [&["sidebar", "primary", "main", "right"], &["second", "left"], &["sidebar-2", "footer", "bottom"], &["header", "top"]];
        // PHP reuses `$slug` in the innermost loop, so the outer checks see what it left behind.
        let mut slug: &str;
        for group in GROUPS {
            for outer in group {
                slug = outer;
                'new_sidebar: for new_sidebar in registered {
                    if !ci_contains(new_sidebar, slug) && !ci_contains(slug, new_sidebar) {
                        continue;
                    }
                    for (sidebar, _) in existing.0.clone() {
                        for inner in group {
                            slug = inner;
                            if !ci_contains(&sidebar, slug) && !ci_contains(slug, &sidebar) {
                                continue;
                            }
                            if existing.get(&sidebar).is_some_and(non_empty) {
                                let merged = merge(new.get(new_sidebar).and_then(Option::as_ref), existing.get(&sidebar).and_then(Option::as_ref));
                                new.set(new_sidebar, Some(merged));
                                existing.remove(&sidebar);
                                continue 'new_sidebar;
                            }
                        }
                    }
                }
            }
        }
    }
    // Move any left over widgets to inactive sidebar.
    for (_, widgets) in &existing.0 {
        if non_empty(widgets) {
            let merged = merge(new.get("wp_inactive_widgets").and_then(Option::as_ref), widgets.as_ref());
            new.set("wp_inactive_widgets", Some(merged));
        }
    }
    // Sidebars_widgets settings from when this theme was previously active.
    if let Some(previous) = previous {
        let mut old = Sidebars(previous.0.iter().filter(|(_, ids)| non_empty(ids)).cloned().collect());
        for (new_sidebar, new_widgets) in &new.0 {
            if old.has(new_sidebar) && non_empty(new_widgets) {
                old.remove(new_sidebar);
            }
        }
        old.0.retain(|(name, _)| !name.starts_with("orphaned_widgets"));
        let mut old = remove_unregistered(&old, widget_ids);
        for (_, ids) in &mut new.0 {
            if ids.is_none() {
                *ids = Some(Vec::new());
            }
        }
        for (old_sidebar, old_widgets) in old.0.clone() {
            for (new_sidebar, new_widgets) in new.0.clone() {
                for (key, widget_id) in old_widgets.iter().flatten() {
                    let Some(active_key) = new_widgets.iter().flatten().find(|(_, id)| id == widget_id).map(|(k, _)| *k) else { continue };
                    if new_sidebar == "wp_inactive_widgets" {
                        if let Some(Some(ids)) = new.0.iter_mut().find(|(n, _)| n == "wp_inactive_widgets").map(|(_, ids)| ids) {
                            ids.retain(|(k, _)| *k != active_key);
                        }
                    } else if let Some(Some(ids)) = old.0.iter_mut().find(|(n, _)| *n == old_sidebar).map(|(_, ids)| ids) {
                        ids.retain(|(k, _)| k != key);
                    }
                }
            }
        }
        // `array_merge()` with string keys: later values replace earlier ones in place.
        for (name, ids) in old.0 {
            new.set(&name, ids);
        }
    }
    new
}

/// `retrieve_widgets()`: the sidebars to store, or `None` when WordPress leaves the option alone.
fn retrieve_widgets(current: &Sidebars, theme_mod_is_array: bool, previous: Option<&Sidebars>, registered: &[String], widget_ids: &[String]) -> Option<Sidebars> {
    if !theme_mod_is_array {
        if current.0.is_empty() {
            return None;
        }
        let mut keys: Vec<&str> = current.0.iter().map(|(n, _)| n.as_str()).collect();
        let mut wanted: Vec<&str> = registered.iter().map(String::as_str).collect();
        keys.sort_unstable();
        wanted.sort_unstable();
        if keys == wanted {
            return None;
        }
    }
    let cleaned = remove_unregistered(current, widget_ids);
    let mut sidebars = map_sidebars_widgets(&cleaned, registered, previous, widget_ids);
    for (_, ids) in &mut sidebars.0 {
        if ids.is_none() {
            *ids = Some(Vec::new());
        }
    }
    // Find hidden/lost multi-widget instances; only keep active and default widgets.
    let shown: Vec<&String> = sidebars.0.iter().flat_map(|(_, ids)| ids.iter().flatten().map(|(_, id)| id)).collect();
    let lost: Ids = widget_ids
        .iter()
        .filter(|id| !shown.contains(id))
        .filter(|id| !matches!(id.rsplit_once('-'), Some((base, n)) if !base.is_empty() && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && n.parse::<u64>().is_ok_and(|n| n < 2)))
        .map(|id| (0, id.clone()))
        .collect();
    let inactive = merge(Some(&lost), sidebars.get("wp_inactive_widgets").and_then(Option::as_ref));
    sidebars.set("wp_inactive_widgets", Some(inactive));
    Some(sidebars)
}

/// What the widget routes work on: the registered sidebars and widgets and `wp_get_sidebars_widgets()` after
/// `retrieve_widgets()`.
struct WidgetState {
    sidebars: Vec<RegisteredSidebar>,
    /// `$wp_registered_widgets`: widget id -> (id base, number).
    widgets: Vec<(String, &'static str, i64)>,
    sidebars_widgets: Sidebars,
}

impl WidgetState {
    fn is_registered(&self, widget_id: &str) -> bool {
        self.widgets.iter().any(|(id, ..)| id == widget_id)
    }

    /// `wp_get_sidebar()`.
    fn sidebar(&self, id: &str, inactive_name: &str) -> Option<RegisteredSidebar> {
        if let Some(sidebar) = self.sidebars.iter().find(|s| s.id == id) {
            return Some(sidebar.clone());
        }
        (id == "wp_inactive_widgets").then(|| RegisteredSidebar { id: id.to_owned(), name: inactive_name.to_owned(), ..RegisteredSidebar::default() })
    }

    fn show_in_rest(&self, id: &str) -> bool {
        self.sidebars.iter().any(|s| s.id == id && s.show_in_rest)
    }
}

impl Rest<'_> {
    /// `WP_Widget::get_settings()` of a widget type.
    fn widget_settings(&self, id_base: &str) -> Vec<(i64, Php)> {
        let Some(Php::Array(entries)) = self.site.options.value(&format!("widget_{id_base}")) else { return Vec::new() };
        entries
            .into_iter()
            .filter_map(|(key, value)| match key {
                Key::Int(n) => Some((n, value)),
                _ => None,
            })
            .collect()
    }

    /// Registers the widgets and runs `retrieve_widgets()`, storing its result like WordPress does.
    async fn widget_state(&self) -> Result<WidgetState> {
        let sidebars = theme_widgets::registered_sidebars(self.theme, self.site);
        let mut widgets = Vec::new();
        for (id_base, ..) in WIDGET_TYPES {
            let numbers: Vec<i64> = self.widget_settings(id_base).into_iter().map(|(n, _)| n).collect();
            // A type without instances is registered once, with a generic template.
            for n in if numbers.is_empty() { vec![1] } else { numbers } {
                widgets.push((format!("{id_base}-{n}"), id_base, n));
            }
        }
        let option = self.site.options.value("sidebars_widgets");
        let mut current = Sidebars::from_php(option.as_ref());
        current.remove("array_version");
        let mods = rustpress_theme::classic::menu::theme_mod(self.site, &self.theme.stylesheet);
        let theme_mod = mods.get("sidebars_widgets");
        let theme_mod_is_array = theme_mod.is_some_and(|m| m.as_array().is_some());
        let previous = theme_mod.and_then(|m| m.get("data")).filter(|d| d.as_array().is_some()).map(|d| Sidebars::from_php(Some(d)));
        let registered: Vec<String> = sidebars.iter().map(|s| s.id.clone()).collect();
        let widget_ids: Vec<String> = widgets.iter().map(|(id, ..)| id.clone()).collect();
        let sidebars_widgets = match retrieve_widgets(&current, theme_mod_is_array, previous.as_ref(), &registered, &widget_ids) {
            Some(stored) => {
                // `wp_set_sidebars_widgets()`.
                let mut value = stored.to_php();
                if let Php::Array(entries) = &mut value {
                    entries.push((Key::Str(b"array_version".to_vec()), Php::Int(3)));
                }
                let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned();
                if let Some(Ok(writer)) = self.access.as_ref().map(|_| self.writer()) {
                    writer.update_option("sidebars_widgets", &serialized).await?;
                }
                stored
            }
            None => current,
        };
        Ok(WidgetState { sidebars, widgets, sidebars_widgets })
    }

    fn widgets_denied(&self) -> RestError {
        self.forbidden("rest_cannot_manage_widgets", "Sorry, you are not allowed to manage widgets on this site.")
    }

    fn context(args: &Args) -> String {
        args.str("context").unwrap_or_else(|| "view".to_owned())
    }

    /// The routes below `wp/v2/sidebars`, `wp/v2/widgets` and `wp/v2/widget-types`.
    pub(crate) async fn widgets_route(&self, segments: &[&str], method: &str, args: &Args) -> Result<Option<Outcome>> {
        if method != "GET" {
            return Ok(None);
        }
        let manage = self.can("edit_theme_options");
        let outcome = match segments {
            ["wp", "v2", "widget-types"] | ["wp", "v2", "widget-types", _] => {
                if !manage {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                let types = self.widget_types(&Self::context(args));
                match segments.get(3) {
                    None => {
                        let mut resp = Resp::item(Value::Array(types.into_iter().map(|(_, t)| t).collect()));
                        resp.allow = Some("GET".to_owned());
                        Ok(resp)
                    }
                    Some(id) => match types.into_iter().find(|(base, _)| base == id) {
                        Some((_, item)) => {
                            let mut resp = Resp::item(item);
                            resp.allow = Some("GET".to_owned());
                            Ok(resp)
                        }
                        None => Err(RestError::new("rest_widget_type_invalid", "Invalid widget type.", 404)),
                    },
                }
            }
            ["wp", "v2", "sidebars"] => {
                let state = self.widget_state().await?;
                let readable = |id: &str| manage || state.show_in_rest(id);
                if !manage && !state.sidebars_widgets.0.iter().any(|(id, _)| state.sidebar(id, "").is_some() && state.show_in_rest(id)) {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                let inactive = self.site.tr("Inactive widgets");
                let items = state.sidebars_widgets.0.iter().filter_map(|(id, _)| state.sidebar(id, &inactive)).filter(|s| readable(&s.id)).map(|s| self.prepare_sidebar(&state, &s, &Self::context(args))).collect();
                let mut resp = Resp::item(Value::Array(items));
                resp.allow = Some("GET".to_owned());
                Ok(resp)
            }
            ["wp", "v2", "sidebars", id] => {
                let state = self.widget_state().await?;
                let inactive = self.site.tr("Inactive widgets");
                let sidebar = state.sidebar(id, &inactive);
                if !manage && !sidebar.as_ref().is_some_and(|s| s.show_in_rest) {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                match sidebar {
                    Some(sidebar) => {
                        let mut resp = Resp::item(self.prepare_sidebar(&state, &sidebar, &Self::context(args)));
                        resp.allow = Some(if manage { "GET, POST, PUT, PATCH" } else { "GET" }.to_owned());
                        Ok(resp)
                    }
                    None => Err(RestError::new("rest_sidebar_not_found", "No sidebar exists with that id.", 404)),
                }
            }
            ["wp", "v2", "widgets"] => {
                let state = self.widget_state().await?;
                let wanted = args.str("sidebar");
                let allowed = manage || wanted.as_deref().is_some_and(|s| state.show_in_rest(s)) || state.sidebars_widgets.0.iter().any(|(id, _)| state.show_in_rest(id));
                if !allowed {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                let context = Self::context(args);
                let mut items = Vec::new();
                for (sidebar_id, ids) in &state.sidebars_widgets.0 {
                    if wanted.as_deref().is_some_and(|w| w != sidebar_id) || !(manage || state.show_in_rest(sidebar_id)) {
                        continue;
                    }
                    for (_, widget_id) in ids.iter().flatten() {
                        if let Some(item) = self.prepare_widget(&state, widget_id, sidebar_id, &context) {
                            items.push(item);
                        }
                    }
                }
                let mut resp = Resp::item(Value::Array(items));
                resp.allow = Some(if manage { "GET, POST" } else { "GET" }.to_owned());
                Ok(resp)
            }
            ["wp", "v2", "widgets", id] => {
                let state = self.widget_state().await?;
                let sidebar_id = state.sidebars_widgets.find(id).map(str::to_owned);
                if !manage && !sidebar_id.as_deref().is_some_and(|s| state.show_in_rest(s)) {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                match sidebar_id {
                    None => Err(RestError::new("rest_widget_not_found", "No widget was found with that id.", 404)),
                    Some(sidebar_id) => match self.prepare_widget(&state, id, &sidebar_id, &Self::context(args)) {
                        Some(item) => {
                            let mut resp = Resp::item(item);
                            resp.allow = Some("GET, POST, PUT, PATCH, DELETE".to_owned());
                            Ok(resp)
                        }
                        None => Err(RestError::new("rest_invalid_widget", "The requested widget is invalid.", 500)),
                    },
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    /// `WP_REST_Widget_Types_Controller::get_widgets()`, sorted by id, each prepared for the response.
    fn widget_types(&self, _context: &str) -> Vec<(String, Value)> {
        let mut types: Vec<_> = WIDGET_TYPES.iter().collect();
        types.sort_by(|a, b| a.0.cmp(b.0));
        types
            .into_iter()
            .map(|(id, name, description, classname)| {
                let mut links = Links::new();
                links.add("collection", Vec::new(), self.rest_url("wp/v2/widget-types"));
                links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/widget-types/{id}")), "targetHints": {"allow": ["GET"]}}));
                let item = json!({
                    "id": id,
                    "name": rustpress_core::formatting::html_entity_decode(&self.site.tr(name)),
                    "description": rustpress_core::formatting::html_entity_decode(&self.site.tr(description)),
                    "is_multi": true,
                    "classname": classname,
                    "_links": links.finish(),
                });
                ((*id).to_owned(), item)
            })
            .collect()
    }

    /// `WP_REST_Sidebars_Controller::prepare_item_for_response()`.
    fn prepare_sidebar(&self, state: &WidgetState, sidebar: &RegisteredSidebar, _context: &str) -> Value {
        let registered = state.sidebars.iter().any(|s| s.id == sidebar.id);
        // A block theme shows no sidebar as active.
        let active = registered && rustpress_theme::classic::is_ported(self.theme);
        let widgets: Vec<String> = state.sidebars_widgets.ids(&sidebar.id).into_iter().filter(|id| state.is_registered(id)).collect();
        let mut links = Links::new();
        links.add("collection", Vec::new(), self.rest_url("wp/v2/sidebars"));
        let allow: &[&str] = if self.can("edit_theme_options") { &["GET", "POST", "PUT", "PATCH"] } else { &["GET"] };
        links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/sidebars/{}", sidebar.id)), "targetHints": {"allow": allow}}));
        links.push_raw("wp:widget", json!({"embeddable": true, "href": format!("{}?sidebar={}", self.rest_url("wp/v2/widgets"), rustpress_core::canonical::php_urlencode(&sidebar.id))}));
        let mut item = Map::new();
        item.insert("id".into(), json!(sidebar.id));
        item.insert("name".into(), json!(sidebar.name));
        item.insert("description".into(), json!(sidebar.description));
        item.insert("class".into(), json!(sidebar.class));
        item.insert("before_widget".into(), json!(sidebar.before_widget));
        item.insert("after_widget".into(), json!(sidebar.after_widget));
        item.insert("before_title".into(), json!(sidebar.before_title));
        item.insert("after_title".into(), json!(sidebar.after_title));
        item.insert("status".into(), json!(if active { "active" } else { "inactive" }));
        item.insert("widgets".into(), json!(widgets));
        item.insert("_links".into(), links.finish());
        Value::Object(item)
    }

    /// `WP_REST_Widgets_Controller::prepare_item_for_response()`; `None` for a widget that is not registered.
    fn prepare_widget(&self, state: &WidgetState, widget_id: &str, sidebar_id: &str, context: &str) -> Option<Value> {
        let (_, id_base, number) = state.widgets.iter().find(|(id, ..)| id == widget_id)?;
        let mut item = Map::new();
        item.insert("id".into(), json!(widget_id));
        item.insert("id_base".into(), json!(id_base));
        item.insert("sidebar".into(), json!(sidebar_id));
        // `wp_render_widget()` renders only in a registered sidebar; a block theme has none.
        item.insert("rendered".into(), json!(""));
        if context == "edit" {
            let instance = self.widget_settings(id_base).into_iter().find(|(n, _)| n == number).map_or(Php::Null, |(_, v)| v);
            item.insert("rendered_form".into(), self.widget_form(id_base, *number, &instance).map_or(Value::Null, Value::String));
            let serialized = rustpress_phpser::serialize(&instance);
            let text = String::from_utf8_lossy(&serialized).into_owned();
            let hash = self.access.as_ref().and_then(|a| a.salts).and_then(|s| s.hash(&text, "auth")).unwrap_or_default();
            let empty = match &instance {
                Php::Null => true,
                Php::Array(items) => items.is_empty(),
                Php::Bool(b) => !b,
                Php::Int(n) => *n == 0,
                Php::Str(s) => s.is_empty() || s == b"0",
                _ => false,
            };
            let raw = if empty { Value::Object(Map::new()) } else { rustpress_core::options::php_to_json(&instance) };
            item.insert("instance".into(), json!({"encoded": base64::engine::general_purpose::STANDARD.encode(&serialized), "hash": hash, "raw": raw}));
        }
        let mut links = Links::new();
        links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/widgets/{widget_id}")), "targetHints": {"allow": ["GET", "POST", "PUT", "PATCH", "DELETE"]}}));
        links.add("collection", Vec::new(), self.rest_url("wp/v2/widgets"));
        links.embeddable("about", self.rest_url(&format!("wp/v2/widget-types/{id_base}")));
        links.add("wp:sidebar", Vec::new(), self.rest_url(&format!("wp/v2/sidebars/{sidebar_id}/")));
        item.insert("_links".into(), links.finish());
        Some(Value::Object(item))
    }

    /// `wp_render_widget_control()`: the form of a widget. Only the block widget's form is ported.
    fn widget_form(&self, id_base: &str, number: i64, instance: &Php) -> Option<String> {
        match id_base {
            "block" => {
                let content = match instance.get("content") {
                    Some(Php::Str(s)) => String::from_utf8_lossy(s).into_owned(),
                    _ => String::new(),
                };
                Some(format!(
                    "<p>\n\t\t\t<label for=\"widget-block-{number}-content\">\n\t\t\t\t{}\t\t\t</label>\n\t\t\t<textarea id=\"widget-block-{number}-content\" name=\"widget-block[{number}][content]\" rows=\"6\" cols=\"50\" class=\"widefat code\">{}</textarea>\n\t\t</p>",
                    self.site.tr("Block HTML:"),
                    rustpress_content::sanitize::esc_textarea(&content)
                ))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(ids: &[&str]) -> Option<Ids> {
        Some(ids.iter().enumerate().map(|(i, id)| (i as i64, (*id).to_owned())).collect())
    }

    fn widget_ids() -> Vec<String> {
        ["pages-1", "search-1", "block-2", "block-3", "block-4", "block-5"].iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn block_theme_keeps_the_sidebars_of_its_previous_visit() {
        let current = Sidebars(vec![("wp_inactive_widgets".into(), list(&[])), ("sidebar-1".into(), list(&["block-2", "block-3"])), ("sidebar-2".into(), list(&["block-4"]))]);
        let previous = current.clone();
        let stored = retrieve_widgets(&current, true, Some(&previous), &[], &widget_ids()).unwrap();
        // The instance in no sidebar is lost and found again among the inactive widgets.
        let mut expected = current;
        expected.set("wp_inactive_widgets", list(&["block-5"]));
        assert_eq!(stored, expected);
    }

    #[test]
    fn unknown_sidebars_move_to_the_inactive_widgets() {
        let current = Sidebars(vec![("wp_inactive_widgets".into(), list(&[])), ("sidebar-1".into(), list(&["block-2", "block-3", "gone-7"]))]);
        let stored = retrieve_widgets(&current, false, None, &[], &widget_ids()).unwrap();
        // Lost instances with a number of 2 or more come first.
        assert_eq!(stored, Sidebars(vec![("wp_inactive_widgets".into(), list(&["block-4", "block-5", "block-2", "block-3"]))]));
    }

    #[test]
    fn same_slug_and_phrase_groups_are_mapped() {
        let current = Sidebars(vec![("wp_inactive_widgets".into(), list(&[])), ("sidebar-1".into(), list(&["block-2"])), ("footer-widgets".into(), list(&["block-3"]))]);
        let registered = vec!["sidebar-1".to_owned(), "sidebar-2".to_owned()];
        let stored = retrieve_widgets(&current, false, None, &registered, &widget_ids()).unwrap();
        assert_eq!(stored, Sidebars(vec![("wp_inactive_widgets".into(), list(&["block-4", "block-5"])), ("sidebar-1".into(), list(&["block-2"])), ("sidebar-2".into(), list(&["block-3"]))]));
    }

    #[test]
    fn matching_registered_sidebars_are_left_alone() {
        let current = Sidebars(vec![("sidebar-1".into(), list(&["block-2"]))]);
        assert_eq!(retrieve_widgets(&current, false, None, &["sidebar-1".to_owned()], &widget_ids()), None);
    }
}
