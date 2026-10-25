//! The embeddable view of a post: the `/post/embed/` page that other sites show in an iframe
//! (WordPress' `theme-compat/embed*.php` templates and the functions behind them in `embed.php`),
//! and the HTML snippet that embeds it (`get_post_embed_html()`).

use std::sync::Arc;

use rustpress_blocks::content::{post_class, the_title_for};
use rustpress_blocks::excerpt::get_the_excerpt_with;
use rustpress_blocks::images::{self, ImageArgs};
use rustpress_blocks::{Env, Registry};
use rustpress_core::formatting::{convert_chars, esc_attr, esc_html, esc_url, wpautop};
use rustpress_core::i18n::sprintf;
use rustpress_core::{QueryResult, Site};
use rustpress_db::Post;

use crate::Result;
use crate::document::{Document, body_classes, document_title, emoji_detection_script, emoji_style, read, wp_version};
use crate::theme::Theme;

/// A pseudo-random number below 2^32 (`wp_rand()`), from the clock and a counter.
fn wp_rand() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    hasher.finish() % 4_294_967_296
}

/// `wp_generate_password( $length, false )`.
fn generate_password(length: usize) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..length).map(|_| CHARS[(wp_rand() % CHARS.len() as u64) as usize] as char).collect()
}

/// `esc_textarea()`.
fn esc_textarea(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#039;")
}

/// `get_post_embed_url()`.
pub async fn post_embed_url(site: &Site, post: &Post) -> Result<String> {
    let permalink = site.permalink(post).await?;
    let embed = format!("{}{}", rustpress_core::urls::trailingslashit(&permalink), site.user_trailingslashit("embed"));
    let home = site.home_url("");
    let path = embed.replacen(&home, "", 1);
    let conflict = site.post_by_path(path.trim_matches('/'), "page").await?.is_some();
    if !site.uses_pretty_permalinks() || conflict {
        return Ok(esc_url(&rustpress_core::urls::add_query_arg(&permalink, "embed", "true")));
    }
    Ok(esc_url(&embed))
}

/// `get_post_embed_html( $width, $height, $post )`: the quote with the iframe and the script
/// that resizes it.
pub async fn post_embed_html(site: &Site, wp_root: &std::path::Path, post: &Post, width: u64, height: u64) -> Result<String> {
    let secret = generate_password(10);
    let embed_url = format!("{}#?secret={secret}", post_embed_url(site, post).await?);
    let title = the_title_for(site, post);
    let mut out = format!(
        "<blockquote class=\"wp-embedded-content\" data-secret=\"{}\"><a href=\"{}\">{}</a></blockquote>",
        esc_attr(&secret),
        esc_url(&site.permalink(post).await?),
        title
    );
    let iframe_title = sprintf(&site.tr("&#8220;%1$s&#8221; &#8212; %2$s"), &[&title, &site.settings.blogname]);
    out.push_str(&format!(
        "<iframe sandbox=\"allow-scripts\" security=\"restricted\" src=\"{}\" width=\"{width}\" height=\"{height}\" title=\"{}\" data-secret=\"{}\" frameborder=\"0\" marginwidth=\"0\" marginheight=\"0\" scrolling=\"no\" class=\"wp-embedded-content\"></iframe>",
        esc_url(&embed_url),
        esc_attr(&iframe_title),
        esc_attr(&secret)
    ));
    let path = "js/wp-embed.min.js";
    if let Some(js) = read(&wp_root.join("wp-includes").join(path)) {
        out.push_str(&inline_script_tag(&js, &site.site_url(&format!("wp-includes/{path}"))));
    }
    Ok(out)
}

/// `wp_get_inline_script_tag()` with the `sourceURL` comment WordPress adds to its own scripts.
fn inline_script_tag(js: &str, source_url: &str) -> String {
    format!("<script>\n{}\n//# sourceURL={}\n</script>\n", js.trim(), esc_url(source_url))
}

