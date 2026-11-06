//! `core/gallery`: `block_core_gallery_render()`.

use rand::seq::SliceRandom;
use regex::Regex;
use rustpress_core::formatting::{esc_attr, esc_url};
use rustpress_db::{Order, OrderBy, PostQuery};
use serde_json::{Map, Value, json};

use crate::Result;
use crate::env::Context;
use crate::html::{TagEditor, tags};
use crate::images::{self, AttrValue, ImageArgs};
use crate::parser::{Block, Content};
use crate::render::BlockCtx;
use crate::supports::safecss_filter_attr;

const GALLERY_FALLBACK_GAP: &str = "var( --wp--style--gallery-gap-default, var( --gallery-block--gutter-size, var( --wp--style--block-gap, 0.5em ) ) )";

/// `empty()` of an attribute.
fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty() || s == "0",
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
    }
}

/// `block_core_gallery_get_column_gap_value()`.
fn gallery_column_gap(gap: Option<&Value>) -> String {
    let text = match gap {
        Some(Value::Object(sides)) => sides.get("left").and_then(Value::as_str).unwrap_or(GALLERY_FALLBACK_GAP).to_owned(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    // Values with characters `safecss_filter_attr()` rejects are dropped.
    let unsafe_value = text.contains(['\\', '(', '&', '=', '}']) || text.contains("/*");
    let mut gap = if text.is_empty() || unsafe_value { String::new() } else { text };
    if let Some(at) = gap.rfind('|').filter(|_| gap.contains("var:preset|spacing|")) {
        gap = format!("var(--wp--preset--spacing--{})", crate::style_engine::to_kebab_case(&gap[at + 1..]));
    }
    let column = if gap.is_empty() { GALLERY_FALLBACK_GAP.to_owned() } else { gap };
    if column == "0" { "0px".to_owned() } else { column }
}

/// `block_core_gallery_resolve_dynamic_source()`.
async fn resolve_dynamic_source(bc: &BlockCtx<'_>, source: &Value) -> Result<Vec<u64>> {
    if source.get("source").and_then(Value::as_str) != Some("core/attached-media") {
        return Ok(Vec::new());
    }
    let post_id = match bc.context.post_id() {
        Some(id) => Some(id),
        None => super::global_post(bc).await?.map(|p| p.id).or_else(|| bc.env.query.posts.first().map(|p| p.id)),
    };
    let Some(post_id) = post_id.filter(|id| *id != 0) else { return Ok(Vec::new()) };
    let args = source.get("args").and_then(Value::as_object);
    let order_by = match args.and_then(|a| a.get("orderBy")).and_then(Value::as_str) {
        Some("title") => OrderBy::Title,
        _ => OrderBy::Date,
    };
    let ascending = args.and_then(|a| a.get("order")).and_then(Value::as_str).is_some_and(|o| o.eq_ignore_ascii_case("asc"));
    let query = PostQuery {
        parent: Some(post_id),
        post_types: vec!["attachment".into()],
        statuses: vec!["inherit".into()],
        mime_prefix: Some("image".into()),
        order_by,
        order: if ascending { Order::Asc } else { Order::Desc },
        limit: Some(100),
        ..PostQuery::default()
    };
    Ok(bc.env.site.db.posts(&query).await?.into_iter().map(|p| p.id).collect())
}

/// `block_core_gallery_render_dynamic_image()` with `block_core_gallery_dynamic_image_link_attributes()`.
async fn render_dynamic_image(bc: &BlockCtx<'_>, id: u64, context: &Context) -> Result<String> {
    let env = bc.env;
    let size = bc.attr_str("sizeSlug").unwrap_or("large").to_owned();
    let ratio = bc.attr_str("aspectRatio").unwrap_or("auto").to_owned();
    let has_ratio = !ratio.is_empty() && ratio != "0" && ratio != "auto";

    let mut img_attrs = vec![("class".to_owned(), AttrValue::Text(format!("wp-image-{id}")))];
    if has_ratio {
        img_attrs.push(("style".to_owned(), AttrValue::Text(safecss_filter_attr(&format!("aspect-ratio:{ratio};object-fit:cover;")))));
    }
    let mut markup = images::get_attachment_image(env, id, &size, &ImageArgs { attrs: img_attrs, post_thumbnail: false, thumbnail_of: None, dimensions: None }).await?;
    if markup.is_empty() {
        return Ok(String::new());
    }
    let Some(attachment) = env.site.db.post(id).await? else { return Ok(String::new()) };

    let mut attrs = Map::new();
    attrs.insert("id".into(), Value::from(id));
    attrs.insert("data-id".into(), Value::String(id.to_string()));
    attrs.insert("sizeSlug".into(), Value::String(size.clone()));

    let link_to = bc.attr_str("linkTo").unwrap_or("none");
    let mut href = None;
    match link_to {
        // Gutenberg uses 'media'/'attachment'; WP Core uses 'file'/'post'.
        "media" | "file" => {
            let (_, attached) = images::load_metadata(env.site, &attachment).await?;
            href = Some(attached.map_or_else(|| attachment.guid.clone(), |file| images::attachment_url(env.site, &file)));
            attrs.insert("linkDestination".into(), Value::from("media"));
        }
        "attachment" | "post" => {
            href = Some(env.site.permalink(&attachment).await?);
            attrs.insert("linkDestination".into(), Value::from("attachment"));
        }
        "lightbox" => {
            attrs.insert("linkDestination".into(), Value::from("none"));
            attrs.insert("lightbox".into(), json!({ "enabled": true }));
        }
        _ => {}
    }
    let opens_blank = href.as_deref().is_some_and(|h| !h.is_empty() && h != "0") && bc.attr_str("linkTarget") == Some("_blank");
    if let Some(href) = &href {
        attrs.insert("href".into(), Value::String(href.clone()));
    }
    if opens_blank {
        attrs.insert("linkTarget".into(), Value::from("_blank"));
        attrs.insert("rel".into(), Value::from("noopener"));
    }
    if has_ratio {
        attrs.insert("aspectRatio".into(), Value::String(ratio));
        attrs.insert("scale".into(), Value::from("cover"));
    }

    // Wrap in a link when the gallery links images somewhere.
    if let Some(href) = href.filter(|h| !h.is_empty() && h != "0") {
        let (target, rel) = if opens_blank { (" target=\"_blank\"", " rel=\"noopener\"") } else { ("", "") };
        markup = format!("<a href=\"{}\"{target}{rel}>{markup}</a>", esc_url(&href));
    }
    if !attachment.post_excerpt.is_empty() {
        markup.push_str(&format!("<figcaption class=\"wp-element-caption\">{}</figcaption>", rustpress_core::kses::kses_post(&attachment.post_excerpt)));
    }
    let figure = format!("<figure class=\"wp-block-image size-{}\">{markup}</figure>", esc_attr(&size));
    let block = Block { name: Some("core/image".into()), attrs, inner_blocks: Vec::new(), inner_html: figure.clone(), inner_content: vec![Content::Html(figure)] };
    env.render_block(&block, context, None).await
}

/// The markup of a gallery whose images are resolved at render time: `None` when there are none.
async fn dynamic_markup(bc: &BlockCtx<'_>, source: &Value) -> Result<Option<String>> {
    let ids = resolve_dynamic_source(bc, source).await?;
    if ids.is_empty() {
        return Ok(None);
    }
    let mut context = bc.context.clone();
    context.values.insert("allowResize".into(), bc.attrs.get("allowResize").cloned().unwrap_or(Value::Bool(false)));
    context.values.insert("imageCrop".into(), bc.attrs.get("imageCrop").cloned().unwrap_or(Value::Bool(true)));
    context.values.insert("fixedHeight".into(), bc.attrs.get("fixedHeight").cloned().unwrap_or(Value::Bool(true)));
    context.values.insert("navigationButtonType".into(), bc.attrs.get("navigationButtonType").cloned().unwrap_or_else(|| Value::from("icon")));

    let mut images_markup = String::new();
    for id in ids {
        images_markup.push_str(&render_dynamic_image(bc, id, &context).await?);
    }
    let mut classes = "wp-block-gallery has-nested-images".to_owned();
    match bc.attrs.get("columns").filter(|v| !v.is_null()) {
        Some(columns) => classes.push_str(&format!(" columns-{}", images::php_int(columns))),
        None => classes.push_str(" columns-default"),
    }
    if bc.attrs.get("imageCrop").is_none_or(|crop| !is_empty(crop)) {
        classes.push_str(" is-cropped");
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes)]);
    Ok(Some(format!("<figure {wrapper}>{images_markup}{}</figure>", bc.content)))
}

