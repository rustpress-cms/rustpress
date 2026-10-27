//! `wp-admin/index.php`: the dashboard and its widgets (`wp-admin/includes/dashboard.php`).

use rustpress_core::OrLog;
use std::collections::HashMap;

use rustpress_core::datetime::SiteTimezone;
use rustpress_core::formatting::{convert_chars, esc_attr, esc_html, esc_url, strip_tags, wp_trim_words, wptexturize};
use rustpress_core::i18n::sprintf;
use rustpress_db::{Comment, CommentListQuery, Order, OrderBy, Post, PostQuery};

use super::{AdminResponse, Ctx, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};

/// `Theme Name:` of the active theme's `style.css`.
fn theme_name(ctx: &Ctx<'_>) -> String {
    let slug = &ctx.site().settings.stylesheet;
    let root = ctx.env.loader.includes.parent();
    let css = root.map(|root| root.join("wp-content").join("themes").join(slug).join("style.css")).and_then(|path| std::fs::read(path).ok()).unwrap_or_default();
    let head = String::from_utf8_lossy(&css[..css.len().min(8192)]).into_owned();
    head.lines().find_map(|line| line.trim_start_matches([' ', '*', '\t']).strip_prefix("Theme Name:").map(|name| name.trim().to_owned())).unwrap_or_else(|| slug.clone())
}

async fn count(ctx: &Ctx<'_>, post_type: &str, status: &str) -> u64 {
    let query = PostQuery { post_types: vec![post_type.into()], statuses: vec![status.into()], ..Default::default() };
    ctx.env.db.count_posts(&query).await.unwrap_or(0)
}

