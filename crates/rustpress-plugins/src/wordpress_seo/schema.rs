//! The schema graph (`Schema_Generator` with its pieces), the context it reads (`Meta_Tags_Context`) and the
//! breadcrumbs (`Breadcrumbs_Generator`).

use rustpress_core::Result;
use rustpress_core::formatting::{esc_html, wp_strip_all_tags};
use serde_json::{Map, Value, json};

use super::images::{self, ImageData};
use super::presentation::{Kind, Model, Object, Presentation};
use super::replace_vars::{self, author_meta, php_trim};
use super::Page;

/// `Meta_Tags_Context::generate_site_represents()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Represents {
    None,
    Company,
    Person(u64),
}

/// `generate_company_logo_id()`: the logo of the settings, else the site logo.
async fn company_logo_id(page: &Page<'_>) -> Result<u64> {
    let id = page.options.string("company_logo_id").trim().parse::<u64>().unwrap_or(0);
    if id != 0 {
        return Ok(id);
    }
    let url = page.options.string("company_logo");
    if !url.is_empty() {
        let id = images::attachment_by_url(page, &url).await?;
        if id != 0 {
            return Ok(id);
        }
    }
    fallback_to_site_logo(page).await
}

/// `fallback_to_site_logo()`: the `site_logo` option, else the theme's `custom_logo`.
async fn fallback_to_site_logo(page: &Page<'_>) -> Result<u64> {
    let site_logo = page.site.options.fetch(&page.site.db, "site_logo").await?.unwrap_or_default();
    let id = super::php_int(&site_logo);
    if id > 0 {
        return Ok(id as u64);
    }
    Ok(rustpress_blocks::images::custom_logo_id(page.site).await.unwrap_or(0))
}

/// `generate_company_name()`.
fn company_name(page: &Page<'_>) -> String {
    let name = page.options.string("company_name");
    if name.is_empty() { site_name(page) } else { name }
}

/// `generate_site_name()`.
fn site_name(page: &Page<'_>) -> String {
    let name = page.options.string("website_name");
    if name.is_empty() { page.site.settings.blogname.clone() } else { name }
}

/// `generate_site_represents()`.
pub async fn site_represents(page: &Page<'_>) -> Result<Represents> {
    match page.options.string("company_or_person").as_str() {
        "company" => {
            if company_name(page).is_empty() || company_logo_id(page).await? < 1 {
                return Ok(Represents::None);
            }
            Ok(Represents::Company)
        }
        "person" => {
            let id = super::php_int(&page.options.string("company_or_person_user_id")).max(0) as u64;
            if page.site.db.user(id).await?.is_none() {
                return Ok(Represents::None);
            }
            Ok(Represents::Person(id))
        }
        _ => Ok(Represents::None),
    }
}

/// `HTML_Helper::smart_strip_tags()`.
pub fn smart_strip_tags(html: &str) -> String {
    use std::sync::LazyLock;
    static BR: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)<br(\s*)?/?>").expect("valid pattern"));
    static CLOSE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)</(p|div|h\d)>").expect("valid pattern"));
    static LI: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)(<li[^>]*>)").expect("valid pattern"));
    static SPACES: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"[\t\n\x0b\x0c\r ]+").expect("valid pattern"));
    if html.is_empty() {
        return String::new();
    }
    let html = html.replace(['\r', '\n'], " ");
    let html = BR.replace_all(&html, " ");
    let html = CLOSE.replace_all(&html, "</$1> ");
    let html = LI.replace_all(&html, "$1• ");
    let html = wp_strip_all_tags(&html);
    php_trim(&SPACES.replace_all(&html, " ")).to_owned()
}

/// The context of the schema pieces.
pub struct Context<'p> {
    pub p: &'p Presentation,
    pub title: String,
    pub description: String,
    pub canonical: String,
    /// `main_schema_id`: the permalink of the context.
    pub permalink: String,
    pub site_url: String,
    pub represents: Represents,
    pub page_type: Vec<String>,
    pub article_type: Vec<String>,
    pub main_image_id: Option<u64>,
    pub main_image_url: Option<String>,
}

impl Context<'_> {
    fn reference(&self, page: &Page<'_>) -> Option<Value> {
        match &self.represents {
            Represents::Person(id) => Some(json!({"@id": person_id(page, &self.site_url, *id)})),
            Represents::Company => Some(json!({"@id": format!("{}#organization", self.site_url)})),
            Represents::None => None,
        }
    }

    fn model(&self) -> &Model {
        &self.p.model
    }

    fn article_needed(&self, page: &Page<'_>) -> bool {
        self.model().object_type == "post" && page.site.post_type_supports(&self.model().object_sub_type, "author") && self.article_type != ["None"]
    }
}

/// `get_search_query()`: the search words, escaped for an attribute.
fn search_query(page: &Page<'_>) -> String {
    rustpress_core::formatting::esc_attr(page.query.vars.get("s").unwrap_or_default())
}

/// `ID_Helper::get_user_schema_id()`.
fn person_id(page: &Page<'_>, site_url: &str, user_id: u64) -> String {
    let login = page.users.get(&user_id).map(|u| u.user_login.clone()).unwrap_or_default();
    if !page.users.contains_key(&user_id) {
        return String::new();
    }
    format!("{site_url}#/schema/person/{}", rustpress_blocks::commenter::wp_hash(&format!("{login}{user_id}"), "auth"))
}

/// A value for `@type`: one type as a string.
fn type_value(types: &[String]) -> Value {
    if types.len() == 1 { Value::String(types[0].clone()) } else { json!(types) }
}

/// `Schema_Types::ARTICLE_TYPES`.
const ARTICLE_TYPES: &[&str] = &["Article", "BlogPosting", "SocialMediaPosting", "NewsArticle", "AdvertiserContentArticle", "SatiricalArticle", "ScholarlyArticle", "TechArticle", "Report", "None"];

