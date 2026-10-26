//! The complete HTML document: `<head>` (title, styles, discovery links,
//! script modules) around the rendered template, as `wp_head()` and
//! `wp_footer()` would print it for a block theme.

use std::path::Path;

use rustpress_blocks::{Registry, ThemeAccess};
use rustpress_core::formatting::{capital_p_dangit, esc_attr, esc_html, esc_url};
use rustpress_core::{Queried, QueryResult, Site};
use serde_json::Value;

use crate::Result;
use crate::global_styles::GlobalStyles;
use crate::page::render_body;
use crate::theme::Theme;

pub struct Document {
    pub status: u16,
    pub html: String,
}

/// What the toolbar needs to know about the page.
pub struct ToolbarInfo<'a> {
    /// The slug of the template that rendered the page (`home`, `single`, ...).
    pub template_slug: &'a str,
    pub query: &'a QueryResult,
}

pub type ToolbarFn = dyn Fn(&ToolbarInfo<'_>) -> String + Send + Sync;

/// Parts of the document the caller makes: the toolbar of a signed-in viewer.
#[derive(Clone, Default)]
pub struct Extras {
    pub toolbar: Option<std::sync::Arc<ToolbarFn>>,
}

/// Inline stylesheets up to this many bytes in total; larger files become `<link>`s.
pub(crate) const INLINE_LIMIT: u64 = 40_000;

/// `sanitize_html_class`.
fn html_class(class: &str) -> String {
    rustpress_core::formatting::sanitize_html_class(class)
}

/// Term/slug class with the numeric fallback of `get_body_class`.
fn slug_class(slug: &str, id: u64) -> String {
    let class = html_class(slug);
    if class.is_empty() || class.chars().all(|c| c.is_ascii_digit()) || class.trim_matches('-').is_empty() { id.to_string() } else { class }
}

/// `get_body_class()` for the standard conditionals.
pub async fn body_classes(theme: &Theme, site: &Site, q: &QueryResult) -> Result<Vec<String>> {
    let f = &q.flags;
    let mut c: Vec<String> = Vec::new();
    let mut push = |s: &str| c.push(s.to_owned());
    if site.is_rtl() {
        push("rtl");
    }
    if f.is_front_page {
        push("home");
    }
    if f.is_home {
        push("blog");
    }
    if f.is_page && site.options.u64("wp_page_for_privacy_policy", 0) != 0 && matches!(&q.queried, Queried::Post(p) if p.id == site.options.u64("wp_page_for_privacy_policy", 0)) {
        push("privacy-policy");
    }
    if f.is_archive {
        push("archive");
    }
    if f.is_date {
        push("date");
    }
    if f.is_search {
        push("search");
        push(if q.posts.is_empty() { "search-no-results" } else { "search-results" });
    }
    if f.is_paged {
        push("paged");
    }
    if f.is_attachment {
        push("attachment");
    }
    if f.is_404 {
        push("error404");
    }
    if f.is_singular() {
        if let Queried::Post(post) = &q.queried {
            push("wp-singular");
            let custom = site.db.post_meta(post.id).await?.into_iter().find(|m| m.meta_key.as_deref() == Some("_wp_page_template")).and_then(|m| m.meta_value).filter(|t| !t.is_empty() && t != "default");
            match custom {
                Some(template) => {
                    c.push(format!("{}-template", post.post_type));
                    let stem = template.rsplit('/').next().unwrap_or(&template).trim_end_matches(".php").replace(['.', '/'], "-");
                    c.push(format!("{}-template-{}", post.post_type, html_class(&stem)));
                    c.push(format!("{}-template-{}", post.post_type, html_class(&template.replace('.', "-"))));
                }
                None => c.push(format!("{}-template-default", post.post_type)),
            }
            if f.is_single {
                c.push("single".into());
                c.push(format!("single-{}", slug_class(&post.post_type, post.id)));
                c.push(format!("postid-{}", post.id));
                if post.post_type == "post" {
                    let formats = site.db.terms_for_post(post.id, Some("post_format")).await?;
                    match formats.first().and_then(|t| t.slug.strip_prefix("post-format-")) {
                        Some(format) => c.push(format!("single-format-{}", html_class(format))),
                        None => c.push("single-format-standard".into()),
                    }
                }
            }
            if f.is_attachment {
                c.push(format!("attachmentid-{}", post.id));
                let mime = post.post_mime_type.split_once('/').map_or(post.post_mime_type.as_str(), |(_, rest)| rest);
                c.push(format!("attachment-{mime}"));
            } else if f.is_page {
                c.push("page".into());
                c.push(format!("page-id-{}", post.id));
                let has_children = !site.db.posts(&rustpress_db::PostQuery { post_types: vec!["page".into()], parent: Some(post.id), limit: Some(1), ..Default::default() }).await?.is_empty();
                if has_children {
                    c.push("page-parent".into());
                }
                if post.post_parent != 0 {
                    c.push("page-child".into());
                    c.push(format!("parent-pageid-{}", post.post_parent));
                }
            }
        }
    } else if f.is_archive {
        match &q.queried {
            Queried::PostType(post_type) if f.is_post_type_archive => {
                c.push("post-type-archive".into());
                c.push(format!("post-type-archive-{}", html_class(post_type)));
            }
            Queried::User(user) if f.is_author => {
                c.push("author".into());
                c.push(format!("author-{}", slug_class(&user.user_nicename, user.id)));
                c.push(format!("author-{}", user.id));
            }
            Queried::Term(term) if f.is_category => {
                c.push("category".into());
                c.push(format!("category-{}", slug_class(&term.slug, term.term_id)));
                c.push(format!("category-{}", term.term_id));
            }
            Queried::Term(term) if f.is_tag => {
                c.push("tag".into());
                c.push(format!("tag-{}", slug_class(&term.slug, term.term_id)));
                c.push(format!("tag-{}", term.term_id));
            }
            Queried::Term(term) if f.is_tax => {
                c.push(format!("tax-{}", html_class(&term.taxonomy)));
                c.push(format!("term-{}", slug_class(&term.slug, term.term_id)));
                c.push(format!("term-{}", term.term_id));
            }
            _ => {}
        }
    }
    if let Some(viewer) = &q.viewer {
        c.push("logged-in".into());
        if viewer.show_admin_bar {
            c.push("admin-bar".into());
            c.push("no-customize-support".into());
        }
    }
    if rustpress_blocks::images::custom_logo_id(site).await? != 0 {
        c.push("wp-custom-logo".into());
    }
    if theme.supports_responsive_embeds() {
        c.push("wp-embed-responsive".into());
    }
    let page = q.vars.u64("page").filter(|p| *p >= 2).or_else(|| (q.paged >= 2).then_some(q.paged));
    if let Some(page) = page.filter(|_| !f.is_404) {
        c.push(format!("paged-{page}"));
        let kind = if f.is_single {
            Some("single")
        } else if f.is_page {
            Some("page")
        } else if f.is_category {
            Some("category")
        } else if f.is_tag {
            Some("tag")
        } else if f.is_date {
            Some("date")
        } else if f.is_author {
            Some("author")
        } else if f.is_search {
            Some("search")
        } else if f.is_post_type_archive {
            Some("post-type")
        } else {
            None
        };
        if let Some(kind) = kind {
            c.push(format!("{kind}-paged-{page}"));
        }
    }
    c.push(format!("wp-theme-{}", html_class(&site.settings.template)));
    if site.settings.stylesheet != site.settings.template {
        c.push(format!("wp-child-theme-{}", html_class(&site.settings.stylesheet)));
    }
    let mut seen = Vec::new();
    c.retain(|x| {
        if seen.contains(x) {
            false
        } else {
            seen.push(x.clone());
            true
        }
    });
    Ok(c)
}

/// `wp_get_document_title()`.
pub async fn document_title(site: &Site, q: &QueryResult) -> Result<String> {
    let f = &q.flags;
    let s = &site.settings;
    let mut parts: Vec<String> = Vec::new();
    let mut tagline: Option<String> = None;
    let mut site_name_last = true;
    if f.is_404 {
        parts.push(site.tr("Page not found"));
    } else if f.is_search {
        parts.push(rustpress_core::i18n::sprintf(&site.tr("Search Results for &#8220;%s&#8221;"), &[&esc_html(q.vars.get("s").unwrap_or(""))]));
    } else if f.is_front_page && !f.is_page {
        parts.push(s.blogname.clone());
        if !s.blogdescription.is_empty() {
            tagline = Some(s.blogdescription.clone());
        }
        site_name_last = false;
    } else if f.is_front_page {
        parts.push(s.blogname.clone());
        site_name_last = false;
        if !s.blogdescription.is_empty() {
            tagline = Some(s.blogdescription.clone());
        }
    } else if f.is_singular() || f.is_posts_page {
        if let Some(post) = q.queried_post() {
            parts.push(post.post_title.clone());
        }
    } else if f.is_post_type_archive {
        if let Queried::PostType(post_type) = &q.queried {
            parts.push(site.post_type_label(post_type));
        }
    } else if f.is_category || f.is_tag || f.is_tax {
        if let Queried::Term(term) = &q.queried {
            parts.push(site.term_name(term));
        }
    } else if f.is_author {
        if let Queried::User(user) = &q.queried {
            parts.push(user.display_name.clone());
        }
    } else if f.is_date {
        let ts = q.posts.first().and_then(|p| site.timezone.local_timestamp(&p.post_date));
        let format = if f.is_day { site.options.string("date_format", "F j, Y") } else if f.is_month { site.trx("F Y", "monthly archives date format") } else { site.trx("Y", "yearly archives date format") };
        if let Some(ts) = ts {
            parts.push(site.date(&format, ts));
        }
    }
    let page = q.vars.u64("page").filter(|p| *p >= 2).or_else(|| (q.paged >= 2).then_some(q.paged));
    if let Some(page) = page.filter(|_| !f.is_404) {
        parts.push(rustpress_core::i18n::sprintf(&site.tr("Page %s"), &[&page.to_string()]));
    }
    parts.extend(tagline);
    if site_name_last && !(f.is_front_page && parts.is_empty()) {
        parts.push(s.blogname.clone());
    }
    let title = parts.join(" - ");
    let title = site.texturize(&title);
    Ok(capital_p_dangit(&title, true))
}

pub(crate) fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub(crate) fn wp_version(wp_root: &Path) -> String {
    read(&wp_root.join("wp-includes").join("version.php"))
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("$wp_version = '").and_then(|r| r.split('\'').next().map(str::to_owned))))
        .unwrap_or_default()
}

