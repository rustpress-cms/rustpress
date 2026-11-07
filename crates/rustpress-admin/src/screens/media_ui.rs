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
            // `wp_localize_script()` decoded the arrow; the message has its entity.
            let translated = |text: &str| match (key.as_str(), text.strip_prefix('\u{2190}')) {
                (_, Some(rest)) => rustpress_core::formatting::html_entity_decode(&ctx.tr(&format!("&#8592;{rest}"))),
                ("unattached" | "mine", _) => ctx.site().trx(text, "media items"),
                ("trash", _) => ctx.site().trx(text, "noun"),
                ("setFeaturedImageTitle" | "setFeaturedImage", _) => ctx.site().trx(text, "post"),
                // The ellipsis is an entity in the message, too.
                _ if text.contains('\u{2026}') => rustpress_core::formatting::html_entity_decode(&ctx.tr(&text.replace('\u{2026}', "&hellip;"))),
                _ => ctx.tr(text),
            };
            out.insert(key.clone(), value.as_str().map_or_else(|| value.clone(), |text| Json::String(translated(text))));
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
/// `get_compat_media_markup( $id, [ 'in_modal' => $in_modal ] )['item']`: the fields of `attachment_fields_to_edit`
/// in their table, after the hidden `menu_order`.
///
/// Differs from WordPress: the taxonomies of attachments in the modal are not listed (WordPress registers none).
pub async fn compat_media_markup(ctx: &Ctx<'_>, post: &rustpress_db::Post, in_modal: bool) -> String {
    let site = ctx.site();
    if site.plugins.is_empty() {
        return String::new();
    }
    let parts = ctx.plugin_parts();
    let mut plugin_ctx = ctx.plugin_context(&parts, if in_modal { "admin-ajax.php" } else { "post.php" });
    if !in_modal {
        plugin_ctx.edited_post = Some(post);
    }
    let fields = site.plugins.attachment_fields_to_edit(&plugin_ctx, post, in_modal).await.unwrap_or_else(|e| {
        tracing::error!(error = %e, "plugin attachment fields failed");
        Vec::new()
    });
    // The fields WordPress removes, and the hidden ones (`_` first).
    const UNSET: [&str; 9] = ["image-size", "align", "image_alt", "post_title", "post_excerpt", "post_content", "url", "menu_order", "image_url"];
    let mut item = String::new();
    for field in fields.iter().filter(|f| !f.id.starts_with('_') && !UNSET.contains(&f.id.as_str())) {
        let id_attr = format!("attachments-{}-{}", post.id, field.id);
        item.push_str(&format!(
            "\t\t<tr class='compat-field-{}'>\t\t\t<th scope='row' class='label'><label for='{id_attr}'><span class='alignleft'>{}</span><br class='clear' /></label></th>\n\t\t\t<td class='field'>{}</td>\n\t\t</tr>\n",
            field.id, field.label, field.html
        ));
    }
    if item.is_empty() {
        return item;
    }
    // `wp_required_field_message()`.
    let indicator = format!("<span class=\"required\">{}</span>", rustpress_core::formatting::esc_html(&site.tr("*")));
    let message = format!("<span class=\"required-field-message\">{}</span>", rustpress_core::i18n::sprintf(&site.tr("Required fields are marked %s"), &[&indicator]));
    format!(
        "<input type=\"hidden\" name=\"attachments[{}][menu_order]\" value=\"{}\" /><p class=\"media-types media-types-required-info\">{message}</p><table class=\"compat-attachment-fields\">{item}</table>",
        post.id, post.menu_order
    )
}

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
    // `array_keys( $shortcode_tags )`: those of WordPress, then the plugins' in the order they register them.
    let mut shortcodes: Vec<String> = ["wp_caption", "caption", "gallery", "playlist", "audio", "video", "embed"].iter().map(|s| (*s).to_owned()).collect();
    shortcodes.extend(ctx.site().plugins.shortcode_tags().into_iter().map(str::to_owned));
    assets.localize.push(("mce-view".into(), "mceViewL10n".into(), json!({"shortcodes": shortcodes})));

    // The settings of the views.
    let db = ctx.env.db;
    let months: Vec<Json> = db
        .attachment_months()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(year, month)| {
            let ts = chrono::NaiveDate::from_ymd_opt(year, month as u32, 1).and_then(|d| d.and_hms_opt(0, 0, 0)).map_or(0, |d| d.and_utc().timestamp());
            json!({"year": year.to_string(), "month": month.to_string(), "text": rustpress_core::i18n::sprintf(&ctx.tr("%1$s %2$d"), &[&site.locale_data().month[(month as usize).saturating_sub(1).min(11)], &year.to_string()]), "_": ts})
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
        "oEmbedProxyUrl": site.rest_url("oembed/1.0/proxy"),
        "embedExts": baked.get("embed_exts").cloned().unwrap_or(Json::Null),
        "embedMimes": baked.get("embed_mimes").cloned().unwrap_or(Json::Null),
        "contentWidth": rustpress_theme::classic::content_width(&ctx.env.theme.stylesheet),
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
            json!(site.rest_url("/")),
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
    let mut templates = TEMPLATES.to_owned();
    // The baked templates have the `crossorigin` WordPress gives audio and video under cross-origin isolation only.
    if !super::client_side_media_processing(ctx) {
        templates = templates.replace("<audio crossorigin=\"anonymous\" ", "<audio ").replace("<video crossorigin=\"anonymous\" ", "<video ");
    }
    let site = ctx.site();
    if site.options.get("wp_attachment_pages_enabled") == Some("1") {
        templates = templates.replace("¦¦View media file⟧", "¦¦View attachment page⟧");
    }
    // `size_format( $max_upload_size )`.
    let size = format!("{} MB", max_upload() / (1024 * 1024));
    templates = templates.replace("⟪%s¦2 ⟦h¦unit symbol¦MB⟧⟫", &format!("⟪%s¦{size}⟫"));
    let name = site.settings.blogname.replace('\\', "\\\\").replace('\'', "\\'");
    templates = templates.replace("<# print( 'RustPress Test' ) #>", &format!("<# print( '{name}' ) #>"));
    super::template_text::translate(site, &templates.replace("http://localhost:8080/wp-admin/", &ctx.admin_url("")))
}