/// Builds the context (`Meta_Tags_Context`).
pub async fn context<'p>(page: &Page<'_>, p: &'p Presentation) -> Result<Context<'p>> {
    let model = &p.model;
    let o = page.options;
    let title = replace_vars::replace(page, &p.title, p.source()).await?;
    let description = replace_vars::replace(page, &p.meta_description, p.source()).await?;
    let permalink = if page.query.flags.is_search {
        rustpress_core::urls::add_query_arg(&rustpress_core::urls::trailingslashit(&page.home()), "s", &rustpress_core::canonical::rawurlencode(&search_query(page)))
    } else {
        p.permalink.clone()
    };
    let site_url = rustpress_core::urls::trailingslashit(&page.home());
    let page_type: Vec<String> = match model.object_type {
        "system-page" if model.object_sub_type == "search-result" => vec!["CollectionPage".into(), "SearchResultsPage".into()],
        "system-page" => vec!["WebPage".into()],
        "user" => vec!["ProfilePage".into()],
        "home-page" | "date-archive" | "term" | "post-type-archive" => vec!["CollectionPage".into()],
        _ => {
            let additional = model.schema_page_type.clone().unwrap_or_else(|| o.string(&format!("schema-page-type-{}", model.object_sub_type)));
            let mut types = vec!["WebPage".to_owned(), additional];
            if page.site.settings.page_for_posts == model.object_id && model.object_type == "post" {
                types.push("CollectionPage".into());
            }
            let mut unique: Vec<String> = Vec::new();
            for t in types {
                if !t.is_empty() && !unique.contains(&t) {
                    unique.push(t);
                }
            }
            unique
        }
    };
    let article_type = {
        let mut additional = model.schema_article_type.clone().unwrap_or_else(|| o.string(&format!("schema-article-type-{}", model.object_sub_type)));
        if !ARTICLE_TYPES.contains(&additional.as_str()) {
            additional = o.title_default(&format!("schema-article-type-{}", model.object_sub_type));
        }
        if additional.to_lowercase().contains("article") {
            vec![additional]
        } else if additional == "None" {
            vec!["None".to_owned()]
        } else if additional != "Article" {
            vec!["Article".to_owned(), additional]
        } else {
            vec!["Article".to_owned()]
        }
    };
    // generate_main_image_id()
    let q = page.query;
    let main_image_id = if q.flags.is_singular() {
        match p.post() {
            Some(post) => singular_post_image(page, post.id, post.post_type == "attachment").await?,
            None => None,
        }
    } else if q.flags.is_author || q.flags.is_tax || q.flags.is_tag || q.flags.is_category || q.flags.is_search || q.flags.is_date || q.flags.is_post_type_archive {
        match q.posts.first() {
            Some(first) => singular_post_image(page, first.id, false).await?,
            None => None,
        }
    } else {
        None
    };
    // generate_main_image_url()
    let main_image_url = match main_image_id {
        Some(id) => Some(images::attachment_image_url(page, id).await?),
        None if q.flags.is_singular() => p.post().and_then(|post| images::first_content_image(&post.post_content)),
        None => None,
    };
    Ok(Context { p, title, description, canonical: p.canonical.clone(), permalink, site_url, represents: site_represents(page).await?, page_type, article_type, main_image_id, main_image_url })
}

/// `get_singular_post_image()`.
async fn singular_post_image(page: &Page<'_>, post_id: u64, attachment: bool) -> Result<Option<u64>> {
    let thumbnail = super::presentation::thumbnail_id(page, post_id).await?;
    if thumbnail > 0 {
        return Ok(Some(thumbnail));
    }
    if attachment {
        return Ok(Some(post_id));
    }
    Ok(None)
}

/// `Language_Helper::add_piece_language()`.
fn add_language(page: &Page<'_>, data: &mut Map<String, Value>) {
    data.insert("inLanguage".into(), Value::String(page.site.language_tag()));
}

/// `Image_Helper::generate_from_attachment_id()`.
async fn image_from_attachment(page: &Page<'_>, schema_id: &str, id: u64, caption: &str) -> Result<Value> {
    let mut data = Map::new();
    data.insert("@type".into(), "ImageObject".into());
    add_language(page, &mut data);
    let attachment = images::attachment(page, id).await?;
    let url = attachment.as_ref().and_then(|a| a.image_src(page, "full")).map(|s| s.0).unwrap_or_default();
    data.insert("@id".into(), schema_id.into());
    data.insert("url".into(), url.clone().into());
    data.insert("contentUrl".into(), url.into());
    if let Some(meta) = attachment.as_ref().and_then(|a| a.meta.as_ref())
        && meta.width() != 0
        && meta.height() != 0
    {
        data.insert("width".into(), meta.0.get("width").cloned().unwrap_or_default());
        data.insert("height".into(), meta.0.get("height").cloned().unwrap_or_default());
    }
    if !caption.is_empty() {
        data.insert("caption".into(), caption.into());
    } else if let Some(a) = &attachment {
        let caption = a.caption(page).await?;
        if !caption.is_empty() {
            data.insert("caption".into(), smart_strip_tags(&caption).into());
        }
    }
    Ok(Value::Object(data))
}

/// `Image_Helper::simple_image_object()`.
fn simple_image(page: &Page<'_>, schema_id: &str, url: &str, caption: &str) -> Value {
    let mut data = Map::new();
    data.insert("@type".into(), "ImageObject".into());
    add_language(page, &mut data);
    data.insert("@id".into(), schema_id.into());
    data.insert("url".into(), url.into());
    data.insert("contentUrl".into(), url.into());
    if !caption.is_empty() {
        data.insert("caption".into(), smart_strip_tags(caption).into());
    }
    Value::Object(data)
}

