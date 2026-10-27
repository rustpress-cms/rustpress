//! RustPress as a CGI program: one process per request, the request read from the environment and stdin, the
//! response written to stdout. This runs RustPress on shared hosting that starts programs from `cgi-bin/` but
//! keeps no processes running (Strato and the like).
//!
//! Without configuration the WordPress directory is the `DOCUMENT_ROOT` of the request and the database comes from
//! its `wp-config.php`; a `rustpress.toml` next to the program and the usual environment variables still count.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderName, HeaderValue, Method, Request, Response, header};
use tower::ServiceExt;

use crate::App;
use crate::config::{Config, ConfigError};

/// The web server started this process as a CGI program.
pub fn is_cgi() -> bool {
    std::env::var("GATEWAY_INTERFACE").is_ok_and(|v| v.starts_with("CGI/"))
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// The configuration of a CGI run.
pub fn config() -> Result<Config, ConfigError> {
    // `SCRIPT_FILENAME` is the program itself; `current_exe()` needs `/proc`, which shared hosting may lack.
    let file = std::env::var_os("RUSTPRESS_CONFIG").map(PathBuf::from).or_else(|| {
        let program = var("SCRIPT_FILENAME").map(PathBuf::from).or_else(|| std::env::current_exe().ok())?;
        let beside = program.with_file_name("rustpress.toml");
        beside.is_file().then_some(beside)
    });
    Config::load_with(file, var("DOCUMENT_ROOT").map(PathBuf::from))
}

/// Answers the one request of this process.
pub async fn serve(app: Arc<App>) -> std::io::Result<()> {
    let request = request_from_env()?;
    let head = request.method() == Method::HEAD;
    let response = match app.router().oneshot(request).await {
        Ok(response) => response,
        Err(never) => match never {},
    };
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

/// What the visitor sees when RustPress cannot start; the details go to the error log of the web server.
pub fn write_failure() {
    let _ = std::io::stdout().lock().write_all(b"Status: 500 Internal Server Error\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nRustPress could not start. The error log of the web server has the details.\n");
}
