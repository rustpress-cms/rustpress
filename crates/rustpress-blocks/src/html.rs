//! A small stand-in for `WP_HTML_Tag_Processor`: finds start tags in an HTML
//! fragment and edits their attributes in place, leaving everything else
//! byte-for-byte untouched.
//!
//! Like WordPress, a modified existing attribute is rewritten as
//! `name="value"` at its original position, and a new attribute is inserted
//! right after the tag name.

use std::ops::Range;

/// One attribute of a tag as found in the source.
#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    /// Decoded value; `None` for boolean attributes like `<input disabled>`.
    pub value: Option<String>,
    /// Byte range of the whole attribute (`name="value"`) in the document.
    range: Range<usize>,
}

/// A start or end tag.
#[derive(Debug, Clone)]
pub struct Tag {
    /// Lowercased tag name.
    pub name: String,
    pub is_closer: bool,
    pub attrs: Vec<Attr>,
    /// Byte offset of `<`.
    pub start: usize,
    /// Byte offset just after `>`.
    pub end: usize,
    /// Offset right after the tag name (where new attributes are inserted).
    name_end: usize,
}

impl Tag {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
            .map(|a| a.value.as_deref().unwrap_or(""))
    }
}

/// Void elements never have an end tag.
pub fn is_void(name: &str) -> bool {
    matches!(
        name,
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta" | "source" | "track" | "wbr"
    )
}

/// All tags of `html` starting at byte `from`, in document order. Comments,
/// doctype and the contents of `script`/`style`/`textarea`/`title` are skipped.
pub fn tags(html: &str, from: usize) -> Vec<Tag> {
    let bytes = html.as_bytes();
    let mut out = Vec::new();
    let mut i = from;
    while i < bytes.len() {
        let Some(rel) = html[i..].find('<') else { break };
        let start = i + rel;
        let rest = &html[start..];
        if rest.starts_with("<!--") {
            i = rest.find("-->").map_or(bytes.len(), |e| start + e + 3);
            continue;
        }
        if rest.starts_with("<!") || rest.starts_with("<?") {
            i = rest.find('>').map_or(bytes.len(), |e| start + e + 1);
            continue;
        }
        let is_closer = rest.starts_with("</");
        let name_start = start + if is_closer { 2 } else { 1 };
        if name_start >= bytes.len() || !bytes[name_start].is_ascii_alphabetic() {
            i = start + 1;
            continue;
        }
        let mut j = name_start;
        while j < bytes.len() && !matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r' | 0x0c | b'/' | b'>') {
            j += 1;
        }
        let name = html[name_start..j].to_ascii_lowercase();
        let name_end = j;
        let mut attrs = Vec::new();
        // Attributes.
        loop {
            while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r' | 0x0c | b'/') {
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            if bytes[j] == b'>' {
                j += 1;
                break;
            }
            let attr_start = j;
            while j < bytes.len() && !matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r' | 0x0c | b'=' | b'>' | b'/') {
                j += 1;
            }
            if j == attr_start {
                j += 1;
                continue;
            }
            let attr_name = html[attr_start..j].to_ascii_lowercase();
            let mut k = j;
            while k < bytes.len() && matches!(bytes[k], b' ' | b'\t' | b'\n' | b'\r' | 0x0c) {
                k += 1;
            }
            let mut value = None;
            if k < bytes.len() && bytes[k] == b'=' {
                k += 1;
                while k < bytes.len() && matches!(bytes[k], b' ' | b'\t' | b'\n' | b'\r' | 0x0c) {
                    k += 1;
                }
                if k < bytes.len() && (bytes[k] == b'"' || bytes[k] == b'\'') {
                    let quote = bytes[k];
                    let vstart = k + 1;
                    let vend = bytes[vstart..].iter().position(|&b| b == quote).map_or(bytes.len(), |p| vstart + p);
                    value = Some(decode_entities(&html[vstart..vend]));
                    j = (vend + 1).min(bytes.len());
                } else {
                    let vstart = k;
                    let mut vend = vstart;
                    while vend < bytes.len() && !matches!(bytes[vend], b' ' | b'\t' | b'\n' | b'\r' | 0x0c | b'>') {
                        vend += 1;
                    }
                    value = Some(decode_entities(&html[vstart..vend]));
                    j = vend;
                }
            }
            attrs.push(Attr { name: attr_name, value, range: attr_start..j });
        }
        out.push(Tag { name: name.clone(), is_closer, attrs, start, end: j, name_end });
        i = j;
        if !is_closer && matches!(name.as_str(), "script" | "style" | "textarea" | "title") {
            let close = format!("</{name}");
            i = html[i..]
                .to_ascii_lowercase()
                .find(&close)
                .map_or(bytes.len(), |p| i + p);
        }
    }
    out
}