/// `get_site_icon_url( $size, $fallback )`.
pub(crate) async fn site_icon_url(env: &Env<'_>, size: u64, fallback: &str) -> Result<String> {
    let id = env.site.options.u64("site_icon", 0);
    if id != 0 {
        let rows = env.site.db.post_meta(id).await?;
        let value = |key: &str| rows.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
        if let (Some(file), Some(meta)) = (value("_wp_attached_file"), value("_wp_attachment_metadata").and_then(|raw| images::Metadata::parse(&raw))) {
            let full = images::attachment_url(env.site, &file);
            if let Some(url) = images::intermediate_url(&full, &meta, size, size) {
                return Ok(url);
            }
            return Ok(full);
        }
    }
    Ok(fallback.to_owned())
}

/// `the_embed_site_title()`.
async fn embed_site_title(env: &Env<'_>) -> Result<String> {
    let site = env.site;
    let fallback = site.site_url("wp-includes/images/w-logo-gray-white-bg.svg");
    let icon = site_icon_url(env, 32, &fallback).await?;
    let mut icon_img = String::new();
    if !icon.is_empty() {
        let icon2 = site_icon_url(env, 64, &fallback).await?;
        let srcset = if !icon2.is_empty() && icon != icon2 { format!(" srcset=\"{} 2x\"", esc_url(&icon2)) } else { String::new() };
        icon_img = format!("<img src=\"{}\"{srcset} width=\"32\" height=\"32\" alt=\"\" class=\"wp-embed-site-icon\" />", esc_url(&icon));
    }
    Ok(format!("<div class=\"wp-embed-site-title\"><a href=\"{}\" target=\"_top\">{icon_img}<span>{}</span></a></div>", esc_url(&site.home_url("")), esc_html(&site.settings.blogname)))
}

/// `the_excerpt_embed()`.
async fn excerpt_embed(env: &Env<'_>, post: &Arc<Post>) -> Result<String> {
    let site = env.site;
    let permalink = site.permalink(post).await?;
    let title = the_title_for(site, post);
    // `wp_embed_excerpt_more()` replaces whatever the theme appends.
    let more = format!(
        " &hellip; <a href=\"{}\" class=\"wp-embed-more\" target=\"_top\">{}</a>",
        esc_url(&permalink),
        sprintf(&site.tr("Continue reading %s"), &[&format!("<span class=\"screen-reader-text\">{title}</span>")])
    );
    let excerpt = get_the_excerpt_with(env, post, site.excerpt_length(), &more).await?;
    let mut out = convert_chars(&site.texturize(&excerpt));
    out = wpautop(&out);
    out = rustpress_core::shortcodes::shortcode_unautop(&out);
    if post.post_type == "attachment" && env.query.flags.is_attachment {
        let (meta, file) = images::load_metadata(site, post).await?;
        if let Some(file) = file {
            let alt = String::new();
            let link = env.in_content_filter(|| images::attachment_link(env, post, meta.as_ref(), &file, "medium", &alt));
            out = format!("<p class=\"attachment\">{link}</p>\n");
        }
    }
    Ok(out)
}

/// The thumbnail of `embed-content.php`: `(attachment id, image size, shape)`.
async fn thumbnail(env: &Env<'_>, post: &Post) -> Result<Option<(u64, String, &'static str)>> {
    let mut id = images::post_thumbnail_id(env, post.id).await?;
    if post.post_type == "attachment" && images::is_image(post) {
        id = post.id;
    }
    if id == 0 {
        return Ok(None);
    }
    let mut aspect = 1.0f64;
    let mut measurements = (1.0f64, 1.0f64);
    let mut size = "full".to_owned();
    if let Some(meta) = images::attachment_metadata(env, id).await? {
        for (name, data) in meta.sizes().into_iter().flatten() {
            let (w, h) = (data.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0), data.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0));
            if h > 0.0 && w / h > aspect {
                aspect = w / h;
                measurements = (w, h);
                size = name.clone();
            }
        }
    }
    let shape = if measurements.0 / measurements.1 >= 1.75 { "rectangular" } else { "square" };
    Ok(Some((id, size, shape)))
}

