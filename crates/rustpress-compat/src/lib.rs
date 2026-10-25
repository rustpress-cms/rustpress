//! Compatibility run: the same URLs against a real WordPress and against RustPress, one
//! comparison per URL, and a Markdown report with the differences. Documented deviations
//! (`compat/known.toml`) do not count as failures.

pub mod compare;
pub mod http;
pub mod known;
pub mod report;
pub mod urls;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::http::Request;
use futures::StreamExt;
use http_body_util::BodyExt;
use rustpress_server::{App, Config};
use tower::ServiceExt;

pub use known::Known;
pub use report::{Finding, Outcome, Report};
pub use urls::{Kind, Target};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the reference WordPress at {0} is not reachable")]
    Unreachable(String),
    #[error("cannot start RustPress: {0}")]
    Server(#[from] rustpress_server::Error),
    #[error("{0}")]
    Known(String),
}

#[derive(Debug, Clone)]
pub struct Options {
    /// `host:port` of the reference WordPress.
    pub reference: String,
    /// The `Host` header to send to it: the host of its `home` option.
    pub host: String,
    pub database_url: String,
    pub wp_root: PathBuf,
    /// At most this many URLs.
    pub limit: usize,
    /// Parallel requests against the (slow) reference.
    pub concurrency: usize,
    /// Only paths matching this regular expression.
    pub only: Option<String>,
    pub known: PathBuf,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            reference: "127.0.0.1:8080".into(),
            host: "localhost:8080".into(),
            database_url: "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into(),
            wp_root: PathBuf::from(r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data"),
            limit: 1500,
            concurrency: 4,
            only: None,
            known: PathBuf::from("compat/known.toml"),
        }
    }
}

/// The `wp-theme-<slug>` body class of the reference's front page, i.e. the active theme.
fn theme_slug(home: &str) -> String {
    home.split("wp-theme-").nth(1).map(|rest| rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect()).unwrap_or_default()
}

/// `<meta name="generator" content="WordPress 7.1.3" />`.
fn wp_version(home: &str) -> String {
    home.split("name=\"generator\" content=\"WordPress ").nth(1).and_then(|rest| rest.split('"').next()).unwrap_or("?").to_owned()
}

async fn ours(app: &Arc<App>, path: &str) -> http::Response {
    let request = Request::builder().uri(path).body(Body::empty()).expect("valid request");
    let response = app.router().oneshot(request).await.expect("infallible");
    let (parts, body) = response.into_parts();
    http::Response {
        status: parts.status.as_u16(),
        headers: parts.headers.iter().map(|(n, v)| (n.as_str().to_owned(), String::from_utf8_lossy(v.as_bytes()).into_owned())).collect(),
        body: body.collect().await.map(|b| b.to_bytes().to_vec()).unwrap_or_default(),
    }
}

/// Fetches one path from both sides: `(ours, theirs)`.
pub async fn fetch_both(options: &Options, path: &str) -> Result<(http::Response, http::Response), Error> {
    let reference = http::Reference::new(&options.reference, &options.host);
    let theirs = {
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || reference.get(&path)).await.expect("task").map_err(|_| Error::Unreachable(options.reference.clone()))?
    };
    let config = Config { database_url: options.database_url.clone(), wp_root: options.wp_root.clone(), reload_secs: 3600, ..Config::default() };
    let app = App::new(config).await?;
    Ok((ours(&app, path).await, theirs))
}

/// Runs the comparison. `progress` is called after each URL with `(done, total, path)`.
pub async fn run(options: &Options, progress: impl Fn(usize, usize, &str)) -> Result<Report, Error> {
    let reference = http::Reference::new(&options.reference, &options.host);
    let home = tokio::task::spawn_blocking({
        let reference = reference.clone();
        move || reference.get("/")
    })
    .await
    .expect("task")
    .map_err(|_| Error::Unreachable(options.reference.clone()))?
    .text();
    let theme = theme_slug(&home);
    let known = Arc::new(Known::load(&options.known).map_err(Error::Known)?);

    let config = Config { database_url: options.database_url.clone(), wp_root: options.wp_root.clone(), reload_secs: 3600, ..Config::default() };
    let app = App::new(config).await?;

    let limit = options.limit;
    let mut targets = tokio::task::spawn_blocking({
        let reference = reference.clone();
        move || urls::discover(&reference, limit)
    })
    .await
    .expect("task");
    if let Some(pattern) = &options.only {
        let regex = regex::Regex::new(pattern).map_err(|e| Error::Known(e.to_string()))?;
        targets.retain(|t| regex.is_match(&t.path));
    }

    let total = targets.len();
    let mut results: Vec<(usize, Outcome)> = Vec::with_capacity(total);
    let mut stream = futures::stream::iter(targets.into_iter().enumerate())
        .map(|(index, target)| {
            let app = app.clone();
            let reference = reference.clone();
            let known = known.clone();
            let theme = theme.clone();
            async move {
                let started = Instant::now();
                let theirs = {
                    let path = target.path.clone();
                    tokio::task::spawn_blocking(move || reference.get(&path).unwrap_or_default()).await.expect("task")
                };
                let theirs_ms = started.elapsed().as_millis();
                let started = Instant::now();
                let mine = ours(&app, &target.path).await;
                let ours_ms = started.elapsed().as_millis();
                let findings = report::classify(compare::compare(target.kind, &mine, &theirs), &target.path, &theme, &known);
                (index, Outcome { target, ours_ms, theirs_ms, findings })
            }
        })
        .buffer_unordered(options.concurrency.max(1));
    while let Some((index, outcome)) = stream.next().await {
        progress(results.len() + 1, total, &outcome.target.path);
        results.push((index, outcome));
    }
    results.sort_by_key(|(index, _)| *index);

    Ok(Report {
        label: theme.clone(),
        wp_version: wp_version(&home),
        generated: chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string(),
        outcomes: results.into_iter().map(|(_, o)| o).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_and_version_are_read_from_the_front_page() {
        let html = "<meta name=\"generator\" content=\"WordPress 7.1.3\" /><body class=\"home wp-embed-responsive wp-theme-twentytwentyfive\">";
        assert_eq!(theme_slug(html), "twentytwentyfive");
        assert_eq!(wp_version(html), "7.1.3");
        assert_eq!(theme_slug("<body>"), "");
    }
}
