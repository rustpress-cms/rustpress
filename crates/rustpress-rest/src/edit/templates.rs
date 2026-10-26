//! `wp/v2/templates` and `wp/v2/template-parts` (`WP_REST_Templates_Controller`, `block-template-utils.php`):
//! the files of the theme and the customizations stored as `wp_template` / `wp_template_part` posts.

use rustpress_core::OrLog;
use std::path::{Path, PathBuf};

use rustpress_blocks::parser::{Block, parse_blocks};
use rustpress_blocks::serializer::{PatternRef, resolve_pattern_blocks, serialize_blocks, serialize_with};
use rustpress_content::content::{PostData, TermRef};
use rustpress_db::{Order, OrderBy, PostQuery};
use serde_json::{Value, json};

use super::Args;
use crate::error::RestError;
use crate::params::Params;
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TKind {
    Template,
    Part,
}

impl TKind {
    fn post_type(self) -> &'static str {
        match self {
            Self::Template => "wp_template",
            Self::Part => "wp_template_part",
        }
    }

    fn rest_base(self) -> &'static str {
        match self {
            Self::Template => "templates",
            Self::Part => "template-parts",
        }
    }

    /// The folder of the files, the old name second.
    fn folders(self) -> [&'static str; 2] {
        match self {
            Self::Template => ["templates", "block-templates"],
            Self::Part => ["parts", "block-template-parts"],
        }
    }
}

/// A template or template part the way `WP_Block_Template` holds it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tpl {
    pub id: String,
    pub theme: String,
    pub content: String,
    pub slug: String,
    pub source: String,
    pub origin: Option<String>,
    pub kind: Option<TKind>,
    pub description: String,
    pub title: String,
    pub status: String,
    pub wp_id: u64,
    pub has_theme_file: bool,
    pub is_custom: bool,
    pub author: u64,
    pub modified: Option<String>,
    pub date: Option<String>,
    pub area: Option<String>,
    pub post_types: Option<Vec<String>>,
}

/// A template file of the theme.
#[derive(Debug, Clone)]
struct FileTpl {
    slug: String,
    path: PathBuf,
    title: Option<String>,
    post_types: Option<Vec<String>>,
    area: Option<String>,
}

/// The titles and descriptions of the default template types (`get_default_block_template_types()`).
const DEFAULT_TYPES: &[(&str, &str, &str, &str, &str)] = &[
    ("index", "Index", "Template name", "Used as a fallback template for all pages when a more specific template is not defined.", ""),
    ("home", "Blog Home", "Template name", "Displays the latest posts as either the site homepage or as the \"Posts page\" as defined under reading settings. If it exists, the Front Page template overrides this template when posts are shown on the homepage.", ""),
    ("front-page", "Front Page", "Template name", "Displays your site's homepage, whether it is set to display latest posts or a static page. The Front Page template takes precedence over all templates.", ""),
    ("singular", "Single Entries", "Template name", "Displays any single entry, such as a post or a page. This template will serve as a fallback when a more specific template (e.g. Single Post, Page, or Attachment) cannot be found.", ""),
    ("single", "Single Posts", "Template name", "Displays a single post on your website unless a custom template has been applied to that post or a dedicated template exists.", ""),
    ("page", "Pages", "Template name", "Displays a static page unless a custom template has been applied to that page or a dedicated template exists.", ""),
    ("archive", "All Archives", "Template name", "Displays any archive, including posts by a single author, category, tag, taxonomy, custom post type, and date. This template will serve as a fallback when more specific templates (e.g. Category or Tag) cannot be found.", ""),
    ("author", "Author Archives", "Template name", "Displays a single author's post archive. This template will serve as a fallback when a more specific template (e.g. Author: Admin) cannot be found.", ""),
    ("category", "Category Archives", "Template name", "Displays a post category archive. This template will serve as a fallback when a more specific template (e.g. Category: Recipes) cannot be found.", ""),
    ("taxonomy", "Taxonomy", "Template name", "Displays a custom taxonomy archive. Like categories and tags, taxonomies have terms which you use to classify things. For example: a taxonomy named \"Art\" can have multiple terms, such as \"Modern\" and \"18th Century.\" This template will serve as a fallback when a more specific template (e.g. Taxonomy: Art) cannot be found.", ""),
    ("date", "Date Archives", "Template name", "Displays a post archive when a specific date is visited (e.g., example.com/2023/).", ""),
    ("tag", "Tag Archives", "Template name", "Displays a post tag archive. This template will serve as a fallback when a more specific template (e.g. Tag: Pizza) cannot be found.", ""),
    ("attachment", "Attachment Pages", "", "Displays when a visitor views the dedicated page that exists for any media attachment.", ""),
    ("search", "Search Results", "Template name", "Displays when a visitor performs a search on your website.", ""),
    ("privacy-policy", "Privacy Policy", "", "Displays your site's Privacy Policy page.", ""),
    ("404", "Page: 404", "Template name", "Displays when a visitor views a non-existent page, such as a dead link or a mistyped URL.", ""),
];

