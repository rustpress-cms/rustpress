//! `wp/v2/posts` and `wp/v2/pages` (`WP_REST_Posts_Controller`).

use rustpress_blocks::content::{post_class, the_content, the_title_for};
use rustpress_blocks::excerpt::get_the_excerpt;
use rustpress_core::formatting::{convert_chars, wpautop};
use rustpress_core::types::PostTypeInfo;
use rustpress_db::{OrderBy, Order, Post, PostQuery};
use serde_json::{Map, Value, json};
use std::sync::Arc;

use crate::error::RestError;
use crate::params::Params;
use crate::util::{LinkParams, Links, env, link_header, rfc3339};
use crate::{Outcome, Resp, Rest, Result};

#[derive(Clone, Copy)]
pub enum Kind<'a> {
    Post,
    Page,
    Attachment,
    /// A post type of a plugin or theme, as far as it is known.
    Custom(&'a PostTypeInfo),
}

impl PartialEq for Kind<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

impl<'a> Kind<'a> {
    pub fn name(self) -> &'a str {
        match self {
            Kind::Post => "post",
            Kind::Page => "page",
            Kind::Attachment => "attachment",
            Kind::Custom(info) => &info.name,
        }
    }

    /// The only status an anonymous visitor may list.
    pub fn public_status(self) -> &'static str {
        if self == Kind::Attachment { "inherit" } else { "publish" }
    }

    pub fn base(self) -> &'a str {
        match self {
            Kind::Post => "posts",
            Kind::Page => "pages",
            Kind::Attachment => "media",
            Kind::Custom(info) => info.rest_base(),
        }
    }

    pub fn hierarchical(self) -> bool {
        match self {
            Kind::Post => false,
            Kind::Page | Kind::Attachment => true,
            Kind::Custom(info) => info.hierarchical,
        }
    }
}

/// Parameters WordPress turns into arrays before building pagination links.
const POST_ARRAYS: &[&str] = &["author", "author_exclude", "exclude", "include", "parent", "parent_exclude", "slug", "status", "categories", "categories_exclude", "tags", "tags_exclude"];

/// The members of an item in the `embed` context.
const EMBED_FIELDS: &[&str] = &["id", "date", "slug", "type", "link", "title", "excerpt", "author", "featured_media", "_links"];

