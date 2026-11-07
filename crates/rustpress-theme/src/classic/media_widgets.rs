//! The media widgets: `WP_Widget_Media::widget()` and the `render_media()` of `WP_Widget_Media_Image`,
//! `WP_Widget_Media_Audio`, `WP_Widget_Media_Video` and `WP_Widget_Media_Gallery`.

use std::sync::Arc;

use rustpress_blocks::images::{self, AttrValue, ImageArgs, LoadingInput, Metadata};
use rustpress_blocks::{media_shortcodes, shortcodes};
use rustpress_core::formatting::{cached_plain_regex, esc_attr, esc_url};
use rustpress_core::shortcodes::Atts;
use rustpress_phpser::{Key, Value};

use super::widgets::{Services, Sidebar, heading, setting_flag, setting_int, setting_str, widget_title};
use crate::Result;

/// `wp_get_audio_extensions()` and `wp_get_video_extensions()`.
const AUDIO_EXTENSIONS: [&str; 5] = ["mp3", "ogg", "flac", "m4a", "wav"];
const VIDEO_EXTENSIONS: [&str; 5] = ["mp4", "m4v", "webm", "ogv", "flv"];

/// The sizes before those of the theme, `get_intermediate_image_sizes()`.
const SITE_SIZES: [&str; 6] = ["thumbnail", "medium", "medium_large", "large", "1536x1536", "2048x2048"];

/// PHP's truth of a string.
fn truthy(text: &str) -> bool {
    !text.is_empty() && text != "0"
}

/// An attachment of the type the widget shows, `is_attachment_with_mime_type()`.
struct Attachment {
    id: u64,
    excerpt: String,
    meta: Option<Metadata>,
    file: String,
    link: String,
}

/// `wp_attachment_is()`: only a row of the type `import` looks at the extension, because the extension of any other row
/// would have to be called `image`, `audio` or `video`.
fn attachment_is(kind: &str, mime_type: &str, file: &str) -> bool {
    if mime_type.starts_with(&format!("{kind}/")) {
        return true;
    }
    if mime_type != "import" {
        return false;
    }
    let extension = file.rsplit_once('.').map_or("", |(_, extension)| extension);
    match kind {
        "image" => ["jpg", "jpeg", "jpe", "gif", "png", "webp", "avif"].contains(&extension),
        "audio" => AUDIO_EXTENSIONS.contains(&extension),
        "video" => VIDEO_EXTENSIONS.contains(&extension),
        _ => false,
    }
}

/// `wp_parse_id_list()`.
fn id_list(value: Option<&Value>) -> Vec<u64> {
    let absint = |text: &str| text.trim().parse::<i64>().map_or(0, i64::unsigned_abs);
    let mut ids: Vec<u64> = Vec::new();
    match value {
        Some(Value::Array(items)) => ids.extend(items.iter().map(|(_, item)| match item {
            Value::Int(n) => n.unsigned_abs(),
            Value::Str(text) => absint(&String::from_utf8_lossy(text)),
            _ => 0,
        })),
        Some(Value::Str(text)) => ids.extend(String::from_utf8_lossy(text).split([' ', '\t', '\n', '\r', '\x0B', '\x0C', ',']).filter(|id| !id.is_empty()).map(absint)),
        Some(Value::Int(n)) => ids.push(n.unsigned_abs()),
        _ => {}
    }
    let mut seen = std::collections::HashSet::new();
    ids.retain(|id| seen.insert(*id));
    ids
}

/// The arguments the widget hands to a shortcode: `array_merge( $instance, ... )`, with the defaults of the instance first.
fn shortcode_atts(settings: &Value, defaults: &[(&str, &str)]) -> Atts {
    let mut atts = Atts::default();
    for (name, value) in defaults {
        atts.set(name, value);
    }
    for (key, value) in settings.as_array().unwrap_or_default() {
        let Key::Str(name) = key else { continue };
        let text = match value {
            Value::Str(text) => String::from_utf8_lossy(text).into_owned(),
            Value::Int(n) => n.to_string(),
            Value::Bool(true) => "1".to_owned(),
            _ => String::new(),
        };
        atts.set(&String::from_utf8_lossy(name), &text);
    }
    atts
}

