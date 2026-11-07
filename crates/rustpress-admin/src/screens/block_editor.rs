//! The block editor of `post.php` and `post-new.php` (`edit-form-blocks.php`): the page around the JavaScript editor,
//! the REST responses it starts with and its settings.

use rustpress_core::OrLog;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_db::Post;
use rustpress_rest::edit::{Access, Body, Request};
use serde_json::{Map, Value, json};

use super::posts::TypeInfo;
use super::{AdminResponse, Ctx, render};
use crate::chrome::{Assets, Screen};

const STATIC: &str = include_str!("../data/editor_static.json");

/// `wp_json_encode( $value, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )`: safe inside an inline script.
pub(super) fn inline_json(value: &Value) -> String {
    rustpress_blocks::php_json::encode_with(value, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E")
}

/// The page an editor runs on: the order of the block data and the encoding of the preloads differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorPage {
    /// `edit-form-blocks.php`.
    Post,
    /// `site-editor.php`.
    Site,
    /// A page of `@wordpress/boot`.
    Boot,
}

/// `get_all_registered_block_bindings_sources()` as the editors list them: the plugins' sources, those of WordPress,
/// then the theme's.
fn bindings_sources(ctx: &Ctx<'_>) -> Value {
    let site = ctx.site();
    let mut sources: Vec<Value> = site.plugins.block_bindings_sources(site).into_iter().map(|(name, label, uses)| json!({"name": name, "label": label, "usesContext": uses})).collect();
    sources.extend([("core/pattern-overrides", "Pattern Overrides", &["pattern/overrides"][..]), ("core/post-data", "Post Data", &["postId", "postType"]), ("core/post-meta", "Post Meta", &["postId", "postType"]), ("core/term-data", "Term Data", &["termId", "taxonomy"])]
        .into_iter()
        .map(|(name, label, uses)| json!({"name": name, "label": site.trx(label, "block bindings source"), "usesContext": uses})));
    let domain = site.settings.template.clone();
    for source in ctx.env.theme.block_bindings_sources() {
        let label = match &source.label_context {
            Some(context) => site.i18n().pgettext(&domain, context, &source.label),
            None => site.i18n().gettext(&domain, &source.label),
        };
        sources.push(json!({"name": source.name, "label": label, "usesContext": source.uses_context}));
    }
    Value::Array(sources)
}

fn statics() -> Value {
    serde_json::from_str(STATIC).unwrap_or(Value::Null)
}

/// `use_block_editor_for_post_type()`: the post type shows in the REST API and has an editor, and no plugin says
/// otherwise. `rp-classic` stands for the must-use plugin `dev/mu-plugins/rustpress-classic-editor.php` of the
/// reference, which compares the classic form.
pub fn uses_block_editor_for_post_type(ctx: &Ctx<'_>, post_type: &str) -> bool {
    if ctx.request.get("rp-classic").is_some() {
        return false;
    }
    let supported = match post_type {
        "post" | "page" => true,
        other => ctx.site().types.post_type(other).is_some_and(|t| t.show_in_rest && t.supports("editor")),
    };
    supported && ctx.site().plugins.use_block_editor_for_post_type(ctx.site(), post_type, true)
}

/// `use_block_editor_for_post()`.
pub async fn uses_block_editor(ctx: &Ctx<'_>, post: &Post) -> bool {
    if post.post_status == "trash" {
        return false;
    }
    let value = uses_block_editor_for_post_type(ctx, &post.post_type);
    if ctx.request.get("rp-classic").is_some() || ctx.site().plugins.is_empty() {
        return value;
    }
    let parts = ctx.plugin_parts();
    let mut plugin_ctx = ctx.plugin_context(&parts, "post.php");
    plugin_ctx.edited_post = Some(post);
    ctx.site().plugins.use_block_editor_for_post(&plugin_ctx, post, value).await.unwrap_or_else(|e| {
        eprintln!("use_block_editor_for_post: {e}");
        value
    })
}

/// The answer of one REST request, in the shape of the preloading middleware.
async fn preload_one(ctx: &Ctx<'_>, method: &str, path: &str, earlier: &[String]) -> Option<Value> {
    let env = ctx.env;
    let writer = env.writer?;
    let (route, query) = path.split_once('?').map_or((path, ""), |(r, q)| (r, q));
    let rest = rustpress_rest::Rest::new(ctx.site(), env.theme, env.registry).with_host(ctx.request.host.as_deref().unwrap_or_default()).in_admin().after_routes(earlier.to_vec());
    let (auth, user_id, token, now) = (env.auth.clone(), ctx.user.id, ctx.token.clone(), env.now);
    let nonce: std::sync::Arc<dyn Fn(&str) -> String + Send + Sync> = std::sync::Arc::new(move |action: &str| auth.create_nonce(action, user_id, &token, now).unwrap_or_default());
    let access = Access { user: ctx.user.clone(), app_password: None, writer: Some(writer), now: env.now, uploads: super::ajax_media::uploads_dir(env), nonce: Some(nonce), roles: Some(env.roles), salts: Some(env.auth.salts()), comment_notify: None };
    let reply = rest.handle_as(route, query, Some(access), Request { method: method.to_owned(), body: Body::None, basic: None, remote_addr: ctx.request.remote_addr.clone(), user_agent: ctx.request.user_agent.clone(), content_type: None, referer: None }).await.ok()?;
    if reply.status >= 400 {
        return None;
    }
    let mut body: Value = serde_json::from_str(&reply.body).unwrap_or(Value::Null);
    keep_empty_objects(&mut body);
    // The headers of the response in the order the controllers set them (the counts are numbers), `Allow` last.
    let header = |name: &str| reply.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
    let mut headers = Map::new();
    for name in ["X-WP-Total", "X-WP-TotalPages"] {
        if let Some(value) = header(name) {
            headers.insert(name.into(), value.parse::<u64>().map_or_else(|_| Value::from(value.clone()), Value::from));
        }
    }
    if let Some(link) = header("Link").filter(|l| !l.contains("api.w.org")) {
        headers.insert("Link".into(), Value::from(link));
    }
    if let Some(allow) = header("Allow") {
        headers.insert("Allow".into(), Value::from(allow));
    }
    Some(json!({"body": body, "headers": Value::Object(headers)}))
}

/// The empty objects of a REST answer stay objects (`new stdClass()`) when it is encoded again.
fn keep_empty_objects(value: &mut Value) {
    match value {
        Value::Object(map) if map.is_empty() => *value = rustpress_blocks::php_json::empty_object(),
        Value::Object(map) => map.values_mut().for_each(keep_empty_objects),
        Value::Array(items) => items.iter_mut().for_each(keep_empty_objects),
        _ => {}
    }
}

