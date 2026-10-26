//! Ports of WordPress' text filters (`wp-includes/formatting.php`):
//! escaping, `wptexturize`, `wpautop`, `convert_chars`, `capital_P_dangit`.

use std::sync::OnceLock;

use fancy_regex::Regex;

macro_rules! re {
    ($pattern:expr) => {{
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new($pattern).expect("valid regex"))
    }};
}

// ---------------------------------------------------------------- escaping

/// `sanitize_html_class()`: percent-encoded octets and everything but `[A-Za-z0-9_-]` go.
pub fn sanitize_html_class(class: &str) -> String {
    let bytes = class.as_bytes();
    let mut stripped = String::with_capacity(class.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() && bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit) && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit) {
            i += 3;
            continue;
        }
        stripped.push(bytes[i] as char);
        i += 1;
    }
    stripped.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect()
}

/// `&` that is not part of a valid entity (`&amp;`, `&#038;`, `&#x26;`).
fn is_entity_at(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'&') {
        return false;
    }
    let Some(end) = bytes[..bytes.len().min(34)].iter().position(|&b| b == b';') else { return false };
    let inner = &s[1..end];
    if let Some(num) = inner.strip_prefix('#') {
        return match num.strip_prefix(['x', 'X']) {
            Some(hex) => !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()),
            None => !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()),
        };
    }
    !inner.is_empty() && inner.as_bytes()[0].is_ascii_alphabetic() && inner.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `_wp_specialchars( $text, ENT_QUOTES )` without double encoding.
pub fn esc_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for (i, c) in text.char_indices() {
        match c {
            '&' if is_entity_at(&text[i..]) => out.push('&'),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#039;"),
            _ => out.push(c),
        }
    }
    out
}

/// `esc_attr` is the same encoding as `esc_html`.
pub fn esc_attr(text: &str) -> String {
    esc_html(text)
}

/// `esc_url()` for http(s), mailto, tel and relative URLs. Other schemes yield `""`.
pub fn esc_url(url: &str) -> String {
    esc_url_context(url, true)
}

/// `esc_url_raw()`: the same cleaning, but ampersands and quotes stay as they are (for storing and redirects).
pub fn esc_url_raw(url: &str) -> String {
    esc_url_context(url, false)
}

/// `preg_match( '/^[a-z0-9-]+?\.php/i', $url )`.
fn is_php_file(url: &str) -> bool {
    let stem = url.bytes().take_while(|b| b.is_ascii_alphanumeric() || *b == b'-').count();
    stem > 0 && url[stem..].get(..4).is_some_and(|s| s.eq_ignore_ascii_case(".php"))
}

fn esc_url_context(url: &str, display: bool) -> String {
    let url = url.trim_start().replace(' ', "%20");
    if url.is_empty() {
        return String::new();
    }
    let cleaned: String = url
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || "-~+_.?#=!&;,/:%@$|*'()[]".contains(*c) || (*c as u32) >= 0x80)
        .collect();
    if cleaned.is_empty() {
        return String::new();
    }
    let cleaned = cleaned.replace(";//", "://");
    // An address without a scheme is taken for a host, unless it is relative (`/x`, `#x`, `?x`, `file.php`).
    let cleaned = if !cleaned.contains(':') && !cleaned.starts_with(['/', '#', '?']) && !is_php_file(&cleaned) { format!("http://{cleaned}") } else { cleaned };
    if let Some(colon) = cleaned.find(':') {
        let scheme = &cleaned[..colon];
        let looks_like_scheme = !scheme.is_empty() && scheme.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
        if looks_like_scheme && !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "ftp" | "ftps" | "mailto" | "news" | "irc" | "irc6" | "ircs" | "gopher" | "nntp" | "feed" | "telnet" | "mms" | "rtsp" | "sms" | "svn" | "tel" | "fax" | "xmpp" | "webcal" | "urn") {
            return String::new();
        }
    }
    if !display {
        return cleaned;
    }
    // Normalise bare ampersands, then use the numeric form WordPress prints.
    let mut out = String::with_capacity(cleaned.len() + 8);
    for (i, c) in cleaned.char_indices() {
        match c {
            '&' if is_entity_at(&cleaned[i..]) => out.push('&'),
            '&' => out.push_str("&amp;"),
            other => out.push(other),
        }
    }
    out.replace("&amp;", "&#038;").replace('\'', "&#039;")
}

// ----------------------------------------------------------- html splitting

