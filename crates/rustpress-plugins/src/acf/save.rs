//! Saving the values a form sends (`acf_form_comment::save_comment()`): `acf_verify_nonce()`, the allowed keys,
//! `wp_kses_post_deep()` for users without `unfiltered_html`, `acf_validate_save_post()` with the `validate_value()`
//! of the field types, and `acf_save_post()` with their `update_value()`.
//!
//! Differs from the plugin: options that change a field when a value comes in (`save_custom`, `save_options`,
//! `save_other_choice`) and files sent by the basic uploader (`ACF_Media::save_files()`) are not handled.

use rustpress_core::native::{CommentSaved, Die};
use rustpress_core::{Result, Site};
use rustpress_db::{MetaKind, Writer};
use rustpress_phpser::{Key, Value};

use super::fields::{Field, FieldGroup, MatchContext, Screen};
use super::php::{is_numeric, key, key_eq, string, strval, truthy};

/// `wp_kses_post_deep()`.
fn kses_deep(value: Value) -> Value {
    match value {
        Value::Str(s) => string(&rustpress_core::kses::kses_post(&String::from_utf8_lossy(&s))),
        Value::Array(items) => Value::Array(items.into_iter().map(|(k, v)| (k, kses_deep(v))).collect()),
        other => other,
    }
}

fn key_text(k: &Key) -> String {
    match k {
        Key::Int(i) => i.to_string(),
        Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
    }
}

fn item<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    match value {
        Value::Array(items) => items.iter().find(|(k, _)| key_eq(k, &key(name))).map(|(_, v)| v),
        _ => None,
    }
}

/// `empty( $value )`.
fn empty(value: &Value) -> bool {
    !truthy(Some(value))
}

/// `acf_strlen()`: characters.
fn strlen(value: &Value) -> usize {
    strval(Some(value)).chars().count()
}

/// `floatval()`.
fn floatval(value: &Value) -> f64 {
    match super::php::numval(value) {
        Value::Int(i) => i as f64,
        Value::Float(f) => f,
        _ => rustpress_blocks::general::floatval(&strval(Some(value))),
    }
}

/// `acf_validate_value()` with the type's `validate_value()`: an invalid value adds its input and message to
/// `errors`.
fn validate_value(site: &Site, value: &Value, field: &Field, required: bool, input: &str, errors: &mut Vec<(String, String)>) {
    let tr = |text: &str| site.trd("acf", text);
    let label = strval(field.get("label"));
    let mut message = rustpress_core::i18n::sprintf(&tr("%s value is required"), &[&label]);
    let mut valid = !(required && empty(value) && !is_numeric(value));
    let mut custom: Option<String> = None;
    let setting = |name: &str| field.get(name).cloned().unwrap_or(Value::Null);
    match field.kind.as_str() {
        "checkbox" => {
            if let Value::Array(items) = value
                && truthy(field.get("allow_custom"))
                && items.iter().any(|(_, v)| empty(v) && *v != string("0"))
            {
                custom = Some(tr("Checkbox custom values cannot be empty. Uncheck any empty values."));
            }
        }
        "email" => {
            let text = strval(Some(value));
            if truthy(Some(value)) && !is_valid_email(&text) {
                custom = Some(rustpress_core::i18n::sprintf(&tr("'%s' is not a valid email address"), &[&rustpress_core::formatting::esc_html(&text)]));
            }
        }
        "group" => {
            if !empty(value) {
                for sub in &field.sub_fields {
                    if let Some(v) = item(value, &sub.key) {
                        let sub_required = required || truthy(sub.get("required"));
                        validate_value(site, v, sub, sub_required, &format!("{input}[{}]", sub.key), errors);
                    }
                }
            }
        }
        "icon_picker" => {
            if empty(value) && !required {
                valid = true;
            } else if required && (empty(value) || item(value, "value").is_none_or(empty)) {
                valid = false;
            } else if let Value::Array(_) = value {
                if item(value, "type").is_none_or(|v| *v == Value::Null) {
                    custom = Some(tr("Icon picker requires an icon type."));
                } else if item(value, "value").is_none_or(|v| *v == Value::Null) {
                    custom = Some(tr("Icon picker requires a value."));
                } else {
                    valid = true;
                }
            }
        }
        "link" => {
            if required && (empty(value) || item(value, "url").is_none_or(empty)) {
                valid = false;
            }
        }
        "number" => {
            let text = strval(Some(value)).replace(',', "");
            let number = string(&text);
            if !is_numeric(&number) {
                if !text.is_empty() && text != "0" {
                    custom = Some(tr("Value must be a number"));
                }
            } else {
                let n = floatval(&number);
                if is_numeric(&setting("min")) && n < floatval(&setting("min")) {
                    custom = Some(rustpress_core::i18n::sprintf(&tr("Value must be equal to or higher than %d"), &[&super::php::intval(field.get("min")).to_string()]));
                }
                if is_numeric(&setting("max")) && n > floatval(&setting("max")) {
                    custom = Some(rustpress_core::i18n::sprintf(&tr("Value must be equal to or lower than %d"), &[&super::php::intval(field.get("max")).to_string()]));
                }
            }
        }
        "relationship" => {
            let count = match value {
                Value::Array(items) if !items.is_empty() => items.len() as i64,
                _ => 0,
            };
            let min = super::php::intval(field.get("min"));
            if count < min {
                let format = site.i18n().ngettext("acf", "%1$s requires at least %2$s selection", "%1$s requires at least %2$s selections", min);
                custom = Some(rustpress_core::i18n::sprintf(&format, &[&label, &min.to_string()]));
            }
        }
        "text" | "textarea" => {
            let max = super::php::intval(field.get("maxlength"));
            if truthy(field.get("maxlength")) && strlen(value) as i64 > max {
                custom = Some(rustpress_core::i18n::sprintf(&tr("Value must not exceed %d characters"), &[&max.to_string()]));
            }
        }
        "true_false" => {
            if required && !truthy(Some(value)) {
                valid = false;
            }
        }
        "url" => {
            let text = strval(Some(value));
            if truthy(Some(value)) && !text.contains("://") && !text.starts_with("//") {
                custom = Some(tr("Value must be a valid URL"));
            }
        }
        _ => {}
    }
    if let Some(text) = custom {
        message = text;
        valid = false;
    }
    if !valid {
        errors.push((input.to_owned(), message));
    }
}

