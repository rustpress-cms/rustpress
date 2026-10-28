//! Attachment images: stored metadata, intermediate sizes, `srcset` / `sizes`,
//! the `<img>` markup of `wp_get_attachment_image()`, the loading optimizations
//! (`decoding`, `loading`, `fetchpriority`) and `wp_filter_content_tags()`
//! (wp-includes/media.php).

use std::collections::HashMap;
use std::sync::Arc;

use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_url, strip_tags};
use rustpress_core::options::{maybe_unserialize, php_to_json};
use rustpress_db::Post;
use serde_json::Value;

use crate::Result;
use crate::env::Env;
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
pub async fn load_metadata(site: &Site, post: &Post) -> std::result::Result<(Option<Metadata>, Option<String>), rustpress_db::Error> {
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

/// PHP's `round()` for the non-negative numbers used here.
fn round(n: f64) -> u64 {
    n.round() as u64
}

/// `wp_constrain_dimensions()`.
pub fn constrain_dimensions(width: u64, height: u64, max_width: u64, max_height: u64) -> (u64, u64) {
    if max_width == 0 && max_height == 0 {
        return (width, height);
    }
    let (mut width_ratio, mut height_ratio) = (1.0f64, 1.0f64);
    let (mut did_width, mut did_height) = (false, false);
    if max_width > 0 && width > 0 && width > max_width {
        width_ratio = max_width as f64 / width as f64;
        did_width = true;
    }
    if max_height > 0 && height > 0 && height > max_height {
        height_ratio = max_height as f64 / height as f64;
        did_height = true;
    }
    let smaller = width_ratio.min(height_ratio);
    let larger = width_ratio.max(height_ratio);
    let ratio = if round(width as f64 * larger) > max_width || round(height as f64 * larger) > max_height { smaller } else { larger };
    let mut w = round(width as f64 * ratio).max(1);
    let mut h = round(height as f64 * ratio).max(1);
    if did_width && w + 1 == max_width {
        w = max_width;
    }
    if did_height && h + 1 == max_height {
        h = max_height;
    }
    (w, h)
}

/// `image_constrain_size_for_editor()` for a named size in the front end.
fn constrain_size_for_display(site: &Site, theme: &dyn crate::env::ThemeAccess, width: u64, height: u64, size: &str) -> (u64, u64) {
    let option = |name: &str| site.options.u64(name, 0);
    let (max_width, max_height) = match size {
        "thumb" | "thumbnail" => {
            let (w, h) = (option("thumbnail_size_w"), option("thumbnail_size_h"));
            if w == 0 && h == 0 { (128, 96) } else { (w, h) }
        }
        "medium" => (option("medium_size_w"), option("medium_size_h")),
        "medium_large" => (option("medium_large_size_w"), option("medium_large_size_h")),
        "large" => (option("large_size_w"), option("large_size_h")),
        other => theme.image_size(other).unwrap_or((width, height)),
    };
    constrain_dimensions(width, height, max_width, max_height)
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
    // The metadata of a PDF has no `file`, so no srcset (`get_attachment_image()` adds a marker).
    if meta.0.get("_preview").is_some() {
        return None;
    }
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

/// `image_get_intermediate_size()` for a size given as `[width, height]`, as the URL of the best
/// generated file: an exact match, else the smallest larger one with the same proportions.
pub fn intermediate_url(full_url: &str, meta: &Metadata, width: u64, height: u64) -> Option<String> {
    intermediate_size(full_url, meta, width, height).map(|(url, _, _)| url)
}

/// [intermediate_url] with the size of the file.
pub fn intermediate_size(full_url: &str, meta: &Metadata, width: u64, height: u64) -> Option<(String, u64, u64)> {
    let sizes = meta.sizes()?;
    let mut candidates: Vec<(u64, &str, u64, u64)> = Vec::new();
    for data in sizes.values() {
        let (w, h) = (data.get("width").and_then(Value::as_u64).unwrap_or(0), data.get("height").and_then(Value::as_u64).unwrap_or(0));
        let Some(file) = data.get("file").and_then(Value::as_str) else { continue };
        if w == width && h == height {
            candidates = vec![(w * h, file, w, h)];
            break;
        }
        if w >= width && h >= height {
            let same_ratio = if width == 0 || height == 0 { matches_ratio(w, h, meta.width(), meta.height()) } else { matches_ratio(w, h, width, height) };
            if same_ratio {
                candidates.push((w * h, file, w, h));
            }
        }
    }
    candidates.sort_by_key(|(area, ..)| *area);
    if let Some((_, file, w, h)) = candidates.first() {
        return Some((full_url.replace(basename(full_url), file), *w, *h));
    }
    // A request that fits into the thumbnail gets the thumbnail (WordPress compares the height with its width, too).
    let thumbnail = sizes.get("thumbnail")?;
    let (w, h) = (thumbnail.get("width").and_then(Value::as_u64).unwrap_or(0), thumbnail.get("height").and_then(Value::as_u64).unwrap_or(0));
    let file = thumbnail.get("file").and_then(Value::as_str)?;
    (width <= w && height <= w).then(|| (full_url.replace(basename(full_url), file), w, h))
}

/// `wp_get_attachment_image_src( $id, array( $width, 0 ) )`: `(url, width, height)`.
pub async fn attachment_image_src_width(env: &Env<'_>, id: u64, width: u64) -> Result<Option<(String, u64, u64)>> {
    let Some(post) = env.site.db.post(id).await? else { return Ok(None) };
    let (meta, attached) = load_metadata(env.site, &post).await?;
    let (Some(meta), Some(attached)) = (meta, attached) else { return Ok(None) };
    let full = attachment_url(env.site, &attached);
    let (url, w, h) = match intermediate_size(&full, &meta, width, 0) {
        Some((url, w, h)) => (url, w, h),
        None => (full, meta.width(), meta.height()),
    };
    let (w, h) = constrain_dimensions(w, h, width, 0);
    Ok(Some((url, w, h)))
}

/// `wp_calculate_image_sizes()` for a size given in pixels.
pub fn calculate_sizes(width: u64) -> Option<String> {
    (width > 0).then(|| format!("(max-width: {width}px) 100vw, {width}px"))
}

// ------------------------------------------------------------ loading attributes

/// What the loading logic remembers between the images of one page
/// (`wp_increase_content_media_count()`, `wp_high_priority_element_flag()`,
/// the main loop and the running `the_content` filter).
#[derive(Debug, Default)]
pub struct MediaState {
    /// Content media seen so far in the main loop or the header.
    pub count: u32,
    /// `wp_high_priority_element_flag( false )` has been called.
    pub high_priority_taken: bool,
    /// `in_the_loop() && is_main_query()`: singular templates render inside the loop.
    pub in_main_loop: bool,
    /// Between `get_header()` and the loop of a classic theme: images there are in the viewport.
    pub in_header: bool,
    /// Nesting depth of `apply_filters( 'the_content' )`.
    pub content_depth: u32,
    /// Decoded `_wp_attachment_metadata` by attachment ID.
    pub metadata: HashMap<u64, Option<Arc<Metadata>>>,
}

/// The attributes `wp_get_loading_optimization_attributes()` looks at.
#[derive(Debug, Clone, Default)]
pub struct LoadingInput {
    pub width: Option<u64>,
    pub height: Option<u64>,
    /// `Some(None)`: the attribute is set to `false` (omit it, but count the image as visible).
    pub loading: Option<Option<String>>,
    pub fetchpriority: Option<String>,
    pub decoding: Option<String>,
}

/// Sets `key` in an ordered attribute list: replaces in place, or appends.
fn put(list: &mut Vec<(String, String)>, key: &str, value: &str) {
    match list.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => slot.1 = value.to_owned(),
        None => list.push((key.to_owned(), value.to_owned())),
    }
}

const MIN_PRIORITY_PIXELS: u64 = 50_000;
const OMIT_LOADING_THRESHOLD: u32 = 3;

/// `wp_get_loading_optimization_attributes()` for block themes (`get_header` never fires there).
pub fn loading_attributes(env: &Env<'_>, tag: &str, input: &LoadingInput, context: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    // The overall template is handled granularly, nested filters belong to the running `the_content`.
    if context == "template" || (tag != "img" && tag != "iframe") {
        return out;
    }
    if context != "the_content" && env.with_state(|s| s.media.content_depth > 0) {
        return out;
    }
    if tag == "img" {
        put(&mut out, "decoding", input.decoding.as_deref().unwrap_or("async"));
    }
    let (Some(width), Some(height)) = (input.width, input.height) else { return out };

    let mut in_viewport: Option<bool> = input.loading.as_ref().map(|loading| loading.as_deref() != Some("lazy"));
    let (mut increase_count, mut maybe_increase_count) = (false, false);

    let existing = input.fetchpriority.as_deref();
    let is_low = existing == Some("low");
    match existing {
        Some("high") => {
            if in_viewport == Some(false) {
                put(&mut out, "fetchpriority", "high");
            } else {
                in_viewport = Some(true);
            }
        }
        Some("low") => {
            in_viewport = Some(false);
            put(&mut out, "fetchpriority", "low");
        }
        Some("auto") => put(&mut out, "fetchpriority", "auto"),
        _ => {}
    }

    if in_viewport.is_none() {
        if matches!(context, "template_part_header" | "get_header_image_tag") || env.with_state(|s| s.media.in_header) {
            in_viewport = Some(true);
            maybe_increase_count = true;
        } else if env.with_state(|s| s.media.in_main_loop) {
            let seen = env.with_state(|s| s.media.count);
            increase_count = true;
            in_viewport = Some(seen < OMIT_LOADING_THRESHOLD);
        }
    }

    if in_viewport == Some(true) {
        maybe_add_fetchpriority_high(env, &mut out, tag, existing, width, height);
    } else if !is_low {
        put(&mut out, "loading", "lazy");
    }

    if existing != Some("auto") && (increase_count || (maybe_increase_count && MIN_PRIORITY_PIXELS <= width * height)) {
        env.with_state(|s| s.media.count += 1);
    }
    out
}

/// `wp_maybe_add_fetchpriority_high_attr()`.
fn maybe_add_fetchpriority_high(env: &Env<'_>, out: &mut Vec<(String, String)>, tag: &str, existing: Option<&str>, width: u64, height: u64) {
    if tag != "img" {
        return;
    }
    if let Some(existing) = existing.filter(|e| *e != "auto") {
        if existing == "high" {
            put(out, "fetchpriority", "high");
            env.with_state(|s| s.media.high_priority_taken = true);
        }
        return;
    }
    if out.iter().any(|(k, v)| k == "loading" && v == "lazy") {
        return;
    }
    if env.with_state(|s| s.media.high_priority_taken) {
        return;
    }
    if MIN_PRIORITY_PIXELS <= width * height {
        if existing != Some("auto") {
            put(out, "fetchpriority", "high");
        }
        env.with_state(|s| s.media.high_priority_taken = true);
    }
}

impl Env<'_> {
    /// Runs `f` as part of a running `the_content` filter (`doing_filter( 'the_content' )`).
    pub fn in_content_filter<R>(&self, f: impl FnOnce() -> R) -> R {
        self.with_state(|s| s.media.content_depth += 1);
        let result = f();
        self.with_state(|s| s.media.content_depth -= 1);
        result
    }
}

