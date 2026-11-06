//! The lightbox of `core/image`: `block_core_image_get_lightbox_settings()`, `block_core_image_render_lightbox()` and
//! `block_core_image_print_lightbox_overlay()`.

use rustpress_core::formatting::{esc_attr, esc_html};
use serde_json::{Map, Value, json};

use crate::Result;
use crate::env::{Context, Env};
use crate::html::{Tag, TagEditor, tags};
use crate::images;
use crate::parser::Block;

/// `block_core_image_get_lightbox_settings()` as far as it decides: `enabled` is exactly `true`.
pub(super) fn is_enabled(env: &Env<'_>, block_attrs: &Map<String, Value>) -> bool {
    let global = env.theme.global_settings();
    let settings = block_attrs
        .get("lightbox")
        .filter(|v| !v.is_null())
        .or_else(|| global.get("blocks").and_then(|b| b.get("core/image")).and_then(|i| i.get("lightbox")))
        .or_else(|| global.get("lightbox"));
    settings.and_then(|s| s.get("enabled")) == Some(&Value::Bool(true))
}

/// `WP_HTML_Tag_Processor::get_attribute()`: `null` when missing, `true` for an attribute without a value.
fn attribute(tag: &Tag, name: &str) -> Value {
    match tag.attrs.iter().find(|a| a.name.eq_ignore_ascii_case(name)) {
        None => Value::Null,
        Some(attr) => attr.value.clone().map_or(Value::Bool(true), Value::String),
    }
}

fn opening(html: &str, from: usize, name: &str) -> Option<Tag> {
    tags(html, from).into_iter().find(|t| !t.is_closer && t.name == name)
}

/// `wp_get_attachment_url()`.
async fn attachment_url(env: &Env<'_>, id: u64) -> Result<Option<(String, Option<images::Metadata>)>> {
    let Some(post) = env.site.db.post(id).await? else { return Ok(None) };
    if post.post_type != "attachment" {
        return Ok(None);
    }
    let (meta, attached) = images::load_metadata(env.site, &post).await?;
    let url = attached.map_or_else(|| post.guid.clone(), |file| images::attachment_url(env.site, &file));
    Ok(Some((url, meta)))
}

/// `wp_get_attachment_image_srcset( $id, $size )` for the size the lightbox asks for: the full image when its
/// dimensions are known, else `large`.
fn lightbox_srcset(env: &Env<'_>, full_url: &str, meta: &images::Metadata) -> Option<String> {
    let large = images::image_downsize(full_url, meta, "large");
    let (src, width, height) = if meta.width() != 0 && meta.height() != 0 { (full_url.to_owned(), meta.width(), meta.height()) } else { (large.url, large.width, large.height) };
    images::srcset(env.site, width, height, &src, meta)
}

