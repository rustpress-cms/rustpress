//! The older actions of `admin-ajax.php`: the classic post screen (trash, thumbnail, custom fields), the link manager,
//! the media modal's last helpers and the embeds. Every action starts with the check WordPress makes first, so the
//! answers to a request that is not allowed are the same.

use rustpress_core::OrLog;
use rustpress_auth::CurrentUser;
use serde_json::{Value as Json, json};

use super::ajax::{json as send_json, text};
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};
use crate::content::Content;

fn int(request: &AdminRequest, name: &str) -> i64 {
    request.request(name).map(|v| v.trim()).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0)
}

/// `wp_send_json_error( $data )`.
fn error(data: Option<Json>) -> AdminResponse {
    match data {
        Some(data) => send_json(200, &json!({"success": false, "data": data})),
        None => send_json(200, &json!({"success": false})),
    }
}

/// `wp_send_json_success( $data )`.
/// `wp_ajax_add_meta()`: a new custom field (`metakeyselect`/`metakeyinput` and `metavalue`), or a changed one
/// (`meta[<id>][key]` and `meta[<id>][value]`).
async fn add_meta(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let tr = |s: &str| ctx.tr(s);
    let nonce = request.request("_ajax_nonce-add-meta").or(request.request("_ajax_nonce")).or(request.request("_wpnonce")).unwrap_or_default();
    if env.auth.verify_nonce(nonce, "add-meta", user.id, token, env.now).is_none() {
        return text(403, "-1");
    }
    let post_id = int(request, "post_id");
    let post = if post_id > 0 { env.db.post(post_id as u64).await.ok().flatten() } else { None };
    let mut response = super::ajax_terms::AjaxResponse::new("add-meta");
    let can_edit = |post: &Option<rustpress_db::Post>| post.as_ref().is_some_and(|p| user.can_edit_post(p));
    if request.post_field("metakeyselect").is_some() || request.post_field("metakeyinput").is_some() {
        if !can_edit(&post) {
            return text(200, "-1");
        }
        let select = request.post_field("metakeyselect").unwrap_or_default().trim().to_owned();
        let input = request.post_field("metakeyinput").unwrap_or_default().trim().to_owned();
        if request.post_field("metakeyselect") == Some("#NONE#") && input.is_empty() {
            return text(200, "1");
        }
        let Some(writer) = env.writer else { return text(200, "0") };
        let Some(mut post) = post else { return text(200, "-1") };
        // An auto-draft is saved as a draft first.
        if post.post_status == "auto-draft" {
            let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
            // `gmdate()`: the time in UTC, the names of PHP (not translated).
            let utc = chrono::DateTime::from_timestamp(env.now, 0).unwrap_or_default().fixed_offset();
            let gmdate = |format: &str| rustpress_core::datetime::php_date(format, &utc, "UTC", "UTC", false);
            let title = rustpress_core::i18n::sprintf(&tr("Draft created on %1$s at %2$s"), &[&gmdate(&tr("F j, Y")), &gmdate(&tr("g:i a"))]);
            let data = crate::content::PostData { id: Some(post.id), post_status: Some("draft".into()), post_title: Some(title), post_type: Some(post.post_type.clone()), ..Default::default() };
            if content.insert_post(&data).await.is_err() {
                return text(200, "0");
            }
            post = env.db.post(post.id).await.ok().flatten().unwrap_or(post);
        }
        // `add_meta()`: the typed key wins over the chosen one.
        let key = if !input.is_empty() { input } else if select != "#NONE#" { select } else { String::new() };
        if key.is_empty() || super::meta_boxes::is_protected_meta(&key) {
            return text(200, &tr("Please provide a custom field value."));
        }
        let value = request.post_field("metavalue").unwrap_or_default().trim().to_owned();
        let Ok(meta_id) = writer.add_post_meta(post.id, &key, &value).await else { return text(200, &tr("Please provide a custom field value.")) };
        let row = super::meta_boxes::list_meta_row(&ctx, meta_id, &key, &value);
        response.add("meta", meta_id, Some(1), Ok(&row), &[("postid", post.id.to_string())]);
        return response.send();
    }
    // Changing a field: `meta[<id>][key]` and `meta[<id>][value]` of the first field in the request.
    let fields = request.form.iter().chain(request.query.iter());
    let Some(meta_id) = fields.filter_map(|(k, _)| k.strip_prefix("meta[")?.split(']').next()?.parse::<u64>().ok()).next() else { return text(200, "0") };
    let field = |name: &str| request.post_field(&format!("meta[{meta_id}][{name}]")).unwrap_or_default().to_owned();
    let (key, value) = (field("key"), field("value"));
    if key.trim().is_empty() {
        return text(200, &tr("Please provide a custom field name."));
    }
    let Some(meta) = env.db.post_meta_by_id(meta_id).await.ok().flatten() else { return text(200, "0") };
    let old_key = meta.meta_key.clone().unwrap_or_default();
    let target = env.db.post(meta.post_id).await.ok().flatten();
    if super::meta_boxes::is_protected_meta(&old_key) || super::meta_boxes::is_protected_meta(&key) || !can_edit(&target) {
        return text(200, "-1");
    }
    if meta.meta_value.as_deref() != Some(value.as_str()) || old_key != key {
        let Some(writer) = env.writer else { return text(200, "0") };
        let changed = writer.update_rows("postmeta", "meta_id", meta_id, &[("meta_key", rustpress_db::Val::Str(key.clone())), ("meta_value", rustpress_db::Val::Str(value.clone()))]).await.unwrap_or(0);
        if changed == 0 {
            return text(200, "0");
        }
    }
    let row = super::meta_boxes::list_meta_row(&ctx, meta_id, &key, &value);
    response.add_with_old("meta", meta_id, Some(meta_id), Some(0), Ok(&row), &[("postid", meta.post_id.to_string())]);
    response.send()
}