async fn preload(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo, global_styles_id: Option<u64>) -> Value {
    let site = ctx.site();
    let base = match post.post_type.as_str() {
        "post" => "posts".to_owned(),
        "page" => "pages".to_owned(),
        other => site.types.post_type(other).map_or_else(|| other.to_owned(), |t| t.rest_base().to_owned()),
    };
    let stylesheet = &site.settings.stylesheet;
    let mut paths: Vec<(String, String)> = vec![
        ("GET".into(), "/wp/v2/types?context=view".into()),
        ("GET".into(), "/wp/v2/taxonomies?context=view".into()),
        ("GET".into(), format!("/wp/v2/{base}/{}?context=edit", post.id)),
        ("GET".into(), format!("/wp/v2/types/{}?context=edit", info.name)),
        ("GET".into(), "/wp/v2/users/me".into()),
        ("OPTIONS".into(), "/wp/v2/media".into()),
        ("OPTIONS".into(), "/wp/v2/pages".into()),
        ("OPTIONS".into(), "/wp/v2/blocks".into()),
        ("OPTIONS".into(), "/wp/v2/templates".into()),
        ("GET".into(), format!("/wp/v2/{base}/{}/autosaves?context=edit", post.id)),
        ("GET".into(), "/wp/v2/settings".into()),
        ("OPTIONS".into(), "/wp/v2/settings".into()),
        ("GET".into(), format!("/wp/v2/global-styles/themes/{stylesheet}?context=view")),
        ("GET".into(), format!("/wp/v2/global-styles/themes/{stylesheet}/variations?context=view")),
        ("GET".into(), "/wp/v2/themes?context=edit&status=active".into()),
    ];
    if let Some(id) = global_styles_id {
        paths.push(("OPTIONS".into(), format!("/wp/v2/global-styles/{id}")));
        // The context the client asks with depends on the user's rights.
        let context = if ctx.can("edit_theme_options") { "edit" } else { "view" };
        paths.push(("GET".into(), format!("/wp/v2/global-styles/{id}?context={context}")));
    }
    paths.push(("GET".into(), "/wp/v2/block-patterns/categories".into()));
    paths.push(("GET".into(), "/?_fields=description,gmt_offset,home,image_max_bit_depth,image_sizes,image_size_threshold,image_strip_meta,name,site_icon,site_icon_url,site_logo,timezone_string,url,page_for_posts,page_on_front,show_on_front".into()));
    let mut lookup = if post.post_type == "page" { "page".to_owned() } else { format!("single-{}", post.post_type) };
    if !post.post_name.is_empty() {
        lookup.push('-');
        lookup.push_str(&post.post_name);
    }
    paths.push(("GET".into(), format!("/wp/v2/templates/lookup?slug={lookup}")));
    paths.push(("GET".into(), "/wp/v2/templates/lookup?slug=front-page".into()));
    paths.push(("GET".into(), "/wp/v2/taxonomies?context=edit".into()));
    paths.push(("OPTIONS".into(), format!("/wp/v2/{base}")));
    if site.post_type_supports(&post.post_type, "author") && post.post_author > 0 {
        paths.push(("GET".into(), format!("/wp/v2/users/{}?context=view&_fields=id,name", post.post_author)));
    }
    preload_paths(ctx, paths).await
}

/// `block_editor_rest_api_preload()`: the answers of the requests in the order they are asked; the `OPTIONS`
/// answers are collected under one key that sits where the first of them was.
pub(super) async fn preload_paths(ctx: &Ctx<'_>, paths: Vec<(String, String)>) -> Value {
    let mut ordered = Map::new();
    let mut options = Map::new();
    // One PHP process answers them all: what a plugin registers for one request stays for the next.
    let mut earlier: Vec<String> = Vec::new();
    for (method, path) in paths {
        let answer = preload_one(ctx, &method, &path, &earlier).await;
        earlier.push(path.split_once('?').map_or(path.as_str(), |(r, _)| r).to_owned());
        let Some(value) = answer else { continue };
        if method == "OPTIONS" {
            if !ordered.contains_key("OPTIONS") {
                ordered.insert("OPTIONS".into(), Value::Null);
            }
            options.insert(path, value);
        } else {
            ordered.insert(path, value);
        }
    }
    if ordered.contains_key("OPTIONS") {
        ordered.insert("OPTIONS".into(), Value::Object(options));
    }
    Value::Object(ordered)
}

/// The user's `wp_global_styles` post of the active theme.
pub(super) async fn global_styles_id(ctx: &Ctx<'_>) -> Option<u64> {
    let site = ctx.site();
    let query = rustpress_db::PostQuery { post_types: vec!["wp_global_styles".into()], statuses: vec!["publish".into(), "draft".into()], names: vec![format!("wp-global-styles-{}", rustpress_core::slug::sanitize_title(&site.settings.stylesheet))], limit: Some(1), ..Default::default() };
    site.db.posts(&query).await.ok()?.first().map(|p| p.id)
}

/// The SVG filters and the `--wp--preset--duotone--*` variables of every duotone preset
/// (`wp_get_global_styles_svg_filters()` and the duotone part of the presets stylesheet).
fn duotone_entries(features: &Value) -> Vec<Value> {
    use rustpress_blocks::duotone::filter_svg;
    use rustpress_blocks::style_engine::to_kebab_case;
    let mut presets: Vec<(String, String, Option<Vec<String>>, String)> = Vec::new();
    if let Some(Value::Object(origins)) = features.pointer("/color/duotone") {
        for origin in ["default", "theme", "custom"] {
            for preset in origins.get(origin).and_then(Value::as_array).into_iter().flatten() {
                let Some(slug) = preset.get("slug").and_then(Value::as_str) else { continue };
                let id = format!("wp-duotone-{}", to_kebab_case(slug));
                let (colors, css) = match preset.get("colors") {
                    Some(Value::Array(a)) => (Some(a.iter().filter_map(Value::as_str).map(str::to_owned).collect()), format!("url(#{id})")),
                    Some(Value::String(s)) => (None, s.clone()),
                    _ => continue,
                };
                match presets.iter_mut().find(|p| p.0 == id) {
                    Some(slot) => *slot = (id, slug.to_owned(), colors, css),
                    None => presets.push((id, slug.to_owned(), colors, css)),
                }
            }
        }
    }
    if presets.is_empty() {
        return Vec::new();
    }
    let svgs: String = presets.iter().filter_map(|(id, _, colors, _)| colors.as_ref().map(|c| filter_svg(id, c))).collect();
    let mut css = String::from(":root{");
    for (_, slug, _, value) in &presets {
        css.push_str(&format!("--wp--preset--duotone--{slug}:{value};"));
    }
    css.push('}');
    vec![json!({"assets": svgs, "__unstableType": "svgs", "isGlobalStyles": false}), json!({"css": css, "__unstableType": "presets", "isGlobalStyles": false})]
}

/// The settings that belong to a post (`edit-form-blocks.php`).
async fn post_settings(ctx: &Ctx<'_>, post: &Post, settings: &mut Map<String, Value>) {
    let env = ctx.env;
    let site = ctx.site();
    let theme = env.theme;
    // The page templates of the theme (`customTemplates` of theme.json that apply to this type).
    let mut templates = Map::new();
    for template in theme.json.custom_templates() {
        let types: Vec<&str> = template.get("postTypes").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_else(|| vec!["page"]);
        if !types.contains(&post.post_type.as_str()) {
            continue;
        }
        let (Some(name), Some(title)) = (template.get("name").and_then(Value::as_str), template.get("title").and_then(Value::as_str)) else { continue };
        templates.insert(name.to_owned(), json!(ctx.tr(title)));
    }
    if templates.is_empty() {
        settings.insert("availableTemplates".into(), json!([]));
    } else {
        let mut all = Map::new();
        all.insert(String::new(), json!(ctx.tr("Default template")));
        all.extend(templates);
        settings.insert("availableTemplates".into(), Value::Object(all));
    }
    settings.insert("disablePostFormats".into(), json!(false));
    settings.insert("titlePlaceholder".into(), json!(ctx.tr("Add title")));
    settings.insert("bodyPlaceholder".into(), json!(ctx.tr("Type / to choose a block")));
    settings.insert("autosaveInterval".into(), json!(60));
    settings.insert("richEditingEnabled".into(), json!(super::editor::can_richedit(ctx, Some("true"))));
    // `wp_check_post_lock()` and `wp_set_post_lock()`.
    let holder = super::ajax::post_lock_holder(env, &ctx.user, post.id).await;
    if let Some(holder) = holder {
        let user = ctx.env.db.user(holder).await.ok().flatten();
        let mut details = Map::new();
        details.insert("name".into(), json!(user.map(|u| u.display_name).unwrap_or_default()));
        if site.show_avatars() {
            details.insert("avatar".into(), json!(rustpress_blocks::content::avatar_url(&site.options, &ctx.env.db.user(holder).await.ok().flatten().map(|u| u.user_email).unwrap_or_default(), 128)));
        }
        settings.insert("postLock".into(), json!({"isLocked": true, "user": Value::Object(details)}));
    } else {
        let lock = format!("{}:{}", env.now, ctx.user.id);
        if let Some(writer) = env.writer {
            writer.update_post_meta(post.id, "_edit_lock", &lock).await.or_log();
        }
        settings.insert("postLock".into(), json!({"isLocked": false, "activePostLock": lock}));
    }
    settings.insert("postLockUtils".into(), json!({"nonce": ctx.create_nonce(&format!("lock-post_{}", post.id)), "unlockNonce": ctx.create_nonce(&format!("update-post_{}", post.id)), "ajaxUrl": ctx.admin_url("admin-ajax.php")}));
    settings.insert("supportsLayout".into(), json!(env.block_theme));
    settings.insert("supportsTemplateMode".into(), json!(env.block_theme));
    // The user's choice to load the custom fields box; without that box (the type has no custom fields) the key goes.
    if site.post_type_supports(&post.post_type, "custom-fields") {
        let enabled = env.db.user_meta(ctx.user.id, "enable_custom_fields").await.ok().flatten().is_some_and(|v| !v.is_empty() && v != "0");
        settings.insert("enableCustomFields".into(), json!(enabled));
    }
}

