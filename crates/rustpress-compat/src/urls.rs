//! Which URLs to compare: fixed probes plus everything the reference WordPress itself
//! lists (sitemaps, the REST API, the assets its pages load).

use crate::http;

/// How a response is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// An HTML document of the block theme.
    Page,
    Feed,
    Sitemap,
    Rest,
    /// CSS, JS, fonts and images from `wp-content` / `wp-includes`.
    Asset,
    /// Everything else (robots.txt, login, admin, ...).
    Other,
}

impl Kind {
    pub const ALL: [Kind; 6] = [Kind::Page, Kind::Feed, Kind::Sitemap, Kind::Rest, Kind::Asset, Kind::Other];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Page => "HTML-Seiten",
            Kind::Feed => "Feeds",
            Kind::Sitemap => "Sitemaps",
            Kind::Rest => "REST-API",
            Kind::Asset => "Dateien",
            Kind::Other => "Sonstiges",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub path: String,
    pub kind: Kind,
}

/// The comparison kind for a request path (with query string).
pub fn classify(path: &str) -> Kind {
    let (p, query) = path.split_once('?').unwrap_or((path, ""));
    if p.starts_with("/wp-json") || query.contains("rest_route=") {
        Kind::Rest
    } else if p.starts_with("/wp-sitemap") || p == "/sitemap.xml" || query.contains("sitemap=") || query.contains("sitemap-stylesheet=") {
        Kind::Sitemap
    } else if p.starts_with("/wp-content/") || p.starts_with("/wp-includes/") {
        Kind::Asset
    } else if p.ends_with("/feed/") || p.ends_with("/feed") || p.contains("/feed/") || query.contains("feed=") {
        Kind::Feed
    } else if p == "/robots.txt" || p == "/favicon.ico" || p.ends_with(".php") || p.starts_with("/wp-admin") {
        Kind::Other
    } else {
        Kind::Page
    }
}

/// The text of every `<loc>` element.
pub fn extract_locs(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<loc>") {
        let after = &rest[i + 5..];
        let Some(end) = after.find("</loc>") else { break };
        out.push(after[..end].trim().replace("&amp;", "&"));
        rest = &after[end + 6..];
    }
    out
}

/// `http://host/path?q` -> `/path?q`; paths pass through.
pub fn to_path(location: &str) -> Option<String> {
    if location.starts_with('/') {
        return Some(location.to_owned());
    }
    let rest = location.strip_prefix("http://").or_else(|| location.strip_prefix("https://"))?;
    let path = &rest[rest.find('/')?..];
    Some(path.to_owned())
}

const PROBES: &[&str] = &[
    "/",
    "/?s=",
    "/?s=test",
    "/?s=gibt-es-garantiert-nicht-xyz",
    "/page/2/",
    "/page/9999/",
    "/gibt-es-nicht/",
    "/index.php",
    "/feed/",
    "/feed/atom/",
    "/comments/feed/",
    "/?feed=rss2",
    "/robots.txt",
    "/favicon.ico",
    "/wp-sitemap.xml",
    "/wp-sitemap.xsl",
    "/wp-sitemap-index.xsl",
    "/wp-sitemap-posts-post-9999.xml",
    "/wp-json/",
    "/wp-json/wp/v2/types",
    "/wp-json/wp/v2/search?search=test",
    "/wp-json/wp/v2/posts",
    "/wp-json/wp/v2/pages",
    "/wp-json/wp/v2/media",
    "/wp-json/wp/v2/categories",
    "/wp-json/wp/v2/tags",
    "/wp-json/wp/v2/users",
    "/wp-json/wp/v2/comments",
    "/wp-json/wp/v2/posts/99999",
    "/wp-json/wp/v2/categories?orderby=include",
    "/wp-json/wp/v2/categories?orderby=include_slugs",
    "/wp-json/wp/v2/posts?orderby=include",
    "/wp-json/wp/v2/posts?per_page=0",
    "/wp-json/wp/v2/posts?context=bogus",
    "/wp-login.php",
    "/wp-admin/",
    "/xmlrpc.php",
    "/wp-cron.php",
];

/// IDs listed by a REST collection of the reference.
fn rest_ids(reference: &http::Reference, route: &str) -> Vec<u64> {
    let path = format!("/wp-json/wp/v2/{route}?per_page=100&_fields=id");
    let Ok(response) = reference.get(&path) else { return Vec::new() };
    serde_json::from_slice::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|v| v.as_array().map(|items| items.iter().filter_map(|i| i.get("id").and_then(serde_json::Value::as_u64)).collect()))
        .unwrap_or_default()
}

/// Same-origin `href`/`src` values below `wp-content` and `wp-includes` in a document.
pub fn asset_links(html: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for attr in ["href=\"", "src=\"", "href='", "src='"] {
        let quote = attr.chars().last().unwrap_or('"');
        let mut rest = html;
        while let Some(i) = rest.find(attr) {
            let after = &rest[i + attr.len()..];
            let Some(end) = after.find(quote) else { break };
            let value = &after[..end];
            if let Some(path) = to_path(value).filter(|p| p.starts_with("/wp-content/") || p.starts_with("/wp-includes/"))
                && !out.contains(&path)
            {
                out.push(path);
            }
            rest = &after[end..];
        }
    }
    out
}

