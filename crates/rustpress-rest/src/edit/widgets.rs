//! `wp/v2/sidebars`, `wp/v2/widgets` and `wp/v2/widget-types` (`WP_REST_Sidebars_Controller`,
//! `WP_REST_Widgets_Controller`, `WP_REST_Widget_Types_Controller`). Like WordPress, every read first runs
//! `retrieve_widgets()`, which maps the widgets of sidebars the theme does not have and stores the result. The options
//! are read from the table: the requests of a batch see what the ones before them saved.

use base64::Engine;
use rustpress_phpser::{Key, Value as Php};
use rustpress_theme::classic::widgets::{self as theme_widgets, RegisteredSidebar};
use serde_json::{Map, Value, json};

use super::Args;
use super::widget_classes::{array_key, json_to_php, parse_str, string};
use crate::error::RestError;
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

/// The core widgets in the order `wp_widgets_init()` registers them: id base, name, description, class name.
pub const WIDGET_TYPES: [(&str, &str, &str, &str); 18] = [
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
    pub(crate) fn set(&mut self, name: &str, ids: Option<Ids>) {
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

    /// `$array[ $name ][] = $id`.
    fn append(&mut self, name: &str, id: &str) {
        if !matches!(self.get(name), Some(Some(_))) {
            self.set(name, Some(Vec::new()));
        }
        if let Some((_, Some(ids))) = self.0.iter_mut().find(|(n, _)| n == name) {
            let next = ids.iter().map(|(k, _)| *k + 1).max().unwrap_or(0);
            ids.push((next, id.to_owned()));
        }
    }

    /// `wp_assign_widget_to_sidebar()`: an empty sidebar takes the widget out of all of them.
    fn assign(&mut self, widget_id: &str, sidebar_id: &str) {
        for (name, ids) in &mut self.0 {
            if name != sidebar_id
                && let Some(ids) = ids
                && let Some(at) = ids.iter().position(|(_, id)| id == widget_id)
            {
                ids.remove(at);
            }
        }
        if !sidebar_id.is_empty() && sidebar_id != "0" {
            self.append(sidebar_id, widget_id);
        }
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
    /// `WP_Widget::get_settings()` of each type.
    settings: Vec<(&'static str, Vec<(Key, Php)>)>,
    /// The widget `save_widget()` registered with `_register_one( $number )`, even without settings.
    saved: Option<String>,
}

impl WidgetState {
    fn is_registered(&self, widget_id: &str) -> bool {
        self.widgets.iter().any(|(id, ..)| id == widget_id)
    }

    fn instances(&self, id_base: &str) -> &[(Key, Php)] {
        self.settings.iter().find(|(base, _)| *base == id_base).map_or(&[], |(_, entries)| entries.as_slice())
    }

    /// `next_widget_id_number()`. The pattern has no start: it matches the end of any widget id.
    fn next_number(&self, id_base: &str) -> i64 {
        let suffix = format!("{id_base}-");
        let highest = self.widgets.iter().filter_map(|(id, ..)| id.rsplit_once('-').filter(|(base, n)| format!("{base}-").ends_with(&suffix) && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))?.1.parse::<i64>().ok()).max();
        highest.unwrap_or(1).max(1) + 1
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

/// The controllers that keep a `$widgets_retrieved` of their own.
#[derive(Clone, Copy)]
enum Controller {
    Sidebars = 0,
    Widgets = 1,
}

/// `wp_parse_widget_id()`.
fn parse_widget_id(id: &str) -> (&str, Option<&str>) {
    match id.rsplit_once('-') {
        Some((base, number)) if !base.is_empty() && !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) => (base, Some(number)),
        _ => (id, None),
    }
}

impl Rest<'_> {
    /// Registers the widgets and reads `wp_get_sidebars_widgets()`, without `retrieve_widgets()`.
    async fn widget_registry(&self) -> Result<WidgetState> {
        let mut names: Vec<String> = WIDGET_TYPES.iter().map(|(id_base, ..)| format!("widget_{id_base}")).collect();
        names.push("sidebars_widgets".to_owned());
        let rows = self.site.db.options_named(&names.iter().map(String::as_str).collect::<Vec<_>>()).await?;
        let option = |name: &str| rows.iter().find(|(n, _)| n == name).map(|(_, value)| rustpress_core::options::maybe_unserialize(value));
        let mut settings = Vec::new();
        let mut widgets = Vec::new();
        for (id_base, ..) in WIDGET_TYPES {
            // `get_settings()` leaves out `_multiwidget` and `__i__`.
            let entries: Vec<(Key, Php)> = match option(&format!("widget_{id_base}")) {
                Some(Php::Array(entries)) => entries.into_iter().filter(|(key, _)| !matches!(key, Key::Str(k) if k == b"_multiwidget" || k == b"__i__")).collect(),
                _ => Vec::new(),
            };
            let numbers: Vec<i64> = entries.iter().filter_map(|(key, _)| if let Key::Int(n) = key { Some(*n) } else { None }).collect();
            // A type without instances is registered once, with a generic template.
            for n in if numbers.is_empty() { vec![1] } else { numbers } {
                widgets.push((format!("{id_base}-{n}"), id_base, n));
            }
            settings.push((id_base, entries));
        }
        let mut sidebars_widgets = Sidebars::from_php(option("sidebars_widgets").as_ref());
        sidebars_widgets.remove("array_version");
        Ok(WidgetState { sidebars: theme_widgets::registered_sidebars(self.theme, self.site), widgets, sidebars_widgets, settings, saved: None })
    }

    /// `wp_set_sidebars_widgets()`.
    async fn store_sidebars(&self, sidebars: &Sidebars) -> Result<()> {
        let mut value = sidebars.to_php();
        if let Php::Array(entries) = &mut value {
            entries.push((Key::Str(b"array_version".to_vec()), Php::Int(3)));
        }
        let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned();
        if let Some(Ok(writer)) = self.access.as_ref().map(|_| self.writer()) {
            writer.update_option("sidebars_widgets", &serialized).await?;
        }
        Ok(())
    }

    /// `WP_Widget::save_settings()`.
    async fn store_widget_settings(&self, id_base: &str, entries: Vec<(Key, Php)>) -> Result<()> {
        let mut entries = entries;
        entries.push((Key::Str(b"_multiwidget".to_vec()), Php::Int(1)));
        let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&Php::Array(entries))).into_owned();
        if let Some(Ok(writer)) = self.access.as_ref().map(|_| self.writer()) {
            writer.update_option(&format!("widget_{id_base}"), &serialized).await?;
        }
        Ok(())
    }

    /// Registers the widgets and runs `retrieve_widgets()` when the controller has not done so in this request,
    /// storing its result like WordPress does.
    async fn widget_state(&self, controller: Controller) -> Result<WidgetState> {
        let mut state = self.widget_registry().await?;
        let current = std::mem::take(&mut state.sidebars_widgets);
        self.retrieve(&mut state, current, controller).await?;
        Ok(state)
    }

    /// `rest_send_allow_header()` asks the permission of each method of the route, and the check of the `GET` method runs
    /// `retrieve_widgets()`: on the global `$sidebars_widgets`, which holds what `wp_get_sidebars_widgets()` read last. After
    /// a widget was created that is the state before it, so the widget counts as lost and moves to the inactive ones.
    async fn retrieve_after_dispatch(&self, controller: Controller, global: Sidebars, saved: Option<(String, &'static str, i64)>) -> Result<()> {
        if self.widgets_retrieved[controller as usize].load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        let mut state = self.widget_registry().await?;
        if let Some(widget) = saved
            && !state.is_registered(&widget.0)
        {
            state.widgets.push(widget);
        }
        self.retrieve(&mut state, global, controller).await
    }

    async fn retrieve(&self, state: &mut WidgetState, current: Sidebars, controller: Controller) -> Result<()> {
        if self.widgets_retrieved[controller as usize].swap(true, std::sync::atomic::Ordering::SeqCst) {
            state.sidebars_widgets = current;
            return Ok(());
        }
        let mods = rustpress_theme::classic::menu::theme_mod(self.site, &self.theme.stylesheet);
        let theme_mod = mods.get("sidebars_widgets");
        let theme_mod_is_array = theme_mod.is_some_and(|m| m.as_array().is_some());
        let previous = theme_mod.and_then(|m| m.get("data")).filter(|d| d.as_array().is_some()).map(|d| Sidebars::from_php(Some(d)));
        let registered: Vec<String> = state.sidebars.iter().map(|s| s.id.clone()).collect();
        let widget_ids: Vec<String> = state.widgets.iter().map(|(id, ..)| id.clone()).collect();
        state.sidebars_widgets = match retrieve_widgets(&current, theme_mod_is_array, previous.as_ref(), &registered, &widget_ids) {
            Some(stored) => {
                self.store_sidebars(&stored).await?;
                stored
            }
            None => current,
        };
        Ok(())
    }

    fn widgets_denied(&self) -> RestError {
        self.forbidden("rest_cannot_manage_widgets", "Sorry, you are not allowed to manage widgets on this site.")
    }

    fn context(args: &Args) -> String {
        args.str("context").unwrap_or_else(|| "view".to_owned())
    }

    /// The routes below `wp/v2/sidebars`, `wp/v2/widgets` and `wp/v2/widget-types`.
    pub(crate) async fn widgets_route(&self, segments: &[&str], method: &str, args: &Args) -> Result<Option<Outcome>> {
        let manage = self.can("edit_theme_options");
        if method != "GET" {
            return self.widgets_write(segments, method, args, manage).await;
        }
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
                let state = self.widget_state(Controller::Sidebars).await?;
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
                let state = self.widget_state(Controller::Sidebars).await?;
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
                let state = self.widget_state(Controller::Widgets).await?;
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
                        if let Some(item) = self.prepare_widget(&state, widget_id, sidebar_id, &context).await? {
                            items.push(item);
                        }
                    }
                }
                let mut resp = Resp::item(Value::Array(items));
                resp.allow = Some(if manage { "GET, POST" } else { "GET" }.to_owned());
                Ok(resp)
            }
            ["wp", "v2", "widgets", id] => {
                let state = self.widget_state(Controller::Widgets).await?;
                let sidebar_id = state.sidebars_widgets.find(id).map(str::to_owned);
                if !manage && !sidebar_id.as_deref().is_some_and(|s| state.show_in_rest(s)) {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                match sidebar_id {
                    None => Err(RestError::new("rest_widget_not_found", "No widget was found with that id.", 404)),
                    Some(sidebar_id) => match self.prepare_widget(&state, id, &sidebar_id, &Self::context(args)).await? {
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

    /// The routes that change something.
    async fn widgets_write(&self, segments: &[&str], method: &str, args: &Args, manage: bool) -> Result<Option<Outcome>> {
        let edit = matches!(method, "POST" | "PUT" | "PATCH");
        let outcome = match segments {
            ["wp", "v2", "widget-types", id, action @ ("encode" | "render")] if method == "POST" => {
                if !manage {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                // `get_item_permissions_check()` looks the type up.
                if !WIDGET_TYPES.iter().any(|(base, ..)| base == id) {
                    return Ok(Some(Err(RestError::new("rest_widget_type_invalid", "Invalid widget type.", 404))));
                }
                match *action {
                    "encode" => self.encode_form_data(id, args).await?,
                    _ => self.render_preview(id, args).await?,
                }
            }
            ["wp", "v2", "sidebars", id] if edit => {
                if !manage {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                self.update_sidebar(id, args).await?
            }
            ["wp", "v2", "widgets"] if method == "POST" => {
                if !manage {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                self.create_widget(args).await?
            }
            ["wp", "v2", "widgets", id] if edit => {
                if !manage {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                self.update_widget(id, args).await?
            }
            ["wp", "v2", "widgets", id] if method == "DELETE" => {
                if !manage {
                    return Ok(Some(Err(self.widgets_denied())));
                }
                self.delete_widget(id, args).await?
            }
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    /// `WP_REST_Sidebars_Controller::update_item()`.
    async fn update_sidebar(&self, id: &str, args: &Args) -> Result<Outcome> {
        let mut state = self.widget_registry().await?;
        if let Some(Value::Array(list)) = args.get("widgets") {
            let requested: Vec<String> = list.iter().filter_map(|v| v.as_str().map(str::to_owned).or_else(|| v.as_i64().map(|n| n.to_string()))).collect();
            for (sidebar_id, widgets) in state.sidebars_widgets.0.clone() {
                for (key, widget_id) in widgets.iter().flatten() {
                    // The widgets named leave any other sidebar, and those left out go to the inactive ones.
                    if sidebar_id != id
                        && requested.contains(widget_id)
                        && let Some((_, Some(ids))) = state.sidebars_widgets.0.iter_mut().find(|(name, _)| *name == sidebar_id)
                    {
                        ids.retain(|(k, _)| k != key);
                    }
                    if sidebar_id == id && !requested.contains(widget_id) {
                        state.sidebars_widgets.append("wp_inactive_widgets", widget_id);
                    }
                }
            }
            state.sidebars_widgets.set(id, Some(requested.into_iter().enumerate().map(|(i, widget_id)| (i as i64, widget_id)).collect()));
            self.store_sidebars(&state.sidebars_widgets).await?;
        }
        let inactive = self.site.tr("Inactive widgets");
        let Some(sidebar) = state.sidebar(id, &inactive) else { return Ok(Err(RestError::new("rest_sidebar_not_found", "No sidebar exists with that id.", 404))) };
        let mut resp = Resp::item(self.prepare_sidebar(&state, &sidebar, "edit"));
        resp.allow = Some("GET, POST, PUT, PATCH".to_owned());
        self.retrieve_after_dispatch(Controller::Sidebars, state.sidebars_widgets, None).await?;
        Ok(Ok(resp))
    }

    /// `WP_REST_Widgets_Controller::create_item()`.
    async fn create_widget(&self, args: &Args) -> Result<Outcome> {
        let mut state = self.widget_registry().await?;
        let sidebar_id = args.str("sidebar").unwrap_or_else(|| "wp_inactive_widgets".to_owned());
        let widget_id = match self.save_widget(&mut state, args, args.str("id").as_deref(), Some(&sidebar_id)).await? {
            Ok(widget_id) => widget_id,
            Err(error) => {
                self.retrieve_after_dispatch(Controller::Widgets, state.sidebars_widgets, None).await?;
                return Ok(Err(error));
            }
        };
        let global = state.sidebars_widgets.clone();
        state.sidebars_widgets.assign(&widget_id, &sidebar_id);
        self.store_sidebars(&state.sidebars_widgets).await?;
        let prepared = self.prepare_widget(&state, &widget_id, &sidebar_id, "edit").await?;
        let saved = state.widgets.iter().find(|(id, ..)| *id == widget_id).cloned();
        self.retrieve_after_dispatch(Controller::Widgets, global, saved).await?;
        match prepared {
            Some(item) => {
                let mut resp = Resp::item(item);
                resp.status = Some(201);
                resp.allow = Some("GET, POST".to_owned());
                Ok(Ok(resp))
            }
            None => Ok(Err(RestError::new("rest_invalid_widget", "The requested widget is invalid.", 500))),
        }
    }

    /// `WP_REST_Widgets_Controller::update_item()`.
    async fn update_widget(&self, id: &str, args: &Args) -> Result<Outcome> {
        let mut state = self.widget_state(Controller::Widgets).await?;
        let mut sidebar_id = state.sidebars_widgets.find(id).map(str::to_owned);
        let known = WIDGET_TYPES.iter().any(|(base, ..)| *base == parse_widget_id(id).0);
        if sidebar_id.is_none() && known {
            return Ok(Err(RestError::new("rest_widget_not_found", "No widget was found with that id.", 404)));
        }
        if args.has("instance") || args.has("form_data") {
            let id_owned = args.str("id").unwrap_or_else(|| id.to_owned());
            if let Err(error) = self.save_widget(&mut state, args, Some(&id_owned), sidebar_id.as_deref()).await? {
                return Ok(Err(error));
            }
        }
        if args.has("sidebar") {
            let wanted = args.str("sidebar").unwrap_or_default();
            if sidebar_id.as_deref() != Some(wanted.as_str()) {
                state.sidebars_widgets.assign(id, &wanted);
                self.store_sidebars(&state.sidebars_widgets).await?;
                sidebar_id = Some(wanted);
            }
        }
        let prepared = match &sidebar_id {
            Some(sidebar_id) => self.prepare_widget(&state, id, sidebar_id, "edit").await?,
            None => None,
        };
        match prepared {
            Some(item) => {
                let mut resp = Resp::item(item);
                resp.allow = Some("GET, POST, PUT, PATCH, DELETE".to_owned());
                Ok(Ok(resp))
            }
            None => Ok(Err(RestError::new("rest_invalid_widget", "The requested widget is invalid.", 500))),
        }
    }

    /// `WP_REST_Widgets_Controller::delete_item()`.
    async fn delete_widget(&self, id: &str, args: &Args) -> Result<Outcome> {
        let mut state = self.widget_state(Controller::Widgets).await?;
        let Some(sidebar_id) = state.sidebars_widgets.find(id).map(str::to_owned) else {
            return Ok(Err(RestError::new("rest_widget_not_found", "No widget was found with that id.", 404)));
        };
        let data = if args.bool("force").unwrap_or(false) {
            let Some(previous) = self.prepare_widget(&state, id, &sidebar_id, "edit").await? else {
                return Ok(Err(RestError::new("rest_invalid_widget", "The requested widget is invalid.", 500)));
            };
            let (id_base, number) = parse_widget_id(id);
            // `update_callback()` with `delete_widget`.
            if let Some(base) = WIDGET_TYPES.iter().map(|(base, ..)| *base).find(|base| *base == id_base) {
                let mut entries = state.instances(base).to_vec();
                if let Some(number) = number {
                    let key = array_key(number);
                    entries.retain(|(k, _)| *k != key);
                }
                self.store_widget_settings(base, entries).await?;
            }
            state.sidebars_widgets.assign(id, "");
            self.store_sidebars(&state.sidebars_widgets).await?;
            // The links stay on the response, outside of its data.
            let mut previous = previous;
            let links = previous.as_object_mut().and_then(|item| item.shift_remove("_links")).unwrap_or(Value::Null);
            json!({"deleted": true, "previous": previous, "_links": links})
        } else {
            state.sidebars_widgets.assign(id, "wp_inactive_widgets");
            self.store_sidebars(&state.sidebars_widgets).await?;
            match self.prepare_widget(&state, id, "wp_inactive_widgets", "edit").await? {
                Some(item) => item,
                None => return Ok(Err(RestError::new("rest_invalid_widget", "The requested widget is invalid.", 500))),
            }
        };
        let mut resp = Resp::item(data);
        resp.allow = Some("GET, POST, PUT, PATCH, DELETE".to_owned());
        Ok(Ok(resp))
    }

    /// `wp_hash()` of a serialized instance.
    fn instance_hash(&self, serialized: &[u8]) -> String {
        let text = String::from_utf8_lossy(serialized).into_owned();
        self.access.as_ref().and_then(|a| a.salts).and_then(|s| s.hash(&text, "auth")).unwrap_or_default()
    }

    /// The instance of `{ "encoded": ..., "hash": ... }`, when its hash is right.
    fn signed_instance(&self, value: &Value) -> Option<Php> {
        let encoded = value.get("encoded")?.as_str()?;
        let hash = value.get("hash")?.as_str()?;
        let serialized = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap_or_default();
        if self.instance_hash(&serialized) != hash {
            return None;
        }
        Some(rustpress_phpser::unserialize(&serialized).unwrap_or(Php::Bool(false)))
    }

    /// `WP_REST_Widgets_Controller::save_widget()`.
    async fn save_widget(&self, state: &mut WidgetState, args: &Args, id: Option<&str>, sidebar_id: Option<&str>) -> Result<std::result::Result<String, RestError>> {
        let invalid = |message: &str| Ok(Err(RestError::new("rest_invalid_widget", message, 400)));
        let wanted_base = args.str("id_base").filter(|base| !base.is_empty() && base != "0");
        let (widget_id, id_base, number) = match (id, wanted_base) {
            (Some(id), _) => {
                let (base, number) = parse_widget_id(id);
                (id.to_owned(), base.to_owned(), number.map(str::to_owned))
            }
            (None, Some(base)) => {
                let known = WIDGET_TYPES.iter().any(|(id_base, ..)| *id_base == base);
                let number = known.then(|| state.next_number(&base));
                (number.map_or_else(|| base.clone(), |n| format!("{base}-{n}")), base, number.map(|n| n.to_string()))
            }
            (None, None) => return invalid("Widget type (id_base) is required."),
        };
        let Some(base) = WIDGET_TYPES.iter().map(|(base, ..)| *base).find(|base| *base == id_base) else {
            return invalid("The provided widget type (id_base) cannot be updated.");
        };
        // What `$_POST` holds while `update_callback()` runs.
        let mut form = Php::Array(Vec::new());
        match (args.get("instance").filter(|v| !v.is_null()), args.str("form_data")) {
            (Some(instance), _) => {
                let new = if let Some(raw) = instance.get("raw").filter(|v| !v.is_null()) {
                    json_to_php(raw)
                } else if instance.get("encoded").is_some_and(|v| !v.is_null()) && instance.get("hash").is_some_and(|v| !v.is_null()) {
                    match self.signed_instance(instance) {
                        Some(new) => new,
                        None => return invalid("The provided instance is malformed."),
                    }
                } else {
                    return invalid("The provided instance is invalid. Must contain raw OR encoded and hash.");
                };
                let key = array_key(number.as_deref().unwrap_or(""));
                if let Php::Array(items) = &mut form {
                    items.push((Key::Str(format!("widget-{base}").into_bytes()), Php::Array(vec![(key, new)])));
                    items.push((Key::Str(b"sidebar".to_vec()), sidebar_id.map_or(Php::Null, |s| Php::Str(s.as_bytes().to_vec()))));
                }
            }
            (None, Some(data)) => form = parse_str(&data),
            (None, None) => {}
        }
        // `WP_Widget::update_callback()`.
        let submitted: Option<Vec<(Key, Php)>> = match form.get(&format!("widget-{base}")) {
            Some(Php::Array(items)) => Some(items.clone()),
            _ if string(form.get("id_base")) == base => {
                let multi = string(form.get("multi_number")).parse::<i64>().unwrap_or(0);
                let number = if multi != 0 { multi } else { string(form.get("widget_number")).parse::<i64>().unwrap_or(0) };
                Some(vec![(Key::Int(number), Php::Array(Vec::new()))])
            }
            _ => None,
        };
        if let Some(submitted) = submitted {
            let mut entries = state.instances(base).to_vec();
            if let Some((number, new)) = submitted.into_iter().next() {
                let old = entries.iter().find(|(key, _)| *key == number).map_or_else(|| Php::Array(Vec::new()), |(_, value)| value.clone());
                let instance = self.widget_update(base, &new, &old);
                match entries.iter_mut().find(|(key, _)| *key == number) {
                    Some(slot) => slot.1 = instance,
                    None => entries.push((number, instance)),
                }
            }
            self.store_widget_settings(base, entries).await?;
        }
        // The sidebars stay as the caller has them; it stores them.
        let sidebars = std::mem::take(&mut state.sidebars_widgets);
        *state = self.widget_registry().await?;
        state.sidebars_widgets = sidebars;
        // `_set( $number )` and `_register_one( $number )`.
        if !state.is_registered(&widget_id)
            && let Some(number) = number.and_then(|n| n.parse().ok())
        {
            state.widgets.push((widget_id.clone(), base, number));
        }
        state.saved = Some(widget_id.clone());
        Ok(Ok(widget_id))
    }

    /// `WP_REST_Widget_Types_Controller::render()`.
    async fn render_preview(&self, id: &str, args: &Args) -> Result<Outcome> {
        let signed = args.get("instance").filter(|v| v.get("encoded").is_some_and(|e| !e.is_null()) && v.get("hash").is_some_and(|h| !h.is_null()));
        let instance = signed.and_then(|signed| self.signed_instance(signed));
        // `render_block_core_legacy_widget()` shows nothing for an instance that is not signed right.
        let id = if signed.is_some() && instance.is_none() { "" } else { id };
        let preview = if rustpress_theme::classic::is_ported(self.theme) { rustpress_theme::classic::legacy_widget_preview(self.theme, self.site, &self.blank, self.registry, id, instance.as_ref()).await? } else { String::new() };
        let mut resp = Resp::item(json!({"preview": preview}));
        resp.allow = Some("POST".to_owned());
        Ok(Ok(resp))
    }

    /// `WP_REST_Widget_Types_Controller::encode_form_data()`.
    async fn encode_form_data(&self, id: &str, args: &Args) -> Result<Outcome> {
        let Some(base) = WIDGET_TYPES.iter().map(|(base, ..)| *base).find(|base| base == &id) else {
            return Ok(Err(RestError::new("rest_invalid_widget", "Cannot preview a widget that does not extend WP_Widget.", 400)));
        };
        let number = args.str("number").and_then(|n| n.trim().parse::<i64>().ok()).unwrap_or(-1).to_string();
        let mut instance = Php::Array(Vec::new());
        if let Some(signed) = args.get("instance").filter(|v| v.get("encoded").is_some_and(|e| !e.is_null()) && v.get("hash").is_some_and(|h| !h.is_null())) {
            match self.signed_instance(signed) {
                Some(value) => instance = value,
                None => return Ok(Err(RestError::new("rest_invalid_widget", "The provided instance is malformed.", 400))),
            }
        }
        if let Some(data) = args.str("form_data") {
            let form = parse_str(&data);
            if let Some(Php::Array(items)) = form.get(&format!("widget-{id}"))
                && let Some((_, new)) = items.first()
            {
                instance = self.widget_update(base, new, &instance);
            }
        }
        let serialized = rustpress_phpser::serialize(&instance);
        let form = self.widget_form(base, &number, &instance, self.rich_text().await).await;
        let preview = self.widget_preview(base, &instance).await?;
        let mut encoded = Map::new();
        encoded.insert("encoded".into(), json!(base64::engine::general_purpose::STANDARD.encode(&serialized)));
        encoded.insert("hash".into(), json!(self.instance_hash(&serialized)));
        encoded.insert("raw".into(), Self::raw_instance(&instance));
        let mut data = Map::new();
        data.insert("form".into(), json!(php_trim(&form)));
        data.insert("preview".into(), json!(php_trim(&preview)));
        data.insert("instance".into(), Value::Object(encoded));
        let mut resp = Resp::item(Value::Object(data));
        resp.allow = Some("POST".to_owned());
        Ok(Ok(resp))
    }

    /// `empty( $instance ) ? new stdClass() : $instance`.
    fn raw_instance(instance: &Php) -> Value {
        let empty = match instance {
            Php::Null => true,
            Php::Array(items) => items.is_empty(),
            Php::Bool(b) => !b,
            Php::Int(n) => *n == 0,
            Php::Str(s) => s.is_empty() || s == b"0",
            _ => false,
        };
        if empty { rustpress_blocks::php_json::empty_object() } else { rustpress_core::options::php_to_json(instance) }
    }

    /// `user_can_richedit()`.
    async fn rich_text(&self) -> bool {
        let Some(user) = self.user() else { return false };
        let setting = self.site.db.user_meta(user.id, "rich_editing").await.ok().flatten();
        setting.as_deref() == Some("true") && self.request.as_ref().and_then(|r| r.user_agent.as_deref()).is_some_and(rustpress_core::formatting::browser_can_richedit)
    }

    /// `WP_REST_Widget_Types_Controller::get_widget_preview()`.
    async fn widget_preview(&self, base: &str, instance: &Php) -> Result<String> {
        if !rustpress_theme::classic::is_ported(self.theme) {
            return Ok(String::new());
        }
        let env = crate::util::env(self);
        let search = || rustpress_theme::classic::seventeen::widget_search_form(self.site, &self.blank, &env);
        let services = theme_widgets::Services { site: self.site, q: &self.blank, env: &env, stylesheet: &self.theme.stylesheet, request_uri: "", search_form: &search };
        Ok(theme_widgets::the_widget(&services, base, instance).await?)
    }

    /// `wp_render_widget()` in a sidebar of a theme that is ported.
    async fn render_widget(&self, state: &WidgetState, widget_id: &str, sidebar_id: &str, id_base: &str, instance: &Php) -> Result<String> {
        let Some(sidebar) = state.sidebars.iter().find(|s| s.id == sidebar_id) else { return Ok(String::new()) };
        if !rustpress_theme::classic::is_ported(self.theme) || !matches!(instance, Php::Array(_)) {
            return Ok(String::new());
        }
        let wrap = theme_widgets::Sidebar { id: &sidebar.id, before_widget: &sidebar.before_widget, after_widget: &sidebar.after_widget, before_title: &sidebar.before_title, after_title: &sidebar.after_title };
        let env = crate::util::env(self);
        let search = || rustpress_theme::classic::seventeen::widget_search_form(self.site, &self.blank, &env);
        let services = theme_widgets::Services { site: self.site, q: &self.blank, env: &env, stylesheet: &self.theme.stylesheet, request_uri: "", search_form: &search };
        Ok(theme_widgets::render_widget(&services, &wrap, widget_id, id_base, instance).await?)
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
                    "name": rustpress_core::formatting::html_entity_decode(&if *id == "search" { self.site.trx(name, "Search widget") } else { self.site.tr(name) }),
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
        links.push_raw("wp:widget", json!({"embeddable": true, "href": rustpress_core::urls::add_query_arg(&self.rest_url("wp/v2/widgets"), "sidebar", &sidebar.id)}));
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
    async fn prepare_widget(&self, state: &WidgetState, widget_id: &str, sidebar_id: &str, context: &str) -> Result<Option<Value>> {
        let Some((_, id_base, number)) = state.widgets.iter().find(|(id, ..)| id == widget_id) else { return Ok(None) };
        let stored = state.instances(id_base).iter().find(|(key, _)| *key == Key::Int(*number)).map(|(_, value)| value.clone());
        let instance = stored.clone().unwrap_or(Php::Null);
        let mut item = Map::new();
        item.insert("id".into(), json!(widget_id));
        item.insert("id_base".into(), json!(id_base));
        item.insert("sidebar".into(), json!(sidebar_id));
        let rendered = if sidebar_id == "wp_inactive_widgets" { String::new() } else { self.render_widget(state, widget_id, sidebar_id, id_base, &instance).await? };
        item.insert("rendered".into(), json!(php_trim(&rendered)));
        if context == "edit" {
            // A type without instances shows the form with a number to fill in later.
            let shown = if stored.is_some() || state.saved.as_deref() == Some(widget_id) { number.to_string() } else { "__i__".to_owned() };
            let empty = Php::Array(Vec::new());
            let form = self.widget_form(id_base, &shown, stored.as_ref().unwrap_or(&empty), self.rich_text().await).await;
            item.insert("rendered_form".into(), json!(php_trim(&form)));
            let serialized = rustpress_phpser::serialize(&instance);
            item.insert("instance".into(), json!({"encoded": base64::engine::general_purpose::STANDARD.encode(&serialized), "hash": self.instance_hash(&serialized), "raw": Self::raw_instance(&instance)}));
        }
        let mut links = Links::new();
        links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/widgets/{widget_id}")), "targetHints": {"allow": ["GET", "POST", "PUT", "PATCH", "DELETE"]}}));
        links.add("collection", Vec::new(), self.rest_url("wp/v2/widgets"));
        links.embeddable("about", self.rest_url(&format!("wp/v2/widget-types/{id_base}")));
        links.add("wp:sidebar", Vec::new(), self.rest_url(&format!("wp/v2/sidebars/{sidebar_id}/")));
        item.insert("_links".into(), links.finish());
        Ok(Some(Value::Object(item)))
    }
}

/// `trim()`.
fn php_trim(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{b}'])
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
