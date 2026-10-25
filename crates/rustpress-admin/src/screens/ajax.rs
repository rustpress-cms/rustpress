//! `wp-admin/admin-ajax.php`: the requests the administration scripts make behind the scenes.

use serde_json::{Map, Value as Json, json};

use super::{AdminEnv, AdminRequest, AdminResponse};
use crate::content::{Content, PostData};
use rustpress_auth::{CurrentUser, Scheme};

pub(super) fn headers(content_type: &str) -> Vec<(String, String)> {
    vec![
        ("Expires".into(), "Wed, 11 Jan 1984 05:00:00 GMT".into()),
        ("Cache-Control".into(), "no-cache, must-revalidate, max-age=0, no-store, private".into()),
        ("X-Robots-Tag".into(), "noindex".into()),
        ("X-Content-Type-Options".into(), "nosniff".into()),
        ("Content-Type".into(), content_type.into()),
    ]
}

/// `wp_send_json( $value )`.
pub fn json(status: u16, value: &Json) -> AdminResponse {
    AdminResponse { status, headers: headers("application/json; charset=UTF-8"), cookies: Vec::new(), body: php_json(value), bytes: Vec::new() }
}

/// `wp_json_encode()` without flags: slashes and everything outside ASCII are escaped.
pub fn php_json(value: &Json) -> String {
    let mut out = String::new();
    for c in value.to_string().chars() {
        match c {
            '/' => out.push_str("\\/"),
            c if c.is_ascii() => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out
}

/// `wp_die( $text )` of the Ajax handlers: plain text.
pub fn text(status: u16, body: &str) -> AdminResponse {
    AdminResponse { status, headers: headers("text/html; charset=UTF-8"), cookies: Vec::new(), body: body.to_owned(), bytes: Vec::new() }
}

/// The `data[...]` fields of a heartbeat request, as a tree.
fn heartbeat_data(request: &AdminRequest) -> Map<String, Json> {
    let mut root = Map::new();
    for (key, value) in &request.form {
        let Some(rest) = key.strip_prefix("data[") else { continue };
        let parts: Vec<&str> = rest.split("][").map(|p| p.trim_end_matches(']')).collect();
        let mut node = &mut root;
        for (index, part) in parts.iter().enumerate() {
            if index + 1 == parts.len() {
                node.insert((*part).to_owned(), Json::String(value.clone()));
            } else {
                let entry = node.entry((*part).to_owned()).or_insert_with(|| Json::Object(Map::new()));
                if !entry.is_object() {
                    *entry = Json::Object(Map::new());
                }
                node = entry.as_object_mut().expect("object");
            }
        }
    }
    root
}

/// The user who has taken over the edit lock of a post, if somebody else holds it (`wp_check_post_lock()`).
pub(super) async fn post_lock_holder(env: &AdminEnv<'_>, user: &CurrentUser, post_id: u64) -> Option<u64> {
    let lock = env.db.post_meta_value(post_id, "_edit_lock").await.ok().flatten()?;
    let (time, holder) = lock.split_once(':')?;
    let (time, holder): (i64, u64) = (time.parse().ok()?, holder.parse().ok()?);
    (time > env.now - 150 && holder != user.id && holder != 0).then_some(holder)
}

fn str_field<'a>(map: &'a Map<String, Json>, key: &str) -> &'a str {
    map.get(key).and_then(Json::as_str).unwrap_or_default()
}

/// `wp_autosave()` through the heartbeat.
async fn autosave(env: &AdminEnv<'_>, user: &CurrentUser, token: &str, fields: &Map<String, Json>) -> Json {
    let tr = |s: &str| env.site.tr(s);
    let fail = |message: String| json!({"success": false, "message": message});
    let post_id: u64 = str_field(fields, "post_id").parse().unwrap_or(0);
    let Some(post) = env.db.post(post_id).await.ok().flatten() else { return fail(tr("Error while saving.")) };
    if env.auth.verify_nonce(str_field(fields, "_wpnonce"), &format!("update-post_{post_id}"), user.id, token, env.now).is_none() {
        return fail(tr("Error while saving."));
    }
    let Some(writer) = env.writer else { return fail(tr("Error while saving.")) };
    let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
    let may_edit = user.can(if post.post_type == "page" { "edit_pages" } else { "edit_posts" }) && (post.post_author == user.id || user.can("edit_others_posts") || user.can("edit_others_pages"));
    if !may_edit {
        return fail(tr("Sorry, you are not allowed to edit this item."));
    }
    let locked = post_lock_holder(env, user, post_id).await.is_some();
    let status = if post.post_status == "auto-draft" { "draft".to_owned() } else { post.post_status.clone() };
    let data = PostData {
        id: Some(post_id),
        post_title: fields.get("post_title").and_then(Json::as_str).map(str::to_owned),
        post_content: fields.get("content").and_then(Json::as_str).map(str::to_owned),
        post_excerpt: fields.get("excerpt").and_then(Json::as_str).map(str::to_owned),
        post_status: Some(status),
        ..Default::default()
    };
    let saved = if !locked && post.post_author == user.id && matches!(post.post_status.as_str(), "auto-draft" | "draft") {
        content.insert_post(&data).await.map(|_| ()).map_err(|e| e.to_string())
    } else {
        content.create_autosave(post_id, &data).await.map_err(|e| e.to_string())
    };
    match saved {
        Ok(()) => json!({"success": true, "message": rustpress_core::i18n::sprintf(&tr("Draft saved at %s."), &[&env.site.date(&tr("g:i:s a"), env.now)])}),
        Err(message) => fail(message),
    }
}

