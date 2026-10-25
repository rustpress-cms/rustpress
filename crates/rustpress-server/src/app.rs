//! Application state and request handling.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, Method, Request, Response, StatusCode, header};
use rustpress_blocks::Registry;
use rustpress_core::{Site, SitemapOutput};
use rustpress_rest::Rest;
use rustpress_db::Db;
use rustpress_theme::{Feed, Theme, render_document, render_feed};
use tokio::sync::{Mutex, RwLock};
use tower_http::trace::TraceLayer;

use crate::Result;
use crate::config::Config;
use crate::statics::Statics;

/// Everything derived from the database at one point in time.
pub struct Loaded {
    pub site: Site,
    pub theme: Theme,
    pub registry: Registry,
    loaded_at: Instant,
}

pub struct App {
    config: Config,
    db: Db,
    statics: Statics,
    loaded: RwLock<Arc<Loaded>>,
    reload: Mutex<()>,
}

impl App {
    pub async fn new(config: Config) -> Result<Arc<Self>> {
        let db = Db::connect(&config.database_url, &config.table_prefix).await?;
        let loaded = Self::load(&config, &db).await?;
        Ok(Arc::new(Self { statics: Statics::new(&config.wp_root), config, db, loaded: RwLock::new(Arc::new(loaded)), reload: Mutex::new(()) }))
    }

    async fn load(config: &Config, db: &Db) -> Result<Loaded> {
        let site = Site::load_at(db.clone(), &config.wp_root).await?;
        let theme = Theme::load(&site, &config.wp_root).await?;
        let registry = Registry::new(config.wp_root.join("wp-includes").join("blocks"));
        Ok(Loaded { site, theme, registry, loaded_at: Instant::now() })
    }

    /// The current snapshot; options and theme are re-read once it is older than `reload_secs`.
    /// A failed reload keeps serving the previous snapshot.
    pub async fn current(&self) -> Arc<Loaded> {
        let snapshot = self.loaded.read().await.clone();
        if snapshot.loaded_at.elapsed() < Duration::from_secs(self.config.reload_secs) {
            return snapshot;
        }
        let _guard = self.reload.lock().await;
        let snapshot = self.loaded.read().await.clone();
        if snapshot.loaded_at.elapsed() < Duration::from_secs(self.config.reload_secs) {
            return snapshot;
        }
        match Self::load(&self.config, &self.db).await {
            Ok(fresh) => {
                let fresh = Arc::new(fresh);
                *self.loaded.write().await = fresh.clone();
                fresh
            }
            Err(e) => {
                tracing::error!(error = %e, "reload failed, keeping previous snapshot");
                snapshot
            }
        }
    }

    pub fn router(self: &Arc<Self>) -> Router {
        Router::new().fallback(handle).with_state(self.clone()).layer(TraceLayer::new_for_http())
    }
}

async fn handle(axum::extract::State(app): axum::extract::State<Arc<App>>, request: Request<Body>) -> Response<Body> {
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        let mut response = text(StatusCode::METHOD_NOT_ALLOWED, "RustPress is read-only: only GET and HEAD are supported.\n");
        response.headers_mut().insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
        return response;
    }
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    if let Some(response) = app.statics.serve(request).await {
        return response;
    }
    match page(&app, &uri, &headers).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!(error = %e, path = uri.path(), "request failed");
            text(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error\n")
        }
    }
}

async fn page(app: &App, uri: &axum::http::Uri, request_headers: &axum::http::HeaderMap) -> Result<Response<Body>> {
    let (path, query) = (uri.path(), uri.query().unwrap_or(""));
    let loaded = app.current().await;
    if let Some((route, rest_query)) = rest_route(&loaded.site.settings.home_path(), path, query) {
        let rest = Rest::new(&loaded.site, &loaded.theme, &loaded.registry);
        let reply = rest.handle(&route, &rest_query).await?;
        return Ok(rest_response(reply));
    }
    let result = loaded.site.query(path, query).await?;
    // Physical PHP endpoints (wp-login.php, xmlrpc.php, ...) do not exist here; they get the
    // 404 page rather than a canonical redirect. Only `/index.php` is WordPress' own alias for `/`.
    let php_endpoint = path.ends_with(".php") && path != "/index.php";
    let canonical = if php_endpoint { None } else { loaded.site.canonical_redirect_for_request(path, query, &result).await? };
    if let Some(target) = canonical {
        // A relative `Location` (RFC 9110) keeps the client's own scheme and host, and
        // never trusts the `Host` header.
        // `WP::send_headers()` ran before the redirect and already set the content type.
        let mut response = redirect(&target);
        singular_headers(&mut response, &result, &loaded);
        // `WP::handle_404()` ran before the redirect and already asked for no caching.
        if result.flags.is_404 && !result.parse_error_404 {
            no_cache(response.headers_mut());
        }
        if !result.flags.is_feed {
            response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
        }
        return Ok(response);
    }
    if result.flags.is_feed {
        let request_uri = uri.path_and_query().map_or(path, |p| p.as_str());
        let Some(feed) = render_feed(&loaded.theme, &loaded.site, &result, &loaded.registry, request_uri).await? else {
            return Ok(text(StatusCode::NOT_FOUND, "This feed is not available.\n"));
        };
        let mut response = feed_response(&feed, request_headers);
        singular_headers(&mut response, &result, &loaded);
        return Ok(with_api_link(response, &loaded));
    }
    let mut result = result;
    if result.flags.is_robots {
        return Ok(with_api_link(typed(StatusCode::OK, "text/plain; charset=utf-8", loaded.site.robots_txt()), &loaded));
    }
    if result.flags.is_favicon {
        let icon = loaded.site.site_url("wp-includes/images/w-logo-gray-white-bg.png");
        let mut response = redirect(&icon);
        *response.status_mut() = StatusCode::FOUND;
        response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
        return Ok(with_api_link(response, &loaded));
    }
    if result.flags.is_sitemap {
        match loaded.site.sitemap(&result).await? {
            SitemapOutput::Xml(xml) => return Ok(typed(StatusCode::OK, "application/xml; charset=UTF-8", xml)),
            SitemapOutput::Stylesheet(xsl) => return Ok(typed(StatusCode::OK, "application/xml; charset=UTF-8", xsl)),
            SitemapOutput::NotFound => result = result.into_404(),
        }
    }
    let document = render_document(&loaded.theme, &loaded.site, &result, &loaded.registry).await?;
    let mut response = Response::new(Body::from(document.html));
    *response.status_mut() = StatusCode::from_u16(document.status).unwrap_or(StatusCode::OK);
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
    if document.status == 404 {
        no_cache(response.headers_mut());
    }
    if result.flags.is_embed {
        response.headers_mut().insert("x-wp-embed", HeaderValue::from_static("true"));
    }
    singular_headers(&mut response, &result, &loaded);
    Ok(with_api_link(response, &loaded))
}

