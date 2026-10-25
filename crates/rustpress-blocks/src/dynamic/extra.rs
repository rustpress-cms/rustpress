//! More dynamic blocks: archives, categories, tag cloud, calendar, latest comments, home link,
//! login/out, comment counts and links, author biography, time to read, read more, shortcode,
//! the term blocks and `core/block`.

use std::sync::Arc;

use regex::Regex;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url, strip_tags, wpautop};
use rustpress_core::i18n::sprintf;
use serde_json::Value;

use crate::Result;
use crate::content::{get_avatar, post_password_required, the_title_for};
use crate::env::Context;
use crate::general::{self, Archives, DropdownCategories, ListCategories, TagCloud, TermsArgs};
use crate::parser::Content;
use crate::render::BlockCtx;

use super::global_post;

fn flag(bc: &BlockCtx<'_>, key: &str) -> bool {
    bc.attrs.get(key).is_some_and(|v| !crate::style_engine::is_empty(v))
}

fn has_link_color(bc: &BlockCtx<'_>) -> bool {
    bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null())
}

fn align_class(bc: &BlockCtx<'_>) -> String {
    bc.attr_str("textAlign").filter(|a| !a.is_empty()).map(|a| format!("has-text-align-{a}")).unwrap_or_default()
}

// ---------------------------------------------------------------------- archives

pub(super) async fn archives(bc: &BlockCtx<'_>) -> Result<String> {
    let env = bc.env;
    let site = env.site;
    let kind = bc.attr_str("type").filter(|t| !t.is_empty()).unwrap_or("monthly").to_owned();
    let show_count = flag(bc, "showPostCounts");
    if flag(bc, "displayAsDropdown") {
        let id = env.with_state(|s| s.unique_id("wp-block-archives-"));
        let list = general::wp_get_archives(env, &Archives { kind: kind.clone(), format: "option".into(), show_post_count: show_count, ..Archives::new() }).await?;
        let label = match kind.as_str() {
            "yearly" => site.tr("Select Year"),
            "monthly" => site.tr("Select Month"),
            "daily" => site.tr("Select Day"),
            "weekly" => site.tr("Select Week"),
            _ => site.tr("Select Post"),
        };
        let show_label = if flag(bc, "showLabel") { "" } else { " screen-reader-text" };
        let mut html = format!(
            "<label for=\"{id}\" class=\"wp-block-archives__label{show_label}\">{}</label>\n\t\t<select id=\"{}\" name=\"archive-dropdown\">\n\t\t<option value=\"\">{}</option>{list}</select>",
            esc_html(&site.tr("Archives")),
            esc_attr(&id),
            esc_html(&label)
        );
        html.push_str(&general::dropdown_script(general::ARCHIVES_DROPDOWN_SCRIPT, &id, &site.home_url(""), "block_core_archives_build_dropdown_script"));
        return Ok(format!("<div {}>{html}</div>", bc.wrapper_attributes(&[("class", "wp-block-archives-dropdown".to_owned())])));
    }
    let list = general::wp_get_archives(env, &Archives { kind, show_post_count: show_count, ..Archives::new() }).await?;
    let wrapper = bc.wrapper_attributes(&[("class", "wp-block-archives-list".to_owned())]);
    if list.is_empty() {
        return Ok(format!("<div {wrapper}>{}</div>", site.tr("No archives to show.")));
    }
    Ok(format!("<ul {wrapper}>{list}</ul>"))
}

// -------------------------------------------------------------------- categories

/// `get_taxonomy()` data of the built-in taxonomies: query var, singular name, name, "no terms".
fn taxonomy_labels(env: &crate::env::Env<'_>, taxonomy: &str) -> (String, String, String, String) {
    let site = env.site;
    match taxonomy {
        "post_tag" => ("tag".into(), site.trx("Tag", "taxonomy singular name"), site.trx("Tags", "taxonomy general name"), site.tr("No tags")),
        _ => ("category_name".into(), site.trx("Category", "taxonomy singular name"), site.trx("Categories", "taxonomy general name"), site.tr("No categories")),
    }
}

