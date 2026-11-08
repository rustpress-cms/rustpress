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
    /// The keys and salts for `wp_hash()`, also without the administration.
    pub salts: rustpress_auth::salts::Salts,
    /// `wp_mail()` for the plugins (contact forms), also on a read-only site.
    pub mail: Option<Arc<dyn rustpress_core::native::Mailer>>,
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
        Self::build(config, true).await
    }

    /// For a process that answers one request and ends (CGI): without the background jobs.
    pub async fn for_one_request(config: Config) -> Result<Arc<Self>> {
        Self::build(config, false).await
    }

    async fn build(config: Config, background: bool) -> Result<Arc<Self>> {
        rustpress_plugins::register();
        let connecting = std::time::Instant::now();
        let db = Db::connect(&config.database_url, &config.table_prefix).await?;
        crate::cgi::note_connect(connecting.elapsed());
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
        if background && app.writer.is_some() {
            tokio::spawn(scheduler(app.clone()));
        }
        Ok(app)
    }

    async fn load(config: &Config, db: &Db) -> Result<Loaded> {
        let mut site = Site::load_at(db.clone(), &config.wp_root).await?;
        if config.request_https {
            for url in [&mut site.settings.home, &mut site.settings.siteurl] {
                if let Some(rest) = url.strip_prefix("http://") {
                    *url = format!("https://{rest}");
                }
            }
        }
        let theme = Theme::load(&site, &config.wp_root).await?;
        let registry = Registry::new(config.wp_root.join("wp-includes").join("blocks"));
        let salts = crate::admin::load_salts(config, &site).await;
        let admin = if config.allow_writes { Some(crate::admin::AdminParts::load(config, &site, salts.clone()).await) } else { None };
        let mail = crate::admin::SmtpMail::for_site(&site).map(|m| Arc::new(m) as Arc<dyn rustpress_core::native::Mailer>);
        Ok(Loaded { site, theme, registry, admin, salts, mail, loaded_at: Instant::now() })
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

/// `get_site_icon_url( $size )`: the site icon in that size (or the full image), when the site has one.
async fn site_icon_url(site: &Site, size: u64) -> Option<String> {
    let id = site.options.u64("site_icon", 0);
    if id == 0 {
        return None;
    }
    let rows = site.db.post_meta(id).await.ok()?;
    let value = |key: &str| rows.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
    let full = rustpress_blocks::images::attachment_url(site, &value("_wp_attached_file")?);
    let meta = value("_wp_attachment_metadata").and_then(|raw| rustpress_blocks::images::Metadata::parse(&raw));
    Some(meta.and_then(|meta| rustpress_blocks::images::intermediate_url(&full, &meta, size, size)).unwrap_or(full))
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
    let salts = loaded.salts.clone();
    rustpress_blocks::commenter::Commenter::new(author, email, url, move |data, scheme| salts.hash(data, scheme).unwrap_or_default())
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
    let not_allowed = || {
        let mut response = text(StatusCode::METHOD_NOT_ALLOWED, "RustPress is read-only: only GET and HEAD are supported.\n");
        response.headers_mut().insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
        response
    };
    // The Customizer posts to the page in its preview: the changes that are not in the changeset yet, and the requests
    // of selective refresh. Any other POST is refused.
    let post = *request.method() == Method::POST;
    if !post && !matches!(*request.method(), Method::GET | Method::HEAD) {
        return not_allowed();
    }
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let method = request.method().clone();
    let (parts, body) = request.into_parts();
    let client = crate::client::Client::of(&app.config, &parts, &app.current().await.site.settings.siteurl);
    let form = if post {
        let form = match axum::body::to_bytes(body, FORM_BODY_LIMIT).await {
            Ok(bytes) => crate::admin::pairs_of_public(&String::from_utf8_lossy(&bytes)),
            Err(_) => return text(StatusCode::PAYLOAD_TOO_LARGE, "Payload Too Large\n"),
        };
        if crate::customize::Request::parse(uri.query().unwrap_or(""), &form).is_none() {
            return not_allowed();
        }
        form
    } else {
        let request = Request::from_parts(parts, body);
        if let Some(response) = app.statics.serve(request).await {
            return response;
        }
        Vec::new()
    };
    match page(&app, &uri, &headers, &method, &client, &form).await {
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
/// What a visitor may send to a plugin's route: a contact form with its files (Contact Form 7 attaches up to 25 MB).
const FORM_BODY_LIMIT: usize = 32 * 1024 * 1024;

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

async fn page(app: &App, uri: &axum::http::Uri, request_headers: &axum::http::HeaderMap, method: &Method, client: &crate::client::Client, form: &[(String, String)]) -> Result<Response<Body>> {
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
    // `_wp_customize_include()`: a page in the preview of the Customizer.
    let customize = crate::customize::Request::parse(query, form);
    let viewer = match crate::admin::frontend_user(&loaded, request_headers).await {
        Some((user, token)) => {
            let user = std::sync::Arc::new(user);
            // `setup_theme()` hides the toolbar inside the frame of the Customizer.
            let in_frame = customize.as_ref().is_some_and(|c| !c.channel.is_empty());
            let show_bar = !in_frame && loaded.site.db.user_meta(user.id, "show_admin_bar_front").await.ok().flatten().is_none_or(|v| v != "false");
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
            if show_bar && let Some(found) = crate::admin::front_toolbar(&loaded, &user, &token, customize.as_ref()).await {
                extras = found;
            }
            signed_in = true;
            Some(viewer)
        }
        None => None,
    };
    let (preview, preview_site) = match &customize {
        Some(request) => match crate::customize::setup_theme(&loaded, viewer.as_ref(), request, form).await {
            Ok((preview, site)) => (Some(preview), site),
            Err(response) => return Ok(response),
        },
        None => (None, None),
    };
    // The options and theme mods as the preview shows them.
    let site: &rustpress_core::Site = preview_site.as_ref().unwrap_or(&loaded.site);
    let before_query = started.elapsed();
    // `init` of the plugins: they may answer the request, change it, or keep `redirect_canonical()` from it.
    let header_list: Vec<(String, String)> = request_headers.iter().map(|(name, value)| (name.as_str().to_owned(), String::from_utf8_lossy(value.as_bytes()).into_owned())).collect();
    let plugin_request = rustpress_core::native::PluginRequest {
        method: method.as_str(),
        uri: uri.path_and_query().map_or(path, |p| p.as_str()),
        headers: &header_list,
        remote_addr: client.ip.as_deref().unwrap_or_default(),
        https: client.https,
        viewer: viewer.as_ref(),
        writer: app.writer.as_ref(),
        state: Default::default(),
    };
    let init = loaded.site.plugins.init(&loaded.site, &plugin_request).await?;
    let rewritten = match &init {
        Some(rustpress_core::native::Init::Response(answer)) => return Ok(plugin_response(answer)),
        Some(rustpress_core::native::Init::Rewrite(target)) => {
            let (p, q) = target.split_once('?').unwrap_or((target, ""));
            Some((p.to_owned(), q.to_owned(), target.clone()))
        }
        _ => None,
    };
    let (path, query) = match &rewritten {
        Some((p, q, _)) => (p.as_str(), q.as_str()),
        None => (path, query),
    };
    let request_uri = match &rewritten {
        Some((_, _, target)) => target.as_str(),
        None => uri.path_and_query().map_or(path, |p| p.as_str()),
    };
    // `send_headers`: what the plugins add to the page and to the redirects of `template_redirect`.
    let sent_headers = loaded.site.plugins.send_headers(&loaded.site, &plugin_request);
    // Plugins answer requests of their own rewrite rules before the main query runs.
    let home_path = loaded.site.settings.home_path();
    let relative = path.strip_prefix(home_path.as_str()).unwrap_or(path).trim_start_matches('/');
    let served = if path.ends_with(".php") && path != "/index.php" { None } else { loaded.site.plugins.serve(&loaded.site, &rustpress_core::native::ServeRequest { path: relative, query, wp_root: &loaded.theme.wp_root }).await? };
    if let Some(rustpress_core::native::Serve::Response(answer)) = served {
        return Ok(plugin_response(&answer));
    }
    let mut result = site.query_as(path, query, viewer.as_ref()).await?;
    result.customize = preview.clone();
    if viewer.is_none() {
        // A read-only site knows its salts as well.
        let auth = rustpress_auth::Auth::new(loaded.salts.clone(), &site.settings.siteurl);
        result.guest_nonce = Some(rustpress_core::query::GuestNonce(std::sync::Arc::new(move |action: &str| auth.create_nonce(action, 0, "", crate::admin::now()).unwrap_or_default())));
    }
    if matches!(served, Some(rustpress_core::native::Serve::NotFound)) {
        result = result.into_404();
    }
    // An error of a plugin's rule on `init`: the 404 template with its status.
    let error = match &init {
        Some(rustpress_core::native::Init::Error { status, by }) => {
            result = result.into_404();
            result.flags.is_feed = false;
            Some((*status, *by))
        }
        _ => None,
    };
    let query_done = started.elapsed();
    result.http_host = request_headers.get(header::HOST).and_then(|v| v.to_str().ok()).map(str::to_owned);
    result.user_agent = request_headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_owned);
    result.referer = request_headers.get(header::REFERER).and_then(|v| v.to_str().ok()).map(str::to_owned);
    // Physical PHP endpoints (wp-login.php, xmlrpc.php, ...) do not exist here; they get the
    // 404 page rather than a canonical redirect. Only `/index.php` is WordPress' own alias for `/`.
    let php_endpoint = path.ends_with(".php") && path != "/index.php";
    // `template_redirect` of the plugins runs before `redirect_canonical()`.
    if !php_endpoint && let Some(plugin) = site.plugins.template_redirect(site, &result).await? {
        return plugin_redirect(&loaded, &plugin_request, &sent_headers, &result, &plugin).await;
    }
    // `handle_render_partials_request()` on `template_redirect`.
    if let Some(preview) = &preview
        && form.iter().any(|(k, v)| k == "wp_customize_render_partials" && !v.is_empty())
    {
        return Ok(crate::customize::render_partials(&loaded, site, &result, preview, form).await);
    }
    // A rule of a plugin that took the request on `init` turns `redirect_canonical()` off.
    // `redirect_canonical()` leaves requests other than GET and HEAD alone.
    let canonical = if php_endpoint || init.is_some() || *method == Method::POST { None } else { site.canonical_redirect_for_request(path, query, &result).await? };
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
        // `redirect_canonical()` passes the absolute address on.
        let home = site.home_url("");
        let origin = home.split_once("://").map_or(home.clone(), |(scheme, rest)| format!("{scheme}://{}", rest.split('/').next().unwrap_or_default()));
        let absolute = if target.starts_with('/') { format!("{origin}{target}") } else { target.clone() };
        let redirected = rustpress_core::native::Redirected { location: &absolute, status: 301, by: "WordPress" };
        let plugin_headers = site.plugins.wp_redirect(site, &plugin_request, &redirected).await?;
        set_headers(&mut response, sent_headers.iter().chain(&plugin_headers));
        // `wp_redirect( $redirect_url, 301 )` in `redirect_canonical()`.
        response.headers_mut().insert("x-redirect-by", HeaderValue::from_static("WordPress"));
        if preview.is_some() {
            crate::customize::preview_headers(response.headers_mut());
        }
        if !result.flags.is_feed {
            response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
        } else if let Some(feed) = rustpress_theme::feed::redirect_headers(site, &result).await? {
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
    // `pre_handle_404` of a plugin: an empty feed of an archive becomes the 404 page.
    let result = if result.flags.is_feed && site.plugins.feed_not_found(&result) {
        let mut not_found = result.into_404();
        not_found.flags.is_feed = false;
        not_found
    } else {
        result
    };
    // `template_redirect` after `redirect_canonical()` on a 404 page.
    if result.flags.is_404 && !php_endpoint && init.is_none() && let Some(plugin) = site.plugins.not_found(site, &result, &plugin_request).await? {
        return plugin_redirect(&loaded, &plugin_request, &sent_headers, &result, &plugin).await;
    }
    if result.flags.is_feed {
        let feed = rustpress_db::Db::with_term_cache(async {
            site.db.prime_loop(&result.posts).await?;
            render_feed(&loaded.theme, site, &result, &loaded.registry, request_uri).await
        })
        .await;
        let Some(feed) = feed? else {
            return Ok(text(StatusCode::NOT_FOUND, "This feed is not available.\n"));
        };
        let mut response = feed_response(&feed, request_headers);
        singular_headers(&mut response, &result, &loaded);
        set_headers(&mut response, &sent_headers);
        return Ok(with_api_link(response, &loaded, &result));
    }
    let mut result = result;
    if result.flags.is_robots {
        let mut response = typed(StatusCode::OK, "text/plain; charset=utf-8", site.robots_txt());
        for (name, value) in site.plugins.robots_txt_headers() {
            response.headers_mut().insert(name, HeaderValue::from_static(value));
        }
        set_headers(&mut response, &sent_headers);
        return Ok(with_api_link(response, &loaded, &result));
    }
    if result.flags.is_favicon {
        let icon = match site_icon_url(site, 32).await {
            Some(icon) => icon,
            None => site.site_url("wp-includes/images/w-logo-gray-white-bg.png"),
        };
        let mut response = redirect(&icon);
        *response.status_mut() = StatusCode::FOUND;
        response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
        let redirected = rustpress_core::native::Redirected { location: &icon, status: 302, by: "WordPress" };
        let plugin_headers = site.plugins.wp_redirect(site, &plugin_request, &redirected).await?;
        set_headers(&mut response, sent_headers.iter().chain(&plugin_headers));
        return Ok(with_api_link(response, &loaded, &result));
    }
    if result.flags.is_sitemap {
        match site.sitemap(&result).await? {
            SitemapOutput::Xml(xml) => return Ok(typed(StatusCode::OK, "application/xml; charset=UTF-8", xml)),
            SitemapOutput::Stylesheet(xsl) => return Ok(typed(StatusCode::OK, "application/xml; charset=UTF-8", xsl)),
            SitemapOutput::NotFound => result = result.into_404(),
        }
    }
    // `wp-trackback.php` for a request that is not a trackback ping: back to the post.
    if result.vars.get("tb").is_some() && result.flags.is_singular() && let Some(post) = result.queried_post() {
        let permalink = site.permalink(post).await?;
        let (target_path, target_query) = rustpress_core::canonical::split_url(&permalink);
        let target = if target_query.is_empty() { target_path } else { format!("{target_path}?{target_query}") };
        let mut response = redirect(&target);
        *response.status_mut() = StatusCode::FOUND;
        response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
        singular_headers(&mut response, &result, &loaded);
        let redirected = rustpress_core::native::Redirected { location: &permalink, status: 302, by: "WordPress" };
        let plugin_headers = site.plugins.wp_redirect(site, &plugin_request, &redirected).await?;
        set_headers(&mut response, sent_headers.iter().chain(&plugin_headers));
        return Ok(with_api_link(response, &loaded, &result));
    }
    // The terms of the posts of the main query are read once (`update_object_term_cache()`).
    let document = rustpress_db::Db::with_term_cache(async {
        site.db.prime_loop(&result.posts).await?;
        rustpress_theme::render_document_with(&loaded.theme, site, &result, &loaded.registry, &extras).await
    })
    .await?;
    // The plugins' output buffers of `template_redirect`, flushed on `shutdown`.
    let mut html = site.plugins.template_output(site, &result, document.html);
    if let Some(preview) = &preview {
        let exports = crate::admin::customize_preview_exports(&loaded, site, &app.config, app.writer.as_ref(), request_headers, client, &result, preview).await;
        html = html.replacen(rustpress_theme::customize_preview::FOOTER_EXPORTS, &exports, 1);
    }
    let mut response = Response::new(Body::from(html));
    *response.status_mut() = StatusCode::from_u16(document.status).unwrap_or(StatusCode::OK);
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
    if preview.is_some() {
        crate::customize::preview_headers(response.headers_mut());
    }
    // `WP::send_headers()`: pages for a signed-in user are not cached.
    if document.status == 404 || signed_in {
        no_cache(response.headers_mut());
    }
    if result.flags.is_embed {
        response.headers_mut().insert("x-wp-embed", HeaderValue::from_static("true"));
    }
    singular_headers(&mut response, &result, &loaded);
    set_headers(&mut response, &sent_headers);
    // `status_header()`, `nocache_headers()` and `X-Redirect-By` of the rule on `wp`.
    if let Some((status, by)) = error {
        *response.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::NOT_FOUND);
        no_cache(response.headers_mut());
        response.headers_mut().insert("x-redirect-by", HeaderValue::from_static(by));
    }
    Ok(with_api_link(response, &loaded, &result))
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
fn with_api_link(mut response: Response<Body>, loaded: &Loaded, result: &rustpress_core::QueryResult) -> Response<Body> {
    if response.status() == StatusCode::NOT_MODIFIED {
        return response;
    }
    let site = &loaded.site;
    // `rest_output_link_header()` and `wp_shortlink_header()` run on `template_redirect`, before the links of the
    // feed or the plugins.
    let mut links = vec![Rest::new(site, &loaded.theme, &loaded.registry).root_link()];
    if let Some(route) = rustpress_theme::head::queried_rest_route(site, result) {
        links.push(format!("<{}>; rel=\"alternate\"; title=\"JSON\"; type=\"application/json\"", rustpress_core::formatting::esc_url_raw(&site.rest_url(&route))));
    }
    if let Some(shortlink) = rustpress_theme::head::shortlink(site, result) {
        links.push(format!("<{shortlink}>; rel=shortlink"));
    }
    let others: Vec<HeaderValue> = response.headers().get_all(header::LINK).iter().cloned().collect();
    let headers = response.headers_mut();
    headers.remove(header::LINK);
    for link in links.iter().filter_map(|l| HeaderValue::from_str(l).ok()).chain(others) {
        headers.append(header::LINK, link);
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

/// A complete answer of a plugin.
fn plugin_response(answer: &rustpress_core::native::PluginResponse) -> Response<Body> {
    let mut response = Response::new(Body::from(answer.body.clone()));
    *response.status_mut() = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::OK);
    for (name, value) in &answer.headers {
        if let (Ok(name), Ok(value)) = (header::HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            response.headers_mut().append(name, value);
        }
    }
    response
}

/// Headers sent with `header()`: each replaces one of the same name.
fn set_headers<'a>(response: &mut Response<Body>, headers: impl IntoIterator<Item = &'a (String, String)>) {
    for (name, value) in headers {
        if let (Ok(name), Ok(value)) = (header::HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            response.headers_mut().insert(name, value);
        }
    }
}

/// A redirect a plugin asks for on `template_redirect`, with what `send_headers` and `wp_redirect` add. It exits
/// before `rest_output_link_header()` (`template_redirect` at priority 11).
async fn plugin_redirect(loaded: &Loaded, request: &rustpress_core::native::PluginRequest<'_>, sent: &[(String, String)], result: &rustpress_core::QueryResult, plugin: &rustpress_core::native::Redirect) -> Result<Response<Body>> {
    let mut response = redirect(&plugin.location);
    if let Some(status) = StatusCode::from_u16(plugin.status).ok().filter(StatusCode::is_redirection) {
        *response.status_mut() = status;
    }
    singular_headers(&mut response, result, loaded);
    let redirected = rustpress_core::native::Redirected { location: &plugin.location, status: plugin.status, by: plugin.by };
    let plugin_headers = loaded.site.plugins.wp_redirect(&loaded.site, request, &redirected).await?;
    set_headers(&mut response, sent.iter().chain(&plugin_headers));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
    headers.insert("x-redirect-by", HeaderValue::from_static(plugin.by));
    Ok(response)
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
    let loaded = app.current().await;
    let path = request.uri().path().to_owned();
    let query = request.uri().query().unwrap_or("").to_owned();
    let Some((route, rest_query)) = rest_route(&loaded.site.settings.home_path(), &path, &query) else { return Err(request) };
    // A read-only site still takes what visitors send to the routes of plugins (a contact form).
    if app.writer.is_none() && (matches!(*request.method(), Method::GET | Method::HEAD) || !Rest::new(&loaded.site, &loaded.theme, &loaded.registry).is_plugin_route(&route)) {
        return Err(request);
    }
    let (parts, body) = request.into_parts();
    let mut method = parts.method.as_str().to_owned();
    // `WP_REST_Server::serve_request()`: `$_GET['_method']`, then the header; `set_method()` upper-cases.
    let requested = rest_query.split('&').find_map(|pair| pair.strip_prefix("_method=")).map(|value| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(value.replace('+', " ").as_bytes())).into_owned());
    if let Some(over) = requested.as_deref().or_else(|| parts.headers.get("x-http-method-override").and_then(|v| v.to_str().ok())) {
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
    let limit = if known {
        UPLOAD_BODY_LIMIT
    } else if Rest::new(&loaded.site, &loaded.theme, &loaded.registry).is_plugin_route(&route) {
        FORM_BODY_LIMIT
    } else {
        VISITOR_BODY_LIMIT
    };
    let bytes = match axum::body::to_bytes(body, limit).await {
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
        rustpress_rest::edit::Body::Raw { bytes: bytes.to_vec(), content_type: content_type.clone(), disposition }
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
    let referer = parts.headers.get(header::REFERER).and_then(|v| v.to_str().ok()).map(str::to_owned);
    let rest = rest.with_mailer(loaded.mail.clone());
    let request = rustpress_rest::edit::Request { method, body: rest_body, basic: crate::admin::basic_credentials(&parts.headers), remote_addr, user_agent, content_type: Some(content_type).filter(|c| !c.is_empty()), referer };
    let reply = rest.handle_as(&route, &rest_query, access, request).await;
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
    for (name, value) in &feed.headers {
        if let (Ok(name), Ok(value)) = (header::HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            headers.append(name, value);
        }
    }
    response
}

fn text(status: StatusCode, body: &'static str) -> Response<Body> {
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=UTF-8"));
    response
}