/// `WP::send_headers()` for single posts: pingback discovery and no caching of protected posts.
fn singular_headers(response: &mut Response<Body>, result: &rustpress_core::QueryResult, loaded: &Loaded) {
    if !result.flags.is_singular() {
        return;
    }
    let Some(post) = result.queried_post() else { return };
    if post.ping_status == "open"
        && let Ok(value) = HeaderValue::from_str(&loaded.site.site_url("xmlrpc.php"))
    {
        response.headers_mut().insert("x-pingback", value);
    }
    if !post.post_password.is_empty() {
        no_cache(response.headers_mut());
    }
}

/// `rest_output_link_header()`: front-end responses point to the REST API root.
fn with_api_link(mut response: Response<Body>, loaded: &Loaded) -> Response<Body> {
    if response.status() == StatusCode::NOT_MODIFIED {
        return response;
    }
    let link = Rest::new(&loaded.site, &loaded.theme, &loaded.registry).root_link();
    if let Ok(value) = HeaderValue::from_str(&link) {
        response.headers_mut().insert(header::LINK, value);
    }
    response
}

/// `nocache_headers()` as WordPress sends them with a 404.
fn no_cache(headers: &mut axum::http::HeaderMap) {
    headers.insert(header::EXPIRES, HeaderValue::from_static("Wed, 11 Jan 1984 05:00:00 GMT"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache, must-revalidate, max-age=0, no-store, private"));
}

fn typed(status: StatusCode, content_type: &str, body: String) -> Response<Body> {
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    if let Ok(value) = HeaderValue::from_str(content_type) {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    response
}

fn redirect(location: &str) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::MOVED_PERMANENTLY;
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response
}

/// `(route, query)` when the request addresses the REST API (`/wp-json/...` or `?rest_route=`).
fn rest_route(home_path: &str, path: &str, query: &str) -> Option<(String, String)> {
    let home = home_path.trim_end_matches('/');
    let relative = path.strip_prefix(home).unwrap_or(path);
    // Path info permalinks: `/index.php/wp-json/...`.
    let relative = relative.strip_prefix("/index.php").filter(|rest| rest.starts_with("/wp-json")).unwrap_or(relative);
    if relative == "/wp-json" || relative.starts_with("/wp-json/") {
        return Some((relative["/wp-json".len()..].to_owned(), query.to_owned()));
    }
    let pairs: Vec<&str> = query.split('&').collect();
    let route = pairs.iter().find_map(|p| p.strip_prefix("rest_route="))?;
    let route = String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(route.replace('+', " ").as_bytes())).into_owned();
    let rest: Vec<&str> = pairs.into_iter().filter(|p| !p.starts_with("rest_route=")).collect();
    Some((route, rest.join("&")))
}

fn rest_response(reply: rustpress_rest::Reply) -> Response<Body> {
    let mut response = Response::new(Body::from(reply.body));
    *response.status_mut() = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::OK);
    for (name, value) in reply.headers {
        if let (Ok(name), Ok(value)) = (header::HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(&value)) {
            response.headers_mut().append(name, value);
        }
    }
    response
}

/// The feed with the headers `WP::send_headers()` sets, or a bare 304.
fn feed_response(feed: &Feed, request: &axum::http::HeaderMap) -> Response<Body> {
    let header_str = |name: header::HeaderName| request.get(name).and_then(|v| v.to_str().ok());
    let not_modified = feed.not_modified(header_str(header::IF_NONE_MATCH), header_str(header::IF_MODIFIED_SINCE));
    let mut response = Response::new(if not_modified { Body::empty() } else { Body::from(feed.body.clone()) });
    if not_modified {
        *response.status_mut() = StatusCode::NOT_MODIFIED;
    }
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&feed.content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Some(value) = feed.last_modified.as_deref().and_then(|v| HeaderValue::from_str(v).ok()) {
        headers.insert(header::LAST_MODIFIED, value);
    }
    if let Ok(value) = HeaderValue::from_str(&feed.etag) {
        headers.insert(header::ETAG, value);
    }
    if feed.nocache {
        no_cache(headers);
    }
    response
}

fn text(status: StatusCode, body: &'static str) -> Response<Body> {
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=UTF-8"));
    response
}
