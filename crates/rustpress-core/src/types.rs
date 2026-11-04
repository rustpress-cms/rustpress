//! Custom post types and taxonomies.
//!
//! WordPress learns these from PHP (`register_post_type()`), which RustPress does not run. What
//! the database does keep is the rewrite rules WordPress built from them, and those carry what
//! the frontend needs: whether a type is viewable, its URL slug, whether it has an archive and
//! whether it is hierarchical. The types are therefore inferred from the stored `rewrite_rules`,
//! for the post types and taxonomies that occur in the tables. A type without rewrite rules
//! (plain permalinks, `rewrite => false`, or not public) stays invisible, which is also how
//! WordPress treats it on the frontend.
//!
//! What the rules do not say (labels, `show_in_rest`, `supports`) is read from the
//! `register_post_type()` / `register_taxonomy()` calls in the active plugins, the must-use
//! plugins and the theme, as far as their arguments are literals ([`scan_registrations`]).

use std::path::{Path, PathBuf};

use crate::php_lit::{Lit, Parser, Vars};
use crate::rewrite::RewriteRules;

/// Post types WordPress registers itself.
const BUILT_IN_POST_TYPES: [&str; 3] = ["post", "page", "attachment"];

/// Taxonomies WordPress registers itself (and the ones that are never public).
const BUILT_IN_TAXONOMIES: [&str; 6] = ["category", "post_tag", "nav_menu", "link_category", "post_format", "wp_theme"];