/// `wp_get_attachment_image( $thumbnail_id, $image_size )`.
async fn thumbnail_image(env: &Env<'_>, id: u64, size: &str) -> Result<String> {
    Ok(images::get_attachment_image(env, id, size, &ImageArgs::default()).await?)
}

/// `print_embed_comments_button()`.
fn comments_button(site: &Site, post: &Post, permalink: &str) -> String {
    let number = post.comment_count;
    if !(number > 0 || post.comment_status == "open") {
        return String::new();
    }
    let hash = if number > 0 { "#comments" } else { "#respond" };
    let counted = sprintf(&site.trn("%s <span class=\"screen-reader-text\">Comment</span>", "%s <span class=\"screen-reader-text\">Comments</span>", number), &[&site.number_i18n(number.max(0) as u64)]);
    format!(
        "\t<div class=\"wp-embed-comments\">\n\t\t<a href=\"{}\" target=\"_top\">\n\t\t\t<span class=\"dashicons dashicons-admin-comments\"></span>\n\t\t\t{counted}\t\t</a>\n\t</div>\n\t",
        esc_url(&format!("{permalink}{hash}"))
    )
}

/// `print_embed_sharing_button()`.
fn sharing_button(site: &Site) -> String {
    format!(
        "\t<div class=\"wp-embed-share\">\n\t\t<button type=\"button\" class=\"wp-embed-share-dialog-open\" aria-label=\"{}\">\n\t\t\t<span class=\"dashicons dashicons-share\"></span>\n\t\t</button>\n\t</div>\n\t",
        esc_attr(&site.tr("Open sharing dialog"))
    )
}

/// `print_embed_sharing_dialog()` for the first post of the query.
async fn sharing_dialog(site: &Site, wp_root: &std::path::Path, post: &Post) -> Result<String> {
    let suffix = format!("{}-{}", post.id, wp_rand());
    let (tab_wp, tab_html) = (format!("wp-embed-share-tab-wordpress-{suffix}"), format!("wp-embed-share-tab-html-{suffix}"));
    let (desc_wp, desc_html) = (format!("wp-embed-share-description-wordpress-{suffix}"), format!("wp-embed-share-description-html-{suffix}"));
    let permalink = esc_url(&site.permalink(post).await?);
    let html = esc_textarea(&post_embed_html(site, wp_root, post, 600, 400).await?);
    Ok(format!(
        "\t<div class=\"wp-embed-share-dialog hidden\" role=\"dialog\" aria-label=\"{sharing}\">
\t\t<div class=\"wp-embed-share-dialog-content\">
\t\t\t<div class=\"wp-embed-share-dialog-text\">
\t\t\t\t<ul class=\"wp-embed-share-tabs\" role=\"tablist\">
\t\t\t\t\t<li class=\"wp-embed-share-tab-button wp-embed-share-tab-button-wordpress\" role=\"presentation\">
\t\t\t\t\t\t<button type=\"button\" role=\"tab\" aria-controls=\"{tab_wp}\" aria-selected=\"true\" tabindex=\"0\">{wordpress}</button>
\t\t\t\t\t</li>
\t\t\t\t\t<li class=\"wp-embed-share-tab-button wp-embed-share-tab-button-html\" role=\"presentation\">
\t\t\t\t\t\t<button type=\"button\" role=\"tab\" aria-controls=\"{tab_html}\" aria-selected=\"false\" tabindex=\"-1\">{html_embed}</button>
\t\t\t\t\t</li>
\t\t\t\t</ul>
\t\t\t\t<div id=\"{tab_wp}\" class=\"wp-embed-share-tab\" role=\"tabpanel\" aria-hidden=\"false\">
\t\t\t\t\t<input type=\"text\" value=\"{permalink}\" class=\"wp-embed-share-input\" aria-label=\"{url}\" aria-describedby=\"{desc_wp}\" tabindex=\"0\" readonly/>

\t\t\t\t\t<p class=\"wp-embed-share-description\" id=\"{desc_wp}\">
\t\t\t\t\t\t{copy_url}\t\t\t\t\t</p>
\t\t\t\t</div>
\t\t\t\t<div id=\"{tab_html}\" class=\"wp-embed-share-tab\" role=\"tabpanel\" aria-hidden=\"true\">
\t\t\t\t\t<textarea class=\"wp-embed-share-input\" aria-label=\"{html_label}\" aria-describedby=\"{desc_html}\" tabindex=\"0\" readonly>{html}</textarea>

\t\t\t\t\t<p class=\"wp-embed-share-description\" id=\"{desc_html}\">
\t\t\t\t\t\t{copy_code}\t\t\t\t\t</p>
\t\t\t\t</div>
\t\t\t</div>

\t\t\t<button type=\"button\" class=\"wp-embed-share-dialog-close\" aria-label=\"{close}\">
\t\t\t\t<span class=\"dashicons dashicons-no\"></span>
\t\t\t</button>
\t\t</div>
\t</div>
\t",
        sharing = esc_attr(&site.tr("Sharing options")),
        wordpress = esc_html(&site.tr("WordPress Embed")),
        html_embed = esc_html(&site.tr("HTML Embed")),
        url = esc_attr(&site.tr("URL")),
        copy_url = site.tr("Copy and paste this URL into your WordPress site to embed"),
        html_label = esc_attr(&site.tr("HTML")),
        copy_code = site.tr("Copy and paste this code into your site to embed"),
        close = esc_attr(&site.tr("Close sharing dialog")),
    ))
}

