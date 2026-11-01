//! Yoast SEO 28.8 (`wordpress-seo/wp-seo.php`), the website: the title, the robots directives and the meta tags of
//! `wpseo_head` with the schema graph. The plugin keeps what it prints in its table `wp_yoast_indexable`; the
//! module builds the same values from the posts, terms, users and options each time (what the plugin's builders do
//! when the row is missing or outdated), so pages look the same before and after the plugin saw them.

mod breadcrumbs;
mod head;
mod images;
mod options;
mod presentation;
mod replace_vars;
mod schema;
mod sitemaps;

use std::collections::HashMap;

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, Head, HeadContext, NativePlugin, PageServices, Redirect, RestFieldContext, RestObject, Robot, Serve, ServeRequest};
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
    /// The object the REST API asks about, with the page type `Meta_Surface` gives it; `None` for the current page.
    pub target: Option<(presentation::Kind, presentation::Object)>,
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
    /// The users the current page names and whether its global post hides its dates.
    async fn current<'q>(&self, site: &Site, q: &'q QueryResult) -> Result<(Option<&'q Post>, HashMap<u64, User>, bool)> {
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
            if let Some(user) = site.db.user(id).await? {
                users.insert(id, user);
            }
        }
        let dates_hidden = match global_post {
            Some(post) => site.plugins.hides_date(site, post).await?,
            None => false,
        };
        Ok((global_post, users, dates_hidden))
    }

    async fn head(&self, ctx: &HeadContext<'_>, robots: &[(String, Robot)]) -> Result<Head> {
        let q = ctx.query;
        let (global_post, users, dates_hidden) = self.current(ctx.site, q).await?;
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
            target: None,
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
        // `template_redirect` at priority 0: the sitemaps of WordPress and `/sitemap.xml`.
        if o.truthy("enable_xml_sitemap")
            && let Some(location) = sitemaps::core_sitemap_redirect(site, query)
        {
            return Ok(yoast(Self::safe(site, &location)));
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

impl Loaded {
    /// `Indexable_Head_Action`: the head of an object for the REST API, `None` where the plugin answers with its 404.
    async fn object_head(&self, ctx: &RestFieldContext<'_>, kind: presentation::Kind, object: presentation::Object) -> Result<Option<(String, serde_json::Value)>> {
        let Some((page_users, p)) = self.object_presentation(ctx.site, ctx.query, ctx.services, kind, object).await? else { return Ok(None) };
        let page = page_users.page(ctx.site, ctx.query, &self.options, ctx.services);
        Ok(Some(head::rest_head(&page, &p).await?))
    }

    /// The presentation of an object with the page type `Meta_Surface` gives it; `None` when the plugin builds no
    /// indexable for it.
    async fn object_presentation(&self, site: &Site, query: &QueryResult, services: &dyn PageServices, kind: presentation::Kind, object: presentation::Object) -> Result<Option<(TargetPage, presentation::Presentation)>> {
        let target = TargetPage::load(site, &self.options, kind, object).await?;
        let page = target.page(site, query, &self.options, services);
        let p = presentation::present(&page).await?;
        if p.model.object_type == "unknown" {
            return Ok(None);
        }
        Ok(Some((target, p)))
    }

    /// `Yoast_Head_REST_Field`: `yoast_head` and `yoast_head_json` of the indexable post types and taxonomies, users
    /// and post types.
    async fn rest_fields_of(&self, object: RestObject<'_>, ctx: &RestFieldContext<'_>) -> Result<Vec<(String, serde_json::Value)>> {
        if !self.options.truthy("enable_headless_rest_endpoints") {
            return Ok(Vec::new());
        }
        let site = ctx.site;
        // `Post_Type_Helper::get_indexable_post_types()` / `Taxonomy_Helper::get_indexable_taxonomies()`.
        let excluded_type = |t: &str| t == "attachment" && self.options.truthy("disable-attachment");
        let head = match object {
            RestObject::Post(post) => {
                if !site.types.public_post_types().contains(&post.post_type.as_str()) || excluded_type(&post.post_type) {
                    return Ok(Vec::new());
                }
                // `Post_Helper::is_post_indexable()`.
                if post.post_status == "auto-draft" || post.post_type == "revision" {
                    None
                } else {
                    self.object_head(ctx, presentation::Kind::PostType, presentation::Object::Post(Box::new(post.clone()))).await?
                }
            }
            RestObject::Term(term) => {
                if !options::public_taxonomies(site).contains(&term.taxonomy) {
                    return Ok(Vec::new());
                }
                self.object_head(ctx, presentation::Kind::TermArchive, presentation::Object::Term(term.clone())).await?
            }
            RestObject::User(user) => self.object_head(ctx, presentation::Kind::AuthorArchive, presentation::Object::User(user.clone())).await?,
            RestObject::PostType(name) => {
                if name == "post" {
                    // `Meta_Surface::for_posts_page()`.
                    let settings = &site.settings;
                    match site.db.post(settings.page_for_posts).await?.filter(|_| settings.page_for_posts != 0) {
                        Some(page) => self.object_head(ctx, presentation::Kind::StaticPostsPage, presentation::Object::Post(Box::new(page))).await?,
                        None => self.object_head(ctx, presentation::Kind::HomePage, presentation::Object::None).await?,
                    }
                } else if site.post_type_archive_link(name).is_some() {
                    self.object_head(ctx, presentation::Kind::PostTypeArchive, presentation::Object::PostType(name.to_owned())).await?
                } else {
                    None
                }
            }
        };
        let (html, json) = match head {
            Some((html, json)) => (serde_json::Value::String(html), json),
            None => (serde_json::Value::Null, serde_json::Value::Null),
        };
        Ok(vec![("yoast_head".to_owned(), html), ("yoast_head_json".to_owned(), json)])
    }
}

/// What a page about an object asked for (REST API, feed links) reads besides the request: its users, the global
/// post `prepare_item_for_response()` sets and the object itself.
pub struct TargetPage {
    users: HashMap<u64, User>,
    global_post: Option<Post>,
    dates_hidden: bool,
    target: (presentation::Kind, presentation::Object),
}

impl TargetPage {
    async fn load(site: &Site, options: &Options, kind: presentation::Kind, object: presentation::Object) -> Result<Self> {
        let mut user_ids: Vec<u64> = Vec::new();
        match &object {
            presentation::Object::Post(p) => user_ids.push(p.post_author),
            presentation::Object::User(u) => user_ids.push(u.id),
            _ => {}
        }
        user_ids.push(php_int(&options.string("company_or_person_user_id")).max(0) as u64);
        let mut users = HashMap::new();
        for id in user_ids.into_iter().filter(|id| *id != 0) {
            if let Some(user) = site.db.user(id).await? {
                users.insert(id, user);
            }
        }
        // `setup_postdata()` runs `the_post` for the post.
        let global_post = match &object {
            presentation::Object::Post(p) => Some((**p).clone()),
            _ => None,
        };
        let dates_hidden = match &global_post {
            Some(post) => site.plugins.hides_date(site, post).await?,
            None => false,
        };
        Ok(Self { users, global_post, dates_hidden, target: (kind, object) })
    }

    fn page<'a>(&'a self, site: &'a Site, query: &'a QueryResult, options: &'a Options, services: &'a dyn PageServices) -> Page<'a> {
        Page {
            site,
            query,
            options,
            services,
            global_post: self.global_post.as_ref(),
            users: self.users.clone(),
            get: Vec::new(),
            now: chrono::Utc::now().timestamp(),
            admin_bar: false,
            dates_hidden: self.dates_hidden,
            target: Some(self.target.clone()),
            custom_query_loop: false,
            pagination_next: None,
            pagination_previous: None,
        }
    }
}

