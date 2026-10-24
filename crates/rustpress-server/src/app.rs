//! Application state and request handling.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, Method, Request, Response, StatusCode, header};
use rustpress_blocks::Registry;
use rustpress_core::Site;
use rustpress_db::Db;
use rustpress_theme::{Theme, render_document};
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
        let site = Site::load(db.clone()).await?;
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
    if let Some(response) = app.statics.serve(request).await {
        return response;
    }
    match page(&app, uri.path(), uri.query().unwrap_or("")).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!(error = %e, path = uri.path(), "request failed");
            text(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error\n")
        }
    }
}

async fn page(app: &App, path: &str, query: &str) -> Result<Response<Body>> {
    let loaded = app.current().await;
    let result = loaded.site.query(path, query).await?;
    if result.flags.is_feed {
        return Ok(text(StatusCode::NOT_IMPLEMENTED, "Feeds are not implemented yet.\n"));
    }
    let document = render_document(&loaded.theme, &loaded.site, &result, &loaded.registry).await?;
    let mut response = Response::new(Body::from(document.html));
    *response.status_mut() = StatusCode::from_u16(document.status).unwrap_or(StatusCode::OK);
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
    Ok(response)
}

fn text(status: StatusCode, body: &'static str) -> Response<Body> {
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=UTF-8"));
    response
}
