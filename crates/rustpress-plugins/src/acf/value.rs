//! Field values: `acf_get_value()` with the `load_value()` of the field types, and `format_value_for_rest()` of the
//! `light` format of the REST API.

use futures::future::BoxFuture;
use rustpress_core::Result;
use rustpress_db::{Db, MetaKind};
use rustpress_phpser::{Key, Value};

use super::fields::Field;
use super::php::{is_numeric, numval, set, strval, truthy};

/// The object a value belongs to (`acf_decode_post_id()`).
#[derive(Debug, Clone, Copy)]
pub struct Object {
    pub kind: MetaKind,
    pub id: u64,
}

/// `acf_is_empty()`: false but not a number.
fn is_empty(value: &Value) -> bool {
    !truthy(Some(value)) && !is_numeric(value)
}

/// `acf_unarray()`: the first item of an array (`reset()`, false for an empty one).
fn unarray(value: Value) -> Value {
    match value {
        Value::Array(items) => items.into_iter().next().map_or(Value::Bool(false), |(_, v)| v),
        other => other,
    }
}

/// `(array) $value`.
fn to_array(value: Value) -> Value {
    match value {
        Value::Array(_) => value,
        Value::Null => Value::Array(Vec::new()),
        other => Value::Array(vec![(Key::Int(0), other)]),
    }
}

/// `acf_get_value()`: the meta value of the field's name, its `default_value` when there is none, then
/// `acf/load_value` of the type.
pub fn get_value<'a>(db: &'a Db, object: Object, field: &'a Field, name: &'a str) -> BoxFuture<'a, Result<Value>> {
    Box::pin(async move {
        let mut value = match db.acf_meta(object.kind, object.id, name).await? {
            Some(raw) => rustpress_core::options::maybe_unserialize(&raw),
            None => Value::Null,
        };
        if value == Value::Null
            && let Some(default) = field.get("default_value").filter(|v| !matches!(v, Value::Null))
        {
            value = default.clone();
        }
        load_value(db, object, field, name, value).await
    })
}

/// The `load_value()` methods of the field types.
async fn load_value(db: &Db, object: Object, field: &Field, name: &str, value: Value) -> Result<Value> {
    Ok(match field.kind.as_str() {
        // `acf_field_group::load_value()`: the values of the sub fields by key, named `{group}_{sub}`.
        "group" => {
            if field.sub_fields.is_empty() {
                return Ok(value);
            }
            let mut out = Vec::new();
            for sub in &field.sub_fields {
                let sub_name = format!("{name}_{}", sub.name);
                let sub_value = get_value(db, object, sub, &sub_name).await?;
                out.push((super::php::key(&sub.key), sub_value));
            }
            Value::Array(out)
        }
        "select" => {
            if truthy(field.get("multiple")) {
                if is_empty(&value) { Value::Array(Vec::new()) } else { to_array(value) }
            } else {
                unarray(value)
            }
        }
        "post_object" | "user" if value == super::php::string("null") => Value::Bool(false),
        // `acf_field_radio::load_value()` (also the button group's): a single value.
        "radio" | "button_group" => match value {
            Value::Array(mut items) => items.pop().map_or(Value::Null, |(_, v)| v),
            other => other,
        },
        // `acf_field_google_map::load_value()`.
        "google_map" => {
            if truthy(Some(&value)) {
                let defaults = Value::Array(vec![(super::php::key("address"), super::php::string("")), (super::php::key("lat"), Value::Int(0)), (super::php::key("lng"), Value::Int(0))]);
                super::php::parse_args(&value, defaults)
            } else {
                Value::Bool(false)
            }
        }
        // `acf_field_taxonomy::load_value()`: `acf_get_valid_terms()` makes integers; `load_terms` reads the
        // object's terms instead.
        "taxonomy" => {
            let mut terms: Vec<Value> = match value {
                Value::Array(items) => items.into_iter().map(|(_, v)| Value::Int(super::php::intval(Some(&v)))).collect(),
                v if is_empty(&v) => Vec::new(),
                v => vec![Value::Int(super::php::intval(Some(&v)))],
            };
            if truthy(field.get("load_terms")) {
                let taxonomy = strval(field.get("taxonomy"));
                let ids: Vec<u64> = if object.kind == MetaKind::Post { db.terms_for_post(object.id, Some(&taxonomy)).await?.iter().map(|t| t.term_id).collect() } else { Vec::new() };
                if ids.is_empty() {
                    return Ok(Value::Bool(false));
                }
                // `array_multisort()` by the position in the stored value (`false` first for the ones not in it).
                let mut order: Vec<(Option<usize>, u64)> = ids.iter().map(|id| (terms.iter().position(|t| *t == Value::Int(*id as i64)), *id)).collect();
                if !terms.is_empty() {
                    order.sort();
                }
                terms = order.into_iter().map(|(_, id)| Value::Int(id as i64)).collect();
            }
            if matches!(strval(field.get("field_type")).as_str(), "select" | "radio") {
                terms.into_iter().next().unwrap_or(Value::Null)
            } else {
                Value::Array(terms.into_iter().enumerate().map(|(i, v)| (Key::Int(i as i64), v)).collect())
            }
        }
        _ => value,
    })
}

