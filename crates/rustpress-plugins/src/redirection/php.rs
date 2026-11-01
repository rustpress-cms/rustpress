//! The PHP functions Redirection builds its URL handling on: `parse_str()`, `http_build_query()`, the URL codings.

/// A query parameter as `parse_str()` leaves it, plus the `null` Redirection uses for a parameter without `=`.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Null,
    Str(String),
    Arr(Params),
}

/// An ordered PHP array of parameters (keys as strings; PHP's integer keys compare the same way).
pub type Params = Vec<(String, Param)>;

pub fn get<'a>(params: &'a Params, key: &str) -> Option<&'a Param> {
    params.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `$array[ $key ] = $value`: in place when the key exists, appended otherwise.
pub fn set(params: &mut Params, key: &str, value: Param) {
    match params.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => slot.1 = value,
        None => params.push((key.to_owned(), value)),
    }
}

pub fn unset(params: &mut Params, key: &str) {
    params.retain(|(k, _)| k != key);
}

/// `urldecode()`.
pub fn urldecode(text: &str) -> String {
    String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(text.as_bytes())).into_owned()
}

/// `rawurldecode()`: `%XX` only, `+` stays.
pub fn rawurldecode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(hi << 4 | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

/// `rawurlencode()`.
pub fn rawurlencode(text: &str) -> String {
    rustpress_core::canonical::rawurlencode(text)
}

/// `urlencode()`: like `rawurlencode()`, but a space becomes `+` and `~` is encoded.
pub fn urlencode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `parse_str()` (`php_default_treat_data()` and `php_register_variable_ex()`): names lose leading spaces, spaces
/// and dots before the first `[` become `_`, `a[b][]` builds arrays.
pub fn parse_str(query: &str) -> Params {
    let mut params = Params::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = match pair.split_once('=') {
            Some((name, value)) => (urldecode(name), urldecode(value)),
            None => (urldecode(pair), String::new()),
        };
        register(&mut params, &name, value);
    }
    params
}

fn register(params: &mut Params, name: &str, value: String) {
    let name = name.trim_start_matches(' ');
    let mut base = String::new();
    let mut rest = None;
    for (i, c) in name.char_indices() {
        match c {
            ' ' | '.' => base.push('_'),
            '[' => {
                rest = Some(&name[i..]);
                break;
            }
            c => base.push(c),
        }
    }
    if base.is_empty() {
        return;
    }
    // The indices: `[a][b][]`; a `[` without `]` makes the rest part of the plain name.
    let mut indices: Vec<String> = Vec::new();
    if let Some(mut tail) = rest {
        while let Some(inner) = tail.strip_prefix('[') {
            match inner.find(']') {
                Some(end) => {
                    indices.push(inner[..end].to_owned());
                    tail = &inner[end + 1..];
                }
                None => {
                    if indices.is_empty() {
                        base.push('_');
                        base.push_str(inner);
                    }
                    break;
                }
            }
        }
    }
    insert(params, &base, &indices, value);
}

fn next_index(params: &Params) -> String {
    let next = params.iter().filter_map(|(k, _)| k.parse::<i64>().ok()).max().map_or(0, |m| m + 1);
    next.max(0).to_string()
}

fn insert(params: &mut Params, key: &str, indices: &[String], value: String) {
    let key = if key.is_empty() { next_index(params) } else { key.to_owned() };
    let Some((first, rest)) = indices.split_first() else {
        set(params, &key, Param::Str(value));
        return;
    };
    if !matches!(get(params, &key), Some(Param::Arr(_))) {
        set(params, &key, Param::Arr(Params::new()));
    }
    if let Some((_, Param::Arr(child))) = params.iter_mut().find(|(k, _)| *k == key) {
        insert(child, first, rest, value);
    }
}

/// `http_build_query()` (`PHP_QUERY_RFC1738`): `null` values are left out.
pub fn http_build_query(params: &Params) -> String {
    let mut parts = Vec::new();
    build(&mut parts, params, None);
    parts.join("&")
}

fn build(parts: &mut Vec<String>, params: &Params, prefix: Option<&str>) {
    for (key, value) in params {
        let name = match prefix {
            Some(prefix) => format!("{prefix}%5B{}%5D", urlencode(key)),
            None => urlencode(key),
        };
        match value {
            Param::Null => {}
            Param::Str(s) => parts.push(format!("{name}={}", urlencode(s))),
            Param::Arr(child) => build(parts, child, Some(&name)),
        }
    }
}

/// `is_numeric()`.
pub fn is_numeric(text: &str) -> bool {
    let t = text.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']).trim_end_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let t = t.strip_prefix(['+', '-']).unwrap_or(t);
    let (mantissa, exponent) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], Some(&t[i + 1..])),
        None => (t, None),
    };
    let (int, frac) = match mantissa.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (mantissa, None),
    };
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = digits(int) && frac.is_none_or(digits) && (!int.is_empty() || frac.is_some_and(|f| !f.is_empty()));
    let exponent_ok = exponent.is_none_or(|e| {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        !e.is_empty() && digits(e)
    });
    mantissa_ok && exponent_ok
}

/// `intval( $text, 10 )`.
pub fn intval(text: &str) -> i64 {
    let t = text.trim_start();
    let (sign, digits) = match t.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, t.strip_prefix('+').unwrap_or(t)),
    };
    let digits: String = digits.chars().take_while(char::is_ascii_digit).collect();
    sign * digits.parse::<i64>().unwrap_or(0)
}

/// `ucwords()`: the first letter of every word after a space, tab or newline.
pub fn ucwords(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut start = true;
    for c in text.chars() {
        out.push(if start { c.to_ascii_uppercase() } else { c });
        start = matches!(c, ' ' | '\t' | '\r' | '\n' | '\x0b' | '\x0c');
    }
    out
}

/// `mb_strtolower( $text, 'UTF-8' )`.
pub fn to_lower(text: &str) -> String {
    text.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Param {
        Param::Str(v.to_owned())
    }

    #[test]
    fn parse_str_builds_arrays_and_renames() {
        assert_eq!(parse_str("x=1&y[]=2&y[]=3"), vec![("x".into(), s("1")), ("y".into(), Param::Arr(vec![("0".into(), s("2")), ("1".into(), s("3"))]))]);
        assert_eq!(parse_str("a.b=1& c d=2&e[x=3"), vec![("a_b".into(), s("1")), ("c_d".into(), s("2")), ("e_x".into(), s("3"))]);
        assert_eq!(parse_str("a[b][c]=1&a[b][d]=2"), vec![("a".into(), Param::Arr(vec![("b".into(), Param::Arr(vec![("c".into(), s("1")), ("d".into(), s("2"))]))]))]);
        assert_eq!(parse_str("k&v=a+b%20c"), vec![("k".into(), s("")), ("v".into(), s("a b c"))]);
    }

    #[test]
    fn build_query() {
        let params = vec![("a".into(), s("1 2")), ("n".into(), Param::Null), ("y".into(), Param::Arr(vec![("0".into(), s("~"))]))];
        assert_eq!(http_build_query(&params), "a=1+2&y%5B0%5D=%7E");
    }

    #[test]
    fn numeric() {
        assert!(is_numeric("1") && is_numeric(" 12") && is_numeric("1.5") && is_numeric(".5") && is_numeric("1e3") && is_numeric("-2"));
        assert!(!is_numeric("") && !is_numeric("1a") && !is_numeric(".") && !is_numeric("1e") && !is_numeric("abc"));
    }
}