/// First start tag at or after `from`.
pub fn first_start_tag(html: &str, from: usize) -> Option<Tag> {
    tags(html, from).into_iter().find(|t| !t.is_closer)
}

/// Escapes a value for use inside a double-quoted attribute (`esc_attr`).
pub fn esc_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
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

/// `esc_html`.
pub fn esc_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
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

/// Decodes the entities that appear in attribute values.
pub fn decode_entities(value: &str) -> String {
    if !value.contains('&') {
        return value.to_owned();
    }
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(semi) = rest.find(';').filter(|&s| s <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Edits the attributes of one tag of a document, like `WP_HTML_Tag_Processor`:
/// changes are collected and applied together. An existing attribute is
/// rewritten in place; new attributes are inserted right after the tag name,
/// sorted by their text (so `class` precedes `style`, `data-a` precedes `data-b`).
pub struct TagEditor {
    html: String,
    tag: Tag,
    /// `(lowercase name, name as given, value)`.
    set: Vec<(String, String, String)>,
    removed: Vec<String>,
}

impl TagEditor {
    /// Edits the first start tag; `None` if the fragment has no tags.
    pub fn first(html: &str) -> Option<Self> {
        Self::at(html, 0)
    }

    /// Edits the first start tag at or after `from`.
    pub fn at(html: &str, from: usize) -> Option<Self> {
        first_start_tag(html, from).map(|tag| Self::from_tag(html, tag))
    }

    pub fn from_tag(html: &str, tag: Tag) -> Self {
        Self { html: html.to_owned(), tag, set: Vec::new(), removed: Vec::new() }
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    /// The document with all pending changes applied.
    pub fn current_html(&self) -> String {
        self.render()
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        let lower = name.to_ascii_lowercase();
        if let Some((_, _, value)) = self.set.iter().find(|(l, _, _)| *l == lower) {
            return Some(value);
        }
        if self.removed.contains(&lower) {
            return None;
        }
        self.tag.attr(name)
    }

    /// `set_attribute`.
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        let lower = name.to_ascii_lowercase();
        self.removed.retain(|r| *r != lower);
        match self.set.iter_mut().find(|(l, _, _)| *l == lower) {
            Some(entry) => entry.2 = value.to_owned(),
            None => self.set.push((lower, name.to_owned(), value.to_owned())),
        }
    }

    /// `add_class`: appends to an existing `class` attribute (once), or creates it.
    pub fn add_class(&mut self, class: &str) {
        match self.get_attribute("class").map(str::to_owned) {
            Some(existing) => {
                if existing.split_ascii_whitespace().any(|c| c == class) {
                    return;
                }
                let updated = if existing.is_empty() { class.to_owned() } else { format!("{existing} {class}") };
                self.set_attribute("class", &updated);
            }
            None => self.set_attribute("class", class),
        }
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.get_attribute("class").is_some_and(|c| c.split_ascii_whitespace().any(|x| x == class))
    }

    pub fn remove_attribute(&mut self, name: &str) {
        let lower = name.to_ascii_lowercase();
        self.set.retain(|(l, _, _)| *l != lower);
        if self.tag.attrs.iter().any(|a| a.name == lower) && !self.removed.contains(&lower) {
            self.removed.push(lower);
        }
    }

    fn render(&self) -> String {
        // (start, end, text) edits on the original document.
        let mut edits: Vec<(usize, usize, String)> = Vec::new();
        let mut inserts: Vec<String> = Vec::new();
        for (lower, name, value) in &self.set {
            let text = format!("{name}=\"{}\"", esc_attr(value));
            match self.tag.attrs.iter().find(|a| a.name == *lower) {
                Some(attr) => edits.push((attr.range.start, attr.range.end, text)),
                None => inserts.push(format!(" {text}")),
            }
        }
        for lower in &self.removed {
            if let Some(attr) = self.tag.attrs.iter().find(|a| a.name == *lower) {
                edits.push((attr.range.start, attr.range.end, String::new()));
            }
        }
        if !inserts.is_empty() {
            inserts.sort();
            edits.push((self.tag.name_end, self.tag.name_end, inserts.concat()));
        }
        edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        let mut out = self.html.clone();
        for (start, end, text) in edits {
            out.replace_range(start..end, &text);
        }
        out
    }

    pub fn finish(self) -> String {
        self.render()
    }
}

/// Sets or appends to the `style` attribute of the first tag
/// (`existing;new` with a separating `;` if needed).
pub fn append_style(editor: &mut TagEditor, css: &str) {
    if css.is_empty() {
        return;
    }
    let updated = match editor.get_attribute("style") {
        Some(existing) if !existing.is_empty() => {
            if existing.ends_with(';') { format!("{existing}{css}") } else { format!("{existing};{css}") }
        }
        _ => css.to_owned(),
    };
    editor.set_attribute("style", &updated);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_tags_and_attributes() {
        let t = first_start_tag("  <div class=\"a b\" id='x' hidden data-n=5>text</div>", 0).unwrap();
        assert_eq!(t.name, "div");
        assert_eq!(t.attr("class"), Some("a b"));
        assert_eq!(t.attr("id"), Some("x"));
        assert_eq!(t.attr("hidden"), Some(""));
        assert_eq!(t.attr("data-n"), Some("5"));
        assert_eq!(t.attr("nope"), None);
    }

    #[test]
    fn skips_comments_and_closers() {
        let html = "<!-- <b> --></p><span>x</span>";
        assert_eq!(first_start_tag(html, 0).unwrap().name, "span");
        let all = tags(html, 0);
        assert_eq!(all.iter().map(|t| (t.name.as_str(), t.is_closer)).collect::<Vec<_>>(), [("p", true), ("span", false), ("span", true)]);
    }

    #[test]
    fn add_class_appends_or_creates() {
        let mut e = TagEditor::first("<p class=\"a\">x</p>").unwrap();
        e.add_class("b");
        e.add_class("a");
        assert_eq!(e.finish(), "<p class=\"a b\">x</p>");

        let mut e = TagEditor::first("<hr/>").unwrap();
        e.add_class("c");
        assert_eq!(e.finish(), "<hr class=\"c\"/>");

        let mut e = TagEditor::first("<div style=\"x:y\">").unwrap();
        e.add_class("c");
        assert_eq!(e.finish(), "<div class=\"c\" style=\"x:y\">");
    }

    #[test]
    fn set_attribute_replaces_in_place_and_normalises_quotes() {
        let mut e = TagEditor::first("<a href='u' class=\"k\" rel=nofollow>x</a>").unwrap();
        e.set_attribute("href", "https://e.com/?a=1&b=2");
        e.set_attribute("title", "say \"hi\"");
        assert_eq!(
            e.finish(),
            "<a title=\"say &quot;hi&quot;\" href=\"https://e.com/?a=1&amp;b=2\" class=\"k\" rel=nofollow>x</a>"
        );
    }

    #[test]
    fn style_is_appended_with_a_separator() {
        let mut e = TagEditor::first("<div style=\"a:b\">").unwrap();
        append_style(&mut e, "c:d;");
        assert_eq!(e.finish(), "<div style=\"a:b;c:d;\">");
        let mut e = TagEditor::first("<div>").unwrap();
        append_style(&mut e, "c:d;");
        assert_eq!(e.finish(), "<div style=\"c:d;\">");
    }

    #[test]
    fn content_after_the_tag_is_untouched() {
        let html = "<div class=\"x\">\n\t<p>äö</p>\n</div>";
        let mut e = TagEditor::first(html).unwrap();
        e.add_class("y");
        assert_eq!(e.finish(), "<div class=\"x y\">\n\t<p>äö</p>\n</div>");
    }

    #[test]
    fn script_and_style_bodies_are_opaque() {
        let all = tags("<script>if (a<b) {}</script><i>", 0);
        assert_eq!(all.iter().filter(|t| !t.is_closer).map(|t| t.name.as_str()).collect::<Vec<_>>(), ["script", "i"]);
    }

    #[test]
    fn entities_are_decoded_in_values() {
        let t = first_start_tag("<a title=\"a &amp; b &#8211; &lt;\">", 0).unwrap();
        assert_eq!(t.attr("title"), Some("a & b – <"));
    }

    #[test]
    fn new_attributes_are_inserted_sorted_after_the_tag_name() {
        let mut e = TagEditor::first("<li class=\"x\">").unwrap();
        e.set_attribute("tabindex", "-1");
        e.set_attribute("data-wp-watch", "w");
        e.set_attribute("data-wp-context", "c");
        e.add_class("y");
        assert_eq!(e.finish(), "<li data-wp-context=\"c\" data-wp-watch=\"w\" tabindex=\"-1\" class=\"x y\">");
        let mut e = TagEditor::first("<div>").unwrap();
        e.set_attribute("style", "a:b");
        e.add_class("k");
        assert_eq!(e.finish(), "<div class=\"k\" style=\"a:b\">");
    }

    #[test]
    fn remove_attribute_leaves_surrounding_spaces() {
        let mut e = TagEditor::first("<div id=\"a\" class=\"b\">").unwrap();
        e.remove_attribute("id");
        assert_eq!(e.finish(), "<div  class=\"b\">");
    }
}