/// Post formats: `(slug, name)`.
const POST_FORMATS: &[(&str, &str)] = &[("standard", "Standard"), ("aside", "Aside"), ("chat", "Chat"), ("gallery", "Gallery"), ("link", "Link"), ("image", "Image"), ("quote", "Quote"), ("status", "Status"), ("video", "Video"), ("audio", "Audio")];

/// `get_default_block_template_types()`: `(slug, title, description)`.
pub fn default_template_types(site: &rustpress_core::Site) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = DEFAULT_TYPES
        .iter()
        .map(|(slug, title, context, description, _)| {
            let title = if context.is_empty() { site.tr(title) } else { site.trx(title, context) };
            ((*slug).to_owned(), title, site.tr(description))
        })
        .collect();
    for (slug, name) in POST_FORMATS {
        let name = site.trx(name, "Post format");
        out.push((format!("taxonomy-post_format-post-format-{slug}"), rustpress_core::i18n::sprintf(&site.trx("Post Format: %s", "Template name"), &[&name]), rustpress_core::i18n::sprintf(&site.tr("Displays the %s post format archive."), &[&name])));
    }
    out
}

/// `block_version()`: 1 when the content has blocks.
fn block_version(content: &str) -> u8 {
    u8::from(content.contains("<!-- wp:"))
}

fn style_name(dir: &Path) -> String {
    let css = std::fs::read_to_string(dir.join("style.css")).unwrap_or_default();
    let head: String = css.chars().take(8192).collect();
    head.lines().find_map(|l| l.trim().trim_start_matches('*').trim().strip_prefix("Theme Name:").map(|v| v.trim().to_owned())).unwrap_or_default()
}

/// All `.html` files below `dir`, with the path relative to it and without the extension.
fn html_files(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            html_files(&path, base, out);
        } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("html")) {
            let relative = path.strip_prefix(base).unwrap_or(&path).with_extension("");
            out.push((relative.to_string_lossy().replace('\\', "/"), path));
        }
    }
}