/// `embed-content.php` for one post.
async fn content(env: &Env<'_>, post: &Arc<Post>) -> Result<String> {
    let site = env.site;
    let permalink = site.permalink(post).await?;
    let classes = post_class(env, post, &["wp-embed"], false, env.query.flags.is_singular()).await?;
    let thumb = thumbnail(env, post).await?;

    let mut out = format!("\t<div class=\"{}\">\n\t\t", esc_attr(&classes.join(" ")));
    if let Some((id, size, "rectangular")) = &thumb {
        out.push_str(&format!("\t\t\t<div class=\"wp-embed-featured-image rectangular\">\n\t\t\t\t<a href=\"{}\" target=\"_top\">\n\t\t\t\t\t{}\t\t\t\t</a>\n\t\t\t</div>\n\t\t", esc_url(&permalink), thumbnail_image(env, *id, size).await?));
    }
    out.push_str(&format!("\n\t\t<p class=\"wp-embed-heading\">\n\t\t\t<a href=\"{}\" target=\"_top\">\n\t\t\t\t{}\t\t\t</a>\n\t\t</p>\n\n\t\t", esc_url(&permalink), the_title_for(site, post)));
    if let Some((id, size, "square")) = &thumb {
        out.push_str(&format!("\t\t\t<div class=\"wp-embed-featured-image square\">\n\t\t\t\t<a href=\"{}\" target=\"_top\">\n\t\t\t\t\t{}\t\t\t\t</a>\n\t\t\t</div>\n\t\t", esc_url(&permalink), thumbnail_image(env, *id, size).await?));
    }
    out.push_str(&format!("\n\t\t<div class=\"wp-embed-excerpt\">{}</div>\n\n\t\t", excerpt_embed(env, post).await?));
    out.push_str(&format!(
        "\n\t\t<div class=\"wp-embed-footer\">\n\t\t\t{}\n\t\t\t<div class=\"wp-embed-meta\">\n\t\t\t\t{}{}\t\t\t</div>\n\t\t</div>\n\t</div>\n",
        embed_site_title(env).await?,
        comments_button(site, post, &permalink),
        sharing_button(site)
    ));
    Ok(out)
}

/// `embed-404.php`.
async fn not_found(env: &Env<'_>) -> Result<String> {
    let site = env.site;
    let link = format!("<strong><a href=\"{}\">{}</a></strong>", esc_url(&site.home_url("")), esc_html(&site.settings.blogname));
    Ok(format!(
        "<div class=\"wp-embed\">
\t<p class=\"wp-embed-heading\">{}</p>

\t<div class=\"wp-embed-excerpt\">
\t\t<p>
\t\t\t{}\t\t</p>
\t</div>

\t
\t<div class=\"wp-embed-footer\">
\t\t{}\t</div>
</div>
",
        site.tr("Oops! That embed cannot be found."),
        sprintf(&site.tr("It looks like nothing was found at this location. Maybe try visiting %s directly?"), &[&link]),
        embed_site_title(env).await?
    ))
}

