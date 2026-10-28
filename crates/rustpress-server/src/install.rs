//! A site that is not set up yet: like `wp-load.php` and `wp_not_installed()` every request is sent to the
//! installer of `rustpress-admin`; only the static files (its styles and scripts) are served.

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, Method, Request, Response, StatusCode, header};
use rustpress_admin::install::{InstallEnv, InstallRequest, fetch};
use rustpress_admin::login::LoginResponse;
use rustpress_db::Writer;

use crate::config::Config;
use crate::statics::Statics;

/// How far the site is.
enum Stage {
    /// An empty webspace: the files of WordPress are downloaded first.
    NoFiles,
    /// No `wp-config.php`: `setup-config.php` writes one.
    NoConfig,
    /// A database, but no site in it: `install.php` creates the tables.
    NoTables { writer: Writer, server_version: String },
}

pub struct Installer {
    config: Config,
    stage: Stage,
    statics: Statics,
    /// `wp-config.php` was written or the site installed: the next request needs another stage.
    finished: std::sync::atomic::AtomicBool,
    /// A download is running (a second click waits for nothing).
    fetching: std::sync::atomic::AtomicBool,
}

/// `$wp_local_package` of a localized download of WordPress.
fn local_package(wp_root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(wp_root.join("wp-includes").join("version.php")).ok()?;
    let value = text.split("$wp_local_package = '").nth(1)?.split('\'').next()?;
    (!value.is_empty()).then(|| value.to_owned())
}

/// `is_blog_installed()`: the options table has a `siteurl`.
async fn is_installed(writer: &Writer) -> bool {
    writer.column_string(&format!("SELECT option_value FROM {} WHERE option_name = 'siteurl' LIMIT 1", writer.table("options"))).await.is_ok_and(|rows| !rows.is_empty())
}

impl Installer {
    /// The installer `config` needs, or `None` when the site is set up (or its database cannot be reached: that
    /// is an error for the site itself to report).
    pub async fn needed(config: &Config) -> Option<Arc<Self>> {
        let statics = Statics::new(&config.wp_root);
        let new = |stage| Some(Arc::new(Self { config: config.clone(), stage, statics, finished: Default::default(), fetching: Default::default() }));
        if config.database_url.is_empty() {
            if config.wp_config_path().is_some() {
                return None;
            }
            if !config.wp_root.join("wp-includes").join("version.php").is_file() {
                return new(Stage::NoFiles);
            }
            return new(Stage::NoConfig);
        }
        let writer = Writer::connect(&config.database_url, &config.table_prefix).await.ok()?;
        let server_version = writer.column_string("SELECT VERSION()").await.ok()?.into_iter().next().unwrap_or_default();
        if is_installed(&writer).await {
            return None;
        }
        new(Stage::NoTables { writer, server_version })
    }

    pub fn router(self: &Arc<Self>) -> Router {
        Router::new().fallback(handle).with_state(self.clone())
    }

    fn env(&self, url: String) -> InstallEnv {
        let includes = self.config.wp_root.join("wp-includes");
        InstallEnv { wp_root: self.config.wp_root.clone(), url, version: crate::admin::wp_version(&includes), local_package: local_package(&self.config.wp_root) }
    }

    /// `wp-config.php` next to `wp-config-sample.php`. It holds the database password: only its owner may
    /// read it (WordPress leaves it at 0666; under CGI the program runs as that owner).
    fn write_config(&self, text: &str) -> bool {
        let path = self.config.wp_root.join("wp-config.php");
        if path.exists() {
            return false;
        }
        let written = std::fs::write(&path, text).is_ok();
        #[cfg(unix)]
        if written {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        if written {
            self.finished.store(true, std::sync::atomic::Ordering::Release);
        }
        written
    }
}

/// `wp_guess_url()`: the scheme and host of the request (RustPress serves the site at the root).
fn guess_url(config: &Config, parts: &axum::http::request::Parts) -> String {
    let client = crate::client::Client::of(config, parts, if config.request_https { "https:" } else { "http:" });
    let host = parts.headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("localhost");
    format!("{}://{host}", if client.https { "https" } else { "http" })
}

fn http(reply: LoginResponse) -> Response<Body> {
    let mut response = Response::new(Body::from(reply.body));
    *response.status_mut() = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::OK);
    for (name, value) in &reply.headers {
        if let (Ok(name), Ok(value)) = (header::HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            response.headers_mut().append(name, value);
        }
    }
    response
}

