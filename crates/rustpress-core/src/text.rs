//! Text filters of comments and content: `make_clickable()` and `convert_smilies()`.

use std::sync::OnceLock;

use fancy_regex::Regex;

use crate::formatting::{esc_attr, esc_url};

macro_rules! re {
    ($pattern:expr) => {{
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new($pattern).expect("valid regex"))
    }};
}

/// Where `make_clickable()` runs, for the `rel` attribute of the links it makes.
pub struct Clickable<'a> {
    /// The host of the home URL (`wp_internal_hosts()`).
    pub home_host: &'a str,
    /// `current_filter() === 'comment_text'`: links also get `ugc`.
    pub comment: bool,
}

fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    Some(host.split(':').next()?.to_ascii_lowercase())
}

/// `_make_clickable_rel_attr()`.
fn clickable_rel(url: &str, opts: &Clickable<'_>) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let scheme = url.split_once(':').map_or(String::new(), |(s, _)| s.to_ascii_lowercase());
    let internal = url_host(url).is_some_and(|h| h == opts.home_host.to_ascii_lowercase());
    if !internal && (scheme == "http" || scheme == "https") {
        parts.push("nofollow");
    }
    if opts.comment {
        parts.push("ugc");
    }
    if parts.is_empty() { String::new() } else { format!(" rel=\"{}\"", esc_attr(&parts.join(" "))) }
}

fn group<'a>(c: &'a fancy_regex::Captures<'_, str>, i: usize) -> &'a str {
    c.get(i).map_or("", |m| m.as_str())
}

/// `make_clickable()`: URLs, `www.`/`ftp.` hosts and email addresses become links.
pub fn make_clickable(text: &str, opts: &Clickable<'_>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut nested = 0i32;
    let mut pieces: Vec<&str> = Vec::new();
    let mut last = 0;
    for m in re!(r"<[^<>]+>").find_iter(text).flatten() {
        pieces.push(&text[last..m.start()]);
        pieces.push(m.as_str());
        last = m.end();
    }
    pieces.push(&text[last..]);
    let code_open = re!(r"(?i)^<(?:code|pre|script|style)[\s>]");
    let scheme_tag = re!(r"^<\s*[\w]{1,20}://");
    let url_class = r"A-Za-z0-9_\x{80}-\x{10FFFF}#%~/@\[\]*(+=&$-";
    let url = Regex::new(&format!(
        r"([\s(<.,;:!?])((?>[A-Za-z0-9_]{{1,20}})://(?=\S{{1,2000}}\s)(?>[{url_class}]*)(?:['.,;:!?)](?>[{url_class}]+))*)(\)?)(\.\w{{2,6}})?"
    ))
    .expect("url regex");
    for piece in pieces {
        if code_open.is_match(piece).unwrap_or(false) {
            nested += 1;
        } else if nested > 0 && matches!(piece.to_ascii_lowercase().as_str(), "</code>" | "</pre>" | "</script>" | "</style>") {
            nested -= 1;
        }
        if nested > 0 || piece.is_empty() || (piece.starts_with('<') && !scheme_tag.is_match(piece).unwrap_or(false)) {
            out.push_str(piece);
            continue;
        }
        let padded = format!(" {piece} ");
        let ret = url.replace_all(&padded, |c: &fancy_regex::Captures<'_, str>| {
            let mut link = group(c, 2).to_owned();
            let closing = group(c, 3);
            let mut suffix = if closing == ")" && link.contains('(') {
                // The closing parenthesis belongs to the URL.
                link.push_str(closing);
                String::new()
            } else {
                closing.to_owned()
            };
            link.push_str(group(c, 4));
            while link.matches('(').count() < link.matches(')').count() {
                let at = link.rfind(')').unwrap_or(0);
                suffix = format!("{}{suffix}", &link[at..]);
                link.truncate(at);
            }
            let href = esc_url(&link);
            if href.is_empty() {
                return group(c, 0).to_owned();
            }
            format!("{}<a href=\"{href}\"{}>{href}</a>{suffix}", group(c, 1), clickable_rel(&href, opts))
        });
        let web = re!(r"(?is)([\s>])((?:www|ftp)\.[A-Za-z0-9_\x{80}-\x{10FFFF}\#$%&~/.\-;:=,?@\[\]+]+)");
        let ret = web.replace_all(&ret, |c: &fancy_regex::Captures<'_, str>| {
            let mut dest = format!("http://{}", group(c, 2));
            let mut tail = String::new();
            if let Some(last) = dest.chars().last().filter(|l| matches!(l, '.' | ',' | ';' | ':' | ')')) {
                tail.push(last);
                dest.pop();
            }
            let dest = esc_url(&dest);
            if dest.is_empty() {
                return group(c, 0).to_owned();
            }
            format!("{}<a href=\"{dest}\"{}>{dest}</a>{tail}", group(c, 1), clickable_rel(&dest, opts))
        });
        let email = re!(r"(?i)([\s>])([.0-9a-z_+-]+)@((?:[0-9a-z-]+\.)+[0-9a-z]{2,})");
        let ret = email.replace_all(&ret, |c: &fancy_regex::Captures<'_, str>| {
            let address = format!("{}@{}", group(c, 2), group(c, 3));
            format!("{}<a href=\"mailto:{address}\">{address}</a>", group(c, 1))
        });
        out.push_str(ret.get(1..ret.len().saturating_sub(1)).unwrap_or(""));
    }
    re!(r"(?i)(<a([ \r\n\t]+[^>]+?>|>))<a [^>]+?>([^>]+?)</a></a>").replace_all(&out, "$1$3</a>").into_owned()
}