// ------------------------------------------------------------- attachment images

/// An attribute passed to `wp_get_attachment_image()`.
#[derive(Debug, Clone)]
pub enum AttrValue {
    Text(String),
    /// `false`: leave the attribute out.
    Omit,
}

/// The optional arguments of `wp_get_attachment_image()`.
#[derive(Debug, Clone, Default)]
pub struct ImageArgs {
    /// `$attr`, in order.
    pub attrs: Vec<(String, AttrValue)>,
    /// Inside `get_the_post_thumbnail()`: adds the `wp-post-image` class.
    pub post_thumbnail: bool,
    /// The `wp_get_attachment_image_src` filter replacing width and height.
    pub dimensions: Option<(u64, u64)>,
}

/// `wp_get_attachment_image( $id, $size, false, $attr )` for an image attachment with known metadata.
pub fn attachment_image(env: &Env<'_>, meta: Option<&Metadata>, attached_file: &str, size: &str, alt_meta: &str, args: &ImageArgs) -> String {
    let site = env.site;
    let full_url = attachment_url(site, attached_file);
    let (src, mut width, mut height) = match meta {
        Some(meta) => {
            let down = image_downsize(&full_url, meta, size);
            let (w, h) = constrain_size_for_display(site, env.theme, down.width, down.height, size);
            (down.url, w, h)
        }
        None => (full_url, 0, 0),
    };
    if let Some((w, h)) = args.dimensions {
        (width, height) = (w, h);
    }

    let mut attr: Vec<(String, AttrValue)> = vec![
        ("src".into(), AttrValue::Text(src.clone())),
        ("class".into(), AttrValue::Text(format!("attachment-{size} size-{size}"))),
        ("alt".into(), AttrValue::Text(strip_tags(alt_meta).trim().to_owned())),
    ];
    for (key, value) in &args.attrs {
        match attr.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value.clone(),
            None => attr.push((key.clone(), value.clone())),
        }
    }
    let text = |attr: &[(String, AttrValue)], key: &str| -> Option<String> {
        attr.iter().find(|(k, _)| k == key).and_then(|(_, v)| if let AttrValue::Text(t) = v { Some(t.clone()) } else { None })
    };
    let input = LoadingInput {
        width: Some(width),
        height: Some(height),
        loading: attr.iter().find(|(k, _)| k == "loading").map(|(_, v)| if let AttrValue::Text(t) = v { Some(t.clone()) } else { None }),
        fetchpriority: text(&attr, "fetchpriority"),
        decoding: text(&attr, "decoding"),
    };
    for (key, value) in loading_attributes(env, "img", &input, "wp_get_attachment_image") {
        match attr.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = AttrValue::Text(value),
            None => attr.push((key, AttrValue::Text(value))),
        }
    }
    // Drop what is invalid or switched off.
    attr.retain(|(key, value)| match (key.as_str(), value) {
        ("decoding", AttrValue::Text(t)) => matches!(t.as_str(), "async" | "sync" | "auto"),
        ("decoding", AttrValue::Omit) => false,
        ("loading" | "fetchpriority", AttrValue::Omit) => false,
        ("loading" | "fetchpriority", AttrValue::Text(t)) => !t.is_empty() && t != "0",
        _ => true,
    });

    let empty = |attr: &[(String, AttrValue)], key: &str| text(attr, key).is_none_or(|t| t.is_empty() || t == "0");
    if empty(&attr, "srcset")
        && let Some(meta) = meta
    {
        let srcset = srcset(site, width, height, &src, meta);
        let sizes = env.theme.filter_calculated_sizes(site, env.query, calculate_sizes(width), width);
        if let Some(srcset) = srcset
            && (sizes.is_some() || !empty(&attr, "sizes"))
        {
            put_attr(&mut attr, "srcset", srcset);
            if empty(&attr, "sizes") {
                put_attr(&mut attr, "sizes", sizes.unwrap_or_default());
            }
        }
    }
    if text(&attr, "loading").as_deref() == Some("lazy")
        && let Some(sizes) = text(&attr, "sizes")
        && !sizes_include_auto(&sizes)
    {
        put_attr(&mut attr, "sizes", format!("auto, {sizes}"));
    }
    if let Some(sizes) = env.theme.filter_attachment_sizes(site, env.query, text(&attr, "sizes").as_deref()) {
        put_attr(&mut attr, "sizes", sizes);
    }
    if args.post_thumbnail {
        let class = text(&attr, "class").unwrap_or_default();
        put_attr(&mut attr, "class", format!("{class} wp-post-image"));
    }

    let mut html = String::from("<img");
    if width != 0 {
        html.push_str(&format!(" width=\"{width}\""));
    }
    if height != 0 {
        html.push_str(&format!(" height=\"{height}\""));
    }
    for (key, value) in &attr {
        if let AttrValue::Text(value) = value {
            html.push_str(&format!(" {key}=\"{}\"", esc_attr(value)));
        }
    }
    html.push_str(" />");
    html
}