pub(crate) const EMOJI_CSS: &str = "\n\timg.wp-smiley, img.emoji {\n\t\tdisplay: inline !important;\n\t\tborder: none !important;\n\t\tbox-shadow: none !important;\n\t\theight: 1em !important;\n\t\twidth: 1em !important;\n\t\tmargin: 0 0.07em !important;\n\t\tvertical-align: -0.1em !important;\n\t\tbackground: none !important;\n\t\tpadding: 0 !important;\n\t}";

pub(crate) fn emoji_style() -> String {
    format!("<style id=\"wp-emoji-styles-inline-css\">\n{EMOJI_CSS}\n/*# sourceURL=wp-emoji-styles-inline-css */\n</style>\n")
}

/// A stylesheet that is inlined when small enough, else linked.
struct Sheet {
    id: String,
    /// Absolute file path and public URL.
    file: Option<(std::path::PathBuf, String)>,
    inline: Option<String>,
    /// Inline styles added to the handle (`wp_add_inline_style()`), e.g. by `register_block_style()`.
    extra: Vec<String>,
    /// Right-to-left languages print `<handle>-rtl-css` with this URL instead of the stylesheet's own.
    rtl_link: Option<String>,
    /// Never inlined (it has no `path` data): MediaElement.
    linked: bool,
}

