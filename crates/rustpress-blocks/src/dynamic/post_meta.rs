//! Post title, date, terms and author blocks.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::kses::kses_post;

use crate::Result;
use crate::content::{post_password_required, the_title_for};
use crate::render::BlockCtx;

use super::{global_post, heading_tag};

fn has_link_color(bc: &BlockCtx<'_>) -> bool {
    bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null())
}

pub(super) async fn post_title(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id() else { return Ok(String::new()) };
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    let mut title = the_title_for(bc.env.site, &post);
    if title.is_empty() {
        return Ok(String::new());
    }
    let tag = heading_tag(bc.attrs.get("level").and_then(|v| v.as_u64()), "h2");
    if bc.attr_bool("isLink") {
        let rel = bc.attr_str("rel").filter(|r| !r.is_empty()).map(|r| format!("rel=\"{}\"", esc_attr(r))).unwrap_or_default();
        let link = bc.env.site.permalink(&post).await?;
        let _ = post_id;
        title = format!("<a href=\"{}\" target=\"{}\" {rel}>{title}</a>", esc_url(&link), esc_attr(bc.attr_str("linkTarget").unwrap_or("_self")));
    }
    let mut classes = Vec::new();
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    Ok(format!("<{tag} {wrapper}>{title}</{tag}>"))
}

pub(super) async fn post_date(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    let modified = bc.attr_str("displayType") == Some("modified");
    let mut classes = Vec::new();
    if modified {
        classes.push("wp-block-post-date__modified-date".to_owned());
    }
    // The post-data binding source hides dates of protected posts.
    if post_password_required(&post) {
        return Ok(String::new());
    }
    let site = &bc.env.site;
    let source = if modified { &post.post_modified } else { &post.post_date };
    let Some(timestamp) = site.timezone.local_timestamp(source) else { return Ok(String::new()) };
    let datetime = bc.attr_str("datetime").map(str::to_owned).unwrap_or_else(|| site.date("c", timestamp));

    let format = bc.attr_str("format").filter(|f| !f.is_empty()).map(str::to_owned).unwrap_or_else(|| site.options.string("date_format", "F j, Y"));
    let formatted = site.date(&format, timestamp);
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    let mut time_tag = format!("<time datetime=\"{}\">{}</time>", esc_attr(&datetime), esc_html(&formatted));
    if bc.attr_bool("isLink") && bc.context.post_id().is_some() {
        let link = site.permalink(&post).await?;
        time_tag = format!("<a href=\"{}\">{time_tag}</a>", esc_url(&link));
    }
    Ok(format!("<div {wrapper}>{time_tag}</div>"))
}

pub(super) async fn post_terms(bc: &BlockCtx<'_>) -> Result<String> {
    let (Some(post_id), Some(taxonomy)) = (bc.context.post_id(), bc.attr_str("term")) else { return Ok(String::new()) };
    if !matches!(taxonomy, "category" | "post_tag") && bc.env.site.db.terms(taxonomy).await?.is_empty() {
        return Ok(String::new());
    }
    let mut classes = vec![format!("taxonomy-{taxonomy}")];
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let separator = bc.attr_str("separator").filter(|s| !s.is_empty()).unwrap_or(" ");
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    let mut prefix = format!("<div {wrapper}>");
    if let Some(p) = bc.attr_str("prefix").filter(|p| !p.is_empty()) {
        prefix.push_str(&format!("<span class=\"wp-block-post-terms__prefix\">{}</span>", kses_post(p)));
    }
    let mut suffix = "</div>".to_owned();
    if let Some(s) = bc.attr_str("suffix").filter(|s| !s.is_empty()) {
        suffix = format!("<span class=\"wp-block-post-terms__suffix\">{}</span>{suffix}", kses_post(s));
    }

    let mut terms = bc.env.site.db.terms_for_post(post_id, Some(taxonomy)).await?;
    if terms.is_empty() {
        return Ok(String::new());
    }
    terms.sort_by_key(|t| (t.name.to_lowercase(), t.term_id));
    let mut links = Vec::new();
    for term in &terms {
        let url = bc.env.site.term_link(term).await?;
        links.push(format!("<a href=\"{}\" rel=\"tag\">{}</a>", esc_url(&url), esc_html(&term.name)));
    }
    let glue = format!("<span class=\"wp-block-post-terms__separator\">{}</span>", esc_html(separator));
    Ok(format!("{prefix}{}{suffix}", links.join(&glue)))
}

/// The author blocks render nothing for post types without author support.
fn supports_author(bc: &BlockCtx<'_>) -> bool {
    match bc.context.values.get("postType").and_then(serde_json::Value::as_str) {
        Some(post_type) => bc.env.site.post_type_supports(post_type, "author"),
        None => true,
    }
}