fn put_attr(attr: &mut Vec<(String, AttrValue)>, key: &str, value: String) {
    match attr.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => slot.1 = AttrValue::Text(value),
        None => attr.push((key.to_owned(), AttrValue::Text(value))),
    }
}

/// `wp_sizes_attribute_includes_valid_auto()`.
fn sizes_include_auto(sizes: &str) -> bool {
    let first = sizes.split(',').next().unwrap_or_default();
    first.trim_matches([' ', '\t', '\x0c', '\r', '\n']).eq_ignore_ascii_case("auto")
}

/// A PDF or video with a generated preview image: `image_downsize()` starts from its `full` size.
/// Returns metadata shaped like an image's and the path of the preview file.
pub fn preview_metadata(meta: Option<&Metadata>, attached: Option<&str>) -> Option<(Metadata, String)> {
    let full = meta?.sizes()?.get("full")?.as_object()?;
    let (file, attached) = (full.get("file")?.as_str()?, attached?);
    let path = match attached.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/{file}"),
        None => file.to_owned(),
    };
    let mut json = meta.map(|m| m.0.clone()).unwrap_or_default();
    if let Some(map) = json.as_object_mut() {
        map.insert("file".into(), Value::from(path.clone()));
        map.insert("_preview".into(), Value::Bool(true));
        map.insert("width".into(), full.get("width").cloned().unwrap_or_default());
        map.insert("height".into(), full.get("height").cloned().unwrap_or_default());
    }
    Some((Metadata(json), path))
}

