//! Post content helpers: `get_the_title()`, `get_the_content()`,
//! `apply_filters( 'the_content' )`, `get_post_class()` and the password form.

use std::sync::Arc;

use rustpress_core::formatting::{capital_p_dangit, convert_chars, esc_attr, esc_url, wpautop, wptexturize};
use rustpress_db::Post;
use serde_json::Value;

use crate::Result;
use crate::env::{Context, Env};

/// `post_password_required()`: without cookie support every protected post requires the password.
pub fn post_password_required(post: &Post) -> bool {
    !post.post_password.is_empty()
}

/// `get_the_title()` including the `the_title` filters.
pub fn the_title(post: &Post) -> String {
    let mut title = post.post_title.clone();
    if !post.post_password.is_empty() {
        title = format!("Protected: {title}");
    } else if post.post_status == "private" {
        title = format!("Private: {title}");
    }
    let title = wptexturize(&title);
    let title = convert_chars(&title);
    let title = title.trim().to_owned();
    capital_p_dangit(&title, true)
}

/// `has_blocks()`.
pub fn has_blocks(content: &str) -> bool {
    content.contains("<!-- wp:")
}

/// `get_the_content()`: page selection (`<!--nextpage-->`), the `<!--more-->`
/// teaser and the password form.
pub fn get_the_content(env: &Env<'_>, post: &Post, page: u64, singular: bool) -> String {
    if post_password_required(post) {
        return password_form(env, post);
    }
    let mut content = post.post_content.clone();
    content = content
        .replace("\n<!--nextpage-->\n", "<!--nextpage-->")
        .replace("\n<!--nextpage-->", "<!--nextpage-->")
        .replace("<!--nextpage-->\n", "<!--nextpage-->");
    let pages: Vec<&str> = content.split("<!--nextpage-->").collect();
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
        } else {
            let text = if custom_text.is_empty() { "Continue reading".to_owned() } else { custom_text };
            output.push_str(&format!(" <a href=\"#more-{id}\" class=\"more-link\">{text}</a>", id = post.id));
        }
        return output;
    }
    page_content
}

/// `get_the_password_form()` for a block theme.
pub fn password_form(env: &Env<'_>, post: &Post) -> String {
    env.with_state(|s| s.note_block("core/button"));
    let field_id = format!("pwbox-{}", post.id);
    let permalink = permalink_sync(env, post);
    format!(
        "<form action=\"{}\" class=\"post-password-form\" method=\"post\"><input type=\"hidden\" name=\"redirect_to\" value=\"{}\" />\n\t<p>This content is password-protected. To view it, please enter the password below.</p>\n\t<p><label for=\"{field_id}\">Password: <input name=\"post_password\" id=\"{field_id}\" type=\"password\" spellcheck=\"false\" required size=\"20\" /></label> <span class=\"wp-block-button\"><input type=\"submit\" name=\"Submit\" class=\"wp-block-button__link wp-element-button\" value=\"Enter\" /></span></p></form>\n\t",
        esc_url(&env.site.site_url("wp-login.php?action=postpass")),
        esc_attr(&permalink),
    )
}

/// Best-effort permalink without database access (used for the password form's redirect field).
fn permalink_sync(env: &Env<'_>, post: &Post) -> String {
    let structure = &env.site.settings.permalink_structure;
    if structure.is_empty() || post.post_type != "post" {
        return env.site.home_url(&format!("?p={}", post.id));
    }
    if structure.contains("%postname%") && !structure.contains("%category%") && !structure.contains("%author%") {
        let date = &post.post_date;
        let path = structure
            .replace("%postname%", &post.post_name)
            .replace("%post_id%", &post.id.to_string())
            .replace("%year%", date.get(0..4).unwrap_or(""))
            .replace("%monthnum%", date.get(5..7).unwrap_or(""))
            .replace("%day%", date.get(8..10).unwrap_or(""));
        return env.site.user_trailingslashit(&env.site.home_url(&path));
    }
    env.site.home_url(&format!("?p={}", post.id))
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
    let raw = raw.replace("]]>", "]]&gt;");
    let blocks = has_blocks(&raw);
    let mut out = env.render_markup(&raw, &context_for_post(post)).await?;
    out = wptexturize(&out);
    if !blocks {
        out = wpautop(&out);
    }
    out = capital_p_dangit(&out, false);
    Ok(out)
}

/// `sanitize_html_class`.
fn sanitize_html_class(class: &str) -> String {
    class.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect()
}

/// `get_post_class( $extra )`.
pub async fn post_class(env: &Env<'_>, post: &Post, extra: &[&str], is_home_unpaged: bool, _is_singular: bool) -> Result<Vec<String>> {
    let mut classes: Vec<String> = extra.iter().map(|c| (*c).to_owned()).collect();
    classes.push(format!("post-{}", post.id));
    classes.push(post.post_type.clone());
    classes.push(format!("type-{}", post.post_type));
    classes.push(format!("status-{}", post.post_status));
    if post.post_type == "post" {
        classes.push("format-standard".into());
    }
    if post_password_required(post) {
        classes.push("post-password-required".into());
    }
    if env.site.settings.sticky_posts.contains(&post.id) && is_home_unpaged {
        classes.push("sticky".into());
    }
    classes.push("hentry".into());
    for term in env.site.db.terms_for_post(post.id, None).await? {
        if !matches!(term.taxonomy.as_str(), "category" | "post_tag") {
            continue;
        }
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
