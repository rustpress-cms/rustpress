//! End-to-end checks of the HTTP layer against the reference WordPress
//! (`dev/up.sh`). Skips when the stack is not running.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use rustpress_server::{App, Config};
use tower::ServiceExt;

const DEFAULT_ROOT: &str = r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data";

struct Reference {
    status: u16,
    body: Vec<u8>,
}

fn fetch(path: &str) -> std::io::Result<Reference> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost:8080\r\nConnection: close\r\n\r\n")?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap_or(0);
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    Ok(Reference { status, body: raw[(split + 4).min(raw.len())..].to_vec() })
}

async fn app() -> Option<std::sync::Arc<App>> {
    let root = PathBuf::from(std::env::var("RUSTPRESS_WP_ROOT").unwrap_or_else(|_| DEFAULT_ROOT.into()));
    if !root.join("wp-includes").is_dir() || fetch("/").is_err() {
        eprintln!("reference stack not available, skipping");
        return None;
    }
    let config = Config {
        database_url: std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into()),
        wp_root: root,
        ..Config::default()
    };
    App::new(config).await.ok()
}

async fn get(app: &std::sync::Arc<App>, method: Method, path: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let request = Request::builder().method(method).uri(path).body(Body::empty()).unwrap();
    let response = app.router().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    (parts.status, parts.headers, body.collect().await.unwrap().to_bytes().to_vec())
}

#[tokio::test]
async fn pages_have_wordpress_status_and_type() {
    let Some(app) = app().await else { return };
    for path in ["/", "/ueber-uns/", "/testbeitrag-1/", "/gibt-es-nicht/", "/?s=test", "/category/uncategorized/"] {
        let (status, headers, body) = get(&app, Method::GET, path).await;
        assert_eq!(status.as_u16(), fetch(path).unwrap().status, "{path}");
        assert_eq!(headers[header::CONTENT_TYPE], "text/html; charset=UTF-8", "{path}");
        assert!(String::from_utf8(body).unwrap().starts_with("<!DOCTYPE html>"), "{path}");
    }
}