/// `filter_var( $email, FILTER_VALIDATE_EMAIL, FILTER_FLAG_EMAIL_UNICODE )`, close enough: one `@`, a local part and
/// a domain with a dot, no spaces.
fn is_valid_email(text: &str) -> bool {
    let Some((local, domain)) = text.rsplit_once('@') else { return false };
    !local.is_empty() && local.len() <= 64 && !domain.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.') && !text.chars().any(|c| c.is_whitespace() || c == '<' || c == '>' || c == '"' || c == ',')
}

/// `acf_idval()`.
fn idval(value: &Value) -> i64 {
    match value {
        v if is_numeric(v) => super::php::intval(Some(v)),
        Value::Array(_) => super::php::intval(item(value, "ID")),
        _ => 0,
    }
}

/// `acf_is_sequential_array()`.
fn sequential(value: &Value) -> bool {
    matches!(value, Value::Array(items) if items.iter().all(|(k, _)| matches!(k, Key::Int(_))))
}

fn map_items(value: Value, f: impl Fn(&Value) -> Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(|(k, v)| (k.clone(), f(v))).collect()),
        other => other,
    }
}

/// The value as stored: `update_metadata()` serializes arrays, writes `false` as nothing and numbers as text.
fn stored(value: &Value) -> String {
    match value {
        Value::Array(_) => String::from_utf8_lossy(&rustpress_phpser::serialize(value)).into_owned(),
        Value::Bool(true) => "1".to_owned(),
        Value::Bool(false) | Value::Null => String::new(),
        other => strval(Some(other)),
    }
}

