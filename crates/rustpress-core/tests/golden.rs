//! Compares the main query with the real WordPress (`dev/up.sh`, port 8080).
//!
//! For each URL the reference page is fetched and its `<body class>`, HTTP
//! status and post titles are compared with what [`Site::query`] computes.
//! Without a reachable database or WordPress the test passes with a note.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use rustpress_core::{Queried, QueryResult, Site};
use rustpress_db::Db;

const WP: &str = "127.0.0.1:8080";

struct Response {
    status: u16,
    location: Option<String>,
    body: String,
}

/// Minimal HTTP/1.0 GET so the test needs no HTTP client dependency.
fn get(path: &str) -> std::io::Result<Response> {
    let mut stream = TcpStream::connect(WP)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost:8080\r\nConnection: close\r\n\r\n")?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let location = head.lines().find_map(|l| l.strip_prefix("Location: ").map(str::to_owned));
    Ok(Response { status, location, body: body.to_owned() })
}

/// Follows redirects (canonical URLs) and returns the final response.
fn fetch(path: &str) -> std::io::Result<Response> {
    let mut path = path.to_owned();
    for _ in 0..5 {
        let response = get(&path)?;
        match (response.status, &response.location) {
            (301 | 302, Some(location)) => {
                path = location.strip_prefix("http://localhost:8080").unwrap_or(location).to_owned();
            }
            _ => return Ok(response),
        }
    }
    panic!("too many redirects for {path}");
}

fn body_classes(html: &str) -> BTreeSet<String> {
    let Some(start) = html.find("<body") else { return BTreeSet::new() };
    let tag = &html[start..html[start..].find('>').map_or(html.len(), |e| start + e)];
    let Some(class) = tag.split("class=\"").nth(1) else { return BTreeSet::new() };
    class.split('"').next().unwrap_or("").split_whitespace().map(str::to_owned).collect()
}

/// Texts of all `wp-block-post-title` headings in document order.
fn post_titles(html: &str) -> Vec<String> {
    let mut titles = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("class=\"wp-block-post-title") {
        rest = &rest[i..];
        let Some(open_end) = rest.find('>') else { break };
        let inner = &rest[open_end + 1..];
        let end = inner.find("</h").unwrap_or(inner.len());
        let mut text = String::new();
        let mut in_tag = false;
        for c in inner[..end].chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => text.push(c),
                _ => {}
            }
        }
        titles.push(text.trim().to_owned());
        rest = &rest[open_end..];
    }
    titles
}

const TRACKED: &[&str] =
    &["home", "blog", "archive", "date", "search", "paged", "error404", "single", "page", "category", "tag", "author", "attachment"];

fn expected_classes(r: &QueryResult) -> BTreeSet<String> {
    let f = &r.flags;
    let mut c = BTreeSet::new();
    let mut add = |name: &str, on: bool| {
        if on {
            c.insert(name.to_owned());
        }
    };
    add("home", f.is_front_page);
    add("blog", f.is_home);
    add("archive", f.is_archive);
    add("date", f.is_date);
    add("search", f.is_search);
    add("paged", f.is_paged);
    add("error404", f.is_404);
    add("single", f.is_single);
    add("page", f.is_page);
    add("category", f.is_category);
    add("tag", f.is_tag);
    add("author", f.is_author);
    add("attachment", f.is_attachment);
    if let Queried::Post(p) = &r.queried {
        if f.is_single {
            c.insert(format!("postid-{}", p.id));
        }
        if f.is_page {
            c.insert(format!("page-id-{}", p.id));
        }
    }
    c
}

async fn site() -> Option<Site> {
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let db = Db::connect(&url, "wp_").await.ok()?;
    get("/").ok()?;
    Site::load(db).await.ok()
}

