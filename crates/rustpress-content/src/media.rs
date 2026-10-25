//! Uploading files: `wp_handle_upload()`, `wp_insert_attachment()` and the metadata of
//! `wp_generate_attachment_metadata()` (image sizes).

use std::path::{Path, PathBuf};

use rustpress_phpser::{Key, Value};

use crate::content::{Content, ContentError, PostData};
use crate::sanitize;

/// The file types WordPress lets a user upload: `(extensions, MIME type)`.
pub const MIME_TYPES: &[(&str, &str)] = &[
    ("jpg|jpeg|jpe", "image/jpeg"),
    ("gif", "image/gif"),
    ("png", "image/png"),
    ("bmp", "image/bmp"),
    ("tiff|tif", "image/tiff"),
    ("webp", "image/webp"),
    ("avif", "image/avif"),
    ("ico", "image/x-icon"),
    ("heic", "image/heic"),
    ("asf|asx", "video/x-ms-asf"),
    ("wmv", "video/x-ms-wmv"),
    ("wmx", "video/x-ms-wmx"),
    ("wm", "video/x-ms-wm"),
    ("avi", "video/avi"),
    ("divx", "video/divx"),
    ("flv", "video/x-flv"),
    ("mov|qt", "video/quicktime"),
    ("mpeg|mpg|mpe", "video/mpeg"),
    ("mp4|m4v", "video/mp4"),
    ("ogv", "video/ogg"),
    ("webm", "video/webm"),
    ("mkv", "video/x-matroska"),
    ("3gp|3gpp", "video/3gpp"),
    ("3g2|3gp2", "video/3gpp2"),
    ("txt|asc|c|cc|h|srt", "text/plain"),
    ("csv", "text/csv"),
    ("tsv", "text/tab-separated-values"),
    ("ics", "text/calendar"),
    ("rtx", "text/richtext"),
    ("css", "text/css"),
    ("htm|html", "text/html"),
    ("vtt", "text/vtt"),
    ("dfxp", "application/ttaf+xml"),
    ("mp3|m4a|m4b", "audio/mpeg"),
    ("aac", "audio/aac"),
    ("ra|ram", "audio/x-realaudio"),
    ("wav|x-wav", "audio/wav"),
    ("ogg|oga", "audio/ogg"),
    ("flac", "audio/flac"),
    ("mid|midi", "audio/midi"),
    ("wma", "audio/x-ms-wma"),
    ("wax", "audio/x-ms-wax"),
    ("mka", "audio/x-matroska"),
    ("rtf", "application/rtf"),
    ("js", "application/javascript"),
    ("pdf", "application/pdf"),
    ("class", "application/java"),
    ("tar", "application/x-tar"),
    ("zip", "application/zip"),
    ("gz|gzip", "application/x-gzip"),
    ("rar", "application/rar"),
    ("7z", "application/x-7z-compressed"),
    ("psd", "application/octet-stream"),
    ("xcf", "application/octet-stream"),
    ("doc", "application/msword"),
    ("pot|pps|ppt", "application/vnd.ms-powerpoint"),
    ("wri", "application/vnd.ms-write"),
    ("xla|xls|xlt|xlw", "application/vnd.ms-excel"),
    ("mdb", "application/vnd.ms-access"),
    ("mpp", "application/vnd.ms-project"),
    ("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
    ("xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
    ("pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation"),
    ("odt", "application/vnd.oasis.opendocument.text"),
    ("odp", "application/vnd.oasis.opendocument.presentation"),
    ("ods", "application/vnd.oasis.opendocument.spreadsheet"),
    ("odg", "application/vnd.oasis.opendocument.graphics"),
    ("odc", "application/vnd.oasis.opendocument.chart"),
    ("odb", "application/vnd.oasis.opendocument.database"),
    ("odf", "application/vnd.oasis.opendocument.formula"),
    ("key", "application/vnd.apple.keynote"),
    ("numbers", "application/vnd.apple.numbers"),
    ("pages", "application/vnd.apple.pages"),
];

/// Types only users with `unfiltered_html` may upload.
const UNFILTERED: [&str; 4] = ["text/html", "application/javascript", "text/css", "application/java"];

/// `wp_check_filetype()`: `(extension, MIME type)`.
pub fn check_filetype(filename: &str) -> Option<(String, String)> {
    let extension = filename.rsplit_once('.')?.1.to_lowercase();
    MIME_TYPES.iter().find(|(exts, _)| exts.split('|').any(|e| e == extension)).map(|(_, mime)| (extension, (*mime).to_owned()))
}

/// What an upload request carries.
pub struct UploadedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// An upload that could not be stored, with the text WordPress shows.
#[derive(Debug)]
pub struct UploadError(pub String);

/// `wp_unique_filename()`: `name.ext`, `name-1.ext`, ... within the directory.
pub fn unique_filename(dir: &Path, name: &str) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) => (stem.to_owned(), format!(".{ext}")),
        None => (name.to_owned(), String::new()),
    };
    let mut candidate = format!("{stem}{ext}");
    let mut number = 1;
    // A size file of the same name also counts as taken (`name-150x150.jpg`).
    while dir.join(&candidate).exists() {
        candidate = format!("{stem}-{number}{ext}");
        number += 1;
    }
    candidate
}

