//! `core/template-part` and `core/pattern`.

use rustpress_core::formatting::esc_attr;

use crate::Result;
use crate::content::context_for_post;
use crate::env::Context;
use crate::render::BlockCtx;

/// The element for a template part area (`get_allowed_block_template_part_areas`).
fn area_tag(area: &str) -> &'static str {
    match area {
        "header" => "header",
        "footer" => "footer",
        _ => "div",
    }
}

/// The context a nested `do_blocks()` call starts with: the current global post.
fn root_context(bc: &BlockCtx<'_>) -> Context {
    match &bc.context.post {
        Some(post) => context_for_post(post),
        None => Context::default(),
    }
}

pub(super) async fn template_part(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(slug) = bc.attr_str("slug") else { return Ok(String::new()) };
    let stylesheet = &bc.env.site.settings.stylesheet;
    if bc.attr_str("theme").is_some_and(|t| t != stylesheet) {
        return Ok(String::new());
    }
    let id = format!("{stylesheet}//{slug}");
    let Some(part) = bc.env.theme.template_part(slug) else { return Ok(String::new()) };

    let area = if matches!(part.area.as_str(), "header" | "footer" | "navigation-overlay" | "uncategorized") { part.area.clone() } else { "uncategorized".to_owned() };
    let fresh = bc.env.with_state(|s| s.active_patterns.insert(format!("part:{id}")));
    if !fresh {
        return Ok(String::new());
    }
    let rendered = bc.env.render_markup(&part.content, &root_context(bc)).await;
    bc.env.with_state(|s| s.active_patterns.remove(&format!("part:{id}")));
    let content = bc.env.site.texturize(&rendered?);
    let content = crate::images::filter_content_tags(bc.env, &content, &format!("template_part_{area}")).await?;

    let requested = bc.attr_str("tagName").filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric()));
    let tag = requested.map_or_else(|| area_tag(&area).to_owned(), esc_attr);
    let wrapper = bc.wrapper_attributes(&[]);
    let content = content.replace("]]>", "]]&gt;");
    Ok(format!("<{tag} {wrapper}>{content}</{tag}>"))
}

pub(super) async fn pattern(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(slug) = bc.attr_str("slug").filter(|s| !s.is_empty()) else { return Ok(String::new()) };
    let Some(markup) = bc.env.theme.pattern(slug) else { return Ok(String::new()) };
    let key = format!("pattern:{slug}");
    if !bc.env.with_state(|s| s.active_patterns.insert(key.clone())) {
        return Ok(String::new());
    }
    let result = bc.env.render_markup(&markup, &root_context(bc)).await;
    bc.env.with_state(|s| s.active_patterns.remove(&key));
    result
}
