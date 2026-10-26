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

/// `wp_json_encode( $value, JSON_HEX_TAG )`: safe inside an inline script.
pub(super) fn inline_json(value: &Value) -> String {
    value.to_string().replace('<', "\\u003C").replace('>', "\\u003E")
}

fn statics() -> Value {
    serde_json::from_str(STATIC).unwrap_or(Value::Null)
}

/// `use_block_editor_for_post()`: the post type shows in the REST API and has an editor.
pub fn uses_block_editor(ctx: &Ctx<'_>, post: &Post) -> bool {
    if ctx.request.get("classic-editor").is_some() || ctx.request.get("rp-classic").is_some() {
        return false;
    }
    if post.post_status == "trash" {
        return false;
    }
    match post.post_type.as_str() {
        "post" | "page" => true,
        other => ctx.site().types.post_type(other).is_some_and(|t| t.show_in_rest && t.supports("editor")),
    }
}

/// The answer of one REST request, in the shape of the preloading middleware.
async fn preload_one(ctx: &Ctx<'_>, method: &str, path: &str) -> Option<Value> {
    let env = ctx.env;
    let writer = env.writer?;
    let (route, query) = path.split_once('?').map_or((path, ""), |(r, q)| (r, q));
    let rest = rustpress_rest::Rest::new(ctx.site(), env.theme, env.registry);
    let (auth, user_id, token, now) = (env.auth.clone(), ctx.user.id, ctx.token.clone(), env.now);
    let nonce: std::sync::Arc<dyn Fn(&str) -> String + Send + Sync> = std::sync::Arc::new(move |action: &str| auth.create_nonce(action, user_id, &token, now).unwrap_or_default());
    let access = Access { user: ctx.user.clone(), app_password: None, writer: Some(writer), now: env.now, uploads: super::ajax_media::uploads_dir(env), nonce: Some(nonce), roles: Some(env.roles) };
    let reply = rest.handle_as(route, query, Some(access), Request { method: method.to_owned(), body: Body::None, basic: None, remote_addr: ctx.request.remote_addr.clone(), user_agent: ctx.request.user_agent.clone() }).await.ok()?;
    if reply.status >= 400 {
        return None;
    }
    let body: Value = serde_json::from_str(&reply.body).unwrap_or(Value::Null);
    let mut headers = Map::new();
    for (name, value) in &reply.headers {
        if matches!(name.as_str(), "Link" | "Allow" | "X-WP-Total" | "X-WP-TotalPages") && !(name == "Link" && value.contains("api.w.org")) {
            headers.insert(name.clone(), Value::from(value.clone()));
        }
    }
    if let Some(allow) = reply.headers.iter().find(|(n, _)| n == "Allow") {
        headers.insert("Allow".into(), Value::from(allow.1.clone()));
    }
    Some(json!({"body": body, "headers": Value::Object(headers)}))
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
        ("OPTIONS".into(), format!("/wp/v2/{base}")),
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
        paths.push(("GET".into(), format!("/wp/v2/global-styles/{id}?context=edit")));
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
    paths.push(("GET".into(), format!("/wp/v2/users/{}?context=view&_fields=id,name", post.post_author)));
    preload_paths(ctx, paths).await
}

