//! `do_shortcode()` and the shortcodes WordPress registers itself (`caption`, `gallery`, ...).
//! Shortcodes of plugins stay in the text: there is no PHP to run them.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use regex::Regex;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url, sanitize_html_class};
use rustpress_core::shortcodes::{self, Atts, CORE_TAGS, Found};
use rustpress_db::{Order, OrderBy, Post, PostQuery};

use crate::Result;
use crate::content::post_password_required;
use crate::env::Env;
use crate::images::{self, AttrValue, ImageArgs};

/// `(int) $string`: the leading digits.
fn php_int(s: &str) -> i64 {
    let s = s.trim_start();
    let end = s.char_indices().find(|(i, c)| !(c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+')))).map_or(s.len(), |(i, _)| i);
    s[..end].parse().unwrap_or(0)
}

/// `empty( $value )` for a string.
fn is_empty(s: &str) -> bool {
    s.is_empty() || s == "0"
}

/// `do_shortcode( $content )`. `post` is `global $post`.
pub fn do_shortcode<'a, 'e>(env: &'a Env<'e>, post: Option<&'a Arc<Post>>, content: &'a str) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
    Box::pin(async move {
        if !content.contains('[') {
            return Ok(content.to_owned());
        }
        let registered = env.site.shortcode_tags();
        let tags = shortcodes::tags_in(content, &registered);
        if tags.is_empty() {
            return Ok(content.to_owned());
        }
        let content = shortcodes::protect_html(content, false);
        let mut out = String::with_capacity(content.len());
        let mut at = 0;
        while let Some(m) = shortcodes::find(&content, at, &tags) {
            out.push_str(&content[at..m.start]);
            out.push_str(&do_shortcode_tag(env, post, &content, &m).await?);
            at = m.end;
        }
        out.push_str(&content[at..]);
        Ok(shortcodes::unescape_invalid(&out))
    })
}

/// `do_shortcode_tag()`.
async fn do_shortcode_tag(env: &Env<'_>, post: Option<&Arc<Post>>, text: &str, m: &Found) -> Result<String> {
    // `[[tag]]` is the way to write a shortcode literally.
    if m.open_escape && m.close_escape {
        return Ok(text[m.start + 1..m.end - 1].to_owned());
    }
    let atts = shortcodes::parse_atts(&m.atts);
    let output = match m.name.as_str() {
        "caption" | "wp_caption" => Some(img_caption_shortcode(env, post, atts, m.content.as_deref().unwrap_or("")).await?),
        "gallery" => Some(gallery_shortcode(env, post, atts).await?),
        "playlist" => Some(playlist_shortcode(env, post, atts).await?),
        "audio" => Some(crate::media_shortcodes::audio(env, post.map_or(0, |p| p.id), &atts).unwrap_or_default()),
        "video" => Some(crate::media_shortcodes::video(env, post.map_or(0, |p| p.id), &atts, m.content.as_deref().unwrap_or("")).unwrap_or_default()),
        // `WP_Embed` registers `[embed]` as `__return_false`; the real work happens before `wpautop()`.
        "embed" => Some(String::new()),
        name if !CORE_TAGS.contains(&name) => {
            let (plugins, outside_loop) = env.with_state(|s| (std::mem::take(&mut s.plugins), s.outside_loop));
            let state = std::sync::Mutex::new(plugins);
            let loop_post = if outside_loop { None } else { post.map(|p| p.id) };
            let in_the_content = env.with_state(|s| s.media.content_depth > 0);
            let ctx = rustpress_core::native::ShortcodeContext { site: env.site, query: env.query, loop_post, global_post: post.map(|p| p.id), in_the_content, block_theme: env.theme.is_block_theme(), state: &state, services: env };
            let output = env.site.plugins.shortcode(name, &atts, m.content.as_deref(), &ctx).await;
            env.with_state(|s| s.plugins = state.into_inner().unwrap_or_default());
            output?
        }
        _ => None,
    };
    let Some(output) = output else { return Ok(text[m.start..m.end].to_owned()) };
    Ok(format!("{}{output}{}", if m.open_escape { "[" } else { "" }, if m.close_escape { "]" } else { "" }))
}

/// `img_caption_shortcode()` for a theme with HTML5 captions.
async fn img_caption_shortcode(env: &Env<'_>, post: Option<&Arc<Post>>, mut atts: Atts, content: &str) -> Result<String> {
    let mut content = content.to_owned();
    // New-style shortcode with the caption inside the shortcode with the link and image tags.
    if atts.get("caption").is_none() {
        static IMAGE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"(?is)((?:<a [^>]+>\s*)?<img [^>]+>(?:\s*</a>)?)(.*)").unwrap());
        if let Some(c) = IMAGE.captures(&content) {
            let (image, caption) = (c[1].to_owned(), c[2].trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B']).to_owned());
            atts.set("caption", &caption);
            content = image;
        }
    }
    let get = |name: &str, default: &str| atts.get(name).unwrap_or(default).to_owned();
    let (id, caption_id, align, width, caption, class) = (get("id", ""), get("caption_id", ""), get("align", "alignnone"), php_int(&get("width", "")), get("caption", ""), get("class", ""));
    if width < 1 || is_empty(&caption) {
        return Ok(content);
    }
    let mut id_attr = String::new();
    let mut unique_id = String::new();
    let mut caption_id_value = String::new();
    if !is_empty(&id) {
        let id = sanitize_html_class(&id);
        unique_id = env.with_state(|s| s.unique_prefixed_id(&format!("{id}-")));
        if let Some(stripped) = unique_id.strip_suffix("-1") {
            unique_id = stripped.to_owned();
        }
        id_attr = format!("id=\"{}\" ", esc_attr(&unique_id));
    }
    if !is_empty(&caption_id) {
        let caption_id = sanitize_html_class(&caption_id);
        caption_id_value = env.with_state(|s| s.unique_prefixed_id(&format!("{caption_id}-")));
        if let Some(stripped) = caption_id_value.strip_suffix("-1") {
            caption_id_value = stripped.to_owned();
        }
    } else if !unique_id.is_empty() {
        caption_id_value = format!("caption-{}", unique_id.replace('_', "-"));
    }
    let (caption_id_attr, describedby) = if caption_id_value.is_empty() {
        (String::new(), String::new())
    } else {
        (format!("id=\"{}\" ", esc_attr(&caption_id_value)), format!("aria-describedby=\"{}\" ", esc_attr(&caption_id_value)))
    };
    let class = format!("wp-caption {align} {class}");
    let class = class.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B']);
    // HTML5 captions never added the extra 10px to the image width.
    let style = format!("style=\"width: {width}px\" ");
    // Captions inside captions are expanded as deep as blocks are rendered, no deeper.
    let depth = env.with_state(|s| {
        s.render_depth += 1;
        s.render_depth
    });
    let inner = if depth > crate::render::MAX_RENDER_DEPTH { Ok(content.clone()) } else { do_shortcode(env, post, &content).await };
    env.with_state(|s| s.render_depth -= 1);
    let inner = inner?;
    Ok(format!(
        "<figure {id_attr}{describedby}{style}class=\"{}\">{inner}<figcaption {caption_id_attr}class=\"wp-caption-text\">{caption}</figcaption></figure>",
        esc_attr(class)
    ))
}

