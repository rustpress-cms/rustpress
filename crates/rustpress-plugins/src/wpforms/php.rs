//! The PHP semantics the plugin's form data needs: the form data is the array `json_decode( $json, true )` gives,
//! kept as a [`serde_json::Value`] with its key order.

use rustpress_core::formatting::{esc_attr, sanitize_html_class};
use serde_json::{Map, Value};

/// `empty( $value )`.
pub fn empty(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => !b,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty() || s == "0",
        Some(Value::Array(a)) => a.is_empty(),
        Some(Value::Object(o)) => o.is_empty(),
    }
}

/// `! empty( $array[ $key ] )`.
pub fn set(value: &Value, key: &str) -> bool {
    !empty(value.get(key))
}

/// `(string) $value` (an array is `Array`).
pub fn string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => if *b { "1".into() } else { String::new() },
        Value::Number(n) => match n.as_i64() {
            Some(i) => i.to_string(),
            None => n.as_u64().map_or_else(|| float(n.as_f64().unwrap_or(0.0)), |u| u.to_string()),
        },
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => "Array".into(),
    }
}

/// The string of `$array[ $key ]`, `''` when it is missing.
pub fn str_of(value: &Value, key: &str) -> String {
    value.get(key).map(string).unwrap_or_default()
}

/// A float as `echo` writes it (`precision` 14, `%.14G`).
pub fn float(f: f64) -> String {
    if f.is_nan() {
        return "NAN".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "INF".into() } else { "-INF".into() };
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let exponent = f.abs().log10().floor() as i32;
    let exponent = {
        // The exponent after rounding to 14 significant digits.
        let rounded: f64 = format!("{:.13e}", f).parse().unwrap_or(f);
        if rounded.abs() >= 10f64.powi(exponent + 1) { exponent + 1 } else { exponent }
    };
    if (-4..14).contains(&exponent) {
        let decimals = (13 - exponent).max(0) as usize;
        let s = format!("{f:.decimals$}");
        let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_owned() } else { s };
        return s;
    }
    let s = format!("{:.13e}", f);
    let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let mantissa = if mantissa.contains('.') { mantissa.trim_end_matches('0').trim_end_matches('.') } else { mantissa };
    let exp: i32 = exp.parse().unwrap_or(0);
    let mantissa = if mantissa.contains('.') { mantissa.to_owned() } else { format!("{mantissa}.0") };
    format!("{mantissa}E{}{}", if exp < 0 { '-' } else { '+' }, exp.abs())
}

/// `is_numeric( $value )`.
pub fn is_numeric(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Number(_)) => true,
        Some(Value::String(s)) => numeric_str(s).is_some(),
        _ => false,
    }
}

/// A numeric string as PHP reads it (leading whitespace, trailing whitespace allowed).
pub fn numeric_str(s: &str) -> Option<f64> {
    let t = s.trim_start_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']).trim_end_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']);
    if t.is_empty() || t.len() != t.trim().len() && t.trim().is_empty() {
        return None;
    }
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let mut digits = i - digits_start;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let frac = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        digits += i - frac;
    }
    if digits == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let exp = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp {
            i = j;
        }
    }
    if i != b.len() {
        return None;
    }
    t.parse().ok()
}

/// `(float) $value`.
pub fn to_float(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::Bool(true)) => 1.0,
        Some(Value::String(s)) => leading_float(s),
        _ => 0.0,
    }
}

/// `(float) $string`: the leading number.
pub fn leading_float(s: &str) -> f64 {
    let t = s.trim_start_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']);
    let b = t.as_bytes();
    let mut end = 0;
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let mut seen = false;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        seen = true;
        end = i;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            seen = true;
            end = i;
        }
    }
    if seen && i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let e = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > e {
            end = j;
        }
    }
    if !seen {
        return 0.0;
    }
    t[..end].parse().unwrap_or(0.0)
}

/// `absint( $value )` / `(int) $value` for the IDs of the form data.
pub fn int(value: Option<&Value>) -> i64 {
    match value {
        Some(Value::Number(n)) => n.as_i64().unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
        Some(Value::Bool(true)) => 1,
        Some(Value::String(s)) => leading_int(s),
        _ => 0,
    }
}

