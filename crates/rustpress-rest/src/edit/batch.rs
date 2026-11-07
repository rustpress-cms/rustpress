//! `batch/v1` (`WP_REST_Server::serve_batch_request_v1()`).

use serde_json::{Map, Value, json};

use super::{Args, Body, Request};
use crate::error::RestError;
use crate::params::Params;
use crate::{Outcome, Resp, Rest, Result};

/// `allow_batch` of the controllers: posts of any type, terms, users and widgets, as lists and as items.
fn allows_batch(rest: &Rest<'_>, path: &str) -> bool {
    let path = path.trim_matches('/');
    let segments: Vec<&str> = path.split('/').collect();
    match segments.as_slice() {
        ["wp", "v2", base] | ["wp", "v2", base, _] => matches!(*base, "posts" | "pages" | "categories" | "tags" | "users" | "widgets" | "wp_pattern_category") || rest.custom_route(base).is_some(),
        _ => false,
    }
}

impl Rest<'_> {
    /// One request of a batch, with its status and headers in the body.
    fn batch_one<'s>(&'s self, method: &'s str, path: &'s str, query: &'s str, body: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + 's>> {
        Box::pin(async move {
            let query = if query.is_empty() { "_envelope=1".to_owned() } else { format!("{query}&_envelope=1") };
            let inherited = self.request.clone().unwrap_or_default();
            let request = Request { method: method.to_owned(), body: Body::Json(body), content_type: Some("application/json".to_owned()), ..inherited };
            let reply = self.handle_as(path, &query, self.access.clone(), request).await?;
            Ok(serde_json::from_str(&reply.body).unwrap_or(Value::Null))
        })
    }

    pub(crate) async fn batch(&self, args: &Args, _params: &Params) -> Result<Outcome> {
        let Some(Value::Array(requests)) = args.get("requests") else {
            return Ok(Err(RestError::missing_params(&["requests"])));
        };
        if requests.len() > 25 {
            let message = rustpress_core::i18n::sprintf(&self.site.trn("%1$s must contain at most %2$s item.", "%1$s must contain at most %2$s items.", 25), &["requests", "25"]);
            return Ok(Err(RestError::invalid_param("requests", &message, "rest_too_many_items")));
        }
        // The answer of a request that cannot run, in the place of its response: with `require-all-validate` nothing
        // ran, otherwise a route that matched still sends what it allows.
        let all_or_nothing = args.str("validation").as_deref() == Some("require-all-validate");
        let mut refused: Vec<Option<Value>> = Vec::new();
        for request in requests {
            let path = request.get("path").and_then(Value::as_str).unwrap_or_default();
            let path = path.split_once('?').map_or(path, |(p, _)| p);
            let error = if self.route_help(path).is_none() {
                Some((RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404), false))
            } else if !allows_batch(self, path) {
                Some((RestError::new("rest_batch_not_allowed", "The requested route does not support batch requests.", 400), true))
            } else {
                None
            };
            refused.push(error.map(|(e, matched)| {
                let headers = if matched && !all_or_nothing { json!({"Allow": self.allow_for(path)}) } else { json!([]) };
                json!({"body": e.to_value(self.site), "status": e.status, "headers": headers})
            }));
        }
        let mut responses = Vec::new();
        if refused.iter().any(Option::is_some) && all_or_nothing {
            let mut data = Map::new();
            data.insert("failed".into(), json!("validation"));
            data.insert("responses".into(), Value::Array(refused.into_iter().map(|answer| answer.unwrap_or(Value::Null)).collect()));
            let mut resp = Resp::item(Value::Object(data));
            resp.status = Some(207);
            resp.allow = Some("POST".to_owned());
            return Ok(Ok(resp));
        }
        for (request, refusal) in requests.iter().zip(refused) {
            if let Some(answer) = refusal {
                responses.push(answer);
                continue;
            }
            let method = request.get("method").and_then(Value::as_str).unwrap_or("POST");
            let path = request.get("path").and_then(Value::as_str).unwrap_or_default();
            let (route, query) = path.split_once('?').unwrap_or((path, ""));
            let body = request.get("body").cloned().unwrap_or_else(|| json!({}));
            responses.push(self.batch_one(method, route, query, body).await?);
        }
        let mut resp = Resp::item(json!({"responses": responses}));
        resp.status = Some(207);
        resp.allow = Some("POST".to_owned());
        Ok(Ok(resp))
    }
}
