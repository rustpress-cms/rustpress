//! Image blocks: `core/image`, `core/post-featured-image` and `core/site-logo`.

use rustpress_core::formatting::{esc_attr, esc_url, strip_tags};
use serde_json::{Value, json};

use crate::Result;
use crate::content::the_title;
use crate::html::{TagEditor, tags};
use crate::images::{self, AttrValue, ImageArgs};
use crate::render::BlockCtx;
use crate::supports::safecss_filter_attr;

use super::global_post;

// ------------------------------------------------------------------ core/image

/// The span of an empty `<figcaption>` after the image (`block_core_image_extract_empty_figcaption_element()`).
fn empty_figcaption_span(content: &str, after: usize) -> Option<(usize, usize)> {
    let all = tags(content, after);
    let opener = all.iter().find(|t| !t.is_closer && t.name == "figcaption")?;
    // Only comments may sit between the opener and its closer.
    let mut at = opener.end;
    loop {
        let rest = &content[at..];
        if rest.starts_with("<!--") {
            at += rest.find("-->")? + 3;
        } else {
            let stripped = rest.strip_prefix("</")?;
            let name_len = stripped.find('>')?;
            return stripped[..name_len].trim().eq_ignore_ascii_case("figcaption").then_some((opener.start, at + 2 + name_len + 1));
        }
    }
}