/// `Image_Helper::generate_from_attachment_meta()`.
fn image_from_meta(page: &Page<'_>, schema_id: &str, meta: &ImageData, caption: &str) -> Value {
    let mut data = Map::new();
    data.insert("@type".into(), "ImageObject".into());
    add_language(page, &mut data);
    data.insert("@id".into(), schema_id.into());
    data.insert("url".into(), meta.url.clone().into());
    data.insert("contentUrl".into(), meta.url.clone().into());
    data.insert("width".into(), meta.width.into());
    data.insert("height".into(), meta.height.into());
    if !caption.is_empty() {
        data.insert("caption".into(), smart_strip_tags(caption).into());
    }
    Value::Object(data)
}

/// A crumb of `Breadcrumbs_Generator::generate()`.
#[derive(Debug, Clone, Default)]
pub struct Crumb {
    pub url: Option<String>,
    pub text: String,
    /// The crumb of the page number has no `url` key at all.
    pub without_url: bool,
}


/// `Breadcrumbs_Generator::generate()`.
pub async fn breadcrumbs(page: &Page<'_>, p: &Presentation) -> Result<Vec<Crumb>> {
    let o = page.options;
    let model = &p.model;
    let home_text = o.string("breadcrumbs-home");
    let home_kinds = matches!(p.kind, Kind::HomePage | Kind::StaticHomePage);
    let settings = &page.site.settings;
    let static_front = settings.show_on_front == rustpress_core::settings::ShowOnFront::Page;
    let mut statics: Vec<Crumb> = Vec::new();
    if !home_text.is_empty() && !home_kinds {
        let front = if static_front { settings.page_on_front } else { 0 };
        if front == 0 {
            statics.push(Crumb { url: Some(page.home()), text: o.string("breadcrumbs-home"), without_url: false });
        } else if let Some(front) = page.site.db.post(front).await? {
            statics.push(post_crumb(page, &front).await?);
        }
    }
    let page_for_posts = settings.page_for_posts;
    if static_front && page_for_posts != 0 {
        let blog_crumb = if model.object_type == "term" {
            taxonomy_parent(page, &model.object_sub_type).as_deref() == Some("post")
        } else {
            o.is_true("breadcrumbs-display-blog-page") && !page.query.flags.is_home && !page.query.flags.is_search && page.query.flags.is_single && p.post().is_some_and(|post| post.post_type == "post")
        };
        if blog_crumb && let Some(blog) = page.site.db.post(page_for_posts).await? {
            statics.push(post_crumb(page, &blog).await?);
        }
    }
    if model.object_type == "post" && !matches!(model.object_sub_type.as_str(), "post" | "page") && page.site.post_type_archive_link(&model.object_sub_type).is_some() {
        statics.push(post_type_archive_crumb(page, &model.object_sub_type));
    }
    if model.object_type == "term"
        && let Some(parent) = taxonomy_parent(page, &model.object_sub_type)
        && parent != "post"
        && page.site.post_type_archive_link(&parent).is_some()
    {
        statics.push(post_type_archive_crumb(page, &parent));
    }
    let mut crumbs = statics;
    if !home_kinds {
        crumbs.extend(ancestors(page, p).await?);
    }
    crumbs.push(own_crumb(page, p).await?);
    if !home_text.is_empty()
        && let Some(first) = crumbs.first_mut()
    {
        first.text = home_text;
    }
    // add_paged_crumb()
    let simple = page.simple_page_id() > 0;
    if (simple || page.query.flags.is_paged) && !(simple && model.number_of_pages.is_none()) {
        let number = page.page_number();
        if number > 1 {
            crumbs.push(Crumb { url: None, text: rustpress_core::i18n::sprintf(&page.site.trd(super::DOMAIN, "Page %s"), &[&number.to_string()]), without_url: true });
        }
    }
    Ok(crumbs)
}

/// `get_taxonomy_post_type_parent()`.
fn taxonomy_parent(page: &Page<'_>, taxonomy: &str) -> Option<String> {
    let parent = page.options.string(&format!("taxonomy-{taxonomy}-ptparent"));
    (!parent.is_empty() && parent != "0").then_some(parent)
}

async fn post_crumb(page: &Page<'_>, post: &rustpress_db::Post) -> Result<Crumb> {
    let model = super::presentation::post_model(page, post).await?;
    Ok(Crumb { url: model.permalink, text: model.breadcrumb_title.unwrap_or_default(), without_url: false })
}

fn post_type_archive_crumb(page: &Page<'_>, post_type: &str) -> Crumb {
    let bctitle = page.options.string(&format!("bctitle-ptarchive-{post_type}"));
    Crumb { url: page.site.post_type_archive_link(post_type), text: if bctitle.is_empty() { page.site.post_type_label(post_type) } else { bctitle }, without_url: false }
}

async fn term_crumb(page: &Page<'_>, term: &rustpress_db::Term) -> Result<Crumb> {
    let meta = page.options.term_meta(&term.taxonomy, term.term_id);
    let bctitle = meta.iter().find(|(k, _)| k == "wpseo_bctitle").map(|(_, v)| v.clone()).unwrap_or_default();
    Ok(Crumb { url: Some(page.site.term_link(term).await?), text: if bctitle.is_empty() { page.site.term_name(term) } else { bctitle }, without_url: false })
}

