//! Blocks that can show the featured image of the current post (`core/cover`, `core/media-text`),
//! plus `core/details` and `core/footnotes`.

use rustpress_core::formatting::{esc_attr, esc_url, strip_tags};
use rustpress_core::kses::kses_post;
use serde_json::Value;

use crate::Result;
use crate::content::post_password_required;
use crate::html::{Tag, TagEditor, tags};
use crate::images::{self, AttrValue, ImageArgs};
use crate::render::BlockCtx;

use super::global_post;

fn has_class(tag: &Tag, class: &str) -> bool {
    tag.attr("class").is_some_and(|c| c.split_ascii_whitespace().any(|x| x == class))
}

/// `round( $point * 100 ) . '%'` pairs of a focal point.
fn focal_point(bc: &BlockCtx<'_>) -> Option<String> {
    let point = bc.attrs.get("focalPoint").filter(|p| !p.is_null())?;
    let part = |key: &str| (point.get(key).and_then(Value::as_f64).unwrap_or(0.0) * 100.0).round() as i64;
    Some(format!("{}% {}%", part("x"), part("y")))
}

/// The featured image of the block's post: its attachment ID.
async fn thumbnail_id(bc: &BlockCtx<'_>) -> Result<u64> {
    match global_post(bc).await? {
        Some(post) => images::post_thumbnail_id(bc.env, post.id).await,
        None => Ok(0),
    }
}

// ------------------------------------------------------------------- core/cover

/// `render_block_core_cover()`: the featured image as the background. The embed-video background
/// needs the oEmbed network and leaves the markup as saved.
pub(super) async fn cover(bc: &BlockCtx<'_>) -> Result<String> {
    let content = bc.content.clone();
    if bc.attr_str("backgroundType") != Some("image") || !bc.attr_bool("useFeaturedImage") {
        return Ok(content);
    }
    let position = focal_point(bc);
    let size = bc.attr_str("sizeSlug").filter(|s| !s.is_empty()).map(str::to_owned);
    let id = thumbnail_id(bc).await?;

    let image = if !(bc.attr_bool("hasParallax") || bc.attr_bool("isRepeated")) {
        if id == 0 {
            String::new()
        } else {
            let mut attrs = vec![("class".to_owned(), AttrValue::Text("wp-block-cover__image-background".into())), ("data-object-fit".to_owned(), AttrValue::Text("cover".into()))];
            if let Some(position) = &position {
                attrs.push(("data-object-position".into(), AttrValue::Text(position.clone())));
                attrs.push(("style".into(), AttrValue::Text(format!("object-position:{position};"))));
            }
            images::get_attachment_image(bc.env, id, size.as_deref().unwrap_or("post-thumbnail"), &ImageArgs { attrs, post_thumbnail: true, dimensions: None }).await?
        }
    } else {
        let url = if id == 0 { None } else { images::attachment_image_url(bc.env, id, size.as_deref().unwrap_or("full")).await?.filter(|u| !u.is_empty()) };
        let Some(url) = url else { return Ok(content) };
        let alt = strip_tags(&images::attachment_alt(bc.env, id).await?).trim().to_owned();
        let mut div = String::from("<div");
        if !alt.is_empty() {
            div.push_str(&format!(" role=\"img\" aria-label=\"{}\"", esc_attr(&alt)));
        }
        let mut classes = format!("wp-block-cover__image-background wp-image-{id}");
        if bc.attr_bool("hasParallax") {
            classes.push_str(" has-parallax");
        }
        if bc.attr_bool("isRepeated") {
            classes.push_str(" is-repeated");
        }
        let style = format!("background-position:{};background-image:url({});", position.as_deref().unwrap_or("50% 50%"), esc_url(&url));
        div.push_str(&format!(" class=\"{classes}\" style=\"{}\"></div>", esc_attr(&style)));
        div
    };

    let inner = tags(&content, 0).into_iter().find(|t| !t.is_closer && t.name == "div" && has_class(t, "wp-block-cover__inner-container"));
    Ok(match inner {
        Some(tag) => format!("{}{image}{}", &content[..tag.start], &content[tag.start..]),
        None => content,
    })
}

// -------------------------------------------------------------- core/media-text

/// Edits the `n`-th start tag matching `matches`, counted in the current `content`.
fn edit_nth(content: &str, n: usize, matches: impl Fn(&Tag) -> bool) -> Option<TagEditor> {
    let tag = tags(content, 0).into_iter().filter(|t| !t.is_closer && matches(t)).nth(n)?;
    Some(TagEditor::from_tag(content, tag))
}

