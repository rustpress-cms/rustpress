//! Small blocks: site title, tagline, logo and the class-adding blocks.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

use crate::Result;
use crate::html::{TagEditor, tags};
use crate::render::BlockCtx;

use super::heading_tag;

/// Adds `class` to the first tag whose name is in `names` (`next_tag()` loop in
/// the heading, list and paragraph blocks).
pub(super) fn add_class_to_first(content: &str, names: &[&str], class: &str) -> String {
    if content.is_empty() {
        return String::new();
    }
    for tag in tags(content, 0) {
        if !tag.is_closer && names.contains(&tag.name.as_str()) {
            let mut editor = TagEditor::from_tag(content, tag);
            editor.add_class(class);
            return editor.finish();
        }
    }
    content.to_owned()
}

fn text_align_class(bc: &BlockCtx<'_>) -> String {
    bc.attr_str("textAlign").filter(|a| !a.is_empty()).map(|a| format!("has-text-align-{a}")).unwrap_or_default()
}

fn has_link_color(bc: &BlockCtx<'_>) -> bool {
    bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null())
}

pub(super) fn site_title(bc: &BlockCtx<'_>) -> Result<String> {
    let title = bc.env.site.settings.blogname.clone();
    if title.trim().is_empty() {
        return Ok(String::new());
    }
    let mut classes = text_align_class(bc);
    if has_link_color(bc) {
        classes.push_str(" has-link-color");
    }
    let tag = heading_tag(bc.attrs.get("level").and_then(|v| v.as_u64()), "h1");
    let is_link = bc.attr_bool("isLink");
    let inner = if is_link {
        let q = bc.env.query;
        let queried_id = q.queried_object_id();
        let settings = &bc.env.site.settings;
        let current = !q.flags.is_paged && (q.flags.is_front_page || (q.flags.is_home && settings.page_for_posts != queried_id));
        let aria = if current { " aria-current=\"page\"" } else { "" };
        let target = bc.attr_str("linkTarget").filter(|t| !t.is_empty()).unwrap_or("_self");
        format!(
            "<a href=\"{}\" target=\"{}\" rel=\"home\"{aria}>{}</a>",
            esc_url(&bc.env.site.home_url("")),
            esc_attr(target),
            esc_html(&title)
        )
    } else {
        esc_html(&title)
    };
    let wrapper = bc.wrapper_attributes(&[("class", classes.trim().to_owned())]);
    Ok(format!("<{tag} {wrapper}>{inner}</{tag}>"))
}

pub(super) fn site_tagline(bc: &BlockCtx<'_>) -> Result<String> {
    let tagline = bc.env.site.settings.blogdescription.clone();
    if tagline.is_empty() {
        return Ok(String::new());
    }
    let wrapper = bc.wrapper_attributes(&[("class", text_align_class(bc))]);
    let tag = match bc.attrs.get("level").and_then(|v| v.as_u64()) {
        Some(0) | None => "p".to_owned(),
        Some(n) => format!("h{n}"),
    };
    Ok(format!("<{tag} {wrapper}>{tagline}</{tag}>"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_goes_to_the_first_matching_tag() {
        assert_eq!(add_class_to_first("\n<h3 class=\"a\">T</h3>\n", &["h1", "h2", "h3"], "wp-block-heading"), "\n<h3 class=\"a wp-block-heading\">T</h3>\n");
        assert_eq!(add_class_to_first("<div><ul><li>x</li></ul></div>", &["ul", "ol"], "wp-block-list"), "<div><ul class=\"wp-block-list\"><li>x</li></ul></div>");
        assert_eq!(add_class_to_first("<span>x</span>", &["p"], "c"), "<span>x</span>");
        assert_eq!(add_class_to_first("", &["p"], "c"), "");
    }
}