pub(super) async fn categories(bc: &BlockCtx<'_>) -> Result<String> {
    let env = bc.env;
    let site = env.site;
    let block_id = env.with_state(|s| s.unique_prefixed_id("wp-block-categories-"));
    let taxonomy = bc.attr_str("taxonomy").filter(|t| !t.is_empty()).unwrap_or("category").to_owned();
    let (query_var, singular, label, no_terms) = taxonomy_labels(env, &taxonomy);
    let mut terms = TermsArgs::new(&taxonomy);
    terms.hierarchical = flag(bc, "showHierarchy");
    terms.hide_empty = !flag(bc, "showEmpty");
    if flag(bc, "showOnlyTopLevel") {
        terms.parent = Some(0);
    }
    let show_count = flag(bc, "showPostCounts");
    if flag(bc, "displayAsDropdown") {
        let selected = env.query.vars.get(&query_var).unwrap_or_default().to_owned();
        let args = DropdownCategories {
            terms,
            show_count,
            depth: 0,
            show_option_none: sprintf(&site.tr("Select %s"), &[&singular]),
            show_option_all: String::new(),
            option_none_value: "-1".into(),
            name: query_var.clone(),
            id: block_id.clone(),
            class: "postform".into(),
            value_field_slug: true,
            selected,
            hide_if_empty: false,
            required: false,
        };
        let mut items = general::wp_dropdown_categories(env, &args).await?;
        let script = general::dropdown_script(general::CATEGORIES_DROPDOWN_SCRIPT, &block_id, &site.home_url(""), "build_dropdown_script_block_core_categories");
        if let Some(i) = items.find("</select>") {
            items.insert_str(i + "</select>".len(), &script);
        }
        let show_label = if flag(bc, "showLabel") { "" } else { " screen-reader-text" };
        let label_text = bc.attr_str("label").filter(|l| !l.is_empty()).map(str::to_owned).unwrap_or(label);
        let wrapper = bc.wrapper_attributes(&[("class", format!("wp-block-categories-dropdown wp-block-categories-taxonomy-{taxonomy}"))]);
        return Ok(format!("<div {wrapper}><label class=\"wp-block-categories__label{show_label}\" for=\"{}\">{label_text}</label>{items}</div>", esc_attr(&block_id)));
    }
    let mut args = ListCategories::new(env, &taxonomy);
    args.terms = terms;
    args.show_count = show_count;
    args.title_li = String::new();
    args.show_option_none = no_terms;
    let items = general::wp_list_categories(env, &args).await?;
    let wrapper = bc.wrapper_attributes(&[("class", format!("wp-block-categories-list wp-block-categories-taxonomy-{taxonomy}"))]);
    Ok(format!("<ul {wrapper}>{items}</ul>"))
}

// ------------------------------------------------------------------- tag cloud

pub(super) async fn tag_cloud(bc: &BlockCtx<'_>) -> Result<String> {
    let smallest = bc.attr_str("smallestFontSize").unwrap_or("8pt").to_owned();
    let largest = bc.attr_str("largestFontSize").unwrap_or("22pt").to_owned();
    let unit = Regex::new(r"(?i)^[0-9.]+([a-z%]+)$").ok().and_then(|re| re.captures(&smallest).map(|c| c[1].to_owned())).unwrap_or_else(|| "pt".to_owned());
    let args = TagCloud {
        taxonomy: bc.attr_str("taxonomy").filter(|t| !t.is_empty()).unwrap_or("post_tag").to_owned(),
        smallest: general::floatval(&smallest),
        largest: general::floatval(&largest),
        unit,
        number: bc.attr_u64("numberOfTags").unwrap_or(45),
        show_count: flag(bc, "showTagCounts"),
        ..TagCloud::new()
    };
    let cloud = general::wp_tag_cloud(bc.env, &args).await?;
    if cloud.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("<p {}>{cloud}</p>", bc.wrapper_attributes(&[])))
}

// ---------------------------------------------------------------------- calendar

