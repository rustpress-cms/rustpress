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
    ("heif", "image/heif"),
    ("heics", "image/heic-sequence"),
    ("heifs", "image/heif-sequence"),
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
    ("docm", "application/vnd.ms-word.document.macroEnabled.12"),
    ("dotx", "application/vnd.openxmlformats-officedocument.wordprocessingml.template"),
    ("dotm", "application/vnd.ms-word.template.macroEnabled.12"),
    ("xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
    ("xlsm", "application/vnd.ms-excel.sheet.macroEnabled.12"),
    ("xlsb", "application/vnd.ms-excel.sheet.binary.macroEnabled.12"),
    ("xltx", "application/vnd.openxmlformats-officedocument.spreadsheetml.template"),
    ("xltm", "application/vnd.ms-excel.template.macroEnabled.12"),
    ("xlam", "application/vnd.ms-excel.addin.macroEnabled.12"),
    ("pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation"),
    ("pptm", "application/vnd.ms-powerpoint.presentation.macroEnabled.12"),
    ("ppsx", "application/vnd.openxmlformats-officedocument.presentationml.slideshow"),
    ("ppsm", "application/vnd.ms-powerpoint.slideshow.macroEnabled.12"),
    ("potx", "application/vnd.openxmlformats-officedocument.presentationml.template"),
    ("potm", "application/vnd.ms-powerpoint.template.macroEnabled.12"),
    ("ppam", "application/vnd.ms-powerpoint.addin.macroEnabled.12"),
    ("sldx", "application/vnd.openxmlformats-officedocument.presentationml.slide"),
    ("sldm", "application/vnd.ms-powerpoint.slide.macroEnabled.12"),
    ("onetoc|onetoc2|onetmp|onepkg", "application/onenote"),
    ("oxps", "application/oxps"),
    ("xps", "application/vnd.ms-xpsdocument"),
    ("odt", "application/vnd.oasis.opendocument.text"),
    ("odp", "application/vnd.oasis.opendocument.presentation"),
    ("ods", "application/vnd.oasis.opendocument.spreadsheet"),
    ("odg", "application/vnd.oasis.opendocument.graphics"),
    ("odc", "application/vnd.oasis.opendocument.chart"),
    ("odb", "application/vnd.oasis.opendocument.database"),
    ("odf", "application/vnd.oasis.opendocument.formula"),
    ("wp|wpd", "application/wordperfect"),
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

/// `wp_check_filetype( $filename, $mimes )` with a list of `(extensions, MIME type)`.
pub fn check_filetype_in(filename: &str, mimes: &[(String, String)]) -> Option<(String, String)> {
    let extension = filename.rsplit_once('.')?.1.to_lowercase();
    mimes.iter().find(|(exts, _)| exts.split('|').any(|e| e == extension)).map(|(_, mime)| (extension, mime.clone()))
}

/// `get_allowed_mime_types( $user )`: the core's types (without those that need `unfiltered_html` for a user
/// without it), then the plugins' `upload_mimes`.
pub fn allowed_mime_types(site: &rustpress_core::Site, user: &rustpress_auth::CurrentUser, rest: bool, media_form_nonce: bool) -> Vec<(String, String)> {
    let unfiltered = user.can("unfiltered_html");
    let core: Vec<(String, String)> = MIME_TYPES.iter().filter(|(_, mime)| unfiltered || !UNFILTERED.contains(mime)).map(|(exts, mime)| ((*exts).to_owned(), (*mime).to_owned())).collect();
    let can = |capability: &str| user.can(capability);
    let uploader = rustpress_core::native::Uploader { site, roles: &user.roles, can: &can, rest, media_form_nonce };
    site.plugins.upload_mimes(&uploader, core)
}

/// The part of `sanitize_file_name()` for names with several extensions: an inner one of two to five letters (and
/// an optional digit) that is no allowed type gets a `_`, so `x.php.jpg` is stored as `x.php_.jpg`.
pub fn mark_inner_extensions(name: &str, allowed: impl Fn(&str) -> bool) -> String {
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() <= 2 {
        return name.to_owned();
    }
    let mut out = parts[0].to_owned();
    for part in &parts[1..parts.len() - 1] {
        out.push('.');
        out.push_str(part);
        let letters = part.trim_end_matches(|c: char| c.is_ascii_digit());
        let shaped = (2..=5).contains(&letters.len()) && letters.chars().all(|c| c.is_ascii_alphabetic()) && part.len() - letters.len() <= 1;
        if shaped && !allowed(part) {
            out.push('_');
        }
    }
    out.push('.');
    out.push_str(parts[parts.len() - 1]);
    out
}

/// `wp_get_image_mime()`: the type of an image from its first bytes.
pub fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    let at = |offset: usize, magic: &[u8]| bytes.get(offset..offset + magic.len()) == Some(magic);
    if at(0, &[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if at(0, &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n']) {
        Some("image/png")
    } else if at(0, b"GIF87a") || at(0, b"GIF89a") {
        Some("image/gif")
    } else if at(0, b"RIFF") && at(8, b"WEBP") {
        Some("image/webp")
    } else if at(4, b"ftypavif") || at(4, b"ftypavis") {
        Some("image/avif")
    } else if at(4, b"ftypheic") || at(4, b"ftypheix") || at(4, b"ftypmif1") || at(4, b"ftypmsf1") {
        Some("image/heic")
    } else if at(0, b"BM") {
        Some("image/bmp")
    } else if at(0, &[b'I', b'I', b'*', 0]) || at(0, &[b'M', b'M', 0, b'*']) {
        Some("image/tiff")
    } else if at(0, &[0, 0, 1, 0]) {
        Some("image/vnd.microsoft.icon")
    } else {
        None
    }
}

/// `wp_check_filetype_and_ext()` for images: an image whose bytes are of another image type gets that extension,
/// a file named as an image that is none is refused. `None`: refused.
pub fn check_image_upload(name: &str, mime: &str, bytes: &[u8]) -> Option<(String, String)> {
    if !mime.starts_with("image/") {
        return Some((name.to_owned(), mime.to_owned()));
    }
    let real = image_mime(bytes)?;
    if real == mime || real == "image/heic" {
        return Some((name.to_owned(), mime.to_owned()));
    }
    let ext = match real {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/tiff" => "tif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        "image/vnd.microsoft.icon" => "ico",
        _ => return None,
    };
    let renamed = match name.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.{ext}"),
        None => format!("{name}.{ext}"),
    };
    let (_, new_mime) = check_filetype(&renamed)?;
    Some((renamed, new_mime))
}

/// What an upload request carries.
#[derive(Default)]
pub struct UploadedFile {
    pub name: String,
    pub bytes: Vec<u8>,
    /// Sent to the REST API.
    pub rest: bool,
    /// The request carries a valid `_wpnonce` for `media-form`.
    pub media_form_nonce: bool,
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

/// The largest width or height an image may have to be decoded. Together with the decoder's allocation limit
/// (512 MiB) this keeps a small file that claims huge dimensions (a decompression bomb) from taking the memory of
/// the server; GD in PHP fails on such images as well, at its memory limit.
const MAX_IMAGE_SIDE: u32 = 20_000;

/// Decodes the image at `path` (its format from the extension, like `image::open`) within the limits.
pub fn open_image(path: &Path) -> image::ImageResult<image::DynamicImage> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    let mut reader = image::ImageReader::open(path)?;
    reader.limits(limits);
    reader.decode()
}

/// Runs work that blocks (decoding, resizing and writing images) on the blocking threads instead of an async worker.
pub async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(work).await.unwrap_or_else(|e| std::panic::resume_unwind(e.into_panic()))
}