/// `wp_get_post_content_block_attributes()`: the attributes of the first post content block of the template the post
/// is shown with (its own, else `page`/`single` or `singular`). Patterns and template parts are not looked into.
async fn post_content_block_attributes(ctx: &Ctx<'_>, post: &Post) -> Option<Value> {
    fn first_post_content(blocks: &[rustpress_blocks::parser::Block]) -> Option<&rustpress_blocks::parser::Block> {
        blocks.iter().find_map(|block| if block.name.as_deref() == Some("core/post-content") { Some(block) } else { first_post_content(&block.inner_blocks) })
    }
    if post.id == 0 {
        return None;
    }
    let theme = ctx.env.theme;
    // `get_page_template_slug()`: `default` is none.
    let own = ctx.env.db.post_meta_value(post.id, "_wp_page_template").await.ok().flatten().filter(|t| !t.is_empty() && t != "default");
    let slug = own.unwrap_or_else(|| {
        let wanted = if post.post_type == "page" { "page" } else { "single" };
        if theme.template_markup(wanted).is_some() { wanted.to_owned() } else { "singular".to_owned() }
    });
    let markup = theme.template_markup(&slug)?;
    let blocks = rustpress_blocks::parser::parse_blocks(&markup);
    let block = first_post_content(&blocks)?;
    // PHP encodes an empty array of attributes as `[]`.
    Some(if block.attrs.is_empty() { json!([]) } else { Value::Object(block.attrs.clone()) })
}

/// `translate_settings_using_i18n_schema()` of `block-i18n.json`: the texts of the block types.
fn translate_block_definitions(ctx: &Ctx<'_>, definitions: &mut Value) {
    let site = ctx.site();
    let text = |value: &mut Value, context: &str| {
        if let Some(text) = value.as_str().filter(|text| !text.is_empty()) {
            *value = json!(site.trx(text, context));
        }
    };
    // `build_template_part_block_area_variations()` takes the texts of `get_allowed_block_template_part_areas()`.
    for variation in definitions.get_mut("core/template-part").and_then(|block| block.get_mut("variations")).and_then(Value::as_array_mut).into_iter().flatten() {
        if !variation.get("name").and_then(Value::as_str).is_some_and(|name| name.starts_with("area_")) {
            continue;
        }
        if let Some(title) = variation.get("title").and_then(Value::as_str) {
            variation["title"] = json!(site.trx(title, "template part area"));
        }
        if let Some(description) = variation.get("description").and_then(Value::as_str) {
            variation["description"] = json!(site.tr(description));
        }
    }
    // `widget-group` has no text domain in its `block.json`.
    for (_, block) in definitions.as_object_mut().into_iter().flatten().filter(|(name, _)| name.as_str() != "core/widget-group") {
        for (key, context) in [("title", "block title"), ("description", "block description")] {
            if let Some(value) = block.get_mut(key) {
                text(value, context);
            }
        }
        for keyword in block.get_mut("keywords").and_then(Value::as_array_mut).into_iter().flatten() {
            text(keyword, "block keyword");
        }
        for style in block.get_mut("styles").and_then(Value::as_array_mut).into_iter().flatten() {
            if let Some(label) = style.get_mut("label") {
                text(label, "block style label");
            }
        }
        for variation in block.get_mut("variations").and_then(Value::as_array_mut).into_iter().flatten().filter(|variation| !variation.get("name").and_then(Value::as_str).is_some_and(|name| name.starts_with("area_"))) {
            for (key, context) in [("title", "block variation title"), ("description", "block variation description")] {
                if let Some(value) = variation.get_mut(key) {
                    text(value, context);
                }
            }
            for keyword in variation.get_mut("keywords").and_then(Value::as_array_mut).into_iter().flatten() {
                text(keyword, "block variation keyword");
            }
        }
    }
}

/// `get_default_block_categories()`.
fn block_categories(ctx: &Ctx<'_>) -> Value {
    let mut categories = statics().get("blockCategories").cloned().unwrap_or(Value::Null);
    for category in categories.as_array_mut().into_iter().flatten() {
        if let Some(title) = category.get("title").and_then(Value::as_str) {
            category["title"] = json!(ctx.site().trx(title, "block category"));
        }
    }
    categories
}

/// `get_allowed_block_template_part_areas()`.
fn template_part_areas(ctx: &Ctx<'_>) -> Value {
    let mut areas = statics().get("defaultTemplatePartAreas").cloned().unwrap_or(Value::Null);
    for area in areas.as_array_mut().into_iter().flatten() {
        if let Some(label) = area.get("label").and_then(Value::as_str) {
            area["label"] = json!(ctx.site().trx(label, "template part area"));
        }
        if let Some(description) = area.get("description").and_then(Value::as_str) {
            area["description"] = json!(ctx.site().tr(description));
        }
    }
    areas
}

/// `WP_Theme_JSON_Resolver::get_core_data()`: the names of the presets of WordPress, by `theme-i18n.json`.
fn translate_presets(ctx: &Ctx<'_>, features: &mut Value) {
    const NAMES: [(&str, &str, &str); 10] = [
        ("typography", "fontSizes", "Font size name"),
        ("typography", "fontFamilies", "Font family name"),
        ("color", "palette", "Color name"),
        ("color", "gradients", "Gradient name"),
        ("color", "duotone", "Duotone name"),
        ("spacing", "spacingSizes", "Space size name"),
        ("dimensions", "aspectRatios", "Aspect ratio name"),
        ("dimensions", "dimensionSizes", "Dimension size name"),
        ("shadow", "presets", "Shadow name"),
        ("border", "radiusSizes", "Border radius size name"),
    ];
    let site = ctx.site();
    let translate = |node: &mut Value| {
        for (group, key, context) in NAMES {
            let presets = node.get_mut(group).and_then(|g| g.get_mut(key)).and_then(|p| p.get_mut("default")).and_then(Value::as_array_mut);
            for preset in presets.into_iter().flatten() {
                if let Some(name) = preset.get("name").and_then(Value::as_str) {
                    preset["name"] = json!(site.trx(name, context));
                }
            }
        }
        // `WP_Theme_JSON::set_spacing_sizes()` makes these names itself.
        let sizes = node.get_mut("spacing").and_then(|g| g.get_mut("spacingSizes")).and_then(|p| p.get_mut("default")).and_then(Value::as_array_mut);
        for size in sizes.into_iter().flatten() {
            let Some(name) = size.get("name").and_then(Value::as_str).map(str::to_owned) else { continue };
            let digits = name.chars().take_while(char::is_ascii_digit).count();
            // `X-Small` and `X-Large` are `sprintf( __( '%sX-Small' ), '' )`.
            let translated = if digits > 0 || name.starts_with("X-") { rustpress_core::i18n::sprintf(&site.tr(&format!("%s{}", &name[digits..])), &[&name[..digits]]) } else { site.tr(&name) };
            size["name"] = json!(translated);
        }
    };
    translate(features);
    for block in features.get_mut("blocks").and_then(Value::as_object_mut).into_iter().flat_map(|blocks| blocks.values_mut()) {
        translate(block);
    }
}

