//! `wp-admin/edit-comments.php` and `comment.php`: moderating comments.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::add_query_arg;
use rustpress_db::{Comment, CommentListQuery};

use super::common::{forbidden, read_only};
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::content::{CommentUpdate, Content};
use crate::list_table::{Column, Table, view_link, views};
use crate::sanitize::esc_textarea;

fn origin(ctx: &Ctx<'_>) -> String {
    let siteurl = &ctx.site().settings.siteurl;
    match siteurl.split_once("://") {
        Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or_default()),
        None => siteurl.clone(),
    }
}

/// `antispambot()`: the address with random characters turned into entities.
pub fn antispambot(email: &str) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut out = String::new();
    for ch in email.chars() {
        let choice = rng.random_range(0..2);
        if ch == '@' {
            out.push_str("&#064;");
        } else if choice == 0 {
            out.push_str(&format!("&#{};", ch as u32));
        } else if choice == 1 {
            out.push_str(&format!("&#x{:x};", ch as u32).replace("&#x", "&#").replace(';', ";"));
        } else {
            out.push(ch);
        }
    }
    out
}

fn avatar(ctx: &Ctx<'_>, email: &str, size: u64) -> String {
    let url = |s: u64| esc_url(&rustpress_blocks::content::avatar_url(&ctx.site().options, email, s));
    format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-{size} photo' height='{size}' width='{size}' loading='lazy' decoding='async'/>", url(size), url(size * 2))
}

/// `get_comment_link()` with the right `comment-page-N`.
pub async fn comment_link(ctx: &Ctx<'_>, comment: &Comment) -> String {
    let site = ctx.site();
    let Some(post) = ctx.env.db.post(comment.comment_post_id).await.ok().flatten() else { return String::new() };
    let permalink = site.permalink(&post).await.unwrap_or_default();
    let mut page = 1u64;
    let mut max_page = 1u64;
    let per_page = site.options.u64("comments_per_page", 50);
    if site.options.get("page_comments") == Some("1") && per_page > 0 {
        let approved = ctx.env.db.approved_comments(post.id).await.unwrap_or_default();
        let top: Vec<&Comment> = approved.iter().filter(|c| c.comment_parent == 0).collect();
        // The top-level ancestor decides the page.
        let mut ancestor = comment;
        while ancestor.comment_parent != 0 {
            match approved.iter().find(|c| c.comment_id == ancestor.comment_parent) {
                Some(parent) => ancestor = parent,
                None => break,
            }
        }
        let descending = site.options.get("comment_order") == Some("desc");
        let key = |c: &Comment| (c.comment_date_gmt.clone(), c.comment_id);
        let before = top.iter().filter(|c| if descending { key(c) > key(ancestor) } else { key(c) < key(ancestor) }).count() as u64;
        page = before / per_page + 1;
        max_page = (top.len() as u64).div_ceil(per_page).max(1);
    }
    let default_page = site.options.string("default_comments_page", "newest");
    // Outside the comment loop WordPress does not know the number of pages, so "newest" always names the page.
    let _ = max_page;
    let wants_page = default_page == "newest" || (default_page == "oldest" && page != 1);
    let base = if site.settings.permalink_structure.is_empty() { permalink.clone() } else { permalink.trim_end_matches('/').to_owned() };
    let link = if wants_page && site.options.get("page_comments") == Some("1") {
        if site.settings.permalink_structure.is_empty() { add_query_arg(&base, "cpage", &page.to_string()) } else { format!("{base}/comment-page-{page}/") }
    } else {
        permalink
    };
    format!("{link}#comment-{}", comment.comment_id)
}

fn count_bubble(ctx: &Ctx<'_>, post_id: u64, approved: u64, pending: u64) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let approved_text = if pending > 0 {
        sprintf(&site.trn("%s approved comment", "%s approved comments", approved as i64), &[&site.number_i18n(approved)])
    } else {
        sprintf(&site.trn("%s comment", "%s comments", approved as i64), &[&site.number_i18n(approved)])
    };
    let pending_text = if pending > 0 { sprintf(&site.trn("%s pending comment", "%s pending comments", pending as i64), &[&site.number_i18n(pending)]) } else { tr("No pending comments") };
    let approved_link = ctx.admin_url(&format!("edit-comments.php?p={post_id}&comment_status=approved"));
    let pending_link = ctx.admin_url(&format!("edit-comments.php?p={post_id}&comment_status=moderated"));
    let approved_html = if approved > 0 {
        format!("<a href=\"{}\" class=\"post-com-count post-com-count-approved\"><span class=\"comment-count-approved\" aria-hidden=\"true\">{}</span><span class=\"screen-reader-text\">{approved_text}</span></a>", esc_url(&approved_link), site.number_i18n(approved))
    } else {
        format!("<span class=\"post-com-count post-com-count-no-comments\"><span class=\"comment-count comment-count-no-comments\" aria-hidden=\"true\">{}</span><span class=\"screen-reader-text\">{}</span></span>", site.number_i18n(approved), tr("No comments"))
    };
    let pending_html = if pending > 0 {
        format!("<a href=\"{}\" class=\"post-com-count post-com-count-pending\"><span class=\"comment-count-pending\" aria-hidden=\"true\">{}</span><span class=\"screen-reader-text\">{pending_text}</span></a>", esc_url(&pending_link), site.number_i18n(pending))
    } else {
        format!("<span class=\"post-com-count post-com-count-pending post-com-count-no-pending\"><span class=\"comment-count comment-count-no-pending\" aria-hidden=\"true\">{}</span><span class=\"screen-reader-text\">{pending_text}</span></span>", site.number_i18n(pending))
    };
    format!("<span class=\"post-com-count-wrapper post-com-count-{post_id}\">{approved_html}{pending_html}</span>")
}

