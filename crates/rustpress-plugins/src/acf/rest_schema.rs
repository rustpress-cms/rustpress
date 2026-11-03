//! The schema of the `acf` field of the REST API (`ACF_Rest_Api::get_schema()`, `build_per_object_schema()`): the
//! `get_rest_schema()` of every field type (`acf_get_field_rest_schema()`), the type-level union of the field groups
//! of an object type (`get_field_groups_by_object_type()`), and the groups of one object where the caller may read it.

use rustpress_core::Site;
use rustpress_phpser::{Key, Value};
use serde_json::{Map, Value as Json, json};

use super::fields::{Field, FieldGroup};
use super::php::{intval, is_numeric, strval, truthy};

fn php(value: &Value) -> Json {
    rustpress_core::options::php_to_json(value)
}

/// `isset( $field[ $key ] ) && '' !== $field[ $key ]`.
fn set_and_not_empty_string(value: Option<&Value>) -> Option<&Value> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::Str(s)) if s.is_empty() => None,
        Some(v) => Some(v),
    }
}

/// `(float) $value`.
fn float(value: &Value) -> f64 {
    super::save::floatval(value)
}

/// `acf_field::get_rest_schema()`.
fn base(field: &Field) -> Map<String, Json> {
    let mut schema = Map::new();
    schema.insert("type".into(), json!(["string", "null"]));
    schema.insert("required".into(), Json::Bool(truthy(field.get("required"))));
    if let Some(default) = set_and_not_empty_string(field.get("default_value")) {
        schema.insert("default".into(), php(default));
    }
    schema
}

/// `acf_field_select::format_rest_choices()`: the keys when they differ from the labels, else the labels, as strings,
/// then the numeric ones as integers.
fn format_rest_choices(choices: Option<&Value>) -> Vec<Json> {
    let items = match choices {
        Some(Value::Array(items)) => items.clone(),
        _ => Vec::new(),
    };
    let key_text = |k: &Key| match k {
        Key::Int(i) => i.to_string(),
        Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
    };
    let keys: Vec<String> = items.iter().map(|(k, _)| key_text(k)).collect();
    let values: Vec<String> = items.iter().map(|(_, v)| strval(Some(v))).collect();
    let chosen = if keys.iter().any(|k| !values.contains(k)) { keys } else { values };
    let mut out: Vec<Json> = chosen.iter().map(|c| Json::String(c.clone())).collect();
    for choice in &chosen {
        if is_numeric(&super::php::string(choice)) {
            out.push(json!(intval(Some(&super::php::string(choice)))));
        }
    }
    out
}

