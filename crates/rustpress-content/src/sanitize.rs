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

/// `sanitize_file_name( $filename )` up to the extensions, which `mark_inner_extensions()` looks at once the allowed
/// types are known. The extension is lowercased, as `wp_unique_filename()` does afterwards.
pub fn file_name(name: &str, locale: &str) -> String {
    const SPECIAL: [char; 33] = [
        '?', '[', ']', '/', '\\', '=', '<', '>', ':', ';', ',', '\'', '"', '&', '$', '#', '*', '(', ')', '|', '~', '`', '!', '{', '}', '%', '+', '\u{2019}', '\u{ab}', '\u{bb}', '\u{201d}', '\u{201c}', '\0',
    ];
    // `\p{Zs}` becomes a space.
    let is_space = |c: char| matches!(c, '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}');
    let plain = rustpress_core::slug::remove_accents(name, locale);
    let cleaned: String = plain.chars().map(|c| if is_space(c) { ' ' } else { c }).filter(|c| !SPECIAL.contains(c)).collect();
    // `/\.{2,}/` and `/[\r\n\t -]+/`.
    let mut collapsed = String::with_capacity(cleaned.len());
    let mut last = None;
    for c in cleaned.chars() {
        let class = match c {
            '.' => Some('.'),
            '\r' | '\n' | '\t' | ' ' | '-' => Some('-'),
            _ => None,
        };
        if class.is_none() || class != last {
            collapsed.push(class.unwrap_or(c));
        }
        last = class;
    }
    let collapsed = collapsed.trim_matches(['.', '-', '_']).to_owned();
    if !collapsed.contains('.') {
        // A file called `jpg` would pass for a file without a name.
        if let Some((ext, _)) = crate::media::check_filetype(&format!("test.{collapsed}")).filter(|(ext, _)| *ext == collapsed) {
            return format!("unnamed-file.{ext}");
        }
    }
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
    fn sanitize_file_name_like_wordpress() {
        for (name, expected) in [
            ("Gr\u{fc}\u{df}e aus K\u{f6}ln.JPG", "Gruse-aus-Koln.jpg"),
            ("a b  c.png", "a-b-c.png"),
            ("a--b__c.png", "a-b__c.png"),
            ("..a..b...png", "a.b.png"),
            ("-_.x._-", "x"),
            ("jpg", "unnamed-file.jpg"),
            ("x", "x"),
            ("tab\tname.png", "tab-name.png"),
            ("nbsp\u{a0}x.png", "nbsp-x.png"),
            ("a+b%20c.png", "ab20c.png"),
            ("quote'\"&.png", "quote.png"),
            ("\u{ab}x\u{bb}\u{201c}y\u{201d}\u{2019}.png", "xy.png"),
            ("\u{c6}\u{d8}\u{c5} \u{df}.png", "AEOA-s.png"),
            ("file.tar.gz", "file.tar.gz"),
        ] {
            assert_eq!(file_name(name, "en_US"), expected, "{name}");
        }
        // German spells the umlauts out.
        assert_eq!(file_name("Gr\u{fc}\u{df}e.png", "de_DE"), "Gruesse.png");
    }

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
