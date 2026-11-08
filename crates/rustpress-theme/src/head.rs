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

/// `get_search_comments_feed_link()` for the current search words.
pub(crate) fn search_comments_feed_link(site: &Site, words: &str, feed: &str) -> String {
    if !site.uses_pretty_permalinks() {
        let link = site.home_url(&format!("?s={}", rustpress_core::canonical::php_urlencode(words)));
        return rustpress_core::urls::add_query_arg(&link, "feed", &format!("comments-{feed}"));
    }
    let link = format!("{}feed/{feed}/", trailingslashit(&search_link(site, words)));
    rustpress_core::urls::add_query_arg(&link, "withcomments", "1")
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

/// `wp_get_shortlink( 0, 'query' )`: `?p=` for a single post of a public type, the home page for the front page.
pub fn shortlink(site: &Site, q: &QueryResult) -> Option<String> {
    let Queried::Post(post) = &q.queried else { return None };
    if !q.flags.is_singular() || post.id == 0 {
        return None;
    }
    let settings = &site.settings;
    if post.post_type == "page" && settings.show_on_front == rustpress_core::settings::ShowOnFront::Page && settings.page_on_front == post.id {
        Some(site.home_url("/"))
    } else if site.types.public_post_types().contains(&post.post_type.as_str()) {
        Some(site.home_url(&format!("?p={}", post.id)))
    } else {
        None
    }
}

/// `rest_get_queried_resource_route()`: the REST route of what the page shows. A type of a plugin needs
/// `show_in_rest` and has its `rest_base`; without a registration its name is assumed.
pub fn queried_rest_route(site: &Site, q: &QueryResult) -> Option<String> {
    let flags = &q.flags;
    match &q.queried {
        Queried::Post(post) if flags.is_singular() => {
            let base = match post.post_type.as_str() {
                "page" => "pages".to_owned(),
                "post" => "posts".to_owned(),
                "attachment" => "media".to_owned(),
                other => match site.types.post_type(other) {
                    Some(info) if info.registered => info.show_in_rest.then(|| info.rest_base().to_owned())?,
                    _ => other.to_owned(),
                },
            };
            Some(format!("wp/v2/{base}/{}", post.id))
        }
        Queried::Term(term) if (flags.is_category || flags.is_tag || flags.is_tax) && term.taxonomy != "post_format" => {
            let base = match term.taxonomy.as_str() {
                "category" => "categories".to_owned(),
                "post_tag" => "tags".to_owned(),
                other => match site.types.taxonomy(other) {
                    Some(info) if info.registered => info.show_in_rest.then(|| info.rest_base().to_owned())?,
                    _ => other.to_owned(),
                },
            };
            Some(format!("wp/v2/{base}/{}", term.term_id))
        }
        Queried::User(user) if flags.is_author => Some(format!("wp/v2/users/{}", user.id)),
        _ => None,
    }
}

/// The functions of the rendering environment a plugin may call (`PageServices`).
pub struct Services<'a>(rustpress_blocks::Env<'a>, &'a std::path::Path);

impl<'a> Services<'a> {
    pub fn new(theme: &'a crate::Theme, site: &'a Site, q: &'a QueryResult, registry: &'a rustpress_blocks::Registry) -> Self {
        Self(rustpress_blocks::Env::new(site, q, theme, registry), &theme.wp_root)
    }
}

impl rustpress_core::native::PageServices for Services<'_> {
    fn excerpt<'a>(&'a self, post: &'a rustpress_db::Post) -> futures::future::BoxFuture<'a, rustpress_core::Result<String>> {
        Box::pin(async move {
            let post = std::sync::Arc::new(post.clone());
            rustpress_blocks::excerpt::get_the_excerpt(&self.0, &post).await.map_err(|e| match e {
                rustpress_blocks::Error::Core(e) => e,
                rustpress_blocks::Error::Db(e) => rustpress_core::Error::Db(e),
            })
        })
    }

    fn wp_root(&self) -> &std::path::Path {
        self.1
    }

    fn content_filters<'a>(&'a self, post: Option<&'a rustpress_db::Post>, text: &'a str, context: &'a str) -> futures::future::BoxFuture<'a, rustpress_core::Result<String>> {
        rustpress_core::native::PageServices::content_filters(&self.0, post, text, context)
    }

    fn do_shortcode<'a>(&'a self, post: Option<&'a rustpress_db::Post>, text: &'a str) -> futures::future::BoxFuture<'a, rustpress_core::Result<String>> {
        rustpress_core::native::PageServices::do_shortcode(&self.0, post, text)
    }

    fn avatar(&self, email: &str, size: u64) -> String {
        rustpress_core::native::PageServices::avatar(&self.0, email, size)
    }
}

/// What the plugins changed in the head, and the blocks they rendered while doing so (in the order their styles were
/// enqueued: a plugin that asks for an excerpt in `wp_head` renders the post's blocks before the styles are printed).
pub struct PluginHead {
    pub head: rustpress_core::native::Head,
    pub blocks: Vec<String>,
}

/// What the active plugins change in the head (`wp_head` at priority 1, `wp_robots`, `pre_get_document_title`).
pub async fn plugin_head(theme: &crate::Theme, site: &Site, q: &QueryResult, registry: &rustpress_blocks::Registry, title_tag: bool, state: Option<&rustpress_blocks::env::State>) -> Result<PluginHead> {
    let robots = crate::document::core_robots(site, q);
    if site.plugins.is_empty() {
        return Ok(PluginHead { head: rustpress_core::native::Head { robots: Some(robots), ..Default::default() }, blocks: Vec::new() });
    }
    let services = Services::new(theme, site, q, registry);
    let ctx = rustpress_core::native::HeadContext {
        site,
        query: q,
        services: &services,
        title_tag,
        admin_bar: q.viewer.as_ref().is_some_and(|v| v.show_admin_bar),
        custom_query_loop: state.is_some_and(|s| s.custom_query_loop),
        pagination_next: state.and_then(|s| s.pagination_next.clone()),
        pagination_previous: state.and_then(|s| s.pagination_previous.clone()),
    };
    let head = site.plugins.wp_head(&ctx, robots).await?;
    Ok(PluginHead { head, blocks: services.0.take_state().used_blocks })
}
