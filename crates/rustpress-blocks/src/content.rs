//! Post content helpers: `get_the_title()`, `get_the_content()`,
//! `apply_filters( 'the_content' )`, `get_post_class()` and the password form.

use std::sync::Arc;

use rustpress_core::formatting::{capital_p_dangit, convert_chars, esc_attr, esc_url, wpautop, wptexturize};
use rustpress_db::Post;
use serde_json::Value;

use crate::Result;
use crate::env::{Context, Env};
use crate::images::{self, LoadingInput};

/// `post_password_required()`: without cookie support every protected post requires the password.
pub fn post_password_required(post: &Post) -> bool {
    !post.post_password.is_empty()
}

/// `get_the_title()` including the `the_title` filters, in English.
pub fn the_title(post: &Post) -> String {
    title_with_prefixes(post, "Protected: %s", "Private: %s", wptexturize)
}

/// [`the_title`] with the "Protected: %s" / "Private: %s" formats translated for the site.
pub fn the_title_for(site: &rustpress_core::Site, post: &Post) -> String {
    title_with_prefixes(post, &site.tr("Protected: %s"), &site.tr("Private: %s"), |text| site.texturize(text))
}

fn title_with_prefixes(post: &Post, protected: &str, private: &str, texturize: impl Fn(&str) -> String) -> String {
    let mut title = post.post_title.clone();
    if !post.post_password.is_empty() {
        title = rustpress_core::i18n::sprintf(protected, &[&title]);
    } else if post.post_status == "private" {
        title = rustpress_core::i18n::sprintf(private, &[&title]);
    }
    let title = texturize(&title);
    let title = convert_chars(&title);
    // PHP's `trim()` leaves non-ASCII spaces (a trailing no-break space) alone.
    let title = title.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\0' | '\x0B')).to_owned();
    capital_p_dangit(&title, true)
}

