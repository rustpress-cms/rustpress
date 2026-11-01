//! Yoast SEO 28.8 (`wordpress-seo/wp-seo.php`), the website: the title, the robots directives and the meta tags of
//! `wpseo_head` with the schema graph. The plugin keeps what it prints in its table `wp_yoast_indexable`; the
//! module builds the same values from the posts, terms, users and options each time (what the plugin's builders do
//! when the row is missing or outdated), so pages look the same before and after the plugin saw them.

mod head;
mod images;
mod options;
mod presentation;
mod replace_vars;
mod schema;

use std::collections::HashMap;

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, Head, HeadContext, NativePlugin, PageServices, Redirect, Robot};
use rustpress_core::{QueryResult, Result, Site};
use rustpress_db::{Post, User};

use options::Options;

/// `WPSEO_VERSION`.
pub const VERSION: &str = "28.8";

const DOMAIN: &str = "wordpress-seo";

pub struct WordpressSeo;

impl NativePlugin for WordpressSeo {
    fn file(&self) -> &'static str {
        "wordpress-seo/wp-seo.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some(DOMAIN)
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move { Ok(Box::new(Loaded { options: Options::load(site) }) as Box<dyn ActivePlugin>) })
    }
}

/// The plugin with its options.
pub struct Loaded {
    options: Options,
}

/// What the plugin reads while it prints one page.
pub struct Page<'a> {
    pub site: &'a Site,
    pub query: &'a QueryResult,
    pub options: &'a Options,
    pub services: &'a dyn PageServices,
    /// `$GLOBALS['post']` after `wp_reset_query()`: the first post of the main query.
    pub global_post: Option<&'a Post>,
    /// The users the page names (authors, the person the site represents).
    pub users: HashMap<u64, User>,
    /// `$_GET`.
    pub get: Vec<(String, String)>,
    pub now: i64,
    pub admin_bar: bool,
    /// `Front_End_Integration::query_loop_next_prev()`: a query block that does not inherit the main query was rendered,
    /// with the last pagination links.
    /// `get_the_date()` returns `false`: a plugin hides the dates of the global post, for which `wp_reset_query()`
    /// ran `the_post`.
    pub dates_hidden: bool,
    pub custom_query_loop: bool,
    pub pagination_next: Option<String>,
    pub pagination_previous: Option<String>,
}

/// `(int) $string`: the leading number.
pub fn php_int(s: &str) -> i64 {
    let s = s.trim_start();
    let (sign, digits) = match s.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, s.strip_prefix('+').unwrap_or(s)),
    };
    let digits: String = digits.chars().take_while(char::is_ascii_digit).collect();
    sign * digits.parse::<i64>().unwrap_or(0)
}

/// `urldecode()`.
pub fn urldecode(s: &str) -> String {
    String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.as_bytes())).into_owned()
}

/// The query string as `$_GET` sees it.
fn query_pairs(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (urldecode(k), urldecode(v))
        })
        .collect()
}

impl Loaded {
    async fn head(&self, ctx: &HeadContext<'_>, robots: &[(String, Robot)]) -> Result<Head> {
        let q = ctx.query;
        let global_post = match &q.queried {
            rustpress_core::Queried::Post(p) if q.flags.is_singular() => Some(&**p),
            _ => q.posts.first(),
        };
        let mut user_ids: Vec<u64> = Vec::new();
        if let rustpress_core::Queried::Post(p) = &q.queried {
            user_ids.push(p.post_author);
        }
        if let rustpress_core::Queried::User(u) = &q.queried {
            user_ids.push(u.id);
        }
        user_ids.push(php_int(&self.options.string("company_or_person_user_id")).max(0) as u64);
        let mut users = HashMap::new();
        for id in user_ids.into_iter().filter(|id| *id != 0) {
            if let Some(user) = ctx.site.db.user(id).await? {
                users.insert(id, user);
            }
        }
        let dates_hidden = match global_post {
            Some(post) => ctx.site.plugins.hides_date(ctx.site, post).await?,
            None => false,
        };
        let page = Page {
            site: ctx.site,
            query: q,
            options: &self.options,
            services: ctx.services,
            global_post,
            users,
            get: query_pairs(&q.raw_query),
            now: chrono::Utc::now().timestamp(),
            admin_bar: ctx.admin_bar,
            dates_hidden,
            custom_query_loop: ctx.custom_query_loop,
            pagination_next: ctx.pagination_next.clone(),
            pagination_previous: ctx.pagination_previous.clone(),
        };
        let presentation = presentation::present(&page).await?;
        let title = head::title(&page, &presentation).await?;
        let html = head::present(&page, &presentation, ctx.title_tag).await?;
        Ok(Head { robots: Some(head::wp_robots(robots, &presentation.robots)), title: Some(title), html, removed: vec!["rel_canonical", "_wp_render_title_tag"] })
    }
}

/// The query string without the parameters `drop` names (`remove_query_arg()` on the raw string).
fn without_params(raw: &str, drop: &[&str]) -> String {
    raw.split('&').filter(|p| !p.is_empty() && !drop.contains(&urldecode(p.split('=').next().unwrap_or_default()).as_str())).collect::<Vec<_>>().join("&")
}

