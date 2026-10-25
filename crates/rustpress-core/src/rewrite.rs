//! Permalink resolution from the `rewrite_rules` option, following
//! `WP::parse_request`: rules are tried in stored order, the first match wins,
//! and `$matches[n]` in the rule's query string is substituted.
//!
//! Patterns are matched as PCRE-style byte patterns (`unicode(false)`), against
//! the raw request first and then its URL-decoded form, like WordPress does.

use std::collections::BTreeMap;

use regex::bytes::{Captures, Regex, RegexBuilder};
use rustpress_phpser::{Key, Value};

/// Decodes `%XX` and `+` like PHP's `urldecode()`.
pub fn urldecode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        match input[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < input.len() && hex(input[i + 1]).is_some() && hex(input[i + 2]).is_some() => {
                out.push(hex(input[i + 1]).unwrap() << 4 | hex(input[i + 2]).unwrap());
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    out
}

fn hex(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

/// WordPress query variables (`p`, `name`, `paged`, ...). Empty values count as
/// unset, the way `WP_Query` treats them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryVars(BTreeMap<String, String>);

impl QueryVars {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses `a=1&b=2` like PHP's `parse_str` (without array syntax).
    pub fn parse(query: &str) -> Self {
        let mut vars = Self::new();
        for pair in query.split('&').filter(|p| !p.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = String::from_utf8_lossy(&urldecode(key.as_bytes())).into_owned();
            let value = String::from_utf8_lossy(&urldecode(value.as_bytes())).into_owned();
            if !key.is_empty() && !key.contains('[') {
                vars.0.insert(key, value);
            }
        }
        vars
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.0.insert(key.to_owned(), value.into());
    }

    pub fn remove(&mut self, key: &str) {
        self.0.remove(key);
    }

    /// The value, or `None` if unset or empty.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str).filter(|v| !v.is_empty())
    }

    /// True if the key is present, even with an empty value (`?s=`).
    pub fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    pub fn u64(&self, key: &str) -> Option<u64> {
        self.get(key)?.trim().parse().ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub pattern: String,
    /// `index.php?name=$matches[1]&page=$matches[2]`
    pub query: String,
    regex: Regex,
}

/// A rule that matched a request.
#[derive(Debug)]
pub struct Match<'a> {
    pub rule: &'a Rule,
    pub vars: QueryVars,
    /// Capture groups, `captures[0]` being the whole match.
    pub captures: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
pub struct RewriteRules {
    rules: Vec<Rule>,
    /// Patterns the `regex` crate cannot compile (lookaround, backreferences).
    pub skipped: Vec<String>,
}

impl RewriteRules {
    /// Builds the rules from the unserialized `rewrite_rules` option.
    pub fn from_option(value: &Value) -> Self {
        let mut rules = Vec::new();
        let mut skipped = Vec::new();
        for (key, query) in value.as_array().unwrap_or_default() {
            let (Key::Str(pattern), Some(query)) = (key, query.as_str()) else { continue };
            let pattern = String::from_utf8_lossy(pattern).into_owned();
            match RegexBuilder::new(&format!("^{pattern}")).unicode(false).build() {
                Ok(regex) => rules.push(Rule { pattern, query: query.to_owned(), regex }),
                Err(_) => skipped.push(pattern),
            }
        }
        Self { rules, skipped }
    }

    /// `(pattern, query)` of every rule, in stored order.
    pub fn patterns(&self) -> impl Iterator<Item = (&str, &str)> {
        self.rules.iter().map(|r| (r.pattern.as_str(), r.query.as_str()))
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// All rules matching `request` (a path without leading/trailing slashes),
    /// in order. The caller takes the first one it accepts. An empty request
    /// can only match the special `$` rule.
    pub fn matches<'a>(&'a self, request: &'a str) -> impl Iterator<Item = Match<'a>> + 'a {
        let raw = request.as_bytes().to_vec();
        let decoded = urldecode(&raw);
        self.rules.iter().filter_map(move |rule| {
            if raw.is_empty() && rule.pattern != "$" {
                return None;
            }
            let caps = rule.regex.captures(&raw).or_else(|| rule.regex.captures(&decoded))?;
            Some(Match { rule, vars: substitute(&rule.query, &caps), captures: collect(&caps) })
        })
    }
}

fn collect(caps: &Captures<'_>) -> Vec<Vec<u8>> {
    (0..caps.len()).map(|i| caps.get(i).map(|m| m.as_bytes().to_vec()).unwrap_or_default()).collect()
}

/// `index.php?a=$matches[1]&b=$matches[2]` -> `{a: ..., b: ...}`.
fn substitute(query: &str, caps: &Captures<'_>) -> QueryVars {
    // `preg_replace('!^.+\?!', '', $query)`: drop everything up to the last `?`.
    let query = query.rsplit_once('?').map_or(query, |(_, rest)| rest);
    let mut out = Vec::with_capacity(query.len());
    let bytes = query.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(rest) = bytes[i..].strip_prefix(b"$matches[")
            && let Some(end) = rest.iter().position(|&b| b == b']')
            && let Some(n) = std::str::from_utf8(&rest[..end]).ok().and_then(|n| n.parse::<usize>().ok())
        {
            if let Some(m) = caps.get(n) {
                out.extend_from_slice(m.as_bytes());
            }
            i += "$matches[".len() + end + 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    QueryVars::parse(&String::from_utf8_lossy(&out))
}

/// The part of the URL path WordPress matches against its rules: the home
/// path prefix and surrounding slashes are removed, and a bare `index.php`
/// is the front page.
pub fn request_path(url_path: &str, home_path: &str) -> String {
    let path = url_path.split('?').next().unwrap_or("").trim_matches('/');
    let home = home_path.trim_matches('/');
    let path = if !home.is_empty() && path.len() >= home.len() && path[..home.len()].eq_ignore_ascii_case(home) {
        path[home.len()..].trim_matches('/')
    } else {
        path
    };
    if path == "index.php" { String::new() } else { path.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpress_phpser::unserialize;

    fn rules(pairs: &[(&str, &str)]) -> RewriteRules {
        let mut raw = format!("a:{}:{{", pairs.len());
        for (k, v) in pairs {
            raw.push_str(&format!("s:{}:\"{k}\";s:{}:\"{v}\";", k.len(), v.len()));
        }
        raw.push('}');
        RewriteRules::from_option(&unserialize(raw.as_bytes()).unwrap())
    }

    #[test]
    fn urldecode_matches_php() {
        assert_eq!(urldecode(b"a%20b+c%C3%A4"), "a b cä".as_bytes());
        assert_eq!(urldecode(b"100%"), b"100%");
        assert_eq!(urldecode(b"%zz%4"), b"%zz%4");
    }

    #[test]
    fn query_vars_parse_like_parse_str() {
        let vars = QueryVars::parse("s=a%20b&paged=2&empty=&x[]=1&flag");
        assert_eq!(vars.get("s"), Some("a b"));
        assert_eq!(vars.u64("paged"), Some(2));
        assert_eq!(vars.get("empty"), None);
        assert!(vars.has("empty"));
        assert!(!vars.has("x[]"));
        assert!(vars.has("flag"));
    }

    #[test]
    fn first_match_wins_and_groups_are_substituted() {
        let r = rules(&[
            ("category/(.+?)/page/?([0-9]{1,})/?$", "index.php?category_name=$matches[1]&paged=$matches[2]"),
            ("category/(.+?)/?$", "index.php?category_name=$matches[1]"),
            ("([^/]+)(?:/([0-9]+))?/?$", "index.php?name=$matches[1]&page=$matches[2]"),
        ]);
        let m = r.matches("category/tech/rust/page/2").next().unwrap();
        assert_eq!(m.vars.get("category_name"), Some("tech/rust"));
        assert_eq!(m.vars.u64("paged"), Some(2));

        let m = r.matches("category/tech").next().unwrap();
        assert_eq!(m.vars.get("category_name"), Some("tech"));
        assert!(!m.vars.has("paged"));

        // An unmatched optional group substitutes to an empty value.
        let m = r.matches("hello-world").next().unwrap();
        assert_eq!(m.vars.get("name"), Some("hello-world"));
        assert!(m.vars.has("page") && m.vars.get("page").is_none());
    }

    #[test]
    fn percent_encoded_requests_match_and_decode() {
        let r = rules(&[("([^/]+)/?$", "index.php?name=$matches[1]")]);
        let m = r.matches("caf%C3%A9").next().unwrap();
        assert_eq!(m.vars.get("name"), Some("café"));
    }

    #[test]
    fn empty_request_matches_nothing_but_dollar() {
        let r = rules(&[("([^/]+)/?$", "index.php?name=$matches[1]"), ("$", "index.php?x=1")]);
        let m = r.matches("").next().unwrap();
        assert_eq!(m.rule.pattern, "$");
    }

    #[test]
    fn unsupported_patterns_are_reported_not_fatal() {
        let r = rules(&[("(?<!a)b", "index.php?x=1"), ("ok", "index.php?y=1")]);
        assert_eq!(r.len(), 1);
        assert_eq!(r.skipped, ["(?<!a)b"]);
    }

    #[test]
    fn request_path_strips_home_and_index() {
        assert_eq!(request_path("/blog/hello/?x=1", "/blog"), "hello");
        assert_eq!(request_path("/BLOG/hello/", "/blog/"), "hello");
        assert_eq!(request_path("/hello/", ""), "hello");
        assert_eq!(request_path("/index.php", ""), "");
        assert_eq!(request_path("/", ""), "");
    }
}