impl Rest<'_> {
    pub(crate) async fn posts_collection(&self, kind: Kind<'_>, params: &Params) -> Result<Outcome> {
        if params.get("context") == Some("edit") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit posts in this post type.", 401)));
        }
        Ok(match self.post_query(kind, params).await? {
            Ok(query) => self.run_posts(kind, params, query).await?,
            Err(e) => Err(e),
        })
    }

    pub(crate) async fn run_posts(&self, kind: Kind<'_>, params: &Params, mut query: PostQuery) -> Result<Outcome> {
        let per_page = query.limit.unwrap_or(10);
        let page = match params.integer("page", 1, 1, None) {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let total = self.site.db.count_posts(&query).await?;
        let max_pages = total.div_ceil(per_page);
        if page > max_pages && total > 0 {
            return Ok(Err(RestError::new("rest_post_invalid_page_number", "The page number requested is larger than the number of pages available.", 400)));
        }
        if query.offset == 0 {
            query.offset = (page - 1).saturating_mul(per_page);
        }
        let posts = self.site.db.posts(&query).await?;
        let ids: Vec<u64> = posts.iter().map(|p| p.id).collect();
        self.site.db.prime_loop(&posts).await?;
        self.site.db.prime_revisions(&ids).await?;
        let context = params.get("context").unwrap_or("view");
        let mut items = Vec::with_capacity(posts.len());
        for post in &posts {
            items.push(if kind == Kind::Attachment { self.prepare_media(post, context).await? } else { self.prepare_post(post, kind, context, params).await? });
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(max_pages);
        resp.link = link_header(self, &format!("wp/v2/{}", kind.base()), params, &LinkParams { arrays: POST_ARRAYS, booleans: &["sticky"] }, page, max_pages);
        Ok(Ok(resp))
    }

    /// Translates the request parameters into a database query.
    pub(crate) async fn post_query(&self, kind: Kind<'_>, params: &Params) -> Result<std::result::Result<PostQuery, RestError>> {
        macro_rules! try_param {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(e)),
                }
            };
        }
        let per_page = try_param!(params.integer("per_page", 10, 1, Some(100)));
        try_param!(params.integer("page", 1, 1, None));
        let offset = try_param!(params.integer("offset", 0, 0, None));
        let order = try_param!(params.one_of("order", &["asc", "desc"], "desc"));
        let mut orderby_values = vec!["author", "date", "id", "include", "modified", "parent", "relevance", "slug", "include_slugs", "title"];
        if kind == Kind::Page || self.supports(kind, "page-attributes") {
            orderby_values.push("menu_order");
        }
        let orderby = try_param!(params.one_of("orderby", &orderby_values, "date"));
        let public_status = kind.public_status();
        let status = params.list("status").unwrap_or_else(|| vec![public_status.to_owned()]);
        if status.iter().any(|s| s != public_status) {
            return Ok(Err(RestError::invalid_param_with_data("status", "Status is forbidden.", "rest_forbidden_status", json!({"status": 401}))));
        }

        let mut query = PostQuery { post_types: vec![kind.name().into()], statuses: vec![public_status.into()], limit: Some(per_page), offset, public_parent: kind == Kind::Attachment, ..Default::default() };
        query.order = if order == "asc" { Order::Asc } else { Order::Desc };
        let include = params.id_list("include").unwrap_or_default();
        let search = params.get("search").filter(|s| !s.is_empty());
        query.order_by = match orderby {
            "author" => OrderBy::Author,
            "id" => OrderBy::Id,
            "include" => {
                if include.is_empty() {
                    return Ok(Err(RestError::new("rest_orderby_include_missing_include", "You need to define an include parameter to order by include.", 400)));
                }
                OrderBy::Include
            }
            "modified" => OrderBy::Modified,
            "parent" => OrderBy::Parent,
            "relevance" => {
                if search.is_none() {
                    return Ok(Err(RestError::new("rest_no_search_term_defined", "You need to define a search term to order by relevance.", 400)));
                }
                OrderBy::Relevance
            }
            "slug" => OrderBy::Slug,
            "title" => OrderBy::Title,
            "menu_order" => OrderBy::MenuOrder,
            _ => OrderBy::Date,
        };
        query.ids = include;
        query.exclude_ids = params.id_list("exclude").unwrap_or_default();
        query.names = params.list("slug").unwrap_or_default();
        if self.supports(kind, "author") {
            query.author_in = params.id_list("author").unwrap_or_default();
            query.author_not_in = params.id_list("author_exclude").unwrap_or_default();
        }
        if kind == Kind::Attachment {
            query.mime_prefix = params.get("media_type").filter(|t| matches!(*t, "image" | "video" | "audio" | "application")).map(str::to_owned);
            query.mime_type = params.get("mime_type").map(str::to_owned);
        }
        if kind != Kind::Post && kind.hierarchical() {
            query.parent_in = params.parent_list("parent").unwrap_or_default();
            query.parent_not_in = params.parent_list("parent_exclude").unwrap_or_default();
        }
        if kind == Kind::Page || self.supports(kind, "page-attributes") {
            query.menu_order = params.get("menu_order").and_then(|v| v.parse().ok());
        }
        if let Some(search) = search {
            query.search = rustpress_core::query::parse_search_terms(search, false);
            query.search_phrase = Some(search.to_owned());
        }
        for (key, slot) in [("after", &mut query.date_after), ("before", &mut query.date_before), ("modified_after", &mut query.modified_after), ("modified_before", &mut query.modified_before)] {
            if let Some(value) = params.get(key) {
                match chrono::NaiveDateTime::parse_from_str(value.trim_end_matches('Z'), "%Y-%m-%dT%H:%M:%S") {
                    Ok(date) => *slot = Some(date.format("%Y-%m-%d %H:%M:%S").to_string()),
                    Err(_) => return Ok(Err(RestError::invalid_param(key, "Invalid date.", "rest_invalid_date"))),
                }
            }
        }
        if kind == Kind::Post {
            if let Some(sticky) = params.get("sticky") {
                let sticky_ids = &self.site.settings.sticky_posts;
                if matches!(sticky, "1" | "true") {
                    let mut ids: Vec<u64> = if query.ids.is_empty() { sticky_ids.clone() } else { sticky_ids.iter().copied().filter(|id| query.ids.contains(id)).collect() };
                    if ids.is_empty() {
                        ids = vec![0];
                    }
                    query.ids = ids;
                } else {
                    query.exclude_ids.extend(sticky_ids.iter().copied());
                }
            }
            for (key, taxonomy, exclude) in [("categories", "category", false), ("tags", "post_tag", false), ("categories_exclude", "category", true), ("tags_exclude", "post_tag", true)] {
                let Some(ids) = params.id_list(key) else { continue };
                let mut taxonomy_ids = Vec::new();
                for id in ids {
                    if let Some(term) = self.site.db.term_by_id(Some(taxonomy), id).await? {
                        taxonomy_ids.push(term.term_taxonomy_id);
                    }
                }
                if exclude {
                    query.tax_not.extend(taxonomy_ids);
                } else if taxonomy_ids.is_empty() {
                    // A filter on terms that do not exist matches nothing.
                    query.ids = vec![0];
                } else {
                    query.tax_groups.push(taxonomy_ids);
                }
            }
        }
        if let Kind::Custom(info) = kind {
            for tax in self.taxonomies_of(&info.name) {
                for (key, exclude) in [(tax.base.clone(), false), (format!("{}_exclude", tax.base), true)] {
                    let Some(ids) = params.id_list(&key) else { continue };
                    let mut taxonomy_ids = Vec::new();
                    for id in ids {
                        if let Some(term) = self.site.db.term_by_id(Some(&tax.name), id).await? {
                            taxonomy_ids.push(term.term_taxonomy_id);
                        }
                    }
                    if exclude {
                        query.tax_not.extend(taxonomy_ids);
                    } else if taxonomy_ids.is_empty() {
                        query.ids = vec![0];
                    } else {
                        query.tax_groups.push(taxonomy_ids);
                    }
                }
            }
        }
        Ok(Ok(query))
    }

    /// `post_type_supports()` for the type of a REST request.
    pub(crate) fn supports(&self, kind: Kind<'_>, feature: &str) -> bool {
        match kind {
            Kind::Custom(info) => info.supports(feature),
            _ => self.site.post_type_supports(kind.name(), feature),
        }
    }

    pub(crate) async fn posts_item(&self, kind: Kind<'_>, id: &str, params: &Params) -> Result<Outcome> {
        let Ok(id) = id.parse::<u64>() else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        if params.get("context") == Some("edit") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit this post.", 401)));
        }
        let post = self.site.db.post(id).await?;
        let Some(post) = post.filter(|p| p.post_type == kind.name() && p.post_status == kind.public_status()) else {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        };
        if let Some(given) = params.get("password").filter(|p| !p.is_empty())
            && !rustpress_core::hash_equals(&post.post_password, given) {
                return Ok(Err(RestError::new("rest_post_incorrect_password", "Incorrect post password.", 403)));
            }
        if kind == Kind::Attachment && post.post_parent != 0 {
            let parent = self.site.db.post(post.post_parent).await?;
            if parent.is_none_or(|p| p.post_status != "publish") {
                return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
            }
        }
        let context = params.get("context").unwrap_or("view");
        let data = if kind == Kind::Attachment { self.prepare_media(&post, context).await? } else { self.prepare_post(&post, kind, context, params).await? };
        let mut resp = Resp::item(data);
        resp.alternate = Some(self.site.permalink(&post).await?);
        Ok(Ok(resp))
    }

    /// Revisions and autosaves are for editors only.
    pub(crate) async fn revisions_denied(&self, kind: Kind<'_>, parent: &str, what: &str) -> Result<Outcome> {
        let post = match parent.parse::<u64>() {
            Ok(id) => self.site.db.post(id).await?,
            Err(_) => None,
        };
        if post.is_none_or(|p| p.post_type != kind.name()) {
            return Ok(Err(RestError::new("rest_post_invalid_parent", "Invalid post parent ID.", 404)));
        }
        Ok(Err(RestError::new("rest_cannot_read", &format!("Sorry, you are not allowed to view {what} of this post."), 401)))
    }

    /// `rest_is_field_included( $field, $fields )` for the `_fields` of the request: fields that are not asked
    /// for are not computed (rendering the content of a hundred posts for `_fields=id` was most of the time).
    pub(crate) fn field_included(&self, field: &str) -> bool {
        let params = Params::parse(&self.blank.raw_query);
        let Some(list) = params.get("_fields").filter(|f| !f.trim().is_empty()) else { return true };
        // `_embed` needs the links, whatever `_fields` says.
        if field == "_links" && params.get("_embed").is_some() {
            return true;
        }
        list.split(',').map(str::trim).any(|accepted| accepted == field || accepted.starts_with(&format!("{field}.")) || field.starts_with(&format!("{accepted}.")))
    }

    /// `prepare_item_for_response()` for the `view` and `embed` contexts.
    pub(crate) async fn prepare_post(&self, post: &Post, kind: Kind<'_>, context: &str, params: &Params) -> Result<Value> {
        let env = env(self);
        let shared = Arc::new(post.clone());
        let protected = !post.post_password.is_empty();
        let unlocked = protected && params.get("password") == Some(post.post_password.as_str());
        let meta = self.site.db.post_meta(post.id).await?;
        let meta_value = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());

        let mut data = Map::new();
        data.insert("id".into(), Value::from(post.id));
        data.insert("date".into(), Value::from(rfc3339(&post.post_date)));
        data.insert("date_gmt".into(), self.date_gmt(&post.post_date_gmt, &post.post_date));
        data.insert("guid".into(), json!({"rendered": post.guid}));
        data.insert("modified".into(), Value::from(rfc3339(&post.post_modified)));
        data.insert("modified_gmt".into(), self.date_gmt(&post.post_modified_gmt, &post.post_modified));
        data.insert("slug".into(), Value::from(post.post_name.clone()));
        data.insert("status".into(), Value::from(post.post_status.clone()));
        data.insert("type".into(), Value::from(post.post_type.clone()));
        let permalink = self.site.permalink(post).await?;
        data.insert("link".into(), Value::from(permalink));

        // The REST API shows titles without the "Protected:" / "Private:" prefixes.
        let plain = Post { post_password: String::new(), post_status: "publish".into(), ..post.clone() };
        let custom = matches!(kind, Kind::Custom(_));
        // The properties of a custom type follow what it supports.
        let has = |feature: &str| !custom || self.supports(kind, feature);
        if has("title") {
            data.insert("title".into(), json!({"rendered": the_title_for(self.site, &plain)}));
        }

        let hidden = protected && !unlocked;
        // Embedded items are not filtered by the `_fields` of the request around them.
        let wanted = |field: &str| context == "embed" || self.field_included(field);
        if has("editor") && wanted("content") {
            let content = if hidden { String::new() } else { the_content(&env, &shared, &post.post_content).await? };
            data.insert("content".into(), json!({"rendered": content, "protected": protected}));
        }
        if has("excerpt") && wanted("excerpt") {
            let excerpt = if hidden {
                String::new()
            } else {
                // The password check passed (`check_password_required()`): the excerpt is made like for any post.
                let open = if protected { Arc::new(Post { post_password: String::new(), ..post.clone() }) } else { shared.clone() };
                let raw = get_the_excerpt(&env, &open).await?;
                wpautop(&convert_chars(&self.site.texturize(&raw)))
            };
            data.insert("excerpt".into(), json!({"rendered": excerpt, "protected": protected}));
        }
        if has("author") {
            data.insert("author".into(), Value::from(post.post_author));
        }
        let featured = meta_value("_thumbnail_id").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        if has("thumbnail") {
            data.insert("featured_media".into(), Value::from(featured));
        }
        if kind == Kind::Page || kind.hierarchical() && custom {
            data.insert("parent".into(), Value::from(post.post_parent));
        }
        if kind == Kind::Page || (custom && has("page-attributes")) {
            data.insert("menu_order".into(), Value::from(post.menu_order));
        }
        if has("comments") {
            data.insert("comment_status".into(), Value::from(post.comment_status.clone()));
            data.insert("ping_status".into(), Value::from(post.ping_status.clone()));
        }
        if kind == Kind::Post {
            data.insert("sticky".into(), Value::Bool(self.site.settings.sticky_posts.contains(&post.id)));
        }
        let template = meta_value("_wp_page_template").filter(|t| !t.is_empty() && t != "default").unwrap_or_default();
        data.insert("template".into(), Value::from(template));
        if (kind == Kind::Post || (custom && has("post-formats"))) && wanted("format") {
            let formats = self.site.db.terms_for_post(post.id, Some("post_format")).await?;
            let format = formats.first().and_then(|t| t.slug.strip_prefix("post-format-")).unwrap_or("standard").to_owned();
            data.insert("format".into(), Value::from(format));
        }
        if !custom {
            data.insert("meta".into(), json!({"footnotes": meta_value("footnotes").unwrap_or_default()}));
        } else if has("custom-fields") {
            data.insert("meta".into(), Value::Array(Vec::new()));
        }
        if kind == Kind::Post {
            for (base, taxonomy) in [("categories", "category"), ("tags", "post_tag")].into_iter().filter(|(base, _)| wanted(base)) {
                let mut terms = self.site.db.terms_for_post(post.id, Some(taxonomy)).await?;
                terms.sort_by_key(|t| (t.name.to_lowercase(), t.term_id));
                data.insert(base.into(), Value::from(terms.iter().map(|t| t.term_id).collect::<Vec<_>>()));
            }
        }
        if let Kind::Custom(info) = kind {
            for tax in self.taxonomies_of(&info.name).into_iter().filter(|tax| wanted(&tax.base)) {
                let mut terms = self.site.db.terms_for_post(post.id, Some(&tax.name)).await?;
                terms.sort_by_key(|t| (t.name.to_lowercase(), t.term_id));
                data.insert(tax.base.clone(), Value::from(terms.iter().map(|t| t.term_id).collect::<Vec<_>>()));
            }
        }
        // `class_list` belongs to the types that have pages of their own.
        if !matches!(kind, Kind::Custom(info) if !info.public) && (wanted("class_list") || wanted("permalink_template") || wanted("generated_slug")) {
            data.insert("class_list".into(), Value::from(post_class(&env, post, &[], false, false).await?));
        }

        if wanted("_links") || wanted("_embedded") {
            data.insert("_links".into(), self.post_links(post, kind, featured).await?);
        }
        if context == "embed" {
            // `WP_Navigation_Fallback::update_wp_navigation_post_schema()` exposes more of a menu.
            let menu = post.post_type == "wp_navigation";
            data.retain(|key, _| EMBED_FIELDS.contains(&key.as_str()) || (menu && matches!(key.as_str(), "status" | "content")));
            for key in ["title", "excerpt"] {
                if let Some(Value::Object(inner)) = data.get_mut(key) {
                    inner.retain(|k, _| k == "rendered" || k == "protected");
                }
            }
            if menu {
                if let Some(Value::Object(title)) = data.get_mut("title") {
                    let mut with_raw = Map::new();
                    with_raw.insert("raw".into(), Value::from(post.post_title.clone()));
                    with_raw.extend(std::mem::take(title));
                    *title = with_raw;
                }
                if let Some(Value::Object(content)) = data.get_mut("content") {
                    let rendered = content.remove("rendered").unwrap_or(Value::Null);
                    let protected = content.remove("protected").unwrap_or(Value::Bool(false));
                    *content = Map::new();
                    content.insert("raw".into(), Value::from(post.post_content.clone()));
                    content.insert("rendered".into(), rendered);
                    content.insert("protected".into(), protected);
                    content.insert("block_version".into(), Value::from(i32::from(rustpress_blocks::content::has_blocks(&post.post_content))));
                }
            }
        }
        Ok(Value::Object(data))
    }

    /// `prepare_links()`.
    pub(crate) async fn post_links(&self, post: &Post, kind: Kind<'_>, featured: u64) -> Result<Value> {
        let base = format!("wp/v2/{}", kind.base());
        let mut links = Links::new();
        links.self_link(self.rest_url(&format!("{base}/{}", post.id)));
        links.add("collection", vec![], self.rest_url(&base));
        links.add("about", vec![], self.rest_url(&format!("wp/v2/types/{}", kind.name())));
        let builtin = matches!(kind, Kind::Post | Kind::Page);
        if (builtin || self.supports(kind, "author")) && post.post_author != 0 {
            links.embeddable("author", self.rest_url(&format!("wp/v2/users/{}", post.post_author)));
        }
        if builtin || self.supports(kind, "comments") {
            links.embeddable("replies", rustpress_core::urls::add_query_arg(&self.rest_url("wp/v2/comments"), "post", &post.id.to_string()));
        }

        if builtin || self.supports(kind, "revisions") {
            let (count, latest) = self.site.db.revision_summary(post.id).await?;
            let revisions_base = self.rest_url(&format!("{base}/{}/revisions", post.id));
            links.add("version-history", vec![("count", Value::from(count))], revisions_base.clone());
            if count > 0 {
                links.add("predecessor-version", vec![("id", Value::from(latest))], format!("{revisions_base}/{latest}"));
            }
        }
        if (kind == Kind::Page || matches!(kind, Kind::Custom(info) if info.hierarchical)) && post.post_parent != 0 {
            links.embeddable("up", self.rest_url(&format!("{base}/{}", post.post_parent)));
        }
        if featured != 0
            && let Some(media) = self.site.db.post(featured).await?
                && (media.post_status == "publish" || media.post_status == "inherit") {
                    links.embeddable("wp:featuredmedia", self.rest_url(&format!("wp/v2/media/{featured}")));
                }
        links.add("wp:attachment", vec![], rustpress_core::urls::add_query_arg(&self.rest_url("wp/v2/media"), "parent", &post.id.to_string()));
        if kind == Kind::Post {
            for (taxonomy, route) in [("category", "wp/v2/categories"), ("post_tag", "wp/v2/tags")] {
                links.add(
                    "wp:term",
                    vec![("taxonomy", Value::from(taxonomy)), ("embeddable", Value::Bool(true))],
                    rustpress_core::urls::add_query_arg(&self.rest_url(route), "post", &post.id.to_string()),
                );
            }
        } else if let Kind::Custom(info) = kind {
            for tax in self.taxonomies_of(&info.name) {
                links.add(
                    "wp:term",
                    vec![("taxonomy", Value::from(tax.name.clone())), ("embeddable", Value::Bool(true))],
                    rustpress_core::urls::add_query_arg(&self.rest_url(&format!("wp/v2/{}", tax.base)), "post", &post.id.to_string()),
                );
            }
        }
        Ok(links.finish())
    }
}
