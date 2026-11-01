//! `Red_Options`: the option `redirection_options` over its defaults, and the custom HTTP headers
//! (`Red_Http_Headers`).

use rustpress_core::Site;
use rustpress_phpser::{Key, Value};

/// The options as `Red_Options::get()` builds them; the values keep their PHP types for the strict comparisons.
#[derive(Debug, Default)]
pub struct Options {
    values: Vec<(String, Value)>,
    /// `get_option( 'redirection_version' )` of very old installs.
    old_version: Option<String>,
}

fn s(v: &str) -> Value {
    Value::Str(v.as_bytes().to_vec())
}

/// `Red_Options::get_default_options()` (the token is random and never read here).
fn defaults() -> Vec<(&'static str, Value)> {
    let empty = || Value::Array(Vec::new());
    vec![
        ("support", Value::Bool(false)),
        ("token", s("")),
        ("monitor_post", Value::Int(0)),
        ("monitor_types", empty()),
        ("associated_redirect", s("")),
        ("auto_target", s("")),
        ("expire_redirect", Value::Int(7)),
        ("expire_404", Value::Int(7)),
        ("log_external", Value::Bool(false)),
        ("log_header", Value::Bool(false)),
        ("track_hits", Value::Bool(true)),
        ("modules", empty()),
        ("redirect_cache", Value::Int(1)),
        ("ip_logging", Value::Int(0)),
        ("ip_headers", empty()),
        ("ip_proxy", empty()),
        ("last_group_id", Value::Int(0)),
        ("rest_api", Value::Int(0)),
        ("https", Value::Bool(false)),
        ("headers", empty()),
        ("database", s("")),
        ("relocate", s("")),
        ("preferred_domain", s("")),
        ("aliases", empty()),
        ("permalinks", empty()),
        ("cache_key", Value::Int(0)),
        ("plugin_update", s("prompt")),
        ("update_notice", Value::Int(0)),
        // `Red_Source_Flags::get_json()`.
        ("flag_query", s("exact")),
        ("flag_case", Value::Bool(false)),
        ("flag_trailing", Value::Bool(false)),
        ("flag_regex", Value::Bool(false)),
    ]
}

fn key_string(key: &Key) -> String {
    match key {
        Key::Int(n) => n.to_string(),
        Key::Str(b) => String::from_utf8_lossy(b).into_owned(),
    }
}

/// `(string) $value`.
pub fn php_string(value: &Value) -> String {
    match value {
        Value::Str(b) => String::from_utf8_lossy(b).into_owned(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => rustpress_blocks::general::php_float(*f),
        Value::Bool(true) => "1".to_owned(),
        _ => String::new(),
    }
}

/// PHP's truthiness.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(b) => !(b.is_empty() || b == b"0"),
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

/// A PHP number as `>=`/`*` see it.
pub fn number(value: &Value) -> f64 {
    match value {
        Value::Int(n) => *n as f64,
        Value::Float(f) => *f,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Str(b) => String::from_utf8_lossy(b).trim().parse().unwrap_or_else(|_| super::php::intval(&String::from_utf8_lossy(b)) as f64),
        _ => 0.0,
    }
}

impl Options {
    /// `Red_Options::build_options()`.
    pub fn load(site: &Site) -> Self {
        let stored = site.options.get("redirection_options");
        let fresh_install = stored.is_none();
        let mut values: Vec<(String, Value)> = match site.options.value("redirection_options") {
            Some(Value::Array(items)) => items.iter().map(|(k, v)| (key_string(k), v.clone())).collect(),
            _ => Vec::new(),
        };
        let defaults = defaults();
        for (key, value) in &defaults {
            match values.iter_mut().find(|(k, _)| k == key) {
                // `isset()`: a stored `null` counts as missing.
                Some(slot) if matches!(slot.1, Value::Null) => slot.1 = value.clone(),
                Some(_) => {}
                None => values.push(((*key).to_owned(), value.clone())),
            }
        }
        let mut options = Self { values, old_version: site.options.get("redirection_version").map(str::to_owned) };
        if fresh_install {
            options.set("flag_case", Value::Bool(true));
            options.set("flag_trailing", Value::Bool(true));
        }
        if number(options.get("monitor_post")) > 0.0 && options.get("monitor_types").as_array().is_some_and(<[_]>::is_empty) {
            options.set("monitor_types", Value::Array(vec![(Key::Int(0), s("post"))]));
        }
        options.values.retain(|(k, _)| defaults.iter().any(|(d, _)| d == k) || k == "database_stage");
        let rest_api = options.get("rest_api");
        if !matches!(rest_api, Value::Int(0 | 1 | 3)) {
            options.set("rest_api", Value::Int(0));
        }
        options
    }

    fn set(&mut self, key: &str, value: Value) {
        match self.values.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => self.values.push((key.to_owned(), value)),
        }
    }

    pub fn get(&self, key: &str) -> &Value {
        static NULL: Value = Value::Null;
        self.values.iter().find(|(k, _)| k == key).map_or(&NULL, |(_, v)| v)
    }

    pub fn string(&self, key: &str) -> String {
        php_string(self.get(key))
    }

    pub fn truthy(&self, key: &str) -> bool {
        truthy(self.get(key))
    }

    /// `$options[ $key ] === $n`.
    pub fn is_int(&self, key: &str, n: i64) -> bool {
        matches!(self.get(key), Value::Int(v) if *v == n)
    }

    /// `$options[ $key ] === true`.
    pub fn is_true(&self, key: &str) -> bool {
        matches!(self.get(key), Value::Bool(true))
    }

    pub fn number(&self, key: &str) -> f64 {
        number(self.get(key))
    }

    /// The strings of an array option.
    pub fn strings(&self, key: &str) -> Vec<String> {
        self.get(key).as_array().unwrap_or_default().iter().map(|(_, v)| php_string(v)).collect()
    }

    /// `Status::needs_installing()`: without a database version the plugin does not start on the website.
    pub fn needs_installing(&self) -> bool {
        self.string("database").is_empty() && self.old_version.is_none()
    }

    /// `Status::does_support( '4.0' )`: the match URLs the website reads exist since database 4.0.
    pub fn supports_match_url(&self) -> bool {
        let version = match self.string("database").as_str() {
            "+OK" => "4.2".to_owned(),
            "" => self.old_version.clone().unwrap_or_default(),
            v => v.to_owned(),
        };
        version_ge(&version, "4.0")
    }

    /// The custom headers (`Red_Http_Headers::normalize()`).
    pub fn headers(&self) -> Vec<Header> {
        self.get("headers").as_array().unwrap_or_default().iter().filter_map(|(_, h)| Header::normalize(h)).collect()
    }
}

