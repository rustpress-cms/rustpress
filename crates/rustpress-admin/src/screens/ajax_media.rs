//! The Ajax actions of the media library and the JSON of `wp_prepare_attachment_for_js()`.

use rustpress_core::OrLog;
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
    attachment_js_with(ctx, post, &[]).await
}

/// [`attachment_js`] with more size names than the chooser has (`image_size_names_choose`, as the site icon adds them).
pub async fn attachment_js_with(ctx: &Ctx<'_>, post: &Post, extra_sizes: &[String]) -> Option<Json> {
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
    let can_edit = ctx.user.can_edit_post(post);
    let can_delete = ctx.user.can_delete_post(post);
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
    let mut nonces = json!({"update": false, "delete": false, "edit": false});
    if can_edit {
        nonces["update"] = json!(nonce("update-post"));
        nonces["edit"] = json!(ctx.create_nonce(&format!("image_editor-{}", post.id)));
    }
    if can_delete {
        nonces["delete"] = json!(nonce("delete-post"));
    }
    response.insert("nonces".into(), nonces);
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
    if db.user(post.post_author).await.ok().flatten().is_some() {
        // `get_edit_user_link()`
        let link = if post.post_author == ctx.user.id {
            ctx.admin_url("profile.php")
        } else if ctx.can("edit_users") {
            ctx.admin_url(&format!("user-edit.php?user_id={}", post.post_author))
        } else {
            String::new()
        };
        response.insert("authorLink".into(), json!(link));
    }
    if post.post_parent != 0
        && let Some(parent) = db.post(post.post_parent).await.ok().flatten()
        && ctx.user.can_read_post(&parent)
    {
        // `get_edit_post_link()` is null without `edit_post`.
        response.insert("uploadedToLink".into(), if ctx.user.can_edit_post(&parent) { json!(ctx.admin_url(&format!("post.php?post={}&action=edit", parent.id))) } else { Json::Null });
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
            names.extend(extra_sizes.iter().cloned());
            if let Some(original) = m.0.get("original_image").and_then(Json::as_str).filter(|o| !o.is_empty()) {
                let dir = url.rsplit_once('/').map_or("", |(d, _)| d);
                response.insert("originalImageURL".into(), json!(format!("{dir}/{original}")));
                response.insert("originalImageName".into(), json!(original));
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
        if (kind == "video" || kind == "audio")
            && let Some(m) = &stored_meta {
                if let Some(w) = m.get("width") {
                    response.insert("width".into(), w.clone());
                }
                if let Some(h) = m.get("height") {
                    response.insert("height".into(), h.clone());
                }
            }
        let (w, h) = (48, 64);
        response.insert("image".into(), json!({"src": icon, "width": w, "height": h}));
        response.insert("thumb".into(), json!({"src": icon, "width": w, "height": h}));
    }
    response.insert("compat".into(), json!({"item": "", "meta": ""}));
    // `get_media_states()`
    let mut states: Vec<String> = Vec::new();
    if site.options.u64("site_icon", 0) == post.id {
        states.push(ctx.tr("Site Icon"));
    }
    if site.options.u64("site_logo", 0) == post.id {
        states.push(ctx.tr("Logo"));
    }
    if !states.is_empty() {
        response.insert("mediaStates".into(), json!(states.join(", ")));
    }
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
fn attachment_query(request: &AdminRequest) -> AttachmentQuery {
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
    query.offset = (paged - 1).saturating_mul(query.limit);
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
        if key.starts_with("query[post__in]")
            && let Ok(id) = value.parse() {
                query.ids.push(id);
            }
        if key.starts_with("query[post__not_in]")
            && let Ok(id) = value.parse() {
                query.exclude_ids.push(id);
            }
    }
    if let Some(author) = q("author").and_then(|v| v.parse::<u64>().ok()) {
        query.author = Some(author);
    }
    query
}

pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, action: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let tr = |t: &str| env.site.tr(t);
    let nonce_ok = |name: &str, nonce_action: &str| request.request(name).is_some_and(|n| env.auth.verify_nonce(n, nonce_action, user.id, token, env.now).is_some());
    if matches!(action, "upload-attachment" | "async-upload") && !nonce_ok("_wpnonce", "media-form") {
        return text(403, "-1");
    }
    // Without `upload_files` the uploads explain themselves, the library answers a bare error, and the crop checks
    // its nonce first (and then `edit_post`).
    if !ctx.can("upload_files") {
        match action {
            "upload-attachment" | "async-upload" => return failure(json!({"message": tr("Sorry, you are not allowed to upload files."), "filename": ""})),
            "crop-image" => {}
            _ => return send_json(200, &json!({"success": false})),
        }
    }
    match action {
        "query-attachments" => {
            let query = attachment_query(request);
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
            let Some(post) = env.db.post(id).await.ok().flatten().filter(|p| user.can_edit_post(p)) else { return send_json(200, &json!({"success": false})) };
            if post.post_type != "attachment" {
                return send_json(200, &json!({"success": false}));
            }
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
                writer.update_post_meta(id, "_wp_attachment_image_alt", &crate::sanitize::text_field(&alt)).await.or_log();
            }
            let Some(updated) = env.db.post(id).await.ok().flatten() else { return send_json(200, &json!({"success": false})) };
            let _ = headers("");
            success(attachment_js(&ctx, &updated).await.unwrap_or(Json::Null))
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
        "set-attachment-thumbnail" => {
            if request.request("thumbnail_id").is_none_or(|v| v.is_empty() || v == "0") || request.request("id").is_none_or(|v| v.is_empty() || v == "0") {
                return send_json(200, &json!({"success": false}));
            }
            text(200, "0")
        }
        "crop-image" => crop_image(&ctx).await,
        _ => text(400, "0"),
    }
}