fn s(text: &str) -> Value {
    Value::Str(text.as_bytes().to_vec())
}

fn k(text: &str) -> Key {
    Key::Str(text.as_bytes().to_vec())
}

fn entry(key: &str, value: Value) -> (Key, Value) {
    (k(key), value)
}

/// `image_resize_dimensions()` with centered crops: `(source x, y, width, height, new width, new height)`.
fn resize_dimensions(orig_w: u32, orig_h: u32, dest_w: u32, dest_h: u32, crop: bool) -> Option<(u32, u32, u32, u32, u32, u32)> {
    if orig_w == 0 || orig_h == 0 || (dest_w == 0 && dest_h == 0) {
        return None;
    }
    let (new_w, new_h, crop_w, crop_h, s_x, s_y);
    if crop {
        let aspect = f64::from(orig_w) / f64::from(orig_h);
        let mut nw = dest_w.min(orig_w);
        let mut nh = dest_h.min(orig_h);
        if nw == 0 {
            nw = (f64::from(nh) * aspect).round() as u32;
        }
        if nh == 0 {
            nh = (f64::from(nw) / aspect).round() as u32;
        }
        let ratio = (f64::from(nw) / f64::from(orig_w)).max(f64::from(nh) / f64::from(orig_h));
        crop_w = (f64::from(nw) / ratio).round() as u32;
        crop_h = (f64::from(nh) / ratio).round() as u32;
        s_x = (f64::from(orig_w.saturating_sub(crop_w)) / 2.0).floor() as u32;
        s_y = (f64::from(orig_h.saturating_sub(crop_h)) / 2.0).floor() as u32;
        new_w = nw;
        new_h = nh;
    } else {
        crop_w = orig_w;
        crop_h = orig_h;
        s_x = 0;
        s_y = 0;
        let (w, h) = rustpress_blocks::images::constrain_dimensions(u64::from(orig_w), u64::from(orig_h), u64::from(dest_w), u64::from(dest_h));
        new_w = w as u32;
        new_h = h as u32;
    }
    let fuzzy = |a: u32, b: u32| (i64::from(a) - i64::from(b)).abs() <= 1;
    if fuzzy(new_w, orig_w) && fuzzy(new_h, orig_h) {
        return None;
    }
    Some((s_x, s_y, crop_w, crop_h, new_w.max(1), new_h.max(1)))
}

fn save_image(image: &image::DynamicImage, path: &Path, mime: &str) -> Result<(), UploadError> {
    let result = match mime {
        "image/jpeg" => {
            let mut out = std::fs::File::create(path).map_err(|e| UploadError(e.to_string()))?;
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 82);
            image.to_rgb8().write_with_encoder(encoder)
        }
        "image/png" => image.save_with_format(path, image::ImageFormat::Png),
        "image/gif" => image.save_with_format(path, image::ImageFormat::Gif),
        "image/webp" => image.save_with_format(path, image::ImageFormat::WebP),
        _ => return Ok(()),
    };
    result.map_err(|e| UploadError(e.to_string()))
}

