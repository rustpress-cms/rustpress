//! The option `wpforms_settings` and `wpforms_setting()`.

use serde_json::{Map, Value};

use super::php;

/// `get_option( 'wpforms_settings' )` as an array.
#[derive(Debug, Clone, Default)]
pub struct Settings(pub Map<String, Value>);

/// A PHP value as the plugin reads it (lists become objects keyed by their index).
pub fn json(value: &rustpress_phpser::Value) -> Value {
    use rustpress_phpser::{Key, Value as P};
    match value {
        P::Null => Value::Null,
        P::Bool(b) => Value::Bool(*b),
        P::Int(i) => Value::from(*i),
        P::Float(f) => serde_json::Number::from_f64(*f).map_or(Value::Null, Value::Number),
        P::Str(s) => Value::String(String::from_utf8_lossy(s).into_owned()),
        P::Array(items) => Value::Object(
            items
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        Key::Int(i) => i.to_string(),
                        Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
                    };
                    (key, json(v))
                })
                .collect(),
        ),
        _ => Value::Null,
    }
}

impl Settings {
    /// From the serialized option.
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(raw) = raw else { return Self::default() };
        match rustpress_phpser::unserialize(raw.as_bytes()).map(|v| json(&v)) {
            Ok(Value::Object(m)) => Self(m),
            _ => Self::default(),
        }
    }

    /// From pairs (tests).
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self(pairs.iter().map(|(k, v)| ((*k).to_owned(), Value::String((*v).to_owned()))).collect())
    }

    /// `! empty( wpforms_setting( $key ) )`.
    pub fn set(&self, key: &str) -> bool {
        !php::empty(self.0.get(key))
    }

    /// `wpforms_setting( $key, $default )` as a string.
    pub fn string(&self, key: &str, default: &str) -> String {
        match self.0.get(key) {
            Some(v) if !php::empty(Some(v)) => super::fields::stripslashes(&php::string(v)),
            _ => default.to_owned(),
        }
    }

    /// `wpforms_setting( $key )`, `false` when it is empty.
    pub fn value(&self, key: &str) -> Value {
        match self.0.get(key) {
            Some(v) if !php::empty(Some(v)) => v.clone(),
            _ => Value::Bool(false),
        }
    }
}