/// `acf_get_field_rest_schema()`: the `get_rest_schema()` of the field's type.
pub fn field_schema(site: &Site, can_upload: bool, field: &Field) -> Map<String, Json> {
    let tr = |text: &str| rustpress_core::formatting::esc_html(&site.trd("acf", text));
    let required = || Json::Bool(truthy(field.get("required")));
    let empty = |key: &str| !truthy(field.get(key));
    let mut schema = Map::new();
    let file = |schema: &mut Map<String, Json>| {
        schema.insert("type".into(), json!(["integer", "null"]));
        schema.insert("required".into(), required());
        for (key, name, int) in [("min_width", "minWidth", true), ("min_height", "minHeight", true), ("min_size", "minSize", false), ("max_width", "maxWidth", true), ("max_height", "maxHeight", true), ("max_size", "maxSize", false), ("mime_types", "mimeTypes", false)] {
            if truthy(field.get(key)) {
                schema.insert(name.into(), if int { json!(intval(field.get(key))) } else { php(field.get(key).unwrap_or(&Value::Null)) });
            }
        }
    };
    match field.kind.as_str() {
        "button_group" | "radio" => {
            schema = base(field);
            if let Some(default) = set_and_not_empty_string(field.get("default_value")) {
                schema.insert("default".into(), php(default));
            }
            if field.kind == "radio" && truthy(field.get("other_choice")) {
                return schema;
            }
            let mut choices = format_rest_choices(field.get("choices"));
            if field.kind == "button_group" {
                choices.push(Json::Null);
                if truthy(field.get("allow_null")) {
                    choices.push(Json::String(String::new()));
                }
            } else if truthy(field.get("allow_null")) {
                choices.push(Json::Null);
            }
            schema.insert("enum".into(), Json::Array(choices));
        }
        "checkbox" => {
            schema.insert("type".into(), json!(["integer", "string", "array", "null"]));
            schema.insert("required".into(), required());
            schema.insert("items".into(), json!({"type": ["string", "integer"]}));
            if let Some(default) = set_and_not_empty_string(field.get("default_value")) {
                schema.insert("default".into(), php(default));
            }
            if !truthy(field.get("allow_custom"))
                && let Some(Json::Object(items)) = schema.get_mut("items")
            {
                items.insert("enum".into(), Json::Array(format_rest_choices(field.get("choices"))));
            }
        }
        "date_picker" | "date_time_picker" | "time_picker" => {
            let description = match field.kind.as_str() {
                "date_picker" => "A `Ymd` formatted date string.",
                "date_time_picker" => "A `Y-m-d H:i:s` formatted date string.",
                _ => "A `H:i:s` formatted time string.",
            };
            schema.insert("type".into(), json!(["string", "null"]));
            schema.insert("description".into(), json!(description));
            schema.insert("required".into(), required());
        }
        "email" | "url" | "oembed" => {
            schema = base(field);
            schema.insert("format".into(), json!(if field.kind == "email" { "email" } else { "uri" }));
        }
        "file" | "image" => file(&mut schema),
        "google_map" => {
            schema.insert("type".into(), json!(["object", "null"]));
            schema.insert("required".into(), required());
            schema.insert(
                "properties".into(),
                json!({
                    "address": {"type": "string"},
                    "lat": {"type": ["string", "float"]},
                    "lng": {"type": ["string", "float"]},
                    "zoom": {"type": ["string", "int"]},
                    "place_id": {"type": "string"},
                    "name": {"type": "string"},
                    "street_number": {"type": ["string", "int"]},
                    "street_name": {"type": "string"},
                    "street_name_short": {"type": "string"},
                    "city": {"type": "string"},
                    "state": {"type": "string"},
                    "state_short": {"type": "string"},
                    "post_code": {"type": ["string", "int"]},
                    "country": {"type": "string"},
                    "country_short": {"type": "string"},
                }),
            );
        }
        "group" => {
            schema.insert("type".into(), json!(["object", "null"]));
            let mut properties = Map::new();
            for sub_field in &field.sub_fields {
                let sub = field_schema(site, can_upload, sub_field);
                if !sub.is_empty() {
                    properties.insert(sub_field.name.clone(), Json::Object(sub));
                }
            }
            schema.insert("properties".into(), Json::Object(properties));
            schema.insert("required".into(), required());
        }
        "icon_picker" => {
            // `get_tabs()`: the media library for who may upload.
            let mut tabs = vec!["dashicons"];
            if can_upload {
                tabs.push("media_library");
            }
            tabs.push("url");
            schema.insert("type".into(), json!(["object", "null"]));
            schema.insert("required".into(), required());
            schema.insert(
                "properties".into(),
                json!({
                    "type": {"description": tr("The type of icon to save."), "type": ["string"], "required": true, "enum": tabs},
                    "value": {"description": tr("The value of icon to save."), "type": ["string", "int"], "required": true},
                }),
            );
        }
        "link" => {
            schema.insert("type".into(), json!(["object", "null"]));
            schema.insert("required".into(), required());
            schema.insert("properties".into(), json!({"title": {"type": "string"}, "url": {"type": "string", "required": true, "format": "uri"}, "target": {"type": "string"}}));
        }
        "number" => {
            schema.insert("type".into(), json!(["number", "null"]));
            schema.insert("required".into(), required());
            if !empty("min") {
                schema.insert("minimum".into(), json!(float(field.get("min").unwrap_or(&Value::Null))));
            }
            if !empty("max") {
                schema.insert("maximum".into(), json!(float(field.get("max").unwrap_or(&Value::Null))));
            }
            if let Some(default) = field.get("default_value").filter(|v| is_numeric(v)) {
                schema.insert("default".into(), json!(float(default)));
            }
        }
        "page_link" => {
            let mut kinds = vec!["integer", "array", "null"];
            let mut item_kinds = vec!["integer"];
            if truthy(field.get("allow_archives")) {
                kinds.push("string");
                item_kinds.push("string");
            }
            schema.insert("type".into(), json!(kinds));
            schema.insert("required".into(), required());
            schema.insert("items".into(), json!({"type": item_kinds}));
            if empty("allow_null") {
                schema.insert("minItems".into(), json!(1));
            }
            if empty("multiple") {
                schema.insert("maxItems".into(), json!(1));
            }
        }
        "post_object" | "user" | "relationship" | "taxonomy" => {
            schema.insert("type".into(), json!(["integer", "array", "null"]));
            schema.insert("required".into(), required());
            schema.insert("items".into(), json!({"type": "integer"}));
            if empty("allow_null") {
                schema.insert("minItems".into(), json!(1));
            }
            match field.kind.as_str() {
                "relationship" => {
                    if !empty("min") {
                        schema.insert("minItems".into(), json!(intval(field.get("min"))));
                    }
                    if !empty("max") {
                        schema.insert("maxItems".into(), json!(intval(field.get("max"))));
                    }
                }
                "taxonomy" => {
                    if matches!(strval(field.get("field_type")).as_str(), "radio" | "select") {
                        schema.insert("maxItems".into(), json!(1));
                    }
                }
                _ => {
                    if empty("multiple") {
                        schema.insert("maxItems".into(), json!(1));
                    }
                }
            }
        }
        "range" => {
            schema.insert("type".into(), json!(["number", "null"]));
            schema.insert("required".into(), required());
            schema.insert("minimum".into(), json!(if empty("min") { 0 } else { intval(field.get("min")) }));
            schema.insert("maximum".into(), json!(if empty("max") { 100 } else { intval(field.get("max")) }));
            if let Some(default) = field.get("default_value").filter(|v| is_numeric(v)) {
                schema.insert("default".into(), json!(intval(Some(default))));
            }
        }
        "select" => {
            schema.insert("type".into(), json!(["string", "array", "int", "null"]));
            schema.insert("required".into(), required());
            schema.insert("items".into(), json!({"type": ["string", "int"], "enum": format_rest_choices(field.get("choices"))}));
            if empty("allow_null") {
                schema.insert("minItems".into(), json!(1));
            }
            if empty("multiple") {
                schema.insert("maxItems".into(), json!(1));
            }
            if let Some(default) = set_and_not_empty_string(field.get("default_value")) {
                schema.insert("default".into(), php(default));
            }
        }
        "text" | "textarea" => {
            schema = base(field);
            if !empty("maxlength") {
                schema.insert("maxLength".into(), json!(intval(field.get("maxlength"))));
            }
        }
        "true_false" => {
            schema.insert("type".into(), json!(["boolean", "null"]));
            schema.insert("required".into(), required());
            if let Some(default) = set_and_not_empty_string(field.get("default_value")) {
                schema.insert("default".into(), Json::Bool(truthy(Some(default))));
            }
        }
        _ => schema = base(field),
    }
    schema
}

