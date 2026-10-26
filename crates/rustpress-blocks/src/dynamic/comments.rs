//! The comment blocks: `core/comments`, `comments-title`, `comment-template`,
//! `comment-author-name`, `comment-content`, `comment-date`,
//! `comment-reply-link`, `avatar` and `post-comments-form`.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::urls::{add_query_arg, remove_query_arg};
use rustpress_db::{Comment, Post};
use serde_json::Value;

use crate::Result;
use crate::content::{comment_text, post_password_required};
use crate::env::Context;

use crate::parser::Content;
use crate::render::BlockCtx;

use super::global_post;

fn has_link_color(bc: &BlockCtx<'_>) -> bool {
    bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null())
}

fn align_and_link_classes(bc: &BlockCtx<'_>) -> String {
    let mut classes = Vec::new();
    if let Some(a) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{a}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    classes.join(" ")
}

fn comments_open(post: &Post) -> bool {
    post.comment_status == "open"
}

async fn current_comment(bc: &BlockCtx<'_>) -> Result<Option<Comment>> {
    let Some(id) = bc.context.get("commentId").and_then(Value::as_u64) else { return Ok(None) };
    Ok(bc.env.site.db.comment(id).await?)
}

/// Renders the block's own inner blocks (used by blocks that skip the generic inner rendering).
async fn render_inner(bc: &BlockCtx<'_>, ctx: &Context) -> Result<String> {
    let mut out = String::new();
    let mut index = 0;
    for chunk in &bc.block.inner_content {
        match chunk {
            Content::Html(html) => out.push_str(html),
            Content::InnerBlock => {
                out.push_str(&bc.env.render_block(&bc.block.inner_blocks[index], ctx, Some(bc.block)).await?);
                index += 1;
            }
        }
    }
    Ok(out)
}

pub(super) async fn comments(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    if bc.context.post_id().is_none() {
        return Ok(String::new());
    }
    if !comments_open(&post) && post.comment_count == 0 {
        return Ok(String::new());
    }
    if bc.attr_bool("legacy") {
        return legacy(bc).await;
    }
    render_inner(bc, bc.available).await
}

/// The legacy comments block (`core/post-comments`, or `core/comments` with `legacy`): the classic comments template.
pub(super) async fn legacy(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    if bc.context.post_id().is_none() || (!comments_open(&post) && post.comment_count == 0) {
        return Ok(String::new());
    }
    let q = bc.env.query;
    let request = format!("{}{}", q.url_path, if q.raw_query.is_empty() { String::new() } else { format!("?{}", q.raw_query) });
    let output = crate::legacy_comments::compat_comments_template(bc.env.site, bc.env, &post, &request, true).await?;
    let mut classes = Vec::new();
    if bc.attrs.contains_key("legacy") {
        classes.push("wp-block-post-comments".to_owned());
    }
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    bc.env.with_state(|s| {
        if !s.scripts.iter().any(|x| x == "comment-reply") {
            s.scripts.push("comment-reply".into());
        }
        // `enqueue_legacy_post_comments_block_styles()`
        s.note_block("core/buttons");
        s.note_block("core/button");
    });
    Ok(format!("<div {wrapper}>{output}</div>"))
}

