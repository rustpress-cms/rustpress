//! The indexable of the current page (`Indexable_*_Builder`, built from the data each time instead of being read
//! from `wp_yoast_indexable`) and its presentation (`Indexable_*_Presentation`).

use rustpress_core::urls::{add_query_arg, trailingslashit};
use rustpress_core::{Queried, Result};
use rustpress_db::{Post, Term, User};

use super::images::{self, ImageData};
use super::replace_vars::{Source, php_trim, strip_all_tags_breaks};
use super::Page;

/// `Current_Page_Helper::get_page_type()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    SearchResult,
    StaticPostsPage,
    StaticHomePage,
    HomePage,
    PostType,
    PostTypeArchive,
    TermArchive,
    AuthorArchive,
    DateArchive,
    ErrorPage,
    Fallback,
}

/// What the presentation reads of the queried object.
#[derive(Debug, Clone)]
pub enum Object {
    None,
    Post(Box<Post>),
    Term(Term),
    User(User),
    PostType(String),
}

/// The indexable (`Yoast\WP\SEO\Models\Indexable`) of the current page.
#[derive(Debug, Clone, Default)]
pub struct Model {
    pub object_type: &'static str,
    pub object_sub_type: String,
    pub object_id: u64,
    pub permalink: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical: Option<String>,
    pub breadcrumb_title: Option<String>,
    pub noindex: Option<bool>,
    pub nofollow: Option<bool>,
    pub noarchive: Option<bool>,
    pub noimageindex: Option<bool>,
    pub nosnippet: Option<bool>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_image_id: Option<u64>,
    pub og_image_source: Option<String>,
    pub og_image_meta: Option<ImageData>,
    pub twitter_title: Option<String>,
    pub twitter_description: Option<String>,
    pub twitter_image: Option<String>,
    pub twitter_image_id: Option<u64>,
    pub twitter_image_source: Option<String>,
    pub reading_time: Option<i64>,
    pub number_of_pages: Option<u64>,
    pub post_status: String,
    pub author_id: u64,
    pub schema_page_type: Option<String>,
    pub schema_article_type: Option<String>,
}

/// `if ( $value )` of a string column.
pub fn set(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|s| !s.is_empty() && s != "0")
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

/// `WPSEO_Meta::get_value( $key, $post_id )`: `_yoast_wpseo_{key}`, else the field's default.
pub async fn post_meta(page: &Page<'_>, post_id: u64, key: &str) -> Result<String> {
    let raw = page.site.db.post_meta_value(post_id, &format!("_yoast_wpseo_{key}")).await?;
    Ok(meta_or_default(raw, key))
}

/// The stored value unless it is a serialized array (none of the fields used here allows one), else the default of
/// the field (`WPSEO_Meta::$defaults`).
fn meta_or_default(raw: Option<String>, key: &str) -> String {
    if let Some(raw) = raw
        && !matches!(rustpress_core::options::maybe_unserialize(&raw), rustpress_phpser::Value::Array(_) | rustpress_phpser::Value::Object { .. })
    {
        return raw;
    }
    match key {
        "linkdex" | "content_score" | "inclusive_language_score" | "seo_title_score" | "meta_description_score" | "meta-robots-noindex" | "meta-robots-nofollow" => "0",
        "is_cornerstone" => "false",
        _ => "",
    }
    .to_owned()
}