/// `Indexable_Hierarchy_Builder`: the parents of a post (or its primary term and that term's parents) and of a term,
/// the oldest first.
async fn ancestors(page: &Page<'_>, p: &Presentation) -> Result<Vec<Crumb>> {
    let mut out = Vec::new();
    match &p.object {
        Object::Post(post) if p.model.object_type == "post" => {
            let mut parent = post.post_parent;
            let mut seen = vec![post.id];
            let mut chain = Vec::new();
            while parent != 0 && !seen.contains(&parent) {
                let Some(parent_post) = page.site.db.post(parent).await? else { break };
                seen.push(parent);
                parent = parent_post.post_parent;
                chain.push(post_crumb(page, &parent_post).await?);
            }
            if post.post_parent == 0 {
                let main = page.options.string(&format!("post_types-{}-maintax", post.post_type));
                if !main.is_empty() && main != "0"
                    && let Some(term) = primary_term(page, post, &main).await?
                {
                    let mut terms = vec![term_crumb(page, &term).await?];
                    terms.extend(term_parents(page, &term).await?);
                    chain.extend(terms);
                }
            }
            chain.reverse();
            out = chain;
        }
        Object::Term(term) => {
            let mut parents = term_parents(page, term).await?;
            parents.reverse();
            out = parents;
        }
        _ => {}
    }
    Ok(out)
}

async fn term_parents(page: &Page<'_>, term: &rustpress_db::Term) -> Result<Vec<Crumb>> {
    let mut out = Vec::new();
    let mut parent = term.parent;
    while parent != 0 {
        let Some(t) = page.site.db.term_by_id(Some(&term.taxonomy), parent).await? else { break };
        parent = t.parent;
        out.push(term_crumb(page, &t).await?);
        if out.len() > 50 {
            break;
        }
    }
    Ok(out)
}

/// `find_primary_term_id_for_post()`: the primary term, else the deepest of the post's terms.
async fn primary_term(page: &Page<'_>, post: &rustpress_db::Post, taxonomy: &str) -> Result<Option<rustpress_db::Term>> {
    let primary = super::presentation::post_meta(page, post.id, &format!("primary_{taxonomy}")).await?.trim().parse::<u64>().unwrap_or(0);
    if primary != 0
        && let Some(term) = page.site.db.term_by_id(None, primary).await?
    {
        return Ok(Some(term));
    }
    let terms = replace_vars::post_terms(page, post.id, taxonomy).await?;
    if terms.is_empty() {
        return Ok(None);
    }
    // `find_deepest_term_id()`: leave out terms that are the parent of another one, then take the one with the most
    // parents.
    let mut candidates: Vec<&rustpress_db::Term> = terms.iter().filter(|t| !terms.iter().any(|o| o.parent == t.term_id)).collect();
    if candidates.is_empty() {
        candidates = terms.iter().collect();
    }
    let mut deepest = candidates[0];
    let mut count: i64 = -1;
    for term in candidates {
        let n = term_parents(page, term).await?.len() as i64;
        if n < count {
            continue;
        }
        if n > count {
            deepest = term;
        }
        count = n;
    }
    Ok(Some(deepest.clone()))
}

/// The crumb of the page itself (`get_post_type_crumb()` of its indexable).
async fn own_crumb(page: &Page<'_>, p: &Presentation) -> Result<Crumb> {
    let model = &p.model;
    let o = page.options;
    let mut crumb = Crumb { url: model.permalink.clone(), text: model.breadcrumb_title.clone().unwrap_or_default(), without_url: false };
    match model.object_type {
        "system-page" if model.object_sub_type == "search-result" => {
            let words = page.query.vars.get("s").unwrap_or_default();
            crumb.text = format!("{} {}", o.string("breadcrumbs-searchprefix"), esc_html(&search_query(page)));
            crumb.url = Some(super::presentation::search_link(page, words));
        }
        "system-page" => crumb.text = o.string("breadcrumbs-404crumb"),
        "user" => {
            let name = author_meta(page, model.object_id, "display_name").await?;
            crumb.text = format!("{} {name}", o.string("breadcrumbs-archiveprefix"));
        }
        "date-archive" => {
            let home = page.home();
            let prefix = o.string("breadcrumbs-archiveprefix");
            let q = page.query;
            let date = page.global_post.map(|p| p.post_date.clone()).unwrap_or_default();
            // `get_the_date()`, `false` while a plugin hides the dates.
            let formatted = |format: &str| if page.dates_hidden { String::new() } else { replace_vars::format_translated(page, &date, format) };
            if q.flags.is_day {
                crumb.url = Some(format!("{home}{}/", formatted("Y/m/d")));
                crumb.text = format!("{prefix} {}", esc_html(&formatted(&page.site.options.string("date_format", "F j, Y"))));
            } else if q.flags.is_month {
                let month = esc_html(php_trim(&replace_vars::single_month_title(page, " ").unwrap_or_default()));
                crumb.url = Some(format!("{home}{}/", formatted("Y/m")));
                crumb.text = format!("{prefix} {month}");
            } else if q.flags.is_year {
                let year = formatted("Y");
                crumb.url = Some(format!("{home}{year}/"));
                crumb.text = format!("{prefix} {year}");
            }
        }
        _ => {}
    }
    Ok(crumb)
}