/// `block_editor_rest_api_preload()`: the answers of the requests in the order they are asked; the `OPTIONS`
/// answers are collected under one key that sits where the first of them was.
pub(super) async fn preload_paths(ctx: &Ctx<'_>, paths: Vec<(String, String)>) -> Value {
    let mut ordered = Map::new();
    let mut options = Map::new();
    for (method, path) in paths {
        let Some(value) = preload_one(ctx, &method, &path).await else { continue };
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
    settings.insert("supportsTemplateMode".into(), json!(env.block_theme));
    settings.insert("enableCustomFields".into(), json!(false));
    settings.insert("postContentAttributes".into(), json!({"align": "full", "layout": {"type": "constrained"}}));
}

pub(super) async fn editor_settings(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo) -> Value {
    let env = ctx.env;
    let site = ctx.site();
    let theme = env.theme;
    let baked = statics();
    let mut settings = Map::new();
    let get = |key: &str| baked.get(key).cloned().unwrap_or(Value::Null);

    let mut mimes = Map::new();
    for (exts, mime) in crate::media::MIME_TYPES {
        mimes.insert((*exts).to_owned(), json!(mime));
    }
    settings.insert("alignWide".into(), json!(false));
    settings.insert("allowedBlockTypes".into(), json!(true));
    settings.insert("allowedMimeTypes".into(), Value::Object(mimes));
    settings.insert("defaultEditorStyles".into(), get("defaultEditorStyles"));
    settings.insert("blockCategories".into(), get("blockCategories"));
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
    settings.insert("imageSizes".into(), get("imageSizes"));
    settings.insert("maxUploadFileSize".into(), json!(super::media_ui::MAX_UPLOAD));
    settings.insert("__experimentalDashboardLink".into(), json!(ctx.admin_url("")));
    settings.insert("__unstableGalleryWithImageBlocks".into(), json!(true));
    for key in ["disableCustomColors", "disableCustomFontSizes", "disableCustomGradients", "disableLayoutStyles"] {
        settings.insert(key.into(), json!(false));
    }
    settings.insert("enableCustomLineHeight".into(), json!(true));
    settings.insert("enableCustomSpacing".into(), json!(true));
    settings.insert("enableCustomUnits".into(), json!(["%", "px", "em", "rem", "vh", "vw"]));
    let site_mode = post.id == 0 && post.post_type.is_empty();
    if site_mode {
        // `site-editor.php`
        settings.insert("siteUrl".into(), json!(site.settings.siteurl));
        settings.insert("postsPerPage".into(), json!(site.options.string("posts_per_page", "10")));
        let types: Vec<Value> = rustpress_rest::edit::default_template_types(site).into_iter().map(|(slug, title, description)| json!({"title": title, "description": description, "slug": slug})).collect();
        settings.insert("defaultTemplateTypes".into(), Value::Array(types));
        settings.insert("supportsTemplatePartsMode".into(), json!(false));
    } else {
        post_settings(ctx, post, &mut settings).await;
    }
    settings.insert("supportsLayout".into(), json!(true));
    settings.insert("__experimentalAdditionalBlockPatterns".into(), json!([]));
    settings.insert("__experimentalAdditionalBlockPatternCategories".into(), json!([]));
    settings.insert("defaultTemplatePartAreas".into(), get("defaultTemplatePartAreas"));
    settings.insert("__experimentalBlockBindingsSupportedAttributes".into(), get("__experimentalBlockBindingsSupportedAttributes"));

    // Global styles.
    let global = rustpress_theme::global_styles::GlobalStyles::new(&theme.json, env.registry);
    let (variables, presets, styles_css) = global.editor_stylesheets();
    let mut styles: Vec<Value> = Vec::new();
    if !variables.is_empty() {
        styles.push(json!({"css": variables, "__unstableType": "presets", "isGlobalStyles": true}));
    }
    if !presets.is_empty() {
        styles.push(json!({"css": presets, "__unstableType": "presets", "isGlobalStyles": true}));
    }
    if !styles_css.is_empty() {
        styles.push(json!({"css": styles_css, "__unstableType": "theme", "isGlobalStyles": true}));
    }
    // The custom CSS of the user (empty entries are what the editor expects).
    styles.push(json!({"css": "", "__unstableType": "user", "isGlobalStyles": false}));
    styles.push(json!({"css": "", "__unstableType": "user", "isGlobalStyles": true}));
    // The editor style of the theme.
    let editor_style = theme.stylesheet_dir().join("assets").join("css").join("editor-style.css");
    if let Ok(css) = std::fs::read_to_string(&editor_style) {
        let url = format!("{}/wp-content/themes/{}/assets/css/editor-style.css", site.settings.siteurl.trim_end_matches('/'), theme.stylesheet);
        styles.push(json!({"css": css, "baseURL": url, "__unstableType": "theme", "isGlobalStyles": false}));
    }
    let mut features = theme.json.settings_by_origin();
    styles.extend(duotone_entries(&features));
    settings.insert("styles".into(), Value::Array(styles));
    // Legacy settings move out of the features into the editor settings (`get_block_editor_settings()`).
    for (path, key, negate) in [
        (["color", "custom"], "disableCustomColors", true),
        (["color", "customGradient"], "disableCustomGradients", true),
        (["typography", "customFontSize"], "disableCustomFontSizes", true),
        (["typography", "lineHeight"], "enableCustomLineHeight", false),
        (["spacing", "units"], "enableCustomUnits", false),
        (["spacing", "padding"], "enableCustomSpacing", false),
        (["spacing", "customSpacingSize"], "disableCustomSpacingSizes", true),
    ] {
        let Some(value) = features.get_mut(path[0]).and_then(Value::as_object_mut).and_then(|group| group.remove(path[1])) else { continue };
        let value = if negate { json!(!value.as_bool().unwrap_or(false)) } else { value };
        settings.insert(key.into(), value);
    }
    settings.insert("__experimentalFeatures".into(), features.clone());
    // The most specific origin that has any items: the user's, the theme's, then core's.
    let palette = |path: &[&str]| {
        let mut node = Some(&features);
        for key in path {
            node = node.and_then(|n| n.get(*key));
        }
        match node {
            Some(Value::Object(origins)) => ["custom", "theme", "default"].iter().find_map(|o| origins.get(*o).filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))).cloned().unwrap_or(json!([])),
            Some(Value::Array(items)) => Value::Array(items.clone()),
            _ => json!([]),
        }
    };
    settings.insert("colors".into(), palette(&["color", "palette"]));
    settings.insert("gradients".into(), palette(&["color", "gradients"]));
    settings.insert("fontSizes".into(), palette(&["typography", "fontSizes"]));
    settings.entry("disableCustomSpacingSizes".to_owned()).or_insert(json!(false));
    settings.insert("spacingSizes".into(), palette(&["spacing", "spacingSizes"]));

    // The assets inside the editor's iframe.
    let root = site.settings.siteurl.trim_end_matches('/').to_owned();
    let version = &env.loader.version;
    let link = |id: &str, path: &str| format!("<link rel='stylesheet' id='{id}-css' href='{root}/wp-includes/css/dist/{path}?ver={version}' media='all' />\n");
    let mut assets = String::new();
    assets.push_str(&link("wp-theme", "theme/design-tokens.min.css"));
    assets.push_str(&link("wp-base-styles", "base-styles/admin-schemes.min.css"));
    assets.push_str(&link("wp-components", "components/style.min.css"));
    assets.push_str(&link("wp-block-library", "block-library/style.min.css"));
    let inline: Vec<&str> = theme.block_style_inline().iter().map(|(_, css)| css.as_str()).collect();
    if !inline.is_empty() {
        assets.push_str(&format!("<style id=\"wp-block-library-inline-css\">\n{}\n/*# sourceURL=wp-block-library-inline-css */\n</style>\n", inline.join("\n")));
    }
    assets.push_str(&link("wp-block-editor-content", "block-editor/content.min.css"));
    assets.push_str(&link("wp-edit-blocks", "block-library/editor.min.css"));
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
    settings.insert("canUpdateBlockBindings".into(), json!(true));
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
    let global = rustpress_theme::global_styles::GlobalStyles::new(&env.theme.json, env.registry);
    let (variables, _, _) = global.editor_stylesheets();
    if !variables.is_empty() {
        assets.inline_styles.push(("global-styles-css-custom-properties".into(), variables));
        assets.styles.push("global-styles-css-custom-properties".into());
    }
}