pub(super) async fn calendar(bc: &BlockCtx<'_>) -> Result<String> {
    let env = bc.env;
    let site = env.site;
    if site.db.count_posts(&rustpress_db::PostQuery { limit: Some(1), ..rustpress_db::PostQuery::default() }).await? == 0 {
        return Ok(String::new());
    }
    let mut override_month = None;
    if let (Some(month), Some(year)) = (bc.attrs.get("month").and_then(Value::as_u64), bc.attrs.get("year").and_then(Value::as_u64)) {
        let structure = &site.settings.permalink_structure;
        if structure.contains("%monthnum%") && structure.contains("%year%") {
            override_month = Some((year as u32, month as u32));
        }
    }
    let mut colors = serde_json::Map::new();
    let preset_text = bc.attrs.get("textColor").and_then(Value::as_str).map(|c| format!("var:preset|color|{c}"));
    let custom_text = bc.attrs.get("style").and_then(|s| s.get("color")).and_then(|c| c.get("text")).and_then(Value::as_str).map(str::to_owned);
    if let Some(text) = preset_text.or(custom_text) {
        colors.insert("text".into(), Value::from(text));
    }
    let preset_bg = bc.attrs.get("backgroundColor").and_then(Value::as_str).map(|c| format!("var:preset|color|{c}"));
    let custom_bg = bc.attrs.get("style").and_then(|s| s.get("color")).and_then(|c| c.get("background")).and_then(Value::as_str).map(str::to_owned);
    if let Some(bg) = preset_bg.or(custom_bg) {
        colors.insert("background".into(), Value::from(bg));
    }
    let styles = crate::style_engine::get_styles(&serde_json::json!({ "color": colors }), true);
    let inline = if styles.css.is_empty() { String::new() } else { format!(" style=\"{}\"", esc_attr(&styles.css)) };
    let mut classnames = if styles.classnames.is_empty() { String::new() } else { format!(" {}", esc_attr(&styles.classnames)) };
    if has_link_color(bc) {
        classnames.push_str(" has-link-color");
    }
    let calendar = general::get_calendar(env, true, override_month).await?;
    let calendar = calendar.replace("<table", &format!("<table{inline}")).replace("class=\"wp-calendar-table", &format!("class=\"wp-calendar-table{classnames}"));
    Ok(format!("<div {}>{calendar}</div>", bc.wrapper_attributes(&[])))
}

// -------------------------------------------------------------- latest comments