#[tokio::test]
async fn main_query_matches_wordpress() {
    let Some(site) = site().await else {
        eprintln!("reference WordPress/database not reachable, skipping");
        return;
    };
    let id_of = |name: &str| {
        let name = name.to_owned();
        let db = site.db.clone();
        async move {
            db.posts(&rustpress_db::PostQuery {
                post_types: vec!["post".into(), "page".into()],
                name: Some(name),
                limit: Some(1),
                ..Default::default()
            })
            .await
            .unwrap()[0]
                .id
        }
    };
    let post_id = id_of("testbeitrag-1").await;
    let page_id = id_of("ueber-uns").await;
    let rust_id = site.db.term_by_slug("category", "rust").await.unwrap().unwrap().term_id;

    let urls: Vec<String> = [
        "/", "/index.php", "/page/2/", "/page/3/", "/page/4/", "/page/1/",
        "/testbeitrag-1/", "/hello-world/", "/klassisch/", "/mehrseitig/", "/mehrseitig/2/", "/mehrseitig/4/",
        "/entwurf/", "/geschuetzt/", "/testbeitrag-1/2/", "/gibt-es-nicht/", "/team/",
        "/ueber-uns/", "/ueber-uns/team/", "/ueber-uns/nope/", "/sample-page/", "/privacy-policy/",
        "/category/news/", "/category/news/page/2/", "/category/news/page/9/", "/category/tech/",
        "/category/tech/rust/", "/category/rust/", "/category/wrong/rust/", "/category/nonexistent/",
        "/tag/alpha/", "/tag/gamma/", "/tag/zzz/", "/tag/alpha,gamma/", "/tag/alpha+beta/",
        "/author/admin/", "/author/nobody/", "/author/admin/page/2/",
        "/2026/", "/2026/01/", "/2026/01/01/", "/2026/10/", "/2026/10/24/", "/2026/10/25/", "/1999/", "/2026/13/", "/2026/02/30/", "/2026/page/2/",
        "/?s=Beitrag", "/?s=nothingmatches", "/?s=", "/search/Testbeitrag/", "/search/Beitrag/page/2/", "/?s=the",
        "/?m=202610", "/?paged=2", "/?cat=999", "/?author=1", "/?author=77", "/?tag=beta", "/?feed=rss2",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([
        format!("/?p={post_id}"), format!("/?p={page_id}"), "/?p=99999".into(), format!("/?page_id={page_id}"),
        format!("/?cat={rust_id}"), format!("/?p={post_id}&page=2"),
    ])
    .collect();

    let mut mismatches = Vec::new();
    for url in &urls {
        let (path, query) = url.split_once('?').unwrap_or((url, ""));
        // Follow canonical redirects like the HTTP client does for WordPress.
        let (mut path, mut query) = (path.to_owned(), query.to_owned());
        let mut ours = site.query(&path, &query).await.unwrap();
        for _ in 0..5 {
            let Some(target) = site.canonical_redirect(&path, &query, &ours).await.unwrap() else { break };
            (path, query) = target.split_once('?').map_or((target.clone(), String::new()), |(p, q)| (p.to_owned(), q.to_owned()));
            ours = site.query(&path, &query).await.unwrap();
        }
        let theirs = fetch(url).unwrap();
        let mut problems = Vec::new();

        if ours.status() != theirs.status {
            problems.push(format!("status {} != WordPress {}", ours.status(), theirs.status));
        }
        let want = expected_classes(&ours);
        let have = body_classes(&theirs.body);
        for class in TRACKED {
            if want.contains(*class) != have.contains(*class) {
                problems.push(format!("class `{class}`: ours {}, WordPress {}", want.contains(*class), have.contains(*class)));
            }
        }
        for class in want.iter().filter(|c| c.starts_with("postid-") || c.starts_with("page-id-")) {
            if !have.contains(class) {
                problems.push(format!("missing class `{class}`"));
            }
        }
        if ours.status() == 200 && !ours.posts.is_empty() && theirs.status == 200 && !ours.flags.is_feed {
            let ours_titles: Vec<String> = ours.posts.iter().map(|p| p.post_title.clone()).collect();
            // "Protected: " is added by the `the_title` filter when rendering, not by the query.
            let theirs_titles: Vec<String> =
                post_titles(&theirs.body).into_iter().map(|t| t.trim_start_matches("Protected: ").to_owned()).collect();
            let n = if ours.flags.is_singular() { 1 } else { ours_titles.len() };
            if theirs_titles.len() < n || theirs_titles[..n] != ours_titles[..n] {
                problems.push(format!("titles: ours {:?}, WordPress {:?}", &ours_titles[..n], &theirs_titles[..theirs_titles.len().min(n + 2)]));
            }
        }
        if !problems.is_empty() {
            let short: Vec<String> = problems.iter().map(|p| p.chars().take(260).collect()).collect();
            mismatches.push(format!("{url}\n    {}", short.join("\n    ")));
        }
    }
    assert!(mismatches.is_empty(), "{} of {} URLs differ:\n{}", mismatches.len(), urls.len(), mismatches.join("\n"));
}
