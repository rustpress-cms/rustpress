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

/// `&` that is not part of a valid entity (`&amp;`, `&#038;`, `&#x26;`).
fn is_entity_at(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'&') {
        return false;
    }
    let Some(end) = s[..s.len().min(34)].find(';') else { return false };
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
    let url = url.trim_start().replace(' ', "%20");
    if url.is_empty() {
        return String::new();
    }
    let cleaned: String = url
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || "-~+_.?#=!&;,/:%@$|*'()[]\\".contains(*c) || (*c as u32) >= 0x80)
        .collect();
    if cleaned.is_empty() {
        return String::new();
    }
    if let Some(colon) = cleaned.find(':') {
        let scheme = &cleaned[..colon];
        let looks_like_scheme = !scheme.is_empty() && scheme.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
        if looks_like_scheme && !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "ftp" | "ftps" | "mailto" | "news" | "irc" | "irc6" | "ircs" | "gopher" | "nntp" | "feed" | "telnet" | "mms" | "rtsp" | "sms" | "svn" | "tel" | "fax" | "xmpp" | "webcal" | "urn") {
            return String::new();
        }
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

struct Texturize {
    apos: Vec<(Regex, &'static str)>,
    quote: Vec<(Regex, &'static str)>,
    dash: Vec<(Regex, &'static str)>,
    static_chars: Vec<(&'static str, &'static str)>,
}

fn texturize_rules() -> &'static Texturize {
    static RULES: OnceLock<Texturize> = OnceLock::new();
    RULES.get_or_init(|| {
        let r = |p: String| Regex::new(&p).expect("valid texturize regex");
        let end = "\\n?\\z";
        Texturize {
            apos: vec![
                (r(format!("'(\\d\\d)'(?={end}|[.,:;!?)}}\\-\\]]|&gt;|{SPACES})")), "<!--apos-->$1&#8217;"),
                (r(format!("'(\\d\\d)\"(?={end}|[.,:;!?)}}\\-\\]]|&gt;|{SPACES})")), "<!--apos-->$1&#8221;"),
                (r(format!("'(?=\\d\\d(?:{end}|(?![%\\d]|[.,]\\d)))")), "<!--apos-->"),
                (r(format!("(?<=\\A|{SPACES})'(\\d[.,\\d]*)'")), "<!--osq-->$1&#8217;"),
                (r(format!("(?<=\\A|[(\\[{{\"\\-]|&lt;|{SPACES})'")), "<!--osq-->"),
                (r(format!("(?<!{SPACES})'(?!{end}|[.,:;!?\"'(){{}}\\[\\]\\-]|&[lg]t;|{SPACES})")), "<!--apos-->"),
            ],
            quote: vec![
                (r(format!("(?<=\\A|{SPACES})\"(\\d[.,\\d]*)\"")), "<!--oq-->$1&#8221;"),
                (r(format!("(?<=\\A|[(\\[{{\\-]|&lt;|{SPACES})\"(?!{SPACES})")), "<!--oq-->"),
            ],
            dash: vec![
                (r("---".into()), "&#8212;"),
                (r(format!("(?<=\\A|{SPACES})--(?={end}|{SPACES})")), "&#8212;"),
                (r("(?<!xn)--".into()), "&#8211;"),
                (r(format!("(?<=\\A|{SPACES})-(?={end}|{SPACES})")), "&#8211;"),
            ],
            static_chars: vec![
                ("...", "&#8230;"),
                ("``", "&#8220;"),
                ("''", "&#8221;"),
                (" (tm)", " &#8482;"),
                ("'tain't", "&#8217;tain&#8217;t"),
                ("'twere", "&#8217;twere"),
                ("'twas", "&#8217;twas"),
                ("'tis", "&#8217;tis"),
                ("'twill", "&#8217;twill"),
                ("'til", "&#8217;til"),
                ("'bout", "&#8217;bout"),
                ("'nuff", "&#8217;nuff"),
                ("'round", "&#8217;round"),
                ("'cause", "&#8217;cause"),
                ("'em", "&#8217;em"),
            ],
        }
    })
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
    let quote_pattern = Regex::new(&format!("{n}(?={end}|[.,:;!?)}}\\-\\]]|&gt;|{SPACES})")).expect("regex");
    let prime_pattern = Regex::new(&format!("(?<=\\d){n}")).expect("regex");
    let flag_after_digit = Regex::new(&format!("(?<=\\d){}", fancy_regex::escape(flag))).expect("regex");
    let flag_no_digit = Regex::new(&format!("(?<!\\d){}", fancy_regex::escape(flag))).expect("regex");

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

/// `wptexturize()`: curly quotes, dashes, ellipses and `x` multiplication.
pub fn wptexturize(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let rules = texturize_rules();
    let no_texturize_tags = ["pre", "code", "kbd", "style", "script", "tt"];
    let mut stack: Vec<String> = Vec::new();
    let mut out = String::with_capacity(text.len() + text.len() / 8);

    // Split into tags/comments and text, like the first regexp of wptexturize.
    let mut rest = text;
    while !rest.is_empty() {
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
        let len = rest.find('<').unwrap_or(rest.len());
        let piece = &rest[..len];
        rest = &rest[len..];
        if piece.trim().is_empty() || !stack.is_empty() {
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
        curl = texturize_primes(&curl, '\'', "&#8242;", "<!--osq-->", "&#8217;");
        curl = curl.replace("<!--apos-->", "&#8217;");
        curl = curl.replace("<!--osq-->", "&#8216;");
    }
    if curl.contains('"') {
        for (re, rep) in &rules.quote {
            curl = replace_all_str(re, &curl, rep);
        }
        curl = texturize_primes(&curl, '"', "&#8243;", "<!--oq-->", "&#8221;");
        curl = curl.replace("<!--oq-->", "&#8220;");
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

fn rx(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid wpautop regex")
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

    let sub = |text: &str, pattern: &str, replacement: &str| replace_all_str(&rx(pattern), text, replacement);

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
