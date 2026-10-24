//! Compares REST responses with the real WordPress (`dev/up.sh`): status, JSON
//! body (compared as text, so key order counts) and the pagination headers.
//! Skips without the reference stack.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use rustpress_blocks::Registry;
use rustpress_core::Site;
use rustpress_db::Db;
use rustpress_rest::Rest;
use rustpress_theme::Theme;
use serde_json::Value;

const DEFAULT_ROOT: &str = r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data";

const ROUTES: &[&str] = &[
    // posts
    "/wp/v2/posts",
    "/wp/v2/posts?per_page=2&page=3",
    "/wp/v2/posts?per_page=100",
    "/wp/v2/posts?search=Beitrag",
    "/wp/v2/posts?search=Beitrag&orderby=relevance",
    "/wp/v2/posts?slug=hello-world",
    "/wp/v2/posts?slug=hello-world,klassisch",
    "/wp/v2/posts?categories=2&per_page=3",
    "/wp/v2/posts?categories=3",
    "/wp/v2/posts?tags=5&per_page=3",
    "/wp/v2/posts?categories_exclude=2&per_page=4",
    "/wp/v2/posts?tags_exclude=5",
    "/wp/v2/posts?author=1&per_page=2",
    "/wp/v2/posts?author_exclude=1",
    "/wp/v2/posts?orderby=title&order=asc&per_page=4",
    "/wp/v2/posts?orderby=id&order=asc&per_page=3",
    "/wp/v2/posts?orderby=slug&per_page=3",
    "/wp/v2/posts?orderby=modified&per_page=3",
    "/wp/v2/posts?include=5,6,7&orderby=include",
    "/wp/v2/posts?include=7,5,6",
    "/wp/v2/posts?exclude=19,18,17",
    "/wp/v2/posts?sticky=true",
    "/wp/v2/posts?sticky=false&per_page=3",
    "/wp/v2/posts?offset=2&per_page=2",
    "/wp/v2/posts?after=2026-10-24T14:11:00&per_page=3",
    "/wp/v2/posts?before=2026-10-24T14:10:45",
    "/wp/v2/posts?_fields=id,title.rendered,link&per_page=3",
    "/wp/v2/posts?_embed&per_page=2",
    "/wp/v2/posts?page=99",
    "/wp/v2/posts?per_page=0",
    "/wp/v2/posts?per_page=101",
    "/wp/v2/posts?orderby=bogus",
    "/wp/v2/posts?order=sideways",
    "/wp/v2/posts?status=draft",
    "/wp/v2/posts?context=edit",
    "/wp/v2/posts?context=embed&per_page=2",
    "/wp/v2/posts?search=nichtvorhanden",
    "/wp/v2/posts?orderby=relevance",
    "/wp/v2/posts?orderby=include",
    "/wp/v2/posts/5",
    "/wp/v2/posts/5?_embed",
    "/wp/v2/posts/5?_fields=id,link",
    "/wp/v2/posts/5?context=embed",
    "/wp/v2/posts/5?context=edit",
    "/wp/v2/posts/19",
    "/wp/v2/posts/19?password=falsch",
    "/wp/v2/posts/9999",
    "/wp/v2/posts/20",
    "/wp/v2/posts/abc",
    "/wp/v2/posts/",
    // pages
    "/wp/v2/pages",
    "/wp/v2/pages/20",
    "/wp/v2/pages/21",
    "/wp/v2/pages/21?_embed",
    "/wp/v2/pages?parent=20",
    "/wp/v2/pages?orderby=menu_order&order=asc",
    "/wp/v2/pages?slug=ueber-uns",
    "/wp/v2/pages/5",
    // terms
    "/wp/v2/categories",
    "/wp/v2/categories?per_page=2&page=2",
    "/wp/v2/categories?hide_empty=true",
    "/wp/v2/categories?orderby=count&order=desc",
    "/wp/v2/categories?orderby=id",
    "/wp/v2/categories?parent=3",
    "/wp/v2/categories?search=ne",
    "/wp/v2/categories?post=5",
    "/wp/v2/categories?slug=news",
    "/wp/v2/categories?include=2,3&orderby=include",
    "/wp/v2/categories?exclude=1",
    "/wp/v2/categories/4",
    "/wp/v2/categories/4?_embed",
    "/wp/v2/categories/999",
    "/wp/v2/categories?page=9",
    "/wp/v2/tags",
    "/wp/v2/tags?per_page=1",
    "/wp/v2/tags/5",
    "/wp/v2/tags?post=5",
    "/wp/v2/tags/999",
    // users
    "/wp/v2/users",
    "/wp/v2/users/1",
    "/wp/v2/users/1?_embed",
    "/wp/v2/users/1?context=embed",
    "/wp/v2/users/99",
    "/wp/v2/users/me",
    "/wp/v2/users?search=adm",
    "/wp/v2/users?roles=administrator",
    // comments
    "/wp/v2/comments",
    "/wp/v2/comments?post=5",
    "/wp/v2/comments?post=19",
    "/wp/v2/comments?order=asc",
    "/wp/v2/comments?search=Guter",
    "/wp/v2/comments?per_page=1&page=2",
    "/wp/v2/comments?_embed&per_page=1",
    "/wp/v2/comments/2",
    "/wp/v2/comments/1?_embed",
    "/wp/v2/comments/999",
    "/wp/v2/comments?status=hold",
    // media and discovery
    "/wp/v2/media",
    "/wp/v2/media/1",
    "/wp/v2/nothing",
    "/wp/v2/posts/5/revisions",
    "/wp/v2/posts/5/revisions/9",
    "/wp/v2/posts/999/revisions",
    "/wp/v2/pages/20/autosaves",
    "/wp/v2/users?page=9",
    "/wp/v2/users?capabilities=x",
    "/wp/v2/comments?page=9",
    "/wp/v2/categories?orderby=count&order=asc",
    "/wp/v2/categories?orderby=name&order=desc",
    "/wp/v2/tags?orderby=count&order=desc",
    "/wp/v2/users?orderby=id&order=desc",
];

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

