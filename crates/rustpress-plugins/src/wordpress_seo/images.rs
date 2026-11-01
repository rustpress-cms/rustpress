//! Images: `WPSEO_Image_Utils`, `Image_Helper`, `WPSEO_Content_Images` and the WordPress functions they call.

use std::sync::LazyLock;

use rustpress_blocks::images::{Metadata, attachment_url, image_downsize, php_int, upload_baseurl};
use rustpress_core::Result;
use rustpress_db::Post;
use serde_json::Value;

use super::Page;

/// An attachment with its metadata.
pub struct Attachment {
    pub post: Post,
    pub meta: Option<Metadata>,
    /// `_wp_attached_file`.
    pub file: Option<String>,
}

/// `get_post( $id )` of an attachment.
pub async fn attachment(page: &Page<'_>, id: u64) -> Result<Option<Attachment>> {
    if id == 0 {
        return Ok(None);
    }
    let Some(post) = page.site.db.post(id).await? else { return Ok(None) };
    if post.post_type != "attachment" {
        return Ok(Some(Attachment { post, meta: None, file: None }));
    }
    let (meta, file) = rustpress_blocks::images::load_metadata(page.site, &post).await?;
    Ok(Some(Attachment { post, meta, file }))
}

impl Attachment {
    /// `wp_get_attachment_url()`.
    pub fn url(&self, page: &Page<'_>) -> Option<String> {
        if self.post.post_type != "attachment" {
            return None;
        }
        Some(match self.file.as_deref().filter(|f| !f.is_empty()) {
            Some(file) => attachment_url(page.site, file),
            None => self.post.guid.clone(),
        })
    }

    /// `wp_attachment_is_image()`: an image type and a file with an image extension.
    pub fn is_image(&self) -> bool {
        let file = self.file.as_deref().unwrap_or(&self.post.guid);
        let ext = file.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
        let image_ext = ["jpg", "jpeg", "jpe", "gif", "png", "bmp", "tif", "tiff", "ico", "heic", "heif", "webp", "avif"].contains(&ext.as_str());
        self.post.post_type == "attachment" && image_ext && self.post.post_mime_type.starts_with("image/")
    }

    /// `Image_Helper::is_valid_attachment()`: an image of a type social networks show.
    pub fn is_valid(&self) -> bool {
        self.is_image() && ["image/jpeg", "image/gif", "image/png", "image/webp"].contains(&self.post.post_mime_type.as_str())
    }

    /// `wp_get_attachment_image_src( $id, $size )`: the URL, width and height.
    pub fn image_src(&self, page: &Page<'_>, size: &str) -> Option<(String, u64, u64)> {
        let url = self.url(page)?;
        match &self.meta {
            Some(meta) => {
                let d = image_downsize(&url, meta, size);
                Some((d.url, d.width, d.height))
            }
            // Without metadata only images have a source.
            None if self.is_image() => Some((url, 0, 0)),
            None => None,
        }
    }

    /// `get_attached_file()`: the path of the file on disk.
    fn path(&self, page: &Page<'_>) -> Option<std::path::PathBuf> {
        let file = self.file.as_deref().filter(|f| !f.is_empty())?;
        Some(page.uploads_dir().join(file.trim_start_matches('/')))
    }

    /// `wp_get_attachment_caption()`, else the alternative text (`Image_Helper::get_caption()`).
    pub async fn caption(&self, page: &Page<'_>) -> Result<String> {
        if !self.post.post_excerpt.is_empty() && self.post.post_type == "attachment" {
            return Ok(self.post.post_excerpt.clone());
        }
        self.alt(page).await
    }

    /// `_wp_attachment_image_alt`.
    pub async fn alt(&self, page: &Page<'_>) -> Result<String> {
        Ok(page.site.db.post_meta_value(self.post.id, "_wp_attachment_image_alt").await?.unwrap_or_default())
    }
}

/// `WPSEO_Image_Utils::get_data()`: an image as the plugin keeps it.
#[derive(Debug, Clone, Default)]
pub struct ImageData {
    pub url: String,
    pub width: u64,
    pub height: u64,
    /// The MIME type (`type`).
    pub mime: String,
    pub filesize: Option<u64>,
    pub path: Option<std::path::PathBuf>,
}

