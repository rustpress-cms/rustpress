//! Post title, date, terms and author blocks.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

use crate::Result;
use crate::content::{post_password_required, the_title};
use crate::render::BlockCtx;

use super::{global_post, heading_tag};

fn has_link_color(bc: &BlockCtx<'_>) -> bool {
    bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null())
}

pub(super) async fn post_title(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id() else { return Ok(String::new()) };
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    let mut title = the_title(&post);
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
    let datetime = bc.attr_str("datetime").map(str::to_owned).unwrap_or_else(|| site.timezone.format("c", timestamp));

    let format = bc.attr_str("format").filter(|f| !f.is_empty()).map(str::to_owned).unwrap_or_else(|| site.options.string("date_format", "F j, Y"));
    let formatted = site.timezone.format(&format, timestamp);
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
        prefix.push_str(&format!("<span class=\"wp-block-post-terms__prefix\">{p}</span>"));
    }
    let mut suffix = "</div>".to_owned();
    if let Some(s) = bc.attr_str("suffix").filter(|s| !s.is_empty()) {
        suffix = format!("<span class=\"wp-block-post-terms__suffix\">{s}</span>{suffix}");
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

pub(super) async fn post_author_name(bc: &BlockCtx<'_>) -> Result<String> {
    let author_id = match bc.context.post_id() {
        Some(_) => global_post(bc).await?.map_or(0, |p| p.post_author),
        None => bc.env.query.vars.u64("author").unwrap_or(0),
    };
    if author_id == 0 {
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