/// The unique class and the `--wp--style--unstable-gallery-gap` variable the widths of the images are calculated from.
fn gap_styles(bc: &BlockCtx<'_>, unique: &str) {
    let empty = Value::Null;
    let style = bc.attrs.get("style").filter(|s| s.is_object()).unwrap_or(&empty);
    let globals = bc.env.theme.global_styles().get("blocks").and_then(|b| b.get("core/gallery")).unwrap_or(&empty);
    let spacing_gap = |of: &Value| -> Option<Option<Value>> {
        let spacing = of.get("spacing")?.as_object()?;
        spacing.contains_key("blockGap").then(|| spacing.get("blockGap").filter(|v| !v.is_null()).cloned())
    };
    let own_gap = spacing_gap(style);
    let global_gap = globals.get("spacing").and_then(|s| s.get("blockGap")).filter(|v| !v.is_null()).cloned();
    let gap = gallery_column_gap(match &own_gap {
        Some(own) => own.as_ref(),
        None => global_gap.as_ref(),
    });
    let selector = format!(".wp-block-gallery.{unique}");
    let property = "--wp--style--unstable-gallery-gap".to_owned();
    let mut rules = vec![(gap, None)];
    for (breakpoint, query) in crate::states::viewport_media_queries(bc.env.theme.global_settings().get("viewport"), false) {
        let viewport_gap = match (style.get(breakpoint).and_then(spacing_gap), globals.get(breakpoint).and_then(spacing_gap)) {
            (Some(gap), _) => gap,
            (None, Some(gap)) if own_gap.is_none() => gap,
            _ => continue,
        };
        if let Some(gap) = viewport_gap {
            rules.push((gallery_column_gap(Some(&gap)), Some(query)));
        }
    }
    bc.env.with_state(|s| {
        for (gap, query) in rules {
            s.block_supports_css.add(&selector, &[(property.clone(), gap)], query.as_deref());
        }
    });
}

