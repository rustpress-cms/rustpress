//! The field types of WPForms Lite (`includes/fields/class-*.php`): their properties
//! (`wpforms_field_properties_{$type}`), their output (`field_display()`) and the population of their values
//! (`WPForms_Field::field_prefill_value_property()`).

use rustpress_core::formatting::{esc_attr, esc_html, esc_url, html_entity_decode, sanitize_html_class};
use rustpress_core::kses::kses_post;
use serde_json::{Map, Value, json};

use super::php::{self, empty, html_attributes, set, str_of, string};
use super::render::{Ctx, required_label};

/// The types whose class Lite loads on `init` (`WPForms_Fields::load()`); `gdpr-checkbox` only with the setting
/// `gdpr`. The other types are Pro fields that print nothing in Lite.
pub fn is_known(ty: &str) -> bool {
    matches!(ty, "text" | "textarea" | "select" | "radio" | "checkbox" | "email" | "name" | "number" | "number-slider" | "gdpr-checkbox")
}

/// `WPForms_Field::field_data()` on `wpforms_field_data`: `wp_parse_args( $field, $this->default_settings )`, the
/// common defaults (`COMMON_DEFAULT_SETTINGS`) and the default choices of a choice field.
pub fn field_data(field: Value, settings: &super::settings::Settings) -> Option<Value> {
    let ty = str_of(&field, "type");
    if ty == "gdpr-checkbox" && !settings.set("gdpr") {
        return None;
    }
    let mut merged = Map::new();
    for (k, v) in [("id", json!(0)), ("type", json!("")), ("label", json!("")), ("description", json!("")), ("size", json!("medium")), ("default_value", json!("")), ("css", json!("")), ("read_only", json!(0))] {
        merged.insert(k.into(), v);
    }
    let labels: &[&str] = match ty.as_str() {
        "select" | "radio" | "checkbox" => &["First Choice", "Second Choice", "Third Choice"],
        "gdpr-checkbox" => &["I consent to having this website store my submitted information so they can respond to my inquiry."],
        _ => &[],
    };
    if !labels.is_empty() {
        let mut choices = Map::new();
        for (i, label) in labels.iter().enumerate() {
            let choice = match ty.as_str() {
                "radio" | "checkbox" => json!({"label": label, "value": "", "image": "", "icon": "", "icon_style": "", "default": ""}),
                "gdpr-checkbox" => json!({"label": label, "value": "", "image": "", "default": ""}),
                _ => json!({"label": label, "value": "", "default": ""}),
            };
            choices.insert((i + 1).to_string(), choice);
        }
        merged.insert("choices".into(), Value::Object(choices));
    }
    if let Value::Object(f) = field {
        for (k, v) in f {
            merged.insert(k, v);
        }
    }
    Some(Value::Object(merged))
}

/// `ReadOnlyField::$allow_read_only`.
pub fn allows_read_only(ty: &str) -> bool {
    ty != "gdpr-checkbox"
}

/// `wpforms_frontend_modern_is_field_requires_fieldset_{$type}`.
pub fn requires_fieldset(field: &Value) -> bool {
    match str_of(field, "type").as_str() {
        "radio" | "checkbox" => true,
        "email" => set(field, "confirmation"),
        "name" => field.get("format").is_some_and(|f| !f.is_null()) && str_of(field, "format") != "simple",
        _ => false,
    }
}

fn id_of(field: &Value) -> i64 {
    php::int(field.get("id"))
}

/// `wpforms_field_properties_{$type}`.
pub fn properties(mut p: Value, field: &Value, ctx: &Ctx<'_, '_>) -> Value {
    let form_id = ctx.form_id;
    let field_id = id_of(field);
    match str_of(field, "type").as_str() {
        "text" => text_properties(&mut p, field),
        "email" => {
            p["inputs"]["primary"]["attr"]["spellcheck"] = json!("false");
            if set(field, "confirmation") {
                p = email_confirmation_properties(p, field, form_id, field_id, ctx);
            }
            if set(field, "filter_type") && set(field, &str_of(field, "filter_type")) {
                p["inputs"]["primary"]["data"]["rule-restricted-email"] = json!(true);
            }
        }
        "name" => p = name_properties(p, field, form_id, field_id, ctx),
        "number" => {
            if php::is_numeric(field.get("min")) {
                p["inputs"]["primary"]["attr"]["min"] = json!(php::to_float(field.get("min")));
            }
            if php::is_numeric(field.get("max")) {
                p["inputs"]["primary"]["attr"]["max"] = json!(php::to_float(field.get("max")));
            }
            p["inputs"]["primary"]["attr"]["step"] = json!("any");
        }
        "select" => select_properties(&mut p, field, form_id, field_id),
        "radio" | "checkbox" => choices_properties(&mut p, field, form_id, field_id, ctx),
        "gdpr-checkbox" => gdpr_properties(&mut p, field, form_id, field_id),
        _ => {}
    }
    p
}

/// `WPForms_Field_Text::field_properties()`: the input mask.
fn text_properties(p: &mut Value, field: &Value) {
    if !set(field, "input_mask") {
        return;
    }
    let mask = str_of(field, "input_mask");
    let primary = &mut p["inputs"]["primary"];
    php::push(&mut primary["class"], "wpforms-masked-input");
    primary["data"]["rule-inputmask-incomplete"] = json!(true);
    if mask.contains("alias:") {
        primary["data"]["inputmask-alias"] = json!(mask.replace("alias:", ""));
    } else if mask.contains("regex:") {
        primary["data"]["inputmask-regex"] = json!(mask.replace("regex:", ""));
    } else if mask.contains("date:") {
        let mask = mask.replace("date:", "");
        primary["data"]["inputmask-alias"] = json!("datetime");
        primary["data"]["inputmask-inputmode"] = json!("");
        primary["data"]["inputmask-inputformat"] = json!(mask);
        // The keys keep the order in which they were set.
        if let Value::Object(data) = &mut primary["data"] {
            data.shift_remove("inputmask-inputmode");
            data.insert("inputmask-inputmode".into(), json!(if mask.contains(['t', 'T']) { "text" } else { "numeric" }));
        }
    } else {
        primary["data"]["inputmask-mask"] = json!(mask);
    }
}

