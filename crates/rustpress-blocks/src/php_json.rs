//! `wp_json_encode()` / `json_encode()` with PHP's default flags, as needed to
//! reproduce hashes that WordPress derives from encoded data (layout
//! container class names are `md5(json_encode(...))`).

use serde_json::Value;

/// Encodes like PHP's `json_encode` with no flags: `/` is escaped, non-ASCII
/// becomes `\uXXXX`, empty objects become `[]` (an empty PHP array).
/// The key of the marker object [empty_object] returns.
const EMPTY_OBJECT_KEY: &str = "\u{0}empty object";

/// A value that is encoded as `{}`; PHP only prints an empty array as `{}` when it was cast to an object.
pub fn empty_object() -> Value {
    let mut map = serde_json::Map::new();
    map.insert(EMPTY_OBJECT_KEY.to_owned(), Value::Null);
    Value::Object(map)
}

pub fn encode(value: &Value) -> String {
    let mut out = String::new();
    write(&mut out, value);
    out
}

fn write(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&number(n)),
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            if map.len() == 1 && map.contains_key(EMPTY_OBJECT_KEY) {
                out.push_str("{}");
                return;
            }
            if map.is_empty() {
                out.push_str("[]");
                return;
            }
            // PHP turns arrays with keys 0..n-1 into lists.
            let is_list = map.keys().enumerate().all(|(i, k)| k.parse::<usize>().ok() == Some(i));
            if is_list {
                out.push('[');
                for (i, item) in map.values().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write(out, item);
                }
                out.push(']');
                return;
            }
            out.push('{');
            for (i, (key, item)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, key);
                out.push(':');
                write(out, item);
            }
            out.push('}');
        }
    }
}

fn number(n: &serde_json::Number) -> String {
    if let Some(f) = n.as_f64().filter(|_| n.is_f64()) {
        // PHP prints floats with the shortest round-trip form, integral ones as `1.0`.
        let s = format!("{f}");
        return if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") { s } else { format!("{s}.0") };
    }
    n.to_string()
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '/' => out.push_str("\\/"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e && (c as u32) < 0x80 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c if (c as u32) < 0x80 => out.push(c),
            c => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// `substr( md5( wp_json_encode( $data ) ), 0, 8 )` prefixed: `wp_unique_id_from_values`.
pub fn unique_id_from_values(data: &Value, prefix: &str) -> String {
    let digest = format!("{:x}", md5::compute(encode(data).as_bytes()));
    format!("{prefix}{}", &digest[..8])
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn matches_php_json_encode() {
        assert_eq!(encode(&json!({"type": "constrained", "a": [1, 2.5, true, null]})), r#"{"type":"constrained","a":[1,2.5,true,null]}"#);
        assert_eq!(encode(&json!("a/b\"c\\d")), r#""a\/b\"c\\d""#);
        let bs = '\\';
        let expected = format!("\"{bs}u00e4{bs}u00f6{bs}u20ac{bs}ud83d{bs}ude00\"");
        assert_eq!(encode(&json!("äö€😀")), expected);
        assert_eq!(encode(&json!({})), "[]");
        assert_eq!(encode(&json!(1.0)), "1.0");
        assert_eq!(encode(&json!("line\nbreak\t")), r#""line\nbreak\t""#);
    }

    #[test]
    fn numeric_keys_become_a_list() {
        assert_eq!(encode(&json!({"0": "a", "1": "b"})), r#"["a","b"]"#);
        assert_eq!(encode(&json!({"1": "a"})), r#"{"1":"a"}"#);
    }

    #[test]
    fn md5_prefix_is_eight_hex_digits() {
        let id = unique_id_from_values(&json!([{"type": "default"}, true, null, false, "0.5em", null]), "wp-container-core-group-is-layout-");
        assert!(id.starts_with("wp-container-core-group-is-layout-"));
        assert_eq!(id.len(), "wp-container-core-group-is-layout-".len() + 8);
    }
}