/// The sizes of the site: `(name, width, height, crop)`.
pub fn registered_sizes(options: &rustpress_core::options::Options) -> Vec<(String, u32, u32, bool)> {
    let number = |name: &str, default: u64| options.u64(name, default) as u32;
    vec![
        ("thumbnail".into(), number("thumbnail_size_w", 150), number("thumbnail_size_h", 150), options.get("thumbnail_crop").is_some_and(|v| !v.is_empty() && v != "0")),
        ("medium".into(), number("medium_size_w", 300), number("medium_size_h", 300), false),
        ("medium_large".into(), number("medium_large_size_w", 768), number("medium_large_size_h", 0), false),
        ("large".into(), number("large_size_w", 1024), number("large_size_h", 1024), false),
        ("1536x1536".into(), 1536, 1536, false),
        ("2048x2048".into(), 2048, 2048, false),
    ]
}

/// The metadata of an uploaded image: sizes written next to the file. Returns `(metadata, final relative path)`.
pub fn generate_image_metadata(options: &rustpress_core::options::Options, uploads: &Path, relative: &str, mime: &str) -> Option<(Value, String, Option<String>)> {
    if !matches!(mime, "image/jpeg" | "image/png" | "image/gif" | "image/webp") {
        return None;
    }
    let full = uploads.join(relative);
    let mut decoded = image::open(&full).ok()?;
    let (mut width, mut height) = (decoded.width(), decoded.height());
    let mut relative = relative.to_owned();
    let mut original_image: Option<String> = None;
    // `big_image_size_threshold`: very large images are scaled down; the original stays.
    if width > 2560 || height > 2560 {
        let (w, h) = rustpress_blocks::images::constrain_dimensions(u64::from(width), u64::from(height), 2560, 2560);
        let scaled = decoded.resize_exact(w as u32, h as u32, image::imageops::FilterType::Lanczos3);
        let (stem, ext) = relative.rsplit_once('.').map(|(a, b)| (a.to_owned(), b.to_owned()))?;
        let scaled_rel = format!("{stem}-scaled.{ext}");
        if save_image(&scaled, &uploads.join(&scaled_rel), mime).is_ok() {
            original_image = full.file_name().map(|n| n.to_string_lossy().into_owned());
            relative = scaled_rel;
            decoded = scaled;
            width = w as u32;
            height = h as u32;
        }
    }
    let full = uploads.join(&relative);
    let dir = full.parent()?.to_path_buf();
    let (stem, ext) = {
        let name = full.file_name()?.to_string_lossy().into_owned();
        let (a, b) = name.rsplit_once('.')?;
        (a.to_owned(), b.to_owned())
    };
    let mut sizes: Vec<(Key, Value)> = Vec::new();
    let mut made: Vec<(u32, u32)> = Vec::new();
    for (name, dest_w, dest_h, crop) in registered_sizes(options) {
        let Some((sx, sy, cw, ch, nw, nh)) = resize_dimensions(width, height, dest_w, dest_h, crop) else { continue };
        if made.contains(&(nw, nh)) && name != "thumbnail" {
            // Same dimensions as a size already written: WordPress reuses the file.
        }
        let file_name = format!("{stem}-{nw}x{nh}.{ext}");
        let target = dir.join(&file_name);
        let cropped = decoded.crop_imm(sx, sy, cw, ch);
        let resized = cropped.resize_exact(nw, nh, image::imageops::FilterType::Lanczos3);
        if save_image(&resized, &target, mime).is_err() {
            continue;
        }
        made.push((nw, nh));
        let filesize = std::fs::metadata(&target).map(|m| m.len() as i64).unwrap_or(0);
        sizes.push(entry(
            &name,
            Value::Array(vec![entry("file", s(&file_name)), entry("width", Value::Int(i64::from(nw))), entry("height", Value::Int(i64::from(nh))), entry("mime-type", s(mime)), entry("filesize", Value::Int(filesize))]),
        ));
    }
    let image_meta = Value::Array(vec![
        entry("aperture", s("0")),
        entry("credit", s("")),
        entry("camera", s("")),
        entry("caption", s("")),
        entry("created_timestamp", s("0")),
        entry("copyright", s("")),
        entry("focal_length", s("0")),
        entry("iso", s("0")),
        entry("shutter_speed", s("0")),
        entry("title", s("")),
        entry("orientation", s("0")),
        entry("keywords", Value::Array(Vec::new())),
    ]);
    let filesize = std::fs::metadata(&full).map(|m| m.len() as i64).unwrap_or(0);
    let mut items = vec![entry("width", Value::Int(i64::from(width))), entry("height", Value::Int(i64::from(height))), entry("file", s(&relative)), entry("filesize", Value::Int(filesize)), entry("sizes", Value::Array(sizes)), entry("image_meta", image_meta)];
    if let Some(original) = &original_image {
        items.push(entry("original_image", s(original)));
    }
    Some((Value::Array(items), relative, original_image))
}