/// The smilies of WordPress: text, replacement (an image file or an emoji).
const SMILIES: &[(&str, &str)] = &[
    (":mrgreen:", "mrgreen.png"),
    (":neutral:", "\u{1F610}"),
    (":twisted:", "\u{1F608}"),
    (":arrow:", "\u{27A1}"),
    (":shock:", "\u{1F62F}"),
    (":smile:", "\u{1F642}"),
    (":???:", "\u{1F615}"),
    (":cool:", "\u{1F60E}"),
    (":evil:", "\u{1F47F}"),
    (":grin:", "\u{1F600}"),
    (":idea:", "\u{1F4A1}"),
    (":oops:", "\u{1F633}"),
    (":razz:", "\u{1F61B}"),
    (":roll:", "\u{1F644}"),
    (":wink:", "\u{1F609}"),
    (":cry:", "\u{1F625}"),
    (":eek:", "\u{1F62E}"),
    (":lol:", "\u{1F606}"),
    (":mad:", "\u{1F621}"),
    (":sad:", "\u{1F641}"),
    ("8-)", "\u{1F60E}"),
    ("8-O", "\u{1F62F}"),
    (":-(", "\u{1F641}"),
    (":-)", "\u{1F642}"),
    (":-?", "\u{1F615}"),
    (":-D", "\u{1F600}"),
    (":-P", "\u{1F61B}"),
    (":-o", "\u{1F62E}"),
    (":-x", "\u{1F621}"),
    (":-|", "\u{1F610}"),
    (";-)", "\u{1F609}"),
    ("8O", "\u{1F62F}"),
    (":(", "\u{1F641}"),
    (":)", "\u{1F642}"),
    (":?", "\u{1F615}"),
    (":D", "\u{1F600}"),
    (":P", "\u{1F61B}"),
    (":o", "\u{1F62E}"),
    (":x", "\u{1F621}"),
    (":|", "\u{1F610}"),
    (";)", "\u{1F609}"),
    (":!:", "\u{2757}"),
    (":?:", "\u{2753}"),
];

/// `(?<=wp_spaces_regexp()|^)`.
fn space_before(before: &str) -> bool {
    before.is_empty() || before.ends_with(['\r', '\n', '\t', ' ', '\u{a0}']) || before.ends_with("&nbsp;")
}

/// `(?=wp_spaces_regexp()|$)`.
fn space_after(after: &str) -> bool {
    after.is_empty() || after.starts_with(['\r', '\n', '\t', ' ', '\u{a0}']) || after.starts_with("&nbsp;")
}

/// `convert_smilies()`; `includes_url` is `includes_url()` of the site.
pub fn convert_smilies(text: &str, includes_url: &str) -> String {
    // The search pattern lists the smilies by descending key.
    let mut ordered: Vec<&(&str, &str)> = SMILIES.iter().collect();
    ordered.sort_by(|a, b| b.0.as_bytes().cmp(a.0.as_bytes()));
    let tags_to_ignore = ["code", "pre", "style", "script", "textarea"];
    let mut output = String::with_capacity(text.len());
    let mut ignore: Option<&str> = None;
    let mut rest = text;
    while !rest.is_empty() {
        let (chunk, is_tag) = if rest.starts_with('<') {
            match rest.find('>') {
                Some(end) => (&rest[..=end], true),
                None => (rest, false),
            }
        } else {
            (&rest[..rest.find('<').unwrap_or(rest.len())], false)
        };
        rest = &rest[chunk.len()..];
        if is_tag {
            if ignore.is_none() {
                ignore = tags_to_ignore.iter().copied().find(|name| chunk[1..].starts_with(name));
            }
            if let Some(name) = ignore
                && chunk.strip_prefix("</").and_then(|c| c.strip_suffix('>')) == Some(name)
            {
                ignore = None;
            }
            output.push_str(chunk);
            continue;
        }
        if ignore.is_some() {
            output.push_str(chunk);
            continue;
        }
        let mut i = 0;
        while i < chunk.len() {
            let here = &chunk[i..];
            let hit = if space_before(&chunk[..i]) { ordered.iter().find(|(smiley, _)| here.starts_with(*smiley) && space_after(&here[smiley.len()..])) } else { None };
            match hit {
                Some((smiley, image)) => {
                    if image.ends_with(".png") {
                        output.push_str(&format!(
                            "<img src=\"{}\" alt=\"{}\" class=\"wp-smiley\" style=\"height: 1em; max-height: 1em;\" />",
                            esc_url(&format!("{includes_url}/images/smilies/{image}")),
                            esc_attr(smiley)
                        ));
                    } else {
                        output.push_str(image);
                    }
                    i += smiley.len();
                }
                None => {
                    let c = here.chars().next().unwrap_or(' ');
                    output.push(c);
                    i += c.len_utf8();
                }
            }
        }
    }
    output
}


