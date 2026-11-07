//! The Underscore templates are baked from the reference (`dev/templates-bake.py`) with their texts marked:
//! `⟦kind¦context¦text⟧`, where the kind tells how WordPress escaped the text (`r`aw, `h`tml, `a`ttribute) and the text
//! may hold the values of a format as `⟪conversion¦value⟫`. This prints them in the language of the site.

use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_core::i18n::sprintf;

/// The end of what starts at `from` (behind an `open`), past the pairs nested in it.
fn closing(text: &str, from: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (at, c) in text[from..].char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            if depth == 0 {
                return Some(from + at);
            }
            depth -= 1;
        }
    }
    None
}

/// The text of a mark as it was given to `__()`, and the values of its conversions, in the order of the format.
fn message(site: &Site, body: &str) -> (String, Vec<String>) {
    let mut id = String::new();
    let mut values: Vec<(usize, String)> = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find('⟪') {
        id.push_str(&rest[..start]);
        let inner = start + '⟪'.len_utf8();
        let Some(end) = closing(rest, inner, '⟪', '⟫') else { break };
        let (conversion, value) = rest[inner..end].split_once('¦').unwrap_or((&rest[inner..end], ""));
        // Without `printf()` the format stayed: `%%s` and `%s`.
        match conversion.strip_prefix('%').filter(|c| c.starts_with('%')) {
            Some(plain) => id.push_str(plain),
            None => {
                id.push_str(conversion);
                let index = conversion[1..].split('$').next().and_then(|n| n.parse().ok()).unwrap_or(values.len() + 1);
                // A translated address has no mark: `esc_url()` would have dropped it.
                let value = if value.starts_with("http") { site.tr(value) } else { translate(site, value) };
                values.push((index, value));
            }
        }
        rest = &rest[end + '⟫'.len_utf8()..];
    }
    id.push_str(rest);
    values.sort_by_key(|(index, _)| *index);
    (id, values.into_iter().map(|(_, value)| value).collect())
}

/// Prints a baked template in the language of `site`.
pub fn translate(site: &Site, template: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('⟦') {
        out.push_str(&rest[..start]);
        let inner = start + '⟦'.len_utf8();
        let Some(end) = closing(rest, inner, '⟦', '⟧') else { break };
        let mut parts = rest[inner..end].splitn(3, '¦');
        let (kind, context, body) = (parts.next().unwrap_or("r"), parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
        let (id, values) = message(site, body);
        // An escaped mark holds the characters `esc_html()` encodes; other entities are part of the message.
        let id = if kind == "r" { id } else { id.replace("&quot;", "\"").replace("&#039;", "'").replace("&lt;", "<").replace("&gt;", ">") };
        let text = if context.is_empty() { site.tr(&id) } else { site.trx(&id, context) };
        let text = if values.is_empty() {
            text
        } else {
            let refs: Vec<&str> = values.iter().map(String::as_str).collect();
            sprintf(&text, &refs)
        };
        out.push_str(&match kind {
            "h" => esc_html(&text),
            "a" => esc_attr(&text),
            _ => text,
        });
        rest = &rest[end + '⟧'.len_utf8()..];
    }
    out.push_str(rest);
    out
}