pub(super) async fn latest_comments(bc: &BlockCtx<'_>) -> Result<String> {
    let env = bc.env;
    let site = env.site;
    let display_content = match bc.attrs.get("displayExcerpt") {
        Some(v) => if v.as_bool() == Some(true) { "excerpt".to_owned() } else { "none".to_owned() },
        None => bc.attr_str("displayContent").unwrap_or("excerpt").to_owned(),
    };
    let number = bc.attr_u64("commentsToShow").unwrap_or(5);
    let comments = site.db.comments_query(&rustpress_db::CommentQuery { limit: Some(number), ..rustpress_db::CommentQuery::default() }).await?;
    let mut items = String::new();
    let (avatar, date) = (flag(bc, "displayAvatar"), flag(bc, "displayDate"));
    for comment in &comments {
        items.push_str("<li class=\"wp-block-latest-comments__comment\">");
        if avatar {
            let html = get_avatar(env, &comment.comment_author_email, 48, &comment.comment_author, "wp-block-latest-comments__comment-avatar", "");
            items.push_str(&html);
        }
        items.push_str("<article><footer class=\"wp-block-latest-comments__comment-meta\">");
        let mut author_url = comment.comment_author_url.trim().to_owned();
        if matches!(author_url.as_str(), "http://" | "https://") {
            author_url.clear();
        }
        if author_url.is_empty() && comment.user_id != 0 && let Some(user) = site.db.user(comment.user_id).await? {
            author_url = site.author_url(&user);
        }
        let name = if comment.comment_author.is_empty() { site.tr("Anonymous") } else { comment.comment_author.clone() };
        let author_markup = if author_url.is_empty() {
            format!("<span class=\"wp-block-latest-comments__comment-author\">{name}</span>")
        } else {
            format!("<a class=\"wp-block-latest-comments__comment-author\" href=\"{}\">{name}</a>", esc_url(&author_url))
        };
        let post = site.db.post(comment.comment_post_id).await?;
        let (link, title) = match &post {
            Some(post) => {
                let title = the_title_for(site, post);
                (format!("{}#comment-{}", site.permalink(post).await?, comment.comment_id), if title.is_empty() { site.tr("(no title)") } else { title })
            }
            None => (String::new(), site.tr("(no title)")),
        };
        let post_title = format!("<a class=\"wp-block-latest-comments__comment-link\" href=\"{}\">{title}</a>", esc_url(&link));
        items.push_str(&sprintf(&site.tr("%1$s on %2$s"), &[&author_markup, &post_title]));
        let ts = site.timezone.local_timestamp(&comment.comment_date).unwrap_or(0);
        if date {
            items.push_str(&format!(
                "<time datetime=\"{}\" class=\"wp-block-latest-comments__comment-date\">{}</time>",
                esc_attr(&site.date("c", ts)),
                site.date(&site.options.string("date_format", "F j, Y"), ts)
            ));
        }
        items.push_str("</footer>");
        let protected = post.as_ref().is_some_and(post_password_required);
        if display_content == "full" {
            let text = if protected { site.tr("Password protected") } else { crate::content::comment_text(site, &comment.comment_content) };
            items.push_str(&format!("<div class=\"wp-block-latest-comments__comment-excerpt\">{}</div>", wpautop(&text)));
        } else if display_content == "excerpt" {
            let text = if protected { site.tr("Password protected") } else { strip_tags(&comment.comment_content.replace(['\n', '\r'], " ")) };
            let length = site.trx("20", "comment_excerpt_length").trim().parse().unwrap_or(20);
            let excerpt = rustpress_core::formatting::wp_trim_words_with(&text, length, "&hellip;", site.locale_data().word_count_type);
            items.push_str(&format!("<div class=\"wp-block-latest-comments__comment-excerpt\">{}</div>", wpautop(&excerpt)));
        }
        items.push_str("</article></li>");
    }
    let mut classes = Vec::new();
    if avatar {
        classes.push("has-avatars");
    }
    if date {
        classes.push("has-dates");
    }
    if display_content != "none" {
        classes.push("has-excerpts");
    }
    if comments.is_empty() {
        classes.push("no-comments");
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    Ok(if comments.is_empty() { format!("<div {wrapper}>{}</div>", site.tr("No comments to show.")) } else { format!("<ol {wrapper}>{items}</ol>") })
}

// ------------------------------------------------------------ home link, log in/out

pub(super) fn home_link(bc: &BlockCtx<'_>) -> Result<String> {
    let site = bc.env.site;
    let q = bc.env.query;
    let label = bc.attr_str("label").filter(|l| !l.is_empty()).map(str::to_owned).unwrap_or_else(|| site.tr("Home"));
    let current = q.flags.is_front_page || (q.flags.is_home && site.settings.page_for_posts != q.queried_object_id());
    let aria = if current { " aria-current=\"page\"" } else { "" };
    let target = if bc.attrs.get("opensInNewTab").and_then(Value::as_bool) == Some(true) { " target=\"_blank\"" } else { "" };
    let description = bc.attr_str("description").filter(|d| !d.is_empty()).map(|d| format!("<span class=\"wp-block-navigation-item__description\">{d}</span>")).unwrap_or_default();
    let mut classes = vec!["wp-block-navigation-item".to_owned()];
    if current {
        classes.push("current-menu-item".into());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    Ok(format!(
        "<li {wrapper}><a class=\"wp-block-home-link__content wp-block-navigation-item__content\" href=\"{}\" rel=\"home\" {target}{aria}>{label}{description}</a></li>",
        esc_url(&site.home_url(""))
    ))
}

pub(super) fn loginout(bc: &BlockCtx<'_>) -> Result<String> {
    let site = bc.env.site;
    let q = bc.env.query;
    let mut url = site.site_url("wp-login.php");
    if bc.attrs.get("redirectToCurrent").and_then(Value::as_bool) != Some(false) {
        let request = format!("{}{}", q.url_path, if q.raw_query.is_empty() { String::new() } else { format!("?{}", q.raw_query) });
        let scheme_host = site.home_url("");
        let origin = scheme_host.split_once("://").map(|(scheme, rest)| format!("{scheme}://{}", rest.split('/').next().unwrap_or_default())).unwrap_or(scheme_host);
        url = rustpress_core::urls::add_query_arg(&url, "redirect_to", &php_urlencode(&format!("{origin}{request}")));
    }
    let link = format!("<a href=\"{}\">{}</a>", esc_url(&url), site.tr("Log in"));
    let wrapper = bc.wrapper_attributes(&[("class", "logged-out".to_owned())]);
    Ok(format!("<div {wrapper}>{link}</div>"))
}

/// PHP `urlencode()`.
fn php_urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// ------------------------------------------------------------- post-related blocks

pub(super) async fn post_comments_count(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id() else { return Ok(String::new()) };
    let count = bc.env.site.db.post(post_id).await?.map_or(0, |p| p.comment_count);
    let wrapper = bc.wrapper_attributes(&[("class", align_class(bc))]);
    Ok(format!("<div {wrapper}>{count}</div>"))
}

pub(super) async fn post_comments_link(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id() else { return Ok(String::new()) };
    let site = bc.env.site;
    let Some(post) = site.db.post(post_id).await? else { return Ok(String::new()) };
    if post.comment_status != "open" {
        return Ok(String::new());
    }
    let mut classes = Vec::new();
    let align = align_class(bc);
    if !align.is_empty() {
        classes.push(align);
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    let number = post.comment_count;
    let link = format!("{}{}", site.permalink(&post).await?, if number > 0 { "#comments" } else { "#respond" });
    let title = the_title_for(site, &post);
    let html = if number == 0 {
        sprintf(&site.tr("No comments<span class=\"screen-reader-text\"> on %s</span>"), &[&title])
    } else {
        sprintf(
            &site.trn("%1$s comment<span class=\"screen-reader-text\"> on %2$s</span>", "%1$s comments<span class=\"screen-reader-text\"> on %2$s</span>", number),
            &[&esc_html(&site.number_i18n(number as u64)), &title],
        )
    };
    Ok(format!("<div {wrapper}><a href={}>{html}</a></div>", esc_url(&link)))
}

pub(super) async fn post_author_biography(bc: &BlockCtx<'_>) -> Result<String> {
    let site = bc.env.site;
    let author_id = match bc.context.post_id() {
        Some(id) => site.db.post(id).await?.map_or(0, |p| p.post_author),
        None => bc.env.query.vars.u64("author").unwrap_or(0),
    };
    if author_id == 0 {
        return Ok(String::new());
    }
    let biography = site.db.user_meta(author_id, "description").await?.unwrap_or_default();
    if biography.is_empty() || biography == "0" {
        return Ok(String::new());
    }
    let wrapper = bc.wrapper_attributes(&[("class", align_class(bc))]);
    Ok(format!("<div {wrapper}>{biography}</div>"))
}

pub(super) async fn read_more(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post_id) = bc.context.post_id() else { return Ok(String::new()) };
    let site = bc.env.site;
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    let mut title = the_title_for(site, &post);
    if title.is_empty() {
        title = sprintf(&site.tr("untitled post %s"), &[&post_id.to_string()]);
    }
    let screen_reader = sprintf(&site.tr(": %s"), &[&title]);
    let justify = bc.attr_str("justifyContent").filter(|j| !j.is_empty()).map(|j| format!("is-justified-{j}")).unwrap_or_default();
    let wrapper = bc.wrapper_attributes(&[("class", justify)]);
    let more = bc.attr_str("content").filter(|c| !c.is_empty()).map(str::to_owned).unwrap_or_else(|| site.tr("Read more"));
    Ok(format!(
        "<a {wrapper} href=\"{}\" target=\"{}\">{more}<span class=\"screen-reader-text\">{screen_reader}</span></a>",
        esc_url(&site.permalink(&post).await?),
        esc_attr(bc.attr_str("linkTarget").unwrap_or("_self"))
    ))
}

pub(super) fn shortcode(bc: &BlockCtx<'_>) -> Result<String> {
    Ok(wpautop(&bc.content))
}

/// `block_core_post_time_to_read_word_count()`.
fn word_count(text: &str, kind: rustpress_core::locale::WordCountType) -> u64 {
    use rustpress_core::locale::WordCountType;
    if text.trim().is_empty() {
        return 0;
    }
    let mut text = format!("{text}\n");
    let replace = |text: &mut String, pattern: &str, with: &str| {
        if let Ok(re) = Regex::new(pattern) {
            *text = re.replace_all(text, with).into_owned();
        }
    };
    replace(&mut text, r"(?i)</?[a-z][^>]*?>", "\n");
    replace(&mut text, r"(?s)<!--.*?-->", "");
    replace(&mut text, r"(?i)&nbsp;|&#160;", " ");
    let words = matches!(kind, WordCountType::Words);
    if words {
        replace(&mut text, r"&\S+?;", "");
        replace(&mut text, "--|\u{2014}", " ");
        replace(&mut text, "[\u{21}-\u{40}\u{5B}-\u{60}\u{7B}-\u{7E}\u{80}-\u{BF}\u{D7}\u{F7}\u{2000}-\u{2BFF}\u{2E00}-\u{2E7F}]", "");
    } else {
        replace(&mut text, r"&\S+?;", "a");
        replace(&mut text, "[\u{10000}-\u{10FFFF}]", "a");
    }
    let pattern = match kind {
        WordCountType::CharactersExcludingSpaces => r"\S".to_owned(),
        WordCountType::CharactersIncludingSpaces => "[^\x0C\n\r\t\x0B\u{AD}\u{2028}\u{2029}]".to_owned(),
        WordCountType::Words => r"\S\s+".to_owned(),
    };
    Regex::new(&pattern).map_or(0, |re| re.find_iter(&text).count() as u64)
}

pub(super) async fn post_time_to_read(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(post) = global_post(bc).await? else { return Ok(String::new()) };
    let site = bc.env.site;
    let content = crate::content::get_the_content(bc.env, &post, 1, bc.env.query.flags.is_singular());
    let rate = bc.attrs.get("averageReadingSpeed").and_then(Value::as_f64).unwrap_or(189.0);
    let kind = site.locale_data().word_count_type;
    let total = word_count(&content, kind);
    let mut parts = Vec::new();
    let mode = bc.attr_str("displayMode").unwrap_or("time");
    if mode == "time" {
        let php_round = |f: f64| f.round() as i64;
        if flag(bc, "displayAsRange") {
            let min = php_round(total as f64 / rate * 0.8).max(1);
            let mut max = php_round(total as f64 / rate * 1.2).max(1);
            if min == max {
                max = min + 1;
            }
            parts.push(sprintf(&site.trx("%1$s–%2$s minutes", "Range of minutes to read"), &[&min.to_string(), &max.to_string()]));
        } else {
            let minutes = php_round(total as f64 / rate).max(1);
            parts.push(sprintf(&site.trn("%s minute", "%s minutes", minutes), &[&minutes.to_string()]));
        }
    }
    if mode == "words" {
        let number = site.number_i18n(total);
        parts.push(if matches!(kind, rustpress_core::locale::WordCountType::Words) {
            sprintf(&site.trn("%s word", "%s words", total as i64), &[&number])
        } else {
            sprintf(&site.trn("%s character", "%s characters", total as i64), &[&number])
        });
    }
    let wrapper = bc.wrapper_attributes(&[("class", align_class(bc))]);
    Ok(format!("<div {wrapper}>{}</div>", parts.join("<br>")))
}

// -------------------------------------------------------------------- term blocks

async fn context_term(bc: &BlockCtx<'_>) -> Result<Option<rustpress_db::Term>> {
    if let (Some(id), Some(taxonomy)) = (bc.context.get("termId").and_then(Value::as_u64), bc.context.get("taxonomy").and_then(Value::as_str)) {
        return Ok(bc.env.site.db.term_by_id(Some(taxonomy), id).await?);
    }
    Ok(match &bc.env.query.queried {
        rustpress_core::Queried::Term(term) => Some(term.clone()),
        _ => None,
    })
}

pub(super) async fn term_name(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(term) = context_term(bc).await? else { return Ok(String::new()) };
    let site = bc.env.site;
    let level = bc.attrs.get("level").and_then(Value::as_u64).unwrap_or(0);
    let tag = if level == 0 { "p".to_owned() } else { format!("h{level}") };
    let mut name = term.name.clone();
    if flag(bc, "isLink") {
        name = format!("<a href=\"{}\">{name}</a>", esc_url(&site.term_link(&term).await?));
    }
    let mut classes = Vec::new();
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    Ok(format!("<{tag} {wrapper}>{name}</{tag}>"))
}

pub(super) async fn term_count(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(term) = context_term(bc).await? else { return Ok(String::new()) };
    let count = term.count;
    let text = match bc.attr_str("bracketType") {
        Some("round") => format!("({count})"),
        Some("square") => format!("[{count}]"),
        Some("curly") => format!("{{{count}}}"),
        Some("angle") => format!("<{count}>"),
        _ => count.to_string(),
    };
    Ok(format!("<div {}>{text}</div>", bc.wrapper_attributes(&[])))
}

pub(super) async fn term_description(bc: &BlockCtx<'_>) -> Result<String> {
    let q = bc.env.query;
    let description = if bc.context.get("termId").is_some() && bc.context.get("taxonomy").is_some() {
        context_term(bc).await?.map(|t| t.description).unwrap_or_default()
    } else if q.flags.is_category || q.flags.is_tag || q.flags.is_tax {
        match &q.queried {
            rustpress_core::Queried::Term(term) => term.description.trim().to_owned(),
            _ => String::new(),
        }
    } else {
        String::new()
    };
    if description.is_empty() || description == "0" {
        return Ok(String::new());
    }
    let mut classes = Vec::new();
    if let Some(align) = bc.attr_str("textAlign") {
        classes.push(format!("has-text-align-{align}"));
    }
    if has_link_color(bc) {
        classes.push("has-link-color".to_owned());
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.join(" "))]);
    Ok(format!("<div {wrapper}>{description}</div>"))
}