/// `Schema_Generator::generate()`: the `@graph`.
pub async fn graph(page: &Page<'_>, ctx: &mut Context<'_>) -> Result<Value> {
    let p = ctx.p;
    let model = &p.model;
    let protected = model.object_type == "post" && p.post().is_some_and(super::Page::password_required);
    let mut graph: Vec<Value> = Vec::new();
    let article = !protected && ctx.article_needed(page);
    let toc = p.post().is_some_and(|post| post.post_content.contains("<!-- wp:yoast-seo/table-of-contents"));

    if article {
        let mut piece = article_piece(page, ctx).await?;
        if toc {
            add_toc(&mut piece);
        }
        graph.push(piece);
    }
    // WebPage
    if model.object_type != "unknown" && !(model.object_type == "system-page" && model.object_sub_type == "404") {
        let mut piece = webpage_piece(page, ctx).await?;
        if protected {
            let keep = ["@type", "@id", "url", "name", "isPartOf", "inLanguage", "datePublished", "dateModified", "breadcrumb"];
            if let Value::Object(map) = &mut piece {
                map.retain(|k, _| keep.contains(&k.as_str()));
                map.insert("@type".into(), "WebPage".into());
            }
        } else if toc && !article {
            add_toc(&mut piece);
        }
        graph.push(validate_type(piece));
    }
    if !protected {
        // Main_Image
        let image_id = format!("{}#primaryimage", ctx.canonical);
        if let Some(id) = ctx.main_image_id.filter(|id| *id != 0) {
            let piece = image_from_attachment(page, &image_id, id, "").await?;
            ctx.main_image_url = piece.get("url").and_then(Value::as_str).map(str::to_owned);
            graph.push(piece);
        } else if let Some(url) = ctx.main_image_url.clone().filter(|u| !u.is_empty()) {
            let id = images::attachment_by_url_linked(page, &url).await?;
            graph.push(if id > 0 { image_from_attachment(page, &image_id, id, "").await? } else { simple_image(page, &image_id, &url, "") });
        }
        // Breadcrumb
        if model.object_type != "unknown" && !(model.object_type == "system-page" && model.object_sub_type == "404")
            && let Some(piece) = breadcrumb_piece(page, ctx).await?
        {
            graph.push(piece);
        }
    }
    graph.push(website_piece(page, ctx));
    if ctx.represents == Represents::Company {
        graph.push(organization_piece(page, ctx).await?);
    }
    if !protected {
        let site_user = match ctx.represents {
            Represents::Person(id) => Some(id),
            _ => None,
        };
        let represents_author = site_user.is_some_and(|id| {
            if article {
                return id == model.author_id;
            }
            model.object_type == "user" && id == model.object_id
        });
        if let Some(id) = site_user.filter(|_| !represents_author)
            && id > 0
        {
            graph.push(person_piece(page, ctx, id, true).await?);
        }
        let author = if model.object_type == "user" {
            Some(model.object_id)
        } else if article {
            Some(model.author_id)
        } else {
            None
        };
        if let Some(id) = author.filter(|id| *id > 0) {
            let mut piece = person_piece(page, ctx, id, represents_author).await?;
            if let Value::Object(map) = &mut piece {
                if !represents_author {
                    map.insert("@type".into(), json!(["Person"]));
                    map.shift_remove("logo");
                }
                if model.object_type == "user" {
                    map.insert("mainEntityOfPage".into(), json!({"@id": ctx.permalink}));
                }
                if model.object_type == "post" && !page.options.is_true("disable-author")
                    && let Some(user) = page.users.get(&id)
                {
                    map.insert("url".into(), page.site.author_url(user).into());
                }
            }
            graph.push(validate_type(piece));
        }
    }
    let graph = remove_empty_breadcrumb(page, ctx, graph);
    Ok(json!({"@context": "https://schema.org", "@graph": graph}))
}

fn add_toc(piece: &mut Value) {
    if let Value::Object(map) = piece {
        match map.get_mut("accessibilityFeature") {
            Some(Value::Array(list)) => list.push("tableOfContents".into()),
            _ => {
                map.insert("accessibilityFeature".into(), json!(["tableOfContents"]));
            }
        }
    }
}

/// `validate_type()`: unique types, one type as a string.
fn validate_type(mut piece: Value) -> Value {
    if let Some(Value::Array(types)) = piece.get("@type").cloned() {
        let mut unique: Vec<Value> = Vec::new();
        for t in types {
            if !unique.contains(&t) {
                unique.push(t);
            }
        }
        piece["@type"] = if unique.len() == 1 { unique.remove(0) } else { Value::Array(unique) };
    }
    piece
}

/// `remove_empty_breadcrumb()`.
fn remove_empty_breadcrumb(page: &Page<'_>, ctx: &Context<'_>, mut graph: Vec<Value>) -> Vec<Value> {
    if matches!(ctx.p.kind, Kind::StaticHomePage | Kind::HomePage) {
        return graph;
    }
    let index = graph.iter().position(|piece| piece.get("@type").and_then(Value::as_str) == Some("BreadcrumbList") && piece.get("itemListElement").and_then(Value::as_array).is_some_and(|l| l.len() == 1));
    let Some(index) = index.filter(|i| *i != 0) else { return graph };
    graph.remove(index);
    for piece in &mut graph {
        let types: Vec<String> = match piece.get("@type") {
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
            _ => Vec::new(),
        };
        if types.iter().any(|t| ctx.page_type.contains(t))
            && let Value::Object(map) = piece
        {
            map.shift_remove("breadcrumb");
        }
    }
    let _ = page;
    graph
}