/// `wp_html_split`: alternating text and tag/comment pieces, text first
/// (possibly empty); odd indexes are tags.
pub fn html_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = input;
    loop {
        match rest.find('<') {
            None => {
                parts.push(rest.to_owned());
                return parts;
            }
            Some(i) => {
                parts.push(rest[..i].to_owned());
                let element = &rest[i..];
                let len = if element.starts_with("<!--") {
                    element[2..].find("-->").map_or(element.len(), |e| 2 + e + 3)
                } else if element.starts_with("<![CDATA[") {
                    element[2..].find("]]>").map_or(element.len(), |e| 2 + e + 3)
                } else {
                    element.find('>').map_or(element.len(), |e| e + 1)
                };
                parts.push(element[..len].to_owned());
                rest = &element[len..];
                if rest.is_empty() {
                    parts.push(String::new());
                    return parts;
                }
            }
        }
    }
}

/// `wp_replace_in_html_tags`: replaces inside tags only.
fn replace_in_html_tags(haystack: &str, pairs: &[(&str, &str)]) -> String {
    let mut parts = html_split(haystack);
    for i in (1..parts.len()).step_by(2) {
        for (needle, replacement) in pairs {
            if parts[i].contains(needle) {
                parts[i] = parts[i].replace(needle, replacement);
            }
        }
    }
    parts.concat()
}

// -------------------------------------------------------------- texturize

const SPACES: &str = "[\\r\\n\\t ]|\u{a0}|&nbsp;";

/// The characters `wptexturize()` writes for quotes, dashes and the like. WordPress takes them
/// from the language pack with `_x()`; [`TexturizeChars::english`] are the msgids.
#[derive(Debug, Clone)]
pub struct TexturizeChars {
    pub opening_quote: String,
    pub closing_quote: String,
    pub apos: String,
    pub prime: String,
    pub double_prime: String,
    pub opening_single_quote: String,
    pub closing_single_quote: String,
    pub en_dash: String,
    pub em_dash: String,
    /// Words that start with an apostrophe and what they become (`'cause`, `'em`, ...).
    pub cockney: Vec<(String, String)>,
}

impl TexturizeChars {
    pub fn english() -> Self {
        let entity = |n: u32| format!("&#{n};");
        Self {
            opening_quote: entity(8220),
            closing_quote: entity(8221),
            apos: entity(8217),
            prime: entity(8242),
            double_prime: entity(8243),
            opening_single_quote: entity(8216),
            closing_single_quote: entity(8217),
            en_dash: entity(8211),
            em_dash: entity(8212),
            cockney: Self::cockney_from(
                "'tain't,'twere,'twas,'tis,'twill,'til,'bout,'nuff,'round,'cause,'em",
                "&#8217;tain&#8217;t,&#8217;twere,&#8217;twas,&#8217;tis,&#8217;twill,&#8217;til,&#8217;bout,&#8217;nuff,&#8217;round,&#8217;cause,&#8217;em",
            ),
        }
    }

    /// Pairs the two comma-separated lists of the language pack; a missing replacement is empty.
    pub fn cockney_from(words: &str, replacements: &str) -> Vec<(String, String)> {
        let replacements: Vec<&str> = replacements.split(',').collect();
        words.split(',').enumerate().map(|(i, w)| (w.to_owned(), replacements.get(i).copied().unwrap_or("").to_owned())).collect()
    }
}

/// The rules of `wptexturize()` for one set of characters.
#[derive(Debug)]
pub struct Texturize {
    chars: TexturizeChars,
    apos: Vec<(Regex, String)>,
    quote: Vec<(Regex, String)>,
    dash: Vec<(Regex, String)>,
    static_chars: Vec<(String, String)>,
}

