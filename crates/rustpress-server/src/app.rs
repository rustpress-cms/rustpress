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
use rustpress_theme::{Feed, Theme, render_feed};
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
    /// Sign-in and the administration area, when writing is allowed.
    pub admin: Option<crate::admin::AdminParts>,
    loaded_at: Instant,
}

pub struct App {
    config: Config,
    db: Db,
    writer: Option<rustpress_db::Writer>,
    statics: Statics,
    loaded: RwLock<Arc<Loaded>>,
    reload: Mutex<()>,
    /// The app itself, for reloading in the background.
    this: std::sync::OnceLock<std::sync::Weak<App>>,
    /// A background reload is running.
    reloading: std::sync::atomic::AtomicBool,
}

impl App {
    pub async fn new(config: Config) -> Result<Arc<Self>> {
        let db = Db::connect(&config.database_url, &config.table_prefix).await?;
        let writer = if config.allow_writes { Some(rustpress_db::Writer::connect(&config.database_url, &config.table_prefix).await?) } else { None };
        let loaded = Self::load(&config, &db).await?;
        let app = Arc::new(Self {
            statics: Statics::new(&config.wp_root),
            config,
            db,
            writer,
            loaded: RwLock::new(Arc::new(loaded)),
            reload: Mutex::new(()),
            this: std::sync::OnceLock::new(),
            reloading: std::sync::atomic::AtomicBool::new(false),
        });
        let _ = app.this.set(Arc::downgrade(&app));
        if app.writer.is_some() {
            tokio::spawn(scheduler(app.clone()));
        }
        Ok(app)
    }

    async fn load(config: &Config, db: &Db) -> Result<Loaded> {
        let site = Site::load_at(db.clone(), &config.wp_root).await?;
        let theme = Theme::load(&site, &config.wp_root).await?;
        let registry = Registry::new(config.wp_root.join("wp-includes").join("blocks"));
        let admin = if config.allow_writes { Some(crate::admin::AdminParts::load(config, &site).await) } else { None };
        Ok(Loaded { site, theme, registry, admin, loaded_at: Instant::now() })
    }

    /// The current snapshot; options and theme are re-read once it is older than `reload_secs`.
    /// A failed reload keeps serving the previous snapshot. The reload runs in the background while requests keep
    /// the snapshot they have, except with `reload_secs` of 1 or less (development: every request sees the
    /// database as it is now).
    pub async fn current(&self) -> Arc<Loaded> {
        let snapshot = self.loaded.read().await.clone();
        if snapshot.loaded_at.elapsed() < Duration::from_secs(self.config.reload_secs) {
            return snapshot;
        }
        if self.config.reload_secs > 1 {
            if !self.reloading.swap(true, std::sync::atomic::Ordering::AcqRel) {
                match self.this.get().and_then(std::sync::Weak::upgrade) {
                    Some(app) => {
                        tokio::spawn(async move {
                            app.reload_if_stale().await;
                            app.reloading.store(false, std::sync::atomic::Ordering::Release);
                        });
                    }
                    None => self.reloading.store(false, std::sync::atomic::Ordering::Release),
                }
            }
            return snapshot;
        }
        self.reload_if_stale().await
    }

    /// Reloads when the snapshot is (still) older than `reload_secs`; returns the snapshot to use.
    async fn reload_if_stale(&self) -> Arc<Loaded> {
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

    /// Re-reads options and theme right away (after the administration area changed something). Requests that
    /// waited for a reload that began after their change share it instead of starting another.
    pub async fn reload_now(&self) {
        let changed_at = Instant::now();
        let _guard = self.reload.lock().await;
        if self.loaded.read().await.loaded_at > changed_at {
            return;
        }
        match Self::load(&self.config, &self.db).await {
            Ok(fresh) => *self.loaded.write().await = Arc::new(fresh),
            Err(e) => tracing::error!(error = %e, "reload failed, keeping previous snapshot"),
        }
    }

    pub fn router(self: &Arc<Self>) -> Router {
        Router::new().fallback(handle).with_state(self.clone()).layer(TraceLayer::new_for_http())
    }
}

/// What `wp-cron.php` does for content: publish scheduled posts, empty old auto-drafts and the trash.
async fn scheduler(app: Arc<App>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let Some(writer) = app.writer.as_ref() else { return };
        let loaded = app.current().await;
        let now = crate::admin::now();
        match rustpress_admin::cron::run(&loaded.site, &app.db, writer, now).await {
            Ok(report) => {
                if report != rustpress_admin::cron::Report::default() {
                    tracing::info!(published = report.published, auto_drafts = report.auto_drafts_deleted, trash = report.trash_purged, "scheduled jobs");
                    app.reload_now().await;
                }
            }
            Err(e) => tracing::error!(error = %e, "scheduled jobs failed"),
        }
    }
}

