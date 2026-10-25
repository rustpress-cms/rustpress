//! `core/post-content`.

use crate::Result;
use crate::content::{get_the_content, the_content};
use crate::render::BlockCtx;

use super::global_post;

pub(super) async fn render(bc: &BlockCtx<'_>) -> Result<String> {
    if bc.context.post_id().is_none() {
        return Ok(String::new());
    }
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    let guard = format!("post-content:{}", post.id);
    if !bc.env.with_state(|s| s.active_patterns.insert(guard.clone())) {
        return Ok(String::new());
    }

    // In the main loop of a singular request the `page` query var selects the page of a split post.
    let q = bc.env.query;
    let is_main_post = q.flags.is_singular() && q.queried_post().is_some_and(|p| p.id == post.id);
    let page = if is_main_post { q.vars.u64("page").unwrap_or(1) } else { 1 };
    let raw = get_the_content(bc.env, &post, page, is_main_post).await?;
    let content = the_content(bc.env, &post, &raw).await;
    bc.env.with_state(|s| s.active_patterns.remove(&guard));
    let content = content?;
    if content.is_empty() {
        return Ok(String::new());
    }

    let tag = bc
        .attr_str("tagName")
        .map(str::to_ascii_lowercase)
        .filter(|t| matches!(t.as_str(), "div" | "main" | "section" | "article"))
        .unwrap_or_else(|| "div".to_owned());
    let wrapper = bc.wrapper_attributes(&[("class", "entry-content".to_owned())]);
    Ok(format!("<{tag} {wrapper}>{content}</{tag}>"))
}
