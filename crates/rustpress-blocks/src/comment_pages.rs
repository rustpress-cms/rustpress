//! Comment pagination: which comments a page shows (`comments_template()`,
//! `build_comment_query_vars_from_block()`), `get_comment_link()` and the page links.

use rustpress_core::formatting::esc_url;
use rustpress_core::urls::{add_query_arg, trailingslashit};
use rustpress_db::Comment;

use crate::Result;
use crate::env::Env;

/// The comments of one page, ready for the walker.
#[derive(Debug, Clone, Default)]
pub struct CommentPage {
    /// The top-level comments of the page, each followed by its descendants.
    pub comments: Vec<Comment>,
    /// `max_num_comment_pages` (0 without pagination).
    pub max_pages: u64,
    /// The page shown; 0 if the comments are not paginated.
    pub page: u64,
}

fn threaded(env: &Env<'_>) -> bool {
    env.site.options.get("thread_comments") == Some("1")
}

pub fn paginated(env: &Env<'_>) -> bool {
    env.site.options.get("page_comments") == Some("1")
}

fn per_page(env: &Env<'_>) -> u64 {
    env.site.options.u64("comments_per_page", 50)
}

fn newest_site(site: &rustpress_core::Site) -> bool {
    site.options.get("default_comments_page") != Some("oldest")
}

fn newest(env: &Env<'_>) -> bool {
    newest_site(env.site)
}

/// The descendants of `parent` in depth-first order (`WP_Comment::get_children( 'flat' )`).
fn descendants<'a>(all: &'a [Comment], parent: u64, out: &mut Vec<&'a Comment>) {
    for child in all.iter().filter(|c| c.comment_parent == parent) {
        out.push(child);
        descendants(all, child.comment_id, out);
    }
}

/// The comments a post shows on the requested page.
/// The comments of a post for `comments_template()`, which also makes the page the default `cpage` of the request.
pub async fn comments_for_template(env: &Env<'_>, post_id: u64) -> Result<CommentPage> {
    let mut page = comments_for_post(env, post_id).await?;
    // `cpage` becomes the newest (or first) page only while there is more than one page, and only for the template.
    if env.query.vars.u64("cpage").unwrap_or(0) == 0 {
        if page.max_pages <= 1 {
            page.page = 0;
        } else {
            env.with_state(|s| s.cpage_override = page.page);
        }
    }
    Ok(page)
}

/// The comments of a post shown on the requested page.
/// The comments `comments_template()` lists: the approved ones and the pending ones of the viewer.
pub async fn listed_comments(env: &Env<'_>, post_id: u64) -> Result<Vec<Comment>> {
    let include = crate::commenter::include_unapproved(env).await?;
    if include.user.is_none() && include.email.is_none() {
        return Ok(env.site.db.approved_comments(post_id).await?);
    }
    Ok(env.site.db.comments_with_pending(post_id, include.user, include.email.as_deref()).await?)
}

pub async fn comments_for_post(env: &Env<'_>, post_id: u64) -> Result<CommentPage> {
    let all = listed_comments(env, post_id).await?;
    let threaded = threaded(env);
    let top: Vec<&Comment> = if threaded { all.iter().filter(|c| c.comment_parent == 0).collect() } else { all.iter().collect() };
    let (mut selected, max_pages, page): (Vec<&Comment>, u64, u64) = if paginated(env) && per_page(env) > 0 {
        let per = per_page(env);
        let max_pages = (top.len() as u64).div_ceil(per);
        let requested = env.query.vars.u64("cpage").unwrap_or(0);
        let page = if requested != 0 {
            requested
        } else if newest(env) {
            max_pages.max(1)
        } else {
            1
        };
        let start = ((page - 1) * per) as usize;
        (top.iter().skip(start).take(per as usize).copied().collect(), max_pages, page)
    } else {
        (top, 0, 0)
    };
    let mut comments: Vec<Comment> = Vec::new();
    for comment in selected.drain(..) {
        comments.push(comment.clone());
        if threaded {
            let mut below = Vec::new();
            descendants(&all, comment.comment_id, &mut below);
            comments.extend(below.into_iter().cloned());
        }
    }
    Ok(CommentPage { comments, max_pages, page })
}