/// `Article::generate()`.
async fn article_piece(page: &Page<'_>, ctx: &Context<'_>) -> Result<Value> {
    let post = ctx.p.post().expect("an article is a post");
    let author = page.users.get(&post.post_author);
    let mut data = Map::new();
    data.insert("@type".into(), type_value(&ctx.article_type));
    data.insert("@id".into(), format!("{}#article", ctx.canonical).into());
    data.insert("isPartOf".into(), json!({"@id": ctx.permalink}));
    data.insert("author".into(), json!({"name": author.map(|u| smart_strip_tags(&u.display_name)).unwrap_or_default(), "@id": person_id(page, &ctx.site_url, post.post_author)}));
    let mut headline = rustpress_blocks::content::the_title_for(page.site, post);
    if headline.is_empty() {
        headline = page.site.trd(super::DOMAIN, "No title");
    }
    data.insert("headline".into(), smart_strip_tags(&headline).into());
    data.insert("datePublished".into(), super::presentation::w3c(&post.post_date_gmt).into());
    if gmt(&post.post_modified_gmt) > gmt(&post.post_date_gmt) {
        data.insert("dateModified".into(), super::presentation::w3c(&post.post_modified_gmt).into());
    }
    data.insert("mainEntityOfPage".into(), json!({"@id": ctx.permalink}));
    data.insert("wordCount".into(), word_count(&post.post_content, &post.post_title).into());
    if post.comment_status == "open" {
        data.insert("commentCount".into(), post.comment_count.into());
    }
    if let Some(reference) = ctx.reference(page) {
        data.insert("publisher".into(), reference);
    }
    if let Some(url) = &ctx.main_image_url {
        data.insert("image".into(), json!({"@id": format!("{}#primaryimage", ctx.canonical)}));
        data.insert("thumbnailUrl".into(), url.clone().into());
    }
    let uncategorized = page.site.tr("Uncategorized");
    for (key, taxonomy) in [("keywords", "post_tag"), ("articleSection", "category")] {
        let names: Vec<String> = replace_vars::post_terms(page, post.id, taxonomy).await?.iter().map(|t| page.site.term_name(t)).filter(|n| *n != uncategorized).collect();
        if !names.is_empty() {
            data.insert(key.into(), json!(names));
        }
    }
    add_language(page, &mut data);
    if page.site.post_type_supports(&post.post_type, "comments") && post.comment_status == "open" {
        data.insert("potentialAction".into(), json!([{"@type": "CommentAction", "name": "Comment", "target": [format!("{}#respond", ctx.canonical)]}]));
    }
    Ok(validate_type(Value::Object(data)))
}

fn gmt(date: &str) -> i64 {
    rustpress_core::datetime::SiteTimezone::gmt_timestamp(date).unwrap_or(0)
}

/// `WebPage::generate()`.
async fn webpage_piece(page: &Page<'_>, ctx: &Context<'_>) -> Result<Value> {
    let model = ctx.model();
    let mut data = Map::new();
    data.insert("@type".into(), json!(ctx.page_type));
    data.insert("@id".into(), ctx.permalink.clone().into());
    data.insert("url".into(), ctx.canonical.clone().into());
    data.insert("name".into(), smart_strip_tags(&ctx.title).into());
    data.insert("isPartOf".into(), json!({"@id": format!("{}#website", ctx.site_url)}));
    if ctx.canonical.is_empty() && page.query.flags.is_search {
        data.insert("url".into(), format!("{}?s={}", ctx.site_url, rustpress_core::canonical::rawurlencode(&search_query(page))).into());
    }
    if page.query.flags.is_front_page
        && let Some(reference) = ctx.reference(page)
    {
        data.insert("about".into(), reference);
    }
    if let Some(url) = &ctx.main_image_url {
        let id = format!("{}#primaryimage", ctx.canonical);
        data.insert("primaryImageOfPage".into(), json!({"@id": id}));
        data.insert("image".into(), json!({"@id": id}));
        data.insert("thumbnailUrl".into(), url.clone().into());
    }
    if model.object_type == "post"
        && let Some(post) = ctx.p.post()
    {
        data.insert("datePublished".into(), super::presentation::w3c(&post.post_date_gmt).into());
        if gmt(&post.post_modified_gmt) > gmt(&post.post_date_gmt) {
            data.insert("dateModified".into(), super::presentation::w3c(&post.post_modified_gmt).into());
        }
        if model.object_sub_type == "post" && ctx.represents == Represents::None {
            data.insert("author".into(), json!({"@id": person_id(page, &ctx.site_url, post.post_author)}));
        }
    }
    if !ctx.description.is_empty() {
        data.insert("description".into(), smart_strip_tags(&ctx.description).into());
    }
    if !(model.object_type == "system-page" && model.object_sub_type == "404") {
        data.insert("breadcrumb".into(), json!({"@id": format!("{}#breadcrumb", ctx.canonical)}));
    }
    add_language(page, &mut data);
    if !ctx.page_type.iter().any(|t| t == "CollectionPage") {
        data.insert("potentialAction".into(), json!([{"@type": "ReadAction", "target": [ctx.canonical]}]));
    }
    Ok(Value::Object(data))
}

/// `Breadcrumb::generate()`.
async fn breadcrumb_piece(page: &Page<'_>, ctx: &Context<'_>) -> Result<Option<Value>> {
    let p = ctx.p;
    let mut crumbs = breadcrumbs(page, p).await?;
    let paged = page.query.flags.is_paged || p.model.number_of_pages.is_some_and(|n| n > 1);
    if paged && p.kind != Kind::StaticPostsPage && crumbs.len() > 1 {
        crumbs.pop();
    }
    // A crumb without a URL key (the page number) is broken: no breadcrumb at all.
    if crumbs.iter().any(|c| c.without_url) {
        return Ok(None);
    }
    if let Some(last) = crumbs.last_mut() {
        last.url = None;
    }
    if p.kind == Kind::StaticHomePage && crumbs.len() > 1 {
        let home = crumbs[0].text.clone();
        let mut last = crumbs.pop().unwrap_or_default();
        last.text = home;
        crumbs = vec![last];
    }
    let crumbs: Vec<Crumb> = crumbs.into_iter().filter(|c| !c.text.is_empty() && c.text != "0").collect();
    let items: Vec<Value> = crumbs
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut item = Map::new();
            item.insert("@type".into(), "ListItem".into());
            item.insert("position".into(), (i + 1).into());
            item.insert("name".into(), smart_strip_tags(&c.text).into());
            if let Some(url) = c.url.as_ref().filter(|u| !u.is_empty()) {
                item.insert("item".into(), url.clone().into());
            }
            Value::Object(item)
        })
        .collect();
    Ok(Some(json!({"@type": "BreadcrumbList", "@id": format!("{}#breadcrumb", ctx.canonical), "itemListElement": items})))
}

