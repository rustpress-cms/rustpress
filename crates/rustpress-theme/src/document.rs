//! The complete HTML document: `<head>` (title, styles, discovery links,
//! script modules) around the rendered template, as `wp_head()` and
//! `wp_footer()` would print it for a block theme.

use std::path::Path;

use rustpress_blocks::{Registry, ThemeAccess};
use rustpress_core::formatting::{capital_p_dangit, esc_attr, esc_html, esc_url, wptexturize};
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

/// Inline stylesheets up to this many bytes in total; larger files become `<link>`s.
const INLINE_LIMIT: u64 = 40_000;

/// `sanitize_html_class`.
fn html_class(class: &str) -> String {
    class.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect()
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
    if f.is_front_page {
        push("home");
    }
    if f.is_home {
        push("blog");
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
                    c.push("single-format-standard".into());
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
    let mut site_name_last = true;
    if f.is_404 {
        parts.push("Page not found".into());
    } else if f.is_search {
        parts.push(format!("Search Results for &#8220;{}&#8221;", esc_html(q.vars.get("s").unwrap_or(""))));
    } else if f.is_front_page && !f.is_page {
        parts.push(s.blogname.clone());
        if !s.blogdescription.is_empty() {
            parts.push(s.blogdescription.clone());
        }
        site_name_last = false;
    } else if f.is_front_page {
        parts.push(s.blogname.clone());
        site_name_last = false;
        if !s.blogdescription.is_empty() {
            parts.push(s.blogdescription.clone());
        }
    } else if f.is_singular() || f.is_posts_page {
        if let Some(post) = q.queried_post() {
            parts.push(post.post_title.clone());
        }
    } else if f.is_category || f.is_tag || f.is_tax {
        if let Queried::Term(term) = &q.queried {
            parts.push(term.name.clone());
        }
    } else if f.is_author {
        if let Queried::User(user) = &q.queried {
            parts.push(user.display_name.clone());
        }
    } else if f.is_date {
        let ts = q.posts.first().and_then(|p| site.timezone.local_timestamp(&p.post_date));
        let format = if f.is_day { "F j, Y" } else if f.is_month { "F Y" } else { "Y" };
        if let Some(ts) = ts {
            parts.push(site.timezone.format(format, ts));
        }
    }
    let page = q.vars.u64("page").filter(|p| *p >= 2).or_else(|| (q.paged >= 2).then_some(q.paged));
    if let Some(page) = page.filter(|_| !f.is_404) {
        parts.push(format!("Page {page}"));
    }
    if site_name_last && !(f.is_front_page && parts.is_empty()) {
        parts.push(s.blogname.clone());
    }
    let title = parts.join(" - ");
    let title = wptexturize(&title);
    Ok(capital_p_dangit(&title, true))
}

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub(crate) fn wp_version(wp_root: &Path) -> String {
    read(&wp_root.join("wp-includes").join("version.php"))
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("$wp_version = '").and_then(|r| r.split('\'').next().map(str::to_owned))))
        .unwrap_or_default()
}

const EMOJI_CSS: &str = "\n\timg.wp-smiley, img.emoji {\n\t\tdisplay: inline !important;\n\t\tborder: none !important;\n\t\tbox-shadow: none !important;\n\t\theight: 1em !important;\n\t\twidth: 1em !important;\n\t\tmargin: 0 0.07em !important;\n\t\tvertical-align: -0.1em !important;\n\t\tbackground: none !important;\n\t\tpadding: 0 !important;\n\t}";

fn emoji_style() -> String {
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
}

fn script_modules(wp_root: &Path, site: &Site, ids: &[String]) -> (String, String, String) {
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
fn font_faces(theme: &Theme) -> String {
    let settings = theme.json.settings();
    let Some(families) = settings.get("typography").and_then(|t| t.get("fontFamilies")).and_then(Value::as_array) else { return String::new() };
    let base = theme.stylesheet_directory_uri();
    let mut css = String::new();
    for family in families {
        let Some(faces) = family.get("fontFace").and_then(Value::as_array) else { continue };
        for face in faces {
            let name = face.get("fontFamily").and_then(Value::as_str).unwrap_or_default();
            let mut rule = format!("font-family:{name};");
            for (key, property) in [("fontStyle", "font-style"), ("fontWeight", "font-weight"), ("fontStretch", "font-stretch")] {
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
                    let format = url.rsplit('.').next().unwrap_or("woff2");
                    format!("url('{url}') format('{format}')")
                }).collect())
                .unwrap_or_default();
            if sources.is_empty() {
                continue;
            }
            rule.push_str(&format!("src:{};", sources.join(", ")));
            css.push_str(&format!("@font-face{{{rule}}}\n"));
        }
    }
    if css.is_empty() { String::new() } else { format!("<style class=\"wp-fonts-local\">\n{css}</style>\n") }
}