impl Texturize {
    pub fn new(chars: TexturizeChars) -> Self {
        let r = |p: String| Regex::new(&p).expect("valid texturize regex");
        let end = "\\n?\\z";
        let (apos_flag, open_sq_flag, open_q_flag) = ("<!--apos-->", "<!--osq-->", "<!--oq-->");
        let mut apos: Vec<(Regex, String)> = Vec::new();
        // '99 and '99" are ambiguous among other patterns; assume an abbreviated year at the end of a quotation.
        if chars.apos != "'" || chars.closing_single_quote != "'" {
            apos.push((r(format!("'(\\d\\d)'(?={end}|[.,:;!?)}}\\-\\]]|&gt;|{SPACES})")), format!("{apos_flag}${{1}}{}", chars.closing_single_quote)));
        }
        if chars.apos != "'" || chars.closing_quote != "\"" {
            apos.push((r(format!("'(\\d\\d)\"(?={end}|[.,:;!?)}}\\-\\]]|&gt;|{SPACES})")), format!("{apos_flag}${{1}}{}", chars.closing_quote)));
        }
        // '99 '99s '99's (apostrophe), but never '9 or '99% or '999 or '99.0.
        if chars.apos != "'" {
            apos.push((r(format!("'(?=\\d\\d(?:{end}|(?![%\\d]|[.,]\\d)))")), apos_flag.to_owned()));
        }
        // Quoted numbers like '0.42'.
        if chars.opening_single_quote != "'" && chars.closing_single_quote != "'" {
            apos.push((r(format!("(?<=\\A|{SPACES})'(\\d[.,\\d]*)'")), format!("{open_sq_flag}${{1}}{}", chars.closing_single_quote)));
        }
        // A single quote at the start, or after (, {, <, [, ", - or spaces.
        if chars.opening_single_quote != "'" {
            apos.push((r(format!("(?<=\\A|[(\\[{{\"\\-]|&lt;|{SPACES})'")), open_sq_flag.to_owned()));
        }
        // An apostrophe in a word: no spaces, double apostrophes or other punctuation around it.
        if chars.apos != "'" {
            apos.push((r(format!("(?<!{SPACES})'(?!{end}|[.,:;!?\"'(){{}}\\[\\]\\-]|&[lg]t;|{SPACES})")), apos_flag.to_owned()));
        }
        let mut quote: Vec<(Regex, String)> = Vec::new();
        if chars.opening_quote != "\"" && chars.closing_quote != "\"" {
            quote.push((r(format!("(?<=\\A|{SPACES})\"(\\d[.,\\d]*)\"")), format!("{open_q_flag}${{1}}{}", chars.closing_quote)));
        }
        if chars.opening_quote != "\"" {
            quote.push((r(format!("(?<=\\A|[(\\[{{\\-]|&lt;|{SPACES})\"(?!{SPACES})")), open_q_flag.to_owned()));
        }
        let dash = vec![
            (r("---".into()), chars.em_dash.clone()),
            (r(format!("(?<=\\A|{SPACES})--(?={end}|{SPACES})")), chars.em_dash.clone()),
            (r("(?<!xn)--".into()), chars.en_dash.clone()),
            (r(format!("(?<=\\A|{SPACES})-(?={end}|{SPACES})")), chars.en_dash.clone()),
        ];
        let mut static_chars = vec![
            ("...".to_owned(), "&#8230;".to_owned()),
            ("``".to_owned(), chars.opening_quote.clone()),
            ("''".to_owned(), chars.closing_quote.clone()),
            (" (tm)".to_owned(), " &#8482;".to_owned()),
        ];
        static_chars.extend(chars.cockney.iter().cloned());
        Self { chars, apos, quote, dash, static_chars }
    }
}

fn texturize_rules() -> &'static Texturize {
    static RULES: OnceLock<Texturize> = OnceLock::new();
    RULES.get_or_init(|| Texturize::new(TexturizeChars::english()))
}
fn replace_all_str(re: &Regex, text: &str, replacement: &str) -> String {
    re.replace_all(text, replacement).into_owned()
}

/// `wptexturize_primes`.
fn texturize_primes(haystack: &str, needle: char, prime: &str, open_quote: &str, close_quote: &str) -> String {
    let flag = "<!--wp-prime-or-quote-->";
    let end = "\\n?\\z";
    let needle_text = needle.to_string();
    let n = fancy_regex::escape(&needle_text);
    let quote_pattern = cached_regex(&format!("{n}(?={end}|[.,:;!?)}}\\-\\]]|&gt;|{SPACES})"));
    let prime_pattern = cached_regex(&format!("(?<=\\d){n}"));
    let flag_after_digit = cached_regex(&format!("(?<=\\d){}", fancy_regex::escape(flag)));
    let flag_no_digit = cached_regex(&format!("(?<!\\d){}", fancy_regex::escape(flag)));

    let mut sentences: Vec<String> = haystack.split(open_quote).map(str::to_owned).collect();
    for (key, sentence) in sentences.iter_mut().enumerate() {
        if !sentence.contains(needle) {
            continue;
        }
        if key != 0 && !sentence.contains(close_quote) {
            let count = quote_pattern.find_iter(sentence).count();
            *sentence = replace_all_str(&quote_pattern, sentence, flag);
            if count > 1 {
                let before = sentence.clone();
                *sentence = replace_all_str(&flag_no_digit, sentence, close_quote);
                if *sentence == before {
                    // No flag without a digit: use the last `flag.` or the last flag.
                    let pos = sentence.rfind(&format!("{flag}.")).or_else(|| sentence.rfind(flag));
                    if let Some(pos) = pos {
                        sentence.replace_range(pos..pos + flag.len(), close_quote);
                    }
                }
                *sentence = replace_all_str(&prime_pattern, sentence, prime);
                *sentence = replace_all_str(&flag_after_digit, sentence, prime);
                *sentence = sentence.replace(flag, close_quote);
            } else if count == 1 {
                *sentence = sentence.replace(flag, close_quote);
                *sentence = replace_all_str(&prime_pattern, sentence, prime);
            } else {
                *sentence = replace_all_str(&prime_pattern, sentence, prime);
            }
        } else {
            *sentence = replace_all_str(&prime_pattern, sentence, prime);
            *sentence = replace_all_str(&quote_pattern, sentence, close_quote);
        }
        if needle == '"' && sentence.contains('"') {
            *sentence = sentence.replace('"', close_quote);
        }
    }
    sentences.join(open_quote)
}