/// `block_core_image_render_lightbox()`, the `render_block_core/image` filter. The markup of an image block always
/// has a `figure`; one without stays as it is.
pub(crate) async fn render(env: &Env<'_>, block: &Block, context: &Context, content: String) -> Result<String> {
    let Some(figure) = opening(&content, 0, "figure") else { return Ok(content) };
    let Some(img) = opening(&content, figure.end, "img") else { return Ok(content) };
    let site = env.site;

    let alt = attribute(&img, "alt");
    let mut uploaded_src = attribute(&img, "src");
    let (mut srcset, mut width, mut height) = (Value::Bool(false), Value::from("none"), Value::from("none"));
    env.with_state(|s| {
        s.interactivity.config(
            "core/image",
            json!({
                "defaultAriaLabel": site.tr("Enlarged image"),
                "closeButtonText": esc_html(&site.tr("Close")),
                "prevButtonText": esc_html(&site.trx("Previous", "previous image in lightbox")),
                "nextButtonText": esc_html(&site.trx("Next", "next image in lightbox")),
            }),
        );
    });
    let aria_label = match alt.as_str().filter(|alt| !alt.is_empty() && *alt != "0") {
        Some(alt) => Value::String(rustpress_core::i18n::sprintf(&site.tr("Enlarged image: %s"), &[alt])),
        None if alt == Value::Bool(true) => Value::String(rustpress_core::i18n::sprintf(&site.tr("Enlarged image: %s"), &["1"])),
        None => Value::Null,
    };

    if let Some(id) = block.attrs.get("id").filter(|v| !v.is_null()) {
        match attachment_url(env, images::php_int(id)).await? {
            Some((url, meta)) => {
                if let Some(meta) = &meta {
                    srcset = lightbox_srcset(env, &url, meta).map_or(Value::Bool(false), Value::String);
                    width = meta.0.get("width").filter(|v| !v.is_null()).cloned().unwrap_or(width);
                    height = meta.0.get("height").filter(|v| !v.is_null()).cloned().unwrap_or(height);
                }
                uploaded_src = Value::String(url);
            }
            None => uploaded_src = Value::Bool(false),
        }
    }

    let image_id = env.with_state(|s| s.uniqid());
    let mut metadata = Map::new();
    metadata.insert(
        image_id.clone(),
        json!({
            "uploadedSrc": uploaded_src,
            "lightboxSrcset": srcset,
            "figureClassNames": attribute(&figure, "class"),
            "figureStyles": attribute(&figure, "style"),
            "imgClassNames": attribute(&img, "class"),
            "imgStyles": attribute(&img, "style"),
            "targetWidth": width,
            "targetHeight": height,
            "scaleAttr": block.attrs.get("scale").cloned().filter(|v| !v.is_null()).unwrap_or(Value::Bool(false)),
            "alt": alt,
            "galleryId": context.get("galleryId").cloned().unwrap_or(Value::Null),
            "customAriaLabel": aria_label,
            "navigationButtonType": context.get("navigationButtonType").filter(|v| !v.is_null()).cloned().unwrap_or_else(|| Value::from("icon")),
            "triggerButtonAriaLabel": site.tr("Enlarge"),
        }),
    );
    env.with_state(|s| s.interactivity.state("core/image", json!({ "metadata": metadata })));

    let mut editor = TagEditor::from_tag(&content, figure);
    editor.add_class("wp-lightbox-container");
    editor.set_attribute("data-wp-interactive", "core/image");
    editor.set_attribute("data-wp-context", &crate::php_json::encode(&json!({ "imageId": image_id })));
    editor.set_attribute("data-wp-key", &image_id);
    let html = editor.finish();

    let Some(figure) = opening(&html, 0, "figure") else { return Ok(html) };
    let Some(img) = opening(&html, figure.end, "img") else { return Ok(html) };
    let mut editor = TagEditor::from_tag(&html, img);
    for (name, value) in [
        ("data-wp-init", "callbacks.setButtonStyles"),
        ("data-wp-on--load", "callbacks.setButtonStyles"),
        ("data-wp-on-window--resize", "callbacks.setButtonStyles"),
        ("data-wp-on--pointerenter", "actions.preloadImageWithDelay"),
        ("data-wp-on--pointerdown", "actions.preloadImage"),
        ("data-wp-on--pointerleave", "actions.cancelPreload"),
        ("data-wp-on--click", "actions.showLightbox"),
        ("data-wp-class--hide", "state.isContentHidden"),
        ("data-wp-class--show", "state.isContentVisible"),
    ] {
        editor.set_attribute(name, value);
    }
    let mut html = editor.finish();

    if let Some(img) = opening(&html, 0, "img") {
        html.insert_str(img.end, include_str!("lightbox_button.html"));
    }
    env.with_state(|s| s.lightbox_overlay = true);
    Ok(html)
}

/// `block_core_image_print_lightbox_overlay()` on `wp_footer`.
pub fn overlay(site: &rustpress_core::Site, theme: &dyn crate::env::ThemeAccess) -> String {
    let (mut background, mut close_color) = ("#fff".to_owned(), "#000".to_owned());
    // `wp_theme_has_theme_json()`.
    if theme.is_block_theme() {
        let color = theme.global_styles().get("color");
        let value = |key: &str| color.and_then(|c| c.get(key)).and_then(Value::as_str).filter(|v| !v.is_empty() && *v != "0").map(esc_attr);
        background = value("background").unwrap_or(background);
        close_color = value("text").unwrap_or(close_color);
    }
    let close_icon = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"20\" height=\"20\" aria-hidden=\"true\" focusable=\"false\"><path d=\"m13.06 12 6.47-6.47-1.06-1.06L12 10.94 5.53 4.47 4.47 5.53 10.94 12l-6.47 6.47 1.06 1.06L12 13.06l6.47 6.47 1.06-1.06L13.06 12Z\"></path></svg>";
    let prev_icon = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"28\" height=\"28\" aria-hidden=\"true\" focusable=\"false\"><path d=\"M14.6 7l-1.2-1L8 12l5.4 6 1.2-1-4.6-5z\"></path></svg>";
    let next_icon = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"28\" height=\"28\" aria-hidden=\"true\" focusable=\"false\"><path d=\"M10.6 6L9.4 7l4.6 5-4.6 5 1.2 1 5.4-6z\"></path></svg>";
    include_str!("lightbox_overlay.html")
        .replace("{$dialog_label}", &esc_attr(&site.tr("Enlarged images")))
        .replace("{$close_button_text}", &esc_attr(&site.tr("Close")))
        .replace("{$prev_button_text}", &esc_attr(&site.trx("Previous", "previous image in lightbox")))
        .replace("{$next_button_text}", &esc_attr(&site.trx("Next", "next image in lightbox")))
        .replace("{$close_button_icon}", close_icon)
        .replace("{$prev_button_icon}", prev_icon)
        .replace("{$next_button_icon}", next_icon)
        .replace("{$close_button_color}", &close_color)
        .replace("{$background_color}", &background)
}