/// `acf_update_value()`: the type's `update_value()`, then the value and its reference (`_{name}` = key).
fn update_value<'a>(writer: &'a Writer, object: (MetaKind, u64), value: Value, field: &'a Field, name: &'a str) -> futures::future::BoxFuture<'a, Result<()>> {
    Box::pin(async move {
        let value = match field.kind.as_str() {
            "checkbox" | "select" if !empty(&value) => map_items(value, |v| string(&strval(Some(v)))),
            "file" | "image" if !empty(&value) => Value::Int(idval(&value)),
            "google_map" => {
                let value = match &value {
                    Value::Str(s) => serde_json::from_slice::<serde_json::Value>(s).ok().map_or(Value::Null, |j| json_to_php(&j)),
                    other => other.clone(),
                };
                match value {
                    v if truthy(Some(&v)) => match v {
                        Value::Array(_) => v,
                        other => Value::Array(vec![(Key::Int(0), other)]),
                    },
                    _ => Value::Bool(false),
                }
            }
            "link" if empty(&value) || item(&value, "url").is_none_or(empty) => string(""),
            "number" if !empty(&value) => string(&strval(Some(&value)).replace(',', "")),
            "page_link" if !empty(&value) => {
                let maybe = |v: &Value| match idval(v) {
                    0 => v.clone(),
                    id => Value::Int(id),
                };
                if sequential(&value) { map_items(value, |v| string(&strval(Some(&maybe(v))))) } else { maybe(&value) }
            }
            "post_object" | "relationship" | "user" if !empty(&value) => {
                if sequential(&value) {
                    map_items(value, |v| string(&idval(v).to_string()))
                } else {
                    Value::Int(idval(&value))
                }
            }
            "taxonomy" => match value {
                Value::Array(items) => Value::Array(items.into_iter().filter(|(_, v)| truthy(Some(v))).collect()),
                other => other,
            },
            "group" => {
                if !matches!(value, Value::Array(_)) || field.sub_fields.is_empty() {
                    Value::Null
                } else {
                    // `prepare_field_for_db()`: the sub fields are named `{group}_{sub}`.
                    for sub in &field.sub_fields {
                        let Some(v) = item(&value, &sub.key).or_else(|| item(&value, &sub.name)) else { continue };
                        let sub_name = format!("{name}_{}", sub.name);
                        update_value(writer, object, v.clone(), sub, &sub_name).await?;
                    }
                    string("")
                }
            }
            _ => value,
        };
        if name.is_empty() {
            return Ok(());
        }
        if value == Value::Null {
            writer.delete_metadata(object.0, object.1, name).await?;
            writer.delete_metadata(object.0, object.1, &format!("_{name}")).await?;
            return Ok(());
        }
        writer.update_metadata(object.0, object.1, name, &stored(&value)).await?;
        writer.update_metadata(object.0, object.1, &format!("_{name}"), &field.key).await?;
        Ok(())
    })
}

/// `json_decode( $json, true )`.
fn json_to_php(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => n.as_i64().map_or_else(|| Value::Float(n.as_f64().unwrap_or(0.0)), Value::Int),
        serde_json::Value::String(s) => string(s),
        serde_json::Value::Array(items) => Value::Array(items.iter().enumerate().map(|(i, v)| (Key::Int(i as i64), json_to_php(v))).collect()),
        serde_json::Value::Object(map) => Value::Array(map.iter().map(|(k, v)| (super::php::key(k), json_to_php(v))).collect()),
    }
}

/// `acf_form_comment::save_comment()` on `comment_post`.
pub async fn save_comment(groups: &[FieldGroup], ctx: &CommentSaved<'_>) -> Result<Option<Die>> {
    let site = ctx.site;
    let nonce = ctx.form.iter().rev().find(|(k, _)| k == "_acf_nonce").map(|(_, v)| v.as_str()).unwrap_or_default();
    if nonce.is_empty() || !(ctx.verify_nonce)(nonce, "comment") {
        return Ok(None);
    }
    let Some(acf) = rustpress_core::native::form_value(ctx.form, "acf") else { return Ok(None) };
    let Value::Array(items) = acf else { return Ok(None) };
    // The fields of the groups shown for the comment's post type.
    let post_type = site.db.post(ctx.post_id).await?.map(|p| p.post_type).unwrap_or_default();
    let matcher = MatchContext { site, viewer: None };
    let mut allowed: Vec<&Field> = Vec::new();
    for group in groups {
        if group.visible(&Screen::Comment { post_type: post_type.clone() }, &matcher).await? {
            allowed.extend(group.fields.iter().filter(|f| !f.key.is_empty()));
        }
    }
    let mut values: Vec<(&Field, Value)> = Vec::new();
    for (k, v) in items {
        if let Some(field) = allowed.iter().find(|f| f.key == key_text(&k)) {
            values.push((field, v));
        }
    }
    if values.is_empty() {
        return Ok(None);
    }
    // `save_comment()` filters every value; `acf_save_post()` filters once more for users without `unfiltered_html`.
    values = values.into_iter().map(|(f, v)| (f, kses_deep(v))).collect();
    let unfiltered = (ctx.can)("unfiltered_html");
    // `acf_validate_save_post( true )`.
    let mut errors = Vec::new();
    for (field, value) in &values {
        validate_value(site, value, field, truthy(field.get("required")), &format!("acf[{}]", field.key), &mut errors);
    }
    if !errors.is_empty() {
        let tr = |text: &str| site.trd("acf", text);
        let mut message = format!("<h2>{}</h2><ul>", tr("Validation failed"));
        for (_, error) in &errors {
            message.push_str(&format!("<li>{error}</li>"));
        }
        message.push_str("</ul>");
        return Ok(Some(Die { title: rustpress_core::formatting::esc_html(&tr("Validation failed")), message: rustpress_core::kses::kses_post(&message), status: 500 }));
    }
    // `acf_save_post( "comment_{$id}" )`.
    if !unfiltered {
        values = values.into_iter().map(|(f, v)| (f, kses_deep(v))).collect();
    }
    for (field, value) in values {
        update_value(ctx.writer, (MetaKind::Comment, ctx.comment_id), value, field, &field.name).await?;
    }
    Ok(None)
}

