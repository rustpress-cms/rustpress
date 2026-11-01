//! Redirection's URLs: `Red_Source_Flags`, `Red_Url`, `Red_Url_Path`, `Red_Url_Query`, `Red_Url_Match` and
//! `Red_Url_Request`.

use std::sync::LazyLock;

use regex::Regex;
use rustpress_phpser::Value;

use super::php::{self, Param, Params};
use super::regex::PcreRegex;

/// `Red_Source_Flags::QUERY_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryMode {
    Ignore,
    Exact,
    Pass,
    ExactOrder,
}

/// A value a flag is set from: the options (PHP) or `match_data` (JSON).
#[derive(Debug, Clone)]
pub enum FlagValue {
    Bool(bool),
    Str(String),
    Other,
}

impl FlagValue {
    pub fn from_php(value: &Value) -> Option<Self> {
        match value {
            Value::Null => None,
            Value::Bool(b) => Some(Self::Bool(*b)),
            Value::Str(b) => Some(Self::Str(String::from_utf8_lossy(b).into_owned())),
            _ => Some(Self::Other),
        }
    }

    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Null => None,
            serde_json::Value::Bool(b) => Some(Self::Bool(*b)),
            serde_json::Value::String(s) => Some(Self::Str(s.clone())),
            _ => Some(Self::Other),
        }
    }
}

/// The four flags by name, as `set_flags()` reads them (`isset()`: missing and `null` are `None`).
#[derive(Debug, Clone, Default)]
pub struct FlagInput {
    pub query: Option<FlagValue>,
    pub case: Option<FlagValue>,
    pub trailing: Option<FlagValue>,
    pub regex: Option<FlagValue>,
}

/// `Red_Source_Flags`.
#[derive(Debug, Clone)]
pub struct Flags {
    pub case: bool,
    pub trailing: bool,
    pub regex: bool,
    pub query: QueryMode,
}

impl Default for Flags {
    fn default() -> Self {
        Self { case: false, trailing: false, regex: false, query: QueryMode::Exact }
    }
}

impl Flags {
    /// `set_flags()`.
    pub fn set(&mut self, input: &FlagInput) {
        if let Some(FlagValue::Str(q)) = &input.query {
            match q.as_str() {
                "ignore" => self.query = QueryMode::Ignore,
                "exact" => self.query = QueryMode::Exact,
                "pass" => self.query = QueryMode::Pass,
                "exactorder" => self.query = QueryMode::ExactOrder,
                _ => {}
            }
        }
        if let Some(FlagValue::Bool(b)) = input.case {
            self.case = b;
        }
        if let Some(FlagValue::Bool(b)) = input.trailing {
            self.trailing = b;
        }
        if let Some(FlagValue::Bool(b)) = input.regex {
            self.regex = b;
            // A regular expression always matches the query exactly.
            if b {
                self.query = QueryMode::Exact;
            }
        }
    }

    /// `new Red_Source_Flags( [ FLAG_CASE => true ] )`.
    pub fn ignoring_case() -> Self {
        Self { case: true, ..Self::default() }
    }

    pub fn is_query_ignore(&self) -> bool {
        self.query == QueryMode::Ignore
    }

    pub fn is_query_pass(&self) -> bool {
        self.query == QueryMode::Pass
    }
}

/// `get_query_before()`: the path part, up to `?` or an escaped `\?` before it.
fn query_before(url: &str) -> &str {
    let qpos = url.find('?');
    let qrpos = url.find("\\?");
    if let (Some(qr), Some(q)) = (qrpos, qpos)
        && qr < q
    {
        return &url[..qr];
    }
    match qpos {
        Some(q) => &url[..q],
        None => url,
    }
}

/// `get_query_after()`.
pub fn query_after(url: &str) -> &str {
    let Some(q) = url.find('?') else { return "" };
    if let Some(qr) = url.find("\\?")
        && qr < q
    {
        return &url[qr + 2..];
    }
    &url[q + 1..]
}

/// `Red_Url_Path`.
pub struct UrlPath {
    path: String,
}