pub fn save_image(image: &image::DynamicImage, path: &Path, mime: &str) -> Result<(), UploadError> {
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
    generate_image_metadata_with(options, true, &[], uploads, relative, mime)
}

/// The sizes `WP_Site_Icon::additional_sizes()` adds: `site_icon-<size>` for every size below 512 (WordPress misspells
/// the key of the width, so the width is open and the height decides).
pub fn site_icon_sizes() -> Vec<(String, u32, u32, bool)> {
    [270u32, 192, 180, 64, 32].into_iter().map(|size| (format!("site_icon-{size}"), 0, size, true)).collect()
}

/// [`generate_image_metadata`] with more sizes than the ones of the site, or with those only (`registered` false: a site icon).
pub fn generate_image_metadata_with(options: &rustpress_core::options::Options, registered: bool, extra: &[(String, u32, u32, bool)], uploads: &Path, relative: &str, mime: &str) -> Option<(Value, String, Option<String>)> {
    if !matches!(mime, "image/jpeg" | "image/png" | "image/gif" | "image/webp") {
        return None;
    }
    let full = uploads.join(relative);
    let mut decoded = open_image(&full).ok()?;
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
    let site_sizes = if registered { registered_sizes(options) } else { Vec::new() };
    for (name, dest_w, dest_h, crop) in site_sizes.into_iter().chain(extra.iter().cloned()) {
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

/// `WP_Image_Editor::multi_resize()` for the sizes of the site: the files written next to `full` and their
/// entries for the metadata, in the order of the sizes. Sizes that would not be smaller than the image are skipped.
pub fn generate_sizes(options: &rustpress_core::options::Options, full: &Path, mime: &str) -> Option<(u32, u32, Vec<(String, Value)>)> {
    let decoded = open_image(full).ok()?;
    let (width, height) = (decoded.width(), decoded.height());
    let dir = full.parent()?.to_path_buf();
    let name = full.file_name()?.to_string_lossy().into_owned();
    let (stem, ext) = name.rsplit_once('.')?;
    let mut sizes = Vec::new();
    for (size, dest_w, dest_h, crop) in registered_sizes(options) {
        let Some((sx, sy, cw, ch, nw, nh)) = resize_dimensions(width, height, dest_w, dest_h, crop) else { continue };
        let file_name = format!("{stem}-{nw}x{nh}.{ext}");
        let target = dir.join(&file_name);
        let resized = decoded.crop_imm(sx, sy, cw, ch).resize_exact(nw, nh, image::imageops::FilterType::Lanczos3);
        if save_image(&resized, &target, mime).is_err() {
            continue;
        }
        let filesize = std::fs::metadata(&target).map(|m| m.len() as i64).unwrap_or(0);
        sizes.push((
            size,
            Value::Array(vec![entry("file", s(&file_name)), entry("width", Value::Int(i64::from(nw))), entry("height", Value::Int(i64::from(nh))), entry("mime-type", s(mime)), entry("filesize", Value::Int(filesize))]),
        ));
    }
    Some((width, height, sizes))
}

/// `WP_Image_Editor_GD::crop()` and `save()` as `wp_crop_image()` uses them: the part `(x, y, width, height)` of the
/// image at `src`, scaled to `dst_w` x `dst_h`, saved next to it as `cropped-<name>`. Returns the path.
pub fn crop_image(src: &Path, x: u32, y: u32, w: u32, h: u32, dst_w: u32, dst_h: u32, mime: &str) -> Result<PathBuf, UploadError> {
    let decoded = open_image(src).map_err(|e| UploadError(e.to_string()))?;
    let (w, h) = (w.min(decoded.width().saturating_sub(x)), h.min(decoded.height().saturating_sub(y)));
    if w == 0 || h == 0 {
        return Err(UploadError("empty crop".into()));
    }
    let (dst_w, dst_h) = (if dst_w == 0 { w } else { dst_w }, if dst_h == 0 { h } else { dst_h });
    let cropped = decoded.crop_imm(x, y, w, h).resize_exact(dst_w, dst_h, image::imageops::FilterType::Lanczos3);
    let dir = src.parent().ok_or_else(|| UploadError("no directory".into()))?;
    let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let target = dir.join(unique_filename(dir, &format!("cropped-{name}")));
    save_image(&cropped, &target, mime)?;
    Ok(target)
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
    let site = content.site;
    // `wp_handle_upload_prefilter` of the plugins comes before every check.
    let mut file = file;
    let can = |capability: &str| content.user.can(capability);
    let uploader = rustpress_core::native::Uploader { site, roles: &content.user.roles, can: &can, rest: file.rest, media_form_nonce: file.media_form_nonce };
    site.plugins.upload_prefilter(&uploader, &mut file.name, &mut file.bytes).map_err(UploadError)?;
    let mimes = allowed_mime_types(site, content.user, file.rest, file.media_form_nonce);
    if file.bytes.is_empty() {
        return Err(UploadError(rustpress_core::i18n::sprintf(&tr("File is empty. Please upload something more substantial. This error could also be caused by uploads being disabled in your php.ini or by %s being defined as smaller than %s in php.ini."), &["post_max_size", "upload_max_filesize"])));
    }
    let allowed = |ext: &str| check_filetype_in(&format!("x.{ext}"), &mimes).is_some();
    let name = mark_inner_extensions(&sanitize::file_name(&file.name), allowed);
    let refused = || UploadError(tr("Sorry, you are not allowed to upload this file type."));
    // A type the core refuses may still pass a plugin's `wp_check_filetype_and_ext`.
    let (name, mime) = match check_filetype_in(&name, &mimes).and_then(|(_, mime)| check_image_upload(&name, &mime, &file.bytes)) {
        Some(checked) => checked,
        None => site.plugins.accept_filetype(&name, &mimes).ok_or_else(refused)?,
    };
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
    let generated = {
        let (options, uploads, relative, mime) = (content.site.options.clone(), uploads.to_path_buf(), relative.clone(), mime.clone());
        blocking(move || generate_image_metadata(&options, &uploads, &relative, &mime)).await
    };
    let metadata = match generated {
        Some((meta, final_relative, _)) => {
            stored_relative = final_relative;
            meta
        }
        None => {
            let filesize = std::fs::metadata(&target).map(|m| m.len() as i64).unwrap_or(0);
            Value::Array(vec![entry("filesize", Value::Int(filesize))])
        }
    };
    let metadata = site.plugins.attachment_metadata(site, &mime, &target, &stored_relative, metadata);
    let w = content.writer;
    w.update_post_meta(id, "_wp_attached_file", &stored_relative).await.map_err(|e| UploadError(e.to_string()))?;
    let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&metadata)).into_owned();
    w.update_post_meta(id, "_wp_attachment_metadata", &serialized).await.map_err(|e| UploadError(e.to_string()))?;
    Ok(id)
}