impl Page<'_> {
    /// `Current_Page_Helper::get_page_type()`.
    pub fn kind(&self) -> Kind {
        let q = self.query;
        let f = &q.flags;
        let settings = &self.site.settings;
        let static_front = settings.show_on_front == rustpress_core::settings::ShowOnFront::Page;
        if f.is_search {
            return Kind::SearchResult;
        }
        if f.is_posts_page && matches!(&q.queried, Queried::Post(p) if p.post_type == "page") {
            return Kind::StaticPostsPage;
        }
        if f.is_front_page && static_front && f.is_page && matches!(&q.queried, Queried::Post(p) if p.id == settings.page_on_front) {
            return Kind::StaticHomePage;
        }
        if f.is_home && (settings.page_on_front == 0 || !static_front) {
            return Kind::HomePage;
        }
        if self.simple_page_id() > 0 {
            return Kind::PostType;
        }
        if f.is_post_type_archive {
            return Kind::PostTypeArchive;
        }
        if f.is_tax || f.is_tag || f.is_category {
            return Kind::TermArchive;
        }
        if f.is_author {
            return Kind::AuthorArchive;
        }
        if f.is_date {
            return Kind::DateArchive;
        }
        if f.is_404 {
            return Kind::ErrorPage;
        }
        Kind::Fallback
    }

    /// `Current_Page_Helper::get_simple_page_id()`.
    pub fn simple_page_id(&self) -> u64 {
        let q = self.query;
        if q.flags.is_singular() {
            return match &q.queried {
                Queried::Post(p) => p.id,
                _ => 0,
            };
        }
        if q.flags.is_home && self.site.settings.show_on_front == rustpress_core::settings::ShowOnFront::Page {
            return self.site.settings.page_for_posts;
        }
        0
    }

    /// `Url_Helper::home()`.
    pub fn home(&self) -> String {
        let home = self.site.home_url("");
        let path = home.split_once("://").map_or("", |(_, rest)| rest.find('/').map_or("", |i| &rest[i..]));
        if path == "/" {
            home
        } else if path.is_empty() {
            trailingslashit(&home)
        } else {
            self.site.user_trailingslashit(&home)
        }
    }

    /// The host of `home_url()`.
    pub fn home_host(&self) -> String {
        let home = self.site.home_url("");
        let rest = home.split_once("://").map_or(home.as_str(), |(_, r)| r);
        rest.split(['/', ':']).next().unwrap_or_default().to_owned()
    }

    /// `wp_upload_dir()['basedir']`.
    pub fn uploads_dir(&self) -> std::path::PathBuf {
        self.services.wp_root().join("wp-content").join("uploads")
    }

    /// `$_GET[ $key ]`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.get.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// `Pagination_Helper::get_key_query_loop()`: a `query-N-page` parameter.
    fn query_loop_key(&self) -> Option<&str> {
        static KEY: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"^query-\d+-page$").expect("valid pattern"));
        self.get.iter().find(|(k, _)| KEY.is_match(k)).map(|(k, _)| k.as_str())
    }

    /// `Pagination_Helper::get_page_number_from_query_loop()`.
    fn query_loop_page(&self) -> u64 {
        self.query_loop_key().and_then(|k| self.get(k)).map_or(0, super::php_int).max(0) as u64
    }

    /// `Pagination_Helper::get_current_archive_page_number()`.
    pub fn archive_page_number(&self) -> u64 {
        let paged = self.query.vars.u64("paged").unwrap_or(0);
        if paged > 1 {
            return paged;
        }
        let loop_page = self.query_loop_page();
        if loop_page > 1 { loop_page } else { 0 }
    }

    /// `Pagination_Helper::get_current_post_page_number()`.
    pub fn post_page_number(&self) -> u64 {
        let loop_page = self.query_loop_page();
        if loop_page > 1 { loop_page } else { self.query.vars.u64("page").unwrap_or(0) }
    }

    /// `Pagination_Helper::get_current_page_number()`.
    pub fn page_number(&self) -> u64 {
        let paged = self.query.vars.u64("paged").unwrap_or(1);
        if paged > 1 {
            return paged;
        }
        let loop_page = self.query_loop_page();
        if loop_page > 1 {
            return loop_page;
        }
        self.query.vars.u64("page").unwrap_or(1)
    }

    /// `Pagination_Helper::get_paginated_url()`.
    pub fn paginated_url(&self, url: &str, page: u64, add_base: bool) -> String {
        let key = self.query_loop_key();
        if self.site.uses_pretty_permalinks() && key.is_none() {
            if let Some((base, query)) = url.split_once('?') {
                let mut url = trailingslashit(base);
                if add_base {
                    url.push_str("page/");
                }
                let url = self.site.user_trailingslashit(&format!("{url}{page}"));
                return format!("{url}?{query}");
            }
            let mut url = trailingslashit(url);
            if add_base {
                url.push_str("page/");
            }
            return self.site.user_trailingslashit(&format!("{url}{page}"));
        }
        add_query_arg(&self.site.user_trailingslashit(url), key.unwrap_or("page"), &page.to_string())
    }

    /// `Current_Page_Helper::get_date_archive_permalink()`.
    pub fn date_archive_permalink(&self) -> String {
        let q = self.query;
        // `get_year_link()` & co. take today for a missing part (`?m=202610` leaves `year` empty).
        let today = |format: &str| self.site.date(format, self.now).parse::<u32>().unwrap_or(0);
        let year = match q.vars.u64("year").unwrap_or(0) as u32 {
            0 => today("Y"),
            y => y,
        };
        let month = match q.vars.u64("monthnum").unwrap_or(0) as u32 {
            0 => today("m"),
            m => m,
        };
        let day = match q.vars.u64("day").unwrap_or(0) as u32 {
            0 => today("j"),
            d => d,
        };
        if q.flags.is_year {
            self.site.date_link(year, 0, 0)
        } else if q.flags.is_month {
            self.site.date_link(year, month, 0)
        } else if q.flags.is_day {
            self.site.date_link(year, month, day)
        } else {
            String::new()
        }
    }

    /// `post_password_required( $post )` for a visitor without the cookie.
    pub fn password_required(post: &Post) -> bool {
        !post.post_password.is_empty()
    }

    /// `Post_Type_Helper::is_indexable()`.
    pub fn post_type_indexable(&self, post_type: &str) -> bool {
        if self.options.truthy(&format!("disable-{post_type}")) {
            return false;
        }
        matches!(self.options.get(&format!("noindex-{post_type}")), None | Some(rustpress_phpser::Value::Bool(false)))
    }
}

/// Builds the indexable of the current page (`Indexable_Repository::for_current_page()`).
pub async fn model(page: &Page<'_>) -> Result<(Model, Object)> {
    let q = page.query;
    let settings = &page.site.settings;
    let static_front = settings.show_on_front == rustpress_core::settings::ShowOnFront::Page;
    let simple = page.simple_page_id();
    let post_id = if simple > 0 {
        Some(simple)
    } else if q.flags.is_front_page && static_front && q.flags.is_page {
        Some(settings.page_on_front)
    } else {
        None
    };
    if let Some(id) = post_id {
        let post = match &q.queried {
            Queried::Post(p) if p.id == id => Some((**p).clone()),
            _ => page.site.db.post(id).await?,
        };
        if let Some(post) = post {
            let model = post_model(page, &post).await?;
            return Ok((model, Object::Post(Box::new(post))));
        }
    } else if q.flags.is_home && (settings.page_on_front == 0 || !static_front) {
        return Ok((home_model(page).await?, Object::None));
    } else if q.flags.is_tax || q.flags.is_tag || q.flags.is_category {
        if let Queried::Term(term) = &q.queried {
            return Ok((term_model(page, term).await?, Object::Term(term.clone())));
        }
    } else if q.flags.is_date {
        let model = Model {
            object_type: "date-archive",
            title: Some(page.options.string("title-archive-wpseo")),
            description: Some(page.options.string("metadesc-archive-wpseo")),
            noindex: Some(page.options.truthy("noindex-archive-wpseo")),
            ..Model::default()
        };
        return Ok((model, Object::None));
    } else if q.flags.is_search {
        return Ok((system_model(page, "search-result"), Object::None));
    } else if q.flags.is_post_type_archive {
        let post_type = q.vars.get("post_type").unwrap_or_default().split(',').next().unwrap_or_default().to_owned();
        return Ok((post_type_archive_model(page, &post_type), Object::PostType(post_type)));
    } else if q.flags.is_author {
        // `check_if_user_should_be_indexed()`: no indexable without author archives, or for an author without public
        // posts while those are not indexed.
        if let Queried::User(user) = &q.queried
            && !page.options.truthy("disable-author")
            && !(page.options.is_true("noindex-author-noposts-wpseo") && page.site.db.count_published_user_posts("post", user.id).await? == 0)
        {
            let model = author_model(page, user).await?;
            return Ok((model, Object::User(user.clone())));
        }
    } else if q.flags.is_404 {
        return Ok((system_model(page, "404"), Object::None));
    }
    Ok((Model { object_type: "unknown", post_status: "unindexed".into(), ..Model::default() }, Object::None))
}

