//! Compares feeds byte for byte with the real WordPress (`dev/up.sh`):
//! body, `Content-Type`, `Last-Modified` and `ETag`. Skips without the stack.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use rustpress_blocks::Registry;
use rustpress_core::Site;
use rustpress_db::Db;
use rustpress_theme::{Theme, render_feed};

const DEFAULT_ROOT: &str = r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data";
const URLS: &[&str] = &[
    "/feed/", "/feed/atom/", "/?feed=rss2", "/category/uncategorized/feed/", "/category/news/feed/atom/", "/tag/alpha/feed/", "/author/admin/feed/",
    "/search/Beitrag/feed/", "/2026/10/feed/", "/testbeitrag-1/feed/", "/testbeitrag-1/feed/atom/", "/hello-world/feed/", "/geschuetzt/feed/",
    "/geschuetzt/feed/atom/", "/ueber-uns/feed/", "/comments/feed/", "/comments/feed/atom/", "/mehrseitig/feed/", "/klassisch/feed/",
];

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

fn fetch(path: &str) -> std::io::Result<Reply> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost:8080\r\nConnection: close\r\n\r\n")?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let headers = head.lines().skip(1).filter_map(|l| l.split_once(": ")).map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_owned())).collect();
    Ok(Reply { status, headers, body: body.to_owned() })
}

fn header<'a>(reply: &'a Reply, name: &str) -> Option<&'a str> {
    reply.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

fn first_difference(ours: &str, theirs: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (ours.lines().collect(), theirs.lines().collect());
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or("<end>"), b.get(i).copied().unwrap_or("<end>"));
        if x != y {
            return format!("line {}:\n      ours: {:?}\n      wp:   {:?}", i + 1, x, y);
        }
    }
    "only whitespace at the very end differs".to_owned()
}

#[tokio::test]
async fn feeds_match_wordpress() {
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
    let site = Site::load_at(db, &root).await.unwrap();
    let theme = Theme::load(&site, &root).await.unwrap();
    let registry = Registry::new(root.join("wp-includes").join("blocks"));

    let only = std::env::var("RUSTPRESS_ONLY").ok();
    let mut problems = Vec::new();
    for url in URLS.iter().filter(|u| only.as_deref().is_none_or(|o| o == **u)) {
        let theirs = fetch(url).unwrap();
        let (path, query) = url.split_once('?').unwrap_or((url, ""));
        let result = site.query(path, query).await.unwrap();
        let ours = render_feed(&theme, &site, &result, &registry, url).await.unwrap();
        let Some(ours) = ours else {
            problems.push(format!("{url}: no feed rendered (WordPress answered {})", theirs.status));
            continue;
        };
        if theirs.status != 200 {
            problems.push(format!("{url}: WordPress answered {}", theirs.status));
            continue;
        }
        if ours.body != theirs.body {
            problems.push(format!("{url}: body differs at {}", first_difference(&ours.body, &theirs.body)));
        }
        for (name, value) in [("content-type", ours.content_type.as_str()), ("last-modified", ours.last_modified.as_deref().unwrap_or("")), ("etag", ours.etag.as_str())] {
            let want = header(&theirs, name).unwrap_or("");
            if want != value {
                problems.push(format!("{url}: header {name}: ours {value:?}, wp {want:?}"));
            }
        }
        if ours.nocache != header(&theirs, "cache-control").is_some_and(|c| c.contains("no-store")) {
            problems.push(format!("{url}: nocache flag {} differs from WordPress", ours.nocache));
        }
    }
    assert!(problems.is_empty(), "{} problems:\n  {}", problems.len(), problems.join("\n  "));
}
