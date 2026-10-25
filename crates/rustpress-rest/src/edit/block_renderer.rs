//! `wp/v2/block-renderer/<namespace>/<name>`: the server-side rendering the editor shows for dynamic blocks.

use std::sync::Arc;

use rustpress_blocks::env::Context;
use rustpress_blocks::parser::Block;
use serde_json::{Map, Value, json};

use super::Args;
use crate::error::RestError;
use crate::params::Params;
use crate::{Outcome, Resp, Rest, Result};

fn insert_path(node: &mut Value, path: &[&str], value: String) {
    let Value::Object(map) = node else { return };
    match path {
        [last] => {
            map.insert((*last).to_owned(), Value::String(value));
        }
        [first, rest @ ..] => insert_path(map.entry((*first).to_owned()).or_insert_with(|| Value::Object(Map::new())), rest, value),
        [] => {}
    }
}

/// `attributes[a][b]=c` of a query string as a JSON object.
fn query_attributes(params: &Params) -> Map<String, Value> {
    let mut root = Value::Object(Map::new());
    for pair in params.raw().split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let decode = |s: &str| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.replace('+', " ").as_bytes())).into_owned();
        let key = decode(key);
        let Some(rest) = key.strip_prefix("attributes[") else { continue };
        let path: Vec<&str> = rest.split("][").map(|s| s.trim_end_matches(']')).collect();
        insert_path(&mut root, &path, decode(value));
    }
    match root {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// Objects whose keys are `0..n` (or empty, from `[]`) are lists; strings take the type the attribute declares.
fn typed(value: Value, schema: Option<&Value>) -> Value {
    let kind = schema.and_then(|s| s.get("type")).and_then(Value::as_str);
    match value {
        Value::Object(map) if kind != Some("object") && (map.keys().enumerate().all(|(i, k)| k == &i.to_string()) || map.keys().all(String::is_empty)) => Value::Array(map.into_iter().map(|(_, v)| typed(v, schema.and_then(|s| s.get("items")))).collect()),
        Value::Object(map) => {
            let props = schema.and_then(|s| s.get("properties"));
            Value::Object(map.into_iter().map(|(k, v)| (k.clone(), typed(v, props.and_then(|p| p.get(&k))))).collect())
        }
        Value::String(s) => match kind {
            Some("boolean") => Value::Bool(!matches!(s.as_str(), "" | "0" | "false")),
            Some("integer") => s.trim().parse::<i64>().map_or(Value::String(s), Value::from),
            Some("number") => s.trim().parse::<i64>().map(Value::from).or_else(|_| s.trim().parse::<f64>().map(Value::from)).unwrap_or(Value::String(s)),
            _ => Value::String(s),
        },
        other => other,
    }
}

impl Rest<'_> {
    pub(super) async fn block_renderer(&self, namespace: &str, name: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let post_id = args.u64("post_id").unwrap_or(0);
        let mut post = None;
        if post_id > 0 {
            post = self.site.db.post(post_id).await?.map(Arc::new);
            if !post.as_ref().is_some_and(|p| self.can_edit_post(p)) {
                return Ok(Err(self.forbidden("block_cannot_read", &self.site.tr("Sorry, you are not allowed to read blocks of this post."))));
            }
        } else if !self.can("edit_posts") {
            return Ok(Err(self.forbidden("block_cannot_read", &self.site.tr("Sorry, you are not allowed to read blocks as this user."))));
        }
        let full = format!("{namespace}/{name}");
        let Some(block_type) = self.registry.get(&full).filter(|_| rustpress_blocks::dynamic::is_dynamic(&full)) else {
            return Ok(Err(RestError::new("block_invalid", &self.site.tr("Invalid block."), 404)));
        };
        let raw = match args.get("attributes") {
            Some(Value::Object(map)) => map.clone(),
            _ => query_attributes(params),
        };
        let attrs: Map<String, Value> = raw.into_iter().map(|(k, v)| (k.clone(), typed(v, block_type.attributes.get(&k)))).collect();
        let block = Block { name: Some(full), attrs, inner_blocks: Vec::new(), inner_html: String::new(), inner_content: Vec::new() };
        let context = match &post {
            Some(post) => rustpress_blocks::content::context_for_post(post),
            None => Context::default(),
        };
        let env = crate::util::env(self);
        let rendered = env.render_block(&block, &context, None).await?;
        Ok(Ok(Resp::item(json!({"rendered": rendered}))))
    }
}