pub(super) async fn term_template(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(query) = bc.context.get("termQuery").and_then(Value::as_object).filter(|q| !q.is_empty()) else { return Ok(String::new()) };
    let site = bc.env.site;
    let q = bc.env.query;
    let flag_of = |key: &str| query.get(key).is_some_and(|v| !crate::style_engine::is_empty(v));
    let taxonomy_of = |taxonomy: &str| taxonomy.to_owned();
    let mut args = TermsArgs::new(query.get("taxonomy").and_then(Value::as_str).unwrap_or("category"));
    args.number = query.get("perPage").and_then(Value::as_u64).unwrap_or(0);
    args.descending = query.get("order").and_then(Value::as_str).is_some_and(|o| o.eq_ignore_ascii_case("desc"));
    args.orderby = match query.get("orderBy").and_then(Value::as_str) {
        Some("count") => "count".into(),
        Some("slug") => "slug".into(),
        Some("id") => "id".into(),
        _ => "name".into(),
    };
    args.hide_empty = flag_of("hideEmpty");
    let inherit = flag_of("inherit") && (q.flags.is_tax || q.flags.is_category || q.flags.is_tag);
    if inherit {
        if let rustpress_core::Queried::Term(term) = &q.queried {
            args.taxonomy = taxonomy_of(&term.taxonomy);
            if term.taxonomy == "category" {
                if flag_of("showNested") {
                    args.child_of = term.term_id;
                } else {
                    args.parent = Some(term.term_id);
                }
            }
        }
    } else if let Some(include) = query.get("include").and_then(Value::as_array).filter(|i| !i.is_empty()) {
        let mut ids: Vec<u64> = Vec::new();
        for id in include.iter().filter_map(Value::as_u64) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        args.include = ids;
        args.orderby = "include".into();
        args.descending = false;
    } else if args.taxonomy == "category" && !flag_of("showNested") {
        args.parent = Some(0);
    }
    let terms = general::get_terms(bc.env, &args).await?;
    if terms.is_empty() {
        return Ok(String::new());
    }
    let mut content = String::new();
    for term in terms {
        let mut ctx = Context::default();
        ctx.values.insert("termId".into(), Value::from(term.term_id));
        ctx.values.insert("taxonomy".into(), Value::from(term.taxonomy.clone()));
        let mut inner = String::new();
        let mut index = 0;
        for chunk in &bc.block.inner_content {
            match chunk {
                Content::Html(html) => inner.push_str(html),
                Content::InnerBlock => {
                    inner.push_str(&bc.env.render_block(&bc.block.inner_blocks[index], &ctx, Some(bc.block)).await?);
                    index += 1;
                }
            }
        }
        let classes = format!("wp-block-term term-{} {} taxonomy-{}", term.term_id, term.taxonomy, term.taxonomy);
        content.push_str(&format!("<li class=\"{}\">{inner}</li>", esc_attr(&classes)));
    }
    let wrapper = bc.wrapper_attributes(&[("class", if has_link_color(bc) { "has-link-color".to_owned() } else { String::new() })]);
    let _ = site;
    Ok(format!("<ul {wrapper}>{content}</ul>"))
}