/// `render_block_core_media_text()`: the featured image as the media.
pub(super) async fn media_text(bc: &BlockCtx<'_>) -> Result<String> {
    let mut content = bc.content.clone();
    if !bc.attr_bool("useFeaturedImage") {
        return Ok(content);
    }
    let id = thumbnail_id(bc).await?;
    let url = if id == 0 { None } else { images::attachment_image_url(bc.env, id, "post-thumbnail").await?.filter(|u| !u.is_empty()) };
    let Some(url) = url else { return Ok(content) };

    let on_right = bc.attr_str("mediaPosition") == Some("right");
    let image_fill = bc.attr_bool("imageFill");
    let focal = focal_point(bc).unwrap_or_else(|| "50% 50%".to_owned());
    let unique_id = bc.env.with_state(|s| s.unique_id("wp-block-media-text__media-"));

    if image_fill {
        let count = tags(&content, 0).into_iter().filter(|t| !t.is_closer && t.name == "div" && has_class(t, "wp-block-media-text")).count();
        for n in 0..count {
            let Some(mut editor) = edit_nth(&content, n, |t| t.name == "div" && has_class(t, "wp-block-media-text")) else { break };
            let classes: Vec<String> = editor.get_attribute("class").unwrap_or("").split_ascii_whitespace().filter(|c| *c != "is-image-fill").map(str::to_owned).collect();
            editor.set_attribute("class", &classes.join(" "));
            editor.add_class("is-image-fill-element");
            content = editor.finish();
        }
    }

    let is_media_figure = |t: &Tag| t.name == "figure" && has_class(t, "wp-block-media-text__media");
    let figures = tags(&content, 0).into_iter().filter(|t| !t.is_closer && is_media_figure(t)).count();
    if figures > 0 {
        let n = if on_right { figures - 1 } else { 0 };
        if let Some(mut editor) = edit_nth(&content, n, is_media_figure) {
            editor.set_attribute("id", &unique_id);
            content = editor.finish();
        }
    }

    // The image goes right after a figure that carries nothing but the new ID and its class.
    let plain = tags(&content, 0)
        .into_iter()
        .find(|t| !t.is_closer && t.name == "figure" && t.attr("id") == Some(unique_id.as_str()) && t.attrs.len() == 2 && t.attr("class") == Some("wp-block-media-text__media") && t.attrs[0].name == "id");
    if let Some(figure) = plain {
        content.insert_str(figure.end, "<img class=\"wp-block-media-text__featured_image\">");
    }

    let Some(figure) = tags(&content, 0).into_iter().find(|t| !t.is_closer && t.name == "figure" && t.attr("id") == Some(unique_id.as_str())) else { return Ok(content) };
    let mut figure_editor = TagEditor::from_tag(&content, figure.clone());
    figure_editor.remove_attribute("id");
    let without_id = figure_editor.finish();
    // `next_tag()` carries on after the figure, in the document with the ID removed.
    let image = tags(&without_id, figure.start).into_iter().find(|t| !t.is_closer && t.name == "img" && has_class(t, "wp-block-media-text__featured_image"));
    let Some(image) = image else { return Ok(content) };
    let alt = strip_tags(&images::attachment_alt(bc.env, id).await?).trim().to_owned();
    let media_size = bc.attr_str("mediaSizeSlug").filter(|s| !s.is_empty()).unwrap_or("full").to_owned();
    let mut editor = TagEditor::from_tag(&without_id, image);
    editor.set_attribute("src", &esc_url(&url));
    editor.set_attribute("class", &format!("wp-image-{id} size-{media_size}"));
    editor.set_attribute("alt", &alt);
    if image_fill {
        editor.set_attribute("style", &format!("object-position:{focal};"));
    }
    Ok(editor.finish())
}

// ------------------------------------------------------------------ core/details

/// `block_core_details_set_img_fetchpriority_low()`: images of a closed details block are not urgent.
pub(super) fn details_images_low_priority(raw_attrs: &serde_json::Map<String, Value>, content: String) -> String {
    if raw_attrs.get("showContent").and_then(Value::as_bool).unwrap_or(false) {
        return content;
    }
    let count = tags(&content, 0).into_iter().filter(|t| !t.is_closer && t.name == "img").count();
    let mut content = content;
    for n in 0..count {
        let Some(mut editor) = edit_nth(&content, n, |t| t.name == "img") else { break };
        editor.set_attribute("fetchpriority", "low");
        content = editor.finish();
    }
    content
}

// ---------------------------------------------------------------- core/footnotes

/// `render_block_core_footnotes()`.
pub(super) async fn footnotes(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id().filter(|id| *id != 0) else { return Ok(String::new()) };
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    if post_password_required(&post) {
        return Ok(String::new());
    }
    let rows = bc.env.site.db.post_meta(post_id).await?;
    let raw = rows.iter().find(|m| m.meta_key.as_deref() == Some("footnotes")).and_then(|m| m.meta_value.clone()).unwrap_or_default();
    if raw.is_empty() || raw == "0" {
        return Ok(String::new());
    }
    let Ok(Value::Array(notes)) = serde_json::from_str::<Value>(&raw) else { return Ok(String::new()) };
    if notes.is_empty() {
        return Ok(String::new());
    }
    let label = bc.env.site.tr("Jump to footnote reference %1$d");
    let mut items = String::new();
    for (index, note) in notes.iter().enumerate() {
        let text = |key: &str| match note.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(other) => other.to_string(),
        };
        let id = esc_attr(&text("id"));
        let aria = esc_attr(&rustpress_core::i18n::sprintf(&label, &[&(index + 1).to_string()]));
        items.push_str(&format!("<li id=\"{id}\">{} <a href=\"#{id}-link\" aria-label=\"{aria}\">↩︎</a></li>", kses_post(&text("content"))));
    }
    Ok(format!("<ol {}>{items}</ol>", bc.wrapper_attributes(&[])))
}