/// `get_classic_theme_supports_block_editor_settings()` and what `WP_Theme_JSON_Resolver::get_theme_data()` does with
/// it for a theme without `theme.json`.
fn classic_features(ctx: &Ctx<'_>, features: &mut Value) {
    let theme = ctx.env.theme;
    let mut put = |group: &str, key: &str, value: Value| {
        if let Some(map) = features.get_mut(group).and_then(Value::as_object_mut) {
            map.insert(key.to_owned(), value);
        }
    };
    put("typography", "lineHeight", json!(theme.adds_theme_support("custom-line-height")));
    put("spacing", "units", if theme.adds_theme_support("custom-units") { json!(["px", "em", "rem", "vh", "vw", "%"]) } else { json!(false) });
    put("spacing", "padding", json!(theme.adds_theme_support("custom-spacing")));
    put("shadow", "defaultPresets", json!(false));
}

/// Which editor `get_block_editor_settings()` is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsFor {
    Post,
    Site,
    /// `widgets.php`, which passes the theme styles among its own settings.
    Widgets,
    /// The Customizer, which passes only the legacy widget settings.
    CustomizeWidgets,
}

pub(super) async fn editor_settings(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo, kind: SettingsFor) -> Value {
    let env = ctx.env;
    let site = ctx.site();
    let theme = env.theme;
    let baked = statics();
    let mut settings = Map::new();
    let get = |key: &str| baked.get(key).cloned().unwrap_or(Value::Null);

    let mut mimes = Map::new();
    for (exts, mime) in crate::media::allowed_mime_types(site, &ctx.user, false, false) {
        mimes.insert(exts, json!(mime));
    }
    settings.insert("alignWide".into(), json!(false));
    settings.insert("allowedBlockTypes".into(), json!(true));
    settings.insert("allowedMimeTypes".into(), Value::Object(mimes));
    settings.insert("defaultEditorStyles".into(), get("defaultEditorStyles"));
    settings.insert("blockCategories".into(), block_categories(ctx));
    settings.insert("isRTL".into(), json!(site.is_rtl()));
    let default_size = site.options.string("image_default_size", "");
    settings.insert("imageDefaultSize".into(), json!(if default_size.is_empty() { "large".to_owned() } else { default_size }));
    settings.insert("imageDimensions".into(), {
        let sizes = crate::media::registered_sizes(&site.options);
        let mut map = Map::new();
        for (name, w, h, crop) in sizes {
            if matches!(name.as_str(), "thumbnail" | "medium" | "large") {
                map.insert(name, json!({"width": w, "height": h, "crop": crop}));
            }
        }
        Value::Object(map)
    });
    settings.insert("imageEditing".into(), json!(true));
    settings.insert("imageSizes".into(), {
        let mut sizes = get("imageSizes");
        for size in sizes.as_array_mut().into_iter().flatten() {
            if let Some(name) = size.get("name").and_then(Value::as_str) {
                size["name"] = json!(site.tr(name));
            }
        }
        sizes
    });
    settings.insert("maxUploadFileSize".into(), json!(super::media_ui::max_upload()));
    settings.insert("__experimentalDashboardLink".into(), json!(ctx.admin_url("")));
    settings.insert("__unstableGalleryWithImageBlocks".into(), json!(true));
    for key in ["disableCustomColors", "disableCustomFontSizes", "disableCustomGradients", "disableLayoutStyles"] {
        settings.insert(key.into(), json!(false));
    }
    settings.insert("enableCustomLineHeight".into(), json!(true));
    settings.insert("enableCustomSpacing".into(), json!(true));
    settings.insert("enableCustomUnits".into(), json!(["%", "px", "em", "rem", "vh", "vw"]));
    let site_mode = kind == SettingsFor::Site;
    let widgets = matches!(kind, SettingsFor::Widgets | SettingsFor::CustomizeWidgets);
    if widgets {
        // `get_legacy_widget_block_editor_settings()`, then the theme styles `widgets.php` passes.
        settings.insert("widgetTypesToHideFromLegacyWidgetBlock".into(), json!(["pages", "calendar", "archives", "media_audio", "media_image", "media_gallery", "media_video", "search", "text", "categories", "recent-posts", "recent-comments", "rss", "tag_cloud", "custom_html", "block"]));
        if kind == SettingsFor::Widgets {
            settings.insert("styles".into(), Value::Null);
        }
    } else if site_mode {
        // `site-editor.php`
        settings.insert("siteUrl".into(), json!(site.settings.siteurl));
        settings.insert("postsPerPage".into(), json!(site.options.string("posts_per_page", "10")));
        // `site-editor.php` passes the theme styles among its own settings: their key comes here.
        settings.insert("styles".into(), Value::Null);
        let types: Vec<Value> = rustpress_rest::edit::default_template_types(site).into_iter().map(|(slug, title, description)| json!({"title": title, "description": description, "slug": slug})).collect();
        settings.insert("defaultTemplateTypes".into(), Value::Array(types));
        // The order of `site-editor.php`'s own settings.
        settings.insert("defaultTemplatePartAreas".into(), template_part_areas(ctx));
        settings.insert("supportsLayout".into(), json!(true));
        settings.insert("supportsTemplatePartsMode".into(), json!(false));
    } else {
        post_settings(ctx, post, &mut settings).await;
    }
    if !widgets {
        // `wp_theme_has_theme_json()`, and `current_theme_supports( 'block-templates' )`.
        settings.insert("supportsLayout".into(), json!(env.block_theme));
        settings.insert("__experimentalAdditionalBlockPatterns".into(), json!([]));
        settings.insert("__experimentalAdditionalBlockPatternCategories".into(), json!([]));
        if env.block_theme {
            settings.insert("defaultTemplatePartAreas".into(), template_part_areas(ctx));
        }
    }
    settings.insert("__experimentalBlockBindingsSupportedAttributes".into(), get("__experimentalBlockBindingsSupportedAttributes"));

    // Global styles.
    let global = rustpress_theme::global_styles::GlobalStyles::new(&theme.json, env.registry).with_other_blocks(ctx.site().plugins.block_names());
    let (variables, presets, styles_css) = global.editor_stylesheets();
    let mut styles: Vec<Value> = Vec::new();
    if !variables.is_empty() {
        styles.push(json!({"css": variables, "__unstableType": "presets", "isGlobalStyles": true}));
    }
    if !presets.is_empty() {
        styles.push(json!({"css": presets, "__unstableType": "presets", "isGlobalStyles": true}));
    }
    let styles_css = if env.block_theme { styles_css } else { global.classic_styles() };
    if !styles_css.is_empty() {
        styles.push(json!({"css": styles_css, "__unstableType": "theme", "isGlobalStyles": true}));
    }
    // The additional CSS of the Customizer, and the custom CSS of the global styles.
    let mods = rustpress_theme::classic::menu::theme_mod(site, &theme.stylesheet);
    let custom_css = rustpress_theme::classic::wp_get_custom_css(site, &mods).await.unwrap_or_default();
    styles.push(json!({"css": custom_css, "__unstableType": "user", "isGlobalStyles": false}));
    styles.push(json!({"css": "", "__unstableType": "user", "isGlobalStyles": true}));
    // `get_block_editor_theme_styles()`.
    let mut editor_styles = vec!["assets/css/editor-style.css"];
    if theme.stylesheet == "twentyseventeen" {
        editor_styles.push("assets/fonts/font-libre-franklin.css");
    }
    for file in editor_styles {
        if let Ok(css) = std::fs::read_to_string(theme.stylesheet_dir().join(file)) {
            let url = format!("{}/wp-content/themes/{}/{file}", site.settings.siteurl.trim_end_matches('/'), theme.stylesheet);
            styles.push(json!({"css": css, "baseURL": url, "__unstableType": "theme", "isGlobalStyles": false}));
        }
    }
    let mut features = theme.json.settings_by_origin();
    translate_presets(ctx, &mut features);
    if !env.block_theme {
        classic_features(ctx, &mut features);
    }
    styles.extend(duotone_entries(&features));
    settings.insert("styles".into(), Value::Array(styles));
    settings.insert("__experimentalFeatures".into(), Value::Null);
    // The most specific origin that has any items: the user's, the theme's, then core's.
    let palette = |features: &Value, path: &[&str]| {
        let mut node = Some(features);
        for key in path {
            node = node.and_then(|n| n.get(*key));
        }
        match node {
            Some(Value::Object(origins)) => ["custom", "theme", "default"].iter().find_map(|o| origins.get(*o).filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))).cloned().unwrap_or(json!([])),
            Some(Value::Array(items)) => Value::Array(items.clone()),
            _ => json!([]),
        }
    };
    // `get_block_editor_settings()`: the presets, then the legacy settings move out of the features (keys that are
    // there already keep their place).
    settings.insert("colors".into(), palette(&features, &["color", "palette"]));
    settings.insert("gradients".into(), palette(&features, &["color", "gradients"]));
    settings.insert("fontSizes".into(), palette(&features, &["typography", "fontSizes"]));
    for (path, key, negate) in [
        (["color", "custom"], "disableCustomColors", true),
        (["color", "customGradient"], "disableCustomGradients", true),
        (["typography", "customFontSize"], "disableCustomFontSizes", true),
        (["typography", "lineHeight"], "enableCustomLineHeight", false),
        (["spacing", "units"], "enableCustomUnits", false),
        (["spacing", "padding"], "enableCustomSpacing", false),
        (["spacing", "customSpacingSize"], "disableCustomSpacingSizes", true),
    ] {
        let Some(value) = features.get_mut(path[0]).and_then(Value::as_object_mut).and_then(|group| group.shift_remove(path[1])) else { continue };
        let value = if negate { json!(!value.as_bool().unwrap_or(false)) } else { value };
        settings.insert(key.into(), value);
    }
    settings.insert("spacingSizes".into(), palette(&features, &["spacing", "spacingSizes"]));
    // The features as they are left.
    if let Some(slot) = settings.get_mut("__experimentalFeatures") {
        *slot = features.clone();
    }

    // The assets inside the editor's iframe.
    let root = site.settings.siteurl.trim_end_matches('/').to_owned();
    let version = &env.loader.version;
    let link = |id: &str, path: &str| format!("<link rel='stylesheet' id='{id}-css' href='{root}/wp-includes/css/dist/{path}?ver={version}' media='all' />\n");
    // `_wp_get_iframed_editor_assets()`: a theme without `theme.json` keeps the reset styles and what they need.
    let classic = !env.block_theme;
    let plain = |id: &str, path: &str| format!("<link rel='stylesheet' id='{id}-css' href='{root}/{path}?ver={version}' media='all' />\n");
    let mut assets = String::new();
    assets.push_str(&link("wp-theme", "theme/design-tokens.min.css"));
    assets.push_str(&link("wp-base-styles", "base-styles/admin-schemes.min.css"));
    assets.push_str(&link("wp-components", "components/style.min.css"));
    if classic {
        assets.push_str(&plain("common", "wp-admin/css/common.min.css"));
        assets.push_str(&plain("forms", "wp-admin/css/forms.min.css"));
        assets.push_str(&link("wp-reset-editor-styles", "block-library/reset.min.css"));
    }
    assets.push_str(&link("wp-block-library", "block-library/style.min.css"));
    let inline: Vec<&str> = theme.block_style_inline().iter().map(|(_, css)| css.as_str()).collect();
    if !inline.is_empty() {
        assets.push_str(&format!("<style id=\"wp-block-library-inline-css\">\n{}\n/*# sourceURL=wp-block-library-inline-css */\n</style>\n", inline.join("\n")));
    }
    assets.push_str(&link("wp-block-editor-content", "block-editor/content.min.css"));
    if classic {
        assets.push_str(&link("wp-editor-classic-layout-styles", "edit-post/classic.min.css"));
    }
    assets.push_str(&link("wp-edit-blocks", "block-library/editor.min.css"));
    if classic {
        if theme.adds_theme_support("wp-block-styles") {
            assets.push_str(&link("wp-block-library-theme", "block-library/theme.min.css"));
        }
        assets.push_str(&plain("classic-theme-styles", "wp-includes/css/classic-themes.min.css"));
    }
    assets.push_str(&rustpress_theme::document::font_faces(theme));
    assets.push_str(&rustpress_theme::document::font_faces_from_style_variations(theme));
    settings.insert("__unstableResolvedAssets".into(), json!({"styles": assets, "scripts": format!("<script src=\"{root}/wp-admin/load-scripts.php?c=0&#038;load%5Bchunk_0%5D=wp-polyfill&#038;ver={version}\"></script>\n")}));
    settings.insert("__unstableIsBlockBasedTheme".into(), json!(env.block_theme));
    settings.insert("localAutosaveInterval".into(), json!(15));
    let o = |name: &str, default: &str| site.options.string(name, default);
    settings.insert(
        "__experimentalDiscussionSettings".into(),
        json!({
            "commentOrder": o("comment_order", "asc"),
            "commentsPerPage": o("comments_per_page", "50"),
            "defaultCommentsPage": o("default_comments_page", "newest"),
            "pageComments": o("page_comments", "0"),
            "threadComments": o("thread_comments", "0"),
            "threadCommentsDepth": o("thread_comments_depth", "5"),
            "defaultCommentStatus": o("default_comment_status", "open"),
            "avatarURL": rustpress_blocks::content::avatar_url(&site.options, "", 96).replace("&r=", "&f=y&r="),
        }),
    );
    if env.block_theme
        && !site_mode
        && let Some(attributes) = post_content_block_attributes(ctx, post).await
    {
        settings.insert("postContentAttributes".into(), attributes);
    }
    // `current_user_can( 'edit_block_binding', $block_editor_context )`: the widgets editor has no post.
    settings.insert("canUpdateBlockBindings".into(), json!(!widgets));
    settings.insert("canEditCSS".into(), json!((ctx.can("edit_css") || ctx.can("unfiltered_html"))));
    let _ = info;
    Value::Object(settings)
}

