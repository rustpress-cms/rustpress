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
use crate::images::{self, LoadingInput};
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
    render_inner(bc, bc.available).await
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
    let title = crate::content::the_title(&post);
    let number = count.to_string();
    let heading = if show_count {
        if show_title {
            if count == 1 {
                format!("One response to &#8220;{title}&#8221;")
            } else {
                format!("{number} responses to &#8220;{title}&#8221;")
            }
        } else if count == 1 {
            "One response".to_owned()
        } else {
            format!("{number} responses")
        }
    } else if show_title {
        if count == 1 { format!("Response to &#8220;{title}&#8221;") } else { format!("Responses to &#8220;{title}&#8221;") }
    } else if count == 1 {
        "Response".to_owned()
    } else {
        "Responses".to_owned()
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
    let mut comments = bc.env.site.db.approved_comments(post_id).await?;
    if comments.is_empty() {
        return Ok(String::new());
    }
    let options = &bc.env.site.options;
    let threaded = options.get("thread_comments") == Some("1");
    let max_depth = options.u64("thread_comments_depth", 5);
    if options.get("comment_order") == Some("desc") {
        comments.reverse();
    }
    let wrapper = bc.wrapper_attributes(&[]);
    let mut state = ListState { alt: 0, thread_alt: 0 };
    let roots: Vec<&Comment> = if threaded { comments.iter().filter(|c| c.comment_parent == 0 || !comments.iter().any(|p| p.comment_id == c.comment_parent)).collect() } else { comments.iter().collect() };
    let html = render_comments(bc, &comments, &roots, threaded, 1, max_depth, &mut state).await?;
    Ok(format!("<ol {wrapper}>{html}</ol>"))
}

