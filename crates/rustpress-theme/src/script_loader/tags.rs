//! `<script>` and `<style>` elements as `WP_HTML_Tag_Processor` writes them: attributes are inserted
//! after the tag name sorted by their text, and values are escaped by the rules of
//! `set_attribute()` (URLs with `esc_url()`, everything else by replacing five characters).

use rustpress_core::formatting::esc_url;

/// An attribute value: text, or a boolean attribute that is written without a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attr {
    Text(String),
    Flag,
}

/// The attributes `wp_kses_uri_attributes()` lists.
fn is_uri_attribute(name: &str) -> bool {
    matches!(
        name,
        "action" | "archive" | "background" | "cite" | "classid" | "codebase" | "data" | "formaction" | "href" | "icon" | "longdesc" | "manifest" | "poster" | "profile" | "src" | "usemap" | "xmlns"
    )
}

/// The escaped text of an attribute value.
pub fn escape_attribute(name: &str, value: &str) -> String {
    if is_uri_attribute(&name.to_ascii_lowercase()) {
        return esc_url(value);
    }
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// The attribute text of a tag made by setting `attrs` on a bare tag, in the order the processor writes it.
pub fn attribute_text(attrs: &[(&str, Attr)]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (index, (name, value)) in attrs.iter().enumerate() {
        // The processor keeps the first value of a repeated name.
        if attrs[..index].iter().any(|(earlier, _)| earlier.eq_ignore_ascii_case(name)) {
            continue;
        }
        parts.push(match value {
            Attr::Flag => format!(" {name}"),
            Attr::Text(text) => format!(" {name}=\"{}\"", escape_attribute(name, text)),
        });
    }
    parts.sort();
    parts.concat()
}

/// `wp_get_script_tag()`.
pub fn script_tag(attrs: &[(&str, Attr)]) -> String {
    format!("<script{}></script>\n", attribute_text(attrs))
}

/// `wp_get_inline_script_tag()`.
pub fn inline_script_tag(data: &str, attrs: &[(&str, Attr)]) -> String {
    format!("<script{}>\n{}\n</script>\n", attribute_text(attrs), data.trim_matches(['\n', '\r', ' ']))
}

/// A `<style>` element with an `id` and its text, like `WP_Styles` makes them (without the final newline).
pub fn inline_style_tag(id: &str, css: &str) -> String {
    format!("<style{}>\n{css}\n</style>", attribute_text(&[("id", Attr::Text(id.to_owned()))]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_are_sorted_and_urls_escaped_like_wordpress() {
        let tag = script_tag(&[("src", Attr::Text("http://x/load.php?c=0&load=a,b".into())), ("id", Attr::Text("a-js".into())), ("async", Attr::Flag), ("data-x", Attr::Text("it's".into()))]);
        assert_eq!(tag, "<script async data-x=\"it&apos;s\" id=\"a-js\" src=\"http://x/load.php?c=0&#038;load=a,b\"></script>\n");
    }

    #[test]
    fn inline_scripts_are_trimmed_and_wrapped() {
        assert_eq!(inline_script_tag("\nvar a = 1;\n", &[("id", Attr::Text("x-js-extra".into()))]), "<script id=\"x-js-extra\">\nvar a = 1;\n</script>\n");
        assert_eq!(inline_script_tag("var a;", &[]), "<script>\nvar a;\n</script>\n");
    }
}
