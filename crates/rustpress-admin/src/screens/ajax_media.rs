//! The Ajax actions of the media library and the JSON of `wp_prepare_attachment_for_js()`.

use rustpress_auth::CurrentUser;
use rustpress_core::options::php_to_json;
use rustpress_db::{AttachmentQuery, Post};
use serde_json::{Map, Value as Json, json};

use super::ajax::{headers, json as send_json, text};
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};
use crate::content::{Content, PostData};
use crate::media;

/// `size_format()`.
pub fn size_format(bytes: u64) -> String {
    let units = [("GB", 1u64 << 30), ("MB", 1 << 20), ("KB", 1 << 10)];
    for (unit, size) in units {
        if bytes >= size {
            return format!("{} {unit}", (bytes as f64 / size as f64).round() as u64);
        }
    }
    format!("{bytes} B")
}

/// `wp_mime_type_icon()` for non-images.
pub fn mime_icon(mime: &str) -> &'static str {
    let (kind, sub) = mime.split_once('/').unwrap_or((mime, ""));
    match kind {
        "audio" => "audio.svg",
        "video" => "video.svg",
        "text" => "text.svg",
        "application" => {
            if sub.contains("pdf") || sub.contains("word") || sub.contains("rtf") || sub.contains("opendocument.text") || sub.contains("pages") {
                "document.svg"
            } else if sub.contains("excel") || sub.contains("spreadsheet") || sub.contains("numbers") {
                "spreadsheet.svg"
            } else if sub.contains("zip") || sub.contains("tar") || sub.contains("rar") || sub.contains("7z") || sub.contains("gzip") {
                "archive.svg"
            } else if sub.contains("javascript") || sub.contains("java") {
                "code.svg"
            } else if sub.contains("powerpoint") || sub.contains("presentation") || sub.contains("keynote") {
                "interactive.svg"
            } else {
                "default.svg"
            }
        }
        _ => "default.svg",
    }
}

fn orientation(width: u64, height: u64) -> &'static str {
    if height > width { "portrait" } else { "landscape" }
}