/// `wp_ajax_crop_image()`: the cropped copy of an image becomes a new attachment (a site icon, a header).
async fn crop_image(ctx: &Ctx<'_>) -> AdminResponse {
    let (env, request, user) = (ctx.env, ctx.request, &ctx.user);
    let absint = |value: &str| value.trim().parse::<i64>().map(i64::unsigned_abs).unwrap_or(0);
    let id = request.request("id").map(absint).unwrap_or(0);
    if !request.request("nonce").is_some_and(|n| env.auth.verify_nonce(n, &format!("image_editor-{id}"), user.id, &ctx.token, env.now).is_some()) {
        return text(403, "-1");
    }
    let Some(parent) = env.db.post(id).await.ok().flatten().filter(|p| p.post_type == "attachment") else { return send_json(200, &json!({"success": false})) };
    if id == 0 || !user.can_edit_post(&parent) {
        return send_json(200, &json!({"success": false}));
    }
    let context = request.request("context").unwrap_or_default().replace('_', "-");
    let detail = |name: &str| request.request(&format!("cropDetails[{name}]")).map(absint).unwrap_or(0) as u32;
    let uploads = uploads_dir(env);
    let meta = env.db.post_meta(id).await.unwrap_or_default();
    let meta_value = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
    let attached = meta_value("_wp_attached_file").unwrap_or_default();
    let failure_message = || send_json(200, &json!({"success": false, "data": {"message": ctx.tr("Image could not be processed.")}}));
    let source = uploads.join(&attached);
    let (x, y, width, height, dst_width, dst_height) = (detail("x1"), detail("y1"), detail("width"), detail("height"), detail("dst_width"), detail("dst_height"));
    let mime = parent.post_mime_type.clone();
    let cropped = rustpress_content::media::blocking(move || rustpress_content::media::crop_image(&source, x, y, width, height, dst_width, dst_height, &mime)).await;
    let Ok(cropped) = cropped else { return failure_message() };
    let Some(writer) = env.writer else { return text(500, "0") };
    // A site icon that already is one needs no copy.
    if meta_value("_wp_attachment_context").as_deref() == Some(context.as_str()) && context == "site-icon" {
        let _ = std::fs::remove_file(&cropped);
        return success(attachment_js(ctx, &parent).await.unwrap_or(Json::Null));
    }
    // `wp_copy_parent_attachment_properties()`
    let basename = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
    let cropped_name = cropped.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let parent_url = rustpress_blocks::images::attachment_url(env.site, &attached);
    let url = parent_url.replace(&basename(&parent_url), &cropped_name);
    let sanitized_title = crate::sanitize::file_name(&parent.post_title);
    let parent_basename = basename(&parent_url);
    let parent_stem = parent_basename.rsplit_once('.').map_or(parent_basename.clone(), |(a, _)| a.to_owned());
    let use_original_title = !parent.post_title.trim().is_empty() && parent_basename != sanitized_title && parent_stem != sanitized_title;
    let relative = match cropped.strip_prefix(&uploads) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => cropped_name.clone(),
    };
    let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
    let data = PostData {
        post_title: Some(if use_original_title { parent.post_title.clone() } else { cropped_name.clone() }),
        post_content: Some(if parent.post_content.trim().is_empty() { url.clone() } else { parent.post_content.clone() }),
        post_excerpt: (!parent.post_excerpt.trim().is_empty()).then(|| parent.post_excerpt.clone()),
        post_status: Some("inherit".into()),
        post_type: Some("attachment".into()),
        post_mime_type: Some(parent.post_mime_type.clone()),
        post_parent: Some(id),
        guid: Some(url),
        post_author: Some(user.id),
        comment_status: Some(env.site.options.string("default_comment_status", "closed")),
        ping_status: Some("closed".into()),
        ..Default::default()
    };
    let Ok(new_id) = content.insert_post(&data).await else { return failure_message() };
    let extra = if context == "site-icon" { rustpress_content::media::site_icon_sizes() } else { Vec::new() };
    let generated = {
        let (options, registered, extra, uploads, relative, mime) = (env.site.options.clone(), context != "site-icon", extra.clone(), uploads.clone(), relative.clone(), parent.post_mime_type.clone());
        rustpress_content::media::blocking(move || rustpress_content::media::generate_image_metadata_with(&options, registered, &extra, &uploads, &relative, &mime)).await
    };
    let (metadata, final_relative) = match generated {
        Some((meta, final_relative, _)) => (meta, final_relative),
        None => return failure_message(),
    };
    writer.update_post_meta(new_id, "_wp_attached_file", &final_relative).await.or_log();
    writer.update_post_meta(new_id, "_wp_attachment_metadata", &String::from_utf8_lossy(&rustpress_phpser::serialize(&metadata))).await.or_log();
    if !context.is_empty() {
        writer.update_post_meta(new_id, "_wp_attachment_context", &context).await.or_log();
    }
    if let Some(alt) = meta_value("_wp_attachment_image_alt").filter(|a| !a.trim().is_empty()) {
        writer.update_post_meta(new_id, "_wp_attachment_image_alt", &alt).await.or_log();
    }
    let Some(created) = env.db.post(new_id).await.ok().flatten() else { return failure_message() };
    let extra_names: Vec<String> = if context == "site-icon" { extra.iter().map(|(name, ..)| name.clone()).collect() } else { Vec::new() };
    success(attachment_js_with(ctx, &created, &extra_names).await.unwrap_or(Json::Null))
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