/// The `comment_text` filter chain for the standard filters.
pub fn comment_text(site: &rustpress_core::Site, raw: &str) -> String {
    let host = site.settings.home.split_once("://").map_or("", |(_, rest)| rest.split(['/', ':']).next().unwrap_or(""));
    let mut text = rustpress_core::text::make_clickable(raw, &rustpress_core::text::Clickable { home_host: host, comment: true });
    text = site.texturize(&text);
    text = convert_chars(&text);
    text = smilies(site, &text);
    text = wpautop(&text);
    capital_p_dangit(&text, false)
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(input.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// `get_avatar()` for an email address.
pub fn get_avatar(env: &Env<'_>, email: &str, size: u64, alt: &str, extra_class: &str, extra_attr: &str) -> String {
    let options = &env.site.options;
    let default = match options.string("avatar_default", "mystery").as_str() {
        "mm" | "mystery" | "mysteryman" => "mm".to_owned(),
        "gravatar_default" => String::new(),
        other => other.to_owned(),
    };
    let rating = options.string("avatar_rating", "G").to_lowercase();
    let hash = if email.is_empty() { String::new() } else { sha256_hex(&email.trim().to_lowercase()) };
    let url_for = |s: u64| {
        let mut url = format!("https://secure.gravatar.com/avatar/{hash}?s={s}");
        if !default.is_empty() {
            url.push_str(&format!("&d={}", default));
        }
        if !rating.is_empty() {
            url.push_str(&format!("&r={rating}"));
        }
        url
    };
    let mut classes = vec!["avatar".to_owned(), format!("avatar-{size}"), "photo".to_owned()];
    if hash.is_empty() {
        classes.push("avatar-default".to_owned());
    }
    if !extra_class.is_empty() {
        classes.push(extra_class.to_owned());
    }
    // `loading`, `fetchpriority` and `decoding` follow the extra attributes, in this order.
    let input = LoadingInput { width: Some(size), height: Some(size), ..Default::default() };
    let optimization = images::loading_attributes(env, "img", &input, "get_avatar");
    // The loading attributes were worked out (and counted) before the option is looked at.
    if !env.site.show_avatars() {
        return String::new();
    }
    let mut extra_attr = extra_attr.to_owned();
    for (name, allowed) in [("loading", &["lazy", "eager"][..]), ("fetchpriority", &["high", "low", "auto"][..]), ("decoding", &["async", "sync", "auto"][..])] {
        if let Some((_, value)) = optimization.iter().find(|(k, v)| k == name && allowed.contains(&v.as_str())) {
            if !extra_attr.is_empty() {
                extra_attr.push(' ');
            }
            extra_attr.push_str(&format!("{name}='{value}'"));
        }
    }
    format!(
        "<img alt='{}' src='{}' srcset='{} 2x' class='{}' height='{size}' width='{size}' {extra_attr}/>",
        esc_attr(alt),
        esc_url(&url_for(size)),
        esc_url(&url_for(size * 2)),
        esc_attr(&classes.join(" "))
    )
}

/// `get_avatar_url( $email, [ 'size' => $size ] )`: the Gravatar URL with the site's defaults.
pub fn avatar_url(options: &rustpress_core::options::Options, email: &str, size: u64) -> String {
    use sha2::{Digest, Sha256};
    let default = match options.string("avatar_default", "mystery").as_str() {
        "mm" | "mystery" | "mysteryman" => "mm".to_owned(),
        "gravatar_default" => String::new(),
        other => other.to_owned(),
    };
    let rating = options.string("avatar_rating", "G").to_lowercase();
    let hash = if email.is_empty() { String::new() } else { Sha256::digest(email.trim().to_lowercase().as_bytes()).iter().map(|b| format!("{b:02x}")).collect() };
    let mut url = format!("https://secure.gravatar.com/avatar/{hash}?s={size}");
    if !default.is_empty() {
        url.push_str(&format!("&d={default}"));
    }
    if !rating.is_empty() {
        url.push_str(&format!("&r={rating}"));
    }
    url
}

/// `has_blocks()`.
pub fn has_blocks(content: &str) -> bool {
    content.contains("<!-- wp:")
}

/// `get_the_content()`: page selection (`<!--nextpage-->`), the `<!--more-->`
/// teaser and the password form.
pub async fn get_the_content(env: &Env<'_>, post: &Post, page: u64, singular: bool) -> Result<String> {
    get_the_content_with(env, post, page, singular, false).await
}

/// [`get_the_content`]; `no_more_link` is `get_the_content( ' )`: the teaser without a link to the rest.
pub async fn get_the_content_with(env: &Env<'_>, post: &Post, page: u64, singular: bool, no_more_link: bool) -> Result<String> {
    // The permalink is only needed for the password form and the "more" link.
    let permalink = if post_password_required(post) || post.post_content.contains("<!--more") { env.site.permalink(post).await? } else { String::new() };
    Ok(content_with_permalink(env, post, page, singular, no_more_link, &permalink))
}

fn content_with_permalink(env: &Env<'_>, post: &Post, page: u64, singular: bool, no_more_link: bool, permalink: &str) -> String {
    if post_password_required(post) {
        return password_form_for(env, post, permalink);
    }
    let mut content = post.post_content.clone();
    content = content
        .replace("\n<!--nextpage-->\n", "<!--nextpage-->")
        .replace("\n<!--nextpage-->", "<!--nextpage-->")
        .replace("<!--nextpage-->\n", "<!--nextpage-->");
    let pages: Vec<&str> = content.split("<!--nextpage-->").collect();
    // `WP_Query::generate_postdata()`: `$more` is on for the queried post or page, in feeds and on later pages of a post.
    let q = env.query;
    let _ = singular;
    let singular = (q.queried_object_id() == post.id && (q.flags.is_page || q.flags.is_single)) || q.flags.is_feed || (pages.len() > 1 && page > 1);
    let index = (page.max(1) as usize).min(pages.len()) - 1;
    let page_content = pages[index].to_owned();

    if let Some(start) = page_content.find("<!--more") {
        let Some(end_rel) = page_content[start..].find("-->") else { return page_content };
        let marker = &page_content[start..start + end_rel + 3];
        let custom_text = marker.trim_start_matches("<!--more").trim_end_matches("-->").trim().to_owned();
        let cleaned = if has_blocks(&page_content) {
            let mut s = page_content.clone();
            while let Some(i) = s.find("<!-- wp:more") {
                let Some(e) = s[i..].find("-->") else { break };
                s.replace_range(i..i + e + 3, "");
            }
            while let Some(i) = s.find("<!-- /wp:more") {
                let Some(e) = s[i..].find("-->") else { break };
                s.replace_range(i..i + e + 3, "");
            }
            s
        } else {
            page_content.clone()
        };
        let (teaser, rest) = cleaned.split_once(marker).map(|(a, b)| (a.to_owned(), b.to_owned())).unwrap_or((cleaned.clone(), String::new()));
        let mut output = teaser;
        if singular {
            output.push_str(&format!("<span id=\"more-{}\"></span>{rest}", post.id));
        } else if !no_more_link {
            // `$more_link_text`: the default names the post for screen readers; a custom one comes from the tag.
            let text = if custom_text.is_empty() {
                let title = rustpress_core::formatting::esc_attr(&rustpress_core::formatting::strip_tags(&the_title_for(env.site, post)));
                format!(
                    "<span aria-label=\"{}\">{}</span>",
                    rustpress_core::i18n::sprintf(&env.site.tr("Continue reading %s"), &[&title]),
                    env.site.tr("(more&hellip;)")
                )
            } else {
                rustpress_core::formatting::strip_tags(&custom_text)
            };
            output.push_str(&format!(" <a href=\"{}#more-{id}\" class=\"more-link\">{text}</a>", permalink, id = post.id));
        }
        return output;
    }
    page_content
}

/// `get_the_password_form()` for a block theme.
pub async fn password_form(env: &Env<'_>, post: &Post) -> Result<String> {
    let permalink = env.site.permalink(post).await?;
    Ok(password_form_for(env, post, &permalink))
}

fn password_form_for(env: &Env<'_>, post: &Post, permalink: &str) -> String {
    env.with_state(|s| s.note_block("core/button"));
    let field_id = format!("pwbox-{}", post.id);
    format!(
        "<form action=\"{}\" class=\"post-password-form\" method=\"post\"><input type=\"hidden\" name=\"redirect_to\" value=\"{}\" />\n\t<p>{}</p>\n\t<p><label for=\"{field_id}\">{} <input name=\"post_password\" id=\"{field_id}\" type=\"password\" spellcheck=\"false\" required size=\"20\" /></label> <span class=\"wp-block-button\"><input type=\"submit\" name=\"Submit\" class=\"wp-block-button__link wp-element-button\" value=\"{}\" /></span></p></form>\n\t",
        esc_url(&env.site.site_url("wp-login.php?action=postpass")),
        esc_attr(permalink),
        env.site.tr("This content is password-protected. To view it, please enter the password below."),
        env.site.tr("Password:"),
        esc_attr(&env.site.trx("Enter", "post password form")),
    )
}

/// The root block context for rendering content of `post` (`render_block()` with `global $post`).
pub fn context_for_post(post: &Arc<Post>) -> Context {
    let mut ctx = Context::default();
    ctx.values.insert("postId".into(), Value::from(post.id));
    ctx.values.insert("postType".into(), Value::from(post.post_type.clone()));
    ctx.post = Some(post.clone());
    ctx
}

/// `apply_filters( 'the_content', $content )` for `post`.
pub async fn the_content(env: &Env<'_>, post: &Arc<Post>, raw: &str) -> Result<String> {
    the_content_prefixed(env, post, raw, "").await
}

/// [`the_content`] with markup that `prepend_attachment()` puts in front of the content.
pub async fn the_content_prefixed(env: &Env<'_>, post: &Arc<Post>, raw: &str, prefix: &str) -> Result<String> {
    // Everything below runs inside the filter: images made by nested blocks leave the
    // loading attributes to the `wp_filter_content_tags` pass at the end.
    env.with_state(|s| s.media.content_depth += 1);
    let result = apply_the_content(env, post, raw, prefix, true).await;
    env.with_state(|s| s.media.content_depth -= 1);
    result
}

/// The `the_content` filters as `wp_trim_excerpt()` runs them on text that
/// `excerpt_remove_blocks()` has already rendered: `wp_filter_content_tags()` is unhooked,
/// since the tags are stripped anyway and would only disturb the image counting.
pub async fn the_content_for_excerpt(env: &Env<'_>, post: &Arc<Post>, rendered: &str) -> Result<String> {
    env.with_state(|s| s.media.content_depth += 1);
    let result = apply_the_content(env, post, rendered, "", false).await;
    env.with_state(|s| s.media.content_depth -= 1);
    result
}

async fn apply_the_content(env: &Env<'_>, post: &Arc<Post>, raw: &str, prefix: &str, filter_tags: bool) -> Result<String> {
    let raw = crate::shortcodes::run_embed_shortcodes(raw).replace("]]>", "]]&gt;");
    let blocks = has_blocks(&raw);
    let mut out = env.render_markup(&raw, &context_for_post(post)).await?;
    out = env.site.texturize(&out);
    let late = env.with_state(|s| s.wpautop_late);
    let mut prefix_done = false;
    if !blocks {
        if late {
            out = rustpress_core::shortcodes::shortcode_unautop(&out);
            out = wpautop(&format!("{prefix}{out}"));
            prefix_done = true;
        } else {
            out = wpautop(&out);
            out = rustpress_core::shortcodes::shortcode_unautop(&out);
        }
    } else if filter_tags {
        env.with_state(|s| s.wpautop_late = true);
    }
    out = capital_p_dangit(&out, false);
    if !prefix_done {
        out = format!("{prefix}{out}");
    }
    out = crate::shortcodes::do_shortcode(env, Some(post), &out).await?;
    let out = if filter_tags { images::filter_content_tags(env, &out, "the_content").await? } else { out };
    Ok(smilies(env.site, &out))
}

/// `convert_smilies()` when the `use_smilies` option is on.
pub fn smilies(site: &rustpress_core::Site, text: &str) -> String {
    if site.options.get("use_smilies") == Some("1") { rustpress_core::text::convert_smilies(text, &site.site_url("wp-includes")) } else { text.to_owned() }
}

/// `sanitize_html_class`.
fn sanitize_html_class(class: &str) -> String {
    rustpress_core::formatting::sanitize_html_class(class)
}

/// `get_post_class( $extra )`.
pub async fn post_class(env: &Env<'_>, post: &Post, extra: &[&str], is_home_unpaged: bool, _is_singular: bool) -> Result<Vec<String>> {
    let mut classes: Vec<String> = extra.iter().map(|c| (*c).to_owned()).collect();
    classes.push(format!("post-{}", post.id));
    classes.push(post.post_type.clone());
    classes.push(format!("type-{}", post.post_type));
    classes.push(format!("status-{}", post.post_status));
    if post.post_type == "post" {
        let formats = env.site.db.terms_for_post(post.id, Some("post_format")).await?;
        match formats.first().and_then(|t| t.slug.strip_prefix("post-format-")) {
            Some(format) => classes.push(format!("format-{}", sanitize_html_class(format))),
            None => classes.push("format-standard".into()),
        }
    }
    if post_password_required(post) {
        classes.push("post-password-required".into());
    }
    // Block themes always support post thumbnails.
    if post.post_type != "attachment" && !post_password_required(post) && images::post_thumbnail_id(env, post.id).await? != 0 {
        classes.push("has-post-thumbnail".into());
    }
    if env.site.settings.sticky_posts.contains(&post.id) && is_home_unpaged {
        classes.push("sticky".into());
    }
    classes.push("hentry".into());
    // `get_the_terms()` per taxonomy (categories, then tags), each ordered by name.
    let mut terms = env.site.db.terms_for_post(post.id, None).await?;
    let custom = |taxonomy: &str| env.site.types.taxonomy(taxonomy).is_some_and(|t| t.object_types.contains(&post.post_type));
    terms.retain(|t| matches!(t.taxonomy.as_str(), "category" | "post_tag" | "post_format") || custom(&t.taxonomy));
    let rank = |taxonomy: &str| match taxonomy {
        "category" => 0,
        "post_tag" => 1,
        "post_format" => 2,
        other => 3 + env.site.types.taxonomies.iter().position(|t| t.name == other).unwrap_or(0),
    };
    terms.sort_by_key(|t| (rank(&t.taxonomy), t.name.to_lowercase()));
    for term in terms {
        let mut term_class = sanitize_html_class(&term.slug);
        if term_class.is_empty() || term_class.chars().all(|c| c.is_ascii_digit()) || term_class.trim_matches('-').is_empty() {
            term_class = term.term_id.to_string();
        }
        if term.taxonomy == "post_tag" {
            classes.push(format!("tag-{term_class}"));
        } else {
            classes.push(sanitize_html_class(&format!("{}-{term_class}", term.taxonomy)));
        }
    }
    let mut seen = Vec::new();
    classes.retain(|c| {
        if seen.contains(c) {
            false
        } else {
            seen.push(c.clone());
            true
        }
    });
    Ok(classes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_filters() {
        let post = Post { post_title: "Hello \"World\" & Wordpress".into(), ..Post::default() };
        assert_eq!(the_title(&post), "Hello &#8220;World&#8221; &#038; WordPress");
        let protected = Post { post_title: "Geheim".into(), post_password: "x".into(), ..Post::default() };
        assert_eq!(the_title(&protected), "Protected: Geheim");
    }

    #[test]
    fn blocks_are_detected() {
        assert!(has_blocks("a<!-- wp:paragraph -->"));
        assert!(!has_blocks("<p>x</p>"));
    }

    #[test]
    fn html_classes_are_sanitized() {
        assert_eq!(sanitize_html_class("a b%c_d-1"), "abc_d-1");
    }
}
