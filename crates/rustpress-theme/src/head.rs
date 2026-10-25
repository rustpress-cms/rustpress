//! Link tags of the document head: the feeds (`feed_links()`, `feed_links_extra()`) and the
//! REST resource of the queried object (`rest_output_link_wp_head()`).

use rustpress_core::formatting::{esc_attr, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::trailingslashit;
use rustpress_core::{Queried, QueryResult, Site};

use crate::Result;

/// `get_search_link()` for the current search words.
fn search_link(site: &Site, words: &str) -> String {
    let encoded = rustpress_core::canonical::php_urlencode(words).replace("%2F", "/");
    site.user_trailingslashit(&site.home_url(&format!("{}search/{encoded}", site.root())))
}

/// `feed_links()` and `feed_links_extra()`.
pub async fn feed_links(site: &Site, q: &QueryResult) -> Result<String> {
    let name = &site.settings.blogname;
    let link = |title: &str, href: &str| format!("<link rel=\"alternate\" type=\"application/rss+xml\" title=\"{}\" href=\"{}\" />\n", esc_attr(title), esc_url(href));
    let separator = site.trx("&raquo;", "feed link");
    let titled = |template: &str, args: &[&str]| -> String {
        let mut all = vec![name.as_str(), separator.as_str()];
        all.extend_from_slice(args);
        sprintf(&site.tr(template), &all)
    };
    let pretty_feeds = site.uses_pretty_permalinks();
    let main_feed = if pretty_feeds { site.user_trailingslashit(&site.home_url(&format!("{}feed", site.root()))) } else { site.home_url("?feed=rss2") };
    let comments_feed = if pretty_feeds { site.user_trailingslashit(&site.home_url(&format!("{}comments/feed", site.root()))) } else { site.home_url("?feed=comments-rss2") };
    let mut out = link(&titled("%1$s %2$s Feed", &[]), &main_feed);
    out.push_str(&link(&titled("%1$s %2$s Comments Feed", &[]), &comments_feed));

    let flags = &q.flags;
    let pretty = site.uses_pretty_permalinks();
    let feed_of = |url: &str| -> String {
        if pretty { format!("{}{}", trailingslashit(url), site.user_trailingslashit("feed")) } else { rustpress_core::urls::add_query_arg(url, "feed", "rss2") }
    };
    if flags.is_singular() {
        if let Queried::Post(post) = &q.queried
            && (post.comment_status == "open" || post.ping_status == "open" || post.comment_count > 0)
        {
            // `get_post_comments_feed_link()` uses the page's own address, also for the front page.
            let permalink = if post.post_type == "page" { site.page_link_of(post).await? } else { site.permalink(post).await? };
            let href = if pretty { format!("{}{}", trailingslashit(&permalink), site.user_trailingslashit("feed")) } else if post.post_type == "page" { format!("{}?feed=rss2&page_id={}", site.home_url("/"), post.id) } else { format!("{}?feed=rss2&p={}", site.home_url("/"), post.id) };
            // `the_title_attribute()`: the filtered title without tags.
            let title = rustpress_core::formatting::strip_tags(&rustpress_blocks::content::the_title_for(site, post));
            out.push_str(&link(&titled("%1$s %2$s %3$s Comments Feed", &[&title]), &href));
        }
    } else if flags.is_post_type_archive {
        if let Queried::PostType(post_type) = &q.queried
            && let Some(archive) = site.post_type_archive_link(post_type)
        {
            let href = if pretty { format!("{}feed/", trailingslashit(&archive)) } else { rustpress_core::urls::add_query_arg(&archive, "feed", "rss2") };
            out.push_str(&link(&titled("%1$s %2$s %3$s Feed", &[&site.post_type_label(post_type)]), &href));
        }
    } else if flags.is_category || flags.is_tag || flags.is_tax {
        if let Queried::Term(term) = &q.queried {
            let name = site.term_name(term);
            let title = if flags.is_category {
                titled("%1$s %2$s %3$s Category Feed", &[&name])
            } else if flags.is_tag {
                titled("%1$s %2$s %3$s Tag Feed", &[&name])
            } else {
                titled("%1$s %2$s %3$s %4$s Feed", &[&name, &site.taxonomy_singular_label(&term.taxonomy)])
            };
            let href = if pretty {
                feed_of(&site.term_link(term).await?)
            } else if flags.is_category {
                site.home_url(&format!("?feed=rss2&cat={}", term.term_id))
            } else if flags.is_tag {
                site.home_url(&format!("?feed=rss2&tag={}", term.slug))
            } else {
                site.home_url(&format!("?feed=rss2&{}={}", term.taxonomy, term.slug))
            };
            out.push_str(&link(&title, &href));
        }
    } else if flags.is_author {
        if let Queried::User(user) = &q.queried {
            let href = if pretty { feed_of(&site.author_url(user)) } else { site.home_url(&format!("?feed=rss2&author={}", user.id)) };
            out.push_str(&link(&titled("%1$s %2$s Posts by %3$s Feed", &[&user.display_name]), &href));
        }
    } else if flags.is_search {
        let words = q.vars.get("s").unwrap_or_default();
        let base = search_link(site, words);
        let href = if pretty { format!("{}feed/rss2/", trailingslashit(&base)) } else { rustpress_core::urls::add_query_arg(&site.home_url("/"), "s", words) + "&feed=rss2" };
        out.push_str(&link(&titled("%1$s %2$s Search Results for &#8220;%3$s&#8221; Feed", &[words]), &href));
    }
    Ok(out)
}

/// `rest_get_queried_resource_route()`: the REST route of what the page shows.
pub fn queried_rest_route(q: &QueryResult) -> Option<String> {
    let flags = &q.flags;
    match &q.queried {
        Queried::Post(post) if flags.is_singular() => {
            let base = match post.post_type.as_str() {
                "page" => "pages",
                "post" => "posts",
                "attachment" => "media",
                other => other,
            };
            Some(format!("wp/v2/{base}/{}", post.id))
        }
        Queried::Term(term) if (flags.is_category || flags.is_tag || flags.is_tax) && term.taxonomy != "post_format" => {
            let base = match term.taxonomy.as_str() {
                "category" => "categories",
                "post_tag" => "tags",
                other => other,
            };
            Some(format!("wp/v2/{base}/{}", term.term_id))
        }
        Queried::User(user) if flags.is_author => Some(format!("wp/v2/users/{}", user.id)),
        _ => None,
    }
}
