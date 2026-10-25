//! The small sanitizing functions of `wp-includes/formatting.php` that the screens need.

use rustpress_core::formatting::wp_strip_all_tags;

/// `sanitize_text_field( $str )`.
pub fn text_field(text: &str) -> String {
    single_line(text, false)
}

/// `sanitize_textarea_field( $str )`.
pub fn textarea_field(text: &str) -> String {
    single_line(text, true)
}

fn single_line(text: &str, keep_newlines: bool) -> String {
    let text = if text.contains('<') {
        // Tags are removed; a stray `<` is encoded (`_sanitize_text_fields` without a tag).
        let stripped = wp_strip_all_tags(&text.replace('<', "<"));
        stripped
    } else {
        text.trim().to_owned()
    };
    // Percent-encoded octets (`%a0`) are dropped.
    let mut cleaned = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && i + 2 < chars.len() + 0 && chars.get(i + 1).is_some_and(char::is_ascii_hexdigit) && chars.get(i + 2).is_some_and(char::is_ascii_hexdigit) {
            i += 3;
            continue;
        }
        cleaned.push(chars[i]);
        i += 1;
    }
    let normalized = cleaned.replace("\r\n", "\n").replace('\r', "\n");
    if keep_newlines {
        normalized.split('\n').map(|line| collapse_spaces(line)).collect::<Vec<_>>().join("\n").trim().to_owned()
    } else {
        collapse_spaces(&normalized.replace('\n', " ")).trim().to_owned()
    }
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