/// `render_block_core_image()`: the `data-id` the gallery adds, whether the lightbox follows and the removal of an
/// empty caption.
pub(super) fn image(bc: &BlockCtx<'_>) -> Result<String> {
    let content = &bc.content;
    if !content.to_ascii_lowercase().contains("<img") {
        return Ok(String::new());
    }
    let Some(img) = tags(content, 0).into_iter().find(|t| !t.is_closer && t.name == "img") else { return Ok(String::new()) };
    if img.attr("src").is_none_or(str::is_empty) {
        return Ok(String::new());
    }
    let img_end = img.end;
    let mut editor = TagEditor::from_tag(content, img);
    if let Some(data_id) = bc.attrs.get("data-id") {
        let text = match data_id {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        editor.set_attribute("data-id", &text);
    }
    // `block_core_image_render_lightbox()` runs on `render_block_core/image` while this was the last image to ask.
    let lightbox = bc.attr_str("linkDestination").unwrap_or("none") == "none" && super::lightbox::is_enabled(bc.env, &bc.raw_attrs);
    bc.env.with_state(|s| {
        s.image_lightbox = lightbox;
        if lightbox {
            s.script_modules.insert("@wordpress/block-library/image/view".to_owned());
        }
    });
    let caption_empty = bc.attrs.get("caption").is_none_or(|c| c.is_null() || c.as_str() == Some(""));
    let span = if caption_empty { empty_figcaption_span(content, img_end) } else { None };
    let output = editor.finish();
    // The span is measured on the original markup but cut from the updated one, like WordPress does.
    Ok(match span {
        Some((start, end)) if end <= output.len() && output.is_char_boundary(start) && output.is_char_boundary(end) => format!("{}{}", &output[..start], &output[end..]),
        _ => output,
    })
}

// ------------------------------------------------------- core/post-featured-image

fn text_attr<'a>(bc: &'a BlockCtx<'_>, key: &str) -> Option<&'a str> {
    bc.attrs.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// `get_block_core_post_featured_image_border_attributes()`: `(class, style)`.
fn border_attributes(bc: &BlockCtx<'_>) -> (String, String) {
    let border = bc.attrs.get("style").and_then(|s| s.get("border"));
    let mut styles = serde_json::Map::new();
    for key in ["radius", "style", "width"] {
        if let Some(value) = border.and_then(|b| b.get(key)).filter(|v| !v.is_null()) {
            styles.insert(key.into(), value.clone());
        }
    }
    let preset = bc.attrs.get("borderColor").and_then(Value::as_str).filter(|c| !c.is_empty()).map(|c| format!("var:preset|color|{c}"));
    let color = preset.map(Value::from).or_else(|| border.and_then(|b| b.get("color")).filter(|v| !v.is_null()).cloned());
    if let Some(color) = color {
        styles.insert("color".into(), color);
    }
    for side in ["top", "right", "bottom", "left"] {
        let mut parts = serde_json::Map::new();
        for key in ["color", "style", "width"] {
            if let Some(value) = border.and_then(|b| b.get(side)).and_then(|s| s.get(key)).filter(|v| !v.is_null()) {
                parts.insert(key.into(), value.clone());
            }
        }
        if !parts.is_empty() {
            styles.insert(side.into(), Value::Object(parts));
        }
    }
    let result = crate::style_engine::get_styles(&json!({ "border": Value::Object(styles) }), false);
    (result.classnames, result.css)
}

/// `get_block_core_post_featured_image_overlay_element_markup()`.
fn overlay_markup(bc: &BlockCtx<'_>) -> String {
    let truthy = |key: &str| bc.attrs.get(key).is_some_and(|v| !matches!(v, Value::Null | Value::Bool(false)) && v.as_str() != Some("") && v.as_u64() != Some(0) && v.as_f64() != Some(0.0));
    if !truthy("dimRatio") {
        return String::new();
    }
    let display = |key: &str| match bc.attrs.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    };
    let mut classes = vec!["wp-block-post-featured-image__overlay".to_owned()];
    let mut styles: Vec<String> = Vec::new();
    let (border_class, border_style) = border_attributes(bc);
    if !border_class.is_empty() {
        classes.push(border_class);
    }
    if !border_style.is_empty() {
        styles.push(border_style);
    }
    classes.push("has-background-dim".into());
    classes.push(format!("has-background-dim-{}", display("dimRatio")));
    if truthy("overlayColor") {
        classes.push(format!("has-{}-background-color", display("overlayColor")));
    }
    if truthy("gradient") || truthy("customGradient") {
        classes.push("has-background-gradient".into());
    }
    if truthy("gradient") {
        classes.push(format!("has-{}-gradient-background", display("gradient")));
    }
    if truthy("customGradient") {
        styles.push(format!("background-image: {};", display("customGradient")));
    }
    if truthy("customOverlayColor") {
        styles.push(format!("background-color: {};", display("customOverlayColor")));
    }
    format!("<span class=\"{}\" style=\"{}\" aria-hidden=\"true\"></span>", esc_attr(&classes.join(" ")), esc_attr(&safecss_filter_attr(&styles.join(" "))))
}

/// The first image of a post for `useFirstImageFromPost`: its attributes copied onto a new tag, with `style` added.
fn first_image(content: &str, style: &str) -> String {
    let Some(img) = tags(content, 0).into_iter().find(|t| !t.is_closer && t.name == "img") else { return String::new() };
    let mut editor = TagEditor::first("<img>").expect("the tag is complete");
    let mut seen: Vec<String> = Vec::new();
    for attr in &img.attrs {
        let name = attr.name.to_ascii_lowercase();
        if seen.contains(&name) {
            continue;
        }
        match &attr.value {
            Some(value) => editor.set_attribute(&name, value),
            None => editor.set_boolean_attribute(&name),
        }
        seen.push(name);
    }
    if !style.is_empty() {
        let merged = match editor.get_attribute("style").filter(|existing| !existing.is_empty()) {
            Some(existing) => format!("{};{style}", existing.trim_end_matches(';')),
            None => style.to_owned(),
        };
        editor.set_attribute("style", &merged);
    }
    editor.finish()
}

pub(super) async fn post_featured_image(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id() else { return Ok(String::new()) };
    let is_link = bc.attr_bool("isLink");
    let size = text_attr(bc, "sizeSlug").unwrap_or("post-thumbnail").to_owned();
    let (border_class, border_style) = border_attributes(bc);
    let overlay = overlay_markup(bc);
    let post = global_post(bc).await?;

    let mut attrs: Vec<(String, AttrValue)> = Vec::new();
    if !border_class.is_empty() {
        attrs.push(("class".into(), AttrValue::Text(border_class)));
    }
    let mut style = border_style;
    if is_link {
        let title = post.as_deref().map(the_title).unwrap_or_default();
        let alt = if title.is_empty() { format!("Untitled post {post_id}") } else { strip_tags(&title).trim().to_owned() };
        attrs.push(("alt".into(), AttrValue::Text(alt)));
    }

    let filtered = |css: String| format!("{};", esc_attr(&safecss_filter_attr(&css)));
    let has = |key: &str| bc.attrs.get(key).is_some_and(|v| !v.is_null() && v.as_str() != Some(""));
    let value = |key: &str| bc.attrs.get(key).map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned)).unwrap_or_default();
    let mut extra = String::new();
    if let Some(ratio) = text_attr(bc, "aspectRatio") {
        if ratio != "auto" {
            extra.push_str(&filtered(format!("aspect-ratio:{ratio}")));
        }
        extra.push_str("width:100%;");
    }
    if has("height") {
        extra.push_str(&filtered(format!("height:{}", value("height"))));
    } else if has("width") {
        extra.push_str("height:auto;");
    }
    if has("width") {
        extra.push_str(&filtered(format!("width:{}", value("width"))));
    }
    if let Some(scale) = text_attr(bc, "scale") {
        extra.push_str(&filtered(format!("object-fit:{scale}")));
    }
    if let Some(shadow) = bc.attrs.get("style").and_then(|s| s.get("shadow")).filter(|s| !s.is_null() && s.as_str() != Some("")) {
        let shadow_styles = crate::style_engine::get_styles(&json!({ "shadow": shadow }), false);
        extra.push_str(&shadow_styles.css);
    }
    if !extra.is_empty() {
        style.push_str(&extra);
    }
    if !style.is_empty() {
        attrs.push(("style".into(), AttrValue::Text(style.clone())));
    }

    let thumbnail = images::post_thumbnail_id(bc.env, post_id).await?;
    let mut featured = if thumbnail != 0 {
        images::get_attachment_image(bc.env, thumbnail, &size, &ImageArgs { attrs, post_thumbnail: true, thumbnail_of: Some(post_id), dimensions: None, requested: None }).await?
    } else {
        String::new()
    };
    if featured.is_empty() && bc.attr_bool("useFirstImageFromPost") {
        featured = post.as_deref().map(|post| first_image(&post.post_content, &style)).unwrap_or_default();
    }
    if featured.is_empty() {
        return Ok(String::new());
    }

    let featured = if is_link {
        let rel = text_attr(bc, "rel").map(|r| format!("rel=\"{}\"", esc_attr(r))).unwrap_or_default();
        let permalink = match &post {
            Some(post) => bc.env.site.permalink(post).await?,
            None => String::new(),
        };
        format!("<a href=\"{}\" target=\"{}\" {rel}>{featured}{overlay}</a>", esc_url(&permalink), esc_attr(bc.attr_str("linkTarget").unwrap_or("_self")))
    } else {
        format!("{featured}{overlay}")
    };
    let wrapper = bc.wrapper_attributes(&[]);
    Ok(format!("<figure {wrapper}>{featured}</figure>"))
}

