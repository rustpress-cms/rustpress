//! PHP semantics on unserialized values: `empty()`, the casts, `wp_parse_args()`, loose `==`.

use rustpress_core::php_lit::Lit;
use rustpress_phpser::{Key, Value};

/// `! empty( $value )`.
pub fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(i)) => *i != 0,
        Some(Value::Float(f)) => *f != 0.0,
        Some(Value::Str(s)) => !s.is_empty() && s != b"0",
        Some(Value::Array(items)) => !items.is_empty(),
        Some(_) => true,
    }
}

/// `(string) $value`.
pub fn strval(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => String::new(),
        Some(Value::Bool(true)) => "1".to_owned(),
        Some(Value::Int(i)) => i.to_string(),
        Some(Value::Float(f)) => float_string(*f),
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Array(_)) => "Array".to_owned(),
        Some(_) => String::new(),
    }
}

/// A float as PHP 8 prints it (`serialize_precision` -1).
pub fn float_string(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 { format!("{}", f as i64) } else { format!("{f}") }
}

/// `(int) $value`.
pub fn intval(value: Option<&Value>) -> i64 {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => 0,
        Some(Value::Bool(true)) => 1,
        Some(Value::Int(i)) => *i,
        Some(Value::Float(f)) => *f as i64,
        Some(Value::Str(s)) => crate::redirection::php::intval(&String::from_utf8_lossy(s)),
        Some(Value::Array(items)) => i64::from(!items.is_empty()),
        Some(_) => 1,
    }
}

/// `is_numeric( $value )`.
pub fn is_numeric(value: &Value) -> bool {
    match value {
        Value::Int(_) | Value::Float(_) => true,
        Value::Str(s) => crate::redirection::php::is_numeric(&String::from_utf8_lossy(s)),
        _ => false,
    }
}

/// `$value + 0` of a numeric value.
pub fn numval(value: &Value) -> Value {
    match value {
        Value::Str(s) => {
            let text = String::from_utf8_lossy(s);
            let t = text.trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
            let integral = !t.contains(['.', 'e', 'E']);
            match t.parse::<i64>() {
                Ok(i) if integral => Value::Int(i),
                _ => t.parse::<f64>().map_or_else(|_| value.clone(), Value::Float),
            }
        }
        other => other.clone(),
    }
}

pub fn key(name: &str) -> Key {
    Key::Str(name.as_bytes().to_vec())
}

pub fn string(text: &str) -> Value {
    Value::Str(text.as_bytes().to_vec())
}

/// `$array[ $name ] = $value`: in place when the key exists, appended otherwise.
/// `$items[ $key ] = $value` with a key as it is.
pub fn set_key(items: &mut Vec<(Key, Value)>, k: Key, value: Value) {
    match items.iter_mut().find(|(existing, _)| key_eq(existing, &k)) {
        Some(slot) => slot.1 = value,
        None => items.push((k, value)),
    }
}

pub fn set(items: &mut Vec<(Key, Value)>, name: &str, value: Value) {
    let k = key(name);
    match items.iter_mut().find(|(existing, _)| key_eq(existing, &k)) {
        Some(slot) => slot.1 = value,
        None => items.push((k, value)),
    }
}

/// Keys compare like PHP array keys: `"5"` is the integer key 5.
pub fn key_eq(a: &Key, b: &Key) -> bool {
    normalize(a) == normalize(b)
}

fn normalize(k: &Key) -> Key {
    match k {
        Key::Str(s) => match std::str::from_utf8(s).ok().and_then(|t| t.parse::<i64>().ok().filter(|i| i.to_string() == t)) {
            Some(i) => Key::Int(i),
            None => k.clone(),
        },
        Key::Int(_) => k.clone(),
    }
}