/// `WPSEO_Image_Utils::get_image( $id, $size )`.
async fn get_image(page: &Page<'_>, a: &Attachment, size: &str) -> Option<ImageData> {
    let meta = a.meta.as_ref();
    let mut image = None;
    if size == "full"
        && let Some(meta) = meta
    {
        image = Some(ImageData {
            url: a.image_src(page, "full").map(|s| s.0).unwrap_or_default(),
            width: meta.width(),
            height: meta.height(),
            mime: meta.0.get("type").and_then(Value::as_str).unwrap_or_default().to_owned(),
            filesize: meta.0.get("filesize").map(php_int),
            path: a.path(page),
        });
    }
    if image.is_none()
        && let Some(entry) = meta.and_then(|m| m.sizes()).and_then(|s| s.get(size)).and_then(Value::as_object)
        && let Some(file) = entry.get("file").and_then(Value::as_str)
    {
        // `image_get_intermediate_size()`.
        let full = a.url(page).unwrap_or_default();
        let dir_url = full.rsplit_once('/').map_or("", |(d, _)| d);
        image = Some(ImageData {
            url: format!("{dir_url}/{file}"),
            width: entry.get("width").map_or(0, php_int),
            height: entry.get("height").map_or(0, php_int),
            mime: String::new(),
            filesize: entry.get("filesize").map(php_int),
            path: a.path(page).and_then(|p| p.parent().map(|d| d.join(file))),
        });
    }
    if image.is_none()
        && let Some((url, width, height)) = a.image_src(page, size)
    {
        image = Some(ImageData { url, width, height, ..ImageData::default() });
    }
    let mut image = image?;
    if image.width == 0 || image.height == 0 {
        return None;
    }
    if image.mime.is_empty() {
        image.mime = a.post.post_mime_type.clone();
    }
    Some(image)
}

/// `Image_Helper::get_best_attachment_variation()`: the first of `full`, `large` and `medium_large` that is at
/// most 2 MB.
pub async fn best_variation(page: &Page<'_>, a: &Attachment) -> Option<ImageData> {
    for size in ["full", "large", "medium_large"] {
        let Some(image) = get_image(page, a, size).await else { continue };
        let usable = match (&image.filesize, &image.path) {
            (Some(size), _) => *size <= 2_097_152,
            (None, Some(path)) => std::fs::metadata(path).map_or(0, |m| m.len()) <= 2_097_152,
            (None, None) => true,
        };
        // `filter_usable_file_size()` only looks at the first variation that exists.
        return usable.then_some(image);
    }
    None
}

/// `Open_Graph\Image_Helper::get_image_by_id()`.
pub async fn open_graph_image(page: &Page<'_>, id: u64) -> Result<Option<ImageData>> {
    let Some(a) = attachment(page, id).await? else { return Ok(None) };
    if !a.is_valid() {
        return Ok(None);
    }
    Ok(best_variation(page, &a).await)
}

/// `Twitter\Image_Helper::get_by_id()`.
pub async fn twitter_image(page: &Page<'_>, id: u64) -> Result<String> {
    let Some(a) = attachment(page, id).await? else { return Ok(String::new()) };
    if !a.is_valid() {
        return Ok(String::new());
    }
    Ok(a.image_src(page, "full").map(|s| s.0).unwrap_or_default())
}

/// `wp_get_attachment_image_url( $id, 'full' )`.
pub async fn attachment_image_url(page: &Page<'_>, id: u64) -> Result<String> {
    Ok(attachment(page, id).await?.and_then(|a| a.image_src(page, "full")).map(|s| s.0).unwrap_or_default())
}

