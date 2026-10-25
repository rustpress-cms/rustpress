//! Helpers shared by the endpoints: links, pagination, dates.

use rustpress_core::canonical::{build_qs, parse_qs};
use serde_json::{Map, Value, json};

use crate::Rest;
use crate::params::Params;

/// `mysql_to_rfc3339()`: `2026-10-24 14:11:09` becomes `2026-10-24T14:11:09`.
pub fn rfc3339(mysql: &str) -> String {
    mysql.replacen(' ', "T", 1)
}

pub const ZERO_DATE: &str = "0000-00-00 00:00:00";

/// The `_links` object, built in WordPress' order with the `wp:` relations compacted.
#[derive(Default)]
pub struct Links {
    map: Map<String, Value>,
}

impl Links {
    pub fn new() -> Self {
        Self::default()
    }

    /// A link; its attributes come before `href`, as WordPress prints them.
    pub fn add(&mut self, rel: &str, attributes: Vec<(&str, Value)>, href: String) -> &mut Self {
        let mut link = Map::new();
        for (key, value) in attributes {
            link.insert(key.to_owned(), value);
        }
        link.insert("href".into(), Value::from(href));
        self.push(rel, Value::Object(link));
        self
    }

    pub fn embeddable(&mut self, rel: &str, href: String) -> &mut Self {
        self.add(rel, vec![("embeddable", Value::Bool(true))], href)
    }

    /// The `self` link carries the `targetHints` of the methods an anonymous visitor may use.
    pub fn self_link(&mut self, href: String) -> &mut Self {
        let mut link = Map::new();
        link.insert("href".into(), Value::from(href));
        link.insert("targetHints".into(), json!({"allow": ["GET"]}));
        self.push("self", Value::Object(link));
        self
    }

    fn push(&mut self, rel: &str, link: Value) {
        match self.map.get_mut(rel) {
            Some(Value::Array(items)) => items.push(link),
            _ => {
                self.map.insert(rel.to_owned(), Value::Array(vec![link]));
            }
        }
    }

    pub fn finish(mut self) -> Value {
        if self.map.keys().any(|k| k.starts_with("wp:")) {
            self.map.insert("curies".into(), json!([{"name": "wp", "href": "https://api.w.org/{rel}", "templated": true}]));
        }
        Value::Object(self.map)
    }
}

/// How WordPress rewrites the request parameters before it builds pagination
/// links: array parameters become `name[0]=a&name[1]=b`, booleans `1` or nothing.
pub struct LinkParams<'a> {
    pub arrays: &'a [&'a str],
    pub booleans: &'a [&'a str],
}

/// `Link: <...>; rel="prev", <...>; rel="next"` for a paged collection.
pub fn link_header(rest: &Rest<'_>, route: &str, params: &Params, shape: &LinkParams<'_>, page: u64, max_pages: u64) -> Option<String> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for (key, value) in parse_qs(params.raw()) {
        let name = key.trim_end_matches("[]").to_owned();
        if shape.arrays.contains(&name.as_str()) {
            continue;
        }
        if shape.booleans.contains(&name.as_str()) {
            let truthy = !matches!(value.as_str(), "" | "0" | "false");
            pairs.push((name, if truthy { "1".into() } else { String::new() }));
        } else {
            pairs.push((key, value));
        }
    }
    // Array parameters keep their position of first appearance.
    let mut ordered: Vec<(String, String)> = Vec::new();
    let mut done: Vec<String> = Vec::new();
    for (key, _) in parse_qs(params.raw()) {
        let name = key.trim_end_matches("[]").to_owned();
        if shape.arrays.contains(&name.as_str()) {
            if !done.contains(&name) {
                done.push(name.clone());
                for (i, item) in params.list(&name).unwrap_or_default().into_iter().enumerate() {
                    ordered.push((format!("{name}[{i}]"), item));
                }
            }
        } else if let Some(pair) = pairs.iter().find(|(k, _)| *k == key || *k == name) {
            ordered.push(pair.clone());
        }
    }
    let url_for = |n: u64| {
        let mut all = ordered.clone();
        match all.iter_mut().find(|(k, _)| k == "page") {
            Some(existing) => existing.1 = n.to_string(),
            None => all.push(("page".into(), n.to_string())),
        }
        let query = build_qs(&all);
        let collection = rest.rest_url(route);
        // `add_query_arg()` re-encodes the route of a plain-permalink URL.
        let collection = match collection.split_once("?rest_route=") {
            Some((base, route)) => format!("{base}?rest_route={}", rustpress_core::canonical::php_urlencode(route)),
            None => collection,
        };
        if collection.contains('?') { format!("{collection}&{query}") } else { format!("{collection}?{query}") }
    };
    let mut parts = Vec::new();
    if page > 1 {
        parts.push(format!("<{}>; rel=\"prev\"", url_for((page - 1).min(max_pages))));
    }
    if max_pages > page {
        parts.push(format!("<{}>; rel=\"next\"", url_for(page + 1)));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}
impl Rest<'_> {
    /// `get_gmt_from_date()`.
    pub fn gmt_from_local(&self, local: &str) -> String {
        self.site
            .timezone
            .local_timestamp(local)
            .and_then(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0))
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| local.to_owned())
    }

    /// `prepare_date_response( $gmt )` for the `*_gmt` fields.
    pub fn date_gmt(&self, gmt: &str, local: &str) -> Value {
        if gmt == ZERO_DATE { Value::from(rfc3339(&self.gmt_from_local(local))) } else { Value::from(rfc3339(gmt)) }
    }

    /// The Gravatar URLs of `avatar_urls` / `author_avatar_urls`.
    pub fn avatar_urls(&self, email: &str) -> Value {
        let mut map = Map::new();
        for size in [24, 48, 96] {
            map.insert(size.to_string(), Value::from(rustpress_blocks::content::avatar_url(&self.site.options, email, size)));
        }
        Value::Object(map)
    }
}

/// The `Env` used to render content outside of a page request.
pub fn env<'a>(rest: &'a Rest<'a>) -> RestEnv<'a> {
    let env = rustpress_blocks::Env::new(rest.site, &rest.blank, rest.theme, rest.registry);
    env.with_state(|s| *s = std::mem::take(&mut *rest.state.lock().expect("rest state lock")));
    RestEnv { env, rest }
}

/// An [`rustpress_blocks::Env`] whose state goes back to the request when it is dropped.
pub struct RestEnv<'a> {
    env: rustpress_blocks::Env<'a>,
    rest: &'a Rest<'a>,
}

impl<'a> std::ops::Deref for RestEnv<'a> {
    type Target = rustpress_blocks::Env<'a>;

    fn deref(&self) -> &Self::Target {
        &self.env
    }
}

impl Drop for RestEnv<'_> {
    fn drop(&mut self) {
        *self.rest.state.lock().expect("rest state lock") = self.env.take_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_put_attributes_before_href_and_add_curies() {
        let mut links = Links::new();
        links.self_link("http://x/a".into()).embeddable("author", "http://x/u".into()).add("wp:term", vec![("taxonomy", json!("category"))], "http://x/t".into());
        let value = links.finish();
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            r#"{"self":[{"href":"http://x/a","targetHints":{"allow":["GET"]}}],"author":[{"embeddable":true,"href":"http://x/u"}],"wp:term":[{"taxonomy":"category","href":"http://x/t"}],"curies":[{"name":"wp","href":"https://api.w.org/{rel}","templated":true}]}"#
        );
    }

    #[test]
    fn dates_lose_the_space() {
        assert_eq!(rfc3339("2026-10-24 14:11:09"), "2026-10-24T14:11:09");
    }
}