/// The request with the cookie of the password of protected posts in reach of the renderer.
async fn handle(axum::extract::State(app): axum::extract::State<Arc<App>>, request: Request<Body>) -> Response<Body> {
    let hash = app.current().await.admin.as_ref().map(|parts| parts.auth.cookie_hash().to_owned());
    let cookie = hash.and_then(|hash| {
        let name = format!("wp-postpass_{hash}");
        request.headers().get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).find_map(|pair| {
            let (key, value) = pair.trim().split_once('=')?;
            (key.trim() == name).then(|| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(value.trim().as_bytes())).into_owned())
        })
    });
    let commenter = commenter_of(&app, &request.headers().clone()).await;
    rustpress_blocks::commenter::COMMENTER.scope(commenter, rustpress_blocks::content::POST_PASSWORD_COOKIE.scope(cookie, handle_request(axum::extract::State(app), request))).await
}

/// `wp_get_current_commenter()`: the cookies `comment_author_*` of the request, cleaned like `sanitize_comment_cookies()`.
async fn commenter_of(app: &Arc<App>, headers: &axum::http::HeaderMap) -> rustpress_blocks::commenter::Commenter {
    let loaded = app.current().await;
    let auth = loaded.admin.as_ref().map(|parts| parts.auth.clone());
    let hash = auth.as_ref().map(|auth| auth.cookie_hash().to_owned()).unwrap_or_default();
    let cookie = |prefix: &str| -> String {
        let name = format!("{prefix}{hash}");
        headers.get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).find_map(|pair| {
            let (key, value) = pair.trim().split_once('=')?;
            (key.trim() == name).then(|| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(value.trim().as_bytes())).into_owned())
        })
        .unwrap_or_default()
    };
    let (author, email, url) = if hash.is_empty() {
        Default::default()
    } else {
        (
            rustpress_admin::comments_post::filter_text(&cookie("comment_author_")),
            rustpress_admin::comments_post::filter_email(&cookie("comment_author_email_")),
            rustpress_admin::comments_post::filter_url(&cookie("comment_author_url_")),
        )
    };
    rustpress_blocks::commenter::Commenter::new(author, email, url, move |data| auth.as_ref().and_then(|auth| auth.salts().hash(data, "auth")).unwrap_or_default())
}

async fn handle_request(axum::extract::State(app): axum::extract::State<Arc<App>>, request: Request<Body>) -> Response<Body> {
    let request = match admin_route(&app, request).await {
        Ok(response) => return response,
        Err(request) => request,
    };
    let request = match comments_post_route(&app, request).await {
        Ok(response) => return response,
        Err(request) => request,
    };
    let request = match rest_edit_route(&app, request).await {
        Ok(response) => return response,
        Err(request) => request,
    };
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

/// `wp-comments-post.php`: only a POST sends a comment; anything else is answered with 405 and `Allow: POST`.
#[allow(clippy::result_large_err)]
async fn comments_post_route(app: &Arc<App>, request: Request<Body>) -> std::result::Result<Response<Body>, Request<Body>> {
    let path = request.uri().path();
    if !(path.ends_with("/wp-comments-post.php") || path == "/wp-comments-post.php") {
        return Err(request);
    }
    if request.method() != Method::POST {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        response.headers_mut().insert(header::ALLOW, HeaderValue::from_static("POST"));
        response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain;charset=UTF-8"));
        return Ok(response);
    }
    if app.writer.is_none() {
        return Err(request);
    }
    let loaded = app.current().await;
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 8 * 1024 * 1024).await.unwrap_or_default();
    let client = crate::client::Client::of(&app.config, &parts, &loaded.site.settings.siteurl);
    let response = crate::admin::comments_post(&loaded, app.writer.as_ref(), &String::from_utf8_lossy(&bytes), &parts.headers, &client).await;
    // A saved comment answers with a redirect; the errors change nothing.
    if response.status() == StatusCode::FOUND {
        app.reload_now().await;
    }
    Ok(response)
}

