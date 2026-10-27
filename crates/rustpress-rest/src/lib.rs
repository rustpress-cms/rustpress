//! A read-only subset of the WordPress REST API (`/wp-json/wp/v2/...`) for
//! anonymous visitors: posts, pages, media, categories, tags, users and
//! comments, with `_fields`, `_embed` and the pagination headers.
//!
//! Everything that needs a login (the `edit` context, drafts, writes) answers
//! with the same 401 errors WordPress gives to anonymous requests.

mod abilities;
mod comments;
mod directory;
pub mod edit;
mod error;
mod fields;
mod media;
mod meta;
mod oembed;
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

pub use error::{Msg, RestError};
pub use params::Params;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] rustpress_db::Error),
    #[error(transparent)]
    Core(#[from] rustpress_core::Error),
    #[error(transparent)]
    Blocks(#[from] rustpress_blocks::Error),
    #[error(transparent)]
    Theme(#[from] rustpress_theme::Error),
    #[error(transparent)]
    Content(#[from] rustpress_content::content::ContentError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// What a route of a plugin or theme serves.
enum Custom {
    PostType(String),
    Taxonomy(String),
}

/// The result of one request: status, headers (without `Content-Length`) and the JSON text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// A file that is sent instead of the text (`wp-block-editor/v1/export`).
    pub bytes: Vec<u8>,
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
    /// A response that is sent as XML instead of JSON (`oembed/1.0/embed?format=xml`).
    pub xml: Option<String>,
    /// The status when it is not 200 (`201` after creating something).
    pub status: Option<u16>,
    /// The `Allow` header of an authenticated response (default: `GET`).
    pub allow: Option<String>,
    /// The `Location` header of a created item.
    pub location: Option<String>,
    /// A file to download: its bytes and the name it is saved under.
    pub download: Option<(Vec<u8>, String)>,
}

impl Resp {
    pub fn item(data: Value) -> Self {
        Self { data, total: None, total_pages: None, link: None, alternate: None, xml: None, status: None, allow: None, location: None, download: None }
    }
}

pub(crate) type Outcome = std::result::Result<Resp, RestError>;

pub struct Rest<'a> {
    pub(crate) site: &'a Site,
    pub(crate) theme: &'a Theme,
    pub(crate) registry: &'a Registry,
    pub(crate) blank: QueryResult,
    /// The render state that PHP keeps in static variables for the whole request (unique ids, ...).
    pub(crate) state: std::sync::Mutex<rustpress_blocks::env::State>,
    /// The signed-in user of this request, when there is one.
    pub(crate) access: Option<edit::Access<'a>>,
    /// The body of a request that is not a plain GET.
    pub(crate) request: Option<edit::Request>,
    /// The patterns of the registry, evaluated once per request.
    pub(crate) patterns: std::sync::OnceLock<Vec<rustpress_theme::registered_patterns::PatternEntry>>,
    /// The host the request was sent to (`$_SERVER['HTTP_HOST']`).
    pub(crate) host: String,
    /// The patterns of the pattern directory, loaded for the routes that list patterns.
    pub(crate) remote_patterns: std::sync::OnceLock<Vec<rustpress_theme::registered_patterns::PatternEntry>>,
    /// The request comes from a screen of the administration (`is_admin()`): the preloads of the editors.
    pub(crate) admin: bool,
}

impl<'a> Rest<'a> {
    pub fn new(site: &'a Site, theme: &'a Theme, registry: &'a Registry) -> Self {
        Self { site, theme, registry, blank: QueryResult::blank(), state: Default::default(), access: None, request: None, patterns: Default::default(), host: String::new(), remote_patterns: Default::default(), admin: false }
    }

    /// The host of the request: client-side media processing is on for `localhost` and for HTTPS.
    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.to_owned();
        self
    }

    /// A request of a screen of the administration (`is_admin()` is true).
    pub fn in_admin(mut self) -> Self {
        self.admin = true;
        self
    }

    /// `wp_is_client_side_media_processing_enabled()`.
    fn client_side_media(&self) -> bool {
        let host = self.host.split(':').next().unwrap_or_default().to_ascii_lowercase();
        self.site.settings.siteurl.starts_with("https") || host == "localhost" || host.ends_with(".localhost")
    }

    /// Handles `GET <route>?<query>`; `route` is the part after `/wp-json` (`/wp/v2/posts`).
    pub async fn handle(&self, route: &str, query: &str) -> Result<Reply> {
        // Blocks that link to "the current page" (query pagination) see the request URI.
        let mut blank = QueryResult::blank();
        if self.site.uses_pretty_permalinks() {
            blank.url_path = format!("/wp-json{route}");
            blank.raw_query = query.to_owned();
        } else {
            blank.url_path = "/".to_owned();
            blank.raw_query = if query.is_empty() { format!("rest_route={route}") } else { format!("rest_route={route}&{query}") };
        }
        blank.http_host = Some(self.host.clone()).filter(|h| !h.is_empty());
        let scoped = Rest { site: self.site, theme: self.theme, registry: self.registry, blank, state: Default::default(), access: None, request: None, patterns: Default::default(), host: self.host.clone(), remote_patterns: Default::default(), admin: self.admin };
        // The terms of the posts of a list are read once for the request (`update_object_term_cache()`).
        rustpress_db::Db::with_term_cache(scoped.handle_request(route, query)).await
    }

    /// Handles a request of a signed-in user (or with a body): `method`, the route after `/wp-json`, the
    /// query string and the body.
    pub async fn handle_as(&self, route: &str, query: &str, access: Option<edit::Access<'_>>, request: edit::Request) -> Result<Reply> {
        let mut blank = QueryResult::blank();
        if self.site.uses_pretty_permalinks() {
            blank.url_path = format!("/wp-json{route}");
            blank.raw_query = query.to_owned();
        } else {
            blank.url_path = "/".to_owned();
            blank.raw_query = if query.is_empty() { format!("rest_route={route}") } else { format!("rest_route={route}&{query}") };
        }
        blank.http_host = Some(self.host.clone()).filter(|h| !h.is_empty());
        // Content rendered for a signed-in user is rendered as that user (private posts in query blocks).
        if let Some(access) = &access {
            let user = access.user.clone();
            let (login, name, email) = (user.login.clone(), user.display_name.clone(), user.email.clone());
            let nonce = access.nonce.clone();
            let viewer = rustpress_core::Viewer::new(user.id, move |capability| user.can(capability));
            blank.viewer = Some(viewer.with_identity(&login, &name, &email, false, move |action| nonce.as_ref().map(|n| n(action)).unwrap_or_default()));
        }
        let scoped = Rest { site: self.site, theme: self.theme, registry: self.registry, blank, state: Default::default(), access, request: Some(request), patterns: Default::default(), host: self.host.clone(), remote_patterns: Default::default(), admin: self.admin };
        // The terms of the posts of a list are read once for the request (`update_object_term_cache()`).
        rustpress_db::Db::with_term_cache(scoped.handle_request(route, query)).await
    }

    async fn handle_request(&self, route: &str, query: &str) -> Result<Reply> {
        let params = Params::parse(query);
        // The registry of patterns includes those of the pattern directory.
        if route.starts_with("/wp/v2/block-patterns/patterns") {
            let local = self.theme.registered_patterns();
            let _ = self.remote_patterns.set(self.remote_pattern_entries(&local).await);
        }
        let outcome = match &self.request {
            Some(_) => match self.dispatch_edit(route, &params).await? {
                Some(outcome) => outcome,
                // A write that no handler takes is no read: WordPress has no route for it.
                None if !matches!(self.method(), "GET" | "HEAD") => Err(crate::error::RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)),
                None => self.dispatch(route, &params).await?,
            },
            None => self.dispatch(route, &params).await?,
        };
        let mut headers = vec![
            ("X-Robots-Tag".to_owned(), "noindex".to_owned()),
            ("X-Content-Type-Options".to_owned(), "nosniff".to_owned()),
            ("Access-Control-Expose-Headers".to_owned(), "X-WP-Total, X-WP-TotalPages, Link".to_owned()),
            ("Access-Control-Allow-Headers".to_owned(), "Authorization, X-WP-Nonce, Content-Disposition, Content-MD5, Content-Type".to_owned()),
        ];
        let mut xml_body = None;
        let mut download: Option<(Vec<u8>, String)> = None;
        // The headers of the response itself, which `?_envelope` moves into the body.
        let mut response_headers: Vec<(String, String)> = Vec::new();
        let (mut status, mut data) = match outcome {
            Ok(resp) => {
                xml_body = resp.xml.clone();
                download = resp.download.clone();
                let allow = resp.allow.clone().unwrap_or_else(|| "GET".to_owned());
                let response_link = resp.link.clone().or_else(|| resp.alternate.as_ref().map(|url| format!("<{url}>; rel=\"alternate\"; type=text/html")));
                if let Some(total) = resp.total {
                    response_headers.push(("X-WP-Total".to_owned(), total.to_string()));
                }
                if let Some(pages) = resp.total_pages {
                    response_headers.push(("X-WP-TotalPages".to_owned(), pages.to_string()));
                }
                if let Some(link) = &response_link {
                    response_headers.push(("Link".to_owned(), link.clone()));
                }
                // An empty list sends no header (`rest_send_allow_header()` with nothing allowed).
                if !allow.is_empty() {
                    response_headers.push(("Allow".to_owned(), allow.clone()));
                }
                let link = response_link.unwrap_or_else(|| self.root_link());
                headers.insert(1, ("Link".to_owned(), link));
                if let Some(total) = resp.total {
                    headers.push(("X-WP-Total".to_owned(), total.to_string()));
                }
                if let Some(pages) = resp.total_pages {
                    headers.push(("X-WP-TotalPages".to_owned(), pages.to_string()));
                }
                if !allow.is_empty() {
                    headers.push(("Allow".to_owned(), allow));
                }
                if let Some(location) = &resp.location {
                    headers.push(("Location".to_owned(), location.clone()));
                }
                (resp.status.unwrap_or(200), resp.data)
            }
            Err(e) => {
                headers.insert(1, ("Link".to_owned(), self.root_link()));
                if e.passed_permission_check() {
                    headers.push(("Allow".to_owned(), self.allow_for(route)));
                    response_headers.push(("Allow".to_owned(), self.allow_for(route)));
                }
                (e.status, e.to_value(self.site))
            }
        };
        headers.push(("Vary".to_owned(), "Origin".to_owned()));
        headers.push(("Content-Type".to_owned(), if xml_body.is_some() { "text/xml; charset=UTF-8" } else { "application/json; charset=UTF-8" }.to_owned()));
        // An oEmbed error is XML, too, when XML was asked for.
        if xml_body.is_none() && status != 200 && route == "/oembed/1.0/embed" && params.get("format") == Some("xml") && let Value::Object(map) = &data {
            xml_body = Some(oembed::xml_of(map));
            if let Some(h) = headers.iter_mut().find(|(n, _)| n == "Content-Type") {
                h.1 = "text/xml; charset=UTF-8".to_owned();
            }
        }
        if let Some((bytes, name)) = download {
            headers.retain(|(n, _)| n != "Content-Type");
            headers.push(("Content-Disposition".to_owned(), format!("attachment; filename={name}")));
            headers.push(("Content-Type".to_owned(), "application/zip".to_owned()));
            return Ok(Reply { status, headers, body: String::new(), bytes });
        }
        if let Some(xml) = xml_body {
            return Ok(Reply { status, headers, body: xml, bytes: Vec::new() });
        }
        if status == 200 || status == 201 {
            if let Some(rels) = embed_param(&params) {
                self.embed_all(&mut data, rels.as_deref()).await?;
            }
            if self.access.is_some() {
                self.refine_hints(&mut data).await;
            }
            if let Some(list) = params.list("_fields") {
                // The links are no field: `rest_filter_response_fields()` runs before `response_to_data()` adds them.
                // The index decides itself which ones it has.
                let index_links = if route.trim_end_matches('/').is_empty() { data.as_object_mut().and_then(|o| o.remove("_links")) } else { None };
                fields::filter(&mut data, &list);
                if let (Some(links), Some(object)) = (index_links, data.as_object_mut()) {
                    object.insert("_links".into(), links);
                }
            }
        }
        // `?_envelope`: status and headers move into the body, the response itself is a plain 200.
        if params.has("_envelope") {
            let mut inner = Map::new();
            for (name, value) in &response_headers {
                let json = if name.starts_with("X-WP-") { value.parse::<u64>().map_or_else(|_| Value::from(value.clone()), Value::from) } else { Value::from(value.clone()) };
                inner.insert(name.clone(), json);
            }
            data = serde_json::json!({"body": data, "status": status, "headers": Value::Object(inner)});
            status = 200;
            headers.retain(|(n, _)| !matches!(n.as_str(), "X-WP-Total" | "X-WP-TotalPages" | "Allow"));
            let root = self.root_link();
            if let Some(h) = headers.iter_mut().find(|(n, _)| n == "Link") {
                h.1 = root;
            }
        }
        // `?_jsonp=callback`.
        if let Some(callback) = params.get("_jsonp").filter(|c| !c.is_empty()) {
            if callback.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
                if let Some(h) = headers.iter_mut().find(|(n, _)| n == "Content-Type") {
                    h.1 = "application/javascript; charset=UTF-8".to_owned();
                }
                return Ok(Reply { status, headers, body: format!("/**/{callback}({})", php_json::encode(&data)), bytes: Vec::new() });
            }
            let error = RestError::new("rest_callback_invalid", "Invalid JSONP callback function.", 400).to_value(self.site);
            return Ok(Reply { status: 400, headers, body: php_json::encode(&error), bytes: Vec::new() });
        }
        Ok(Reply { status, headers, body: php_json::encode(&data), bytes: Vec::new() })
    }

    /// Routes a request to its endpoint.
    pub(crate) async fn dispatch(&self, route: &str, params: &Params) -> Result<Outcome> {
        let route = if route.is_empty() { "/" } else { route.trim_end_matches('/') };
        let route = if route.is_empty() { "/" } else { route };
        let segments: Vec<&str> = route.trim_start_matches('/').split('/').collect();
        let context = params.get("context").unwrap_or("view");
        if !matches!(context, "view" | "embed" | "edit") {
            return Ok(Err(RestError::invalid_param("context", Msg::new("%1$s is not one of %2$l.").with(&["context"]).with_list(&["view", "embed", "edit"]), "rest_not_in_enum")));
        }
        match segments.as_slice() {
            [""] => Ok(Ok(Resp::item(self.index(params)))),
            _ if namespaces().iter().any(|ns| *ns == segments.join("/")) => Ok(Ok(Resp::item(self.namespace_index(&segments.join("/"))))),
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
            ["wp", "v2", "categories"] => self.terms_collection(&terms::Tax::category(), params).await,
            ["wp", "v2", "categories", id] => self.terms_item(&terms::Tax::category(), id, params).await,
            ["wp", "v2", "tags"] => self.terms_collection(&terms::Tax::tag(), params).await,
            ["wp", "v2", "tags", id] => self.terms_item(&terms::Tax::tag(), id, params).await,
            ["wp", "v2", "wp_pattern_category"] => self.terms_collection(&terms::Tax::pattern_category(), params).await,
            ["wp", "v2", "wp_pattern_category", id] => self.terms_item(&terms::Tax::pattern_category(), id, params).await,
            ["wp", "v2", "users"] => self.users_collection(params).await,
            ["wp", "v2", "users", "me"] => Ok(Err(RestError::new("rest_not_logged_in", "You are not currently logged in.", 401))),
            ["wp", "v2", "users", id] => self.users_item(id, params).await,
            ["wp", "v2", "comments"] => self.comments_collection(params).await,
            ["wp", "v2", "comments", id] => self.comments_item(id, params).await,
            ["wp", "v2", "types"] => Ok(self.types_collection(params)),
            ["wp", "v2", "types", name] => Ok(self.types_item(name, params)),
            ["wp", "v2", "taxonomies"] => Ok(self.taxonomies_collection(params)),
            ["wp", "v2", "taxonomies", name] => Ok(self.taxonomies_item(name, params)),
            ["wp", "v2", "statuses"] => Ok(self.statuses_collection(params)),
            ["wp", "v2", "statuses", slug] => Ok(self.statuses_item(slug, params)),
            ["wp", "v2", "search"] => self.search(params).await,
            ["oembed", "1.0", "embed"] => self.oembed_embed(params).await,
            ["oembed", "1.0", "proxy"] => Ok(self.oembed_proxy()),
            ["wp", "v2", base @ ("blocks" | "navigation")] | ["wp", "v2", base @ ("blocks" | "navigation"), _] => {
                let info = meta::core_post_type(base);
                match segments.get(3) {
                    None => self.posts_collection(posts::Kind::Custom(&info), params).await,
                    Some(id) => self.posts_item(posts::Kind::Custom(&info), id, params).await,
                }
            }
            ["wp", "v2", "font-families" | "font-collections", ..] => self.fonts_route(&segments, "GET", &edit::Args::default(), params).await,
            ["wp-abilities", "v1", ..] => self.abilities_route(&segments, "GET", &edit::Args::default(), params).await,
            ["wp-block-editor", "v1", "url-details"] if params.get("url").is_none() => Ok(Err(RestError::missing_params(&["url"]))),
            ["wp-block-editor", "v1", "url-details"] => Ok(Err(RestError::new("rest_cannot_view_url_details", "Sorry, you are not allowed to process remote URLs.", 401))),
            ["wp-block-editor", "v1", "export"] => Ok(Err(RestError::new("rest_cannot_export_templates", "Sorry, you are not allowed to export templates and template parts.", 401))),
            ["wp-block-editor", "v1", "navigation-fallback"] => Ok(Err(RestError::new("rest_cannot_create", "Sorry, you are not allowed to create Navigation Menus as this user.", 401))),
            ["wp-site-health", "v1", "tests", _] | ["wp-site-health", "v1", "directory-sizes"] => Ok(Err(RestError::new("rest_forbidden", "Sorry, you are not allowed to do that.", 401))),
            ["wp", "v2", rest @ ..] if meta::denied(rest, params).is_some() => Ok(Err(meta::denied(rest, params).unwrap_or_else(|| RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)))),
            ["wp", "v2", base] | ["wp", "v2", base, _] if self.custom_route(base).is_some() => self.custom_route_dispatch(base, segments.get(3).copied(), params).await,
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    /// The post type or taxonomy of a custom route (`wp/v2/book`, `wp/v2/genre`).
    fn custom_route(&self, base: &str) -> Option<Custom> {
        if let Some(info) = self.site.types.post_types.iter().find(|t| t.show_in_rest && t.rest_base() == base) {
            return Some(Custom::PostType(info.name.clone()));
        }
        self.site.types.taxonomies.iter().find(|t| t.show_in_rest && t.rest_base() == base).map(|t| Custom::Taxonomy(t.name.clone()))
    }

    async fn custom_route_dispatch(&self, base: &str, id: Option<&str>, params: &Params) -> Result<Outcome> {
        match self.custom_route(base) {
            Some(Custom::PostType(name)) => {
                let Some(info) = self.site.types.post_type(&name) else { return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))) };
                match id {
                    None => self.posts_collection(posts::Kind::Custom(info), params).await,
                    Some(id) => self.posts_item(posts::Kind::Custom(info), id, params).await,
                }
            }
            Some(Custom::Taxonomy(name)) => {
                let Some(tax) = self.tax(&name) else { return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))) };
                match id {
                    None => self.terms_collection(&tax, params).await,
                    Some(id) => self.terms_item(&tax, id, params).await,
                }
            }
            None => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    /// `rest_url( $route )`.
    pub(crate) fn rest_url(&self, route: &str) -> String {
        self.site.rest_url(route)
    }

    /// `Link: <api root>; rel="https://api.w.org/"`.
    pub fn root_link(&self) -> String {
        format!("<{}>; rel=\"https://api.w.org/\"", self.rest_url("/"))
    }

    /// Fetches the data behind an `href` of a link, as the `embed` context.
    async fn fetch_href(&self, href: &str) -> Result<Outcome> {
        let (path, query) = rustpress_core::canonical::split_url(href);
        let route = match query.split('&').find_map(|p| p.strip_prefix("rest_route=")) {
            Some(route) => String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(route.as_bytes())).into_owned(),
            None => {
                let base = rustpress_core::canonical::split_url(&self.site.home_url(&format!("{}wp-json", self.site.root()))).0;
                path.strip_prefix(&base).unwrap_or(&path).to_owned()
            }
        };
        let query: String = query.split('&').filter(|p| !p.starts_with("rest_route=") && !p.is_empty()).collect::<Vec<_>>().join("&");
        let mut params = Params::parse(&query);
        params.set("context", "embed");
        // `WP_REST_Server::embed_links()`: embedded collections are not cut at the default of 10.
        if params.get("per_page").is_none_or(|p| p.is_empty() || p == "0") {
            params.set("per_page", "100");
        }
        // What a visitor may not see is read as the signed-in user (the author of a comment, for instance).
        let outcome = self.dispatch(&route, &params).await?;
        if outcome.is_err()
            && self.access.is_some()
            && matches!(self.method(), "GET" | "HEAD")
            && let Some(outcome) = Box::pin(self.dispatch_edit(&route, &params)).await?
        {
            return Ok(outcome);
        }
        Ok(outcome)
    }

    /// `_embed`: replaces nothing, adds `_embedded` next to `_links` of every item.
    async fn embed_all(&self, data: &mut Value, rels: Option<&[String]>) -> Result<()> {
        match data {
            Value::Array(items) => {
                for item in items {
                    self.embed_one(item, rels).await?;
                }
            }
            Value::Object(_) => self.embed_one(data, rels).await?,
            _ => {}
        }
        Ok(())
    }

    async fn embed_one(&self, item: &mut Value, rels: Option<&[String]>) -> Result<()> {
        let Some(links) = item.get("_links").and_then(Value::as_object).cloned() else { return Ok(()) };
        let mut embedded = Map::new();
        for (rel, entries) in &links {
            if rel == "curies" || rels.is_some_and(|wanted| !wanted.iter().any(|w| w == rel)) {
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
                    Err(e) => e.to_value(self.site),
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

    /// `GET /wp-json/`. Its links depend on `_fields` (`get_index()`), they stay when the fields are filtered.
    fn index(&self, params: &Params) -> Value {
        let s = &self.site.settings;
        let o = &self.site.options;
        // `get_option( 'gmt_offset' )`: the stored text, unless a time zone name makes
        // `wp_timezone_override_offset()` compute the number.
        let gmt_offset = if o.string("timezone_string", "").is_empty() {
            Value::from(o.string("gmt_offset", "0"))
        } else {
            let v = o.string("gmt_offset", "0");
            let n = v.trim().parse::<f64>().unwrap_or(0.0);
            // A whole number prints without a fraction.
            if n.fract() == 0.0 { Value::from(n as i64) } else { Value::from(n) }
        };
        let routes = self.routes_of(None);
        let mut out = Map::new();
        out.insert("name".into(), Value::from(s.blogname.clone()));
        out.insert("description".into(), Value::from(s.blogdescription.clone()));
        out.insert("url".into(), Value::from(s.siteurl.trim_end_matches('/').to_owned()));
        out.insert("home".into(), Value::from(s.home.trim_end_matches('/').to_owned()));
        out.insert("gmt_offset".into(), gmt_offset);
        out.insert("timezone_string".into(), Value::from(o.string("timezone_string", "")));
        out.insert("page_for_posts".into(), Value::from(s.page_for_posts));
        out.insert("page_on_front".into(), Value::from(s.page_on_front));
        out.insert("show_on_front".into(), Value::from(if s.show_on_front == rustpress_core::ShowOnFront::Page { "page" } else { "posts" }));
        out.insert("namespaces".into(), Value::from(namespaces()));
        // `wp_is_application_passwords_available()`: over HTTPS, or on a local site.
        let available = s.siteurl.starts_with("https") || self.site.environment_type() == "local";
        out.insert(
            "authentication".into(),
            if available { serde_json::json!({"application-passwords": {"endpoints": {"authorization": self.site.site_url("wp-admin/authorize-application.php")}}}) } else { Value::Array(Vec::new()) },
        );
        out.insert("routes".into(), Value::Object(routes));
        // The settings of client-side media processing, for who may upload.
        if self.client_side_media() && self.can("upload_files") {
            let mut sizes = Map::new();
            for (name, width, height, crop) in rustpress_content::media::registered_sizes(o) {
                sizes.insert(name, serde_json::json!({"width": width, "height": height, "crop": crop}));
            }
            out.insert("image_sizes".into(), Value::Object(sizes));
            out.insert("image_size_threshold".into(), Value::from(2560));
            out.insert("image_strip_meta".into(), Value::from(true));
            out.insert("image_max_bit_depth".into(), Value::from(16));
        }
        let mut fields = params.list("_fields").unwrap_or_default();
        if fields.is_empty() {
            fields.push("_links".to_owned());
        }
        if embed_param(params).is_some() {
            fields.push("_embedded".to_owned());
        }
        let included = |name: &str| fields::is_included(name, &fields);
        let all_links = included("_links") || included("_embedded");
        let mut links = Map::new();
        let mut featured = Vec::new();
        if all_links {
            links.insert("help".into(), serde_json::json!([{"href": "https://developer.wordpress.org/rest-api/"}]));
            // `add_active_theme_link_to_index()`: for who may switch themes or edit.
            if self.can("switch_themes") || self.can("edit_posts") {
                links.insert("wp:active-theme".into(), serde_json::json!([{"href": self.rest_url(&format!("wp/v2/themes/{}", self.theme.stylesheet))}]));
            }
        }
        // `add_site_logo_to_index()` and `add_site_icon_to_index()`.
        out.insert("site_logo".into(), Value::from(o.u64("site_logo", 0)));
        if all_links || included("site_logo") {
            let id = o.u64("site_logo", 0);
            if id != 0 {
                featured.push(serde_json::json!({"embeddable": true, "type": "site_logo", "href": self.rest_url(&format!("wp/v2/media/{id}"))}));
            }
        }
        out.insert("site_icon".into(), Value::from(o.u64("site_icon", 0)));
        out.insert("site_icon_url".into(), Value::from(""));
        if all_links || included("site_icon") || included("site_icon_url") {
            let id = o.u64("site_icon", 0);
            if id != 0 {
                featured.push(serde_json::json!({"embeddable": true, "type": "site_icon", "href": self.rest_url(&format!("wp/v2/media/{id}"))}));
            }
        }
        if !featured.is_empty() {
            links.insert("wp:featuredmedia".into(), Value::Array(featured));
        }
        if links.keys().any(|k| k.starts_with("wp:")) {
            links.insert("curies".into(), serde_json::json!([{"name": "wp", "href": "https://api.w.org/{rel}", "templated": true}]));
        }
        if !links.is_empty() {
            out.insert("_links".into(), Value::Object(links));
        }
        Value::Object(out)
    }

    /// The routes of the reference, optionally of one namespace, with the links of this site.
    fn routes_of(&self, namespace: Option<&str>) -> Map<String, Value> {
        let mut routes = Map::new();
        for (route, entry) in golden_routes() {
            // The routes for client-side media processing exist on `localhost` and over HTTPS only.
            if !self.client_side_media() && route.starts_with("/wp/v2/media/") && (route.ends_with("/sideload") || route.ends_with("/finalize")) {
                continue;
            }
            if namespace.is_some_and(|ns| entry.get("namespace").and_then(Value::as_str) != Some(ns)) {
                continue;
            }
            let mut entry = entry.clone();
            if let Some(links) = entry.pointer_mut("/_links/self").and_then(Value::as_array_mut) {
                for link in links {
                    if let Some(href) = link.get("href").and_then(Value::as_str).and_then(|h| h.strip_prefix("@REST@")).map(str::to_owned) {
                        link["href"] = Value::from(self.rest_url(if href == "/" { "/" } else { href.trim_start_matches('/') }));
                    }
                }
            }
            routes.insert(route.clone(), entry);
        }
        routes
    }

    /// `rest_handle_options_request()`: the description of the route a path matches (`get_data_for_route()` in the
    /// `help` context, with the schema), and the methods of its endpoints.
    pub(crate) fn route_help(&self, path: &str) -> Option<(Value, Vec<String>)> {
        let (route, entry) = route_patterns().iter().find(|(_, pattern)| pattern.is_match(path)).and_then(|(route, _)| golden_routes().get(route).map(|e| (route, e)))?;
        let schemas = route_schemas();
        let schema = schemas.get("routes").and_then(|r| r.get(route)).and_then(Value::as_u64).and_then(|i| schemas.get("schemas")?.get(i as usize)).cloned();
        let mut data = Map::new();
        for (key, value) in entry.as_object()? {
            if key == "_links"
                && let Some(schema) = &schema
            {
                data.insert("schema".into(), self.with_rest_urls(schema.clone()));
            }
            data.insert(key.clone(), self.with_rest_urls(value.clone()));
        }
        if !data.contains_key("_links")
            && let Some(schema) = schema
        {
            data.insert("schema".into(), self.with_rest_urls(schema));
        }
        let methods = entry.get("methods").and_then(Value::as_array).map(|m| m.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default();
        Some((Value::Object(data), methods))
    }

    /// Puts the address of this site's API where the baked data says `@REST@`.
    fn with_rest_urls(&self, value: Value) -> Value {
        match value {
            Value::String(s) => match s.strip_prefix("@REST@") {
                Some(path) => Value::from(self.rest_url(if path.is_empty() || path == "/" { "/" } else { path.trim_start_matches('/') })),
                None => Value::String(s),
            },
            Value::Array(items) => Value::Array(items.into_iter().map(|v| self.with_rest_urls(v)).collect()),
            Value::Object(map) => Value::Object(map.into_iter().map(|(k, v)| (k, self.with_rest_urls(v))).collect()),
            other => other,
        }
    }

    /// `GET /wp-json/<namespace>`.
    pub(crate) fn namespace_index(&self, namespace: &str) -> Value {
        serde_json::json!({
            "namespace": namespace,
            "routes": Value::Object(self.routes_of(Some(namespace))),
            "_links": {"up": [{"href": self.rest_url("/")}]},
        })
    }
}

/// The routes WordPress lists in its index (`dev/gen-rest-index.py` takes them from the reference).
fn golden_routes() -> &'static Map<String, Value> {
    static ROUTES: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    ROUTES.get_or_init(|| serde_json::from_str(include_str!("index_routes.json")).expect("the route table is valid JSON"))
}

/// The patterns of the routes, in the order of the index (`dev/gen-rest-index.py`).
fn route_patterns() -> &'static [(String, regex::Regex)] {
    static PATTERNS: std::sync::OnceLock<Vec<(String, regex::Regex)>> = std::sync::OnceLock::new();
    PATTERNS.get_or_init(|| golden_routes().keys().filter_map(|route| regex::Regex::new(&format!("(?i)^{route}$")).ok().map(|re| (route.clone(), re))).collect())
}

/// The schema of every route that has one (`dev/gen-rest-schemas.py`): `{"schemas": [...], "routes": {route: index}}`.
fn route_schemas() -> &'static Value {
    static SCHEMAS: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    SCHEMAS.get_or_init(|| serde_json::from_str(include_str!("route_schemas.json")).expect("the route schemas are valid JSON"))
}