fn pushpop_element(text: &str, stack: &mut Vec<String>, disabled: &[&str]) {
    let bytes = text.as_bytes();
    let (opening, name_offset) = if bytes.len() > 1 && bytes[1] != b'/' {
        (true, 1)
    } else if stack.is_empty() {
        return;
    } else {
        (false, 2)
    };
    let end = match text.find(' ') {
        Some(space) => space,
        None => text.len().saturating_sub(1),
    };
    let tag = text.get(name_offset..end.max(name_offset)).unwrap_or("");
    if disabled.contains(&tag) {
        if opening {
            stack.push(tag.to_owned());
        } else if stack.last().map(String::as_str) == Some(tag) {
            stack.pop();
        }
    }
}

fn fix_ampersands(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, c) in text.char_indices() {
        if c == '&' && !is_entity_at(&text[i..]) {
            out.push_str("&#038;");
        } else {
            out.push(c);
        }
    }
    out
}

/// `wptexturize()` with the English characters.
pub fn wptexturize(text: &str) -> String {
    wptexturize_with(text, texturize_rules())
}

/// `wptexturize()`: curly quotes, dashes, ellipses and `x` multiplication, written with the
/// characters of `rules` (which depend on the language).
pub fn wptexturize_with(text: &str, rules: &Texturize) -> String {
    if text.is_empty() {
        return String::new();
    }
    let no_texturize_tags = ["pre", "code", "kbd", "style", "script", "tt"];
    let no_texturize_shortcodes = ["code"];
    let mut stack: Vec<String> = Vec::new();
    let mut shortcode_stack: Vec<String> = Vec::new();
    let mut out = String::with_capacity(text.len() + text.len() / 8);
    let tagnames = crate::shortcodes::tags_in_closing(text, crate::shortcodes::CORE_TAGS);

    // Split into tags/comments, shortcodes and text, like the first regexp of wptexturize.
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(len) = (rest.starts_with('[') && !tagnames.is_empty()).then(|| crate::shortcodes::texturize_delimiter_len(rest, &tagnames)).flatten() {
            let piece = &rest[..len];
            rest = &rest[len..];
            out.push_str(piece);
            if !piece.starts_with("[[") && !piece.ends_with("]]") {
                pushpop_element(piece, &mut shortcode_stack, &no_texturize_shortcodes);
            }
            continue;
        }
        if rest.starts_with('<') {
            let len = if rest.starts_with("<!--") {
                rest[2..].find("-->").map_or(rest.len(), |e| 2 + e + 3)
            } else {
                rest.find('>').map_or(rest.len(), |e| e + 1)
            };
            let piece = &rest[..len];
            rest = &rest[len..];
            if piece.starts_with("<!--") {
                out.push_str(piece);
            } else {
                let fixed = fix_ampersands(piece);
                pushpop_element(&fixed, &mut stack, &no_texturize_tags);
                out.push_str(&fixed);
            }
            continue;
        }
        let mut len = rest.find('<').unwrap_or(rest.len());
        if !tagnames.is_empty() {
            // Text ends where a shortcode begins.
            let mut at = 1;
            while let Some(i) = rest.get(at..len).and_then(|s| s.find('[')) {
                if crate::shortcodes::texturize_delimiter_len(&rest[at + i..], &tagnames).is_some() {
                    len = at + i;
                    break;
                }
                at += i + 1;
            }
        }
        let piece = &rest[..len];
        rest = &rest[len..];
        if piece.trim().is_empty() || !stack.is_empty() || !shortcode_stack.is_empty() {
            out.push_str(piece);
            continue;
        }
        out.push_str(&texturize_text(piece, rules));
    }
    out
}

fn texturize_text(piece: &str, rules: &Texturize) -> String {
    let mut curl = piece.to_owned();
    for (from, to) in &rules.static_chars {
        curl = curl.replace(from, to);
    }
    if curl.contains('\'') {
        for (re, rep) in &rules.apos {
            curl = replace_all_str(re, &curl, rep);
        }
        curl = texturize_primes(&curl, '\'', &rules.chars.prime, "<!--osq-->", &rules.chars.closing_single_quote);
        curl = curl.replace("<!--apos-->", &rules.chars.apos);
        curl = curl.replace("<!--osq-->", &rules.chars.opening_single_quote);
    }
    if curl.contains('"') {
        for (re, rep) in &rules.quote {
            curl = replace_all_str(re, &curl, rep);
        }
        curl = texturize_primes(&curl, '"', &rules.chars.double_prime, "<!--oq-->", &rules.chars.closing_quote);
        curl = curl.replace("<!--oq-->", &rules.chars.opening_quote);
    }
    if curl.contains('-') {
        for (re, rep) in &rules.dash {
            curl = replace_all_str(re, &curl, rep);
        }
    }
    if re!(r"(?<=\d)x\d").is_match(&curl).unwrap_or(false) {
        curl = replace_all_str(re!(r"\b(\d(?:(?<=0)[\d.,]+|(?<!0)[\d.,]*))x(\d[\d.,]*)\b"), &curl, "$1&#215;$2");
    }
    fix_ampersands(&curl)
}