/// The right-to-left variant of a stylesheet (`style.min.css` -> `style-rtl.min.css`): its file and URL,
/// if the language is right-to-left and the file exists.
fn rtl_variant(rtl: bool, path: &std::path::Path, url: &str) -> Option<(std::path::PathBuf, String)> {
    if !rtl {
        return None;
    }
    let with_rtl = |text: &str| -> String {
        if let Some(stem) = text.strip_suffix(".min.css") { format!("{stem}-rtl.min.css") } else if let Some(stem) = text.strip_suffix(".css") { format!("{stem}-rtl.css") } else { text.to_owned() }
    };
    let candidate = std::path::PathBuf::from(with_rtl(&path.to_string_lossy()));
    if !candidate.is_file() {
        return None;
    }
    let swapped = match url.split_once('?') {
        Some((base, query)) => format!("{}?{query}", with_rtl(base)),
        None => with_rtl(url),
    };
    Some((candidate, swapped))
}
pub(crate) fn script_modules(wp_root: &Path, site: &Site, ids: &[String]) -> (String, String, String) {
    // Parse `script-modules-packages.php` (entries of a PHP array literal).
    let text = read(&wp_root.join("wp-includes").join("assets").join("script-modules-packages.php")).unwrap_or_default();
    let base = site.site_url("wp-includes/js/dist/script-modules");
    let mut importmap: Vec<(String, String)> = Vec::new();
    let mut preloads = String::new();
    let mut scripts = String::new();
    let entry = |file: &str| -> Option<(String, Vec<(String, String)>)> {
        let marker = format!("'{file}' => array(");
        let start = text.find(&marker)? + marker.len();
        let end = text[start..].find("\n\t),").map_or(text.len(), |e| start + e);
        let block = &text[start..end];
        let version = block.split("'version' => '").nth(1)?.split('\'').next()?.to_owned();
        let mut deps = Vec::new();
        for chunk in block.split("'id' => '").skip(1) {
            let id = chunk.split('\'').next().unwrap_or_default().to_owned();
            let import = chunk.split("'import' => '").nth(1).and_then(|r| r.split('\'').next()).unwrap_or("static").to_owned();
            deps.push((id, import));
        }
        Some((version, deps))
    };
    let url_for = |id: &str, version: &str| -> String {
        let path = id.trim_start_matches("@wordpress/");
        format!("{base}/{path}.min.js?ver={version}")
    };
    for id in ids {
        let key = format!("{}.js", id.trim_start_matches("@wordpress/"));
        let Some((version, deps)) = entry(&key) else { continue };
        for (dep, import) in &deps {
            let dep_key = format!("{}/index.js", dep.trim_start_matches("@wordpress/"));
            if let Some((dep_version, _)) = entry(&dep_key) {
                let url = url_for(&format!("{}/index", dep.trim_start_matches("@wordpress/")), &dep_version);
                if !importmap.iter().any(|(i, _)| i == dep) {
                    importmap.push((dep.clone(), url.clone()));
                    if import == "static" {
                        preloads.push_str(&format!("<link rel=\"modulepreload\" href=\"{url}\" id=\"{dep}-js-modulepreload\" fetchpriority=\"low\">\n"));
                    }
                }
            }
        }
        let url = url_for(id, &version);
        // Core blocks support client-side navigation (`data-wp-router-options`).
        let router = if id.starts_with("@wordpress/block-library") { "data-wp-router-options=\"{&quot;loadOnClientNavigation&quot;:true}\" " } else { "" };
        scripts.push_str(&format!("<script {router}fetchpriority=\"low\" id=\"{id}-js-module\" src=\"{url}\" type=\"module\"></script>\n"));
    }
    let map = if importmap.is_empty() {
        String::new()
    } else {
        let entries: Vec<String> = importmap.iter().map(|(id, url)| format!("\"{id}\":\"{}\"", url.replace('/', "\\/").replace("\\/", "/"))).collect();
        format!("<script id=\"wp-importmap\" type=\"importmap\">\n{{\"imports\":{{{}}}}}\n</script>\n", entries.join(","))
    };
    (map, preloads, scripts)
}

/// `@font-face` rules for the font families that ship font files.
pub fn font_faces(theme: &Theme) -> String {
    // `WP_Font_Face_Resolver::get_fonts_from_theme_json()`: the families of every origin (theme, then the user's fonts).
    let settings = theme.json.settings_by_origin();
    let Some(origins) = settings.get("typography").and_then(|t| t.get("fontFamilies")) else { return String::new() };
    let families: Vec<Value> = match origins {
        Value::Array(list) => list.clone(),
        Value::Object(map) => ["default", "theme", "custom"].iter().filter_map(|origin| map.get(*origin).and_then(Value::as_array)).flatten().cloned().collect(),
        _ => return String::new(),
    };
    font_faces_of(theme, &families)
}

/// `wp_print_font_faces_from_style_variations()`: the fonts the style variations of the theme add.
pub fn font_faces_from_style_variations(theme: &Theme) -> String {
    let families: Vec<Value> = theme
        .style_variations()
        .iter()
        .filter_map(|variation| variation.pointer("/settings/typography/fontFamilies/theme").and_then(Value::as_array))
        .flatten()
        .cloned()
        .collect();
    font_faces_of(theme, &families)
}