/// `acf_format_numerics()`.
fn format_numerics(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(|(k, v)| (k, if is_numeric(&v) { numval(&v) } else { v })).collect()),
        v if is_numeric(&v) => numval(&v),
        v => v,
    }
}

/// `format_value_for_rest()` of the field types (the `light` format).
pub fn format_for_rest(value: Value, field: &Field) -> Value {
    match field.kind.as_str() {
        "file" | "image" | "number" | "range" | "page_link" | "post_object" | "relationship" | "user" => format_numerics(value),
        "google_map" => {
            if truthy(Some(&value)) {
                format_numerics(value)
            } else {
                Value::Null
            }
        }
        "date_picker" => {
            if truthy(Some(&value)) {
                super::php::string(&strval(Some(&value)))
            } else {
                Value::Null
            }
        }
        "true_false" => Value::Bool(truthy(Some(&value))),
        // `acf_field_group::format_value_for_rest()`: each sub field's value moves from its key to its name.
        "group" => {
            let Value::Array(mut items) = value else { return value };
            if items.is_empty() || field.sub_fields.is_empty() {
                return Value::Array(items);
            }
            for sub in &field.sub_fields {
                let k = super::php::key(&sub.key);
                let position = items.iter().position(|(existing, _)| super::php::key_eq(existing, &k));
                let sub_value = position.map_or(Value::Null, |i| items.remove(i).1);
                set(&mut items, &sub.name, format_for_rest(sub_value, sub));
            }
            Value::Array(items)
        }
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acf::php::string;

    fn field(kind: &str, key: &str, name: &str, sub_fields: Vec<Field>) -> Field {
        Field { key: key.into(), name: name.into(), kind: kind.into(), settings: Value::Array(Vec::new()), sub_fields }
    }

    #[test]
    fn rest_values_of_the_light_format() {
        assert_eq!(format_for_rest(string("42"), &field("number", "k", "n", vec![])), Value::Int(42));
        assert_eq!(format_for_rest(string("7.5"), &field("range", "k", "n", vec![])), Value::Float(7.5));
        assert_eq!(format_for_rest(string("abc"), &field("number", "k", "n", vec![])), string("abc"));
        assert_eq!(format_for_rest(Value::Int(0), &field("true_false", "k", "n", vec![])), Value::Bool(false));
        assert_eq!(format_for_rest(string(""), &field("date_picker", "k", "n", vec![])), Value::Null);
        assert_eq!(format_for_rest(Value::Bool(false), &field("google_map", "k", "n", vec![])), Value::Null);
        let list = Value::Array(vec![(Key::Int(0), string("5")), (Key::Int(1), string("x"))]);
        assert_eq!(format_for_rest(list, &field("relationship", "k", "n", vec![])), Value::Array(vec![(Key::Int(0), Value::Int(5)), (Key::Int(1), string("x"))]));
    }

    #[test]
    fn groups_rename_their_values() {
        let group = field("group", "field_g", "g", vec![field("text", "field_a", "a", vec![]), field("number", "field_b", "b", vec![])]);
        let value = Value::Array(vec![(Key::Str(b"field_a".to_vec()), string("x")), (Key::Str(b"field_b".to_vec()), string("3"))]);
        assert_eq!(format_for_rest(value, &group), Value::Array(vec![(Key::Str(b"a".to_vec()), string("x")), (Key::Str(b"b".to_vec()), Value::Int(3))]));
    }
}