/// The tags `wp_kses_allowed_html( 'post' )` allows for the gallery's item tags.
fn allowed_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address" | "article" | "aside" | "blockquote" | "caption" | "cite" | "code" | "dd" | "div" | "dl" | "dt" | "em" | "figcaption" | "figure" | "footer" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "header" | "li" | "main" | "nav" | "ol" | "p" | "pre" | "section" | "span" | "strong" | "ul"
    )
}

/// `tag_escape()`.
fn tag_escape(tag: &str) -> String {
    tag.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':').collect::<String>().to_lowercase()
}

/// `wp_get_attachment_link( $id, $size, $permalink, false, false, $attr )`.
pub async fn attachment_link_html(env: &Env<'_>, attachment: &Post, size: &str, permalink: bool, args: &ImageArgs) -> Result<String> {
    let site = env.site;
    let (meta, attached) = images::load_metadata(site, attachment).await?;
    let Some(attached) = attached else { return Ok(site.tr("Missing Attachment")) };
    let _ = meta;
    let mut url = images::attachment_url(site, &attached);
    if permalink {
        url = site.permalink(attachment).await?;
    }
    let mut text = if !size.is_empty() && size != "none" { images::get_attachment_image(env, attachment.id, size, args).await? } else { String::new() };
    if text.trim().is_empty() {
        text = attachment.post_title.clone();
    }
    if text.trim().is_empty() {
        let file = attached.rsplit('/').next().unwrap_or_default();
        text = esc_html(file.rsplit_once('.').map_or(file, |(stem, _)| stem));
    }
    Ok(format!("<a href='{}'>{text}</a>", esc_url(&url)))
}