fn font_faces_of(theme: &Theme, families: &[Value]) -> String {
    let base = theme.stylesheet_directory_uri();
    let mut css = String::new();
    for family in families {
        let Some(faces) = family.get("fontFace").and_then(Value::as_array) else { continue };
        for face in faces {
            let name = face.get("fontFamily").and_then(Value::as_str).unwrap_or_default();
            // Names with spaces get quotes (`WP_Font_Face::maybe_add_quotes()`).
            let name = if name.contains(' ') && !name.contains('"') && !name.contains('\'') { format!("\"{name}\"") } else { name.to_owned() };
            let mut rule = format!("font-family:{name};");
            for (key, property) in [("fontStyle", "font-style"), ("fontWeight", "font-weight")] {
                if let Some(v) = face.get(key).and_then(Value::as_str) {
                    rule.push_str(&format!("{property}:{v};"));
                }
            }
            rule.push_str(&format!("font-display:{};", face.get("fontDisplay").and_then(Value::as_str).unwrap_or("fallback")));
            let sources: Vec<String> = face
                .get("src")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).map(|s| {
                    let url = match s.strip_prefix("file:./") {
                        Some(rel) => format!("{base}/{rel}"),
                        None => s.to_owned(),
                    };
                    let format = match url.rsplit('.').next().unwrap_or("woff2") {
                        "ttf" => "truetype",
                        "otf" => "opentype",
                        other => other,
                    };
                    format!("url('{url}') format('{format}')")
                }).collect())
                .unwrap_or_default();
            if sources.is_empty() {
                continue;
            }
            rule.push_str(&format!("src:{};", sources.join(", ")));
            // ont-stretch follows the source in WordPress' output.
            if let Some(v) = face.get("fontStretch").and_then(Value::as_str) {
                rule.push_str(&format!("font-stretch:{v};"));
            }
            css.push_str(&format!("@font-face{{{rule}}}\n"));
        }
    }
    if css.is_empty() { String::new() } else { format!("<style class=\"wp-fonts-local\">\n{css}</style>\n") }
}

