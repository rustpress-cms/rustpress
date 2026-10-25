//! URL generation: the inverse of the rewrite rules. Ports of
//! `get_permalink()`, `get_page_link()`, `get_term_link()`,
//! `get_author_posts_url()`, `get_year_link()` and `get_pagenum_link()` for
//! the standard post types and taxonomies.

use rustpress_db::{Post, Term, User};

use crate::Result;
use crate::site::Site;

/// `trailingslashit` / `untrailingslashit` helpers.
pub fn trailingslashit(url: &str) -> String {
    format!("{}/", url.trim_end_matches(['/', '\\']))
}

pub fn untrailingslashit(url: &str) -> String {
    url.trim_end_matches(['/', '\\']).to_owned()
}

impl Site {
    /// `home_url( $path )`.
    pub fn home_url(&self, path: &str) -> String {
        let home = self.settings.home.trim_end_matches('/');
        if path.is_empty() { home.to_owned() } else { format!("{home}/{}", path.trim_start_matches('/')) }
    }

    /// `site_url( $path )`.
    pub fn site_url(&self, path: &str) -> String {
        let base = self.settings.siteurl.trim_end_matches('/');
        if path.is_empty() { base.to_owned() } else { format!("{base}/{}", path.trim_start_matches('/')) }
    }

