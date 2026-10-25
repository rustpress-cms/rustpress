//! `wp_enqueue_media()`: the scripts, settings and Underscore templates of the media modal and the library.

use serde_json::{Map, Value as Json, json};

use super::Ctx;
use crate::chrome::Assets;

const TEMPLATES: &str = include_str!("../data/media_templates.html");
const BAKE: &str = include_str!("../data/media_bake.json");

fn bake() -> Json {
    serde_json::from_str(BAKE).unwrap_or(Json::Null)
}

/// `wp_max_upload_size()`: what the server accepts, in bytes.
pub const MAX_UPLOAD: u64 = 64 * 1024 * 1024;

fn admin_path(ctx: &Ctx<'_>, file: &str) -> String {
    ctx.admin_url(file).strip_prefix(&ctx.env.loader.site_url).map_or_else(|| format!("/wp-admin/{file}"), str::to_owned)
}

/// The strings of `_wpMediaViewsL10n`, translated.
fn views_strings(ctx: &Ctx<'_>) -> Map<String, Json> {
    let mut out = Map::new();
    if let Some(Json::Object(views)) = bake().get("views") {
        for (key, value) in views {
            out.insert(key.clone(), value.as_str().map_or_else(|| value.clone(), |text| Json::String(ctx.tr(text))));
        }
    }
    out
}

/// The `post` part of the settings for a post being edited.
fn post_settings(ctx: &Ctx<'_>, post_id: u64) -> Json {
    if post_id == 0 {
        return json!({"id": 0});
    }
    json!({"id": post_id, "nonce": ctx.create_nonce(&format!("update-post_{post_id}"))})
}