pub(super) async fn comments_title(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    if post_password_required(&post) {
        return Ok(String::new());
    }
    let count = post.comment_count;
    if count == 0 {
        return Ok(String::new());
    }
    let show_title = bc.attr_bool("showPostTitle");
    let show_count = bc.attr_bool("showCommentsCount");
    let title = crate::content::the_title_for(bc.env.site, &post);
    let site = bc.env.site;
    let number = site.number_i18n(count as u64);
    let quoted = |text: String| rustpress_core::i18n::sprintf(&text, &[&title]);
    let heading = if show_count {
        if show_title {
            if count == 1 {
                quoted(site.tr("One response to &#8220;%s&#8221;"))
            } else {
                rustpress_core::i18n::sprintf(&site.trn("%1$s response to &#8220;%2$s&#8221;", "%1$s responses to &#8220;%2$s&#8221;", count), &[&number, &title])
            }
        } else if count == 1 {
            site.tr("One response")
        } else {
            rustpress_core::i18n::sprintf(&site.trn("%s response", "%s responses", count), &[&number])
        }
    } else if show_title {
        if count == 1 { quoted(site.tr("Response to &#8220;%s&#8221;")) } else { quoted(site.tr("Responses to &#8220;%s&#8221;")) }
    } else if count == 1 {
        site.tr("Response")
    } else {
        site.tr("Responses")
    };
    let tag = format!("h{}", bc.attrs.get("level").and_then(Value::as_u64).unwrap_or(2));
    let wrapper = bc.wrapper_attributes(&[("class", bc.attr_str("textAlign").filter(|a| !a.is_empty()).map(|a| format!("has-text-align-{a}")).unwrap_or_default())]);
    Ok(format!("<{tag} id=\"comments\" {wrapper}>{heading}</{tag}>"))
}

/// Threaded rendering of the comment list.
pub(super) async fn comment_template(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id().filter(|id| *id != 0) else { return Ok(String::new()) };
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    if post_password_required(&post) {
        return Ok(String::new());
    }
    let page = crate::comment_pages::comments_for_post(bc.env, post_id).await?;
    let comments = page.comments;
    if comments.is_empty() {
        return Ok(String::new());
    }
    let options = &bc.env.site.options;
    let threaded = options.get("thread_comments") == Some("1");
    let max_depth = options.u64("thread_comments_depth", 5);
    let wrapper = bc.wrapper_attributes(&[]);
    let mut state = ListState { alt: 0, thread_alt: 0, post_author: post.post_author };
    let mut roots: Vec<&Comment> = if threaded { comments.iter().filter(|c| c.comment_parent == 0 || !comments.iter().any(|p| p.comment_id == c.comment_parent)).collect() } else { comments.iter().collect() };
    // `array_reverse()` of the top-level comments only; replies keep their order.
    if options.get("comment_order") == Some("desc") {
        roots.reverse();
    }
    let html = render_comments(bc, &comments, &roots, threaded, 1, max_depth, &mut state).await?;
    Ok(format!("<ol {wrapper}>{html}</ol>"))
}

struct ListState {
    alt: u32,
    thread_alt: u32,
    post_author: u64,
}

#[allow(clippy::too_many_arguments)]
fn render_comments<'a>(
    bc: &'a BlockCtx<'a>,
    all: &'a [Comment],
    level: &'a [&'a Comment],
    threaded: bool,
    depth: u64,
    max_depth: u64,
    state: &'a mut ListState,
) -> futures::future::BoxFuture<'a, Result<String>> {
    Box::pin(async move {
        let mut html = String::new();
        for comment in level {
            let mut ctx = bc.available.clone();
            ctx.values.insert("commentId".into(), Value::from(comment.comment_id));
            let mut inner = render_inner(bc, &ctx).await?;
            let children: Vec<&Comment> = all.iter().filter(|c| c.comment_parent == comment.comment_id).collect();
            let nicename = if comment.user_id != 0 { bc.env.site.db.user(comment.user_id).await?.map(|u| u.user_nicename) } else { None };
            let classes = comment_class(comment, depth, state, nicename.as_deref());
            if !children.is_empty() && threaded {
                if depth < max_depth {
                    let nested = render_comments(bc, all, &children, threaded, depth + 1, max_depth, state).await?;
                    inner.push_str(&format!("<ol>{nested}</ol>"));
                } else {
                    inner.push_str(&render_comments(bc, all, &children, threaded, depth, max_depth, state).await?);
                }
            }
            html.push_str(&format!("<li id=\"comment-{}\" class=\"{}\">{inner}</li>", comment.comment_id, esc_attr(&classes)));
        }
        Ok(html)
    })
}