/// The inline scripts both editors start with: the REST client, the data stores, preferences and the blocks.
/// `enqueue_editor_block_styles_assets()`: the block styles of the theme for the editor.
pub(super) fn block_styles_script(ctx: &Ctx<'_>) -> String {
    let flags = rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false };
    let mut lines = vec!["( function() {".to_owned()];
    for (block, styles) in ctx.env.theme.registered_block_styles() {
        for (_, name, label, is_default) in styles {
            let mut style = Map::new();
            style.insert("name".into(), json!(name));
            style.insert("label".into(), json!(label));
            if is_default {
                style.insert("isDefault".into(), json!(true));
            }
            let json = rustpress_blocks::php_json::encode_with(&Value::Object(style), flags).replace('<', &format!("{}u003C", char::from(92u8))).replace('>', &format!("{}u003E", char::from(92u8)));
            lines.push(format!("	wp.blocks.registerBlockStyle( '{block}', {json} );"));
        }
    }
    lines.push("} )();".to_owned());
    lines.join("
")
}

/// What the editors print in the head besides their styles: the inline CSS of the block styles of the theme (twice, as
/// WordPress registers it twice) and the custom properties of the global styles.
pub(super) fn editor_head_styles(ctx: &Ctx<'_>, assets: &mut Assets) {
    let env = ctx.env;
    for _ in 0..2 {
        for code in env.theme.registered_block_style_inline_styles() {
            assets.inline_styles.push(("wp-edit-blocks".into(), code));
        }
    }
    let global = rustpress_theme::global_styles::GlobalStyles::new(&env.theme.json, env.registry).with_other_blocks(ctx.site().plugins.block_names());
    let (variables, _, _) = global.editor_stylesheets();
    if !variables.is_empty() {
        assets.inline_styles.push(("global-styles-css-custom-properties".into(), variables));
        assets.styles.push("global-styles-css-custom-properties".into());
    }
    if !env.block_theme {
        assets.classic_editor_styles = true;
        if env.theme.adds_theme_support("wp-block-styles") {
            assets.late_styles.push("wp-block-library-theme".into());
        }
        assets.late_styles.push("classic-theme-styles".into());
    }
    // `twentyseventeen_block_editor_styles()` on `enqueue_block_editor_assets`.
    if env.theme.stylesheet == "twentyseventeen" {
        let root = format!("{}/wp-content/themes/twentyseventeen/assets", ctx.site().settings.siteurl.trim_end_matches('/'));
        assets.theme_styles = format!(
            "<link rel='stylesheet' id='twentyseventeen-block-editor-style-css' href='{root}/css/editor-blocks.css?ver=20240824' media='all' />\n<link rel='stylesheet' id='twentyseventeen-fonts-css' href='{root}/fonts/font-libre-franklin.css?ver=20230328' media='all' />\n"
        );
    }
}