/// `Indexable_Post_Builder::build()`.
pub async fn post_model(page: &Page<'_>, post: &Post) -> Result<Model> {
    // `get_post_custom()`: all fields of the post at once, the first value of each.
    let stored = page.site.db.post_meta(post.id).await?;
    let meta = |key: &'static str| {
        let name = format!("_yoast_wpseo_{key}");
        let raw = stored.iter().find(|m| m.meta_key.as_deref() == Some(name.as_str())).and_then(|m| m.meta_value.clone());
        std::future::ready(Ok::<_, rustpress_core::Error>(meta_or_default(raw, key)))
    };
    let permalink = if post.post_type == "attachment" {
        images::attachment(page, post.id).await?.and_then(|a| a.url(page))
    } else {
        Some(page.site.permalink(post).await?)
    };
    let noindex = match super::php_int(&meta("meta-robots-noindex").await?) {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    };
    let adv = meta("meta-robots-adv").await?;
    let adv: Vec<&str> = adv.split(',').collect();
    let opt = |v: String| non_empty(v);
    let id = |v: String| v.trim().parse::<u64>().ok().filter(|n| *n != 0);
    let mut model = Model {
        object_type: "post",
        object_sub_type: post.post_type.clone(),
        object_id: post.id,
        permalink,
        title: opt(meta("title").await?),
        description: opt(meta("metadesc").await?),
        canonical: opt(meta("canonical").await?),
        breadcrumb_title: opt(meta("bctitle").await?),
        noindex,
        nofollow: Some(meta("meta-robots-nofollow").await? == "1"),
        noimageindex: adv.contains(&"noimageindex").then_some(true),
        noarchive: adv.contains(&"noarchive").then_some(true),
        nosnippet: adv.contains(&"nosnippet").then_some(true),
        og_title: opt(meta("opengraph-title").await?),
        og_description: opt(meta("opengraph-description").await?),
        og_image: opt(meta("opengraph-image").await?),
        og_image_id: id(meta("opengraph-image-id").await?),
        twitter_title: opt(meta("twitter-title").await?),
        twitter_description: opt(meta("twitter-description").await?),
        twitter_image: opt(meta("twitter-image").await?),
        twitter_image_id: id(meta("twitter-image-id").await?),
        reading_time: opt(meta("estimated-reading-time-minutes").await?).map(|v| super::php_int(&v)),
        number_of_pages: Some(post.post_content.matches("<!--nextpage-->").count() as u64 + 1).filter(|n| *n > 1),
        post_status: post.post_status.clone(),
        author_id: post.post_author,
        schema_page_type: opt(meta("schema_page_type").await?),
        schema_article_type: opt(meta("schema_article_type").await?),
        ..Model::default()
    };
    if model.breadcrumb_title.is_none() {
        model.breadcrumb_title = Some(strip_all_tags_breaks(&rustpress_blocks::content::the_title_for(page.site, post)));
    }
    social_images(page, &mut model, || alternative_post_image(page, post)).await?;
    Ok(model)
}