/// `get_comment_class()`.
fn comment_class(comment: &Comment, depth: u64, state: &mut ListState, nicename: Option<&str>) -> String {
    let mut classes = vec![if comment.comment_type.is_empty() { "comment".to_owned() } else { comment.comment_type.clone() }];
    if let Some(nicename) = nicename {
        classes.push("byuser".into());
        let class = rustpress_core::formatting::sanitize_html_class(nicename);
        classes.push(format!("comment-author-{}", if class.is_empty() { comment.user_id.to_string() } else { class }));
        if comment.user_id == state.post_author {
            classes.push("bypostauthor".into());
        }
    }
    if state.alt % 2 == 1 {
        classes.push("odd".into());
        classes.push("alt".into());
    } else {
        classes.push("even".into());
    }
    state.alt += 1;
    if depth == 1 {
        if state.thread_alt % 2 == 1 {
            classes.push("thread-odd".into());
            classes.push("thread-alt".into());
        } else {
            classes.push("thread-even".into());
        }
        state.thread_alt += 1;
    }
    classes.push(format!("depth-{depth}"));
    classes.join(" ")
}

pub(super) async fn comment_author_name(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    let wrapper = bc.wrapper_attributes(&[("class", align_and_link_classes(bc))]);
    let mut author = comment_author(bc.env.site, &comment);
    // The site of a comment nobody has approved is linked only for who is known by name.
    let show_link = comment.comment_approved != "0" || !crate::commenter::current_commenter().0.is_empty();
    if show_link && !comment.comment_author_url.is_empty() && bc.attr_bool("isLink") && bc.attr_str("linkTarget").is_some_and(|t| !t.is_empty()) {
        author = format!(
            "<a rel=\"external nofollow ugc\" href=\"{}\" target=\"{}\" >{author}</a>",
            esc_url(&comment.comment_author_url),
            esc_attr(bc.attr_str("linkTarget").unwrap_or_default())
        );
    }
    Ok(format!("<div {wrapper}>{author}</div>"))
}

fn comment_author(site: &rustpress_core::Site, comment: &Comment) -> String {
    if comment.comment_author.is_empty() { site.tr("Anonymous") } else { comment.comment_author.clone() }
}

pub(super) async fn comment_content(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    if comment.comment_content.is_empty() {
        return Ok(String::new());
    }
    let mut text = comment_text(bc.env.site, &comment.comment_content);
    let mut note = String::new();
    if comment.comment_approved == "0" {
        let (author, email, _) = crate::commenter::current_commenter();
        let site = bc.env.site;
        let message = if email.is_empty() {
            site.tr("Your comment is awaiting moderation. This is a preview; your comment will be visible after it has been approved.")
        } else {
            site.tr("Your comment is awaiting moderation.")
        };
        note = format!("<p><em class=\"comment-awaiting-moderation\">{message}</em></p>");
        // Links of a comment nobody has approved are shown only to who is known by name.
        if author.is_empty() {
            text = rustpress_core::kses::kses(&text, &rustpress_core::kses::Allowed::none());
        }
    }
    let wrapper = bc.wrapper_attributes(&[("class", align_and_link_classes(bc))]);
    Ok(format!("<div {wrapper}>{note}{text}</div>"))
}

pub(super) async fn comment_date(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    let site = bc.env.site;
    let class = if has_link_color(bc) { "has-link-color" } else { "" };
    let wrapper = bc.wrapper_attributes(&[("class", class.to_owned())]);
    let Some(ts) = site.timezone.local_timestamp(&comment.comment_date) else { return Ok(String::new()) };
    let format = bc.attr_str("format").filter(|f| !f.is_empty()).map(str::to_owned).unwrap_or_else(|| site.options.string("date_format", &site.tr("F j, Y")));
    let mut formatted = esc_html(&site.date(&format, ts));
    if bc.attr_bool("isLink") {
        let link = comment_link(bc, &comment).await?;
        formatted = format!("<a href=\"{}\">{formatted}</a>", esc_url(&link));
    }
    Ok(format!("<div {wrapper}><time datetime=\"{}\">{formatted}</time></div>", esc_attr(&site.date("c", ts))))
}