/// The robots meta tag of the embed template: never indexed.
fn robots(site: &Site) -> String {
    let public = site.options.get("blog_public").is_some_and(|v| !v.is_empty() && v != "0");
    let mut directives = vec!["noindex"];
    directives.push(if public { "follow" } else { "nofollow" });
    if public {
        directives.push("max-image-preview:large");
    }
    format!("<meta name='robots' content='{}' />\n", directives.join(", "))
}

/// Renders the embed template for a request.
pub async fn render(theme: &Theme, site: &Site, q: &QueryResult, registry: &Registry) -> Result<Document> {
    let wp_root = &theme.wp_root;
    let version = wp_version(wp_root);
    let includes_url = site.site_url("wp-includes");
    let env = Env::new(site, q, theme, registry);

    let title = document_title(site, q).await?;
    let rtl = site.is_rtl();
    let mut head = String::new();
    head.push_str(&format!("\t<title>{}</title>\n\t<meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\">\n\t", esc_html(&title)));
    head.push_str(&robots(site));
    if let Some(canonical) = crate::document::canonical_url(site, q, 0).await?.filter(|_| q.flags.is_singular()) {
        head.push_str(&format!("<link rel=\"canonical\" href=\"{}\" />\n", esc_url(&canonical)));
    }
    head.push_str(&emoji_style());
    if let Some(css) = read(&wp_root.join("wp-includes").join("css").join("wp-embed-template.min.css")) {
        head.push_str(&format!("<style id=\"wp-embed-template-inline-css\">\n{}\n/*# sourceURL=wp-embed-template-inline-css */\n</style>\n", css.trim_end()));
    }

    let mut body = String::new();
    let posts: Vec<Arc<Post>> = q.posts.iter().cloned().map(Arc::new).collect();
    // The embed template runs `the_post()` in the main loop.
    env.with_state(|s| s.media.in_main_loop = true);
    if posts.is_empty() {
        body.push_str(&not_found(&env).await?);
    } else {
        for post in &posts {
            body.push_str(&content(&env, post).await?);
        }
    }
    // `embed_footer`: the sharing dialog, the template script, then the queued scripts and styles.
    if let Some(first) = posts.first() {
        body.push_str(&sharing_dialog(site, wp_root, first).await?);
    }
    if let Some(js) = read(&wp_root.join("wp-includes").join("js").join("wp-embed-template.min.js")) {
        body.push_str(&inline_script_tag(&js, &format!("{includes_url}/js/wp-embed-template.min.js")));
    }
    let state = env.take_state();
    let mut handles: Vec<String> = Vec::new();
    for name in &state.used_blocks {
        if let Some(block_type) = registry.get(name) {
            for handle in &block_type.styles {
                if !handles.contains(handle) {
                    handles.push(handle.clone());
                }
            }
        }
    }
    for handle in &handles {
        let Some(slug) = handle.strip_prefix("wp-block-") else { continue };
        if registry.blocks_dir().join(slug).join("style.min.css").is_file() {
            body.push_str(&format!("<link rel='stylesheet' id='{handle}-css' href='{includes_url}/blocks/{slug}/style.min.css?ver={version}' media='all' />\n"));
        }
    }
    body.push_str(&emoji_detection_script(site, wp_root, &version));

    let classes = body_classes(theme, site, q).await?;
    let html = format!(
        "<!DOCTYPE html>\n<html {}lang=\"{}\" class=\"no-js\">\n<head>\n{head}</head>\n<body class=\"{}\">\n{body}</body>\n</html>\n",
        if rtl { "dir=\"rtl\" " } else { "" },
        esc_attr(&site.language_tag()),
        esc_attr(&classes.join(" ")),
    );
    let status = if q.flags.is_404 { 404 } else { 200 };
    Ok(Document { status, html })
}