/// The template with its `@@text@@` parts translated.
fn translate(ctx: &Ctx<'_>, template: &str) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("@@") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("@@") else { break };
        out.push_str(&ctx.tr(&after[..end]));
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// What a dashboard widget is: `(id, title, aria label, column)`.
struct Widget {
    id: &'static str,
    title: String,
    aria: String,
    column: &'static str,
}

/// `do_meta_boxes()`: the box around a widget.
fn postbox(ctx: &Ctx<'_>, widget: &Widget, closed: bool, hidden: bool, body: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let id = widget.id;
    // `'postbox ' . postbox_classes() . $hidden_class`
    let classes = format!("postbox {}{}", if closed { "closed" } else { "" }, if hidden { " hide-if-js" } else { "" });
    let tooltip = |class: &str, label: &str, extra: &str, indicator: &str, bubble: &str| {
        format!(
            "<span class=\"wp-tooltip wp-is-tooltip\">\n\t\t\t\t<button aria-label=\"{label}\" type=\"button\" class=\"{class} wp-tooltip__toggle\"{extra} aria-describedby=\"{id}-title\">\n\t\t\t\t\t\t\t<span class=\"screen-reader-text\">{label}</span>\n\t\t\t\t\t\t\t<span class=\"{indicator}\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t</button>\n\t\t\t\t<span popover=\"hint\" id=\"{id}-{bubble}\" class=\"wp-tooltip__bubble\" role=\"tooltip\"><span id=\"{id}-{bubble}-text\" class=\"wp-tooltip__text\">{label}</span></span></span>"
        )
    };
    let expanded = if closed { "false" } else { "true" };
    let higher = tooltip("handle-order-higher", &tr("Move up"), "", "order-higher-indicator", "handle-order-higher-description");
    let lower = tooltip("handle-order-lower", &tr("Move down"), "", "order-lower-indicator", "handle-order-lower-description");
    let toggle = tooltip("handlediv", &tr("Show or hide panel"), &format!(" aria-expanded=\"{expanded}\""), "toggle-indicator", "handlediv");
    format!(
        "<div id=\"{id}\" class=\"{classes}\"  role=\"region\" aria-label=\"{}\">\n<div class=\"postbox-header\"><h2 class=\"hndle\" id=\"{id}-title\">{}</h2>\n<div class=\"handle-actions hide-if-no-js\">{higher}{lower}{toggle}</div></div><div class=\"inside\">\n{body}</div>\n</div>\n",
        esc_attr(&widget.aria),
        widget.title
    )
}

// ------------------------------------------------------------------ widgets

/// The counts `WP_Site_Health` caches in the transient `health-check-site-status-result`.
async fn health_counts(ctx: &Ctx<'_>) -> Option<(i64, i64, i64)> {
    let site = ctx.site();
    let raw = site.options.fetch(&site.db, "_transient_health-check-site-status-result").await.ok().flatten()?;
    // A transient that has a timeout has expired when the timeout is over.
    if let Some(timeout) = site.options.fetch(&site.db, "_transient_timeout_health-check-site-status-result").await.ok().flatten().and_then(|t| t.parse::<i64>().ok())
        && timeout < ctx.env.now
    {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let count = |key: &str| value.get(key).and_then(serde_json::Value::as_i64).unwrap_or(0);
    Some((count("good"), count("recommended"), count("critical")))
}

/// `wp_dashboard_site_health()`.
async fn site_health(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let counts = health_counts(ctx).await;
    let link = esc_url(&ctx.admin_url("site-health.php"));
    let (_, recommended, critical) = counts.unwrap_or((0, 0, 0));
    let total = recommended + critical;
    let label = if counts.is_none() { tr("No information yet&hellip;") } else { tr("Results are still loading&hellip;") };
    let body = match counts {
        None => format!(
            "\t\t\t\t\t\t\t\t\t\t\t<p>\n\t\t\t\t\t{}\t\t\t\t</p>\n\t\t\t\n",
            sprintf(&tr("Site health checks will automatically run periodically to gather information about your site. You can also <a href=\"%s\">visit the Site Health screen</a> to gather information about your site now."), &[&link])
        ),
        Some(_) => {
            let sentence = if total <= 0 {
                tr("Great job! Your site currently passes all site health checks.")
            } else if critical == 1 {
                tr("Your site has a critical issue that should be addressed as soon as possible to improve its performance and security.")
            } else if critical > 1 {
                tr("Your site has critical issues that should be addressed as soon as possible to improve its performance and security.")
            } else if recommended == 1 {
                tr("Your site&#8217;s health is looking good, but there is still one thing you can do to improve its performance and security.")
            } else {
                tr("Your site&#8217;s health is looking good, but there are still some things you can do to improve its performance and security.")
            };
            let mut body = format!("\t\t\t\t\t\t\t<p>\n\t\t\t\t\t\t\t\t\t\t\t{sentence}\t\t\t\t\t\t\t\t\t</p>\n\t\t\t\n");
            if total > 0 {
                let text = ctx.site().trn("Take a look at the <strong>%1$d item</strong> on the <a href=\"%2$s\">Site Health screen</a>.", "Take a look at the <strong>%1$d items</strong> on the <a href=\"%2$s\">Site Health screen</a>.", total);
                body.push_str(&format!("\t\t\t\t\t\t\t<p>\n\t\t\t\t\t{}\t\t\t\t</p>\n", sprintf(&text, &[&total.to_string(), &link])));
            }
            body
        }
    };
    format!(
        "\t<div class=\"health-check-widget\">\n\t\t<div class=\"health-check-widget-title-section site-health-progress-wrapper loading hide-if-no-js\">\n\t\t\t<div class=\"site-health-progress\">\n\t\t\t\t<svg aria-hidden=\"true\" focusable=\"false\" width=\"100%\" height=\"100%\" viewBox=\"0 0 200 200\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n\t\t\t\t\t<circle r=\"90\" cx=\"100\" cy=\"100\" fill=\"transparent\" stroke-dasharray=\"565.48\" stroke-dashoffset=\"0\"></circle>\n\t\t\t\t\t<circle id=\"bar\" r=\"90\" cx=\"100\" cy=\"100\" fill=\"transparent\" stroke-dasharray=\"565.48\" stroke-dashoffset=\"0\"></circle>\n\t\t\t\t</svg>\n\t\t\t</div>\n\t\t\t<div class=\"site-health-progress-label\">\n\t\t\t\t\t\t\t\t\t{label}\t\t\t\t\t\t\t</div>\n\t\t</div>\n\n\t\t<div class=\"site-health-details\">\n{body}\t\t\t\t\t</div>\n\t</div>\n\n\t"
    )
}

/// `wp_dashboard_right_now()`.
async fn at_a_glance(ctx: &Ctx<'_>) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let mut items = String::new();
    for post_type in ["post", "page"] {
        let n = count(ctx, post_type, "publish").await;
        if n == 0 {
            continue;
        }
        let text = if post_type == "post" { site.trn("%s Published post", "%s Published posts", n as i64) } else { site.trn("%s Published page", "%s Published pages", n as i64) };
        let text = sprintf(&text, &[&site.number_i18n(n)]);
        if ctx.can(if post_type == "post" { "edit_posts" } else { "edit_pages" }) {
            let url = esc_url(&ctx.admin_url(&format!("edit.php?post_status=publish&post_type={post_type}")));
            items.push_str(&format!("<li class=\"{post_type}-count\"><a href=\"{url}\">{}</a></li>", esc_html(&text)));
        } else {
            items.push_str(&format!("<li class=\"{post_type}-count\"><span>{text}</span></li>"));
        }
    }
    let counts = ctx.env.db.comment_counts(None).await.unwrap_or_default();
    if counts.approved > 0 || counts.moderated > 0 {
        let approved = sprintf(&site.trn("%s Comment", "%s Comments", counts.approved as i64), &[&site.number_i18n(counts.approved)]);
        let moderated = sprintf(&site.trn("%s Comment in moderation", "%s Comments in moderation", counts.moderated as i64), &[&site.number_i18n(counts.moderated)]);
        items.push_str(&format!(
            "\t\t<li class=\"comment-count\">\n\t\t\t<a href=\"edit-comments.php\">{approved}</a>\n\t\t</li>\n\t\t\t\t<li class=\"comment-mod-count{}\">\n\t\t\t<a href=\"edit-comments.php?comment_status=moderated\" class=\"comments-in-moderation-text\">{moderated}</a>\n\t\t</li>\n\t\t",
            if counts.moderated == 0 { " hidden" } else { "" }
        ));
    }
    let theme = theme_name(ctx);
    let theme_html = if ctx.can("switch_themes") { format!("<a href=\"themes.php\">{}</a>", esc_html(&theme)) } else { esc_html(&theme) };
    let message = sprintf(&tr("WordPress %1$s running %2$s theme."), &[&ctx.env.loader.version, &theme_html]);
    let mut out = format!("\t<div class=\"main\">\n\t<ul>\n\t{items}\t</ul>\n\t<p id='wp-version-message'><span id=\"wp-version\">{message}</span></p>");
    if ctx.can("manage_options") && site.options.get("blog_public") == Some("0") {
        out.push_str(&format!("<p class='search-engines-info'><a href='options-reading.php'>{}</a></p>", tr("Search engines discouraged")));
    }
    out.push_str("\t</div>\n\t");
    out
}

/// `get_the_time( $format )` of a post: the local time of `post_date`.
fn post_time(ctx: &Ctx<'_>, post: &Post, format: &str) -> String {
    let ts = ctx.site().timezone.local_timestamp(&post.post_date).unwrap_or(0);
    ctx.site().date(format, ts)
}

/// `_draft_or_post_title()`, escaped.
fn draft_or_title(ctx: &Ctx<'_>, post: &Post) -> String {
    let title = rustpress_blocks::content::the_title_for(ctx.site(), post);
    if title.is_empty() { esc_html(&ctx.tr("(no title)")) } else { esc_html(&title) }
}

/// `wp_dashboard_recent_posts()`; empty when there is nothing to list.
async fn recent_posts(ctx: &Ctx<'_>, status: &str, order: Order, title: &str, id: &str) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let query = PostQuery { post_types: vec!["post".into()], statuses: vec![status.into()], order_by: OrderBy::Date, order, limit: Some(5), ..Default::default() };
    let posts = ctx.env.db.posts(&query).await.unwrap_or_default();
    if posts.is_empty() {
        return String::new();
    }
    let now = ctx.env.now;
    let today = site.date("Y-m-d", now);
    let tomorrow = site.date("Y-m-d", now + 86_400);
    let year = site.date("Y", now);
    let time_format = site.options.string("time_format", "g:i a");
    let mut out = format!("<div id=\"{id}\" class=\"activity-block\"><h3>{title}</h3><ul>");
    for post in &posts {
        // The post time as if it were UTC, the way `get_the_time( 'U' )` returns it.
        let shifted = SiteTimezone::gmt_timestamp(&post.post_date).unwrap_or(0);
        let day = site.date_utc("Y-m-d", shifted);
        let date = if day == today {
            tr("Today")
        } else if day == tomorrow {
            tr("Tomorrow")
        } else if site.date_utc("Y", shifted) != year {
            site.date_utc(&tr("M jS Y"), shifted)
        } else {
            site.date_utc(&tr("M jS"), shifted)
        };
        let link = if ctx.can("edit_posts") { esc_url(&ctx.admin_url(&format!("post.php?post={}&action=edit", post.id))).replace("&#038;", "&amp;") } else { esc_url(&site.permalink(post).await.unwrap_or_default()) };
        let title = draft_or_title(ctx, post);
        let when = sprintf(&site.trx("%1$s, %2$s", "dashboard"), &[&date, &post_time(ctx, post, &time_format)]);
        out.push_str(&format!("<li><span>{when}</span> <a href=\"{link}\" aria-label=\"{}\">{title}</a></li>", esc_attr(&sprintf(&tr("Edit &#8220;%s&#8221;"), &[&title]))));
    }
    out.push_str("</ul></div>");
    out
}

/// `comment_class()` for the dashboard list.
async fn comment_classes(ctx: &Ctx<'_>, comment: &Comment, index: usize, status: &str) -> String {
    let mut classes: Vec<String> = vec![if comment.comment_type.is_empty() { "comment".to_owned() } else { comment.comment_type.clone() }];
    if comment.user_id > 0
        && let Ok(Some(user)) = ctx.env.db.user(comment.user_id).await {
            classes.push("byuser".into());
            classes.push(format!("comment-author-{}", rustpress_core::formatting::sanitize_html_class(&user.user_nicename)));
            if let Ok(Some(post)) = ctx.env.db.post(comment.comment_post_id).await
                && post.post_author == comment.user_id
            {
                classes.push("bypostauthor".into());
            }
        }
    if index % 2 == 1 {
        classes.extend(["odd".into(), "alt".into()]);
        classes.extend(["thread-odd".into(), "thread-alt".into()]);
    } else {
        classes.extend(["even".into(), "thread-even".into()]);
    }
    classes.push("depth-1".into());
    classes.push("comment-item".into());
    classes.push(status.to_owned());
    classes.join(" ")
}

/// `get_comment_author_link()`.
fn author_link(ctx: &Ctx<'_>, comment: &Comment, author: &str) -> String {
    let url = comment.comment_author_url.trim();
    if url.is_empty() || url == "http://" {
        return author.to_owned();
    }
    let home = ctx.site().settings.home.clone();
    let host = |u: &str| u.split("://").nth(1).unwrap_or(u).split(['/', '?', '#']).next().unwrap_or_default().to_lowercase();
    let internal = host(url) == host(&home);
    let rel = if internal { "ugc" } else { "ugc external nofollow" };
    format!("<a href=\"{}\" class=\"url\" rel=\"{rel}\">{author}</a>", esc_url(url))
}

/// `_wp_dashboard_recent_comments_row()`.
pub(super) async fn comment_row(ctx: &Ctx<'_>, comment: &Comment, index: usize) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let id = comment.comment_id;
    let post_id = comment.comment_post_id;
    let status = match comment.comment_approved.as_str() {
        "1" => "approved",
        "spam" => "spam",
        "trash" => "trash",
        _ => "unapproved",
    };
    let post = ctx.env.db.post(post_id).await.ok().flatten();
    let post_link = match &post {
        Some(post) if post_id > 0 => format!("<a href=\"{}\">{}</a>", esc_url(&site.permalink(post).await.unwrap_or_default()), draft_or_title(ctx, post)),
        _ => String::new(),
    };
    let mut actions = String::new();
    if ctx.can("moderate_comments") || ctx.can("edit_posts") {
        let approve_nonce = ctx.create_nonce(&format!("approve-comment_{id}"));
        let del_nonce = ctx.create_nonce(&format!("delete-comment_{id}"));
        let url = |action: &str, nonce: &str| esc_url(&format!("comment.php?action={action}&p={post_id}&c={id}&_wpnonce={nonce}"));
        let mut parts: Vec<(&str, String)> = Vec::new();
        parts.push(("approve", format!("<a href=\"{}\" data-wp-lists=\"dim:the-comment-list:comment-{id}:unapproved:e7e7d3:e7e7d3:new=approved\" class=\"vim-a aria-button-if-js\" aria-label=\"{}\">{}</a>", url("approvecomment", &approve_nonce), esc_attr(&tr("Approve this comment")), tr("Approve"))));
        parts.push(("unapprove", format!("<a href=\"{}\" data-wp-lists=\"dim:the-comment-list:comment-{id}:unapproved:e7e7d3:e7e7d3:new=unapproved\" class=\"vim-u aria-button-if-js\" aria-label=\"{}\">{}</a>", url("unapprovecomment", &approve_nonce), esc_attr(&tr("Unapprove this comment")), tr("Unapprove"))));
        parts.push(("reply hide-if-no-js", format!("<button type=\"button\" onclick=\"window.commentReply && commentReply.open('{id}','{post_id}');\" class=\"vim-r button-link hide-if-no-js\" aria-label=\"{}\">{}</button>", esc_attr(&tr("Reply to this comment")), site.trx("Reply", "verb"))));
        parts.push(("edit", format!("<a href=\"comment.php?action=editcomment&amp;c={id}\" aria-label=\"{}\">{}</a>", esc_attr(&tr("Edit this comment")), tr("Edit"))));
        parts.push(("spam", format!("<a href=\"{}\" data-wp-lists=\"delete:the-comment-list:comment-{id}::spam=1\" class=\"vim-s vim-destructive aria-button-if-js\" aria-label=\"{}\">{}</a>", url("spamcomment", &del_nonce), esc_attr(&tr("Mark this comment as spam")), site.trx("Spam", "verb"))));
        parts.push(("trash", format!("<a href=\"{}\" data-wp-lists=\"delete:the-comment-list:comment-{id}::trash=1\" class=\"delete vim-d vim-destructive aria-button-if-js\" aria-label=\"{}\">{}</a>", url("trashcomment", &del_nonce), esc_attr(&tr("Move this comment to the Trash")), site.trx("Trash", "verb"))));
        let link = super::comments::comment_link(ctx, comment).await;
        let view_class = if status == "approved" { "view" } else { "view hidden" };
        parts.push((view_class, format!("<a class=\"comment-link\" href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&link), esc_attr(&tr("View this comment")), tr("View"))));
        for (i, (class, html)) in parts.iter().enumerate() {
            let separator = if ((*class == "approve" || *class == "unapprove") && i == 1) || i == 0 { "" } else { " | " };
            actions.push_str(&format!("<span class='{class}'>{separator}{html}</span>"));
        }
    }
    let author = {
        let name = if comment.comment_author.is_empty() {
            match comment.user_id {
                0 => tr("Anonymous"),
                uid => ctx.env.db.user(uid).await.ok().flatten().map_or_else(|| tr("Anonymous"), |u| u.display_name),
            }
        } else {
            comment.comment_author.clone()
        };
        esc_html(&convert_chars(&wptexturize(&name)))
    };
    let pending = format!("<span class=\"approve\">{}</span>", tr("[Pending]"));
    let avatar = if site.show_avatars() {
        let url = |s: u64| esc_url(&rustpress_blocks::content::avatar_url(&site.options, &comment.comment_author_email, s));
        Some(format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-50 photo' height='50' width='50' loading='lazy' decoding='async'/>", url(50), url(100)))
    } else {
        None
    };
    let excerpt = {
        let text = if post.as_ref().is_some_and(|p| !p.post_password.is_empty()) { tr("Password protected") } else { strip_tags(&comment.comment_content.replace(['\n', '\r'], " ")) };
        convert_chars(&wp_trim_words(&text, 20, "&hellip;"))
    };
    let classes = comment_classes(ctx, comment, index, status).await;
    let is_comment = comment.comment_type.is_empty() || comment.comment_type == "comment";
    let avatar = avatar.filter(|_| is_comment);
    let row_class = if avatar.is_some() { " has-avatar" } else { "" };
    let mut out = format!("\n\t\t<li id=\"comment-{id}\" class=\"{classes}\">\n\n\t\t\t{}", avatar.unwrap_or_default());
    if is_comment {
        let meta = if post_link.is_empty() {
            sprintf(&tr("From %1$s %2$s"), &[&format!("<cite class=\"comment-author\">{}</cite>", author_link(ctx, comment, &author)), &pending])
        } else {
            sprintf(&tr("From %1$s on %2$s %3$s"), &[&format!("<cite class=\"comment-author\">{}</cite>", author_link(ctx, comment, &author)), &post_link, &pending])
        };
        out.push_str(&format!("\n\t\t\t\n\t\t\t<div class=\"dashboard-comment-wrap has-row-actions {row_class}\">\n\t\t\t<p class=\"comment-meta\">\n\t\t\t\t{meta}\t\t\t</p>\n\n\t\t\t\t"));
    } else {
        let kind = match comment.comment_type.as_str() {
            "pingback" => tr("Pingback"),
            "trackback" => tr("Trackback"),
            other => {
                let mut c = other.chars();
                c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
            }
        };
        let kind = esc_html(&kind);
        let meta = if post_link.is_empty() { sprintf(&site.trx("%1$s %2$s", "dashboard"), &[&format!("<strong>{kind}</strong>"), &pending]) } else { sprintf(&site.trx("%1$s on %2$s %3$s", "dashboard"), &[&format!("<strong>{kind}</strong>"), &post_link, &pending]) };
        out.push_str(&format!("\n\t\t\t\t\t\t<div class=\"dashboard-comment-wrap has-row-actions\">\n\t\t\t<p class=\"comment-meta\">\n\t\t\t\t{meta}\t\t\t</p>\n\t\t\t<p class=\"comment-author\">{}</p>\n\n\t\t\t", author_link(ctx, comment, &author)));
    }
    out.push_str(&format!("\t\t\t<blockquote><p>{excerpt}</p></blockquote>\n\t\t\t"));
    if !actions.is_empty() {
        out.push_str(&format!("\t\t\t<p class=\"row-actions\">{actions}</p>\n\t\t\t"));
    }
    out.push_str("\t\t\t</div>\n\t\t</li>\n\t");
    out
}

/// `wp_dashboard_recent_comments()`; empty when there is nothing to list.
async fn recent_comments(ctx: &Ctx<'_>) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    let query = CommentListQuery { status: "all".into(), orderby: "comment_date_gmt".into(), desc: true, limit: 5, ..Default::default() };
    let comments = db.list_comments_admin(&query).await.map(|(c, _)| c).unwrap_or_default();
    if comments.is_empty() {
        return String::new();
    }
    let mut out = format!("<div id=\"latest-comments\" class=\"activity-block table-view-list\"><h3>{}</h3><ul id=\"the-comment-list\" data-wp-lists=\"list:comment\">", tr("Recent Comments"));
    // The posts and commenting users of the rows, read together.
    db.prime_posts(&comments.iter().map(|c| c.comment_post_id).collect::<Vec<_>>()).await.or_log();
    db.prime_users(&comments.iter().map(|c| c.user_id).collect::<Vec<_>>()).await.or_log();
    for (index, comment) in comments.iter().enumerate() {
        out.push_str(&comment_row(ctx, comment, index).await);
    }
    out.push_str("</ul>");
    if ctx.can("edit_posts") {
        let counts = db.comment_counts(None).await.unwrap_or_default();
        let mine = db.list_comments_admin(&CommentListQuery { status: "all".into(), user_id: Some(ctx.user.id), limit: 1, ..Default::default() }).await.map(|(_, n)| n).unwrap_or(0);
        let link = |status: &str, label: &str, class: &str, n: u64, extra: &str| {
            let url = ctx.admin_url(&format!("edit-comments.php?comment_status={status}{extra}"));
            let text = format!("{label} <span class=\"count\">(<span class=\"{class}-count\">{}</span>)</span>", site.number_i18n(n));
            (status.to_owned(), crate::list_table::view_link(&url, &text, false))
        };
        let mut links = vec![
            link("all", &tr("All"), "all", counts.all, ""),
            link("mine", &tr("Mine"), "mine", mine, &format!("&user_id={}", ctx.user.id)),
            link("moderated", &tr("Pending"), "pending", counts.moderated, ""),
            link("approved", &tr("Approved"), "approved", counts.approved, ""),
            link("spam", &tr("Spam"), "spam", counts.spam, ""),
            link("trash", &tr("Trash"), "trash", counts.trash, ""),
        ];
        if !ctx.can("moderate_comments") {
            links.retain(|(class, _)| class == "all" || class == "mine" || class == "approved");
        }
        out.push_str(&format!("<h3 class=\"screen-reader-text\">{}</h3>", tr("View more comments")));
        // `WP_Comments_List_Table::views()` without the label of the list.
        let items: Vec<String> = links.iter().map(|(class, html)| format!("\t<li class='{class}'>{html}")).collect();
        let last = items.len();
        let mut list = String::from("<ul class='subsubsub'>\n");
        for (i, item) in items.iter().enumerate() {
            list.push_str(item);
            list.push_str(if i + 1 == last { "</li>\n" } else { " |</li>\n" });
        }
        list.push_str("</ul>");
        out.push_str(&list);
    }
    // `wp_comment_trashnotice()` ends with the indentation of its closing `<?php`, then the widget closes.
    let form = super::comments::reply_form_html(ctx, true);
    match form.strip_suffix("\n\n") {
        Some(form) => out.push_str(&format!("{form}\n\t")),
        None => out.push_str(&form),
    }
    out.push_str("</div>");
    out
}

/// `wp_dashboard_site_activity()`.
async fn activity(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let future = recent_posts(ctx, "future", Order::Asc, &tr("Publishing Soon"), "future-posts").await;
    let published = recent_posts(ctx, "publish", Order::Desc, &tr("Recently Published"), "published-posts").await;
    let comments = recent_comments(ctx).await;
    if future.is_empty() && published.is_empty() && comments.is_empty() {
        return format!("<div id=\"activity-widget\"><div class=\"no-activity\"><p>{}</p></div></div>", tr("No activity yet!"));
    }
    // The comment markup closes its own containers.
    format!("<div id=\"activity-widget\">{future}{published}{comments}</div>")
}

/// `wp_dashboard_quick_press()`.
pub async fn quick_press(ctx: &Ctx<'_>, message: Option<(&str, &str)>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let post_id = quick_draft_id(ctx).await;
    let notice = message.map_or_else(String::new, |(text, kind)| format!("<div class=\"notice notice-{kind}\" role=\"alert\"><p>{text}</p></div>
"));
    let mut out = format!(
        "\n\t<form name=\"post\" action=\"{}\" method=\"post\" id=\"quick-press\" class=\"hide-if-no-js\">\n\n\t\t{notice}\n\t\t<div class=\"input-text-wrap\" id=\"title-wrap\">\n\t\t\t<label for=\"title\">\n\t\t\t\t{}\t\t\t</label>\n\t\t\t<input type=\"text\" name=\"post_title\" id=\"title\" autocomplete=\"off\" />\n\t\t</div>\n\n\t\t<div class=\"textarea-wrap\" id=\"description-wrap\">\n\t\t\t<label for=\"content\">{}</label>\n\t\t\t<textarea name=\"content\" id=\"content\" placeholder=\"{}\" class=\"mceEditor\" rows=\"3\" cols=\"15\" autocomplete=\"off\"></textarea>\n\t\t</div>\n\n\t\t<p class=\"submit\">\n\t\t\t<input type=\"hidden\" name=\"action\" id=\"quickpost-action\" value=\"post-quickdraft-save\" />\n\t\t\t<input type=\"hidden\" name=\"post_ID\" value=\"{post_id}\" />\n\t\t\t<input type=\"hidden\" name=\"post_type\" value=\"post\" />\n\t\t\t{}\t\t\t<input type=\"submit\" name=\"save\" id=\"save-post\" class=\"button button-primary\" value=\"{}\"  />\t\t\t<br class=\"clear\" />\n\t\t</p>\n\n\t</form>\n\t",
        esc_url(&ctx.admin_url("post.php")),
        tr("Title"),
        tr("Content"),
        esc_attr(&tr("What&#8217;s on your mind?")),
        ctx.nonce_field("add-post"),
        esc_attr(&tr("Save Draft"))
    );
    out.push_str(&recent_drafts(ctx).await);
    out
}

/// The auto-draft the quick draft form saves into (`dashboard_quick_press_last_post_id`).
async fn quick_draft_id(ctx: &Ctx<'_>) -> u64 {
    let db = ctx.env.db;
    let key = format!("{}dashboard_quick_press_last_post_id", db.table_prefix());
    let last: u64 = db.user_meta(ctx.user.id, &key).await.ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0);
    if last != 0
        && let Ok(Some(post)) = db.post(last).await
        && post.post_status == "auto-draft"
    {
        return last;
    }
    let Some(writer) = ctx.env.writer else { return 0 };
    let content = crate::content::Content { site: ctx.env.site, db, writer, user: &ctx.user, now: ctx.env.now };
    let data = crate::content::PostData { post_title: Some(ctx.tr("Auto Draft")), post_type: Some("post".into()), post_status: Some("auto-draft".into()), post_author: Some(ctx.user.id), post_content: Some(String::new()), ..Default::default() };
    match content.insert_post(&data).await {
        Ok(id) => {
            writer.update_user_meta(ctx.user.id, &key, &id.to_string()).await.or_log();
            id
        }
        Err(_) => 0,
    }
}

/// `wp_dashboard_recent_drafts()`.
async fn recent_drafts(ctx: &Ctx<'_>) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let query = PostQuery { post_types: vec!["post".into()], statuses: vec!["draft".into()], author: Some(ctx.user.id), order_by: OrderBy::Modified, order: Order::Desc, limit: Some(4), ..Default::default() };
    let drafts = ctx.env.db.posts(&query).await.unwrap_or_default();
    if drafts.is_empty() {
        return String::new();
    }
    let mut out = String::from("<div class=\"drafts\">");
    if drafts.len() > 3 {
        out.push_str(&format!("<p class=\"view-all\"><a href=\"{}\">{}</a></p>\n", esc_url(&ctx.admin_url("edit.php?post_status=draft")), tr("View all drafts")));
    }
    out.push_str(&format!("<h2 class=\"hide-if-no-js\">{}</h2>\n<ul>", tr("Your Recent Drafts")));
    let length: usize = site.trx("10", "draft_length").trim().parse().unwrap_or(10);
    for draft in drafts.iter().take(3) {
        let title = draft_or_title(ctx, draft);
        let url = esc_url(&ctx.admin_url(&format!("post.php?post={}&action=edit", draft.id)));
        let ts = site.timezone.local_timestamp(&draft.post_date).unwrap_or(0);
        out.push_str(&format!(
            "<li>\n<div class=\"draft-title\"><a href=\"{url}\" aria-label=\"{}\">{title}</a><time datetime=\"{}\">{}</time></div>",
            esc_attr(&sprintf(&tr("Edit &#8220;%s&#8221;"), &[&title])),
            site.date("c", ts),
            site.date(&tr("F j, Y"), ts)
        ));
        let content = wp_trim_words(&draft.post_content, length, &tr("&hellip;"));
        if !content.is_empty() {
            out.push_str(&format!("<p class=\"draft-content\">{content}</p>"));
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</div>");
    out
}

/// `wp_dashboard_events_news()`.
fn events_news(ctx: &Ctx<'_>) -> String {
    let admin = ctx.admin_url("");
    translate(ctx, include_str!("dashboard_primary.tpl")).replace("{{admin}}", &admin)
}

/// `wp_print_community_events_templates()`.
fn events_templates(ctx: &Ctx<'_>) -> String {
    let template = include_str!("dashboard_events.tpl");
    let organize = ctx.tr("https://make.wordpress.org/community/organize-event-landing-page/");
    let welcome = ctx.tr("https://make.wordpress.org/community/handbook/meetup-organizer/welcome/");
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("@@") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("@@") else { break };
        let text = ctx.tr(&after[..end]);
        let text = match &after[..end] {
            "Attend an upcoming event near %s." => text.replace("%s", "<strong>{{ data.location.description }}</strong>"),
            "%s could not be located. Please try another nearby city. For example: Kansas City; Springfield; Portland." => text.replace("%s", "<em>{{data.unknownCity}}</em>"),
            "Want more events? <a href=\"%s\">Help organize the next one</a>!" => text.replace("%s", &organize),
            t if t.starts_with("There are no events scheduled near %1$s") => text.replace("%1$s", "{{ data.location.description }}").replace("%2$s", &welcome),
            t if t.starts_with("There are no events scheduled near you") => text.replace("%s", &welcome),
            _ => text,
        };
        out.push_str(&text);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// `wp_welcome_panel()` inside its box.
async fn welcome_panel(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let version = ctx.env.loader.version.split('-').next().unwrap_or_default().to_owned();
    let svg = ctx.env.loader.includes.parent().and_then(|root| rustpress_core::files::read(&root.join("wp-admin").join("images").join("dashboard-background.svg")).map(|svg| svg.to_string())).unwrap_or_default();
    let body = translate(ctx, include_str!("dashboard_welcome.tpl"))
        .replace("{{background_svg}}", &svg)
        .replace("{{admin}}", &ctx.admin_url(""))
        .replace("{{welcome_nonce}}", &ctx.create_nonce("welcome-panel-nonce"))
        .replace("Learn more about the %s version.", &sprintf(&tr("Learn more about the %s version."), &[&esc_html(&version)]));
    let option: i64 = ctx.env.db.user_meta(ctx.user.id, "show_welcome_panel").await.ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0);
    let hide = !welcome_shown(ctx, option);
    format!("\t<div id=\"welcome-panel\" class=\"welcome-panel{}\">\n{body}", if hide { " hidden" } else { "" })
}

/// Whether the welcome panel shows: `show_welcome_panel` is 0 to hide it, 2 shows it only to the user with the
/// site's admin email.
fn welcome_shown(ctx: &Ctx<'_>, option: i64) -> bool {
    let admin_email = ctx.site().options.string("admin_email", "");
    !(option == 0 || (option == 2 && ctx.user.email != admin_email))
}

/// The help of the dashboard (`index.php`).
fn help(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let p = |s: &str| format!("<p>{}</p>", tr(s));
    let mut content = p("The boxes on your Dashboard screen are:");
    if ctx.can("edit_theme_options") {
        content.push_str(&p("<strong>Welcome</strong> &mdash; Shows links for some of the most common tasks when setting up a new site."));
    }
    if ctx.can("view_site_health_checks") {
        content.push_str(&p("<strong>Site Health Status</strong> &mdash; Informs you of any potential issues that should be addressed to improve the performance or security of your website."));
    }
    if ctx.can("edit_posts") {
        content.push_str(&p("<strong>At a Glance</strong> &mdash; Displays a summary of the content on your site and identifies which theme and version of WordPress you are using."));
    }
    content.push_str(&p("<strong>Activity</strong> &mdash; Shows the upcoming scheduled posts, recently published posts, and the most recent comments on your posts and allows you to moderate them."));
    if ctx.can("edit_posts") {
        content.push_str(&p("<strong>Quick Draft</strong> &mdash; Allows you to create a new post and save it as a draft. Also displays links to the 3 most recent draft posts you've started."));
    }
    content.push_str(&format!(
        "<p>{}</p>",
        sprintf(&tr("<strong>WordPress Events and News</strong> &mdash; Upcoming events near you as well as the latest news from the official WordPress project and the <a href=\"%s\">WordPress Planet</a>."), &[&tr("https://planet.wordpress.org/")])
    ));
    // The version links to its notes unless it is a development version.
    let version = &ctx.env.loader.version;
    let mut version_text = sprintf(&tr("Version %s"), &[version]);
    if !["alpha", "beta", "RC"].iter().any(|v| version.contains(v)) {
        let url = sprintf(&rustpress_core::formatting::esc_url(&tr("https://wordpress.org/documentation/wordpress-version/version-%s/")), &[&rustpress_core::slug::sanitize_title(version)]);
        version_text = format!("<a href=\"{url}\">{version_text}</a>");
    }
    Help {
        tabs: vec![
            HelpTab {
                id: "overview".into(),
                title: tr("Overview"),
                content: format!("{}{}", p("Welcome to your WordPress Dashboard!"), p("The Dashboard is the first place you will come to every time you log into your site. It is where you will find all your WordPress tools. If you need help, just click the &#8220;Help&#8221; tab above the screen title.")),
            },
            HelpTab {
                id: "help-navigation".into(),
                title: tr("Navigation"),
                content: format!(
                    "{}{}",
                    p("The left-hand navigation menu provides links to all of the WordPress administration screens, with submenu items displayed on hover. You can minimize this menu to a narrow icon strip by clicking on the Collapse Menu arrow at the bottom."),
                    p("Links in the Toolbar at the top of the screen connect your dashboard and the front end of your site, and provide access to your profile and helpful WordPress information.")
                ),
            },
            HelpTab {
                id: "help-layout".into(),
                title: tr("Layout"),
                content: format!(
                    "{}{}{}{}",
                    p("You can use the following controls to arrange your Dashboard screen to suit your workflow. This is true on most other administration screens as well."),
                    p("<strong>Screen Options</strong> &mdash; Use the Screen Options tab to choose which Dashboard boxes to show."),
                    p("<strong>Drag and Drop</strong> &mdash; To rearrange the boxes, drag and drop by clicking on the title bar of the selected box and releasing when you see a gray dotted-line rectangle appear in the location you want to place the box."),
                    p("<strong>Box Controls</strong> &mdash; Click the title bar of the box to expand or collapse it. Some boxes added by plugins may have configurable content, and will show a &#8220;Configure&#8221; link in the title bar if you hover over it.")
                ),
            },
            HelpTab { id: "help-content".into(), title: tr("Content"), content },
        ],
        sidebar: format!(
            "<p><strong>{}</strong></p><p>{}</p><p>{}</p><p>{version_text}</p>",
            tr("For more information:"),
            tr("<a href=\"https://wordpress.org/documentation/article/dashboard-screen/\">Documentation on Dashboard</a>"),
            tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
        ),
    }
}

/// The meta of a screen: closed and hidden boxes and their order.
async fn box_state(ctx: &Ctx<'_>) -> (Vec<String>, Vec<String>, HashMap<String, Vec<String>>) {
    let db = ctx.env.db;
    let list = |raw: Option<String>| -> Vec<String> {
        raw.and_then(|r| rustpress_phpser::unserialize(r.as_bytes()).ok())
            .map(|value| match rustpress_core::options::php_to_json(&value) {
                serde_json::Value::Array(items) => items.into_iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default()
    };
    let closed = list(db.user_meta(ctx.user.id, "closedpostboxes_dashboard").await.ok().flatten());
    let hidden = list(db.user_meta(ctx.user.id, "metaboxhidden_dashboard").await.ok().flatten());
    let mut order = HashMap::new();
    if let Some(raw) = db.user_meta(ctx.user.id, "meta-box-order_dashboard").await.ok().flatten()
        && let Ok(value) = rustpress_phpser::unserialize(raw.as_bytes())
        && let serde_json::Value::Object(map) = rustpress_core::options::php_to_json(&value)
    {
        for (column, ids) in map {
            order.insert(column, ids.as_str().unwrap_or_default().split(',').filter(|s| !s.is_empty()).map(str::to_owned).collect());
        }
    }
    (closed, hidden, order)
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let title = tr("Dashboard");
    // The link of the welcome panel's close button works without JavaScript.
    if let (Some(flag), Some(writer)) = (ctx.request.get("welcome"), ctx.env.writer) {
        writer.update_user_meta(ctx.user.id, "show_welcome_panel", if flag.is_empty() || flag == "0" { "0" } else { "1" }).await.or_log();
    }
    let (closed, hidden, order) = box_state(ctx).await;

    let mut widgets: Vec<Widget> = Vec::new();
    if ctx.can("view_site_health_checks") {
        widgets.push(Widget { id: "dashboard_site_health", title: tr("Site Health Status"), aria: tr("Site Health Status"), column: "normal" });
    }
    if ctx.can("edit_posts") {
        widgets.push(Widget { id: "dashboard_right_now", title: tr("At a Glance"), aria: tr("At a Glance"), column: "normal" });
    }
    widgets.push(Widget { id: "dashboard_activity", title: tr("Activity"), aria: tr("Activity"), column: "normal" });
    if ctx.can("edit_posts") {
        let title = format!("<span class=\"hide-if-no-js\">{}</span> <span class=\"hide-if-js\">{}</span>", tr("Quick Draft"), tr("Your Recent Drafts"));
        widgets.push(Widget { id: "dashboard_quick_press", title, aria: format!("{} {}", tr("Quick Draft"), tr("Your Recent Drafts")), column: "side" });
    }
    widgets.push(Widget { id: "dashboard_primary", title: tr("WordPress Events and News"), aria: tr("WordPress Events and News"), column: "side" });

    let mut columns = String::new();
    for (index, column) in ["normal", "side", "column3", "column4"].iter().enumerate() {
        let mut ids: Vec<&Widget> = widgets.iter().filter(|w| w.column == *column).collect();
        // Boxes the user moved here come first, in the saved order; the rest keeps its place.
        if let Some(saved) = order.get(*column) {
            let mut sorted: Vec<&Widget> = Vec::new();
            for id in saved {
                if let Some(w) = widgets.iter().find(|w| w.id == id) {
                    sorted.push(w);
                }
            }
            ids.retain(|w| !sorted.iter().any(|s| s.id == w.id) && !order.iter().any(|(c, list)| c != column && list.iter().any(|i| i == w.id)));
            sorted.extend(ids);
            ids = sorted;
        } else {
            ids.retain(|w| !order.iter().any(|(c, list)| c != column && list.iter().any(|i| i == w.id)));
        }
        let mut boxes = String::new();
        for widget in ids {
            let body = match widget.id {
                "dashboard_site_health" => site_health(ctx).await,
                "dashboard_right_now" => at_a_glance(ctx).await,
                "dashboard_activity" => activity(ctx).await,
                "dashboard_quick_press" => quick_press(ctx, None).await,
                _ => events_news(ctx),
            };
            // Whether a widget closes its own box differs per widget (the markup follows `dashboard.php`).
            let html = postbox(ctx, widget, closed.iter().any(|c| c == widget.id), hidden.iter().any(|h| h == widget.id), &body);
            boxes.push_str(&html);
        }
        columns.push_str(&format!("\t<div id=\"postbox-container-{}\" class=\"postbox-container\">\n\t<div id=\"{column}-sortables\" class=\"meta-box-sortables\">{boxes}</div>\t</div>\n", index + 1));
    }

    // The empty line after the panel belongs to it (`<?php endif; ?>` swallows the line break before it).
    let welcome = if ctx.can("edit_theme_options") { format!("{}\n", welcome_panel(ctx).await) } else { String::new() };
    let content = format!(
        "<div class=\"wrap\">\n\t<h1>{title}</h1>\n\n\t\n\n{welcome}\t<div id=\"dashboard-widgets-wrap\">\n\t<div id=\"dashboard-widgets\" class=\"metabox-holder\">\n{columns}</div>\n\n\t<input type=\"hidden\" id=\"closedpostboxesnonce\" name=\"closedpostboxesnonce\" value=\"{}\" /><input type=\"hidden\" id=\"meta-box-order-nonce\" name=\"meta-box-order-nonce\" value=\"{}\" />\t</div><!-- dashboard-widgets-wrap -->\n\n</div><!-- wrap -->\n\n\n{}",
        ctx.create_nonce("closedpostboxes"),
        ctx.create_nonce("meta-box-order"),
        // The templates end with the indentation of their closing `<?php`.
        events_templates(ctx).trim_end_matches('\n')
    );
    // Screen Options: the boxes in the order they were added, and the welcome panel.
    let welcome_option: i64 = ctx.env.db.user_meta(ctx.user.id, "show_welcome_panel").await.ok().flatten().and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    let options = crate::chrome::ScreenOptions {
        meta_boxes: Some(widgets.iter().map(|w| (w.id.to_owned(), w.title.clone(), hidden.iter().any(|h| h == w.id))).collect()),
        welcome: ctx.can("edit_theme_options").then(|| welcome_shown(ctx, welcome_option)),
        ..Default::default()
    };
    let screen = Screen { file: "index.php".into(), parent_file: "index.php".into(), submenu_file: Some("index.php".into()), title, help: Some(help(ctx)), options: Some(options), ..Default::default() };
    let list_args = "<script>list_args = {\"class\":\"WP_Comments_List_Table\",\"screen\":{\"id\":\"dashboard\",\"base\":\"dashboard\"}};</script>\n";
    let mut scripts = Vec::new();
    let mut styles = vec!["thickbox".to_owned()];
    let mut localize = vec![(
        "dashboard".to_owned(),
        "communityEventsData".to_owned(),
        serde_json::json!({"nonce": ctx.create_nonce("community_events"), "cache": "", "time_format": ctx.site().options.string("time_format", &ctx.tr("g:i a"))}),
    )];
    if ctx.can("view_site_health_checks") {
        styles.insert(0, "site-health".to_owned());
        scripts.push("site-health".to_owned());
        let (good, recommended, critical) = health_counts(ctx).await.unwrap_or((0, 0, 0));
        localize.push((
            "site-health".to_owned(),
            "SiteHealth".to_owned(),
            serde_json::json!({
                "screen": "dashboard",
                "nonce": {"site_status": ctx.create_nonce("health-check-site-status"), "site_status_result": ctx.create_nonce("health-check-site-status-result")},
                "site_status": {"direct": [], "async": [], "issues": {"good": good, "recommended": recommended, "critical": critical}}
            }),
        ));
    }
    scripts.push("dashboard".to_owned());
    if ctx.can("install_plugins") {
        scripts.extend(["plugin-install".to_owned(), "updates".to_owned()]);
    }
    if ctx.can("upload_files") {
        scripts.push("media-upload".to_owned());
    }
    let assets = Assets { scripts, styles, localize, late_scripts: vec!["wplink".into(), "jquery-ui-autocomplete".into()], pre_footer: format!("{list_args}{list_args}"), footer: super::editor::quicktags_editor_js(ctx, &[("replycontent", "strong,em,link,block,del,ins,img,ul,ol,li,code,close")]), ..Default::default() };
    render(ctx, &screen, &content, &assets).await
}

/// `post.php?action=post-quickdraft-save`: saves the Quick Draft form and answers with the form again.
pub async fn quickdraft_save(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let html = |body: String| super::ajax::text(200, &body);
    if !ctx.can("edit_posts") {
        return html(String::new());
    }
    if !ctx.verify_nonce("add-post") {
        return html(quick_press(ctx, Some((&tr("Unable to submit this form, please refresh and try again."), "error"))).await);
    }
    let request = ctx.request;
    let post_id: u64 = request.post_field("post_ID").and_then(|p| p.parse().ok()).unwrap_or(0);
    let title = request.post_field("post_title").unwrap_or_default().trim().to_owned();
    let content = request.post_field("content").unwrap_or_default().trim().to_owned();
    if title.is_empty() && content.is_empty() {
        return html(quick_press(ctx, Some((&tr("Cannot create a draft post with empty title and content."), "error"))).await);
    }
    let Some(writer) = ctx.env.writer else { return html(quick_press(ctx, Some((&tr("Unable to submit this form, please refresh and try again."), "error"))).await) };
    let Some(post) = ctx.env.db.post(post_id).await.ok().flatten().filter(|p| ctx.user.can_edit_post(p)) else {
        return html(quick_press(ctx, Some((&tr("Unable to submit this form, please refresh and try again."), "error"))).await);
    };
    // The content of a block editor post is wrapped in a paragraph block.
    let content = if !content.is_empty() && !content.contains("<!-- wp:paragraph -->") && super::block_editor::uses_block_editor(ctx, &post) { format!("<!-- wp:paragraph -->{}<!-- /wp:paragraph -->", content.replace("\r\n", "<br />").replace(['\r', '\n'], "<br />")) } else { content };
    let saver = crate::content::Content { site: ctx.env.site, db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let data = crate::content::PostData { id: Some(post.id), post_title: Some(title), post_content: Some(content), post_status: Some("draft".into()), ..Default::default() };
    if let Err(e) = saver.insert_post(&data).await {
        return html(quick_press(ctx, Some((&esc_html(&e.to_string()), "error"))).await);
    }
    html(quick_press(ctx, Some((&tr("Draft created successfully."), "success"))).await)
}