/// `convert_chars()`: bare ampersands to `&#038;`.
pub fn convert_chars(content: &str) -> String {
    if content.contains('&') {
        replace_all_str(re!(r"(?i)&([^#])(?![a-z1-4]{1,8};)"), content, "&#038;$1")
    } else {
        content.to_owned()
    }
}

/// `capital_P_dangit()` as applied to titles and content.
pub fn capital_p_dangit(text: &str, is_title: bool) -> String {
    if is_title {
        return text.replace("Wordpress", "WordPress");
    }
    text.replace(" Wordpress", " WordPress")
        .replace("&#8216;Wordpress", "&#8216;WordPress")
        .replace("&#8220;Wordpress", "&#8220;WordPress")
        .replace(">Wordpress", ">WordPress")
        .replace("(Wordpress", "(WordPress")
}

// ----------------------------------------------------------------- wpautop

const ALLBLOCKS: &str = "(?:table|thead|tfoot|caption|col|colgroup|tbody|tr|td|th|div|dl|dd|dt|ul|ol|li|pre|form|map|area|blockquote|address|style|p|h[1-6]|hr|fieldset|legend|section|article|aside|hgroup|header|footer|nav|figure|figcaption|details|menu|summary)";

fn rx(pattern: &str) -> &'static Regex {
    cached_regex(pattern)
}

/// [`cached_regex`] for the `regex` crate (no look-around).
pub fn cached_plain_regex(pattern: &str) -> &'static regex::Regex {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, &'static regex::Regex>>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(found) = cache.lock().expect("regex cache").get(pattern) {
        return found;
    }
    let compiled = regex::Regex::new(pattern).unwrap_or_else(|error| {
        // A broken pattern matches nothing instead of taking the request down.
        tracing::error!(%error, pattern, "invalid regex");
        regex::Regex::new(r"\b\B").expect("never matches")
    });
    let compiled: &'static regex::Regex = Box::leak(Box::new(compiled));
    cache.lock().expect("regex cache").entry(pattern.to_owned()).or_insert(compiled)
}

/// A regex compiled once per pattern. The patterns of the filters are a small fixed set (built from constants), so
/// the cache stays small; compiling them on every call was the largest cost of `wpautop()`.
pub fn cached_regex(pattern: &str) -> &'static Regex {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, &'static Regex>>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(found) = cache.lock().expect("regex cache").get(pattern) {
        return found;
    }
    let compiled = Regex::new(pattern).unwrap_or_else(|error| {
        tracing::error!(%error, pattern, "invalid regex");
        Regex::new(r"(?!)").expect("never matches")
    });
    let compiled: &'static Regex = Box::leak(Box::new(compiled));
    cache.lock().expect("regex cache").entry(pattern.to_owned()).or_insert(compiled)
}

