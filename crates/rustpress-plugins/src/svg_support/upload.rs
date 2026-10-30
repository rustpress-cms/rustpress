//! SVG uploads: the type for the chosen roles (`upload_mimes`, `wp_check_filetype_and_ext`), the check and the
//! cleaning of the file (`bodhi_svgs_sanitize_svg()`), the metadata with a size per registered image size
//! (`bodhi_svgs_generate_svg_attachment_metadata()`), the sizes of the media modal (`bodhi_svgs_response_for_svg()`)
//! and the SVG previews of the media library's templates (`bodhi_svgs_final_output()`).

use std::io::{Read, Write};
use std::path::Path;

use rustpress_core::Site;
use rustpress_core::native::Uploader;
use rustpress_phpser::{Key, Value};

use super::Settings;
use super::admin::DOMAIN;

const SVG: &str = "image/svg+xml";

/// The roles of a setting (`(array) $bodhi_svgs_options[ $key ]`).
fn roles(settings: &Settings, key: &str) -> Vec<String> {
    match settings.get(key) {
        Some(Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        Some(Value::Null) | None => Vec::new(),
        Some(other) => other.as_str().map(|s| vec![s.to_owned()]).unwrap_or_default(),
    }
}

/// `bodhi_svgs_upload_mimes()`: SVG for the roles of `restrict` that may upload files.
pub(super) fn upload_mimes(settings: &Settings, uploader: &Uploader<'_>, mimes: &mut Vec<(String, String)>) {
    if settings.get("restrict").is_none() {
        return;
    }
    let allowed = roles(settings, "restrict");
    if !uploader.roles.iter().any(|r| allowed.contains(r)) || !(uploader.can)("upload_files") {
        return;
    }
    for ext in ["svg", "svgz"] {
        match mimes.iter_mut().find(|(e, _)| e == ext) {
            Some((_, mime)) => *mime = SVG.to_owned(),
            None => mimes.push((ext.to_owned(), SVG.to_owned())),
        }
    }
}

/// `bodhi_svgs_upload_check()`: an SVG the core refused (it finds no image in it) passes when its type is allowed.
pub(super) fn accept_filetype(name: &str, mimes: &[(String, String)]) -> Option<(String, String)> {
    let (ext, mime) = rustpress_content::media::check_filetype_in(name, mimes)?;
    (matches!(ext.as_str(), "svg" | "svgz") && mime == SVG).then(|| (name.to_owned(), mime))
}

/// `bodhi_svgs_is_gzipped()`.
fn is_gzipped(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b, 0x08])
}

fn gunzip(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes).read_to_end(&mut out).ok()?;
    Some(out)
}

/// What the plugin takes for SVG content: an `svg` start tag after an optional declaration, comments and doctype,
/// and a closing tag somewhere.
fn looks_like_svg(content: &str) -> bool {
    let start = regex::Regex::new(r"(?is)^[\s\n]*(?:<\?xml[^>]*>[\s\n]*)?(?:<!--.*?-->[\s\n]*)*(?:<!DOCTYPE[^>]*>[\s\n]*)?(?:<!--.*?-->[\s\n]*)*<svg[^>]*>").expect("valid pattern");
    start.is_match(content) && content.contains("</svg>")
}