/// `(int) $string`.
pub fn leading_int(s: &str) -> i64 {
    let t = s.trim_start_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']);
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        // A float string such as ".5" or "1e3" is cast through its float.
        return leading_float(t) as i64;
    }
    let mut n: i64 = digits.parse().unwrap_or(i64::MAX);
    let after = &rest[digits.len()..];
    if after.starts_with(['.', 'e', 'E']) {
        n = leading_float(t).abs() as i64;
    }
    if neg { -n } else { n }
}

/// `wpforms_sanitize_classes( $classes, true )` of an array: the unique, non-empty classes, sanitized.
pub fn sanitize_classes(classes: &Value) -> String {
    match classes {
        Value::Array(list) => unique_classes(list.iter().map(string)),
        Value::Object(map) => unique_classes(map.values().map(string)),
        Value::String(s) if !s.is_empty() => unique_classes(s.trim().split(' ').map(str::to_owned)),
        _ => String::new(),
    }
}

/// `wpforms_sanitize_classes( $string )`: the string's classes, sanitized.
pub fn sanitize_class_string(classes: &str) -> String {
    if classes.is_empty() {
        return String::new();
    }
    unique_classes(classes.trim().split(' ').map(str::to_owned))
}

/// `array_unique()`, then the non-empty ones through `sanitize_html_class()`.
fn unique_classes(classes: impl Iterator<Item = String>) -> String {
    let mut seen: Vec<String> = Vec::new();
    for class in classes {
        if !seen.contains(&class) {
            seen.push(class);
        }
    }
    seen.iter().filter(|c| !c.is_empty() && c.as_str() != "0").map(|c| sanitize_html_class(c)).collect::<Vec<_>>().join(" ")
}

/// `wpforms_html_attributes( $id, $class, $datas, $atts )`.
pub fn html_attributes(id: &str, class: &Value, data: &Value, attr: &Value) -> String {
    let mut parts = Vec::new();
    let id = id.trim();
    if !id.is_empty() && id != "0" {
        let id = sanitize_html_class(id);
        if !id.is_empty() && id != "0" {
            parts.push(format!("id=\"{id}\""));
        }
    }
    if !empty(Some(class)) {
        // A string comes back as an array from `wpforms_sanitize_classes( $string, true )`.
        let class = if class.is_string() { "Array".to_owned() } else { sanitize_classes(class) };
        if !class.is_empty() && class != "0" {
            parts.push(format!("class=\"{class}\""));
        }
    }
    if let Value::Object(data) = data {
        for (name, value) in data {
            parts.push(format!("data-{}=\"{}\"", sanitize_html_class(name), esc_attr(&string(value))));
        }
    }
    if let Value::Object(attr) = attr {
        for (name, value) in attr {
            if string(value) == "0" || !empty(Some(value)) {
                let name = if name.starts_with('[') { format!("[{}]", sanitize_html_class(name.trim_matches(['[', ']']))) } else { sanitize_html_class(name) };
                parts.push(format!("{name}=\"{}\"", esc_attr(&string(value))));
            }
        }
    }
    parts.join(" ").trim().to_owned()
}

/// `$array[ $key ]`, created as an empty array when missing.
pub fn entry<'a>(map: &'a mut Value, key: &str) -> &'a mut Value {
    if !map.is_object() {
        *map = Value::Object(Map::new());
    }
    let Value::Object(m) = map else { unreachable!() };
    m.entry(key.to_owned()).or_insert_with(|| Value::Object(Map::new()))
}

/// `$array[] = $value`.
pub fn push(list: &mut Value, value: impl Into<Value>) {
    match list {
        Value::Array(a) => a.push(value.into()),
        Value::Object(m) => {
            let next = m.keys().filter_map(|k| k.parse::<i64>().ok()).max().map_or(0, |n| n + 1);
            m.insert(next.to_string(), value.into());
        }
        _ => *list = Value::Array(vec![value.into()]),
    }
}