/// `wpautop( $text, true )`.
pub fn wpautop(text: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    let mut text = format!("{text}\n");

    // Protect <pre> contents.
    let mut pre_tags: Vec<(String, String)> = Vec::new();
    if text.contains("<pre") {
        let mut parts: Vec<&str> = text.split("</pre>").collect();
        let last = parts.pop().unwrap_or("");
        let mut rebuilt = String::new();
        for (i, part) in parts.iter().enumerate() {
            match part.find("<pre") {
                None => rebuilt.push_str(part),
                Some(start) => {
                    let name = format!("<pre wp-pre-tag-{i}></pre>");
                    pre_tags.push((name.clone(), format!("{}</pre>", &part[start..])));
                    rebuilt.push_str(&part[..start]);
                    rebuilt.push_str(&name);
                }
            }
        }
        rebuilt.push_str(last);
        text = rebuilt;
    }

    let sub = |text: &str, pattern: &str, replacement: &str| replace_all_str(rx(pattern), text, replacement);

    text = sub(&text, r"<br\s*/?>\s*<br\s*/?>", "\n\n");
    text = sub(&text, &format!(r"(<{ALLBLOCKS}[\s/>])"), "\n\n$1");
    text = sub(&text, &format!(r"(</{ALLBLOCKS}>)"), "$1\n\n");
    text = sub(&text, r"(<hr\s*?/?>)", "$1\n\n");
    text = text.replace("\r\n", "\n").replace('\r', "\n");
    text = replace_in_html_tags(&text, &[("\n", " <!-- wpnl --> ")]);

    if text.contains("<option") {
        text = sub(&text, r"\s*<option", "<option");
        text = sub(&text, r"</option>\s*", "</option>");
    }
    if text.contains("</object>") {
        text = sub(&text, r"(<object[^>]*>)\s*", "$1");
        text = sub(&text, r"\s*</object>", "</object>");
        text = sub(&text, r"\s*(</?(?:param|embed)[^>]*>)\s*", "$1");
    }
    if text.contains("<source") || text.contains("<track") {
        text = sub(&text, r"([<\[](?:audio|video)[^>\]]*[>\]])\s*", "$1");
        text = sub(&text, r"\s*([<\[]/(?:audio|video)[>\]])", "$1");
        text = sub(&text, r"\s*(<(?:source|track)[^>]*>)\s*", "$1");
    }
    if text.contains("<figcaption") {
        text = sub(&text, r"\s*(<figcaption[^>]*>)", "$1");
        text = sub(&text, r"</figcaption>\s*", "</figcaption>");
    }
    text = sub(&text, r"\n\n+", "\n\n");

    let mut out = String::new();
    for paragraph in rx(r"\n\s*\n").split(&text).filter_map(Result::ok).filter(|p| !p.is_empty()) {
        out.push_str("<p>");
        out.push_str(paragraph.trim_matches('\n'));
        out.push_str("</p>\n");
    }
    text = out;

    text = sub(&text, r"<p>\s*</p>", "");
    text = sub(&text, r"<p>([^<]+)</(div|address|form)>", "<p>$1</p></$2>");
    text = sub(&text, &format!(r"<p>\s*(</?{ALLBLOCKS}[^>]*>)\s*</p>"), "$1");
    text = sub(&text, r"<p>(<li.+?)</p>", "$1");
    text = sub(&text, r#"(?i)<p><blockquote((?:[^>"']|"[^"]*"|'[^']*')*)>"#, "<blockquote$1><p>");
    text = text.replace("</blockquote></p>", "</p></blockquote>");
    text = sub(&text, &format!(r"<p>\s*(</?{ALLBLOCKS}[^>]*>)"), "$1");
    text = sub(&text, &format!(r"(</?{ALLBLOCKS}[^>]*>)\s*</p>"), "$1");

    // Preserve newlines in script/style/svg/math, convert the rest to <br />.
    let preserved = rx(r"(?s)<(script|style|svg|math).*?</\1>")
        .replace_all(&text, |caps: &fancy_regex::Captures<'_, str>| caps[0].replace('\n', "<WPPreserveNewline />"))
        .into_owned();
    text = preserved.replace("<br>", "<br />").replace("<br/>", "<br />");
    text = sub(&text, r"(?<!<br />)\s*\n", "<br />\n");
    text = text.replace("<WPPreserveNewline />", "\n");

    text = sub(&text, &format!(r"(</?{ALLBLOCKS}[^>]*>)\s*<br />"), "$1");
    text = sub(&text, r"<br />(\s*</?(?:p|li|div|dl|dd|dt|th|pre|td|ul|ol)[^>]*>)", "$1");
    text = sub(&text, r"\n</p>$", "</p>");

    for (name, original) in &pre_tags {
        text = text.replace(name, original);
    }
    if text.contains("<!-- wpnl -->") {
        text = text.replace(" <!-- wpnl --> ", "\n").replace("<!-- wpnl -->", "\n");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_does_not_double_encode_entities() {
        assert_eq!(esc_html("Tom & Jerry <b>\"x\"</b> 'y'"), "Tom &amp; Jerry &lt;b&gt;&quot;x&quot;&lt;/b&gt; &#039;y&#039;");
        assert_eq!(esc_html("a &amp; b &#8211; c &nbsp;"), "a &amp; b &#8211; c &nbsp;");
        assert_eq!(esc_attr("x\"y"), "x&quot;y");
    }

    #[test]
    fn urls_are_escaped_like_esc_url() {
        assert_eq!(esc_url("http://a.test/?x=1&y=2"), "http://a.test/?x=1&#038;y=2");
        assert_eq!(esc_url("/path with space"), "/path%20with%20space");
        assert_eq!(esc_url("javascript:alert(1)"), "");
        assert_eq!(esc_url("#"), "#");
        assert_eq!(esc_url("mailto:a@b.c"), "mailto:a@b.c");
        assert_eq!(esc_url(""), "");
    }

    #[test]
    fn html_split_alternates_text_and_tags() {
        assert_eq!(html_split("a<b>c<!-- x -->d"), ["a", "<b>", "c", "<!-- x -->", "d"]);
        assert_eq!(html_split("<p>x</p>"), ["", "<p>", "x", "</p>", ""]);
        assert_eq!(html_split("plain"), ["plain"]);
    }

    #[test]
    fn escaping_does_not_slice_inside_multibyte_characters() {
        // The entity check looks at the next 34 bytes, which can end inside a character.
        let text = "&ааааааааааааааааааааааааааааааааа";
        assert_eq!(esc_html(text), format!("&amp;{}", &text[1..]));
    }

    #[test]
    fn texturize_quotes_dashes_and_ellipsis() {
        assert_eq!(wptexturize("\"Hello\" -- world..."), "&#8220;Hello&#8221; &#8212; world&#8230;");
        assert_eq!(wptexturize("Don't stop"), "Don&#8217;t stop");
        assert_eq!(wptexturize("'single'"), "&#8216;single&#8217;");
        assert_eq!(wptexturize("a - b"), "a &#8211; b");
        assert_eq!(wptexturize("10x20"), "10&#215;20");
        // Verified against WordPress: a lone pair is a closing quote, not primes.
        assert_eq!(wptexturize("5'10\""), "5&#8217;10&#8221;");
        assert_eq!(wptexturize("a - b -- c --- d"), "a &#8211; b &#8212; c &#8212; d");
        assert_eq!(wptexturize("(tm)"), "(tm)");
        assert_eq!(wptexturize("a (tm)"), "a &#8482;");
    }

    #[test]
    fn texturize_leaves_code_and_tags_alone() {
        assert_eq!(wptexturize("<code>\"x\" -- y</code> \"z\""), "<code>\"x\" -- y</code> &#8220;z&#8221;");
        assert_eq!(wptexturize("<a title=\"x\">\"q\"</a>"), "<a title=\"x\">&#8220;q&#8221;</a>");
        assert_eq!(wptexturize("<!-- \"c\" -->"), "<!-- \"c\" -->");
    }

    #[test]
    fn texturize_escapes_bare_ampersands() {
        assert_eq!(wptexturize("A & B &amp; C"), "A &#038; B &amp; C");
    }

    #[test]
    fn convert_chars_and_capital_p() {
        assert_eq!(convert_chars("a & b &amp; c"), "a &#038; b &amp; c");
        assert_eq!(capital_p_dangit("I use Wordpress", true), "I use WordPress");
        assert_eq!(capital_p_dangit("<b>Wordpress</b> and Wordpress", false), "<b>WordPress</b> and WordPress");
    }

    #[test]
    fn wpautop_wraps_paragraphs_and_breaks() {
        assert_eq!(wpautop("Zeile eins\n\nZeile zwei\nUmbruch"), "<p>Zeile eins</p>\n<p>Zeile zwei<br />\nUmbruch</p>\n");
        assert_eq!(wpautop("   "), "");
    }

    #[test]
    fn wpautop_does_not_wrap_block_elements() {
        assert_eq!(wpautop("<div>x</div>\n\ntext"), "<div>x</div>\n<p>text</p>\n");
        assert_eq!(wpautop("<ul>\n<li>a</li>\n</ul>"), "<ul>\n<li>a</li>\n</ul>\n");
    }

    #[test]
    fn wpautop_keeps_pre_untouched() {
        let out = wpautop("a\n\n<pre>x\n\ny</pre>\n\nb");
        assert!(out.contains("<pre>x\n\ny</pre>"), "{out}");
        assert!(out.starts_with("<p>a</p>"), "{out}");
    }
}


/// PHP's `strip_tags()` without an allow-list: removes `<...>` tags, comments and
/// processing instructions; a `<` not followed by a tag-like character stays.
/// `_wp_specialchars( $text )` with the defaults of a filter: `&`, `<` and `>` are encoded (an entity stays), quotes not.
pub fn specialchars_noquotes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        match bytes[i] {
            b'&' => {
                // An entity that is complete stays as it is.
                let rest = &text[i + 1..];
                let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '#')).filter(|n| *n > 0 && rest[*n..].starts_with(';'));
                match end {
                    Some(n) => {
                        out.push_str(&text[i..i + n + 2]);
                        i += n + 2;
                        continue;
                    }
                    None => out.push_str("&amp;"),
                }
            }
            b'<' => out.push_str("&lt;"),
            b'>' => out.push_str("&gt;"),
            _ => {
                let c = text[i..].chars().next().unwrap_or(' ');
                out.push(c);
                i += c.len_utf8();
                continue;
            }
        }
        i += 1;
    }
    out
}