/// `Indexable_Social_Image_Trait::handle_social_images()`.
async fn social_images<F, Fut>(page: &Page<'_>, model: &mut Model, alternative: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Option<Alternative>>>,
{
    if set(&model.og_image) || model.og_image_id.is_some() {
        model.og_image_source = Some("set-by-user".into());
        og_image_meta(page, model).await?;
    }
    if set(&model.twitter_image) || model.twitter_image_id.is_some() {
        model.twitter_image_source = Some("set-by-user".into());
    }
    if let Some(id) = model.twitter_image_id {
        model.twitter_image = non_empty(images::twitter_image(page, id).await?);
    }
    if model.og_image_source.is_some() && model.twitter_image_source.is_some() {
        return Ok(());
    }
    let Some(alternative) = alternative().await? else { return Ok(()) };
    match alternative {
        Alternative::Id(id, source) => {
            if model.og_image_source.is_none() && model.og_image_id.is_none() {
                model.og_image_id = Some(id);
                model.og_image_source = Some(source.into());
                og_image_meta(page, model).await?;
            }
            if !set(&model.twitter_image) && model.twitter_image_id.is_none() {
                model.twitter_image = non_empty(images::twitter_image(page, id).await?);
                model.twitter_image_id = Some(id);
                model.twitter_image_source = Some(source.into());
            }
        }
        Alternative::Url(url, source) => {
            if model.og_image_source.is_none() && model.og_image_id.is_none() {
                model.og_image = Some(url.clone());
                model.og_image_source = Some(source.into());
            }
            if !set(&model.twitter_image) && model.twitter_image_id.is_none() {
                model.twitter_image = Some(url);
                model.twitter_image_source = Some(source.into());
            }
        }
    }
    Ok(())
}

/// `set_open_graph_image_meta_data()`.
async fn og_image_meta(page: &Page<'_>, model: &mut Model) -> Result<()> {
    let Some(id) = model.og_image_id else { return Ok(()) };
    if let Some(image) = images::open_graph_image(page, id).await? {
        model.og_image = Some(image.url.clone());
        model.og_image_meta = Some(image);
    }
    Ok(())
}

/// An image found when none is set (`find_alternative_image()`).
pub enum Alternative {
    Id(u64, &'static str),
    Url(String, &'static str),
}

/// `Indexable_Post_Builder::find_alternative_image()`.
async fn alternative_post_image(page: &Page<'_>, post: &Post) -> Result<Option<Alternative>> {
    if post.post_type == "attachment"
        && let Some(a) = images::attachment(page, post.id).await?
        && a.is_valid()
    {
        return Ok(Some(Alternative::Id(post.id, "attachment-image")));
    }
    let thumbnail = thumbnail_id(page, post.id).await?;
    if thumbnail != 0 {
        return Ok(Some(Alternative::Id(thumbnail, "featured-image")));
    }
    if post.post_content.contains("[gallery")
        && let Some(url) = gallery_image(page, post).await?
    {
        return Ok(Some(Alternative::Url(url, "gallery-image")));
    }
    if let Some(url) = images::first_content_image(&post.post_content) {
        return Ok(Some(Alternative::Url(url, "first-content-image")));
    }
    Ok(None)
}

/// `get_post_gallery_images()`, first image: the first `src` of the first `[gallery]` as WordPress prints it (in the
/// gallery's `size`, `thumbnail` by default).
async fn gallery_image(page: &Page<'_>, post: &Post) -> Result<Option<String>> {
    let Some(start) = post.post_content.find("[gallery") else { return Ok(None) };
    let rest = &post.post_content[start + "[gallery".len()..];
    let Some(end) = rest.find(']') else { return Ok(None) };
    let atts = rustpress_core::shortcodes::parse_atts(&rest[..end]);
    let size = atts.get("size").filter(|s| !s.is_empty()).unwrap_or("thumbnail").to_owned();
    let list = atts.get("ids").filter(|s| !s.is_empty()).or_else(|| atts.get("include")).unwrap_or_default().to_owned();
    let mut ids: Vec<u64> = list.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    if ids.is_empty() {
        ids = page.site.db.attachment_children(post.id).await?;
    }
    for id in ids {
        if let Some(a) = images::attachment(page, id).await?
            && a.is_image()
            && let Some(src) = a.image_src(page, &size)
        {
            return Ok(Some(src.0));
        }
    }
    Ok(None)
}

/// `get_post_thumbnail_id()` when `has_post_thumbnail()`.
pub async fn thumbnail_id(page: &Page<'_>, post_id: u64) -> Result<u64> {
    Ok(page.site.db.post_meta_value(post_id, "_thumbnail_id").await?.and_then(|v| v.trim().parse().ok()).unwrap_or(0))
}

/// `Indexable_Home_Page_Builder::build()`.
async fn home_model(page: &Page<'_>) -> Result<Model> {
    let o = page.options;
    let mut description = o.string("metadesc-home-wpseo");
    if description.is_empty() {
        description = page.site.settings.blogdescription.clone();
    }
    let mut model = Model {
        object_type: "home-page",
        title: Some(o.string("title-home-wpseo")),
        breadcrumb_title: Some(o.string("breadcrumbs-home")),
        permalink: Some(page.home()),
        description: Some(description),
        noindex: Some(page.site.options.get("blog_public") == Some("0")),
        og_title: Some(o.string("open_graph_frontpage_title")),
        og_image: Some(o.string("open_graph_frontpage_image")),
        og_image_id: o.string("open_graph_frontpage_image_id").trim().parse().ok().filter(|n| *n != 0),
        og_description: Some(o.string("open_graph_frontpage_desc")),
        ..Model::default()
    };
    if set(&model.og_image) || model.og_image_id.is_some() {
        model.og_image_source = Some("set-by-user".into());
        og_image_meta(page, &mut model).await?;
    }
    Ok(model)
}

/// `Indexable_Term_Builder::build()`.
async fn term_model(page: &Page<'_>, term: &Term) -> Result<Model> {
    let meta = page.options.term_meta(&term.taxonomy, term.term_id);
    let get = |key: &str| meta.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).filter(|v| !v.is_empty());
    let mut model = Model {
        object_type: "term",
        object_sub_type: term.taxonomy.clone(),
        object_id: term.term_id,
        permalink: Some(page.site.term_link(term).await?),
        noindex: match get("wpseo_noindex").as_deref() {
            Some("noindex") => Some(true),
            Some("index") => Some(false),
            _ => None,
        },
        canonical: get("wpseo_canonical"),
        title: get("wpseo_title"),
        description: get("wpseo_desc"),
        breadcrumb_title: get("wpseo_bctitle"),
        og_title: get("wpseo_opengraph-title"),
        og_description: get("wpseo_opengraph-description"),
        og_image: get("wpseo_opengraph-image"),
        og_image_id: get("wpseo_opengraph-image-id").and_then(|v| v.trim().parse().ok()).filter(|n| *n != 0),
        twitter_title: get("wpseo_twitter-title"),
        twitter_description: get("wpseo_twitter-description"),
        twitter_image: get("wpseo_twitter-image"),
        twitter_image_id: get("wpseo_twitter-image-id").and_then(|v| v.trim().parse().ok()).filter(|n| *n != 0),
        ..Model::default()
    };
    if !set(&model.breadcrumb_title) {
        model.breadcrumb_title = Some(page.site.term_name(term));
    }
    let description = term.description.clone();
    social_images(page, &mut model, || async move {
        // `Image_Helper::get_term_content_image()`: the first image of `term_description()`.
        let html = term_description(&description);
        Ok(images::first_content_image(&html).map(|url| Alternative::Url(url, "first-content-image")))
    })
    .await?;
    Ok(model)
}

/// `term_description()`: the description with the `term_description` filters.
pub fn term_description(description: &str) -> String {
    if description.is_empty() {
        return String::new();
    }
    let text = rustpress_core::formatting::wptexturize(description);
    let text = rustpress_core::formatting::convert_chars(&text);
    rustpress_core::formatting::wpautop(&text)
}

/// `Indexable_Author_Builder::build()`.
async fn author_model(page: &Page<'_>, user: &User) -> Result<Model> {
    let meta = |key: &'static str| async move { Ok::<_, rustpress_core::Error>(page.site.db.user_meta(user.id, key).await?.filter(|v| !v.is_empty())) };
    let noindex = meta("wpseo_noindex_author").await?.as_deref() == Some("on");
    let mut model = Model {
        object_type: "user",
        object_id: user.id,
        permalink: Some(page.site.author_url(user)),
        title: meta("wpseo_title").await?,
        description: meta("wpseo_metadesc").await?,
        noindex: Some(noindex),
        ..Model::default()
    };
    let email = user.user_email.clone();
    social_images(page, &mut model, || async move {
        let url = rustpress_blocks::content::avatar_url(&page.site.options, &email, 500).replacen("http://", "https://", 1);
        Ok(Some(Alternative::Url(url, "gravatar-image")))
    })
    .await?;
    Ok(model)
}

/// `Indexable_Post_Type_Archive_Builder::build()`.
fn post_type_archive_model(page: &Page<'_>, post_type: &str) -> Model {
    let o = page.options;
    let bctitle = o.string(&format!("bctitle-ptarchive-{post_type}"));
    Model {
        object_type: "post-type-archive",
        object_sub_type: post_type.to_owned(),
        title: Some(o.string(&format!("title-ptarchive-{post_type}"))),
        description: Some(o.string(&format!("metadesc-ptarchive-{post_type}"))),
        breadcrumb_title: Some(if bctitle.is_empty() { page.site.post_type_label(post_type) } else { bctitle }),
        permalink: page.site.post_type_archive_link(post_type),
        noindex: Some(o.truthy(&format!("noindex-ptarchive-{post_type}"))),
        ..Model::default()
    }
}

/// `Indexable_System_Page_Builder::build()`.
fn system_model(page: &Page<'_>, sub_type: &str) -> Model {
    let o = page.options;
    let (title, crumb) = if sub_type == "404" { ("title-404-wpseo", Some(o.string("breadcrumbs-404crumb"))) } else { ("title-search-wpseo", None) };
    Model { object_type: "system-page", object_sub_type: sub_type.to_owned(), title: Some(o.string(title)), noindex: Some(true), breadcrumb_title: crumb, ..Model::default() }
}

/// One `robots` value as the presentation keeps it (`index` => `noindex`, ...), in order.
pub type Robots = Vec<(String, String)>;

/// The presentation of the current page.
pub struct Presentation {
    pub kind: Kind,
    pub model: Model,
    pub object: Object,
    pub permalink: String,
    pub title: String,
    pub meta_description: String,
    pub robots: Robots,
    pub canonical: String,
    pub rel_prev: String,
    pub rel_next: String,
    pub og_type: &'static str,
    pub og_title: String,
    pub og_description: String,
    pub og_images: Vec<ImageData>,
    pub og_url: String,
    pub article_publisher: String,
    pub article_author: String,
    pub article_published_time: String,
    pub article_modified_time: String,
    pub og_site_name: String,
    pub twitter_title: String,
    pub twitter_description: String,
    pub twitter_image: String,
    pub twitter_creator: String,
    pub twitter_site: String,
    pub reading_time: Option<i64>,
}

impl Presentation {
    /// `$presentation->source` for the variables.
    pub fn source(&self) -> Source<'_> {
        match &self.object {
            Object::Post(p) if self.model.object_type == "post" => Source::Post(p),
            Object::Term(t) => Source::Term(t),
            Object::User(u) => Source::Author(u.id),
            Object::PostType(t) => Source::PostType(t),
            _ => Source::None,
        }
    }

    pub fn post(&self) -> Option<&Post> {
        match &self.object {
            Object::Post(p) if self.model.object_type == "post" => Some(p),
            _ => None,
        }
    }
}

/// `filter_robots()` with the plugin's own `wpseo_robots_array` filters.
fn filter_robots(page: &Page<'_>, robots: Vec<(&str, Option<String>)>) -> Robots {
    let noindex = robots.iter().any(|(k, v)| *k == "index" && v.as_deref() == Some("noindex"));
    let mut out: Robots = Vec::new();
    for (key, value) in robots {
        let Some(value) = value else { continue };
        if noindex && matches!(key, "imageindex" | "archive" | "snippet" | "max-snippet" | "max-image-preview" | "max-video-preview") {
            continue;
        }
        if value.is_empty() {
            continue;
        }
        // The string is split again: the key of `noindex` is `index`, of `max-snippet:-1` `max-snippet`.
        let mut k = value.strip_prefix("no").unwrap_or(&value).to_owned();
        if let Some(colon) = k.find(':') {
            k = value[..colon].to_owned();
        }
        match out.iter_mut().find(|(o, _)| *o == k) {
            Some((_, v)) => *v = value,
            None => out.push((k, value)),
        }
    }
    // `Indexing_Controls` (a site hidden from search engines) and `Comment_Link_Fixer` (`?replytocom`).
    if page.site.options.get("blog_public") == Some("0") || page.get("replytocom").is_some() {
        match out.iter_mut().find(|(k, _)| k == "index") {
            Some((_, v)) => *v = "noindex".into(),
            None => out.push(("index".into(), "noindex".into())),
        }
    }
    out
}

fn base_robots(model: &Model) -> Vec<(&'static str, Option<String>)> {
    vec![
        ("index", Some(if model.noindex == Some(true) { "noindex" } else { "index" }.to_owned())),
        ("follow", Some(if model.nofollow == Some(true) { "nofollow" } else { "follow" }.to_owned())),
        ("max-snippet", Some("max-snippet:-1".into())),
        ("max-image-preview", Some("max-image-preview:large".into())),
        ("max-video-preview", Some("max-video-preview:-1".into())),
    ]
}

fn set_robot(robots: &mut [(&'static str, Option<String>)], key: &str, value: Option<&str>) {
    if let Some((_, v)) = robots.iter_mut().find(|(k, _)| *k == key) {
        *v = value.map(str::to_owned);
    }
}

/// `generate_robots()` of the presentation.
async fn robots(page: &Page<'_>, kind: Kind, model: &Model, object: &Object) -> Result<Robots> {
    let o = page.options;
    let mut robots = base_robots(model);
    match kind {
        Kind::PostType | Kind::StaticHomePage | Kind::StaticPostsPage => {
            robots.push(("imageindex", (model.noimageindex == Some(true)).then(|| "noimageindex".into())));
            robots.push(("archive", (model.noarchive == Some(true)).then(|| "noarchive".into())));
            robots.push(("snippet", (model.nosnippet == Some(true)).then(|| "nosnippet".into())));
            if model.nosnippet == Some(true) {
                set_robot(&mut robots, "max-snippet", None);
            }
            if model.noimageindex == Some(true) {
                set_robot(&mut robots, "max-image-preview", None);
            }
            if model.noindex.is_none() && (model.post_status == "private" || !page.post_type_indexable(&model.object_sub_type)) {
                set_robot(&mut robots, "index", Some("noindex"));
            }
        }
        Kind::AuthorArchive => {
            let user = match object {
                Object::User(u) => Some(u),
                _ => None,
            };
            let noindex = if o.truthy("noindex-author-wpseo") || user.is_none() {
                true
            } else if let Some(user) = user {
                (o.truthy("noindex-author-noposts-wpseo") && count_public_posts(page, user.id).await? == 0)
                    || page.site.db.user_meta(user.id, "wpseo_noindex_author").await?.as_deref() == Some("on")
            } else {
                false
            };
            if noindex {
                set_robot(&mut robots, "index", Some("noindex"));
            }
        }
        Kind::DateArchive => {
            if o.truthy("noindex-archive-wpseo") {
                set_robot(&mut robots, "index", Some("noindex"));
            }
        }
        Kind::PostTypeArchive => {
            if o.truthy(&format!("noindex-ptarchive-{}", model.object_sub_type)) {
                set_robot(&mut robots, "index", Some("noindex"));
            }
        }
        Kind::TermArchive => {
            let taxonomy = match object {
                Object::Term(t) => t.taxonomy.clone(),
                _ => String::new(),
            };
            if multiple_terms(page) {
                set_robot(&mut robots, "index", Some("noindex"));
            } else {
                if o.truthy(&format!("noindex-tax-{taxonomy}")) {
                    set_robot(&mut robots, "index", Some("noindex"));
                }
                if let Some(noindex) = model.noindex {
                    set_robot(&mut robots, "index", Some(if noindex { "noindex" } else { "index" }));
                }
            }
        }
        Kind::SearchResult | Kind::ErrorPage => set_robot(&mut robots, "index", Some("noindex")),
        Kind::HomePage | Kind::Fallback => {}
    }
    Ok(filter_robots(page, robots))
}

/// `count_user_posts( $id, 'post', true )`: the author's public posts.
async fn count_public_posts(page: &Page<'_>, user_id: u64) -> Result<u64> {
    Ok(page.site.db.count_published_user_posts("post", user_id).await?)
}

/// `Current_Page_Helper::is_multiple_terms_page()`: a term archive for more than one term (`?cat=1,2`, `tag=a+b`).
fn multiple_terms(page: &Page<'_>) -> bool {
    let vars = &page.query.vars;
    ["cat", "tag", "category_name", "tag_id"].iter().filter_map(|v| vars.get(v)).any(|v| v.contains([',', '+', ' ']))
}

/// `generate_presentation()`: everything the presenters print, for the current page.
pub async fn present(page: &Page<'_>) -> Result<Presentation> {
    let kind = page.kind();
    let (model, object) = model(page).await?;
    let o = page.options;
    let post = match &object {
        Object::Post(p) if model.object_type == "post" => Some(&**p),
        _ => None,
    };
    let singular = matches!(kind, Kind::PostType | Kind::StaticHomePage | Kind::StaticPostsPage);

    // generate_permalink()
    let permalink = if page.query.flags.is_date {
        page.date_archive_permalink()
    } else if page.query.flags.is_attachment {
        trailingslashit(&page.site.home_url(page.query.url_path.trim_matches('/')))
    } else {
        model.permalink.clone().unwrap_or_default()
    };

    // generate_title()
    let title = match kind {
        _ if set(&model.title) => model.title.clone().unwrap_or_default(),
        Kind::PostType | Kind::StaticHomePage | Kind::StaticPostsPage => {
            let key = format!("title-{}", model.object_sub_type);
            let value = o.string(&key);
            if truthy_str(&value) { value } else { o.title_default(&key) }
        }
        Kind::HomePage => o.title_default("title-home-wpseo"),
        Kind::TermArchive => {
            let key = format!("title-tax-{}", model.object_sub_type);
            let value = o.string(&key);
            if truthy_str(&value) { value } else { o.title_default(&key) }
        }
        Kind::AuthorArchive => {
            let value = o.string("title-author-wpseo");
            if truthy_str(&value) { value } else { o.title_default("title-author-wpseo") }
        }
        Kind::PostTypeArchive => o.title_default(&format!("title-ptarchive-{}", model.object_sub_type)),
        Kind::DateArchive => o.title_default("title-archive-wpseo"),
        Kind::SearchResult => o.title_default("title-search-wpseo"),
        Kind::ErrorPage => o.title_default("title-404-wpseo"),
        Kind::Fallback => String::new(),
    };

    // generate_meta_description()
    let meta_description = match kind {
        _ if set(&model.description) => model.description.clone().unwrap_or_default(),
        Kind::PostType | Kind::StaticHomePage | Kind::StaticPostsPage => o.string(&format!("metadesc-{}", model.object_sub_type)),
        Kind::HomePage => o.string("metadesc-home-wpseo"),
        Kind::TermArchive => o.string(&format!("metadesc-tax-{}", model.object_sub_type)),
        Kind::AuthorArchive => {
            let value = o.string("metadesc-author-wpseo");
            if truthy_str(&value) { value } else { o.title_default("metadesc-author-wpseo") }
        }
        _ => String::new(),
    };

    let robots = robots(page, kind, &model, &object).await?;

    // generate_canonical()
    let archive_page = page.archive_page_number();
    let paged_archive = |url: &str| if archive_page > 1 { page.paginated_url(url, archive_page, true) } else { url.to_owned() };
    let canonical = match kind {
        Kind::StaticPostsPage => if set(&model.canonical) { model.canonical.clone().unwrap_or_default() } else { paged_archive(&permalink) },
        Kind::PostType | Kind::StaticHomePage => {
            if set(&model.canonical) {
                model.canonical.clone().unwrap_or_default()
            } else {
                let mut url = permalink.clone();
                let current = page.post_page_number();
                if current > 1 && model.number_of_pages.is_some_and(|n| current <= n) {
                    url = page.paginated_url(&url, current, kind == Kind::StaticHomePage);
                }
                ensure_absolute_url(page, &url)
            }
        }
        Kind::TermArchive if multiple_terms(page) => String::new(),
        Kind::HomePage | Kind::AuthorArchive | Kind::TermArchive => {
            if set(&model.canonical) {
                model.canonical.clone().unwrap_or_default()
            } else if permalink.is_empty() {
                String::new()
            } else {
                paged_archive(&permalink)
            }
        }
        Kind::PostTypeArchive => if permalink.is_empty() { String::new() } else { paged_archive(&permalink) },
        Kind::DateArchive => paged_archive(&page.date_archive_permalink()),
        _ => {
            if set(&model.canonical) {
                model.canonical.clone().unwrap_or_default()
            } else {
                permalink.clone()
            }
        }
    };

    // generate_rel_prev() / generate_rel_next()
    let (rel_prev, rel_next) = match kind {
        Kind::PostType | Kind::StaticHomePage => match model.number_of_pages {
            None => (String::new(), String::new()),
            Some(pages) => {
                let base = model.permalink.clone().unwrap_or_default();
                let current = page.post_page_number().max(1);
                let add_base = kind == Kind::StaticHomePage;
                let prev = match current {
                    0 | 1 => String::new(),
                    2 => base.clone(),
                    n => page.paginated_url(&base, n - 1, add_base),
                };
                let next = if pages <= current { String::new() } else { page.paginated_url(&base, current + 1, add_base) };
                (prev, next)
            }
        },
        Kind::HomePage | Kind::AuthorArchive | Kind::PostTypeArchive | Kind::StaticPostsPage | Kind::TermArchive | Kind::DateArchive => {
            let base = if kind == Kind::DateArchive { page.date_archive_permalink() } else { permalink.clone() };
            let current = archive_page.max(1);
            let prev = match current {
                1 => String::new(),
                2 => base.clone(),
                n => page.paginated_url(&base, n - 1, true),
            };
            let next = if page.query.max_pages <= current { String::new() } else { page.paginated_url(&base, current + 1, true) };
            (prev, next)
        }
        _ => (String::new(), String::new()),
    };

    let og_type = match kind {
        Kind::PostType | Kind::StaticPostsPage | Kind::TermArchive | Kind::SearchResult => "article",
        Kind::AuthorArchive => "profile",
        _ => "website",
    };
    let og_title = if set(&model.og_title) { model.og_title.clone().unwrap_or_default() } else { title.clone() };

    // generate_open_graph_description()
    let mut og_description = if set(&model.og_description) { model.og_description.clone().unwrap_or_default() } else { meta_description.clone() };
    let mut excerpt = None;
    if og_description.is_empty() {
        match (kind, post) {
            (Kind::PostType | Kind::StaticHomePage | Kind::StaticPostsPage, Some(post)) => {
                let text = rustpress_core::formatting::wp_strip_all_tags(&page.services.excerpt(post).await?);
                excerpt = Some(text.clone());
                og_description = page.site.strip_shortcodes(&text);
            }
            (Kind::TermArchive, _) => og_description = term_description_text(&object),
            _ => {}
        }
    }

    let open_graph_enabled = o.is_true("opengraph");
    // generate_open_graph_images()
    let og_images = if !open_graph_enabled || (singular && post.is_some_and(Page::password_required)) {
        Vec::new()
    } else if kind == Kind::AuthorArchive {
        let mut images = Vec::new();
        add_from_templates(page, &model, &mut images).await?;
        if images.is_empty() {
            open_graph_images(page, &model).await?
        } else {
            images
        }
    } else {
        open_graph_images(page, &model).await?
    };

    let og_url = match kind {
        Kind::DateArchive => page.date_archive_permalink(),
        Kind::SearchResult => {
            let words = page.query.vars.get("s").unwrap_or_default();
            static PAGE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"^page/\d+$").expect("valid pattern"));
            if !words.is_empty() && !PAGE.is_match(words) { search_link(page, words) } else { String::new() }
        }
        Kind::StaticPostsPage => permalink.clone(),
        _ => if set(&model.canonical) { model.canonical.clone().unwrap_or_default() } else { permalink.clone() },
    };

    let represents = super::schema::site_represents(page).await?;
    let (article_publisher, article_author, article_published_time, article_modified_time) = match post.filter(|_| singular) {
        Some(post) => {
            let publisher = match &represents {
                super::schema::Represents::Person(id) => super::replace_vars::author_meta(page, *id, "facebook").await?,
                _ => o.string("facebook_site"),
            };
            let author = if model.object_sub_type == "post" { super::replace_vars::author_meta(page, post.post_author, "facebook").await? } else { String::new() };
            let published = if model.object_sub_type == "post" { w3c(&post.post_date_gmt) } else { String::new() };
            let modified = if gmt(&post.post_modified_gmt) > gmt(&post.post_date_gmt) { w3c(&post.post_modified_gmt) } else { String::new() };
            (publisher, author, published, modified)
        }
        None => Default::default(),
    };

    // generate_twitter_title()
    let twitter_title = if kind == Kind::SearchResult {
        title.clone()
    } else if set(&model.twitter_title) {
        model.twitter_title.clone().unwrap_or_default()
    } else if open_graph_enabled && !og_title.is_empty() {
        String::new()
    } else {
        title.clone()
    };

    // generate_twitter_description()
    let mut twitter_description = if set(&model.twitter_description) {
        model.twitter_description.clone().unwrap_or_default()
    } else if open_graph_enabled && !og_description.is_empty() {
        String::new()
    } else {
        meta_description.clone()
    };
    if twitter_description.is_empty() && !(open_graph_enabled && !og_description.is_empty()) {
        match (kind, post) {
            (Kind::PostType | Kind::StaticHomePage | Kind::StaticPostsPage, Some(post)) => {
                twitter_description = match excerpt {
                    Some(text) => text,
                    None => rustpress_core::formatting::wp_strip_all_tags(&page.services.excerpt(post).await?),
                };
            }
            (Kind::TermArchive, _) => twitter_description = term_description_text(&object),
            _ => {}
        }
    }

    // generate_twitter_image()
    let twitter_image = if singular && post.is_some_and(Page::password_required) {
        String::new()
    } else {
        let image = twitter_images(page, &model).await?;
        match image {
            Some(url) if model.twitter_image_source.as_deref() == Some("set-by-user") => url,
            _ if open_graph_enabled && !og_images.is_empty() => String::new(),
            Some(url) => url,
            None => String::new(),
        }
    };

    let twitter_creator = match post.filter(|_| singular && model.object_sub_type == "post") {
        Some(post) => {
            let account = php_trim(&super::replace_vars::author_meta(page, post.post_author, "twitter").await?).trim_start_matches('@').to_owned();
            if !account.is_empty() {
                format!("@{account}")
            } else if !o.string("twitter_site").is_empty() {
                format!("@{}", o.string("twitter_site"))
            } else {
                String::new()
            }
        }
        None => String::new(),
    };
    let twitter_site = match &represents {
        super::schema::Represents::Person(id) => {
            let account = super::replace_vars::author_meta(page, *id, "twitter").await?;
            if account.is_empty() { o.string("twitter_site") } else { account }
        }
        _ => o.string("twitter_site"),
    };

    // generate_estimated_reading_time_minutes()
    let reading_time = match (model.reading_time, post) {
        (Some(minutes), _) => Some(minutes),
        (None, Some(post)) => Some((super::schema::str_word_count(&rustpress_core::formatting::wp_strip_all_tags(&post.post_content), "") as f64 / 200.0).round() as i64),
        (None, None) => None,
    };

    Ok(Presentation {
        kind,
        permalink,
        title,
        meta_description,
        robots,
        canonical,
        rel_prev,
        rel_next,
        og_type,
        og_title,
        og_description,
        og_images,
        og_url,
        article_publisher,
        article_author,
        article_published_time,
        article_modified_time,
        og_site_name: super::replace_vars::site_name(page),
        twitter_title,
        twitter_description,
        twitter_image,
        twitter_creator,
        twitter_site,
        reading_time,
        model,
        object,
    })
}

fn truthy_str(value: &str) -> bool {
    !value.is_empty() && value != "0"
}

/// `Taxonomy_Helper::get_term_description()`.
fn term_description_text(object: &Object) -> String {
    match object {
        Object::Term(t) => rustpress_core::formatting::wp_strip_all_tags(&term_description(&t.description)),
        _ => String::new(),
    }
}

/// `get_search_link()`.
pub fn search_link(page: &Page<'_>, words: &str) -> String {
    let site = page.site;
    if site.uses_pretty_permalinks() {
        let encoded = rustpress_core::canonical::php_urlencode(words).replace("%2F", "/");
        site.user_trailingslashit(&site.home_url(&format!("{}search/{encoded}", site.root())))
    } else {
        site.home_url(&format!("?s={}", rustpress_core::canonical::php_urlencode(words)))
    }
}

/// `Url_Helper::ensure_absolute_url()`.
pub fn ensure_absolute_url(page: &Page<'_>, url: &str) -> String {
    if url.is_empty() || url.starts_with("http") || url.starts_with("//") {
        return url.to_owned();
    }
    build_absolute_url(page, url)
}

/// `Url_Helper::build_absolute_url()`.
pub fn build_absolute_url(page: &Page<'_>, path: &str) -> String {
    let home = page.site.home_url("");
    let (scheme, rest) = home.split_once("://").unwrap_or(("http", &home));
    let host = rest.split(['/', ':']).next().unwrap_or_default();
    let path = path.split(['?', '#']).next().unwrap_or_default();
    format!("{scheme}://{host}/{}", path.trim_start_matches('/'))
}

/// `strtotime()` of a GMT date of the database.
fn gmt(date: &str) -> i64 {
    rustpress_core::datetime::SiteTimezone::gmt_timestamp(date).unwrap_or(0)
}

/// `Date_Helper::format()`: a GMT date of the database in `DATE_W3C`.
pub fn w3c(date: &str) -> String {
    match chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S") {
        Ok(d) => d.format("%Y-%m-%dT%H:%M:%S+00:00").to_string(),
        Err(_) => date.to_owned(),
    }
}

