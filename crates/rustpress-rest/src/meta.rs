//! The routes that describe the site's content model: `wp/v2/types`, `wp/v2/taxonomies`,
//! `wp/v2/statuses` and `wp/v2/search`.

use rustpress_blocks::content::the_title_for;
use rustpress_blocks::php_json::empty_object;
use rustpress_db::{OrderBy, Order, PostQuery};
use serde_json::{Map, Value, json};

use crate::error::{Msg, RestError};
use crate::params::Params;
use crate::util::{LinkParams, Links, link_header_with};
use crate::{Outcome, Resp, Rest, Result};

const NAMESPACE: &str = "wp/v2";

/// Taxonomies that exist in WordPress but have no REST routes.
const HIDDEN_TAXONOMIES: [&str; 4] = ["post_format", "link_category", "wp_theme", "wp_template_part_area"];

/// Post types that exist in WordPress but have no REST routes.
const HIDDEN_POST_TYPES: [&str; 5] = ["revision", "customize_changeset", "oembed_cache", "user_request", "custom_css"];

/// A post type as `wp/v2/types` describes it.
pub(crate) struct TypeEntry {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) hierarchical: bool,
    pub(crate) has_archive: Value,
    pub(crate) icon: Value,
    pub(crate) taxonomies: Vec<String>,
    pub(crate) rest_base: String,
}

/// A taxonomy as `wp/v2/taxonomies` describes it.
pub(crate) struct TaxonomyEntry {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) types: Vec<String>,
    pub(crate) hierarchical: bool,
    pub(crate) rest_base: String,
}

fn embed_only(data: &mut Map<String, Value>, keep: &[&str]) {
    data.retain(|key, _| keep.contains(&key.as_str()));
}

/// The post types WordPress registers for blocks, as far as anonymous visitors can list them.
pub(crate) fn core_post_type(base: &str) -> rustpress_core::types::PostTypeInfo {
    let (name, supports): (&str, &[&str]) = if base == "blocks" { ("wp_block", &["title", "editor", "revisions", "custom-fields"]) } else { ("wp_navigation", &["title", "editor", "revisions"]) };
    let mut info = rustpress_core::types::PostTypeInfo::core(name, base);
    info.supports = Some(supports.iter().map(|s| (*s).to_owned()).collect());
    info.public = false;
    info
}

/// The routes that only a logged-in editor can use: the error WordPress sends a visitor.
/// `route` is the path after `wp/v2/`.
pub(crate) fn denied(route: &[&str], params: &Params) -> Option<RestError> {
    let message = |code: &str, text: &str| Some(RestError::new(code, text, 401));
    match route {
        ["settings", ..] => message("rest_forbidden", "Sorry, you are not allowed to do that."),
        ["block-types", ..] => message("rest_block_type_cannot_view", "Sorry, you are not allowed to manage block types."),
        ["themes", ..] if params.get("status") == Some("active") => message("rest_cannot_view_active_theme", "Sorry, you are not allowed to view the active theme."),
        ["themes", ..] => message("rest_cannot_view_themes", "Sorry, you are not allowed to view themes."),
        ["plugins", ..] => message("rest_cannot_view_plugins", "Sorry, you are not allowed to manage plugins for this site."),
        ["sidebars" | "widgets" | "widget-types", ..] => message("rest_cannot_manage_widgets", "Sorry, you are not allowed to manage widgets on this site."),
        ["menus"] => message("rest_cannot_view", "Sorry, you are not allowed to view menus."),
        ["menu-items"] => message("rest_cannot_view", "Sorry, you are not allowed to view menu items."),
        ["menu-locations", ..] => message("rest_cannot_view", "Sorry, you are not allowed to view menu locations."),
        ["templates" | "template-parts", ..] => message("rest_cannot_manage_templates", "Sorry, you are not allowed to access the templates on this site."),
        ["icons", ..] => message("rest_cannot_view", "Sorry, you are not allowed to view the registered icons."),
        ["icon-collections", ..] => message("rest_cannot_view", "Sorry, you are not allowed to view the registered icon collections."),
        ["block-renderer", ..] if params.get("context") != Some("edit") => Some(RestError::invalid_param("context", Msg::new("%1$s is not %2$s.").with(&["context", "edit"]), "rest_not_in_enum")),
        ["block-renderer", ..] => message("block_cannot_read", "Sorry, you are not allowed to read blocks as this user."),
        ["font-families", ..] => message("rest_cannot_read", "Sorry, you are not allowed to access font families."),
        ["global-styles", "themes", ..] => message("rest_cannot_read_global_styles", "Sorry, you are not allowed to access the global styles on this site."),
        ["block-directory", ..] => message("rest_block_directory_cannot_view", "Sorry, you are not allowed to browse the block directory."),
        ["pattern-directory", ..] => message("rest_pattern_directory_cannot_view", "Sorry, you are not allowed to browse the local block pattern directory."),
        ["block-patterns", "patterns"] => message("rest_cannot_view", "Sorry, you are not allowed to view the registered block patterns."),
        ["block-patterns", "categories"] => message("rest_cannot_view", "Sorry, you are not allowed to view the registered block pattern categories."),
        _ => None,
    }
}

