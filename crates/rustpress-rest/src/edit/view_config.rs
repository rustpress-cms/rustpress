//! `wp/v2/view-config`: how the DataViews of the site editor list an entity (`wp_get_entity_view_config()`).

use serde_json::{Value, json};

use super::Args;
use super::templates::TKind;
use crate::error::RestError;
use crate::{Outcome, Resp, Rest, Result};

/// `cast_empty_objects()`: an empty object of the configuration is printed as `{}`, not as an empty PHP array.
fn cast_empty_objects(value: &mut Value) {
    match value {
        Value::Object(map) if map.is_empty() => *value = rustpress_blocks::php_json::empty_object(),
        Value::Object(map) => map.values_mut().for_each(cast_empty_objects),
        Value::Array(items) => items.iter_mut().for_each(cast_empty_objects),
        _ => {}
    }
}

fn locked_filter(field: &str, operator: &str, value: &str) -> Value {
    json!({"filters": [{"field": field, "operator": operator, "value": value, "isLocked": true}]})
}

impl Rest<'_> {
    /// `WP_REST_View_Config_Controller::get_required_capability()`.
    fn view_config_capability(&self, kind: &str, name: &str) -> Option<String> {
        match kind {
            "postType" => self.type_entries().iter().any(|e| e.name == name).then(|| match name {
                "page" => "edit_pages".to_owned(),
                "wp_template" | "wp_template_part" | "wp_navigation" | "wp_global_styles" | "wp_font_family" | "wp_font_face" => "edit_theme_options".to_owned(),
                _ => "edit_posts".to_owned(),
            }),
            "taxonomy" => self.taxonomy_entries().iter().any(|e| e.name == name).then(|| "manage_categories".to_owned()),
            "root" => Some("manage_options".to_owned()),
            _ => Some("edit_posts".to_owned()),
        }
    }

    fn default_posttype_form(&self) -> Value {
        let s = |text: &str| self.site.tr(text);
        json!({
            "layout": {"type": "panel"},
            "fields": [
                {"id": "featured_media", "layout": {"type": "regular", "labelPosition": "none"}},
                {"id": "post-content-info", "layout": {"type": "regular", "labelPosition": "none"}},
                {"id": "excerpt", "layout": {"type": "panel", "labelPosition": "top"}},
                {"id": "status", "label": s("Status"), "children": [{"id": "status", "layout": {"type": "regular", "labelPosition": "none"}}, "scheduled_date", "password", "sticky"]},
                "date",
                "slug",
                "author",
                "template",
                {"id": "discussion", "label": s("Discussion"), "children": [{"id": "comment_status", "layout": {"type": "regular", "labelPosition": "none"}}, "ping_status"]},
                "parent",
                "format",
                "revisions"
            ]
        })
    }

    /// The template parts areas, in the order the view list shows them.
    fn view_config_areas(&self) -> Vec<(&'static str, String)> {
        let s = |text: &str, context: &str| if context.is_empty() { self.site.tr(text) } else { self.site.trx(text, context) };
        vec![("header", s("Header", "template part area")), ("footer", s("Footer", "template part area")), ("navigation-overlay", s("Navigation Overlay", "template part area")), ("uncategorized", s("General", "template part area"))]
    }

    async fn template_view_list(&self) -> Result<Vec<Value>> {
        let templates = self.block_templates(TKind::Template, None, None, None, &[]).await?;
        let blog_name = self.site.options.string("blogname", "");
        let mut seen: Vec<String> = Vec::new();
        let (mut registered, mut users) = (Vec::new(), Vec::new());
        for template in &templates {
            let mut source = "user";
            if template.has_theme_file && (template.origin.as_deref() == Some("theme") || (template.origin.as_deref().is_none_or(str::is_empty) && matches!(template.source.as_str(), "theme" | "custom"))) {
                source = "theme";
            } else if template.origin.as_deref() == Some("plugin") {
                source = "plugin";
            } else if !template.has_theme_file && template.source == "custom" && template.author == 0 {
                source = "site";
            }
            let author = match source {
                "theme" => self.theme_name(&template.theme).unwrap_or_else(|| template.theme.clone()),
                "plugin" => template.theme.clone(),
                "site" => blog_name.clone(),
                _ => match self.site.db.user(template.author).await? {
                    Some(user) => user.display_name,
                    None => self.site.tr("Unknown author"),
                },
            };
            if author.is_empty() || seen.contains(&author) {
                continue;
            }
            seen.push(author.clone());
            let entry = json!({"title": author, "slug": author, "view": locked_filter("author", "is", &author)});
            if source == "user" { users.push(entry) } else { registered.push(entry) }
        }
        let mut list = vec![json!({"title": self.site.tr("All templates"), "slug": "all"})];
        list.extend(registered);
        list.extend(users);
        Ok(list)
    }

    /// The `Name` of a theme from its `style.css`.
    fn theme_name(&self, stylesheet: &str) -> Option<String> {
        let dir = if stylesheet == self.theme.stylesheet { self.theme.stylesheet_dir().to_path_buf() } else { self.theme.wp_root.join("wp-content").join("themes").join(stylesheet) };
        let css = std::fs::read_to_string(dir.join("style.css")).ok()?;
        css.lines().take(40).find_map(|line| {
            let (key, value) = line.trim().trim_start_matches('*').trim().split_once(':')?;
            key.trim().eq_ignore_ascii_case("Theme Name").then(|| value.trim().to_owned())
        })
    }

    /// `GET wp/v2/view-config?kind=…&name=…`.
    pub(super) async fn view_config(&self, args: &Args) -> Result<Outcome> {
        let (Some(kind), Some(name)) = (args.str("kind"), args.str("name")) else {
            let missing: Vec<&str> = ["kind", "name"].into_iter().filter(|k| args.str(k).is_none()).collect();
            return Ok(Err(RestError::new("rest_missing_callback_param", &format!("Missing parameter(s): {}", missing.join(", ")), 400)));
        };
        let Some(capability) = self.view_config_capability(&kind, &name) else {
            return Ok(Err(RestError::new("rest_view_config_invalid_entity", &self.site.tr("Invalid entity kind or name."), 404)));
        };
        if !self.can(&capability) {
            return Ok(Err(self.forbidden("rest_cannot_read", &self.site.tr("Sorry, you are not allowed to read view config."))));
        }
        let all_items = if kind == "postType" {
            self.type_entries().into_iter().find(|e| e.name == name).and_then(|e| self.prepare_type_edit(&e).pointer("/labels/all_items").and_then(Value::as_str).map(str::to_owned)).unwrap_or_else(|| self.site.tr("All items"))
        } else {
            self.site.tr("All items")
        };
        let mut default_view = json!({"type": "table", "filters": [], "sort": {"field": "title", "direction": "asc"}, "perPage": 20, "fields": ["author", "status"], "titleField": "title"});
        let mut default_layouts = json!({"table": {}, "grid": {}, "list": {}});
        let mut view_list = vec![json!({"title": all_items, "slug": "all"})];
        let mut form = if kind == "postType" { self.default_posttype_form() } else { json!({}) };
        let s = |text: &str| self.site.tr(text);
        if kind == "postType" {
            match name.as_str() {
                "page" => {
                    let table_layout = json!({"styles": {"author": {"align": "start"}}});
                    default_layouts = json!({"table": {"layout": table_layout}, "grid": {}, "list": {}});
                    default_view = json!({"type": "list", "filters": [], "perPage": 20, "sort": {"field": "title", "direction": "asc"}, "showLevels": true, "titleField": "title", "mediaField": "featured_media", "fields": ["author", "status"]});
                    for (title, slug, status) in [("Published", "published", "publish"), ("Scheduled", "future", "future"), ("Drafts", "drafts", "draft"), ("Pending", "pending", "pending"), ("Private", "private", "private")] {
                        view_list.push(json!({"title": s(title), "slug": slug, "view": locked_filter("status", "isAny", status)}));
                    }
                    view_list.push(json!({"title": s("Trash"), "slug": "trash", "view": {"type": "table", "layout": table_layout, "filters": [{"field": "status", "operator": "isAny", "value": "trash", "isLocked": true}]}}));
                }
                "wp_block" => {
                    default_layouts = json!({"table": {"layout": {"styles": {"author": {"width": "1%"}}}}, "grid": {"layout": {"badgeFields": ["sync-status"]}}});
                    default_view = json!({"type": "grid", "perPage": 20, "titleField": "title", "mediaField": "preview", "fields": ["sync-status"], "filters": [], "layout": {"badgeFields": ["sync-status"]}});
                    view_list = vec![json!({"title": s("All patterns"), "slug": "all-patterns"}), json!({"title": s("My patterns"), "slug": "my-patterns"})];
                    let mut categories: Vec<(String, String)> = self.theme.pattern_categories().into_iter().map(|c| (c.name, c.label)).collect();
                    if !categories.iter().any(|(n, _)| n == "uncategorized") {
                        categories.push(("uncategorized".into(), s("Uncategorized")));
                    }
                    for term in self.site.db.terms("wp_pattern_category").await? {
                        match categories.iter_mut().find(|(n, _)| *n == term.slug) {
                            Some(existing) => existing.1 = term.name,
                            None => categories.push((term.slug, term.name)),
                        }
                    }
                    categories.sort_by_key(|(_, label)| label.to_lowercase());
                    for (slug, label) in categories {
                        view_list.push(json!({"title": label, "slug": slug}));
                    }
                    form = json!({"layout": {"type": "panel"}, "fields": [{"id": "excerpt", "layout": {"type": "panel", "labelPosition": "top"}}, {"id": "post-content-info", "layout": {"type": "regular", "labelPosition": "none"}}, "sync-status", "revisions"]});
                }
                "wp_template_part" => {
                    default_layouts = json!({"table": {"layout": {"styles": {"author": {"width": "1%"}}}}, "grid": {"layout": {}}});
                    default_view = json!({"type": "grid", "perPage": 20, "titleField": "title", "mediaField": "preview", "fields": ["author"], "filters": [], "layout": {}});
                    view_list = vec![json!({"title": s("All template parts"), "slug": "all-parts"})];
                    for (area, label) in self.view_config_areas() {
                        view_list.push(json!({"title": label, "slug": area, "view": locked_filter("area", "is", area)}));
                    }
                    form = json!({"layout": {"type": "panel"}, "fields": [{"id": "last_edited_date", "layout": {"type": "panel", "labelPosition": "none"}}, "revisions"]});
                }
                "wp_template" => {
                    default_view = json!({"type": "grid", "perPage": 20, "sort": {"field": "title", "direction": "asc"}, "titleField": "title", "descriptionField": "description", "mediaField": "preview", "fields": ["author", "active", "slug", "theme"], "filters": [], "showMedia": true});
                    default_layouts = json!({"table": {"showMedia": false}, "grid": {"showMedia": true}, "list": {"showMedia": false}});
                    view_list = self.template_view_list().await?;
                    form = json!({"layout": {"type": "panel"}, "fields": [{"id": "description", "layout": {"type": "panel", "labelPosition": "top"}}, {"id": "description_readonly", "layout": {"type": "regular", "labelPosition": "none"}}, {"id": "last_edited_date", "layout": {"type": "panel", "labelPosition": "none"}}, "revisions", "posts_page_title", "posts_per_page", "default_comment_status"]});
                }
                _ => {}
            }
        }
        let mut config = json!({"kind": kind, "name": name, "version": 1, "default_view": default_view, "default_layouts": default_layouts, "view_list": view_list, "form": form});
        cast_empty_objects(&mut config);
        Ok(Ok(Resp::item(config)))
    }
}