/// `Images::add_image()`: relative URLs become absolute, every URL once.
fn add_image(page: &Page<'_>, images: &mut Vec<ImageData>, mut image: ImageData) {
    if image.url.is_empty() {
        return;
    }
    if !(image.url.starts_with("http") || image.url.starts_with("//")) && image.url.starts_with('/') {
        image.url = build_absolute_url(page, &image.url);
    }
    if images.iter().any(|i| i.url == image.url) {
        return;
    }
    images.push(image);
}

/// `Open_Graph\Images::add_image_by_id()`.
async fn add_image_by_id(page: &Page<'_>, images: &mut Vec<ImageData>, id: u64) -> Result<()> {
    if let Some(image) = images::open_graph_image(page, id).await? {
        add_image(page, images, image);
    }
    Ok(())
}

/// `Images::add_image_by_url()`.
async fn add_image_by_url(page: &Page<'_>, images: &mut Vec<ImageData>, url: &str) -> Result<()> {
    if url.is_empty() {
        return Ok(());
    }
    let id = images::attachment_by_url_linked(page, url).await?;
    if id != 0 {
        return add_image_by_id(page, images, id).await;
    }
    add_image(page, images, ImageData { url: url.to_owned(), ..ImageData::default() });
    Ok(())
}

/// `Open_Graph_Image_Generator::add_from_templates()`.
async fn add_from_templates(page: &Page<'_>, model: &Model, images: &mut Vec<ImageData>) -> Result<()> {
    if !images.is_empty() {
        return Ok(());
    }
    if let Some(id) = model.og_image_id {
        return add_image_by_id(page, images, id).await;
    }
    if set(&model.og_image) {
        add_image_by_url(page, images, model.og_image.as_deref().unwrap_or_default()).await?;
    }
    Ok(())
}