/// `Website::generate()`.
fn website_piece(page: &Page<'_>, ctx: &Context<'_>) -> Value {
    let mut data = Map::new();
    data.insert("@type".into(), "WebSite".into());
    data.insert("@id".into(), format!("{}#website", ctx.site_url).into());
    data.insert("url".into(), ctx.site_url.clone().into());
    data.insert("name".into(), smart_strip_tags(&site_name(page)).into());
    data.insert("description".into(), page.site.settings.blogdescription.clone().into());
    if let Some(reference) = ctx.reference(page) {
        data.insert("publisher".into(), reference);
    }
    let alternate = page.options.string("alternate_website_name");
    if !alternate.is_empty() {
        data.insert("alternateName".into(), smart_strip_tags(&alternate).into());
    }
    data.insert(
        "potentialAction".into(),
        json!([{"@type": "SearchAction", "target": {"@type": "EntryPoint", "urlTemplate": format!("{}?s={{search_term_string}}", ctx.site_url)}, "query-input": {"@type": "PropertyValueSpecification", "valueRequired": true, "valueName": "search_term_string"}}]),
    );
    add_language(page, &mut data);
    Value::Object(data)
}

/// `Organization::generate()`.
async fn organization_piece(page: &Page<'_>, ctx: &Context<'_>) -> Result<Value> {
    let o = page.options;
    let logo_id = format!("{}#/schema/logo/image/", ctx.site_url);
    let name = company_name(page);
    let meta = company_logo_meta(page);
    let logo = match meta {
        Some(meta) => image_from_meta(page, &logo_id, &meta, &name),
        None => image_from_attachment(page, &logo_id, company_logo_id(page).await?, &name).await?,
    };
    let mut data = Map::new();
    data.insert("@type".into(), "Organization".into());
    data.insert("@id".into(), format!("{}#organization", ctx.site_url).into());
    data.insert("name".into(), smart_strip_tags(&name).into());
    let alternate = o.string("company_alternate_name");
    if !alternate.is_empty() && alternate != "0" {
        data.insert("alternateName".into(), alternate.into());
    }
    data.insert("url".into(), ctx.site_url.clone().into());
    let logo_ref = logo.get("@id").cloned().unwrap_or_default();
    data.insert("logo".into(), logo);
    data.insert("image".into(), json!({"@id": logo_ref}));
    // `Social_Profiles_Helper::get_organization_social_profiles()`.
    let mut same_as: Vec<String> = Vec::new();
    let mut push = |url: String| {
        if !url.is_empty() && url != "0" && !same_as.contains(&url) {
            same_as.push(url);
        }
    };
    push(super::urldecode(&o.string("facebook_site")));
    let twitter = o.string("twitter_site");
    if !twitter.is_empty() {
        push(format!("https://x.com/{twitter}"));
    }
    if let Some(rustpress_phpser::Value::Array(urls)) = o.get("other_social_urls") {
        for (_, url) in urls {
            let url = super::options::php_string(url);
            if super::options::truthy(&rustpress_phpser::Value::Str(url.clone().into_bytes())) {
                push(super::urldecode(&url));
            }
        }
    }
    if !same_as.is_empty() {
        data.insert("sameAs".into(), json!(same_as));
    }
    if ctx.page_type.iter().any(|t| t == "ProfilePage") && ctx.page_type.len() > 1 {
        data.insert("mainEntityOfPage".into(), json!({"@id": ctx.permalink}));
    }
    Ok(Value::Object(data))
}

/// The `company_logo_meta` option (an array of `WPSEO_Image_Utils::get_data()`).
fn company_logo_meta(page: &Page<'_>) -> Option<ImageData> {
    let value = page.options.get("company_logo_meta")?;
    let url = value.get("url").map(super::options::php_string).filter(|u| !u.is_empty())?;
    let num = |key: &str| value.get(key).map(super::options::php_string).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    Some(ImageData { url, width: num("width"), height: num("height"), ..ImageData::default() })
}

/// `Person::build_person_data()`.
async fn person_piece(page: &Page<'_>, ctx: &Context<'_>, user_id: u64, represents: bool) -> Result<Value> {
    let mut data = Map::new();
    data.insert("@type".into(), json!(["Person", "Organization"]));
    data.insert("@id".into(), person_id(page, &ctx.site_url, user_id).into());
    let Some(user) = page.users.get(&user_id) else { return Ok(Value::Object(data)) };
    let name = smart_strip_tags(&user.display_name);
    data.insert("name".into(), name.clone().into());
    let pronouns = smart_strip_tags(&author_meta(page, user_id, "wpseo_pronouns").await?);
    if !pronouns.is_empty() {
        data.insert("pronouns".into(), pronouns.into());
    }
    // add_image()
    let logo_id = format!("{}#/schema/person/image/", ctx.site_url);
    let mut image_id = None;
    if represents
        && let Some(meta) = person_logo_meta(page).await?
    {
        let image = image_from_meta(page, &meta.url, &meta, &name);
        image_id = image.get("@id").cloned();
        data.insert("image".into(), image);
    }
    if !data.contains_key("image") && page.site.show_avatars() {
        let url = rustpress_blocks::content::avatar_url(&page.site.options, &user.user_email, 96);
        let image = simple_image(page, &url, &url, &user.display_name);
        image_id = image.get("@id").cloned();
        data.insert("image".into(), image);
    }
    data.insert("logo".into(), json!({"@id": image_id.unwrap_or_else(|| logo_id.into())}));
    let description = author_meta(page, user_id, "description").await?;
    if !description.is_empty() {
        data.insert("description".into(), smart_strip_tags(&description).into());
    }
    let mut same_as: Vec<String> = Vec::new();
    if !user.user_url.is_empty() {
        same_as.push(user.user_url.clone());
    }
    for profile in ["facebook", "instagram", "linkedin", "pinterest", "twitter", "myspace", "youtube", "soundcloud", "tumblr", "wikipedia"] {
        let mut url = author_meta(page, user_id, profile).await?;
        if !url.is_empty() && profile == "twitter" {
            url = format!("https://x.com/{url}");
        }
        if !url.is_empty() && url != "0" {
            same_as.push(url);
        }
    }
    let mut unique: Vec<String> = Vec::new();
    for url in same_as {
        if !unique.contains(&url) {
            unique.push(url);
        }
    }
    if !unique.is_empty() {
        data.insert("sameAs".into(), json!(unique));
    }
    Ok(Value::Object(data))
}

