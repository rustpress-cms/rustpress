//! Shortcodes (`shortcodes.php`): the scanner, the attribute parser and `strip_shortcodes()`.
//! What a shortcode renders is decided where the pieces it needs live (`rustpress-blocks`).

use std::sync::LazyLock;

use regex::Regex;

/// The shortcodes WordPress registers itself, in registration order. Others come from plugins,
/// which do not run here; their tags stay in the text.
pub const CORE_TAGS: &[&str] = &["wp_playlist", "audio", "video", "embed", "caption", "wp_caption", "gallery"];

/// One match of `get_shortcode_regex()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Byte range of the whole match in the text.
    pub start: usize,
    pub end: usize,
    /// Group 1: the optional second opening bracket of `[[tag]]`.
    pub open_escape: bool,
    /// Group 2.
    pub name: String,
    /// Group 3: the text between the name and the closing bracket.
    pub atts: String,
    /// Group 5: the enclosed content, if the shortcode has a closing tag.
    pub content: Option<String>,
    /// Group 6: the optional second closing bracket.
    pub close_escape: bool,
    /// The end of the match without group 6.
    pub core_end: usize,
}

fn word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Matches the pattern at `at`, where the text has a `[`.
fn match_at(text: &str, at: usize, tags: &[&str]) -> Option<Found> {
    let bytes = text.as_bytes();
    let mut j = at + 1;
    let open_escape = bytes.get(j) == Some(&b'[');
    if open_escape {
        j += 1;
    }
    let rest = &text[j..];
    let name = tags.iter().find(|tag| {
        rest.starts_with(**tag) && !bytes.get(j + tag.len()).is_some_and(|b| word_byte(*b) || *b == b'-')
    })?;
    let name_end = j + name.len();
    let mut k = name_end;
    loop {
        match bytes.get(k)? {
            b']' => break,
            b'/' if bytes.get(k + 1) == Some(&b']') => break,
            _ => k += 1,
        }
    }
    let atts = text[name_end..k].to_owned();
    let (content, core_end) = if bytes[k] == b'/' {
        (None, k + 2)
    } else {
        k += 1;
        let closing = format!("[/{name}]");
        match text[k..].find(&closing) {
            Some(p) => (Some(text[k..k + p].to_owned()), k + p + closing.len()),
            None => (None, k),
        }
    };
    let close_escape = bytes.get(core_end) == Some(&b']');
    Some(Found {
        start: at,
        end: core_end + usize::from(close_escape),
        open_escape,
        name: (*name).to_owned(),
        atts,
        content,
        close_escape,
        core_end,
    })
}

/// The next match at or after `from`.
pub fn find(text: &str, from: usize, tags: &[&str]) -> Option<Found> {
    let mut at = from;
    while let Some(i) = text.get(at..)?.find('[') {
        let start = at + i;
        if let Some(found) = match_at(text, start, tags) {
            return Some(found);
        }
        at = start + 1;
    }
    None
}

/// The registered tags that appear in `content` as `[name`, in registration order.
pub fn tags_in<'a>(content: &str, registered: &[&'a str]) -> Vec<&'a str> {
    if !content.contains('[') {
        return Vec::new();
    }
    static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^<>&/\[\]\x00-\x20=]+)").unwrap());
    let seen: Vec<&str> = NAME.captures_iter(content).filter_map(|c| c.get(1)).map(|m| m.as_str()).collect();
    registered.iter().copied().filter(|tag| seen.contains(tag)).collect()
}

/// Like [`tags_in`], but also finds closing tags (`[/tag`), as `wptexturize()` does.
pub fn tags_in_closing<'a>(content: &str, registered: &[&'a str]) -> Vec<&'a str> {
    if !content.contains('[') {
        return Vec::new();
    }
    static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[/?([^<>&/\[\]\x00-\x20=]+)").unwrap());
    let seen: Vec<&str> = NAME.captures_iter(content).filter_map(|c| c.get(1)).map(|m| m.as_str()).collect();
    registered.iter().copied().filter(|tag| seen.contains(tag)).collect()
}

/// The length of a shortcode delimiter at the start of `s`, as `_get_wptexturize_shortcode_regex()`
/// matches it: `[tag ...]`, `[/tag]`, `[[tag]]`, with HTML allowed inside.
pub fn texturize_delimiter_len(s: &str, tags: &[&str]) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 1;
    if matches!(bytes.get(i), Some(b'/' | b'[')) {
        i += 1;
    }
    let tag = tags.iter().find(|tag| s[i..].starts_with(**tag) && bytes.get(i + tag.len()).is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r' | b']' | b'/')))?;
    i += tag.len();
    loop {
        match bytes.get(i)? {
            b'<' => {
                // `<[^\[\]>]*>`
                let close = s[i + 1..].find(['[', ']', '>'])?;
                if bytes[i + 1 + close] != b'>' {
                    return None;
                }
                i += close + 2;
            }
            b'[' | b'>' => return None,
            b']' => break,
            _ => i += 1,
        }
    }
    i += 1;
    if bytes.get(i) == Some(&b']') {
        i += 1;
    }
    Some(i)
}