/// `version_compare( $a, $b, '>=' )` for dotted numbers.
fn version_ge(a: &str, b: &str) -> bool {
    let parts = |v: &str| v.split(['.', '-', '_', '+']).map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    let (a, b) = (parts(a), parts(b));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    true
}

/// A custom header of the settings.
#[derive(Debug, Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
    /// `site` or `redirect`.
    pub location: String,
}

impl Header {
    fn normalize(value: &Value) -> Option<Self> {
        let field = |key: &str| value.get(key).filter(|v| !matches!(v, Value::Null));
        let text = |key: &str| field(key).map(|v| sanitize(&rustpress_content::sanitize::text_field(&php_string(v)))).unwrap_or_default();
        let location = if field("location").is_some_and(|v| matches!(v, Value::Str(b) if b == b"redirect")) { "redirect" } else { "site" };
        let (name, kind, header_value) = (text("headerName"), text("type"), text("headerValue"));
        if name.is_empty() || kind.is_empty() {
            return None;
        }
        Some(Self { name: dash_case(&name), value: header_value, location: location.to_owned() })
    }
}

/// `Red_Http_Headers::dash_case()`.
pub fn dash_case(name: &str) -> String {
    let spaced: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' }).collect();
    let collapsed = spaced.split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
    super::php::ucwords(&collapsed).replace(' ', "-")
}

/// `Red_Http_Headers::sanitize()` (and `Redirection_Request::sanitize_url()` without the UTF-8 check): no line
/// breaks or anything after them, no control characters.
pub fn sanitize(text: &str) -> String {
    let cut = match text.find(['\r', '\n', '\t']) {
        // `.*?$` stops before a final newline.
        Some(i) if text.ends_with('\n') && text.len() - 1 > i => format!("{}\n", &text[..i]),
        Some(i) => text[..i].to_owned(),
        None => text.to_owned(),
    };
    cut.chars().filter(|c| !is_other(*c)).collect()
}

/// `[^\PC\s]` with `/u`: a character of the category "Other" that is no ASCII whitespace.
fn is_other(c: char) -> bool {
    if matches!(c, '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' ') {
        return false;
    }
    let n = c as u32;
    c.is_control()
        || matches!(n, 0xAD | 0x600..=0x605 | 0x61C | 0x6DD | 0x70F | 0x890..=0x891 | 0x8E2 | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD | 0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0001 | 0xE0020..=0xE007F)
        || matches!(n, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

/// The site headers, the later of one name in the place of the first (`get_site_headers()`).
pub fn site_headers(headers: &[Header]) -> Vec<Header> {
    remove_dupes(headers.iter().filter(|h| h.location == "site").cloned().collect())
}

/// `get_redirect_headers()`: the site headers, then the redirect headers over them.
pub fn redirect_headers(headers: &[Header]) -> Vec<Header> {
    let mut all = site_headers(headers);
    all.extend(headers.iter().filter(|h| h.location == "redirect").cloned());
    remove_dupes(all)
}

fn remove_dupes(headers: Vec<Header>) -> Vec<Header> {
    let mut out: Vec<Header> = Vec::new();
    for header in headers {
        match out.iter_mut().find(|h| h.name == header.name) {
            Some(slot) => *slot = header,
            None => out.push(header),
        }
    }
    out
}

/// `Red_Http_Headers::run()`: the headers as `header()` sends them.
pub fn run(headers: &[Header]) -> Vec<(String, String)> {
    let mut done: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for header in headers {
        if !done.contains(&header.name.as_str()) {
            out.push((sanitize(&dash_case(&header.name)), sanitize(&header.value)));
            done.push(&header.name);
        }
    }
    out
}
