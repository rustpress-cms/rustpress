//! A minimal blocking HTTP/1.0 client for the reference WordPress (no redirects, no TLS).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// A response as far as the comparison needs it.
#[derive(Debug, Clone, Default)]
pub struct Response {
    pub status: u16,
    /// Header names in lowercase, in the order received.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The media type without parameters.
    pub fn media_type(&self) -> String {
        self.header("content-type").unwrap_or_default().split(';').next().unwrap_or_default().trim().to_ascii_lowercase()
    }
}

/// Where the reference WordPress listens and which `Host` it expects (WordPress redirects
/// requests for any other host to the one in its `home` option).
#[derive(Debug, Clone)]
pub struct Reference {
    /// `host:port` to connect to.
    pub addr: String,
    /// The `Host` header to send.
    pub host: String,
}

impl Reference {
    pub fn new(addr: &str, host: &str) -> Self {
        Self { addr: addr.to_owned(), host: host.to_owned() }
    }

    /// `GET path`.
    pub fn get(&self, path: &str) -> std::io::Result<Response> {
        get(&self.addr, &self.host, path)
    }
}

/// `GET path` against `addr` (`host:port`), sending `Host: host`.
pub fn get(addr: &str, host: &str, path: &str) -> std::io::Result<Response> {
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: {host}\r\nAccept: */*\r\nConnection: close\r\n\r\n")?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    Ok(parse(&raw))
}

pub fn parse(raw: &[u8]) -> Response {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n");
    let (head, body) = match split {
        Some(at) => (&raw[..at], &raw[at + 4..]),
        None => (raw, &raw[raw.len()..]),
    };
    let head = String::from_utf8_lossy(head);
    let mut lines = head.lines();
    let status = lines.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|s| s.parse().ok()).unwrap_or(0);
    let headers = lines.filter_map(|l| l.split_once(':')).map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_owned())).collect();
    Response { status, headers, body: body.to_vec() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responses_are_split_into_status_headers_and_body() {
        let r = parse(b"HTTP/1.1 301 Moved Permanently\r\nLocation: /x/\r\nContent-Type: text/html; charset=UTF-8\r\n\r\nbody");
        assert_eq!(r.status, 301);
        assert_eq!(r.header("location"), Some("/x/"));
        assert_eq!(r.media_type(), "text/html");
        assert_eq!(r.text(), "body");
    }

    #[test]
    fn a_truncated_response_has_no_body() {
        let r = parse(b"HTTP/1.1 200 OK\r\nA: b");
        assert_eq!(r.status, 200);
        assert!(r.body.is_empty());
    }
}