pub(super) async fn shared_inline(ctx: &Ctx<'_>, assets: &mut Assets, preloaded: &Value, page: EditorPage) {
    let boot = page == EditorPage::Boot;
    let site = ctx.site();
    let root = site.rest_url("/");
    let nonce = ctx.create_nonce("wp_rest");
    // `block_editor_rest_api_preload()` encodes with `JSON_HEX_TAG | JSON_UNESCAPED_SLASHES`, the boot pages plainly.
    let preloaded = if boot {
        rustpress_blocks::php_json::encode_with(preloaded, rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: false })
    } else {
        rustpress_blocks::php_json::encode_with(preloaded, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E")
    };
    // The fetch layer: root URL, nonce and the answers the editor starts with.
    assets.inline.retain(|(handle, _, _)| handle != "wp-api-fetch");
    assets.inline.push((
        "wp-api-fetch".into(),
        format!(
            "wp.apiFetch.use( wp.apiFetch.createRootURLMiddleware( {} ) );\nwp.apiFetch.nonceMiddleware = wp.apiFetch.createNonceMiddleware( {} );\nwp.apiFetch.use( wp.apiFetch.nonceMiddleware );\nwp.apiFetch.use( wp.apiFetch.mediaUploadMiddleware );\nwp.apiFetch.nonceEndpoint = {};\nwp.apiFetch.use( wp.apiFetch.createPreloadingMiddleware( {} ) );",
            json!(root),
            json!(nonce),
            json!(site.site_url("wp-admin/admin-ajax.php?action=rest-nonce")),
            preloaded
        ),
        "after".into(),
    ));
    assets.inline.push((
        "wp-data".into(),
        format!("( function() {{\n\tvar userId = {};\n\tvar storageKey = \"WP_DATA_USER_\" + userId;\n\twp.data\n\t\t.use( wp.data.plugins.persistence, {{ storageKey: storageKey }} );\n}} )();", ctx.user.id),
        "after".into(),
    ));
    let preferences = site
        .db
        .user_meta(ctx.user.id, &format!("{}persisted_preferences", site.db.table_prefix()))
        .await
        .ok()
        .flatten()
        .and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok())
        .map_or_else(|| json!(""), |value| rustpress_core::options::php_to_json(&value));
    assets.inline.push((
        "wp-preferences".into(),
        format!("( function() {{\n\t\t\t\tvar serverData = {};\n\t\t\t\tvar userId = \"{}\";\n\t\t\t\tvar persistenceLayer = wp.preferencesPersistence.__unstableCreatePersistenceLayer( serverData, userId );\n\t\t\t\tvar preferencesStore = wp.preferences.store;\n\t\t\t\twp.data.dispatch( preferencesStore ).setPersistenceLayer( persistenceLayer );\n\t\t\t}} ) ();", inline_json(&preferences), ctx.user.id),
        "after".into(),
    ));
    assets.inline.push(("wp-editor".into(), "Object.assign( window.wp.editor, window.wp.oldEditor );".into(), "after".into()));
    assets.inline.push(("editor".into(), "window.wp.oldEditor = window.wp.editor;".into(), "after".into()));
    assets.inline.push((
        "wp-block-library".into(),
        format!(
            "window.wpEditorL10n = {{\n\t\ttinymce: {{\n\t\t\tbaseURL: {},\n\t\t\tsuffix: \".min\",\n\t\t\tsettings: {{plugins:\"charmap,colorpicker,hr,lists,media,paste,tabfocus,textcolor,fullscreen,wordpress,wpautoresize,wpeditimage,wpemoji,wpgallery,wplink,wpdialogs,wptextpattern,wpview\",toolbar1:\"formatselect,bold,italic,bullist,numlist,blockquote,alignleft,aligncenter,alignright,link,unlink,wp_more,spellchecker,wp_add_media,wp_adv\",toolbar2:\"strikethrough,hr,forecolor,pastetext,removeformat,charmap,outdent,indent,undo,redo,wp_help\",toolbar3:\"\",toolbar4:\"\",external_plugins:[],classic_block_editor:true}},\n\t\t}}\n\t}}",
            json!(site.site_url("wp-includes/js/tinymce"))
        ),
        "before".into(),
    ));
    let mut definitions = statics().get("blockDefinitions").cloned().unwrap_or(Value::Null);
    translate_block_definitions(ctx, &mut definitions);
    // The variations that follow the post types and taxonomies of this site.
    let rest = rustpress_rest::Rest::new(site, ctx.env.theme, ctx.env.registry);
    for (block, variations) in rest.type_variations() {
        if let Some(slot) = definitions.get_mut(block).and_then(|b| b.get_mut("variations")) {
            *slot = Value::Array(variations);
        }
    }
    // `build_template_part_block_area_variations()`: without a part of a theme for the area, the inserter offers it.
    if !ctx.env.block_theme
        && let Some(Value::Array(variations)) = definitions.get_mut("core/template-part").and_then(|b| b.get_mut("variations"))
    {
        variations.retain(|variation| variation.get("name").and_then(Value::as_str).is_some_and(|name| name.starts_with("area_")));
        for variation in variations {
            variation["scope"] = json!(["inserter"]);
        }
    }
    let categories = format!("wp.blocks.setCategories( {} );", inline_json(&block_categories(ctx)));
    let definitions = format!("wp.blocks.unstable__bootstrapServerSideBlockDefinitions({});", inline_json(&definitions));
    let sources = format!("for ( const source of {} ) {{ wp.blocks.registerBlockBindingsSource( source ); }}", inline_json(&bindings_sources(ctx)));
    // The post editor sets the categories first, the site editor last.
    let order = match page {
        EditorPage::Post => vec![categories, definitions, sources],
        EditorPage::Site => vec![definitions, sources, categories],
        EditorPage::Boot => Vec::new(),
    };
    for code in order {
        assets.inline.push(("wp-blocks".into(), code, "after".into()));
    }
}