pub fn strip_tags(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let next = bytes.get(i + 1).copied().unwrap_or(b' ');
            if text[i..].starts_with("<!--") {
                match text[i + 4..].find("-->") {
                    Some(end) => i += 4 + end + 3,
                    None => i = bytes.len(),
                }
                continue;
            }
            if next.is_ascii_alphabetic() || matches!(next, b'/' | b'!' | b'?') {
                let mut quote: Option<u8> = None;
                let mut j = i + 1;
                while j < bytes.len() {
                    match (quote, bytes[j]) {
                        (None, b'"' | b'\'') => quote = Some(bytes[j]),
                        (Some(q), c) if c == q => quote = None,
                        (None, b'>') => break,
                        _ => {}
                    }
                    j += 1;
                }
                i = (j + 1).min(bytes.len());
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `wp_strip_all_tags( $text )`: script/style blocks go with their content.
pub fn wp_strip_all_tags(text: &str) -> String {
    static SCRIPT: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = SCRIPT.get_or_init(|| fancy_regex::Regex::new(r"(?is)<(script|style)[^>]*?>.*?</\1>").expect("valid regex"));
    php_trim(&strip_tags(&re.replace_all(text, ""))).to_owned()
}

/// PHP's `trim()`: spaces, tabs, line breaks, NUL and vertical tabs (no other white space such as U+00A0).
pub fn php_trim(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{b}'])
}

/// `ent2ncr()`: named entities become numeric character references.
pub fn ent2ncr(text: &str) -> String {
    static ENTITY: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = ENTITY.get_or_init(|| fancy_regex::Regex::new(r"&(\w+);").expect("valid regex"));
    re.replace_all(text, |caps: &fancy_regex::Captures<'_, str>| {
        let name = &caps[1];
        match crate::entities::ENTITIES.iter().find(|(n, _)| *n == name) {
            Some((_, code)) => format!("&#{code};"),
            None => caps[0].to_owned(),
        }
    })
    .into_owned()
}

/// `html_entity_decode( $text, ENT_COMPAT )`: double quotes and `&amp;` are decoded, single quotes are not.
pub fn html_entity_decode(text: &str) -> String {
    static ENTITY: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = ENTITY.get_or_init(|| fancy_regex::Regex::new(r"&(#[0-9]+|#[xX][0-9a-fA-F]+|\w+);").expect("valid regex"));
    re.replace_all(text, |caps: &fancy_regex::Captures<'_, str>| {
        let body = &caps[1];
        let code = if let Some(hex) = body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")) {
            u32::from_str_radix(hex, 16).ok()
        } else if let Some(dec) = body.strip_prefix('#') {
            dec.parse().ok()
        } else {
            crate::entities::ENTITIES.iter().find(|(n, _)| *n == body).map(|(_, c)| *c)
        };
        match code.filter(|c| *c != 39).and_then(char::from_u32) {
            Some(c) => c.to_string(),
            None => caps[0].to_owned(),
        }
    })
    .into_owned()
}

