//! `wp/v2/block-types` (`WP_REST_Block_Types_Controller`): the block types WordPress registers, read from its
//! `block.json` files, with the attributes the block supports add (`WP_Block_Supports::register_attributes()`), the
//! variations built on the server and the block styles of the theme.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{Map, Value, json};

use super::Args;
use super::templates::TKind;
use crate::error::RestError;
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

/// One registered block type: its name, its `block.json` (as PHP decodes and encodes it) and whether it renders on the
/// server.
struct Registered {
    name: String,
    json: Map<String, Value>,
    dynamic: bool,
}

/// `json_decode( $json, true )` and back: empty objects become empty arrays.
fn php_json(value: Value) -> Value {
    match value {
        Value::Object(map) if map.is_empty() => Value::Array(Vec::new()),
        Value::Object(map) => Value::Object(map.into_iter().map(|(k, v)| (k, php_json(v))).collect()),
        Value::Array(items) => Value::Array(items.into_iter().map(php_json).collect()),
        other => other,
    }
}

/// PHP's truthiness of a decoded JSON value.
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty() && s != "0",
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

/// `block_has_support()`: `true` or an array at the path, else the default when nothing is set there.
fn has_support(supports: &Value, path: &[&str], default: bool) -> bool {
    let mut current = supports;
    for key in path {
        match current.get(*key) {
            Some(Value::Null) | None => return default,
            Some(next) => current = next,
        }
    }
    matches!(current, Value::Bool(true) | Value::Array(_) | Value::Object(_))
}

/// The attributes a block gets from its supports, in the order WordPress registers the supports.
fn add_support_attributes(attributes: &mut Map<String, Value>, supports: &Value) {
    let add = |attributes: &mut Map<String, Value>, name: &str, schema: Value| {
        if !attributes.contains_key(name) {
            attributes.insert(name.to_owned(), schema);
        }
    };
    let object = || json!({"type": "object"});
    let string = || json!({"type": "string"});
    // align
    if has_support(supports, &["align"], false) {
        add(attributes, "align", json!({"type": "string", "enum": ["left", "center", "right", "wide", "full", ""]}));
    }
    // custom-classname
    if has_support(supports, &["customClassName"], true) {
        add(attributes, "className", string());
    }
    // colors
    let color = supports.get("color");
    let is_array = matches!(color, Some(Value::Object(_) | Value::Array(_)));
    let flag = |key: &str| if is_array { truthy(color.and_then(|c| c.get(key))) } else { false };
    let set = |key: &str| color.and_then(|c| c.get(key)).is_some_and(|v| !v.is_null());
    let all = matches!(color, Some(Value::Bool(true)));
    let text = all || flag("text") || (is_array && !set("text"));
    let background = all || flag("background") || (is_array && !set("background"));
    let gradients = flag("gradients");
    if text || background || gradients || flag("link") || flag("button") || flag("heading") {
        add(attributes, "style", object());
    }
    if background {
        add(attributes, "backgroundColor", string());
    }
    if text {
        add(attributes, "textColor", string());
    }
    if gradients {
        add(attributes, "gradient", string());
    }
    // typography
    if let Some(typography) = supports.get("typography").filter(|t| truthy(Some(t))) {
        let has = |key: &str| truthy(typography.get(key));
        let keys = ["__experimentalFontFamily", "fontSize", "__experimentalFontStyle", "__experimentalFontWeight", "__experimentalLetterSpacing", "lineHeight", "textAlign", "textColumns", "__experimentalTextDecoration", "__experimentalTextTransform", "textIndent", "__experimentalWritingMode"];
        if keys.iter().any(|k| has(k)) {
            add(attributes, "style", object());
        }
        if has("fontSize") {
            add(attributes, "fontSize", string());
        }
        if has("__experimentalFontFamily") {
            add(attributes, "fontFamily", string());
        }
    }
    // border
    if has_support(supports, &["__experimentalBorder"], false) {
        add(attributes, "style", object());
    }
    if matches!(supports.get("__experimentalBorder"), Some(Value::Bool(true))) || has_support(supports, &["__experimentalBorder", "color"], false) {
        add(attributes, "borderColor", string());
    }
    // layout
    if has_support(supports, &["layout"], false) || has_support(supports, &["__experimentalLayout"], false) {
        add(attributes, "layout", object());
    }
    // position, spacing, dimensions, duotone
    for path in [&["position"][..], &["spacing"], &["dimensions"], &["filter", "duotone"]] {
        if has_support(supports, path, false) {
            add(attributes, "style", object());
        }
    }
    // shadow: only rewrites what is there.
    if has_support(supports, &["shadow"], false) {
        if attributes.contains_key("style") {
            attributes.insert("style".into(), object());
        }
        if attributes.contains_key("shadow") {
            attributes.insert("shadow".into(), string());
        }
    }
    // background
    if has_support(supports, &["background"], false) {
        add(attributes, "style", object());
    }
    // aria-label, anchor
    if has_support(supports, &["ariaLabel"], false) {
        add(attributes, "ariaLabel", string());
    }
    if has_support(supports, &["anchor"], false) {
        add(attributes, "anchor", string());
    }
    // custom-css
    if has_support(supports, &["customCSS"], true) {
        add(attributes, "style", object());
    }
}