/// `WPForms_Field_Email::confirmation_field_properties()`.
fn email_confirmation_properties(p: Value, field: &Value, form_id: i64, field_id: i64, ctx: &Ctx<'_, '_>) -> Value {
    let primary_id = string(&p["inputs"]["primary"]["id"]);
    let props = json!({
        "inputs": {
            "primary": {
                "block": ["wpforms-field-row-block", "wpforms-one-half", "wpforms-first"],
                "class": ["wpforms-field-email-primary"],
                "sublabel": {"hidden": set(field, "sublabel_hide"), "value": ctx.tr("Email")},
            },
            "secondary": {
                "attr": {
                    "name": format!("wpforms[fields][{field_id}][secondary]"),
                    "value": "",
                    "placeholder": if set(field, "confirmation_placeholder") { field["confirmation_placeholder"].clone() } else { json!("") },
                    "spellcheck": "false",
                },
                "block": ["wpforms-field-row-block", "wpforms-one-half"],
                "class": ["wpforms-field-email-secondary"],
                "data": {"rule-confirm": format!("#{primary_id}")},
                "id": format!("wpforms-{form_id}-field_{field_id}-secondary"),
                "required": if set(field, "required") { "required" } else { "" },
                "sublabel": {"hidden": set(field, "sublabel_hide"), "value": ctx.tr("Confirm Email")},
                "value": "",
            },
        },
    });
    let mut p = php::array_merge_recursive(p, props);
    p["inputs"]["primary"]["attr"]["name"] = json!(format!("wpforms[fields][{field_id}][primary]"));
    let size_class = format!("wpforms-field-{}", sanitize_html_class(&str_of(field, "size")));
    if let Some(list) = p["inputs"]["primary"]["class"].as_array_mut() {
        list.retain(|c| { let c = string(c); c != size_class && c != "wpforms-error" });
    }
    if set(&p["error"]["value"], "primary") {
        php::push(&mut p["inputs"]["primary"]["class"], "wpforms-error");
    }
    if set(&p["error"]["value"], "secondary") {
        php::push(&mut p["inputs"]["secondary"]["class"], "wpforms-error");
    }
    if set(field, "required") {
        php::push(&mut p["inputs"]["secondary"]["class"], "wpforms-field-required");
    }
    if !set(field, "sublabel_hide") {
        if let Value::Object(attr) = &mut p["label"]["attr"] {
            attr.shift_remove("for");
        }
    }
    p
}

/// `WPForms_Field_Name::field_properties()`.
fn name_properties(mut p: Value, field: &Value, form_id: i64, field_id: i64, ctx: &Ctx<'_, '_>) -> Value {
    let format = if set(field, "format") { esc_attr(&str_of(field, "format")) } else { "first-last".to_owned() };
    let smart = |key: &str| if set(field, key) { (ctx.env.smart_tags)(&str_of(field, key), ctx.form_data, "field-properties") } else { String::new() };
    let text = |key: &str| if set(field, key) { field[key].clone() } else { json!("") };
    if format == "simple" {
        p["inputs"]["primary"]["attr"]["placeholder"] = text("simple_placeholder");
        p["inputs"]["primary"]["attr"]["value"] = json!(smart("simple_default"));
        return p;
    }
    if let Value::Object(inputs) = &mut p["inputs"] {
        inputs.shift_remove("primary");
    }
    if !set(field, "sublabel_hide") {
        if let Value::Object(attr) = &mut p["label"]["attr"] {
            attr.shift_remove("for");
        }
    }
    let hidden = set(field, "sublabel_hide");
    let required = if set(field, "required") { "required" } else { "" };
    let props = json!({
        "inputs": {
            "first": {
                "attr": {"name": format!("wpforms[fields][{field_id}][first]"), "value": smart("first_default"), "placeholder": text("first_placeholder")},
                "block": ["wpforms-field-row-block", "wpforms-first"],
                "class": ["wpforms-field-name-first"],
                "data": {},
                "id": format!("wpforms-{form_id}-field_{field_id}"),
                "required": required,
                "sublabel": {"hidden": hidden, "value": ctx.tr("First")},
            },
            "middle": {
                "attr": {"name": format!("wpforms[fields][{field_id}][middle]"), "value": smart("middle_default"), "placeholder": text("middle_placeholder")},
                "block": ["wpforms-field-row-block", "wpforms-one-fifth"],
                "class": ["wpforms-field-name-middle"],
                "data": {},
                "id": format!("wpforms-{form_id}-field_{field_id}-middle"),
                "required": "",
                "sublabel": {"hidden": hidden, "value": ctx.tr("Middle")},
            },
            "last": {
                "attr": {"name": format!("wpforms[fields][{field_id}][last]"), "value": smart("last_default"), "placeholder": text("last_placeholder")},
                "block": ["wpforms-field-row-block"],
                "class": ["wpforms-field-name-last"],
                "data": {},
                "id": format!("wpforms-{form_id}-field_{field_id}-last"),
                "required": required,
                "sublabel": {"hidden": hidden, "value": ctx.tr("Last")},
            },
        },
    });
    let mut p = php::array_merge_recursive(p, props);
    let common_error = !empty(p["error"].get("value")) && p["error"]["value"].is_string();
    let column = if format == "first-last" { "wpforms-one-half" } else { "wpforms-two-fifths" };
    if set(&p["error"]["value"], "first") || common_error {
        php::push(&mut p["inputs"]["first"]["class"], "wpforms-error");
    }
    if set(field, "required") {
        php::push(&mut p["inputs"]["first"]["class"], "wpforms-field-required");
    }
    php::push(&mut p["inputs"]["first"]["block"], column);
    if common_error {
        php::push(&mut p["inputs"]["middle"]["class"], "wpforms-error");
    }
    if set(&p["error"]["value"], "last") || common_error {
        php::push(&mut p["inputs"]["last"]["class"], "wpforms-error");
    }
    if set(field, "required") {
        php::push(&mut p["inputs"]["last"]["class"], "wpforms-field-required");
    }
    php::push(&mut p["inputs"]["last"]["block"], column);
    p
}