fn author_block(ctx: &Ctx<'_>, comment: &Comment) -> String {
    let mut out = String::new();
    let author = if comment.comment_author.is_empty() { ctx.tr("Anonymous") } else { comment.comment_author.clone() };
    let avatar_html = if ctx.site().show_avatars() && comment.comment_type != "pingback" && comment.comment_type != "trackback" { avatar(ctx, &comment.comment_author_email, 32) } else { String::new() };
    out.push_str(&format!("<strong>{avatar_html} {}</strong><br />", esc_html(&author)));
    let url = comment.comment_author_url.trim();
    if !url.is_empty() && url != "http://" {
        let display = url.trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("www.");
        let display = display.split(['?', '#']).next().unwrap_or(display).trim_end_matches('/');
        out.push_str(&format!("<a href=\"{}\" rel=\"noopener noreferrer\">{}</a><br />", esc_url(url), esc_html(display)));
    }
    if !comment.comment_author_email.is_empty() {
        let email = antispambot(&comment.comment_author_email);
        out.push_str(&format!("<a href=\"mailto:{email}\">{email}</a><br />"));
    }
    out
}

/// One `<tr>` of the list.
pub async fn row_html(ctx: &Ctx<'_>, comment: &Comment, index: usize, current_status: &str, counts: &std::collections::HashMap<u64, (u64, u64)>) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    let id = comment.comment_id;
    let status = match comment.comment_approved.as_str() {
        "1" => "approved",
        "0" => "unapproved",
        "spam" => "spam",
        "trash" => "trash",
        _ => "unapproved",
    };
    let kind = if comment.comment_type.is_empty() { "comment" } else { comment.comment_type.as_str() };
    let (alt, thread_alt) = if index % 2 == 0 { ("even", "thread-even") } else { ("odd alt", "thread-odd thread-alt") };
    let post = db.post(comment.comment_post_id).await.ok().flatten();
    let can_moderate = ctx.can("moderate_comments");
    let del_nonce = ctx.create_nonce(&format!("delete-comment_{id}"));
    let approve_nonce = ctx.create_nonce(&format!("approve-comment_{id}"));
    let url = |action: &str, nonce: &str| esc_url(&format!("comment.php?action={action}&c={id}&_wpnonce={nonce}"));
    let post_id = comment.comment_post_id;
    // The row actions.
    let mut actions: Vec<(String, String)> = Vec::new();
    let mut push = |class: &str, html: String| actions.push((class.to_owned(), html));
    if can_moderate {
        let approve = format!("<a href=\"{}\" data-wp-lists=\"dim:the-comment-list:comment-{id}:unapproved:e7e7d3:e7e7d3:new=approved\" class=\"vim-a aria-button-if-js\" aria-label=\"{}\">{}</a>", url("approvecomment", &approve_nonce), tr("Approve this comment"), tr("Approve"));
        let unapprove = format!("<a href=\"{}\" data-wp-lists=\"dim:the-comment-list:comment-{id}:unapproved:e7e7d3:e7e7d3:new=unapproved\" class=\"vim-u aria-button-if-js\" aria-label=\"{}\">{}</a>", url("unapprovecomment", &approve_nonce), tr("Unapprove this comment"), tr("Unapprove"));
        if current_status != "all" && !current_status.is_empty() {
            if status == "approved" {
                push("unapprove", unapprove);
            } else if status == "unapproved" {
                push("approve", approve);
            }
        } else {
            push("approve", approve);
            push("unapprove", unapprove);
        }
        if status != "spam" && status != "trash" {
            push("reply hide-if-no-js", format!("<button type=\"button\" data-comment-id=\"{id}\" data-post-id=\"{post_id}\" data-action=\"replyto\" class=\"vim-r comment-inline button-link\" aria-expanded=\"false\" aria-label=\"{}\">{}</button>", tr("Reply to this comment"), tr("Reply")));
            push("quickedit hide-if-no-js", format!("<button type=\"button\" data-comment-id=\"{id}\" data-post-id=\"{post_id}\" data-action=\"edit\" class=\"vim-q comment-inline button-link\" aria-expanded=\"false\" aria-label=\"{}\">{}</button>", tr("Quick edit this comment inline"), tr("Quick&nbsp;Edit")));
            push("edit", format!("<a href=\"comment.php?action=editcomment&amp;c={id}\" aria-label=\"{}\">{}</a>", tr("Edit this comment"), tr("Edit")));
        }
        if status != "spam" {
            push("spam", format!("<a href=\"{}\" data-wp-lists=\"delete:the-comment-list:comment-{id}::spam=1\" class=\"vim-s vim-destructive aria-button-if-js\" aria-label=\"{}\">{}</a>", url("spamcomment", &del_nonce), tr("Mark this comment as spam"), tr("Spam")));
        } else {
            push("unspam", format!("<a href=\"{}\" data-wp-lists=\"delete:the-comment-list:comment-{id}:e7e7d3:action=dim-comment&amp;new=approved\" class=\"vim-z vim-destructive aria-button-if-js\" aria-label=\"{}\">{}</a>", url("unspamcomment", &del_nonce), tr("Restore this comment from the spam"), tr("Not Spam")));
        }
        if status == "trash" {
            push("untrash", format!("<a href=\"{}\" data-wp-lists=\"delete:the-comment-list:comment-{id}:e7e7d3:action=dim-comment&amp;new=approved\" class=\"vim-z vim-destructive aria-button-if-js\" aria-label=\"{}\">{}</a>", url("untrashcomment", &del_nonce), tr("Restore this comment from the Trash"), tr("Restore")));
        }
        if status == "spam" || status == "trash" {
            push("delete", format!("<a href=\"{}\" data-wp-lists=\"delete:the-comment-list:comment-{id}::delete=1\" class=\"delete vim-d vim-destructive aria-button-if-js\" aria-label=\"{}\">{}</a>", url("deletecomment", &del_nonce), tr("Delete this comment permanently"), tr("Delete Permanently")));
        } else {
            push("trash", format!("<a href=\"{}\" data-wp-lists=\"delete:the-comment-list:comment-{id}::trash=1\" class=\"delete vim-d vim-destructive aria-button-if-js\" aria-label=\"{}\">{}</a>", url("trashcomment", &del_nonce), tr("Move this comment to the Trash"), tr("Trash")));
        }
    }
    // Separators: none before the first action or before "unapprove" when it is second.
    let mut actions_html = String::from("<div class=\"row-actions\">");
    for (i, (class, html)) in actions.iter().enumerate() {
        let first = i == 0 || ((class == "unapprove") && i == 1);
        let sep = if first { "" } else { " | " };
        actions_html.push_str(&format!("<span class='{class}'>{sep}{html}</span>"));
    }
    actions_html.push_str("</div>");
    if actions.is_empty() {
        actions_html.clear();
    } else {
        actions_html.push_str(&format!("<button type=\"button\" class=\"toggle-row\"><span class=\"screen-reader-text\">{}</span></button>", tr("Show more details")));
    }
    let author = author_block(ctx, comment);
    let reply_to = if comment.comment_parent != 0 {
        match db.comment_any(comment.comment_parent).await.ok().flatten() {
            Some(parent) => {
                let link = comment_link(ctx, &parent).await;
                format!("{} ", sprintf(&tr("In reply to %s."), &[&format!("<a href=\"{}\">{}</a>", esc_url(&link), esc_html(&parent.comment_author))]))
            }
            None => String::new(),
        }
    } else {
        String::new()
    };
    let text = rustpress_blocks::content::comment_text(site, &comment.comment_content);
    let raw_text = esc_textarea(&comment.comment_content);
    let inline = format!(
        "\n\t\t<div id=\"inline-{id}\" class=\"hidden\">\n\t\t\t<textarea class=\"comment\" rows=\"1\" cols=\"1\">{raw_text}</textarea>\n\t\t\t<div class=\"author-email\">{}</div>\n\t\t\t<div class=\"author\">{}</div>\n\t\t\t<div class=\"author-url\">{}</div>\n\t\t\t<div class=\"comment_status\">{}</div>\n\t\t</div>\n\t\t",
        esc_html(&comment.comment_author_email),
        esc_html(&comment.comment_author),
        esc_html(&comment.comment_author_url),
        esc_html(&comment.comment_approved)
    );
    let response = match &post {
        Some(post) => {
            let title = rustpress_blocks::content::the_title_for(site, post);
            let edit = if ctx.can("edit_posts") { format!("<a href='{}' class='comments-edit-item-link'>{}</a>", ctx.admin_url(&format!("post.php?post={}&amp;action=edit", post.id)), esc_html(&title)) } else { esc_html(&title) };
            let view = site.permalink(post).await.unwrap_or_default();
            let (approved, pending) = counts.get(&post.id).copied().unwrap_or((0, 0));
            format!("<div class=\"response-links\">{edit}<a href='{}' class='comments-view-item-link'>{}</a>{} </div>", esc_url(&view), tr("View Post"), count_bubble(ctx, post.id, approved, pending))
        }
        None => String::new(),
    };
    let link = comment_link(ctx, comment).await;
    let time = sprintf(&tr("%1$s at %2$s"), &[&site.date(&tr("Y/m/d"), rustpress_core::datetime::SiteTimezone::gmt_timestamp(&comment.comment_date_gmt).unwrap_or(0)), &site.date(&tr("g:i a"), rustpress_core::datetime::SiteTimezone::gmt_timestamp(&comment.comment_date_gmt).unwrap_or(0))]);
    let checkbox = if can_moderate { format!("\t\t<input id=\"cb-select-{id}\" type=\"checkbox\" name=\"delete_comments[]\" value=\"{id}\" />\n\t\t<label for=\"cb-select-{id}\">\n\t\t\t<span class=\"screen-reader-text\">\n\t\t\t{}\t\t\t</span>\n\t\t</label>\n\t\t\t", tr("Select comment")) } else { String::new() };
    format!(
        "<tr id='comment-{id}' class='{kind} {alt} {thread_alt} depth-1 {status}'><td class=\"check-column\">{checkbox}</td><td class='author column-author' data-colname=\"{a}\">{author}</td><th class='comment column-comment has-row-actions column-primary' data-colname=\"{c}\" scope=\"row\"><div class=\"comment-author\">{author}</div>{reply_to}{text}{inline}{actions_html}</th><td class='response column-response' data-colname=\"{r}\">{response}</td><td class='date column-date' data-colname=\"{d}\"><div class=\"submitted-on\">{}</div></td></tr>\n",
        if status == "approved" { format!("<a href=\"{}\">{time}</a>", esc_url(&link)) } else { time.clone() },
        a = tr("Author"),
        c = tr("Comment"),
        r = tr("In response to"),
        d = tr("Submitted on"),
    )
}