/// `wp_get_attachment_image( $id, $size, false, $attr )`: loads the attachment first.
pub async fn get_attachment_image(env: &Env<'_>, attachment_id: u64, size: &str, args: &ImageArgs) -> Result<String> {
    let Some(post) = env.site.db.post(attachment_id).await? else { return Ok(String::new()) };
    if post.post_type != "attachment" {
        return Ok(String::new());
    }
    let (meta, attached) = load_metadata(env.site, &post).await?;
    let (meta, attached) = if is_image(&post) {
        (meta, attached)
    } else {
        match preview_metadata(meta.as_ref(), attached.as_deref()) {
            Some((meta, path)) => (Some(meta), Some(path)),
            None => return Ok(String::new()),
        }
    };
    let Some(attached) = attached else { return Ok(String::new()) };
    let rows = env.site.db.post_meta(attachment_id).await?;
    let alt = rows.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value.clone()).unwrap_or_default();
    Ok(attachment_image(env, meta.as_ref(), &attached, size, &alt, args))
}

/// `get_theme_mod( 'custom_logo' )`: the `site_logo` option wins over the theme mod.
pub async fn custom_logo_id(site: &Site) -> Result<u64> {
    if let Some(raw) = site.options.fetch(&site.db, "site_logo").await? {
        return Ok(raw.trim().parse().unwrap_or(0));
    }
    let name = format!("theme_mods_{}", site.settings.stylesheet);
    let Some(raw) = site.options.fetch(&site.db, &name).await? else { return Ok(0) };
    let mods = php_to_json(&maybe_unserialize(&raw));
    Ok(mods.get("custom_logo").and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))).unwrap_or(0))
}

