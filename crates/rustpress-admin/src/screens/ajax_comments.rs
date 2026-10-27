//! The Ajax actions of the comment screens: `dim-comment`, `delete-comment`, `replyto-comment`, `edit-comment`.

use rustpress_core::OrLog;
use rustpress_auth::CurrentUser;
use rustpress_core::i18n::sprintf;

use super::ajax::text;
use super::ajax_terms::AjaxResponse;
use super::comments;
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};
use rustpress_db::Comment;
use crate::content::{CommentUpdate, Content};

fn status_name(approved: &str) -> &'static str {
    match approved {
        "1" => "approved",
        "spam" => "spam",
        "trash" => "trash",
        _ => "unapproved",
    }
}

/// `_wp_ajax_delete_comment_response()`.
async fn counts_response(ctx: &Ctx<'_>, id: u64, action: &str) -> AdminResponse {
    let site = ctx.site();
    let counts = ctx.env.db.comment_counts(None).await.unwrap_or_default();
    let comment = ctx.env.db.comment_any(id).await.ok().flatten();
    let status = comment.as_ref().map_or("", |c| status_name(&c.comment_approved));
    let mut x = AjaxResponse::new(action);
    x.add(
        "comment",
        id,
        None,
        Ok(""),
        &[
            ("status", status.to_owned()),
            ("postId", comment.as_ref().map_or_else(String::new, |c| c.comment_post_id.to_string())),
            ("time", ctx.env.now.to_string()),
            ("in_moderation", counts.moderated.to_string()),
            ("i18n_comments_text", sprintf(&site.trn("%s Comment", "%s Comments", counts.approved as i64), &[&site.number_i18n(counts.approved)])),
            ("i18n_moderation_text", sprintf(&site.trnx("%s in moderation", "%s in moderation", counts.moderated as i64, "comments"), &[&site.number_i18n(counts.moderated)])),
        ],
    );
    x.send()
}

/// `current_user_can( 'edit_comment', $comment )`: `edit_post` of the post the comment is on.
/// The approved and pending comments of a post, for the count bubble of a row.
async fn post_comment_counts(env: &AdminEnv<'_>, post_id: u64) -> std::collections::HashMap<u64, (u64, u64)> {
    let approved = env.db.count_comments_for_post_with_status(post_id, "1").await.unwrap_or(0);
    let pending = env.db.count_comments_for_post_with_status(post_id, "0").await.unwrap_or(0);
    std::collections::HashMap::from([(post_id, (approved, pending))])
}

pub async fn can_edit_comment(env: &AdminEnv<'_>, user: &CurrentUser, comment: &Comment) -> bool {
    let post = env.db.post(comment.comment_post_id).await.ok().flatten();
    user.can_edit_comment(post.as_ref())
}

/// The `pre_comment_content` filters of a comment written in the administration: somebody with `unfiltered_html`
/// keeps the HTML only with the `_wp_unfiltered_html_comment` nonce of the form, everybody else gets `wp_filter_kses`.
pub fn filter_admin_comment(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, content: &str) -> String {
    let unfiltered = user.can("unfiltered_html") && env.auth.create_nonce("unfiltered-html-comment", user.id, token, env.now).is_some_and(|n| request.request("_wp_unfiltered_html_comment") == Some(n.as_str()));
    rustpress_core::kses::comment_content_filter(content, &env.site.settings.home, !unfiltered)
}

/// The checks of `edit_comment()` for a new `comment_parent`.
pub async fn check_new_parent(env: &AdminEnv<'_>, comment: &Comment, parent_id: u64, new_status: Option<&str>) -> Result<(), String> {
    let tr = |s: &str| env.site.tr(s);
    if parent_id == 0 || parent_id == comment.comment_parent {
        return Ok(());
    }
    if env.site.options.u64("thread_comments", 0) == 0 {
        return Err(tr("The comment parent cannot be changed because threaded comments are disabled."));
    }
    if parent_id == comment.comment_id {
        return Err(tr("A comment cannot be a reply to itself."));
    }
    let parent = env.db.comment_any(parent_id).await.ok().flatten();
    let parent_status = parent.as_ref().map(|p| status_name(&p.comment_approved));
    let approved = matches!(new_status.unwrap_or(&comment.comment_approved), "1" | "approve");
    let Some(parent) = parent.filter(|p| p.comment_post_id == comment.comment_post_id && p.comment_type == comment.comment_type && !matches!(parent_status, Some("spam" | "trash")) && (!approved || parent_status == Some("approved"))) else {
        return Err(tr("Invalid parent comment."));
    };
    // The ancestors of the new parent: the comment must not become a reply to one of its own replies.
    let mut seen = std::collections::HashSet::new();
    let mut depth = 1u64;
    let mut ancestor = Some(parent);
    while let Some(current) = ancestor.take().filter(|a| a.comment_parent != 0 && seen.insert(a.comment_id)) {
        if current.comment_parent == comment.comment_id {
            return Err(tr("A comment cannot be a reply to one of its own replies."));
        }
        depth += 1;
        ancestor = env.db.comment_any(current.comment_parent).await.ok().flatten();
    }
    let max_depth = env.site.options.u64("thread_comments_depth", 0);
    if max_depth > 0 {
        let mut height = 1u64;
        let mut level = vec![comment.comment_id];
        while !level.is_empty() && depth + height <= max_depth {
            let query = rustpress_db::CommentQuery { parent_in: level, types: Vec::new(), ..Default::default() };
            level = env.db.comments_query(&query).await.unwrap_or_default().into_iter().map(|c| c.comment_id).collect();
            if !level.is_empty() {
                height += 1;
            }
        }
        if depth + height > max_depth {
            return Err(tr("The comment cannot be moved there because it or its replies would exceed the maximum threading depth."));
        }
    }
    Ok(())
}

pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, action: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let Some(writer) = env.writer else { return text(500, "0") };
    let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
    // `check_ajax_referer()`: `_ajax_nonce`, else `_wpnonce`, or the field named.
    let referer_ok = |field: Option<&str>, nonce_action: &str| {
        let nonce = match field {
            Some(name) => request.request(name),
            None => request.request("_ajax_nonce").or(request.request("_wpnonce")),
        };
        nonce.is_some_and(|n| env.auth.verify_nonce(n, nonce_action, user.id, token, env.now).is_some())
    };
    let tr = |s: &str| env.site.tr(s);
    let now = || text(200, &env.now.to_string());
    match action {
        "dim-comment" => {
            let id: u64 = request.request("id").and_then(|v| v.parse().ok()).unwrap_or(0);
            let Some(comment) = env.db.comment_any(id).await.ok().flatten() else {
                let mut x = AjaxResponse::new("dim-comment");
                // `WP_Ajax_Response::add()` takes the ID of an error as 0.
                x.add("comment", 0, Some(1), Err(("invalid_comment", &sprintf(&tr("Comment %d does not exist"), &[&id.to_string()]))), &[]);
                return x.send();
            };
            if !can_edit_comment(env, user, &comment).await && !user.can("moderate_comments") {
                return text(200, "-1");
            }
            let current = status_name(&comment.comment_approved);
            if request.request("new") == Some(current) {
                return now();
            }
            if !referer_ok(None, &format!("approve-comment_{id}")) {
                return text(403, "-1");
            }
            let target = if matches!(current, "unapproved" | "spam") { "approve" } else { "hold" };
            if target == "approve" {
                comments::approve_comment(&ctx, &content, &comment).await;
            } else {
                content.set_comment_status(id, target).await.or_log();
            }
            counts_response(&ctx, id, "dim-comment").await
        }
        "delete-comment" => {
            let id: u64 = request.request("id").and_then(|v| v.parse().ok()).unwrap_or(0);
            let Some(comment) = env.db.comment_any(id).await.ok().flatten() else { return now() };
            if !can_edit_comment(env, user, &comment).await {
                return text(200, "-1");
            }
            if !referer_ok(None, &format!("delete-comment_{id}")) {
                return text(403, "-1");
            }
            let flag = |name: &str| request.request(name) == Some("1");
            let status = status_name(&comment.comment_approved);
            let done = if flag("trash") {
                if status == "trash" {
                    return now();
                }
                content.trash_comment(id).await
            } else if flag("untrash") {
                if status != "trash" {
                    return now();
                }
                content.untrash_comment(id).await
            } else if flag("spam") {
                if status == "spam" {
                    return now();
                }
                content.spam_comment(id).await
            } else if flag("unspam") {
                if status != "spam" {
                    return now();
                }
                content.unspam_comment(id).await
            } else if flag("delete") {
                content.delete_comment(id).await
            } else {
                return text(200, "-1");
            };
            if done.unwrap_or(false) { counts_response(&ctx, id, "delete-comment").await } else { text(200, "0") }
        }
        "replyto-comment" => {
            if !referer_ok(Some("_ajax_nonce-replyto-comment"), "replyto-comment") {
                return text(403, "-1");
            }
            let post_id: u64 = request.request("comment_post_ID").and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0).max(0) as u64;
            let Some(post) = env.db.post(post_id).await.ok().flatten() else { return text(200, "-1") };
            if !user.can_edit_post(&post) {
                return text(200, "-1");
            }
            if post.post_status.is_empty() {
                return text(200, "1");
            }
            if matches!(post.post_status.as_str(), "draft" | "pending" | "trash") {
                return text(200, &tr("You cannot reply to a comment on a draft post."));
            }
            let body = request.request("content").unwrap_or_default().trim().to_owned();
            if body.is_empty() {
                return text(200, &tr("Please type your comment text."));
            }
            let comment_type = request.request("comment_type").map_or_else(|| "comment".to_owned(), |t| t.trim().to_owned());
            let mut parent: u64 = request.request("comment_ID").and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0).unsigned_abs();
            // Approve the parent with the reply.
            let mut approved_parent = None;
            if request.request("approve_parent").is_some_and(|v| !v.is_empty() && v != "0")
                && let Some(p) = env.db.comment_any(parent).await.ok().flatten().filter(|p| p.comment_approved == "0" && p.comment_post_id == post_id)
            {
                if !can_edit_comment(env, user, &p).await {
                    return text(200, "-1");
                }
                if comments::approve_comment(&ctx, &content, &p).await {
                    approved_parent = Some(p);
                }
            }
            // `wp_new_comment()`: only an approved or waiting comment can be replied to.
            if parent != 0 && !env.db.comment_any(parent).await.ok().flatten().is_some_and(|p| matches!(status_name(&p.comment_approved), "approved" | "unapproved")) {
                parent = 0;
            }
            let filtered = filter_admin_comment(env, request, user, token, &body);
            if env.db.duplicate_comment(post_id, parent, &user.display_name, &user.email, &filtered).await.ok().flatten().is_some() {
                return text(200, &tr("Duplicate comment detected; it looks as though you&#8217;ve already said that!"));
            }
            // `wp_allow_comment()`: whoever may edit a post that is not a draft is its author or may moderate.
            let approved = if post.post_author == user.id || user.can("moderate_comments") { "1" } else { "0" };
            let id = match content.insert_reply(post_id, parent, &filtered, &comment_type, approved, request.remote_addr.as_deref().unwrap_or_default(), request.user_agent.as_deref().unwrap_or_default()).await {
                Ok(id) => id,
                Err(e) => return text(200, &e.to_string()),
            };
            // `comment_post`: the moderators and the author of the post are told.
            env.mail_ctx().new_comment_notify(env.roles, id, true, user.id).await;
            let Some(new_comment) = env.db.comment_any(id).await.ok().flatten() else { return text(200, "1") };
            let html = match request.request("mode") {
                Some("dashboard") => super::dashboard::comment_row(&ctx, &new_comment, 0).await,
                Some("single") => comments::row_html_in(&ctx, &new_comment, 0, "all", &std::collections::HashMap::new(), comments::RowTable::PostBox).await,
                _ => comments::row_html(&ctx, &new_comment, 0, "all", &post_comment_counts(env, post_id).await).await,
            };
            let position: i64 = request.request("position").and_then(|p| p.parse().ok()).filter(|p| *p != 0).unwrap_or(-1);
            let counts = env.db.comment_counts(None).await.unwrap_or_default();
            let site = env.site;
            let mut supplemental: Vec<(&str, String)> = vec![
                ("in_moderation", counts.moderated.to_string()),
                ("i18n_comments_text", sprintf(&site.trn("%s Comment", "%s Comments", counts.approved as i64), &[&site.number_i18n(counts.approved)])),
                ("i18n_moderation_text", sprintf(&site.trn("%s Comment in moderation", "%s Comments in moderation", counts.moderated as i64), &[&site.number_i18n(counts.moderated)])),
            ];
            if let Some(p) = &approved_parent {
                supplemental.push(("parent_approved", p.comment_id.to_string()));
                supplemental.push(("parent_post_id", p.comment_post_id.to_string()));
            }
            let mut x = AjaxResponse::new("replyto-comment");
            x.add("comment", id, Some(position), Ok(&html), &supplemental);
            x.send()
        }
        _ => {
            // edit-comment
            if !referer_ok(Some("_ajax_nonce-replyto-comment"), "replyto-comment") {
                return text(403, "-1");
            }
            let id: u64 = request.request("comment_ID").and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0).max(0) as u64;
            let Some(comment) = env.db.comment_any(id).await.ok().flatten() else { return text(200, "-1") };
            if !can_edit_comment(env, user, &comment).await {
                return text(200, "-1");
            }
            if request.request("content") == Some("") {
                return text(200, &tr("Please type your comment text."));
            }
            let status = request.request("status").or(request.request("comment_status"));
            if let Some(parent) = request.request("comment_parent").and_then(|p| p.trim().parse::<i64>().ok())
                && let Err(message) = check_new_parent(env, &comment, parent.max(0) as u64, status).await
            {
                return text(200, &message);
            }
            let field = |name: &str| request.request(name).map(str::to_owned);
            let update = CommentUpdate {
                author: field("newcomment_author"),
                author_email: field("newcomment_author_email"),
                author_url: field("newcomment_author_url"),
                content: field("content").map(|c| filter_admin_comment(env, request, user, token, &c)),
                status: None,
                date: None,
            };
            if let Err(e) = content.update_comment(id, &update).await {
                return text(200, &e.to_string());
            }
            let Some(updated) = env.db.comment_any(id).await.ok().flatten() else { return text(200, "-1") };
            // `$_POST['checkbox']` of 1 is a row of the comments screen, anything else one of the box of the edit form.
            let html = if request.post_field("checkbox") == Some("1") {
                comments::row_html(&ctx, &updated, 0, "all", &post_comment_counts(env, updated.comment_post_id).await).await
            } else {
                comments::row_html_in(&ctx, &updated, 0, "all", &std::collections::HashMap::new(), comments::RowTable::PostBox).await
            };
            let position: i64 = request.request("position").and_then(|p| p.parse().ok()).filter(|p| *p != 0).unwrap_or(-1);
            let mut x = AjaxResponse::new("edit-comment");
            x.add("edit_comment", id, Some(position), Ok(&html), &[]);
            x.send()
        }
    }
}