impl Loaded {
    /// `Feed_Improvements::get_url_for_queried_object()`: the canonical of the queried object (the home page without
    /// one), as `Meta` gives it (empty for an object that is not indexed).
    async fn feed_url(&self, site: &Site, query: &QueryResult, services: &dyn PageServices) -> Result<(Option<String>, bool)> {
        use presentation::{Kind, Object};
        let (kind, object, has_object) = match &query.queried {
            rustpress_core::Queried::Post(p) => (Kind::PostType, Object::Post(p.clone()), true),
            rustpress_core::Queried::Term(t) => (Kind::TermArchive, Object::Term(t.clone()), true),
            rustpress_core::Queried::User(u) => (Kind::AuthorArchive, Object::User(u.clone()), true),
            rustpress_core::Queried::PostType(t) => (Kind::PostTypeArchive, Object::PostType(t.clone()), true),
            rustpress_core::Queried::None => return Ok((self.home_canonical(site, query, services).await?, false)),
        };
        let _ = has_object;
        Ok((self.canonical_of(site, query, services, kind, object).await?, true))
    }

    /// `Meta::canonical`: the canonical presenter's value.
    async fn canonical_of(&self, site: &Site, query: &QueryResult, services: &dyn PageServices, kind: presentation::Kind, object: presentation::Object) -> Result<Option<String>> {
        let Some((_, p)) = self.object_presentation(site, query, services, kind, object).await? else { return Ok(None) };
        if p.robots.iter().any(|(_, v)| v == "noindex") {
            return Ok(Some(String::new()));
        }
        Ok(Some(urldecode(p.canonical.trim())))
    }