/// `WP_Widget_Media::widget()`.
pub(super) async fn display(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, base: &str, settings: &Value) -> Result<Option<String>> {
    let site = s.site;
    let post = match u64::try_from(setting_int(settings, "attachment_id")) {
        Ok(id) if id != 0 => site.db.post(id).await?.filter(|post| post.post_type == "attachment"),
        _ => None,
    };
    let has_content = if base == "media_gallery" {
        // `WP_Widget_Media_Gallery::has_content()`.
        let ids = id_list(settings.get("ids"));
        let mut all = !ids.is_empty();
        for id in ids {
            all = all && site.db.post(id).await?.is_some_and(|post| post.post_type == "attachment");
        }
        all
    } else {
        post.is_some() || truthy(&setting_str(settings, "url"))
    };
    if !has_content {
        return Ok(None);
    }
    let (title, _) = widget_title(site, settings, None);
    let heading = if truthy(&title) { heading(sidebar, &title) } else { String::new() };
    let mut attachment = None;
    if let Some(post) = post {
        let (meta, file) = images::load_metadata(site, &post).await?;
        if let Some(file) = file.filter(|file| !file.is_empty() && attachment_is(base.trim_start_matches("media_"), &post.post_mime_type, file)) {
            let link = site.permalink(&post).await?;
            attachment = Some(Attachment { id: post.id, excerpt: post.post_excerpt.clone(), meta, file, link });
        }
    }
    let body = match base {
        "media_image" => image(s, settings, attachment.as_ref()).await?,
        "media_audio" => audio(s, settings, attachment.as_ref()),
        "media_video" => video(s, settings, attachment.as_ref()),
        _ => gallery(s, settings).await?,
    };
    Ok(Some(format!("{before}{heading}{body}{}", sidebar.after_widget)))
}

/// `_wp_get_image_size_from_meta()`: the width.
fn width_from_meta(size: &str, meta: Option<&Metadata>) -> u64 {
    let Some(meta) = meta else { return 0 };
    if size == "full" {
        return meta.width();
    }
    meta.sizes().and_then(|sizes| sizes.get(size)).and_then(|entry| entry.get("width")).and_then(serde_json::Value::as_u64).unwrap_or(0)
}

/// The tag of an image that is no attachment.
fn plain_image(s: &Services<'_>, settings: &Value) -> String {
    let dimension = |name: &str| match settings.get(name) {
        Some(Value::Int(0)) | None => String::new(),
        Some(_) => setting_str(settings, name),
    };
    let pixels = |name: &str| u64::try_from(setting_int(settings, name)).unwrap_or(0);
    let mut attr: Vec<(String, String)> = vec![
        ("class".into(), format!("image {}", setting_str(settings, "image_classes"))),
        ("src".into(), setting_str(settings, "url")),
        ("alt".into(), setting_str(settings, "alt")),
        ("width".into(), dimension("width")),
        ("height".into(), dimension("height")),
    ];
    let input = LoadingInput { width: Some(pixels("width")), height: Some(pixels("height")), ..LoadingInput::default() };
    attr.extend(images::loading_attributes(s.env, "img", &input, "widget_media_image"));
    let attributes: String = attr.iter().map(|(name, value)| format!(" {name}=\"{}\"", esc_attr(value))).collect();
    format!("<img{attributes} />")
}

/// `WP_Widget_Media_Image::render_media()`.
async fn image(s: &Services<'_>, settings: &Value, attachment: Option<&Attachment>) -> Result<String> {
    let site = s.site;
    let text = |name: &str| setting_str(settings, name);
    let (mut html, caption, width) = if let Some(attachment) = attachment {
        let caption = if matches!(settings.get("caption"), Some(Value::Null)) {
            attachment.excerpt.clone()
        } else if truthy(text("caption").trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B'])) {
            text("caption")
        } else {
            String::new()
        };
        let mut class = format!("image wp-image-{} {}", attachment.id, text("image_classes"));
        let mut attrs = vec![("style".to_owned(), AttrValue::Text("max-width: 100%; height: auto;".into()))];
        if truthy(&text("image_title")) {
            attrs.push(("title".into(), AttrValue::Text(text("image_title"))));
        }
        if truthy(&text("alt")) {
            attrs.push(("alt".into(), AttrValue::Text(text("alt"))));
        }
        let stored = text("size");
        let size = if settings.get("size").is_none() { "medium".to_owned() } else { stored };
        let theme = rustpress_core::image_sizes::of_theme(s.stylesheet);
        let named = size == "full" || SITE_SIZES.contains(&size.as_str()) || theme.iter().any(|(name, ..)| *name == size);
        let (pixels_w, pixels_h) = (u64::try_from(setting_int(settings, "width")).unwrap_or(0), u64::try_from(setting_int(settings, "height")).unwrap_or(0));
        let (size_class, requested, width) = if named { (size.clone(), None, width_from_meta(&size, attachment.meta.as_ref())) } else { (format!("{pixels_w}x{pixels_h}"), Some((pixels_w, pixels_h)), pixels_w) };
        class.push_str(&format!(" attachment-{size_class} size-{size_class}"));
        attrs.insert(0, ("class".into(), AttrValue::Text(class)));
        let html = images::get_attachment_image(s.env, attachment.id, &size_class, &ImageArgs { attrs, requested, ..ImageArgs::default() }).await?;
        (html, caption, width)
    } else {
        if !truthy(&text("url")) {
            return Ok(String::new());
        }
        (plain_image(s, settings), text("caption"), u64::try_from(setting_int(settings, "width")).unwrap_or(0))
    };

    let link_type = text("link_type");
    let link_type = if settings.get("link_type").is_none() { "custom".to_owned() } else { link_type };
    let url = match (link_type.as_str(), attachment) {
        ("file", Some(attachment)) => images::attachment_url(site, &attachment.file),
        ("file", None) => text("url"),
        ("post", Some(attachment)) => attachment.link.clone(),
        ("custom", _) if truthy(&text("link_url")) => text("link_url"),
        _ => String::new(),
    };
    if truthy(&url) {
        let mut link = format!("<a href=\"{}\"", esc_url(&url));
        if truthy(&text("link_classes")) {
            link.push_str(&format!(" class=\"{}\"", esc_attr(&text("link_classes"))));
        }
        if truthy(&text("link_rel")) {
            link.push_str(&format!(" rel=\"{}\"", esc_attr(&text("link_rel"))));
        }
        if setting_flag(settings, "link_target_blank") {
            link.push_str(" target=\"_blank\"");
        }
        html = format!("{link}>{html}</a>");
    }
    if truthy(&caption) {
        let mut atts = Atts::default();
        atts.set("width", &width.to_string());
        atts.set("caption", &caption);
        html = shortcodes::img_caption_shortcode(s.env, None, atts, &html).await?;
    }
    Ok(html)
}