/// `array_merge_recursive( $a, $b )`.
pub fn array_merge_recursive(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Object(mut a), Value::Object(b)) => {
            for (k, v) in b {
                let numeric = k.parse::<i64>().is_ok();
                if numeric {
                    push_object(&mut a, v);
                    continue;
                }
                match a.remove(&k) {
                    Some(existing) => {
                        let merged = merge_value(existing, v);
                        a.insert(k, merged);
                    }
                    None => {
                        a.insert(k, v);
                    }
                }
            }
            Value::Object(a)
        }
        (Value::Array(mut a), Value::Array(b)) => {
            a.extend(b);
            Value::Array(a)
        }
        (Value::Object(mut a), Value::Array(b)) => {
            for v in b {
                push_object(&mut a, v);
            }
            Value::Object(a)
        }
        (Value::Array(a), Value::Object(b)) => {
            let mut m: Map<String, Value> = a.into_iter().enumerate().map(|(i, v)| (i.to_string(), v)).collect();
            for (k, v) in b {
                if k.parse::<i64>().is_ok() {
                    push_object(&mut m, v);
                } else {
                    m.insert(k, v);
                }
            }
            Value::Object(m)
        }
        (_, b) => b,
    }
}

/// Two values under the same string key: both arrays merge, otherwise they become a list of both.
fn merge_value(existing: Value, v: Value) -> Value {
    match (existing, v) {
        (e @ (Value::Object(_) | Value::Array(_)), v @ (Value::Object(_) | Value::Array(_))) => array_merge_recursive(e, v),
        (Value::Array(mut e), v) => {
            e.push(v);
            Value::Array(e)
        }
        (Value::Object(mut e), v) => {
            push_object(&mut e, v);
            Value::Object(e)
        }
        (e, Value::Array(v)) => {
            let mut list = vec![e];
            list.extend(v);
            Value::Array(list)
        }
        (e, Value::Object(v)) => array_merge_recursive(Value::Array(vec![e]), Value::Object(v)),
        (e, v) => Value::Array(vec![e, v]),
    }
}

fn push_object(m: &mut Map<String, Value>, v: Value) {
    let next = m.keys().filter_map(|k| k.parse::<i64>().ok()).max().map_or(0, |n| n + 1);
    m.insert(next.to_string(), v);
}

/// `checked( '1', $value, false )` / `selected( $a, $b, false )`: ` checked='checked'` when both are equal strings.
pub fn helper(a: &str, b: &str, kind: &str) -> String {
    if a == b { format!(" {kind}='{kind}'") } else { String::new() }
}

/// `esc_js()`.
pub fn esc_js(text: &str) -> String {
    let text = rustpress_core::formatting::specialchars_noquotes(text);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => out.push_str("\\'"),
            '"' => out.push_str("&quot;"),
            '\\' => out.push_str("\\\\"),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str("\\n");
            }
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    // `esc_js()` turns `&#x27;`/`&#039;` back into quotes before escaping.
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn floats_print_like_php() {
        assert_eq!(float(1.0), "1");
        assert_eq!(float(10.5), "10.5");
        assert_eq!(float(0.1 + 0.2), "0.3");
        assert_eq!(float(0.25), "0.25");
        assert_eq!(float(1e20), "1.0E+20");
        assert_eq!(float(-3.0), "-3");
        assert_eq!(float(100000.0), "100000");
        assert_eq!(float(0.0001), "0.0001");
        assert_eq!(float(0.00001), "1.0E-5");
    }

    #[test]
    fn numeric_strings() {
        assert!(is_numeric(Some(&json!("1"))));
        assert!(is_numeric(Some(&json!(" 1.5"))));
        assert!(is_numeric(Some(&json!("1e3"))));
        assert!(!is_numeric(Some(&json!("x"))));
        assert!(!is_numeric(Some(&json!(""))));
        assert!(!is_numeric(Some(&json!("1x"))));
        assert_eq!(leading_int("12abc"), 12);
        assert_eq!(leading_int("1.9"), 1);
    }

    #[test]
    fn attributes() {
        let html = html_attributes("my id", &json!(["a", "b", "a", "", "c d"]), &json!({"x": true, "y": 0}), &json!({"name": "n", "value": "", "placeholder": "0", "z": false}));
        assert_eq!(html, "id=\"myid\" class=\"a b cd\" data-x=\"1\" data-y=\"0\" name=\"n\" placeholder=\"0\"");
    }

    #[test]
    fn merge_recursive() {
        let a = json!({"inputs": {"primary": {"class": ["x"], "attr": {"name": "a"}}}});
        let b = json!({"inputs": {"primary": {"class": ["y"], "attr": {"name": "b"}}, "secondary": {"id": "s"}}});
        assert_eq!(array_merge_recursive(a, b), json!({"inputs": {"primary": {"class": ["x", "y"], "attr": {"name": ["a", "b"]}}, "secondary": {"id": "s"}}}));
    }
}
