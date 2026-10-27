//! Canonical redirects: a port of `redirect_canonical()`, `redirect_guess_404_permalink()`
//! and `wp_old_slug_redirect()` from `wp-includes/canonical.php` and `query.php`.
//!
//! WordPress compares full URLs; here only `path?query` is handled, because the
//! scheme, host and port are whatever the client (or a reverse proxy) used.
//! Results are returned as `path?query` and the caller adds the origin.

use std::sync::OnceLock;

use fancy_regex::Regex;
use rustpress_db::Post;

use crate::Result;
use crate::query::{Queried, QueryResult};
use crate::rewrite::QueryVars;
use crate::settings::ShowOnFront;
use crate::site::Site;
use crate::urls::{trailingslashit, untrailingslashit};

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid regex"))
}

/// Splits `scheme://host/path?query#fragment` (or a bare `/path?query`) into path and query.
pub fn split_url(url: &str) -> (String, String) {
    let url = url.split('#').next().unwrap_or(url);
    let rest = match url.find("://") {
        Some(i) => {
            let after = &url[i + 3..];
            after.find(['/', '?']).map_or("", |j| &after[j..])
        }
        None => url,
    };
    match rest.split_once('?') {
        Some((path, query)) => (if path.is_empty() { "/".into() } else { path.to_owned() }, query.to_owned()),
        None => (if rest.is_empty() { "/".into() } else { rest.to_owned() }, String::new()),
    }
}

pub fn join(path: &str, query: &str) -> String {
    if query.is_empty() { path.to_owned() } else { format!("{path}?{query}") }
}

pub fn php_urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn rawurlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `parse_str` for flat queries: decoded pairs in order, later duplicates win in place.
pub fn parse_qs(query: &str) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for part in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = part.split_once('=').unwrap_or((part, ""));
        let decode = |s: &str| String::from_utf8_lossy(&crate::rewrite::urldecode(s.replace('+', " ").as_bytes())).into_owned();
        let (k, v) = (decode(k), decode(v));
        match pairs.iter_mut().find(|(key, _)| *key == k) {
            Some(existing) => existing.1 = v,
            None => pairs.push((k, v)),
        }
    }
    pairs
}

/// `build_query( urlencode_deep( $pairs ) )` as `add_query_arg` does it.
pub fn build_qs(pairs: &[(String, String)]) -> String {
    let parts: Vec<String> = pairs
        .iter()
        .map(|(k, v)| {
            let (k, v) = (php_urlencode(k), php_urlencode(v));
            if v.is_empty() { k } else { format!("{k}={v}") }
        })
        .collect();
    parts.join("&")
}

/// `remove_query_arg( $keys, $query )` on a bare query string.
pub fn remove_args(query: &str, keys: &[&str]) -> String {
    if !query.contains('=') {
        return query.to_owned();
    }
    let mut pairs = parse_qs(query);
    pairs.retain(|(k, _)| !keys.contains(&k.as_str()));
    build_qs(&pairs)
}

/// `add_query_arg( $args, $url )` for a `path?query` URL.
pub fn add_args(url: &str, args: &[(String, String)]) -> String {
    let (path, query) = split_url(url);
    let mut pairs = parse_qs(&query);
    for (k, v) in args {
        match pairs.iter_mut().find(|(key, _)| key == k) {
            Some(existing) => existing.1 = v.clone(),
            None => pairs.push((k.clone(), v.clone())),
        }
    }
    join(&path, &build_qs(&pairs))
}

/// `_remove_qs_args_if_not_in_url()`.
fn remove_args_if_not_in_url(query: &str, keys: &[&str], url: &str) -> String {
    let (_, url_query) = split_url(url);
    if url_query.is_empty() {
        return remove_args(query, keys);
    }
    let present = parse_qs(&url_query);
    let missing: Vec<&str> = keys.iter().copied().filter(|k| !present.iter().any(|(p, _)| p == k)).collect();
    remove_args(query, &missing)
}