fn fetch(path_and_query: &str) -> std::io::Result<Reply> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    write!(stream, "GET /wp-json{path_and_query} HTTP/1.0\r\nHost: localhost:8080\r\nConnection: close\r\n\r\n")?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let headers = head.lines().skip(1).filter_map(|l| l.split_once(": ")).map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_owned())).collect();
    Ok(Reply { status, headers, body: body.to_owned() })
}

/// The path of the first member where two JSON documents differ.
fn first_json_difference(path: &str, ours: &Value, theirs: &Value) -> Option<String> {
    match (ours, theirs) {
        (Value::Object(a), Value::Object(b)) => {
            let (ak, bk): (Vec<_>, Vec<_>) = (a.keys().collect(), b.keys().collect());
            if ak != bk {
                return Some(format!("{path}: keys\n      ours: {ak:?}\n      wp:   {bk:?}"));
            }
            a.iter().find_map(|(k, v)| first_json_difference(&format!("{path}.{k}"), v, &b[k]))
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{path}: {} items, wp has {}", a.len(), b.len()));
            }
            a.iter().zip(b).enumerate().find_map(|(i, (x, y))| first_json_difference(&format!("{path}[{i}]"), x, y))
        }
        _ if ours == theirs => None,
        _ => Some(format!("{path}:\n      ours: {ours}\n      wp:   {theirs}")),
    }
}

#[tokio::test]
async fn rest_matches_wordpress() {
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
    let rest = Rest::new(&site, &theme, &registry);

    let only = std::env::var("RUSTPRESS_ONLY").ok();
    let mut problems = Vec::new();
    for route in ROUTES.iter().filter(|r| only.as_deref().is_none_or(|o| o == **r)) {
        let theirs = fetch(route).unwrap();
        let (path, query) = route.split_once('?').unwrap_or((route, ""));
        let ours = rest.handle(path, query).await.unwrap();
        let mut found = Vec::new();
        if ours.status != theirs.status {
            found.push(format!("status {} != {}", ours.status, theirs.status));
        }
        if ours.body != theirs.body {
            let detail = match (serde_json::from_str::<Value>(&ours.body), serde_json::from_str::<Value>(&theirs.body)) {
                (Ok(a), Ok(b)) => first_json_difference("$", &a, &b).unwrap_or_else(|| "same JSON, different text (number/escape format)".into()),
                _ => "body is not JSON".into(),
            };
            found.push(format!("body differs at {detail}"));
        }
        for name in ["x-wp-total", "x-wp-totalpages", "link", "content-type"] {
            let ours_value = ours.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str());
            let theirs_value = theirs.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
            if ours_value != theirs_value {
                found.push(format!("header {name}: ours {ours_value:?}, wp {theirs_value:?}"));
            }
        }
        if !found.is_empty() {
            let short: Vec<String> = found.iter().map(|f| f.chars().take(700).collect()).collect();
            problems.push(format!("{route}\n    {}", short.join("\n    ")));
        }
    }
    assert!(problems.is_empty(), "{} of {} routes differ:\n{}", problems.len(), ROUTES.len(), problems.join("\n"));
}
