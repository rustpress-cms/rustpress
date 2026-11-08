//! The complete HTML document: `<head>` (title, styles, discovery links,
//! script modules) around the rendered template, as `wp_head()` and
//! `wp_footer()` would print it for a block theme.

use std::path::Path;

use rustpress_blocks::{Registry, ThemeAccess};
use rustpress_core::formatting::{capital_p_dangit, esc_attr, esc_html, esc_url};
use rustpress_core::native::Robot;
use rustpress_core::{Queried, QueryResult, Site};
use serde_json::Value;

use crate::Result;
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
    // `custom-background` for the ported themes that support it with a default color.
    if crate::classic::is_twentyone(theme) && crate::classic::twentyone::has_custom_background(&crate::classic::menu::theme_mod(site, &theme.stylesheet)) {
        c.push("custom-background".into());
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
    // `array_filter()`: an empty part (a post without a title) is left out.
    parts.retain(|part| !part.is_empty());
    let title = parts.join(" - ");
    let title = site.texturize(&title);
    Ok(capital_p_dangit(&title, true))
}

/// A file of WordPress or the theme, from the process-wide cache.
pub(crate) fn read(path: &Path) -> Option<String> {
    rustpress_core::files::read(path).map(|text| text.to_string())
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
pub(crate) fn rtl_variant(rtl: bool, path: &std::path::Path, url: &str) -> Option<(std::path::PathBuf, String)> {
    if !rtl {
        return None;
    }
    let with_rtl = |text: &str| -> String {
        if let Some(stem) = text.strip_suffix(".min.css") { format!("{stem}-rtl.min.css") } else if let Some(stem) = text.strip_suffix(".css") { format!("{stem}-rtl.css") } else { text.to_owned() }
    };
    let candidate = std::path::PathBuf::from(with_rtl(&path.to_string_lossy()));
    if !rustpress_core::files::is_file(&candidate) {
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

pub(crate) async fn discovery_links(site: &Site, q: &QueryResult, version: &str, cpage_override: u64, rel_canonical: bool, locale_stylesheet: &str) -> Result<String> {
    let mut out = format!("<link rel=\"https://api.w.org/\" href=\"{}\" />", esc_url(&site.rest_url("/")));
    if let Some(route) = crate::head::queried_rest_route(site, q) {
        out.push_str(&format!("<link rel=\"alternate\" title=\"JSON\" type=\"application/json\" href=\"{}\" />", esc_url(&site.rest_url(&route))));
    }
    out.push_str(&format!("<link rel=\"EditURI\" type=\"application/rsd+xml\" title=\"RSD\" href=\"{}\" />\n", esc_url(&site.site_url("xmlrpc.php?rsd"))));
    out.push_str(locale_stylesheet);
    out.push_str(&format!("<meta name=\"generator\" content=\"WordPress {version}\" />\n"));
    if q.flags.is_singular()
        && let Queried::Post(_) = &q.queried {
            if rel_canonical && let Some(canonical) = canonical_url(site, q, cpage_override).await? {
                out.push_str(&format!("<link rel=\"canonical\" href=\"{}\" />
", esc_url(&canonical)));
            }
            if let Some(shortlink) = crate::head::shortlink(site, q) {
                out.push_str(&format!("<link rel='shortlink' href='{}' />
", esc_url(&shortlink)));
            }
        }
    Ok(out)
}

/// `locale_stylesheet()`: `<locale>.css` of the theme, or its `rtl.css` for a language that runs from right to left.
pub(crate) fn locale_stylesheet(theme: &Theme, site: &Site) -> String {
    let dir = theme.stylesheet_dir();
    let locale = site.locale();
    let file = if rustpress_core::files::is_file(&dir.join(format!("{locale}.css"))) {
        format!("{locale}.css")
    } else if site.is_rtl() && rustpress_core::files::is_file(&dir.join("rtl.css")) {
        "rtl.css".to_owned()
    } else {
        return String::new();
    };
    format!("<link rel=\"stylesheet\" href=\"{}/{file}\" media=\"screen\" />", theme.stylesheet_directory_uri())
}

/// The styles of the toolbar: `dashicons`, `admin-bar` and the bump of `_admin_bar_bump_cb()`.
pub(crate) fn toolbar_styles(includes_url: &str, version: &str, rtl: bool) -> String {
    let bar = if rtl { "-rtl" } else { "" };
    format!("<link rel='stylesheet' id='dashicons-css' href='{includes_url}/css/dashicons.min.css?ver={version}' media='all' />\n<link rel='stylesheet' id='admin-bar{bar}-css' href='{includes_url}/css/admin-bar{bar}.min.css?ver={version}' media='all' />\n<style id=\"admin-bar-inline-css\">\n\n\t\t@media screen {{ html {{ margin-top: 32px !important; }} }}\n\t\t@media screen and ( max-width: 782px ) {{ html {{ margin-top: 46px !important; }} }}\n\t\n@media print {{ #wpadminbar {{ display:none; }} }}\n/*# sourceURL=admin-bar-inline-css */\n</style>\n")
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

/// The directives of `wp_robots()` after core's filters (`wp_robots_noindex()`, `wp_robots_noindex_search()`,
/// `wp_robots_max_image_preview_large()`, `wp_robots_noindex_replytocom()`).
pub(crate) fn core_robots(site: &Site, q: &QueryResult) -> Vec<(String, Robot)> {
    fn set(robots: &mut Vec<(String, Robot)>, name: &str, value: Robot) {
        match robots.iter_mut().find(|(n, _)| n == name) {
            Some((_, v)) => *v = value,
            None => robots.push((name.to_owned(), value)),
        }
    }
    let public = site.options.get("blog_public").is_some_and(|v| !v.is_empty() && v != "0");
    // `wp_robots_no_robots()`.
    let no_robots = |robots: &mut Vec<(String, Robot)>| {
        set(robots, "noindex", Robot::Flag(true));
        set(robots, if public { "follow" } else { "nofollow" }, Robot::Flag(true));
    };
    let mut robots = Vec::new();
    if !public {
        no_robots(&mut robots);
    }
    // `wp_robots_noindex_embeds()` and `wp_robots_noindex_search()`.
    if q.flags.is_embed {
        no_robots(&mut robots);
    }
    if q.flags.is_search {
        no_robots(&mut robots);
    }
    if public {
        set(&mut robots, "max-image-preview", Robot::Value("large".into()));
        // `wp_robots_noindex_replytocom()`: reply links are not for search engines.
        // `unapproved` with `moderation-hash` is the preview of a comment that waits.
        let vars = rustpress_core::QueryVars::parse(&q.raw_query);
        if vars.has("replytocom") || (vars.has("unapproved") && vars.has("moderation-hash")) {
            no_robots(&mut robots);
        }
    }
    // `WP_Customize_Manager::customize_preview_init()` adds `wp_robots_no_robots()` last.
    if q.customize.is_some() {
        no_robots(&mut robots);
    }
    robots
}

/// `$GLOBALS['post']` after `wp_reset_postdata()`: the queried post of a singular page, else the first post of the
/// main query.
pub(crate) fn global_post_id(q: &QueryResult) -> Option<u64> {
    match &q.queried {
        Queried::Post(p) if q.flags.is_singular() => Some(p.id),
        _ => q.posts.first().map(|p| p.id),
    }
}

/// The path part of an absolute URL, without a trailing slash.
fn url_path(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let path = rest.find('/').map_or("", |i| &rest[i..]);
    path.trim_end_matches('/').to_owned()
}

/// `wp_print_speculation_rules()`: prefetch for same-site links, with WordPress' exclusions.
pub(crate) fn speculation_rules(site: &Site, theme: &Theme) -> String {
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
pub(crate) fn mediaelement_scripts(site: &Site, wp_root: &std::path::Path, includes_url: &str, version: &str, vimeo: bool, wp_mediaelement: bool) -> String {
    let l10n = mejs_l10n(site, wp_root);
    let settings = rustpress_blocks::php_json::encode(&serde_json::json!({
        "pluginPath": "/wp-includes/js/mediaelement/", "classPrefix": "mejs-", "stretching": "responsive",
        "audioShortcodeLibrary": "mediaelement", "videoShortcodeLibrary": "mediaelement"
    })).replace("\\/", "/");
    let mut out = format!("<script id=\"mediaelement-core-js-before\">\nvar mejsL10n = {l10n};\n//# sourceURL=mediaelement-core-js-before\n</script>\n");
    out.push_str(&format!("<script id=\"mediaelement-core-js\" src=\"{includes_url}/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17\"></script>\n"));
    out.push_str(&format!("<script id=\"mediaelement-migrate-js\" src=\"{includes_url}/js/mediaelement/mediaelement-migrate.min.js?ver={version}\"></script>\n"));
    out.push_str(&format!("<script id=\"mediaelement-js-extra\">\nvar _wpmejsSettings = {settings};\n//# sourceURL=mediaelement-js-extra\n</script>\n"));
    if wp_mediaelement {
        out.push_str(&format!("<script id=\"wp-mediaelement-js\" src=\"{includes_url}/js/mediaelement/wp-mediaelement.min.js?ver={version}\"></script>\n"));
    }
    if vimeo {
        out.push_str(&format!("<script id=\"mediaelement-vimeo-js\" src=\"{includes_url}/js/mediaelement/renderers/vimeo.min.js?ver=4.2.17\"></script>\n"));
    }
    out
}

/// The JSON of `mejsL10n`, which `wp_default_scripts()` adds on `init`: the strings are read from WordPress's own
/// script-loader and translated.
pub(crate) fn mejs_l10n(site: &Site, wp_root: &std::path::Path) -> String {
    let mut strings = serde_json::Map::new();
    if let Some(php) = read(&wp_root.join("wp-includes").join("script-loader.php"))
        && let Some(re) = Some(rustpress_core::formatting::cached_plain_regex(r"'(mejs\.[a-z-]+)'\s*=>\s*__\(\s*'((?:[^'\\]|\\.)*)'\s*\)"))
    {
        for c in re.captures_iter(&php) {
            strings.insert(c[1].to_owned(), serde_json::Value::from(site.tr(&c[2].replace("\\'", "'"))));
        }
    }
    let language = site.locale().split(['_', '-']).next().unwrap_or("en").to_lowercase();
    rustpress_blocks::php_json::encode(&serde_json::json!({ "language": language, "strings": strings })).replace('<', "\\u003C").replace('>', "\\u003E").replace("\\/", "/")
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
        return crate::classic::render_document(theme, site, query, registry, extras).await;
    }
    let started = std::time::Instant::now();
    let rendered = render_body(theme, site, query, registry).await?;
    let body_done = started.elapsed();
    let state = &rendered.state;
    // `wp_head` of the plugins runs after the template was rendered.
    let crate::head::PluginHead { head: plugin_head, blocks: head_blocks } = crate::head::plugin_head(theme, site, query, registry, false, Some(state)).await?;
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
            Some(block) if rustpress_core::files::is_file(&registry.blocks_dir().join(block).join("theme.min.css")) => (block, "theme.min.css"),
            _ => (slug, "style.min.css"),
        };
        let path = registry.blocks_dir().join(slug).join(file_name);
        if rustpress_core::files::is_file(&path) {
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
                    let ver = if extra.theme_version { theme.version() } else { version.clone() };
                    let url = format!("{}/{}?ver={ver}", theme.template_directory_uri(), extra.path);
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
    if rustpress_core::files::is_file(&common) {
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
    let mut by_size: Vec<(usize, u64)> = sheets.iter().enumerate().filter_map(|(i, s)| s.file.as_ref().filter(|_| !s.linked).and_then(|(p, _)| rustpress_core::files::size(p)).map(|len| (i, len))).collect();
    // The skip link and the theme's own stylesheet are enqueued later but compete for the same budget.
    const SKIP_LINK: usize = usize::MAX;
    const THEME_STYLE: usize = usize::MAX - 1;
    let skip_link_file = wp_root.join("wp-includes").join("css").join("wp-block-template-skip-link.min.css");
    let theme_style_file = theme.theme_style_handle().and_then(|_| if theme.theme_style_path() { ["style.min.css", "style.css"].iter().map(|f| theme.template_dir().join(f)).find(|p| rustpress_core::files::is_file(p)) } else { Some(theme.template_dir().join("style.css")) });
    if let Some(len) = rustpress_core::files::size(&skip_link_file) {
        by_size.push((SKIP_LINK, len));
    }
    if let Some(len) = theme_style_file.as_ref().filter(|_| theme.theme_style_path()).and_then(|p| rustpress_core::files::size(p)) {
        by_size.push((THEME_STYLE, len));
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
    let preview = query.customize.as_deref();
    if preview.is_some() {
        styles.push_str(&crate::customize_preview::styles(&includes_url, &version, rtl, toolbar_on));
    } else if toolbar_on {
        styles.push_str(&toolbar_styles(&includes_url, &version, rtl));
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
        let mut chunks = theme.global_style_chunks(registry, site, Some(&queued));
        // `wp_enqueue_global_styles()` of a block theme: the Additional CSS of the Customizer follows the stylesheet,
        // in the preview always and between milestones the live preview finds.
        let mut customizer_css = match preview.and_then(|p| p.custom_css.clone()) {
            Some(css) => css,
            None => crate::classic::wp_get_custom_css(site, &crate::classic::menu::theme_mod(site, &theme.stylesheet)).await?,
        }
        .trim()
        .to_owned();
        if !customizer_css.is_empty() || preview.is_some() {
            if preview.is_some() {
                let (before, after) = ("/*BEGIN_CUSTOMIZER_CUSTOM_CSS*/", "/*END_CUSTOMIZER_CUSTOM_CSS*/");
                customizer_css = format!("{before}\n{}\n{after}", customizer_css.replace(before, "").replace(after, ""));
            }
            if let Some(first) = chunks.first_mut() {
                first.push('\n');
                first.push_str(&customizer_css);
            }
        }
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
    // `wp_enqueue_block_custom_css()`.
    let custom_css: String = state.inline_styles.iter().filter(|(handle, _)| handle == "wp-block-custom-css").map(|(_, css)| css.as_str()).collect::<Vec<_>>().join("\n");
    if !custom_css.is_empty() {
        styles.push_str(&format!("<style id=\"wp-block-custom-css-inline-css\">\n{custom_css}\n/*# sourceURL=wp-block-custom-css-inline-css */\n</style>\n"));
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
    if !skip_link_inline && rustpress_core::files::is_file(&skip_link_file) {
        styles.push_str(&format!("<link rel='stylesheet' id='wp-block-template-skip-link-css' href='{includes_url}/css/wp-block-template-skip-link.min.css?ver={version}' media='all' />\n"));
    } else if let Some(css) = read(&skip_link_file) {
        styles.push_str(&format!("<style id=\"wp-block-template-skip-link-inline-css\">\n{}\n/*# sourceURL=/wp-includes/css/wp-block-template-skip-link.min.css */\n</style>\n", css.trim_end()));
    }
    // Plugins enqueue on `wp_enqueue_scripts` before the theme does: their stylesheets of priority 10 come before the
    // theme's, later ones after it.
    let plugin_assets = site.plugins.assets_with(site, Some(&state.plugins));
    let printed: &[&str] = if toolbar_on { &["dashicons", "admin-bar"] } else { &[] };
    let mut style_printer = crate::plugin_assets::StylePrinter::new(wp_root, site).with_printed(printed);
    let (early_styles, late_styles): (Vec<_>, Vec<_>) = plugin_assets.styles.iter().cloned().partition(|a| a.priority <= 10);
    styles.push_str(&style_printer.links(&early_styles));
    // The theme's style.css is only printed when its functions.php enqueues it.
    let theme_css = theme.theme_style_handle().and_then(|handle| ["style.min.css", "style.css"].iter().find_map(|f| read(&theme.template_dir().join(f)).map(|c| (handle, *f, c))));
    if !theme_style_inline && let Some(handle) = theme.theme_style_handle() && theme_style_file.is_some() {
        let file = theme_style_file.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("style.css");
        styles.push_str(&format!("<link rel='stylesheet' id='{handle}-css' href='{}/{file}?ver={}' media='all' />\n", theme.template_directory_uri(), theme.version()));
    } else if let Some((handle, file, css)) = theme_css {
        styles.push_str(&format!("<style id=\"{handle}-inline-css\">\n{}\n/*# sourceURL={}/{file} */\n</style>\n", css.trim_end(), theme.template_directory_uri()));
    }
    // The media widgets and the Text widget of the preview enqueue the players' styles, unless the content did.
    if preview.is_some() && !handles.iter().any(|h| h == "wp-mediaelement") {
        styles.push_str(&crate::customize_preview::media_styles(&includes_url, &version));
    }
    styles.push_str(&style_printer.links(&late_styles));
    // Blocks a plugin rendered in `wp_head` that the template did not use: their stylesheets were enqueued after the
    // theme's (and after `wp_maybe_inline_styles()` ran), so they are linked.
    let mut late: Vec<String> = Vec::new();
    for name in head_blocks.iter().filter(|n| !state.used_blocks.contains(n)) {
        let Some(block_type) = registry.get(name) else { continue };
        for handle in &block_type.styles {
            if !handles.contains(handle) && !late.contains(handle) {
                late.push(handle.clone());
            }
        }
    }
    for handle in &late {
        let Some(slug) = handle.strip_prefix("wp-block-") else { continue };
        if rustpress_core::files::is_file(&registry.blocks_dir().join(slug).join("style.min.css")) {
            styles.push_str(&format!("<link rel='stylesheet' id='{handle}-css' href='{includes_url}/blocks/{slug}/style.min.css?ver={version}' media='all' />\n"));
        }
    }
    let fonts = font_faces(theme);

    // --- scripts ---------------------------------------------------------
    let modules: Vec<String> = state.script_modules.iter().cloned().collect();
    let (importmap, preloads, module_scripts) = script_modules(wp_root, site, &modules);
    // Signed-in viewers get no speculation rules.
    let speculation = if query.viewer.is_some() { String::new() } else { speculation_rules(site, theme) };
    // A plugin that resets the query on `wp_head` runs `the_post` for the global post after the template.
    let mut looped = state.looped_posts.clone();
    if site.plugins.resets_query_in_head()
        && let Some(id) = global_post_id(query)
    {
        looped.push(id);
    }
    let plugin_footer = site.plugins.wp_footer(site, &looped).await?;
    let lightbox = if state.lightbox_overlay { rustpress_blocks::dynamic::lightbox::overlay(site, theme) } else { String::new() };
    let playlist_templates = if state.playlist || preview.is_some() { rustpress_blocks::shortcodes::playlist_templates(site) } else { String::new() };
    let nav_menus = preview.map(crate::customize_preview::nav_menus_exports).unwrap_or_default();
    let mut footer_scripts = format!("{playlist_templates}{nav_menus}{speculation}{}{plugin_footer}{module_scripts}{}{lightbox}", rustpress_blocks::duotone::footer_svgs(&state.duotone), state.interactivity.data_script().unwrap_or_default());
    // MediaElement: jQuery in the head, the player scripts in the footer. The scripts plugins enqueue for the head
    // follow, after their dependencies.
    let mut printer = crate::plugin_assets::ScriptPrinter::new(&includes_url, "3.7.1", "3.4.1").with_packages(&theme.wp_root, site);
    let mut head_scripts = String::new();
    if state.mediaelement || state.playlist || preview.is_some() {
        head_scripts.push_str(&printer.core("jquery"));
    }
    head_scripts.push_str(&printer.scripts(&plugin_assets.scripts, false));
    let styles = format!("{styles}{head_scripts}");
    // The preview queues its own scripts first (`wp_loaded`), and with them the players of the media widgets.
    let mut preview_scripts = preview.map(|_| crate::customize_preview::FooterScripts::new(site, wp_root, &[], &["jquery", "jquery-core", "jquery-migrate"]));
    if let Some(scripts) = preview_scripts.as_mut() {
        scripts.enqueue("customize-preview");
        footer_scripts.push_str(&scripts.print());
    }
    if toolbar_on {
        footer_scripts.push_str(&format!("<script id=\"hoverintent-js-js\" src=\"{includes_url}/js/hoverintent-js.min.js?ver=2.2.1\"></script>\n<script id=\"admin-bar-js\" src=\"{includes_url}/js/admin-bar.min.js?ver={version}\"></script>\n"));
    }
    if let Some(scripts) = preview_scripts.as_mut() {
        for handle in ["wp-mediaelement", "mediaelement-vimeo", "wp-playlist", "customize-selective-refresh", "customize-preview-nav-menus"] {
            scripts.enqueue(handle);
        }
        if state.scripts.iter().any(|s| s == "comment-reply") {
            scripts.enqueue("comment-reply");
        }
        footer_scripts.push_str(&scripts.print());
    }
    // `wp-playlist` needs `wp-util`, `backbone` and `mediaelement`, but not `wp-mediaelement`.
    if state.playlist && preview.is_none() {
        footer_scripts.push_str(&printer.core("wp-util"));
        footer_scripts.push_str(&printer.core("backbone"));
    }
    if (state.mediaelement || state.playlist) && preview.is_none() {
        footer_scripts.push_str(&mediaelement_scripts(site, wp_root, &includes_url, &version, state.vimeo, state.mediaelement));
    }
    if state.playlist && preview.is_none() {
        printer.mark_printed(&["mediaelement", "mediaelement-core", "mediaelement-migrate"]);
        footer_scripts.push_str(&printer.core("wp-playlist"));
    }
    if state.scripts.iter().any(|s| s == "comment-reply") && preview.is_none() {
        footer_scripts.push_str(&format!("<script async data-wp-strategy=\"async\" fetchpriority=\"low\" id=\"comment-reply-js\" src=\"{includes_url}/js/comment-reply.min.js?ver={version}\"></script>\n"));
    }
    footer_scripts.push_str(&printer.scripts(&plugin_assets.scripts, true));
    footer_scripts.push_str(&emoji_detection_script(site, wp_root, &version));
    // `wp_print_footer_scripts` after `_wp_footer_scripts()`.
    footer_scripts.push_str(&state.plugins.footer_scripts.concat());
    if preview.is_some() {
        footer_scripts.push_str(crate::customize_preview::FOOTER_EXPORTS);
    }

    // --- head ------------------------------------------------------------
    let mut robots = rustpress_core::native::robots_tag(plugin_head.robots.as_deref().unwrap_or_default());
    // `wp_post_preview_js()`: the preview window is named after the post.
    if query.flags.is_preview && let Queried::Post(post) = &query.queried {
        robots.push_str(&format!("<script>\n\t( function() {{\n\t\tvar query = document.location.search;\n\n\t\tif ( query && query.indexOf( 'preview=true' ) !== -1 ) {{\n\t\t\twindow.name = 'wp-preview-{}';\n\t\t}}\n\n\t\tif ( window.addEventListener ) {{\n\t\t\twindow.addEventListener( 'pagehide', function() {{ window.name = ''; }} );\n\t\t}}\n\t}}());\n\t//# sourceURL=wp_post_preview_js\t\n</script>\n", post.id));
    }
    // Plugins hook into `wp_head` after the actions of core at the same priority.
    robots.push_str(&plugin_head.html);
    let title = match plugin_head.title {
        Some(title) => title,
        None => document_title(site, query).await?,
    };
    let lang = site.language_tag();
    let classes = body_classes(theme, site, query).await?;

    // `customize_preview_loading_style()` on `wp_head` after the actions of priority 10 of core.
    let loading = preview.map(crate::customize_preview::loading_style).unwrap_or_default();
    // `wp_site_icon()`: the default icon in the preview, so that the live preview can replace it.
    let mut site_icon = crate::classic::wp_site_icon(site).await?;
    if site_icon.is_empty() && preview.is_some() {
        site_icon = "<link rel=\"icon\" href=\"/favicon.ico\" sizes=\"32x32\" />\n".to_owned();
    }
    let head = format!(
        // Plugins' `wp_head` actions of priority 0 run before `_block_template_viewport_meta_tag()`, which is hooked
        // when the template is loaded; the actions plugins hook on `init` at priority 10 come after the script modules.
        "<meta charset=\"UTF-8\" />\n\t{}<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n{robots}<title>{}</title>\n{}{}{}{styles}{}{importmap}{preloads}{}{loading}{fonts}{site_icon}",
        plugin_head.first,
        esc_html(&title),
        rustpress_core::native::resource_hints(site, &plugin_assets),
        crate::head::feed_links(site, query).await?,
        oembed_links(site, query).await?,
        discovery_links(site, query, &version, state.cpage_override, !plugin_head.removed.contains(&"rel_canonical"), &locale_stylesheet(theme, site)).await?
            // A ported block theme's `wp_head` actions of priority 10 come after core's (`rel_canonical`, the shortlink).
            + &plugin_head.late
            + &theme.block_theme_port().map(|port| port.wp_head(query)).unwrap_or_default(),
        plugin_head.late_init,
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
    tracing::debug!(body_ms = body_done.as_secs_f64() * 1000.0, total_ms = started.elapsed().as_secs_f64() * 1000.0, path = query.url_path.as_str(), "document rendered");
    Ok(Document { status: query.status(), html })
}