/// `get_comments_pagenum_link()`.
pub fn pagenum_link(site: &rustpress_core::Site, permalink: &str, pagenum: u64, max_page: u64) -> String {
    let paged_url = |pagenum: u64| {
        if site.uses_pretty_permalinks() {
            site.user_trailingslashit(&format!("{}comment-page-{pagenum}", trailingslashit(permalink)))
        } else {
            add_query_arg(permalink, "cpage", &pagenum.to_string())
        }
    };
    let mut result = permalink.to_owned();
    if newest_site(site) {
        if pagenum != max_page {
            result = paged_url(pagenum);
        }
    } else if pagenum > 1 {
        result = paged_url(pagenum);
    }
    result.push_str("#comments");
    result
}

/// `get_page_of_comment()` for all comment types.
pub fn page_of_comment(env: &Env<'_>, all: &[Comment], comment: &Comment) -> u64 {
    let per = if paginated(env) { per_page(env) } else { 0 };
    if per < 1 {
        return 1;
    }
    let max_depth = if threaded(env) { env.site.options.u64("thread_comments_depth", 5) } else { 0 };
    let mut target = comment;
    // A reply sits on the page of its top-level ancestor.
    if max_depth > 1 {
        while target.comment_parent != 0 {
            match all.iter().find(|c| c.comment_id == target.comment_parent) {
                Some(parent) => target = parent,
                None => break,
            }
        }
    }
    let older = all.iter().filter(|c| c.comment_parent == 0 && c.comment_date_gmt < target.comment_date_gmt).count() as u64;
    if older == 0 { 1 } else { (older + 1).div_ceil(per) }
}

/// `get_comment_link()`. `in_loop` is the `cpage` of the running comment list, if there is one.
pub async fn comment_link(env: &Env<'_>, comment: &Comment, in_loop: Option<u64>) -> Result<String> {
    let site = env.site;
    let Some(post) = site.db.post(comment.comment_post_id).await? else { return Ok(format!("#comment-{}", comment.comment_id)) };
    let mut link = site.permalink(&post).await?;
    let mut cpage = 0;
    if paginated(env) && per_page(env) > 0 {
        cpage = match in_loop {
            Some(page) => page,
            None => {
                let all = listed_comments(env, post.id).await?;
                page_of_comment(env, &all, comment)
            }
        };
        if !newest(env) && cpage == 1 {
            cpage = 0;
        }
    }
    if cpage != 0 && paginated(env) {
        link = if site.uses_pretty_permalinks() { format!("{}comment-page-{cpage}", trailingslashit(&link)) } else { add_query_arg(&link, "cpage", &cpage.to_string()) };
    }
    if site.uses_pretty_permalinks() {
        link = site.user_trailingslashit(&link);
    }
    Ok(format!("{link}#comment-{}", comment.comment_id))
}

/// `get_previous_comments_link()`: `None` on the first page.
pub async fn previous_link(env: &Env<'_>, label: &str, page: u64, permalink: &str, attributes: &str) -> Option<String> {
    if page <= 1 {
        return None;
    }
    Some(format!("<a href=\"{}\" {attributes}>{}</a>", esc_url(&pagenum_link(env.site, permalink, page - 1, 0)), replace_ampersands(label)))
}

/// `get_next_comments_link()`: `None` on the last page.
pub fn next_link(env: &Env<'_>, label: &str, page: u64, max_page: u64, permalink: &str, attributes: &str) -> Option<String> {
    let page = page.max(1);
    let next = page + 1;
    if max_page == 0 || next > max_page {
        return None;
    }
    Some(format!("<a href=\"{}\" {attributes}>{}</a>", esc_url(&pagenum_link(env.site, permalink, next, max_page)), replace_ampersands(label)))
}

/// `preg_replace( '/&([^#])(?![a-z]{1,8};)/i', '&#038;$1', $label )`.
fn replace_ampersands(label: &str) -> String {
    let bytes = label.as_bytes();
    let mut out = String::with_capacity(label.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&'
            && let Some(&next) = bytes.get(i + 1)
            && next != b'#'
        {
            let rest = &label[i + 1..];
            let entity = rest.char_indices().take(9).find(|(_, c)| *c == ';').map(|(n, _)| n);
            let is_entity = entity.is_some_and(|n| (1..=8).contains(&n) && rest[..n].chars().all(|c| c.is_ascii_alphabetic()));
            if !is_entity {
                out.push_str("&#038;");
                i += 1;
                continue;
            }
        }
        let c = label[i..].chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::replace_ampersands;

    #[test]
    fn ampersands_in_labels() {
        assert_eq!(replace_ampersands("&laquo; Older"), "&laquo; Older");
        assert_eq!(replace_ampersands("A & B"), "A &#038; B");
        assert_eq!(replace_ampersands("&#8211; x"), "&#8211; x");
    }
}
