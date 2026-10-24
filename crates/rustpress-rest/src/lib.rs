//! A read-only subset of the WordPress REST API (`/wp-json/wp/v2/...`) for
//! anonymous visitors: posts, pages, media, categories, tags, users and
//! comments, with `_fields`, `_embed` and the pagination headers.
//!
//! Everything that needs a login (the `edit` context, drafts, writes) answers
//! with the same 401 errors WordPress gives to anonymous requests.

mod comments;
mod error;
mod fields;
mod media;
mod params;
mod posts;
mod terms;
mod users;
mod util;

use rustpress_blocks::Registry;
use rustpress_blocks::php_json;
use rustpress_core::{QueryResult, Site};
use rustpress_theme::Theme;
use serde_json::{Map, Value};

pub use error::RestError;
pub use params::Params;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] rustpress_db::Error),
    #[error(transparent)]
    Core(#[from] rustpress_core::Error),
    #[error(transparent)]
    Blocks(#[from] rustpress_blocks::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The result of one request: status, headers (without `Content-Length`) and the JSON text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// The data of a successful endpoint call.
pub(crate) struct Resp {
    pub data: Value,
    pub total: Option<u64>,
    pub total_pages: Option<u64>,
    /// Value of the `Link` header (pagination).
    pub link: Option<String>,
    /// The permalink of a single post or page, sent as `Link: <...>; rel="alternate"`.
    pub alternate: Option<String>,
}

impl Resp {
    pub fn item(data: Value) -> Self {
        Self { data, total: None, total_pages: None, link: None, alternate: None }
    }
}

pub(crate) type Outcome = std::result::Result<Resp, RestError>;

pub struct Rest<'a> {
    pub(crate) site: &'a Site,
    pub(crate) theme: &'a Theme,
    pub(crate) registry: &'a Registry,
    pub(crate) blank: QueryResult,
}

impl<'a> Rest<'a> {
    pub fn new(site: &'a Site, theme: &'a Theme, registry: &'a Registry) -> Self {
        Self { site, theme, registry, blank: QueryResult::blank() }
    }

    /// Handles `GET <route>?<query>`; `route` is the part after `/wp-json` (`/wp/v2/posts`).
    pub async fn handle(&self, route: &str, query: &str) -> Result<Reply> {
        let params = Params::parse(query);
        let outcome = self.dispatch(route, &params).await?;
        let mut headers = vec![
            ("X-Robots-Tag".to_owned(), "noindex".to_owned()),
            ("X-Content-Type-Options".to_owned(), "nosniff".to_owned()),
            ("Access-Control-Expose-Headers".to_owned(), "X-WP-Total, X-WP-TotalPages, Link".to_owned()),
            ("Access-Control-Allow-Headers".to_owned(), "Authorization, X-WP-Nonce, Content-Disposition, Content-MD5, Content-Type".to_owned()),
        ];
        let (status, mut data) = match outcome {
            Ok(resp) => {
                let link = resp.link.clone().or_else(|| resp.alternate.as_ref().map(|url| format!("<{url}>; rel=\"alternate\"; type=text/html"))).unwrap_or_else(|| self.root_link());
                headers.insert(1, ("Link".to_owned(), link));
                if let Some(total) = resp.total {
                    headers.push(("X-WP-Total".to_owned(), total.to_string()));
                }
                if let Some(pages) = resp.total_pages {
                    headers.push(("X-WP-TotalPages".to_owned(), pages.to_string()));
                }
                headers.push(("Allow".to_owned(), "GET".to_owned()));
                (200, resp.data)
            }
            Err(e) => {
                headers.insert(1, ("Link".to_owned(), self.root_link()));
                (e.status, e.to_value())
            }
        };
        headers.push(("Vary".to_owned(), "Origin".to_owned()));
        headers.push(("Content-Type".to_owned(), "application/json; charset=UTF-8".to_owned()));
        if status == 200 {
            if params.truthy("_embed") {
                self.embed_all(&mut data).await?;
            }
            if let Some(list) = params.list("_fields") {
                fields::filter(&mut data, &list);
            }
        }
        Ok(Reply { status, headers, body: php_json::encode(&data) })
    }

    /// Routes a request to its endpoint.
    pub(crate) async fn dispatch(&self, route: &str, params: &Params) -> Result<Outcome> {
        let route = if route.is_empty() { "/" } else { route.trim_end_matches('/') };
        let route = if route.is_empty() { "/" } else { route };
        let segments: Vec<&str> = route.trim_start_matches('/').split('/').collect();
        let context = params.get("context").unwrap_or("view");
        if !matches!(context, "view" | "embed" | "edit") {
            return Ok(Err(RestError::invalid_param("context", "context is not one of view, embed, and edit.", "rest_not_in_enum")));
        }
        match segments.as_slice() {
            [""] => Ok(Ok(Resp::item(self.index()))),
            ["wp", "v2"] => Ok(Ok(Resp::item(self.namespace_index()))),
            ["wp", "v2", "posts"] => self.posts_collection(posts::Kind::Post, params).await,
            ["wp", "v2", "posts", id] => self.posts_item(posts::Kind::Post, id, params).await,
            ["wp", "v2", "pages"] => self.posts_collection(posts::Kind::Page, params).await,
            ["wp", "v2", "pages", id] => self.posts_item(posts::Kind::Page, id, params).await,
            ["wp", "v2", "posts" | "pages", parent, "revisions" | "autosaves"] | ["wp", "v2", "posts" | "pages", parent, "revisions" | "autosaves", _] => {
                let kind = if segments[2] == "posts" { posts::Kind::Post } else { posts::Kind::Page };
                self.revisions_denied(kind, parent, segments[4]).await
            }
            ["wp", "v2", "media"] => self.media_collection(params).await,
            ["wp", "v2", "media", id] => self.media_item(id, params).await,
            ["wp", "v2", "categories"] => self.terms_collection("category", params).await,
            ["wp", "v2", "categories", id] => self.terms_item("category", id, params).await,
            ["wp", "v2", "tags"] => self.terms_collection("post_tag", params).await,
            ["wp", "v2", "tags", id] => self.terms_item("post_tag", id, params).await,
            ["wp", "v2", "users"] => self.users_collection(params).await,
            ["wp", "v2", "users", "me"] => Ok(Err(RestError::new("rest_not_logged_in", "You are not currently logged in.", 401))),
            ["wp", "v2", "users", id] => self.users_item(id, params).await,
            ["wp", "v2", "comments"] => self.comments_collection(params).await,
            ["wp", "v2", "comments", id] => self.comments_item(id, params).await,
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    /// `rest_url( $route )`.
    pub(crate) fn rest_url(&self, route: &str) -> String {
        let route = route.trim_start_matches('/');
        if self.site.uses_pretty_permalinks() {
            self.site.home_url(&format!("wp-json/{route}"))
        } else {
            self.site.home_url(&format!("?rest_route=/{route}"))
        }
    }

    /// `Link: <api root>; rel="https://api.w.org/"`.
    fn root_link(&self) -> String {
        format!("<{}>; rel=\"https://api.w.org/\"", self.rest_url("/"))
    }

    /// Fetches the data behind an `href` of a link, as the `embed` context.
    async fn fetch_href(&self, href: &str) -> Result<Outcome> {
        let (path, query) = rustpress_core::canonical::split_url(href);
        let route = match query.split('&').find_map(|p| p.strip_prefix("rest_route=")) {
            Some(route) => String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(route.as_bytes())).into_owned(),
            None => {
                let base = rustpress_core::canonical::split_url(&self.site.home_url("wp-json")).0;
                path.strip_prefix(&base).unwrap_or(&path).to_owned()
            }
        };
        let query: String = query.split('&').filter(|p| !p.starts_with("rest_route=") && !p.is_empty()).collect::<Vec<_>>().join("&");
        let mut params = Params::parse(&query);
        params.set("context", "embed");
        self.dispatch(&route, &params).await
    }

    /// `_embed`: replaces nothing, adds `_embedded` next to `_links` of every item.
    async fn embed_all(&self, data: &mut Value) -> Result<()> {
        match data {
            Value::Array(items) => {
                for item in items {
                    self.embed_one(item).await?;
                }
            }
            Value::Object(_) => self.embed_one(data).await?,
            _ => {}
        }
        Ok(())
    }

    async fn embed_one(&self, item: &mut Value) -> Result<()> {
        let Some(links) = item.get("_links").and_then(Value::as_object).cloned() else { return Ok(()) };
        let mut embedded = Map::new();
        for (rel, entries) in &links {
            if rel == "curies" {
                continue;
            }
            let Some(entries) = entries.as_array() else { continue };
            let mut results = Vec::new();
            for entry in entries {
                if entry.get("embeddable") != Some(&Value::Bool(true)) {
                    results.push(Value::Array(Vec::new()));
                    continue;
                }
                let href = entry.get("href").and_then(Value::as_str).unwrap_or_default();
                results.push(match self.fetch_href(href).await? {
                    Ok(resp) => resp.data,
                    Err(e) => e.to_value(),
                });
            }
            // `count( array_filter( $embeds ) )`: empty collections do not count.
            if results.iter().any(|r| !matches!(r, Value::Array(a) if a.is_empty())) {
                embedded.insert(rel.clone(), Value::Array(results));
            }
        }
        if !embedded.is_empty()
            && let Some(object) = item.as_object_mut() {
                object.insert("_embedded".into(), Value::Object(embedded));
            }
        Ok(())
    }

    /// `GET /wp-json/`.
    fn index(&self) -> Value {
        let s = &self.site.settings;
        let o = &self.site.options;
        let gmt_offset = {
            let v = o.string("gmt_offset", "0");
            if v.is_empty() { "0".to_owned() } else { v }
        };
        let mut routes = Map::new();
        routes.insert("/".into(), route_entry("", &["GET"], &self.rest_url("/")));
        routes.insert("/wp/v2".into(), route_entry("wp/v2", &["GET"], &self.rest_url("wp/v2")));
        for name in ["posts", "pages", "media", "categories", "tags", "users", "comments"] {
            routes.insert(format!("/wp/v2/{name}"), route_entry("wp/v2", &["GET"], &self.rest_url(&format!("wp/v2/{name}"))));
            routes.insert(format!("/wp/v2/{name}/(?P<id>[\\d]+)"), route_entry("wp/v2", &["GET"], &self.rest_url(&format!("wp/v2/{name}"))));
        }
        let mut out = Map::new();
        out.insert("name".into(), Value::from(s.blogname.clone()));
        out.insert("description".into(), Value::from(s.blogdescription.clone()));
        out.insert("url".into(), Value::from(s.siteurl.trim_end_matches('/').to_owned()));
        out.insert("home".into(), Value::from(s.home.trim_end_matches('/').to_owned()));
        out.insert("gmt_offset".into(), Value::from(gmt_offset));
        out.insert("timezone_string".into(), Value::from(o.string("timezone_string", "")));
        out.insert("page_for_posts".into(), Value::from(s.page_for_posts));
        out.insert("page_on_front".into(), Value::from(s.page_on_front));
        out.insert("show_on_front".into(), Value::from(if s.show_on_front == rustpress_core::ShowOnFront::Page { "page" } else { "posts" }));
        out.insert("namespaces".into(), Value::from(vec!["wp/v2"]));
        out.insert("authentication".into(), Value::Array(Vec::new()));
        out.insert("routes".into(), Value::Object(routes));
        out.insert("site_logo".into(), Value::from(o.u64("site_logo", 0)));
        out.insert("site_icon".into(), Value::from(o.u64("site_icon", 0)));
        out.insert("site_icon_url".into(), Value::from(""));
        out.insert("_links".into(), serde_json::json!({"help": [{"href": "https://developer.wordpress.org/rest-api/"}]}));
        Value::Object(out)
    }

    fn namespace_index(&self) -> Value {
        let mut routes = Map::new();
        for name in ["posts", "pages", "media", "categories", "tags", "users", "comments"] {
            routes.insert(format!("/wp/v2/{name}"), route_entry("wp/v2", &["GET"], &self.rest_url(&format!("wp/v2/{name}"))));
        }
        serde_json::json!({
            "namespace": "wp/v2",
            "routes": Value::Object(routes),
            "_links": {"up": [{"href": self.rest_url("/")}]},
        })
    }
}

fn route_entry(namespace: &str, methods: &[&str], href: &str) -> Value {
    serde_json::json!({
        "namespace": namespace,
        "methods": methods,
        "endpoints": [{"methods": methods, "args": {"context": {"default": "view", "required": false}}}],
        "_links": {"self": [{"href": href}]},
    })
}