impl UrlPath {
    pub fn new(url: &str) -> Self {
        static SCHEME: LazyLock<Regex> = LazyLock::new(|| Regex::new("^https?://").unwrap());
        let mut path = url.to_owned();
        if SCHEME.is_match(url)
            && let Some((_, rest)) = url.split_once("://")
        {
            let parts: Vec<&str> = rest.split('/').collect();
            path = format!("/{}", parts[1..].join("/"));
        }
        Self { path: php::urldecode(query_before(&path)) }
    }

    pub fn get(&self) -> &str {
        &self.path
    }

    /// `get_without_trailing_slash()`.
    pub fn without_trailing_slash(&self) -> String {
        if self.path == "/" {
            return self.path.clone();
        }
        match self.path.strip_suffix('/') {
            Some(p) => p.to_owned(),
            // `@/$@` also matches before a final newline.
            None => match self.path.strip_suffix("/\n") {
                Some(p) => format!("{p}\n"),
                None => self.path.clone(),
            },
        }
    }

    /// `is_match()`.
    pub fn is_match(&self, url: &str, flags: &Flags) -> bool {
        let target = UrlPath::new(url);
        let (mut target_path, mut source_path) = (target.get().to_owned(), self.get().to_owned());
        if flags.trailing {
            source_path = self.without_trailing_slash();
            target_path = target.without_trailing_slash();
        }
        if flags.case {
            source_path = php::to_lower(&source_path);
            target_path = php::to_lower(&target_path);
        }
        target_path == source_path
    }
}

/// A parameter of the plugin's own `parse_str()`.
struct RawParam {
    name: String,
    /// The names `parse_str()` gave the same part, joined.
    parse_str: String,
}

/// `Red_Url_Query::parse_str()`.
fn raw_params(query: &str) -> Vec<RawParam> {
    if query.is_empty() {
        return Vec::new();
    }
    query
        .split('&')
        .map(|part| {
            let name = part.split('=').next().unwrap_or_default().replace('+', " ");
            let name = ["[", "]", "%5B", "%5D"].iter().fold(name, |n, s| n.replace(s, ""));
            let parse_str = php::parse_str(part).into_iter().map(|(k, _)| k).collect::<String>();
            RawParam { name, parse_str }
        })
        .collect()
}

/// `get_url_query()`: the parameters, and the query to match exactly when `parse_str()` found none.
fn url_query(url: &str) -> (Params, Option<String>) {
    let query = query_after(url);
    let internal = raw_params(query);
    let mut params = php::parse_str(query);
    for (key, value) in &mut params {
        if matches!(value, Param::Str(s) if s.is_empty()) && !url.contains(&format!("{key}=")) {
            *value = Param::Null;
        }
    }
    for raw in internal {
        if raw.parse_str != raw.name {
            let keys: Vec<String> = params.iter().map(|(k, _)| k.clone()).collect();
            for key in keys {
                if key == raw.parse_str
                    && let Some(value) = php::get(&params, &key).cloned()
                {
                    php::unset(&mut params, &key);
                    php::set(&mut params, &raw.name, value);
                }
            }
        }
    }
    let exact = (params.is_empty() && url.contains('?')).then(|| query.to_owned());
    (params, exact)
}

fn is_string_match(first: &str, second: &str, case: bool) -> bool {
    if case { php::to_lower(first) == php::to_lower(second) } else { first == second }
}

/// What `get_query_same()` keeps of a parameter (`false` drops it).
fn kept(value: &Param) -> bool {
    !matches!(value, Param::Arr(a) if a.is_empty())
}

/// `get_query_same()`.
fn query_same(source: &Params, target: &Params, case: bool, depth: usize) -> Params {
    if depth > 10 {
        return Params::new();
    }
    let keys = |q: &Params| {
        let mut map: Vec<(String, String)> = Vec::new();
        for (k, _) in q {
            let lower = php::to_lower(k);
            match map.iter_mut().find(|(l, _)| *l == lower) {
                Some(slot) => slot.1 = k.clone(),
                None => map.push((lower, k.clone())),
            }
        }
        map
    };
    let (source_keys, target_keys) = (keys(source), keys(target));
    let mut same = Params::new();
    for (lower, original) in &source_keys {
        let Some((_, target_key)) = target_keys.iter().find(|(l, _)| l == lower) else { continue };
        let (Some(source_value), Some(target_value)) = (php::get(source, original), php::get(target, target_key)) else { continue };
        let add = match (source_value, target_value) {
            (Param::Arr(s), Param::Arr(t)) => {
                let child = query_same(s, t, case, depth + 1);
                (child.len() == s.len()).then_some(Param::Arr(child))
            }
            (Param::Str(s), Param::Str(t)) => is_string_match(s, t, case).then(|| Param::Str(s.clone())),
            (Param::Null, Param::Null) => Some(Param::Null),
            _ => None,
        };
        if let Some(add) = add.filter(kept) {
            php::set(&mut same, original, add);
        }
    }
    same
}