// ----------------------------------------------------------------------- core/file

/// `render_block_core_file()`: the PDF preview of a file block is interactive.
pub(super) fn file(bc: &BlockCtx<'_>) -> Result<String> {
    let content = bc.content.clone();
    if !bc.attr_bool("displayPreview") {
        return Ok(content);
    }
    bc.env.with_state(|s| s.script_modules.insert("@wordpress/block-library/file/view".to_owned()));
    let Some(mut editor) = TagEditor::first(&content) else { return Ok(content) };
    editor.set_attribute("data-wp-interactive", "core/file");
    let html = editor.finish();
    let Some(object) = tags(&html, 0).into_iter().find(|t| !t.is_closer && t.name == "object") else { return Ok(html) };
    let mut editor = TagEditor::from_tag(&html, object);
    editor.set_attribute("data-wp-bind--hidden", "!state.hasPdfPreview");
    editor.set_boolean_attribute("hidden");
    let filename = editor.get_attribute("aria-label").map(rustpress_core::formatting::html_entity_decode).filter(|f| !f.is_empty() && f != "PDF embed");
    let label = match filename {
        Some(name) => rustpress_core::i18n::sprintf(&bc.env.site.tr("Embed of %s."), &[&name]),
        None => bc.env.site.tr("PDF embed"),
    };
    editor.set_attribute("aria-label", &label);
    Ok(editor.finish())
}

// ------------------------------------------------------------------ core/site-logo