/// `get_post_thumbnail_id()`.
pub async fn post_thumbnail_id(env: &Env<'_>, post_id: u64) -> Result<u64> {
    let rows = env.site.db.post_meta(post_id).await?;
    Ok(rows.iter().find(|m| m.meta_key.as_deref() == Some("_thumbnail_id")).and_then(|m| m.meta_value.as_deref()).and_then(|v| v.trim().parse().ok()).unwrap_or(0))
}

/// `wp_get_attachment_link( $id, $size, false )`.
pub fn attachment_link(env: &Env<'_>, post: &Post, meta: Option<&Metadata>, attached_file: &str, size: &str, alt: &str) -> String {
    let url = attachment_url(env.site, attached_file);
    let image = match meta {
        Some(_) if is_image(post) => attachment_image(env, meta, attached_file, size, alt, &ImageArgs::default()),
        // A PDF shows its preview image.
        Some(_) => preview_metadata(meta, Some(attached_file)).map(|(m, path)| attachment_image(env, Some(&m), &path, size, alt, &ImageArgs::default())).unwrap_or_default(),
        None => String::new(),
    };
    let mut text = image;
    if text.trim().is_empty() {
        text = post.post_title.clone();
    }
    if text.trim().is_empty() {
        let name = basename(attached_file);
        text = rustpress_core::formatting::esc_html(name.rsplit_once('.').map_or(name, |(stem, _)| stem));
    }
    format!("<a href='{}'>{text}</a>", esc_url(&url))
}

// ------------------------------------------------------- wp_filter_content_tags

/// The decoded metadata of an attachment, cached for the request.
pub async fn attachment_metadata(env: &Env<'_>, id: u64) -> Result<Option<Arc<Metadata>>> {
    if let Some(hit) = env.with_state(|s| s.media.metadata.get(&id).cloned()) {
        return Ok(hit);
    }
    let rows = env.site.db.post_meta(id).await?;
    let meta = rows
        .iter()
        .find(|m| m.meta_key.as_deref() == Some("_wp_attachment_metadata"))
        .and_then(|m| m.meta_value.as_deref())
        .and_then(Metadata::parse)
        .map(Arc::new);
    env.with_state(|s| s.media.metadata.insert(id, meta.clone()));
    Ok(meta)
}

/// `/<(img|iframe)\s[^>]+>/`: `(start, end, is_img)` of each match.
fn media_tags(content: &str) -> Vec<(usize, usize, bool)> {
    let bytes = content.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(rel) = content[at..].find('<') {
        let start = at + rel;
        let rest = &content[start + 1..];
        let (name_len, is_img) = if rest.starts_with("img") {
            (3, true)
        } else if rest.starts_with("iframe") {
            (6, false)
        } else {
            at = start + 1;
            continue;
        };
        let space = start + 1 + name_len;
        // PCRE `\s`: space, \t, \n, \v, \f, \r.
        if bytes.get(space).is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'))
            && let Some(close_rel) = content[space + 1..].find('>')
            && close_rel > 0
        {
            let end = space + 1 + close_rel + 1;
            found.push((start, end, is_img));
            at = end;
        } else {
            at = start + 1;
        }
    }
    found
}

/// The first `name="value"` style match of PHP's `preg_match( '/ name=["\']VALUE["\']/' )`,
/// with `VALUE` made of characters accepted by `valid`.
fn quoted_attr(tag: &str, name: &str, valid: fn(char) -> bool) -> Option<String> {
    let needle = format!(" {name}=");
    let mut at = 0;
    while let Some(rel) = tag[at..].find(&needle) {
        let value_start = at + rel + needle.len();
        let rest = &tag[value_start..];
        if rest.starts_with(['"', '\'']) {
            let body = &rest[1..];
            let len: usize = body.chars().take_while(|c| valid(*c)).map(char::len_utf8).sum();
            if len > 0 && body[len..].starts_with(['"', '\'']) {
                return Some(body[..len].to_owned());
            }
        }
        at = value_start;
    }
    None
}