/// `gallery_shortcode()` for a theme with HTML5 galleries.
/// `get_post_galleries( $post )`: the galleries of the shortcodes, then those of the blocks.
pub async fn post_galleries(env: &Env<'_>, post: &Arc<Post>) -> Result<Vec<String>> {
    let content = &post.post_content;
    let has_block = content.contains("<!-- wp:gallery");
    let mut galleries = Vec::new();
    let registered = env.site.shortcode_tags();
    let mut at = 0;
    while let Some(found) = shortcodes::find(content, at, &registered) {
        at = found.end;
        if found.name == "gallery" {
            let mut atts = shortcodes::parse_atts(&found.atts);
            // The gallery of the post, unless the shortcode names another one.
            if atts.get("id").is_none() {
                atts.set("id", &post.id.to_string());
            }
            galleries.push(gallery_shortcode(env, Some(post), atts).await?);
        }
    }
    if has_block {
        let mut queue: std::collections::VecDeque<crate::parser::Block> = crate::parser::parse_blocks(content).into();
        while let Some(block) = queue.pop_front() {
            match block.name.as_deref() {
                None => {}
                Some("core/gallery") if block.inner_blocks.is_empty() => galleries.push(block.inner_html.clone()),
                Some("core/gallery") => {
                    let images: Vec<&str> = block.inner_blocks.iter().map(|b| b.inner_html.as_str()).collect();
                    galleries.push(format!("<figure>{}</figure>", images.join(" ")));
                }
                Some(_) => queue.extend(block.inner_blocks),
            }
        }
    }
    Ok(galleries)
}

/// `wp_get_attachment_image_src( $id, $size )` as JSON.
async fn image_source(env: &Env<'_>, id: u64, size: &str) -> Result<Option<serde_json::Value>> {
    let Some(attachment) = env.site.db.post(id).await? else { return Ok(None) };
    let (meta, attached) = images::load_metadata(env.site, &attachment).await?;
    let (Some(meta), Some(attached)) = (meta, attached) else { return Ok(None) };
    let image = images::image_downsize(&images::attachment_url(env.site, &attached), &meta, size);
    Ok(Some(serde_json::json!({ "src": image.url, "width": image.width, "height": image.height })))
}

