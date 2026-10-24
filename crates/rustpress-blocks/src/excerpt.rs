//! Post excerpts: `get_the_excerpt()` through `wp_trim_excerpt()`.

use std::sync::Arc;

use rustpress_db::Post;

use crate::content::{context_for_post, get_the_content, has_blocks, post_password_required, the_content_for_excerpt};
use crate::env::{Context, Env};
use crate::parser::{Block, parse_blocks};
use crate::Result;

const INNER: &[&str] = &["core/freeform", "core/heading", "core/html", "core/list", "core/media-text", "core/paragraph", "core/preformatted", "core/pullquote", "core/quote", "core/table", "core/verse"];
const WRAPPERS: &[&str] = &["core/columns", "core/column", "core/group"];

/// `get_the_excerpt()`: the explicit excerpt, else the first 55 words of the content.
/// The result is plain text/HTML before any output filters (`the_excerpt`, `the_excerpt_rss`).
pub async fn get_the_excerpt(env: &Env<'_>, post: &Arc<Post>) -> Result<String> {
    let more = match env.theme.excerpt_more() {
        Some(template) => {
            let permalink = rustpress_core::formatting::esc_url(&env.site.permalink(post).await?);
            rustpress_core::i18n::sprintf(&template, &[&permalink, &crate::content::the_title_for(env.site, post)])
        }
        None => " [&hellip;]".to_owned(),
    };
    get_the_excerpt_with(env, post, env.site.excerpt_length(), &more).await
}

/// [`get_the_excerpt`] with the `excerpt_length` and `excerpt_more` filters applied.
pub async fn get_the_excerpt_with(env: &Env<'_>, post: &Arc<Post>, length: usize, more: &str) -> Result<String> {
    if post_password_required(post) {
        return Ok(env.site.tr("There is no excerpt because this is a protected post."));
    }
    if !post.post_excerpt.trim().is_empty() {
        return Ok(post.post_excerpt.clone());
    }
    let content = get_the_content(env, post, 1, true);
    let content = excerpt_remove_blocks(env, post, &content).await?;
    let html = the_content_for_excerpt(env, post, &content).await?.replace("]]>", "]]&gt;");
    Ok(rustpress_core::formatting::wp_trim_words_with(&html, length, more, env.site.locale_data().word_count_type))
}

fn allowed(name: &Option<String>) -> bool {
    name.as_deref().is_none_or(|n| INNER.contains(&n) || WRAPPERS.contains(&n))
}

fn hidden(block: &Block) -> bool {
    block.attrs.get("metadata").and_then(|m| m.get("blockVisibility")) == Some(&serde_json::Value::Bool(false))
}

/// `excerpt_remove_blocks()`: only text-like blocks survive.
pub async fn excerpt_remove_blocks(env: &Env<'_>, post: &Arc<Post>, content: &str) -> Result<String> {
    if !has_blocks(content) {
        return Ok(content.to_owned());
    }
    let context = context_for_post(post);
    let mut output = String::new();
    for block in parse_blocks(content) {
        if hidden(&block) || !allowed(&block.name) {
            continue;
        }
        if !block.inner_blocks.is_empty() {
            if block.name.as_deref().is_some_and(|n| WRAPPERS.contains(&n)) {
                output.push_str(&render_inner(env, &block, &context).await?);
                continue;
            }
            // Skip blocks with disallowed or nested inner blocks.
            if block.inner_blocks.iter().any(|b| !b.name.as_deref().is_none_or(|n| INNER.contains(&n)) || !b.inner_blocks.is_empty()) {
                continue;
            }
        }
        output.push_str(&env.render_block(&block, &context, None).await?);
    }
    Ok(output)
}

/// `_excerpt_render_inner_blocks()`.
async fn render_inner(env: &Env<'_>, block: &Block, context: &Context) -> Result<String> {
    let mut out = String::new();
    for child in &block.inner_blocks {
        if hidden(child) || !allowed(&child.name) {
            continue;
        }
        if child.inner_blocks.is_empty() {
            out.push_str(&env.render_block(child, context, None).await?);
        } else {
            out.push_str(&Box::pin(render_inner(env, child, context)).await?);
        }
    }
    Ok(out)
}