/// The choices of a field as the form data holds them.
fn choices(field: &Value) -> Vec<(String, Value)> {
    match field.get("choices") {
        Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Some(Value::Array(a)) => a.iter().enumerate().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        _ => Vec::new(),
    }
}

/// `WPForms_Field_Select::field_properties()`.
fn select_properties(p: &mut Value, field: &Value, form_id: i64, field_id: i64) {
    if let Value::Object(inputs) = &mut p["inputs"] {
        inputs.shift_remove("primary");
    }
    p["input_container"] = json!({"class": [], "data": {}, "id": format!("wpforms-{form_id}-field_{field_id}"), "attr": {"name": format!("wpforms[fields][{field_id}]")}});
    for (key, choice) in choices(field) {
        let depth = if choice.get("depth").is_some() { php::int(choice.get("depth")).abs() } else { 1 };
        let label = choice.get("label").filter(|v| !v.is_null()).cloned().unwrap_or(json!(""));
        let value = if field.get("show_values").is_some_and(|v| !v.is_null()) { choice.get("value").cloned().unwrap_or(Value::Null) } else { label.clone() };
        p["inputs"][key.as_str()] = json!({
            "container": {"attr": {}, "class": [format!("choice-{key}"), format!("depth-{depth}")], "data": {}, "id": ""},
            "label": {"attr": {"for": format!("wpforms-{form_id}-field_{field_id}_{key}")}, "class": ["wpforms-field-label-inline"], "data": {}, "id": "", "text": label},
            "attr": {"name": format!("wpforms[fields][{field_id}]"), "value": value},
            "class": [],
            "data": {},
            "id": format!("wpforms-{form_id}-field_{field_id}_{key}"),
            "required": if set(field, "required") { "required" } else { "" },
            "default": choice.get("default").is_some_and(|v| !v.is_null()),
        });
    }
    if set(field, "size") {
        php::push(&mut p["input_container"]["class"], format!("wpforms-field-{}", esc_attr(&str_of(field, "size"))));
    }
    if set(field, "required") {
        php::push(&mut p["input_container"]["class"], "wpforms-field-required");
    }
    let style = str_of(field, "style");
    if set(field, "style") && (style == "classic" || style == "modern") {
        php::push(&mut p["container"]["class"], format!("wpforms-field-select-style-{style}"));
    }
}

/// `WPForms_Field_Radio::field_properties()` and `WPForms_Field_Checkbox::field_properties()`.
fn choices_properties(p: &mut Value, field: &Value, form_id: i64, field_id: i64, ctx: &Ctx<'_, '_>) {
    let checkbox = str_of(field, "type") == "checkbox";
    if let Value::Object(inputs) = &mut p["inputs"] {
        inputs.shift_remove("primary");
    }
    if let Value::Object(attr) = &mut p["label"]["attr"] {
        attr.shift_remove("for");
    }
    p["input_container"] = json!({"class": [if set(field, "random") { "wpforms-randomize" } else { "" }], "data": {}, "attr": {}, "id": format!("wpforms-{form_id}-field_{field_id}")});
    let limit = checkbox && set(field, "choice_limit") && php::int(field.get("choice_limit")) > 0;
    if limit {
        p["input_container"]["data"]["choice-limit"] = field["choice_limit"].clone();
    }
    let all = choices(field);
    let count = all.len();
    for (key, choice) in all {
        let depth = if choice.get("depth").is_some() { php::int(choice.get("depth")).abs() } else { 1 };
        let label = choice.get("label").filter(|v| !v.is_null()).cloned().unwrap_or(json!(""));
        let show_values = if checkbox { field.get("show_values").is_some_and(|v| !v.is_null()) } else { set(field, "show_values") };
        let mut value = if show_values { choice.get("value").cloned().unwrap_or(Value::Null) } else { label.clone() };
        // `'' === $value` (checkbox) and `$value === ''` (radio): only the empty string.
        if value.as_str() == Some("") {
            value = if checkbox && count == 1 { json!(ctx.tr("Checked")) } else { json!(ctx.tr("Choice %s").replace("%s", &key)) };
        }
        let other = choice.get("other").is_some_and(|o| !empty(Some(o)));
        let mut container_class = vec![json!(format!("choice-{key}")), json!(format!("depth-{depth}"))];
        if other {
            container_class.push(json!("wpforms-other-choice"));
        } else if !checkbox {
            container_class.push(json!(""));
        }
        let text_of = |k: &str| choice.get(k).filter(|v| !v.is_null()).cloned().unwrap_or(json!(""));
        let mut input = json!({
            "container": {"attr": {}, "class": container_class, "data": {}, "id": ""},
            "label": {"attr": {"for": format!("wpforms-{form_id}-field_{field_id}_{key}")}, "class": ["wpforms-field-label-inline"], "data": {}, "id": "", "text": label},
            "attr": {"name": if checkbox { format!("wpforms[fields][{field_id}][]") } else { format!("wpforms[fields][{field_id}]") }, "value": value},
            "class": [],
            "data": if other { json!({"other-choice": "true"}) } else { json!({}) },
            "id": format!("wpforms-{form_id}-field_{field_id}_{key}"),
            "icon": text_of("icon"),
            "icon_style": text_of("icon_style"),
            "image": text_of("image"),
            "required": if set(field, "required") { "required" } else { "" },
            "default": choice.get("default").is_some_and(|v| !v.is_null()),
        });
        if limit {
            input["data"]["rule-check-limit"] = json!("true");
        }
        p["inputs"][key.as_str()] = input;
    }
    if set(field, "required") {
        php::push(&mut p["input_container"]["class"], "wpforms-field-required");
    }
    if set(field, "choices_images") {
        let style = str_of(field, "choices_images_style");
        php::push(&mut p["input_container"]["class"], "wpforms-image-choices");
        php::push(&mut p["input_container"]["class"], format!("wpforms-image-choices-{}", sanitize_html_class(&style)));
        if let Value::Object(inputs) = &mut p["inputs"] {
            for input in inputs.values_mut() {
                php::push(&mut input["container"]["class"], "wpforms-image-choices-item");
                if style == "modern" || style == "classic" {
                    php::push(&mut input["class"], "wpforms-screen-reader-element");
                }
            }
        }
    }
    if checkbox && set(field, "disclaimer_format") {
        php::push(&mut p["description"]["class"], "wpforms-disclaimer-description");
        p["description"]["value"] = json!(nl2br(&string(&p["description"]["value"])));
    }
    if let Value::Object(inputs) = &mut p["inputs"] {
        for input in inputs.values_mut() {
            if input["default"].as_bool() == Some(true) {
                php::push(&mut input["container"]["class"], "wpforms-selected");
            }
        }
    }
}

