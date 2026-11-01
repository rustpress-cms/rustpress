//! `Redirection_Request`, `Redirection_IP` and the logs (`Red_Redirect_Log`, `Red_404_Log`).

use std::net::IpAddr;

use rustpress_core::native::PluginRequest;
use rustpress_core::{Result, Site};
use rustpress_db::Val;

use super::options::{Options, sanitize};

fn text_field(text: &str) -> String {
    rustpress_content::sanitize::text_field(text)
}

/// `Redirection_Request::sanitize_url()`.
pub fn sanitize_url(text: &str) -> String {
    sanitize(text)
}

/// `get_request_url()`: `REQUEST_URI` cleaned and without slashes added by PHP.
pub fn request_url(request: &PluginRequest<'_>) -> String {
    stripslashes(&sanitize_url(request.uri))
}

/// `stripslashes()`.
fn stripslashes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('0') => out.push('\0'),
                Some(next) => out.push(next),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn user_agent(request: &PluginRequest<'_>) -> String {
    request.header("user-agent").map(text_field).unwrap_or_default()
}

pub fn referrer(request: &PluginRequest<'_>) -> String {
    request.header("referer").map(sanitize_url).unwrap_or_default()
}

pub fn method(request: &PluginRequest<'_>) -> String {
    text_field(request.method)
}

/// `get_request_server_name()`: `HTTP_HOST` without its port.
pub fn request_server_name(request: &PluginRequest<'_>) -> String {
    text_field(request.host()).split(':').next().unwrap_or_default().to_owned()
}

/// `get_server_name()`: `SERVER_NAME`, which Apache takes from the `Host` header (`UseCanonicalName Off`).
pub fn server_name(request: &PluginRequest<'_>) -> String {
    request_server_name(request)
}

/// `get_server()`.
pub fn server(request: &PluginRequest<'_>) -> String {
    format!("{}://{}", if request.https { "https" } else { "http" }, server_name(request))
}

/// `get_header( $name )`: `$_SERVER['HTTP_*']`.
pub fn header(request: &PluginRequest<'_>, name: &str) -> Option<String> {
    let wanted = name.to_ascii_lowercase().replace('_', "-");
    request.headers.iter().find(|(n, _)| n.replace('_', "-") == wanted).map(|(_, v)| text_field(v))
}

/// `get_cookie( $name )`: `$_COOKIE` (PHP turns `.` and spaces in names into `_` and decodes the values).
pub fn cookie(request: &PluginRequest<'_>, name: &str) -> Option<String> {
    let cookies = request.headers.iter().filter(|(n, _)| n == "cookie").flat_map(|(_, v)| v.split(';'));
    let mut found = None;
    for pair in cookies {
        let Some((key, value)) = pair.trim_start().split_once('=') else { continue };
        let key: String = key.trim_start().chars().map(|c| if c == '.' || c == ' ' { '_' } else { c }).collect();
        // The first cookie of a name wins.
        if key == name && found.is_none() {
            found = Some(text_field(&super::php::urldecode(value)));
        }
    }
    found
}

/// `get_accept_language()`.
pub fn accept_language(request: &PluginRequest<'_>) -> Vec<String> {
    let Some(header) = request.header("accept-language") else { return Vec::new() };
    let header = text_field(header);
    let languages = header.split(';').next().unwrap_or_default().replace(' ', "");
    languages.split(',').map(str::to_owned).collect()
}

/// `Redirection_IP`: the last address of a list, normalized; empty when it is none.
pub fn normalize_ip(ip: &str) -> String {
    let ip = text_field(ip);
    let part = ip.split(',').rev().map(str::trim).find(|p| !p.is_empty()).unwrap_or_default();
    part.parse::<IpAddr>().map(|ip| ip.to_string()).unwrap_or_default()
}

/// `get_ip()` with the `redirection_request_ip` filter of the IP logging setting.
pub fn ip(request: &PluginRequest<'_>, options: &Options) -> String {
    let mut ip = normalize_ip(request.remote_addr);
    let proxies = options.strings("ip_proxy");
    if proxies.is_empty() || proxies.contains(&ip) {
        for name in options.strings("ip_headers") {
            let header = name.strip_prefix("HTTP_").map(|h| h.to_ascii_lowercase().replace('_', "-"));
            let value = match header {
                Some(h) => request.header(&h).map(str::to_owned),
                None if name == "REMOTE_ADDR" => Some(request.remote_addr.to_owned()),
                None => None,
            };
            if let Some(value) = value {
                ip = normalize_ip(&value);
                break;
            }
        }
    }
    if options.is_int("ip_logging", 0) {
        String::new()
    } else if options.is_int("ip_logging", 2) {
        mask_ip(&ip)
    } else {
        ip
    }
}