pub(super) async fn post_author_name(bc: &BlockCtx<'_>) -> Result<String> {
    let author_id = match bc.context.post_id() {
        Some(_) => global_post(bc).await?.map_or(0, |p| p.post_author),
        None => bc.env.query.vars.u64("author").unwrap_or(0),
    };
    if author_id == 0 || !supports_author(bc) {
        return Ok(String::new());
    }
    let Some(author) = bc.env.site.db.user(author_id).await? else { return Ok(String::new()) };
    let mut name = author.display_name.clone();
    if bc.attr_bool("isLink") {
        name = format!(
            "<a href=\"{}\" target=\"{}\" class=\"wp-block-post-author-name__link\">{name}</a>",
            esc_url(&bc.env.site.author_url(&author)),
            esc_attr(bc.attr_str("linkTarget").unwrap_or("_self"))
        );
    }
    let mut classes = Vec::new();
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    Ok(format!("<div {wrapper}>{name}</div>"))
}

/// `core/post-excerpt`.
pub(super) async fn post_excerpt(bc: &BlockCtx<'_>) -> Result<String> {
    if bc.context.post_id().is_none() {
        return Ok(String::new());
    }
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    let more_text = match bc.attr_str("moreText").filter(|t| !t.is_empty()) {
        Some(text) => {
            let link = bc.env.site.permalink(&post).await?;
            format!("<a class=\"wp-block-post-excerpt__more-link\" href=\"{}\">{}</a>", esc_url(&link), kses_post(text))
        }
        None => String::new(),
    };
    // The excerpt is generated with a length of 101 words (the editor's limit) and cut to the block's length.
    let more_default = if more_text.is_empty() { " [&hellip;]" } else { "" };
    let mut excerpt = crate::excerpt::get_the_excerpt_with(bc.env, &post, 101, more_default).await?;
    if let Some(length) = bc.attr_u64("excerptLength") {
        excerpt = rustpress_core::formatting::wp_trim_words_with(&excerpt, length as usize, &bc.env.site.tr("&hellip;"), bc.env.site.locale_data().word_count_type);
    }
    let mut classes = Vec::new();
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    let mut content = format!("<p class=\"wp-block-post-excerpt__excerpt\">{excerpt}");
    let on_new_line = bc.attrs.get("showMoreOnNewLine").is_none_or(|v| v.as_bool().unwrap_or(true));
    if on_new_line && !more_text.is_empty() {
        content.push_str(&format!("</p><p class=\"wp-block-post-excerpt__more-text\">{more_text}</p>"));
    } else {
        content.push_str(&format!(" {more_text}</p>"));
    }
    Ok(format!("<div {wrapper}>{content}</div>"))
}

/// `core/post-author`.
pub(super) async fn post_author(bc: &BlockCtx<'_>) -> Result<String> {
    let author_id = match bc.context.post_id() {
        None => bc.env.query.vars.u64("author").unwrap_or(0),
        Some(_) => global_post(bc).await?.map_or(0, |p| p.post_author),
    };
    if author_id == 0 || !supports_author(bc) {
        return Ok(String::new());
    }
    let Some(user) = bc.env.site.db.user(author_id).await? else { return Ok(String::new()) };
    let size = bc.attr_u64("avatarSize").filter(|s| *s != 0);
    let avatar = size.map(|size| super::comments::avatar_img(bc, &user.user_email, size, "", "", "")).unwrap_or_default();
    let link = bc.env.site.author_url(&user);
    let mut author_name = esc_html(&user.display_name);
    if bc.attr_bool("isLink") && bc.attr_str("linkTarget").is_some_and(|t| !t.is_empty()) {
        author_name = format!("<a href=\"{}\" target=\"{}\">{author_name}</a>", esc_url(&link), esc_attr(bc.attr_str("linkTarget").unwrap_or_default()));
    }
    let byline = bc.attr_str("byline").filter(|b| !b.is_empty());
    let mut classes = Vec::new();
    if let Some(justification) = bc.attr_str("itemsJustification") {
        classes.push(format!("items-justified-{justification}"));
    }
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    let mut out = format!("<div {wrapper}>");
    if bc.attr_bool("showAvatar") {
        out.push_str(&format!("<div class=\"wp-block-post-author__avatar\">{avatar}</div>"));
    }
    out.push_str("<div class=\"wp-block-post-author__content\">");
    if let Some(byline) = byline {
        out.push_str(&format!("<p class=\"wp-block-post-author__byline\">{}</p>", kses_post(byline)));
    }
    out.push_str(&format!("<p class=\"wp-block-post-author__name\">{author_name}</p>"));
    if bc.attr_bool("showBio") {
        let bio = bc.env.site.db.user_meta(author_id, "description").await?.unwrap_or_default();
        out.push_str(&format!("<p class=\"wp-block-post-author__bio\">{bio}</p>"));
    }
    out.push_str("</div></div>");
    Ok(out)
}