//! Attachment images: stored metadata, intermediate sizes, `srcset` / `sizes`
//! and the `<img>` markup of `wp_get_attachment_image()` (wp-includes/media.php).

use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_url};
use rustpress_core::options::{maybe_unserialize, php_to_json};
use rustpress_db::Post;
use serde_json::Value;

use crate::html::TagEditor;

/// `wp_upload_dir()['baseurl']`.
pub fn upload_baseurl(site: &Site) -> String {
    let custom = site.options.string("upload_url_path", "");
    if custom.is_empty() { site.site_url("wp-content/uploads") } else { custom.trim_end_matches('/').to_owned() }
}

/// The decoded `_wp_attachment_metadata` of an attachment.
#[derive(Debug, Clone)]
pub struct Metadata(pub Value);

impl Metadata {
    pub fn parse(raw: &str) -> Option<Self> {
        let value = php_to_json(&maybe_unserialize(raw));
        value.is_object().then_some(Self(value))
    }

    pub fn file(&self) -> Option<&str> {
        self.0.get("file").and_then(Value::as_str)
    }

    pub fn width(&self) -> u64 {
        self.0.get("width").and_then(Value::as_u64).unwrap_or(0)
    }

    pub fn height(&self) -> u64 {
        self.0.get("height").and_then(Value::as_u64).unwrap_or(0)
    }

    pub fn sizes(&self) -> Option<&serde_json::Map<String, Value>> {
        self.0.get("sizes").and_then(Value::as_object)
    }
}

/// Reads `_wp_attachment_metadata` and `_wp_attached_file` of an attachment.
pub async fn load_metadata(site: &Site, post: &Post) -> Result<(Option<Metadata>, Option<String>), rustpress_db::Error> {
    let meta = site.db.post_meta(post.id).await?;
    let value = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
    Ok((value("_wp_attachment_metadata").and_then(|raw| Metadata::parse(&raw)), value("_wp_attached_file")))
}

/// `wp_get_attachment_url()`.
pub fn attachment_url(site: &Site, attached_file: &str) -> String {
    format!("{}/{}", upload_baseurl(site), attached_file.trim_start_matches('/'))
}