// ------------------------------------------------------------------- core/block

pub(super) async fn reusable_block(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(reference) = bc.attrs.get("ref").and_then(Value::as_u64).filter(|r| *r != 0) else { return Ok(String::new()) };
    let Some(post) = bc.env.site.db.post(reference).await? else { return Ok(String::new()) };
    if post.post_type != "wp_block" {
        return Ok(String::new());
    }
    let seen = format!("core/block:{reference}");
    if bc.env.with_state(|s| s.active_patterns.contains(&seen)) {
        return Ok(String::new());
    }
    if post.post_status != "publish" || !post.post_password.is_empty() {
        return Ok(String::new());
    }
    bc.env.with_state(|s| s.active_patterns.insert(seen.clone()));
    let post = Arc::new(post);
    let result = bc.env.render_markup(&post.post_content, bc.available).await;
    bc.env.with_state(|s| s.active_patterns.remove(&seen));
    result
}

// -------------------------------------------------------------------- social link

/// `block_core_social_link_services()`: `(key, name, icon)` read from WordPress's own `social-link.php`.
fn social_services(bc: &BlockCtx<'_>) -> &'static [(String, String, String)] {
    static SERVICES: std::sync::OnceLock<Vec<(String, String, String)>> = std::sync::OnceLock::new();
    SERVICES.get_or_init(|| {
        let Ok(php) = std::fs::read_to_string(bc.env.registry.blocks_dir().join("social-link.php")) else { return Vec::new() };
        let unescape = |s: &str| s.replace("\\'", "'").replace("\\\\", "\\");
        let Ok(re) = Regex::new(r"'([a-z0-9]+)'\s*=>\s*array\(\s*'name'\s*=>\s*_x\(\s*'((?:[^'\\]|\\.)*)'\s*,\s*'[^']*'\s*\),\s*'icon'\s*=>\s*'((?:[^'\\]|\\.)*)',?\s*\)") else { return Vec::new() };
        re.captures_iter(&php).map(|c| (c[1].to_owned(), unescape(&c[2]), unescape(&c[3]))).collect()
    })
}

pub(super) fn social_link(bc: &BlockCtx<'_>) -> Result<String> {
    let site = bc.env.site;
    let services = social_services(bc);
    let service = bc.attr_str("service").filter(|s| !s.is_empty()).unwrap_or("Icon");
    let Some(url) = bc.attr_str("url").map(str::trim).filter(|u| !u.is_empty() && *u != "0") else { return Ok(String::new()) };
    let lookup = |key: &str| services.iter().find(|(k, _, _)| k == key);
    let fallback = lookup("share");
    let entry = lookup(service).or(fallback);
    let label = bc.attr_str("label").map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned);
    let name = label.unwrap_or_else(|| entry.map(|(_, name, _)| site.trx(name, "social link block variation name")).unwrap_or_default());
    let icon = entry.map(|(_, _, icon)| icon.as_str()).unwrap_or_default();
    let rel = bc.attr_str("rel").unwrap_or_default();
    let show_labels = bc.context.get("showLabels").and_then(Value::as_bool).unwrap_or(false);
    let open_in_new_tab = bc.context.get("openInNewTab").and_then(Value::as_bool).unwrap_or(false);

    let mut url = url.to_owned();
    if url.contains('@') && !url.contains("://") && !url.contains(' ') && !url.starts_with("mailto:") {
        url = format!("mailto:{url}");
    }
    let has_scheme = Regex::new(r"^[A-Za-z][A-Za-z0-9+.-]*:").is_ok_and(|re| re.is_match(&url));
    if !has_scheme && !url.starts_with("//") && !url.starts_with('#') {
        url = format!("https://{url}");
    }

    let present = |key: &str| bc.context.get(key).filter(|v| !v.is_null()).map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned));
    let mut classes = format!("wp-social-link wp-social-link-{service} ");
    let mut named = Vec::new();
    if let Some(color) = present("iconColor") {
        named.push(format!("has-{color}-color"));
    }
    if let Some(color) = present("iconBackgroundColor") {
        named.push(format!("has-{color}-background-color"));
    }
    classes.push_str(&named.join(" "));
    let mut style = String::new();
    if let Some(value) = present("iconColorValue") {
        style.push_str(&format!("color:{value};"));
    }
    if let Some(value) = present("iconBackgroundColorValue") {
        style.push_str(&format!("background-color:{value};"));
    }
    let wrapper = bc.wrapper_attributes(&[("class", classes.trim_end().to_owned()), ("style", style)]);
    let label_class = if show_labels { "" } else { " screen-reader-text" };
    let mut link = format!("<li {wrapper}><a href=\"{}\" class=\"wp-block-social-link-anchor\">{icon}<span class=\"wp-block-social-link-label{label_class}\">{}</span></a></li>", esc_url(&url), esc_html(&name));
    let extra = if open_in_new_tab {
        Some(format!(" rel=\"{}\" target=\"_blank\"", esc_attr(format!("{rel} noopener nofollow").trim())))
    } else if !rel.is_empty() {
        Some(format!(" rel=\"{}\"", esc_attr(rel.trim())))
    } else {
        None
    };
    if let Some(extra) = extra
        && let Some(at) = link.find("class=\"wp-block-social-link-anchor\"")
    {
        let end = at + "class=\"wp-block-social-link-anchor\"".len();
        link.insert_str(end, &extra);
    }
    Ok(link)
}