/// The directory for new uploads: `Y/m` unless the site stores them flat.
pub fn upload_subdir(content: &Content<'_>) -> String {
    if content.site.options.get("uploads_use_yearmonth_folders") == Some("1") {
        content.site.date("Y/m", content.now)
    } else {
        String::new()
    }
}

/// Stores the file and makes the attachment: `media_handle_upload()`. Returns the attachment ID.
pub async fn handle_upload(content: &Content<'_>, uploads: &Path, file: UploadedFile, post_parent: u64) -> Result<u64, UploadError> {
    let tr = |t: &str| content.site.tr(t);
    if file.bytes.is_empty() {
        return Err(UploadError(rustpress_core::i18n::sprintf(&tr("File is empty. Please upload something more substantial. This error could also be caused by uploads being disabled in your php.ini or by %s being defined as smaller than %s in php.ini."), &["post_max_size", "upload_max_filesize"])));
    }
    let name = sanitize::file_name(&file.name);
    let Some((_ext, mime)) = check_filetype(&name) else {
        return Err(UploadError(tr("Sorry, you are not allowed to upload this file type.")));
    };
    if UNFILTERED.contains(&mime.as_str()) && !content.user.can("unfiltered_html") {
        return Err(UploadError(tr("Sorry, you are not allowed to upload this file type.")));
    }
    let subdir = upload_subdir(content);
    let dir: PathBuf = if subdir.is_empty() { uploads.to_path_buf() } else { uploads.join(&subdir) };
    std::fs::create_dir_all(&dir).map_err(|e| UploadError(e.to_string()))?;
    let unique = unique_filename(&dir, &name);
    let target = dir.join(&unique);
    std::fs::write(&target, &file.bytes).map_err(|e| UploadError(e.to_string()))?;
    let relative = if subdir.is_empty() { unique.clone() } else { format!("{subdir}/{unique}") };
    let base_url = rustpress_blocks::images::upload_baseurl(content.site);
    let url = format!("{}/{relative}", base_url.trim_end_matches('/'));
    let title = {
        let stem = unique.rsplit_once('.').map_or(unique.as_str(), |(a, _)| a);
        stem.to_owned()
    };
    let data = PostData {
        post_title: Some(title),
        post_content: Some(String::new()),
        post_status: Some("inherit".into()),
        post_type: Some("attachment".into()),
        post_mime_type: Some(mime.clone()),
        post_parent: Some(post_parent),
        guid: Some(url),
        post_author: Some(content.user.id),
        comment_status: Some(content.site.options.string("default_comment_status", "closed")),
        ping_status: Some("closed".into()),
        ..Default::default()
    };
    let id = content.insert_post(&data).await.map_err(|e| UploadError(match e {
        ContentError::Message(m) => m,
        other => other.to_string(),
    }))?;
    let mut stored_relative = relative.clone();
    let metadata = match generate_image_metadata(&content.site.options, uploads, &relative, &mime) {
        Some((meta, final_relative, _)) => {
            stored_relative = final_relative;
            meta
        }
        None => {
            let filesize = std::fs::metadata(&target).map(|m| m.len() as i64).unwrap_or(0);
            Value::Array(vec![entry("filesize", Value::Int(filesize))])
        }
    };
    let w = content.writer;
    w.update_post_meta(id, "_wp_attached_file", &stored_relative).await.map_err(|e| UploadError(e.to_string()))?;
    let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&metadata)).into_owned();
    w.update_post_meta(id, "_wp_attachment_metadata", &serialized).await.map_err(|e| UploadError(e.to_string()))?;
    Ok(id)
}