/// `get_query_diff()`.
fn query_diff(source: &Params, target: &Params, depth: usize) -> Params {
    if depth > 10 {
        return Params::new();
    }
    let mut diff = Params::new();
    for (key, value) in source {
        let other = php::get(target, key);
        if let (Param::Arr(s), Some(Param::Arr(t))) = (value, other) {
            let add = query_diff(s, t, depth + 1);
            if !add.is_empty() {
                php::set(&mut diff, key, Param::Arr(add));
            }
        } else if match other {
            None => true,
            Some(other) => matches!(value, Param::Arr(_)) || matches!(other, Param::Arr(_)) || other != value,
        } {
            php::set(&mut diff, key, value.clone());
        }
    }
    diff
}

/// `Red_Url_Query`.
pub struct UrlQuery {
    original: Params,
    matching: Params,
    exact: Option<String>,
}

impl UrlQuery {
    pub fn new(url: &str, flags: &Flags) -> Self {
        let (original, mut exact) = url_query(url);
        let mut matching = original.clone();
        if flags.case {
            let (lower, lower_exact) = url_query(&php::to_lower(url));
            matching = lower;
            exact = lower_exact.or(exact);
        }
        Self { original, matching, exact }
    }

    /// `is_match()`.
    pub fn is_match(&self, url: &str, flags: &Flags) -> bool {
        let url = if flags.case { php::to_lower(url) } else { url.to_owned() };
        if let Some(exact) = &self.exact {
            return is_string_match(query_after(&url), exact, flags.case);
        }
        let (target, _) = url_query(&url);
        let matched = query_same(&self.matching, &target, flags.case, 0);
        if matched.len() != self.matching.len() {
            return false;
        }
        if flags.is_query_ignore() || flags.is_query_pass() {
            return true;
        }
        query_diff(&self.matching, &target, 0).is_empty() && query_diff(&target, &self.matching, 0).is_empty()
    }

    /// `add_to_target()`: with the query passed on, the request's other parameters follow the target.
    pub fn add_to_target(target_url: &str, requested_url: &str, flags: &Flags) -> String {
        if !flags.is_query_pass() || target_url.is_empty() {
            return target_url.to_owned();
        }
        let source = UrlQuery::new(target_url, flags);
        let request = UrlQuery::new(requested_url, flags);
        let mut diff = query_diff(&source.original, &request.original, 0);
        for (key, value) in query_diff(&request.original, &source.original, 0) {
            php::set(&mut diff, &key, value);
        }
        diff.retain(|(key, _)| php::get(&source.original, key).is_none());
        build_url(target_url, &diff)
    }

    /// `get_url_with_query()`.
    pub fn url_with_query(&self, url: &str) -> String {
        build_url(url, &self.original)
    }
}

/// `Red_Url_Query::build_url()`.
pub fn build_url(url: &str, params: &Params) -> String {
    static INDEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%5B\d*%5D").unwrap());
    let mapped: Params = params.iter().map(|(k, v)| (k.clone(), if matches!(v, Param::Null) { Param::Str(String::new()) } else { v.clone() })).collect();
    let mut query = INDEX.replace_all(&php::http_build_query(&mapped), "[]").into_owned();
    for (key, value) in params {
        if matches!(value, Param::Null) {
            let search = format!("{}=", php::rawurlencode(key)).replace("%20", "+");
            let replace = php::rawurlencode(key).replace("%20", "+");
            query = query.replace(&search, &replace);
        }
    }
    let query = query.replace("%252B", "+");
    if query.is_empty() {
        return url.to_owned();
    }
    let mut url = url.to_owned();
    let mut query = query;
    // A fragment goes after the query.
    if let Some((_, fragment)) = url.split_once('#') {
        let fragment = fragment.to_owned();
        url = url.replace(&format!("#{fragment}"), "");
        query.push('#');
        query.push_str(&fragment);
    }
    let glue = if url.contains('?') { '&' } else { '?' };
    format!("{url}{glue}{query}")
}

