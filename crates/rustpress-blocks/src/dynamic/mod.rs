//! Dynamic blocks: blocks whose HTML is generated on the server. Each
//! submodule is a port of the matching `wp-includes/blocks/<name>.php`.

use std::sync::Arc;

use futures::future::BoxFuture;
use rustpress_db::Post;

use crate::Result;
use crate::html::TagEditor;
use crate::render::BlockCtx;

mod adjacent;
mod comments;
mod media;
mod navigation;
mod post_content;
mod post_meta;
mod query;
mod search;
mod simple;
mod template;

/// Blocks with a server-side render callback.
pub fn is_dynamic(name: &str) -> bool {
    matches!(
        name,
        "core/template-part"
            | "core/pattern"
            | "core/post-content"
            | "core/post-title"
            | "core/post-date"
            | "core/post-excerpt"
            | "core/post-terms"
            | "core/post-author-name"
            | "core/post-author"
            | "core/site-title"
            | "core/site-tagline"
            | "core/site-logo"
            | "core/image"
            | "core/gallery"
            | "core/post-featured-image"
            | "core/heading"
            | "core/list"
            | "core/navigation"
            | "core/page-list"
            | "core/navigation-link"
            | "core/query"
            | "core/post-template"
            | "core/query-title"
            | "core/query-no-results"
            | "core/query-pagination"
            | "core/query-pagination-next"
            | "core/query-pagination-previous"
            | "core/query-pagination-numbers"
            | "core/search"
            | "core/comments"
            | "core/comments-title"
            | "core/comment-template"
            | "core/comment-author-name"
            | "core/comment-content"
            | "core/comment-date"
            | "core/comment-reply-link"
            | "core/avatar"
            | "core/post-comments-form"
            | "core/comments-pagination"
            | "core/comments-pagination-previous"
            | "core/comments-pagination-next"
            | "core/comments-pagination-numbers"
            | "core/post-navigation-link"
    )
}

pub fn render<'a>(bc: &'a BlockCtx<'a>) -> BoxFuture<'a, Result<String>> {
    Box::pin(async move {
        match bc.block.name.as_deref().unwrap_or_default() {
            "core/template-part" => template::template_part(bc).await,
            "core/pattern" => template::pattern(bc).await,
            "core/post-content" => post_content::render(bc).await,
            "core/post-title" => post_meta::post_title(bc).await,
            "core/post-date" => post_meta::post_date(bc).await,
            "core/post-excerpt" => post_meta::post_excerpt(bc).await,
            "core/post-terms" => post_meta::post_terms(bc).await,
            "core/post-author-name" => post_meta::post_author_name(bc).await,
            "core/post-author" => post_meta::post_author(bc).await,
            "core/site-title" => simple::site_title(bc),
            "core/site-tagline" => simple::site_tagline(bc),
            "core/site-logo" => media::site_logo(bc).await,
            "core/image" => media::image(bc),
            "core/gallery" => media::gallery(bc),
            "core/post-featured-image" => media::post_featured_image(bc).await,
            "core/heading" => Ok(simple::add_class_to_first(&bc.content, &["h1", "h2", "h3", "h4", "h5", "h6"], "wp-block-heading")),
            "core/list" => Ok(simple::add_class_to_first(&bc.content, &["ol", "ul"], "wp-block-list")),
            "core/navigation" => navigation::navigation(bc).await,
            "core/page-list" => navigation::page_list(bc).await,
            "core/navigation-link" => navigation::navigation_link(bc).await,
            "core/query" => query::query(bc),
            "core/post-template" => query::post_template(bc).await,
            "core/query-title" => query::query_title(bc).await,
            "core/query-no-results" => query::query_no_results(bc).await,
            "core/query-pagination" => query::query_pagination(bc),
            "core/query-pagination-next" => query::pagination_link(bc, true).await,
            "core/query-pagination-previous" => query::pagination_link(bc, false).await,
            "core/query-pagination-numbers" => query::pagination_numbers(bc).await,
            "core/search" => search::search(bc),
            "core/comments" => comments::comments(bc).await,
            "core/comments-title" => comments::comments_title(bc).await,
            "core/comment-template" => comments::comment_template(bc).await,
            "core/comment-author-name" => comments::comment_author_name(bc).await,
            "core/comment-content" => comments::comment_content(bc).await,
            "core/comment-date" => comments::comment_date(bc).await,
            "core/comment-reply-link" => comments::comment_reply_link(bc).await,
            "core/avatar" => comments::avatar(bc).await,
            "core/post-comments-form" => comments::post_comments_form(bc).await,
            "core/comments-pagination" => comments::comments_pagination(bc).await,
            "core/comments-pagination-previous" | "core/comments-pagination-next" | "core/comments-pagination-numbers" => Ok(String::new()),
            "core/post-navigation-link" => adjacent::render(bc).await,
            _ => Ok(bc.content.clone()),
        }
    })
}

/// `render_block_{name}` filters: run on the output of the generic pipeline.
pub fn name_filter(name: &str, content: String) -> String {
    match name {
        "core/paragraph" if !content.is_empty() => simple::add_class_to_first(&content, &["p"], "wp-block-paragraph"),
        _ => content,
    }
}

/// `global $post`: the post of the block context, loaded on demand.
pub(crate) async fn global_post(bc: &BlockCtx<'_>) -> Result<Option<Arc<Post>>> {
    if let Some(post) = &bc.context.post {
        return Ok(Some(post.clone()));
    }
    match bc.context.post_id() {
        Some(id) => Ok(bc.env.site.db.post(id).await?.map(Arc::new)),
        None => Ok(None),
    }
}

/// `h{level}` or `p` for level 0.
pub(crate) fn heading_tag(level: Option<u64>, default: &str) -> String {
    match level {
        Some(0) => "p".to_owned(),
        Some(n) => format!("h{n}"),
        None => default.to_owned(),
    }
}

#[allow(dead_code)]
pub(crate) fn first_tag_name(html: &str) -> Option<String> {
    TagEditor::first(html).map(|e| e.tag().name.clone())
}
