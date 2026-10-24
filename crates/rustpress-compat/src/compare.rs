//! Comparing one response of RustPress with the one of the reference WordPress.

use crate::http::Response;
use crate::urls::{Kind, to_path};

/// One way in which the two responses differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// `status`, `header:<name>`, `title`, `body-class`, `head-ids`, `block-supports-css`, `body`, `bytes`.
    pub aspect: String,
    pub detail: String,
}

impl Difference {
    fn new(aspect: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { aspect: aspect.into(), detail: detail.into() }
    }
}

/// Headers that say something about the content or its caching.
const HEADERS: &[&str] = &["content-type", "location", "cache-control", "expires", "etag", "last-modified", "link", "x-wp-total", "x-wp-totalpages", "x-robots-tag", "x-pingback", "allow"];

fn header_value(response: &Response, name: &str) -> Option<String> {
    let value = response.header(name)?;
    Some(if name == "location" { to_path(value).unwrap_or_else(|| value.to_owned()) } else if name == "link" { strip_origin(value) } else { value.to_owned() })
}

/// `Link` headers carry absolute URLs; only the path matters here.
fn strip_origin(link: &str) -> String {
    let mut out = String::new();
    let mut rest = link;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..=start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('>') else { rest = after; break };
        out.push_str(&to_path(&after[..end]).unwrap_or_else(|| after[..end].to_owned()));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

pub fn compare(kind: Kind, ours: &Response, theirs: &Response) -> Vec<Difference> {
    let mut out = Vec::new();
    if ours.status != theirs.status {
        out.push(Difference::new("status", format!("RustPress {} statt {}", ours.status, theirs.status)));
    }
    for name in HEADERS {
        let (a, b) = (header_value(ours, name), header_value(theirs, name));
        if a != b {
            out.push(Difference::new(format!("header:{name}"), format!("RustPress {} statt {}", show(&a), show(&b))));
        }
    }
    let redirect = (300..400).contains(&theirs.status) && (300..400).contains(&ours.status);
    if redirect || ours.status != theirs.status {
        return out;
    }
    match kind {
        Kind::Page if ours.media_type() == "text/html" && theirs.media_type() == "text/html" => {
            let (mine, theirs_text) = (unbrand(&ours.text()), unbrand(&theirs.text()));
            compare_documents(&mine, &theirs_text, &mut out);
            // Equal in every respect that matters, but not byte for byte.
            if out.iter().all(|d| d.aspect.starts_with("header:")) && mine != theirs_text {
                out.push(Difference::new("whitespace", describe_difference(mine.as_bytes(), theirs_text.as_bytes())));
            }
        }
        _ => {
            if ours.body != theirs.body {
                out.push(Difference::new("bytes", describe_difference(&ours.body, &theirs.body)));
            }
        }
    }
    out
}

/// The credit line of Twenty Seventeen names RustPress instead of WordPress on purpose.
fn unbrand(html: &str) -> String {
    static CREDIT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r#"(?s)<a href="[^"]*" class="imprint">.*?</a>"#).unwrap());
    CREDIT.replace_all(html, "<a class=\"imprint\"></a>").into_owned()
}

fn show(value: &Option<String>) -> String {
    value.as_ref().map_or_else(|| "(fehlt)".to_owned(), |v| format!("`{v}`"))
}

fn between<'a>(html: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = html.find(open)? + open.len();
    let end = html[start..].find(close)? + start;
    Some(&html[start..end])
}

/// Every `id` on style, link and script tags of the head, in order.
pub fn head_ids(html: &str) -> Vec<String> {
    let head = between(html, "<head>", "</head>").unwrap_or_default();
    let mut ids = Vec::new();
    let mut rest = head;
    while let Some(i) = rest.find('<') {
        rest = &rest[i + 1..];
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        if tag.starts_with("style") || tag.starts_with("link") || tag.starts_with("script") {
            for quote in ['"', '\''] {
                if let Some(p) = tag.find(&format!(" id={quote}")) {
                    let value = &tag[p + 5..];
                    if let Some(e) = value.find(quote) {
                        ids.push(value[..e].to_owned());
                        break;
                    }
                }
            }
        }
    }
    ids
}

/// The text of `<style id="...">` in the head.
pub fn style_text(html: &str, id: &str) -> Option<String> {
    let head = between(html, "<head>", "</head>")?;
    let open = [format!("<style id='{id}'>"), format!("<style id=\"{id}\">")].into_iter().find(|o| head.contains(o.as_str()))?;
    let start = head.find(&open)? + open.len();
    Some(head[start..start + head[start..].find("</style>")?].to_owned())
}