pub(crate) async fn discovery_links(site: &Site, q: &QueryResult, version: &str, cpage_override: u64) -> Result<String> {
    let mut out = format!("<link rel=\"https://api.w.org/\" href=\"{}\" />", esc_url(&site.rest_url("/")));
    if let Some(route) = crate::head::queried_rest_route(q) {
        out.push_str(&format!("<link rel=\"alternate\" title=\"JSON\" type=\"application/json\" href=\"{}\" />", esc_url(&site.rest_url(&route))));
    }
    out.push_str(&format!("<link rel=\"EditURI\" type=\"application/rsd+xml\" title=\"RSD\" href=\"{}\" />\n", esc_url(&site.site_url("xmlrpc.php?rsd"))));
    out.push_str(&format!("<meta name=\"generator\" content=\"WordPress {version}\" />\n"));
    if q.flags.is_singular()
        && let Queried::Post(post) = &q.queried {
            if let Some(canonical) = canonical_url(site, q, cpage_override).await? {
                out.push_str(&format!("<link rel=\"canonical\" href=\"{}\" />
", esc_url(&canonical)));
            }
            let settings = &site.settings;
            let shortlink = if post.post_type == "page" && settings.show_on_front == rustpress_core::settings::ShowOnFront::Page && settings.page_on_front == post.id { site.home_url("/") } else { site.home_url(&format!("?p={}", post.id)) };
            out.push_str(&format!("<link rel='shortlink' href='{}' />
", esc_url(&shortlink)));
        }
    Ok(out)
}

/// `wp_get_canonical_url()` of the queried post: a later page of the post or of its comments is
/// part of the address.
pub(crate) async fn canonical_url(site: &Site, q: &QueryResult, cpage_override: u64) -> Result<Option<String>> {
    let Queried::Post(post) = &q.queried else { return Ok(None) };
    // Only a published post has a canonical address.
    if post.post_status != "publish" && post.post_type != "attachment" {
        return Ok(None);
    }
    let mut canonical = site.permalink(post).await?;
    if q.queried_object_id() == post.id {
        let page = q.vars.u64("page").unwrap_or(0);
        if page >= 2 {
            canonical = if site.uses_pretty_permalinks() { format!("{}{}", rustpress_core::urls::trailingslashit(&canonical), site.user_trailingslashit(&page.to_string())) } else { rustpress_core::urls::add_query_arg(&canonical, "page", &page.to_string()) };
        }
        let cpage = q.vars.u64("cpage").filter(|c| *c != 0).unwrap_or(cpage_override);
        if cpage != 0 {
            canonical = rustpress_blocks::comment_pages::pagenum_link(site, &site.permalink(post).await?, cpage, 0);
        }
    }
    Ok(Some(canonical))
}

/// `wp_oembed_add_discovery_links()`: the oEmbed endpoints of a single post or page.
pub(crate) async fn oembed_links(site: &Site, q: &QueryResult) -> Result<String> {
    if !q.flags.is_singular() {
        return Ok(String::new());
    }
    let Queried::Post(post) = &q.queried else { return Ok(String::new()) };
    let permalink = rustpress_core::canonical::php_urlencode(&site.permalink(post).await?);
    let endpoint = if site.settings.permalink_structure.is_empty() { format!("{}?rest_route=%2Foembed%2F1.0%2Fembed&url=", site.home_url("index.php")) } else { format!("{}?url=", site.rest_url("oembed/1.0/embed")) };
    let json = format!("{endpoint}{permalink}");
    let xml = format!("{json}&format=xml");
    Ok(format!(
        "<link rel=\"alternate\" title=\"oEmbed (JSON)\" type=\"application/json+oembed\" href=\"{}\" />
<link rel=\"alternate\" title=\"oEmbed (XML)\" type=\"text/xml+oembed\" href=\"{}\" />
",
        esc_url(&json),
        esc_url(&xml)
    ))
}

/// The `robots` meta tag from `wp_robots()`.
pub(crate) fn robots_meta(site: &Site, q: &QueryResult) -> String {
    let public = site.options.get("blog_public").is_some_and(|v| !v.is_empty() && v != "0");
    let mut directives: Vec<&str> = Vec::new();
    if !public {
        directives.extend(["noindex", "nofollow"]);
    } else {
        if q.flags.is_search {
            directives.extend(["noindex", "follow"]);
        }
        directives.push("max-image-preview:large");
        // `wp_robots_noindex_replytocom()`: reply links are not for search engines.
        // `unapproved` with `moderation-hash` is the preview of a comment that waits.
        let vars = rustpress_core::QueryVars::parse(&q.raw_query);
        if (vars.has("replytocom") || (vars.has("unapproved") && vars.has("moderation-hash"))) && !q.flags.is_search {
            directives.extend(["noindex", "follow"]);
        }
    }
    format!("<meta name='robots' content='{}' />
", directives.join(", "))
}

/// The path part of an absolute URL, without a trailing slash.
fn url_path(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let path = rest.find('/').map_or("", |i| &rest[i..]);
    path.trim_end_matches('/').to_owned()
}

/// `wp_print_speculation_rules()`: prefetch for same-site links, with WordPress' exclusions.
fn speculation_rules(site: &Site, theme: &Theme) -> String {
    // Only with pretty permalinks, and never for logged-in users (there are none here).
    if site.settings.permalink_structure.is_empty() {
        return String::new();
    }
    let mut excluded: Vec<String> = Vec::new();
    let mut add = |pattern: String| {
        if !excluded.contains(&pattern) {
            excluded.push(pattern);
        }
    };
    let site_path = url_path(&site.site_url(""));
    let home_path = url_path(&site.home_url(""));
    add(format!("{site_path}/wp-*.php"));
    add(format!("{site_path}/wp-admin/*"));
    add(format!("{}/*", url_path(&rustpress_blocks::images::upload_baseurl(site))));
    add(format!("{}/*", url_path(&site.site_url("wp-content"))));
    add(format!("{}/*", url_path(&site.site_url("wp-content/plugins"))));
    add(format!("{}/*", url_path(&theme.template_directory_uri())));
    add(format!("{}/*", url_path(&theme.stylesheet_directory_uri())));
    add(format!("{home_path}/*\\?(.+)"));
    let rules = serde_json::json!({
        "prefetch": [{
            "source": "document",
            "where": {"and": [
                {"href_matches": format!("{home_path}/*")},
                {"not": {"href_matches": excluded}},
                {"not": {"selector_matches": "a[rel~=\"nofollow\"]"}},
                {"not": {"selector_matches": ".no-prefetch, .no-prefetch a"}},
            ]},
            "eagerness": "conservative",
        }],
    });
    let json = rules.to_string().replace('<', "\\u003C").replace('>', "\\u003E");
    format!("<script type=\"speculationrules\">
{json}
</script>
")
}

/// `_print_emoji_detection_script()`: the settings and the loader module.
pub(crate) fn emoji_detection_script(site: &Site, wp_root: &Path, version: &str) -> String {
    let loader_url = site.site_url("wp-includes/js/wp-emoji-loader.min.js");
    let Some(loader) = read(&wp_root.join("wp-includes").join("js").join("wp-emoji-loader.min.js")) else { return String::new() };
    // The CDN paths carry the Unicode emoji version, which WordPress keeps in formatting.php.
    let formatting = read(&wp_root.join("wp-includes").join("formatting.php")).unwrap_or_default();
    let cdn = formatting.split("https://s.w.org/images/core/emoji/").nth(1).and_then(|rest| rest.split('/').next()).unwrap_or("17.0.2").to_owned();
    let settings = serde_json::json!({
        "baseUrl": format!("https://s.w.org/images/core/emoji/{cdn}/72x72/"),
        "ext": ".png",
        "svgUrl": format!("https://s.w.org/images/core/emoji/{cdn}/svg/"),
        "svgExt": ".svg",
        "source": {"concatemoji": site.site_url(&format!("wp-includes/js/wp-emoji-release.min.js?ver={version}"))},
    });
    let json = settings.to_string().replace('<', "\\u003C").replace('>', "\\u003E");
    format!(
        "<script id=\"wp-emoji-settings\" type=\"application/json\">
{json}
</script>
<script type=\"module\">
{}
//# sourceURL={}
</script>
",
        loader.trim_end(),
        esc_url(&loader_url)
    )
}

/// The scripts `wp_enqueue_script( 'wp-mediaelement' )` brings to the footer.
fn mediaelement_scripts(site: &Site, wp_root: &std::path::Path, includes_url: &str, version: &str, vimeo: bool) -> String {
    // The strings of `mejsL10n` are read from WordPress's own script-loader and translated.
    let mut strings = serde_json::Map::new();
    if let Ok(php) = std::fs::read_to_string(wp_root.join("wp-includes").join("script-loader.php"))
        && let Ok(re) = regex::Regex::new(r"'(mejs\.[a-z-]+)'\s*=>\s*__\(\s*'((?:[^'\\]|\\.)*)'\s*\)")
    {
        for c in re.captures_iter(&php) {
            strings.insert(c[1].to_owned(), serde_json::Value::from(site.tr(&c[2].replace("\\'", "'"))));
        }
    }
    let language = site.locale().split(['_', '-']).next().unwrap_or("en").to_lowercase();
    let l10n = rustpress_blocks::php_json::encode(&serde_json::json!({ "language": language, "strings": strings })).replace('<', "\\u003C").replace('>', "\\u003E");
    let settings = rustpress_blocks::php_json::encode(&serde_json::json!({
        "pluginPath": "/wp-includes/js/mediaelement/", "classPrefix": "mejs-", "stretching": "responsive",
        "audioShortcodeLibrary": "mediaelement", "videoShortcodeLibrary": "mediaelement"
    })).replace("\\/", "/");
    let mut out = format!("<script id=\"mediaelement-core-js-before\">\nvar mejsL10n = {};\n//# sourceURL=mediaelement-core-js-before\n</script>\n", l10n.replace("\\/", "/"));
    out.push_str(&format!("<script id=\"mediaelement-core-js\" src=\"{includes_url}/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17\"></script>\n"));
    out.push_str(&format!("<script id=\"mediaelement-migrate-js\" src=\"{includes_url}/js/mediaelement/mediaelement-migrate.min.js?ver={version}\"></script>\n"));
    out.push_str(&format!("<script id=\"mediaelement-js-extra\">\nvar _wpmejsSettings = {settings};\n//# sourceURL=mediaelement-js-extra\n</script>\n"));
    out.push_str(&format!("<script id=\"wp-mediaelement-js\" src=\"{includes_url}/js/mediaelement/wp-mediaelement.min.js?ver={version}\"></script>\n"));
    if vimeo {
        out.push_str(&format!("<script id=\"mediaelement-vimeo-js\" src=\"{includes_url}/js/mediaelement/renderers/vimeo.min.js?ver=4.2.17\"></script>\n"));
    }
    out
}

/// Renders a request into a complete HTML document.
pub async fn render_document(theme: &Theme, site: &Site, query: &QueryResult, registry: &Registry) -> Result<Document> {
    render_document_with(theme, site, query, registry, &Extras::default()).await
}

/// [`render_document`] with the toolbar of the signed-in viewer.
pub async fn render_document_with(theme: &Theme, site: &Site, query: &QueryResult, registry: &Registry, extras: &Extras) -> Result<Document> {
    if query.flags.is_embed {
        return crate::embed::render(theme, site, query, registry).await;
    }
    if crate::classic::is_ported(theme) {
        return crate::classic::render_document(theme, site, query, registry).await;
    }
    let rendered = render_body(theme, site, query, registry).await?;
    let state = &rendered.state;
    let wp_root = &theme.wp_root;
    let version = wp_version(wp_root);
    let includes_url = site.site_url("wp-includes");
    let rtl = site.is_rtl();

    // --- stylesheets, in the order WordPress prints them -------------------
    let mut sheets: Vec<Sheet> = Vec::new();
    let mut handles: Vec<String> = Vec::new();
    for name in &state.used_blocks {
        if name == "wp:mediaelement" {
            for handle in ["mediaelement", "wp-mediaelement"] {
                if !handles.iter().any(|h| h == handle) {
                    handles.push(handle.to_owned());
                }
            }
            continue;
        }
        let Some(block_type) = registry.get(name) else { continue };
        for handle in &block_type.styles {
            if !handles.contains(handle) {
                handles.push(handle.clone());
            }
        }
        // `add_theme_support( 'wp-block-styles' )` adds the `theme.css` handle of every core block.
        if theme.wp_block_styles() && let Some(short) = name.strip_prefix("core/") {
            let handle = format!("wp-block-{short}-theme");
            if !handles.contains(&handle) {
                handles.push(handle);
            }
        }
    }
    for handle in &handles {
        if let Some((file, ver)) = match handle.as_str() {
            "mediaelement" => Some(("mediaelementplayer-legacy.min.css", "4.2.17".to_owned())),
            "wp-mediaelement" => Some(("wp-mediaelement.min.css", version.clone())),
            _ => None,
        } {
            let path = wp_root.join("wp-includes").join("js").join("mediaelement").join(file);
            sheets.push(Sheet { id: handle.clone(), file: Some((path, format!("{includes_url}/js/mediaelement/{file}?ver={ver}"))), inline: None, extra: Vec::new(), rtl_link: None, linked: true });
            continue;
        }
        let Some(slug) = handle.strip_prefix("wp-block-") else { continue };
        // `wp-block-group-theme` is the `theme.min.css` of the group block.
        let (slug, file_name) = match slug.strip_suffix("-theme") {
            Some(block) if registry.blocks_dir().join(block).join("theme.min.css").is_file() => (block, "theme.min.css"),
            _ => (slug, "style.min.css"),
        };
        let path = registry.blocks_dir().join(slug).join(file_name);
        if path.is_file() {
            // Block styles are read from the -rtl file, but name the usual URL as their source.
            let url = format!("{includes_url}/blocks/{slug}/{file_name}?ver={version}");
            let variant = rtl_variant(rtl, &path, &url);
            let (path, rtl_link) = match variant {
                Some((rtl_path, rtl_url)) => (rtl_path, Some(rtl_url)),
                None => (path, None),
            };
            sheets.push(Sheet { id: handle.clone(), file: Some((path, url)), inline: None, extra: Vec::new(), rtl_link, linked: false });
            // Stylesheets the theme enqueues for this block (`wp_enqueue_block_style()`) follow it.
            for extra in theme.block_stylesheets().iter().filter(|s| file_name == "style.min.css" && s.block.strip_prefix("core/") == Some(slug) && state.rendered_blocks.contains(&s.block)) {
                if let Some(file) = theme.theme_file(&extra.path) {
                    let url = format!("{}/{}?ver={}", theme.template_directory_uri(), extra.path, theme.version());
                    sheets.push(Sheet { id: extra.handle.clone(), file: Some((file, url)), inline: None, extra: Vec::new(), rtl_link: None, linked: false });
                }
            }
        }
    }
    // With block style variations WordPress queues the emoji style later.
    let has_variations = state.inline_styles.iter().any(|(handle, _)| handle == "block-style-variation-styles");
    if !has_variations {
        sheets.push(Sheet { id: "wp-emoji-styles".into(), file: None, inline: Some(EMOJI_CSS.into()), extra: Vec::new(), rtl_link: None, linked: false });
    }
    let common = wp_root.join("wp-includes").join("css").join("dist").join("block-library").join("common.min.css");
    if common.is_file() {
        let url = format!("{includes_url}/css/dist/block-library/common.min.css?ver={version}");
        let rtl_link = rtl_variant(rtl, &common, &url).map(|(_, rtl_url)| rtl_url);
        sheets.push(Sheet { id: "wp-block-library".into(), file: Some((common, url)), inline: None, extra: Vec::new(), rtl_link, linked: false });
    }
    // The inline CSS of the theme's block styles goes to the block's own stylesheet when it has one (and is
    // dropped if the block is not on the page), else to the common block stylesheet.
    for (block, css) in theme.block_style_inline() {
        let own = block.strip_prefix("core/").map(|short| format!("wp-block-{short}"));
        let target = own.filter(|handle| registry.get(block).is_some_and(|bt| bt.styles.contains(handle))).unwrap_or_else(|| "wp-block-library".to_owned());
        if let Some(sheet) = sheets.iter_mut().find(|s| s.id == target) {
            sheet.extra.push(css.clone());
        }
    }
    // Smallest files first until the inline budget is used up.
    let mut by_size: Vec<(usize, u64)> = sheets.iter().enumerate().filter_map(|(i, s)| s.file.as_ref().filter(|_| !s.linked).and_then(|(p, _)| std::fs::metadata(p).ok()).map(|m| (i, m.len()))).collect();
    // The skip link and the theme's own stylesheet are enqueued later but compete for the same budget.
    const SKIP_LINK: usize = usize::MAX;
    const THEME_STYLE: usize = usize::MAX - 1;
    let skip_link_file = wp_root.join("wp-includes").join("css").join("wp-block-template-skip-link.min.css");
    let theme_style_file = theme.theme_style_handle().and_then(|_| if theme.theme_style_path() { ["style.min.css", "style.css"].iter().map(|f| theme.template_dir().join(f)).find(|p| p.is_file()) } else { Some(theme.template_dir().join("style.css")) });
    if let Ok(meta) = std::fs::metadata(&skip_link_file) {
        by_size.push((SKIP_LINK, meta.len()));
    }
    if let Some(meta) = theme_style_file.as_ref().filter(|_| theme.theme_style_path()).and_then(|p| std::fs::metadata(p).ok()) {
        by_size.push((THEME_STYLE, meta.len()));
    }
    by_size.sort_by_key(|(_, size)| *size);
    let mut budget = 0u64;
    let mut inline_ids: Vec<usize> = Vec::new();
    for (i, size) in by_size {
        if budget + size > INLINE_LIMIT {
            break;
        }
        budget += size;
        inline_ids.push(i);
    }
    let skip_link_inline = inline_ids.contains(&SKIP_LINK);
    let theme_style_inline = inline_ids.contains(&THEME_STYLE);
    inline_ids.retain(|i| *i < THEME_STYLE);
    for i in inline_ids {
        if let Some((path, _)) = &sheets[i].file {
            sheets[i].inline = read(path);
        }
    }

    let mut styles = String::new();
    styles.push_str("<style id=\"wp-img-auto-sizes-contain-inline-css\">\nimg:is([sizes=auto i],[sizes^=\"auto,\" i]){contain-intrinsic-size:3000px 1500px}\n/*# sourceURL=wp-img-auto-sizes-contain-inline-css */\n</style>\n");
    let toolbar_on = query.viewer.as_ref().is_some_and(|v| v.show_admin_bar);
    if toolbar_on {
        styles.push_str(&format!("<link rel='stylesheet' id='dashicons-css' href='{includes_url}/css/dashicons.min.css?ver={version}' media='all' />\n<link rel='stylesheet' id='admin-bar-css' href='{includes_url}/css/admin-bar.min.css?ver={version}' media='all' />\n<style id=\"admin-bar-inline-css\">\n\n\t\t@media screen {{ html {{ margin-top: 32px !important; }} }}\n\t\t@media screen and ( max-width: 782px ) {{ html {{ margin-top: 46px !important; }} }}\n\t\n@media print {{ #wpadminbar {{ display:none; }} }}\n/*# sourceURL=admin-bar-inline-css */\n</style>\n"));
    }
    for sheet in &sheets {
        match (&sheet.inline, &sheet.file) {
            (Some(css), Some((_, url))) => {
                let absolute = url.split('?').next().unwrap_or(url);
                // The common block stylesheet is registered with a site-relative path, block styles with a URL.
                let source = if sheet.id == "wp-block-library" { absolute.strip_prefix(includes_url.trim_end_matches("/wp-includes")).unwrap_or(absolute) } else { absolute };
                if sheet.extra.is_empty() {
                    styles.push_str(&format!("<style id=\"{}-inline-css\">\n{}\n/*# sourceURL={source} */\n</style>\n", sheet.id, css));
                } else {
                    // With further inline styles the sourceURL names the handle instead of the file.
                    styles.push_str(&format!("<style id=\"{id}-inline-css\">\n{}\n{}\n/*# sourceURL={id}-inline-css */\n</style>\n", css, sheet.extra.join("\n"), id = sheet.id));
                }
            }
            (Some(css), None) => styles.push_str(&format!("<style id=\"{}-inline-css\">\n{css}\n/*# sourceURL={}-inline-css */\n</style>\n", sheet.id, sheet.id)),
            (None, Some((_, url))) => {
                styles.push_str(&format!("<link rel='stylesheet' id='{}{}-css' href='{}' media='all' />\n", sheet.id, if sheet.rtl_link.is_some() { "-rtl" } else { "" }, sheet.rtl_link.as_deref().unwrap_or(url)));
                if !sheet.extra.is_empty() {
                    styles.push_str(&format!("<style id=\"{id}-inline-css\">\n{}\n/*# sourceURL={id}-inline-css */\n</style>\n", sheet.extra.join("\n"), id = sheet.id));
                }
            }
            _ => {}
        }
    }
    let global = {
        // Core blocks without a stylesheet are queued under their handle, too: global styles attach to it.
        let mut queued = handles.clone();
        // `wp-block-post-comments` is not registered when WordPress adds the global styles of the blocks.
        queued.retain(|h| h != "wp-block-post-comments");
        for name in state.used_blocks.iter().filter(|n| *n != "core/post-comments") {
            if let Some(short) = name.strip_prefix("core/") {
                // `core/query` drops its style handle unless it is interactive (query.php).
                if short == "query" && !state.script_modules.contains("@wordpress/block-library/query/view") {
                    continue;
                }
                let handle = format!("wp-block-{short}");
                if !queued.contains(&handle) {
                    queued.push(handle);
                }
            }
        }
        let mut chunks = GlobalStyles::new(&theme.json, registry).stylesheet_chunks(Some(&queued));
        let presets = rustpress_blocks::duotone::global_styles_presets(&state.duotone);
        if !presets.is_empty() {
            chunks.push(presets);
        }
        chunks.join("\n")
    };
    styles.push_str(&format!("<style id=\"global-styles-inline-css\">\n{global}\n/*# sourceURL=global-styles-inline-css */\n</style>\n"));
    // Each block instance adds its own inline style; they are printed one per line.
    let variation_css: String = state.inline_styles.iter().filter(|(handle, _)| handle == "block-style-variation-styles").map(|(_, css)| css.as_str()).collect::<Vec<_>>().join("\n");
    if !variation_css.is_empty() {
        styles.push_str(&format!("<style id=\"block-style-variation-styles-inline-css\">\n{variation_css}\n/*# sourceURL=block-style-variation-styles-inline-css */\n</style>\n"));
    }
    if has_variations {
        styles.push_str(&emoji_style());
    }
    // Duotone rules are stored when the styles are enqueued, after the blocks have been rendered.
    let supports = {
        let mut store = state.block_supports_css.clone();
        for (selector, filter) in &state.duotone.css {
            store.add(selector, &[("filter".to_owned(), filter.clone())], None);
        }
        store.compile()
    };
    if !supports.is_empty() {
        styles.push_str(&format!("<style id=\"core-block-supports-inline-css\">\n{supports}\n/*# sourceURL=core-block-supports-inline-css */\n</style>\n"));
    }
    if !skip_link_inline && skip_link_file.is_file() {
        styles.push_str(&format!("<link rel='stylesheet' id='wp-block-template-skip-link-css' href='{includes_url}/css/wp-block-template-skip-link.min.css?ver={version}' media='all' />\n"));
    } else if let Some(css) = read(&skip_link_file) {
        styles.push_str(&format!("<style id=\"wp-block-template-skip-link-inline-css\">\n{}\n/*# sourceURL=/wp-includes/css/wp-block-template-skip-link.min.css */\n</style>\n", css.trim_end()));
    }
    // The theme's style.css is only printed when its functions.php enqueues it.
    let theme_css = theme.theme_style_handle().and_then(|handle| ["style.min.css", "style.css"].iter().find_map(|f| read(&theme.template_dir().join(f)).map(|c| (handle, *f, c))));
    if !theme_style_inline && let Some(handle) = theme.theme_style_handle() && theme_style_file.is_some() {
        let file = theme_style_file.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("style.css");
        styles.push_str(&format!("<link rel='stylesheet' id='{handle}-css' href='{}/{file}?ver={}' media='all' />\n", theme.template_directory_uri(), theme.version()));
    } else if let Some((handle, file, css)) = theme_css {
        styles.push_str(&format!("<style id=\"{handle}-inline-css\">\n{}\n/*# sourceURL={}/{file} */\n</style>\n", css.trim_end(), theme.template_directory_uri()));
    }
    let fonts = font_faces(theme);

    // --- scripts ---------------------------------------------------------
    let modules: Vec<String> = state.script_modules.iter().cloned().collect();
    let (importmap, preloads, module_scripts) = script_modules(wp_root, site, &modules);
    // Signed-in viewers get no speculation rules.
    let speculation = if query.viewer.is_some() { String::new() } else { speculation_rules(site, theme) };
    let mut footer_scripts = format!("{speculation}{}{module_scripts}{}", rustpress_blocks::duotone::footer_svgs(&state.duotone), state.interactivity.data_script().unwrap_or_default());
    // MediaElement: jQuery in the head, the player scripts in the footer.
    let styles = if state.mediaelement { format!("{styles}<script id=\"jquery-core-js\" src=\"{includes_url}/js/jquery/jquery.min.js?ver=3.7.1\"></script>\n<script id=\"jquery-migrate-js\" src=\"{includes_url}/js/jquery/jquery-migrate.min.js?ver=3.4.1\"></script>\n") } else { styles };
    if toolbar_on {
        footer_scripts.push_str(&format!("<script id=\"hoverintent-js-js\" src=\"{includes_url}/js/hoverintent-js.min.js?ver=2.2.1\"></script>\n<script id=\"admin-bar-js\" src=\"{includes_url}/js/admin-bar.min.js?ver={version}\"></script>\n"));
    }
    if state.mediaelement {
        footer_scripts.push_str(&mediaelement_scripts(site, wp_root, &includes_url, &version, state.vimeo));
    }
    if state.scripts.iter().any(|s| s == "comment-reply") {
        footer_scripts.push_str(&format!("<script async data-wp-strategy=\"async\" fetchpriority=\"low\" id=\"comment-reply-js\" src=\"{includes_url}/js/comment-reply.min.js?ver={version}\"></script>\n"));
    }
    footer_scripts.push_str(&emoji_detection_script(site, wp_root, &version));

    // --- head ------------------------------------------------------------
    let mut robots = robots_meta(site, query);
    // `wp_post_preview_js()`: the preview window is named after the post.
    if query.flags.is_preview && let Queried::Post(post) = &query.queried {
        robots.push_str(&format!("<script>\n\t( function() {{\n\t\tvar query = document.location.search;\n\n\t\tif ( query && query.indexOf( 'preview=true' ) !== -1 ) {{\n\t\t\twindow.name = 'wp-preview-{}';\n\t\t}}\n\n\t\tif ( window.addEventListener ) {{\n\t\t\twindow.addEventListener( 'pagehide', function() {{ window.name = ''; }} );\n\t\t}}\n\t}}());\n\t//# sourceURL=wp_post_preview_js\t\n</script>\n", post.id));
    }
    let title = document_title(site, query).await?;
    let lang = site.language_tag();
    let classes = body_classes(theme, site, query).await?;

    let head = format!(
        "<meta charset=\"UTF-8\" />\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n{robots}<title>{}</title>\n{}{}{styles}{}{importmap}{preloads}{fonts}",
        esc_html(&title),
        crate::head::feed_links(site, query).await?,
        oembed_links(site, query).await?,
        discovery_links(site, query, &version, state.cpage_override).await?,
    );
    let bar = match (&extras.toolbar, toolbar_on) {
        (Some(toolbar), true) => toolbar(&ToolbarInfo { template_slug: &rendered.template_slug, query }),
        _ => String::new(),
    };
    let html = format!(
        "<!DOCTYPE html>\n<html {}lang=\"{}\">\n<head>\n\t{head}</head>\n\n<body class=\"{}\">\n{bar}\n{}\n{footer_scripts}</body>\n</html>\n",
        if rtl { "dir=\"rtl\" " } else { "" },
        esc_attr(&lang),
        esc_attr(&classes.join(" ")),
        rendered.html
    );
    Ok(Document { status: query.status(), html })
}