impl Rest<'_> {
    fn core_types(&self) -> Vec<TypeEntry> {
        let s = self.site;
        let entry = |name: &str, label: String, description: String, hierarchical: bool, icon: Option<&str>, taxonomies: &[&str], base: &str| TypeEntry {
            name: name.into(),
            label,
            description,
            hierarchical,
            has_archive: Value::Bool(false),
            icon: icon.map_or(Value::Null, Value::from),
            taxonomies: taxonomies.iter().map(|t| (*t).to_owned()).collect(),
            rest_base: base.into(),
        };
        vec![
            entry("post", s.trx("Posts", "post type general name"), String::new(), false, Some("dashicons-admin-post"), &["category", "post_tag"], "posts"),
            entry("page", s.trx("Pages", "post type general name"), String::new(), true, Some("dashicons-admin-page"), &[], "pages"),
            entry("attachment", s.trx("Media", "post type general name"), String::new(), false, Some("dashicons-admin-media"), &[], "media"),
            entry("nav_menu_item", s.tr("Navigation Menu Items"), String::new(), false, None, &["nav_menu"], "menu-items"),
            entry("wp_block", s.trx("Patterns", "post type general name"), String::new(), false, None, &["wp_pattern_category"], "blocks"),
            entry("wp_template", s.trx("Templates", "post type general name"), s.tr("Templates to include in your theme."), false, None, &[], "templates"),
            entry("wp_template_part", s.trx("Template Parts", "post type general name"), s.tr("Template parts to include in your templates."), false, None, &[], "template-parts"),
            entry("wp_global_styles", s.trx("Global Styles", "post type general name"), s.tr("Global styles to include in themes."), false, None, &[], "global-styles"),
            entry("wp_navigation", s.trx("Navigation Menus", "post type general name"), s.tr("Navigation menus that can be inserted into your site."), false, None, &[], "navigation"),
            entry("wp_font_family", s.tr("Font Families"), String::new(), false, None, &[], "font-families"),
            entry("wp_font_face", s.tr("Font Faces"), String::new(), false, None, &[], "font-families/(?P<font_family_id>[\\d]+)/font-faces"),
        ]
    }

    /// Every post type with REST routes, in registration order.
    pub(crate) fn type_entries(&self) -> Vec<TypeEntry> {
        let mut all = self.core_types();
        for info in self.site.types.post_types.iter().filter(|t| t.show_in_rest) {
            // `has_archive` is what the registration said: `true`, or the slug without the permalink front.
            let front = self.site.front();
            let bare = |s: &str| s.strip_prefix(front.as_str()).unwrap_or(s).to_owned();
            let has_archive = match &info.archive_slug {
                None => Value::Bool(false),
                Some(slug) if bare(slug) == bare(&info.slug) => Value::Bool(true),
                Some(slug) => Value::from(bare(slug)),
            };
            all.push(TypeEntry {
                name: info.name.clone(),
                label: info.plural_label(),
                description: info.description.clone(),
                hierarchical: info.hierarchical,
                has_archive,
                icon: info.menu_icon.clone().map_or(Value::Null, Value::from),
                taxonomies: self.taxonomies_of(&info.name).into_iter().map(|t| t.name).collect(),
                rest_base: info.rest_base().to_owned(),
            });
        }
        all
    }

    pub(crate) fn prepare_type(&self, entry: &TypeEntry, context: &str) -> Value {
        let mut data = Map::new();
        data.insert("description".into(), Value::from(entry.description.clone()));
        data.insert("hierarchical".into(), Value::Bool(entry.hierarchical));
        data.insert("has_archive".into(), entry.has_archive.clone());
        data.insert("name".into(), Value::from(entry.label.clone()));
        data.insert("slug".into(), Value::from(entry.name.clone()));
        data.insert("icon".into(), entry.icon.clone());
        data.insert("taxonomies".into(), Value::from(entry.taxonomies.clone()));
        data.insert("rest_base".into(), Value::from(entry.rest_base.clone()));
        data.insert("rest_namespace".into(), Value::from(NAMESPACE));
        data.insert("template".into(), Value::Array(Vec::new()));
        data.insert("template_lock".into(), Value::Bool(false));
        let mut links = Links::new();
        links.add("collection", vec![], self.rest_url("wp/v2/types"));
        links.add("wp:items", vec![], self.rest_url(&format!("wp/v2/{}", entry.rest_base)));
        data.insert("_links".into(), links.finish());
        if context == "embed" {
            embed_only(&mut data, &["name", "slug", "icon", "rest_base", "rest_namespace", "template", "template_lock", "_links"]);
        }
        Value::Object(data)
    }

    pub(crate) fn types_collection(&self, params: &Params) -> Outcome {
        if params.get("context") == Some("edit") {
            return Err(RestError::new("rest_cannot_view", "Sorry, you are not allowed to edit posts in this post type.", 401));
        }
        let context = params.get("context").unwrap_or("view");
        let mut data = Map::new();
        for entry in self.type_entries() {
            data.insert(entry.name.clone(), self.prepare_type(&entry, context));
        }
        Ok(Resp::item(Value::Object(data)))
    }

    pub(crate) fn types_item(&self, name: &str, params: &Params) -> Outcome {
        let entry = self.type_entries().into_iter().find(|e| e.name == name);
        let Some(entry) = entry else {
            let exists = HIDDEN_POST_TYPES.contains(&name) || self.site.types.post_type(name).is_some() || self.site.types.hidden_post_types.iter().any(|t| t == name);
            return Err(if exists {
                RestError::new("rest_cannot_read_type", "Cannot view post type.", 401).after_permission_check()
            } else {
                RestError::new("rest_type_invalid", "Invalid post type.", 404).after_permission_check()
            });
        };
        if params.get("context") == Some("edit") {
            return Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit posts in this post type.", 401).after_permission_check());
        }
        Ok(Resp::item(self.prepare_type(&entry, params.get("context").unwrap_or("view"))))
    }

    // ------------------------------------------------------------------------- taxonomies

    pub(crate) fn taxonomy_entries(&self) -> Vec<TaxonomyEntry> {
        let s = self.site;
        let mut all = vec![
            TaxonomyEntry { name: "category".into(), label: s.trx("Categories", "taxonomy general name"), description: String::new(), types: vec!["post".into()], hierarchical: true, rest_base: "categories".into() },
            TaxonomyEntry { name: "post_tag".into(), label: s.trx("Tags", "taxonomy general name"), description: String::new(), types: vec!["post".into()], hierarchical: false, rest_base: "tags".into() },
            TaxonomyEntry { name: "nav_menu".into(), label: s.tr("Navigation Menus"), description: String::new(), types: vec!["nav_menu_item".into()], hierarchical: false, rest_base: "menus".into() },
            TaxonomyEntry { name: "wp_pattern_category".into(), label: s.trx("Pattern Categories", "taxonomy general name"), description: String::new(), types: vec!["wp_block".into()], hierarchical: false, rest_base: "wp_pattern_category".into() },
        ];
        for info in self.site.types.taxonomies.iter().filter(|t| t.show_in_rest) {
            all.push(TaxonomyEntry {
                name: info.name.clone(),
                label: info.plural_label(),
                description: info.description.clone(),
                types: info.object_types.clone(),
                hierarchical: info.hierarchical,
                rest_base: info.rest_base().to_owned(),
            });
        }
        all
    }

    pub(crate) fn prepare_taxonomy(&self, entry: &TaxonomyEntry, context: &str) -> Value {
        let mut data = Map::new();
        data.insert("name".into(), Value::from(entry.label.clone()));
        data.insert("slug".into(), Value::from(entry.name.clone()));
        data.insert("description".into(), Value::from(entry.description.clone()));
        data.insert("types".into(), Value::from(entry.types.clone()));
        data.insert("hierarchical".into(), Value::Bool(entry.hierarchical));
        data.insert("rest_base".into(), Value::from(entry.rest_base.clone()));
        data.insert("rest_namespace".into(), Value::from(NAMESPACE));
        let mut links = Links::new();
        links.add("collection", vec![], self.rest_url("wp/v2/taxonomies"));
        links.add("wp:items", vec![], self.rest_url(&format!("wp/v2/{}", entry.rest_base)));
        data.insert("_links".into(), links.finish());
        if context == "embed" {
            embed_only(&mut data, &["name", "slug", "rest_base", "rest_namespace", "_links"]);
        }
        Value::Object(data)
    }

    pub(crate) fn taxonomies_collection(&self, params: &Params) -> Outcome {
        if params.get("context") == Some("edit") {
            return Err(RestError::new("rest_cannot_view", "Sorry, you are not allowed to manage terms in this taxonomy.", 401));
        }
        let context = params.get("context").unwrap_or("view");
        let wanted = params.get("type").filter(|t| !t.is_empty());
        let mut data = Map::new();
        for entry in self.taxonomy_entries() {
            if wanted.is_some_and(|t| !entry.types.iter().any(|x| x == t)) {
                continue;
            }
            data.insert(entry.name.clone(), self.prepare_taxonomy(&entry, context));
        }
        // An empty result is still a JSON object.
        Ok(Resp::item(if data.is_empty() { empty_object() } else { Value::Object(data) }))
    }

    pub(crate) fn taxonomies_item(&self, name: &str, params: &Params) -> Outcome {
        let Some(entry) = self.taxonomy_entries().into_iter().find(|e| e.name == name) else {
            let hidden = HIDDEN_TAXONOMIES.contains(&name) || self.site.types.taxonomy(name).is_some();
            return Err(if hidden {
                RestError::new("rest_forbidden", "Sorry, you are not allowed to do that.", 401)
            } else {
                RestError::new("rest_taxonomy_invalid", "Invalid taxonomy.", 404).after_permission_check()
            });
        };
        if params.get("context") == Some("edit") {
            return Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to manage terms in this taxonomy.", 401).after_permission_check());
        }
        Ok(Resp::item(self.prepare_taxonomy(&entry, params.get("context").unwrap_or("view"))))
    }

    // ------------------------------------------------------------------------- statuses

    fn prepare_status(&self, slug: &str, context: &str) -> Value {
        let mut data = Map::new();
        data.insert("name".into(), Value::from(self.site.tr("Published")));
        if context == "edit" {
            data.insert("private".into(), Value::Bool(false));
            data.insert("protected".into(), Value::Bool(false));
        }
        if context != "embed" {
            data.insert("public".into(), Value::Bool(true));
            data.insert("queryable".into(), Value::Bool(true));
        }
        if context == "edit" {
            data.insert("show_in_list".into(), Value::Bool(true));
        }
        data.insert("slug".into(), Value::from(slug));
        if context != "embed" {
            data.insert("date_floating".into(), Value::Bool(false));
        }
        let mut links = Links::new();
        links.add("archives", vec![], self.rest_url("wp/v2/posts"));
        data.insert("_links".into(), links.finish());
        Value::Object(data)
    }

    pub(crate) fn statuses_collection(&self, params: &Params) -> Outcome {
        if params.get("context") == Some("edit") {
            return Err(RestError::new("rest_cannot_view", "Sorry, you are not allowed to manage post statuses.", 401));
        }
        let mut data = Map::new();
        data.insert("publish".into(), self.prepare_status("publish", params.get("context").unwrap_or("view")));
        Ok(Resp::item(Value::Object(data)))
    }

    pub(crate) fn statuses_item(&self, slug: &str, params: &Params) -> Outcome {
        const OTHERS: [&str; 11] = ["future", "draft", "pending", "private", "trash", "auto-draft", "inherit", "request-pending", "request-confirmed", "request-failed", "request-completed"];
        if slug == "publish" {
            return Ok(Resp::item(self.prepare_status("publish", params.get("context").unwrap_or("view"))));
        }
        Err(if OTHERS.contains(&slug) {
            RestError::new("rest_cannot_read_status", "Cannot view status.", 401)
        } else {
            RestError::new("rest_status_invalid", "Invalid status.", 404)
        })
    }

    // ------------------------------------------------------------------------- search

    /// The post types a search can address: public, with REST routes, without attachments.
    fn search_post_types(&self) -> Vec<String> {
        let mut all = vec!["post".to_owned(), "page".to_owned()];
        all.extend(self.site.types.post_types.iter().filter(|t| t.show_in_rest && t.public).map(|t| t.name.clone()));
        all
    }

    /// The taxonomies a search can address: public, with REST routes.
    fn search_taxonomies(&self) -> Vec<String> {
        let mut all = vec!["category".to_owned(), "post_tag".to_owned()];
        all.extend(self.site.types.taxonomies.iter().filter(|t| t.show_in_rest && t.public).map(|t| t.name.clone()));
        all
    }

    /// The `_links` of a search result.
    fn search_links(&self, item: &str, about: &str) -> Value {
        json!({
            "self": [{"embeddable": true, "href": self.rest_url(item), "targetHints": {"allow": ["GET"]}}],
            "about": [{"href": self.rest_url(about)}],
            "collection": [{"href": self.rest_url("wp/v2/search")}],
        })
    }

    pub(crate) async fn search(&self, params: &Params) -> Result<Outcome> {
        macro_rules! try_param {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(e)),
                }
            };
        }
        let context = try_param!(params.one_of("context", &["view", "embed"], "view"));
        let _ = context;
        let page = try_param!(params.integer("page", 1, 1, None));
        let per_page = try_param!(params.integer("per_page", 10, 1, Some(100)));
        let kind = try_param!(params.one_of("type", &["post", "term", "post-format"], "post"));
        let post_types = self.search_post_types();
        let taxonomies = self.search_taxonomies();
        let mut allowed: Vec<&str> = post_types.iter().chain(taxonomies.iter()).map(String::as_str).collect();
        allowed.push("any");
        let subtypes = params.list("subtype").unwrap_or_else(|| vec!["any".to_owned()]);
        for (i, subtype) in subtypes.iter().enumerate() {
            if !allowed.contains(&subtype.as_str()) {
                let key = format!("subtype[{i}]");
                return Ok(Err(RestError::invalid_param("subtype", Msg::new("%1$s is not one of %2$l.").with(&[&key]).with_list(&allowed), "rest_not_in_enum")));
            }
        }
        let any = subtypes.iter().any(|s| s == "any");
        let search = params.get("search").filter(|s| !s.is_empty()).map(str::to_owned);
        let exclude = params.id_list("exclude").unwrap_or_default();
        let include = params.id_list("include").unwrap_or_default();

        let (items, total) = match kind {
            "post" => {
                let types: Vec<String> = post_types.iter().filter(|t| any || subtypes.contains(t)).cloned().collect();
                if types.is_empty() {
                    (Vec::new(), 0)
                } else {
                    let mut query = PostQuery { post_types: types, statuses: vec!["publish".into()], limit: Some(per_page), offset: (page - 1).saturating_mul(per_page), ..Default::default() };
                    query.exclude_ids = exclude;
                    query.ids = include;
                    if let Some(search) = &search {
                        query.search = rustpress_core::query::parse_search_terms(search, false);
                        query.search_phrase = Some(search.clone());
                        query.order_by = OrderBy::Relevance;
                    } else {
                        query.order_by = OrderBy::Date;
                        query.order = Order::Desc;
                    }
                    let found = self.site.db.count_posts(&query).await?;
                    // A page past the end finds no rows, and WordPress then reports no total either.
                    let total = if (page - 1).saturating_mul(per_page) < found { found } else { 0 };
                    let mut out = Vec::new();
                    if total > 0 {
                        for post in self.site.db.posts(&query).await? {
                            let base = self.post_type_base(&post.post_type).unwrap_or_else(|| post.post_type.clone());
                            out.push(json!({
                                "id": post.id,
                                "title": the_title_for(self.site, &post),
                                "url": self.site.permalink(&post).await?,
                                "type": "post",
                                "subtype": post.post_type,
                                "_links": self.search_links(&format!("wp/v2/{base}/{}", post.id), &format!("wp/v2/types/{}", post.post_type)),
                            }));
                        }
                    }
                    (out, total)
                }
            }
            "term" => {
                let wanted: Vec<&String> = taxonomies.iter().filter(|t| any || subtypes.contains(t)).collect();
                let needle = search.as_ref().map(|s| s.to_lowercase());
                let mut terms = Vec::new();
                for taxonomy in wanted {
                    for term in self.site.db.terms(taxonomy).await? {
                        let matches = needle.as_ref().is_none_or(|n| term.name.to_lowercase().contains(n) || term.slug.to_lowercase().contains(n));
                        if matches && !exclude.contains(&term.term_id) && (include.is_empty() || include.contains(&term.term_id)) {
                            terms.push(term);
                        }
                    }
                }
                terms.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.term_id.cmp(&b.term_id)));
                let total = terms.len() as u64;
                let mut out = Vec::new();
                for term in terms.iter().skip(((page - 1).saturating_mul(per_page)) as usize).take(per_page as usize) {
                    let base = self.tax(&term.taxonomy).map_or_else(|| term.taxonomy.clone(), |t| t.base);
                    out.push(json!({
                        "id": term.term_id,
                        "title": term.name,
                        "url": self.site.term_link(term).await?,
                        "type": term.taxonomy,
                        "_links": self.search_links(&format!("wp/v2/{base}/{}", term.term_id), &format!("wp/v2/taxonomies/{}", term.taxonomy)),
                    }));
                }
                (out, total)
            }
            // The post formats a theme supports; block themes have none.
            _ => (Vec::new(), 0),
        };
        let max_pages = total.div_ceil(per_page);
        if page > max_pages && total > 0 {
            return Ok(Err(RestError::new("rest_search_invalid_page_number", "The page number requested is larger than the number of pages available.", 400)));
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(max_pages);
        resp.link = link_header_with(self, "wp/v2/search", params, &LinkParams { arrays: &["exclude", "include", "subtype"], booleans: &[] }, page, max_pages, false);
        Ok(Ok(resp))
    }
}