/// From the skip link to the end of `div.wp-site-blocks`.
pub fn body_region(html: &str) -> String {
    let start = html.find("<a class=\"skip-link").or_else(|| html.find("<div class=\"wp-site-blocks\">")).unwrap_or(0);
    let end = html.find("<script type=\"speculationrules\">").or_else(|| html.rfind("</body>")).unwrap_or(html.len());
    html[start..end.max(start)].to_owned()
}

/// The elements of a head as `(key, markup)`: `<tag id=..>` for elements with an `id`,
/// otherwise the tag with its attributes.
pub fn head_elements(head: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = head;
    while let Some(i) = rest.find('<') {
        rest = &rest[i..];
        if rest.starts_with("<!--") {
            rest = &rest[rest.find("-->").map_or(rest.len(), |e| e + 3)..];
            continue;
        }
        let Some(open_end) = rest.find('>') else { break };
        let open = &rest[..=open_end];
        let name: String = open[1..].chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        let (element, after) = if matches!(name.as_str(), "style" | "script" | "title") {
            let close = format!("</{name}>");
            let end = rest.find(&close).map_or(rest.len(), |e| e + close.len());
            (&rest[..end], &rest[end..])
        } else {
            (open, &rest[open_end + 1..])
        };
        let id = open.split(" id=").nth(1).and_then(|v| v.chars().next().filter(|q| *q == '"' || *q == '\'').map(|q| v[1..].split(q).next().unwrap_or_default().to_owned()));
        let key = match id {
            Some(id) => format!("<{name} id={id}>"),
            None if name == "style" || name == "script" => format!("<{name}> {}", element.chars().take(60).collect::<String>().replace('\n', " ")),
            None => open.to_owned(),
        };
        out.push((key, element.to_owned()));
        rest = after;
    }
    out
}

/// Everything after the template: speculation rules, script modules, emoji loader.
pub fn footer_region(html: &str) -> String {
    let start = html.find("<script type=\"speculationrules\">").or_else(|| html.find("<script fetchpriority=\"low\"")).or_else(|| html.rfind("</body>")).unwrap_or(html.len());
    html[start..].to_owned()
}

/// Collapses whitespace runs; whitespace between tags is irrelevant.
pub fn normalize(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut last_space = false;
    for c in html.chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out.replace("> <", "><").trim().to_owned()
}

