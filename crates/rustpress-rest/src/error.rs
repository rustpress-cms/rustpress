//! `WP_Error` as the REST API serializes it.

use serde_json::{Map, Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestError {
    pub code: String,
    pub message: String,
    pub status: u16,
    /// Extra members of `data` after `status` (`params`, `details`).
    extra: Vec<(String, Value)>,
}

impl RestError {
    pub fn new(code: &str, message: &str, status: u16) -> Self {
        Self { code: code.to_owned(), message: message.to_owned(), status, extra: Vec::new() }
    }

    /// `rest_invalid_param` for one parameter.
    pub fn invalid_param(param: &str, message: &str, code: &str) -> Self {
        let mut error = Self::new("rest_invalid_param", &format!("Invalid parameter(s): {param}"), 400);
        error.extra.push(("params".into(), json!({ param: message })));
        error.extra.push(("details".into(), json!({ param: {"code": code, "message": message, "data": null} })));
        error
    }

    /// `rest_invalid_param` whose detail carries its own `WP_Error` data (the forbidden status case).
    pub fn invalid_param_with_data(param: &str, message: &str, code: &str, data: Value) -> Self {
        let mut error = Self::invalid_param(param, message, code);
        error.extra[1] = ("details".into(), json!({ param: {"code": code, "message": message, "data": data} }));
        error
    }

    pub fn to_value(&self) -> Value {
        let mut data = Map::new();
        data.insert("status".into(), Value::from(self.status));
        for (key, value) in &self.extra {
            data.insert(key.clone(), value.clone());
        }
        json!({"code": self.code, "message": self.message, "data": Value::Object(data)})
    }
}
