//! Renders pages with the block theme and compares the body with the real
//! WordPress (`dev/up.sh`). Needs the reference stack and the WordPress files;
//! without them the test passes with a note.
//!
//! `RUSTPRESS_WP_ROOT` points at the WordPress directory (default: the
//! reference volume through `\\wsl.localhost`), `RUSTPRESS_TEST_DB` at the DB.
//! `RUSTPRESS_ONLY=/path/` limits the run to one URL and prints both bodies.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use rustpress_blocks::Registry;
use rustpress_core::Site;
use rustpress_db::Db;
use rustpress_theme::{Theme, render_body};

const WP: &str = "127.0.0.1:8080";
const DEFAULT_ROOT: &str = r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data";

fn get(path: &str) -> std::io::Result<(u16, Option<String>, String)> {
    let mut stream = TcpStream::connect(WP)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost:8080\r\nConnection: close\r\n\r\n")?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let location = head.lines().find_map(|l| l.strip_prefix("Location: ").map(str::to_owned));
    Ok((status, location, body.to_owned()))
}

fn fetch(path: &str) -> std::io::Result<(u16, String)> {
    let mut path = path.to_owned();
    for _ in 0..5 {
        let (status, location, body) = get(&path)?;
        match (status, location) {
            (301 | 302, Some(l)) => path = l.strip_prefix("http://localhost:8080").unwrap_or(&l).to_owned(),
            _ => return Ok((status, body)),
        }
    }
    panic!("too many redirects");
}

/// From the skip link to the end of `div.wp-site-blocks`.
fn reference_body(html: &str) -> String {
    let start = html.find("<a class=\"skip-link").or_else(|| html.find("<div class=\"wp-site-blocks\">")).unwrap_or(0);
    let end = html.find("<script type=\"speculationrules\">").or_else(|| html.rfind("</body>")).unwrap_or(html.len());
    html[start..end].to_owned()
}

fn normalize(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut last_space = false;
    for c in html.chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out.replace("> <", "><").trim().to_owned()
}

fn first_difference(a: &str, b: &str) -> Option<usize> {
    a.char_indices().zip(b.chars()).find(|((_, x), y)| x != y).map(|((i, _), _)| i).or_else(|| (a.len() != b.len()).then_some(a.len().min(b.len())))
}

fn window(s: &str, at: usize, before: usize, after: usize) -> String {
    let start = s.char_indices().map(|(i, _)| i).rev().find(|&i| i <= at.saturating_sub(before)).unwrap_or(0);
    let end = s.char_indices().map(|(i, _)| i).find(|&i| i >= (at + after).min(s.len())).unwrap_or(s.len());
    s[start..end].to_owned()
}

async fn setup() -> Option<(Site, Theme, Registry)> {
    let root = PathBuf::from(std::env::var("RUSTPRESS_WP_ROOT").unwrap_or_else(|_| DEFAULT_ROOT.into()));
    if !root.join("wp-includes").is_dir() {
        eprintln!("WordPress files not found at {}, skipping", root.display());
        return None;
    }
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let db = Db::connect(&url, "wp_").await.ok()?;
    get("/").ok()?;
    let site = Site::load_at(db, &root).await.ok()?;
    let theme = Theme::load(&site, &root).await.ok()?;
    let registry = Registry::new(root.join("wp-includes").join("blocks"));
    Some((site, theme, registry))
}

#[tokio::test]
async fn page_bodies_match_wordpress() {
    let Some((site, theme, registry)) = setup().await else { return };
    let urls: Vec<String> = match std::env::var("RUSTPRESS_ONLY") {
        Ok(one) => vec![one],
        Err(_) => [
            "/ueber-uns/",
            "/ueber-uns/team/",
            "/gibt-es-nicht/",
            "/testbeitrag-1/",
            "/",
            "/category/news/",
            "/page/2/",
            "/?s=Beitrag",
            "/klassisch/",
            "/geschuetzt/",
            // Images: featured images, image blocks, a gallery, classic <img> tags and the site logo.
            "/testbeitrag-2/",
            "/bilder/",
            "/klassische-bilder/",
            "/logo/",
            "/viele-bilder/",
            "/cover-bilder/",
            "/bildliste/",
            // The database `category` template with an inheriting query loop of featured images.
            "/category/news/",
            "/category/tech/rust/page/2/",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    };
    let verbose = std::env::var("RUSTPRESS_ONLY").is_ok();

    let mut failures = Vec::new();
    for url in &urls {
        let (path, query) = url.split_once('?').unwrap_or((url, ""));
        let result = site.query(path, query).await.unwrap();
        let rendered = render_body(&theme, &site, &result, &registry).await.unwrap();
        let (_, reference) = fetch(url).unwrap();
        let ours = normalize(&rendered.html);
        let theirs = normalize(&reference_body(&reference));
        if verbose {
            eprintln!("--- OURS ({})\n{ours}\n--- THEIRS\n{theirs}", rendered.template_slug);
        }
        if ours != theirs {
            let at = first_difference(&ours, &theirs).unwrap_or(0);
            failures.push(format!(
                "{url} (template `{}`, {} vs {} bytes) differs at {at}:\n  ours  : …{}\n  theirs: …{}",
                rendered.template_slug,
                ours.len(),
                theirs.len(),
                window(&ours, at, 120, 200),
                window(&theirs, at, 120, 200)
            ));
        }
    }
    assert!(failures.is_empty(), "{} of {} pages differ:\n{}", failures.len(), urls.len(), failures.join("\n\n"));
}