/// The block editor for `post`.
pub async fn render_editor(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo) -> AdminResponse {
    let site = ctx.site();
    let env = ctx.env;
    let tr = |s: &str| ctx.tr(s);
    let gs_id = global_styles_id(ctx).await;
    let preloaded = preload(ctx, post, info, gs_id).await;
    let mut settings = editor_settings(ctx, post, info, SettingsFor::Post).await;
    // `add_meta_boxes` of the plugins (`register_and_do_post_meta_boxes()`).
    let plugin_boxes = {
        let parts = ctx.plugin_parts();
        let mut plugin_ctx = ctx.plugin_context(&parts, if post.post_status == "auto-draft" { "post-new.php" } else { "post.php" });
        plugin_ctx.edited_post = Some(post);
        plugin_ctx.block_editor = true;
        site.plugins.post_meta_boxes(&plugin_ctx, post).await.unwrap_or_else(|e| {
            tracing::error!(error = %e, "plugin post meta boxes failed");
            rustpress_core::native::PostMetaBoxes::default()
        })
    };
    // Without the custom fields box the editor does not offer it.
    if plugin_boxes.removed.iter().any(|id| id == "postcustom")
        && let Some(map) = settings.as_object_mut()
    {
        map.shift_remove("enableCustomFields");
    }
    let boxes: Vec<super::meta_boxes::MetaBox> = plugin_boxes.boxes.iter().cloned().map(super::meta_boxes::MetaBox::from_plugin).collect();
    let state = super::meta_boxes::BoxState::load(ctx, &post.post_type, &post.post_type).await;
    let location = |context: &str| super::meta_boxes::do_meta_boxes(ctx, &boxes, context, &state);
    let hidden_fields: String = plugin_boxes
        .hidden_fields
        .iter()
        .map(|part| match part {
            rustpress_core::native::FormPart::Html(html) => html.clone(),
            rustpress_core::native::FormPart::MetaBoxes(context) => location(context),
        })
        .collect();
    let is_new = post.post_status == "auto-draft";
    let heading = if is_new {
        if info.hierarchical { tr("Add Page") } else { tr("Add Post") }
    } else {
        sprintf(&tr("Edit %s"), &[&info.singular])
    };

    let mut assets = Assets {
        scripts: ["heartbeat", "wp-edit-post", "media-editor", "media-audiovideo", "mce-view", "image-edit", "imgareaselect", "quicktags", "wplink", "jquery-ui-autocomplete", "thickbox", "media-upload", "wp-block-styles", "wp-block-directory", "wp-format-library"].iter().map(|s| (*s).to_owned()).collect(),
        editor_modules: true,
        no_emoji: true,
        styles: ["media-views", "editor-buttons", "wp-edit-post", "wp-block-directory", "wp-format-library", "wp-edit-blocks", "wp-commands"].iter().map(|s| (*s).to_owned()).collect(),
        head_early: "\t<script>\n\t\tif ( typeof performance !== 'undefined' && performance.navigation && performance.navigation.type === 2 ) {\n\t\t\tdocument.location.reload( true );\n\t\t}\n\t</script>\n\t".into(),
        ..Default::default()
    };
    super::media_ui::enqueue(ctx, &mut assets, post.id).await;
    // `enqueue_block_editor_assets` of the plugins.
    {
        let parts = ctx.plugin_parts();
        let hook = if is_new { "post-new.php" } else { "post.php" };
        let mut plugin_ctx = ctx.plugin_context(&parts, hook);
        // The global `$post` of the editor.
        plugin_ctx.edited_post = Some(post);
        plugin_ctx.block_editor = true;
        // `block_editor_settings_all` with the post as the context.
        site.plugins.block_editor_settings(&plugin_ctx, post).await.unwrap_or_else(|e| tracing::error!(error = %e, "plugin block_editor_settings failed"));
        for script in site.plugins.block_editor_assets(&plugin_ctx).scripts {
            // `get_import_map()` walks the queue from its end: a last script that needs `wp-edit-post` brings the
            // module dependencies of the editor (the router) before those of the block library.
            if script.deps.iter().any(|d| d == "wp-edit-post") {
                assets.route_first = true;
            }
            assets.scripts.push(script.handle.clone());
            assets.plugin_page_scripts.push(script);
        }
    }
    // `wp_enqueue_editor()`: the defaults of `wp.editor` for the classic block (`admin_print_footer_scripts`, 45).
    let rich_editing = ctx.env.db.user_meta(ctx.user.id, "rich_editing").await.ok().flatten();
    assets.footer = super::editor::default_editor_scripts(ctx, super::editor::can_richedit(ctx, rich_editing.as_deref()));
    editor_head_styles(ctx, &mut assets);
    assets.inline.push(("wp-block-styles".into(), block_styles_script(ctx), "after".into()));
    // Post locks: the heartbeat checks every ten seconds, and the meta box loader nonces are renewed.
    assets.inline.push(("heartbeat".into(), "jQuery( function() {
		wp.heartbeat.interval( 10 );
	} );".into(), "after".into()));
    assets.inline.push(("heartbeat".into(), include_str!("../data/heartbeat_nonces.js").replace(char::from(13u8), ""), "after".into()));
    // A new post starts with what the form is made of (`get_default_post_to_edit()` empties the title).
    let initial_edits = if is_new {
        let mut edits = Map::new();
        for (feature, key) in [("title", "title"), ("editor", "content"), ("excerpt", "excerpt")] {
            if site.post_type_supports(&post.post_type, feature) {
                let value = match key {
                    "content" => post.post_content.clone(),
                    "excerpt" => post.post_excerpt.clone(),
                    _ => String::new(),
                };
                edits.insert(key.to_owned(), json!(value));
            }
        }
        Value::Object(edits)
    } else {
        json!([])
    };
    let init = format!(
        "( function() {{\n\twindow._wpLoadBlockEditor = new Promise( function( resolve ) {{\n\t\twp.domReady( function() {{\n\t\t\tresolve( wp.editPost.initializeEditor( 'editor', {}, {}, {}, {} ) );\n\t\t}} );\n\t}} );\n}} )();\nwindow._wpLoadBlockEditor.then( function() {{\n\t\twp.data.dispatch( 'core/edit-post' ).setAvailableMetaBoxesPerLocation( {} );\n\t}} );",
        json!(post.post_type),
        post.id,
        inline_json(&settings),
        inline_json(&initial_edits),
        inline_json(&super::meta_boxes::per_location(&boxes, &state))
    );
    shared_inline(ctx, &mut assets, &preloaded, EditorPage::Post).await;
    assets.inline.push(("wp-edit-post".into(), init, "after".into()));
    // `WP_Privacy_Policy_Content::notice()` on `admin_notices`: the block editor shows it as a warning of its own.
    if ctx.can("manage_privacy_options") && site.options.u64("wp_page_for_privacy_policy", 0) == post.id && post.id != 0 {
        let action = json!({"url": esc_url(&ctx.admin_url("options-privacy.php?tab=policyguide")), "label": tr("View Privacy Policy Guide.")});
        let action = rustpress_blocks::php_json::encode_with(&action, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E");
        let message = tr("Need help putting together your new Privacy Policy page? Check out the guide for recommendations on what content to include, along with policies suggested by your plugins and theme.");
        assets.inline.push(("wp-notices".into(), format!("wp.data.dispatch( \"core/notices\" ).createWarningNotice( \"{message}\", {{ actions: [ {action} ], isDismissible: false }} )"), "after".into()));
    }
    assets.inline.push(("wp-editor".into(), format!("var _wpMetaBoxUrl = {};", json!(format!("{}?post={}&action=edit&meta-box-loader=1&meta-box-loader-nonce={}", ctx.admin_url("post.php"), post.id, ctx.create_nonce("meta-box-loader")))), "before".into()));

    // An installed Classic Editor gets a link to activate it, otherwise one to install it.
    let wp_root = ctx.env.loader.includes.parent().unwrap_or(&ctx.env.loader.includes);
    let installed = wp_root.join("wp-content/plugins/classic-editor/classic-editor.php").is_file();
    let classic_link = if installed {
        format!("plugins.php?action=activate&amp;plugin=classic-editor%2Fclassic-editor.php&amp;_wpnonce={}", ctx.create_nonce("activate-plugin_classic-editor/classic-editor.php"))
    } else {
        esc_url(&ctx.admin_url(&format!("update.php?action=install-plugin&plugin=classic-editor&_wpnonce={}", ctx.create_nonce("install-plugin_classic-editor")))).replace("&#038;", "&amp;")
    };
    let content = format!(
        "
<div class=\"block-editor\">\n\t<h1 class=\"screen-reader-text hide-if-no-js\">{heading}</h1>\n\t<div id=\"editor\" class=\"block-editor__container hide-if-no-js\"></div>\n\t<div id=\"metaboxes\" class=\"hidden\">\n\t\t\t<form class=\"metabox-base-form\">\n\t{nonce_field}\t<input type=\"hidden\" id=\"user-id\" name=\"user_ID\" value=\"{user}\" />\n\t<input type=\"hidden\" id=\"hiddenaction\" name=\"action\" value=\"editpost\" />\n\t<input type=\"hidden\" id=\"originalaction\" name=\"originalaction\" value=\"editpost\" />\n\t<input type=\"hidden\" id=\"post_type\" name=\"post_type\" value=\"{pt}\" />\n\t<input type=\"hidden\" id=\"original_post_status\" name=\"original_post_status\" value=\"{status}\" />\n\t<input type=\"hidden\" id=\"referredby\" name=\"referredby\" value=\"\" />\n\n\t{original_referer}{auto_draft}<input type='hidden' id='post_ID' name='post_ID' value='{id}' /><input type=\"hidden\" id=\"meta-box-order-nonce\" name=\"meta-box-order-nonce\" value=\"{order_nonce}\" /><input type=\"hidden\" id=\"closedpostboxesnonce\" name=\"closedpostboxesnonce\" value=\"{closed_nonce}\" /><input type=\"hidden\" id=\"samplepermalinknonce\" name=\"samplepermalinknonce\" value=\"{sample_nonce}\" />{hidden_fields}\t</form>\n\t<form id=\"toggle-custom-fields-form\" method=\"post\" action=\"{post_php}\">\n\t\t<input type=\"hidden\" id=\"toggle-custom-fields-nonce\" name=\"toggle-custom-fields-nonce\" value=\"{toggle_nonce}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{referer}\" />\t\t<input type=\"hidden\" name=\"action\" value=\"toggle-custom-fields\" />\n\t</form>\n\t\t\t<form class=\"metabox-location-side\" onsubmit=\"return false;\">\n\t\t\t<div id=\"poststuff\" class=\"sidebar-open\">\n\t\t\t\t<div id=\"postbox-container-2\" class=\"postbox-container\">\n\t\t\t\t\t{side}\t\t\t\t</div>\n\t\t\t</div>\n\t\t</form>\n\t\t\t<form class=\"metabox-location-normal\" onsubmit=\"return false;\">\n\t\t\t<div id=\"poststuff\" class=\"sidebar-open\">\n\t\t\t\t<div id=\"postbox-container-2\" class=\"postbox-container\">\n\t\t\t\t\t{normal}\t\t\t\t</div>\n\t\t\t</div>\n\t\t</form>\n\t\t\t<form class=\"metabox-location-advanced\" onsubmit=\"return false;\">\n\t\t\t<div id=\"poststuff\" class=\"sidebar-open\">\n\t\t\t\t<div id=\"postbox-container-2\" class=\"postbox-container\">\n\t\t\t\t\t{advanced}\t\t\t\t</div>\n\t\t\t</div>\n\t\t</form>\n\t\t\t</div>\n\n\t\t<div class=\"wrap hide-if-js block-editor-no-js\">\n\t\t<h1 class=\"wp-heading-inline\">{heading}</h1>\n\t\t<div class=\"notice notice-error\"><p>{no_js}</p></div>\t</div>\n</div>\n",
        heading = esc_html(&heading),
        nonce_field = ctx.nonce_field(&format!("update-post_{}", post.id)),
        user = ctx.user.id,
        pt = esc_attr(&post.post_type),
        status = esc_attr(&post.post_status),
        id = post.id,
        // `wp_original_referer_field()`, except for drafts.
        original_referer = if post.post_status == "draft" { "" } else { "<input type=\"hidden\" name=\"_wp_original_http_referer\" value=\"\" />" },
        auto_draft = if is_new { "<input type='hidden' id='auto_draft' name='auto_draft' value='1' />" } else { "" },
        order_nonce = ctx.create_nonce("meta-box-order"),
        closed_nonce = ctx.create_nonce("closedpostboxes"),
        sample_nonce = ctx.create_nonce("samplepermalink"),
        side = location("side"),
        normal = location("normal"),
        advanced = location("advanced"),
        toggle_nonce = ctx.create_nonce("toggle-custom-fields"),
        post_php = esc_url(&ctx.admin_url("post.php")),
        referer = super::referer_field_value(&ctx.request.request_uri),
        no_js = sprintf(
            &tr(if installed {
                "The block editor requires JavaScript. Please enable JavaScript in your browser settings, or activate the <a href=\"%s\">Classic Editor plugin</a>."
            } else {
                "The block editor requires JavaScript. Please enable JavaScript in your browser settings, or install the <a href=\"%s\">Classic Editor plugin</a>."
            }),
            &[&classic_link]
        ),
    );
    let title = if is_new { heading.clone() } else { sprintf(&tr("Edit %s &#8220;%s&#8221;"), &[&info.singular, &esc_html(&post.post_title)]) };
    let view = if post.post_status == "auto-draft" { None } else { site.permalink(post).await.ok().map(|url| (sprintf(&tr("View %s"), &[&info.singular]), url)) };
    // `wp_admin_bar_edit_menu()`: a draft has its preview instead.
    let view_preview = (view.is_some() && post.post_status == "draft").then_some(post.id);
    let view = view.map(|(label, url)| if view_preview.is_some() { (label, super::common::preview_url(&url)) } else { (label, url) });
    let screen = Screen {
        file: if is_new { "post-new.php".into() } else { "post.php".into() },
        parent_file: if post.post_type == "post" { "edit.php".into() } else { format!("edit.php?post_type={}", post.post_type) },
        submenu_file: Some(match (post.post_status == "auto-draft", post.post_type == "post") {
            (true, true) => "post-new.php".into(),
            (true, false) => format!("post-new.php?post_type={}", post.post_type),
            (false, true) => "edit.php".into(),
            (false, false) => format!("edit.php?post_type={}", post.post_type),
        }),
        post_type: post.post_type.clone(),
        title,
        help: Some(crate::chrome::Help::default()),
        view_item: view,
        view_preview,
        fullscreen: true,
        body_classes: vec!["block-editor-page".into(), "wp-embed-responsive".into()],
        edited_post: Some(post.clone()),
        block_editor: true,
        ..Default::default()
    };
    // The editor a plugin prints on `admin_footer`, with what `_WP_Editors::enqueue_scripts()` adds for it.
    if let Some(hidden) = &plugin_boxes.hidden_editor {
        let editor_post = super::editor::EditorPost::of(ctx, post).await;
        let (html, entry) = super::editor::default_editor_for(ctx, &hidden.id, "", false, Some(&editor_post));
        assets.footer_first = format!("{}{html}{}", hidden.before, hidden.after);
        let rich = entry.tinymce.is_some();
        // The default editor's scripts already printed TinyMCE and the link dialog.
        assets.footer.push_str(&super::editor::editors_js_with(ctx, &[entry], false));
        if rich {
            assets.post_header_scripts.push("editor".into());
        }
        if ctx.can("upload_files") {
            assets.post_header_scripts.push("wp-embed".into());
            assets.post_header_styles.push("thickbox".into());
        }
    }
    let _ = env;
    render(ctx, &screen, &content, &assets).await
}