/// The probes for a site with plain permalinks (`?p=1`, `?paged=2`, `?rest_route=`).
const PLAIN_PROBES: &[&str] = &[
    "/",
    "/?s=",
    "/?s=test",
    "/?s=gibt-es-garantiert-nicht-xyz",
    "/?paged=2",
    "/?paged=9999",
    "/?p=99999",
    "/index.php",
    "/?feed=rss2",
    "/?feed=atom",
    "/?feed=comments-rss2",
    "/robots.txt",
    "/favicon.ico",
    "/?sitemap=index",
    "/?sitemap-stylesheet=sitemap",
    "/?sitemap-stylesheet=index",
    "/?rest_route=/",
    "/?rest_route=/wp/v2/types",
    "/?rest_route=/wp/v2/search&search=test",
    "/?rest_route=/wp/v2/posts",
    "/?rest_route=/wp/v2/pages",
    "/?rest_route=/wp/v2/media",
    "/?rest_route=/wp/v2/categories",
    "/?rest_route=/wp/v2/tags",
    "/?rest_route=/wp/v2/users",
    "/?rest_route=/wp/v2/comments",
    "/?rest_route=/wp/v2/posts/99999",
    "/?rest_route=/wp/v2/posts&context=bogus",
    "/wp-login.php",
    "/wp-admin/",
    "/xmlrpc.php",
    "/wp-cron.php",
];

/// IDs listed by a REST collection of a site with plain permalinks.
fn rest_ids_plain(reference: &http::Reference, route: &str) -> Vec<u64> {
    let path = format!("/?rest_route=/wp/v2/{route}&per_page=100&_fields=id");
    let Ok(response) = reference.get(&path) else { return Vec::new() };
    serde_json::from_slice::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|v| v.as_array().map(|items| items.iter().filter_map(|i| i.get("id").and_then(serde_json::Value::as_u64)).collect()))
        .unwrap_or_default()
}

/// The same discovery for a site with plain permalinks.
fn discover_plain(reference: &http::Reference, limit: usize) -> Vec<Target> {
    let mut paths: Vec<String> = PLAIN_PROBES.iter().map(|p| (*p).to_owned()).collect();
    let mut pages: Vec<String> = Vec::new();
    let mut newest: Option<(String, String, String)> = None;
    if let Ok(index) = reference.get("/?sitemap=index") {
        for sitemap in extract_locs(&index.text()).into_iter().filter_map(|l| to_path(&l)) {
            paths.push(sitemap.clone());
            if let Ok(response) = reference.get(&sitemap) {
                let text = response.text();
                if newest.is_none()
                    && let Some(at) = text.find("<lastmod>")
                    && let Some(date) = text.get(at + 9..at + 19)
                    && let [y, m, d] = date.split('-').collect::<Vec<_>>()[..]
                {
                    newest = Some((y.to_owned(), m.to_owned(), d.to_owned()));
                }
                pages.extend(extract_locs(&text).into_iter().filter_map(|l| to_path(&l)));
            }
        }
    }
    if let Some((y, m, d)) = newest {
        paths.extend([format!("/?m={y}"), format!("/?m={y}{m}"), format!("/?m={y}{m}{d}"), format!("/?m={y}13"), format!("/?m={y}{m}{d}&paged=2")]);
    }
    for page in &pages {
        paths.push(page.clone());
        if page.contains("cat=") || page.contains("tag=") || page.contains("author=") || page.contains("taxonomy") {
            paths.push(format!("{page}&paged=2"));
        } else if page != "/" {
            paths.push(format!("{page}&feed=rss2"));
        }
    }
    for (route, take) in [("posts", 40), ("pages", 20), ("media", 20), ("categories", 20), ("tags", 20), ("users", 10), ("comments", 20)] {
        for id in rest_ids_plain(reference, route).into_iter().take(take) {
            paths.push(format!("/?rest_route=/wp/v2/{route}/{id}"));
        }
    }
    paths.push("/?rest_route=/wp/v2/posts&_embed&per_page=10".to_owned());
    paths.push("/?rest_route=/wp/v2/pages&_embed&per_page=10".to_owned());
    if let Ok(home) = reference.get("/") {
        paths.extend(asset_links(&home.text()).into_iter().take(40));
    }
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.clone()));
    paths.truncate(limit);
    paths.into_iter().map(|path| Target { kind: classify(&path), path }).collect()
}

