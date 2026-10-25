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

impl Rest<'_> {
    fn theme_object(&self) -> Value {
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
        links.add("self", vec![], self.rest_url(&format!("wp/v2/themes/{}", self.theme.stylesheet)));
        links.add("collection", vec![], self.rest_url("wp/v2/themes"));
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
            "default_template_types": [],
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
                let list = if args.str("status").as_deref() == Some("active") || args.str("status").is_none() { vec![self.theme_object()] } else { Vec::new() };
                let mut resp = Resp::item(Value::Array(list));
                resp.allow = Some("GET".to_owned());
                Ok(resp)
            }
            ["wp", "v2", "themes", name] if get && self.can("edit_theme_options") && *name == self.theme.stylesheet => Ok(Resp::item(self.theme_object())),
            ["wp", "v2", "plugins"] | ["wp", "v2", "sidebars"] | ["wp", "v2", "widgets"] | ["wp", "v2", "menus"] | ["wp", "v2", "menu-items"] | ["wp", "v2", "widget-types"] | ["wp", "v2", "menu-locations"] | ["wp", "v2", "font-families"] | ["wp", "v2", "template-parts"] | ["wp", "v2", "block-patterns", "patterns"] | ["wp", "v2", "pattern-directory", "patterns"] | ["wp", "v2", "block-directory", "search"] if get => empty_list(),
            ["wp", "v2", "block-types"] | ["wp", "v2", "block-types", _] | ["wp", "v2", "block-types", _, _] if get => empty_list(),
            ["wp", "v2", "block-patterns", "categories"] if get => Ok(Resp::item(self.pattern_categories())),
            ["wp", "v2", "global-styles", "themes", stylesheet] if get && self.can("edit_theme_options") => Ok(Resp::item(self.global_styles_theme(stylesheet))),
            ["wp", "v2", "global-styles", "themes", _, "variations"] if get => Ok(Resp::item(Value::Array(Vec::new()))),
            ["wp", "v2", "global-styles", id] if self.can("edit_theme_options") && id.parse::<u64>().is_ok() => return self.global_styles_user(id, method, args).await.map(Some),
            ["wp", "v2", "templates", "lookup"] if get => self.template_lookup(args.str("slug").unwrap_or_default().as_str()),
            ["wp", "v2", "templates"] | ["wp", "v2", "templates", _] if get => empty_list(),
            ["wp", "v2", "comments"] if get && params.get("context") == Some("edit") => {
                // Notes (`type=note`) are not supported; the editor asks for them once.
                empty_list()
            }
            ["wp", "v2", base @ ("blocks" | "navigation")] | ["wp", "v2", base @ ("blocks" | "navigation"), _] if get => {
                let info = crate::meta::core_post_type(base);
                let kind = crate::posts::Kind::Custom(&info);
                return self.posts_route(kind, segments.get(3).copied(), method, args, params).await.map(Some);
            }
            ["wp", "v2", "statuses"] if get && params.get("context") == Some("edit") && self.can("edit_posts") => Ok(Resp::item(self.statuses_edit())),
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    fn pattern_categories(&self) -> Value {
        let s = |t: &str| self.site.tr(t);
        json!([
            {"name": "banner", "label": s("Banners"), "description": s("Bold sections designed to showcase key content.")},
            {"name": "buttons", "label": s("Buttons"), "description": s("Patterns that contain buttons and call to actions.")},
            {"name": "columns", "label": s("Columns"), "description": s("Multi-column patterns with more complex layouts.")},
            {"name": "text", "label": s("Text"), "description": s("Patterns containing mostly text.")},
            {"name": "query", "label": s("Posts"), "description": s("Display your latest posts in lists, grids or other layouts.")},
            {"name": "featured", "label": s("Featured"), "description": s("A set of high quality curated patterns.")},
            {"name": "call-to-action", "label": s("Call to action"), "description": s("Sections whose purpose is to trigger a specific action.")},
            {"name": "team", "label": s("Team"), "description": s("A variety of designs to display the members of your team.")},
            {"name": "testimonials", "label": s("Testimonials"), "description": s("Share reviews and feedback about your brand/business.")},
            {"name": "services", "label": s("Services"), "description": s("Describe your services in a visually appealing way.")},
            {"name": "contact", "label": s("Contact"), "description": s("Display your contact information.")},
            {"name": "about", "label": s("About"), "description": s("Introduce yourself.")},
            {"name": "portfolio", "label": s("Portfolio"), "description": s("Showcase your latest work.")},
            {"name": "gallery", "label": s("Gallery"), "description": s("Different layouts for displaying images.")},
            {"name": "media", "label": s("Media"), "description": s("Different layouts containing video or audio.")},
            {"name": "posts", "label": s("Posts"), "description": s("Display your latest posts in lists, grids or other layouts.")},
            {"name": "footer", "label": s("Footers"), "description": s("A variety of footer designs displaying information and site navigation.")},
            {"name": "header", "label": s("Headers"), "description": s("A variety of header designs displaying your site title and navigation.")},
        ])
    }

    fn global_styles_theme(&self, stylesheet: &str) -> Value {
        let mut data = Map::new();
        if stylesheet == self.theme.stylesheet {
            data.insert("styles".into(), self.theme.json.styles());
            data.insert("settings".into(), self.theme.json.settings());
        }
        let mut links = Links::new();
        links.add("self", vec![], self.rest_url(&format!("wp/v2/global-styles/themes/{stylesheet}")));
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
        links.add("version-history", vec![("count", json!(0))], self.rest_url(&format!("wp/v2/global-styles/{id}/revisions")));
        links.add("wp:action-publish", vec![], self.rest_url(&format!("wp/v2/global-styles/{id}")));
        let body = json!({
            "id": id,
            "title": {"raw": if post.post_title.is_empty() { "Custom Styles".to_owned() } else { post.post_title.clone() }, "rendered": if post.post_title.is_empty() { "Custom Styles".to_owned() } else { post.post_title.clone() }},
            "settings": stored.get("settings").cloned().filter(|v| !v.is_null()).unwrap_or_else(|| json!({})),
            "styles": stored.get("styles").cloned().filter(|v| !v.is_null()).unwrap_or_else(|| json!({})),
            "_links": links.finish(),
        });
        let mut resp = Resp::item(body);
        resp.allow = Some("GET, POST, PUT, PATCH".to_owned());
        Ok(Ok(resp))
    }

    fn template_lookup(&self, slug: &str) -> Outcome {
        // Candidates in the order of the template hierarchy for a slug like `single-post-hello-world`.
        let mut candidates: Vec<String> = Vec::new();
        let mut parts: Vec<&str> = slug.split('-').collect();
        while !parts.is_empty() {
            candidates.push(parts.join("-"));
            parts.pop();
        }
        candidates.push(if slug.starts_with("page") { "singular".to_owned() } else if slug.starts_with("single") { "singular".to_owned() } else { "index".to_owned() });
        candidates.push("index".to_owned());
        let Some((found, markup)) = self.theme.pick_template(&candidates) else {
            return Err(RestError::new("rest_template_not_found", "No templates exist with that id.", 404));
        };
        let title: String = {
            let mut c = found.chars();
            c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
        };
        Ok(Resp::item(json!({
            "id": format!("{}//{found}", self.theme.stylesheet),
            "theme": self.theme.stylesheet,
            "content": {"raw": markup},
            "slug": found,
            "source": "theme",
            "origin": "theme",
            "type": "wp_template",
            "description": "",
            "title": {"raw": title, "rendered": title},
            "status": "publish",
            "wp_id": null,
            "has_theme_file": true,
            "is_custom": false,
            "author": 0,
            "modified": null,
            "author_text": self.site.options.string("blogname", ""),
            "original_source": "theme",
        })))
    }

    fn statuses_edit(&self) -> Value {
        let s = |t: &str| self.site.tr(t);
        let entry = |slug: &str, name: String, private: bool, protected: bool, public: bool, queryable: bool, show_in_list: bool, floating: bool| {
            json!({"name": name, "private": private, "protected": protected, "public": public, "queryable": queryable, "show_in_list": show_in_list, "slug": slug, "date_floating": floating})
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