/// `wp_playlist_shortcode()`.
async fn playlist_shortcode(env: &Env<'_>, post: Option<&Arc<Post>>, mut atts: Atts) -> Result<String> {
    use serde_json::{Map, Value, json};
    let site = env.site;
    env.with_state(|s| s.playlist_instances += 1);
    if let Some(ids) = atts.non_empty("ids").map(str::to_owned) {
        // 'ids' is explicitly ordered, unless you specify otherwise.
        if atts.non_empty("orderby").is_none() {
            atts.set("orderby", "post__in");
        }
        atts.set("include", &ids);
    }
    let get = |name: &str, default: &str| atts.get(name).unwrap_or(default).to_owned();
    let (order, orderby) = (get("order", "ASC"), get("orderby", "menu_order ID"));
    let id = php_int(&get("id", &post.map_or(0, |p| p.id).to_string())) as u64;
    let (include, exclude, style) = (get("include", ""), get("exclude", ""), get("style", "light"));
    let kind = if get("type", "audio") == "audio" { "audio" } else { "video" };
    let flag = |name: &str| atts.get(name).is_none_or(crate::media_shortcodes::validate_boolean);
    let id_list = |list: &str| -> Vec<u64> { list.split(|c: char| !c.is_ascii_alphanumeric()).filter(|s| !s.is_empty()).map(|s| php_int(s) as u64).collect() };

    let mut query = PostQuery { post_types: vec!["attachment".into()], statuses: vec!["inherit".into()], mime_prefix: Some(kind.into()), ..PostQuery::default() };
    query.order = if order.eq_ignore_ascii_case("DESC") { Order::Desc } else { Order::Asc };
    query.order_by = match orderby.to_lowercase().as_str() {
        "post__in" => OrderBy::Include,
        "title" => OrderBy::Title,
        "id" => OrderBy::Id,
        "date" => OrderBy::Date,
        _ => OrderBy::MenuOrder,
    };
    let mut parent = 0;
    if !is_empty(&include) {
        query.ids = id_list(&include);
        if query.ids.is_empty() {
            return Ok(String::new());
        }
        // `get_posts()` shows exactly the included posts.
        query.limit = Some(query.ids.len() as u64);
    } else {
        parent = id;
        query.parent = Some(id);
        if !is_empty(&exclude) {
            query.exclude_ids = id_list(&exclude);
        }
    }
    let attachments = site.db.posts(&query).await?;
    if parent != 0 {
        // Terminate the shortcode execution if the user cannot read the post or it is password-protected. A visitor
        // has no `read` capability, so `current_user_can( 'read_post', ... )` fails for everybody who is not signed in.
        let readable = match (&env.query.viewer, site.db.post(parent).await?) {
            (Some(viewer), Some(p)) => viewer.can("read") && viewer.can_read_post(&p) && !post_password_required(&p),
            _ => false,
        };
        if !readable {
            return Ok(String::new());
        }
    }
    if attachments.is_empty() {
        return Ok(String::new());
    }
    let mut links = String::new();
    if env.query.flags.is_feed {
        let mut output = String::from("\n");
        for attachment in &attachments {
            output.push_str(&attachment_link_html(env, attachment, "thumbnail", false, &ImageArgs::default()).await?);
            output.push('\n');
        }
        return Ok(output);
    }

    // Default padding and border of the wrapper: 22 pixels.
    let content_width = env.with_state(|s| s.content_width);
    let default_height = 360_u64;
    let theme_width = if content_width == 0 { 640 } else { content_width.saturating_sub(22) };
    let mut theme_height = if content_width == 0 { default_height } else { (default_height as f64 * theme_width as f64 / 640.0).round() as u64 };
    let mut tracks = Vec::new();
    for attachment in &attachments {
        let (meta, attached) = images::load_metadata(site, attachment).await?;
        let url = attached.map_or_else(|| attachment.guid.clone(), |file| images::attachment_url(site, &file));
        let mut track = Map::new();
        track.insert("src".into(), Value::String(url.clone()));
        track.insert("type".into(), crate::media_shortcodes::media_type(&url).map_or(Value::Bool(false), Value::from));
        track.insert("title".into(), Value::String(attachment.post_title.clone()));
        track.insert("caption".into(), Value::String(attachment.post_excerpt.clone()));
        track.insert("description".into(), Value::String(attachment.post_content.clone()));
        let mut track_meta = Map::new();
        if let Some(meta) = &meta {
            // `wp_get_attachment_id3_keys()`.
            for key in ["artist", "album", "genre", "year", "length_formatted"] {
                if let Some(value) = meta.0.get(key).filter(|v| !crate::style_engine::is_empty(v) && v.as_str() != Some("0")) {
                    track_meta.insert(key.to_owned(), value.clone());
                }
            }
        }
        track.insert("meta".into(), Value::Object(track_meta));
        if let (Some(meta), "video") = (&meta, kind) {
            let (width, height) = (meta.width(), meta.height());
            let (width, height) = if width != 0 && height != 0 {
                theme_height = (height as f64 * theme_width as f64 / width as f64).round() as u64;
                (Value::from(width), Value::from(height))
            } else {
                (Value::from(640), Value::from(default_height))
            };
            track.insert("dimensions".into(), json!({ "original": { "width": width, "height": height }, "resized": { "width": theme_width, "height": theme_height } }));
        }
        // The string itself decides here: `images="false"` counts as true.
        if atts.get("images").is_none_or(|images| !images.is_empty() && images != "0") {
            let thumb_id = images::post_thumbnail_id(env, attachment.id).await?;
            if thumb_id != 0 {
                if let Some(image) = image_source(env, thumb_id, "full").await? {
                    track.insert("image".into(), image);
                }
                if let Some(thumb) = image_source(env, thumb_id, "thumbnail").await? {
                    track.insert("thumb".into(), thumb);
                }
            } else {
                // `wp_mime_type_icon( $id, '.svg' )`.
                let icon = json!({ "src": format!("{}/wp-includes/images/media/{kind}.svg", site.settings.siteurl.trim_end_matches('/')), "width": 48, "height": 64 });
                track.insert("image".into(), icon.clone());
                track.insert("thumb".into(), icon);
            }
        }
        tracks.push(Value::Object(track));
        links.push_str(&format!("<li>{}</li>", attachment_link_html(env, attachment, "thumbnail", false, &ImageArgs::default()).await?));
    }
    let data = json!({
        "type": kind,
        "tracklist": flag("tracklist"),
        "tracknumbers": flag("tracknumbers"),
        "images": flag("images"),
        "artists": flag("artists"),
        "tracks": tracks,
    });
    // JSON_HEX_TAG | JSON_UNESCAPED_SLASHES.
    let encoded = crate::php_json::encode_with(&data, crate::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E");
    env.with_state(|s| {
        if !s.playlist {
            s.playlist = true;
            s.note_block("wp:mediaelement");
        }
    });
    let current = if kind == "audio" { "\t\t<div class=\"wp-playlist-current-item\"></div>\n\t" } else { "" };
    let height = if kind == "video" { format!(" height=\"{theme_height}\"") } else { String::new() };
    Ok(format!(
        "<div class=\"wp-playlist wp-{kind}-playlist wp-playlist-{}\">\n\t{current}\t<{kind} controls=\"controls\" preload=\"none\" width=\"{}\"\n\t\t{height}\t></{kind}>\n\t<div class=\"wp-playlist-next\"></div>\n\t<div class=\"wp-playlist-prev\"></div>\n\t<noscript>\n\t<ol>\n\t\t{links}\t</ol>\n\t</noscript>\n\t<script type=\"application/json\" class=\"wp-playlist-script\">{encoded}</script>\n</div>\n\t",
        rustpress_core::formatting::esc_attr(&style),
        theme_width,
    ))
}

/// `wp_underscore_playlist_templates()` on `wp_footer`, before everything else there.
pub fn playlist_templates(site: &rustpress_core::Site) -> String {
    let title = site.trx("&#8220;%s&#8221;", "playlist item title");
    let templates = include_str!("playlist_templates.html")
        .replace("{$title:{{ data.title }}}", &rustpress_core::i18n::sprintf(&title, &["{{ data.title }}"]))
        .replace("{$title:{{{ data.title }}}}", &rustpress_core::i18n::sprintf(&title, &["{{{ data.title }}}"]));
    format!("{templates}\t")
}

async fn gallery_shortcode(env: &Env<'_>, post: Option<&Arc<Post>>, mut atts: Atts) -> Result<String> {
    let site = env.site;
    let instance = env.with_state(|s| {
        s.gallery_instances += 1;
        s.gallery_instances
    });
    if let Some(ids) = atts.non_empty("ids").map(str::to_owned) {
        // 'ids' is explicitly ordered, unless you specify otherwise.
        if atts.non_empty("orderby").is_none() {
            atts.set("orderby", "post__in");
        }
        atts.set("include", &ids);
    }
    let get = |name: &str, default: &str| atts.get(name).unwrap_or(default).to_owned();
    let order = get("order", "ASC");
    let orderby = get("orderby", "menu_order ID");
    let id = php_int(&get("id", &post.map_or(0, |p| p.id).to_string())) as u64;
    let (include, exclude, link, size) = (get("include", ""), get("exclude", ""), get("link", ""), get("size", "thumbnail"));
    let id_list = |list: &str| -> Vec<u64> { list.split(|c: char| !c.is_ascii_alphanumeric()).filter(|s| !s.is_empty()).map(|s| php_int(s) as u64).filter(|n| *n > 0).collect() };

    let mut query = PostQuery { post_types: vec!["attachment".into()], statuses: vec!["inherit".into()], mime_prefix: Some("image".into()), ..PostQuery::default() };
    query.order = if order.eq_ignore_ascii_case("DESC") { Order::Desc } else { Order::Asc };
    query.order_by = match orderby.to_lowercase().as_str() {
        "post__in" => OrderBy::Include,
        "title" => OrderBy::Title,
        "id" => OrderBy::Id,
        "date" => OrderBy::Date,
        "rand" | "menu_order id" | "menu_order" => OrderBy::MenuOrder,
        _ => OrderBy::MenuOrder,
    };
    let mut parent = 0;
    if !is_empty(&include) {
        query.ids = id_list(&include);
        if query.ids.is_empty() {
            return Ok(String::new());
        }
        // `get_posts()` shows exactly the included posts.
        query.limit = Some(query.ids.len() as u64);
    } else {
        parent = id;
        query.parent = Some(id);
        if !is_empty(&exclude) {
            query.exclude_ids = id_list(&exclude);
        }
    }
    let attachments = site.db.posts(&query).await?;

    if parent != 0 {
        // Terminate the shortcode execution if the post is not public or is password-protected.
        match site.db.post(parent).await? {
            Some(p) if p.post_status == "publish" && !post_password_required(&p) => {}
            _ => return Ok(String::new()),
        }
    }
    if attachments.is_empty() {
        return Ok(String::new());
    }

    if env.query.flags.is_feed {
        let mut output = String::from("\n");
        for attachment in &attachments {
            if !link.is_empty() {
                if link == "none" {
                    output.push_str(&images::get_attachment_image(env, attachment.id, &size, &ImageArgs::default()).await?);
                } else {
                    output.push_str(&attachment_link_html(env, attachment, &size, false, &ImageArgs::default()).await?);
                }
            } else {
                output.push_str(&attachment_link_html(env, attachment, &size, true, &ImageArgs::default()).await?);
            }
            output.push('\n');
        }
        return Ok(output);
    }

    let tag = |name: &str, default: &str, fallback: &str| {
        let t = tag_escape(&get(name, default));
        if allowed_tag(&t) { t } else { fallback.to_owned() }
    };
    let itemtag = tag("itemtag", "figure", "dl");
    let captiontag = tag("captiontag", "figcaption", "dd");
    let icontag = tag("icontag", "div", "dt");
    let columns = php_int(&get("columns", "3"));
    let selector = format!("gallery-{instance}");
    let size_class = sanitize_html_class(&size);
    let mut output = format!("<div id='{selector}' class='gallery galleryid-{id} gallery-columns-{columns} gallery-size-{size_class}'>");

    for attachment in &attachments {
        let args = if attachment.post_excerpt.trim().is_empty() {
            ImageArgs::default()
        } else {
            ImageArgs { attrs: vec![("aria-describedby".into(), AttrValue::Text(format!("{selector}-{}", attachment.id)))], ..ImageArgs::default() }
        };
        let image_output = if link == "file" {
            attachment_link_html(env, attachment, &size, false, &args).await?
        } else if link == "none" {
            images::get_attachment_image(env, attachment.id, &size, &args).await?
        } else {
            attachment_link_html(env, attachment, &size, true, &args).await?
        };
        let orientation = match images::attachment_metadata(env, attachment.id).await? {
            Some(meta) if meta.0.get("height").is_some() && meta.0.get("width").is_some() => {
                if meta.height() > meta.width() { "portrait" } else { "landscape" }
            }
            _ => "",
        };
        output.push_str(&format!("<{itemtag} class='gallery-item'>"));
        output.push_str(&format!("\n\t\t\t<{icontag} class='gallery-icon {orientation}'>\n\t\t\t\t{image_output}\n\t\t\t</{icontag}>"));
        if !attachment.post_excerpt.trim().is_empty() {
            output.push_str(&format!(
                "\n\t\t\t\t<{captiontag} class='wp-caption-text gallery-caption' id='{selector}-{}'>\n\t\t\t\t{}\n\t\t\t\t</{captiontag}>",
                attachment.id,
                site.texturize(&attachment.post_excerpt)
            ));
        }
        output.push_str(&format!("</{itemtag}>"));
    }
    output.push_str("\n\t\t</div>\n");
    Ok(output)
}


/// `WP_Embed::run_shortcode()` (`the_content` priority 8): `[embed]` runs before the blocks and `wpautop()`.
pub fn run_embed_shortcodes(content: &str) -> String {
    if !content.contains("[embed") {
        return content.to_owned();
    }
    let tags = ["embed"];
    let content = shortcodes::protect_html(content, true);
    let mut out = String::with_capacity(content.len());
    let mut at = 0;
    while let Some(m) = shortcodes::find(&content, at, &tags) {
        out.push_str(&content[at..m.start]);
        if m.open_escape && m.close_escape {
            out.push_str(&content[m.start + 1..m.end - 1]);
        } else {
            out.push_str(&format!("{}{}{}", if m.open_escape { "[" } else { "" }, crate::media_shortcodes::embed(m.content.as_deref().unwrap_or("")), if m.close_escape { "]" } else { "" }));
        }
        at = m.end;
    }
    out.push_str(&content[at..]);
    shortcodes::unescape_invalid(&out)
}