/// Adds what `wp_enqueue_media()` and `wp_print_media_templates()` add to the page.
pub async fn enqueue(ctx: &Ctx<'_>, assets: &mut Assets, post_id: u64) {
    let site = ctx.site();
    let baked = bake();
    let ajax_path = admin_path(ctx, "admin-ajax.php");
    for handle in ["media-editor", "media-audiovideo", "mce-view", "image-edit", "imgareaselect"] {
        assets.scripts.push(handle.to_owned());
    }
    for handle in ["media-views", "wp-mediaelement", "imgareaselect"] {
        assets.styles.push(handle.to_owned());
    }
    assets.localize.push(("wp-util".into(), "_wpUtilSettings".into(), json!({"ajax": {"url": ajax_path}})));
    assets.localize.push(("media-models".into(), "_wpMediaModelsL10n".into(), json!({"settings": {"ajaxurl": ajax_path, "post": {"id": post_id}}})));
    // The uploader.
    let mut plupload = Map::new();
    if let Some(Json::Object(strings)) = baked.get("plupload_l10n") {
        for (key, value) in strings {
            plupload.insert(key.clone(), value.as_str().map_or_else(|| value.clone(), |text| Json::String(ctx.tr(text))));
        }
    }
    assets.localize.push(("wp-plupload".into(), "pluploadL10n".into(), Json::Object(plupload)));
    assets.localize.push((
        "wp-plupload".into(),
        "_wpPluploadSettings".into(),
        json!({
            "defaults": {
                "file_data_name": "async-upload",
                "url": admin_path(ctx, "async-upload.php"),
                "filters": {"max_file_size": format!("{MAX_UPLOAD}b"), "mime_types": [{"extensions": baked.get("mime_extensions").cloned().unwrap_or(Json::Null)}]},
                "multipart_params": {"action": "upload-attachment", "_wpnonce": ctx.create_nonce("media-form")},
            },
            "browser": {"mobile": false, "supported": true},
            "limitExceeded": false,
        }),
    ));
    assets.localize.push(("wp-api-request".into(), "wpApiSettings".into(), json!({"root": site.site_url("wp-json/"), "nonce": ctx.create_nonce("wp_rest"), "versionString": "wp/v2/"})));
    assets.localize.push(("mediaelement".into(), "_wpmejsSettings".into(), json!({"pluginPath": "/wp-includes/js/mediaelement/", "classPrefix": "mejs-", "stretching": "responsive", "audioShortcodeLibrary": "mediaelement", "videoShortcodeLibrary": "mediaelement"})));
    assets.localize.push(("mce-view".into(), "mceViewL10n".into(), json!({"shortcodes": ["wp_caption", "caption", "gallery", "playlist", "audio", "video", "embed"]})));

    // The settings of the views.
    let db = ctx.env.db;
    let months: Vec<Json> = db
        .attachment_months()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(year, month)| {
            let ts = chrono::NaiveDate::from_ymd_opt(year, month as u32, 1).and_then(|d| d.and_hms_opt(0, 0, 0)).map_or(0, |d| d.and_utc().timestamp());
            json!({"year": year.to_string(), "month": format!("{month:02}"), "text": rustpress_core::i18n::sprintf(&ctx.tr("%1$s %2$d"), &[&site.locale_data().month[(month as usize).saturating_sub(1).min(11)], &year.to_string()]), "_": ts})
        })
        .map(|mut m| {
            m.as_object_mut().map(|o| o.remove("_"));
            m
        })
        .collect();
    let mime_counts = db.attachment_mime_counts().await.unwrap_or_default();
    let has = |prefix: &str| mime_counts.iter().any(|(m, n)| m.starts_with(prefix) && *n > 0);
    let mut mime_types = Map::new();
    if let Some(Json::Object(types)) = baked.get("mime_types") {
        for (key, value) in types {
            mime_types.insert(key.clone(), value.as_str().map_or_else(|| value.clone(), |text| Json::String(ctx.tr(text))));
        }
    }
    let settings = json!({
        "tabs": [],
        "tabUrl": format!("{}?chromeless=1", ctx.admin_url("media-upload.php")),
        "mimeTypes": mime_types,
        "captions": true,
        "nonce": {"sendToEditor": ctx.create_nonce("media-send-to-editor"), "setAttachmentThumbnail": ctx.create_nonce("set-attachment-thumbnail")},
        "post": post_settings(ctx, post_id),
        "defaultProps": {"link": site.options.string("image_default_link_type", "none"), "align": site.options.string("image_default_align", ""), "size": site.options.string("image_default_size", "")},
        "attachmentCounts": {"audio": i32::from(has("audio/")), "video": i32::from(has("video/"))},
        "oEmbedProxyUrl": site.site_url("wp-json/oembed/1.0/proxy"),
        "embedExts": baked.get("embed_exts").cloned().unwrap_or(Json::Null),
        "embedMimes": baked.get("embed_mimes").cloned().unwrap_or(Json::Null),
        "contentWidth": Json::Null,
        "months": months,
        "mediaTrash": 0,
        "infiniteScrolling": 1,
    });
    let mut views = views_strings(ctx);
    views.insert("settings".into(), settings);
    assets.localize.push(("media-views".into(), "_wpMediaViewsL10n".into(), Json::Object(views)));
    assets.inline.push((
        "wp-api-fetch".into(),
        format!(
            "wp.apiFetch.use( wp.apiFetch.createRootURLMiddleware( {} ) );\nwp.apiFetch.nonceMiddleware = wp.apiFetch.createNonceMiddleware( {} );\nwp.apiFetch.use( wp.apiFetch.nonceMiddleware );\nwp.apiFetch.use( wp.apiFetch.mediaUploadMiddleware );\nwp.apiFetch.nonceEndpoint = {};",
            json!(site.site_url("wp-json/")),
            json!(ctx.create_nonce("wp_rest")),
            json!(site.site_url("wp-admin/admin-ajax.php?action=rest-nonce"))
        ),
        "after".into(),
    ));
    assets.pre_footer.push_str(&templates(ctx));
}

/// `wp_print_media_templates()`.
pub fn templates(ctx: &Ctx<'_>) -> String {
    let _ = ctx;
    format!("\n{TEMPLATES}")
}
