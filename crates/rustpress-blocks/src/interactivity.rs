//! The server-side part of the Interactivity API (`wp_interactivity_process_directives`)
//! as far as core blocks need it: `data-wp-bind--*` directives whose state is
//! not defined on the server remove the bound attribute.

use crate::html::{TagEditor, tags};

/// Removes `NAME` from every tag that carries `data-wp-bind--NAME`, because the
/// referenced state has no server-side value (it is `null`, so the attribute
/// is dropped). The directive attribute itself stays for the client.
pub fn process_directives(html: &str) -> String {
    if !html.contains("data-wp-bind--") {
        return html.to_owned();
    }
    let mut out = html.to_owned();
    let all = tags(&out, 0);
    // Back to front, so earlier byte offsets stay valid.
    for tag in all.into_iter().rev() {
        if tag.is_closer {
            continue;
        }
        let bound: Vec<String> = tag
            .attrs
            .iter()
            .filter_map(|a| a.name.strip_prefix("data-wp-bind--").map(str::to_owned))
            .filter(|name| tag.attr(name).is_some())
            .collect();
        if bound.is_empty() {
            continue;
        }
        let mut editor = TagEditor::from_tag(&out, tag);
        for name in bound {
            editor.remove_attribute(&name);
        }
        out = editor.finish();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bound_attributes_without_server_state_are_removed() {
        let html = "<button data-wp-bind--aria-expanded=\"state.isSubmenuOpen\" aria-label=\"x\" aria-expanded=\"false\" class=\"c\" ><svg></svg></button>";
        assert_eq!(
            process_directives(html),
            "<button data-wp-bind--aria-expanded=\"state.isSubmenuOpen\" aria-label=\"x\"  class=\"c\" ><svg></svg></button>"
        );
        // A trailing attribute leaves the space before `>` like WordPress does.
        let html = "<a class=\"c\" data-wp-bind--aria-expanded=\"state.x\" aria-expanded=\"false\">t</a>";
        assert_eq!(process_directives(html), "<a class=\"c\" data-wp-bind--aria-expanded=\"state.x\" >t</a>");
    }

    #[test]
    fn markup_without_directives_is_unchanged() {
        assert_eq!(process_directives("<p aria-expanded=\"false\">x</p>"), "<p aria-expanded=\"false\">x</p>");
    }
}