pub(super) async fn shared_inline(ctx: &Ctx<'_>, assets: &mut Assets, preloaded: &Value) {
    let site = ctx.site();
    let root = site.site_url("wp-json/");
    let nonce = ctx.create_nonce("wp_rest");
    // The fetch layer: root URL, nonce and the answers the editor starts with.
    assets.inline.retain(|(handle, _, _)| handle != "wp-api-fetch");
    assets.inline.push((
        "wp-api-fetch".into(),
        format!(
            "wp.apiFetch.use( wp.apiFetch.createRootURLMiddleware( {} ) );\nwp.apiFetch.nonceMiddleware = wp.apiFetch.createNonceMiddleware( {} );\nwp.apiFetch.use( wp.apiFetch.nonceMiddleware );\nwp.apiFetch.use( wp.apiFetch.mediaUploadMiddleware );\nwp.apiFetch.nonceEndpoint = {};\nwp.apiFetch.use( wp.apiFetch.createPreloadingMiddleware( {} ) );",
            json!(root),
            json!(nonce),
            json!(site.site_url("wp-admin/admin-ajax.php?action=rest-nonce")),
            inline_json(preloaded)
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
    let definitions = statics().get("blockDefinitions").cloned().unwrap_or(Value::Null);
    let categories = statics().get("blockCategories").cloned().unwrap_or(Value::Null);
    assets.inline.push((
        "wp-blocks".into(),
        format!("wp.blocks.setCategories( {} );\nwp.blocks.unstable__bootstrapServerSideBlockDefinitions({});", inline_json(&categories), inline_json(&definitions)),
        "after".into(),
    ));
}

/// The block editor for `post`.
pub async fn render_editor(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo) -> AdminResponse {
    let site = ctx.site();
    let env = ctx.env;
    let tr = |s: &str| ctx.tr(s);
    let gs_id = global_styles_id(ctx).await;
    let preloaded = preload(ctx, post, info, gs_id).await;
    let settings = editor_settings(ctx, post, info).await;
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
        head_early: "\t<script>\n\t\tif ( typeof performance !== 'undefined' && performance.navigation && performance.navigation.type === 2 ) {\n\t\t\tdocument.location.reload( true );\n\t\t}\n\t</script>\n".into(),
        ..Default::default()
    };
    super::media_ui::enqueue(ctx, &mut assets, post.id).await;
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
        "( function() {{\n\twindow._wpLoadBlockEditor = new Promise( function( resolve ) {{\n\t\twp.domReady( function() {{\n\t\t\tresolve( wp.editPost.initializeEditor( 'editor', {}, {}, {}, {} ) );\n\t\t}} );\n\t}} );\n}} )();\nwindow._wpLoadBlockEditor.then( function() {{\n\t\twp.data.dispatch( 'core/edit-post' ).setAvailableMetaBoxesPerLocation( {{\"side\":[],\"normal\":[],\"advanced\":[]}} );\n\t}} );",
        json!(post.post_type),
        post.id,
        inline_json(&settings),
        inline_json(&initial_edits)
    );
    shared_inline(ctx, &mut assets, &preloaded).await;
    assets.inline.push(("wp-edit-post".into(), init, "after".into()));
    assets.inline.push(("wp-editor".into(), format!("var _wpMetaBoxUrl = {};", json!(format!("{}?post={}&action=edit&meta-box-loader=1&meta-box-loader-nonce={}", ctx.admin_url("post.php"), post.id, ctx.create_nonce("meta-box-loader")))), "before".into()));

    let classic_link = esc_url(&ctx.admin_url(&format!("update.php?action=install-plugin&plugin=classic-editor&_wpnonce={}", ctx.create_nonce("install-plugin_classic-editor")))).replace("&#038;", "&amp;");
    let content = format!(
        "
<div class=\"block-editor\">\n\t<h1 class=\"screen-reader-text hide-if-no-js\">{heading}</h1>\n\t<div id=\"editor\" class=\"block-editor__container hide-if-no-js\"></div>\n\t<div id=\"metaboxes\" class=\"hidden\">\n\t\t\t<form class=\"metabox-base-form\">\n\t{nonce_field}\t<input type=\"hidden\" id=\"user-id\" name=\"user_ID\" value=\"{user}\" />\n\t<input type=\"hidden\" id=\"hiddenaction\" name=\"action\" value=\"editpost\" />\n\t<input type=\"hidden\" id=\"originalaction\" name=\"originalaction\" value=\"editpost\" />\n\t<input type=\"hidden\" id=\"post_type\" name=\"post_type\" value=\"{pt}\" />\n\t<input type=\"hidden\" id=\"original_post_status\" name=\"original_post_status\" value=\"{status}\" />\n\t<input type=\"hidden\" id=\"referredby\" name=\"referredby\" value=\"\" />\n\n\t<input type=\"hidden\" name=\"_wp_original_http_referer\" value=\"\" />{auto_draft}<input type='hidden' id='post_ID' name='post_ID' value='{id}' /><input type=\"hidden\" id=\"meta-box-order-nonce\" name=\"meta-box-order-nonce\" value=\"{order_nonce}\" /><input type=\"hidden\" id=\"closedpostboxesnonce\" name=\"closedpostboxesnonce\" value=\"{closed_nonce}\" /><input type=\"hidden\" id=\"samplepermalinknonce\" name=\"samplepermalinknonce\" value=\"{sample_nonce}\" />\t</form>\n\t<form id=\"toggle-custom-fields-form\" method=\"post\" action=\"{post_php}\">\n\t\t<input type=\"hidden\" id=\"toggle-custom-fields-nonce\" name=\"toggle-custom-fields-nonce\" value=\"{toggle_nonce}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{referer}\" />\t\t<input type=\"hidden\" name=\"action\" value=\"toggle-custom-fields\" />\n\t</form>\n\t\t\t<form class=\"metabox-location-side\" onsubmit=\"return false;\">\n\t\t\t<div id=\"poststuff\" class=\"sidebar-open\">\n\t\t\t\t<div id=\"postbox-container-2\" class=\"postbox-container\">\n\t\t\t\t\t<div id=\"side-sortables\" class=\"meta-box-sortables\"></div>\t\t\t\t</div>\n\t\t\t</div>\n\t\t</form>\n\t\t\t<form class=\"metabox-location-normal\" onsubmit=\"return false;\">\n\t\t\t<div id=\"poststuff\" class=\"sidebar-open\">\n\t\t\t\t<div id=\"postbox-container-2\" class=\"postbox-container\">\n\t\t\t\t\t<div id=\"normal-sortables\" class=\"meta-box-sortables\"></div>\t\t\t\t</div>\n\t\t\t</div>\n\t\t</form>\n\t\t\t<form class=\"metabox-location-advanced\" onsubmit=\"return false;\">\n\t\t\t<div id=\"poststuff\" class=\"sidebar-open\">\n\t\t\t\t<div id=\"postbox-container-2\" class=\"postbox-container\">\n\t\t\t\t\t<div id=\"advanced-sortables\" class=\"meta-box-sortables\"></div>\t\t\t\t</div>\n\t\t\t</div>\n\t\t</form>\n\t\t\t</div>\n\n\t\t<div class=\"wrap hide-if-js block-editor-no-js\">\n\t\t<h1 class=\"wp-heading-inline\">{heading}</h1>\n\t\t<div class=\"notice notice-error\"><p>{no_js}</p></div>\t</div>\n</div>\n",
        heading = esc_html(&heading),
        nonce_field = ctx.nonce_field(&format!("update-post_{}", post.id)),
        user = ctx.user.id,
        pt = esc_attr(&post.post_type),
        status = esc_attr(&post.post_status),
        id = post.id,
        auto_draft = if is_new { "<input type='hidden' id='auto_draft' name='auto_draft' value='1' />" } else { "" },
        order_nonce = ctx.create_nonce("meta-box-order"),
        closed_nonce = ctx.create_nonce("closedpostboxes"),
        sample_nonce = ctx.create_nonce("samplepermalink"),
        toggle_nonce = ctx.create_nonce("toggle-custom-fields"),
        post_php = esc_url(&ctx.admin_url("post.php")),
        referer = esc_url(&ctx.request.request_uri),
        no_js = sprintf(&tr("The block editor requires JavaScript. Please enable JavaScript in your browser settings, or install the <a href=\"%s\">Classic Editor plugin</a>."), &[&classic_link]),
    );
    let title = if is_new { heading.clone() } else { sprintf(&tr("Edit %s &#8220;%s&#8221;"), &[&info.singular, &esc_html(&post.post_title)]) };
    let view = if post.post_status == "auto-draft" { None } else { site.permalink(post).await.ok().map(|url| (sprintf(&tr("View %s"), &[&info.singular]), url)) };
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
        fullscreen: true,
        body_classes: vec!["block-editor-page".into(), "wp-embed-responsive".into()],
        ..Default::default()
    };
    let _ = env;
    render(ctx, &screen, &content, &assets).await
}
