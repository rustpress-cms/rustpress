//! `wp_enqueue_media()`: the scripts, settings and Underscore templates of the media modal and the library.

use serde_json::{Map, Value as Json, json};

use super::Ctx;
use crate::chrome::Assets;

const TEMPLATES: &str = include_str!("../data/media_templates.html");
const BAKE: &str = include_str!("../data/media_bake.json");

fn bake() -> Json {
    serde_json::from_str(BAKE).unwrap_or(Json::Null)
}

/// `wp_max_upload_size()`: what the server accepts, in bytes (`RUSTPRESS_MAX_UPLOAD_SIZE`, 64 MiB unless set).
pub fn max_upload() -> u64 {
    static SIZE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *SIZE.get_or_init(|| std::env::var("RUSTPRESS_MAX_UPLOAD_SIZE").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(64 * 1024 * 1024))
}

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
async fn post_settings(ctx: &Ctx<'_>, post_id: u64) -> Json {
    if post_id == 0 {
        return json!({"id": 0});
    }
    let mut settings = json!({"id": post_id, "nonce": ctx.create_nonce(&format!("update-post_{post_id}"))});
    let Ok(Some(post)) = ctx.env.db.post(post_id).await else { return settings };
    // Audio and video files support featured images on their own (`attachment:audio`, `attachment:video`).
    let theme_support = ctx.env.block_theme || ctx.env.theme.adds_theme_support("post-thumbnails");
    let thumbnail_support = (theme_support && ctx.site().post_type_supports(&post.post_type, "thumbnail"))
        || (post.post_type == "attachment" && (post.post_mime_type.starts_with("audio/") || post.post_mime_type.starts_with("video/")));
    if thumbnail_support {
        let featured = ctx.env.db.post_meta(post_id).await.unwrap_or_default().into_iter().find(|m| m.meta_key.as_deref() == Some("_thumbnail_id")).and_then(|m| m.meta_value).filter(|v| !v.is_empty() && v != "0");
        settings["featuredImageId"] = featured.map_or(json!(-1), Json::String);
    }
    settings
}

/// Adds what `wp_enqueue_media()` and `wp_print_media_templates()` add to the page.
pub async fn enqueue(ctx: &Ctx<'_>, assets: &mut Assets, post_id: u64) {
    let site = ctx.site();
    let baked = bake();
    for handle in ["media-editor", "media-audiovideo", "mce-view", "image-edit", "imgareaselect"] {
        assets.scripts.push(handle.to_owned());
    }
    for handle in ["media-views", "wp-mediaelement", "imgareaselect"] {
        assets.styles.push(handle.to_owned());
    }
    // The uploader (`wp_plupload_default_settings()` adds its data as it is: the values keep their types);
    // `pluploadL10n` comes from the script loader.
    let plupload = json!({
        "defaults": {
            "file_data_name": "async-upload",
            "url": admin_path(ctx, "async-upload.php"),
            "filters": {"max_file_size": format!("{}b", max_upload()), "mime_types": [{"extensions": crate::media::allowed_mime_types(site, &ctx.user, false, true).iter().map(|(exts, _)| exts.replace('|', ",")).collect::<Vec<_>>().join(",")}]},
            "multipart_params": {"action": "upload-attachment", "_wpnonce": ctx.create_nonce("media-form")},
        },
        "browser": {"mobile": false, "supported": true},
        "limitExceeded": false,
    });
    let encoded = rustpress_blocks::php_json::encode_with(&plupload, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E");
    assets.raw_data.push(("wp-plupload".into(), format!("var _wpPluploadSettings = {encoded};")));
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
        "post": post_settings(ctx, post_id).await,
        "defaultProps": {"link": site.options.string("image_default_link_type", "none"), "align": site.options.string("image_default_align", ""), "size": site.options.string("image_default_size", "")},
        // The playlist filters default to showing both.
        "attachmentCounts": {"audio": 1, "video": 1},
        "oEmbedProxyUrl": site.site_url("wp-json/oembed/1.0/proxy"),
        "embedExts": baked.get("embed_exts").cloned().unwrap_or(Json::Null),
        "embedMimes": baked.get("embed_mimes").cloned().unwrap_or(Json::Null),
        "contentWidth": Json::Null,
        "months": months,
        "mediaTrash": 0,
        "infiniteScrolling": 1,
    });
    let mut views = views_strings(ctx);
    // Two labels of the post type being edited (posts without one): those of pages for hierarchical types.
    let post_type = match post_id {
        0 => None,
        id => db.post(id).await.ok().flatten().map(|p| p.post_type),
    };
    if post_type.and_then(|t| super::posts::type_info(ctx, &t)).is_some_and(|info| info.hierarchical) {
        views.insert("insertIntoPost".into(), json!(ctx.tr("Insert into page")));
        views.insert("uploadedToThisPost".into(), json!(ctx.tr("Uploaded to this page")));
    }
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
    // The templates were taken from the reference; its address is replaced by the site's.
    // `wp_print_media_templates()` leaves an empty line between two templates.
    let mut templates = TEMPLATES.replace("</script>\n\t\t<script type=\"text/html\"", "</script>\n\n\t\t<script type=\"text/html\"");
    // The baked templates have the `crossorigin` WordPress gives audio and video under cross-origin isolation only.
    if !super::client_side_media_processing(ctx) {
        templates = templates.replace("<audio crossorigin=\"anonymous\" ", "<audio ").replace("<video crossorigin=\"anonymous\" ", "<video ");
    }
    format!("\n{}\n\t", templates.replace("http://localhost:8080/wp-admin/", &ctx.admin_url("")))
}