/// Handles a request to `admin-ajax.php`.
pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest) -> AdminResponse {
    let action = if request.file == "async-upload.php" { "async-upload".to_owned() } else { request.request("action").unwrap_or_default().to_owned() };
    let (scheme, name) = if env.https { (Scheme::SecureAuth, env.auth.cookie_name(Scheme::SecureAuth)) } else { (Scheme::Auth, env.auth.cookie_name(Scheme::Auth)) };
    let session = match request.cookie(&name) {
        Some(cookie) => env.auth.validate_cookie(env.db, cookie, scheme, env.now, false).await.ok().flatten(),
        None => None,
    };
    let Some(session) = session else {
        return if action == "heartbeat" { json(200, &json!({"nonces_expired": true})) } else { text(400, "0") };
    };
    let Ok(user) = CurrentUser::load(env.db, env.roles, &session.user).await else { return text(500, "0") };
    let nonce_ok = |nonce: &str, nonce_action: &str| env.auth.verify_nonce(nonce, nonce_action, user.id, &session.token, env.now).is_some();
    let sanitize_key = |s: &str| s.chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-')).collect::<String>();
    let write_meta = |key: String, value: String| async move {
        if let Some(writer) = env.writer {
            let _ = writer.update_user_meta(user.id, &key, &value).await;
        }
    };

    match action.as_str() {
        "inline-save" => super::quick_edit::inline_save(env, request, &user, &session.token).await,
        "heartbeat" => {
            let mut response = Map::new();
            let nonce_valid = nonce_ok(request.request("_nonce").unwrap_or_default(), "heartbeat-nonce");
            if !nonce_valid {
                response.insert("nonces_expired".into(), Json::Bool(true));
                return json(200, &Json::Object(response));
            }
            let data = heartbeat_data(request);
            if let Some(Json::Object(autosave_fields)) = data.get("wp_autosave") {
                response.insert("wp_autosave".into(), autosave(env, &user, &session.token, autosave_fields).await);
            }
            if let Some(Json::Object(received)) = data.get("wp-refresh-post-lock") {
                let post_id: u64 = str_field(received, "post_id").parse().unwrap_or(0);
                if post_id != 0 && user.can("edit_posts") {
                    let mut send = Map::new();
                    match post_lock_holder(env, &user, post_id).await {
                        Some(holder) => {
                            let name = env.db.users_by_ids(&[holder]).await.ok().and_then(|u| u.into_iter().next()).map(|u| u.display_name).unwrap_or_default();
                            send.insert("lock_error".into(), json!({"name": name, "text": rustpress_core::i18n::sprintf(&env.site.tr("%s has taken over and is currently editing."), &[&name])}));
                        }
                        None => {
                            let lock = format!("{}:{}", env.now, user.id);
                            if let Some(writer) = env.writer {
                                let _ = writer.update_post_meta(post_id, "_edit_lock", &lock).await;
                            }
                            send.insert("new_lock".into(), Json::String(lock));
                        }
                    }
                    response.insert("wp-refresh-post-lock".into(), Json::Object(send));
                }
            }
            if let Some(Json::Object(received)) = data.get("wp-refresh-post-nonces") {
                let post_id: u64 = str_field(received, "post_id").parse().unwrap_or(0);
                let nonce = |a: &str| env.auth.create_nonce(a, user.id, &session.token, env.now).unwrap_or_default();
                let mut value = json!({"check": 1});
                if post_id != 0 && user.can("edit_posts") {
                    value = json!({"replace": {
                        "getpermalinknonce": nonce("getpermalink"),
                        "samplepermalinknonce": nonce("samplepermalink"),
                        "closedpostboxesnonce": nonce("closedpostboxes"),
                        "_ajax_linking_nonce": nonce("internal-linking"),
                        "_wpnonce": nonce(&format!("update-post_{post_id}")),
                    }});
                }
                response.insert("wp-refresh-post-nonces".into(), value);
            }
            response.insert("server_time".into(), json!(env.now));
            response.insert("wp-auth-check".into(), Json::Bool(true));
            json(200, &Json::Object(response))
        }
        "update-welcome-panel" => {
            if !nonce_ok(request.request("welcomepanelnonce").unwrap_or_default(), "welcome-panel-nonce") {
                return text(403, "-1");
            }
            if !user.can("edit_theme_options") {
                return text(403, "-1");
            }
            let visible = request.request("visible").is_some_and(|v| !v.is_empty() && v != "0");
            write_meta("show_welcome_panel".to_owned(), if visible { "1" } else { "0" }.to_owned()).await;
            text(200, "1")
        }
        // The feeds of the "WordPress Events and News" widget come from wordpress.org; RustPress makes no outgoing requests.
        "dashboard-widgets" => text(200, ""),
        "closed-postboxes" => {
            if !nonce_ok(request.request("closedpostboxesnonce").unwrap_or_default(), "closedpostboxes") {
                return text(403, "-1");
            }
            let page = request.request("page").unwrap_or_default().to_owned();
            if sanitize_key(&page) != page {
                return text(400, "0");
            }
            let list = |name: &str| -> Vec<String> { request.request(name).map(|v| v.split(',').filter(|s| !s.is_empty()).map(str::to_owned).collect()).unwrap_or_default() };
            let serialize = |items: &[String]| {
                let value = rustpress_phpser::Value::Array(items.iter().enumerate().map(|(i, s)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Str(s.as_bytes().to_vec()))).collect());
                String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned()
            };
            write_meta(format!("closedpostboxes_{page}"), serialize(&list("closed"))).await;
            let hidden: Vec<String> = list("hidden").into_iter().filter(|h| !["submitdiv", "linksubmitdiv", "manage-menu", "create-menu"].contains(&h.as_str())).collect();
            write_meta(format!("metaboxhidden_{page}"), serialize(&hidden)).await;
            text(200, "1")
        }
        "meta-box-order" => {
            if !nonce_ok(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), "meta-box-order") {
                return text(403, "-1");
            }
            let page = request.request("page").unwrap_or_default().to_owned();
            if sanitize_key(&page) != page {
                return text(400, "0");
            }
            let mut order: Vec<(String, String)> = Vec::new();
            for (key, value) in &request.form {
                if let Some(context) = key.strip_prefix("order[").and_then(|r| r.strip_suffix(']')) {
                    order.push((context.to_owned(), value.clone()));
                }
            }
            if !order.is_empty() {
                let value = rustpress_phpser::Value::Array(order.iter().map(|(k, v)| (rustpress_phpser::Key::Str(k.as_bytes().to_vec()), rustpress_phpser::Value::Str(v.as_bytes().to_vec()))).collect());
                write_meta(format!("meta-box-order_{page}"), String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned()).await;
            }
            if let Some(columns) = request.request("page_columns").filter(|c| !c.is_empty() && *c != "auto") {
                write_meta(format!("screen_layout_{page}"), columns.parse::<i64>().unwrap_or(2).to_string()).await;
            }
            json(200, &json!({"success": true}))
        }
        "hidden-columns" => {
            if !nonce_ok(request.request("screenoptionnonce").unwrap_or_default(), "screen-options-nonce") {
                return text(403, "-1");
            }
            let page = request.request("page").unwrap_or_default().to_owned();
            if sanitize_key(&page) != page {
                return text(400, "0");
            }
            let hidden: Vec<String> = request.request("hidden").map(|v| v.split(',').filter(|s| !s.is_empty()).map(str::to_owned).collect()).unwrap_or_default();
            let value = rustpress_phpser::Value::Array(hidden.iter().enumerate().map(|(i, s)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Str(s.as_bytes().to_vec()))).collect());
            write_meta(format!("manage{page}columnshidden"), String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned()).await;
            text(200, "1")
        }
        "add-tag" | "inline-save-tax" | "delete-tag" => super::ajax_terms::handle(env, request, &user, &session.token, &action).await,
        "dim-comment" | "delete-comment" | "replyto-comment" | "edit-comment" => super::ajax_comments::handle(env, request, &user, &session.token, &action).await,
        "query-attachments" | "get-attachment" | "save-attachment" | "delete-post" | "upload-attachment" | "async-upload" | "set-attachment-thumbnail" | "crop-image" => super::ajax_media::handle(env, request, &user, &session.token, &action).await,
        "image-editor" => super::image_edit::editor(env, request, &user, &session.token).await,
        "imgedit-preview" => super::image_edit::preview(env, request, &user, &session.token).await,
        "ajax-tag-search" | "get-tagcloud" | "wp-remove-post-lock" | "dismiss-wp-pointer" | "sample-permalink" | "find_posts" | "save-user-color-scheme" | "generate-password" | "destroy-sessions" | "autocomplete-user" | "edit-theme-plugin-file" | "delete-plugin" | "delete-theme" | "toggle-auto-updates" | "query-themes" | "install-theme" | "install-plugin" | "update-theme" | "update-plugin" | "search-plugins" | "search-install-plugins" => super::ajax_misc::handle(env, request, &user, &session.token, &action).await,
        "get-revision-diffs" => super::revision::ajax_diffs(env, request, &user, &session.token).await,
        "rest-nonce" => text(200, &env.auth.create_nonce("wp_rest", user.id, &session.token, env.now).unwrap_or_default()),
        "date_format" | "time_format" => {
            let format = rustpress_core::formatting::strip_tags(request.post_field("date").unwrap_or_default());
            text(200, &env.site.date(&format, env.now))
        }
        "wp-compression-test" | "logged-in" => text(200, "1"),
        _ => text(400, "0"),
    }
}