/// The names in the lines of a `require-*-blocks.php` file.
fn required_blocks(dir: &Path, file: &str, dynamic: bool) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(dir.join(file)) else { return Vec::new() };
    text.lines()
        .filter_map(|line| {
            if dynamic {
                line.split("/blocks/").nth(1).and_then(|l| l.split(".php").next()).map(str::to_owned)
            } else {
                line.trim().strip_prefix('\'').and_then(|l| l.split('\'').next()).map(str::to_owned)
            }
        })
        .collect()
}

/// The core blocks in the order they register: the widget blocks, the dynamic blocks, the static ones, and the
/// legacy `core/post-comments` last.
fn load_registered(dir: &Path) -> Vec<Registered> {
    // A block with a PHP file is dynamic when that file gives it a render callback.
    let read = |slug: &str, php: bool| -> Option<Registered> {
        let text = std::fs::read_to_string(dir.join(slug).join("block.json")).ok()?;
        let Value::Object(json) = php_json(serde_json::from_str(&text).ok()?) else { return None };
        let dynamic = php && std::fs::read_to_string(dir.join(format!("{slug}.php"))).is_ok_and(|code| code.contains("'render_callback'"));
        Some(Registered { name: format!("core/{slug}"), json, dynamic })
    };
    let mut out: Vec<Registered> = ["legacy-widget", "widget-group"].iter().filter_map(|slug| read(slug, true)).collect();
    for slug in required_blocks(dir, "require-dynamic-blocks.php", true) {
        if !out.iter().any(|b| b.name == format!("core/{slug}")) {
            out.extend(read(&slug, true));
        }
    }
    for slug in required_blocks(dir, "require-static-blocks.php", false) {
        out.extend(read(&slug, false));
    }
    out
}

/// The registered blocks of a WordPress installation, read once.
fn registered(dir: &Path) -> Arc<Vec<Registered>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<Vec<Registered>>>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.entry(dir.to_path_buf()).or_insert_with(|| Arc::new(load_registered(dir))).clone()
}

/// `generate_block_asset_handle()` of a core block.
fn asset_handle(name: &str, field: &str, index: usize) -> String {
    let mut handle = name.replacen("core/", "wp-block-", 1);
    if field.starts_with("editor") {
        handle.push_str("-editor");
    }
    if field.starts_with("view") {
        handle.push_str("-view");
    }
    if field.to_ascii_lowercase().ends_with("scriptmodule") {
        handle.push_str("-script-module");
    }
    if index > 0 {
        handle.push_str(&format!("-{}", index + 1));
    }
    handle
}