/// `/src="([^"]+)"/` followed by `explode( '?', ... )[0]`.
fn image_source(tag: &str) -> Option<String> {
    let mut at = 0;
    while let Some(rel) = tag[at..].find("src=\"") {
        let start = at + rel + 5;
        let end = tag[start..].find('"')?;
        if end > 0 {
            let src = &tag[start..start + end];
            return Some(src.split('?').next().unwrap_or(src).to_owned());
        }
        at = start;
    }
    None
}

/// `wp_image_src_get_dimensions()`.
fn src_dimensions(src: &str, meta: &Metadata) -> Option<(u64, u64)> {
    if let Some(file) = meta.file()
        && src.contains(basename(file))
    {
        return Some((meta.width(), meta.height()));
    }
    let name = basename(src);
    meta.sizes()?.values().find(|entry| entry.get("file").and_then(Value::as_str) == Some(name)).map(|entry| {
        (entry.get("width").and_then(Value::as_u64).unwrap_or(0), entry.get("height").and_then(Value::as_u64).unwrap_or(0))
    })
}

/// `wp_img_tag_add_width_and_height_attr()`.
fn add_width_and_height(image: String, meta: Option<&Metadata>) -> String {
    let Some(src) = image_source(&image).filter(|s| !s.is_empty()) else { return image };
    let Some((mut width, mut height)) = meta.and_then(|m| src_dimensions(&src, m)) else { return image };
    if width == 0 || height == 0 {
        return image;
    }
    // A width enforced through the style attribute (inline images) wins.
    if let Some(at) = image.find("style=\"width:") {
        let rest = image[at + 13..].trim_start();
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() && rest[digits.len()..].starts_with("px;\"")
            && let Ok(style_width) = digits.parse::<u64>()
            && style_width > 0
        {
            height = round(height as f64 * style_width as f64 / width as f64);
            width = style_width;
        }
    }
    image.replace("<img", &format!("<img width=\"{width}\" height=\"{height}\""))
}

/// `/-e[0-9]{13}/`: the marker the image editor leaves in the file name.
fn edit_hash(file: &str) -> Option<&str> {
    let bytes = file.as_bytes();
    (0..bytes.len()).find(|&i| bytes[i..].starts_with(b"-e") && bytes.len() >= i + 15 && bytes[i + 2..i + 15].iter().all(u8::is_ascii_digit)).map(|i| &file[i..i + 15])
}

/// `wp_image_add_srcset_and_sizes()`.
fn add_srcset_and_sizes(env: &Env<'_>, image: String, meta: Option<&Metadata>) -> String {
    let Some(meta) = meta.filter(|m| m.sizes().is_some_and(|s| !s.is_empty())) else { return image };
    let Some(src) = image_source(&image).filter(|s| !s.is_empty()) else { return image };
    // Bail out if the image was edited after it was inserted.
    if let Some(hash) = meta.file().and_then(edit_hash)
        && !basename(&src).contains(hash)
    {
        return image;
    }
    let number = |name: &str| quoted_attr(&image, name, |c| c.is_ascii_digit()).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let (mut width, mut height) = (number("width"), number("height"));
    if width == 0 || height == 0 {
        match src_dimensions(&src, meta) {
            Some(size) => (width, height) = size,
            None => return image,
        }
    }
    let Some(srcset) = srcset(env.site, width, height, &src, meta) else { return image };
    let mut attr = format!(" srcset=\"{}\"", esc_attr(&srcset));
    if !image.contains(" sizes=") {
        let Some(sizes) = env.theme.filter_calculated_sizes(env.site, env.query, calculate_sizes(width), width) else { return image };
        attr.push_str(&format!(" sizes=\"{}\"", esc_attr(&sizes)));
    }
    // `preg_replace( '/<img ([^>]+?)[\/ ]*>/', '<img $1' . $attr . ' />', $image )`
    let Some(inner) = image.strip_prefix("<img ").and_then(|s| s.strip_suffix('>')) else { return image };
    let inner = inner.trim_end_matches(['/', ' ']);
    if inner.is_empty() {
        return image;
    }
    format!("<img {inner}{attr} />")
}