    /// `WP_Rewrite::$root`: `index.php/` with path info permalinks, else nothing. Everything that
    /// does not follow the structure of the posts (pages, feeds, search, ...) lives below it.
    pub fn root(&self) -> &'static str {
        if self.using_index_permalinks() { "index.php/" } else { "" }
    }

    /// A rewrite slug below `index.php/` for path info permalinks (slugs of types that follow the
    /// permalink front already start with it).
    pub fn rooted(&self, slug: &str) -> String {
        if slug.starts_with("index.php/") { slug.to_owned() } else { format!("{}{slug}", self.root()) }
    }

    /// `WP_Rewrite::using_index_permalinks()`.
    pub fn using_index_permalinks(&self) -> bool {
        let structure = self.settings.permalink_structure.trim_start_matches('/');
        !self.settings.permalink_structure.is_empty() && structure.starts_with("index.php")
    }

    /// `rest_url( $path )`: below `wp-json` with pretty permalinks, else `index.php?rest_route=`.
    pub fn rest_url(&self, path: &str) -> String {
        let path = format!("/{}", path.trim_start_matches('/'));
        if self.uses_pretty_permalinks() {
            format!("{}{path}", self.home_url(&format!("{}wp-json", self.root())))
        } else {
            format!("{}?rest_route={path}", self.home_url("index.php"))
        }
    }

    /// `url_to_postid()`: the ID of the post a URL of this site shows, 0 if there is none.
    pub async fn url_to_postid(&self, url: &str) -> Result<u64> {
        let host_of = |u: &str| -> String {
            let rest = u.split_once("://").map_or("", |(_, r)| r);
            let host = rest.split(['/', '?', '#']).next().unwrap_or("");
            let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
            let host = host.split(':').next().unwrap_or("");
            host.strip_prefix("www.").unwrap_or(host).to_owned()
        };
        let (url_host, home_host) = (host_of(url), host_of(&self.settings.home));
        if !url_host.is_empty() && url_host != home_host {
            return Ok(0);
        }
        // `?p=N`, `?page_id=N`, `?attachment_id=N`.
        if let Ok(re) = regex::Regex::new(r"[?&](?:p|page_id|attachment_id)=(\d+)")
            && let Some(caps) = re.captures(url)
        {
            let id: u64 = caps[1].parse().unwrap_or(0);
            if id != 0 {
                return Ok(id);
            }
        }
        let url = url.split('#').next().unwrap_or("");
        let url = url.split('?').next().unwrap_or("");
        // The scheme and `www.` follow the home URL.
        let home = self.settings.home.trim_end_matches('/');
        let home_scheme = home.split_once("://").map_or("http", |(s, _)| s);
        let mut url = match url.split_once("://") {
            Some((_, rest)) => format!("{home_scheme}://{rest}"),
            None => url.to_owned(),
        };
        if home.contains("://www.") && !url.contains("://www.") {
            url = url.replace("://", "://www.");
        } else if !home.contains("://www.") {
            url = url.replace("://www.", "://");
        }
        if url.trim_matches('/') == home && self.settings.show_on_front == crate::settings::ShowOnFront::Page {
            let front = self.settings.page_on_front;
            if front != 0 && self.db.post(front).await?.is_some() {
                return Ok(front);
            }
        }
        if self.rewrite.is_empty() {
            return Ok(0);
        }
        // Without path info permalinks `index.php/` is not part of the URL.
        let index_permalinks = self.settings.permalink_structure.trim_start_matches('/').starts_with("index.php");
        if !index_permalinks {
            url = url.replace("index.php/", "");
        }
        let home_slash = format!("{home}/");
        let path = if trailingslashit(&url).contains(&home_slash) {
            url.replace(home, "")
        } else {
            let home_path = home.split_once("://").and_then(|(_, r)| r.find('/').map(|i| r[i..].to_owned())).unwrap_or_default();
            trailingslashit(&url).strip_prefix(&format!("{home_path}/")).map_or_else(|| trailingslashit(&url), str::to_owned)
        };
        let path = path.trim_matches('/');
        let result = self.query(&format!("/{path}"), "").await?;
        if result.flags.is_singular() && let Some(post) = result.posts.first() {
            return Ok(post.id);
        }
        Ok(0)
    }

    pub fn uses_pretty_permalinks(&self) -> bool {
        !self.settings.permalink_structure.is_empty()
    }

    /// `user_trailingslashit()`: follows the permalink structure.
    pub fn user_trailingslashit(&self, url: &str) -> String {
        if self.settings.permalink_structure.ends_with('/') { trailingslashit(url) } else { untrailingslashit(url) }
    }

    /// The part of the permalink structure before the first rewrite tag
    /// (`WP_Rewrite::$front`), without its leading slash.
    pub fn front(&self) -> String {
        let structure = &self.settings.permalink_structure;
        let front = structure.find('%').map_or(structure.as_str(), |i| &structure[..i]);
        front.trim_start_matches('/').to_owned()
    }

    /// `get_permalink( $post )`.
    pub async fn permalink(&self, post: &Post) -> Result<String> {
        match post.post_type.as_str() {
            "page" => return self.page_link(post).await,
            "attachment" => return self.attachment_link(post).await,
            "post" => {}
            // The types WordPress registers itself (`wp_navigation`, ...) are linked like posts.
            other if !crate::types::is_internal_post_type(other) => return self.custom_permalink(post, other).await,
            _ => {}
        }
        let structure = &self.settings.permalink_structure;
        let unpublished = matches!(post.post_status.as_str(), "draft" | "pending" | "auto-draft" | "future");
        if structure.is_empty() || unpublished {
            return Ok(self.home_url(&format!("?p={}", post.id)));
        }

        let mut category = String::new();
        if structure.contains("%category%") {
            let mut cats = self.db.terms_for_post(post.id, Some("category")).await?;
            cats.sort_by_key(|t| t.term_id);
            if let Some(first) = cats.first() {
                category = self.term_path(first).await?;
            } else if let Some(default) = self.db.term_by_id(Some("category"), self.options.u64("default_category", 0)).await? {
                // Posts without a category show the default one in their address.
                category = default.slug;
            }
        }
        let author = if structure.contains("%author%") {
            self.db.user(post.post_author).await?.map(|u| u.user_nicename).unwrap_or_default()
        } else {
            String::new()
        };
        let date = date_parts(&post.post_date);
        let replacements = [
            ("%year%", date.0.as_str()),
            ("%monthnum%", date.1.as_str()),
            ("%day%", date.2.as_str()),
            ("%hour%", date.3.as_str()),
            ("%minute%", date.4.as_str()),
            ("%second%", date.5.as_str()),
            ("%postname%", post.post_name.as_str()),
            ("%post_id%", &post.id.to_string()),
            ("%category%", category.as_str()),
            ("%author%", author.as_str()),
            ("%pagename%", post.post_name.as_str()),
        ];
        let mut path = structure.clone();
        for (tag, value) in replacements {
            path = path.replace(tag, value);
        }
        Ok(self.user_trailingslashit(&self.home_url(&path)))
    }

    /// `get_post_permalink()` for a custom post type: its rewrite slug and the post name or page path.
    async fn custom_permalink(&self, post: &Post, post_type: &str) -> Result<String> {
        let unpublished = matches!(post.post_status.as_str(), "draft" | "pending" | "auto-draft" | "future");
        if let Some(info) = self.types.post_type(post_type).filter(|_| !unpublished && !self.settings.permalink_structure.is_empty()) {
            let tail = if info.hierarchical { self.page_uri(post).await? } else { post.post_name.clone() };
            return Ok(self.user_trailingslashit(&self.home_url(&format!("{}/{tail}", self.rooted(&info.slug)))));
        }
        // Plain permalinks: the query variable of the type with the slug, when the post is published.
        if let Some(var) = self.types.post_type(post_type).and_then(|info| info.query_var.clone().map(|var| (var, info.hierarchical))).filter(|_| !unpublished) {
            let slug = if var.1 { self.page_uri(post).await? } else { post.post_name.clone() };
            return Ok(self.home_url(&format!("?{}={slug}", var.0)));
        }
        Ok(self.home_url(&format!("?post_type={post_type}&p={}", post.id)))
    }

    /// `get_post_comments_feed_link( $post_id, $feed )`: the address of the comment feed of one post.
    pub async fn post_comments_feed_link(&self, post: &Post, feed: &str) -> Result<String> {
        let unattached = post.post_type == "attachment" && post.post_parent == 0;
        if self.uses_pretty_permalinks() {
            let base = if self.settings.show_on_front == crate::ShowOnFront::Page && self.settings.page_on_front == post.id { self.page_link_of(post).await? } else { self.permalink(post).await? };
            if unattached {
                let mut url = self.home_url("/feed/");
                if feed != "rss2" {
                    url.push_str(&format!("{feed}/"));
                }
                return Ok(add_query_arg(&url, "attachment_id", &post.id.to_string()));
            }
            let mut url = format!("{}feed", trailingslashit(&base));
            if feed != "rss2" {
                url.push_str(&format!("/{feed}"));
            }
            return Ok(self.user_trailingslashit(&url));
        }
        let key = if unattached { "attachment_id" } else if post.post_type == "page" { "page_id" } else { "p" };
        let url = add_query_arg(&self.home_url("/"), "feed", feed);
        Ok(add_query_arg(&url, key, &post.id.to_string()))
    }

    /// `get_post_type_archive_link()`.
    pub fn post_type_archive_link(&self, post_type: &str) -> Option<String> {
        let info = self.types.post_type(post_type)?;
        let slug = info.archive_slug.as_deref()?;
        Some(if self.settings.permalink_structure.is_empty() {
            self.home_url(&format!("?post_type={post_type}"))
        } else {
            self.user_trailingslashit(&self.home_url(&self.rooted(slug)))
        })
    }

    /// `get_page_uri()`: `parent/child`.
    pub async fn page_uri(&self, page: &Post) -> Result<String> {
        let mut uri = page.post_name.clone();
        let mut parent = page.post_parent;
        let mut guard = 0;
        while parent != 0 && guard < 50 {
            let Some(p) = self.db.post(parent).await? else { break };
            uri = format!("{}/{uri}", p.post_name);
            parent = p.post_parent;
            guard += 1;
        }
        Ok(uri)
    }

    /// `_get_page_link()`: the address of the page itself, also when it is the front page.
    pub async fn page_link_of(&self, page: &Post) -> Result<String> {
        if self.settings.permalink_structure.is_empty() || matches!(page.post_status.as_str(), "draft" | "pending" | "auto-draft") {
            return Ok(self.home_url(&format!("?page_id={}", page.id)));
        }
        let uri = self.page_uri(page).await?;
        Ok(self.user_trailingslashit(&self.home_url(&format!("{}{uri}", self.root()))))
    }

    /// `get_page_link()`.
    pub async fn page_link(&self, page: &Post) -> Result<String> {
        use crate::settings::ShowOnFront;
        if self.settings.show_on_front == ShowOnFront::Page && self.settings.page_on_front == page.id {
            return Ok(self.user_trailingslashit(&self.home_url("")));
        }
        if self.settings.permalink_structure.is_empty() || matches!(page.post_status.as_str(), "draft" | "pending" | "auto-draft") {
            return Ok(self.home_url(&format!("?page_id={}", page.id)));
        }
        let uri = self.page_uri(page).await?;
        Ok(self.user_trailingslashit(&self.home_url(&format!("{}{uri}", self.root()))))
    }

    /// `get_attachment_link()`.
    async fn attachment_link(&self, attachment: &Post) -> Result<String> {
        let plain = self.home_url(&format!("/?attachment_id={}", attachment.id));
        if self.settings.permalink_structure.is_empty() {
            return Ok(plain);
        }
        let parent = if attachment.post_parent != 0 { self.db.post(attachment.post_parent).await? } else { None };
        if attachment.post_parent != 0 && (attachment.post_parent == attachment.id || parent.as_ref().is_none_or(|p| matches!(p.post_type.as_str(), "revision" | "nav_menu_item"))) {
            return Ok(plain);
        }
        match parent {
            Some(parent) => {
                let parent_link = Box::pin(self.permalink(&parent)).await?;
                if parent_link.contains('?') {
                    return Ok(plain);
                }
                let numeric = !attachment.post_name.is_empty() && attachment.post_name.bytes().all(|b| b.is_ascii_digit());
                let name = if numeric || self.settings.permalink_structure.contains("%category%") { format!("attachment/{}", attachment.post_name) } else { attachment.post_name.clone() };
                Ok(self.user_trailingslashit(&format!("{}/{name}", untrailingslashit(&parent_link))))
            }
            None => Ok(self.home_url(&self.user_trailingslashit(&attachment.post_name))),
        }
    }
    /// Slugs of a hierarchical term and its ancestors, `parent/child`.
    async fn term_path(&self, term: &Term) -> Result<String> {
        let all = self.db.terms(&term.taxonomy).await?;
        let mut path = term.slug.clone();
        let mut parent = term.parent;
        let mut guard = 0;
        while parent != 0 && guard < 50 {
            let Some(p) = all.iter().find(|t| t.term_id == parent) else { break };
            path = format!("{}/{path}", p.slug);
            parent = p.parent;
            guard += 1;
        }
        Ok(path)
    }

    /// `get_term_link( $term )` for categories, tags and other taxonomies.
    pub async fn term_link(&self, term: &Term) -> Result<String> {
        if self.settings.permalink_structure.is_empty() {
            let query = match term.taxonomy.as_str() {
                "category" => format!("cat={}", term.term_id),
                "post_tag" => format!("tag={}", term.slug),
                "post_format" => format!("post_format={}", term.slug.strip_prefix("post-format-").unwrap_or(&term.slug)),
                other => match self.types.taxonomy(other).and_then(|t| t.query_var.as_deref()) {
                    Some(var) => format!("{var}={}", term.slug),
                    None => format!("taxonomy={other}&term={}", term.slug),
                },
            };
            return Ok(self.home_url(&format!("?{query}")));
        }
        if term.taxonomy == "post_format" {
            let slug = term.slug.strip_prefix("post-format-").unwrap_or(&term.slug);
            return Ok(self.user_trailingslashit(&self.home_url(&format!("{}type/{slug}", self.front()))));
        }
        if let Some(info) = self.types.taxonomy(&term.taxonomy) {
            let tail = if info.rewrite_hierarchical { self.term_path(term).await? } else { term.slug.clone() };
            return Ok(self.user_trailingslashit(&self.home_url(&format!("{}/{tail}", self.rooted(&info.slug)))));
        }
        let base = match term.taxonomy.as_str() {
            "category" => self.options.string("category_base", "").trim_matches('/').to_owned(),
            "post_tag" => self.options.string("tag_base", "").trim_matches('/').to_owned(),
            _ => String::new(),
        };
        let default_base = match term.taxonomy.as_str() {
            "category" => "category",
            "post_tag" => "tag",
            other => other,
        };
        let front = self.front();
        let prefix = if base.is_empty() { format!("{front}{default_base}") } else { format!("{}{base}", self.root()) };
        let slug = if term.taxonomy == "category" { self.term_path(term).await? } else { term.slug.clone() };
        Ok(self.user_trailingslashit(&self.home_url(&format!("{prefix}/{slug}"))))
    }

    /// `get_author_posts_url()`.
    pub fn author_url(&self, user: &User) -> String {
        if self.settings.permalink_structure.is_empty() {
            return self.home_url(&format!("?author={}", user.id));
        }
        let base = self.options.string("author_base", "author");
        let base = if base.is_empty() { "author".to_owned() } else { base };
        self.user_trailingslashit(&self.home_url(&format!("{}{base}/{}", self.front(), user.user_nicename)))
    }

    /// `get_year_link()`, `get_month_link()`, `get_day_link()`; zero values are left out.
    pub fn date_link(&self, year: u32, month: u32, day: u32) -> String {
        if self.settings.permalink_structure.is_empty() {
            let mut query = format!("m={year}");
            if month != 0 {
                query.push_str(&format!("{month:02}"));
                if day != 0 {
                    query.push_str(&format!("{day:02}"));
                }
            }
            return self.home_url(&format!("?{query}"));
        }
        // `WP_Rewrite::get_date_permastruct()`: the order of the permalink structure, below `date/`
        // when the post ID comes early.
        let structure = &self.settings.permalink_structure;
        let order: [&str; 3] = ["%year%/%monthnum%/%day%", "%day%/%monthnum%/%year%", "%monthnum%/%day%/%year%"]
            .into_iter()
            .find(|endian| structure.contains(endian))
            .map_or(["year", "monthnum", "day"], |endian| if endian.starts_with("%day%") { ["day", "monthnum", "year"] } else if endian.starts_with("%monthnum%") { ["monthnum", "day", "year"] } else { ["year", "monthnum", "day"] });
        let mut front = self.front();
        let tokens: Vec<&str> = structure.split('%').skip(1).step_by(2).collect();
        if tokens.iter().take(3).any(|t| *t == "post_id") {
            front.push_str("date/");
        }
        let parts: Vec<String> = order
            .iter()
            .filter_map(|part| match *part {
                "year" => Some(year.to_string()),
                "monthnum" if month != 0 => Some(format!("{month:02}")),
                "day" if month != 0 && day != 0 => Some(format!("{day:02}")),
                _ => None,
            })
            .collect();
        let path = format!("{front}{}", parts.join("/"));
        self.user_trailingslashit(&self.home_url(&path))
    }

    /// `get_pagenum_link( $pagenum )` for a request (`path?query`): the home
    /// path and an existing `/page/N/` are removed first.
    pub fn pagenum_link(&self, request_uri: &str, pagenum: u64) -> String {
        let request = remove_query_arg(request_uri, "paged");
        let home_root = self.settings.home_path();
        let home_root = home_root.trim_end_matches('/');
        let mut request = if !home_root.is_empty() && request.len() >= home_root.len() && request[..home_root.len()].eq_ignore_ascii_case(home_root) {
            request[home_root.len()..].to_owned()
        } else {
            request
        };
        request = request.trim_start_matches('/').to_owned();
        let base = trailingslashit(&self.home_url(""));

        if !self.uses_pretty_permalinks() {
            let joined = format!("{base}{request}");
            return if pagenum > 1 { add_query_arg(&joined, "paged", &pagenum.to_string()) } else { joined };
        }
        let (path, query_string) = match request.find('?') {
            Some(i) => (request[..i].to_owned(), request[i..].to_owned()),
            None => (request.clone(), String::new()),
        };
        let mut path = strip_page_suffix(&format!("/{path}")).trim_start_matches('/').to_owned();
        if let Some(rest) = path.strip_prefix("index.php") {
            path = rest.to_owned();
        }
        let mut path = path.trim_start_matches('/').to_owned();
        if self.using_index_permalinks() && (pagenum > 1 || !path.is_empty()) {
            path = format!("index.php/{path}");
        }
        if pagenum > 1 {
            let page_part = self.user_trailingslashit(&format!("page/{pagenum}"));
            path = if path.is_empty() { page_part } else { format!("{}{page_part}", trailingslashit(&path)) };
        }
        // `user_trailingslashit( implode( '/', $parts ), 'paged' )`.
        let joined = if path.is_empty() { untrailingslashit(&base) } else { format!("{}{path}", base) };
        format!("{}{query_string}", self.user_trailingslashit(&joined))
    }
    /// `get_pagenum_link()` for a base URL (without query string and page part).
    pub fn paged_url(&self, base_url: &str, page: u64, query: &str) -> String {
        let (path_part, mut existing_query) = match base_url.split_once('?') {
            Some((p, q)) => (p.to_owned(), q.to_owned()),
            None => (base_url.to_owned(), String::new()),
        };
        if !query.is_empty() {
            if !existing_query.is_empty() {
                existing_query.push('&');
            }
            existing_query.push_str(query);
        }
        let mut url = if self.settings.permalink_structure.is_empty() {
            if page > 1 {
                if !existing_query.is_empty() {
                    existing_query.push('&');
                }
                existing_query.push_str(&format!("paged={page}"));
            }
            path_part
        } else {
            let stripped = strip_page_suffix(&path_part);
            if page > 1 {
                self.user_trailingslashit(&format!("{}/page/{page}", untrailingslashit(&stripped)))
            } else {
                stripped
            }
        };
        if !existing_query.is_empty() {
            url.push('?');
            url.push_str(&existing_query);
        }
        url
    }
}

