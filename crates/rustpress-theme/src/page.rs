//! Rendering a request with the block theme: template selection and the
//! template's body HTML (`get_the_block_template_html()`).

use rustpress_blocks::env::State;
use rustpress_blocks::html::{TagEditor, first_start_tag, tags};
use rustpress_blocks::{Env, Registry};
use rustpress_core::formatting::{esc_html, esc_url, wptexturize};
use rustpress_core::hierarchy::template_candidates;
use rustpress_core::{Queried, QueryResult, Site};

use crate::Result;
use crate::theme::Theme;

/// The rendered template body and what rendering collected.
pub struct RenderedBody {
    pub template_slug: String,
    /// `<div class="wp-site-blocks">...</div>` with the skip link in front.
    pub html: String,
    pub state: State,
}

/// The `_wp_page_template` meta of the queried page, if any.
async fn page_template(site: &Site, query: &QueryResult) -> Result<Option<String>> {
    if !query.flags.is_page {
        return Ok(None);
    }
    let Queried::Post(page) = &query.queried else { return Ok(None) };
    let meta = site.db.post_meta(page.id).await?;
    Ok(meta.into_iter().find(|m| m.meta_key.as_deref() == Some("_wp_page_template")).and_then(|m| m.meta_value))
}

pub async fn render_body(theme: &Theme, site: &Site, query: &QueryResult, registry: &Registry) -> Result<RenderedBody> {
    let custom = page_template(site, query).await?;
    let candidates = template_candidates(query, &site.settings, custom.as_deref());
    let Some((slug, markup)) = theme.pick_template(&candidates) else {
        return Ok(RenderedBody { template_slug: String::new(), html: String::new(), state: State::default() });
    };

    let env = Env::new(site, query, theme, registry);
    let root = env.root_context();
    // A singular template renders inside the main loop, even when it has no query block.
    let in_loop = query.flags.is_singular() && query.posts.len() == 1;
    env.with_state(|s| s.media.in_main_loop = in_loop);
    let content = env.render_markup(&markup, &root).await?;
    env.with_state(|s| s.media.in_main_loop = false);
    let content = wptexturize(&content);
    let content = rustpress_blocks::images::filter_content_tags(&env, &content, "template").await?.replace("]]>", "]]&gt;");
    let wrapped = format!("<div class=\"wp-site-blocks\">{content}</div>");
    let html = add_skip_link(&wrapped);
    Ok(RenderedBody { template_slug: slug, html, state: env.take_state() })
}

/// `_block_template_add_skip_link`: gives the first `<main>` an id and puts a
/// link to it in front of `div.wp-site-blocks`.
pub fn add_skip_link(template_html: &str) -> String {
    let all = tags(template_html, 0);
    let Some(wrapper) = all.iter().find(|t| !t.is_closer && t.name == "div" && t.attr("class").is_some_and(|c| c.split_whitespace().any(|x| x == "wp-site-blocks"))) else {
        return template_html.to_owned();
    };
    let wrapper_start = wrapper.start;
    let Some(main) = all.iter().find(|t| !t.is_closer && t.name == "main" && t.start >= wrapper.end) else {
        return template_html.to_owned();
    };
    let mut html = template_html.to_owned();
    let mut target_id = main.attr("id").filter(|id| !id.is_empty()).map(str::to_owned);
    if target_id.is_none() {
        let mut editor = TagEditor::from_tag(&html, main.clone());
        editor.set_attribute("id", "wp--skip-link--target");
        html = editor.finish();
        target_id = Some("wp--skip-link--target".to_owned());
    }
    let id = target_id.unwrap_or_default();
    let link = format!("<a class=\"skip-link screen-reader-text\" id=\"wp-skip-link\" href=\"{}\">{}</a>", esc_url(&format!("#{id}")), esc_html("Skip to content"));
    html.insert_str(wrapper_start, &link);
    let _ = first_start_tag;
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_link_targets_the_first_main() {
        let html = "<div class=\"wp-site-blocks\"><header>h</header><main class=\"m\">x</main></div>";
        assert_eq!(
            add_skip_link(html),
            "<a class=\"skip-link screen-reader-text\" id=\"wp-skip-link\" href=\"#wp--skip-link--target\">Skip to content</a><div class=\"wp-site-blocks\"><header>h</header><main id=\"wp--skip-link--target\" class=\"m\">x</main></div>"
        );
    }

    #[test]
    fn an_existing_main_id_is_reused() {
        let html = "<div class=\"wp-site-blocks\"><main id=\"content\">x</main></div>";
        assert!(add_skip_link(html).starts_with("<a class=\"skip-link screen-reader-text\" id=\"wp-skip-link\" href=\"#content\">"));
    }

    #[test]
    fn nothing_changes_without_a_main_element() {
        let html = "<div class=\"wp-site-blocks\"><p>x</p></div>";
        assert_eq!(add_skip_link(html), html);
    }
}