fn lowercase_octets(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() && bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit) && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit) {
            out.push('%');
            out.push((bytes[i + 1] as char).to_ascii_lowercase());
            out.push((bytes[i + 2] as char).to_ascii_lowercase());
            i += 3;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

const PUNCTUATION: &[&str] = &[" ", "%20", "!", "%21", "\"", "%22", "'", "%27", "(", "%28", ")", "%29", ",", "%2C", ".", "%2E", ";", "%3B", "{", "%7B", "}", "%7D", "%E2%80%9C", "%E2%80%9D"];

fn punctuation_pattern() -> String {
    PUNCTUATION.iter().map(|p| fancy_regex::escape(p).into_owned()).collect::<Vec<_>>().join("|")
}

fn regex_replace(cell: &'static OnceLock<Regex>, pattern: &str, text: &str, replacement: &str) -> String {
    re(cell, pattern).replace_all(text, replacement).into_owned()
}

impl Site {
    /// The canonical `path?query` for a request, or `None` if the request is canonical.
    /// [Self::canonical_redirect] as a request sees it. WordPress builds the target of paging and feed
    /// redirects without the port of the site and then finds that it still is not canonical (its
    /// guard against chained redirects): on a port other than 80 or 443 these are never redirected.
    pub async fn canonical_redirect_for_request(&self, path: &str, query: &str, q: &QueryResult) -> Result<Option<String>> {
        // `redirect_canonical()` leaves previews alone; a published post with `?preview=true` but without a valid
        // preview nonce is none.
        if q.flags.is_preview {
            let published = matches!(&q.queried, crate::query::Queried::Post(post) if post.post_status == "publish");
            let vars = crate::rewrite::QueryVars::parse(&q.raw_query);
            let valid = q.viewer.as_ref().is_some_and(|viewer| vars.get("preview_nonce").is_some_and(|nonce| viewer.check_nonce(nonce, &format!("post_preview_{}", vars.u64("preview_id").unwrap_or(0)))));
            if !(q.vars.u64("p").is_some_and(|p| p != 0) && published && !valid) {
                return Ok(None);
            }
        }
        // A 404 never reaches the paging and feed part of `redirect_canonical()`.
        let paging = (q.vars.u64("paged").unwrap_or(0) != 0 || (q.flags.is_feed && !q.flags.is_404) || q.vars.u64("cpage").unwrap_or(0) != 0) && q.vars.get("sitemap").is_none_or(str::is_empty);
        let has_port = self.settings.home.rsplit_once("://").is_some_and(|(_, rest)| rest.split('/').next().is_some_and(|host| host.contains(':')));
        // A comment feed of a single post with plain permalinks is redirected to `get_post_comments_feed_link()`,
        // which carries the port.
        let from_feed_link = q.flags.is_feed && q.flags.is_singular() && !self.uses_pretty_permalinks();
        // An attachment (also the feed of one) still goes to its file or to the feed address.
        if paging && has_port && !from_feed_link && !q.flags.is_attachment {
            return Ok(None);
        }
        self.canonical_redirect(path, query, q).await
    }

    pub async fn canonical_redirect(&self, path: &str, query: &str, q: &QueryResult) -> Result<Option<String>> {
        if let Some(target) = self.old_slug_redirect(q).await? {
            return Ok(Some(target));
        }
        let Some(target) = self.canonical_target(path, query, q).await? else { return Ok(None) };
        // Protect against chained redirects: the target must itself be canonical.
        let (target_path, target_query) = split_url(&target);
        if self.canonical_target(&target_path, &target_query, q).await?.is_some() {
            return Ok(None);
        }
        Ok(Some(target))
    }

    /// `wp_old_slug_redirect()`.
    async fn old_slug_redirect(&self, q: &QueryResult) -> Result<Option<String>> {
        let name = q.vars.get("name").unwrap_or("");
        if !q.flags.is_404 || name.is_empty() {
            return Ok(None);
        }
        let post_type = if let Some(t) = q.vars.get("post_type").filter(|t| !t.is_empty()) {
            t
        } else if q.vars.has("attachment") {
            "attachment"
        } else if q.vars.has("pagename") {
            "page"
        } else {
            "post"
        };
        if post_type == "page" {
            return Ok(None);
        }
        let num = |key: &str| q.vars.u64(key).filter(|v| *v != 0).map(|v| v as u32);
        let Some(id) = self.db.post_by_old_slug(name, post_type, num("year"), num("monthnum"), num("day")).await? else { return Ok(None) };
        let Some(post) = self.db.post(id).await? else { return Ok(None) };
        Ok(Some(join_pair(&self.permalink(&post).await?)))
    }

    async fn post_url(&self, id: u64) -> Result<Option<(String, Post)>> {
        let Some(post) = self.db.post(id).await? else { return Ok(None) };
        let url = join_pair(&self.permalink(&post).await?);
        Ok(Some((url, post)))
    }

    /// `is_post_publicly_viewable()`.
    async fn publicly_viewable(&self, post: &Post) -> Result<bool> {
        if !self.is_post_type_viewable(&post.post_type) {
            return Ok(false);
        }
        let status = if post.post_type == "attachment" && post.post_status == "inherit" {
            if post.post_parent != 0 {
                match self.db.post(post.post_parent).await? {
                    Some(parent) => parent.post_status,
                    None => "publish".to_owned(),
                }
            } else {
                "publish".to_owned()
            }
        } else {
            post.post_status.clone()
        };
        Ok(status == "publish")
    }

    /// `redirect_guess_404_permalink()`.
    async fn guess_404(&self, q: &QueryResult) -> Result<Option<String>> {
        let name = q.vars.get("name").unwrap_or("");
        if name.is_empty() {
            return Ok(None);
        }
        let post_type = q.vars.get("post_type").filter(|t| !t.is_empty());
        if post_type.is_some_and(|t| !self.is_post_type_viewable(t)) {
            return Ok(None);
        }
        let num = |key: &str| q.vars.u64(key).filter(|v| *v != 0).map(|v| v as u32);
        let Some(id) = self.db.guess_post_by_name(name, post_type, num("year"), num("monthnum"), num("day")).await? else { return Ok(None) };
        let Some((url, _)) = self.post_url(id).await? else { return Ok(None) };
        let page = q.vars.u64("page").unwrap_or(0);
        if page > 1 && q.vars.get("feed").is_none_or(str::is_empty) {
            return Ok(Some(format!("{}{}", trailingslashit(&url), self.user_trailingslashit(&page.to_string()))));
        }
        Ok(Some(url))
    }

    async fn canonical_target(&self, path: &str, query: &str, q: &QueryResult) -> Result<Option<String>> {
        static NBSP: OnceLock<Regex> = OnceLock::new();
        static PAGING: OnceLock<Regex> = OnceLock::new();
        static FEED: OnceLock<Regex> = OnceLock::new();
        static CPAGING: OnceLock<Regex> = OnceLock::new();
        static PAGING_STRIP: OnceLock<Regex> = OnceLock::new();
        static FEED_STRIP: OnceLock<Regex> = OnceLock::new();
        static CPAGING_STRIP: OnceLock<Regex> = OnceLock::new();
        static INDEX: OnceLock<Regex> = OnceLock::new();
        static INDEX2: OnceLock<Regex> = OnceLock::new();
        static SLASHES: OnceLock<Regex> = OnceLock::new();

        let f = &q.flags;
        if f.is_search || f.is_favicon || q.vars.has("tb") {
            return Ok(None);
        }
        let get = QueryVars::parse(query);
        let using_permalinks = self.uses_pretty_permalinks();
        let settings = &self.settings;

        let mut rpath = regex_replace(&NBSP, "(?i)(%C2%A0)+$", path, "");
        let mut rquery = query.to_owned();
        // Previews are never authorized here, so `preview` is just dropped from the URL.
        if q.vars.get("preview").is_some_and(|v| !v.is_empty()) {
            rquery = remove_args(&rquery, &["preview"]);
        }
        let mut redirect_url: Option<String> = None;
        let mut redirect_obj: Option<Post> = None;
        let qv = |key: &str| q.vars.u64(key).unwrap_or(0);

        // The comment feed of a post: `?p=ID&feed=...` goes to `get_post_comments_feed_link()`.
        // `WP_Query::get_posts()` copies `attachment_id` and `page_id` into `p`.
        let mut feed_post_id = qv("p");
        if qv("attachment_id") != 0 {
            feed_post_id = qv("attachment_id");
        }
        if qv("page_id") != 0 && !(settings.show_on_front == ShowOnFront::Page && settings.page_for_posts == qv("page_id")) {
            feed_post_id = qv("page_id");
        }
        if f.is_feed && feed_post_id != 0 {
            let feed = q.vars.get("feed").filter(|v| !v.is_empty()).unwrap_or("rss2").to_owned();
            if let Some(post) = self.db.post(feed_post_id).await? {
                let link = self.post_comments_feed_link(&post, &feed).await?;
                if !link.is_empty() {
                    let url = join_pair(&link);
                    rquery = remove_args_if_not_in_url(&rquery, &["p", "page_id", "attachment_id", "pagename", "name", "post_type", "feed"], &url);
                    redirect_url = Some(url);
                    redirect_obj = Some(post);
                }
            }
        }

        if f.is_404 {
            // ?page_id, ?p=, ?attachment_id= to their respective URLs.
            let post_id = qv("p").max(qv("page_id")).max(qv("attachment_id"));
            if post_id != 0
                && let Some(post) = self.db.post(post_id).await?
                    && self.is_post_type_viewable(&post.post_type) && post.post_status != "auto-draft" {
                        let url = join_pair(&self.permalink(&post).await?);
                        rquery = remove_args_if_not_in_url(&rquery, &["p", "page_id", "attachment_id", "pagename", "name", "post_type"], &url);
                        redirect_url = Some(url);
                        redirect_obj = Some(post);
                    }

            let (year, month, day) = (qv("year"), qv("monthnum"), qv("day"));
            if year != 0 && month != 0 && day != 0 {
                let valid = chrono::NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32).is_some();
                if !valid {
                    let url = join_pair(&self.date_link(year as u32, month as u32, 0));
                    rquery = remove_args_if_not_in_url(&rquery, &["year", "monthnum", "day"], &url);
                    redirect_url = Some(url);
                }
            } else if year != 0 && month > 12 {
                let url = join_pair(&self.date_link(year as u32, 0, 0));
                rquery = remove_args_if_not_in_url(&rquery, &["year", "monthnum"], &url);
                redirect_url = Some(url);
            }

            // Strip off non-existing <!--nextpage--> links from single posts or pages.
            if qv("page") != 0 {
                let post_id = match &q.queried {
                    Queried::Post(post) => post.id,
                    _ => q.posts.first().map_or(0, |p| p.id),
                };
                if post_id != 0 {
                    if let Some((url, post)) = self.post_url(post_id).await? {
                        redirect_url = Some(url);
                        redirect_obj = Some(post);
                    }
                    rquery = remove_args(&rquery, &["page"]);
                }
            }

            if redirect_url.is_none()
                && let Some(url) = self.guess_404(q).await? {
                    let url = join_pair(&url);
                    rquery = remove_args_if_not_in_url(&rquery, &["page", "feed", "p", "page_id", "attachment_id", "pagename", "name", "post_type"], &url);
                    redirect_url = Some(url);
                }
        } else if using_permalinks {
            let queried_id = q.queried_object_id();
            if f.is_attachment {
                // Only `attachment`/`attachment_id` asked: the attachment's own link (and without `?attachment_id`, also
                // when the file is where it goes in the end, below).
                if q.vars.iter().all(|(k, _)| matches!(k, "attachment" | "attachment_id")) && redirect_url.is_none() {
                    if get.get("attachment_id").is_some_and(|v| !v.is_empty()) {
                        if let Some((url, post)) = self.post_url(qv("attachment_id")).await? {
                            rquery = remove_args(&rquery, &["attachment_id"]);
                            redirect_url = Some(url);
                            redirect_obj = Some(post);
                        }
                    } else if let Queried::Post(post) = &q.queried {
                        redirect_url = Some(join_pair(&self.permalink(post).await?));
                        redirect_obj = Some((**post).clone());
                    }
                }
            } else if f.is_single && get.get("p").is_some_and(|v| !v.is_empty()) {
                if let Some((url, post)) = self.post_url(qv("p")).await? {
                    rquery = remove_args(&rquery, &["p", "post_type"]);
                    redirect_url = Some(url);
                    redirect_obj = Some(post);
                }
            } else if f.is_single && get.get("name").is_some_and(|v| !v.is_empty()) {
                if let Some((url, post)) = self.post_url(queried_id).await? {
                    rquery = remove_args(&rquery, &["name"]);
                    redirect_url = Some(url);
                    redirect_obj = Some(post);
                }
            } else if f.is_page && get.get("page_id").is_some_and(|v| !v.is_empty()) {
                if let Some((url, post)) = self.post_url(qv("page_id")).await? {
                    rquery = remove_args(&rquery, &["page_id"]);
                    redirect_url = Some(url);
                    redirect_obj = Some(post);
                }
            } else if f.is_page && !f.is_feed && settings.show_on_front == ShowOnFront::Page && queried_id == settings.page_on_front {
                redirect_url = Some(join_pair(&self.home_url("/")));
            } else if f.is_home
                && get.get("page_id").is_some_and(|v| !v.is_empty())
                && settings.show_on_front == ShowOnFront::Page
                && qv("page_id") == settings.page_for_posts
            {
                if let Some((url, post)) = self.post_url(settings.page_for_posts).await? {
                    rquery = remove_args(&rquery, &["page_id"]);
                    redirect_url = Some(url);
                    redirect_obj = Some(post);
                }
            } else if get.get("m").is_some_and(|v| !v.is_empty()) && (f.is_year || f.is_month || f.is_day) {
                let m = q.vars.get("m").unwrap_or("");
                let part = |range: std::ops::Range<usize>| m.get(range).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                redirect_url = match m.len() {
                    4 => Some(join_pair(&self.date_link(part(0..4), 0, 0))),
                    6 => Some(join_pair(&self.date_link(part(0..4), part(4..6), 0))),
                    8 => Some(join_pair(&self.date_link(part(0..4), part(4..6), part(6..8)))),
                    _ => None,
                };
                if redirect_url.is_some() {
                    rquery = remove_args(&rquery, &["m"]);
                }
            } else if f.is_year || f.is_month || f.is_day {
                let (year, month, day) = (qv("year") as u32, qv("monthnum") as u32, qv("day") as u32);
                let given = |key: &str| get.get(key).is_some_and(|v| !v.is_empty());
                if f.is_day && year != 0 && month != 0 && given("day") {
                    redirect_url = Some(join_pair(&self.date_link(year, month, day)));
                    rquery = remove_args(&rquery, &["year", "monthnum", "day"]);
                } else if f.is_month && year != 0 && given("monthnum") {
                    redirect_url = Some(join_pair(&self.date_link(year, month, 0)));
                    rquery = remove_args(&rquery, &["year", "monthnum"]);
                } else if f.is_year && given("year") {
                    redirect_url = Some(join_pair(&self.date_link(year, 0, 0)));
                    rquery = remove_args(&rquery, &["year"]);
                }
            } else if f.is_author && get.get("author").is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit())) {
                if let Queried::User(user) = &q.queried {
                    let has_posts = self
                        .db
                        .count_posts(&rustpress_db::PostQuery { author: Some(user.id), statuses: vec!["publish".into()], post_types: Vec::new(), ..Default::default() })
                        .await?
                        > 0;
                    if has_posts {
                        redirect_url = Some(join_pair(&self.author_url(user)));
                        rquery = remove_args(&rquery, &["author"]);
                    }
                }
            } else if f.is_category || f.is_tag || f.is_tax {
                let custom_vars: Vec<&str> = self.types.taxonomies.iter().map(|t| t.name.as_str()).collect();
                let term_count = ["cat", "category_name", "tag", "term"]
                    .iter()
                    .chain(custom_vars.iter())
                    .filter_map(|k| q.vars.get(k))
                    .map(|v| v.split(['+', ',', ' ']).filter(|s| !s.is_empty()).count())
                    .sum::<usize>();
                if let Queried::Term(term) = &q.queried
                    && term_count <= 1 && term.term_id != 0 && !rquery.is_empty() {
                        let mut remove = vec!["term", "taxonomy"];
                        if f.is_category {
                            remove.extend(["category_name", "cat"]);
                        } else if f.is_tag {
                            remove.extend(["tag", "tag_id"]);
                        } else {
                            // A custom taxonomy has a query variable of its own.
                            remove.push(&term.taxonomy);
                        }
                        let all_from_get = q.vars.iter().all(|(k, _)| get.has(k));
                        if all_from_get {
                            let tax_url = join_pair(&self.term_link(term).await?);
                            rquery = remove_args(&rquery, &remove);
                            let (tax_path, tax_query) = split_url(&tax_url);
                            if tax_query.is_empty() {
                                rpath = tax_path;
                            } else {
                                rquery = add_args(&format!("?{rquery}"), &parse_qs(&tax_query)).trim_start_matches('?').to_owned();
                            }
                        }
                    }
            }

            // Post paging.
            if f.is_singular() && qv("page") != 0 {
                let page = qv("page");
                if redirect_url.is_none()
                    && let Some((url, post)) = self.post_url(q.queried_object_id()).await? {
                        redirect_url = Some(url);
                        redirect_obj = Some(post);
                    }
                if page > 1
                    && let Some(url) = &redirect_url {
                        let base = trailingslashit(url);
                        let tail = if f.is_front_page { self.user_trailingslashit(&format!("page/{page}")) } else { self.user_trailingslashit(&page.to_string()) };
                        redirect_url = Some(format!("{base}{tail}"));
                    }
                rquery = remove_args(&rquery, &["page"]);
            }

            if let Some(sitemap) = q.vars.get("sitemap").filter(|s| !s.is_empty()) {
                let url = self.sitemap_url_for(sitemap, q.vars.get("sitemap-subtype").filter(|s| !s.is_empty()), qv("paged"));
                rquery = remove_args(&rquery, &["sitemap", "sitemap-subtype", "paged"]);
                redirect_url = Some(join_pair(&url));
            } else if qv("paged") != 0 || f.is_feed || qv("cpage") != 0 {
                let paged = qv("paged");
                let cpage = qv("cpage");
                let feed = q.vars.get("feed").unwrap_or("");
                let paging = r"/page/?[0-9]+?(/+)?$";
                let feed_re = r"/(comments/?)?(feed|rss2?|rdf|atom)(/+)?$";
                let comment_paging = r"/comment-page-[0-9]+(/+)?$";
                while re(&PAGING, paging).is_match(&rpath).unwrap_or(false) || re(&FEED, feed_re).is_match(&rpath).unwrap_or(false) || re(&CPAGING, comment_paging).is_match(&rpath).unwrap_or(false) {
                    rpath = regex_replace(&PAGING_STRIP, paging, &rpath, "/");
                    rpath = regex_replace(&FEED_STRIP, r"/(comments/?)?(feed|rss2?|rdf|atom)(/+|$)", &rpath, "/");
                    rpath = regex_replace(&CPAGING_STRIP, r"/comment-page-[0-9]+?(/+)?$", &rpath, "/");
                }

                let mut addl_path = String::new();
                let default_feed = "rss2";
                let feeds = ["feed", "rdf", "rss", "rss2", "atom"];
                if f.is_feed && feeds.contains(&feed) {
                    if !f.is_singular() && q.vars.has("withcomments") {
                        addl_path.push_str("comments/");
                    }
                    let format = if feed == "rss" { if default_feed == "rss2" { "" } else { "rss2" } } else if default_feed == feed || feed == "feed" { "" } else { feed };
                    addl_path.push_str(&self.user_trailingslashit(&format!("feed/{format}")));
                    rquery = remove_args(&rquery, &["feed"]);
                }

                if paged > 0 {
                    rquery = remove_args(&rquery, &["paged"]);
                    if !f.is_feed {
                        if !f.is_single {
                            if !addl_path.is_empty() {
                                addl_path = trailingslashit(&addl_path);
                            }
                            if paged > 1 {
                                addl_path.push_str(&self.user_trailingslashit(&format!("page/{paged}")));
                            }
                        }
                    } else if paged > 1 {
                        rquery = add_args(&format!("?{rquery}"), &[("paged".into(), paged.to_string())]).trim_start_matches('?').to_owned();
                    }
                }

                let default_comments_page = self.options.string("default_comments_page", "newest");
                if self.options.get("page_comments") == Some("1") && ((default_comments_page == "newest" && cpage > 0) || (default_comments_page != "newest" && cpage > 1)) {
                    if !addl_path.is_empty() {
                        addl_path = trailingslashit(&addl_path);
                    }
                    addl_path.push_str(&self.user_trailingslashit(&format!("comment-page-{cpage}")));
                    rquery = remove_args(&rquery, &["cpage"]);
                }

                rpath = regex_replace(&INDEX, r"/index\.php/?$", &rpath, "/");
                rpath = self.user_trailingslashit(&rpath);
                if !addl_path.is_empty() && self.using_index_permalinks() && !rpath.contains("/index.php/") {
                    rpath = format!("{}index.php/", trailingslashit(&rpath));
                }
                if !addl_path.is_empty() {
                    rpath = format!("{}{addl_path}", trailingslashit(&rpath));
                }
                redirect_url = Some(rpath.clone());
            }
        }

        // Attachment pages disabled: redirect to the file.
        let mut is_attachment_redirect = false;
        if f.is_attachment && self.options.get("wp_attachment_pages_enabled") != Some("1")
            && let Queried::Post(post) = &q.queried {
                if let Some(url) = self.attachment_file_url(post).await? {
                    let url = join_pair_keep_origin(&url);
                    if redirect_url.as_deref() != Some(url.as_str()) {
                        redirect_url = Some(url);
                    }
                    redirect_obj = self.db.post(post.post_parent).await?;
                }
                is_attachment_redirect = true;
            }

        rquery = rquery.strip_prefix('?').unwrap_or(&rquery).to_owned();

        // Tack on any additional query vars.
        if let Some(url) = &redirect_url
            && !rquery.is_empty() {
                let mut parsed = parse_qs(&rquery);
                let (_, url_query) = split_url(url);
                if parsed.iter().any(|(k, v)| k == "name" && !v.is_empty()) && !url_query.is_empty() {
                    let in_url = parse_qs(&url_query);
                    if !in_url.iter().any(|(k, v)| k == "name" && !v.is_empty()) {
                        parsed.retain(|(k, _)| k != "name");
                    }
                }
                let encoded: Vec<(String, String)> = parsed.iter().map(|(k, v)| (rawurlencode(k), rawurlencode(v))).collect();
                redirect_url = Some(add_args(url, &encoded));
            }
        if let Some(url) = &redirect_url {
            let (p, qs) = split_url(url);
            rpath = p;
            rquery = qs;
        }

        // Trailing /index.php.
        rpath = regex_replace(&INDEX2, r"/index\.php/*?$", &rpath, "/");

        // Remove trailing spaces and end punctuation from the path.
        let punct = punctuation_pattern();
        let cached = crate::formatting::cached_regex;
        rpath = cached(&format!("({punct})+$")).replace(&rpath, "").into_owned();
        if !rquery.is_empty() {
            rquery = cached(&format!("((^|&)(p|page_id|cat|tag)=[^&]*?)({punct})+$")).replace(&rquery, "$1").into_owned();
            rquery = cached("(^|&)(p|page_id|cat|tag)=?(&|$)").replace_all(&rquery, "&").trim_matches('&').to_owned();
            rquery = cached("(^|&)feed=rss(&|$)").replace(&rquery, "${1}feed=rss2${2}").into_owned();
            rquery = rquery.strip_prefix('?').unwrap_or(&rquery).trim_start_matches('&').to_owned();
        }

        if !self.using_index_permalinks() {
            rpath = rpath.replace("/index.php/", "/");
        }

        // Trailing slashes.
        if using_permalinks && !is_attachment_redirect && !f.is_404 && (!f.is_front_page || qv("paged") > 1) {
            rpath = self.user_trailingslashit(&rpath);
        } else if f.is_front_page {
            rpath = trailingslashit(&rpath);
        }

        // Remove trailing slash for robots.txt or sitemap requests.
        if f.is_robots || q.vars.get("sitemap").is_some_and(|v| !v.is_empty()) || q.vars.get("sitemap-stylesheet").is_some_and(|v| !v.is_empty()) {
            rpath = untrailingslashit(&rpath);
        }

        // Strip multiple slashes out of the URL.
        if rpath.contains("//") {
            rpath = regex_replace(&SLASHES, "/+", &rpath, "/");
        }

        // Always trailing slash the front page URL.
        let home_path = self.settings.home_path();
        let home_path = if home_path.is_empty() { "/".to_owned() } else { home_path };
        if trailingslashit(&rpath) == trailingslashit(&home_path) {
            rpath = trailingslashit(&rpath);
        }

        let original = join(path, query);
        let candidate = join(&rpath, &rquery);
        if candidate == original {
            return Ok(None);
        }
        // Hex-encoded octets are case-insensitive.
        if lowercase_octets(&original) == candidate {
            return Ok(None);
        }

        if let Some(post) = &redirect_obj
            && !self.publicly_viewable(post).await? {
                return Ok(None);
            }
        if redirect_url.is_none() && !using_permalinks && !f.is_404 && !f.is_front_page && rpath == path {
            return Ok(None);
        }
        Ok(Some(candidate))
    }

    /// `wp_get_attachment_url()`: the uploads URL of the attached file.
    async fn attachment_file_url(&self, post: &Post) -> Result<Option<String>> {
        let meta = self.db.post_meta(post.id).await?;
        let Some(file) = meta.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attached_file")).and_then(|m| m.meta_value.clone()) else {
            return Ok(None);
        };
        let base = self.options.string("upload_url_path", "");
        let base = if base.is_empty() { self.site_url("wp-content/uploads") } else { base };
        Ok(Some(format!("{}/{}", base.trim_end_matches('/'), file.trim_start_matches('/'))))
    }
}