/// `wp_prepare_attachment_for_js()`.
pub async fn attachment_js(ctx: &Ctx<'_>, post: &Post) -> Option<Json> {
    if post.post_type != "attachment" {
        return None;
    }
    let site = ctx.site();
    let db = ctx.env.db;
    let meta_rows = db.post_meta(post.id).await.ok()?;
    let meta = |key: &str| meta_rows.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
    let attached = meta("_wp_attached_file").unwrap_or_default();
    let metadata = meta("_wp_attachment_metadata").and_then(|raw| rustpress_blocks::images::Metadata::parse(&raw));
    let url = rustpress_blocks::images::attachment_url(site, &attached);
    let (kind, subtype) = post.post_mime_type.split_once('/').map_or((post.post_mime_type.clone(), String::new()), |(a, b)| (a.to_owned(), b.to_owned()));
    let timestamp = |gmt: &str| rustpress_core::datetime::SiteTimezone::gmt_timestamp(gmt).unwrap_or(0) * 1000;
    let author_name = match db.user(post.post_author).await.ok().flatten() {
        Some(user) => user.display_name,
        None => ctx.tr("(no author)"),
    };
    let link = site.permalink(post).await.unwrap_or_default();
    let can_edit = ctx.can("upload_files");
    let nonce = |action: &str| ctx.create_nonce(&format!("{action}_{}", post.id));
    let mut response = Map::new();
    response.insert("id".into(), json!(post.id));
    response.insert("title".into(), json!(post.post_title));
    response.insert("filename".into(), json!(attached.rsplit('/').next().unwrap_or_default()));
    response.insert("url".into(), json!(url));
    response.insert("link".into(), json!(link));
    response.insert("alt".into(), json!(meta("_wp_attachment_image_alt").unwrap_or_default()));
    response.insert("author".into(), json!(post.post_author.to_string()));
    response.insert("description".into(), json!(post.post_content));
    response.insert("caption".into(), json!(post.post_excerpt));
    response.insert("name".into(), json!(post.post_name));
    response.insert("status".into(), json!(post.post_status));
    response.insert("uploadedTo".into(), json!(post.post_parent));
    response.insert("date".into(), json!(timestamp(&post.post_date_gmt)));
    response.insert("modified".into(), json!(timestamp(&post.post_modified_gmt)));
    response.insert("menuOrder".into(), json!(post.menu_order));
    response.insert("mime".into(), json!(post.post_mime_type));
    response.insert("type".into(), json!(kind));
    response.insert("subtype".into(), json!(subtype));
    response.insert("icon".into(), json!(format!("{}/wp-includes/images/media/{}", site.settings.siteurl.trim_end_matches('/'), mime_icon(&post.post_mime_type))));
    let date_ts = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date_gmt).unwrap_or(0);
    response.insert("dateFormatted".into(), json!(site.date(&site.options.string("date_format", "F j, Y"), date_ts)));
    response.insert("nonces".into(), json!({"update": false, "delete": false, "edit": false}));
    if can_edit {
        response.insert("nonces".into(), json!({"update": nonce("update-post"), "delete": nonce("delete-post"), "edit": nonce("edit-post")}));
    }
    response.insert("editLink".into(), if can_edit { json!(ctx.admin_url(&format!("post.php?post={}&action=edit", post.id))) } else { json!(false) });
    // Audio and video carry their tags.
    let stored_meta = meta("_wp_attachment_metadata").map(|raw| php_to_json(&rustpress_core::options::maybe_unserialize(&raw)));
    if kind == "audio" || kind == "video" {
        let get = |key: &str| stored_meta.as_ref().and_then(|m| m.get(key)).cloned().unwrap_or(Json::Bool(false));
        if kind == "audio" {
            response.insert("meta".into(), json!({"genre": get("genre"), "year": get("year"), "length_formatted": get("length_formatted"), "artist": get("artist"), "album": get("album"), "bitrate": get("bitrate"), "bitrate_mode": get("bitrate_mode")}));
        } else {
            response.insert("meta".into(), json!({"artist": get("artist"), "album": get("album"), "bitrate": get("bitrate"), "bitrate_mode": get("bitrate_mode")}));
        }
    } else {
        response.insert("meta".into(), Json::Bool(false));
    }
    response.insert("authorName".into(), json!(author_name));
    if post.post_parent != 0
        && let Some(parent) = db.post(post.post_parent).await.ok().flatten()
    {
        response.insert("uploadedToLink".into(), if ctx.can("edit_posts") { json!(ctx.admin_url(&format!("post.php?post={}&action=edit", parent.id))) } else { json!("") });
        response.insert("uploadedToTitle".into(), json!(if parent.post_title.is_empty() { ctx.tr("(no title)") } else { parent.post_title.clone() }));
    }
    let filesize = stored_meta.as_ref().and_then(|m| m.get("filesize")).and_then(Json::as_u64).unwrap_or_else(|| {
        let uploads = uploads_dir(ctx.env);
        std::fs::metadata(uploads.join(&attached)).map(|m| m.len()).unwrap_or(0)
    });
    response.insert("filesizeInBytes".into(), json!(filesize));
    response.insert("filesizeHumanReadable".into(), json!(size_format(filesize)));
    response.insert("context".into(), json!(meta("_wp_attachment_context").unwrap_or_default()));
    if kind == "image" {
        if let Some(m) = &metadata {
            let (w, h) = (m.width(), m.height());
            let mut sizes = Map::new();
            let mut names: Vec<String> = ["thumbnail", "medium", "large"].iter().map(|s| (*s).to_owned()).collect();
            if let Some(stored) = m.sizes() {
                for key in stored.keys() {
                    if !names.contains(key) {
                        names.push(key.clone());
                    }
                }
            }
            for name in names {
                let down = rustpress_blocks::images::image_downsize(&url, m, &name);
                if down.url != url {
                    sizes.insert(name, json!({"height": down.height, "width": down.width, "url": down.url, "orientation": orientation(down.width, down.height)}));
                }
            }
            sizes.insert("full".into(), json!({"url": url, "height": h, "width": w, "orientation": orientation(w, h)}));
            response.insert("height".into(), json!(h));
            response.insert("width".into(), json!(w));
            response.insert("orientation".into(), json!(orientation(w, h)));
            response.insert("sizes".into(), Json::Object(sizes));
        }
    } else {
        let icon = format!("{}/wp-includes/images/media/{}", site.settings.siteurl.trim_end_matches('/'), mime_icon(&post.post_mime_type));
        if kind == "video" || kind == "audio" {
            if let Some(m) = &stored_meta {
                if let Some(w) = m.get("width") {
                    response.insert("width".into(), w.clone());
                }
                if let Some(h) = m.get("height") {
                    response.insert("height".into(), h.clone());
                }
            }
        }
        let (w, h) = if matches!(mime_icon(&post.post_mime_type), "default.svg") { (48, 64) } else { (48, 64) };
        response.insert("image".into(), json!({"src": icon, "width": w, "height": h}));
        response.insert("thumb".into(), json!({"src": icon, "width": w, "height": h}));
    }
    response.insert("compat".into(), json!({"item": "", "meta": ""}));
    Some(Json::Object(response))
}