/// `get_custom_logo()`.
async fn custom_logo(bc: &BlockCtx<'_>, width: Option<u64>) -> Result<String> {
    let id = images::custom_logo_id(bc.env.site).await?;
    if id == 0 {
        return Ok(String::new());
    }
    let env = bc.env;
    let site = env.site;
    let rows = site.db.post_meta(id).await?;
    let alt_meta = rows.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value.clone()).unwrap_or_default();

    let mut attrs: Vec<(String, AttrValue)> = vec![("class".into(), AttrValue::Text("custom-logo".into())), ("loading".into(), AttrValue::Omit)];
    if alt_meta.is_empty() || alt_meta == "0" {
        attrs.push(("alt".into(), AttrValue::Text(site.settings.blogname.clone())));
    }

    // The block filters `wp_get_attachment_image_src` to scale the logo to its width.
    let mut dimensions = None;
    if let Some(width) = width.filter(|w| *w > 0)
        && let Some(post) = site.db.post(id).await?
    {
        let (meta, _) = images::load_metadata(site, &post).await?;
        if let Some(meta) = meta.filter(|m| m.width() > 0 && m.height() > 0) {
            let height = width as f64 / (meta.width() as f64 / meta.height() as f64);
            dimensions = Some((width, height as u64));
        }
    }
    let image = images::get_attachment_image(env, id, "full", &ImageArgs { attrs, post_thumbnail: false, thumbnail_of: None, dimensions, requested: None }).await?;
    if image.is_empty() {
        return Ok(String::new());
    }
    let q = env.query;
    let settings = &site.settings;
    let current = !q.flags.is_paged && (q.flags.is_front_page || (q.flags.is_home && settings.page_for_posts != q.queried_object_id()));
    let aria = if current { " aria-current=\"page\"" } else { "" };
    Ok(format!("<a href=\"{}\" class=\"custom-logo-link\" rel=\"home\"{aria}>{image}</a>", esc_url(&site.home_url("/"))))
}

/// `#<a.*?>(.*?)</a>#i` replaced by `\1`.
fn unlink(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let Some(start) = lower.find("<a") else { return html.to_owned() };
    let Some(open_end) = lower[start..].find('>').map(|i| start + i + 1) else { return html.to_owned() };
    let Some(close) = lower[open_end..].find("</a>").map(|i| open_end + i) else { return html.to_owned() };
    format!("{}{}{}", &html[..start], &html[open_end..close], &html[close + 4..])
}

pub(super) async fn site_logo(bc: &BlockCtx<'_>) -> Result<String> {
    let width = bc.attrs.get("width").and_then(|w| w.as_u64().or_else(|| w.as_f64().map(|f| f as u64)).or_else(|| w.as_str().and_then(|s| s.trim().parse().ok())));
    let mut logo = custom_logo(bc, width).await?;
    if logo.is_empty() {
        return Ok(String::new());
    }
    let is_link = bc.attr_bool("isLink");
    let target = bc.attr_str("linkTarget").unwrap_or("_self");
    if !is_link {
        logo = unlink(&logo);
    } else if target == "_blank"
        && let Some(anchor) = tags(&logo, 0).into_iter().find(|t| !t.is_closer && t.name == "a")
        && anchor.attr("rel") == Some("home")
    {
        let mut editor = TagEditor::from_tag(&logo, anchor);
        editor.set_attribute("aria-label", &bc.env.site.tr("(Home link, opens in a new tab)"));
        editor.set_attribute("target", target);
        logo = editor.finish();
    }
    let class = if width.is_none_or(|w| w == 0) { "is-default-size".to_owned() } else { String::new() };
    let wrapper = bc.wrapper_attributes(&[("class", class)]);
    Ok(format!("<div {wrapper}>{logo}</div>"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_figcaption_is_found_only_when_nothing_is_inside() {
        let html = "<figure><img src=\"a\"/><figcaption class=\"wp-element-caption\"></figcaption></figure>";
        let (start, end) = empty_figcaption_span(html, 0).unwrap();
        assert_eq!(&html[start..end], "<figcaption class=\"wp-element-caption\"></figcaption>");
        assert!(empty_figcaption_span("<figure><figcaption>Text</figcaption></figure>", 0).is_none());
        // Comments do not count as content.
        let commented = "<figure><figcaption><!-- x --></figcaption></figure>";
        let (start, end) = empty_figcaption_span(commented, 0).unwrap();
        assert_eq!(&commented[start..end], "<figcaption><!-- x --></figcaption>");
    }

    #[test]
    fn the_logo_link_is_removed_but_its_content_stays() {
        assert_eq!(unlink("<a href=\"/\" class=\"x\"><img src=\"l\" /></a>"), "<img src=\"l\" />");
        assert_eq!(unlink("<img src=\"l\" />"), "<img src=\"l\" />");
    }
}