    /// `Meta_Surface::for_home_page()->canonical`.
    async fn home_canonical(&self, site: &Site, query: &QueryResult, services: &dyn PageServices) -> Result<Option<String>> {
        let settings = &site.settings;
        if settings.show_on_front == rustpress_core::settings::ShowOnFront::Page && settings.page_on_front != 0 {
            return match site.db.post(settings.page_on_front).await? {
                Some(front) => self.canonical_of(site, query, services, presentation::Kind::StaticHomePage, presentation::Object::Post(Box::new(front))).await,
                None => Ok(None),
            };
        }
        self.canonical_of(site, query, services, presentation::Kind::HomePage, presentation::Object::None).await
    }

    /// `RSS_Footer_Embed::rss_replace_vars()`.
    async fn rss_text(&self, site: &Site, post: &Post, template: &str) -> Result<String> {
        if template.is_empty() {
            return Ok(String::new());
        }
        let link = |url: &str, text: &str| format!("<a href=\"{}\">{}</a>", rustpress_core::formatting::esc_url(url), rustpress_core::formatting::esc_html(text));
        let author = match site.db.user(post.post_author).await? {
            Some(user) => link(&site.author_url(&user), &user.display_name),
            None => String::new(),
        };
        let name = &site.settings.blogname;
        let home = site.home_url("");
        let vars = [
            ("%%AUTHORLINK%%", author),
            ("%%POSTLINK%%", link(&site.permalink(post).await?, &rustpress_blocks::content::the_title_for(site, post))),
            ("%%BLOGLINK%%", link(&home, name)),
            ("%%BLOGDESCLINK%%", format!("<a href=\"{}\">{} - {}</a>", rustpress_core::formatting::esc_url(&home), rustpress_core::formatting::esc_html(name), rustpress_core::formatting::esc_html(&site.settings.blogdescription))),
        ];
        let mut text = head::stripslashes(php_trim(template));
        for (key, value) in &vars {
            text = replace_ignore_case(&text, key, value);
        }
        Ok(rustpress_core::formatting::wpautop(&text))
    }
}

/// `str_ireplace()` for an ASCII needle.
fn replace_ignore_case(text: &str, needle: &str, value: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let needle_lower = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find(&needle_lower) {
        out.push_str(&text[at..at + found]);
        out.push_str(value);
        at += found + needle.len();
    }
    out.push_str(&text[at..]);
    out
}