async fn comment_link(bc: &BlockCtx<'_>, comment: &Comment) -> Result<String> {
    crate::comment_pages::comment_link(bc.env, comment, None).await
}

pub(super) async fn comment_reply_link(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    let options = &bc.env.site.options;
    if options.get("thread_comments") != Some("1") {
        return Ok(String::new());
    }
    let max_depth = options.u64("thread_comments_depth", 5);
    let all = crate::comment_pages::listed_comments(bc.env, comment.comment_post_id).await?;
    let mut depth = 1;
    let mut parent = comment.comment_parent;
    while parent != 0 {
        depth += 1;
        parent = all.iter().find(|c| c.comment_id == parent).map_or(0, |c| c.comment_parent);
    }
    if depth == 0 || max_depth <= depth {
        return Ok(String::new());
    }
    let Some(post) = bc.env.site.db.post(comment.comment_post_id).await? else { return Ok(String::new()) };
    if !comments_open(&post) {
        return Ok(String::new());
    }
    let permalink = if crate::comment_pages::paginated(bc.env) {
        comment_link(bc, &comment).await?.replace(&format!("#comment-{}", comment.comment_id), "")
    } else {
        bc.env.site.permalink(&post).await?
    };
    let author = comment_author(bc.env.site, &comment);
    let reply_to = rustpress_core::i18n::sprintf(&bc.env.site.tr("Reply to %s"), &[&author]);
    let data = format!(
        "data-commentid=\"{}\" data-postid=\"{}\" data-belowelement=\"comment-{}\" data-respondelement=\"respond\" data-replyto=\"{}\"",
        comment.comment_id,
        post.id,
        comment.comment_id,
        esc_attr(&reply_to)
    );
    let href = format!("{}#respond", esc_url(&add_query_arg(&permalink, "replytocom", &comment.comment_id.to_string())));
    let link = format!("<a rel=\"nofollow\" class=\"comment-reply-link\" href=\"{href}\" {data} aria-label=\"{}\">{}</a>", esc_attr(&reply_to), bc.env.site.trx("Reply", "verb"));
    let wrapper = bc.wrapper_attributes(&[("class", align_and_link_classes(bc))]);
    Ok(format!("<div {wrapper}>{link}</div>"))
}

/// `core/comment-edit-link`: the link to the editor of the comment, for somebody who may edit it.
pub(super) async fn comment_edit_link(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    let Some(viewer) = bc.env.query.viewer.as_ref() else { return Ok(String::new()) };
    // `current_user_can( 'edit_comment' )`: whoever may edit the post of the comment.
    let Some(post) = bc.env.site.db.post(comment.comment_post_id).await? else { return Ok(String::new()) };
    if !viewer.can_edit_post(&post) {
        return Ok(String::new());
    }
    let link = bc.env.site.site_url(&format!("wp-admin/comment.php?action=editcomment&c={}", comment.comment_id));
    let class = bc.attr_str("textAlign").map(|a| format!("has-text-align-{a}")).unwrap_or_default();
    let wrapper = bc.wrapper_attributes(&[("class", class)]);
    let target = bc.attr_str("linkTarget").filter(|t| !t.is_empty()).map(|t| format!("target=\"{}\"", esc_attr(t))).unwrap_or_default();
    Ok(format!("<div {wrapper}><a href=\"{}\" {target}>{}</a></div>", esc_url(&link), bc.env.site.tr("Edit")))
}

// ---------------------------------------------------------------------- avatar

/// `get_avatar()` for an email address.
pub(super) fn avatar_img(bc: &BlockCtx<'_>, email: &str, size: u64, alt: &str, extra_class: &str, extra_attr: &str) -> String {
    crate::content::get_avatar(bc.env, email, size, alt, extra_class, extra_attr)
}