/// The post of the template when a sidebar is printed: the loop leaves the last one in `$post`.
fn current_post(s: &Services<'_>) -> u64 {
    s.q.posts.last().map_or(0, |post| post.id)
}

/// `WP_Widget_Media_Audio::render_media()`.
fn audio(s: &Services<'_>, settings: &Value, attachment: Option<&Attachment>) -> String {
    let mut atts = shortcode_atts(settings, &[("preload", "none")]);
    let src = attachment.map_or_else(|| setting_str(settings, "url"), |attachment| images::attachment_url(s.site, &attachment.file));
    atts.set("src", &src);
    media_shortcodes::audio(s.env, current_post(s), &atts).unwrap_or_default()
}

/// `WP_Widget_Media_Video::inject_video_max_width_style()`.
fn inject_video_max_width_style(html: &str) -> String {
    let html = cached_plain_regex(r#"[ \t\n\r\x0B\x0C]height="[0-9]+""#).replace_all(html, "");
    let html = cached_plain_regex(r#"[ \t\n\r\x0B\x0C]width="[0-9]+""#).replace_all(&html, "");
    cached_plain_regex(r"width:[ \t\n\r\x0B\x0C]*[0-9]+px").replace_all(&html, "width:100%").into_owned()
}

/// `WP_Widget_Media_Video::render_media()`. Any other address goes to `wp_oembed_get()`, which prints nothing for an
/// address without a provider. Asking a provider or one of the site's own posts is not ported.
fn video(s: &Services<'_>, settings: &Value, attachment: Option<&Attachment>) -> String {
    let src = attachment.map_or_else(|| setting_str(settings, "url"), |attachment| images::attachment_url(s.site, &attachment.file));
    if !truthy(&src) {
        return String::new();
    }
    let youtube = cached_plain_regex(r"^https?://(?:www\.)?(?:youtube\.com/watch|youtu\.be/)").is_match(&src);
    let vimeo = cached_plain_regex(r"^https?://(.+\.)?vimeo\.com/.*").is_match(&src);
    if attachment.is_none() && !youtube && !vimeo {
        return String::new();
    }
    let mut atts = shortcode_atts(settings, &[("preload", "metadata")]);
    atts.set("src", &src);
    let html = media_shortcodes::video(s.env, current_post(s), &atts, &setting_str(settings, "content")).unwrap_or_default();
    inject_video_max_width_style(&html)
}

/// `WP_Widget_Media_Gallery::render_media()`.
async fn gallery(s: &Services<'_>, settings: &Value) -> Result<String> {
    let mut atts = shortcode_atts(settings, &[("columns", "3"), ("size", "thumbnail"), ("link_type", "post")]);
    let ids: Vec<String> = id_list(settings.get("ids")).iter().map(u64::to_string).collect();
    atts.set("ids", &ids.join(","));
    let link = atts.get("link_type").unwrap_or_default().to_owned();
    atts.set("link", &link);
    if setting_flag(settings, "orderby_random") {
        atts.set("orderby", "rand");
    }
    let post = s.q.posts.last().cloned().map(Arc::new);
    Ok(shortcodes::gallery_shortcode(s.env, post.as_ref(), atts).await?)
}