/// Post types without a frontend of their own.
pub fn is_internal_post_type(post_type: &str) -> bool {
    matches!(post_type, "revision" | "nav_menu_item" | "customize_changeset" | "oembed_cache" | "user_request" | "custom_css")
        || post_type.starts_with("wp_")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostTypeInfo {
    pub name: String,
    /// The URL prefix of single items (`buecher`, with the permalink front if there is one).
    pub slug: String,
    /// The URL of the archive, if the type has one.
    pub archive_slug: Option<String>,
    pub hierarchical: bool,
    /// `query_var`: `?book=slug` with plain permalinks; `None` when switched off.
    pub query_var: Option<String>,
    /// `labels->name`; `None` until a registration says (WordPress then says "Posts" or "Pages").
    pub label: Option<String>,
    pub singular: Option<String>,
    pub show_in_rest: bool,
    pub rest_base: Option<String>,
    /// `supports`; `None` is the default, title and editor.
    pub supports: Option<Vec<String>>,
    /// Whether a registration was found; without one nothing is known about `supports`.
    pub registered: bool,
    /// `public`: part of sitemaps and searches (unless excluded).
    pub public: bool,
    /// `publicly_queryable`: the type has pages of its own.
    pub queryable: bool,
    pub exclude_from_search: bool,
    pub description: String,
    pub menu_icon: Option<String>,
}

impl PostTypeInfo {
    /// A type WordPress registers itself, with the details the REST API needs.
    pub fn core(name: &str, rest_base: &str) -> Self {
        let mut info = Self::new(name, name.to_owned(), None, false);
        info.registered = true;
        info.rest_base = Some(rest_base.to_owned());
        info.show_in_rest = true;
        info
    }

    fn new(name: &str, slug: String, archive_slug: Option<String>, hierarchical: bool) -> Self {
        Self { name: name.to_owned(), query_var: Some(name.to_owned()), slug, archive_slug, hierarchical, label: None, singular: None, show_in_rest: false, rest_base: None, supports: None, registered: false, public: true, queryable: true, exclude_from_search: false, description: String::new(), menu_icon: None }
    }

    /// `$post_type_object->labels->name`.
    pub fn plural_label(&self) -> String {
        self.label.clone().unwrap_or_else(|| (if self.hierarchical { "Pages" } else { "Posts" }).to_owned())
    }

    /// `$post_type_object->labels->singular_name`.
    pub fn singular_label(&self) -> String {
        self.singular.clone().or_else(|| self.label.clone()).unwrap_or_else(|| (if self.hierarchical { "Page" } else { "Post" }).to_owned())
    }

    /// The `rest_base` of the type's REST routes.
    pub fn rest_base(&self) -> &str {
        self.rest_base.as_deref().unwrap_or(&self.name)
    }

    /// `post_type_supports()`. Without a registration everything is assumed to be supported.
    pub fn supports(&self, feature: &str) -> bool {
        match &self.supports {
            Some(list) => list.iter().any(|f| f == feature),
            None if self.registered => matches!(feature, "title" | "editor"),
            None => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxonomyInfo {
    pub name: String,
    pub slug: String,
    /// `hierarchical`: terms have parents. The rewrite rules do not say; registrations do, and the
    /// terms themselves are the fallback.
    pub hierarchical: bool,
    /// `rewrite['hierarchical']`: term links carry the path of the ancestors.
    pub rewrite_hierarchical: bool,
    /// `query_var`: `?genre=slug` with plain permalinks; `None` when switched off.
    pub query_var: Option<String>,
    /// Post types that have objects in the taxonomy.
    pub object_types: Vec<String>,
    pub label: Option<String>,
    pub singular: Option<String>,
    pub show_in_rest: bool,
    pub rest_base: Option<String>,
    pub public: bool,
    pub queryable: bool,
    pub description: String,
    /// Whether a registration was found; without one `show_in_rest` is not known.
    pub registered: bool,
}

impl TaxonomyInfo {
    /// `$taxonomy->labels->name`.
    pub fn plural_label(&self) -> String {
        self.label.clone().unwrap_or_else(|| (if self.hierarchical { "Categories" } else { "Tags" }).to_owned())
    }

    /// `$taxonomy->labels->singular_name`.
    pub fn singular_label(&self) -> String {
        self.singular.clone().or_else(|| self.label.clone()).unwrap_or_else(|| (if self.hierarchical { "Category" } else { "Tag" }).to_owned())
    }

    pub fn rest_base(&self) -> &str {
        self.rest_base.as_deref().unwrap_or(&self.name)
    }
}

/// What `WP_Post_Type::add_rewrite_rules()` or `WP_Taxonomy::add_rewrite_rules()` adds for a custom type: the
/// rewrite tag, the archive rules and the permastruct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteType {
    pub name: String,
    pub taxonomy: bool,
    /// `rewrite['slug']`.
    pub slug: String,
    /// `rewrite['with_front']`; `None` for a type only the stored rules know, whose slug carries the old front.
    pub with_front: Option<bool>,
    /// The tag is `(.+?)`: a hierarchical post type, a taxonomy with `rewrite['hierarchical']`.
    pub hierarchical: bool,
    pub query_var: Option<String>,
    /// The archive slug of `has_archive` (post types).
    pub archive: Option<String>,
    /// `rewrite['feeds']` (post types; a taxonomy's permastruct always has feeds).
    pub feeds: bool,
    /// `rewrite['pages']`: the paged archive rule (post types).
    pub pages: bool,
    /// `rewrite['ep_mask']`.
    pub ep_mask: u32,
    /// Registered on `init` before priority 10: the archive rules come before those of the REST API.
    pub early: bool,
}

impl RewriteType {
    /// `WP_Post_Type::set_props()` (as in the administration) for a literal registration; `None` with
    /// `'rewrite' => false`.
    fn of_post_type(reg: &Registration) -> Option<Self> {
        let args = &reg.args;
        let rewrite = match args.get("rewrite") {
            Some(Lit::Bool(false)) => return None,
            Some(rewrite @ Lit::Array(_)) => rewrite.clone(),
            _ => Lit::Array(Vec::new()),
        };
        let set = |key: &str| rewrite.get(key).filter(|v| !matches!(v, Lit::Null));
        let slug = set("slug").filter(|v| v.truthy() == Some(true)).and_then(Lit::as_str).map_or_else(|| reg.name.clone(), str::to_owned);
        let has_archive = args.get("has_archive").cloned().unwrap_or(Lit::Bool(false));
        let archive = match &has_archive {
            v if v.truthy() == Some(false) => None,
            Lit::Bool(true) | Lit::Unknown => Some(slug.clone()),
            Lit::Str(s) => Some(s.clone()),
            Lit::Int(i) => Some(i.to_string()),
            _ => Some(slug.clone()),
        };
        let feeds = match set("feeds") {
            Some(feeds) if archive.is_some() => feeds.truthy().unwrap_or(true),
            _ => archive.is_some(),
        };
        let ep_mask = match (set("ep_mask"), args.get("permalink_epmask")) {
            (Some(Lit::Int(mask)), _) | (None, Some(Lit::Int(mask))) => *mask as u32,
            _ => 1,
        };
        Some(Self {
            name: reg.name.clone(),
            taxonomy: false,
            slug,
            with_front: Some(set("with_front").is_none_or(|v| v.truthy().unwrap_or(true))),
            hierarchical: args.get("hierarchical").and_then(Lit::truthy).unwrap_or(false),
            query_var: query_var(args, &reg.name),
            archive,
            feeds,
            pages: set("pages").is_none_or(|v| v.truthy().unwrap_or(true)),
            ep_mask,
            early: reg.priority < 10,
        })
    }

    /// `WP_Taxonomy::set_props()` (as in the administration) for a literal registration; `None` with
    /// `'rewrite' => false`.
    fn of_taxonomy(reg: &Registration) -> Option<Self> {
        let args = &reg.args;
        let rewrite = match args.get("rewrite") {
            Some(Lit::Bool(false)) => return None,
            Some(rewrite @ Lit::Array(_)) => rewrite.clone(),
            _ => Lit::Array(Vec::new()),
        };
        let set = |key: &str| rewrite.get(key).filter(|v| !matches!(v, Lit::Null));
        let slug = set("slug").filter(|v| v.truthy() == Some(true)).and_then(Lit::as_str).map_or_else(|| crate::slug::sanitize_title(&reg.name), str::to_owned);
        let hierarchical = args.get("hierarchical").and_then(Lit::truthy).unwrap_or(false);
        Some(Self {
            name: reg.name.clone(),
            taxonomy: true,
            slug,
            with_front: Some(set("with_front").is_none_or(|v| v.truthy().unwrap_or(true))),
            hierarchical: hierarchical && set("hierarchical").is_some_and(|v| v.truthy().unwrap_or(false)),
            query_var: query_var(args, &reg.name),
            archive: None,
            feeds: true,
            pages: true,
            ep_mask: match set("ep_mask") {
                Some(Lit::Int(mask)) => *mask as u32,
                _ => 0,
            },
            early: reg.priority < 10,
        })
    }

    /// A post type only the stored rules know.
    fn inferred_post_type(info: &PostTypeInfo) -> Self {
        Self {
            name: info.name.clone(),
            taxonomy: false,
            slug: info.slug.clone(),
            with_front: None,
            hierarchical: info.hierarchical,
            query_var: info.query_var.clone(),
            archive: info.archive_slug.clone(),
            feeds: info.archive_slug.is_some(),
            pages: true,
            ep_mask: 1,
            early: false,
        }
    }

    /// A taxonomy only the stored rules know.
    fn inferred_taxonomy(info: &TaxonomyInfo) -> Self {
        Self {
            name: info.name.clone(),
            taxonomy: true,
            slug: info.slug.clone(),
            with_front: None,
            hierarchical: info.rewrite_hierarchical,
            query_var: info.query_var.clone(),
            archive: None,
            feeds: true,
            pages: true,
            ep_mask: 0,
            early: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Types {
    pub post_types: Vec<PostTypeInfo>,
    pub taxonomies: Vec<TaxonomyInfo>,
    /// Post types that exist but have no pages of their own (not publicly queryable).
    pub hidden_post_types: Vec<String>,
    /// Every type with rewrite rules in registration order, also the ones without pages of their own: the rules of a
    /// changed permalink structure are built from them. Empty until [Types::apply] ran.
    rewrite_post_types: Vec<PostTypeInfo>,
    rewrite_taxonomies: Vec<TaxonomyInfo>,
    /// What the custom types add to the rewrite rules, in the order they were registered (the literal registrations,
    /// then the types only the stored rules know). `None` until [Types::apply] ran.
    rewrite_types: Option<Vec<RewriteType>>,
    /// The post types the `register_post_type()` calls name, in their order, with their labels (also those without
    /// pages of their own).
    pub registered_post_types: Vec<RegisteredLabels>,
    /// Every `register_post_type()`/`register_taxonomy()` with literal arguments, in their order; a type registered
    /// again keeps its place and takes the later arguments.
    pub registrations: Vec<Registration>,
}

/// What the administration needs of a post type a plugin or the theme registered (`WP_Post_Type`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminPostType {
    pub name: String,
    pub hierarchical: bool,
    /// `show_ui`.
    pub show_ui: bool,
    /// `show_in_menu === true`: a menu of its own (a string puts it below another menu).
    pub own_menu: bool,
    /// `show_in_admin_bar`.
    pub show_in_admin_bar: bool,
    /// `menu_position` when it is an integer.
    pub menu_position: Option<i64>,
    pub menu_icon: Option<String>,
    /// The labels the registration gives (`labels` and `label`).
    pub labels: Vec<(String, String)>,
}

impl AdminPostType {
    /// A label as `_get_custom_object_labels()` completes them; `None` when the type's default applies.
    pub fn label(&self, key: &str) -> Option<String> {
        let given = |k: &str| self.labels.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        let name = given("name").filter(|n| !n.is_empty()).or_else(|| given("label").filter(|l| !l.is_empty()));
        match key {
            "name" => name,
            "singular_name" => given("singular_name").or(name),
            "name_admin_bar" => given("name_admin_bar").or_else(|| given("singular_name").or(name.clone())).or_else(|| Some(self.name.clone())),
            "menu_name" => given("menu_name").or(name),
            "all_items" => given("all_items").or_else(|| given("menu_name").or(name)),
            other => given(other),
        }
    }
}

/// What the administration needs of a taxonomy a plugin or the theme registered (`WP_Taxonomy`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminTaxonomy {
    pub name: String,
    pub object_types: Vec<String>,
    pub show_ui: bool,
    pub show_in_menu: bool,
    /// `meta_box_cb` is not `false` (or a PHP callback RustPress cannot run): the classic editor shows its box.
    pub meta_box: bool,
    pub labels: Vec<(String, String)>,
}

impl AdminTaxonomy {
    /// `labels->menu_name`: the given one, else the name; `None` when the default (Tags/Categories) applies.
    pub fn menu_name(&self) -> Option<String> {
        let given = |k: &str| self.labels.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        let name = given("name").filter(|n| !n.is_empty()).or_else(|| given("label").filter(|l| !l.is_empty()));
        given("menu_name").or(name)
    }
}

/// A registered type with the flags lists of types filter by (`get_post_types( $args )`, `get_taxonomies( $args )`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredType {
    pub name: String,
    pub hierarchical: bool,
    pub public: bool,
    pub publicly_queryable: bool,
    pub show_in_nav_menus: bool,
    pub show_in_rest: bool,
    /// `labels->name` (or `label`), as given.
    pub label: Option<String>,
    /// `labels->singular_name`, else the name.
    pub singular: Option<String>,
}

/// The labels of an argument list: `labels[...]` and `label`.
fn labels_of(args: &Lit) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(Lit::Array(items)) = args.get("labels") {
        for (key, value) in items {
            if let (Some(Lit::Str(k)), Some(v)) = (key, value.as_str()) {
                out.push((k.clone(), v.to_owned()));
            }
        }
    }
    if let Some(label) = args.get("label").and_then(Lit::as_str) {
        out.push(("label".to_owned(), label.to_owned()));
    }
    out
}

impl Types {
    /// `get_post_types( [ '_builtin' => false ], 'objects' )` of the registrations, in their order.
    pub fn admin_post_types(&self) -> Vec<AdminPostType> {
        let flag = |args: &Lit, key: &str, default: bool| match args.get(key) {
            None | Some(Lit::Null) => default,
            Some(value) => value.truthy().unwrap_or(true),
        };
        self.registrations
            .iter()
            .filter(|r| !r.taxonomy && !BUILT_IN_POST_TYPES.contains(&r.name.as_str()) && !is_internal_post_type(&r.name))
            .map(|r| {
                let args = &r.args;
                let public = flag(args, "public", false);
                let show_ui = flag(args, "show_ui", public);
                let (own_menu, in_menu) = match args.get("show_in_menu") {
                    None | Some(Lit::Null) => (show_ui, show_ui),
                    Some(Lit::Str(parent)) => (false, !parent.is_empty()),
                    Some(value) => {
                        let on = value.truthy().unwrap_or(true);
                        (on, on)
                    }
                };
                let show_in_admin_bar = match args.get("show_in_admin_bar") {
                    None | Some(Lit::Null) => in_menu,
                    Some(value) => value.truthy().unwrap_or(true),
                };
                AdminPostType {
                    name: r.name.clone(),
                    hierarchical: flag(args, "hierarchical", false),
                    show_ui,
                    own_menu: show_ui && own_menu,
                    show_in_admin_bar,
                    menu_position: match args.get("menu_position") {
                        Some(Lit::Int(i)) => Some(*i),
                        _ => None,
                    },
                    menu_icon: args.get("menu_icon").and_then(Lit::as_str).map(str::to_owned),
                    labels: labels_of(args),
                }
            })
            .collect()
    }

    /// The post types (or taxonomies) of the registrations that are not WordPress's own, in their order, with the
    /// defaults of `register_post_type()`/`register_taxonomy()` for their flags.
    pub fn registered_types(&self, taxonomy: bool) -> Vec<RegisteredType> {
        let flag = |args: &Lit, key: &str, default: bool| match args.get(key) {
            None | Some(Lit::Null) => default,
            Some(value) => value.truthy().unwrap_or(true),
        };
        self.registrations
            .iter()
            .filter(|r| r.taxonomy == taxonomy && if taxonomy { !BUILT_IN_TAXONOMIES.contains(&r.name.as_str()) } else { !BUILT_IN_POST_TYPES.contains(&r.name.as_str()) && !is_internal_post_type(&r.name) })
            .map(|r| {
                let args = &r.args;
                let public = flag(args, "public", taxonomy);
                let labels = labels_of(args);
                let given = |k: &str| labels.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
                let label = given("name").filter(|n| !n.is_empty()).or_else(|| given("label").filter(|l| !l.is_empty()));
                RegisteredType {
                    name: r.name.clone(),
                    hierarchical: flag(args, "hierarchical", false),
                    public,
                    publicly_queryable: flag(args, "publicly_queryable", public),
                    show_in_nav_menus: flag(args, "show_in_nav_menus", public),
                    show_in_rest: flag(args, "show_in_rest", false),
                    singular: given("singular_name").or_else(|| label.clone()),
                    label,
                }
            })
            .collect()
    }

    /// `get_taxonomies( [ '_builtin' => false ], 'objects' )` of the registrations, in their order.
    pub fn admin_taxonomies(&self) -> Vec<AdminTaxonomy> {
        let flag = |args: &Lit, key: &str, default: bool| match args.get(key) {
            None | Some(Lit::Null) => default,
            Some(value) => value.truthy().unwrap_or(true),
        };
        self.registrations
            .iter()
            .filter(|r| r.taxonomy && !BUILT_IN_TAXONOMIES.contains(&r.name.as_str()))
            .map(|r| {
                let public = flag(&r.args, "public", true);
                let show_ui = flag(&r.args, "show_ui", public);
                AdminTaxonomy { name: r.name.clone(), object_types: r.object_types.clone(), show_ui, show_in_menu: flag(&r.args, "show_in_menu", show_ui), meta_box: !matches!(r.args.get("meta_box_cb"), Some(Lit::Bool(false) | Lit::Unknown)), labels: labels_of(&r.args) }
            })
            .collect()
    }
}

/// The name and labels of a registered type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegisteredLabels {
    pub name: String,
    /// `labels->name` (or `label`).
    pub label: Option<String>,
    /// `labels->singular_name`.
    pub singular: Option<String>,
}

/// A post type from its rules: the single pages (`{var}=$matches[1]`) and the archive (`post_type={name}`).
fn detect_post_type(rules: &RewriteRules, name: &str, var: &str) -> Option<PostTypeInfo> {
    let single_only = format!("{var}=$matches[1]");
    let single_paged = format!("{var}=$matches[1]&page=$matches[2]");
    let archive = format!("post_type={name}");
    let mut single: Option<&str> = None;
    let mut archive_slug = None;
    for (pattern, query) in rules.patterns() {
        let q = query_of(query);
        if single.is_none() && (q == single_only || q == single_paged) {
            single = Some(pattern);
        } else if archive_slug.is_none() && q == archive && pattern.ends_with("/?$") {
            archive_slug = Some(pattern.trim_end_matches("/?$").to_owned());
        }
    }
    if single.is_none() && archive_slug.is_none() {
        return None;
    }
    let (slug, hierarchical) = match single {
        Some(pattern) => (literal_prefix(pattern).to_owned(), pattern.contains("(.+?)")),
        None => (archive_slug.clone().unwrap_or_default(), false),
    };
    Some(PostTypeInfo::new(name, slug, archive_slug, hierarchical))
}

/// The text of a rewrite pattern before its first group, without the trailing slash.
fn literal_prefix(pattern: &str) -> &str {
    pattern.split('(').next().unwrap_or("").trim_end_matches('/')
}

/// The query of a rule without `index.php?`.
fn query_of(query: &str) -> &str {
    query.rsplit_once('?').map_or(query, |(_, rest)| rest)
}

impl Types {
    /// Infers the custom types that are in use from the stored rewrite rules.
    pub fn infer(rules: &RewriteRules, used_post_types: &[String], used_taxonomies: &[String], relations: &[(String, String)], with_parents: &[String]) -> Self {
        let mut types = Self::default();
        for name in used_post_types {
            if BUILT_IN_POST_TYPES.contains(&name.as_str()) || is_internal_post_type(name) {
                continue;
            }
            match detect_post_type(rules, name, name) {
                Some(info) => types.post_types.push(info),
                None => types.hidden_post_types.push(name.clone()),
            }
        }
        for name in used_taxonomies {
            if BUILT_IN_TAXONOMIES.contains(&name.as_str()) {
                continue;
            }
            let wanted = format!("{name}=$matches[1]");
            if let Some((pattern, _)) = rules.patterns().find(|(_, query)| query_of(query) == wanted) {
                types.taxonomies.push(TaxonomyInfo {
                    name: name.clone(),
                    slug: literal_prefix(pattern).to_owned(),
                    hierarchical: with_parents.contains(name),
                    rewrite_hierarchical: pattern.contains("(.+?)"),
                    query_var: Some(name.clone()),
                    object_types: relations.iter().filter(|(t, _)| t == name).map(|(_, p)| p.clone()).collect(),
                    label: None,
                    singular: None,
                    show_in_rest: false,
                    rest_base: None,
                    public: true,
                    queryable: true,
                    description: String::new(),
                    registered: false,
                });
            }
        }
        types
    }

    /// The used post types whose rules name another query variable (`'query_var' => 'event'`): only the
    /// registration tells which variable to look for.
    pub fn infer_query_vars(&mut self, rules: &RewriteRules, registrations: &[Registration]) {
        for reg in registrations.iter().filter(|r| !r.taxonomy) {
            if !self.hidden_post_types.contains(&reg.name) {
                continue;
            }
            let Some(var) = query_var(&reg.args, &reg.name).filter(|v| *v != reg.name) else { continue };
            if let Some(info) = detect_post_type(rules, &reg.name, &var) {
                self.hidden_post_types.retain(|h| h != &reg.name);
                self.post_types.push(info);
            }
        }
    }

    /// Adds what the `register_*()` calls say about the inferred types.
    pub fn apply(&mut self, registrations: &[Registration], create_missing: bool) {
        for reg in registrations {
            let args = &reg.args;
            // A computed argument list says nothing; the inferred defaults stay.
            if !matches!(args, Lit::Array(_)) {
                continue;
            }
            // Without rewrite rules (plain permalinks) nothing could be inferred: the registration is all there is. A
            // type that is not publicly queryable has no rules to be found by in any case.
            let queryable = match args.get("publicly_queryable") {
                Some(value) => value.truthy().unwrap_or(true),
                None => args.get("public").is_some_and(|v| v.truthy().unwrap_or(true)),
            };
            if create_missing || (!reg.taxonomy && !queryable) {
                let slug = match args.get("rewrite") {
                    Some(rewrite) => rewrite.get("slug").and_then(Lit::as_str).map(str::to_owned).unwrap_or_else(|| reg.name.clone()),
                    None => reg.name.clone(),
                };
                let hierarchical = args.get("hierarchical").and_then(Lit::truthy).unwrap_or(false);
                if reg.taxonomy {
                    if !self.taxonomies.iter().any(|t| t.name == reg.name) {
                        self.taxonomies.push(TaxonomyInfo {
                            name: reg.name.clone(),
                            slug,
                            hierarchical,
                            rewrite_hierarchical: hierarchical,
                            query_var: Some(reg.name.clone()),
                            object_types: reg.object_types.clone(),
                            label: None,
                            singular: None,
                            show_in_rest: false,
                            rest_base: None,
                            public: true,
                            queryable: true,
                            description: String::new(),
                            registered: false,
                        });
                    }
                } else if !self.post_types.iter().any(|t| t.name == reg.name) && !BUILT_IN_POST_TYPES.contains(&reg.name.as_str()) && !is_internal_post_type(&reg.name) {
                    let archive = match args.get("has_archive") {
                        Some(Lit::Str(archive)) => Some(archive.clone()),
                        Some(value) if value.truthy().unwrap_or(true) => Some(slug.clone()),
                        _ => None,
                    };
                    self.hidden_post_types.retain(|h| h != &reg.name);
                    self.post_types.push(PostTypeInfo::new(&reg.name, slug, archive, hierarchical));
                }
            }
            // A flag that is set but computed counts as set.
            let flag = |key: &str, default: bool| match args.get(key) {
                None => default,
                Some(value) => value.truthy().unwrap_or(true),
            };
            let text = |key: &str| args.get(key).and_then(Lit::as_str).map(str::to_owned);
            let labels = args.get("labels");
            let label = labels.and_then(|l| l.get("name")).and_then(Lit::as_str).map(str::to_owned).or_else(|| text("label"));
            let singular = labels.and_then(|l| l.get("singular_name")).and_then(Lit::as_str).map(str::to_owned);
            let show_in_rest = args.get("show_in_rest").and_then(Lit::truthy).unwrap_or(false);
            let rest_base = text("rest_base");
            match self.registrations.iter_mut().find(|r| r.taxonomy == reg.taxonomy && r.name == reg.name) {
                Some(existing) => *existing = reg.clone(),
                None => self.registrations.push(reg.clone()),
            }
            if !reg.taxonomy && !self.registered_post_types.iter().any(|r| r.name == reg.name) {
                self.registered_post_types.push(RegisteredLabels { name: reg.name.clone(), label: label.clone(), singular: singular.clone() });
            }
            if reg.taxonomy {
                if let Some(t) = self.taxonomies.iter_mut().find(|t| t.name == reg.name) {
                    t.registered = true;
                    t.label = label;
                    t.description = text("description").unwrap_or_default();
                    t.singular = singular;
                    t.show_in_rest = show_in_rest;
                    t.rest_base = rest_base;
                    t.public = flag("public", true);
                    t.query_var = query_var(args, &reg.name);
                    t.hierarchical = flag("hierarchical", t.hierarchical);
                    t.queryable = flag("publicly_queryable", t.public);
                    // The registration's object types in their order, then the ones only the terms show.
                    let mut object_types = reg.object_types.clone();
                    for object_type in &t.object_types {
                        if !object_types.contains(object_type) {
                            object_types.push(object_type.clone());
                        }
                    }
                    t.object_types = object_types;
                }
            } else if let Some(t) = self.post_types.iter_mut().find(|t| t.name == reg.name) {
                t.registered = true;
                t.query_var = query_var(args, &reg.name);
                t.description = text("description").unwrap_or_default();
                t.menu_icon = text("menu_icon");
                t.label = label;
                t.singular = singular;
                t.show_in_rest = show_in_rest;
                t.rest_base = rest_base;
                t.supports = match args.get("supports") {
                    Some(l @ Lit::Array(_)) => Some(l.strings()),
                    Some(Lit::Bool(false)) => Some(Vec::new()),
                    _ => None,
                };
                let public = flag("public", false);
                t.public = public;
                t.exclude_from_search = flag("exclude_from_search", !public);
                t.queryable = flag("publicly_queryable", public);
            }
        }
        // `get_post_types()` and `get_taxonomies()` list the types in the order they were registered; the ones no
        // registration names follow.
        let position = |name: &str, taxonomy: bool| registrations.iter().position(|r| r.taxonomy == taxonomy && r.name == name).unwrap_or(usize::MAX);
        self.post_types.sort_by_key(|t| position(&t.name, false));
        self.taxonomies.sort_by_key(|t| position(&t.name, true));
        self.rewrite_post_types = self.post_types.clone();
        self.rewrite_taxonomies = self.taxonomies.clone();
        // The rewrite rules of the registrations with literal arguments in their order, also of the types that have no
        // posts or terms yet; a computed registration and a type no registration names keep what the rules say.
        let mut rewrite_types = Vec::new();
        let mut covered: Vec<(bool, String)> = Vec::new();
        for reg in &self.registrations {
            let built_in = if reg.taxonomy { BUILT_IN_TAXONOMIES.contains(&reg.name.as_str()) } else { BUILT_IN_POST_TYPES.contains(&reg.name.as_str()) || is_internal_post_type(&reg.name) };
            if built_in || covered.iter().any(|(t, n)| *t == reg.taxonomy && n == &reg.name) {
                continue;
            }
            let found = match (&reg.args, reg.taxonomy) {
                (Lit::Array(_), false) => RewriteType::of_post_type(reg),
                (Lit::Array(_), true) => RewriteType::of_taxonomy(reg),
                (_, false) => self.rewrite_post_types.iter().find(|t| t.name == reg.name).map(RewriteType::inferred_post_type),
                (_, true) => self.rewrite_taxonomies.iter().find(|t| t.name == reg.name).map(RewriteType::inferred_taxonomy),
            };
            covered.push((reg.taxonomy, reg.name.clone()));
            rewrite_types.extend(found);
        }
        for info in &self.rewrite_post_types {
            if !covered.iter().any(|(t, n)| !*t && n == &info.name) {
                rewrite_types.push(RewriteType::inferred_post_type(info));
            }
        }
        for info in &self.rewrite_taxonomies {
            if !covered.iter().any(|(t, n)| *t && n == &info.name) {
                rewrite_types.push(RewriteType::inferred_taxonomy(info));
            }
        }
        self.rewrite_types = Some(rewrite_types);
        // Types that cannot be requested on the frontend have no pages, whatever their rules say.
        let hidden: Vec<String> = self.post_types.iter().filter(|t| !t.queryable).map(|t| t.name.clone()).collect();
        self.hidden_post_types.extend(hidden);
        self.post_types.retain(|t| t.queryable);
        self.taxonomies.retain(|t| t.queryable);
    }

    /// `is_post_type_viewable()`: the built-in public types and the custom ones with pages. Without
    /// rewrite rules (plain permalinks) nothing is known about custom types, and they all count.
    pub fn is_viewable(&self, name: &str, rules_known: bool) -> bool {
        match name {
            "post" | "page" | "attachment" => true,
            _ if is_internal_post_type(name) => false,
            _ => self.post_type(name).is_some() || (!rules_known && !self.hidden_post_types.iter().any(|h| h == name)),
        }
    }

    /// The post types that have rewrite rules, with the ones that have no pages.
    pub fn rewrite_post_types(&self) -> &[PostTypeInfo] {
        if self.rewrite_post_types.is_empty() { &self.post_types } else { &self.rewrite_post_types }
    }

    /// The taxonomies that have rewrite rules, with the ones that have no pages.
    pub fn rewrite_taxonomies(&self) -> &[TaxonomyInfo] {
        if self.rewrite_taxonomies.is_empty() { &self.taxonomies } else { &self.rewrite_taxonomies }
    }

    /// `unregister_post_type()`/`unregister_taxonomy()` after `init`, and with `again` registering the type anew (as a
    /// plugin does when it saves a type): its rewrite rules come behind all others.
    pub fn register_again(&mut self, taxonomy: bool, name: &str, again: Option<RewriteType>) {
        if self.rewrite_types.is_none() {
            self.rewrite_types = Some(self.rewrite_types());
        }
        let Some(types) = self.rewrite_types.as_mut() else { return };
        types.retain(|t| !(t.taxonomy == taxonomy && t.name == name));
        if let Some(mut entry) = again {
            entry.early = false;
            types.push(entry);
        }
    }

    /// What the custom types add to the rewrite rules, in the order WordPress registers them. Without registrations
    /// (before [Types::apply]): the post types the stored rules know, then their taxonomies.
    pub fn rewrite_types(&self) -> Vec<RewriteType> {
        match &self.rewrite_types {
            Some(types) => types.clone(),
            None => self.rewrite_post_types().iter().map(RewriteType::inferred_post_type).chain(self.rewrite_taxonomies().iter().map(RewriteType::inferred_taxonomy)).collect(),
        }
    }

    pub fn post_type(&self, name: &str) -> Option<&PostTypeInfo> {
        self.post_types.iter().find(|t| t.name == name)
    }

    /// `post_type_exists()`: a type WordPress registers itself, or one the site's registrations or rules know.
    pub fn post_type_exists(&self, name: &str) -> bool {
        matches!(name, "post" | "page" | "attachment" | "revision" | "nav_menu_item" | "custom_css" | "customize_changeset" | "oembed_cache" | "user_request" | "wp_block" | "wp_template" | "wp_template_part" | "wp_global_styles" | "wp_navigation" | "wp_font_family" | "wp_font_face")
            || self.any_post_type(name).is_some()
            || self.registrations.iter().any(|r| !r.taxonomy && r.name == name)
    }

    /// A post type of the site, also one that cannot be requested on the frontend (`get_post_type_object()` for the
    /// administration).
    pub fn any_post_type(&self, name: &str) -> Option<&PostTypeInfo> {
        self.post_type(name).or_else(|| self.rewrite_post_types.iter().find(|t| t.name == name))
    }

    pub fn taxonomy(&self, name: &str) -> Option<&TaxonomyInfo> {
        self.taxonomies.iter().find(|t| t.name == name)
    }

    /// `in_array( $name, get_post_types() )`: a built-in, internal or registered post type.
    pub fn is_post_type(&self, name: &str) -> bool {
        BUILT_IN_POST_TYPES.contains(&name) || is_internal_post_type(name) || self.post_type(name).is_some()
    }

    /// `in_array( $name, get_taxonomies() )`: a built-in or registered taxonomy.
    pub fn is_taxonomy(&self, name: &str) -> bool {
        BUILT_IN_TAXONOMIES.contains(&name) || matches!(name, "wp_template_part_area" | "wp_pattern_category") || self.taxonomy(name).is_some()
    }

    /// `get_post_types( [ 'public' => true ] )`: the built-in public types, then the custom ones.
    pub fn public_post_types(&self) -> Vec<&str> {
        let mut all: Vec<&str> = BUILT_IN_POST_TYPES.to_vec();
        all.extend(self.post_types.iter().filter(|t| t.public).map(|t| t.name.as_str()));
        all
    }

    /// The types a search covers (`exclude_from_search` is false): everything public but attachments.
    pub fn searchable_post_types(&self) -> Vec<&str> {
        self.public_post_types().into_iter().filter(|t| *t != "attachment" && !self.post_types.iter().any(|p| p.name == *t && p.exclude_from_search)).collect()
    }
}

/// The `query_var` argument: the name by default, a string as given, `None` for `false`.
fn query_var(args: &Lit, name: &str) -> Option<String> {
    match args.get("query_var") {
        Some(Lit::Str(var)) if !var.is_empty() => Some(var.clone()),
        Some(value) if value.truthy() == Some(false) => None,
        _ => Some(name.to_owned()),
    }
}

/// One `register_post_type()` or `register_taxonomy()` call with its literal arguments.
#[derive(Debug, Clone)]
pub struct Registration {
    pub name: String,
    pub taxonomy: bool,
    /// The argument array; `Lit::Unknown` if it is computed.
    pub args: Lit,
    /// `register_taxonomy()`'s object types.
    pub object_types: Vec<String>,
    /// The priority of the `init` callback that registers the type: below 10 the type's archive rules come before
    /// those of the REST API and the sitemaps.
    pub priority: i32,
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The registration calls in a PHP source, in order. Variables assigned an array earlier in the
/// file (`$labels = array( ... )`) are followed.
pub fn scan_source(php: &str) -> Vec<Registration> {
    let mut out = Vec::new();
    let mut vars = Vars::new();
    let mut i = 0;
    while i < php.len() {
        let rest = &php[i..];
        let Some(c) = rest.chars().next() else { break };
        if c == '$' {
            let name_len = rest[1..].find(|ch: char| !is_identifier_char(ch)).unwrap_or(rest.len() - 1);
            let after = rest[1 + name_len..].trim_start();
            if name_len > 0 && after.starts_with('=') && !after.starts_with("==") && !after.starts_with("=>") {
                let value_at = php.len() - after.len() + 1;
                let mut parser = Parser::new(php, value_at, &vars);
                let value = parser.expr();
                let end = parser.at;
                if matches!(value, Lit::Array(_)) {
                    vars.insert(rest[1..1 + name_len].to_owned(), value);
                }
                i = end.max(i + 1);
                continue;
            }
        } else if c == 'r' && !php[..i].ends_with(is_identifier_char) {
            let mut advanced = false;
            for (call, taxonomy) in [("register_post_type", false), ("register_taxonomy", true)] {
                if let Some(tail) = rest.strip_prefix(call)
                    && tail.trim_start().starts_with('(')
                {
                    let args_at = php.len() - tail.trim_start().len() + 1;
                    let mut parser = Parser::new(php, args_at, &vars);
                    let name = parser.expr();
                    let mut object_types = Vec::new();
                    if taxonomy && parser.eat(",") {
                        object_types = match parser.expr() {
                            Lit::Str(s) => vec![s],
                            l @ Lit::Array(_) => l.strings(),
                            _ => Vec::new(),
                        };
                    }
                    let args = if parser.eat(",") { parser.expr() } else { Lit::Unknown };
                    let end = parser.at;
                    if let Lit::Str(name) = name {
                        out.push(Registration { name, taxonomy, args, object_types, priority: 10 });
                    }
                    i = end.max(i + call.len());
                    advanced = true;
                    break;
                }
            }
            if advanced {
                continue;
            }
        }
        i += c.len_utf8();
    }
    out
}

/// PHP files below `dir`, at most `depth` levels down and a few hundred files.
fn php_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    const LIMIT: usize = 400;
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if out.len() >= LIMIT {
            return;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if depth > 0 && !matches!(name, "node_modules" | "vendor" | "tests" | "languages" | "assets" | ".git") {
                php_files(&path, depth - 1, out);
            }
        } else if name.ends_with(".php") {
            out.push(path);
        }
    }
}

