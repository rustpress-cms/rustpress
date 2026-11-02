//! The filters ACF hooks to `acf/rest/format_value_for_rest` for the `standard` format: users become IDs for
//! readers without `list_users` (`acf_rest_apply_user_data_sanitizer()`), and references to posts the reader may
//! not see become the IDs of the `light` format (`acf_rest_apply_reference_sanitizer()`, ACF 6.8.10).

use futures::future::BoxFuture;
use rustpress_core::{Result, Site, Viewer};
use rustpress_db::Post;
use rustpress_phpser::{Key, Value};

use super::fields::Field;
use super::php::{intval, is_numeric, key, key_eq, truthy};

/// The field types whose `standard` value expands a post or an attachment.
fn is_reference_leaf(kind: &str) -> bool {
    matches!(kind, "relationship" | "post_object" | "image" | "gallery" | "file")
}

/// `acf_idval()`.
fn idval(value: &Value) -> Value {
    match value {
        v if is_numeric(v) => Value::Int(intval(Some(v))),
        Value::Array(_) => value.get("ID").cloned().unwrap_or(Value::Int(0)),
        _ => Value::Int(0),
    }
}

/// The value of a key of an array, `None` when the key is missing (`array_key_exists()`).
fn lookup<'v>(items: &'v [(Key, Value)], name: &str) -> Option<&'v Value> {
    let k = key(name);
    items.iter().find(|(existing, _)| key_eq(existing, &k)).map(|(_, v)| v)
}

/// What the filters need: the site and the reader.
pub struct Sanitizer<'a> {
    pub site: &'a Site,
    pub viewer: Option<&'a Viewer>,
}