// ------------------------------------------------------------------- emoji

/// The emoji lists of `_wp_emoji_list()`, read from WordPress's `formatting.php`: entities and partials.
fn emoji_lists(formatting_php: &str) -> &'static (Vec<String>, Vec<String>) {
    static LISTS: OnceLock<(Vec<String>, Vec<String>)> = OnceLock::new();
    LISTS.get_or_init(|| {
        let list = |name: &str| -> Vec<String> {
            let Some(start) = formatting_php.find(&format!("${name} = array(")) else { return Vec::new() };
            let rest = &formatting_php[start..];
            let Some(end) = rest.find(");") else { return Vec::new() };
            re!(r"'([^']*)'").captures_iter(&rest[..end]).flatten().map(|c| c[1].to_owned()).collect()
        };
        (list("entities"), list("partials"))
    })
}

/// `html_entity_decode()` of an emoji entity such as `&#x1f468;&#x200d;&#x2764;`.
fn decode_emoji_entities(entity: &str) -> String {
    entity.split(';').filter_map(|part| part.strip_prefix("&#x")).filter_map(|hex| u32::from_str_radix(hex, 16).ok()).filter_map(char::from_u32).collect()
}

/// `wp_staticize_emoji()`: emoji characters become images (`the_content_feed`).
pub fn staticize_emoji(text: &str, formatting_php: &str) -> String {
    let (entities, partials) = emoji_lists(formatting_php);
    let mut text = text.to_owned();
    if !text.contains("&#x") {
        if text.is_ascii() {
            return text;
        }
        // `wp_encode_emoji()`
        let mut encoded = text.clone();
        for partial in partials {
            let ch = decode_emoji_entities(partial);
            if !ch.is_empty() && encoded.contains(&ch) {
                encoded = encoded.replace(&ch, partial);
            }
        }
        if encoded == text {
            return encoded;
        }
        text = encoded;
    }
    let possible: Vec<(&String, String)> = entities.iter().filter(|e| text.contains(e.as_str())).map(|e| (e, decode_emoji_entities(e))).collect();
    if possible.is_empty() {
        return text;
    }
    let cdn = "https://s.w.org/images/core/emoji/17.0.2/72x72/";
    let mut output = String::with_capacity(text.len());
    let mut ignore: Option<&str> = None;
    let mut rest = text.as_str();
    while !rest.is_empty() {
        let (chunk, is_tag) = if rest.starts_with('<') { rest.find('>').map_or((rest, false), |e| (&rest[..=e], true)) } else { (&rest[..rest.find('<').unwrap_or(rest.len())], false) };
        rest = &rest[chunk.len()..];
        if is_tag {
            if ignore.is_none() {
                ignore = ["code", "pre", "style", "script", "textarea"].into_iter().find(|n| chunk == format!("<{n}>"));
            }
            if let Some(name) = ignore
                && chunk == format!("</{name}>")
            {
                ignore = None;
            }
            output.push_str(chunk);
            continue;
        }
        let mut content = chunk.to_owned();
        if ignore.is_none() && content.contains("&#x") {
            for (entity, ch) in &possible {
                if !content.contains(entity.as_str()) {
                    continue;
                }
                let file = entity.replace(";&#x", "-").replace("&#x", "").replace(';', "");
                let image = format!("<img src=\"{cdn}{file}.png\" alt=\"{ch}\" class=\"wp-smiley\" style=\"height: 1em; max-height: 1em;\" />");
                content = content.replace(entity.as_str(), &image);
            }
        }
        output.push_str(&content);
    }
    output.replace("&#xfe0f;", "")
}
#[cfg(test)]
mod tests {
    use super::*;

    const OPTS: Clickable<'static> = Clickable { home_host: "localhost", comment: true };

    #[test]
    fn urls_and_emails_become_links() {
        assert_eq!(make_clickable("Link http://example.com/a, mehr", &OPTS), "Link <a href=\"http://example.com/a\" rel=\"nofollow ugc\">http://example.com/a</a>, mehr");
        assert_eq!(make_clickable("<a href=\"http://x.y/\">http://x.y/</a>", &OPTS), "<a href=\"http://x.y/\">http://x.y/</a>");
        assert!(make_clickable("a@b.de", &OPTS).contains("mailto:a@b.de"));
    }

    #[test]
    fn smilies_need_spaces_around_them() {
        assert_eq!(convert_smilies("Hallo :) Welt", ""), "Hallo \u{1F642} Welt");
        assert_eq!(convert_smilies("Ohne:)Leerzeichen", ""), "Ohne:)Leerzeichen");
        assert_eq!(convert_smilies("<code>:)</code> :)", ""), "<code>:)</code> \u{1F642}");
    }
}
