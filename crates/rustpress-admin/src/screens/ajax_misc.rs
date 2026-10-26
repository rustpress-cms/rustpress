//! More actions of `admin-ajax.php`: tag search and cloud, post locks, pointers, permalink samples,
//! the "find posts" box, the colour scheme and password helpers of the profile.

use rustpress_core::OrLog;
use rustpress_auth::CurrentUser;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::slug::sanitize_title;
use rustpress_db::{PostQuery, Val};
use serde_json::json;

use super::ajax::{json, text};
use super::terms::tax_info;
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};
use crate::content::Content;

/// `sanitize_key()`.
fn sanitize_key(value: &str) -> String {
    value.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-')).collect()
}

/// `natcasesort()` order of two names (digits compare as numbers).
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut ia, mut ib) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ia.peek().copied(), ib.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(&c) = ia.peek().filter(|c| c.is_ascii_digit()) {
                    na.push(c);
                    ia.next();
                }
                let mut nb = String::new();
                while let Some(&c) = ib.peek().filter(|c| c.is_ascii_digit()) {
                    nb.push(c);
                    ib.next();
                }
                let (na, nb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let order = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                ia.next();
                ib.next();
            }
        }
    }
}

pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, action: &str) -> AdminResponse {
    if action == "health-check-get-sizes" {
        if !nonce_ok_early_for(env, request, user, token, "health-check-get-sizes") {
            return text(403, "-1");
        }
        if !user.can("view_site_health_checks") {
            return super::ajax::json(403, &json!({"success": false}));
        }
        return super::ajax::json(200, &super::site_health_info::sizes(env).await);
    }
    if action == "health-check-site-status-result" {
        if !nonce_ok_early(env, request, user, token) {
            return text(403, "-1");
        }
        if !user.can("view_site_health_checks") {
            return json(200, &json!({"success": false}));
        }
        // `set_transient( 'health-check-site-status-result', wp_json_encode( $_POST['counts'] ) )`.
        let mut counts = serde_json::Map::new();
        for (key, value) in &request.form {
            if let Some(name) = key.strip_prefix("counts[").and_then(|k| k.strip_suffix(']')) {
                counts.insert(name.to_owned(), value.parse::<i64>().map_or_else(|_| json!(value), |n| json!(n)));
            }
        }
        if let Some(writer) = env.writer {
            writer.update_option_with_autoload("_transient_health-check-site-status-result", &serde_json::Value::Object(counts).to_string(), "on").await.or_log();
        }
        return json(200, &json!({"success": true}));
    }
    if action == "edit-theme-plugin-file" {
        return super::file_editor::ajax_edit(env, request, user, token).await;
    }
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let site = env.site;
    let tr = |s: &str| site.tr(s);
    let nonce_ok = |nonce: &str, nonce_action: &str| env.auth.verify_nonce(nonce, nonce_action, user.id, token, env.now).is_some();
    let param = |name: &str| request.request(name).unwrap_or_default().to_owned();
    match action {
        "ajax-tag-search" => {
            let Some(tax) = request.get("tax") else { return text(200, "0") };
            let Some(info) = tax_info(&ctx, &sanitize_key(tax), None) else { return text(200, "0") };
            if !user.can("edit_posts") {
                return text(403, "-1");
            }
            let mut search = request.get("q").unwrap_or_default().to_owned();
            if let Some((_, last)) = search.rsplit_once(',') {
                search = last.to_owned();
            }
            let search = search.trim().to_lowercase();
            if search.chars().count() < 2 {
                return text(200, "");
            }
            let number = request.get("number").and_then(|n| n.parse::<usize>().ok()).unwrap_or(0);
            let mut names: Vec<String> = env.db.terms(&info.name).await.unwrap_or_default().into_iter().map(|t| t.name).filter(|n| n.to_lowercase().contains(&search)).collect();
            names.sort_by_key(|n| n.to_lowercase());
            if number > 0 {
                names.truncate(number);
            }
            text(200, &names.join("\n"))
        }
        "get-tagcloud" => {
            let Some(tax) = request.post_field("tax") else { return text(200, "0") };
            let Some(info) = tax_info(&ctx, &sanitize_key(tax), None) else { return text(200, "0") };
            if !user.can("edit_posts") {
                return text(403, "-1");
            }
            let all = env.db.terms(&info.name).await.unwrap_or_default();
            // `hide_empty`: a term without posts stays when one of its descendants has some.
            let has_posts = |id: u64| -> bool {
                let mut stack = vec![id];
                while let Some(current) = stack.pop() {
                    if all.iter().any(|t| t.term_id == current && t.count > 0) {
                        return true;
                    }
                    stack.extend(all.iter().filter(|t| t.parent == current).map(|t| t.term_id));
                }
                false
            };
            let mut tags: Vec<_> = all.iter().filter(|t| has_posts(t.term_id)).cloned().collect();
            tags.sort_by(|a, b| b.count.cmp(&a.count).then(a.term_id.cmp(&b.term_id)));
            tags.truncate(45);
            if tags.is_empty() {
                return text(200, &site.types.taxonomy(&info.name).map_or_else(|| tr("No tags found."), |_| tr("No items found.")));
            }
            tags.sort_by(|a, b| natural_cmp(&a.name, &b.name));
            let counts: Vec<f64> = tags.iter().map(|t| ((t.count as f64 + 1.0).log10() * 100.0).round()).collect();
            let min = counts.iter().copied().fold(f64::INFINITY, f64::min);
            let max = counts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let spread = if max - min <= 0.0 { 1.0 } else { max - min };
            let step = 14.0 / spread;
            let mut links = Vec::new();
            for (i, tag) in tags.iter().enumerate() {
                let formatted = rustpress_core::i18n::sprintf(&site.trn("%s item", "%s items", tag.count), &[&site.number_i18n(tag.count.max(0) as u64)]);
                let size = 8.0 + (counts[i] - min) * step;
                let size = rustpress_blocks::general::php_float(size);
                links.push(format!(
                    "<a href=\"#\" role=\"button\" class=\"{}\" style=\"font-size: {size}pt;\" aria-label=\"{} ({})\">{}</a>",
                    esc_attr(&format!("tag-cloud-link tag-link-{} tag-link-position-{}", tag.term_id, i + 1)),
                    esc_attr(&tag.name),
                    esc_attr(&formatted),
                    esc_html(&tag.name)
                ));
            }
            text(200, &format!("<ul class='wp-tag-cloud' role='list'>\n\t<li>{}</li>\n</ul>\n", links.join("</li>\n\t<li>")))
        }
        "wp-remove-post-lock" => {
            let post_id: u64 = request.post_field("post_ID").and_then(|p| p.parse().ok()).unwrap_or(0);
            let lock = param("active_post_lock");
            if post_id == 0 || lock.is_empty() {
                return text(200, "0");
            }
            let Some(post) = env.db.post(post_id).await.ok().flatten() else { return text(200, "0") };
            if !nonce_ok(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), &format!("update-post_{post_id}")) {
                return text(403, "-1");
            }
            let may_edit = user.can_edit_post(&post);
            if !may_edit {
                return text(403, "-1");
            }
            let holder: u64 = lock.split(':').nth(1).and_then(|h| h.parse().ok()).unwrap_or(0);
            if holder != user.id {
                return text(200, "0");
            }
            if let Some(writer) = env.writer {
                writer.update_post_meta(post_id, "_edit_lock", &format!("{}:{}", env.now - 150 + 5, holder)).await.or_log();
            }
            text(200, "1")
        }
        "dismiss-wp-pointer" => {
            let pointer = param("pointer");
            if request.request("pointer").is_none() || sanitize_key(&pointer) != pointer {
                return text(200, "0");
            }
            let current = env.db.user_meta(user.id, "dismissed_wp_pointers").await.ok().flatten().unwrap_or_default();
            let mut dismissed: Vec<String> = current.split(',').filter(|p| !p.is_empty()).map(str::to_owned).collect();
            if dismissed.contains(&pointer) {
                return text(200, "0");
            }
            dismissed.push(pointer);
            if let Some(writer) = env.writer {
                writer.update_user_meta(user.id, "dismissed_wp_pointers", &dismissed.join(",")).await.or_log();
            }
            text(200, "1")
        }
        "sample-permalink" => {
            if !nonce_ok(request.request("samplepermalinknonce").unwrap_or_default(), "samplepermalink") {
                return text(403, "-1");
            }
            let post_id: u64 = request.post_field("post_id").and_then(|p| p.parse().ok()).unwrap_or(0);
            if post_id == 0 {
                return text(200, "");
            }
            let Some(stored) = env.db.post(post_id).await.ok().flatten() else { return text(403, "-1") };
            let mut post = stored.clone();
            let may_edit = user.can_edit_post(&post);
            if !may_edit {
                return text(403, "-1");
            }
            let title = param("new_title");
            let mut status = post.post_status.clone();
            if matches!(status.as_str(), "draft" | "pending" | "future") {
                status = "publish".into();
                post.post_name = sanitize_title(if post.post_name.is_empty() { &post.post_title } else { &post.post_name });
            }
            if let Some(name) = request.post_field("new_slug") {
                post.post_name = sanitize_title(if name.is_empty() { &title } else { name });
            }
            let Some(writer) = env.writer else { return text(500, "0") };
            let content = Content { site, db: env.db, writer, user, now: env.now };
            post.post_name = content.unique_post_slug(&post.post_name, post.id, &status, &post.post_type, post.post_parent).await.unwrap_or(post.post_name.clone());
            post.post_status = "publish".into();
            let sample = site.permalink(&post).await.unwrap_or_default();
            // The address of the link is the one the post has now; drafts get their preview.
            let view_link = if matches!(stored.post_status.as_str(), "publish" | "private") { site.permalink(&stored).await.unwrap_or_default() } else { site.site_url(&format!("?p={}&preview=true", stored.id)) };
            let name = post.post_name.clone();
            let link = sample;
            if name.is_empty() {
                return text(200, &format!("<strong>{}</strong>\n<span id=\"sample-permalink\"><a href=\"{}\">{}</a></span>\n", tr("Permalink:"), esc_url(&view_link), esc_html(&link)));
            }
            let shown = link.replacen(&format!("/{name}/"), &format!("/<span id=\"editable-post-name\">{}</span>/", esc_html(&name)), 1);
            text(
                200,
                &format!(
                    "<strong>{}</strong>\n<span id=\"sample-permalink\"><a href=\"{}\">{shown}</a></span>\n&lrm;<span id=\"edit-slug-buttons\"><button type=\"button\" class=\"edit-slug button button-small hide-if-no-js\" aria-label=\"{}\">{}</button></span>\n<span id=\"editable-post-name-full\">{}</span>\n",
                    tr("Permalink:"),
                    esc_url(&view_link),
                    esc_attr(&tr("Edit permalink")),
                    tr("Edit"),
                    esc_html(&name)
                ),
            )
        }
        "find_posts" => {
            if !nonce_ok(request.request("_ajax_nonce").unwrap_or_default(), "find-posts") {
                return text(403, "-1");
            }
            let mut query = PostQuery { post_types: site.types.public_post_types().into_iter().filter(|t| *t != "attachment").map(str::to_owned).collect(), statuses: ["publish", "future", "draft", "pending", "private"].map(String::from).to_vec(), ..Default::default() };
            let phrase = param("ps");
            if !phrase.is_empty() {
                query.search = phrase.split_whitespace().map(str::to_owned).collect();
                query.search_phrase = Some(phrase.clone());
            }
            // MySQL leaves posts of the same date in the order of their IDs.
            let mut posts = env.db.posts(&query).await.unwrap_or_default();
            posts.sort_by(|a, b| b.post_date.cmp(&a.post_date).then(a.id.cmp(&b.id)));
            posts.truncate(50);
            if posts.is_empty() {
                return json(200, &json!({"success": false, "data": tr("No items found.")}));
            }
            let mut html = format!(
                "<table class=\"widefat\"><thead><tr><th class=\"found-radio\"><br /></th><th>{}</th><th class=\"no-break\">{}</th><th class=\"no-break\">{}</th><th class=\"no-break\">{}</th></tr></thead><tbody>",
                tr("Title"),
                tr("Type"),
                tr("Date"),
                tr("Status")
            );
            let mut alternate = false;
            for post in &posts {
                let title = if post.post_title.trim().is_empty() { tr("(no title)") } else { post.post_title.clone() };
                alternate = !alternate;
                let stat = match post.post_status.as_str() {
                    "publish" | "private" => tr("Published"),
                    "future" => tr("Scheduled"),
                    "pending" => tr("Pending Review"),
                    "draft" => tr("Draft"),
                    _ => String::new(),
                };
                let time = if post.post_date.starts_with("0000-00-00") {
                    String::new()
                } else {
                    let ts = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
                    site.date(&tr("Y/m/d"), ts)
                };
                let kind = super::posts::type_info(&ctx, &post.post_type).map_or_else(|| post.post_type.clone(), |t| t.singular);
                html.push_str(&format!(
                    "<tr class=\"{}\"><td class=\"found-radio\"><input type=\"radio\" id=\"found-{id}\" name=\"found_post_id\" value=\"{id}\"></td><td><label for=\"found-{id}\">{}</label></td><td class=\"no-break\">{}</td><td class=\"no-break\">{}</td><td class=\"no-break\">{} </td></tr>\n\n",
                    if alternate { "found-posts alternate" } else { "found-posts" },
                    esc_html(&title),
                    esc_html(&kind),
                    esc_html(&time),
                    esc_html(&stat),
                    id = post.id
                ));
            }
            html.push_str("</tbody></table>");
            json(200, &json!({"success": true, "data": html}))
        }
        "wp-link-ajax" => {
            // `_WP_Editors::wp_link_query()`: the published posts of the public types, 20 a page.
            if !nonce_ok(request.request("_ajax_linking_nonce").unwrap_or_default(), "internal-linking") {
                return text(403, "-1");
            }
            let phrase = request.post_field("term").or(request.post_field("search")).unwrap_or_default().to_owned();
            let page = request.post_field("page").and_then(|p| p.parse::<u64>().ok()).filter(|p| *p > 0).unwrap_or(1);
            let mut query = PostQuery { post_types: site.types.public_post_types().into_iter().map(str::to_owned).collect(), statuses: vec!["publish".into()], ..Default::default() };
            if !phrase.is_empty() {
                query.search = phrase.split_whitespace().map(str::to_owned).collect();
                query.search_phrase = Some(phrase.clone());
            }
            let mut posts = env.db.posts(&query).await.unwrap_or_default();
            posts.sort_by(|a, b| b.post_date.cmp(&a.post_date).then(a.id.cmp(&b.id)));
            let posts: Vec<_> = posts.into_iter().skip(((page - 1).saturating_mul(20)) as usize).take(20).collect();
            if posts.is_empty() {
                return text(200, "0");
            }
            let mut results = Vec::new();
            for post in &posts {
                let info = if post.post_type == "post" {
                    site.date(&tr("Y/m/d"), site.timezone.local_timestamp(&post.post_date).unwrap_or(0))
                } else {
                    super::posts::type_info(&ctx, &post.post_type).map_or_else(|| post.post_type.clone(), |t| t.singular)
                };
                let title = rustpress_blocks::content::the_title_for(site, post);
                results.push(json!({
                    "ID": post.id,
                    "title": esc_html(&rustpress_core::formatting::strip_tags(&title)).trim().to_owned(),
                    "permalink": site.permalink(post).await.unwrap_or_default(),
                    "info": info,
                }));
            }
            let mut response = text(200, &format!("{}
", super::ajax::php_json(&serde_json::Value::Array(results))));
            response.headers.retain(|(k, _)| k != "Content-Type");
            response.headers.push(("Content-Type".into(), "text/html; charset=UTF-8".into()));
            response
        }
        "get-community-events" => {
            if !nonce_ok(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), "community_events") {
                return text(403, "-1");
            }
            // RustPress makes no requests to WordPress.org, where the events come from.
            json(200, &json!({"success": false, "data": {"error": tr("RustPress does not contact WordPress.org to look up events.")}}))
        }
        "send-password-reset" => {
            let user_id = request.post_field("user_id").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            if !nonce_ok(request.request("nonce").unwrap_or_default(), &format!("reset-password-for-{user_id}")) {
                return text(403, "-1");
            }
            if !(user.id == user_id || user.can("edit_users")) {
                return json(200, &json!({"success": false, "data": tr("Cannot send password reset, permission denied.")}));
            }
            let Some(target) = env.db.user(user_id).await.ok().flatten() else { return json(200, &json!({"success": false, "data": tr("Cannot send password reset, permission denied.")})) };
            match env.mail_ctx().retrieve_password(&target.user_login, None).await {
                Ok(()) => json(200, &json!({"success": true, "data": rustpress_core::i18n::sprintf(&tr("A password reset link was emailed to %s."), &[&target.display_name])})),
                Err(errors) => json(200, &json!({"success": false, "data": errors.first().map(|e| e.1.clone()).unwrap_or_default()})),
            }
        }
        "save-user-color-scheme" => {
            if !nonce_ok(request.request("nonce").unwrap_or_default(), "save-color-scheme") {
                return text(403, "-1");
            }
            let scheme = sanitize_key(&param("color_scheme"));
            if !["fresh", "light", "modern", "blue", "coffee", "ectoplasm", "midnight", "ocean", "sunrise"].contains(&scheme.as_str()) {
                return json(200, &json!({"success": false}));
            }
            let previous = env.db.user_meta(user.id, "admin_color").await.ok().flatten().unwrap_or_default();
            if let Some(writer) = env.writer {
                writer.update_user_meta(user.id, "admin_color", &scheme).await.or_log();
            }
            json(200, &json!({"success": true, "data": {"previousScheme": format!("admin-color-{previous}"), "currentScheme": format!("admin-color-{scheme}")}}))
        }
        "generate-password" => json(200, &json!({"success": true, "data": crate::sanitize::random_password(24, true)})),
        "destroy-sessions" => {
            let target: u64 = param("user_id").parse().unwrap_or(0);
            let failure = || json(200, &json!({"success": false, "data": {"message": tr("Could not log out user sessions. Please try again.")}}));
            let Some(target_user) = env.db.user(target).await.ok().flatten() else { return failure() };
            if !(user.can("edit_users") || target == user.id) || !nonce_ok(&param("nonce"), &format!("update-user_{target}")) {
                return failure();
            }
            let Some(writer) = env.writer else { return failure() };
            let message = if target == user.id {
                rustpress_auth::session::destroy_other_sessions(env.db, writer, target, token, env.now).await.or_log();
                tr("You are now logged out everywhere else.")
            } else {
                rustpress_auth::session::destroy_all_sessions(writer, target).await.or_log();
                rustpress_core::i18n::sprintf(&tr("%s has been logged out."), &[&target_user.display_name])
            };
            json(200, &json!({"success": true, "data": {"message": message}}))
        }
        "delete-plugin" => {
            if !nonce_ok(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), "updates") {
                return text(403, "-1");
            }
            let (slug, plugin) = (param("slug"), param("plugin"));
            if slug.is_empty() || plugin.is_empty() {
                return json(200, &json!({"success": false, "data": {"slug": "", "errorCode": "no_plugin_specified", "errorMessage": tr("No plugin specified.")}}));
            }
            let mut status = json!({"delete": "plugin", "slug": sanitize_key(&slug)});
            let fail = |mut status: serde_json::Value, message: String| {
                status["errorMessage"] = json!(message);
                json(200, &json!({"success": false, "data": status}))
            };
            if !user.can("delete_plugins") || plugin.contains("..") || plugin.starts_with('/') || plugin.contains('\\') {
                return fail(status, tr("Sorry, you are not allowed to delete plugins for this site."));
            }
            let Some(root) = env.loader.includes.parent() else { return fail(status, tr("Plugin could not be deleted.")) };
            let found = crate::plugins::installed(root).into_iter().find(|p| p.file == plugin);
            let name = found.as_ref().map(|p| p.name.clone()).unwrap_or_default();
            status["plugin"] = json!(plugin);
            status["pluginName"] = json!(name);
            let active: Vec<String> = match site.options.value("active_plugins") {
                Some(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
                _ => Vec::new(),
            };
            if active.contains(&plugin) {
                return fail(status, tr("You cannot delete a plugin while it is active on the main site."));
            }
            if env.writer.is_none() {
                return fail(status, tr("Plugin could not be deleted."));
            }
            // Only a plugin that is installed is deleted: a path like `./x.php` passes `validate_file()` but would
            // name the whole plugin directory.
            if found.is_none() || plugin.split('/').any(|part| part.is_empty() || part == "." || part == "..") || plugin.matches('/').count() > 1 {
                return fail(status, tr("Plugin could not be deleted."));
            }
            let plugins_dir = root.join("wp-content").join("plugins");
            let removed = match plugin.split_once('/') {
                Some((folder, _)) => std::fs::remove_dir_all(plugins_dir.join(folder)),
                None => std::fs::remove_file(plugins_dir.join(&plugin)),
            };
            if removed.is_err() {
                return fail(status, tr("Plugin could not be deleted."));
            }
            json(200, &json!({"success": true, "data": status}))
        }
        "delete-theme" => {
            if !nonce_ok(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), "updates") {
                return text(403, "-1");
            }
            let slug = param("slug");
            if slug.is_empty() {
                return json(200, &json!({"success": false, "data": {"slug": "", "errorCode": "no_theme_specified", "errorMessage": tr("No theme specified.")}}));
            }
            let stylesheet: String = slug.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect();
            let status = json!({"delete": "theme", "slug": stylesheet});
            let fail = |message: String| json(200, &json!({"success": false, "data": {"delete": "theme", "slug": stylesheet, "errorMessage": message}}));
            if !user.can("delete_themes") {
                return fail(tr("Sorry, you are not allowed to delete themes on this site."));
            }
            let Some(root) = env.loader.includes.parent() else { return fail(tr("Theme could not be deleted.")) };
            if crate::themes::find(root, &stylesheet).is_none() {
                return fail(tr("The requested theme does not exist."));
            }
            if stylesheet == site.settings.stylesheet || stylesheet == site.settings.template {
                return fail(tr("Theme could not be deleted."));
            }
            if env.writer.is_none() || std::fs::remove_dir_all(root.join("wp-content").join("themes").join(&stylesheet)).is_err() {
                return fail(tr("Theme could not be deleted."));
            }
            json(200, &json!({"success": true, "data": status}))
        }
        "toggle-auto-updates" => {
            if !nonce_ok(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), "updates") {
                return text(403, "-1");
            }
            let error = |message: String| json(200, &json!({"success": false, "data": {"error": message}}));
            let (kind, asset, state) = (param("type"), param("asset"), param("state"));
            if kind.is_empty() || asset.is_empty() || state.is_empty() {
                return error(tr("Invalid data. No selected item."));
            }
            let asset = rustpress_core::formatting::strip_tags(&String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(asset.as_bytes())));
            if state != "enable" && state != "disable" {
                return error(tr("Invalid data. Unknown state."));
            }
            let Some(root) = env.loader.includes.parent() else { return error(tr("Invalid data. The item does not exist.")) };
            let (option, known): (&str, Vec<String>) = match kind.as_str() {
                "plugin" => {
                    if !user.can("update_plugins") {
                        return error(tr("Sorry, you are not allowed to modify plugins."));
                    }
                    ("auto_update_plugins", crate::plugins::installed(root).into_iter().map(|p| p.file).collect())
                }
                "theme" => {
                    if !user.can("update_themes") {
                        return error(tr("Sorry, you are not allowed to modify themes."));
                    }
                    ("auto_update_themes", crate::themes::installed(root).into_iter().map(|t| t.stylesheet).collect())
                }
                _ => return error(tr("Invalid data. Unknown type.")),
            };
            if !known.contains(&asset) {
                return error(tr("Invalid data. The item does not exist."));
            }
            let mut auto: Vec<String> = match site.options.value(option) {
                Some(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
                _ => Vec::new(),
            };
            if state == "disable" {
                auto.retain(|a| a != &asset);
            } else if !auto.contains(&asset) {
                auto.push(asset);
            }
            auto.retain(|a| known.contains(a));
            if let Some(writer) = env.writer {
                let items = auto.iter().enumerate().map(|(i, a)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Str(a.clone().into_bytes()))).collect();
                writer.update_option(option, &String::from_utf8_lossy(&rustpress_phpser::serialize(&rustpress_phpser::Value::Array(items)))).await.or_log();
            }
            json(200, &json!({"success": true}))
        }
        // Everything that needs WordPress.org: RustPress does not connect to it.
        "query-themes" | "install-theme" | "install-plugin" | "update-theme" | "update-plugin" | "search-plugins" | "search-install-plugins" => {
            if action == "query-themes" && !user.can("install_themes") {
                return json(200, &json!({"success": false}));
            }
            // `check_ajax_referer( 'updates' )`.
            if action != "query-themes" && !nonce_ok(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), "updates") {
                return text(403, "-1");
            }
            let message = tr("An unexpected error occurred. Something may be wrong with WordPress.org or this server&#8217;s configuration. If you continue to have problems, please try the <a href=\"https://wordpress.org/support/forums/\">support forums</a>.");
            json(200, &json!({"success": false, "data": {"slug": param("slug"), "errorCode": "plugins_api_failed", "errorMessage": message, "error": message}}))
        }
        // Multisite only: single sites answer -1.
        "autocomplete-user" => text(200, "-1"),
        _ => text(400, "0"),
    }
}

/// The attachments of the media list move to another parent or lose it.
pub async fn attach(ctx: &Ctx<'_>, ids: &[u64], parent: u64) -> u64 {
    let Some(writer) = ctx.env.writer else { return 0 };
    let mut count = 0;
    for id in ids {
        let Some(post) = ctx.env.db.post(*id).await.ok().flatten().filter(|p| p.post_type == "attachment") else { continue };
        let may_edit = ctx.user.can_edit_post(&post);
        if !may_edit {
            continue;
        }
        if writer.update_rows("posts", "ID", *id, &[("post_parent", Val::Int(parent as i64))]).await.is_ok() {
            count += 1;
        }
    }
    count
}

/// The nonce of the Site Health results (`check_ajax_referer( 'health-check-site-status-result' )`).
fn nonce_ok_early_for(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, nonce_action: &str) -> bool {
    let nonce = request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default();
    env.auth.verify_nonce(nonce, nonce_action, user.id, token, env.now).is_some()
}

fn nonce_ok_early(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str) -> bool {
    let nonce = request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default();
    env.auth.verify_nonce(nonce, "health-check-site-status-result", user.id, token, env.now).is_some()
}