impl Sanitizer<'_> {
    /// Both filters in the order ACF adds them.
    pub async fn apply(&self, formatted: Value, raw: &Value, field: &Field) -> Result<Value> {
        let mut formatted = formatted;
        if matches!(field.kind.as_str(), "user" | "group") && !self.viewer.is_some_and(|v| v.can("list_users")) {
            formatted = user_data(formatted, raw, field);
        }
        if is_reference_leaf(&field.kind) || field.kind == "group" || field.kind == "icon_picker" {
            formatted = self.reference_data(formatted, raw, field).await?;
        }
        Ok(formatted)
    }

    /// `acf_rest_sanitize_reference_data()`.
    fn reference_data<'b>(&'b self, formatted: Value, raw: &'b Value, field: &'b Field) -> BoxFuture<'b, Result<Value>> {
        Box::pin(async move {
            if field.kind == "icon_picker" {
                return self.reduce_icon_picker(formatted, raw).await;
            }
            if is_reference_leaf(&field.kind) {
                return self.reduce_reference(formatted, raw).await;
            }
            let Value::Array(mut items) = formatted else { return Ok(formatted) };
            if field.kind != "group" {
                return Ok(Value::Array(items));
            }
            let raw_items: &[(Key, Value)] = match raw {
                Value::Array(items) => items,
                _ => &[],
            };
            for sub in &field.sub_fields {
                let Some(position) = items.iter().position(|(k, _)| key_eq(k, &key(&sub.name))) else { continue };
                if sub.name.is_empty() {
                    continue;
                }
                let raw_sub = lookup(raw_items, &sub.key).cloned().unwrap_or(Value::Null);
                let value = std::mem::replace(&mut items[position].1, Value::Null);
                items[position].1 = self.reference_data(value, &raw_sub, sub).await?;
            }
            Ok(Value::Array(items))
        })
    }

    /// `acf_rest_reduce_reference()`: all or nothing.
    async fn reduce_reference(&self, formatted: Value, raw: &Value) -> Result<Value> {
        if !truthy(Some(&formatted)) {
            return Ok(formatted);
        }
        let (ids, multi): (Vec<i64>, bool) = match raw {
            Value::Array(items) => (items.iter().map(|(_, v)| intval(Some(v))).filter(|i| *i != 0).collect(), true),
            v if is_numeric(v) && intval(Some(v)) > 0 => (vec![intval(Some(v))], false),
            _ => {
                let sequential = match &formatted {
                    Value::Array(items) => items.is_empty() || items.iter().all(|(k, _)| matches!(k, Key::Int(_))),
                    _ => false,
                };
                let items: Vec<&Value> = match &formatted {
                    Value::Array(items) if sequential => items.iter().map(|(_, v)| v).collect(),
                    other => vec![other],
                };
                let ids = items
                    .into_iter()
                    .filter_map(|item| match item {
                        Value::Array(_) => item.get("ID").map(|id| intval(Some(id))),
                        v if is_numeric(v) => Some(intval(Some(v))),
                        _ => None,
                    })
                    .filter(|i| *i != 0)
                    .collect();
                (ids, sequential)
            }
        };
        if ids.is_empty() {
            return Ok(formatted);
        }
        for id in &ids {
            if !self.exposable(*id).await? {
                return Ok(if multi { Value::Array(ids.iter().enumerate().map(|(i, id)| (Key::Int(i as i64), Value::Int(*id))).collect()) } else { Value::Int(ids[0]) });
            }
        }
        Ok(formatted)
    }

    /// `acf_rest_reduce_icon_picker_reference()`.
    async fn reduce_icon_picker(&self, formatted: Value, raw: &Value) -> Result<Value> {
        let Value::Array(items) = &formatted else { return Ok(formatted) };
        let kind = lookup(items, "type").cloned().or_else(|| raw.get("type").cloned());
        if kind != Some(super::php::string("media_library")) {
            return Ok(formatted);
        }
        let target = match raw.get("value").filter(|v| is_numeric(v)) {
            Some(v) => intval(Some(v)),
            None => match lookup(items, "value") {
                Some(expanded @ Value::Array(_)) => expanded.get("ID").map_or(0, |id| intval(Some(id))),
                Some(v) if is_numeric(v) => intval(Some(v)),
                _ => 0,
            },
        };
        if target <= 0 || self.exposable(target).await? {
            return Ok(formatted);
        }
        Ok(Value::Array(vec![(key("type"), super::php::string("media_library")), (key("value"), Value::Int(target))]))
    }

    /// `acf_rest_reference_is_exposable()`: the post's type shows in REST and the posts controller would show it.
    async fn exposable(&self, id: i64) -> Result<bool> {
        let Some(post) = (if id > 0 { self.site.db.post(id as u64).await? } else { None }) else { return Ok(false) };
        if !super::post_types(self.site).iter().any(|(name, _)| *name == post.post_type) {
            return Ok(false);
        }
        self.check_read_permission(post).await
    }

    /// `WP_REST_Posts_Controller::check_read_permission()`.
    async fn check_read_permission(&self, post: Post) -> Result<bool> {
        let mut current = post;
        for _ in 0..32 {
            if current.post_status == "publish" || self.read_post(&current).await? {
                return Ok(true);
            }
            // `acf-disabled` is the only other public status.
            if current.post_status == "acf-disabled" {
                return Ok(true);
            }
            if current.post_status != "inherit" {
                return Ok(false);
            }
            if current.post_parent == 0 {
                return Ok(true);
            }
            match self.site.db.post(current.post_parent).await? {
                Some(parent) => current = parent,
                None => return Ok(true),
            }
        }
        Ok(false)
    }

    /// `current_user_can( 'read_post', $post->ID )`: nobody without a login.
    async fn read_post(&self, post: &Post) -> Result<bool> {
        let Some(viewer) = self.viewer else { return Ok(false) };
        let mut effective = post.clone();
        // `get_post_status()`: an attachment that inherits has the status of its parent.
        if post.post_type == "attachment" && post.post_status == "inherit" {
            effective.post_status = match self.site.db.post(post.post_parent).await? {
                Some(parent) if post.post_parent != 0 && post.post_parent != post.id => parent.post_status,
                _ => "publish".to_owned(),
            };
        }
        Ok(match effective.post_status.as_str() {
            "publish" | "acf-disabled" => viewer.can("read"),
            _ if effective.post_author == viewer.id && effective.post_author != 0 => viewer.can("read"),
            "private" => viewer.can_read_private(&effective.post_type),
            _ => viewer.can_edit_post(&effective),
        })
    }
}

/// `acf_rest_sanitize_user_data()`.
fn user_data(formatted: Value, raw: &Value, field: &Field) -> Value {
    if field.kind == "user" {
        if !truthy(Some(&formatted)) {
            return formatted;
        }
        if truthy(field.get("multiple"))
            && let Value::Array(items) = &formatted
        {
            return Value::Array(items.iter().enumerate().map(|(i, (_, user))| (Key::Int(i as i64), idval(user))).collect());
        }
        return idval(&formatted);
    }
    let Value::Array(mut items) = formatted else { return formatted };
    if field.kind != "group" {
        return Value::Array(items);
    }
    let raw_items: &[(Key, Value)] = match raw {
        Value::Array(items) => items,
        _ => &[],
    };
    for sub in &field.sub_fields {
        if sub.name.is_empty() {
            continue;
        }
        let Some(position) = items.iter().position(|(k, _)| key_eq(k, &key(&sub.name))) else { continue };
        let raw_sub = lookup(raw_items, &sub.key).cloned().unwrap_or(Value::Null);
        let value = std::mem::replace(&mut items[position].1, Value::Null);
        items[position].1 = user_data(value, &raw_sub, sub);
    }
    Value::Array(items)
}
