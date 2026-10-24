//! Static files from the WordPress directory.
//!
//! Only plain asset types are served, and only from `wp-content/`, `wp-includes/`
//! and the document root itself. PHP, configs, logs, dotfiles and everything
//! else stay private.

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{HeaderValue, Request, Response, header};
use tower::ServiceExt;
use tower_http::services::ServeDir;

const EXTENSIONS: &[&str] = &[
    "css", "js", "mjs", "map", "png", "jpg", "jpeg", "gif", "webp", "avif", "svg", "ico", "bmp", "woff", "woff2", "ttf", "otf", "eot", "txt", "pdf",
    "mp3", "m4a", "ogg", "wav", "mp4", "m4v", "webm", "mov",
];

/// Whether a request path (percent-decoded, starting with `/`) names a servable file location.
pub fn is_static_candidate(path: &str) -> bool {
    let relative = path.trim_start_matches('/');
    if relative.is_empty() || relative.ends_with('/') {
        return false;
    }
    if relative.split('/').any(|segment| segment.is_empty() || segment.starts_with('.') || segment.contains('\\') || segment.contains('\0')) {
        return false;
    }
    let Some((_, extension)) = relative.rsplit_once('.') else { return false };
    if !EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()) {
        return false;
    }
    let top_level = !relative.contains('/');
    top_level || relative.starts_with("wp-content/") || relative.starts_with("wp-includes/")
}

#[derive(Clone)]
pub struct Statics {
    root: PathBuf,
    service: ServeDir,
}

impl Statics {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self { service: ServeDir::new(&root).append_index_html_on_directories(false), root }
    }

    /// Serves the file if it exists; `None` lets the request continue to WordPress routing.
    pub async fn serve(&self, request: Request<Body>) -> Option<Response<Body>> {
        let decoded = percent_decode(request.uri().path())?;
        if !is_static_candidate(&decoded) || !self.exists(&decoded) {
            return None;
        }
        let etag = self.etag(&decoded);
        if let Some(etag) = &etag
            && request.headers().get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()).is_some_and(|sent| sent.split(',').any(|candidate| candidate.trim().trim_start_matches("W/") == etag))
        {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = axum::http::StatusCode::NOT_MODIFIED;
            if let Ok(value) = HeaderValue::from_str(etag) {
                response.headers_mut().insert(header::ETAG, value);
            }
            return Some(response);
        }
        let response = self.service.clone().oneshot(request).await.ok()?;
        let mut response = response.map(Body::new);
        if response.status().is_success() {
            response.headers_mut().entry(header::CACHE_CONTROL).or_insert(HeaderValue::from_static("public, max-age=86400"));
            if let Some(value) = etag.as_deref().and_then(|e| HeaderValue::from_str(e).ok()) {
                response.headers_mut().insert(header::ETAG, value);
            }
        }
        Some(response)
    }

    /// Apache's default `ETag` (`FileETag MTime Size`): the size and the modification time in
    /// microseconds, both in hex, so clients and caches see what the reference server sent.
    fn etag(&self, decoded: &str) -> Option<String> {
        let metadata = std::fs::metadata(self.root.join(decoded.trim_start_matches('/'))).ok()?;
        let micros = metadata.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_micros();
        Some(format!("\"{:x}-{:x}\"", metadata.len(), micros))
    }

    fn exists(&self, decoded: &str) -> bool {
        let path: &Path = &self.root.join(decoded.trim_start_matches('/'));
        path.is_file()
    }
}

fn percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_assets_under_known_directories() {
        assert!(is_static_candidate("/wp-content/themes/twentytwentyfive/style.css"));
        assert!(is_static_candidate("/wp-includes/js/dist/script-modules/interactivity/index.min.js"));
        assert!(is_static_candidate("/wp-content/uploads/2024/01/photo.JPG"));
        assert!(is_static_candidate("/favicon.ico"));
    }

    #[test]
    fn code_configs_and_dotfiles_are_private() {
        for path in [
            "/wp-config.php",
            "/wp-content/uploads/evil.php",
            "/wp-content/debug.log",
            "/wp-content/themes/x/theme.json",
            "/wp-content/.htaccess",
            "/wp-content/uploads/.hidden/a.png",
            "/wp-admin/css/common.css",
            "/wp-content/../wp-config.php",
            "/wp-content//uploads/a.png",
            "/",
            "/wp-content/uploads/",
            "/ueber-uns/",
            "/readme",
        ] {
            assert!(!is_static_candidate(path), "{path} must not be served");
        }
    }

    #[test]
    fn percent_decoding() {
        assert_eq!(percent_decode("/a%20b.png").as_deref(), Some("/a b.png"));
        assert_eq!(percent_decode("/%2e%2e/x.png").as_deref(), Some("/../x.png"));
        assert_eq!(percent_decode("/bad%zz"), None);
    }
}