// -------------------------------------------------------------------- core/video

/// `render_block_core_video()`: width, height and aspect ratio from the attachment's metadata,
/// so the browser can reserve the space before the video loads.
pub(super) async fn video(bc: &BlockCtx<'_>) -> Result<String> {
    let content = bc.content.clone();
    if !content.contains("<video") {
        return Ok(content);
    }
    let Some(id) = bc.attrs.get("id").and_then(Value::as_u64).filter(|id| *id > 0) else { return Ok(content) };
    match bc.env.site.db.post(id).await? {
        Some(post) if post.post_type == "attachment" => {}
        _ => return Ok(content),
    }
    let Some(meta) = images::attachment_metadata(bc.env, id).await? else { return Ok(content) };
    let (width, height) = (meta.width(), meta.height());
    if width == 0 || height == 0 {
        return Ok(content);
    }
    let Some(tag) = tags(&content, 0).into_iter().find(|t| !t.is_closer && t.name == "video") else { return Ok(content) };
    let mut editor = TagEditor::from_tag(&content, tag);
    editor.set_attribute("width", &width.to_string());
    editor.set_attribute("height", &height.to_string());
    let style = editor.get_attribute("style").unwrap_or("").to_owned();
    editor.set_attribute("style", &format!("aspect-ratio: {width} / {height};{style}"));
    Ok(editor.finish())
}

// ---------------------------------------------------------------- core/accordion

/// The first start tag at or after `from` with the class, edited by `change`; returns the new document
/// and the offset right after the edited tag.
fn change_tag(content: &str, from: usize, class: &str, change: impl FnOnce(&mut TagEditor)) -> Option<(String, usize)> {
    let tag = tags(content, from).into_iter().find(|t| !t.is_closer && has_class(t, class))?;
    let mut editor = TagEditor::from_tag(content, tag);
    change(&mut editor);
    let (start, _, text) = editor.tag_replacement();
    Some((editor.finish(), start + text.len()))
}

/// `render_block_core_accordion()`.
pub(super) fn accordion(bc: &BlockCtx<'_>) -> Result<String> {
    let content = bc.content.clone();
    if content.is_empty() {
        return Ok(content);
    }
    let autoclose = if bc.attr_bool("autoclose") { "true" } else { "false" };
    let updated = change_tag(&content, 0, "wp-block-accordion", |e| {
        e.set_attribute("data-wp-interactive", "core/accordion");
        e.set_attribute("data-wp-context", &format!("{{ \"autoclose\": {autoclose}, \"accordionItems\": [] }}"));
    });
    bc.env.with_state(|s| s.script_modules.insert("@wordpress/block-library/accordion/view".to_owned()));
    Ok(updated.map_or(content, |(html, _)| html))
}

/// `block_core_accordion_item_render()`.
pub(super) fn accordion_item(bc: &BlockCtx<'_>) -> Result<String> {
    let mut content = bc.content.clone();
    if content.is_empty() {
        return Ok(content);
    }
    let unique_id = bc.env.with_state(|s| {
        let id = s.unique_id("accordion-item-");
        s.interactivity.state_closure("core/accordion", "isOpen");
        id
    });
    let open = bc.attr_bool("openByDefault");
    let chain = change_tag(&content, 0, "wp-block-accordion-item", |e| {
        e.set_attribute("data-wp-context", &format!("{{ \"id\": \"{unique_id}\", \"openByDefault\": {} }}", if open { "true" } else { "false" }));
        e.set_attribute("data-wp-class--is-open", "state.isOpen");
        e.set_attribute("data-wp-init", "callbacks.initAccordionItems");
        e.set_attribute("data-wp-on-window--hashchange", "callbacks.hashChange");
    })
    .and_then(|(html, at)| {
        change_tag(&html, at, "wp-block-accordion-heading__toggle", |e| {
            e.set_attribute("data-wp-on--click", "actions.toggle");
            e.set_attribute("id", &unique_id);
            e.set_attribute("aria-controls", &format!("{unique_id}-panel"));
            e.set_attribute("data-wp-bind--aria-expanded", "state.isOpen");
        })
    })
    .and_then(|(html, at)| {
        change_tag(&html, at, "wp-block-accordion-panel", |e| {
            e.set_attribute("id", &format!("{unique_id}-panel"));
            e.set_attribute("aria-labelledby", &unique_id);
            e.set_attribute("data-wp-bind--hidden", "state.isHidden");
            e.set_attribute("data-wp-on--beforematch", "actions.handleBeforeMatch");
        })
    });
    if let Some((html, _)) = chain {
        content = html;
    }
    if !open {
        content = details_images_low_priority(&serde_json::Map::new(), content);
    }
    Ok(content)
}