/// `do_shortcodes_in_html_tags()` for text without attribute shortcodes: stray brackets inside
/// tags and comments are written as entities so that they cannot form a shortcode.
/// [`unescape_invalid`] undoes it.
pub fn protect_html(content: &str, ignore_html: bool) -> String {
    let content = content.replace("&#91;", "&#091;").replace("&#93;", "&#093;");
    let mut out = String::with_capacity(content.len());
    for (i, element) in crate::formatting::html_split(&content).iter().enumerate() {
        if i % 2 == 0 || element.is_empty() {
            out.push_str(element);
            continue;
        }
        let _ = ignore_html;
        // Shortcodes inside attributes are not processed; every bracket becomes an entity.
        out.push_str(&element.replace('[', "&#91;").replace(']', "&#93;"));
    }
    out
}

/// `unescape_invalid_shortcodes()`.
pub fn unescape_invalid(content: &str) -> String {
    content.replace("&#91;", "[").replace("&#93;", "]")
}

/// `strip_shortcodes()`: removes the shortcodes WordPress knows, keeping their content out.
pub fn strip_shortcodes(content: &str) -> String {
    if !content.contains('[') {
        return content.to_owned();
    }
    let tags = tags_in(content, CORE_TAGS);
    if tags.is_empty() {
        return content.to_owned();
    }
    let content = protect_html(content, true);
    let mut out = String::with_capacity(content.len());
    let mut at = 0;
    while let Some(m) = find(&content, at, &tags) {
        out.push_str(&content[at..m.start]);
        if m.open_escape && m.close_escape {
            out.push_str(&content[m.start + 1..m.end - 1]);
        } else {
            if m.open_escape {
                out.push('[');
            }
            if m.close_escape {
                out.push(']');
            }
        }
        at = m.end;
    }
    out.push_str(&content[at..]);
    unescape_invalid(&out)
}

/// The attributes of a shortcode (`shortcode_parse_atts()`): named ones and positional ones.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Atts {
    pub named: Vec<(String, String)>,
    pub positional: Vec<String>,
}

impl Atts {
    /// The value of a named attribute (the last one wins, as in a PHP array).
    pub fn get(&self, name: &str) -> Option<&str> {
        self.named.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// `! empty( $atts[ name ] )`: set and not `""`/`"0"`.
    pub fn non_empty(&self, name: &str) -> Option<&str> {
        self.get(name).filter(|v| !v.is_empty() && *v != "0")
    }

    pub fn set(&mut self, name: &str, value: &str) {
        match self.named.iter_mut().find(|(k, _)| k == name) {
            Some(slot) => slot.1 = value.to_owned(),
            None => self.named.push((name.to_owned(), value.to_owned())),
        }
    }
}

/// `stripcslashes()`.
fn stripcslashes(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' || i + 1 >= bytes.len() {
            if bytes[i] != b'\\' {
                out.push(bytes[i]);
            }
            i += 1;
            continue;
        }
        i += 1;
        match bytes[i] {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'a' => out.push(7),
            b'v' => out.push(11),
            b'b' => out.push(8),
            b'f' => out.push(12),
            b'x' if bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit) => {
                let mut value = 0u32;
                let mut n = 0;
                while n < 2 && bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit) {
                    value = value * 16 + (bytes[i + 1] as char).to_digit(16).unwrap();
                    i += 1;
                    n += 1;
                }
                out.push(value as u8);
            }
            b'0'..=b'7' => {
                let mut value = 0u32;
                let mut n = 0;
                while n < 3 && bytes.get(i).is_some_and(|b| (b'0'..=b'7').contains(b)) {
                    value = value * 8 + u32::from(bytes[i] - b'0');
                    i += 1;
                    n += 1;
                }
                out.push(value as u8);
                continue;
            }
            other => out.push(other),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `shortcode_parse_atts()`.
