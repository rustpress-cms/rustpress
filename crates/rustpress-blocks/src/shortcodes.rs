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
        let tags = shortcodes::tags_in(content, CORE_TAGS);
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
    let inner = do_shortcode(env, post, &content).await?;
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
