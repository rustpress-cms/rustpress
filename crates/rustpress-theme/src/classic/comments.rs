//! comments_template() for Twenty Seventeen: the comment list and form come from the shared
//! legacy_comments module of the blocks crate.

use rustpress_blocks::content::the_title_for;
use rustpress_blocks::legacy_comments::{comment_form, list_comments};
use rustpress_core::Site;
use rustpress_db::Post;

use crate::Result;

pub use rustpress_blocks::legacy_comments::{ListArgs, comments_open};

/// The text of the pagination links of the comment list (`the_comments_pagination()` arguments).
pub struct PaginationText {
    pub prev_text: String,
    pub next_text: String,
}

/// `comments.php` of Twenty Seventeen around the list and the form.
#[allow(clippy::too_many_arguments)]
pub async fn comments_template(site: &Site, env: &rustpress_blocks::Env<'_>, post: &Post, args: &ListArgs, request_uri: &str, pagination: &PaginationText, theme_text: &(dyn Fn(&str) -> String + Sync), theme_text_x: &(dyn Fn(&str, &str) -> String + Sync)) -> Result<String> {
    if rustpress_blocks::content::post_password_required(post) {
        return Ok(String::new());
    }
    let page = rustpress_blocks::comment_pages::comments_for_template(env, post.id).await?;
    let comments = page.comments.clone();
    let args = &ListArgs { avatar_size: args.avatar_size, reply_text: args.reply_text.clone(), short_ping: args.short_ping, cpage: Some(page.page) };
    let count = post.comment_count;
    let mut out = String::from("\n<div id=\"comments\" class=\"comments-area\">\n\n\t");
    if !comments.is_empty() {
        let title = the_title_for(site, post);
        let heading = if count == 1 {
            rustpress_core::i18n::sprintf(&theme_text_x("One Reply to &ldquo;%s&rdquo;", "comments title"), &[&title])
        } else {
            let template = site.i18n().npgettext("twentyseventeen", "comments title", "%1$s Reply to &ldquo;%2$s&rdquo;", "%1$s Replies to &ldquo;%2$s&rdquo;", count);
            rustpress_core::i18n::sprintf(&template, &[&site.number_i18n(count.max(0) as u64), &title])
        };
        out.push_str(&format!("\t\t<h2 class=\"comments-title\">\n\t\t\t{heading}\t\t</h2>\n\n\t\t<ol class=\"comment-list\">\n\t\t\t"));
        out.push_str(&list_comments(site, env, post, &comments, args).await?);
        out.push_str("\t\t</ol>\n\n\t\t");
        out.push_str(&super::tags::comments_pagination(site, &site.permalink(post).await?, request_uri, page.page, page.max_pages, &pagination.prev_text, &pagination.next_text));
    }
    if !comments_open(post) && count != 0 {
        out.push_str(&format!("\n\n\t\t<p class=\"no-comments\">{}</p>\n\t\t", theme_text("Comments are closed.")));
    }
    out.push_str(&comment_form(site, post, request_uri, false).await?);
    out.push_str("\n</div><!-- #comments -->\n");
    Ok(out)
}
