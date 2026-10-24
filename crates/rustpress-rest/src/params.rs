//! Request parameters, parsed the way PHP's `parse_str` does.

use crate::error::RestError;

#[derive(Debug, Clone, Default)]
pub struct Params {
    /// Decoded pairs in request order. `a[]=1&a[]=2` becomes two entries named `a`.
    pairs: Vec<(String, String)>,
    raw: String,
}

impl Params {
    pub fn parse(query: &str) -> Self {
        let mut pairs: Vec<(String, String)> = Vec::new();
        for part in query.split('&').filter(|p| !p.is_empty()) {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            let decode = |s: &str| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.replace('+', " ").as_bytes())).into_owned();
            let (key, value) = (decode(key), decode(value));
            if let Some(name) = key.split('[').next().filter(|_| key.contains('[')) {
                pairs.push((format!("{name}[]"), value));
            } else if let Some(existing) = pairs.iter_mut().find(|(k, _)| *k == key) {
                existing.1 = value;
            } else {
                pairs.push((key, value));
            }
        }
        Self { pairs, raw: query.to_owned() }
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn set(&mut self, key: &str, value: &str) {
        match self.pairs.iter_mut().find(|(k, _)| k == key) {
            Some(existing) => existing.1 = value.to_owned(),
            None => self.pairs.push((key.to_owned(), value.to_owned())),
        }
    }

    pub fn has(&self, key: &str) -> bool {
        self.pairs.iter().any(|(k, _)| k == key || k.strip_suffix("[]") == Some(key))
    }

    /// A scalar parameter; for arrays the first element.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs.iter().find(|(k, _)| k == key || k.strip_suffix("[]") == Some(key)).map(|(_, v)| v.as_str())
    }

    /// `rest_sanitize_array`: a comma separated string or an array parameter.
    pub fn list(&self, key: &str) -> Option<Vec<String>> {
        if !self.has(key) {
            return None;
        }
        let mut out = Vec::new();
        for (k, v) in &self.pairs {
            if k == key || k.strip_suffix("[]") == Some(key) {
                out.extend(v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned));
            }
        }
        Some(out)
    }

    /// `wp_parse_id_list`: positive integers only.
    pub fn id_list(&self, key: &str) -> Option<Vec<u64>> {
        self.list(key).map(|items| items.iter().filter_map(|s| s.parse::<u64>().ok()).filter(|id| *id > 0).collect())
    }

    /// `rest_sanitize_boolean`-ish: present and not `0`, `false` or empty.
    pub fn truthy(&self, key: &str) -> bool {
        self.get(key).is_some_and(|v| !matches!(v, "" | "0" | "false"))
            || (self.has(key) && self.get(key) == Some(""))
    }

    /// An integer parameter with the bounds of the endpoint schema.
    pub fn integer(&self, key: &str, default: u64, min: u64, max: Option<u64>) -> Result<u64, RestError> {
        let Some(value) = self.get(key) else { return Ok(default) };
        let number: i64 = value.parse().map_err(|_| RestError::invalid_param(key, &format!("{key} is not of type integer."), "rest_invalid_type"))?;
        if number < min as i64 {
            let message = match max {
                Some(max) => format!("{key} must be between {min} (inclusive) and {max} (inclusive)"),
                None => format!("{key} must be greater than or equal to {min}"),
            };
            return Err(RestError::invalid_param(key, &message, if max.is_some() { "rest_out_of_bounds" } else { "rest_out_of_bounds" }));
        }
        if let Some(max) = max.filter(|max| number > *max as i64) {
            return Err(RestError::invalid_param(key, &format!("{key} must be between {min} (inclusive) and {max} (inclusive)"), "rest_out_of_bounds"));
        }
        Ok(number as u64)
    }

    /// A value restricted to an enum.
    pub fn one_of<'a>(&'a self, key: &str, allowed: &[&str], default: &'a str) -> Result<&'a str, RestError> {
        match self.get(key) {
            None => Ok(default),
            Some(v) if allowed.contains(&v) => Ok(v),
            Some(_) => Err(RestError::invalid_param(key, &format!("{key} is not one of {}.", list_and(allowed)), "rest_not_in_enum")),
        }
    }
}


/// `wp_sprintf_l`: `a and b`, `a, b, and c`.
pub fn list_and(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}