#[cfg(test)]
mod upload_tests {
    use super::*;

    fn allowed(ext: &str) -> bool {
        check_filetype(&format!("x.{ext}")).is_some()
    }

    #[test]
    fn inner_extensions_get_an_underscore() {
        assert_eq!(mark_inner_extensions("x.php.jpg", allowed), "x.php_.jpg");
        assert_eq!(mark_inner_extensions("x.phtml5.jpg", allowed), "x.phtml5_.jpg");
        // Allowed types, numbers and long parts stay.
        assert_eq!(mark_inner_extensions("x.jpg.png", allowed), "x.jpg.png");
        assert_eq!(mark_inner_extensions("report.2024.pdf", allowed), "report.2024.pdf");
        assert_eq!(mark_inner_extensions("a.verylongpart.pdf", allowed), "a.verylongpart.pdf");
        assert_eq!(mark_inner_extensions("plain.jpg", allowed), "plain.jpg");
    }

    #[test]
    fn images_are_checked_by_their_bytes() {
        let png = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n', 0, 0];
        assert_eq!(image_mime(&png), Some("image/png"));
        assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(image_mime(b"<?php echo 1;"), None);
        // A PNG named .jpg is renamed, PHP named .jpg is refused, other files pass.
        assert_eq!(check_image_upload("a.jpg", "image/jpeg", &png), Some(("a.png".to_owned(), "image/png".to_owned())));
        assert_eq!(check_image_upload("a.jpg", "image/jpeg", b"<?php echo 1;"), None);
        assert_eq!(check_image_upload("a.pdf", "application/pdf", b"%PDF-1.7"), Some(("a.pdf".to_owned(), "application/pdf".to_owned())));
    }

    #[test]
    fn images_beyond_the_limits_are_not_decoded() {
        let dir = std::env::temp_dir().join(format!("rustpress-open-image-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (small, wide) = (dir.join("small.png"), dir.join("wide.png"));
        image::DynamicImage::new_luma8(40, 30).save(&small).unwrap();
        // A few hundred bytes that claim more pixels than any upload may have.
        image::DynamicImage::new_luma8(MAX_IMAGE_SIDE + 1, 1).save(&wide).unwrap();
        assert_eq!(open_image(&small).map(|i| (i.width(), i.height())).ok(), Some((40, 30)));
        assert!(open_image(&wide).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