/// `nl2br()`.
pub fn nl2br(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match (b[i], b.get(i + 1)) {
            (b'\r', Some(b'\n')) | (b'\n', Some(b'\r')) => {
                out.push_str("<br />");
                out.push(b[i] as char);
                out.push(b[i + 1] as char);
                i += 2;
            }
            (b'\r' | b'\n', _) => {
                out.push_str("<br />");
                out.push(b[i] as char);
                i += 1;
            }
            _ => {
                let len = utf8_len(b[i]);
                out.push_str(&text[i..i + len]);
                i += len;
            }
        }
    }
    out
}

fn utf8_len(first: u8) -> usize {
    match first {
        0xF0..=0xFF => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

/// `WPForms_Field_GDPR_Checkbox::field_properties()`.
fn gdpr_properties(p: &mut Value, field: &Value, form_id: i64, field_id: i64) {
    if let Value::Object(inputs) = &mut p["inputs"] {
        inputs.shift_remove("primary");
    }
    if let Value::Object(attr) = &mut p["label"]["attr"] {
        attr.shift_remove("for");
    }
    p["input_container"] = json!({"class": [], "data": {}, "attr": {}, "id": format!("wpforms-{form_id}-field_{field_id}")});
    let list = if set(field, "choices") { choices(field) } else { Vec::new() };
    for (key, choice) in list {
        let label = choice.get("label").cloned().unwrap_or(Value::Null);
        p["inputs"][key.as_str()] = json!({
            "container": {"attr": {}, "class": [format!("choice-{key}")], "data": {}, "id": ""},
            "label": {"attr": {"for": format!("wpforms-{form_id}-field_{field_id}_{key}")}, "class": ["wpforms-field-label-inline"], "data": {}, "id": "", "text": label},
            "attr": {"name": format!("wpforms[fields][{field_id}][]"), "value": label},
            "class": [],
            "data": {},
            "id": format!("wpforms-{form_id}-field_{field_id}_{key}"),
            "image": "",
            "required": if set(field, "required") { "required" } else { "" },
            "default": "",
        });
    }
    if set(field, "required") {
        php::push(&mut p["input_container"]["class"], "wpforms-field-required");
    }
}

/// `WPForms_Field::field_prefill_value_property()`: values from the query string (`wpf{form}_{field}`, with the form
/// setting `dynamic_population`) and, after a failed submission, from `$_POST['wpforms']['fields']`.
pub fn prefill(mut p: Value, field: &Value, ctx: &Ctx<'_, '_>) -> Value {
    if set(&ctx.form_data["settings"], "dynamic_population") && !ctx.env.get.is_empty() {
        let original = p.clone();
        static KEY: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re = KEY.get_or_init(|| regex::Regex::new(r"(?i)wpf(\d+)_(\d+)(.*)").expect("valid regex"));
        // `$_GET` keeps the last value of a repeated key, at the place of its first.
        let mut get: Vec<(String, String)> = Vec::new();
        for (k, v) in ctx.env.get {
            match get.iter_mut().find(|(n, _)| n == k) {
                Some(slot) => slot.1 = v.clone(),
                None => get.push((k.clone(), v.clone())),
            }
        }
        for (key, raw) in &get {
            let Some(m) = re.captures(key) else { continue };
            let form = php::leading_int(&m[1]);
            let field_id = php::leading_int(&m[2]);
            let input = if m[3].is_empty() { "primary".to_owned() } else { sanitize_key(m[3].trim_matches('_')) };
            if ctx.form_id != form || id_of(field) != field_id {
                continue;
            }
            let mut values = vec![raw.clone()];
            if !raw.is_empty() && raw != "0" {
                remove_choices_defaults(field, &mut p);
                if matches!(str_of(field, "type").as_str(), "checkbox" | "select") {
                    let decoded = String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(raw.as_bytes())).into_owned();
                    values = decoded.split('|').map(str::to_owned).collect();
                }
            }
            for value in values {
                p = populate_single(&value, &input, p, field, ctx);
            }
        }
        p = encode_shortcode_delimiters(p, &original);
    }
    let Some(sub) = ctx.env.submitted.filter(|s| s.form_id == ctx.form_id) else { return p };
    let id = str_of(field, "id");
    let posted = &sub.post["fields"];
    let raw = posted.get(php::leading_int(&id).to_string());
    if empty(Some(posted)) || raw.is_none() || !posted.is_object() {
        return p;
    }
    let raw = raw.cloned().unwrap_or(Value::Null);
    let original = p.clone();
    if !empty(Some(&raw)) {
        remove_choices_defaults(field, &mut p);
    }
    match &raw {
        Value::Object(m) => {
            for (input, value) in m {
                if let Value::String(s) = value {
                    p = populate_single(s, &sanitize_key(input), p, field, ctx);
                }
            }
        }
        Value::Array(list) => {
            for (input, value) in list.iter().enumerate() {
                if let Value::String(s) = value {
                    p = populate_single(s, &input.to_string(), p, field, ctx);
                }
            }
        }
        Value::String(s) => p = populate_single(s, "primary", p, field, ctx),
        _ => {}
    }
    encode_shortcode_delimiters(p, &original)
}

/// `sanitize_key()`.
pub fn sanitize_key(key: &str) -> String {
    key.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect()
}

/// `WPForms_Field::field_prefill_remove_choices_defaults()`.
fn remove_choices_defaults(field: &Value, p: &mut Value) {
    if !set(field, "dynamic_choices") && !set(field, "choices") {
        return;
    }
    fn walk(v: &mut Value) {
        match v {
            Value::Object(m) => {
                for (k, v) in m.iter_mut() {
                    if k == "default" && !v.is_object() && !v.is_array() {
                        *v = json!(false);
                    } else {
                        walk(v);
                    }
                }
            }
            Value::Array(a) => a.iter_mut().for_each(walk),
            Value::String(s) if s == "wpforms-selected" => *v = json!(""),
            _ => {}
        }
    }
    walk(&mut p["inputs"]);
}

/// `WPForms_Field::get_field_populated_single_property_value()`.
fn populate_single(raw: &str, input: &str, mut p: Value, field: &Value, ctx: &Ctx<'_, '_>) -> Value {
    let value = rustpress_content::sanitize::text_field(raw);
    let value = stripslashes(&value);
    if set(field, "dynamic_choices") {
        return p;
    }
    if set(field, "choices") && field["choices"].is_object() {
        let key_name = if field.get("show_values").is_some() { "value" } else { "label" };
        let mut found = None;
        for (key, choice) in choices(field) {
            let matches = choice.get(key_name).is_some_and(|v| !v.is_null() && rustpress_content::sanitize::text_field(&string(v)).to_uppercase() == value.to_uppercase())
                || (empty(choice.get(key_name)) && value == ctx.tr("Choice %d").replace("%d", &php::leading_int(&key).to_string()));
            if matches {
                found = Some(key);
                break;
            }
        }
        if let Some(key) = found {
            p["inputs"][key.as_str()]["default"] = json!(true);
            php::push(&mut p["inputs"][key.as_str()]["container"]["class"], "wpforms-selected");
        }
        return p;
    }
    if !input.is_empty() && p["inputs"].get(input).is_some() {
        p["inputs"][input]["attr"]["value"] = json!(value);
    }
    p
}

/// `stripslashes()`.
pub fn stripslashes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('0') => out.push('\0'),
                Some(n) => out.push(n),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `WPForms_Field::encode_shortcode_delimiters()`: populated values cannot open a shortcode.
fn encode_shortcode_delimiters(mut p: Value, original: &Value) -> Value {
    if let Value::Object(inputs) = &mut p["inputs"] {
        for (key, input) in inputs.iter_mut() {
            let Some(Value::String(value)) = input.get("attr").and_then(|a| a.get("value")) else { continue };
            if original["inputs"].get(key).and_then(|i| i.get("attr")).and_then(|a| a.get("value")).and_then(Value::as_str) == Some(value.as_str()) {
                continue;
            }
            let encoded = value.replace('[', "&#091;").replace(']', "&#093;");
            input["attr"]["value"] = json!(encoded);
        }
    }
    p
}

/// `WPForms_Field::field_display_sublabel()`.
fn sublabel(out: &mut String, key: &str, position: &str, field: &Value) {
    let input = &field["properties"]["inputs"][key];
    let sub = &input["sublabel"];
    if empty(sub.get("value")) {
        return;
    }
    let field_position = if set(sub, "position") { str_of(sub, "position") } else { "after".to_owned() };
    if field_position != position {
        return;
    }
    let mut classes = vec![json!("wpforms-field-sublabel"), json!(field_position)];
    if set(sub, "hidden") {
        classes.push(json!("wpforms-sublabel-hide"));
    }
    out.push_str(&format!("<label for=\"{}\" class=\"{}\">{}</label>", esc_attr(&string(&input["id"])), php::sanitize_classes(&json!(classes)), esc_html(&str_of(sub, "value"))));
}

/// `WPForms_Field::field_display_error()`.
fn display_error(out: &mut String, key: &str, field: &Value) {
    let errors = &field["properties"]["error"]["value"];
    if !set(errors, key) {
        return;
    }
    out.push_str(&format!("<label class=\"wpforms-error\" for=\"{}\">{}</label>", esc_attr(&string(&field["properties"]["inputs"][key]["id"])), esc_html(&str_of(errors, key))));
}

fn attrs(input: &Value) -> String {
    html_attributes(&string(&input["id"]), &input["class"], &input["data"], &input["attr"])
}

/// `TextLengthLimit::add_length_limit_attributes()`.
fn length_limit(mut primary: Value, field: &Value, form_id: i64) -> Value {
    let (min, max) = length_limits(field);
    if min == 0 && max == 0 {
        return primary;
    }
    let mode = if set(field, "limit_mode") && sanitize_key(&str_of(field, "limit_mode")) != "characters" { "words" } else { "characters" };
    primary["data"]["form-id"] = json!(form_id.to_string());
    primary["data"]["field-id"] = field["id"].clone();
    php::push(&mut primary["class"], if max > 0 { format!("wpforms-limit-{mode}-enabled") } else { format!("wpforms-limit-min-{mode}-enabled") });
    if max > 0 {
        primary["data"]["text-limit"] = json!(max);
    }
    if max > 0 && mode == "characters" {
        primary["attr"]["maxlength"] = json!(max);
    }
    if min > 0 {
        primary["data"]["rule-text-limit-min"] = json!(min);
    }
    primary
}

/// `TextLengthLimit::get_enforced_length_limits()`.
pub fn length_limits(field: &Value) -> (i64, i64) {
    if !set(field, "limit_enabled") {
        return (0, 0);
    }
    let min = if set(field, "limit_min") { php::int(field.get("limit_min")).abs() } else { 0 };
    let max = if set(field, "limit_count") { php::int(field.get("limit_count")).abs() } else { 0 };
    let mut min = if max > 0 { min.min(max) } else { min };
    if set(field, "read_only") || set(field, "calculation_is_enabled") || set(field, "enable_address_autocomplete") {
        min = 0;
    }
    (min, max)
}

/// `field_display()` of the field's type.
pub fn display(out: &mut String, field: &Value, ctx: &Ctx<'_, '_>) {
    let props = &field["properties"];
    match str_of(field, "type").as_str() {
        "text" => {
            let primary = length_limit(props["inputs"]["primary"].clone(), field, ctx.form_id);
            out.push_str(&format!("<input type=\"text\" {} {}>", attrs(&primary), string(&primary["required"])));
        }
        "textarea" => {
            let mut primary = props["inputs"]["primary"].clone();
            let mut value = String::new();
            if let Some(v) = primary["attr"].get("value") {
                value = rustpress_content::sanitize::esc_textarea(&html_entity_decode(&string(v))).replace('[', "&#091;").replace(']', "&#093;");
                if let Value::Object(attr) = &mut primary["attr"] {
                    attr.shift_remove("value");
                }
            }
            let primary = length_limit(primary, field, ctx.form_id);
            out.push_str(&format!("<textarea {} {}>{}</textarea>", attrs(&primary), string(&primary["required"]), value));
        }
        "email" => email_display(out, field),
        "name" => name_display(out, field),
        "number" => {
            let primary = &props["inputs"]["primary"];
            out.push_str(&format!("<input type=\"number\" {} {}>", attrs(primary), esc_attr(&string(&primary["required"]))));
        }
        "number-slider" => slider_display(out, field, ctx),
        "select" => select_display(out, field, ctx),
        "radio" | "checkbox" => choices_display(out, field, ctx),
        "gdpr-checkbox" => gdpr_display(out, field, ctx),
        _ => {}
    }
}

fn email_display(out: &mut String, field: &Value) {
    let props = &field["properties"];
    let mut primary = props["inputs"]["primary"].clone();
    let mut secondary = props["inputs"].get("secondary").filter(|s| !empty(Some(s))).cloned();
    if set(field, "disable_suggestions") {
        php::entry(&mut primary, "data")["disable-suggestions"] = json!(true);
        if let Some(s) = secondary.as_mut() {
            php::entry(s, "data")["disable-suggestions"] = json!(true);
        }
    }
    if !set(field, "confirmation") {
        out.push_str(&format!("<input type=\"email\" {} {}>", attrs(&primary), esc_attr(&string(&primary["required"]))));
        display_error(out, "primary", field);
        return;
    }
    let secondary = secondary.unwrap_or(Value::Null);
    out.push_str(&format!("<div class=\"wpforms-field-row wpforms-field-{}\">", sanitize_html_class(&str_of(field, "size"))));
    for (key, input) in [("primary", &primary), ("secondary", &secondary)] {
        out.push_str(&format!("<div {}>", html_attributes("", &input["block"], &Value::Null, &Value::Null)));
        sublabel(out, key, "before", field);
        out.push_str(&format!("<input type=\"email\" {} {}>", attrs(input), esc_attr(&string(&input["required"]))));
        sublabel(out, key, "after", field);
        display_error(out, key, field);
        out.push_str("</div>");
    }
    out.push_str("</div>");
}

fn name_display(out: &mut String, field: &Value) {
    let props = &field["properties"];
    let format = if set(field, "format") { esc_attr(&str_of(field, "format")) } else { "first-last".to_owned() };
    if format == "simple" {
        let primary = &props["inputs"]["primary"];
        out.push_str(&format!("<input type=\"text\" {} {}>", attrs(primary), esc_attr(&string(&primary["required"]))));
        return;
    }
    out.push_str(&format!("<div class=\"wpforms-field-row wpforms-field-{}\">", sanitize_html_class(&str_of(field, "size"))));
    let keys: &[&str] = if format == "first-middle-last" { &["first", "middle", "last"] } else { &["first", "last"] };
    for key in keys {
        let input = &props["inputs"][*key];
        out.push_str(&format!("<div {}>", html_attributes("", &input["block"], &Value::Null, &Value::Null)));
        sublabel(out, key, "before", field);
        out.push_str(&format!("<input type=\"text\" {} {}>", attrs(input), esc_attr(&string(&input["required"]))));
        sublabel(out, key, "after", field);
        display_error(out, key, field);
        out.push_str("</div>");
    }
    out.push_str("</div>");
}

/// `WPForms_Field_Number_Slider::field_display()` with the template `fields/number-slider/frontend`.
fn slider_display(out: &mut String, field: &Value, ctx: &Ctx<'_, '_>) {
    let primary = &field["properties"]["inputs"]["primary"];
    let value_display = if field.get("value_display").is_some_and(|v| !v.is_null()) { esc_attr(&str_of(field, "value_display")) } else { esc_html(&ctx.tr("Selected Value: {value}")) };
    let hint_value = if set(&primary["attr"], "value") { php::to_float(primary["attr"].get("value")) } else { 0.0 };
    let hint = value_display.replace("{value}", &format!("<b>{}</b>", php::float(hint_value)));
    let number = |key: &str, default: f64| if php::is_numeric(field.get(key)) { php::to_float(field.get(key)) } else { default };
    let (min, max, step) = (number("min", 0.0), number("max", 10.0), number("step", 1.0));
    let decode = |s: &str| {
        let mut s = s.to_owned();
        loop {
            let next = html_entity_decode(&s);
            if next == s {
                return s;
            }
            s = next;
        }
    };
    out.push_str(&format!(
        "\n<input\n\ttype=\"range\"\n\t{}\t{}\tmin=\"{}\"\n\tmax=\"{}\"\n\tstep=\"{}\">\n\n<div class=\"wpforms-field-number-slider-hint\"\n\tdata-hint=\"{}\">\n\t{}</div>\n",
        attrs(primary),
        if set(primary, "required") { "required" } else { "" },
        esc_attr(&php::float(min)),
        esc_attr(&php::float(max)),
        esc_attr(&php::float(step)),
        esc_attr(&kses_post(&decode(&value_display))),
        kses_post(&decode(&hint))
    ));
}

/// `WPForms_Field::get_choices_label()`.
fn choices_label(label: &str, key: &str, field: &Value, ctx: &Ctx<'_, '_>) -> String {
    let label = label.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{b}']);
    if (set(field, "choices_icons") || set(field, "choices_images")) && label.is_empty() {
        return String::new();
    }
    if label.is_empty() { ctx.tr("Choice %d").replace("%d", &php::leading_int(key).to_string()) } else { label.to_owned() }
}

fn select_display(out: &mut String, field: &Value, ctx: &Ctx<'_, '_>) {
    let props = &field["properties"];
    let mut container = props["input_container"].clone();
    let placeholder = if set(field, "placeholder") { str_of(field, "placeholder") } else { String::new() };
    let multiple = set(field, "multiple");
    let modern = str_of(field, "style") == "modern" && set(field, "style");
    let choices: Vec<(String, Value)> = props["inputs"].as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
    if choices.is_empty() {
        return;
    }
    if container["class"].as_array().is_some_and(|c| c.iter().any(|c| c == "wpforms-field-required")) {
        container["attr"]["required"] = json!("required");
    }
    if multiple {
        container["attr"]["multiple"] = json!("multiple");
        if set(&container["attr"], "name") {
            let name = format!("{}[]", str_of(&container["attr"], "name"));
            container["attr"]["name"] = json!(name);
        }
    }
    if modern {
        php::push(&mut container["class"], "choicesjs-select");
        if set(field, "size") {
            container["data"]["size-class"] = json!(format!("wpforms-field-row wpforms-field-{}", sanitize_html_class(&str_of(field, "size"))));
        }
        container["data"]["search-enabled"] = json!(choices.len() >= 8);
    }
    let has_default = choices.iter().any(|(_, c)| !empty(c.get("default")));
    out.push_str(&format!("<select {}>", attrs(&container)));
    if !placeholder.is_empty() || modern {
        out.push_str(&format!("<option value=\"\" class=\"placeholder\" disabled {}>{}</option>", php::helper("", if has_default || multiple { "1" } else { "" }, "selected"), esc_html(&placeholder)));
    }
    for (key, choice) in &choices {
        let label = choices_label(&string(&choice["label"]["text"]), key, field, ctx);
        let value = match choice["attr"].get("value") {
            Some(v) if !v.is_null() && v.as_str() != Some("") => string(v),
            _ => label.clone(),
        };
        let data = choice["container"].get("data").cloned().unwrap_or(Value::Null);
        let data_html = if empty(Some(&data)) { String::new() } else { html_attributes("", &Value::Null, &data, &Value::Null) };
        let selected_html = if choice["attr"].get("selected").is_some_and(|v| !empty(Some(v))) { " selected=\"selected\"" } else { "" };
        static SPAN: std::sync::OnceLock<rustpress_core::kses::Allowed> = std::sync::OnceLock::new();
        let span = SPAN.get_or_init(|| rustpress_core::kses::Allowed::tags(&[("span", &["class"])]));
        out.push_str(&format!(
            "<option value=\"{}\" {} class=\"{}\" {} {}>{}</option>",
            esc_attr(&value),
            php::helper("1", if empty(choice.get("default")) { "" } else { "1" }, "selected"),
            esc_attr(&php::sanitize_classes(&choice["container"]["class"])),
            data_html,
            selected_html,
            rustpress_core::kses::kses(&label, span)
        ));
    }
    out.push_str("</select>");
}

fn choices_display(out: &mut String, field: &Value, ctx: &Ctx<'_, '_>) {
    let checkbox = str_of(field, "type") == "checkbox";
    let props = &field["properties"];
    let images = !set(field, "dynamic_choices") && !set(field, "choices_icons") && set(field, "choices_images");
    let container = &props["input_container"];
    let choices: Vec<(String, Value)> = props["inputs"].as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
    if choices.is_empty() {
        return;
    }
    out.push_str(&format!("<ul {}>", attrs(container)));
    let mut other_html = String::new();
    let checked = |c: &Value| php::helper("1", if c["default"].as_bool() == Some(true) { "1" } else { "" }, "checked");
    for (key, mut choice) in choices {
        let label = choices_label(&string(&choice["label"]["text"]), &key, field, ctx);
        let required = if checkbox && set(field, "disclaimer_format") && set(&choice, "required") && set(field, "label_hide") { "<span class=\"wpforms-required-label\">*</span>".to_owned() } else { String::new() };
        let required = if required.is_empty() { required } else { format!(" {required}") };
        let c = &choice["container"];
        out.push_str(&format!("<li {}>", html_attributes(&string(&c["id"]), &c["class"], &c["data"], &c["attr"])));
        let ty = if checkbox { "checkbox" } else { "radio" };
        if images {
            choice["label"]["attr"]["tabindex"] = json!(0);
            if label.is_empty() {
                php::push(&mut choice["label"]["class"], "wpforms-field-label-inline-empty");
            }
            let l = &choice["label"];
            out.push_str(&format!("<label {}>", html_attributes(&string(&l["id"]), &l["class"], &l["data"], &l["attr"])));
            out.push_str("<span class=\"wpforms-image-choices-image\">");
            if set(&choice, "image") {
                out.push_str(&format!("<img src=\"{}\" alt=\"{}\"{}>", esc_url(&str_of(&choice, "image")), esc_attr(&label), if label.is_empty() { String::new() } else { format!(" title=\"{}\"", esc_attr(&label)) }));
            }
            out.push_str("</span>");
            if str_of(field, "choices_images_style") == "none" {
                out.push_str("<br>");
            }
            choice["attr"]["tabindex"] = json!("-1");
            out.push_str(&format!("<input type=\"{ty}\" {} {} {}>", attrs(&choice), esc_attr(&string(&choice["required"])), checked(&choice)));
            let text = if checkbox { label.clone() } else { string(&choice["label"]["text"]) };
            out.push_str(&format!("<span class=\"wpforms-image-choices-label\">{}</span>", kses_post(&text)));
            out.push_str("</label>");
        } else {
            out.push_str(&format!("<input type=\"{ty}\" {} {} {}>", attrs(&choice), esc_attr(&string(&choice["required"])), checked(&choice)));
            let l = &choice["label"];
            let required = if required.is_empty() { String::new() } else { rustpress_core::kses::kses(&required, &rustpress_core::kses::Allowed::tags(&[("span", &["class"])])) };
            out.push_str(&format!("<label {}>{}{}</label>", html_attributes(&string(&l["id"]), &l["class"], &l["data"], &l["attr"]), kses_post(&label), required));
        }
        // The input of the "Other" choice, printed under the list. A checkbox keeps only the last choice's.
        let html = other_input(field, &choice, &key, ctx);
        if checkbox || !html.is_empty() {
            other_html = html;
        }
        out.push_str("</li>");
    }
    out.push_str("</ul>");
    out.push_str(&other_html);
}

/// The text input of the "Other" choice (`render_other_input()` of the checkbox, inline in the radio).
fn other_input(field: &Value, choice: &Value, key: &str, ctx: &Ctx<'_, '_>) -> String {
    if !set(&choice["data"], "other-choice") {
        return String::new();
    }
    let size = if set(field, "other_size") { sanitize_html_class(&str_of(field, "other_size")) } else { "medium".to_owned() };
    let hidden = if set(choice, "default") { "" } else { " wpforms-hidden" };
    let original = field["choices"].get(key).and_then(|c| c.get("value"));
    let default = match original {
        Some(v) if !v.is_null() && v.as_str() != Some("") => v.clone(),
        _ => json!(""),
    };
    let mut atts = json!({"name": format!("wpforms[fields][{}][other]", str_of(field, "id")), "value": default});
    if !set(choice, "default") {
        atts["disabled"] = json!("disabled");
    }
    if set(field, "other_placeholder") {
        atts["placeholder"] = field["other_placeholder"].clone();
    }
    let required = !set(field, "read_only");
    format!(
        "<input type=\"text\" {}{}>",
        html_attributes(&format!("wpforms-{}-field_{}_other", ctx.form_id, str_of(field, "id")), &json!(["wpforms-other-input", if required { "wpforms-field-required" } else { "" }, format!("wpforms-field-{size}"), hidden]), &Value::Null, &atts),
        if required { " required" } else { "" }
    )
}

fn gdpr_display(out: &mut String, field: &Value, ctx: &Ctx<'_, '_>) {
    let props = &field["properties"];
    let container = &props["input_container"];
    out.push_str(&format!("<ul {}>", attrs(container)));
    if let Some(inputs) = props["inputs"].as_object() {
        for choice in inputs.values() {
            let required = if set(choice, "required") && set(field, "label_hide") { required_label(ctx.modern) } else { "" };
            let c = &choice["container"];
            out.push_str(&format!("<li {}>", html_attributes(&string(&c["id"]), &c["class"], &c["data"], &c["attr"])));
            out.push_str(&format!("<input type=\"checkbox\" {} {} {}>", attrs(choice), esc_attr(&string(&choice["required"])), php::helper("1", &string(&choice["default"]), "checked")));
            let l = &choice["label"];
            out.push_str(&format!("<label {}>{}{}</label>", html_attributes(&string(&l["id"]), &l["class"], &l["data"], &l["attr"]), kses_post(&string(&l["text"])), kses_post(required)));
            out.push_str("</li>");
        }
    }
    out.push_str("</ul>");
}