/// The registrations of the must-use plugins, the active plugins and the theme(s).
pub fn scan_registrations(wp_root: &Path, active_plugins: &[String], theme_dirs: &[PathBuf]) -> Vec<Registration> {
    let content = wp_root.join("wp-content");
    let mut files = Vec::new();
    php_files(&content.join("mu-plugins"), 0, &mut files);
    for plugin in active_plugins {
        match plugin.split_once('/') {
            Some((dir, _)) if !dir.is_empty() => php_files(&content.join("plugins").join(dir), 3, &mut files),
            _ => files.push(content.join("plugins").join(plugin)),
        }
    }
    for dir in theme_dirs {
        php_files(dir, 2, &mut files);
    }
    let mut out = Vec::new();
    for file in files {
        // Big files are bundled libraries, not registrations.
        if std::fs::metadata(&file).is_ok_and(|m| m.len() > 1_500_000) {
            continue;
        }
        if let Some(php) = crate::files::read(&file)
            && (php.contains("register_post_type") || php.contains("register_taxonomy"))
        {
            out.extend(scan_source(&php));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpress_phpser::unserialize;

    fn rules(pairs: &[(&str, &str)]) -> RewriteRules {
        let mut raw = format!("a:{}:{{", pairs.len());
        for (k, v) in pairs {
            raw.push_str(&format!("s:{}:\"{k}\";s:{}:\"{v}\";", k.len(), v.len()));
        }
        raw.push('}');
        RewriteRules::from_option(&unserialize(raw.as_bytes()).unwrap())
    }

    #[test]
    fn registrations_are_read_from_source() {
        let php = r#"<?php
        add_action( 'init', function () {
            $labels = array( 'name' => _x( 'Books', 'post type general name', 'd' ), 'singular_name' => __( 'Book', 'd' ) );
            $args = array( 'labels' => $labels, 'public' => true, 'show_in_rest' => true, 'supports' => array( 'title', 'thumbnail' ) );
            register_post_type( 'book', $args );
            register_taxonomy( 'genre', array( 'book' ), array( 'label' => 'Genres', 'hierarchical' => true ) );
            register_post_type( 'broken', computed() );
        } );"#;
        let regs = scan_source(php);
        assert_eq!(regs.len(), 3);
        assert_eq!((regs[0].name.as_str(), regs[0].taxonomy), ("book", false));
        assert_eq!(regs[0].args.get("labels").and_then(|l| l.get("name")).and_then(Lit::as_str), Some("Books"));
        assert_eq!((regs[1].name.as_str(), regs[1].taxonomy, regs[1].object_types.clone()), ("genre", true, vec!["book".to_owned()]));
        assert_eq!(regs[2].args, Lit::Unknown);
    }

    #[test]
    fn registrations_fill_in_labels() {
        let r = rules(&[("buecher/([^/]+)(?:/([0-9]+))?/?$", "index.php?book=$matches[1]&page=$matches[2]")]);
        let mut types = Types::infer(&r, &["book".to_owned()], &[], &[], &[]);
        types.apply(&scan_source("register_post_type( 'book', array( 'label' => 'Buecher', 'public' => true, 'show_in_rest' => true, 'supports' => array( 'title' ) ) );"), false);
        let book = types.post_type("book").unwrap();
        assert_eq!((book.plural_label().as_str(), book.singular_label().as_str()), ("Buecher", "Buecher"));
        assert!(book.show_in_rest && book.supports("title") && !book.supports("editor") && !book.exclude_from_search);
    }

    #[test]
    fn types_come_from_their_rewrite_rules() {
        let r = rules(&[
            ("buecher/?$", "index.php?post_type=book"),
            ("buecher/([^/]+)/embed/?$", "index.php?book=$matches[1]&embed=true"),
            ("buecher/([^/]+)(?:/([0-9]+))?/?$", "index.php?book=$matches[1]&page=$matches[2]"),
            ("termine/(.+?)(?:/([0-9]+))?/?$", "index.php?event=$matches[1]&page=$matches[2]"),
            ("kalender/?$", "index.php?post_type=event"),
            ("genre/(.+?)/?$", "index.php?genre=$matches[1]"),
        ]);
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let types = Types::infer(&r, &names(&["post", "book", "event", "secret", "wp_template"]), &names(&["category", "genre", "hidden"]), &[("genre".to_owned(), "book".to_owned())], &names(&["genre"]));
        assert_eq!(types.post_types.len(), 2);
        let book = types.post_type("book").unwrap();
        assert_eq!((book.slug.as_str(), book.archive_slug.as_deref(), book.hierarchical), ("buecher", Some("buecher"), false));
        let event = types.post_type("event").unwrap();
        assert_eq!((event.slug.as_str(), event.archive_slug.as_deref(), event.hierarchical), ("termine", Some("kalender"), true));
        assert!(types.post_type("secret").is_none());
        let genre = types.taxonomy("genre").unwrap();
        assert_eq!((genre.slug.as_str(), genre.hierarchical, genre.rewrite_hierarchical), ("genre", true, true));
        assert_eq!(genre.object_types, ["book"]);
        assert!(types.taxonomy("hidden").is_none());
        assert_eq!(types.public_post_types(), ["post", "page", "attachment", "book", "event"]);
        assert_eq!(types.searchable_post_types(), ["post", "page", "book", "event"]);
    }
}