/// `ACF_Rest_Api::build_schema_from_field_groups()` without its `arg_options` (`get_public_item_schema()` drops them):
/// the fields of the groups that show in REST, by name.
pub fn build(site: &Site, can_upload: bool, groups: &[&FieldGroup]) -> Json {
    let mut properties = Map::new();
    for group in groups {
        for field in group.fields.iter().filter(|f| super::fields::show_in_rest(&f.kind)) {
            properties.insert(field.name.clone(), Json::Object(field_schema(site, can_upload, field)));
        }
    }
    json!({"description": "ACF field data", "type": "object", "properties": properties})
}

/// `acf_get_location_rule( $param )->get_object_type()`: the `$object_type` of ACF's location types (free), empty
/// for the others.
pub(super) fn object_type_of(param: &str) -> &'static str {
    match param {
        "attachment" => "attachment",
        "comment" => "comment",
        "nav_menu_item" => "menu_item",
        "nav_menu" => "menu",
        "page_parent" | "page_template" | "page_type" | "page" | "post_category" | "post_format" | "post_status" | "post_taxonomy" | "post_template" | "post_type" | "post" => "post",
        "taxonomy" => "term",
        "user_form" | "user_role" => "user",
        "widget" => "widget",
        _ => "",
    }
}

/// `ACF_Rest_Api::object_type_has_field_group()`: a group belongs to an object type when a rule group has a rule of
/// that type – for posts a `post_type` rule that fits the post type, for terms a `taxonomy` rule that fits the
/// taxonomy, for users and comments any of their rules.
pub fn object_type_has_field_group(group: &FieldGroup, object_type: &str, sub_type: &str) -> bool {
    group.location.iter().any(|rules| {
        let mut matched = false;
        for rule in rules {
            if object_type_of(&rule.param) != object_type {
                continue;
            }
            let value = strval(Some(&rule.value));
            let fits = |param: &str| {
                rule.param == param
                    && !(rule.operator == "==" && sub_type != value)
                    && !(rule.operator == "!=" && sub_type == value)
            };
            if (object_type == "post" && fits("post_type")) || (object_type == "term" && fits("taxonomy")) || matches!(object_type, "user" | "comment") {
                matched = true;
            }
        }
        matched
    })
}