impl Loaded {
    /// `Url_Helper::recreate_current_url()`: the address of the request with another query string.
    fn current_url(site: &Site, query: &QueryResult, query_string: &str) -> String {
        let home = site.home_url("");
        let (scheme, rest) = home.split_once("://").unwrap_or(("http", &home));
        let host = query.http_host.clone().unwrap_or_else(|| rest.split('/').next().unwrap_or_default().to_owned());
        let mut url = format!("{scheme}://{host}{}", query.url_path);
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(query_string);
        }
        url
    }

    /// `wp_safe_redirect()`: a host other than the site's goes to the administration instead.
    fn safe(site: &Site, location: &str) -> String {
        let host_of = |url: &str| url.split_once("://").map(|(_, r)| r.split(['/', '?', '#', ':']).next().unwrap_or_default().to_ascii_lowercase());
        match host_of(location) {
            Some(host) if Some(host.clone()) != host_of(&site.home_url("")) => site.site_url("wp-admin/"),
            _ => location.to_owned(),
        }
    }

    async fn redirect(&self, site: &Site, query: &QueryResult) -> Result<Option<Redirect>> {
        let o = &self.options;
        let f = &query.flags;
        let get = query_pairs(&query.raw_query);
        let yoast = |location: String| Some(Redirect { location, status: 301, by: "Yoast SEO" });
        // `wp`: archive_redirect(), category_redirect(), page_redirect() (priority 99).
        let post_format = f.is_tax && matches!(&query.queried, rustpress_core::Queried::Term(t) if t.taxonomy == "post_format");
        if (o.truthy("disable-date") && f.is_date) || (o.truthy("disable-author") && f.is_author) || (o.truthy("disable-post_format") && post_format) {
            return Ok(yoast(Self::safe(site, &site.home_url(""))));
        }
        if get.iter().any(|(k, v)| k == "cat" && v == "-1") {
            let url = Self::current_url(site, query, &without_params(&query.raw_query, &["cat"]));
            return Ok(Some(Redirect { location: Self::safe(site, &url), status: 301, by: "Stripping cat=-1 from the URL" }));
        }
        let simple = if f.is_singular() { query.queried_post().map(|p| p.id) } else if f.is_home && site.settings.show_on_front == rustpress_core::settings::ShowOnFront::Page { Some(site.settings.page_for_posts).filter(|id| *id != 0) } else { None };
        if let Some(id) = simple
            && let Some(target) = site.db.post_meta_value(id, "_yoast_wpseo_redirect").await?.filter(|t| !t.is_empty())
        {
            return Ok(yoast(Self::safe(site, &target)));
        }
        // `template_redirect` at priority 1: attachment_redirect(), replytocom_redirect().
        if f.is_attachment
            && o.truthy("disable-attachment")
            && let Some(post) = query.queried_post()
        {
            let file = site.db.post_meta_value(post.id, "_wp_attached_file").await?.filter(|f| !f.is_empty());
            let url = match file {
                Some(file) => rustpress_blocks::images::attachment_url(site, &file),
                None => post.guid.clone(),
            };
            if !url.is_empty() {
                return Ok(yoast(url));
            }
        }
        if let Some((_, hash)) = get.iter().find(|(k, _)| k == "replytocom")
            && let Some(post) = query.queried_post().filter(|_| f.is_singular())
        {
            let mut url = site.permalink(post).await?;
            // `remove_query_arg( 'replytocom', sanitize_text_field( $_SERVER['QUERY_STRING'] ) )`.
            let rest = rustpress_content::sanitize::text_field(&without_params(&query.raw_query, &["replytocom"]));
            if !rest.is_empty() {
                url.push('?');
                url.push_str(&rest);
            }
            url.push_str(&format!("#comment-{}", rustpress_content::sanitize::text_field(hash)));
            return Ok(yoast(url));
        }
        // `disable_date_queries()`: with date archives off, date parameters are dropped.
        const DATE_VARS: [&str; 7] = ["year", "m", "monthnum", "day", "hour", "minute", "second"];
        if o.truthy("disable-date") && get.iter().any(|(k, _)| DATE_VARS.contains(&k.as_str())) {
            let url = Self::current_url(site, query, &without_params(&query.raw_query, &DATE_VARS));
            return Ok(yoast(Self::safe(site, &url)));
        }
        Ok(None)
    }
}

impl ActivePlugin for Loaded {
    fn wp_head<'a>(&'a self, ctx: &'a HeadContext<'a>, robots: &'a [(String, Robot)]) -> BoxFuture<'a, Result<Option<Head>>> {
        Box::pin(async move { Ok(Some(self.head(ctx, robots).await?)) })
    }

    /// `call_wpseo_head()` and the context memoizer call `wp_reset_query()`.
    fn resets_query_in_head(&self) -> bool {
        true
    }

    /// The redirects of `Redirects` (on `wp`, then `template_redirect`) and `Comment_Link_Fixer`.
    fn template_redirect<'a>(&'a self, site: &'a Site, query: &'a QueryResult) -> BoxFuture<'a, Result<Option<Redirect>>> {
        Box::pin(self.redirect(site, query))
    }

    /// `Indexing_Controls::nofollow_link()`.
    fn loginout(&self, link: String) -> String {
        link.replace("<a ", "<a rel=\"nofollow\" ")
    }

    /// `Comment_Link_Fixer::remove_reply_to_com()`: the reply links point at the comment.
    fn comment_reply_link(&self, link: String) -> String {
        static REPLY: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r#"href=(["'])(?:.*(?:\?|&|&#038;)replytocom=(\d+)#respond)"#).expect("valid pattern"));
        REPLY.replace(&link, "href=${1}#comment-${2}").into_owned()
    }
}