struct ListState {
    alt: u32,
    thread_alt: u32,
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
            let classes = comment_class(comment, depth, state);
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
fn comment_class(comment: &Comment, depth: u64, state: &mut ListState) -> String {
    let mut classes = vec![if comment.comment_type.is_empty() { "comment".to_owned() } else { comment.comment_type.clone() }];
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
    let mut author = comment_author(&comment);
    if !comment.comment_author_url.is_empty() && bc.attr_bool("isLink") && bc.attr_str("linkTarget").is_some_and(|t| !t.is_empty()) {
        author = format!(
            "<a rel=\"external nofollow ugc\" href=\"{}\" target=\"{}\" >{author}</a>",
            esc_url(&comment.comment_author_url),
            esc_attr(bc.attr_str("linkTarget").unwrap_or_default())
        );
    }
    Ok(format!("<div {wrapper}>{author}</div>"))
}

fn comment_author(comment: &Comment) -> String {
    if comment.comment_author.is_empty() { "Anonymous".to_owned() } else { comment.comment_author.clone() }
}

pub(super) async fn comment_content(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    if comment.comment_content.is_empty() {
        return Ok(String::new());
    }
    let text = comment_text(&comment.comment_content);
    let wrapper = bc.wrapper_attributes(&[("class", align_and_link_classes(bc))]);
    Ok(format!("<div {wrapper}>{text}</div>"))
}

pub(super) async fn comment_date(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    let site = bc.env.site;
    let class = if has_link_color(bc) { "has-link-color" } else { "" };
    let wrapper = bc.wrapper_attributes(&[("class", class.to_owned())]);
    let Some(ts) = site.timezone.local_timestamp(&comment.comment_date) else { return Ok(String::new()) };
    let format = bc.attr_str("format").filter(|f| !f.is_empty()).map(str::to_owned).unwrap_or_else(|| site.options.string("date_format", "F j, Y"));
    let mut formatted = esc_html(&site.timezone.format(&format, ts));
    if bc.attr_bool("isLink") {
        let link = comment_link(bc, &comment).await?;
        formatted = format!("<a href=\"{}\">{formatted}</a>", esc_url(&link));
    }
    Ok(format!("<div {wrapper}><time datetime=\"{}\">{formatted}</time></div>", esc_attr(&site.timezone.format("c", ts))))
}

async fn comment_link(bc: &BlockCtx<'_>, comment: &Comment) -> Result<String> {
    let post = bc.env.site.db.post(comment.comment_post_id).await?;
    let base = match post {
        Some(p) => bc.env.site.permalink(&p).await?,
        None => String::new(),
    };
    Ok(format!("{base}#comment-{}", comment.comment_id))
}

pub(super) async fn comment_reply_link(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    let options = &bc.env.site.options;
    if options.get("thread_comments") != Some("1") {
        return Ok(String::new());
    }
    let max_depth = options.u64("thread_comments_depth", 5);
    let all = bc.env.site.db.approved_comments(comment.comment_post_id).await?;
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
    let permalink = bc.env.site.permalink(&post).await?;
    let author = comment_author(&comment);
    let reply_to = format!("Reply to {author}");
    let data = format!(
        "data-commentid=\"{}\" data-postid=\"{}\" data-belowelement=\"comment-{}\" data-respondelement=\"respond\" data-replyto=\"{}\"",
        comment.comment_id,
        post.id,
        comment.comment_id,
        esc_attr(&reply_to)
    );
    let href = format!("{}#respond", esc_url(&add_query_arg(&permalink, "replytocom", &comment.comment_id.to_string())));
    let link = format!("<a rel=\"nofollow\" class=\"comment-reply-link\" href=\"{href}\" {data} aria-label=\"{}\">Reply</a>", esc_attr(&reply_to));
    let wrapper = bc.wrapper_attributes(&[("class", align_and_link_classes(bc))]);
    Ok(format!("<div {wrapper}>{link}</div>"))
}

// ---------------------------------------------------------------------- avatar

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(input.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// `get_avatar()` for an email address.
fn avatar_img(bc: &BlockCtx<'_>, email: &str, size: u64, alt: &str, extra_class: &str, extra_attr: &str) -> String {
    let options = &bc.env.site.options;
    let default = match options.string("avatar_default", "mystery").as_str() {
        "mm" | "mystery" | "mysteryman" => "mm".to_owned(),
        "gravatar_default" => String::new(),
        other => other.to_owned(),
    };
    let rating = options.string("avatar_rating", "G").to_lowercase();
    let hash = if email.is_empty() { String::new() } else { sha256_hex(&email.trim().to_lowercase()) };
    let url_for = |s: u64| {
        let mut url = format!("https://secure.gravatar.com/avatar/{hash}?s={s}");
        if !default.is_empty() {
            url.push_str(&format!("&d={}", default));
        }
        if !rating.is_empty() {
            url.push_str(&format!("&r={rating}"));
        }
        url
    };
    let mut classes = vec!["avatar".to_owned(), format!("avatar-{size}"), "photo".to_owned()];
    if hash.is_empty() {
        classes.push("avatar-default".to_owned());
    }
    if !extra_class.is_empty() {
        classes.push(extra_class.to_owned());
    }
    // `loading`, `fetchpriority` and `decoding` follow the extra attributes, in this order.
    let input = LoadingInput { width: Some(size), height: Some(size), ..Default::default() };
    let optimization = images::loading_attributes(bc.env, "img", &input, "get_avatar");
    let mut extra_attr = extra_attr.to_owned();
    for (name, allowed) in [("loading", &["lazy", "eager"][..]), ("fetchpriority", &["high", "low", "auto"][..]), ("decoding", &["async", "sync", "auto"][..])] {
        if let Some((_, value)) = optimization.iter().find(|(k, v)| k == name && allowed.contains(&v.as_str())) {
            if !extra_attr.is_empty() {
                extra_attr.push(' ');
            }
            extra_attr.push_str(&format!("{name}='{value}'"));
        }
    }
    format!(
        "<img alt='{}' src='{}' srcset='{} 2x' class='{}' height='{size}' width='{size}' {extra_attr}/>",
        esc_attr(alt),
        esc_url(&url_for(size)),
        esc_url(&url_for(size * 2)),
        esc_attr(&classes.join(" "))
    )
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
        let alt = format!("{} Avatar", user.display_name);
        let mut avatar = avatar_img(bc, &user.user_email, size, &alt, &image_classes, &image_styles);
        if bc.attr_bool("isLink") {
            let label = if bc.attr_str("linkTarget") == Some("_blank") { format!("aria-label=\"{}\"", esc_attr(&format!("({} author archive, opens in a new tab)", user.display_name))) } else { String::new() };
            avatar = format!(
                "<a href=\"{}\" target=\"{}\" {label} class=\"wp-block-avatar__link\">{avatar}</a>",
                esc_url(&bc.env.site.author_url(&user)),
                esc_attr(bc.attr_str("linkTarget").unwrap_or("_self"))
            );
        }
        return Ok(format!("<div {wrapper}>{avatar}</div>"));
    }

    let Some(comment) = current_comment(bc).await? else { return Ok(String::new()) };
    let alt = format!("{} Avatar", comment.comment_author);
    let email = if comment.user_id != 0 {
        bc.env.site.db.user(comment.user_id).await?.map(|u| u.user_email).unwrap_or(comment.comment_author_email.clone())
    } else {
        comment.comment_author_email.clone()
    };
    let mut avatar = avatar_img(bc, &email, size, &alt, &image_classes, &image_styles);
    if bc.attr_bool("isLink") && !comment.comment_author_url.is_empty() {
        let label = if bc.attr_str("linkTarget") == Some("_blank") { format!("aria-label=\"{}\"", esc_attr(&format!("({} website link, opens in a new tab)", comment.comment_author))) } else { String::new() };
        avatar = format!(
            "<a href=\"{}\" target=\"{}\" {label} class=\"wp-block-avatar__link\">{avatar}</a>",
            esc_url(&comment.comment_author_url),
            esc_attr(bc.attr_str("linkTarget").unwrap_or("_self"))
        );
    }
    Ok(format!("<div {wrapper}>{avatar}</div>"))
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
    let required_indicator = " <span class=\"required\">*</span>";
    let required_text = " <span class=\"required-field-message\">Required fields are marked <span class=\"required\">*</span></span>";
    let required_attr = " required";

    let author = format!(
        "<p class=\"comment-form-author\"><label for=\"author\">Name{}</label> <input id=\"author\" name=\"author\" type=\"text\" value=\"\" size=\"30\" maxlength=\"245\" autocomplete=\"name\"{} /></p>",
        if require { required_indicator } else { "" },
        if require { required_attr } else { "" }
    );
    let email = format!(
        "<p class=\"comment-form-email\"><label for=\"email\">Email{}</label> <input id=\"email\" name=\"email\" type=\"email\" value=\"\" size=\"30\" maxlength=\"100\" aria-describedby=\"email-notes\" autocomplete=\"email\"{} /></p>",
        if require { required_indicator } else { "" },
        if require { required_attr } else { "" }
    );
    let url = "<p class=\"comment-form-url\"><label for=\"url\">Website</label> <input id=\"url\" name=\"url\" type=\"url\" value=\"\" size=\"30\" maxlength=\"200\" autocomplete=\"url\" /></p>";
    let cookies = if options.get("show_comments_cookies_opt_in") == Some("1") {
        "<p class=\"comment-form-cookies-consent\"><input id=\"wp-comment-cookies-consent\" name=\"wp-comment-cookies-consent\" type=\"checkbox\" value=\"yes\" /> <label for=\"wp-comment-cookies-consent\">Save my name, email, and website in this browser for the next time I comment.</label></p>"
    } else {
        ""
    };
    let comment_field = format!(
        "<p class=\"comment-form-comment\"><label for=\"comment\">Comment{required_indicator}</label> <textarea id=\"comment\" name=\"comment\" cols=\"45\" rows=\"8\" maxlength=\"65525\"{required_attr}></textarea></p>"
    );
    let notes = format!("<p class=\"comment-notes\"><span id=\"email-notes\">Your email address will not be published.</span>{required_text}</p>");

    let q = bc.env.query;
    let request = remove_query_arg(&format!("{}{}", q.url_path, if q.raw_query.is_empty() { String::new() } else { format!("?{}", q.raw_query) }), "replytocom");
    let threaded = options.get("thread_comments") == Some("1");
    let cancel = if threaded {
        format!(" <small><a rel=\"nofollow\" id=\"cancel-comment-reply-link\" href=\"{}#respond\" style=\"display:none;\">Cancel reply</a></small>", esc_url(&request))
    } else {
        String::new()
    };
    let mut fields = String::new();
    for field in [&author, &email, url, cookies] {
        if !field.is_empty() {
            fields.push_str(field);
            fields.push('\n');
        }
    }
    let submit = "<input name=\"submit\" type=\"submit\" id=\"submit\" class=\"wp-block-button__link wp-element-button\" value=\"Post Comment\" />";
    let id_fields = format!("<input type='hidden' name='comment_post_ID' value='{post_id}' id='comment_post_ID' />\n<input type='hidden' name='comment_parent' id='comment_parent' value='0' />\n");
    let action = esc_url(&site.site_url("/wp-comments-post.php"));
    bc.env.with_state(|s| if !s.scripts.iter().any(|x| x == "comment-reply") { s.scripts.push("comment-reply".to_owned()) });
    Ok(format!(
        "\t<div id=\"respond\" {wrapper}>\n\t\t<h3 id=\"reply-title\" class=\"comment-reply-title\">Leave a Reply{cancel}</h3><form action=\"{action}\" method=\"post\" id=\"commentform\" class=\"comment-form\">{notes}{comment_field}{fields}<p class=\"form-submit wp-block-button\">{submit} {id_fields}</p></form>\t</div><!-- #respond -->\n\t"
    ))
}