impl Loaded {
    /// `Breadcrumbs_Presenter` with the presentation of the current page (`Meta_Tags_Context_Memoizer::for_current_page()`).
    async fn current_breadcrumbs(&self, site: &Site, q: &QueryResult, services: &dyn PageServices) -> Result<String> {
        let (global_post, users, dates_hidden) = self.current(site, q).await?;
        let page = Page {
            site,
            query: q,
            options: &self.options,
            services,
            global_post,
            users,
            get: query_pairs(&q.raw_query),
            now: chrono::Utc::now().timestamp(),
            admin_bar: false,
            dates_hidden,
            target: None,
            custom_query_loop: false,
            pagination_next: None,
            pagination_previous: None,
        };
        let presentation = presentation::present(&page).await?;
        breadcrumbs::present(&page, &presentation).await
    }
}

impl ActivePlugin for Loaded {
    fn block_names(&self) -> &'static [&'static str] {
        &["yoast-seo/breadcrumbs"]
    }

    /// `Breadcrumbs_Block::present()`.
    fn render_block<'a>(&'a self, _name: &'a str, attrs: &'a serde_json::Map<String, serde_json::Value>, ctx: &'a rustpress_core::native::BlockContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            // In the REST API the block shows the breadcrumbs of the post (`Meta_Tags_Context_Memoizer::get( $indexable,
            // 'Post_Type' )`).
            let presented = match ctx.post.filter(|post| ctx.rest && post.id != 0) {
                Some(post) => {
                    let target = TargetPage::load(ctx.site, &self.options, presentation::Kind::PostType, presentation::Object::Post(Box::new(post.clone()))).await?;
                    let page = target.page(ctx.site, ctx.query, &self.options, ctx.services);
                    breadcrumbs::present(&page, &presentation::present(&page).await?).await?
                }
                None => self.current_breadcrumbs(ctx.site, ctx.query, ctx.services).await?,
            };
            Ok(breadcrumbs::block(attrs, &presented))
        })
    }

    fn shortcode_tags(&self) -> &'static [&'static str] {
        &["wpseo_breadcrumb"]
    }

    /// `Breadcrumbs_Integration::render()`.
    fn shortcode<'a>(&'a self, _tag: &'a str, _atts: &'a rustpress_core::shortcodes::Atts, _content: Option<&'a str>, ctx: &'a rustpress_core::native::ShortcodeContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(self.current_breadcrumbs(ctx.site, ctx.query, ctx.services))
    }

    /// `Feed_Improvements`: the channel link and the canonical header, `X-Robots-Tag` on comment feeds.
    fn feed<'a>(&'a self, ctx: &'a rustpress_core::native::FeedContext<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::FeedChanges>>> {
        Box::pin(async move {
            let (url, has_object) = self.feed_url(ctx.site, ctx.query, ctx.services).await?;
            let mut headers = Vec::new();
            // `send_canonical_header()` on `do_feed_rss2` (not for comments) and `do_feed_rss`.
            if ctx.kind == "rss" || (ctx.kind == "rss2" && !ctx.comments) {
                let home = self.home_canonical(ctx.site, ctx.query, ctx.services).await?.unwrap_or_default();
                let link = url.clone().unwrap_or_default();
                if (!link.is_empty() && link != home) || !has_object {
                    headers.push(("Link".to_owned(), format!("<{link}>; rel=\"canonical\"")));
                }
            }
            if ctx.kind == "rss2" && ctx.comments {
                headers.push(("X-Robots-Tag".to_owned(), "noindex, follow".to_owned()));
            }
            Ok(Some(rustpress_core::native::FeedChanges { url, headers }))
        })
    }

    /// `RSS_Footer_Embed`: the texts before and after every item.
    fn feed_item<'a>(&'a self, site: &'a Site, post: &'a Post, excerpt: bool, text: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let before = self.options.string("rssbefore");
            let after = self.options.string("rssafter");
            if before.is_empty() && !options::truthy(&rustpress_phpser::Value::Str(after.clone().into_bytes())) {
                return Ok(text);
            }
            let text = if excerpt { rustpress_core::formatting::wpautop(&text) } else { text };
            Ok(format!("{}{text}{}", self.rss_text(site, post, &before).await?, self.rss_text(site, post, &after).await?))
        })
    }

    /// `Handle_404::is_feed_404()`: a feed of an archive or a post without posts and without a queried object.
    fn feed_not_found(&self, q: &QueryResult) -> bool {
        q.flags.is_feed && q.posts.is_empty() && matches!(q.queried, rustpress_core::Queried::None) && (q.flags.is_archive || q.flags.is_singular())
    }

    fn rest_fields<'a>(&'a self, object: RestObject<'a>, ctx: &'a RestFieldContext<'a>) -> BoxFuture<'a, Result<Vec<(String, serde_json::Value)>>> {
        Box::pin(self.rest_fields_of(object, ctx))
    }

    fn wp_head<'a>(&'a self, ctx: &'a HeadContext<'a>, robots: &'a [(String, Robot)]) -> BoxFuture<'a, Result<Option<Head>>> {
        Box::pin(async move { Ok(Some(self.head(ctx, robots).await?)) })
    }

    /// The XML sitemaps and their stylesheet.
    fn serve<'a>(&'a self, site: &'a Site, request: &'a ServeRequest<'a>) -> BoxFuture<'a, Result<Option<Serve>>> {
        Box::pin(async move {
            if !self.options.truthy("enable_xml_sitemap") {
                return Ok(None);
            }
            sitemaps::serve(site, &self.options, request).await
        })
    }

    /// `Robots_Txt_Integration::filter_robots()`: the block of WordPress gives way to the plugin's.
    fn robots_txt(&self, site: &Site, text: String) -> String {
        robots_txt(site, &self.options, &text)
    }

    /// `Indexing_Controls::noindex_robots()`.
    fn robots_txt_headers(&self) -> &'static [(&'static str, &'static str)] {
        &[("x-robots-tag", "noindex, follow")]
    }

    /// `Disable_Core_Sitemaps::maybe_disable_core_sitemaps()`.
    fn disables_core_sitemaps(&self) -> bool {
        self.options.truthy("enable_xml_sitemap")
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

/// `Robots_Txt_Integration::filter_robots()` with `Robots_Txt_Presenter::present()`.
fn robots_txt(site: &Site, options: &Options, text: &str) -> String {
    static DEFAULT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"User-agent: \*[\r\n]+Disallow: /wp-admin/[\r\n]+Allow: /wp-admin/admin-ajax\.php[\r\n]+").expect("valid pattern"));
    let text = DEFAULT.replace(text, "").into_owned();
    // `register_robots_rules`: the user agents in the order they were first named.
    let mut agents: Vec<(&str, Vec<&str>)> = Vec::new();
    let mut disallow = |agent: &'static str, path: &'static str| match agents.iter_mut().find(|(a, _)| *a == agent) {
        Some((_, paths)) => paths.push(path),
        None => agents.push((agent, vec![path])),
    };
    if options.truthy("deny_search_crawling") {
        disallow("*", "/?s=");
        disallow("*", "/page/*/?s=");
        disallow("*", "/search/");
    }
    if options.truthy("deny_wp_json_crawling") {
        disallow("*", "/wp-json/");
        disallow("*", "/?rest_route=");
    }
    if options.truthy("deny_adsbot_crawling") {
        disallow("AdsBot", "/");
    }
    let mut block = String::from("# START YOAST BLOCK\n# ---------------------------\n");
    if !agents.iter().any(|(a, _)| *a == "*") {
        block.push_str("User-agent: *\nDisallow:\n\n");
    }
    for (agent, paths) in &agents {
        block.push_str(&format!("User-agent: {agent}\n"));
        for path in paths {
            block.push_str(&format!("Disallow: {path}\n"));
        }
        block.push('\n');
    }
    if options.truthy("enable_xml_sitemap") {
        let base = if site.using_index_permalinks() { "index.php/" } else { "/" };
        block.push_str(&format!("Sitemap: {}\n", rustpress_core::formatting::esc_url(&site.home_url(&format!("{base}sitemap_index.xml")))));
    }
    block.push_str("# ---------------------------\n# END YOAST BLOCK");
    php_trim(&format!("{text}\n{block}\n")).to_owned()
}

/// PHP's `trim()`.
pub(crate) fn php_trim(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0b'])
}