/// The routes of the administration area; the request comes back for everything else.
#[allow(clippy::result_large_err)]
async fn admin_route(app: &Arc<App>, request: Request<Body>) -> std::result::Result<Response<Body>, Request<Body>> {
    let path = request.uri().path().to_owned();
    let is_login = path.ends_with("/wp-login.php") || path == "/wp-login.php";
    let is_styles = path.ends_with("/wp-admin/load-styles.php");
    let is_scripts = path.ends_with("/wp-admin/load-scripts.php");
    // A file that `wp-admin` does not have is a request for the site: its 404 page.
    let admin_file = admin_file_of(&path).filter(|file| app.config.wp_root.join("wp-admin").join(file).is_file());
    if !(is_login || is_styles || is_scripts || admin_file.is_some()) || app.writer.is_none() {
        return Err(request);
    }
    let loaded = app.current().await;
    let query = request.uri().query().unwrap_or("").to_owned();
    if is_styles {
        return Ok(crate::admin::load_styles(&loaded, &app.config, &query, request.headers()));
    }
    if is_scripts {
        return Ok(crate::admin::load_scripts(&loaded, &app.config, &query, request.headers()));
    }
    let post = request.method() == Method::POST;
    if !post && !matches!(*request.method(), Method::GET | Method::HEAD) {
        return Err(request);
    }
    let request_uri = request.uri().path_and_query().map(|p| p.as_str().to_owned()).unwrap_or_default();
    let (parts, body) = request.into_parts();
    // Only somebody who is signed in may send an upload-sized body; a visitor's form is small.
    let signed_in = post && crate::admin::frontend_user(&loaded, &parts.headers).await.is_some();
    let limit = if signed_in { UPLOAD_BODY_LIMIT } else { VISITOR_BODY_LIMIT };
    let bytes = match axum::body::to_bytes(body, limit).await {
        Ok(bytes) => bytes,
        Err(_) => return Ok(text(StatusCode::PAYLOAD_TOO_LARGE, "Payload Too Large\n")),
    };
    if let Some(file) = admin_file {
        let client = crate::client::Client::of(&app.config, &parts, &loaded.site.settings.siteurl);
        let response = crate::admin::admin_page(&loaded, &app.config, app.writer.as_ref(), &file, post, &request_uri, &query, &bytes, &parts.headers, &client).await;
        // A visitor's POST reaches no screen that writes (it is sent to the sign-in).
        if signed_in {
            app.reload_now().await;
        }
        return Ok(response);
    }
    let body_text = String::from_utf8_lossy(&bytes).into_owned();
    let client = crate::client::Client::of(&app.config, &parts, &loaded.site.settings.siteurl);
    let response = crate::admin::login_page(&loaded, app.writer.as_ref(), post, &path, &request_uri, &query, &body_text, &parts.headers, &client).await;
    // The sign-in forms that change something (a sign-in, a new password, a registration) answer with a redirect.
    if post && response.status().is_redirection() {
        app.reload_now().await;
    }
    Ok(response)
}

/// The body a signed-in user may send (an upload) and the body a visitor may send (a form).
const UPLOAD_BODY_LIMIT: usize = 256 * 1024 * 1024;
const VISITOR_BODY_LIMIT: usize = 64 * 1024;

/// `index.php` for `wp-admin/`, the file name for `wp-admin/<file>.php`; `None` for anything else
/// (static files, `load-styles.php`, sub-directories).
fn admin_file_of(path: &str) -> Option<String> {
    let rest = path.rsplit_once("/wp-admin/").map(|(_, rest)| rest)?;
    if rest.is_empty() {
        return Some("index.php".to_owned());
    }
    let valid = rest.ends_with(".php") && !rest.contains('/') && !rest.starts_with("load-");
    valid.then(|| rest.to_owned())
}