fn help(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    Help {
        tabs: vec![HelpTab { id: "overview".into(), title: tr("Overview"), content: format!("<p>{}</p>", tr("You can manage comments made on your site similarly to the way you manage posts and other content. This screen is customizable in the same ways as other management screens, and you can act on comments using the on-hover action links or the Bulk Actions.")) }],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p>", tr("For more information:"), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    }
}

/// The notices after an action (`approved=1&ids=...`).
fn notice(ctx: &Ctx<'_>) -> String {
    let site = ctx.site();
    let get = |name: &str| ctx.request.get(name).and_then(|v| v.parse::<i64>().ok()).filter(|n| *n > 0);
    let mut parts: Vec<String> = Vec::new();
    let number = |n: i64| site.number_i18n(n as u64);
    if let Some(n) = get("approved") {
        parts.push(sprintf(&site.trn("%s comment approved.", "%s comments approved.", n), &[&number(n)]));
    }
    if let Some(n) = get("spammed") {
        parts.push(sprintf(&site.trn("%s comment marked as spam.", "%s comments marked as spam.", n), &[&number(n)]));
    }
    if let Some(n) = get("unspammed") {
        parts.push(sprintf(&site.trn("%s comment restored from the spam", "%s comments restored from the spam", n), &[&number(n)]));
    }
    if let Some(n) = get("trashed") {
        parts.push(sprintf(&site.trn("%s comment moved to the Trash.", "%s comments moved to the Trash.", n), &[&number(n)]));
    }
    if let Some(n) = get("untrashed") {
        parts.push(sprintf(&site.trn("%s comment restored from the Trash", "%s comments restored from the Trash", n), &[&number(n)]));
    }
    if let Some(n) = get("deleted") {
        parts.push(sprintf(&site.trn("%s comment permanently deleted", "%s comments permanently deleted", n), &[&number(n)]));
    }
    if let Some(n) = get("unapproved") {
        parts.push(sprintf(&site.trn("%s comment unapproved.", "%s comments unapproved.", n), &[&number(n)]));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("<div id=\"moderated\" class=\"updated notice is-dismissible\"><p>{}</p></div>\n", parts.join("<br/>\n"))
}

/// `edit-comments.php`.
pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    if !ctx.can("edit_posts") {
        return forbidden(ctx, "Sorry, you are not allowed to edit comments.");
    }
    // Bulk actions.
    let action = {
        let a = request.request("action").unwrap_or_default();
        if a.is_empty() || a == "-1" { request.request("action2").unwrap_or_default().to_owned() } else { a.to_owned() }
    };
    if request.post_field("bulk_action").is_some() || request.get("bulk_action").is_some() || (!action.is_empty() && request.request("delete_comments[]").is_some()) {
        if let Some(response) = bulk(ctx, &action).await {
            return response;
        }
    }
    let status_param = request.get("comment_status").unwrap_or("all").to_owned();
    let status_param = if ["all", "mine", "moderated", "approved", "spam", "trash"].contains(&status_param.as_str()) { status_param } else { "all".to_owned() };
    let db = ctx.env.db;
    let counts = db.comment_counts(None).await.unwrap_or_default();
    let post_id = request.get("p").and_then(|v| v.parse::<u64>().ok()).filter(|p| *p > 0);
    let search = request.get("s").map(|s| s.trim().to_owned()).unwrap_or_default();
    let kind = request.get("comment_type").unwrap_or_default().to_owned();
    let orderby = request.get("orderby").filter(|o| ["comment_author", "comment_post_ID", "comment_date", "comment_ID"].contains(o)).unwrap_or("comment_date").to_owned();
    let desc = request.get("order").map_or(true, |o| o.eq_ignore_ascii_case("desc"));
    let per_page = db.user_meta(ctx.user.id, "edit_comments_per_page").await.ok().flatten().and_then(|v| v.parse::<u64>().ok()).filter(|v| *v >= 1).unwrap_or(20);
    let paged = request.get("paged").and_then(|p| p.parse::<u64>().ok()).filter(|p| *p > 0).unwrap_or(1);
    let (status, user_id) = match status_param.as_str() {
        "mine" => ("all", Some(ctx.user.id)),
        "moderated" => ("0", None),
        "approved" => ("1", None),
        other => (other, None),
    };
    let query = CommentListQuery { status: status.to_owned(), post_id, user_id, kind: kind.clone(), search: (!search.is_empty()).then(|| search.clone()), orderby: orderby.clone(), desc, limit: per_page, offset: (paged - 1) * per_page };
    let (comments, total) = db.list_comments_admin(&query).await.unwrap_or_default();
    // Counts of the posts in the list.
    let mut post_counts: std::collections::HashMap<u64, (u64, u64)> = std::collections::HashMap::new();
    for comment in &comments {
        if !post_counts.contains_key(&comment.comment_post_id) {
            let c = db.comment_counts(Some(comment.comment_post_id)).await.unwrap_or_default();
            post_counts.insert(comment.comment_post_id, (c.approved, c.moderated));
        }
    }
    let mine = db.list_comments_admin(&CommentListQuery { status: "all".into(), user_id: Some(ctx.user.id), limit: 1, ..Default::default() }).await.map(|(_, n)| n).unwrap_or(0);

    let mut links: Vec<(String, String)> = Vec::new();
    let link_for = |status: &str, label: &str, class_name: &str, n: u64, extra: &str| -> (String, String) {
        let mut url = ctx.admin_url(&format!("edit-comments.php?comment_status={status}{extra}"));
        if let Some(p) = post_id {
            url = add_query_arg(&url, "p", &p.to_string());
        }
        let text = format!("{label} <span class=\"count\">(<span class=\"{class_name}-count\">{}</span>)</span>", site.number_i18n(n));
        (status.to_owned(), view_link(&url, &text, status_param == status))
    };
    links.push(link_for("all", &tr("All"), "all", counts.all, ""));
    links.push(link_for("mine", &tr("Mine"), "mine", mine, &format!("&user_id={}", ctx.user.id)));
    links.push(link_for("moderated", &tr("Pending"), "pending", counts.moderated, ""));
    links.push(link_for("approved", &tr("Approved"), "approved", counts.approved, ""));
    links.push(link_for("spam", &tr("Spam"), "spam", counts.spam, ""));
    links.push(link_for("trash", &tr("Trash"), "trash", counts.trash, ""));
    if !ctx.can("moderate_comments") {
        links.retain(|(class, _)| class == "all" || class == "mine" || class == "approved");
    }

    let mut rows = String::new();
    for (index, comment) in comments.iter().enumerate() {
        rows.push_str(&row_html(ctx, comment, index, &status_param, &post_counts).await);
    }
    let columns = vec![
        Column::new("cb", ""),
        Column::new("author", &tr("Author")).sortable("comment_author", false, Some(&tr("Author")), None),
        Column::new("comment", &tr("Comment")),
        Column::new("response", &tr("In response to")).sortable("comment_post_ID", false, Some(&tr("In Response To")), None),
        Column::new("date", &tr("Submitted on")).sortable("comment_date", false, None, None),
    ];
    let bulk_actions: Vec<(String, String)> = match status_param.as_str() {
        "spam" => vec![("unspam".into(), tr("Not Spam")), ("delete".into(), tr("Delete permanently"))],
        "trash" => vec![("untrash".into(), tr("Restore")), ("delete".into(), tr("Delete permanently"))],
        "moderated" => vec![("approve".into(), tr("Approve")), ("spam".into(), tr("Mark as spam")), ("trash".into(), tr("Move to Trash"))],
        "approved" => vec![("unapprove".into(), tr("Unapprove")), ("spam".into(), tr("Mark as spam")), ("trash".into(), tr("Move to Trash"))],
        _ => vec![("unapprove".into(), tr("Unapprove")), ("approve".into(), tr("Approve")), ("spam".into(), tr("Mark as spam")), ("trash".into(), tr("Move to Trash"))],
    };
    let table = Table {
        site,
        plural: "comments".into(),
        singular: String::new(),
        columns,
        primary: "comment".into(),
        current_url: format!("{}{}", origin(ctx), request.request_uri),
        orderby: request.get("orderby").unwrap_or_default().to_owned(),
        order: if desc { "desc".into() } else { "asc".into() },
        total_items: total,
        per_page,
        current_page: paged,
        bulk_actions: if ctx.can("moderate_comments") { bulk_actions } else { Vec::new() },
        has_items: !comments.is_empty(),
    };
    let type_select = format!(
        "<div class=\"alignleft actions\"><label class=\"screen-reader-text\" for=\"filter-by-comment-type\">{}</label><select id=\"filter-by-comment-type\" name=\"comment_type\">\t<option value=''>{}</option>\t<option value='comment'{}>{}</option>\n\t<option value='pings'{}>{}</option>\n</select><input type=\"submit\" name=\"filter_action\" id=\"post-query-submit\" class=\"button button-compact\" value=\"{}\"  /></div>",
        tr("Filter by comment type"),
        tr("All comment types"),
        if kind == "comment" { " selected='selected'" } else { "" },
        tr("Comments"),
        if kind == "pings" { " selected='selected'" } else { "" },
        tr("Pings"),
        tr("Filter")
    );
    let caption = if orderby == "comment_date" {
        if desc { tr("Ordered by Comment Date, descending.") } else { tr("Ordered by Comment Date, ascending.") }
    } else {
        tr("Ordered by Comment Date, descending.")
    };
    let nonce_field = format!("<input type=\"hidden\" id=\"_ajax_fetch_list_nonce\" name=\"_ajax_fetch_list_nonce\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />{}", ctx.create_nonce("fetch-list-wp_list_comments_table"), esc_url(&request.request_uri), ctx.nonce_field("bulk-comments"));
    let table_html = table.display(&nonce_field, &type_select, "", &caption, &rows, &tr("Comments list")).replace("<tbody id=\"the-list\"\n\t\t\t\t>", "<tbody id=\"the-comment-list\" data-wp-lists=\"list:comment\">");
    let table_html = table_html.replace("<tbody id=\"the-list\"\n\t\t\t\t\t>", "<tbody id=\"the-comment-list\" data-wp-lists=\"list:comment\">");

    let no_items = if status_param == "moderated" { tr("No comments awaiting moderation.") } else { tr("No comments found.") };
    let table_html = table_html.replacen("\t</tbody>\n\n\t<tfoot>", &format!("\t</tbody>\n\t<tbody id=\"the-extra-comment-list\" data-wp-lists=\"list:comment\" style=\"display: none;\">\n\t\t<tr class=\"no-items\"><td class=\"colspanchange\" colspan=\"5\">{no_items}</td></tr>\t</tbody>\n\n\t<tfoot>"), 1);
    let title = tr("Comments");
    let mut h = format!("<div class=\"wrap\">\n<h1 class=\"wp-heading-inline\">\n{}</h1>\n\n", esc_html(&title));
    if !search.is_empty() {
        h.push_str(&format!("<span class=\"subtitle\">{}</span>\n", sprintf(&tr("Search results for: %s"), &[&format!("<strong>{}</strong>", esc_html(&search))])));
    }
    h.push_str("\n<hr class=\"wp-header-end\">\n\n");
    h.push_str(&notice(ctx));
    h.push('\n');
    h.push_str(&views(&tr("Filter comments list"), &links));
    h.push_str(&format!(
        "\n<form id=\"comments-form\" method=\"get\">\n\n<p class=\"search-box\">\n\t<label class=\"screen-reader-text\" for=\"comment-search-input\">{}</label>\n\t<input type=\"search\" id=\"comment-search-input\" name=\"s\" value=\"{}\" />\n\t\t<input type=\"submit\" id=\"search-submit\" class=\"button button-compact\" value=\"{}\"  /></p>\n\t\t\n<input type=\"hidden\" name=\"comment_status\" value=\"{}\" />\n<input type=\"hidden\" name=\"pagegen_timestamp\" value=\"{}\" />\n\n<input type=\"hidden\" name=\"_total\" value=\"{total}\" />\n<input type=\"hidden\" name=\"_per_page\" value=\"{per_page}\" />\n<input type=\"hidden\" name=\"_page\" value=\"{paged}\" />\n\n\n{}\n</form>\n",
        tr("Search Comments:"),
        esc_attr(&search),
        esc_attr(&tr("Search Comments")),
        esc_attr(&status_param),
        site.date("Y-m-d H:i:s", ctx.env.now),
        table_html
    ));
    h.push_str("</div>
");
    h.push_str(&reply_form(ctx));
    let screen = Screen { file: "edit-comments.php".into(), parent_file: "edit-comments.php".into(), submenu_file: Some("edit-comments.php".into()), title, help: Some(help(ctx)), ..Default::default() };
    render(ctx, &screen, &h, &Assets { scripts: vec!["admin-comments".into(), "wp-lists".into()], ..Default::default() }).await
}

/// `wp_comment_reply()`: the inline editor and reply form.
fn reply_form(ctx: &Ctx<'_>) -> String {
    let template = include_str!("reply_form.tpl");
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("@@") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("@@") else { break };
        let text = ctx.tr(&after[..end]);
        out.push_str(&text.replace("%s", "<strong></strong>"));
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out.replace("{{reply_nonce}}", &ctx.create_nonce("replyto-comment")).replace("{{unfiltered}}", &ctx.create_nonce("unfiltered-html-comment")).replace("{{editor_css}}", &format!("{}/wp-includes/css/editor.min.css?ver={}", ctx.site().settings.siteurl.trim_end_matches('/'), ctx.env.loader.version))
}

/// The bulk actions of the list.
async fn bulk(ctx: &Ctx<'_>, action: &str) -> Option<AdminResponse> {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    if !request.post || action.is_empty() || action == "-1" {
        return None;
    }
    if !ctx.verify_nonce("bulk-comments") {
        return Some(die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired.")));
    }
    if !ctx.can("moderate_comments") {
        return Some(forbidden(ctx, "Sorry, you are not allowed to edit comments."));
    }
    let Some(writer) = ctx.env.writer else { return Some(read_only(ctx)) };
    let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let ids: Vec<u64> = request.form.iter().filter(|(k, _)| k == "delete_comments[]").filter_map(|(_, v)| v.parse().ok()).collect();
    let (mut approved, mut unapproved, mut spammed, mut unspammed, mut trashed, mut untrashed, mut deleted) = (0, 0, 0, 0, 0, 0, 0);
    for id in &ids {
        match action {
            "approve" => {
                if content.set_comment_status(*id, "approve").await.unwrap_or(false) {
                    approved += 1;
                }
            }
            "unapprove" => {
                if content.set_comment_status(*id, "hold").await.unwrap_or(false) {
                    unapproved += 1;
                }
            }
            "spam" => {
                if content.spam_comment(*id).await.unwrap_or(false) {
                    spammed += 1;
                }
            }
            "unspam" => {
                if content.unspam_comment(*id).await.unwrap_or(false) {
                    unspammed += 1;
                }
            }
            "trash" => {
                if content.trash_comment(*id).await.unwrap_or(false) {
                    trashed += 1;
                }
            }
            "untrash" => {
                if content.untrash_comment(*id).await.unwrap_or(false) {
                    untrashed += 1;
                }
            }
            "delete" => {
                if content.delete_comment(*id).await.unwrap_or(false) {
                    deleted += 1;
                }
            }
            _ => {}
        }
    }
    let mut url = ctx.admin_url("edit-comments.php");
    for (name, value) in [("approved", approved), ("unapproved", unapproved), ("spammed", spammed), ("unspammed", unspammed), ("trashed", trashed), ("untrashed", untrashed), ("deleted", deleted)] {
        if value > 0 {
            url = add_query_arg(&url, name, &value.to_string());
        }
    }
    if let Some(status) = request.get("comment_status").or(request.post_field("comment_status")).filter(|s| *s != "all") {
        url = add_query_arg(&url, "comment_status", status);
    }
    Some(redirect(&url))
}

/// `comment.php`.
pub async fn comment_php(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let action = request.request("action").unwrap_or("editcomment").to_owned();
    let id: u64 = request.request("c").or(request.request("comment_ID")).and_then(|v| v.parse().ok()).unwrap_or(0);
    let Some(comment) = ctx.env.db.comment_any(id).await.ok().flatten() else {
        return die(ctx.env, 200, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Invalid comment ID.")));
    };
    if !ctx.can("edit_posts") || !ctx.can("moderate_comments") && !ctx.can("edit_comment") && !ctx.can("edit_others_posts") {
        return forbidden(ctx, "Sorry, you are not allowed to edit comments on this post.");
    }
    match action.as_str() {
        "editcomment" => edit_form(ctx, &comment).await,
        "editedcomment" => {
            if !ctx.verify_nonce(&format!("update-comment_{id}")) {
                return die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
            }
            let Some(writer) = ctx.env.writer else { return read_only(ctx) };
            let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
            let field = |name: &str| request.post_field(name).map(str::to_owned);
            let date = match (field("aa"), field("mm"), field("jj"), field("hh"), field("mn"), field("ss")) {
                (Some(aa), Some(mm), Some(jj), Some(hh), Some(mn), ss) => Some(format!("{:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}", aa, mm, jj, hh, mn, ss.unwrap_or_else(|| "00".into()))),
                _ => None,
            };
            let update = CommentUpdate {
                author: field("newcomment_author"),
                author_email: field("newcomment_author_email"),
                author_url: field("newcomment_author_url"),
                content: field("content"),
                status: field("comment_status"),
                date: date.filter(|d| crate::content::valid_date(d)),
            };
            let _ = content.update_comment(id, &update).await;
            if let Some(parent) = field("comment_parent").and_then(|p| p.parse::<u64>().ok()) {
                let _ = writer.update_rows("comments", "comment_ID", id, &[("comment_parent", rustpress_db::Val::Int(parent as i64))]).await;
            }
            redirect(&ctx.admin_url(&format!("edit-comments.php?p={}#comment-{id}", comment.comment_post_id)))
        }
        "approvecomment" | "unapprovecomment" | "spamcomment" | "unspamcomment" | "trashcomment" | "untrashcomment" | "deletecomment" => {
            let nonce_action = if matches!(action.as_str(), "approvecomment" | "unapprovecomment") { format!("approve-comment_{id}") } else { format!("delete-comment_{id}") };
            if !ctx.verify_nonce(&nonce_action) {
                return die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
            }
            let Some(writer) = ctx.env.writer else { return read_only(ctx) };
            let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
            let param = match action.as_str() {
                "approvecomment" => {
                    let _ = content.set_comment_status(id, "approve").await;
                    "approved"
                }
                "unapprovecomment" => {
                    let _ = content.set_comment_status(id, "hold").await;
                    "unapproved"
                }
                "spamcomment" => {
                    let _ = content.spam_comment(id).await;
                    "spammed"
                }
                "unspamcomment" => {
                    let _ = content.unspam_comment(id).await;
                    "unspammed"
                }
                "trashcomment" => {
                    let _ = content.trash_comment(id).await;
                    "trashed"
                }
                "untrashcomment" => {
                    let _ = content.untrash_comment(id).await;
                    "untrashed"
                }
                _ => {
                    let _ = content.delete_comment(id).await;
                    "deleted"
                }
            };
            let url = add_query_arg(&ctx.admin_url("edit-comments.php"), param, "1");
            redirect(&url)
        }
        _ => die(ctx.env, 200, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Unknown action."))),
    }
}

async fn edit_form(ctx: &Ctx<'_>, comment: &Comment) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let id = comment.comment_id;
    let post = ctx.env.db.post(comment.comment_post_id).await.ok().flatten();
    let link = comment_link(ctx, comment).await;
    let mut h = format!("{}<div class=\"wrap\">\n<h1>{}</h1>\n\n<div id=\"poststuff\">\n", ctx.nonce_field(&format!("update-comment_{id}")), tr("Edit Comment"));
    h.push_str(&format!("<input type=\"hidden\" name=\"action\" value=\"editedcomment\" />\n<input type=\"hidden\" name=\"comment_ID\" value=\"{id}\" />\n<input type=\"hidden\" name=\"comment_post_ID\" value=\"{}\" />\n\n", comment.comment_post_id));
    h.push_str("<div id=\"post-body\" class=\"metabox-holder columns-2\">\n<div id=\"post-body-content\" class=\"edit-form-section edit-comment-section\">\n<div class=\"inside\">\n");
    h.push_str(&format!("\t<div id=\"comment-link-box\">\n\t\t<strong>{}</strong>\n\t\t<span id=\"sample-permalink\">\n\t\t\t<a href=\"{}\">\n\t\t\t\t{}\t\t\t</a>\n\t\t</span>\n\t</div>\n</div>\n", tr("Permalink:"), esc_url(&link), esc_html(&link)));
    h.push_str(&format!(
        "<div id=\"namediv\" class=\"stuffbox\">\n<div class=\"inside\">\n<h2 class=\"edit-comment-author\">{}</h2>\n<fieldset>\n<legend class=\"screen-reader-text\">\n\t{}</legend>\n<table class=\"form-table editcomment\" role=\"presentation\">\n<tbody>\n<tr>\n\t<td class=\"first\"><label for=\"name\">{}</label></td>\n\t<td><input type=\"text\" name=\"newcomment_author\" size=\"30\" value=\"{}\" id=\"name\" /></td>\n</tr>\n<tr>\n\t<td class=\"first\"><label for=\"email\">{}</label></td>\n\t<td>\n\t\t<input type=\"text\" name=\"newcomment_author_email\" size=\"30\" class=\"code\" value=\"{}\" id=\"email\" />\n\t</td>\n</tr>\n<tr>\n\t<td class=\"first\"><label for=\"newcomment_author_url\">{}</label></td>\n\t<td>\n\t\t<input type=\"text\" id=\"newcomment_author_url\" name=\"newcomment_author_url\" size=\"30\" class=\"code\" value=\"{}\" />\n\t</td>\n</tr>\n</tbody>\n</table>\n</fieldset>\n</div>\n</div>\n\n",
        tr("Author"),
        tr("Comment Author"),
        tr("Name"),
        esc_attr(&comment.comment_author),
        tr("Email"),
        esc_attr(&comment.comment_author_email),
        tr("URL"),
        esc_attr(&comment.comment_author_url)
    ));
    h.push_str(&format!(
        "<div id=\"postdiv\" class=\"postarea\">\n<label for=\"content\" class=\"screen-reader-text\">\n\t{}</label>\n<div id=\"wp-content-wrap\" class=\"wp-core-ui wp-editor-wrap html-active\"><div id=\"wp-content-editor-container\" class=\"wp-editor-container\"><div id=\"qt_content_toolbar\" class=\"quicktags-toolbar hide-if-no-js\"></div><textarea class=\"wp-editor-area\" rows=\"20\" cols=\"40\" name=\"content\" id=\"content\">{}</textarea></div>\n</div>\n\n</div>\n</div><!-- /post-body-content -->\n\n",
        tr("Comment"),
        esc_textarea(&comment.comment_content)
    ));
    // The save box.
    let status_label = match comment.comment_approved.as_str() {
        "1" => tr("Approved"),
        "0" => tr("Pending"),
        "spam" => tr("Spam"),
        _ => tr("Trash"),
    };
    let radio = |value: &str, label: String| format!("<label><input type=\"radio\"{} name=\"comment_status\" value=\"{value}\" />{label}</label>", if comment.comment_approved == value { " checked='checked'" } else { "" });
    let comment_time = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&comment.comment_date_gmt).unwrap_or(0);
    let submitted = sprintf(&tr("Submitted on: %s"), &[&format!("<b>{}</b>", site.date(&format!("{} {}", tr("M j, Y"), tr("\\a\\t H:i")).replace("\\a\\t", "\\a\\t"), comment_time))]);
    let response_to = match &post {
        Some(post) => format!("{} <b><a href='{}'>{}</a></b>", tr("In response to:"), esc_url(&ctx.admin_url(&format!("post.php?post={}&action=edit", post.id))), esc_html(&rustpress_blocks::content::the_title_for(site, post))),
        None => String::new(),
    };
    let parent_row = {
        let display = if comment.comment_parent != 0 {
            match ctx.env.db.comment_any(comment.comment_parent).await.ok().flatten() {
                Some(parent) => format!("{} <b><a href=\"{}\">{}</a></b>", tr("In reply to:"), esc_url(&comment_link(ctx, &parent).await), esc_html(&parent.comment_author)),
                None => String::new(),
            }
        } else {
            String::new()
        };
        let siblings = ctx.env.db.list_comments_admin(&CommentListQuery { status: "1".into(), post_id: Some(comment.comment_post_id), kind: "comment".into(), orderby: "comment_ID".into(), ..Default::default() }).await.map(|(c, _)| c).unwrap_or_default();
        let mut options = String::new();
        for sibling in siblings.iter().filter(|c| c.comment_id != id) {
            let plain = rustpress_core::formatting::wp_strip_all_tags(&sibling.comment_content);
            let excerpt: String = if plain.chars().count() > 50 { format!("{}&hellip;", plain.chars().take(50).collect::<String>()) } else { plain };
            options.push_str(&format!(
                "\t<option value='{}' data-author='{}'{}>{}</option>\n",
                sibling.comment_id,
                esc_attr(&sibling.comment_author),
                if sibling.comment_id == comment.comment_parent { " selected='selected'" } else { "" },
                sprintf(&tr("%1$s: %2$s"), &[&esc_html(&sibling.comment_author), &excerpt])
            ));
        }
        format!(
            "\n<span id=\"comment-parent-display\">\n{display}</span>\n<a href=\"#edit_comment_parent\" class=\"edit-comment-parent hide-if-no-js\"><span aria-hidden=\"true\">{edit}</span> <span class=\"screen-reader-text\">\n\t{edit_parent}</span></a>\n<fieldset id=\"comment-parent-div\" class=\"hide-if-js\">\n<legend class=\"screen-reader-text\">\n\t{parent_legend}</legend>\n<label for=\"comment_parent\">{parent_legend}</label>\n<select name=\"comment_parent\" id=\"comment_parent\">\n\t<option value=\"0\">\n\t\t{none}\t</option>\n\t\t{options}</select>\n<input type=\"hidden\" id=\"hidden_comment_parent\" value=\"{parent}\" />\n<p>\n<a href=\"#edit_comment_parent\" class=\"save-comment-parent hide-if-no-js button\">{ok}</a>\n<a href=\"#edit_comment_parent\" class=\"cancel-comment-parent hide-if-no-js button-cancel\">{cancel}</a>\n</p>\n</fieldset>\n",
            edit = tr("Edit"),
            edit_parent = tr("Edit parent comment"),
            parent_legend = tr("Parent comment"),
            none = tr("None (top-level comment)"),
            parent = comment.comment_parent,
            ok = tr("OK"),
            cancel = tr("Cancel")
        )
    };
    h.push_str(&format!(
        "<div id=\"postbox-container-1\" class=\"postbox-container\">\n<div id=\"submitdiv\" class=\"stuffbox\" >\n<h2>{save}</h2>\n<div class=\"inside\">\n<div class=\"submitbox\" id=\"submitcomment\">\n<div id=\"minor-publishing\">\n\n<div id=\"misc-publishing-actions\">\n\n<div class=\"misc-pub-section misc-pub-comment-status\" id=\"comment-status\">\n{status}: <span id=\"comment-status-display\">\n{status_label}</span>\n\n<fieldset id=\"comment-status-radio\">\n<legend class=\"screen-reader-text\">\n\t{status_legend}</legend>\n{approved}<br />\n{pending}<br />\n{spam}\n</fieldset>\n</div><!-- .misc-pub-section -->\n\n<div class=\"misc-pub-section curtime misc-pub-curtime\">\n<span id=\"timestamp\">\n{submitted}</span>\n<a href=\"#edit_timestamp\" class=\"edit-timestamp hide-if-no-js\"><span aria-hidden=\"true\">{edit}</span> <span class=\"screen-reader-text\">\n\t{edit_date}</span></a>\n<fieldset id='timestampdiv' class='hide-if-js'>\n<legend class=\"screen-reader-text\">\n\t{date_legend}</legend>\n{timestamp}\n\t</fieldset>\n</div>\n\n\n<div class=\"misc-pub-section misc-pub-response-to\">\n\t{response_to}</div>\n\n<div class=\"misc-pub-section misc-pub-reply-to\">{parent_row}</div>\n\n</div> <!-- misc actions -->\n<div class=\"clear\"></div>\n</div>\n\n<div id=\"major-publishing-actions\">\n<div id=\"delete-action\">\n<a class='submitdelete deletion' href='comment.php?action=trashcomment&amp;c={id}&amp;_wpnonce={nonce}'>{trash}</a>\n</div>\n<div id=\"publishing-action\">\n<input type=\"submit\" name=\"save\" id=\"save\" class=\"button button-primary button-large\" value=\"{update}\"  /></div>\n<div class=\"clear\"></div>\n</div>\n</div>\n</div>\n</div><!-- /submitdiv -->\n</div>\n\n",
        save = tr("Save"),
        status = tr("Status"),
        status_legend = tr("Comment status"),
        approved = radio("1", tr("Approved")),
        pending = radio("0", tr("Pending")),
        spam = radio("spam", tr("Spam")),
        edit = tr("Edit"),
        edit_date = tr("Edit date and time"),
        date_legend = tr("Date and time"),
        timestamp = super::post_edit::timestamp_fields(ctx, &comment.comment_date),
        nonce = ctx.create_nonce(&format!("delete-comment_{id}")),
        trash = tr("Move to Trash"),
        update = tr("Update")
    ));
    h.push_str(&format!("<div id=\"postbox-container-2\" class=\"postbox-container\">\n<div id=\"normal-sortables\" class=\"meta-box-sortables\"></div></div>\n\n<input type=\"hidden\" name=\"c\" value=\"{id}\" />\n<input type=\"hidden\" name=\"p\" value=\"{}\" />\n<input name=\"referredby\" type=\"hidden\" id=\"referredby\" value=\"\" />\n<input type=\"hidden\" name=\"_wp_original_http_referer\" value=\"\" /><input type=\"hidden\" name=\"noredir\" value=\"1\" />\n\n</div><!-- /post-body -->\n</div>\n</div>\n</form>\n\n<script>\ntry{{document.post.name.focus();}}catch(e){{}}\n</script>\n\t\n", comment.comment_post_id));
    // The form wraps everything from the nonce on.
    let h = format!("<form name=\"post\" action=\"comment.php\" method=\"post\" id=\"post\">\n{h}");
    let screen = Screen { file: "comment.php".into(), parent_file: "edit-comments.php".into(), submenu_file: Some("edit-comments.php".into()), title: tr("Edit Comment"), help: Some(help(ctx)), ..Default::default() };
    render(ctx, &screen, &h, &Assets { scripts: vec!["comment".into(), "quicktags".into()], footer: "<script>window.quicktags && quicktags({id: 'content'});</script>\n".into(), ..Default::default() }).await
}