/// `wp_trim_words( $text, $num_words, $more )` for space-separated languages.
pub fn wp_trim_words(text: &str, num_words: usize, more: &str) -> String {
    wp_trim_words_with(text, num_words, more, crate::locale::WordCountType::Words)
}

/// `wp_trim_words()` for the word count type of the language: languages without spaces between
/// words (Chinese, Japanese, Korean) count characters instead.
pub fn wp_trim_words_with(text: &str, num_words: usize, more: &str, count: crate::locale::WordCountType) -> String {
    let stripped = wp_strip_all_tags(text);
    if count != crate::locale::WordCountType::Words {
        let collapsed = stripped.split(['\n', '\r', '\t', ' ']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
        let chars: Vec<char> = collapsed.chars().collect();
        return if chars.len() > num_words { format!("{}{more}", chars[..num_words].iter().collect::<String>()) } else { collapsed };
    }
    let words: Vec<&str> = stripped.split(['\n', '\r', '\t', ' ']).filter(|w| !w.is_empty()).collect();
    if words.len() > num_words {
        format!("{}{more}", words[..num_words].join(" "))
    } else {
        words.join(" ")
    }
}

#[cfg(test)]
mod feed_helper_tests {
    use super::*;

    #[test]
    fn strip_tags_like_php() {
        assert_eq!(strip_tags("a <b>bold</b> and <a href=\"x>y\">link</a>"), "a bold and link");
        assert_eq!(strip_tags("1 < 2 and 3 > 2"), "1 < 2 and 3 > 2");
        assert_eq!(strip_tags("x<!-- hidden -->y"), "xy");
    }

    #[test]
    fn entities() {
        assert_eq!(ent2ncr("Caf&eacute; &amp; &unknown; &#8211;"), "Caf&#233; &#38; &unknown; &#8211;");
        assert_eq!(html_entity_decode("&quot;a&quot; &amp; &#8211; &#039; &eacute;"), "\"a\" & \u{2013} &#039; \u{e9}");
    }

    #[test]
    fn languages_without_spaces_are_trimmed_by_characters() {
        use crate::locale::WordCountType::CharactersExcludingSpaces;
        assert_eq!(wp_trim_words_with("<p>日本語の　テキスト です</p>", 4, "…", CharactersExcludingSpaces), "日本語の…");
        assert_eq!(wp_trim_words_with("短い", 4, "…", CharactersExcludingSpaces), "短い");
    }

    #[test]
    fn trim_words() {
        assert_eq!(wp_trim_words("<p>one two  three\nfour</p>", 3, "..."), "one two three...");
        assert_eq!(wp_trim_words("one two", 3, "..."), "one two");
    }
}