pub(super) async fn avatar(bc: &BlockCtx<'_>) -> Result<String> {
    let size = bc.attr_u64("size").unwrap_or(96);
    let wrapper = bc.wrapper_attributes(&[]);
    let border = crate::style_engine::get_styles(&serde_json::json!({ "border": bc.attrs.get("style").and_then(|s| s.get("border")).cloned().unwrap_or(Value::Null) }), false);
    let mut image_classes = "wp-block-avatar__image".to_owned();
    if !border.classnames.is_empty() {
        image_classes.push(' ');
        image_classes.push_str(&border.classnames);
    }
    let image_styles = if border.css.is_empty() { String::new() } else { format!(" style=\"{}\"", esc_attr(&border.css)) };

    if bc.context.get("commentId").is_none() {
        let author_id = bc.attr_u64("userId").or_else(|| bc.context.post_id().and(None)).unwrap_or(0);
        let author_id = if author_id != 0 {
            author_id
        } else if bc.context.post_id().is_some() {
            global_post(bc).await?.map_or(0, |p| p.post_author)
        } else {
            bc.env.query.vars.u64("author").unwrap_or(0)
        };
        if author_id == 0 {
            return Ok(String::new());
        }
        let Some(user) = bc.env.site.db.user(author_id).await? else { return Ok(String::new()) };
        let alt = rustpress_core::i18n::sprintf(&bc.env.site.tr("%s Avatar"), &[&user.display_name]);
        let mut avatar = avatar_img(bc, &user.user_email, size, &alt, &image_classes, &image_styles);
        if bc.attr_bool("isLink") {
            let label = if bc.attr_str("linkTarget") == Some("_blank") { format!("aria-label=\"{}\"", esc_attr(&rustpress_core::i18n::sprintf(&bc.env.site.tr("(%s author archive, opens in a new tab)"), &[&user.display_name]))) } else { String::new() };
            avatar = format!(
                "<a href=\"{}\" target=\"{}\" {label} class=\"wp-block-avatar__link\">{avatar}</a>",
                esc_url(&bc.env.site.author_url(&user)),
                esc_attr(bc.attr_str("linkTarget").unwrap_or("_self"))
            );
        }
        return Ok(format!("<div {wrapper}>{avatar}</div>"));
    }

    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    let alt = rustpress_core::i18n::sprintf(&bc.env.site.tr("%s Avatar"), &[&comment.comment_author]);
    let email = if comment.user_id != 0 {
        bc.env.site.db.user(comment.user_id).await?.map(|u| u.user_email).unwrap_or(comment.comment_author_email.clone())
    } else {
        comment.comment_author_email.clone()
    };
    // `get_avatar_comment_types()`: only plain comments have avatars.
    if !comment.comment_type.is_empty() && comment.comment_type != "comment" {
        return Ok(format!("<div {wrapper}></div>"));
    }
    let mut avatar = avatar_img(bc, &email, size, &alt, &image_classes, &image_styles);
    if bc.attr_bool("isLink") && !comment.comment_author_url.is_empty() {
        let label = if bc.attr_str("linkTarget") == Some("_blank") { format!("aria-label=\"{}\"", esc_attr(&rustpress_core::i18n::sprintf(&bc.env.site.tr("(%s website link, opens in a new tab)"), &[&comment.comment_author]))) } else { String::new() };
        avatar = format!(
            "<a href=\"{}\" target=\"{}\" {label} class=\"wp-block-avatar__link\">{avatar}</a>",
            esc_url(&comment.comment_author_url),
            esc_attr(bc.attr_str("linkTarget").unwrap_or("_self"))
        );
    }
    Ok(format!("<div {wrapper}>{avatar}</div>"))
}

// ------------------------------------------------------------ comments pagination


/// `get_comments_pagination_arrow()`.
fn pagination_arrow(bc: &BlockCtx<'_>, kind: &str) -> Option<String> {
    let setting = bc.context.get("comments/paginationArrow").and_then(Value::as_str).filter(|s| !s.is_empty())?;
    let arrow = match (setting, kind) {
        ("arrow", "next") => "→",
        ("arrow", _) => "←",
        ("chevron", "next") => "»",
        ("chevron", _) => "«",
        _ => return None,
    };
    Some(format!("<span class='wp-block-comments-pagination-{kind}-arrow is-arrow-{setting}' aria-hidden='true'>{arrow}</span>"))
}