/// `shuffle()` of the image blocks of a gallery with `randomOrder`.
fn shuffle_images(content: String) -> String {
    let pattern = Regex::new(r"(?s)<figure[^>]*\bwp-block-image\b[^>]*>.*?</figure>").expect("the pattern is valid");
    let mut figures: Vec<String> = pattern.find_iter(&content).map(|m| m.as_str().to_owned()).collect();
    figures.shuffle(&mut rand::rng());
    let mut figures = figures.into_iter();
    pattern.replace_all(&content, |_: &regex::Captures<'_>| figures.next().unwrap_or_default()).into_owned()
}

/// The lightbox of a gallery: the directives of the wrapper, the position and the labels of its images.
fn lightbox_navigation(bc: &BlockCtx<'_>, content: String) -> String {
    let Some(gallery_id) = bc.context.get("galleryId").filter(|id| !id.is_null()).cloned() else { return content };
    let metadata = bc.env.with_state(|s| s.interactivity.namespace_state("core/image").and_then(|state| state.get("metadata").cloned()));
    let Some(Value::Object(metadata)) = metadata else { return content };
    let total = metadata.values().filter(|image| image.get("galleryId") == Some(&gallery_id)).count();
    if total == 0 {
        return content;
    }
    let Some(mut editor) = TagEditor::first(&content) else { return content };
    editor.set_attribute("data-wp-interactive", "core/gallery");
    editor.set_attribute("data-wp-context", &crate::php_json::encode(&json!({ "galleryId": gallery_id })));
    let html = editor.finish();

    let site = bc.env.site;
    let first_end = tags(&html, 0).first().map_or(0, |t| t.end);
    let mut index = 0;
    for figure in tags(&html, first_end).into_iter().filter(|t| !t.is_closer && t.name == "figure") {
        let Some(key) = figure.attr("data-wp-key").filter(|k| !k.is_empty() && *k != "0") else { continue };
        let Some(image) = metadata.get(key) else { continue };
        let position = (index + 1).to_string();
        let count = total.to_string();
        let label = match image.get("alt").filter(|alt| !is_empty(alt)) {
            Some(alt) => {
                let alt = alt.as_str().map_or_else(|| "1".to_owned(), str::to_owned);
                rustpress_core::i18n::sprintf(&site.tr("Enlarged image %1$s of %2$s: %3$s"), &[&position, &count, &alt])
            }
            None => rustpress_core::i18n::sprintf(&site.tr("Enlarged image %1$s of %2$s"), &[&position, &count]),
        };
        let mut update = Map::new();
        update.insert(key.to_owned(), json!({
            "customAriaLabel": label,
            "triggerButtonAriaLabel": rustpress_core::i18n::sprintf(&site.tr("Enlarge %1$s of %2$s"), &[&position, &count]),
            "order": index,
        }));
        bc.env.with_state(|s| s.interactivity.state("core/image", json!({ "metadata": update })));
        index += 1;
    }
    html
}

/// `block_core_gallery_render()`.
pub(super) async fn gallery(bc: &BlockCtx<'_>) -> Result<String> {
    let mut content = bc.content.clone();
    if let Some(source) = bc.attrs.get("dynamicContent").filter(|v| !is_empty(v)) {
        match dynamic_markup(bc, source).await? {
            Some(markup) => content = markup,
            None => return Ok(String::new()),
        }
    }
    let unique = bc.env.with_state(|s| s.unique_id("wp-block-gallery-"));
    if let Some(mut editor) = TagEditor::first(&content) {
        editor.add_class(&unique);
        content = editor.finish();
    }
    gap_styles(bc, &unique);
    if bc.attrs.get("randomOrder").is_some_and(|random| !is_empty(random)) {
        content = shuffle_images(content);
    }
    Ok(lightbox_navigation(bc, content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_gaps_become_css_values() {
        assert_eq!(gallery_column_gap(None), GALLERY_FALLBACK_GAP);
        assert_eq!(gallery_column_gap(Some(&json!("0"))), "0px");
        assert_eq!(gallery_column_gap(Some(&json!("2em"))), "2em");
        assert_eq!(gallery_column_gap(Some(&json!("var:preset|spacing|50"))), "var(--wp--preset--spacing--50)");
        assert_eq!(gallery_column_gap(Some(&json!({"top": "1em", "left": "3px"}))), "3px");
        // Anything with characters that could break out of the declaration is replaced.
        assert_eq!(gallery_column_gap(Some(&json!("1em;color:red}"))), GALLERY_FALLBACK_GAP);
        assert_eq!(gallery_column_gap(Some(&json!("calc(1em)"))), GALLERY_FALLBACK_GAP);
    }

    #[test]
    fn shuffling_keeps_the_images_and_what_surrounds_them() {
        let html = "<figure class=\"wp-block-gallery\"><figure class=\"wp-block-image a\">1</figure><figure class=\"wp-block-image\">2</figure><figcaption>c</figcaption></figure>";
        let shuffled = shuffle_images(html.to_owned());
        assert_eq!(shuffled.len(), html.len());
        assert!(shuffled.contains(">1</figure>") && shuffled.contains(">2</figure>") && shuffled.ends_with("<figcaption>c</figcaption></figure>"));
    }
}