async fn page(app: &App, uri: &axum::http::Uri, request_headers: &axum::http::HeaderMap) -> Result<Response<Body>> {
    let (path, query) = (uri.path(), uri.query().unwrap_or(""));
    let started = std::time::Instant::now();
    let loaded = app.current().await;
    if let Some((route, rest_query)) = rest_route(&loaded.site.settings.home_path(), path, query) {
        let rest = Rest::new(&loaded.site, &loaded.theme, &loaded.registry).with_host(request_headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or_default());
        let reply = rest.handle(&route, &rest_query).await?;
        return Ok(rest_response(reply));
    }
    let mut extras = rustpress_theme::Extras::default();
    let mut signed_in = false;
    let viewer = match crate::admin::frontend_user(&loaded, request_headers).await {
        Some((user, token)) => {
            let user = std::sync::Arc::new(user);
            let show_bar = loaded.site.db.user_meta(user.id, "show_admin_bar_front").await.ok().flatten().is_none_or(|v| v != "false");
            let parts = loaded.admin.as_ref();
            let nonce_auth = parts.map(|p| p.auth.clone());
            let (uid, session) = (user.id, token.clone());
            let nonce = move |action: &str| nonce_auth.as_ref().and_then(|auth| auth.create_nonce(action, uid, &session, crate::admin::now())).unwrap_or_default();
            let can_user = user.clone();
            let verify_auth = parts.map(|p| p.auth.clone());
            let verify_session = token.clone();
            let privacy_page = loaded.site.options.u64("wp_page_for_privacy_policy", 0);
            let verify = move |nonce: &str, action: &str| verify_auth.as_ref().is_some_and(|auth| auth.verify_nonce(nonce, action, uid, &verify_session, crate::admin::now()).is_some());
            let viewer = rustpress_core::Viewer::new(user.id, move |capability| can_user.can(capability)).with_identity(&user.login, &user.display_name, &user.email, show_bar, nonce).with_verifier(privacy_page, verify);
            if show_bar && let Some(found) = crate::admin::front_toolbar(&loaded, &user, &token).await {
                extras = found;
            }
            signed_in = true;
            Some(viewer)
        }
        None => None,
    };
    let before_query = started.elapsed();
    let mut result = loaded.site.query_as(path, query, viewer.as_ref()).await?;
    let query_done = started.elapsed();
    result.http_host = request_headers.get(header::HOST).and_then(|v| v.to_str().ok()).map(str::to_owned);
    // Physical PHP endpoints (wp-login.php, xmlrpc.php, ...) do not exist here; they get the
    // 404 page rather than a canonical redirect. Only `/index.php` is WordPress' own alias for `/`.
    let php_endpoint = path.ends_with(".php") && path != "/index.php";
    let canonical = if php_endpoint { None } else { loaded.site.canonical_redirect_for_request(path, query, &result).await? };
    let canonical_done = started.elapsed();
    tracing::debug!(setup_ms = before_query.as_secs_f64() * 1000.0, query_ms = (query_done - before_query).as_secs_f64() * 1000.0, canonical_ms = (canonical_done - query_done).as_secs_f64() * 1000.0, path, "page prepared");
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
        } else if let Some(feed) = rustpress_theme::feed::redirect_headers(&loaded.site, &result).await? {
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
        }
        return Ok(response);
    }
    if result.flags.is_feed {
        let request_uri = uri.path_and_query().map_or(path, |p| p.as_str());
        let feed = rustpress_db::Db::with_term_cache(async {
            loaded.site.db.prime_loop(&result.posts).await?;
            render_feed(&loaded.theme, &loaded.site, &result, &loaded.registry, request_uri).await
        })
        .await;
        let Some(feed) = feed? else {
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
    // `wp-trackback.php` for a request that is not a trackback ping: back to the post.
    if result.vars.get("tb").is_some() && result.flags.is_singular() && let Some(post) = result.queried_post() {
        let permalink = loaded.site.permalink(post).await?;
        let (target_path, target_query) = rustpress_core::canonical::split_url(&permalink);
        let target = if target_query.is_empty() { target_path } else { format!("{target_path}?{target_query}") };
        let mut response = redirect(&target);
        *response.status_mut() = StatusCode::FOUND;
        response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
        singular_headers(&mut response, &result, &loaded);
        return Ok(with_api_link(response, &loaded));
    }
    // The terms of the posts of the main query are read once (`update_object_term_cache()`).
    let document = rustpress_db::Db::with_term_cache(async {
        loaded.site.db.prime_loop(&result.posts).await?;
        rustpress_theme::render_document_with(&loaded.theme, &loaded.site, &result, &loaded.registry, &extras).await
    })
    .await?;
    let mut response = Response::new(Body::from(document.html));
    *response.status_mut() = StatusCode::from_u16(document.status).unwrap_or(StatusCode::OK);
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
    // `WP::send_headers()`: pages for a signed-in user are not cached.
    if document.status == 404 || signed_in {
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

/// REST requests of signed-in users and every request with a body: the authenticated layer of the REST API.
#[allow(clippy::result_large_err)]
async fn rest_edit_route(app: &Arc<App>, request: Request<Body>) -> std::result::Result<Response<Body>, Request<Body>> {
    if app.writer.is_none() {
        return Err(request);
    }
    let loaded = app.current().await;
    let path = request.uri().path().to_owned();
    let query = request.uri().query().unwrap_or("").to_owned();
    let Some((route, rest_query)) = rest_route(&loaded.site.settings.home_path(), &path, &query) else { return Err(request) };
    let (parts, body) = request.into_parts();
    let mut method = parts.method.as_str().to_owned();
    if let Some(over) = parts.headers.get("x-http-method-override").and_then(|v| v.to_str().ok()) {
        method = over.to_uppercase();
    }
    let is_read = matches!(method.as_str(), "GET" | "HEAD");
    let has_nonce = parts.headers.contains_key("x-wp-nonce") || parts.headers.contains_key("authorization") || rest_query.contains("_wpnonce=");
    if is_read && !has_nonce {
        return Err(Request::from_parts(parts, Body::empty()));
    }
    // The credentials in the headers decide how much may be sent; the nonce of a form body is checked after.
    let known = matches!(crate::admin::rest_auth(&loaded, &parts.headers, &rest_query, None).await, crate::admin::RestAuth::User(..) | crate::admin::RestAuth::AppPassword(..))
        || crate::admin::frontend_user(&loaded, &parts.headers).await.is_some();
    let bytes = match axum::body::to_bytes(body, if known { UPLOAD_BODY_LIMIT } else { VISITOR_BODY_LIMIT }).await {
        Ok(bytes) => bytes,
        Err(_) => return Ok(text(StatusCode::PAYLOAD_TOO_LARGE, "Payload Too Large\n")),
    };
    let content_type = parts.headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or_default().to_owned();
    let rest_body = if bytes.is_empty() {
        rustpress_rest::edit::Body::None
    } else if let Some(boundary) = rustpress_admin::multipart::boundary(&content_type) {
        let (fields, files) = rustpress_admin::multipart::parse(&boundary, &bytes);
        rustpress_rest::edit::Body::Multipart { fields, files: files.into_iter().map(|f| rustpress_rest::edit::UploadedPart { field: f.field, filename: f.filename, content_type: f.content_type, bytes: f.bytes }).collect() }
    } else if content_type.starts_with("application/json") {
        serde_json::from_slice(&bytes).map_or(rustpress_rest::edit::Body::None, rustpress_rest::edit::Body::Json)
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        rustpress_rest::edit::Body::Form(crate::admin::pairs_of_public(&String::from_utf8_lossy(&bytes)))
    } else {
        let disposition = parts.headers.get("content-disposition").and_then(|v| v.to_str().ok()).unwrap_or_default().to_owned();
        rustpress_rest::edit::Body::Raw { bytes: bytes.to_vec(), content_type, disposition }
    };
    let form_nonce = match &rest_body {
        rustpress_rest::edit::Body::Form(pairs) => pairs.iter().find(|(k, _)| k == "_wpnonce").map(|(_, v)| v.clone()),
        rustpress_rest::edit::Body::Json(value) => value.get("_wpnonce").and_then(|v| v.as_str()).map(str::to_owned),
        _ => None,
    };
    let auth = crate::admin::rest_auth(&loaded, &parts.headers, &rest_query, form_nonce.as_deref()).await;
    let mut app_password = None;
    let mut nonce: Option<std::sync::Arc<dyn Fn(&str) -> String + Send + Sync>> = None;
    let user = match auth {
        crate::admin::RestAuth::User(user, token) => {
            if let Some(admin) = &loaded.admin {
                let (auth, user_id) = (admin.auth.clone(), user.id);
                nonce = Some(std::sync::Arc::new(move |action: &str| auth.create_nonce(action, user_id, &token, crate::admin::now()).unwrap_or_default()));
            }
            Some(user)
        }
        crate::admin::RestAuth::AppPassword(user, uuid) => {
            if let Some(writer) = app.writer.as_ref() {
                let ip = crate::client::Client::of(&app.config, &parts, &loaded.site.settings.siteurl).ip;
                rustpress_auth::app_passwords::record_usage(&loaded.site.db, writer, user.id, &uuid, ip.as_deref(), crate::admin::now()).await;
            }
            app_password = Some(uuid);
            Some(user)
        }
        // `rest_application_password_check_errors()`: the error of the failed application password, as 401.
        crate::admin::RestAuth::BadAppPassword(code, message) => {
            let body = serde_json::json!({"code": code, "message": loaded.site.tr(message), "data": {"status": 401}}).to_string();
            let mut response = Response::new(Body::from(body));
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=UTF-8"));
            return Ok(response);
        }
        crate::admin::RestAuth::BadNonce => {
            let body = r#"{"code":"rest_cookie_invalid_nonce","message":"Cookie check failed","data":{"status":403}}"#;
            let mut response = Response::new(Body::from(body));
            *response.status_mut() = StatusCode::FORBIDDEN;
            response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=UTF-8"));
            return Ok(response);
        }
        crate::admin::RestAuth::Anonymous => None,
    };
    if user.is_none() && is_read {
        return Err(Request::from_parts(parts, Body::empty()));
    }
    let uploads = app.config.wp_root.join("wp-content").join("uploads");
    let roles = loaded.admin.as_ref().map(|admin| &admin.roles);
    let access = user.map(|user| rustpress_rest::edit::Access { user, app_password, writer: app.writer.as_ref(), now: crate::admin::now(), uploads, nonce, roles, salts: loaded.admin.as_ref().map(|admin| admin.auth.salts()), comment_notify: comment_notify(&loaded, app.writer.clone()) });
    let remote_addr = crate::client::Client::of(&app.config, &parts, &loaded.site.settings.siteurl).ip;
    let user_agent = parts.headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_owned);
    let rest = Rest::new(&loaded.site, &loaded.theme, &loaded.registry).with_host(parts.headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or_default());
    let reply = rest.handle_as(&route, &rest_query, access, rustpress_rest::edit::Request { method, body: rest_body, basic: crate::admin::basic_credentials(&parts.headers), remote_addr, user_agent }).await;
    let response = match reply {
        Ok(reply) => rest_response(reply),
        Err(e) => {
            tracing::error!(error = %e, path = path.as_str(), "rest request failed");
            text(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error\n")
        }
    };
    // Only a request that was allowed to write changed anything.
    if !is_read && response.status().is_success() {
        app.reload_now().await;
    }
    Ok(response)
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
    let mut response = Response::new(if reply.bytes.is_empty() { Body::from(reply.body) } else { Body::from(reply.bytes) });
    *response.status_mut() = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::OK);
    for (name, value) in reply.headers {
        if let (Ok(name), Ok(value)) = (header::HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(&value)) {
            response.headers_mut().append(name, value);
        }
    }
    response
}

/// The feed with the headers `WP::send_headers()` sets, or a bare 304.
/// The mail of `rest_insert_comment` and of an approving `wp_set_comment_status()` for the REST API.
fn comment_notify(loaded: &std::sync::Arc<Loaded>, writer: Option<rustpress_db::Writer>) -> Option<rustpress_rest::edit::CommentNotify> {
    loaded.admin.as_ref()?;
    let loaded = loaded.clone();
    Some(std::sync::Arc::new(move |event| {
        let (loaded, writer) = (loaded.clone(), writer.clone());
        Box::pin(async move {
            let Some(admin) = loaded.admin.as_ref() else { return };
            let mail = rustpress_admin::mail::MailCtx { site: &loaded.site, db: &loaded.site.db, writer: writer.as_ref(), now: crate::admin::now(), mailer: admin.mailer.as_ref() };
            match event {
                rustpress_rest::edit::CommentEvent::Approved { comment, by } => mail.new_comment_notify(&admin.roles, comment, false, by).await,
                rustpress_rest::edit::CommentEvent::Inserted { comment, by, creating } => {
                    // `wp_new_comment_via_rest_notify_postauthor()` is about notes only.
                    if loaded.site.db.comment_any(comment).await.ok().flatten().is_some_and(|c| c.comment_type == "note") {
                        mail.new_comment_notify(&admin.roles, comment, false, by).await;
                        if creating {
                            mail.notify_note_mentions(&admin.roles, comment).await;
                        }
                    }
                }
            }
        })
    }))
}

fn feed_response(feed: &Feed, request: &axum::http::HeaderMap) -> Response<Body> {
    let header_str = |name: header::HeaderName| request.get(name).and_then(|v| v.to_str().ok());
    let not_modified = feed.not_modified(header_str(header::IF_NONE_MATCH), header_str(header::IF_MODIFIED_SINCE));
    let mut response = Response::new(if not_modified { Body::empty() } else { Body::from(feed.body.clone()) });
    if not_modified {
        *response.status_mut() = StatusCode::NOT_MODIFIED;
    } else if feed.not_found {
        *response.status_mut() = StatusCode::NOT_FOUND;
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
