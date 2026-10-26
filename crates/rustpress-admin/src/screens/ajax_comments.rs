//! The Ajax actions of the comment screens: `dim-comment`, `delete-comment`, `replyto-comment`, `edit-comment`.

use rustpress_auth::CurrentUser;
use rustpress_core::i18n::sprintf;

use super::ajax::text;
use super::ajax_terms::AjaxResponse;
use super::comments;
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};
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

pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, action: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let Some(writer) = env.writer else { return text(500, "0") };
    let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
    let nonce_ok = |name: &str, nonce_action: &str| request.request(name).is_some_and(|n| env.auth.verify_nonce(n, nonce_action, user.id, token, env.now).is_some());
    if !ctx.can("moderate_comments") && !ctx.can("edit_posts") {
        return text(403, "-1");
    }
    match action {
        "dim-comment" => {
            let id: u64 = request.request("id").and_then(|v| v.parse().ok()).unwrap_or(0);
            let Some(comment) = env.db.comment_any(id).await.ok().flatten() else {
                let mut x = AjaxResponse::new("dim-comment");
                x.add("comment", id, Some(1), Err(("invalid_comment", &sprintf(&env.site.tr("Comment %d does not exist"), &[&id.to_string()]))), &[]);
                return x.send();
            };
            if !ctx.can("moderate_comments") {
                return text(403, &env.site.tr("Sorry, you are not allowed to edit comments on this post."));
            }
            let current = status_name(&comment.comment_approved);
            if request.request("new") == Some(current) {
                return text(200, &env.now.to_string());
            }
            if !nonce_ok("_ajax_nonce", &format!("approve-comment_{id}")) {
                return text(403, "-1");
            }
            let target = if matches!(current, "unapproved" | "spam") { "approve" } else { "hold" };
            let _ = content.set_comment_status(id, target).await;
            counts_response(&ctx, id, "dim-comment").await
        }
        "delete-comment" => {
            let id: u64 = request.request("id").and_then(|v| v.parse().ok()).unwrap_or(0);
            let Some(comment) = env.db.comment_any(id).await.ok().flatten() else { return text(200, &env.now.to_string()) };
            if !ctx.can("moderate_comments") {
                return text(403, "-1");
            }
            if !nonce_ok("_ajax_nonce", &format!("delete-comment_{id}")) {
                return text(403, "-1");
            }
            let flag = |name: &str| request.request(name) == Some("1");
            let status = status_name(&comment.comment_approved);
            let ok = if flag("spam") {
                if status == "spam" {
                    return text(200, "1");
                }
                content.spam_comment(id).await.unwrap_or(false)
            } else if flag("unspam") {
                content.unspam_comment(id).await.unwrap_or(false)
            } else if flag("trash") {
                if status == "trash" {
                    return text(200, "1");
                }
                content.trash_comment(id).await.unwrap_or(false)
            } else if flag("untrash") {
                content.untrash_comment(id).await.unwrap_or(false)
            } else {
                content.delete_comment(id).await.unwrap_or(false)
            };
            if ok { counts_response(&ctx, id, "delete-comment").await } else { text(200, "0") }
        }
        "replyto-comment" => {
            if !nonce_ok("_ajax_nonce-replyto-comment", "replyto-comment") {
                return text(403, "-1");
            }
            let post_id: u64 = request.request("comment_post_ID").and_then(|v| v.parse().ok()).unwrap_or(0);
            let parent: u64 = request.request("comment_ID").and_then(|v| v.parse().ok()).unwrap_or(0);
            let body = request.request("content").unwrap_or_default().trim().to_owned();
            if body.is_empty() {
                return text(500, &env.site.tr("Please type your comment text."));
            }
            let Some(post) = env.db.post(post_id).await.ok().flatten() else { return text(500, &env.site.tr("Invalid post ID.")) };
            if post.comment_status != "open" && !ctx.can("edit_posts") {
                return text(500, &env.site.tr("Sorry, comments are closed for this item."));
            }
            let id = match content.insert_reply(post_id, parent, &body, "1").await {
                Ok(id) => id,
                Err(e) => return text(500, &e.to_string()),
            };
            let Some(new_comment) = env.db.comment_any(id).await.ok().flatten() else { return text(500, "0") };
            let counts = std::collections::HashMap::new();
            let html = comments::row_html(&ctx, &new_comment, 0, "all", &counts).await;
            let position: i64 = request.request("position").and_then(|p| p.parse().ok()).unwrap_or(-1);
            let mut x = AjaxResponse::new("replyto-comment");
            let supplemental: Vec<(&str, String)> = match env.db.comment_any(parent).await.ok().flatten() {
                Some(p) => vec![("in_reply_to", p.comment_author.clone()), ("in_reply_to_link", comments::comment_link(&ctx, &p).await), ("parent_approved", status_name(&p.comment_approved).to_owned())],
                None => Vec::new(),
            };
            x.add("comment", id, Some(position), Ok(&html), &supplemental);
            x.send()
        }
        _ => {
            // edit-comment
            if !nonce_ok("_ajax_nonce", "replyto-comment") {
                return text(403, "-1");
            }
            let id: u64 = request.request("comment_ID").and_then(|v| v.parse().ok()).unwrap_or(0);
            if env.db.comment_any(id).await.ok().flatten().is_none() {
                return text(500, &env.site.tr("Invalid comment ID."));
            }
            let field = |name: &str| request.request(name).map(str::to_owned);
            let update = CommentUpdate {
                author: field("newcomment_author"),
                author_email: field("newcomment_author_email"),
                author_url: field("newcomment_author_url"),
                content: field("content"),
                status: None,
                date: None,
            };
            if let Err(e) = content.update_comment(id, &update).await {
                return text(500, &e.to_string());
            }
            let Some(updated) = env.db.comment_any(id).await.ok().flatten() else { return text(500, "0") };
            let counts = std::collections::HashMap::new();
            let html = comments::row_html(&ctx, &updated, 0, "all", &counts).await;
            let mut x = AjaxResponse::new("edit-comment");
            x.add("comment", id, Some(-1), Ok(&html), &[]);
            x.send()
        }
    }
}