/// `Redirection::mask_ip()`.
fn mask_ip(ip: &str) -> String {
    let ip = ip.trim();
    if ip.contains(':') {
        return match ip.parse::<std::net::Ipv6Addr>() {
            Ok(v6) => {
                let mut segments = v6.segments();
                segments[4..].fill(0);
                std::net::Ipv6Addr::from(segments).to_string()
            }
            Err(_) => String::new(),
        };
    }
    let mut parts: Vec<String> = if ip.is_empty() { Vec::new() } else { ip.split('.').map(str::to_owned).collect() };
    if let Some(last) = parts.last_mut() {
        *last = "0".to_owned();
    }
    parts.join(".")
}

/// `get_request_headers()`: the `HTTP_*` headers as `Name-Like-This`, without the secret ones.
fn request_headers(request: &PluginRequest<'_>) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (name, value) in request.headers {
        if matches!(name.as_str(), "content-type" | "content-length") {
            continue;
        }
        let pretty = super::php::ucwords(&name.replace(['-', '_'], " ")).replace(' ', "-");
        if ["authorization", "cookie", "host", "proxy-authorization"].contains(&pretty.to_ascii_lowercase().as_str()) {
            continue;
        }
        let value = text_field(value);
        // Apache joins repeated headers.
        match map.get_mut(&pretty) {
            Some(serde_json::Value::String(existing)) => {
                existing.push_str(", ");
                existing.push_str(&value);
            }
            _ => {
                map.insert(pretty, serde_json::Value::String(value));
            }
        }
    }
    serde_json::Value::Object(map)
}

/// `JSON_NUMERIC_CHECK`: numeric strings become numbers.
fn numeric_check(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) if super::php::is_numeric(&s) => match s.trim().parse::<i64>() {
            Ok(n) => n.into(),
            Err(_) => s.trim().parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(serde_json::Value::String(s), serde_json::Value::Number),
        },
        serde_json::Value::Object(map) => serde_json::Value::Object(map.into_iter().map(|(k, v)| (k, numeric_check(v))).collect()),
        serde_json::Value::Array(items) => serde_json::Value::Array(items.into_iter().map(numeric_check).collect()),
        other => other,
    }
}

/// The request details a log entry stores.
pub struct Details {
    pub target: Option<String>,
    pub http_code: i64,
    pub redirect_id: Option<u64>,
    pub redirect_by: Option<String>,
    /// `request_data`: the headers, when the settings log them.
    pub log_headers: bool,
}

/// `Red_Log::sanitize_create()` and the inserts of `Red_Redirect_Log::create()` / `Red_404_Log::create()`.
pub async fn log(site: &Site, request: &PluginRequest<'_>, options: &Options, table: &str, url: &str, details: Details) -> Result<()> {
    let Some(writer) = request.writer else { return Ok(()) };
    let cut = |text: String, max: usize| text.chars().take(max).collect::<String>();
    let created = rustpress_core::datetime::SiteTimezone::from_options(&site.options).format("Y-m-d H:i:s", chrono::Utc::now().timestamp());
    let mut fields: Vec<(&str, Val)> = vec![
        ("url", Val::Str(cut(text_field(&super::php::urldecode(url)), 2000))),
        ("domain", Val::Str(cut(text_field(&server(request)), 255))),
        ("ip", Val::Str(cut(text_field(&ip(request, options)), 45))),
        ("created", Val::Str(created)),
    ];
    let agent = user_agent(request);
    if !agent.is_empty() {
        fields.push(("agent", Val::Str(cut(text_field(&agent), 255))));
    }
    let referrer = referrer(request);
    if !referrer.is_empty() {
        fields.push(("referrer", Val::Str(cut(text_field(&referrer), 255))));
    }
    let method = method(request).to_uppercase();
    if !method.is_empty() {
        let supported = ["GET", "HEAD", "POST", "PUT", "DELETE", "CONNECT", "OPTIONS", "TRACE", "PATCH"];
        fields.push(("request_method", Val::Str(if supported.contains(&method.as_str()) { method } else { String::new() })));
    }
    if details.http_code != 0 {
        fields.push(("http_code", Val::Int(details.http_code)));
    }
    if details.log_headers {
        let data = serde_json::json!({ "headers": request_headers(request) });
        let flags = rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: true };
        fields.push(("request_data", Val::Str(rustpress_blocks::php_json::encode_with(&numeric_check(data), flags))));
    }
    if table == "redirection_logs" {
        fields.push(("redirection_id", Val::Int(details.redirect_id.map_or(0, |id| id as i64))));
        if let Some(target) = details.target {
            fields.push(("sent_to", Val::Str(target)));
        }
        if let Some(by) = details.redirect_by {
            fields.push(("redirect_by", Val::Str(by.chars().take(50).collect::<String>().to_lowercase())));
        }
    }
    writer.insert_row(table, &fields).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks() {
        assert_eq!(mask_ip("192.168.1.77"), "192.168.1.0");
        assert_eq!(mask_ip("2001:db8:1:2:3:4:5:6"), "2001:db8:1:2::");
        assert_eq!(normalize_ip("10.0.0.1, 1.2.3.4 "), "1.2.3.4");
        assert_eq!(normalize_ip("nope"), "");
    }
}
