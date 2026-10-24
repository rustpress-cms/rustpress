//! Link tags of the document head: the feeds (`feed_links()`, `feed_links_extra()`) and the
//! REST resource of the queried object (`rest_output_link_wp_head()`).

use rustpress_core::formatting::{esc_attr, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::trailingslashit;
use rustpress_core::{Queried, QueryResult, Site};

use crate::Result;

/// `get_search_link()` for the current search words.
fn search_link(site: &Site, words: &str) -> String {
    let encoded = rustpress_core::canonical::rawurlencode(words).replace("%2F", "/");
    site.user_trailingslashit(&site.home_url(&format!("search/{encoded}")))
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
    let mut out = link(&titled("%1$s %2$s Feed", &[]), &site.user_trailingslashit(&site.home_url("feed")));
    out.push_str(&link(&titled("%1$s %2$s Comments Feed", &[]), &site.user_trailingslashit(&site.home_url("comments/feed"))));

    let flags = &q.flags;
    let pretty = site.uses_pretty_permalinks();
    let feed_of = |url: &str| -> String {
        if pretty { format!("{}{}", trailingslashit(url), site.user_trailingslashit("feed")) } else { rustpress_core::urls::add_query_arg(url, "feed", "rss2") }
    };
    if flags.is_singular() {
        if let Queried::Post(post) = &q.queried
            && (post.comment_status == "open" || post.ping_status == "open" || post.comment_count > 0)
        {
            let permalink = site.permalink(post).await?;
            let href = if pretty { format!("{}feed/", trailingslashit(&permalink)) } else { rustpress_core::urls::add_query_arg(&permalink, "feed", "rss2") };
            // `the_title_attribute()`: the filtered title without tags.
            let title = rustpress_core::formatting::strip_tags(&rustpress_blocks::content::the_title_for(site, post));
            out.push_str(&link(&titled("%1$s %2$s %3$s Comments Feed", &[&title]), &href));
        }
    } else if flags.is_category || flags.is_tag || flags.is_tax {
        if let Queried::Term(term) = &q.queried {
            let title = if flags.is_category {
                titled("%1$s %2$s %3$s Category Feed", &[&term.name])
            } else if flags.is_tag {
                titled("%1$s %2$s %3$s Tag Feed", &[&term.name])
            } else {
                titled("%1$s %2$s %3$s %4$s Feed", &[&term.name, &term.taxonomy])
            };
            out.push_str(&link(&title, &feed_of(&site.term_link(term).await?)));
        }
    } else if flags.is_author {
        if let Queried::User(user) = &q.queried {
            out.push_str(&link(&titled("%1$s %2$s Posts by %3$s Feed", &[&user.display_name]), &feed_of(&site.author_url(user))));
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
        Queried::Term(term) if flags.is_category || flags.is_tag || flags.is_tax => {
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