/// The namespaces in the order of the index.
pub(crate) fn namespaces() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for entry in golden_routes().values() {
        if let Some(ns) = entry.get("namespace").and_then(Value::as_str).filter(|ns| !ns.is_empty())
            && !out.iter().any(|seen| seen == ns)
        {
            out.push(ns.to_owned());
        }
    }
    out
}

/// `rest_parse_embed_param()`: `None` without `_embed`; `Some(None)` embeds every relation (also for `0`, `1`,
/// `true` and an empty value, which PHP reads as "all"), otherwise the relations named.
fn embed_param(params: &Params) -> Option<Option<Vec<String>>> {
    if !params.has("_embed") {
        return None;
    }
    let value = params.get("_embed").unwrap_or_default();
    if matches!(value, "" | "0" | "1" | "true") {
        return Some(None);
    }
    let rels: Vec<String> = value.split([',', ' ', '\t', '\n', '\r']).filter(|r| !r.is_empty()).map(str::to_owned).collect();
    Some(if rels.is_empty() { None } else { Some(rels) })
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_route_pattern_compiles() {
        assert_eq!(super::route_patterns().len(), super::golden_routes().len());
    }

    #[test]
    fn options_paths_find_their_route() {
        let find = |path: &str| super::route_patterns().iter().find(|(_, re)| re.is_match(path)).map(|(route, _)| route.clone());
        assert_eq!(find("/wp/v2/settings").as_deref(), Some("/wp/v2/settings"));
        assert_eq!(find("/wp/v2/posts/12").as_deref(), Some(r"/wp/v2/posts/(?P<id>[\d]+)"));
        assert_eq!(find("/wp/v2/templates/twentytwentyfive//index").as_deref(), Some(r#"/wp/v2/templates/(?P<id>([^\/:<>\*\?"\|]+(?:\/[^\/:<>\*\?"\|]+)?)[\/\w%-]+)"#));
    }
}