#[tokio::test]
async fn head_has_no_body() {
    let Some(app) = app().await else { return };
    let (status, headers, body) = get(&app, Method::HEAD, "/ueber-uns/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers.contains_key(header::CONTENT_TYPE));
    assert!(body.is_empty());
}

#[tokio::test]
async fn static_assets_are_byte_identical() {
    let Some(app) = app().await else { return };
    for path in [
        "/wp-includes/blocks/navigation/style.min.css",
        "/wp-content/themes/twentytwentyfive/assets/fonts/manrope/Manrope-VariableFont_wght.woff2",
        "/wp-includes/js/dist/script-modules/interactivity/index.min.js",
    ] {
        let (status, headers, body) = get(&app, Method::GET, path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body, fetch(path).unwrap().body, "{path}");
        assert!(headers.contains_key(header::CONTENT_TYPE), "{path}");
        assert!(headers.contains_key(header::CACHE_CONTROL), "{path}");
    }
}

#[tokio::test]
async fn private_files_are_not_served() {
    let Some(app) = app().await else { return };
    for path in ["/wp-config.php", "/wp-content/themes/twentytwentyfive/theme.json", "/wp-content/index.php", "/wp-content/%2e%2e/wp-config.php", "/.htaccess"] {
        let (status, _, body) = get(&app, Method::GET, path).await;
        let text = String::from_utf8_lossy(&body);
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(!text.contains("DB_PASSWORD") && !text.contains("<?php"), "{path} leaked source");
    }
}

#[tokio::test]
async fn writes_are_rejected() {
    let Some(app) = app().await else { return };
    let request = Request::builder().method(Method::POST).uri("/wp-comments-post.php").body(Body::empty()).unwrap();
    let response = app.router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(response.headers()[header::ALLOW], "GET, HEAD");
}

#[tokio::test]
async fn canonical_redirects_are_relative_301s() {
    let Some(app) = app().await else { return };
    for (from, to) in [("/testbeitrag-1", "/testbeitrag-1/"), ("/?p=5", "/testbeitrag-1/"), ("/hello-wor", "/hello-world/"), ("/?page_id=20", "/ueber-uns/"), ("/?cat=1&x=1", "/category/uncategorized/?x=1")] {
        let (status, headers, _) = get(&app, Method::GET, from).await;
        assert_eq!(status, StatusCode::MOVED_PERMANENTLY, "{from}");
        assert_eq!(headers[header::LOCATION], to, "{from}");
    }
    let (status, headers, _) = get(&app, Method::GET, "/ueber-uns/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!headers.contains_key(header::LOCATION));
}

#[tokio::test]
async fn feeds_are_served_like_wordpress() {
    let Some(app) = app().await else { return };
    for (path, content_type) in [("/feed/", "application/rss+xml; charset=UTF-8"), ("/testbeitrag-1/feed/atom/", "application/atom+xml; charset=UTF-8")] {
        let (status, headers, body) = get(&app, Method::GET, path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(headers[header::CONTENT_TYPE], content_type, "{path}");
        assert_eq!(body, fetch(path).unwrap().body, "{path}");
        assert!(headers.contains_key(header::ETAG) && headers.contains_key(header::LAST_MODIFIED), "{path}");
    }
}

#[tokio::test]
async fn feeds_support_conditional_get() {
    let Some(app) = app().await else { return };
    let (_, headers, _) = get(&app, Method::GET, "/feed/").await;
    let (etag, modified) = (headers[header::ETAG].clone(), headers[header::LAST_MODIFIED].clone());
    for (name, value) in [(header::IF_NONE_MATCH, etag), (header::IF_MODIFIED_SINCE, modified)] {
        let request = Request::builder().uri("/feed/").header(name, value).body(Body::empty()).unwrap();
        let response = app.router().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert!(response.into_body().collect().await.unwrap().to_bytes().is_empty());
    }
    let request = Request::builder().uri("/feed/").header(header::IF_NONE_MATCH, "\"nope\"").body(Body::empty()).unwrap();
    assert_eq!(app.router().oneshot(request).await.unwrap().status(), StatusCode::OK);
}

#[tokio::test]
async fn unsupported_feed_types_are_not_found() {
    let Some(app) = app().await else { return };
    let (status, _, _) = get(&app, Method::GET, "/feed/rdf/").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sitemaps_and_robots_are_byte_identical() {
    let Some(app) = app().await else { return };
    for path in [
        "/wp-sitemap.xml",
        "/wp-sitemap-posts-post-1.xml",
        "/wp-sitemap-posts-page-1.xml",
        "/wp-sitemap-taxonomies-category-1.xml",
        "/wp-sitemap-taxonomies-post_tag-1.xml",
        "/wp-sitemap-users-1.xml",
        "/wp-sitemap.xsl",
        "/wp-sitemap-index.xsl",
        "/robots.txt",
    ] {
        let theirs = fetch(path).unwrap();
        let (status, headers, body) = get(&app, Method::GET, path).await;
        assert_eq!(status.as_u16(), theirs.status, "{path}");
        assert_eq!(String::from_utf8_lossy(&body), String::from_utf8_lossy(&theirs.body), "{path}");
        let expected = if path == "/robots.txt" { "text/plain; charset=utf-8" } else { "application/xml; charset=UTF-8" };
        assert_eq!(headers[header::CONTENT_TYPE], expected, "{path}");
    }
}

#[tokio::test]
async fn missing_sitemaps_are_regular_404s() {
    let Some(app) = app().await else { return };
    for path in ["/wp-sitemap-posts-post-9.xml", "/wp-sitemap-posts-nope-1.xml", "/wp-sitemap-taxonomies-post_tag-2.xml"] {
        let theirs = fetch(path).unwrap();
        let (status, headers, body) = get(&app, Method::GET, path).await;
        assert_eq!(status.as_u16(), theirs.status, "{path}");
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(String::from_utf8_lossy(&body).starts_with("<!DOCTYPE html>"), "{path}");
        assert!(headers[header::CACHE_CONTROL].to_str().unwrap().contains("no-store"), "{path}");
    }
}

#[tokio::test]
async fn favicon_redirects_like_wordpress() {
    let Some(app) = app().await else { return };
    let (status, headers, _) = get(&app, Method::GET, "/favicon.ico").await;
    assert_eq!(status, StatusCode::FOUND);
    assert!(headers[header::LOCATION].to_str().unwrap().ends_with("/wp-includes/images/w-logo-gray-white-bg.png"));
}

#[tokio::test]
async fn rest_api_is_served_like_wordpress() {
    let Some(app) = app().await else { return };
    for path in ["/wp-json/wp/v2/posts/5", "/wp-json/wp/v2/posts?per_page=2&page=2", "/wp-json/wp/v2/categories", "/?rest_route=/wp/v2/pages/20", "/wp-json/wp/v2/posts/9999"] {
        let theirs = fetch(path).unwrap();
        let (status, headers, body) = get(&app, Method::GET, path).await;
        assert_eq!(status.as_u16(), theirs.status, "{path}");
        assert_eq!(String::from_utf8_lossy(&body), String::from_utf8_lossy(&theirs.body), "{path}");
        assert_eq!(headers[header::CONTENT_TYPE], "application/json; charset=UTF-8", "{path}");
    }
    let (_, headers, _) = get(&app, Method::GET, "/wp-json/wp/v2/posts?per_page=1").await;
    assert_eq!(headers["x-wp-total"], "18");
    assert_eq!(headers["x-wp-totalpages"], "18");
}