/// `path?query` form of an absolute URL.
fn join_pair(url: &str) -> String {
    let (path, query) = split_url(url);
    join(&path, &query)
}

/// For attachment files the URL may live on another host (CDN); keep it absolute then.
fn join_pair_keep_origin(url: &str) -> String {
    join_pair(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_url_handles_origins_and_queries() {
        assert_eq!(split_url("http://localhost:8080/foo/?a=1#x"), ("/foo/".into(), "a=1".into()));
        assert_eq!(split_url("http://localhost:8080"), ("/".into(), String::new()));
        assert_eq!(split_url("http://localhost:8080?p=1"), ("/".into(), "p=1".into()));
        assert_eq!(split_url("/a/b/"), ("/a/b/".into(), String::new()));
    }

    #[test]
    fn query_helpers_follow_add_query_arg() {
        assert_eq!(remove_args("p=3&x=a b", &["p"]), "x=a+b");
        assert_eq!(remove_args("p=3", &["p"]), "");
        assert_eq!(remove_args("justflag", &["p"]), "justflag");
        assert_eq!(add_args("/a/?x=1", &[("y".into(), "2".into())]), "/a/?x=1&y=2");
        assert_eq!(add_args("/a/?x=1", &[("x".into(), "9".into())]), "/a/?x=9");
        assert_eq!(remove_args_if_not_in_url("p=1&name=a", &["p", "name"], "/a/?name=a"), "name=a");
    }

    #[test]
    fn octets_are_lowercased() {
        assert_eq!(lowercase_octets("/%C3%9Cber/"), "/%c3%9cber/");
        assert_eq!(lowercase_octets("/100%/"), "/100%/");
    }
}