/// `Open_Graph_Image_Generator::generate()`.
async fn open_graph_images(page: &Page<'_>, model: &Model) -> Result<Vec<ImageData>> {
    let mut images = Vec::new();
    if let Some(meta) = &model.og_image_meta {
        add_image(page, &mut images, meta.clone());
    } else if let Some(id) = model.og_image_id {
        add_image_by_id(page, &mut images, id).await?;
    } else if set(&model.og_image) {
        add_image(page, &mut images, ImageData { url: model.og_image.clone().unwrap_or_default(), ..ImageData::default() });
    }
    add_from_templates(page, model, &mut images).await?;
    if images.is_empty() {
        let default_id = page.options.string("og_default_image_id").trim().parse::<u64>().unwrap_or(0);
        if default_id != 0 {
            add_image_by_id(page, &mut images, default_id).await?;
        } else {
            let url = page.options.string("og_default_image");
            add_image_by_url(page, &mut images, &url).await?;
        }
    }
    Ok(images)
}

/// The first image of `Twitter_Image_Generator::generate()`.
async fn twitter_images(page: &Page<'_>, model: &Model) -> Result<Option<String>> {
    if let Some(id) = model.twitter_image_id {
        return Ok(non_empty(images::twitter_image(page, id).await?));
    }
    let Some(url) = model.twitter_image.as_deref().filter(|u| !u.is_empty()) else { return Ok(None) };
    let id = images::attachment_by_url_linked(page, url).await?;
    if id != 0 {
        return Ok(non_empty(images::twitter_image(page, id).await?));
    }
    let url = if !(url.starts_with("http") || url.starts_with("//")) && url.starts_with('/') { build_absolute_url(page, url) } else { url.to_owned() };
    Ok(Some(url))
}