/// `acf_get_field( $key )`: a field of any group by its key, sub fields included.
fn field_by_key<'a>(groups: &'a [FieldGroup], key: &str) -> Option<&'a Field> {
    fn find<'a>(fields: &'a [Field], key: &str) -> Option<&'a Field> {
        fields.iter().find_map(|f| if f.key == key { Some(f) } else { find(&f.sub_fields, key) })
    }
    groups.iter().find_map(|g| find(&g.fields, key))
}

/// `acf_validation::ajax_validate_save_post()` (`wp_ajax_acf/validate_save_post`, also for visitors): the errors of
/// every value of a known field, for the form's script.
pub fn ajax_validate(groups: &[FieldGroup], ctx: &rustpress_core::native::AdminContext<'_>) -> rustpress_core::native::AjaxAnswer {
    let site = ctx.site;
    let tr = |text: &str| site.trd("acf", text);
    let request = |name: &str| ctx.form.iter().chain(ctx.query.iter()).rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
    let nonce = request("nonce").unwrap_or_default();
    let encode = |data: serde_json::Value| rustpress_blocks::php_json::encode_with(&serde_json::json!({"success": true, "data": data}), rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: false });
    if nonce.is_empty() || !(ctx.verify_nonce)(nonce, "acf_nonce") {
        let message = if nonce.is_empty() { tr("ACF was unable to perform validation because no nonce was received by the server.") } else { tr("ACF was unable to perform validation because the provided nonce failed verification.") };
        let url = rustpress_core::formatting::esc_url("https://www.advancedcustomfields.com/resources/validation-nonce-errors/?utm_source=ACF+Free&utm_medium=insideplugin&utm_campaign=docs&utm_content=validation-nonce");
        let data = serde_json::json!({"valid": 0, "errors": [{"input": false, "message": message, "action": {"label": tr("Learn more"), "url": url}}]});
        return rustpress_core::native::AjaxAnswer::json(200, encode(data));
    }
    let mut errors = Vec::new();
    if let Some(Value::Array(items)) = rustpress_core::native::form_value(ctx.form, "acf")
        && !items.is_empty()
    {
        for (k, v) in &items {
            let key = key_text(k);
            if let Some(field) = field_by_key(groups, &key) {
                validate_value(site, v, field, truthy(field.get("required")), &format!("acf[{key}]"), &mut errors);
            }
        }
    }
    let data = if errors.is_empty() {
        serde_json::json!({"valid": 1, "errors": 0})
    } else {
        let list: Vec<serde_json::Value> = errors.into_iter().map(|(input, message)| serde_json::json!({"input": input, "message": message})).collect();
        serde_json::json!({"valid": 0, "errors": list})
    };
    rustpress_core::native::AjaxAnswer::json(200, encode(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_values() {
        assert_eq!(stored(&Value::Bool(false)), "");
        assert_eq!(stored(&Value::Int(5)), "5");
        let list = Value::Array(vec![(Key::Int(0), string("2")), (Key::Int(1), string("x"))]);
        assert_eq!(stored(&list), "a:2:{i:0;s:1:\"2\";i:1;s:1:\"x\";}");
    }

    #[test]
    fn ids_and_sequences() {
        assert_eq!(idval(&string("12")), 12);
        assert_eq!(idval(&Value::Array(vec![(key("ID"), Value::Int(7))])), 7);
        assert_eq!(idval(&string("abc")), 0);
        assert!(sequential(&Value::Array(vec![(Key::Int(0), string("a"))])));
        assert!(!sequential(&Value::Array(vec![(key("a"), string("a"))])));
    }

    #[test]
    fn email_addresses() {
        assert!(is_valid_email("a@b.de"));
        assert!(!is_valid_email("nope"));
        assert!(!is_valid_email("a b@c.de"));
    }
}