fn success(data: Json) -> AdminResponse {
    send_json(200, &json!({"success": true, "data": data}))
}

/// `current_user_can( 'delete_post', $id )` for posts and pages.
fn can_delete(user: &CurrentUser, post: &rustpress_db::Post) -> bool {
    user.can_delete_post(post)
}

pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, action: &str) -> AdminResponse {
    let tr = |s: &str| env.site.tr(s);
    // `check_ajax_referer( $action, $query_arg )`: the named field first, then `_ajax_nonce` and `_wpnonce`.
    let check = |nonce_action: &str, arg: Option<&str>| -> bool {
        let value = arg.and_then(|a| request.request(a)).or_else(|| request.request("_ajax_nonce")).or_else(|| request.request("_wpnonce")).unwrap_or_default();
        env.auth.verify_nonce(value, nonce_action, user.id, token, env.now).is_some()
    };
    let denied = || text(403, "-1");
    match action {
        // `cache_oembed()` asks other servers; RustPress does not.
        "oembed-cache" => text(200, "0"),
        "delete-link" => {
            let id = int(request, "id");
            if !check(&format!("delete-bookmark_{id}"), None) {
                return denied();
            }
            if !user.can("manage_links") {
                return text(200, "-1");
            }
            // The link manager is a plugin; there is no bookmark to delete.
            text(200, "1")
        }
        "add-link-category" => {
            if !check("add-link-category", None) {
                return denied();
            }
            if !user.can("manage_links") {
                return text(200, "-1");
            }
            super::ajax_terms::AjaxResponse::new("add-link-category").send()
        }
        "add-user" => {
            if !check("add-user", None) {
                return denied();
            }
            if !user.can("create_users") {
                return text(200, "-1");
            }
            // New users are made on `user-new.php`; this Ajax form belongs to the multisite screen.
            text(200, "0")
        }
        "delete-meta" => {
            // `wp_ajax_delete_meta()`
            let id = int(request, "id");
            if !check(&format!("delete-meta_{id}"), None) {
                return denied();
            }
            let Some(meta) = (if id > 0 { env.db.post_meta_by_id(id as u64).await.ok().flatten() } else { None }) else { return text(200, "1") };
            let key = meta.meta_key.clone().unwrap_or_default();
            // `delete_post_meta` maps to `edit_post` of the post; protected keys are never deleted here.
            let post = env.db.post(meta.post_id).await.ok().flatten();
            if super::meta_boxes::is_protected_meta(&key) || !post.as_ref().is_some_and(|p| user.can_edit_post(p)) {
                return text(200, "-1");
            }
            let deleted = match env.writer {
                Some(writer) => writer.delete_rows("postmeta", "meta_id", meta.meta_id).await.unwrap_or(0) > 0,
                None => false,
            };
            text(200, if deleted { "1" } else { "0" })
        }
        "add-meta" => return add_meta(env, request, user, token).await,
        "trash-post" | "untrash-post" | "delete-post" | "delete-page" => {
            let id = int(request, "id");
            if !check(&format!("{action}_{id}"), None) {
                return denied();
            }
            let post = if id > 0 { env.db.post(id as u64).await.ok().flatten() } else { None };
            let Some(post) = post else {
                // A post that is gone has nothing to be allowed for, for `delete-post` it is done.
                return text(200, if matches!(action, "delete-post" | "delete-page") { "1" } else { "-1" });
            };
            if !can_delete(user, &post) {
                return text(200, "-1");
            }
            let Some(writer) = env.writer else { return text(200, "0") };
            let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
            let done = match action {
                "trash-post" => content.trash_post(post.id).await,
                "untrash-post" => content.untrash_post(post.id).await,
                // Attachments go for good (the media trash is off).
                _ if post.post_type == "attachment" => content.delete_attachment(post.id, super::ajax_media::uploads_dir(env)).await,
                // `wp_delete_post()` moves to the trash first when it is on; a second delete removes.
                _ => content.delete_post(post.id, post.post_status == "trash").await,
            };
            text(200, if done.unwrap_or(false) { "1" } else { "0" })
        }
        "get-permalink" => {
            if !check("getpermalink", Some("getpermalinknonce")) {
                return denied();
            }
            let post_id = int(request, "post_id");
            if post_id == 0 {
                return text(200, "");
            }
            let post = env.db.post(post_id as u64).await.ok().flatten();
            let allowed = post.as_ref().is_some_and(|p| user.can_edit_post(p));
            match (allowed, post) {
                (true, Some(post)) => {
                    let link = env.site.permalink(&post).await.unwrap_or_default();
                    // `get_preview_post_link()`
                    let preview = rustpress_core::urls::add_query_arg(&link, "preview", "true");
                    text(200, &preview)
                }
                _ => text(200, "-1"),
            }
        }
        "set-post-thumbnail" | "get-post-thumbnail-html" => {
            let post_id = int(request, "post_id") as u64;
            let post = env.db.post(post_id).await.ok().flatten();
            let allowed = post.as_ref().is_some_and(|p| user.can_edit_post(p));
            let json_request = request.request("json").is_some_and(|v| !v.is_empty() && v != "0");
            if action == "set-post-thumbnail" {
                if !allowed {
                    return text(200, "-1");
                }
                let nonce_action = if json_request { format!("update-post_{post_id}") } else { format!("set_post_thumbnail-{post_id}") };
                if !check(&nonce_action, None) {
                    return denied();
                }
            } else {
                if !check(&format!("update-post_{post_id}"), None) {
                    return denied();
                }
                if !allowed {
                    return text(200, "-1");
                }
            }
            let Some(post) = post else { return text(200, "0") };
            let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
            // `(int) $_POST['thumbnail_id']`; -1 stands for none.
            let thumbnail_id = request.post_field("thumbnail_id").map_or(0, |v| {
                let v = v.trim();
                let (sign, digits) = v.strip_prefix('-').map_or((1, v), |d| (-1, d));
                let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
                sign * digits[..end].parse::<i64>().unwrap_or(0)
            });
            let box_html = |id: u64, image: Option<String>| super::meta_boxes::post_thumbnail_html(&ctx, post.id, id, image.as_deref());
            if action == "get-post-thumbnail-html" {
                let id = u64::try_from(thumbnail_id).unwrap_or(0);
                let image = if id != 0 { super::meta_boxes::post_thumbnail_image(&ctx, id).await } else { None };
                return success(Json::from(box_html(id, image)));
            }
            let Some(writer) = env.writer else { return text(200, "0") };
            let current = env.db.post_meta_value(post.id, "_thumbnail_id").await.ok().flatten();
            let reply = |html: String| if json_request { success(Json::from(html)) } else { text(200, &html) };
            if thumbnail_id == -1 {
                // `delete_post_thumbnail()`: true when there was one.
                if current.is_none() || writer.delete_post_meta(post.id, "_thumbnail_id").await.unwrap_or(0) == 0 {
                    return text(200, "0");
                }
                return reply(box_html(0, None));
            }
            // `set_post_thumbnail()`: an attachment with an image is stored, anything else removes the thumbnail.
            let id = u64::try_from(thumbnail_id).unwrap_or(0);
            let image = if id != 0 { super::meta_boxes::post_thumbnail_image(&ctx, id).await } else { None };
            let changed = match image.as_deref() {
                None => false,
                Some("") => current.is_some() && writer.delete_post_meta(post.id, "_thumbnail_id").await.unwrap_or(0) > 0,
                // `update_post_meta()` reports no change when the value stays the same.
                Some(_) if current.as_deref() == Some(id.to_string().as_str()) => false,
                Some(_) => writer.update_post_meta(post.id, "_thumbnail_id", &id.to_string()).await.is_ok(),
            };
            if !changed {
                return text(200, "0");
            }
            reply(box_html(id, image))
        }
        "save-attachment-compat" | "save-attachment-order" => {
            let key = if action == "save-attachment-compat" { "id" } else { "post_id" };
            let id = request.request(key).map(|v| v.trim().parse::<i64>().unwrap_or(0).unsigned_abs()).unwrap_or(0);
            if request.request(key).is_none() || id == 0 {
                return error(None);
            }
            let has_attachments = |id: Option<u64>| {
                request.form.iter().chain(request.query.iter()).any(|(k, v)| !v.is_empty() && k.starts_with("attachments[") && id.is_none_or(|id| k.starts_with(&format!("attachments[{id}]"))))
            };
            if !has_attachments(if action == "save-attachment-compat" { Some(id) } else { None }) {
                return error(None);
            }
            if !check(&format!("update-post_{id}"), Some("nonce")) {
                return denied();
            }
            let post = env.db.post(id).await.ok().flatten();
            let allowed = post.as_ref().is_some_and(|p| user.can_edit_post(p));
            if !allowed {
                return error(None);
            }
            if action == "save-attachment-order" {
                if let Some(writer) = env.writer {
                    let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
                    for (key, value) in request.form.iter().chain(request.query.iter()) {
                        let Some(attachment) = key.strip_prefix("attachments[").and_then(|r| r.strip_suffix(']')).and_then(|r| r.parse::<u64>().ok()) else { continue };
                        let Some(target) = env.db.post(attachment).await.ok().flatten().filter(|p| p.post_type == "attachment" && user.can_edit_post(p)) else { continue };
                        let data = crate::content::PostData { id: Some(target.id), menu_order: value.parse().ok(), ..Default::default() };
                        content.insert_post(&data).await.or_log();
                    }
                }
                return send_json(200, &json!({"success": true}));
            }
            let post = post.expect("checked above");
            if post.post_type != "attachment" {
                return error(None);
            }
            let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
            match super::ajax_media::attachment_js(&ctx, &post).await {
                Some(item) => success(item),
                None => error(None),
            }
        }
        "media-create-image-subsizes" => {
            if !check("media-form", None) {
                return denied();
            }
            if !user.can("upload_files") {
                return error(Some(json!({"message": tr("Sorry, you are not allowed to upload files.")})));
            }
            let id = int(request, "attachment_id");
            if id == 0 {
                return error(Some(json!({"message": tr("Upload failed. Please reload and try again.")})));
            }
            let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
            // The sizes are made when the file is uploaded.
            if request.request("_legacy_support").is_some_and(|v| !v.is_empty() && v != "0") {
                return success(json!({"id": id}));
            }
            match env.db.post(id as u64).await.ok().flatten() {
                Some(post) => match super::ajax_media::attachment_js(&ctx, &post).await {
                    Some(item) => success(item),
                    None => error(Some(json!({"message": tr("Upload failed.")}))),
                },
                None => error(Some(json!({"message": tr("Upload failed.")}))),
            }
        }
        "send-link-to-editor" | "send-attachment-to-editor" => {
            if !check("media-send-to-editor", Some("nonce")) {
                return denied();
            }
            if action == "send-link-to-editor" {
                let src = request.post_field("src").unwrap_or_default();
                if src.is_empty() {
                    return error(None);
                }
                let src = if src.contains("://") { src.to_owned() } else { format!("http://{src}") };
                let src = rustpress_core::formatting::esc_url_raw(&src);
                if src.is_empty() {
                    return error(None);
                }
                let text_of = request.post_field("link_text").unwrap_or_default().trim().to_owned();
                let name = if text_of.is_empty() { src.rsplit('/').next().unwrap_or_default().to_owned() } else { text_of };
                return success(Json::from(format!("<a href=\"{}\">{name}</a>", rustpress_core::formatting::esc_url(&src))));
            }
            let id = request.request("attachment[id]").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            match env.db.post(id).await.ok().flatten() {
                Some(post) if post.post_type == "attachment" => {
                    let url = request.request("attachment[url]").unwrap_or_default();
                    let title = request.request("attachment[post_title]").unwrap_or_default();
                    let html = if post.post_mime_type.starts_with("image") {
                        let alt = request.request("attachment[image_alt]").unwrap_or_default();
                        let src = post.guid.clone();
                        let image = format!("<img src=\"{}\" alt=\"{}\" class=\"wp-image-{id}\" />", rustpress_core::formatting::esc_url(&src), rustpress_core::formatting::esc_attr(alt));
                        if url.is_empty() { image } else { format!("<a href=\"{}\">{image}</a>", rustpress_core::formatting::esc_url(url)) }
                    } else if url.is_empty() {
                        title.to_owned()
                    } else {
                        format!("<a href=\"{}\">{title}</a>", rustpress_core::formatting::esc_url(url))
                    };
                    success(Json::from(html))
                }
                _ => error(None),
            }
        }
        "parse-embed" => {
            let shortcode = request.post_field("shortcode").unwrap_or_default();
            if shortcode.is_empty() {
                return error(None);
            }
            let post_id = int(request, "post_ID");
            let allowed = if post_id > 0 {
                env.db.post(post_id as u64).await.ok().flatten().is_some_and(|p| user.can_edit_post(&p))
            } else {
                user.can("edit_posts")
            };
            if !allowed {
                return error(None);
            }
            // Embeds are found by asking the other site; RustPress does not.
            let url = shortcode.trim_start_matches(|c| c != ']').trim_start_matches(']').split("[/embed]").next().unwrap_or_default();
            error(Some(json!({"type": "not-embeddable", "message": rustpress_core::i18n::sprintf(&tr("%s failed to embed."), &[&rustpress_core::formatting::esc_url(url)])})))
        }
        "parse-media-shortcode" => {
            let shortcode = request.post_field("shortcode").unwrap_or_default();
            if shortcode.is_empty() {
                return error(None);
            }
            // Only media shortcodes are previewed.
            let media = ["audio", "embed", "playlist", "video", "gallery"];
            let mut rest = shortcode;
            let mut found = Vec::new();
            while let Some(at) = rest.find('[') {
                let tail = &rest[at + 1..];
                let name: String = tail.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect();
                if !name.is_empty() && !tail.starts_with('/') {
                    found.push(name);
                }
                rest = tail;
            }
            if found.iter().any(|name| !media.contains(&name.as_str())) {
                return error(None);
            }
            if found.iter().any(|name| name == "embed") {
                return error(None);
            }
            error(Some(json!({"type": "no-items", "message": tr("No items found.")})))
        }
        "save-wporg-username" => {
            if !user.can("install_themes") && !user.can("install_plugins") {
                return error(None);
            }
            if !check(&format!("save_wporg_username_{}", user.id), None) {
                return denied();
            }
            let username = request.request("username").unwrap_or_default();
            if username.is_empty() {
                return error(None);
            }
            if let Some(writer) = env.writer {
                writer.update_user_meta(user.id, "wporg_favorites", username).await.or_log();
            }
            success(Json::Bool(true))
        }
        "activate-plugin" => {
            if !check("updates", None) {
                return denied();
            }
            let (name, slug, plugin) = (request.post_field("name").unwrap_or_default(), request.post_field("slug").unwrap_or_default(), request.post_field("plugin").unwrap_or_default());
            if name.is_empty() || slug.is_empty() || plugin.is_empty() {
                return error(Some(json!({"slug": "", "pluginName": "", "plugin": "", "errorCode": "no_plugin_specified", "errorMessage": tr("No plugin specified.")})));
            }
            let mut status = json!({"activate": "plugin", "slug": slug, "pluginName": name, "plugin": plugin});
            if !user.can("activate_plugins") {
                status["errorMessage"] = Json::from(tr("Sorry, you are not allowed to activate plugins on this site."));
                return error(Some(status));
            }
            let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
            match super::plugins::activate_one(&ctx, plugin).await {
                Ok(()) => success(status),
                Err(message) => {
                    status["errorMessage"] = Json::from(message);
                    error(Some(status))
                }
            }
        }
        // Without a theme that has menus or widgets there is nothing to save; the check comes first, as in WordPress.
        "add-menu-item" => {
            if !check("add-menu_item", Some("menu-settings-column-nonce")) {
                return denied();
            }
            text(200, "-1")
        }
        "menu-locations-save" => {
            if !user.can("edit_theme_options") {
                return text(200, "-1");
            }
            if !check("add-menu_item", Some("menu-settings-column-nonce")) {
                return denied();
            }
            text(200, "-1")
        }
        "menu-get-metabox" | "menu-quick-search" => text(200, if user.can("edit_theme_options") { "" } else { "-1" }),
        "widgets-order" | "save-widget" | "delete-inactive-widgets" => {
            if !check("save-sidebar-widgets", Some("savewidgets")) {
                return denied();
            }
            text(200, "-1")
        }
        // These end in a fatal error in WordPress 7.1.
        "update-widget" | "health-check-is-in-debug-mode" => AdminResponse {
            status: 500,
            headers: vec![("Content-Type".into(), "text/html; charset=UTF-8".into())],
            cookies: Vec::new(),
            body: "<p>There has been a critical error on this website.</p><p><a href=\"https://wordpress.org/documentation/article/faq-troubleshooting/\">Learn more about troubleshooting WordPress.</a></p>".into(),
            bytes: Vec::new(),
        },
        "health-check-dotorg-communication" | "health-check-background-updates" | "health-check-loopback-requests" | "wp-fullscreen-save-post" => denied(),
        "press-this-save-post" | "press-this-add-category" => error(Some(json!({"errorMessage": tr("The Press This plugin is required.")}))),
        "get-comments" => {
            if !check("get-comments", None) {
                return denied();
            }
            let post_id = int(request, "p").max(0) as u64;
            if post_id == 0 {
                return text(200, "-1");
            }
            let post = env.db.post(post_id).await.ok().flatten();
            let allowed = post.as_ref().is_some_and(|p| user.can_edit_post(p));
            if !allowed {
                return text(200, "-1");
            }
            // `WP_Post_Comments_List_Table::prepare_items()`: approved and pending, newest first, `start` and `number` of
            // the request.
            // The box lists ten (`get_per_page()`), whatever `number` fetched; no `number` fetches 10 + 8.
            const PER_PAGE: usize = 10;
            let int = |name: &str| request.request(name).and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0);
            let number = if request.request("number").is_some() { int("number") } else { (PER_PAGE + 8) as i64 };
            // `number` 0 (or less) is no limit for `get_comments()`.
            let limit = u64::try_from(number).ok().filter(|n| *n > 0).unwrap_or(u64::MAX / 2);
            let query = rustpress_db::CommentListQuery { status: "all".into(), post_id: Some(post_id), limit, offset: u64::try_from(int("start")).unwrap_or(0), desc: true, ..Default::default() };
            let mut comments = env.db.list_comments_admin(&query).await.map(|(c, _)| c).unwrap_or_default();
            comments.truncate(PER_PAGE);
            if comments.is_empty() {
                return text(200, "1");
            }
            let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
            let mut rows = String::new();
            for (index, comment) in comments.iter().enumerate() {
                rows.push_str(&super::comments::row_html(&ctx, comment, index, "all", &std::collections::HashMap::new()).await);
            }
            let mut response = super::ajax_terms::AjaxResponse::new("get-comments");
            response.add("comments", 0, Some(1), Ok(&rows), &[]);
            response.send()
        }
        _ => text(400, "0"),
    }
}