pub fn uploads_dir(env: &AdminEnv<'_>) -> std::path::PathBuf {
    env.loader.includes.parent().map(|root| root.join("wp-content").join("uploads")).unwrap_or_default()
}

fn success(value: Json) -> AdminResponse {
    send_json(200, &json!({"success": true, "data": value}))
}

fn failure(value: Json) -> AdminResponse {
    send_json(200, &json!({"success": false, "data": value}))
}

/// The attachment query of `query-attachments`: `query[...]` fields of the form.
fn attachment_query(request: &AdminRequest, user: &CurrentUser) -> AttachmentQuery {
    let q = |name: &str| request.post_field(&format!("query[{name}]")).filter(|v| !v.is_empty());
    let mut query = AttachmentQuery { orderby: "date".into(), desc: true, limit: 40, ..Default::default() };
    if let Some(mime) = q("post_mime_type") {
        query.mime = Some(mime.to_owned());
    }
    if let Some(search) = q("s") {
        query.search = Some(search.to_owned());
    }
    if let Some(order) = q("order") {
        query.desc = !order.eq_ignore_ascii_case("asc");
    }
    if let Some(orderby) = q("orderby") {
        query.orderby = match orderby {
            "title" => "title".into(),
            "ID" | "id" => "ID".into(),
            "modified" => "modified".into(),
            _ => "date".into(),
        };
    }
    if let Some(per_page) = q("posts_per_page").and_then(|v| v.parse::<u64>().ok()) {
        query.limit = if per_page == 0 { 40 } else { per_page.min(100) };
    }
    let paged = q("paged").and_then(|v| v.parse::<u64>().ok()).unwrap_or(1).max(1);
    query.offset = (paged - 1) * query.limit;
    if let Some(parent) = q("post_parent").and_then(|v| v.parse::<u64>().ok()) {
        query.parent = Some(parent);
    }
    if let Some(year) = q("year").and_then(|v| v.parse().ok()) {
        query.year = Some(year);
    }
    if let Some(month) = q("monthnum").and_then(|v| v.parse().ok()) {
        query.month = Some(month);
    }
    if let Some(status) = q("post_status") {
        query.status = Some(status.to_owned());
    }
    for (key, value) in &request.form {
        if key.starts_with("query[post__in]") {
            if let Ok(id) = value.parse() {
                query.ids.push(id);
            }
        }
        if key.starts_with("query[post__not_in]") {
            if let Ok(id) = value.parse() {
                query.exclude_ids.push(id);
            }
        }
    }
    if let Some(author) = q("author").and_then(|v| v.parse::<u64>().ok()) {
        query.author = Some(author);
    }
    if !user.can("edit_others_posts") && !user.can("upload_files") {
        query.author = Some(user.id);
    }
    query
}

pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, action: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let tr = |t: &str| env.site.tr(t);
    let nonce_ok = |name: &str, nonce_action: &str| request.request(name).is_some_and(|n| env.auth.verify_nonce(n, nonce_action, user.id, token, env.now).is_some());
    if !ctx.can("upload_files") {
        return failure(json!({"message": tr("Sorry, you are not allowed to upload files."), "filename": ""}));
    }
    match action {
        "query-attachments" => {
            let query = attachment_query(request, user);
            let limit = query.limit;
            let (posts, total) = env.db.list_attachments(&query).await.unwrap_or_default();
            let mut items = Vec::new();
            for post in &posts {
                if let Some(item) = attachment_js(&ctx, post).await {
                    items.push(item);
                }
            }
            let mut response = success(Json::Array(items));
            response.headers.push(("X-WP-Total".into(), total.to_string()));
            response.headers.push(("X-WP-TotalPages".into(), total.div_ceil(limit.max(1)).to_string()));
            response
        }
        "get-attachment" => {
            let id: u64 = request.request("id").and_then(|v| v.parse().ok()).unwrap_or(0);
            match env.db.post(id).await.ok().flatten() {
                Some(post) if post.post_type == "attachment" => match attachment_js(&ctx, &post).await {
                    Some(item) => success(item),
                    None => send_json(200, &json!({"success": false})),
                },
                _ => send_json(200, &json!({"success": false})),
            }
        }
        "save-attachment" => {
            let id: u64 = request.request("id").and_then(|v| v.parse().ok()).unwrap_or(0);
            if !nonce_ok("nonce", &format!("update-post_{id}")) {
                return send_json(200, &json!({"success": false}));
            }
            let Some(post) = env.db.post(id).await.ok().flatten() else { return send_json(200, &json!({"success": false})) };
            let Some(writer) = env.writer else { return text(500, "0") };
            let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
            let changes = |name: &str| request.request(&format!("changes[{name}]")).map(str::to_owned);
            let data = PostData {
                id: Some(id),
                post_title: changes("title"),
                post_excerpt: changes("caption"),
                post_content: changes("description"),
                ..Default::default()
            };
            if (data.post_title.is_some() || data.post_excerpt.is_some() || data.post_content.is_some()) && content.insert_post(&data).await.is_err() {
                return send_json(200, &json!({"success": false}));
            }
            if let Some(alt) = changes("alt") {
                let _ = writer.update_post_meta(id, "_wp_attachment_image_alt", &crate::sanitize::text_field(&alt)).await;
            }
            let _ = post;
            let Some(updated) = env.db.post(id).await.ok().flatten() else { return send_json(200, &json!({"success": false})) };
            let _ = headers("");
            success(attachment_js(&ctx, &updated).await.unwrap_or(Json::Null))
        }
        "delete-post" => {
            let id: u64 = request.request("id").and_then(|v| v.parse().ok()).unwrap_or(0);
            if !nonce_ok("_ajax_nonce", &format!("delete-post_{id}")) {
                return text(403, "-1");
            }
            let Some(writer) = env.writer else { return text(500, "0") };
            let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
            // Attachments are deleted for good (the media trash is off).
            match content.delete_attachment(id, uploads_dir(env)).await {
                Ok(true) => text(200, "1"),
                _ => text(200, "0"),
            }
        }
        "upload-attachment" | "async-upload" => {
            if !nonce_ok("_wpnonce", "media-form") {
                return failure(json!({"message": tr("Sorry, you are not allowed to upload files."), "filename": ""}));
            }
            let mut response = upload_response(&ctx, request).await;
            if action == "async-upload" {
                response.headers.retain(|(k, _)| k != "Content-Type");
                response.headers.push(("Content-Type".into(), "text/html; charset=UTF-8".into()));
            }
            response
        }
        "set-attachment-thumbnail" => text(200, "0"),
        _ => text(400, "0"),
    }
}

/// Stores the uploaded files and answers like `wp_ajax_upload_attachment()`.
pub async fn upload_response(ctx: &Ctx<'_>, request: &AdminRequest) -> AdminResponse {
    let env = ctx.env;
    let tr = |t: &str| env.site.tr(t);
    let Some(file) = request.files.iter().find(|f| f.field == "async-upload").or(request.files.first()) else {
        return failure(json!({"message": tr("No file was uploaded."), "filename": ""}));
    };
    let Some(writer) = env.writer else { return text(500, "0") };
    let content = Content { site: env.site, db: env.db, writer, user: &ctx.user, now: env.now };
    let post_id: u64 = request.request("post_id").and_then(|v| v.parse().ok()).unwrap_or(0);
    let uploaded = media::UploadedFile { name: file.filename.clone(), bytes: file.bytes.clone() };
    match media::handle_upload(&content, &uploads_dir(env), uploaded, post_id).await {
        Ok(id) => match env.db.post(id).await.ok().flatten() {
            Some(post) => success(attachment_js(ctx, &post).await.unwrap_or(Json::Null)),
            None => failure(json!({"message": tr("Unknown error."), "filename": file.filename})),
        },
        Err(media::UploadError(message)) => failure(json!({"message": message, "filename": file.filename})),
    }
}