impl Rest<'_> {
    /// The default template types: `(slug, title, description)`.
    pub(crate) fn default_template_types(&self) -> Vec<(String, String, String)> {
        default_template_types(self.site)
    }

    fn template_roots(&self) -> Vec<(String, PathBuf)> {
        let mut roots = vec![(self.theme.stylesheet.clone(), self.theme.stylesheet_dir().to_path_buf())];
        if self.theme.stylesheet != self.theme.template {
            roots.push((self.theme.template.clone(), self.theme.template_dir().to_path_buf()));
        }
        roots
    }

    /// `_get_block_templates_files()`: the files of the child theme win over those of the parent.
    fn file_templates(&self, kind: TKind) -> Vec<FileTpl> {
        let custom = self.theme.json.custom_templates();
        let parts = self.theme.json.template_parts();
        let mut out: Vec<FileTpl> = Vec::new();
        for (_, dir) in self.template_roots() {
            let folder = kind.folders().iter().map(|f| dir.join(f)).find(|p| p.is_dir());
            let Some(folder) = folder else { continue };
            let mut files = Vec::new();
            html_files(&folder, &folder, &mut files);
            for (slug, path) in files {
                if out.iter().any(|f| f.slug == slug) {
                    continue;
                }
                let mut item = FileTpl { slug: slug.clone(), path, title: None, post_types: None, area: None };
                match kind {
                    TKind::Template => {
                        if let Some(info) = custom.iter().find(|c| c.get("name").and_then(Value::as_str) == Some(slug.as_str())) {
                            item.title = info.get("title").and_then(Value::as_str).map(str::to_owned);
                            item.post_types = Some(info.get("postTypes").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_else(|| vec!["page".to_owned()]));
                        }
                    }
                    TKind::Part => {
                        let info = parts.iter().find(|p| p.get("name").and_then(Value::as_str) == Some(slug.as_str()) && p.get("area").is_some());
                        match info {
                            Some(info) => {
                                item.title = info.get("title").and_then(Value::as_str).map(str::to_owned);
                                item.area = Some(info.get("area").and_then(Value::as_str).unwrap_or("uncategorized").to_owned());
                            }
                            None => item.area = Some("uncategorized".to_owned()),
                        }
                    }
                }
                out.push(item);
            }
        }
        out
    }

    fn file_template(&self, kind: TKind, slug: &str) -> Option<FileTpl> {
        self.file_templates(kind).into_iter().find(|f| f.slug == slug)
    }

    /// `_inject_theme_attribute_in_template_part_block()` on every block of the markup.
    pub(crate) fn with_theme_attribute(&self, markup: &str) -> String {
        let mut blocks = parse_blocks(markup);
        let theme = self.theme.stylesheet.clone();
        serialize_with(&mut blocks, &|block: &mut Block| {
            if block.name.as_deref() == Some("core/template-part") && !block.attrs.contains_key("theme") {
                block.attrs.insert("theme".into(), Value::String(theme.clone()));
            }
        })
    }

    /// `_build_block_template_result_from_file()`.
    fn tpl_from_file(&self, kind: TKind, file: &FileTpl) -> Tpl {
        let stylesheet = self.theme.stylesheet.clone();
        let defaults = self.default_template_types();
        let raw = std::fs::read_to_string(&file.path).unwrap_or_default();
        // The theme attribute of template parts is added where it is missing.
        let mut blocks = parse_blocks(&raw);
        let theme = stylesheet.clone();
        let content = serialize_with(&mut blocks, &|block: &mut Block| {
            if block.name.as_deref() == Some("core/template-part") && !block.attrs.contains_key("theme") {
                block.attrs.insert("theme".into(), Value::String(theme.clone()));
            }
        });
        let mut tpl = Tpl {
            id: format!("{stylesheet}//{}", file.slug),
            theme: stylesheet,
            content,
            slug: file.slug.clone(),
            source: "theme".into(),
            kind: Some(kind),
            title: self.site.tr(file.title.as_deref().unwrap_or(&file.slug)),
            status: "publish".into(),
            has_theme_file: true,
            is_custom: true,
            ..Default::default()
        };
        if file.title.is_none() {
            tpl.title = file.slug.clone();
        }
        if kind == TKind::Template {
            if let Some((_, title, description)) = defaults.iter().find(|(s, _, _)| *s == file.slug) {
                tpl.description = description.clone();
                tpl.title = title.clone();
                tpl.is_custom = false;
            }
            tpl.post_types = file.post_types.clone();
        } else {
            tpl.area = file.area.clone();
        }
        tpl
    }

    /// `_build_block_template_result_from_post()`.
    async fn tpls_from_db(&self, kind: TKind, wp_id: Option<u64>) -> Result<Vec<Tpl>> {
        let db = &self.site.db;
        let mut query = PostQuery { post_types: vec![kind.post_type().into()], statuses: vec!["publish".into()], order_by: OrderBy::Date, order: Order::Desc, ..Default::default() };
        if let Some(id) = wp_id {
            query.statuses = vec!["auto-draft".into(), "draft".into(), "publish".into()];
            query.ids = vec![id];
        }
        let posts = db.posts(&query).await?;
        let defaults = self.default_template_types();
        let mut out = Vec::new();
        for post in posts {
            let theme_terms = db.terms_for_post(post.id, Some("wp_theme")).await?;
            let Some(theme) = theme_terms.first().map(|t| t.name.clone()) else { continue };
            if theme != self.theme.stylesheet {
                continue;
            }
            let meta = db.post_meta(post.id).await?;
            let meta_of = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone()).unwrap_or_default();
            let file = self.file_template(kind, &post.post_name);
            let has_theme_file = file.is_some();
            let origin = meta_of("origin");
            let mut tpl = Tpl {
                id: format!("{theme}//{}", post.post_name),
                theme,
                content: self.with_theme_attribute(&post.post_content),
                slug: post.post_name.clone(),
                source: "custom".into(),
                origin: if origin.is_empty() { None } else { Some(origin) },
                kind: Some(kind),
                description: post.post_excerpt.clone(),
                title: post.post_title.clone(),
                status: post.post_status.clone(),
                wp_id: post.id,
                has_theme_file,
                is_custom: meta_of("is_wp_suggestion").is_empty(),
                author: post.post_author,
                modified: Some(post.post_modified.clone()),
                date: Some(post.post_date.clone()),
                ..Default::default()
            };
            if kind == TKind::Template {
                if has_theme_file {
                    tpl.post_types = file.as_ref().and_then(|f| f.post_types.clone());
                }
                if defaults.iter().any(|(s, _, _)| *s == tpl.slug) {
                    tpl.is_custom = false;
                }
                // A template without a title gets one from its slug.
                if tpl.description.is_empty() && (tpl.title.is_empty() || tpl.title == tpl.slug) {
                    self.describe_custom_template(&mut tpl).await;
                }
            } else {
                let area = db.terms_for_post(post.id, Some("wp_template_part_area")).await?;
                tpl.area = area.first().map(|t| t.name.clone());
            }
            out.push(tpl);
        }
        Ok(out)
    }

    /// The title and description of a template named `page-about`, `single-post-hello`, `category-news`, ...
    async fn describe_custom_template(&self, tpl: &mut Tpl) {
        let Some((kind, rest)) = tpl.slug.split_once('-') else { return };
        let site = self.site;
        let tr = |s: &str| site.tr(s);
        match kind {
            "category" | "tag" => {
                let taxonomy = if kind == "category" { "category" } else { "post_tag" };
                if let Ok(Some(term)) = site.db.term_by_slug(taxonomy, rest).await {
                    let label = if kind == "category" { rustpress_core::i18n::sprintf(&tr("Category: %s"), &[&term.name]) } else { rustpress_core::i18n::sprintf(&tr("Tag: %s"), &[&term.name]) };
                    tpl.title = label;
                    tpl.description = rustpress_core::i18n::sprintf(&tr("Template for %s"), &[&term.name]);
                } else {
                    tpl.title = if kind == "category" { rustpress_core::i18n::sprintf(&tr("Deleted category: %s"), &[rest]) } else { rustpress_core::i18n::sprintf(&tr("Deleted tag: %s"), &[rest]) };
                }
            }
            "page" => {
                let query = PostQuery { post_types: vec!["page".into()], statuses: vec!["publish".into()], name: Some(rest.to_owned()), limit: Some(1), ..Default::default() };
                if let Ok(posts) = site.db.posts(&query).await
                    && let Some(page) = posts.first()
                {
                    tpl.title = rustpress_core::i18n::sprintf(&site.trx("Page: %s", "Template name"), &[&page.post_title]);
                    tpl.description = rustpress_core::i18n::sprintf(&tr("Template for %s"), &[&page.post_title]);
                } else {
                    tpl.title = rustpress_core::i18n::sprintf(&tr("Not found: %s"), &[rest]);
                }
            }
            _ => {}
        }
    }

    /// `get_block_templates()`.
    pub(crate) async fn block_templates(&self, kind: TKind, wp_id: Option<u64>, area: Option<&str>, post_type: Option<&str>, slugs: &[String]) -> Result<Vec<Tpl>> {
        let mut result: Vec<Tpl> = Vec::new();
        for tpl in self.tpls_from_db(kind, wp_id).await? {
            if !slugs.is_empty() && !slugs.contains(&tpl.slug) {
                continue;
            }
            if kind == TKind::Part && area.is_some_and(|a| tpl.area.as_deref() != Some(a)) {
                continue;
            }
            if post_type.is_some() && !tpl.is_custom {
                continue;
            }
            if let (Some(pt), Some(types)) = (post_type, &tpl.post_types)
                && !types.iter().any(|t| t == pt)
            {
                continue;
            }
            result.push(tpl);
        }
        if wp_id.is_none() {
            let have: Vec<String> = result.iter().map(|t| t.slug.clone()).collect();
            let defaults = self.default_template_types();
            for file in self.file_templates(kind) {
                if have.contains(&file.slug) || (!slugs.is_empty() && !slugs.contains(&file.slug)) {
                    continue;
                }
                if kind == TKind::Part && area.is_some_and(|a| file.area.as_deref() != Some(a)) {
                    continue;
                }
                if let Some(pt) = post_type {
                    let listed = file.post_types.as_ref().is_some_and(|t| t.iter().any(|x| x == pt));
                    let unlisted_custom = file.post_types.is_none() && !defaults.iter().any(|(s, _, _)| *s == file.slug);
                    if !listed && !unlisted_custom {
                        continue;
                    }
                }
                result.push(self.tpl_from_file(kind, &file));
            }
        }
        Ok(result)
    }

    /// `get_block_template( $id )`.
    pub(crate) async fn block_template(&self, kind: TKind, id: &str) -> Result<Option<Tpl>> {
        let Some((theme, slug)) = id.split_once("//") else { return Ok(None) };
        if theme == self.theme.stylesheet {
            let mut found = self.block_templates(kind, None, None, None, &[slug.to_owned()]).await?;
            if let Some(pos) = found.iter().position(|t| t.slug == slug) {
                return Ok(Some(found.swap_remove(pos)));
            }
        }
        Ok(None)
    }

    pub(crate) fn registered_patterns(&self) -> &[rustpress_theme::registered_patterns::PatternEntry] {
        self.patterns.get_or_init(|| {
            let mut all = self.theme.registered_patterns();
            all.extend(self.remote_patterns.get().cloned().unwrap_or_default());
            all
        })
    }

    fn pattern_lookup(&self) -> impl Fn(&str) -> Option<PatternRef> + '_ {
        move |slug: &str| self.registered_patterns().iter().find(|p| p.name == slug).map(|p| PatternRef { title: p.title.clone(), description: p.description.clone(), categories: p.categories.clone(), content: p.content.clone() })
    }

    /// The content with its `core/pattern` blocks resolved.
    pub(crate) fn resolved_content(&self, content: &str) -> String {
        let blocks = resolve_pattern_blocks(parse_blocks(content), &self.pattern_lookup());
        serialize_blocks(&blocks)
    }

    fn original_source(&self, tpl: &Tpl) -> &'static str {
        if tpl.has_theme_file && (tpl.origin.as_deref() == Some("theme") || (tpl.origin.is_none() && matches!(tpl.source.as_str(), "theme" | "custom"))) {
            return "theme";
        }
        if tpl.origin.as_deref() == Some("plugin") {
            return "plugin";
        }
        if !tpl.has_theme_file && tpl.source == "custom" && tpl.author == 0 {
            return "site";
        }
        "user"
    }

    async fn author_text(&self, tpl: &Tpl) -> String {
        match self.original_source(tpl) {
            "theme" => {
                let name = style_name(&self.template_roots().into_iter().find(|(s, _)| *s == tpl.theme).map(|(_, d)| d).unwrap_or_default());
                if name.is_empty() { tpl.theme.clone() } else { name }
            }
            "plugin" => tpl.theme.clone(),
            "site" => self.site.options.string("blogname", ""),
            _ => match self.site.db.user(tpl.author).await {
                Ok(Some(user)) => user.display_name,
                _ => self.site.tr("Unknown author"),
            },
        }
    }

    /// `WP_REST_Templates_Controller::prepare_item_for_response()`.
    pub(crate) async fn tpl_json(&self, tpl: &Tpl, edit: bool) -> Value {
        let kind = tpl.kind.unwrap_or(TKind::Template);
        let content = self.resolved_content(&tpl.content);
        let mut data = serde_json::Map::new();
        data.insert("id".into(), json!(tpl.id));
        data.insert("theme".into(), json!(tpl.theme));
        let mut content_obj = serde_json::Map::new();
        content_obj.insert("raw".into(), json!(content));
        if edit {
            content_obj.insert("block_version".into(), json!(block_version(&content)));
        }
        data.insert("content".into(), Value::Object(content_obj));
        data.insert("slug".into(), json!(tpl.slug));
        data.insert("source".into(), json!(tpl.source));
        data.insert("origin".into(), tpl.origin.clone().map_or(Value::Null, Value::from));
        data.insert("type".into(), json!(kind.post_type()));
        data.insert("description".into(), json!(tpl.description));
        let rendered = if tpl.wp_id != 0 { self.site.texturize(&tpl.title) } else { tpl.title.clone() };
        data.insert("title".into(), json!({"raw": tpl.title, "rendered": rendered}));
        data.insert("status".into(), json!(tpl.status));
        data.insert("wp_id".into(), json!(tpl.wp_id));
        data.insert("has_theme_file".into(), json!(tpl.has_theme_file));
        if kind == TKind::Template {
            data.insert("is_custom".into(), json!(tpl.is_custom));
        }
        data.insert("author".into(), json!(tpl.author));
        if kind == TKind::Part {
            data.insert("area".into(), json!(tpl.area.clone().unwrap_or_default()));
        }
        let date = |v: &Option<String>| v.as_ref().filter(|d| !d.starts_with("0000")).map_or(Value::Null, |d| Value::from(d.replace(' ', "T")));
        data.insert("modified".into(), date(&tpl.modified));
        data.insert("date".into(), date(&tpl.date));
        data.insert("author_text".into(), json!(self.author_text(tpl).await));
        data.insert("original_source".into(), json!(self.original_source(tpl)));

        let mut links = Links::new();
        let self_href = self.rest_url(&format!("wp/v2/{}/{}", kind.rest_base(), tpl.id));
        let mut self_link = serde_json::Map::new();
        self_link.insert("href".into(), json!(self_href));
        self_link.insert("targetHints".into(), json!({"allow": if self.can("edit_theme_options") { json!(["GET", "POST", "PUT", "PATCH", "DELETE"]) } else { json!(["GET"]) }}));
        links.push_raw("self", Value::Object(self_link));
        links.add("collection", vec![], self.rest_url(&format!("wp/v2/{}", kind.rest_base())));
        links.add("about", vec![], self.rest_url(&format!("wp/v2/types/{}", kind.post_type())));
        if tpl.wp_id != 0 {
            let revisions = self.site.db.posts(&PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(tpl.wp_id), order_by: OrderBy::Id, order: Order::Desc, ..Default::default() }).await.unwrap_or_default();
            links.add("version-history", vec![("count", json!(revisions.len()))], self.rest_url(&format!("wp/v2/{}/{}/revisions", kind.rest_base(), tpl.id)));
            if let Some(latest) = revisions.first() {
                links.add("predecessor-version", vec![("id", json!(latest.id))], self.rest_url(&format!("wp/v2/{}/{}/revisions/{}", kind.rest_base(), tpl.id, latest.id)));
            }
        }
        if self.can("publish_posts") || self.can("edit_theme_options") {
            links.add("https://api.w.org/action-publish", vec![], self_href.clone());
        }
        if self.can("unfiltered_html") {
            links.add("https://api.w.org/action-unfiltered-html", vec![], self_href.clone());
        }
        let mut links_value = links.finish();
        // The action links are written as `wp:action-...` in the answer.
        if let Value::Object(map) = &mut links_value {
            let renamed: Vec<(String, Value)> = map.iter().filter(|(k, _)| k.starts_with("https://api.w.org/")).map(|(k, v)| (k.clone(), v.clone())).collect();
            for (key, value) in renamed {
                map.shift_remove(&key);
                map.insert(key.replace("https://api.w.org/", "wp:"), value);
            }
            if map.keys().any(|k| k.starts_with("wp:")) && !map.contains_key("curies") {
                map.insert("curies".into(), json!([{"name": "wp", "href": "https://api.w.org/{rel}", "templated": true}]));
            }
        }
        data.insert("_links".into(), links_value);
        Value::Object(data)
    }

    /// `get_template_hierarchy()`.
    fn template_hierarchy(&self, slug: &str, is_custom: bool, prefix: &str) -> Vec<String> {
        if slug == "index" {
            return vec!["index".into()];
        }
        if is_custom {
            return vec!["page".into(), "singular".into(), "index".into()];
        }
        if slug == "front-page" {
            return vec!["front-page".into(), "home".into(), "index".into()];
        }
        let mut hierarchy = vec![slug.to_owned()];
        if !prefix.is_empty() {
            let kind = prefix.split('-').next().unwrap_or_default().to_owned();
            if prefix != slug && prefix != kind {
                hierarchy.push(prefix.to_owned());
            }
            if slug != kind {
                hierarchy.push(kind);
            }
        } else if let Some(head) = ["author", "category", "archive", "tag", "page"].iter().find(|h| slug.strip_prefix(&format!("{h}-")).is_some_and(|r| !r.is_empty())) {
            hierarchy.push((*head).to_owned());
        } else if let Some((kind, remaining)) = slug.split_once('-').filter(|(k, r)| matches!(*k, "taxonomy" | "single") && !r.is_empty()) {
            let items: Vec<String> = if kind == "single" {
                let mut v = vec!["post".to_owned(), "page".to_owned(), "attachment".to_owned()];
                v.extend(self.site.types.post_types.iter().map(|t| t.name.clone()));
                v
            } else {
                let mut v = vec!["category".to_owned(), "post_tag".to_owned()];
                v.extend(self.site.types.taxonomies.iter().map(|t| t.name.clone()));
                v
            };
            for item in items {
                if !remaining.starts_with(&item) {
                    continue;
                }
                if remaining == item {
                    hierarchy.push(kind.to_owned());
                    break;
                }
                if remaining.len() > item.len() + 1 {
                    hierarchy.push(format!("{kind}-{item}"));
                    hierarchy.push(kind.to_owned());
                    break;
                }
            }
        }
        if ["author", "taxonomy", "category", "tag"].iter().any(|p| slug.starts_with(p)) || slug == "date" {
            hierarchy.push("archive".into());
        }
        if slug == "attachment" {
            hierarchy.push("single".into());
        }
        if slug.starts_with("single") || slug.starts_with("page") || slug == "attachment" {
            hierarchy.push("singular".into());
        }
        hierarchy.push("index".into());
        hierarchy
    }

    /// `get_template_fallback()`.
    async fn template_lookup(&self, args: &Args, params: &Params) -> Result<Outcome> {
        let _ = params;
        let slug = args.str("slug").unwrap_or_default();
        let is_custom = args.bool("is_custom").unwrap_or(false);
        let prefix = args.str("template_prefix").unwrap_or_default();
        let mut hierarchy = self.template_hierarchy(&slug, is_custom, &prefix);
        let mut found: Option<Tpl>;
        loop {
            let slugs: Vec<String> = hierarchy.clone();
            let mut templates = self.block_templates(TKind::Template, None, None, None, &slugs).await?;
            templates.sort_by_key(|t| slugs.iter().position(|s| *s == t.slug).unwrap_or(usize::MAX));
            found = templates.into_iter().next();
            hierarchy.remove(0);
            if hierarchy.is_empty() || found.as_ref().is_some_and(|t| !t.content.is_empty()) {
                break;
            }
        }
        let body = match found {
            Some(tpl) => self.tpl_json(&tpl, args.str("context").as_deref() == Some("edit")).await,
            None => json!({}),
        };
        Ok(Ok(Resp::item(body)))
    }

    fn templates_permission(&self) -> std::result::Result<(), RestError> {
        if self.can("edit_posts") || self.can("edit_pages") {
            Ok(())
        } else {
            Err(self.forbidden("rest_cannot_manage_templates", "Sorry, you are not allowed to access the templates on this site."))
        }
    }

    fn templates_manage_permission(&self) -> std::result::Result<(), RestError> {
        if self.can("edit_theme_options") {
            Ok(())
        } else {
            Err(self.forbidden("rest_cannot_manage_templates", "Sorry, you are not allowed to access the templates on this site."))
        }
    }

    /// The routes below `wp/v2/templates` and `wp/v2/template-parts`.
    pub(crate) async fn templates_route(&self, kind: TKind, segments: &[&str], method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let edit = params.get("context") == Some("edit");
        let rest = &segments[3.min(segments.len())..];
        // The list.
        if rest.is_empty() {
            return match method {
                "GET" => {
                    if let Err(e) = self.templates_permission() {
                        return Ok(Err(e));
                    }
                    let slugs = args.list("slug").unwrap_or_default();
                    let templates = self.block_templates(kind, args.u64("wp_id"), args.str("area").as_deref(), args.str("post_type").as_deref(), &slugs).await?;
                    let mut items = Vec::new();
                    for tpl in &templates {
                        items.push(self.tpl_json(tpl, edit).await);
                    }
                    Ok(Ok(Resp::item(Value::Array(items))))
                }
                "POST" => self.templates_write(kind, None, args).await,
                _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
            };
        }
        if rest == ["lookup"] && method == "GET" {
            // The required parameter is checked before the permission.
            if args.str("slug").is_none() {
                return Ok(Err(RestError::missing_params(&["slug"])));
            }
            if let Err(e) = self.templates_permission() {
                return Ok(Err(e));
            }
            if kind == TKind::Part {
                let slug = args.str("slug").unwrap_or_default();
                let found = self.block_templates(TKind::Part, None, None, None, std::slice::from_ref(&slug)).await?.into_iter().next();
                return Ok(Ok(Resp::item(match found {
                    Some(tpl) => self.tpl_json(&tpl, edit).await,
                    None => json!({}),
                })));
            }
            return self.template_lookup(args, params).await;
        }
        // `theme//slug` (with the second slash written or not), possibly followed by `/revisions` or `/autosaves`.
        let joined = rest.join("/");
        let (id, sub) = match joined.rsplit_once('/') {
            Some((head, tail)) if matches!(tail, "revisions" | "autosaves") => (head.to_owned(), Some(tail.to_owned())),
            _ => (joined.clone(), None),
        };
        let id = if id.contains("//") { id } else { id.replacen('/', "//", 1) };
        if sub.is_some() {
            if let Err(e) = self.templates_permission() {
                return Ok(Err(e));
            }
            let mut resp = Resp::item(Value::Array(Vec::new()));
            resp.total = Some(0);
            resp.total_pages = Some(0);
            return Ok(Ok(resp));
        }
        match method {
            "GET" => {
                if let Err(e) = self.templates_permission() {
                    return Ok(Err(e));
                }
                let tpl = if args.str("source").is_some_and(|s| s == "theme" || s == "plugin") {
                    let slug = id.split_once("//").map(|(_, s)| s.to_owned()).unwrap_or_default();
                    self.file_template(kind, &slug).map(|f| self.tpl_from_file(kind, &f))
                } else {
                    self.block_template(kind, &id).await?
                };
                match tpl {
                    Some(tpl) => Ok(Ok(Resp::item(self.tpl_json(&tpl, edit).await))),
                    None => Ok(Err(RestError::new("rest_template_not_found", "No templates exist with that id.", 404))),
                }
            }
            "POST" | "PUT" | "PATCH" => self.templates_write(kind, Some(&id), args).await,
            "DELETE" => self.templates_delete(kind, &id, args).await,
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    /// `create_item()` and `update_item()`.
    async fn templates_write(&self, kind: TKind, id: Option<&str>, args: &Args) -> Result<Outcome> {
        if let Err(e) = self.templates_manage_permission() {
            return Ok(Err(e));
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let existing = match id {
            Some(id) => match self.block_template(kind, id).await? {
                Some(t) => Some(t),
                None => return Ok(Err(RestError::new("rest_template_not_found", "No templates exist with that id.", 404))),
            },
            None => None,
        };
        // Resetting a customized theme template deletes the stored one.
        if let (Some(tpl), Some("theme")) = (&existing, args.str("source").as_deref()) {
            if tpl.wp_id != 0 {
                content.delete_post(tpl.wp_id, true).await.or_log();
            }
            let id = tpl.id.clone();
            return Ok(match self.block_template(kind, &id).await? {
                Some(fresh) => Ok(Resp::item(self.tpl_json(&fresh, true).await)),
                None => Err(RestError::new("rest_template_not_found", "No templates exist with that id.", 404)),
            });
        }
        let mut data = PostData::default();
        match &existing {
            None => {
                data.post_type = Some(kind.post_type().to_owned());
                data.post_status = Some("publish".into());
                data.tax_input.push(("wp_theme".into(), vec![TermRef::Name(args.str("theme").unwrap_or_else(|| self.theme.stylesheet.clone()))]));
                data.post_name = args.str("slug");
            }
            Some(tpl) if tpl.source != "custom" => {
                data.post_name = Some(tpl.slug.clone());
                data.post_type = Some(kind.post_type().to_owned());
                data.post_status = Some("publish".into());
                data.tax_input.push(("wp_theme".into(), vec![TermRef::Name(tpl.theme.clone())]));
                data.meta_input.push(("origin".into(), tpl.source.clone()));
            }
            Some(tpl) => {
                data.post_name = Some(tpl.slug.clone());
                data.id = Some(tpl.wp_id);
                data.post_status = Some("publish".into());
            }
        }
        let keep = existing.as_ref().filter(|t| t.source != "custom");
        data.post_content = args.raw_text("content").or_else(|| keep.map(|t| t.content.clone()));
        data.post_title = args.raw_text("title").or_else(|| keep.map(|t| t.title.clone()));
        data.post_excerpt = args.str("description").or_else(|| keep.map(|t| t.description.clone()));
        if kind == TKind::Template && let Some(flag) = args.bool("is_wp_suggestion") {
            data.meta_input.push(("is_wp_suggestion".into(), if flag { "1".into() } else { String::new() }));
        }
        if kind == TKind::Part {
            let area = args.str("area").or_else(|| keep.and_then(|t| t.area.clone())).unwrap_or_else(|| "uncategorized".into());
            data.tax_input.push(("wp_template_part_area".into(), vec![TermRef::Name(area)]));
        }
        if let Some(author) = args.u64("author").filter(|a| *a > 0) {
            data.post_author = Some(author);
        }
        let post_id = match content.insert_post(&data).await {
            Ok(id) => id,
            Err(e) => return Ok(Err(RestError::new("rest_template_insert_error", &e.to_string(), 400))),
        };
        let created = self.block_templates(kind, Some(post_id), None, None, &[]).await?;
        let Some(first) = created.first() else { return Ok(Err(RestError::new("rest_template_insert_error", "No templates exist with that id.", 400))) };
        let body = self.tpl_json(first, true).await;
        let mut resp = Resp::item(body);
        if existing.is_none() || existing.as_ref().is_some_and(|t| t.source != "custom") {
            resp.status = Some(if id.is_none() { 201 } else { 200 });
        }
        if id.is_none() {
            resp.location = Some(self.rest_url(&format!("wp/v2/{}/{}", kind.rest_base(), first.id)));
        }
        Ok(Ok(resp))
    }

    async fn templates_delete(&self, kind: TKind, id: &str, args: &Args) -> Result<Outcome> {
        if let Err(e) = self.templates_manage_permission() {
            return Ok(Err(e));
        }
        let Some(tpl) = self.block_template(kind, id).await? else { return Ok(Err(RestError::new("rest_template_not_found", "No templates exist with that id.", 404))) };
        if tpl.source != "custom" {
            return Ok(Err(RestError::new("rest_invalid_template", "Templates based on theme files can't be removed.", 400)));
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let force = args.bool("force").unwrap_or(false);
        if force {
            let previous = self.tpl_json(&tpl, true).await;
            if !content.delete_post(tpl.wp_id, true).await.unwrap_or(false) {
                return Ok(Err(RestError::new("rest_cannot_delete", "The template cannot be deleted.", 500)));
            }
            return Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))));
        }
        if tpl.status == "trash" {
            return Ok(Err(RestError::new("rest_template_already_trashed", "The template has already been deleted.", 410)));
        }
        if !content.trash_post(tpl.wp_id).await.unwrap_or(false) {
            return Ok(Err(RestError::new("rest_cannot_delete", "The template cannot be deleted.", 500)));
        }
        let mut trashed = tpl.clone();
        trashed.status = "trash".into();
        Ok(Ok(Resp::item(self.tpl_json(&trashed, true).await)))
    }
}
