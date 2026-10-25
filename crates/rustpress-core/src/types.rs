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
            let single_only = format!("{name}=$matches[1]");
            let single_paged = format!("{name}=$matches[1]&page=$matches[2]");
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
                types.hidden_post_types.push(name.clone());
                continue;
            }
            let (slug, hierarchical) = match single {
                Some(pattern) => (literal_prefix(pattern).to_owned(), pattern.contains("(.+?)")),
                None => (archive_slug.clone().unwrap_or_default(), false),
            };
            types.post_types.push(PostTypeInfo::new(name, slug, archive_slug, hierarchical));
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
                });
            }
        }
        types
    }

    /// Adds what the `register_*()` calls say about the inferred types.
    pub fn apply(&mut self, registrations: &[Registration], create_missing: bool) {
        for reg in registrations {
            let args = &reg.args;
            // A computed argument list says nothing; the inferred defaults stay.
            if !matches!(args, Lit::Array(_)) {
                continue;
            }
            // Without rewrite rules (plain permalinks) nothing could be inferred: the registration is all there is.
            if create_missing {
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
            if reg.taxonomy {
                if let Some(t) = self.taxonomies.iter_mut().find(|t| t.name == reg.name) {
                    t.label = label;
                    t.description = text("description").unwrap_or_default();
                    t.singular = singular;
                    t.show_in_rest = show_in_rest;
                    t.rest_base = rest_base;
                    t.public = flag("public", true);
                    t.query_var = query_var(args, &reg.name);
                    t.hierarchical = flag("hierarchical", t.hierarchical);
                    t.queryable = flag("publicly_queryable", t.public);
                    for object_type in &reg.object_types {
                        if !t.object_types.contains(object_type) {
                            t.object_types.push(object_type.clone());
                        }
                    }
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
        self.rewrite_post_types = self.post_types.clone();
        self.rewrite_taxonomies = self.taxonomies.clone();
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

    pub fn post_type(&self, name: &str) -> Option<&PostTypeInfo> {
        self.post_types.iter().find(|t| t.name == name)
    }

    pub fn taxonomy(&self, name: &str) -> Option<&TaxonomyInfo> {
        self.taxonomies.iter().find(|t| t.name == name)
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
                        out.push(Registration { name, taxonomy, args, object_types });
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
        if let Ok(php) = std::fs::read_to_string(&file)
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
