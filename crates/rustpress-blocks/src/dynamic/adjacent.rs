//! `core/post-navigation-link`.

use rustpress_core::formatting::esc_url;
use rustpress_db::Post;
use serde_json::Value;

use crate::Result;
use crate::content::the_title;
use crate::render::BlockCtx;

use super::global_post;

/// `get_adjacent_post()` within the same post type, by publication date (then ID).
async fn adjacent(bc: &BlockCtx<'_>, post: &Post, previous: bool) -> Result<Option<Post>> {
    let db = &bc.env.site.db;
    match db.adjacent_post_id(&post.post_type, &post.post_date, post.id, previous).await? {
        Some(id) => Ok(db.post(id).await?),
        None => Ok(None),
    }
}

pub(super) async fn render(bc: &BlockCtx<'_>) -> Result<String> {
    let q = bc.env.query;
    if !q.flags.is_singular() {
        return Ok(String::new());
    }
    let kind = bc.attr_str("type").unwrap_or("next");
    if kind != "next" && kind != "previous" {
        return Ok(String::new());
    }
    let mut classes = format!("post-navigation-link-{kind}");
    if let Some(a) = bc.attr_str("textAlign") {
        classes.push_str(&format!(" has-text-align-{a}"));
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes)]);

    let mut format = "%link".to_owned();
    let mut link = if kind == "next" { "Next".to_owned() } else { "Previous".to_owned() };
    let mut label = String::new();
    if let Some(l) = bc.attr_str("label").filter(|l| !l.is_empty()) {
        label = l.to_owned();
        link = label.clone();
    }
    let show_title = bc.attr_bool("showTitle");
    let link_label = bc.attrs.get("linkLabel").is_some_and(|v| !crate::style_engine::is_empty(v));
    if show_title {
        if !link_label {
            if !label.is_empty() {
                format = format!("<span class=\"post-navigation-link__label\">{label}</span> %link");
            }
            link = "%title".to_owned();
        } else if !label.is_empty() {
            link = format!("<span class=\"post-navigation-link__label\">{label}</span> <span class=\"post-navigation-link__title\">%title</span>");
        } else {
            let label = if kind == "next" { "Next:" } else { "Previous:" };
            link = format!("<span class=\"post-navigation-link__label\">{label}</span> <span class=\"post-navigation-link__title\">%title</span>");
        }
    }
    if let Some(arrow) = bc.attr_str("arrow").filter(|a| *a != "none") {
        let glyph = match (arrow, kind) {
            ("arrow", "next") => Some("→"),
            ("arrow", _) => Some("←"),
            ("chevron", "next") => Some("»"),
            ("chevron", _) => Some("«"),
            _ => None,
        };
        if let Some(glyph) = glyph {
            format = if kind == "next" {
                format!("%link<span class=\"wp-block-post-navigation-link__arrow-next is-arrow-{arrow}\" aria-hidden=\"true\">{glyph}</span>")
            } else {
                format!("<span class=\"wp-block-post-navigation-link__arrow-previous is-arrow-{arrow}\" aria-hidden=\"true\">{glyph}</span>%link")
            };
        }
    }

    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    let content = match adjacent(bc, &post, kind == "previous").await? {
        None => String::new(),
        Some(other) => {
            let title = if other.post_title.is_empty() {
                if kind == "next" { "Next Post".to_owned() } else { "Previous Post".to_owned() }
            } else {
                the_title(&Post { post_password: String::new(), ..other.clone() })
            };
            let inlink = link.replace("%title", &title);
            let permalink = bc.env.site.permalink(&other).await?;
            let rel = if kind == "next" { "next" } else { "prev" };
            let anchor = format!("<a href=\"{}\" rel=\"{rel}\">{inlink}</a>", esc_url(&permalink));
            format.replace("%link", &anchor)
        }
    };
    let _: Option<&Value> = None;
    Ok(format!("<div {wrapper}>{content}</div>"))
}