/// `Red_Url::is_match()`: the source of a redirect against the requested (decoded) URL.
pub fn is_match(source: &str, requested: &str, flags: &Flags) -> bool {
    let source = source.replace(' ', "%20");
    if flags.regex {
        return PcreRegex::new(&source, flags.case).is_match(requested);
    }
    UrlPath::new(&source).is_match(requested, flags) && UrlQuery::new(&source, flags).is_match(requested, flags)
}

/// `Red_Url_Match::get_url()`: the path the table is searched by.
pub fn match_url(url: &str) -> String {
    let path = UrlPath::new(url).without_trailing_slash();
    let mut path = php::rawurlencode(&path);
    for c in ["/", ":", "[", "]", "@", "~", ",", "(", ")", ";"] {
        path = path.replace(&php::rawurlencode(c), c);
    }
    let path = php::to_lower(&path);
    if path.is_empty() { "/".to_owned() } else { path }
}

/// `Red_Url_Match::get_url_with_params()`.
pub fn match_url_with_params(url: &str) -> String {
    UrlQuery::new(url, &Flags::ignoring_case()).url_with_query(&match_url(url))
}

/// `Red_Url_Request`: the requested URL decoded, and with its own query again.
pub struct RequestUrl {
    pub original: String,
    pub decoded: String,
}

impl RequestUrl {
    pub fn new(url: &str) -> Self {
        let decoded = php::rawurldecode(url);
        let original = match (decoded.split_once('?'), url.split('?').nth(1)) {
            (Some((path, _)), Some(query)) => format!("{path}?{query}"),
            _ => decoded.clone(),
        };
        Self { original, decoded }
    }

    pub fn is_valid(&self) -> bool {
        !self.decoded.is_empty()
    }

    /// `is_protected_url()`: the REST API is never redirected.
    pub fn is_protected(&self, rest_api: &str) -> bool {
        !rest_api.is_empty() && self.decoded.starts_with(rest_api)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(case: bool, trailing: bool, query: QueryMode) -> Flags {
        Flags { case, trailing, regex: false, query }
    }

    #[test]
    fn match_urls() {
        assert_eq!(match_url("/old-page/"), "/old-page");
        assert_eq!(match_url("/über/"), "/%c3%bcber");
        assert_eq!(match_url("/a b/"), "/a%20b");
        assert_eq!(match_url("/CaseTest/"), "/casetest");
        assert_eq!(match_url("/q-exact/?x=1&y[]=2"), "/q-exact");
        assert_eq!(match_url_with_params("/q-order/?b=2&a=1"), "/q-order?b=2&a=1");
        assert_eq!(match_url("/"), "/");
    }

    #[test]
    fn queries() {
        let exact = flags(true, true, QueryMode::Exact);
        assert!(is_match("/q-exact/?x=1&y[]=2", "/q-exact/?y[]=2&x=1", &exact));
        assert!(!is_match("/q-exact/?x=1&y[]=2", "/q-exact/?x=1", &exact));
        assert!(is_match("/q-ignore/?k=v", "/q-ignore/?k=v&z=1", &flags(false, false, QueryMode::Ignore)));
        assert!(is_match("/old-page/", "/OLD-PAGE", &exact));
        assert!(!is_match("/CaseTest/", "/casetest/", &flags(false, false, QueryMode::Exact)));
        let pass = flags(false, false, QueryMode::Pass);
        assert_eq!(UrlQuery::add_to_target("/hello-world/?a=1", "/q-pass/?b=2&a=9", &pass), "/hello-world/?a=1&b=2");
        assert_eq!(UrlQuery::add_to_target("/t/#f", "/s/?flag&b=c d", &pass), "/t/?flag&b=c+d#f");
    }

    #[test]
    fn requests() {
        let r = RequestUrl::new("/%C3%BCber/?q=%41&x=1");
        assert_eq!(r.decoded, "/über/?q=A&x=1");
        assert_eq!(r.original, "/über/?q=%41&x=1");
        assert!(RequestUrl::new("/wp-json/wp/v2/posts").is_protected("/wp-json/"));
    }
}