pub fn parse_atts(text: &str) -> Atts {
    static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
        let ws = r"[ \t\n\x0B\x0C\r]";
        Regex::new(&format!(
            r#"([A-Za-z0-9_-]+){ws}*={ws}*"([^"]*)"(?:{ws}|$)|([A-Za-z0-9_-]+){ws}*={ws}*'([^']*)'(?:{ws}|$)|([A-Za-z0-9_-]+){ws}*={ws}*([^ \t\n\x0B\x0C\r'"]+)(?:{ws}|$)|"([^"]*)"(?:{ws}|$)|'([^']*)'(?:{ws}|$)|([^ \t\n\x0B\x0C\r]+)(?:{ws}|$)"#
        ))
        .unwrap()
    });
    let text: String = {
        let mut out = String::with_capacity(text.len());
        let mut in_run = false;
        for c in text.chars() {
            if c == '\u{a0}' || c == '\u{200b}' {
                if !in_run {
                    out.push(' ');
                }
                in_run = true;
            } else {
                out.push(c);
                in_run = false;
            }
        }
        out
    };
    let mut atts = Atts::default();
    for m in PATTERN.captures_iter(&text) {
        let group = |i: usize| m.get(i).map(|g| g.as_str());
        if let Some(key) = group(1).filter(|k| !k.is_empty()) {
            atts.set(&key.to_lowercase(), &stripcslashes(group(2).unwrap_or("")));
        } else if let Some(key) = group(3).filter(|k| !k.is_empty()) {
            atts.set(&key.to_lowercase(), &stripcslashes(group(4).unwrap_or("")));
        } else if let Some(key) = group(5).filter(|k| !k.is_empty()) {
            atts.set(&key.to_lowercase(), &stripcslashes(group(6).unwrap_or("")));
        } else if let Some(v) = group(7).filter(|v| !v.is_empty()) {
            atts.positional.push(stripcslashes(v));
        } else if let Some(v) = group(8).filter(|v| !v.is_empty()) {
            atts.positional.push(stripcslashes(v));
        } else if let Some(v) = group(9) {
            atts.positional.push(stripcslashes(v));
        }
    }
    // Reject any unclosed HTML elements.
    static BALANCED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[^<]*(?:<[^>]*>[^<]*)*$").unwrap());
    for value in atts.named.iter_mut().map(|(_, v)| v).chain(atts.positional.iter_mut()) {
        if value.contains('<') && !BALANCED.is_match(value) {
            value.clear();
        }
    }
    atts
}

/// `shortcode_unautop()`: unwraps a shortcode that `wpautop()` put in a paragraph of its own.
pub fn shortcode_unautop(text: &str) -> String {
    let tags = CORE_TAGS;
    let spaces_len = |s: &str| -> usize {
        let mut n = 0;
        loop {
            let rest = &s[n..];
            if let Some(c) = rest.chars().next().filter(|c| matches!(c, '\r' | '\n' | '\t' | ' ')) {
                n += c.len_utf8();
            } else if rest.starts_with('\u{a0}') {
                n += '\u{a0}'.len_utf8();
            } else if rest.starts_with("&nbsp;") {
                n += 6;
            } else {
                return n;
            }
        }
    };
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(p) = text[at..].find("<p>") {
        let open = at + p;
        let after = open + 3;
        let lead = spaces_len(&text[after..]);
        let begin = after + lead;
        let matched = text[begin..].starts_with('[').then(|| match_at(text, begin, tags)).flatten();
        if let Some(m) = matched {
            let end = m.core_end;
            let trail = spaces_len(&text[end..]);
            if text[end + trail..].starts_with("</p>") {
                out.push_str(&text[at..open]);
                out.push_str(&text[begin..end]);
                at = end + trail + 4;
                continue;
            }
        }
        out.push_str(&text[at..after]);
        at = after;
    }
    out.push_str(&text[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_enclosing_self_closing_and_escaped_shortcodes() {
        let tags = ["caption", "gallery"];
        let m = find("a [caption id=\"x\" width=3]<img>Text[/caption] b", 0, &tags).unwrap();
        assert_eq!((m.name.as_str(), m.atts.as_str(), m.content.as_deref()), ("caption", " id=\"x\" width=3", Some("<img>Text")));
        let m = find("[gallery ids=\"1,2\" /]", 0, &tags).unwrap();
        assert_eq!((m.content, m.atts.as_str()), (None, " ids=\"1,2\" "));
        let m = find("[[gallery]]", 0, &tags).unwrap();
        assert!(m.open_escape && m.close_escape);
        assert!(find("[gallery-x]", 0, &tags).is_none());
        assert!(find("[gallery", 0, &tags).is_none());
    }

    #[test]
    fn parses_attributes_like_php() {
        let atts = parse_atts(r#" id="a b" Width=300 'single' flag caption='Its' x='a\'b'"#);
        assert_eq!(atts.get("id"), Some("a b"));
        assert_eq!(atts.get("width"), Some("300"));
        assert_eq!(atts.positional, ["single", "flag", "x='a'b'"]);
        assert_eq!(atts.get("caption"), Some("Its"));
    }

    #[test]
    fn strips_and_unautops() {
        assert_eq!(strip_shortcodes("a [gallery ids=\"1\"] b [[gallery]] c [x]"), "a  b [gallery] c [x]");
        assert_eq!(shortcode_unautop("<p>[gallery ids=\"1\"]</p>\n<p>text [gallery]</p>"), "[gallery ids=\"1\"]\n<p>text [gallery]</p>");
    }
}
