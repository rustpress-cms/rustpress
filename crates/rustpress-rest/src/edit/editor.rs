//! The routes the block editor reads at start-up: the active theme, global styles, templates,
//! patterns, block types, comments (notes) and the admin-only lists WordPress answers with `[]`.

use serde_json::{Map, Value, json};

use super::Args;
use crate::error::RestError;
use crate::params::Params;
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

fn empty_list() -> Outcome {
    let mut resp = Resp::item(Value::Array(Vec::new()));
    resp.total = Some(0);
    resp.total_pages = Some(0);
    Ok(resp)
}

/// The value of a `Key: value` line of the header comment of `style.css`.
fn style_header(css: &str, key: &str) -> Option<String> {
    let head: String = css.chars().take(8192).collect();
    head.lines().find_map(|line| {
        let line = line.trim().trim_start_matches('*').trim();
        let (k, v) = line.split_once(':')?;
        k.trim().eq_ignore_ascii_case(key).then(|| v.trim().to_owned())
    })
}

/// PHP's `empty()` of a decoded `settings` or `styles` value: those answer with an empty object.
fn is_empty_config(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// `get_allowed_block_template_part_areas()`: `(area, label, description, icon, tag)`.
const PART_AREAS: [(&str, &str, &str, &str, &str); 4] = [
    ("uncategorized", "General", "General templates often perform a specific role like displaying post content, and are not tied to any particular area.", "layout", "div"),
    ("header", "Header", "The Header template defines a page area that typically contains a title, logo, and main navigation.", "header", "header"),
    ("footer", "Footer", "The Footer template defines a page area that typically contains site credits, social links, or any other combination of blocks.", "footer", "footer"),
    ("navigation-overlay", "Navigation Overlay", "The Navigation Overlay template defines an overlay area that typically contains navigation links and can be toggled open and closed.", "navigation-overlay", "div"),
];

impl Rest<'_> {
    async fn theme_object(&self) -> Value {
        let dir = self.theme.stylesheet_dir().to_path_buf();
        let css = std::fs::read_to_string(dir.join("style.css")).unwrap_or_default();
        let header = |key: &str| style_header(&css, key).unwrap_or_default();
        let rendered_author = {
            let name = header("Author");
            let uri = header("Author URI");
            if uri.is_empty() { name } else { format!("<a href=\"{uri}\">{name}</a>") }
        };
        let content_url = format!("{}/wp-content/themes/{}", self.site.settings.siteurl.trim_end_matches('/'), self.theme.stylesheet);
        let tags: Vec<String> = header("Tags").split(',').map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()).collect();
        let screenshot = ["png", "jpg", "jpeg", "gif", "webp"].iter().find(|e| dir.join(format!("screenshot.{e}")).is_file()).map(|e| format!("{content_url}/screenshot.{e}")).unwrap_or_default();
        let supports = json!({
            "align-wide": false, "automatic-feed-links": true, "block-templates": true, "block-template-parts": false, "custom-background": false, "custom-header": false, "custom-logo": false,
            "customize-selective-refresh-widgets": false, "dark-editor-style": false, "disable-custom-colors": false, "disable-custom-font-sizes": false, "disable-custom-gradients": false,
            "disable-layout-styles": false, "editor-color-palette": false, "editor-font-sizes": false, "editor-gradient-presets": false, "editor-spacing-sizes": false, "editor-styles": true,
            "html5": ["comment-form", "comment-list", "search-form", "gallery", "caption", "style", "script"],
            "formats": ["standard", "aside", "audio", "chat", "gallery", "image", "link", "quote", "status", "video"],
            "post-thumbnails": true, "responsive-embeds": true, "title-tag": false, "wp-block-styles": false
        });
        let mut links = Links::new();
        links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/themes/{}", self.theme.stylesheet)), "targetHints": {"allow": ["GET"]}}));
        links.add("collection", vec![], self.rest_url("wp/v2/themes"));
        let query = rustpress_db::PostQuery { post_types: vec!["wp_global_styles".into()], statuses: vec!["publish".into(), "draft".into()], names: vec![format!("wp-global-styles-{}", rustpress_core::slug::sanitize_title(&self.theme.stylesheet))], limit: Some(1), ..Default::default() };
        if let Some(post) = self.site.db.posts(&query).await.ok().and_then(|posts| posts.into_iter().next()) {
            links.add("wp:user-global-styles", vec![], self.rest_url(&format!("wp/v2/global-styles/{}", post.id)));
        }
        if self.can("edit_theme_options") {
            links.push_raw("wp:export-theme", json!({"targetHints": {"allow": ["GET"]}, "href": self.rest_url("wp-block-editor/v1/export")}));
        }
        let template_types: Vec<Value> = self.default_template_types().into_iter().map(|(slug, title, description)| json!({"title": title, "description": description, "slug": slug})).collect();
        let part_areas: Vec<Value> = PART_AREAS
            .iter()
            .map(|(area, label, description, icon, tag)| json!({"area": area, "label": self.site.tr(label), "description": self.site.tr(description), "icon": icon, "area_tag": tag}))
            .collect();
        let mut theme = json!({
            "stylesheet": self.theme.stylesheet,
            "template": self.theme.template,
            "requires_php": header("Requires PHP"),
            "requires_wp": header("Requires at least"),
            "textdomain": header("Text Domain"),
            "version": header("Version"),
            "screenshot": screenshot,
            "author": {"raw": header("Author"), "rendered": rendered_author},
            "author_uri": {"raw": header("Author URI"), "rendered": header("Author URI")},
            "description": {"raw": header("Description"), "rendered": header("Description")},
            "name": {"raw": header("Theme Name"), "rendered": header("Theme Name")},
            "tags": {"raw": tags, "rendered": tags.join(", ")},
            "theme_uri": {"raw": header("Theme URI"), "rendered": header("Theme URI")},
            "status": "active",
            "theme_supports": supports,
            "is_block_theme": true,
            "stylesheet_uri": content_url,
            "template_uri": content_url,
            "default_template_types": template_types,
            "default_template_part_areas": part_areas,
        });
        if let Value::Object(map) = &mut theme {
            map.insert("_links".into(), links.finish());
        }
        theme
    }

    /// Everything that is not a post, a term, a user, media or a setting.
    pub(crate) async fn editor_route(&self, segments: &[&str], method: &str, args: &Args, params: &Params) -> Result<Option<Outcome>> {
        let get = method == "GET";
        let outcome = match segments {
            ["wp", "v2", "themes"] if get && self.can("edit_theme_options") => {
                let list = if args.str("status").as_deref() == Some("active") || args.str("status").is_none() { vec![self.theme_object().await] } else { Vec::new() };
                let mut resp = Resp::item(Value::Array(list));
                resp.allow = Some("GET".to_owned());
                Ok(resp)
            }
            ["wp", "v2", "themes", name] if get && self.can("edit_theme_options") && *name == self.theme.stylesheet => Ok(Resp::item(self.theme_object().await)),
            ["wp", "v2", "plugins"] | ["wp", "v2", "sidebars"] | ["wp", "v2", "widgets"] | ["wp", "v2", "menus"] | ["wp", "v2", "menu-items"] | ["wp", "v2", "widget-types"] | ["wp", "v2", "menu-locations"] | ["wp", "v2", "pattern-directory", "patterns"] | ["wp", "v2", "block-directory", "search"] if get => empty_list(),
            ["wp", "v2", "block-types"] | ["wp", "v2", "block-types", _] | ["wp", "v2", "block-types", _, _] if get => empty_list(),
            ["wp", "v2", "block-patterns", "patterns"] if get => Ok(Resp::item(self.patterns_json())),
            ["wp", "v2", "block-patterns", "categories"] if get => Ok(Resp::item(self.pattern_categories())),
            ["wp", "v2", "global-styles", "themes", stylesheet] if get && self.can("edit_theme_options") => Ok(Resp::item(self.global_styles_theme(stylesheet))),
            ["wp", "v2", "global-styles", "themes", stylesheet, "variations"] if get && self.can("edit_theme_options") => {
                if *stylesheet != self.theme.stylesheet {
                    Err(RestError::new("rest_theme_not_found", "Theme not found.", 404))
                } else {
                    Ok(Resp::item(Value::Array(self.theme.style_variations())))
                }
            }
            ["wp", "v2", "global-styles", id] if self.can("edit_theme_options") && id.parse::<u64>().is_ok() => return self.global_styles_user(id, method, args).await.map(Some),
            ["wp", "v2", "comments"] if get && params.get("context") == Some("edit") => {
                // Notes (`type=note`) are not supported; the editor asks for them once.
                empty_list()
            }
            ["wp", "v2", base @ ("blocks" | "navigation")] | ["wp", "v2", base @ ("blocks" | "navigation"), _] => {
                let info = crate::meta::core_post_type(base);
                let kind = crate::posts::Kind::Custom(&info);
                return self.posts_route(kind, segments.get(3).copied(), method, args, params).await.map(Some);
            }
            ["wp", "v2", "statuses"] if get && self.can("edit_posts") => Ok(Resp::item(self.statuses_edit(params.get("context").unwrap_or("view")))),
            ["wp", "v2", "statuses", slug] if get && self.access.is_some() => {
                let all = self.statuses_edit("view");
                let internal = ["auto-draft", "inherit", "request-pending", "request-confirmed", "request-failed", "request-completed"];
                if !self.can("edit_posts") && *slug != "publish" || internal.contains(slug) {
                    Err(RestError::new("rest_cannot_read_status", "Cannot view status.", 403))
                } else if all.get(*slug).is_some() {
                    Ok(Resp::item(self.statuses_edit(params.get("context").unwrap_or("view"))[*slug].clone()))
                } else {
                    Err(RestError::new("rest_status_invalid", "Invalid status.", 404))
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    fn pattern_categories(&self) -> Value {
        Value::Array(self.theme.pattern_categories().into_iter().map(|c| json!({"name": c.name, "label": c.label, "description": c.description})).collect())
    }

    fn global_styles_theme(&self, stylesheet: &str) -> Value {
        let mut data = Map::new();
        if stylesheet == self.theme.stylesheet {
            data.insert("styles".into(), self.theme.json.styles());
            data.insert("settings".into(), self.theme.json.settings_by_origin());
        }
        let mut links = Links::new();
        links.self_link(self.rest_url(&format!("wp/v2/global-styles/themes/{stylesheet}")));
        data.insert("_links".into(), links.finish());
        Value::Object(data)
    }

    async fn global_styles_user(&self, id: &str, method: &str, args: &Args) -> Result<Outcome> {
        let id: u64 = id.parse().unwrap_or(0);
        let Some(post) = self.site.db.post(id).await?.filter(|p| p.post_type == "wp_global_styles") else {
            return Ok(Err(RestError::new("rest_global_styles_not_found", "No global styles config exists with that id.", 404)));
        };
        if matches!(method, "POST" | "PUT" | "PATCH") {
            let content = match self.content() {
                Ok(c) => c,
                Err(e) => return Ok(Err(e)),
            };
            let mut stored: Value = serde_json::from_str(&post.post_content).unwrap_or_else(|_| json!({}));
            if let Value::Object(map) = &mut stored {
                for key in ["styles", "settings"] {
                    if let Some(value) = args.get(key) {
                        map.insert(key.to_owned(), value.clone());
                    }
                }
                map.insert("isGlobalStylesUserThemeJSON".into(), Value::Bool(true));
                map.insert("version".into(), json!(3));
            }
            let data = rustpress_content::content::PostData { id: Some(id), post_content: Some(stored.to_string()), post_title: args.raw_text("title"), ..Default::default() };
            if let Err(e) = content.insert_post(&data).await {
                return Ok(Err(RestError::new("rest_cannot_edit", &e.to_string(), 500)));
            }
        }
        let post = self.site.db.post(id).await?.unwrap_or(post);
        let stored: Value = serde_json::from_str(&post.post_content).unwrap_or_else(|_| json!({}));
        let mut links = Links::new();
        let mut self_link = Map::new();
        self_link.insert("href".into(), json!(self.rest_url(&format!("wp/v2/global-styles/{id}"))));
        self_link.insert("targetHints".into(), json!({"allow": ["GET", "POST", "PUT", "PATCH"]}));
        links.push_raw("self", Value::Object(self_link));
        links.add("about", vec![], self.rest_url("wp/v2/types/wp_global_styles"));
        let revisions = self.site.db.revision_ids(id).await?.len();
        links.add("version-history", vec![("count", json!(revisions))], self.rest_url(&format!("wp/v2/global-styles/{id}/revisions")));
        links.add("wp:action-publish", vec![], self.rest_url(&format!("wp/v2/global-styles/{id}")));
        if self.can("edit_css") || self.can("edit_theme_options") {
            links.add("wp:action-edit-css", vec![], self.rest_url(&format!("wp/v2/global-styles/{id}")));
        }
        let body = json!({
            "id": id,
            "title": {"raw": if post.post_title.is_empty() { "Custom Styles".to_owned() } else { post.post_title.clone() }, "rendered": if post.post_title.is_empty() { "Custom Styles".to_owned() } else { post.post_title.clone() }},
            "settings": stored.get("settings").cloned().filter(|v| !is_empty_config(v)).unwrap_or_else(rustpress_blocks::php_json::empty_object),
            "styles": stored.get("styles").cloned().filter(|v| !is_empty_config(v)).unwrap_or_else(rustpress_blocks::php_json::empty_object),
            "_links": links.finish(),
        });
        let mut resp = Resp::item(body);
        resp.allow = Some("GET, POST, PUT, PATCH".to_owned());
        Ok(Ok(resp))
    }

    /// `WP_REST_Block_Patterns_Controller::get_items()`.
    fn patterns_json(&self) -> Value {
        let migrate = |category: &str| match category {
            "buttons" => "call-to-action".to_owned(),
            "columns" => "text".to_owned(),
            "query" => "posts".to_owned(),
            other => other.to_owned(),
        };
        let mut items = Vec::new();
        for pattern in self.registered_patterns() {
            let mut item = Map::new();
            item.insert("name".into(), json!(pattern.name));
            item.insert("title".into(), json!(pattern.title));
            item.insert("content".into(), json!(self.resolved_content(&self.with_theme_attribute(&pattern.content))));
            if !pattern.description.is_empty() {
                item.insert("description".into(), json!(pattern.description));
            }
            if let Some(width) = pattern.viewport_width {
                item.insert("viewport_width".into(), json!(width));
            }
            if let Some(inserter) = pattern.inserter {
                item.insert("inserter".into(), json!(inserter));
            }
            if !pattern.categories.is_empty() {
                item.insert("categories".into(), json!(pattern.categories.iter().map(|c| migrate(c)).collect::<Vec<_>>()));
            }
            for (key, list) in [("keywords", &pattern.keywords), ("block_types", &pattern.block_types), ("post_types", &pattern.post_types), ("template_types", &pattern.template_types)] {
                if !list.is_empty() {
                    item.insert(key.into(), json!(list));
                }
            }
            if let Some(source) = &pattern.source {
                item.insert("source".into(), json!(source));
            }
            items.push(Value::Object(item));
        }
        Value::Array(items)
    }

    /// `WP_REST_Post_Statuses_Controller`: the statuses of a signed-in editor, with the fields of the context.
    fn statuses_edit(&self, context: &str) -> Value {
        let s = |t: &str| self.site.tr(t);
        let entry = |slug: &str, name: String, private: bool, protected: bool, public: bool, queryable: bool, show_in_list: bool, floating: bool| {
            let mut data = Map::new();
            data.insert("name".into(), json!(name));
            if context == "edit" {
                data.insert("private".into(), json!(private));
                data.insert("protected".into(), json!(protected));
            }
            if context != "embed" {
                data.insert("public".into(), json!(public));
                data.insert("queryable".into(), json!(queryable));
            }
            if context == "edit" {
                data.insert("show_in_list".into(), json!(show_in_list));
            }
            data.insert("slug".into(), json!(slug));
            if context != "embed" {
                data.insert("date_floating".into(), json!(floating));
            }
            let mut links = Links::new();
            links.add("archives", vec![], self.rest_url(&if slug == "publish" { "wp/v2/posts".to_owned() } else { format!("wp/v2/posts?status={slug}") }));
            data.insert("_links".into(), links.finish());
            Value::Object(data)
        };
        json!({
            "publish": entry("publish", s("Published"), false, false, true, true, true, false),
            "future": entry("future", s("Scheduled"), false, true, false, false, true, false),
            "draft": entry("draft", s("Draft"), false, true, false, false, true, true),
            "pending": entry("pending", s("Pending"), false, true, false, false, true, true),
            "private": entry("private", s("Private"), true, false, false, false, true, false),
            "trash": entry("trash", s("Trash"), false, false, false, false, false, false),
        })
    }
}