fn compare_documents(ours: &str, theirs: &str, out: &mut Vec<Difference>) {
    let (a, b) = (between(ours, "<title>", "</title>"), between(theirs, "<title>", "</title>"));
    if a != b {
        out.push(Difference::new("title", format!("RustPress {a:?} statt {b:?}")));
    }
    let (a, b) = (between(ours, "<body class=\"", "\""), between(theirs, "<body class=\"", "\""));
    if a != b {
        out.push(Difference::new("body-class", format!("RustPress {a:?} statt {b:?}")));
    }
    let (a, b) = (head_ids(ours), head_ids(theirs));
    if a != b {
        let missing: Vec<&String> = b.iter().filter(|i| !a.contains(i)).collect();
        let extra: Vec<&String> = a.iter().filter(|i| !b.contains(i)).collect();
        let detail = if missing.is_empty() && extra.is_empty() { "andere Reihenfolge".to_owned() } else { format!("fehlt: {missing:?}, zu viel: {extra:?}") };
        out.push(Difference::new("head-ids", detail));
    }
    let id = "core-block-supports-inline-css";
    let (a, b) = (style_text(ours, id), style_text(theirs, id));
    if a != b {
        out.push(Difference::new("block-supports-css", describe_difference(a.unwrap_or_default().as_bytes(), b.unwrap_or_default().as_bytes())));
    }
    let (a, b) = (normalize(&body_region(ours)), normalize(&body_region(theirs)));
    if a != b {
        out.push(Difference::new("body", describe_difference(a.as_bytes(), b.as_bytes())));
    }
    // The rest of the head, element by element.
    let (a, b) = (head_elements(between(ours, "<head>", "</head>").unwrap_or_default()), head_elements(between(theirs, "<head>", "</head>").unwrap_or_default()));
    for (key, text) in &b {
        match a.iter().find(|(k, _)| k == key) {
            None => out.push(Difference::new("head", format!("fehlt: {key}"))),
            Some((_, mine)) if normalize(mine) != normalize(text) => out.push(Difference::new("head", format!("{key} ist anders:
{}", describe_difference(normalize(mine).as_bytes(), normalize(text).as_bytes())))),
            _ => {}
        }
    }
    for (key, _) in &a {
        if !b.iter().any(|(k, _)| k == key) {
            out.push(Difference::new("head", format!("zu viel: {key}")));
        }
    }
    let (a, b) = (normalize(&footer_region(ours)), normalize(&footer_region(theirs)));
    if a != b {
        out.push(Difference::new("footer", describe_difference(a.as_bytes(), b.as_bytes())));
    }
}

/// Where two byte strings first differ, with a little context from both.
pub fn describe_difference(ours: &[u8], theirs: &[u8]) -> String {
    let at = ours.iter().zip(theirs).position(|(a, b)| a != b).unwrap_or(ours.len().min(theirs.len()));
    let window = |bytes: &[u8]| {
        let start = at.saturating_sub(60);
        let end = (at + 100).min(bytes.len());
        String::from_utf8_lossy(&bytes[start.min(bytes.len())..end]).replace('\n', "⏎").replace('`', "'")
    };
    format!("{} statt {} Bytes, erster Unterschied bei {at}:\nRustPress: …{}…\nWordPress: …{}…", ours.len(), theirs.len(), window(ours), window(theirs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(status: u16, headers: &[(&str, &str)], body: &str) -> Response {
        Response { status, headers: headers.iter().map(|(n, v)| ((*n).to_owned(), (*v).to_owned())).collect(), body: body.as_bytes().to_vec() }
    }

    #[test]
    fn identical_responses_have_no_differences() {
        let r = response(200, &[("content-type", "text/xml")], "<a/>");
        assert!(compare(Kind::Feed, &r, &r.clone()).is_empty());
    }

    #[test]
    fn status_headers_and_bytes_are_reported() {
        let ours = response(200, &[("content-type", "text/xml")], "<a/>");
        let theirs = response(200, &[("content-type", "application/xml")], "<b/>");
        let aspects: Vec<String> = compare(Kind::Feed, &ours, &theirs).into_iter().map(|d| d.aspect).collect();
        assert_eq!(aspects, ["header:content-type", "bytes"]);
        let moved = response(404, &[], "");
        assert_eq!(compare(Kind::Feed, &moved, &theirs)[0].aspect, "status");
    }

    #[test]
    fn redirect_targets_are_compared_as_paths() {
        let ours = response(301, &[("location", "/x/")], "");
        let theirs = response(301, &[("location", "http://localhost:8080/x/")], "redirecting");
        assert!(compare(Kind::Page, &ours, &theirs).is_empty());
        let other = response(301, &[("location", "/y/")], "");
        assert_eq!(compare(Kind::Page, &ours, &other)[0].aspect, "header:location");
    }

    #[test]
    fn link_headers_ignore_the_origin() {
        let ours = response(200, &[("link", "</wp-json/>; rel=\"https://api.w.org/\"")], "");
        let theirs = response(200, &[("link", "<http://localhost:8080/wp-json/>; rel=\"https://api.w.org/\"")], "");
        assert!(compare(Kind::Rest, &ours, &theirs).is_empty());
    }

    #[test]
    fn documents_differ_in_the_parts_that_matter() {
        let page = |title: &str, class: &str, body: &str| {
            let html = format!("<!DOCTYPE html><html><head><title>{title}</title><style id='a'>x</style></head><body class=\"{class}\"><div class=\"wp-site-blocks\">{body}</div></body></html>");
            response(200, &[("content-type", "text/html; charset=UTF-8")], &html)
        };
        let base = page("T", "home", "<p>a</p>");
        assert!(compare(Kind::Page, &base, &page("T", "home", "<p>a</p>")).is_empty());
        // Whitespace between tags is reported on its own, not as a different body.
        let aspects: Vec<String> = compare(Kind::Page, &base, &page("T", "home", "<p>a</p>\n\n")).into_iter().map(|d| d.aspect).collect();
        assert_eq!(aspects, ["whitespace"]);
        let aspects: Vec<String> = compare(Kind::Page, &base, &page("U", "blog", "<p>b</p>")).into_iter().map(|d| d.aspect).collect();
        // The title is also an element of the head, so the head differs too.
        assert_eq!(aspects, ["title", "body-class", "body", "head"]);
    }

    #[test]
    fn differences_point_at_the_first_changed_byte() {
        let text = describe_difference(b"abcdef", b"abcxef");
        assert!(text.contains("erster Unterschied bei 3"), "{text}");
    }
}