/// `wp_attachment_is_image()`.
pub fn is_image(post: &Post) -> bool {
    post.post_mime_type.starts_with("image/")
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// `wp_constrain_dimensions()` with a maximum width only.
fn constrain_to_width(width: u64, height: u64, max_width: u64) -> (u64, u64) {
    if max_width == 0 {
        return (width, height);
    }
    let ratio = if width > max_width { max_width as f64 / width as f64 } else { 1.0 };
    let scaled = |n: u64| ((n as f64 * ratio).round() as u64).max(1);
    let (mut w, h) = (scaled(width), scaled(height));
    if width > max_width && w + 1 == max_width {
        w = max_width;
    }
    (w, h)
}

/// `wp_image_matches_ratio()`.
fn matches_ratio(source_w: u64, source_h: u64, target_w: u64, target_h: u64) -> bool {
    let (constrained, expected) = if source_w > target_w {
        (constrain_to_width(source_w, source_h, target_w), (target_w, target_h))
    } else {
        (constrain_to_width(target_w, target_h, source_w), (source_w, source_h))
    };
    constrained.0.abs_diff(expected.0) <= 1 && constrained.1.abs_diff(expected.1) <= 1
}

/// The URL and size `image_downsize()` picks for a named size.
pub struct Downsized {
    pub url: String,
    pub width: u64,
    pub height: u64,
}

pub fn image_downsize(full_url: &str, meta: &Metadata, size: &str) -> Downsized {
    if let Some(entry) = meta.sizes().and_then(|s| s.get(size)).and_then(Value::as_object)
        && let Some(file) = entry.get("file").and_then(Value::as_str) {
            let full_name = basename(full_url);
            return Downsized {
                url: full_url.replace(full_name, file),
                width: entry.get("width").and_then(Value::as_u64).unwrap_or(0),
                height: entry.get("height").and_then(Value::as_u64).unwrap_or(0),
            };
        }
    Downsized { url: full_url.to_owned(), width: meta.width(), height: meta.height() }
}

/// `wp_calculate_image_srcset()`.
pub fn srcset(site: &Site, width: u64, height: u64, image_src: &str, meta: &Metadata) -> Option<String> {
    let file = meta.file()?;
    let sizes = meta.sizes()?;
    if width < 1 {
        return None;
    }
    let dirname = file.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();
    let baseurl = format!("{}/{dirname}", upload_baseurl(site));
    let mut entries: Vec<(u64, u64, String)> = sizes
        .values()
        .filter_map(|s| Some((s.get("width")?.as_u64()?, s.get("height")?.as_u64()?, s.get("file")?.as_str()?.to_owned())))
        .collect();
    entries.push((meta.width(), meta.height(), basename(file).to_owned()));

    let mut src_matched = false;
    // (width, url) in output order; a repeated width replaces the earlier entry in place.
    let mut sources: Vec<(u64, String)> = Vec::new();
    for (w, h, name) in entries {
        let is_src = !src_matched && image_src.contains(&format!("{dirname}{name}"));
        src_matched |= is_src;
        if w > 2048 && !is_src {
            continue;
        }
        if !matches_ratio(width, height, w, h) {
            continue;
        }
        let url = format!("{baseurl}{name}");
        if is_src {
            sources.retain(|(sw, _)| *sw != w);
            sources.insert(0, (w, url));
        } else if let Some(existing) = sources.iter_mut().find(|(sw, _)| *sw == w) {
            existing.1 = url;
        } else {
            sources.push((w, url));
        }
    }
    if !src_matched || sources.len() < 2 {
        return None;
    }
    Some(sources.iter().map(|(w, url)| format!("{} {w}w", url.replace(' ', "%20"))).collect::<Vec<_>>().join(", "))
}

/// `wp_get_attachment_image()` for a named size, after the content filters
/// have added `decoding`, `loading` and the `auto` sizes.
pub fn attachment_image_html(site: &Site, post: &Post, meta: &Metadata, attached_file: &str, size: &str, alt: &str) -> String {
    let full_url = attachment_url(site, attached_file);
    let image = image_downsize(&full_url, meta, size);
    let mut html = format!(
        "<img width=\"{}\" height=\"{}\" src=\"{}\" class=\"attachment-{size} size-{size}\" alt=\"{}\"",
        image.width,
        image.height,
        esc_url(&image.url),
        esc_attr(alt)
    );
    if let Some(srcset) = srcset(site, image.width, image.height, &image.url, meta) {
        html.push_str(&format!(" srcset=\"{}\" sizes=\"auto, (max-width: {w}px) 100vw, {w}px\"", esc_attr(&srcset), w = image.width));
    }
    html.push_str(" />");
    let _ = post;
    add_loading_attributes(&html)
}

/// `wp_filter_content_tags()` for one image: `decoding="async"`, then `loading="lazy"`.
pub fn add_loading_attributes(img: &str) -> String {
    let Some(mut editor) = TagEditor::first(img) else { return img.to_owned() };
    editor.set_attribute("decoding", "async");
    let once = editor.finish();
    let Some(mut editor) = TagEditor::first(&once) else { return once };
    editor.set_attribute("loading", "lazy");
    editor.finish()
}

/// `wp_get_attachment_link( $id, $size, false )`.
pub fn attachment_link(site: &Site, post: &Post, meta: Option<&Metadata>, attached_file: &str, size: &str, alt: &str) -> String {
    let url = attachment_url(site, attached_file);
    let text = match meta {
        Some(meta) if is_image(post) => attachment_image_html(site, post, meta, attached_file, size, alt),
        _ => post.post_title.clone(),
    };
    format!("<a href='{}'>{text}</a>", esc_url(&url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_matching_allows_one_pixel_of_rounding() {
        assert!(matches_ratio(300, 141, 1024, 480));
        assert!(matches_ratio(300, 141, 768, 360));
        assert!(matches_ratio(300, 141, 1500, 703));
        assert!(!matches_ratio(300, 141, 150, 150));
    }

    #[test]
    fn loading_attributes_end_up_in_wordpress_order() {
        assert_eq!(add_loading_attributes("<img width=\"1\" src=\"a\" />"), "<img loading=\"lazy\" decoding=\"async\" width=\"1\" src=\"a\" />");
    }
}
