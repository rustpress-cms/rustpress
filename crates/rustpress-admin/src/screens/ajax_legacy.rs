//! The older actions of `admin-ajax.php`: the classic post screen (trash, thumbnail, custom fields), the link manager,
//! the media modal's last helpers and the embeds. Every action starts with the check WordPress makes first, so the
//! answers to a request that is not allowed are the same.

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
fn success(data: Json) -> AdminResponse {
    send_json(200, &json!({"success": true, "data": data}))
}

/// `current_user_can( 'delete_post', $id )` for posts and pages.
fn can_delete(user: &CurrentUser, post: &rustpress_db::Post) -> bool {
    let page = post.post_type == "page";
    let own = post.post_author == user.id;
    let (posts, others, published, private) = if page { ("delete_pages", "delete_others_pages", "delete_published_pages", "delete_private_pages") } else { ("delete_posts", "delete_others_posts", "delete_published_posts", "delete_private_posts") };
    user.can(if own { posts } else { others })
        && match post.post_status.as_str() {
            "publish" | "future" => user.can(published),
            "private" => user.can(private),
            _ => true,
        }
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
            let id = int(request, "id");
            if !check(&format!("delete-meta_{id}"), None) {
                return denied();
            }
            // Custom fields are shown by the classic post screen only; no field, nothing to delete.
            text(200, "1")
        }
        "add-meta" => {
            if !check("add-meta", Some("_ajax_nonce-add-meta")) {
                return denied();
            }
            let post_id = int(request, "post_id") as u64;
            let post = env.db.post(post_id).await.ok().flatten();
            if request.request("metakeyselect").is_some() || request.request("metakeyinput").is_some() {
                let allowed = post.as_ref().is_some_and(|p| user.can(if p.post_type == "page" { "edit_pages" } else { "edit_posts" }) && (p.post_author == user.id || user.can("edit_others_posts")));
                if !allowed {
                    return text(200, "-1");
                }
                if request.request("metakeyselect") == Some("#NONE#") && request.request("metakeyinput").is_none_or(str::is_empty) {
                    return text(200, "1");
                }
                return text(200, &tr("Please provide a custom field value."));
            }
            text(200, "0")
        }
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
            let allowed = post.as_ref().is_some_and(|p| user.can(if p.post_type == "page" { "edit_pages" } else { "edit_posts" }) && (p.post_author == user.id || user.can("edit_others_posts")));
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
            let allowed = post.as_ref().is_some_and(|p| user.can(if p.post_type == "page" { "edit_pages" } else { "edit_posts" }) && (p.post_author == user.id || user.can("edit_others_posts")));
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
            // The classic "Featured image" box; the block editor sets the image through the REST API.
            text(200, "0")
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
            let allowed = post.as_ref().is_some_and(|p| user.can(if p.post_type == "page" { "edit_pages" } else { "edit_posts" }) && (p.post_author == user.id || user.can("edit_others_posts")));
            if !allowed {
                return error(None);
            }
            if action == "save-attachment-order" {
                if let Some(writer) = env.writer {
                    let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
                    for (key, value) in request.form.iter().chain(request.query.iter()) {
                        let Some(attachment) = key.strip_prefix("attachments[").and_then(|r| r.strip_suffix(']')).and_then(|r| r.parse::<u64>().ok()) else { continue };
                        let Some(target) = env.db.post(attachment).await.ok().flatten().filter(|p| p.post_type == "attachment") else { continue };
                        let data = crate::content::PostData { id: Some(target.id), menu_order: value.parse().ok(), ..Default::default() };
                        let _ = content.insert_post(&data).await;
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
                env.db.post(post_id as u64).await.ok().flatten().is_some_and(|p| user.can(if p.post_type == "page" { "edit_pages" } else { "edit_posts" }) && (p.post_author == user.id || user.can("edit_others_posts")))
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
                let _ = writer.update_user_meta(user.id, "wporg_favorites", username).await;
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
            let allowed = post.as_ref().is_some_and(|p| user.can(if p.post_type == "page" { "edit_pages" } else { "edit_posts" }) && (p.post_author == user.id || user.can("edit_others_posts")));
            if !allowed {
                return text(200, "-1");
            }
            let count = env.db.count_comments_for_post_with_status(post_id, "1").await.unwrap_or(0) + env.db.count_comments_for_post_with_status(post_id, "0").await.unwrap_or(0);
            // The rows of the comment table in the box of the classic post screen belong to that screen; without them there is "no items".
            let _ = count;
            text(200, "1")
        }
        _ => text(400, "0"),
    }
}