/// Everything worth comparing, in a stable order, without duplicates.
pub fn discover(reference: &http::Reference, limit: usize) -> Vec<Target> {
    // Plain permalinks: the REST discovery link of the home page names `rest_route`.
    if reference.get("/").is_ok_and(|r| r.text().contains("api.w.org/\" href=\"") && r.text().contains("?rest_route=")) {
        return discover_plain(reference, limit);
    }
    let mut paths: Vec<String> = PROBES.iter().map(|p| (*p).to_owned()).collect();

    // What the sitemaps list (posts, pages, term archives and authors).
    let mut pages: Vec<String> = Vec::new();
    let mut lastmod_year_month: Option<(String, String, String)> = None;
    if let Ok(index) = reference.get("/wp-sitemap.xml") {
        let index_text = index.text();
        let mut sitemaps = extract_locs(&index_text).into_iter().filter_map(|l| to_path(&l)).collect::<Vec<_>>();
        if !index_text.contains("<sitemapindex") {
            sitemaps.clear();
            pages.extend(extract_locs(&index_text).into_iter().filter_map(|l| to_path(&l)));
        }
        for sitemap in sitemaps {
            paths.push(sitemap.clone());
            if let Ok(response) = reference.get(&sitemap) {
                let text = response.text();
                if lastmod_year_month.is_none()
                    && let Some(at) = text.find("<lastmod>")
                    && let Some(date) = text.get(at + 9..at + 19)
                {
                    let parts: Vec<&str> = date.split('-').collect();
                    if let [y, m, d] = parts[..] {
                        lastmod_year_month = Some((y.to_owned(), m.to_owned(), d.to_owned()));
                    }
                }
                pages.extend(extract_locs(&text).into_iter().filter_map(|l| to_path(&l)));
            }
        }
    }

    // Date archives around the newest content, plus one that cannot exist.
    if let Some((y, m, d)) = lastmod_year_month {
        paths.extend([format!("/{y}/"), format!("/{y}/{m}/"), format!("/{y}/{m}/{d}/"), format!("/{y}/13/"), format!("/{y}/{m}/{d}/page/2/")]);
    }

    for page in &pages {
        paths.push(page.clone());
        let archive = page.contains("/category/") || page.contains("/tag/") || page.contains("/author/");
        if archive {
            paths.push(format!("{}/page/2/", page.trim_end_matches('/')));
        } else if page != "/" {
            // The comment feed of a single post or page.
            paths.push(format!("{}/feed/", page.trim_end_matches('/')));
        }
    }

    // The REST API: each collection item on its own, embedded.
    for (route, take) in [("posts", 40), ("pages", 20), ("media", 20), ("categories", 20), ("tags", 20), ("users", 10), ("comments", 20)] {
        for id in rest_ids(reference, route).into_iter().take(take) {
            paths.push(format!("/wp-json/wp/v2/{route}/{id}"));
        }
    }
    paths.push("/wp-json/wp/v2/posts?_embed&per_page=10".to_owned());
    paths.push("/wp-json/wp/v2/pages?_embed&per_page=10".to_owned());
    for id in rest_ids(reference, "posts").into_iter().take(5) {
        paths.push(format!("/?p={id}"));
    }
    for id in rest_ids(reference, "pages").into_iter().take(3) {
        paths.push(format!("/?page_id={id}"));
    }

    // Files the front page loads.
    if let Ok(home) = reference.get("/") {
        paths.extend(asset_links(&home.text()).into_iter().take(40));
    }

    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.clone()));
    paths.truncate(limit);
    paths.into_iter().map(|path| Target { kind: classify(&path), path }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_classified() {
        assert_eq!(classify("/"), Kind::Page);
        assert_eq!(classify("/hello-world/"), Kind::Page);
        assert_eq!(classify("/category/news/feed/"), Kind::Feed);
        assert_eq!(classify("/?feed=rss2"), Kind::Feed);
        assert_eq!(classify("/wp-sitemap-posts-post-1.xml"), Kind::Sitemap);
        assert_eq!(classify("/wp-json/wp/v2/posts?per_page=2"), Kind::Rest);
        assert_eq!(classify("/?rest_route=/wp/v2/pages/20"), Kind::Rest);
        assert_eq!(classify("/wp-content/themes/x/style.css?ver=1"), Kind::Asset);
        assert_eq!(classify("/robots.txt"), Kind::Other);
        assert_eq!(classify("/wp-login.php"), Kind::Other);
    }

    #[test]
    fn sitemap_locations_become_paths() {
        let xml = "<urlset><url><loc>http://localhost:8080/a/</loc></url><url><loc>http://localhost:8080/b/?x=1</loc></url></urlset>";
        let paths: Vec<String> = extract_locs(xml).iter().filter_map(|l| to_path(l)).collect();
        assert_eq!(paths, vec!["/a/", "/b/?x=1"]);
        assert_eq!(to_path("/already"), Some("/already".to_owned()));
        assert_eq!(to_path("mailto:x"), None);
    }

    #[test]
    fn asset_links_are_collected_once() {
        let html = "<link rel='stylesheet' href='http://localhost:8080/wp-includes/css/a.css?ver=1'><script src=\"/wp-content/x.js\"></script><a href=\"/post/\">p</a><link href=\"http://localhost:8080/wp-includes/css/a.css?ver=1\">";
        assert_eq!(asset_links(html), vec!["/wp-includes/css/a.css?ver=1", "/wp-content/x.js"]);
    }
}