async fn discovery_links(site: &Site, q: &QueryResult, version: &str) -> Result<String> {
    let mut out = format!("<link rel=\"https://api.w.org/\" href=\"{}\" />", esc_url(&site.home_url("wp-json/")));
    if let Some(route) = crate::head::queried_rest_route(q) {
        out.push_str(&format!("<link rel=\"alternate\" title=\"JSON\" type=\"application/json\" href=\"{}\" />", esc_url(&site.home_url(&format!("wp-json/{route}")))));
    }
    out.push_str(&format!("<link rel=\"EditURI\" type=\"application/rsd+xml\" title=\"RSD\" href=\"{}\" />
", esc_url(&site.site_url("xmlrpc.php?rsd"))));
    out.push_str(&format!("<meta name=\"generator\" content=\"WordPress {version}\" />
"));
    if q.flags.is_singular()
        && let Queried::Post(post) = &q.queried {
            out.push_str(&format!("<link rel=\"canonical\" href=\"{}\" />
", esc_url(&site.permalink(post).await?)));
            let settings = &site.settings;
            let shortlink = if post.post_type == "page" && settings.show_on_front == rustpress_core::settings::ShowOnFront::Page && settings.page_on_front == post.id { site.home_url("/") } else { site.home_url(&format!("?p={}", post.id)) };
            out.push_str(&format!("<link rel='shortlink' href='{}' />
", esc_url(&shortlink)));
        }
    Ok(out)
}

/// `wp_oembed_add_discovery_links()`: the oEmbed endpoints of a single post or page.
async fn oembed_links(site: &Site, q: &QueryResult) -> Result<String> {
    if !q.flags.is_singular() {
        return Ok(String::new());
    }
    let Queried::Post(post) = &q.queried else { return Ok(String::new()) };
    let permalink = rustpress_core::canonical::php_urlencode(&site.permalink(post).await?);
    let endpoint = if site.settings.permalink_structure.is_empty() { site.home_url("?rest_route=/oembed/1.0/embed&url=") } else { site.home_url("wp-json/oembed/1.0/embed?url=") };
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
fn robots_meta(site: &Site, q: &QueryResult) -> String {
    let public = site.options.get("blog_public").is_some_and(|v| !v.is_empty() && v != "0");
    let mut directives: Vec<&str> = Vec::new();
    if !public {
        directives.extend(["noindex", "nofollow"]);
    } else {
        if q.flags.is_search {
            directives.extend(["noindex", "follow"]);
        }
        directives.push("max-image-preview:large");
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
fn emoji_detection_script(site: &Site, wp_root: &Path, version: &str) -> String {
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

/// Renders a request into a complete HTML document.
pub async fn render_document(theme: &Theme, site: &Site, query: &QueryResult, registry: &Registry) -> Result<Document> {
    let rendered = render_body(theme, site, query, registry).await?;
    let state = &rendered.state;
    let wp_root = &theme.wp_root;
    let version = wp_version(wp_root);
    let includes_url = site.site_url("wp-includes");

    // --- stylesheets, in the order WordPress prints them -------------------
    let mut sheets: Vec<Sheet> = Vec::new();
    let mut handles: Vec<String> = Vec::new();
    for name in &state.used_blocks {
        let Some(block_type) = registry.get(name) else { continue };
        for handle in &block_type.styles {
            if !handles.contains(handle) {
                handles.push(handle.clone());
            }
        }
    }
    for handle in &handles {
        let Some(slug) = handle.strip_prefix("wp-block-") else { continue };
        let path = registry.blocks_dir().join(slug).join("style.min.css");
        if path.is_file() {
            sheets.push(Sheet { id: handle.clone(), file: Some((path, format!("{includes_url}/blocks/{slug}/style.min.css?ver={version}"))), inline: None, extra: Vec::new() });
        }
    }
    // With block style variations WordPress queues the emoji style later.
    let has_variations = state.inline_styles.iter().any(|(handle, _)| handle == "block-style-variation-styles");
    if !has_variations {
        sheets.push(Sheet { id: "wp-emoji-styles".into(), file: None, inline: Some(EMOJI_CSS.into()), extra: Vec::new() });
    }
    let common = wp_root.join("wp-includes").join("css").join("dist").join("block-library").join("common.min.css");
    if common.is_file() {
        sheets.push(Sheet { id: "wp-block-library".into(), file: Some((common, format!("{includes_url}/css/dist/block-library/common.min.css?ver={version}"))), inline: None, extra: Vec::new() });
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
    let mut by_size: Vec<(usize, u64)> = sheets.iter().enumerate().filter_map(|(i, s)| s.file.as_ref().and_then(|(p, _)| std::fs::metadata(p).ok()).map(|m| (i, m.len()))).collect();
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
    for i in inline_ids {
        if let Some((path, _)) = &sheets[i].file {
            sheets[i].inline = read(path);
        }
    }

    let mut styles = String::new();
    styles.push_str("<style id=\"wp-img-auto-sizes-contain-inline-css\">\nimg:is([sizes=auto i],[sizes^=\"auto,\" i]){contain-intrinsic-size:3000px 1500px}\n/*# sourceURL=wp-img-auto-sizes-contain-inline-css */\n</style>\n");
    for sheet in &sheets {
        match (&sheet.inline, &sheet.file) {
            (Some(css), Some((_, url))) => {
                let absolute = url.split('?').next().unwrap_or(url);
                // The common block stylesheet is registered with a site-relative path, block styles with a URL.
                let source = if sheet.id == "wp-block-library" { absolute.strip_prefix(includes_url.trim_end_matches("/wp-includes")).unwrap_or(absolute) } else { absolute };
                if sheet.extra.is_empty() {
                    styles.push_str(&format!("<style id=\"{}-inline-css\">\n{}\n/*# sourceURL={source} */\n</style>\n", sheet.id, css.trim_end()));
                } else {
                    // With further inline styles the sourceURL names the handle instead of the file.
                    styles.push_str(&format!("<style id=\"{id}-inline-css\">\n{}\n{}\n/*# sourceURL={id}-inline-css */\n</style>\n", css.trim_end(), sheet.extra.join("\n"), id = sheet.id));
                }
            }
            (Some(css), None) => styles.push_str(&format!("<style id=\"{}-inline-css\">\n{css}\n/*# sourceURL={}-inline-css */\n</style>\n", sheet.id, sheet.id)),
            (None, Some((_, url))) => {
                styles.push_str(&format!("<link rel='stylesheet' id='{}-css' href='{}' media='all' />\n", sheet.id, url));
                if !sheet.extra.is_empty() {
                    styles.push_str(&format!("<style id=\"{id}-inline-css\">\n{}\n/*# sourceURL={id}-inline-css */\n</style>\n", sheet.extra.join("\n"), id = sheet.id));
                }
            }
            _ => {}
        }
    }
    let global = GlobalStyles::new(&theme.json, registry).stylesheet_chunks(Some(&handles)).join("\n");
    styles.push_str(&format!("<style id=\"global-styles-inline-css\">\n{global}\n/*# sourceURL=global-styles-inline-css */\n</style>\n"));
    let variation_css: String = state.inline_styles.iter().filter(|(handle, _)| handle == "block-style-variation-styles").map(|(_, css)| css.as_str()).collect();
    if !variation_css.is_empty() {
        styles.push_str(&format!("<style id=\"block-style-variation-styles-inline-css\">\n{variation_css}\n/*# sourceURL=block-style-variation-styles-inline-css */\n</style>\n"));
    }
    if has_variations {
        styles.push_str(&emoji_style());
    }
    let supports = state.block_supports_css.compile();
    if !supports.is_empty() {
        styles.push_str(&format!("<style id=\"core-block-supports-inline-css\">\n{supports}\n/*# sourceURL=core-block-supports-inline-css */\n</style>\n"));
    }
    if let Some(css) = read(&wp_root.join("wp-includes").join("css").join("wp-block-template-skip-link.min.css")) {
        styles.push_str(&format!("<style id=\"wp-block-template-skip-link-inline-css\">\n{}\n/*# sourceURL=/wp-includes/css/wp-block-template-skip-link.min.css */\n</style>\n", css.trim_end()));
    }
    let theme_css = ["style.min.css", "style.css"].iter().find_map(|f| read(&theme.template_dir().join(f)).map(|c| (*f, c)));
    if let Some((file, css)) = theme_css {
        styles.push_str(&format!("<style id=\"{}-style-inline-css\">\n{}\n/*# sourceURL={}/{file} */\n</style>\n", site.settings.template, css.trim_end(), theme.template_directory_uri()));
    }
    let fonts = font_faces(theme);

    // --- scripts ---------------------------------------------------------
    let modules: Vec<String> = state.script_modules.iter().cloned().collect();
    let (importmap, preloads, module_scripts) = script_modules(wp_root, site, &modules);
    let mut footer_scripts = format!("{}{module_scripts}", speculation_rules(site, theme));
    if state.scripts.iter().any(|s| s == "comment-reply") {
        footer_scripts.push_str(&format!("<script async data-wp-strategy=\"async\" fetchpriority=\"low\" id=\"comment-reply-js\" src=\"{includes_url}/js/comment-reply.min.js?ver={version}\"></script>\n"));
    }
    footer_scripts.push_str(&emoji_detection_script(site, wp_root, &version));

    // --- head ------------------------------------------------------------
    let robots = robots_meta(site, query);
    let title = document_title(site, query).await?;
    let lang = site.options.get("WPLANG").filter(|l| !l.is_empty()).map(|l| l.replace('_', "-")).unwrap_or_else(|| "en-US".to_owned());
    let classes = body_classes(theme, site, query).await?;

    let head = format!(
        "<meta charset=\"UTF-8\" />\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n{robots}<title>{}</title>\n{}{}{styles}{}{importmap}{preloads}{fonts}",
        esc_html(&title),
        crate::head::feed_links(site, query).await?,
        oembed_links(site, query).await?,
        discovery_links(site, query, &version).await?,
    );
    let html = format!(
        "<!DOCTYPE html>\n<html lang=\"{}\">\n<head>\n\t{head}</head>\n\n<body class=\"{}\">\n\n{}\n{footer_scripts}</body>\n</html>\n",
        esc_attr(&lang),
        esc_attr(&classes.join(" ")),
        rendered.html
    );
    Ok(Document { status: query.status(), html })
}
