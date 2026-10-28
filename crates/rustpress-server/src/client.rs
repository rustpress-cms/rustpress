//! The visitor behind a request: its address and whether it came over HTTPS. Behind a proxy named in
//! `trusted_proxies` the `X-Forwarded-For` and `X-Forwarded-Proto` headers of that proxy count; from anybody else
//! they are ignored (anybody can send them).

use axum::http::request::Parts;

use crate::config::Config;

#[derive(Debug, Clone, Default)]
pub struct Client {
    /// `$_SERVER['REMOTE_ADDR']`
    pub ip: Option<String>,
    /// `is_ssl()`
    pub https: bool,
}

/// Whether the web server in front received the request over TLS, when it says so itself (CGI: `HTTPS=on`).
#[derive(Debug, Clone, Copy)]
pub struct ServerHttps(pub bool);

impl Client {
    /// `siteurl` decides `https` when no trusted proxy says how the request came in (RustPress itself speaks HTTP).
    pub fn of(config: &Config, parts: &Parts, siteurl: &str) -> Self {
        let peer = parts.extensions.get::<axum::extract::ConnectInfo<std::net::SocketAddr>>().map(|c| c.0.ip().to_string());
        let trusted = |ip: &str| config.trusted_proxies.iter().any(|p| p == ip);
        let via_proxy = peer.as_deref().is_some_and(trusted);
        let header = |name: &str| parts.headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_owned);
        let mut ip = peer.clone();
        let mut https = match parts.extensions.get::<ServerHttps>() {
            Some(ServerHttps(on)) => *on,
            None => siteurl.starts_with("https:"),
        };
        if via_proxy {
            // The last address that is not one of our proxies is the visitor.
            if let Some(forwarded) = header("x-forwarded-for") {
                let hops: Vec<&str> = forwarded.split(',').map(str::trim).filter(|h| !h.is_empty()).collect();
                if let Some(visitor) = hops.iter().rev().find(|h| !trusted(h)).or(hops.first()) {
                    ip = Some((*visitor).to_owned());
                }
            }
            if let Some(proto) = header("x-forwarded-proto") {
                https = proto.split(',').next().is_some_and(|p| p.trim().eq_ignore_ascii_case("https"));
            }
        }
        Self { ip, https }
    }
}