/// `bodhi_svgs_sanitize()`: the file cleaned (gzipped again when it was); `None` when it cannot be cleaned.
fn sanitize_file(settings: &Settings, bytes: &[u8]) -> Option<Vec<u8>> {
    let zipped = is_gzipped(bytes);
    let dirty = if zipped { gunzip(bytes)? } else { bytes.to_vec() };
    let dirty = String::from_utf8(dirty).ok()?;
    let minify = settings.get("minify_svg").and_then(Value::as_str) == Some("on");
    let clean = super::sanitizer::sanitize(&dirty, minify)?;
    if !zipped {
        return Some(clean.into_bytes());
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(clean.as_bytes()).ok()?;
    encoder.finish().ok()
}

/// `bodhi_svgs_sanitize_svg()` on `wp_handle_upload_prefilter`.
pub(super) fn prefilter(site: &Site, settings: &Settings, uploader: &Uploader<'_>, name: &str, bytes: &mut Vec<u8>) -> Result<(), String> {
    let t = |text: &str| site.trd(DOMAIN, text);
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    if !name.is_empty() && ext != "svg" && ext != "svgz" {
        return Ok(());
    }
    let content = if is_gzipped(bytes) { gunzip(bytes).ok_or_else(|| t("File is not a valid SVG."))? } else { bytes.clone() };
    if !looks_like_svg(&String::from_utf8_lossy(&content)) {
        return Err(t("File is not a valid SVG."));
    }
    if !uploader.rest && !uploader.media_form_nonce {
        return Err(t("Security check failed."));
    }
    let bypass = roles(settings, "sanitize_on_upload_roles");
    if (uploader.can)("upload_files") && !uploader.roles.iter().any(|r| bypass.contains(r)) {
        *bytes = sanitize_file(settings, bytes).ok_or_else(|| t("Sorry, this file couldn't be sanitized for security reasons and wasn't uploaded."))?;
    }
    Ok(())
}

/// `bodhi_svgs_get_dimensions()`: `width` and `height` of the root element as written (`'0'` for a file that does
/// not load, empty when missing).
fn dimensions(path: &Path) -> (String, String) {
    let Some(content) = std::fs::read(path).ok().filter(|b| !b.is_empty()) else { return ("0".into(), "0".into()) };
    let content = if is_gzipped(&content) { gunzip(&content).unwrap_or_default() } else { content };
    let text = String::from_utf8_lossy(&content);
    let mut root = None;
    for token in xmlparser::Tokenizer::from(text.as_ref()) {
        match token {
            Ok(xmlparser::Token::ElementStart { .. }) if root.is_none() => root = Some((String::new(), String::new())),
            Ok(xmlparser::Token::Attribute { prefix, local, value, .. }) if prefix.as_str().is_empty() => {
                if let Some((width, height)) = root.as_mut() {
                    match local.as_str() {
                        "width" => *width = value.as_str().to_owned(),
                        "height" => *height = value.as_str().to_owned(),
                        _ => {}
                    }
                }
            }
            Ok(xmlparser::Token::ElementEnd { .. }) if root.is_some() => break,
            Ok(_) => {}
            Err(_) => return ("0".into(), "0".into()),
        }
    }
    root.unwrap_or(("0".into(), "0".into()))
}

/// PHP's `intval()` of a string: the leading number, truncated.
fn intval(text: &str) -> i64 {
    let text = text.trim_start_matches([' ', '\t', '\n', '\r', '\x0B', '\x0C']);
    let end = text.char_indices().find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')) || c == '.' || c == 'e' || c == 'E')).map_or(text.len(), |(i, _)| i);
    let mut candidate = &text[..end];
    while !candidate.is_empty() {
        if let Ok(n) = candidate.parse::<f64>() {
            return n as i64;
        }
        candidate = &candidate[..candidate.len() - 1];
    }
    0
}

/// PHP's `round( $value, 2 )`.
fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// `get_option( $name )`: the string, or `false`.
fn option(site: &Site, name: &str) -> Value {
    site.options.get(name).map_or(Value::Bool(false), |v| Value::Str(v.as_bytes().to_vec()))
}

/// PHP's number of an option value in arithmetic (`false` is 0).
fn number(value: &Value) -> f64 {
    match value {
        Value::Str(s) => intval_float(&String::from_utf8_lossy(s)),
        Value::Int(n) => *n as f64,
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

fn intval_float(text: &str) -> f64 {
    text.trim().parse::<f64>().unwrap_or_else(|_| intval(text) as f64)
}

fn key(name: &str) -> Key {
    Key::Str(name.as_bytes().to_vec())
}

/// `bodhi_svgs_generate_svg_attachment_metadata()`.
pub(super) fn metadata(site: &Site, mime: &str, path: &Path, relative: &str, metadata: Value) -> Value {
    if mime != SVG {
        return metadata;
    }
    let (w, h) = dimensions(path);
    let (width, height) = (intval(&w), intval(&h));
    let filename = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut sizes = Vec::new();
    for (size, ..) in rustpress_content::media::registered_sizes(&site.options) {
        let mut entry: Vec<(Key, Value)> = Vec::new();
        if width != 0 && height != 0 {
            // `$_wp_additional_image_sizes` is not in scope there: every size reads its options.
            let size_w = option(site, &format!("{size}_size_w"));
            let new_height = if width > height {
                let ratio = round2(width as f64 / height as f64);
                (number(&size_w) / ratio).round()
            } else {
                let ratio = round2(height as f64 / width as f64);
                (number(&size_w) * ratio).round()
            };
            entry.push((key("width"), size_w));
            entry.push((key("height"), Value::Float(new_height)));
            entry.push((key("crop"), Value::Bool(false)));
        } else {
            entry.push((key("width"), option(site, &format!("{size}_size_w"))));
            entry.push((key("height"), option(site, &format!("{size}_size_h"))));
            entry.push((key("crop"), option(site, &format!("{size}_crop"))));
        }
        entry.push((key("file"), Value::Str(filename.as_bytes().to_vec())));
        entry.push((key("mime-type"), Value::Str(SVG.as_bytes().to_vec())));
        sizes.push((key(&size), Value::Array(entry)));
    }
    // `str_replace( $basedir, '', $file )`: the path below the uploads directory with its leading slash.
    Value::Array(vec![
        (key("width"), Value::Int(width)),
        (key("height"), Value::Int(height)),
        (key("file"), Value::Str(format!("/{}", relative.trim_start_matches('/')).into_bytes())),
        (key("sizes"), Value::Array(sizes)),
    ])
}

/// PHP 8's `$a > $b` for two strings: as numbers when both are numeric.
fn php_greater(a: &str, b: &str) -> bool {
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => x > y,
        _ => a > b,
    }
}

/// `bodhi_svgs_response_for_svg()`: an SVG without sizes gets the full size from its file.
pub(super) fn prepare_attachment_for_js(mime: &str, path: Option<&Path>, response: &mut serde_json::Map<String, serde_json::Value>) {
    if mime != SVG || response.get("sizes").is_some_and(|s| s.as_object().is_some_and(|o| !o.is_empty()) || s.as_array().is_some_and(|a| !a.is_empty())) {
        return;
    }
    let (width, height) = path.map_or(("0".into(), "0".into()), dimensions);
    let url = response.get("url").cloned().unwrap_or(serde_json::Value::Null);
    let orientation = if php_greater(&width, &height) { "landscape" } else { "portrait" };
    response.insert("sizes".into(), serde_json::json!({"full": {"url": url, "width": width, "height": height, "orientation": orientation}}));
}

/// `bodhi_svgs_final_output()`: the templates of the media library show SVGs from their URL.
pub(super) fn media_templates(html: String) -> String {
    html.replace(
        "<# } else if ( 'image' === data.type && data.sizes && data.sizes.full ) { #>",
        "<# } else if ( 'svg+xml' === data.subtype ) { #>\n\t\t\t\t\t<img class=\"details-image\" src=\"{{ data.url }}\" draggable=\"false\" />\n\t\t\t\t\t<# } else if ( 'image' === data.type && data.sizes && data.sizes.full ) { #>",
    )
    .replace(
        "<# } else if ( 'image' === data.type && data.sizes ) { #>",
        "<# } else if ( 'svg+xml' === data.subtype ) { #>\n\t\t\t\t\t<div class=\"centered\">\n\t\t\t\t\t\t<img src=\"{{ data.url }}\" class=\"thumbnail\" draggable=\"false\" />\n\t\t\t\t\t</div>\n\t\t\t\t\t<# } else if ( 'image' === data.type && data.sizes ) { #>",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn php_numbers() {
        assert_eq!(intval("100px"), 100);
        assert_eq!(intval(" 12.9"), 12);
        assert_eq!(intval(""), 0);
        assert_eq!(intval("1e3"), 1000);
        assert!(php_greater("100", "50"));
        assert!(!php_greater("", "0"));
    }

    #[test]
    fn svg_start_is_recognized() {
        assert!(looks_like_svg("<?xml version=\"1.0\"?>\n<!-- x -->\n<!DOCTYPE svg>\n<svg width=\"1\"></svg>"));
        assert!(!looks_like_svg("<html><svg></svg></html>"));
        assert!(!looks_like_svg("<svg>"));
    }
}