async fn handle(axum::extract::State(installer): axum::extract::State<Arc<Installer>>, request: Request<Body>) -> Response<Body> {
    let path = request.uri().path().to_owned();
    let get = matches!(*request.method(), Method::GET | Method::HEAD);
    let post = request.method() == Method::POST;
    if get && !path.ends_with(".php") {
        let probe = Request::builder().method(request.method().clone()).uri(request.uri().clone()).body(Body::empty());
        if let Ok(mut probe) = probe {
            *probe.headers_mut() = request.headers().clone();
            if let Some(response) = installer.statics.serve(probe).await {
                return response;
            }
        }
    }
    let query = request.uri().query().unwrap_or("").to_owned();
    let (parts, body) = request.into_parts();
    let url = guess_url(&installer.config, &parts);
    let env = installer.env(url.clone());
    let form = if post {
        match axum::body::to_bytes(body, 64 * 1024).await {
            Ok(bytes) => crate::admin::pairs_of_public(&String::from_utf8_lossy(&bytes)),
            Err(_) => return Response::builder().status(StatusCode::PAYLOAD_TOO_LARGE).body(Body::from("Payload Too Large\n")).unwrap_or_default(),
        }
    } else {
        Vec::new()
    };
    let query = crate::admin::pairs_of_public(&query);
    let install_request = InstallRequest { post, query: &query, form: &form };
    let setup = path.ends_with("/wp-admin/setup-config.php");
    let install = path.ends_with("/wp-admin/install.php");
    let reply = match &installer.stage {
        Stage::NoFiles if post && setup => fetch(&installer, &url, &form).await,
        Stage::NoFiles => fetch::page(&tokio::task::spawn_blocking(fetch::translations).await.unwrap_or_default(), None),
        Stage::NoConfig if setup => env.setup_config(&install_request, |text| installer.write_config(text)).await,
        // `wp-load.php` without `wp-config.php`.
        Stage::NoConfig => rustpress_admin::install::redirect(&format!("{url}/wp-admin/setup-config.php"), false),
        Stage::NoTables { .. } if setup => env.config_exists(),
        Stage::NoTables { writer, server_version } if install => {
            let reply = env.install(&install_request, writer, server_version, crate::admin::now()).await;
            if is_installed(writer).await {
                installer.finished.store(true, std::sync::atomic::Ordering::Release);
            }
            reply
        }
        // `wp_not_installed()`.
        Stage::NoTables { .. } => rustpress_admin::install::redirect(&format!("{url}/wp-admin/install.php"), true),
    };
    if matches!(installer.stage, Stage::NoFiles) {
        return http(reply);
    }
    http(LoginResponse { body: rustpress_admin::install::brand::rebrand(&reply.body), ..reply })
}

/// Downloads WordPress into the empty webspace, then goes on with `setup-config.php` in the chosen language.
async fn fetch(installer: &Arc<Installer>, url: &str, form: &[(String, String)]) -> LoginResponse {
    use std::sync::atomic::Ordering;
    let language: String = form.iter().find(|(k, _)| k == "language").map(|(_, v)| v.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect()).unwrap_or_default();
    if installer.fetching.swap(true, Ordering::AcqRel) {
        return fetch::page(&[], Some("The download is already running. Reload this page in a minute."));
    }
    let root = installer.config.wp_root.clone();
    let locale = language.clone();
    let result = tokio::task::spawn_blocking(move || fetch::install(&root, &locale, fetch::Mode::Fresh)).await.unwrap_or_else(|e| Err(e.to_string()));
    installer.fetching.store(false, Ordering::Release);
    match result {
        Ok(done) => {
            tracing::info!(written = done.written, kept = done.kept, "WordPress {} downloaded", fetch::VERSION);
            installer.finished.store(true, Ordering::Release);
            let next = if language.is_empty() || language == "en_US" { String::new() } else { format!("?step=0&language={language}") };
            rustpress_admin::install::redirect(&format!("{url}/wp-admin/setup-config.php{next}"), false)
        }
        Err(e) => {
            tracing::error!(error = %e, "downloading WordPress failed");
            fetch::page(&tokio::task::spawn_blocking(fetch::translations).await.unwrap_or_default(), Some(&e))
        }
    }
}

/// The site of a server that keeps running: the installer until the site is set up, then RustPress.
pub struct Gate {
    load: Box<dyn Fn() -> Result<Config, crate::config::ConfigError> + Send + Sync>,
    current: tokio::sync::RwLock<Current>,
}

#[derive(Clone)]
enum Current {
    Installer(Arc<Installer>),
    Site(Router),
}

impl Gate {
    /// `load` reads the configuration again once the installer wrote `wp-config.php`.
    pub async fn start(load: Box<dyn Fn() -> Result<Config, crate::config::ConfigError> + Send + Sync>) -> Result<Arc<Self>, String> {
        let current = Self::current_for(&load).await?;
        Ok(Arc::new(Self { load, current: tokio::sync::RwLock::new(current) }))
    }

    async fn current_for(load: &(dyn Fn() -> Result<Config, crate::config::ConfigError> + Send + Sync)) -> Result<Current, String> {
        let config = load().map_err(|e| e.to_string())?;
        if let Some(installer) = Installer::needed(&config).await {
            tracing::info!("the site is not set up yet: serving the installer");
            return Ok(Current::Installer(installer));
        }
        let config = if config.database_url.is_empty() { crate::Config::load_with(None, Some(config.wp_root.clone())).map_err(|e| e.to_string())? } else { config };
        Ok(Current::Site(crate::App::new(config).await.map_err(|e| e.to_string())?.router()))
    }

    pub fn router(self: &Arc<Self>) -> Router {
        Router::new().fallback(dispatch).with_state(self.clone())
    }
}

async fn dispatch(axum::extract::State(gate): axum::extract::State<Arc<Gate>>, request: Request<Body>) -> Response<Body> {
    use tower::ServiceExt;
    let current = gate.current.read().await.clone();
    match current {
        Current::Site(router) => router.oneshot(request).await.unwrap_or_else(|never| match never {}),
        Current::Installer(installer) => {
            let response = installer.router().oneshot(request).await.unwrap_or_else(|never| match never {});
            if installer.finished.load(std::sync::atomic::Ordering::Acquire) {
                match Gate::current_for(&gate.load).await {
                    Ok(next) => *gate.current.write().await = next,
                    Err(e) => tracing::error!(error = %e, "the installed site does not start"),
                }
            }
            response
        }
    }
}
