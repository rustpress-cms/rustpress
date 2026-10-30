//! `core/latest-posts` (`render_block_core_latest_posts()`).

use std::sync::Arc;

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_db::{Order, OrderBy, PostQuery};
use serde_json::Value;

use crate::Result;
use crate::content::{post_password_required, the_title_for};
use crate::images::{self, AttrValue, ImageArgs};
use crate::render::BlockCtx;

pub(super) async fn latest_posts(bc: &BlockCtx<'_>) -> Result<String> {
    let site = bc.env.site;
    let attrs = &bc.attrs;
    let number = attrs.get("postsToShow").and_then(Value::as_u64).unwrap_or(5);
    let order = if attrs.get("order").and_then(Value::as_str) == Some("asc") { Order::Asc } else { Order::Desc };
    let order_by = match attrs.get("orderBy").and_then(Value::as_str) {
        Some("title") => OrderBy::Title,
        Some("modified") => OrderBy::Modified,
        Some("id") => OrderBy::Id,
        Some("author") => OrderBy::Author,
        Some("name") => OrderBy::Slug,
        Some("menu_order") => OrderBy::MenuOrder,
        _ => OrderBy::Date,
    };
    let mut query = PostQuery { post_types: vec!["post".into()], limit: Some(number), order_by, order, ..PostQuery::default() };
    if let Some(categories) = attrs.get("categories").and_then(Value::as_array) {
        let mut ids = Vec::new();
        for category in categories {
            if let Some(id) = category.get("id").and_then(Value::as_u64)
                && let Some(term) = site.db.term_by_id(Some("category"), id).await?
            {
                ids.push(term.term_taxonomy_id);
            }
        }
        if !ids.is_empty() {
            query.tax_groups.push(ids);
        }
    }
    if let Some(author) = attrs.get("selectedAuthor").and_then(Value::as_u64) {
        query.author = Some(author);
    }
    let posts = site.db.posts(&query).await?;

    let display = |key: &str| attrs.get(key).is_some_and(|v| v.as_bool() == Some(true));
    let mut items = String::new();
    // The block walks its posts with `the_post()` and then sets up the post that was current before.
    let previous = super::global_post(bc).await?;
    for post in posts {
        let post = Arc::new(post);
        bc.env.with_state(|s| s.looped_posts.push(post.id));
        let link = esc_url(&site.permalink(&post).await?);
        let mut title = the_title_for(site, &post);
        if title.is_empty() {
            title = site.tr("(no title)");
        }
        items.push_str("<li>");
        if display("displayFeaturedImage") {
            let thumbnail = images::post_thumbnail_id(bc.env, post.id).await?;
            if thumbnail != 0 {
                let mut style = String::new();
                if let Some(width) = attrs.get("featuredImageSizeWidth").filter(|v| !v.is_null()) {
                    style.push_str(&format!("max-width:{}px;", scalar(width)));
                }
                if let Some(height) = attrs.get("featuredImageSizeHeight").filter(|v| !v.is_null()) {
                    style.push_str(&format!("max-height:{}px;", scalar(height)));
                }
                let mut classes = "wp-block-latest-posts__featured-image".to_owned();
                if let Some(align) = attrs.get("featuredImageAlign").and_then(Value::as_str) {
                    classes.push_str(&format!(" align{align}"));
                }
                let size = attrs.get("featuredImageSizeSlug").and_then(Value::as_str).unwrap_or("thumbnail");
                let image_attrs = vec![("style".to_owned(), AttrValue::Text(esc_attr(&style)))];
                let mut image = images::get_attachment_image(bc.env, thumbnail, size, &ImageArgs { attrs: image_attrs, post_thumbnail: true, thumbnail_of: Some(post.id), dimensions: None }).await?;
                if display("addLinkToFeaturedImage") {
                    image = format!("<a href=\"{link}\" aria-label=\"{}\">{image}</a>", esc_attr(&title));
                }
                items.push_str(&format!("<div class=\"{}\">{image}</div>", esc_attr(&classes)));
            }
        }
        items.push_str(&format!("<a class=\"wp-block-latest-posts__post-title\" href=\"{link}\">{title}</a>"));
        if display("displayAuthor")
            && let Some(author) = site.db.user(post.post_author).await?
            && !author.display_name.is_empty()
        {
            items.push_str(&format!("<div class=\"wp-block-latest-posts__post-author\">{}</div>", rustpress_core::i18n::sprintf(&site.tr("by %s"), &[&author.display_name])));
        }
        if display("displayPostDate") && site.plugins.hides_date(site, &post).await? {
            items.push_str("<time datetime=\"\" class=\"wp-block-latest-posts__post-date\"></time>");
        } else if display("displayPostDate") {
            let ts = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
            items.push_str(&format!(
                "<time datetime=\"{}\" class=\"wp-block-latest-posts__post-date\">{}</time>",
                esc_attr(&site.date("c", ts)),
                site.date(&site.options.string("date_format", "F j, Y"), ts)
            ));
        }
        let content_kind = attrs.get("displayPostContentRadio").and_then(Value::as_str).unwrap_or("excerpt");
        if display("displayPostContent") && content_kind == "excerpt" {
            let length = attrs.get("excerptLength").and_then(Value::as_u64).unwrap_or(55) as usize;
            let mut excerpt = crate::excerpt::get_the_excerpt_with(bc.env, &post, length, " [&hellip;]").await?;
            if let Some(stripped) = excerpt.strip_suffix(" [&hellip;]") {
                excerpt = format!(
                    "{stripped}{}",
                    rustpress_core::i18n::sprintf(&site.tr("… <a class=\"wp-block-latest-posts__read-more\" href=\"%1$s\" rel=\"noopener\">Read more<span class=\"screen-reader-text\">: %2$s</span></a>"), &[&link, &esc_html(&title)])
                );
            }
            if post_password_required(&post) {
                excerpt = site.tr("This content is password protected.");
            }
            items.push_str(&format!("<div class=\"wp-block-latest-posts__post-excerpt\">{excerpt}</div>"));
        }
        if display("displayPostContent") && content_kind == "full_post" {
            let content = if post_password_required(&post) {
                site.tr("This content is password protected.")
            } else {
                crate::content::the_content(bc.env, &post, &post.post_content).await?
            };
            items.push_str(&format!("<div class=\"wp-block-latest-posts__post-full-content\">{}</div>", content.replace("]]>", "]]&gt;")));
        }
        items.push_str("</li>\n");
    }
    if let Some(previous) = previous {
        bc.env.with_state(|s| s.looped_posts.push(previous.id));
    }

    let layout = attrs.get("layout");
    let legacy = if attrs.get("postLayout").and_then(Value::as_str) == Some("grid") { "grid" } else { "default" };
    let layout_type = layout.and_then(|l| l.get("type")).and_then(Value::as_str).unwrap_or(legacy);
    let columns = layout.and_then(|l| l.get("columnCount")).or_else(|| attrs.get("columns")).filter(|v| !v.is_null() && v.as_u64() != Some(0));
    let mut classes = vec!["wp-block-latest-posts__list".to_owned()];
    if layout_type == "grid" {
        classes.push("is-grid".into());
        if let Some(columns) = columns {
            classes.push(format!("columns-{}", scalar(columns)));
            if layout.and_then(|l| l.get("minimumColumnWidth")).is_some_and(|v| !v.is_null() && v.as_str() != Some("")) {
                classes.push("has-native-responsive-grid".into());
            }
        }
    }
    if display("displayPostDate") {
        classes.push("has-dates".into());
    }
    if display("displayAuthor") {
        classes.push("has-author".into());
    }
    if attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some() {
        classes.push("has-link-color".into());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    Ok(format!("<ul {wrapper}>{items}</ul>"))
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