/// The handles of a script field: names stay, `file:` paths get the generated handle.
fn script_handles(name: &str, json: &Map<String, Value>, field: &str) -> Vec<String> {
    let values: Vec<&str> = match json.get(field) {
        Some(Value::String(s)) => vec![s.as_str()],
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    values.iter().enumerate().map(|(i, v)| if v.starts_with("file:") { asset_handle(name, field, i) } else { (*v).to_owned() }).collect()
}

/// `register_block_style_handle()` of a core block: the first handle is the block's own (also when `block.json` names
/// none), the others stay as they are.
fn style_handles(name: &str, json: &Map<String, Value>, field: &str) -> Vec<String> {
    let mut handles = vec![asset_handle(name, field, 0)];
    if let Some(Value::Array(list)) = json.get(field) {
        handles.extend(list.iter().skip(1).filter_map(Value::as_str).filter(|h| !h.starts_with("file:")).map(str::to_owned));
    }
    handles
}

impl Rest<'_> {
    fn block_text(&self, text: &str, context: &str) -> String {
        self.site.i18n().pgettext("default", context, text)
    }

    /// `block_core_navigation_link_build_variations()`.
    fn navigation_link_variations(&self) -> Vec<Value> {
        let variation = |name: &str, kind: &str, singular: &str| json!({"name": name, "title": rustpress_core::i18n::sprintf(&self.site.tr("%s link"), &[singular]), "description": " ", "attributes": {"type": name, "kind": kind}});
        let mut built_ins = vec![
            variation("post", "post-type", &self.site.trx("Post", "post type singular name")),
            variation("page", "post-type", &self.site.trx("Page", "post type singular name")),
            variation("category", "taxonomy", &self.site.taxonomy_singular_label("category")),
        ];
        let mut tag = variation("post_tag", "taxonomy", &self.site.taxonomy_singular_label("post_tag"));
        tag["name"] = json!("tag");
        tag["attributes"] = json!({"type": "tag", "kind": "taxonomy"});
        built_ins.push(tag);
        if self.theme.adds_theme_support("post-formats") {
            built_ins.push(json!({"name": "post_format", "title": self.site.tr("Post Format Link"), "description": self.site.tr("A link to a post format"), "attributes": {"type": "post_format", "kind": "taxonomy"}}));
        }
        let types = &self.site.types;
        built_ins.extend(types.post_types.iter().filter(|t| t.public && !matches!(t.name.as_str(), "post" | "page" | "attachment")).map(|t| variation(&t.name, "post-type", &t.singular_label())));
        built_ins.extend(types.taxonomies.iter().filter(|t| t.public && !matches!(t.name.as_str(), "category" | "post_tag" | "post_format")).map(|t| variation(&t.name, "taxonomy", &t.singular_label())));
        built_ins
    }

    /// `block_core_post_terms_build_variations()`.
    fn post_terms_variations(&self) -> Vec<Value> {
        let variation = |name: &str, label: String| {
            let mut v = json!({"name": name, "title": label, "description": rustpress_core::i18n::sprintf(&self.site.tr("Display a list of assigned terms from the taxonomy: %s"), &[&label]), "attributes": {"term": name}, "isActive": ["term"], "scope": ["inserter", "transform"]});
            if name == "category" {
                v["isDefault"] = json!(true);
            }
            v
        };
        let mut out = vec![variation("category", self.site.taxonomy_label("category")), variation("post_tag", self.site.taxonomy_label("post_tag"))];
        out.extend(self.site.types.taxonomies.iter().filter(|t| t.queryable && t.show_in_rest && !matches!(t.name.as_str(), "category" | "post_tag" | "post_format")).map(|t| variation(&t.name, t.plural_label())));
        out
    }

    /// `build_template_part_block_variations()`: the areas, then the template parts of the theme.
    async fn template_part_variations(&self) -> Result<Vec<Value>> {
        let areas = super::editor::PART_AREAS;
        let parts = self.block_templates(TKind::Part, None, None, None, &[]).await?;
        let instances: Vec<Value> = parts
            .iter()
            .map(|part| {
                let area = part.area.clone().unwrap_or_else(|| "uncategorized".to_owned());
                let icon = areas.iter().find(|a| a.0 == area).map(|a| a.3);
                let attributes = json!({"slug": part.slug, "theme": part.theme, "area": area});
                json!({
                    "name": format!("instance_{}", rustpress_core::slug::sanitize_title(&part.slug)),
                    "title": part.title,
                    // `$description || '&nbsp;'` is `true`, a string to the schema.
                    "description": "1",
                    "attributes": attributes,
                    "scope": if area == "navigation-overlay" { json!([]) } else { json!(["inserter"]) },
                    "icon": icon,
                    "example": {"attributes": attributes},
                })
            })
            .collect();
        let mut out: Vec<Value> = areas
            .iter()
            .filter(|a| a.0 != "uncategorized" && a.0 != "navigation-overlay")
            .map(|(area, label, description, icon, _)| {
                let used = instances.iter().any(|v| v["attributes"]["area"] == *area);
                json!({"name": format!("area_{area}"), "title": self.site.tr(label), "description": self.site.tr(description), "attributes": {"area": area}, "scope": if used { json!([]) } else { json!(["inserter"]) }, "icon": icon})
            })
            .collect();
        out.extend(instances);
        Ok(out)
    }

    /// The styles of a block: those of its `block.json`, then those the theme registers.
    fn block_styles(&self, name: &str, json: &Map<String, Value>, partials: bool) -> Vec<Value> {
        let mut styles: Vec<Value> = json.get("styles").and_then(Value::as_array).cloned().unwrap_or_default();
        for style in &mut styles {
            if let Some(label) = style.get("label").and_then(Value::as_str).map(|l| self.block_text(l, "block style label")) {
                style["label"] = json!(label);
            }
        }
        if let Some((_, registered)) = self.theme.registered_block_styles_with(partials).into_iter().find(|(block, _)| block == name) {
            for (_, style, label, is_default) in registered {
                let mut item = Map::new();
                item.insert("name".into(), json!(style));
                item.insert("label".into(), json!(label));
                if is_default {
                    item.insert("is_default".into(), json!(true));
                }
                if let Some(css) = self.theme.registered_block_style_inline(name, &style) {
                    item.insert("inline_style".into(), json!(css));
                }
                styles.push(Value::Object(item));
            }
        }
        styles
    }

    /// `WP_REST_Block_Types_Controller::prepare_item_for_response()`.
    /// `partials`: the theme's style variations are registered already (see [`Rest::block_types_route`]).
    async fn prepare_block_type(&self, block: &Registered, partials: bool) -> Result<Value> {
        let json = &block.json;
        let name = block.name.as_str();
        let get = |key: &str| json.get(key).cloned().unwrap_or(Value::Null);
        let mut attributes = json.get("attributes").and_then(Value::as_object).cloned().unwrap_or_default();
        for global in ["lock", "metadata"] {
            if !attributes.contains_key(global) {
                attributes.insert(global.into(), json!({"type": "object"}));
            }
        }
        let supports = get("supports");
        add_support_attributes(&mut attributes, &supports);
        let keywords: Vec<String> = json.get("keywords").and_then(Value::as_array).map(|k| k.iter().filter_map(Value::as_str).map(|k| self.block_text(k, "block keyword")).collect()).unwrap_or_default();
        let variations = match name {
            "core/navigation-link" => self.navigation_link_variations(),
            "core/post-terms" => self.post_terms_variations(),
            "core/template-part" => self.template_part_variations().await?,
            _ => Vec::new(),
        };
        let handles = |field: &str| script_handles(name, json, field);
        let (editor_scripts, scripts, view_scripts) = (handles("editorScript"), handles("script"), handles("viewScript"));
        let modules: Vec<String> = match json.get("viewScriptModule") {
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
            _ => Vec::new(),
        };
        let editor_styles = style_handles(name, json, "editorStyle");
        let styles = style_handles(name, json, "style");
        let view_styles = handles("viewStyle");
        let first = |list: &[String]| list.first().map_or(Value::Null, |h| json!(h));
        let mut data = Map::new();
        data.insert("attributes".into(), Value::Object(attributes));
        data.insert("is_dynamic".into(), json!(block.dynamic));
        data.insert("api_version".into(), json.get("apiVersion").cloned().unwrap_or(json!(1)));
        data.insert("name".into(), json!(name));
        data.insert("title".into(), json!(json.get("title").and_then(Value::as_str).map(|t| self.block_text(t, "block title")).unwrap_or_default()));
        data.insert("description".into(), json!(json.get("description").and_then(Value::as_str).map(|t| self.block_text(t, "block description")).unwrap_or_default()));
        data.insert("icon".into(), get("icon"));
        data.insert("category".into(), get("category"));
        data.insert("keywords".into(), json!(keywords));
        data.insert("parent".into(), get("parent"));
        data.insert("ancestor".into(), get("ancestor"));
        data.insert("allowed_blocks".into(), get("allowedBlocks"));
        data.insert("provides_context".into(), json.get("providesContext").cloned().unwrap_or(json!([])));
        data.insert("uses_context".into(), json.get("usesContext").cloned().unwrap_or(json!([])));
        data.insert("selectors".into(), json.get("selectors").cloned().unwrap_or(json!([])));
        data.insert("supports".into(), json.get("supports").cloned().unwrap_or(json!([])));
        data.insert("styles".into(), json!(self.block_styles(name, json, partials || name == "core/template-part")));
        data.insert("textdomain".into(), Value::Null);
        data.insert("example".into(), get("example"));
        data.insert("editor_script_handles".into(), json!(editor_scripts));
        data.insert("script_handles".into(), json!(scripts));
        data.insert("view_script_handles".into(), json!(view_scripts));
        data.insert("view_script_module_ids".into(), json!(modules));
        data.insert("editor_style_handles".into(), json!(editor_styles));
        data.insert("style_handles".into(), json!(styles));
        data.insert("view_style_handles".into(), json!(view_styles));
        data.insert("variations".into(), json!(variations));
        data.insert("block_hooks".into(), json.get("blockHooks").cloned().unwrap_or(json!([])));
        data.insert("editor_script".into(), first(&editor_scripts));
        data.insert("script".into(), first(&scripts));
        data.insert("view_script".into(), first(&view_scripts));
        data.insert("editor_style".into(), first(&editor_styles));
        data.insert("style".into(), first(&styles));
        data.insert("_links".into(), self.block_type_links(name, block.dynamic));
        Ok(Value::Object(data))
    }

    fn block_type_links(&self, name: &str, dynamic: bool) -> Value {
        let namespace = name.split('/').next().unwrap_or_default();
        let mut links = Links::new();
        links.add("collection", Vec::new(), self.rest_url("wp/v2/block-types"));
        links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/block-types/{name}")), "targetHints": {"allow": ["GET"]}}));
        links.add("up", Vec::new(), self.rest_url(&format!("wp/v2/block-types/{namespace}")));
        if dynamic {
            links.add("wp:render-block", Vec::new(), format!("{}?context=edit", self.rest_url(&format!("wp/v2/block-renderer/{name}"))));
        }
        links.finish()
    }

    /// `register_legacy_post_comments_block()`: the block `core/post-comments` stands for.
    fn legacy_post_comments(&self) -> Value {
        let mut attributes = json!({"textAlign": {"type": "string"}, "lock": {"type": "object"}, "metadata": {"type": "object"}}).as_object().cloned().unwrap_or_default();
        let supports = json!({
            "html": false,
            "align": ["wide", "full"],
            "typography": {"fontSize": true, "lineHeight": true, "__experimentalFontStyle": true, "__experimentalFontWeight": true, "__experimentalLetterSpacing": true, "__experimentalTextTransform": true, "__experimentalDefaultControls": {"fontSize": true}},
            "color": {"gradients": true, "link": true, "__experimentalDefaultControls": {"background": true, "text": true}},
            "inserter": false
        });
        add_support_attributes(&mut attributes, &supports);
        let styles = ["wp-block-post-comments", "wp-block-buttons", "wp-block-button"];
        json!({
            "attributes": attributes, "is_dynamic": true, "api_version": 1, "name": "core/post-comments", "title": "", "description": "", "icon": null,
            "category": "theme", "keywords": [], "parent": null, "ancestor": null, "allowed_blocks": null, "provides_context": [], "uses_context": ["postId", "postType"],
            "selectors": [], "supports": supports, "styles": [], "textdomain": null, "example": null, "editor_script_handles": [], "script_handles": [],
            "view_script_handles": [], "view_script_module_ids": [], "editor_style_handles": [], "style_handles": styles, "view_style_handles": [],
            "variations": [], "block_hooks": [], "editor_script": null, "script": null, "view_script": null, "editor_style": null, "style": styles[0],
            "_links": self.block_type_links("core/post-comments", true),
        })
    }

    /// The routes below `wp/v2/block-types`.
    pub(crate) async fn block_types_route(&self, segments: &[&str], args: &Args) -> Result<Outcome> {
        // `check_read_permission()`: editing posts of any type shown in the REST API.
        let can_read = self.can("edit_posts") || self.can("edit_pages") || self.can("edit_theme_options");
        if !can_read {
            return Ok(Err(self.forbidden("rest_block_type_cannot_view", "Sorry, you are not allowed to manage block types.")));
        }
        let blocks = registered(self.registry.blocks_dir());
        let (namespace, wanted) = match segments {
            ["wp", "v2", "block-types"] => (args.str("namespace"), None),
            ["wp", "v2", "block-types", namespace] => (Some((*namespace).to_owned()), None),
            [.., namespace, name] => (None, Some(format!("{namespace}/{name}"))),
            _ => (None, None),
        };
        if let Some(wanted) = wanted {
            let item = if wanted == "core/post-comments" {
                Some(self.legacy_post_comments())
            } else {
                match blocks.iter().find(|b| b.name == wanted) {
                    Some(block) => Some(self.prepare_block_type(block, false).await?),
                    None => None,
                }
            };
            return Ok(match item {
                Some(item) => {
                    let mut resp = Resp::item(item);
                    resp.allow = Some("GET".to_owned());
                    Ok(resp)
                }
                None => Err(RestError::new("rest_block_type_invalid", "Invalid block type.", 404)),
            });
        }
        let mut items = Vec::new();
        // The variations of `core/template-part` read the templates, which reads the theme's `theme.json` data and
        // registers its style variations; the blocks after it have them.
        let mut partials = false;
        for block in blocks.iter() {
            if namespace.as_deref().is_none_or(|ns| block.name.split('/').next() == Some(ns)) {
                items.push(self.prepare_block_type(block, partials).await?);
                partials |= block.name == "core/template-part";
            }
        }
        if namespace.as_deref().is_none_or(|ns| ns == "core") {
            items.push(self.legacy_post_comments());
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.allow = Some("GET".to_owned());
        Ok(Ok(resp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_add_attributes_in_registration_order() {
        let mut attributes = Map::new();
        let supports = json!({"align": ["wide"], "color": {"gradients": true, "text": false}, "typography": {"fontSize": true}, "anchor": true});
        add_support_attributes(&mut attributes, &supports);
        let keys: Vec<&str> = attributes.keys().map(String::as_str).collect();
        assert_eq!(keys, ["align", "className", "style", "backgroundColor", "gradient", "fontSize", "anchor"]);
    }

    #[test]
    fn custom_css_and_class_names_can_be_switched_off() {
        let mut attributes = Map::new();
        add_support_attributes(&mut attributes, &json!({"customClassName": false, "customCSS": false, "html": false}));
        assert!(attributes.is_empty());
    }

    #[test]
    fn empty_objects_become_arrays() {
        assert_eq!(php_json(json!({"a": {}, "b": [{}], "c": {"d": 1}})), json!({"a": [], "b": [[]], "c": {"d": 1}}));
    }

    #[test]
    fn asset_handles_follow_the_field() {
        assert_eq!(asset_handle("core/navigation", "viewScriptModule", 0), "wp-block-navigation-view-script-module");
        assert_eq!(asset_handle("core/paragraph", "editorStyle", 1), "wp-block-paragraph-editor-2");
    }
}