fn letters(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// `wp_img_tag_add_loading_optimization_attrs()`.
fn add_loading_optimization(env: &Env<'_>, mut image: String, context: &str) -> String {
    let number = |name: &str| quoted_attr(&image, name, |c| c.is_ascii_digit()).and_then(|v| v.parse::<u64>().ok());
    let input = LoadingInput {
        width: number("width"),
        height: number("height"),
        loading: quoted_attr(&image, "loading", letters).map(Some),
        fetchpriority: quoted_attr(&image, "fetchpriority", letters),
        decoding: quoted_attr(&image, "decoding", letters),
    };
    let optimization = loading_attributes(env, "img", &input, context);
    let value = |key: &str| optimization.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());

    if !image.contains(" src=\"") {
        return image;
    }
    if input.decoding.is_none()
        && let Some(decoding) = value("decoding").filter(|d| matches!(*d, "async" | "sync" | "auto"))
    {
        image = image.replace("<img", &format!("<img decoding=\"{}\"", esc_attr(decoding)));
    }
    if !image.contains(" width=\"") || !image.contains(" height=\"") {
        return image;
    }
    if input.loading.is_none()
        && let Some(loading) = value("loading").filter(|l| matches!(*l, "lazy" | "eager"))
    {
        image = image.replace("<img", &format!("<img loading=\"{}\"", esc_attr(loading)));
    }
    if input.fetchpriority.is_none()
        && let Some(priority) = value("fetchpriority").filter(|p| !p.is_empty())
    {
        image = image.replace("<img", &format!("<img fetchpriority=\"{}\"", esc_attr(priority)));
    }
    image
}

/// `wp_img_tag_add_auto_sizes()`.
fn add_auto_sizes(image: String) -> String {
    let Some(mut editor) = TagEditor::first(&image) else { return image };
    let trim = |s: &str| s.trim_matches([' ', '\t', '\x0c', '\r', '\n']).to_ascii_lowercase();
    if editor.get_attribute("loading").map(trim).as_deref() != Some("lazy") {
        return image;
    }
    if editor.get_attribute("width").is_none_or(str::is_empty) {
        return image;
    }
    let Some(sizes) = editor.get_attribute("sizes").map(str::to_owned) else { return image };
    if sizes_include_auto(&sizes) {
        return image;
    }
    editor.set_attribute("sizes", &format!("auto, {sizes}"));
    editor.finish()
}

/// `wp_iframe_tag_add_loading_attr()`.
fn add_iframe_loading(env: &Env<'_>, iframe: String, context: &str) -> String {
    let input = LoadingInput {
        width: iframe.contains(" width=\"").then_some(100),
        height: iframe.contains(" height=\"").then_some(100),
        ..Default::default()
    };
    let optimization = loading_attributes(env, "iframe", &input, context);
    if !iframe.contains(" src=\"") || !iframe.contains(" width=\"") || !iframe.contains(" height=\"") {
        return iframe;
    }
    match optimization.iter().find(|(k, _)| k == "loading") {
        Some((_, value)) => {
            let value = if matches!(value.as_str(), "lazy" | "eager") { value.as_str() } else { "lazy" };
            iframe.replace("<iframe", &format!("<iframe loading=\"{value}\""))
        }
        None => iframe,
    }
}

/// `wp_filter_content_tags( $content, $context )`: width/height, `srcset`/`sizes` and the
/// loading optimizations for the images of a blob of HTML.
pub async fn filter_content_tags(env: &Env<'_>, content: &str, context: &str) -> Result<String> {
    let matches = media_tags(content);
    if matches.is_empty() {
        return Ok(content.to_owned());
    }
    // Unique tags, with the attachment ID of images (`0` if the class is missing).
    let mut images: Vec<(String, u64)> = Vec::new();
    let mut iframes: Vec<String> = Vec::new();
    for &(start, end, is_img) in &matches {
        let tag = &content[start..end];
        if is_img {
            let id = attachment_id_class(tag);
            match images.iter_mut().find(|(t, _)| t == tag) {
                Some(entry) => entry.1 = id,
                None => images.push((tag.to_owned(), id)),
            }
        } else if !iframes.iter().any(|t| t == tag) {
            iframes.push(tag.to_owned());
        }
    }

    // `_prime_post_caches( $attachment_ids, false, true )`: the rows and meta of all images in two queries.
    let mut ids: Vec<u64> = images.iter().map(|(_, id)| *id).filter(|id| *id > 0).collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() > 1 {
        env.site.db.prime_posts(&ids).await?;
        env.site.db.prime_post_meta(&ids).await?;
    }

    let original = content;
    let mut content = content.to_owned();
    for &(start, end, is_img) in &matches {
        let tag_text = &original[start..end];
        if is_img {
            let Some(index) = images.iter().position(|(t, _)| t == tag_text) else { continue };
            let (tag, id) = images.remove(index);
            let meta = if id > 0 { attachment_metadata(env, id).await? } else { None };
            let mut filtered = tag.clone();
            if id > 0 && !filtered.contains(" width=") && !filtered.contains(" height=") {
                filtered = add_width_and_height(filtered, meta.as_deref());
            }
            if id > 0 && !filtered.contains(" srcset=") {
                filtered = add_srcset_and_sizes(env, filtered, meta.as_deref());
            }
            filtered = add_loading_optimization(env, filtered, context);
            filtered = add_auto_sizes(filtered);
            if filtered != tag {
                content = content.replace(&tag, &filtered);
            }
        } else if let Some(index) = iframes.iter().position(|t| t == tag_text) {
            let tag = iframes.remove(index);
            let mut filtered = tag.clone();
            if !filtered.contains(" loading=") {
                filtered = add_iframe_loading(env, filtered, context);
            }
            if filtered != tag {
                content = content.replace(&tag, &filtered);
            }
        }
    }
    Ok(content)
}