/// The page the pagination blocks work on: `(page, max pages)` (`build_comment_query_vars_from_block()`).
async fn pagination_state(bc: &BlockCtx<'_>) -> Result<Option<(Option<u64>, u64, String)>> {
    let Some(post_id) = bc.context.post_id().filter(|id| *id != 0) else { return Ok(None) };
    let Some(post) = global_post(bc).await? else { return Ok(None) };
    let page = crate::comment_pages::comments_for_post(bc.env, post_id).await?;
    let permalink = bc.env.site.permalink(&post).await?;
    Ok(Some(((page.page != 0).then_some(page.page), page.max_pages, permalink)))
}

pub(super) async fn comments_pagination_previous(bc: &BlockCtx<'_>) -> Result<String> {
    let Some((paged, _, permalink)) = pagination_state(bc).await? else { return Ok(String::new()) };
    if !bc.env.query.flags.is_singular() {
        return Ok(String::new());
    }
    let mut label = bc.attr_str("label").filter(|l| !l.is_empty()).map(str::to_owned).unwrap_or_else(|| bc.env.site.tr("Older Comments"));
    if let Some(arrow) = pagination_arrow(bc, "previous") {
        label = format!("{arrow}{label}");
    }
    let page = paged.unwrap_or_else(|| bc.env.query.vars.u64("cpage").unwrap_or(0));
    let attributes = bc.wrapper_attributes(&[]);
    Ok(crate::comment_pages::previous_link(bc.env, &label, page, &permalink, &attributes).await.unwrap_or_default())
}

pub(super) async fn comments_pagination_next(bc: &BlockCtx<'_>) -> Result<String> {
    let Some((paged, max_page, permalink)) = pagination_state(bc).await? else { return Ok(String::new()) };
    if !bc.env.query.flags.is_singular() {
        return Ok(String::new());
    }
    let mut label = bc.attr_str("label").filter(|l| !l.is_empty()).map(str::to_owned).unwrap_or_else(|| bc.env.site.tr("Newer Comments"));
    if let Some(arrow) = pagination_arrow(bc, "next") {
        label.push_str(&arrow);
    }
    let page = paged.unwrap_or_else(|| bc.env.query.vars.u64("cpage").unwrap_or(0));
    let attributes = bc.wrapper_attributes(&[]);
    Ok(crate::comment_pages::next_link(bc.env, &label, page, max_page, &permalink, &attributes).unwrap_or_default())
}

pub(super) async fn comments_pagination_numbers(bc: &BlockCtx<'_>) -> Result<String> {
    let Some((paged, max_page, permalink)) = pagination_state(bc).await? else { return Ok(String::new()) };
    if !bc.env.query.flags.is_singular() {
        return Ok(String::new());
    }
    let site = bc.env.site;
    let base = if site.uses_pretty_permalinks() {
        site.user_trailingslashit(&format!("{}comment-page-%#%", rustpress_core::urls::trailingslashit(&permalink)))
    } else {
        add_query_arg(&permalink, "cpage", "%#%")
    };
    let current = paged.unwrap_or(1);
    let Some(links) = super::query::paginate_links(&base, "", max_page, current, 1, &[], "#comments") else { return Ok(String::new()) };
    Ok(format!("<div {}>{links}</div>", bc.wrapper_attributes(&[])))
}

/// `core/comments-pagination`: the wrapper around the previous/numbers/next blocks.
pub(super) async fn comments_pagination(bc: &BlockCtx<'_>) -> Result<String> {
    if bc.content.trim().is_empty() {
        return Ok(String::new());
    }
    if global_post(bc).await?.is_some_and(|post| post_password_required(&post)) {
        return Ok(String::new());
    }
    let has_link_color = bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null());
    let class = if has_link_color { "has-link-color" } else { "" };
    let wrapper = bc.wrapper_attributes(&[("aria-label", bc.env.site.tr("Comments pagination")), ("class", class.to_owned())]);
    Ok(format!("<nav {wrapper}>{}</nav>", bc.content))
}

