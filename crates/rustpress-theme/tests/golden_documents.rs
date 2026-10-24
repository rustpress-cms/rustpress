//! Compares the `<head>` essentials of whole documents with the real WordPress:
//! `<title>`, `<body class>`, and the ids of all `<style>`/`<link>`/`<script>` tags.
//!
//! Same prerequisites as `golden_pages.rs`; skips without the reference stack.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use rustpress_blocks::Registry;
use rustpress_core::Site;
use rustpress_db::Db;
use rustpress_theme::{Theme, render_document};

const DEFAULT_ROOT: &str = r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data";
const URLS: &[&str] = &["/", "/ueber-uns/", "/testbeitrag-1/", "/gibt-es-nicht/", "/category/uncategorized/", "/?s=test", "/bilder/", "/cover-bilder/", "/bildliste/"];

fn fetch(path: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost:8080\r\nConnection: close\r\n\r\n")?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    Ok(text.split_once("\r\n\r\n").map(|(_, b)| b.to_owned()).unwrap_or(text))
}

fn between(html: &str, open: &str, close: &str) -> Option<String> {
    let start = html.find(open)? + open.len();
    let end = html[start..].find(close)? + start;
    Some(html[start..end].to_owned())
}

/// The text of `<style id="...">` in the head.
fn style_text(html: &str, id: &str) -> Option<String> {
    let head = between(html, "<head>", "</head>")?;
    let open = [format!("<style id='{id}'>"), format!("<style id=\"{id}\">")].into_iter().find(|o| head.contains(o.as_str()))?;
    let start = head.find(&open)? + open.len();
    Some(head[start..start + head[start..].find("</style>")?].to_owned())
}

/// Every `id="..."`/`id='...'` on style, link and script tags in the head, in order.
fn head_ids(html: &str) -> Vec<String> {
    let head = between(html, "<head>", "</head>").unwrap_or_default();
    let mut ids = Vec::new();
    let mut rest = head.as_str();
    while let Some(i) = rest.find('<') {
        rest = &rest[i + 1..];
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        if tag.starts_with("style") || tag.starts_with("link") || tag.starts_with("script") {
            for quote in ['"', '\''] {
                if let Some(p) = tag.find(&format!(" id={quote}")) {
                    let value = &tag[p + 5..];
                    if let Some(e) = value.find(quote) {
                        ids.push(value[..e].to_owned());
                        break;
                    }
                }
            }
        }
    }
    ids
}

#[tokio::test]
async fn documents_match_wordpress() {
    let root = PathBuf::from(std::env::var("RUSTPRESS_WP_ROOT").unwrap_or_else(|_| DEFAULT_ROOT.into()));
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let (Ok(db), true) = (Db::connect(&url, "wp_").await, root.join("wp-includes").is_dir()) else {
        eprintln!("reference stack not available, skipping");
        return;
    };
    if fetch("/").is_err() {
        eprintln!("WordPress not reachable, skipping");
        return;
    }
    let site = Site::load(db).await.unwrap();
    let theme = Theme::load(&site, &root).await.unwrap();
    let registry = Registry::new(root.join("wp-includes").join("blocks"));

    let mut problems = Vec::new();
    for path in URLS {
        let (p, qs) = path.split_once('?').unwrap_or((path, ""));
        let result = site.query(p, qs).await.unwrap();
        let doc = render_document(&theme, &site, &result, &registry).await.unwrap();
        let reference = fetch(path).unwrap();

        let (t_ref, t_ours) = (between(&reference, "<title>", "</title>"), between(&doc.html, "<title>", "</title>"));
        if t_ref != t_ours {
            problems.push(format!("{path}: title\n  wp:   {t_ref:?}\n  ours: {t_ours:?}"));
        }
        let (c_ref, c_ours) = (between(&reference, "<body class=\"", "\""), between(&doc.html, "<body class=\"", "\""));
        if c_ref != c_ours {
            problems.push(format!("{path}: body class\n  wp:   {c_ref:?}\n  ours: {c_ours:?}"));
        }
        let (i_ref, i_ours) = (head_ids(&reference), head_ids(&doc.html));
        if i_ref != i_ours {
            let missing: Vec<_> = i_ref.iter().filter(|i| !i_ours.contains(i)).collect();
            let extra: Vec<_> = i_ours.iter().filter(|i| !i_ref.contains(i)).collect();
            problems.push(format!("{path}: head ids\n  missing: {missing:?}\n  extra:   {extra:?}\n  wp:   {i_ref:?}\n  ours: {i_ours:?}"));
        }
        // The CSS the blocks of the page add (layout classes, gallery gaps).
        let id = "core-block-supports-inline-css";
        let (s_ref, s_ours) = (style_text(&reference, id), style_text(&doc.html, id));
        assert!(s_ref.is_some(), "{path}: the reference has no {id}");
        if s_ref != s_ours {
            problems.push(format!("{path}: {id}\n  wp:   {s_ref:?}\n  ours: {s_ours:?}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
