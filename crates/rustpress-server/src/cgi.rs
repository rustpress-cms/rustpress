//! RustPress as a CGI program: one process per request, the request read from the environment and stdin, the
//! response written to stdout. This runs RustPress on shared hosting that starts programs from `cgi-bin/` but
//! keeps no processes running (Strato and the like).
//!
//! Without configuration the WordPress directory is the `DOCUMENT_ROOT` of the request and the database comes from
//! its `wp-config.php`; a `rustpress.toml` next to the program and the usual environment variables still count.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderName, HeaderValue, Method, Request, Response, header};
use tower::ServiceExt;

use crate::config::{Config, ConfigError};

/// The web server started this process as a CGI program.
pub fn is_cgi() -> bool {
    std::env::var("GATEWAY_INTERFACE").is_ok_and(|v| v.starts_with("CGI/"))
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// The configuration of a CGI run.
/// Its `database_url` is empty while the site has no `wp-config.php` yet.
pub fn config() -> Result<Config, ConfigError> {
    // The web server starts the program in its own directory; `current_exe()` would need `/proc`, which shared
    // hosting may lack.
    let file = std::env::var_os("RUSTPRESS_CONFIG").map(PathBuf::from).or_else(|| {
        let beside = std::env::current_dir().ok()?.join("rustpress.toml");
        beside.is_file().then_some(beside)
    });
    let mut config = Config::load_without_database(file, var("DOCUMENT_ROOT").map(|root| local_path(PathBuf::from(root))))?;
    config.request_https = https();
    Ok(config)
}

/// `HTTPS=on`: the web server received the request over TLS.
fn https() -> bool {
    var("HTTPS").is_some_and(|v| v.eq_ignore_ascii_case("on"))
}

/// A path as the web server names it, as this process sees it. On shared hosting the web server may see the
/// webspace under another mount point than the program it starts (Strato: `/home/strato/http/…/htdocs` for
/// Apache, `/mnt/web…/htdocs` for the program). The working directory is the directory of `SCRIPT_FILENAME`
/// under the program's own mount point, so the two differ only in what precedes their common end.
fn local_path(path: PathBuf) -> PathBuf {
    if path.exists() {
        return path;
    }
    let script_dir = var("SCRIPT_FILENAME").map(PathBuf::from).and_then(|p| p.parent().map(Path::to_path_buf));
    let mapped = script_dir.zip(std::env::current_dir().ok()).and_then(|(script_dir, cwd)| translate(&path, &script_dir, &cwd));
    mapped.filter(|p| p.exists()).unwrap_or(path)
}

/// `path` with the part before the common end of `server_dir` and `local_dir` swapped.
fn translate(path: &Path, server_dir: &Path, local_dir: &Path) -> Option<PathBuf> {
    let server: Vec<_> = server_dir.components().collect();
    let local: Vec<_> = local_dir.components().collect();
    let common = server.iter().rev().zip(local.iter().rev()).take_while(|(a, b)| a == b).count();
    if common == 0 || common == server.len() {
        return None;
    }
    let from: PathBuf = server[..server.len() - common].iter().collect();
    let to: PathBuf = local[..local.len() - common].iter().collect();
    Some(to.join(path.strip_prefix(&from).ok()?))
}

/// Answers the one request of this process.
///
/// `Server-Timing` reports start-up (`load`, since `started`) and the whole run (`app`), so the time
/// spent in RustPress can be told apart from the web server and the network in the browser.
pub async fn serve(router: axum::Router, started: std::time::Instant, load: std::time::Duration) -> std::io::Result<()> {
    let request = request_from_env()?;
    let head = request.method() == Method::HEAD;
    let mut response = match router.oneshot(request).await {
        Ok(response) => response,
        Err(never) => match never {},
    };
    let timing = format!("load;dur={:.1}, app;dur={:.1}", load.as_secs_f64() * 1000.0, started.elapsed().as_secs_f64() * 1000.0);
    if let Ok(value) = timing.parse() {
        response.headers_mut().insert("server-timing", value);
    }
    write_response(response, head).await
}

/// The request as the web server hands it over (RFC 3875): meta-variables, `HTTP_*` headers and the body on stdin.
fn request_from_env() -> std::io::Result<Request<Body>> {
    let invalid = |e: String| std::io::Error::new(std::io::ErrorKind::InvalidData, e);
    let method = var("REQUEST_METHOD").unwrap_or_else(|| "GET".to_owned());
    // Apache keeps the address the visitor asked for in `REQUEST_URI`, also after a rewrite to the program.
    let uri = var("REQUEST_URI").unwrap_or_else(|| {
        let path = format!("{}{}", var("SCRIPT_NAME").unwrap_or_default(), var("PATH_INFO").unwrap_or_default());
        match var("QUERY_STRING") {
            Some(query) => format!("{path}?{query}"),
            None => path,
        }
    });
    let mut builder = Request::builder().method(method.as_str()).uri(uri.as_str());
    for (key, value) in std::env::vars_os() {
        let (Some(key), Some(value)) = (key.to_str(), value.to_str()) else { continue };
        let name = match key {
            "CONTENT_TYPE" => "content-type".to_owned(),
            "CONTENT_LENGTH" => "content-length".to_owned(),
            // `HTTP_PROXY` comes from a `Proxy:` header of the visitor (httpoxy), never a real header to pass on.
            "HTTP_PROXY" => continue,
            _ => match key.strip_prefix("HTTP_") {
                Some(name) => name.to_ascii_lowercase().replace('_', "-"),
                None => continue,
            },
        };
        if let (Ok(name), Ok(value)) = (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            builder = builder.header(name, value);
        }
    }
    // Apache hands `Authorization` (application passwords) to CGI programs only through a rewrite rule, and after
    // the internal redirect to the program the variable carries a `REDIRECT_` prefix.
    if std::env::var_os("HTTP_AUTHORIZATION").is_none() && let Some(value) = var("REDIRECT_HTTP_AUTHORIZATION").and_then(|v| HeaderValue::from_str(&v).ok()) {
        builder = builder.header(header::AUTHORIZATION, value);
    }
    let length: usize = var("CONTENT_LENGTH").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut body = Vec::with_capacity(length);
    std::io::stdin().take(length as u64).read_to_end(&mut body)?;
    let ip = var("REMOTE_ADDR").and_then(|v| v.parse::<IpAddr>().ok()).unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    let port = var("REMOTE_PORT").and_then(|v| v.parse().ok()).unwrap_or(0);
    builder
        .extension(ConnectInfo(SocketAddr::new(ip, port)))
        .extension(crate::client::ServerHttps(https()))
        .body(Body::from(body))
        .map_err(|e| invalid(e.to_string()))
}

/// The response in CGI form: a `Status:` line, the headers, a blank line and the body.
async fn write_response(response: Response<Body>, head: bool) -> std::io::Result<()> {
    let (parts, body) = response.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX).await.map_err(std::io::Error::other)?;
    let mut out = Vec::with_capacity(body.len() + 512);
    write!(out, "Status: {} {}\r\n", parts.status.as_u16(), parts.status.canonical_reason().unwrap_or(""))?;
    for (name, value) in &parts.headers {
        if name == header::TRANSFER_ENCODING || name == header::CONNECTION {
            continue;
        }
        out.extend_from_slice(name.as_str().as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    if !head {
        out.extend_from_slice(&body);
    }
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&out)?;
    stdout.flush()
}

/// What the visitor sees when RustPress cannot start. The details go to the error log of the web server and to
/// `rustpress-error.log` next to the program, since shared hosting rarely shows the former (in `cgi-bin/` the web
/// server runs files instead of handing them out, so the log stays private).
pub fn write_failure(message: &str) {
    if let Ok(mut log) = std::fs::OpenOptions::new().create(true).append(true).open("rustpress-error.log") {
        let _ = writeln!(log, "{} {} {message}", unix_seconds(), var("REQUEST_URI").unwrap_or_default());
    }
    let _ = std::io::stdout().lock().write_all(b"Status: 500 Internal Server Error\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nRustPress could not start. The error log of the web server has the details.\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_between_mount_points() {
        let mapped = translate(
            Path::new("/home/strato/http/power/rid/45/67/1234567/htdocs/STRATO-apps/wordpress_02/app/"),
            Path::new("/home/strato/http/power/rid/45/67/1234567/htdocs/cgi-bin"),
            Path::new("/mnt/web100/c1/67/1234567/htdocs/cgi-bin"),
        );
        assert_eq!(mapped, Some(PathBuf::from("/mnt/web100/c1/67/1234567/htdocs/STRATO-apps/wordpress_02/app")));
        assert_eq!(translate(Path::new("/a/x"), Path::new("/a/cgi-bin"), Path::new("/b/other")), None);
        assert_eq!(translate(Path::new("/elsewhere/x"), Path::new("/a/cgi-bin"), Path::new("/b/cgi-bin")), None);
    }
}

/// Seconds since the Unix epoch, enough to order the lines of the error log.
fn unix_seconds() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