// --------------------------------------------------------------- comment form

pub(super) async fn post_comments_form(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id() else { return Ok(String::new()) };
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    if post_password_required(&post) {
        return Ok(String::new());
    }
    let mut classes = vec!["comment-respond".to_owned()];
    if let Some(a) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{a}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    if !comments_open(&post) {
        return Ok(String::new());
    }
    let site = bc.env.site;
    let options = &site.options;
    let require = options.get("require_name_email") != Some("0");
    let indicator = format!("<span class=\"required\">{}</span>", rustpress_core::formatting::esc_html(&site.tr("*")));
    let required_indicator = format!(" {indicator}");
    let required_indicator = required_indicator.as_str();
    let required_text = format!(" <span class=\"required-field-message\">{}</span>", rustpress_core::i18n::sprintf(&site.tr("Required fields are marked %s"), &[&indicator]));
    let required_attr = " required";

    let (commenter_author, commenter_email, commenter_url) = crate::commenter::current_commenter();
    let author = format!(
        "<p class=\"comment-form-author\"><label for=\"author\">{}{}</label> <input id=\"author\" name=\"author\" type=\"text\" value=\"{}\" size=\"30\" maxlength=\"245\" autocomplete=\"name\"{} /></p>",
        site.tr("Name"),
        if require { required_indicator } else { "" },
        esc_attr(&commenter_author),
        if require { required_attr } else { "" }
    );
    let email = format!(
        "<p class=\"comment-form-email\"><label for=\"email\">{}{}</label> <input id=\"email\" name=\"email\" type=\"email\" value=\"{}\" size=\"30\" maxlength=\"100\" aria-describedby=\"email-notes\" autocomplete=\"email\"{} /></p>",
        site.tr("Email"),
        if require { required_indicator } else { "" },
        esc_attr(&commenter_email),
        if require { required_attr } else { "" }
    );
    let url = format!("<p class=\"comment-form-url\"><label for=\"url\">{}</label> <input id=\"url\" name=\"url\" type=\"url\" value=\"{}\" size=\"30\" maxlength=\"200\" autocomplete=\"url\" /></p>", site.tr("Website"), esc_attr(&commenter_url));
    let cookies = if options.get("show_comments_cookies_opt_in") == Some("1") {
        format!("<p class=\"comment-form-cookies-consent\"><input id=\"wp-comment-cookies-consent\" name=\"wp-comment-cookies-consent\" type=\"checkbox\" value=\"yes\"{} /> <label for=\"wp-comment-cookies-consent\">{}</label></p>", if commenter_email.is_empty() { "" } else { " checked" }, site.tr("Save my name, email, and website in this browser for the next time I comment."))
    } else {
        String::new()
    };
    let comment_label = site.trx("Comment", "noun");
    let comment_field = format!(
        "<p class=\"comment-form-comment\"><label for=\"comment\">{comment_label}{required_indicator}</label> <textarea id=\"comment\" name=\"comment\" cols=\"45\" rows=\"8\" maxlength=\"65525\"{required_attr}></textarea></p>"
    );
    let mut notes = format!("<p class=\"comment-notes\"><span id=\"email-notes\">{}</span>{required_text}</p>", site.tr("Your email address will not be published."));
    // A signed-in viewer is named instead of asked, and has no fields to fill in.
    let viewer = bc.env.query.viewer.as_ref();
    let mut unfiltered = String::new();
    if let Some(viewer) = viewer {
        let permalink = site.permalink(&post).await?;
        let logout = rustpress_core::formatting::esc_html(&format!("{}?action=logout&redirect_to={}&_wpnonce={}", site.site_url("wp-login.php"), rustpress_core::canonical::php_urlencode(&permalink), viewer.create_nonce("log-out")));
        notes = format!(
            "<p class=\"logged-in-as\">{}{required_text}</p>",
            rustpress_core::i18n::sprintf(
                &site.tr("Logged in as %1$s. <a href=\"%2$s\">Edit your profile</a>. <a href=\"%3$s\">Log out?</a>"),
                &[&rustpress_core::formatting::esc_html(&viewer.display_name), &esc_url(&site.site_url("wp-admin/profile.php")), &logout]
            )
        );
        if viewer.can("unfiltered_html") {
            unfiltered = format!(
                "<input type=\"hidden\" id=\"_wp_unfiltered_html_comment_disabled\" name=\"_wp_unfiltered_html_comment_disabled\" value=\"{}\" /><script>\n(function(){{if(window===window.parent){{document.getElementById('_wp_unfiltered_html_comment_disabled').name='_wp_unfiltered_html_comment';}}}})();\n//# sourceURL=wp_comment_form_unfiltered_html_nonce\n</script>\n",
                viewer.create_nonce(&format!("unfiltered-html-comment_{post_id}"))
            );
        }
    }

    let q = bc.env.query;
    let threaded = options.get("thread_comments") == Some("1");
    // `?replytocom=ID`: the form answers that comment (`comment_form_title()`, `cancel_comment_reply_link()`).
    let reply_to = rustpress_core::QueryVars::parse(&q.raw_query).u64("replytocom").filter(|id| *id != 0);
    let parent = match reply_to {
        Some(id) => site.db.comment(id).await?.filter(|c| !c.comment_author.is_empty()),
        None => None,
    };
    let cancel = if threaded {
        let style = if reply_to.is_some() { "" } else { " style=\"display:none;\"" };
        let link = esc_url(&remove_query_arg(&remove_query_arg(&remove_query_arg(&format!("{}{}", q.url_path, if q.raw_query.is_empty() { String::new() } else { format!("?{}", q.raw_query) }), "replytocom"), "unapproved"), "moderation-hash"));
        format!(" <small><a rel=\"nofollow\" id=\"cancel-comment-reply-link\" href=\"{link}#respond\"{style}>{}</a></small>", site.tr("Cancel reply"))
    } else {
        String::new()
    };
    let mut fields = String::new();
    for field in [&author, &email, &url, &cookies] {
        if !field.is_empty() && viewer.is_none() {
            fields.push_str(field);
            fields.push('\n');
        }
    }
    let submit = format!("<input name=\"submit\" type=\"submit\" id=\"submit\" class=\"wp-block-button__link wp-element-button\" value=\"{}\" />", esc_attr(&site.tr("Post Comment")));
    let id_fields = format!("<input type='hidden' name='comment_post_ID' value='{post_id}' id='comment_post_ID' />\n<input type='hidden' name='comment_parent' id='comment_parent' value='{}' />\n", reply_to.unwrap_or(0));
    let action = esc_url(&site.site_url("/wp-comments-post.php"));
    bc.env.with_state(|s| if !s.scripts.iter().any(|x| x == "comment-reply") { s.scripts.push("comment-reply".to_owned()) });
    let reply_title = match &parent {
        Some(comment) => rustpress_core::i18n::sprintf(&site.tr("Leave a Reply to %s"), &[&format!("<a href=\"#comment-{}\">{}</a>", comment.comment_id, esc_html(&comment.comment_author))]),
        None => site.tr("Leave a Reply"),
    };
    Ok(format!(
        "\t<div id=\"respond\" {wrapper}>\n\t\t<h3 id=\"reply-title\" class=\"comment-reply-title\">{reply_title}{cancel}</h3><form action=\"{action}\" method=\"post\" id=\"commentform\" class=\"comment-form\">{notes}{comment_field}{fields}<p class=\"form-submit wp-block-button\">{submit} {id_fields}</p>{unfiltered}</form>\t</div><!-- #respond -->\n\t"
    ))
}