/// `WPSEO_Image_Utils::get_attachment_by_url()`: the attachment of an upload URL (sizes stripped), or 0.
pub async fn attachment_by_url(page: &Page<'_>, url: &str) -> Result<u64> {
    static SIZED: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"^(.*)-\d+x\d+\.(jpg|png|gif)$").expect("valid pattern"));
    let url = SIZED.replace(url, "$1.$2").into_owned();
    let base = upload_baseurl(page.site);
    if !url.starts_with(&base) {
        return Ok(0);
    }
    let id = attachment_url_to_postid(page, &url).await?;
    if id != 0 {
        return Ok(id);
    }
    // `get_scaled_image_id()`.
    let (dir, name) = url.rsplit_once('/').unwrap_or(("", &url));
    if let Some((stem, ext)) = name.rsplit_once('.').filter(|(stem, _)| !stem.is_empty()) {
        return attachment_url_to_postid(page, &format!("{dir}/{stem}-scaled.{ext}")).await;
    }
    Ok(0)
}

/// `Image_Helper::get_attachment_by_url()` with the default options: the SEO links table first (when the plugin
/// saved the post's links), else [`attachment_by_url`].
pub async fn attachment_by_url_linked(page: &Page<'_>, url: &str) -> Result<u64> {
    let parsed_external = match url.split_once("://") {
        Some((scheme, rest)) => {
            let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
            let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
            let host = host.split(':').next().unwrap_or_default();
            !matches!(scheme, "http" | "https") || host != page.home_host()
        }
        None => false,
    };
    if parsed_external {
        return Ok(0);
    }
    if !page.options.truthy("disable-attachment") {
        return attachment_by_url(page, url).await;
    }
    if let Some(target) = page.site.db.seo_link_target(url).await? {
        return Ok(target);
    }
    attachment_by_url(page, url).await
}

/// `attachment_url_to_postid()`.
async fn attachment_url_to_postid(page: &Page<'_>, url: &str) -> Result<u64> {
    let base = upload_baseurl(page.site);
    let mut path = url.to_owned();
    let site_scheme = base.split("://").next().unwrap_or_default();
    if let Some((scheme, _)) = url.split_once("://")
        && scheme != site_scheme
    {
        path = path.replace(scheme, site_scheme);
    }
    if let Some(rest) = path.strip_prefix(&format!("{base}/")) {
        path = rest.to_owned();
    }
    let rows = page.site.db.attachments_by_file(&path).await?;
    let Some(first) = rows.first() else { return Ok(0) };
    if rows.len() > 1
        && let Some((id, _)) = rows.iter().find(|(_, v)| *v == path)
    {
        return Ok(*id);
    }
    Ok(first.0)
}

/// `WPSEO_Content_Images::get_images_from_content()`, first image: the first `src` of an `<img>` that is a URL.
pub fn first_content_image(content: &str) -> Option<String> {
    static IMG: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"<img [^>]+>").expect("valid pattern"));
    static SRC: LazyLock<fancy_regex::Regex> = LazyLock::new(|| fancy_regex::Regex::new(r#"src=(["'])(.*?)\1"#).expect("valid pattern"));
    if !content.contains("<img") {
        return None;
    }
    IMG.find_iter(content).find_map(|tag| {
        let caps = SRC.captures(tag.as_str()).ok().flatten()?;
        let src = caps.get(2)?.as_str();
        (!src.is_empty() && filter_validate_url(src)).then(|| src.to_owned())
    })
}

/// `filter_var( $url, FILTER_VALIDATE_URL )`: a scheme, a host for the web schemes and no characters outside of
/// ASCII, spaces or control characters.
pub fn filter_validate_url(url: &str) -> bool {
    if !url.is_ascii() || url.bytes().any(|b| b <= b' ' || b == 0x7f) {
        return false;
    }
    let Some((scheme, rest)) = url.split_once(':') else { return false };
    let scheme_ok = scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    if !scheme_ok {
        return false;
    }
    if matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        let Some(authority) = rest.strip_prefix("//") else { return false };
        let host = authority.split(['/', '?', '#']).next().unwrap_or_default();
        let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
        let host = host.split(':').next().unwrap_or_default();
        return !host.is_empty() && host.split('.').all(|label| !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !label.starts_with('-') && !label.ends_with('-'));
    }
    true
}
