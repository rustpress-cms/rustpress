//! The small sanitizing functions of `wp-includes/formatting.php` that the screens need.

use rustpress_core::formatting::{php_trim, wp_strip_all_tags};

/// `sanitize_text_field( $str )`.
pub fn text_field(text: &str) -> String {
    single_line(text, false)
}

/// `sanitize_textarea_field( $str )`.
pub fn textarea_field(text: &str) -> String {
    single_line(text, true)
}

/// `_sanitize_text_fields()`.
fn single_line(text: &str, keep_newlines: bool) -> String {
    let mut filtered = text.to_owned();
    if filtered.contains('<') {
        // A `<` that opens no tag stays as `&lt;`; tags go; `<` before a line break is encoded so that removing the
        // line breaks cannot make a tag.
        filtered = wp_strip_all_tags(&rustpress_core::kses::pre_kses_less_than(&filtered));
        filtered = filtered.replace("<\n", "&lt;\n");
    }
    if !keep_newlines {
        // `preg_replace( '/[\r\n\t ]+/', ' ', ... )`
        let mut collapsed = String::with_capacity(filtered.len());
        let mut in_run = false;
        for c in filtered.chars() {
            if matches!(c, '\r' | '\n' | '\t' | ' ') {
                if !in_run {
                    collapsed.push(' ');
                }
                in_run = true;
            } else {
                collapsed.push(c);
                in_run = false;
            }
        }
        filtered = collapsed;
    }
    filtered = php_trim(&filtered).to_owned();
    // Percent-encoded octets go, until none is left.
    let mut found = false;
    while let Some(octet) = percent_octet(&filtered) {
        filtered = filtered.replace(&octet, "");
        found = true;
    }
    if found {
        let mut single = String::with_capacity(filtered.len());
        for c in filtered.chars() {
            if !(c == ' ' && single.ends_with(' ')) {
                single.push(c);
            }
        }
        filtered = php_trim(&single).to_owned();
    }
    filtered
}

/// The first `%` with two hexadecimal digits (`/%[a-f0-9]{2}/i`).
fn percent_octet(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    (0..bytes.len().saturating_sub(2)).find(|&i| bytes[i] == b'%' && bytes[i + 1].is_ascii_hexdigit() && bytes[i + 2].is_ascii_hexdigit()).map(|i| text[i..i + 3].to_owned())
}

fn collapse_spaces(line: &str) -> String {
    line.split([' ', '\t']).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ")
}

/// `sanitize_user( $username, $strict )`.
pub fn user_login(login: &str, strict: bool) -> String {
    let stripped = wp_strip_all_tags(login);
    // Remove percent-encoded octets.
    let mut out = String::new();
    let chars: Vec<char> = stripped.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && chars.get(i + 1).is_some_and(char::is_ascii_hexdigit) && chars.get(i + 2).is_some_and(char::is_ascii_hexdigit) {
            i += 3;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    let out = out.replace("&amp;", "").replace(['<', '>'], "");
    let out: String = if strict { out.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '.' | '-' | '*' | '@')).collect() } else { out };
    collapse_spaces(out.trim())
}

/// `sanitize_key( $key )`.
pub fn key(key: &str) -> String {
    key.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-')).collect()
}

/// `sanitize_file_name( $filename )` for the characters that matter on a file system.
pub fn file_name(name: &str) -> String {
    let special = ['?', '[', ']', '/', '\\', '=', '<', '>', ':', ';', ',', '\'', '"', '&', '$', '#', '*', '(', ')', '|', '~', '`', '!', '{', '}', '%', '+', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{00ab}', '\u{00bb}', '\u{2039}', '\u{203a}', '\u{2014}', '\u{2013}', '\u{00a0}'];
    let mut cleaned: String = name.chars().filter(|c| !special.contains(c) && !c.is_control()).collect();
    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join("-");
    let mut collapsed = String::new();
    for c in cleaned.chars() {
        if c == '-' && collapsed.ends_with('-') {
            continue;
        }
        collapsed.push(c);
    }
    let collapsed = collapsed.trim_matches(['.', '-', '_']).to_owned();
    // Non-ASCII letters are kept; the extension is lowercased like WordPress's mime handling expects.
    match collapsed.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem}.{}", ext.to_lowercase()),
        _ => collapsed,
    }
}

/// `esc_textarea()`.
pub fn esc_textarea(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#039;")
}

/// `wp_generate_password( $length, $special_chars, false )`.
pub fn random_password(length: usize, special: bool) -> String {
    use rand::Rng;
    let mut alphabet = String::from("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
    if special {
        alphabet.push_str("!@#$%^&*()");
    }
    let alphabet: Vec<char> = alphabet.chars().collect();
    let mut rng = rand::rng();
    (0..length).map(|_| alphabet[rng.random_range(0..alphabet.len())]).collect()
}

/// `is_email()`: the shape WordPress accepts.
pub fn is_email(email: &str) -> bool {
    if email.len() < 6 || email.len() > 254 {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else { return false };
    if local.is_empty() || domain.contains('@') {
        return false;
    }
    if !local.chars().all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(c)) {
        return false;
    }
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|p| !p.is_empty() && !p.starts_with('-') && !p.ends_with('-') && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

/// `sanitize_email()`: the cleaned address, or an empty string when nothing valid is left.
pub fn sanitize_email(email: &str) -> String {
    let email = email.trim();
    if email.len() < 6 {
        return String::new();
    }
    let Some((local, domain)) = email.split_once('@') else { return String::new() };
    let local: String = local.chars().filter(|c| c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(*c)).collect();
    if local.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = domain.split('.').map(|p| p.trim_matches('-').chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect::<String>()).filter(|p: &String| !p.is_empty()).collect();
    if parts.len() < 2 {
        return String::new();
    }
    format!("{local}@{}", parts.join("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_text_field_like_wordpress() {
        assert_eq!(text_field("  a <b>bold</b>  text "), "a bold text");
        assert_eq!(text_field("a < b"), "a &lt; b");
        assert_eq!(text_field("x\n\t y"), "x y");
        assert_eq!(text_field("100%25 sure %e2%80%99"), "100 sure");
        assert_eq!(text_field("%2%41"), "%2");
        assert_eq!(text_field("\u{a0}nbsp\u{a0}"), "\u{a0}nbsp\u{a0}");
        assert_eq!(textarea_field("line one\n  line  two "), "line one\n  line  two");
        assert_eq!(text_field("<script>alert(1)</script>ok"), "ok");
    }
}