/// `wp_unslash()` (`stripslashes_deep()`) of every string: ACF unslashes what it saves, which only changes values that
/// come from the database rather than from a (slashed) form.
pub fn wp_unslash(value: Value) -> Value {
    match value {
        Value::Str(s) => {
            let mut out = Vec::with_capacity(s.len());
            let mut bytes = s.into_iter();
            while let Some(b) = bytes.next() {
                if b != b'\\' {
                    out.push(b);
                    continue;
                }
                match bytes.next() {
                    Some(b'0') => out.push(0),
                    Some(next) => out.push(next),
                    None => {}
                }
            }
            Value::Str(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(|(k, v)| (k, wp_unslash(v))).collect()),
        other => other,
    }
}

/// `wp_parse_args( $args, $defaults )`: the defaults with the given values over them, keys of the defaults first.
pub fn parse_args(args: &Value, defaults: Value) -> Value {
    let Value::Array(mut out) = defaults else { return args.clone() };
    if let Value::Array(items) = args {
        for (k, v) in items {
            match out.iter_mut().find(|(existing, _)| key_eq(existing, k)) {
                Some(slot) => slot.1 = v.clone(),
                None => out.push((k.clone(), v.clone())),
            }
        }
    }
    Value::Array(out)
}

/// `$a == $b` for the scalars location rules compare (PHP 8).
pub fn loose_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), other) | (other, Value::Bool(x)) => *x == truthy(Some(other)),
        (Value::Null, Value::Str(s)) | (Value::Str(s), Value::Null) => s.is_empty(),
        (Value::Null, other) | (other, Value::Null) => !truthy(Some(other)),
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => as_f64(a) == as_f64(b),
        (Value::Int(_) | Value::Float(_), Value::Str(s)) | (Value::Str(s), Value::Int(_) | Value::Float(_)) => {
            let number = if matches!(a, Value::Str(_)) { b } else { a };
            if is_numeric(&Value::Str(s.clone())) {
                as_f64(number) == as_f64(&numval(&Value::Str(s.clone())))
            } else {
                // PHP 8 compares the number as a string.
                strval(Some(number)).as_bytes() == s.as_slice()
            }
        }
        (Value::Str(x), Value::Str(y)) => {
            let (vx, vy) = (Value::Str(x.clone()), Value::Str(y.clone()));
            if is_numeric(&vx) && is_numeric(&vy) { as_f64(&numval(&vx)) == as_f64(&numval(&vy)) } else { x == y }
        }
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.iter().any(|(k2, v2)| key_eq(k, k2) && loose_eq(v, v2))),
        _ => false,
    }
}

fn as_f64(value: &Value) -> f64 {
    match value {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

/// A PHP value as a literal of `register_*()` arguments.
pub fn to_lit(value: &Value) -> Lit {
    match value {
        Value::Null => Lit::Null,
        Value::Bool(b) => Lit::Bool(*b),
        Value::Int(i) => Lit::Int(*i),
        Value::Float(f) => Lit::Float(*f),
        Value::Str(s) => Lit::Str(String::from_utf8_lossy(s).into_owned()),
        Value::Array(items) => Lit::Array(
            items
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        Key::Int(i) => Lit::Int(*i),
                        Key::Str(s) => Lit::Str(String::from_utf8_lossy(s).into_owned()),
                    };
                    (Some(key), to_lit(v))
                })
                .collect(),
        ),
        _ => Lit::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loose_comparisons() {
        assert!(loose_eq(&Value::Int(5), &string("5")));
        assert!(!loose_eq(&Value::Int(0), &string("")));
        assert!(loose_eq(&Value::Int(0), &string("0")));
        assert!(!loose_eq(&Value::Int(0), &string("abc")));
        assert!(loose_eq(&string("1e1"), &string("10")));
        assert!(loose_eq(&Value::Bool(false), &string("")));
        assert!(loose_eq(&Value::Null, &string("")));
    }

    #[test]
    fn numbers() {
        assert_eq!(numval(&string("12")), Value::Int(12));
        assert_eq!(numval(&string("1.5")), Value::Float(1.5));
        assert_eq!(numval(&string(" 7")), Value::Int(7));
    }
}