/// `/wp-image-([0-9]+)/i` as an `absint()`.
fn attachment_id_class(tag: &str) -> u64 {
    let lower = tag.to_ascii_lowercase();
    let mut at = 0;
    while let Some(rel) = lower[at..].find("wp-image-") {
        let start = at + rel + 9;
        let digits: String = lower[start..].chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            return digits.parse().unwrap_or(0);
        }
        at = start;
    }
    0
}

/// `get_post_meta( $id, '_wp_attachment_image_alt', true )`.
pub async fn attachment_alt(env: &Env<'_>, id: u64) -> Result<String> {
    let rows = env.site.db.post_meta(id).await?;
    Ok(rows.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value.clone()).unwrap_or_default())
}

/// `wp_get_attachment_image_url( $id, $size )` for an image attachment.
pub async fn attachment_image_url(env: &Env<'_>, id: u64, size: &str) -> Result<Option<String>> {
    let Some(post) = env.site.db.post(id).await? else { return Ok(None) };
    if post.post_type != "attachment" || !is_image(&post) {
        return Ok(None);
    }
    let (meta, attached) = load_metadata(env.site, &post).await?;
    let Some(attached) = attached else { return Ok(None) };
    let full_url = attachment_url(env.site, &attached);
    Ok(Some(match meta {
        Some(meta) => image_downsize(&full_url, &meta, size).url,
        None => full_url,
    }))
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
    fn dimensions_are_constrained_like_wordpress() {
        assert_eq!(constrain_dimensions(1500, 703, 0, 0), (1500, 703));
        assert_eq!(constrain_dimensions(1500, 703, 300, 300), (300, 141));
        assert_eq!(constrain_dimensions(465, 700, 177, 177), (118, 177));
        assert_eq!(constrain_dimensions(100, 50, 300, 300), (100, 50));
    }

    #[test]
    fn media_tags_follow_the_wordpress_pattern() {
        let html = "<p><img src=\"a\"> <img> <img\nsrc='b' /> <IMG src=x> <iframe src=\"u\"></iframe></p>";
        let found: Vec<&str> = media_tags(html).iter().map(|&(s, e, _)| &html[s..e]).collect();
        assert_eq!(found, vec!["<img src=\"a\">", "<img\nsrc='b' />", "<iframe src=\"u\">"]);
    }

    #[test]
    fn quoted_attributes_and_sources_are_extracted() {
        let tag = "<img data-src=\"x\" width=\"12\" height='7' src=\"http://h/a.png?v=1\" loading=\"lazy\">";
        assert_eq!(quoted_attr(tag, "width", |c| c.is_ascii_digit()).as_deref(), Some("12"));
        assert_eq!(quoted_attr(tag, "height", |c| c.is_ascii_digit()).as_deref(), Some("7"));
        assert_eq!(quoted_attr(tag, "loading", letters).as_deref(), Some("lazy"));
        // The regex is not anchored, so `data-src` is found first, like in WordPress.
        assert_eq!(image_source(tag).as_deref(), Some("x"));
        assert_eq!(attachment_id_class("<img class=\"a WP-Image-42 b\">"), 42);
        assert_eq!(attachment_id_class("<img class=\"wp-image-\">"), 0);
    }

    #[test]
    fn srcset_is_appended_in_front_of_the_closing_slash() {
        assert!(sizes_include_auto("auto, (max-width: 5px) 100vw, 5px"));
        assert!(sizes_include_auto(" AUTO"));
        assert!(!sizes_include_auto("(max-width: 5px) 100vw, 5px"));
    }
}