/// `generate_person_logo_meta()`: the stored meta of the person's logo, else the best variation of the logo or of the
/// site logo.
async fn person_logo_meta(page: &Page<'_>) -> Result<Option<ImageData>> {
    let value = page.options.get("person_logo_meta");
    if let Some(value) = value
        && let Some(url) = value.get("url").map(super::options::php_string).filter(|u| !u.is_empty())
    {
        let num = |key: &str| value.get(key).map(super::options::php_string).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        return Ok(Some(ImageData { url, width: num("width"), height: num("height"), ..ImageData::default() }));
    }
    let mut id = page.options.string("person_logo_id").trim().parse::<u64>().unwrap_or(0);
    if id == 0 {
        id = fallback_to_site_logo(page).await?;
    }
    let Some(a) = images::attachment(page, id).await? else { return Ok(None) };
    Ok(images::best_variation(page, &a).await)
}

/// `Article::word_count()`: the title and content without `<pre>`/`<code>`, tags and entities, counted by
/// `str_word_count()` (with the Cyrillic letters when the text has some).
fn word_count(content: &str, title: &str) -> usize {
    use std::sync::LazyLock;
    static CODE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| fancy_regex::Regex::new(r"(?si)<(pre|code)[^>]*?>.*?</\1>").expect("valid pattern"));
    static ENTITY: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)&[a-z0-9]+;").expect("valid pattern"));
    static CYRILLIC: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)[а-я]").expect("valid pattern"));
    let text = format!("{title} {content}");
    let text = CODE.replace_all(&text, "").replace("><", "> <");
    let text = wp_strip_all_tags(&text);
    let characters = if CYRILLIC.is_match(&text) {
        let mut chars: Vec<char> = Vec::new();
        for c in "абвгдеёжзийклмнопрстуфхцчшщъыьэюяабвгґдеєжзиіїйклмнопрстуфхцчшщьюя".chars() {
            if !chars.contains(&c) {
                chars.push(c);
            }
        }
        let lower: String = chars.iter().collect();
        format!("{lower}{}", lower.to_uppercase())
    } else {
        String::new()
    };
    let text = ENTITY.replace_all(&htmlentities(&text), " ").into_owned();
    str_word_count(&text, &characters)
}

/// `htmlentities( $text )` (`ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401`): characters with a named entity of HTML 4.01
/// become `&name;`, `'` becomes `&#039;`. Only whether an entity is used matters here, so every one is `&e;`.
fn htmlentities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\'' => out.push_str("&#039;"),
            '&' | '"' | '<' | '>' => out.push_str("&e;"),
            c if has_html401_entity(c as u32) => out.push_str("&e;"),
            c => out.push(c),
        }
    }
    out
}

/// The code points with a named entity in HTML 4.01 (besides `&"<>`).
fn has_html401_entity(cp: u32) -> bool {
    matches!(cp, 160..=255 | 338 | 339 | 352 | 353 | 376 | 402 | 710 | 732 | 913..=929 | 931..=937 | 945..=969 | 977 | 978 | 982
        | 8194 | 8195 | 8201 | 8204..=8207 | 8211 | 8212 | 8216..=8218 | 8220..=8222 | 8224..=8226 | 8230 | 8240 | 8242 | 8243
        | 8249 | 8250 | 8254 | 8260 | 8364 | 8465 | 8472 | 8476 | 8482 | 8501 | 8592..=8596 | 8629 | 8656..=8660 | 8704
        | 8706 | 8707 | 8709 | 8711..=8713 | 8715 | 8719 | 8721 | 8722 | 8727 | 8730 | 8733 | 8734 | 8736 | 8743..=8747
        | 8756 | 8764 | 8773 | 8776 | 8800 | 8801 | 8804 | 8805 | 8834..=8836 | 8838 | 8839 | 8853 | 8855 | 8869 | 8901
        | 8968..=8971 | 9001 | 9002 | 9674 | 9824 | 9827 | 9829 | 9830)
}

/// PHP's `str_word_count( $text, 0, $characters )`: bytes, ASCII letters, `'`, `-` and the bytes of `characters` make
/// words; a leading `'`/`-` of the text and a trailing `-` do not count.
pub fn str_word_count(text: &str, characters: &str) -> usize {
    let mut extra = [false; 256];
    for b in characters.bytes() {
        extra[b as usize] = true;
    }
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut end = bytes.len();
    if end > 0 && ((bytes[0] == b'\'' && !extra[b'\'' as usize]) || (bytes[0] == b'-' && !extra[b'-' as usize])) {
        start = 1;
    }
    if end > start && bytes[end - 1] == b'-' && !extra[b'-' as usize] {
        end -= 1;
    }
    let word = |b: u8| b.is_ascii_alphabetic() || extra[b as usize] || b == b'\'' || b == b'-';
    let mut count = 0;
    let mut i = start;
    while i < end {
        let s = i;
        while i < end && word(bytes[i]) {
            i += 1;
        }
        if i > s {
            count += 1;
        }
        i += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words_like_php() {
        assert_eq!(str_word_count("Hello World again", ""), 3);
        assert_eq!(str_word_count("don&#039;t e-mail -x", ""), 4);
        assert_eq!(word_count("<p>Ein Gebäude &amp; mehr</p>", "Bilder"), 6);
    }
}