/// `add_query_arg( $key, $value, $url )`: sets or replaces one query parameter. Like WordPress it
/// parses and rebuilds the whole query: existing values are re-encoded (`%20` becomes `+`) and
/// a parameter with an empty value loses its `=` (`?s=` becomes `?s`).
pub fn add_query_arg(url: &str, key: &str, value: &str) -> String {
    rebuild_query(url, key, Some(value))
}

/// `remove_query_arg( $key, $url )`.
pub fn remove_query_arg(url: &str, key: &str) -> String {
    rebuild_query(url, key, None)
}

fn rebuild_query(url: &str, key: &str, value: Option<&str>) -> String {
    let (before_fragment, fragment) = match url.split_once('#') {
        Some((a, b)) => (a, Some(b)),
        None => (url, None),
    };
    let (path, query) = before_fragment.split_once('?').unwrap_or((before_fragment, ""));
    let mut pairs: Vec<(String, String)> = crate::canonical::parse_qs(query).into_iter().map(|(k, v)| (k, crate::canonical::php_urlencode(&v))).collect();
    match (pairs.iter().position(|(k, _)| k == key), value) {
        (Some(at), Some(v)) => pairs[at].1 = v.to_owned(),
        (Some(at), None) => {
            pairs.remove(at);
        }
        (None, Some(v)) => pairs.push((key.to_owned(), v.to_owned())),
        (None, None) => {}
    }
    let query = pairs.iter().map(|(k, v)| if v.is_empty() { k.clone() } else { format!("{k}={v}") }).collect::<Vec<_>>().join("&");
    let mut out = if query.is_empty() { path.to_owned() } else { format!("{path}?{query}") };
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// Removes a trailing `/page/N/`.
fn strip_page_suffix(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if let Some(i) = trimmed.rfind("/page/")
        && trimmed[i + 6..].bytes().all(|b| b.is_ascii_digit()) && i + 6 < trimmed.len() {
            return format!("{}/", &trimmed[..i]);
        }
    path.to_owned()
}

fn date_parts(date: &str) -> (String, String, String, String, String, String) {
    let part = |range: std::ops::Range<usize>| date.get(range).unwrap_or("00").to_owned();
    (part(0..4), part(5..7), part(8..10), part(11..13), part(14..16), part(17..19))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slash_helpers() {
        assert_eq!(trailingslashit("http://a/b"), "http://a/b/");
        assert_eq!(trailingslashit("http://a/b//"), "http://a/b/");
        assert_eq!(untrailingslashit("http://a/b/"), "http://a/b");
    }

    #[test]
    fn page_suffix_is_removed() {
        assert_eq!(strip_page_suffix("http://a/category/x/page/2/"), "http://a/category/x/");
        assert_eq!(strip_page_suffix("http://a/category/x/"), "http://a/category/x/");
        assert_eq!(strip_page_suffix("http://a/page/12"), "http://a/");
        assert_eq!(strip_page_suffix("http://a/pages/x/"), "http://a/pages/x/");
    }

    #[test]
    fn query_args_are_set_and_removed() {
        assert_eq!(add_query_arg("http://a/?x=1", "p", "2"), "http://a/?x=1&p=2");
        assert_eq!(add_query_arg("http://a/?p=1", "p", "2"), "http://a/?p=2");
        assert_eq!(add_query_arg("http://a/", "p", "2"), "http://a/?p=2");
        assert_eq!(add_query_arg("http://a/#top", "p", "2"), "http://a/?p=2#top");
        assert_eq!(remove_query_arg("http://a/?paged=2&x=1", "paged"), "http://a/?x=1");
        assert_eq!(remove_query_arg("http://a/?paged=2", "paged"), "http://a/");
        assert_eq!(remove_query_arg("http://a/", "paged"), "http://a/");
    }

    #[test]
    fn date_is_split_from_the_mysql_format() {
        let (y, mo, d, h, mi, s) = date_parts("2026-10-24 14:07:09");
        assert_eq!((y.as_str(), mo.as_str(), d.as_str(), h.as_str(), mi.as_str(), s.as_str()), ("2026", "10", "24", "14", "07", "09"));
    }
}

/// `http://host:port/path` -> `/path`: the part of a URL after its origin (`wp_parse_url( ..., PHP_URL_PATH )`
/// kept for relative links such as the AJAX address).
pub fn strip_origin(url: &str) -> String {
    match url.split_once("://") {
        Some((_, rest)) => match rest.find('/') {
            Some(at) => rest[at..].trim_end_matches('/').to_owned(),
            None => String::new(),
        },
        None => url.trim_end_matches('/').to_owned(),
    }
}
