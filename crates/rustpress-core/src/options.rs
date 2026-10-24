use std::collections::HashMap;

use rustpress_db::Db;
use rustpress_phpser::{Value, unserialize};
use serde_json::Value as Value2;

use crate::Result;

/// `maybe_unserialize()`: values that parse as PHP-serialized data become
/// structured values, everything else stays a plain string.
pub fn maybe_unserialize(raw: &str) -> Value {
    // Like `is_serialized()`: `N;` is the only value shorter than 4 bytes.
    let looks_serialized = raw == "N;"
        || (raw.len() > 3
            && matches!(raw.as_bytes()[0], b'a' | b'O' | b's' | b'i' | b'd' | b'b')
            && matches!(raw.as_bytes()[raw.len() - 1], b';' | b'}'));
    if looks_serialized && let Ok(value) = unserialize(raw.as_bytes()) {
        return value;
    }
    Value::Str(raw.as_bytes().to_vec())
}

/// Autoloaded options in memory, everything else fetched on demand.
#[derive(Debug, Clone, Default)]
pub struct Options {
    autoloaded: HashMap<String, String>,
}

impl Options {
    pub async fn load(db: &Db) -> Result<Self> {
        Ok(Self { autoloaded: db.autoload_options().await?.into_iter().collect() })
    }

    /// Builds an option set from known values (tests, defaults).
    pub fn from_pairs<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        Self { autoloaded: pairs.into_iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect() }
    }

    /// Raw value of an autoloaded option.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.autoloaded.get(name).map(String::as_str)
    }

    pub fn string(&self, name: &str, default: &str) -> String {
        self.get(name).unwrap_or(default).to_owned()
    }

    pub fn u64(&self, name: &str, default: u64) -> u64 {
        self.get(name).and_then(|v| v.trim().parse().ok()).unwrap_or(default)
    }

    /// Unserialized value of an autoloaded option.
    pub fn value(&self, name: &str) -> Option<Value> {
        self.get(name).map(maybe_unserialize)
    }

    /// Like [`Options::get`], but falls back to the database for options that
    /// are not autoloaded.
    pub async fn fetch(&self, db: &Db, name: &str) -> Result<Option<String>> {
        match self.get(name) {
            Some(value) => Ok(Some(value.to_owned())),
            None => Ok(db.option(name).await?),
        }
    }
}

/// A PHP value as JSON: lists (keys `0..n`) become arrays, other arrays objects
/// with their keys as strings; key order is kept.
pub fn php_to_json(value: &rustpress_phpser::Value) -> Value2 {
    use rustpress_phpser::{Key, Value as Php};
    match value {
        Php::Null | Php::Reference(_) | Php::ObjectRef(_) => Value2::Null,
        Php::Bool(b) => Value2::Bool(*b),
        Php::Int(i) => Value2::from(*i),
        Php::Float(f) => serde_json::Number::from_f64(*f).map_or(Value2::Null, Value2::Number),
        Php::Str(s) => Value2::String(String::from_utf8_lossy(s).into_owned()),
        Php::Array(items) | Php::Object { props: items, .. } => {
            let is_list = items.iter().enumerate().all(|(i, (k, _))| matches!(k, Key::Int(n) if *n == i as i64));
            if is_list {
                Value2::Array(items.iter().map(|(_, v)| php_to_json(v)).collect())
            } else {
                Value2::Object(
                    items
                        .iter()
                        .map(|(k, v)| {
                            let key = match k {
                                Key::Int(n) => n.to_string(),
                                Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
                            };
                            (key, php_to_json(v))
                        })
                        .collect(),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_strings_stay_strings() {
        assert_eq!(maybe_unserialize("hello"), Value::Str(b"hello".to_vec()));
        assert_eq!(maybe_unserialize("/%postname%/"), Value::Str(b"/%postname%/".to_vec()));
        // Looks like the start of a serialized value but is not valid.
        assert_eq!(maybe_unserialize("a:oops;"), Value::Str(b"a:oops;".to_vec()));
        assert_eq!(maybe_unserialize(""), Value::Str(Vec::new()));
    }

    #[test]
    fn serialized_values_are_parsed() {
        let value = maybe_unserialize("a:1:{s:1:\"x\";i:2;}");
        assert_eq!(value.get("x").and_then(Value::as_i64), Some(2));
        assert_eq!(maybe_unserialize("b:0;"), Value::Bool(false));
        assert_eq!(maybe_unserialize("N;"), Value::Null);
    }

    #[test]
    fn typed_getters_fall_back() {
        let options = Options::from_pairs([("posts_per_page", "5"), ("bad", "x")]);
        assert_eq!(options.u64("posts_per_page", 10), 5);
        assert_eq!(options.u64("bad", 10), 10);
        assert_eq!(options.u64("missing", 10), 10);
        assert_eq!(options.string("missing", "d"), "d");
    }
}