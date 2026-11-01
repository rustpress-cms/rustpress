//! `WPSEO_Replace_Vars`: the `%%var%%` variables of the title and description templates.

use std::sync::LazyLock;

use rustpress_core::Result;
use rustpress_core::formatting::{esc_html, wp_strip_all_tags};
use rustpress_db::{Post, Term};

use super::Page;

/// The object the variables are taken from (`$presentation->source`, cast to an array).
#[derive(Debug, Clone, Copy)]
pub enum Source<'a> {
    None,
    Post(&'a Post),
    Term(&'a Term),
    /// `[ 'post_author' => $id ]` of an author archive.
    Author(u64),
    /// `[ 'post_type' => $name ]` of a post type archive.
    PostType(&'a str),
}

impl Source<'_> {
    fn id(&self) -> u64 {
        match self {
            Source::Post(p) => p.id,
            _ => 0,
        }
    }

    fn post_author(&self) -> u64 {
        match self {
            Source::Post(p) => p.post_author,
            Source::Author(id) => *id,
            _ => 0,
        }
    }

    fn term(&self) -> Option<&Term> {
        match self {
            Source::Term(t) => Some(t),
            _ => None,
        }
    }
}

/// `String_Helper::standardize_whitespace()`.
pub fn standardize_whitespace(text: &str) -> String {
    let text = text.replace(['\t', '\n', '\r', '\x0c'], " ").replace("  ", " ");
    php_trim(&text).to_owned()
}

/// PHP's `trim()` with its default characters.
pub fn php_trim(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0b'])
}

/// `String_Helper::strip_shortcode()`.
fn strip_shortcode(page: &Page<'_>, text: &str) -> String {
    static BRACKETS: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?s)\[[^\]]+\]").expect("valid pattern"));
    BRACKETS.replace_all(&page.site.strip_shortcodes(text), "").into_owned()
}

/// `WPSEO_Replace_Vars::replace( $text, $args )`.
pub async fn replace(page: &Page<'_>, text: &str, source: Source<'_>) -> Result<String> {
    let text = wp_strip_all_tags(text);
    if !text.contains("%%") {
        return Ok(standardize_whitespace(&text));
    }
    static VARS: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)%%([^%]+(%%single)?)%%?").expect("valid pattern"));
    let found: Vec<(String, bool)> = VARS.captures_iter(&text).map(|c| (c[1].to_owned(), c.get(2).is_some())).collect();
    let mut replacements: Vec<(String, String)> = Vec::new();
    for (var, single) in &found {
        let replacement = if let Some(field) = var.strip_prefix("cf_") {
            custom_field(page, source, field).await?
        } else if let Some(tax) = var.strip_prefix("ct_desc_") {
            custom_tax_description(page, source, tax).await?
        } else if let Some(tax) = var.strip_prefix("ct_") {
            let id = source.id();
            if id == 0 { None } else { Some(get_terms(page, source, id, tax, *single).await?).filter(|n| !n.is_empty()) }
        } else {
            retrieve(page, source, var).await?
        };
        if let Some(replacement) = replacement {
            let key = format!("%%{var}%%");
            if !replacements.iter().any(|(k, _)| *k == key) {
                replacements.push((key, replacement));
            }
        }
    }
    // `str_replace( array_keys(), array_values(), $text )`: one key after the other.
    let mut text = text;
    for (key, value) in &replacements {
        text = text.replace(key, value);
    }
    // Remove the variables nothing was found for.
    for (var, _) in &found {
        text = text.replace(&format!("%%{var}%%"), "");
    }
    if let Some((_, sep)) = replacements.iter().find(|(k, _)| k == "%%sep%%")
        && !sep.is_empty()
    {
        let quoted = regex::escape(sep);
        if let Ok(doubled) = regex::Regex::new(&format!(r"{quoted}(?:\s*{quoted})*")) {
            text = doubled.replace_all(&text, regex::NoExpand(sep)).into_owned();
        }
    }
    Ok(standardize_whitespace(&text))
}

/// `WPSEO_Replace_Vars::retrieve_*()` of the standard variables; `None` where nothing is found.
async fn retrieve(page: &Page<'_>, source: Source<'_>, var: &str) -> Result<Option<String>> {
    let site = page.site;
    let q = page.query;
    let post = match source {
        Source::Post(p) => Some(p),
        _ => None,
    };
    let term = source.term();
    Ok(match var {
        "sep" => Some(page.options.title_separator()),
        "sitename" => Some(site_name(page)).filter(|n| !n.is_empty()),
        "sitedesc" => Some(wp_strip_all_tags(&site.settings.blogdescription)).filter(|d| !d.is_empty()),
        "title" => post.map(|p| p.post_title.clone()).filter(|t| !t.is_empty()),
        "excerpt" => excerpt(page, post),
        "excerpt_only" | "caption" => post.filter(|p| !p.post_excerpt.is_empty() && p.post_password.is_empty()).map(|p| wp_strip_all_tags(&p.post_excerpt)),
        "post_content" => post.filter(|p| !p.post_content.is_empty() && p.post_password.is_empty()).map(|p| wp_strip_all_tags(&page.site.strip_shortcodes(&strip_shortcode(page, &p.post_content)))),
        "id" => post.map(|p| p.id.to_string()),
        "date" => date(page, post),
        "modified" => post.filter(|p| !p.post_modified.is_empty()).map(|p| format_translated(page, &p.post_modified, &date_format(site))),
        "parent_title" => match post.filter(|p| p.post_parent != 0) {
            Some(p) => site.db.post(p.post_parent).await?.map(|parent| rustpress_blocks::content::the_title_for(site, &parent)),
            None => None,
        },
        "searchphrase" => q.vars.get("s").filter(|s| !s.is_empty()).map(esc_html),
        "page" => {
            let (nr, max) = pagenumbering(page);
            (max > 1 && nr > 1).then(|| rustpress_core::i18n::sprintf(&format!("{} {}", page.options.title_separator(), site.trd(super::DOMAIN, "Page %1$d of %2$d")), &[&nr.to_string(), &max.to_string()]))
        }
        "pagenumber" => Some(pagenumbering(page).0.to_string()),
        "pagetotal" => Some(pagenumbering(page).1.to_string()),
        "pt_single" | "pt_plural" => post_type_name(page, source, var == "pt_single"),
        "term_title" => term.map(|t| site.term_name(t)).filter(|n| !n.is_empty()),
        "term_description" | "category_description" | "tag_description" => term.filter(|t| t.term_id != 0).and_then(|t| Some(wp_strip_all_tags(&t.description)).filter(|d| !d.is_empty())),
        "term_hierarchy" => match term {
            Some(t) if t.term_id != 0 => term_hierarchy(page, t).await?.filter(|h| !h.is_empty()).map(|h| esc_html(&h)),
            _ => None,
        },
        "category" => {
            let id = source.id();
            if id == 0 { None } else { Some(get_terms(page, source, id, "category", false).await?).filter(|c| !c.is_empty()) }
        }
        "tag" => {
            let id = source.id();
            if id == 0 { None } else { Some(get_terms(page, source, id, "post_tag", false).await?).filter(|c| !c.is_empty()) }
        }
        "primary_category" => match post {
            Some(p) => primary_category(page, p).await?,
            None => None,
        },
        "category_title" => match post {
            Some(p) => first_hierarchical_term(page, p).await?,
            None => None,
        },
        "name" | "user_description" | "author_first_name" | "author_last_name" => {
            let user = userid(page, source);
            let field = match var {
                "name" => "display_name",
                "user_description" => "description",
                "author_first_name" => "first_name",
                _ => "last_name",
            };
            Some(author_meta(page, user, field).await?).filter(|v| !v.is_empty())
        }
        "userid" => Some(userid(page, source).to_string()),
        "term404" => ["pagename", "name"].iter().find_map(|v| q.vars.get(v).filter(|s| !s.is_empty())).map(|s| rustpress_content::sanitize::text_field(&s.replace('-', " "))),
        "focuskw" => match (post, term) {
            (Some(p), _) => Some(super::presentation::post_meta(page, p.id, "focuskw").await?).filter(|k| !k.is_empty()),
            (None, Some(t)) if t.term_id != 0 => page.options.term_meta(&t.taxonomy, t.term_id).into_iter().find(|(k, _)| k == "wpseo_focuskw").map(|(_, v)| v).filter(|k| !k.is_empty()),
            _ => None,
        },
        "permalink" => match post {
            Some(p) => Some(site.permalink(p).await?),
            None => None,
        },
        "post_year" | "post_month" | "post_day" => post.filter(|_| !page.dates_hidden).map(|p| {
            let format = match var {
                "post_year" => "Y",
                "post_month" => "F",
                _ => "d",
            };
            format_translated(page, &p.post_date, format)
        }),
        "currentdate" => Some(site.date(&date_format(site), page.now)),
        "currenttime" => Some(site.date(&site.options.string("time_format", "g:i a"), page.now)),
        "currentday" => Some(site.date("j", page.now)),
        "currentmonth" => Some(site.date("F", page.now)),
        "currentyear" => Some(site.date("Y", page.now)),
        _ => None,
    })
}

/// `Site_Helper::get_site_name()`.
pub fn site_name(page: &Page<'_>) -> String {
    strip_all_tags_breaks(&page.site.settings.blogname)
}

/// `wp_strip_all_tags( $text, true )`: line breaks and runs of white space become one space.
pub fn strip_all_tags_breaks(text: &str) -> String {
    static BREAKS: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"[\r\n\t ]+").expect("valid pattern"));
    php_trim(&BREAKS.replace_all(&wp_strip_all_tags(text), " ")).to_owned()
}

/// `wp_html_excerpt( $str, $count )`.
fn wp_html_excerpt(text: &str, count: usize) -> String {
    static ENTITY_END: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"&[^;\s]{0,6}$").expect("valid pattern"));
    let text = strip_all_tags_breaks(text);
    let excerpt: String = text.chars().take(count).collect();
    let excerpt = ENTITY_END.replace(&excerpt, "").into_owned();
    if excerpt != text { php_trim(&excerpt).to_owned() } else { excerpt }
}

/// `get_option( 'date_format' )`.
fn date_format(site: &rustpress_core::Site) -> String {
    site.options.string("date_format", "F j, Y")
}

/// `Date_Helper::format_translated()`: a date of the database in a format of `date_i18n()`.
pub fn format_translated(page: &Page<'_>, date: &str, format: &str) -> String {
    match rustpress_core::datetime::SiteTimezone::gmt_timestamp(date) {
        Some(ts) => page.site.date_utc(format, ts),
        None => String::new(),
    }
}

/// `retrieve_excerpt()`.
fn excerpt(page: &Page<'_>, post: Option<&Post>) -> Option<String> {
    let post = post.filter(|p| p.post_password.is_empty())?;
    let limit = if page.site.locale() == "ja" { 80 } else { 156 };
    if !post.post_excerpt.is_empty() {
        return Some(wp_strip_all_tags(&strip_shortcode(page, &post.post_excerpt)));
    }
    if post.post_content.is_empty() {
        return None;
    }
    let content = wp_strip_all_tags(&page.site.strip_shortcodes(&strip_shortcode(page, &post.post_content)));
    if content.chars().count() <= limit {
        return Some(content);
    }
    let mut excerpt = wp_html_excerpt(&content, limit);
    if let Some(at) = excerpt.rfind(' ').filter(|at| *at > 0) {
        excerpt.truncate(at);
    }
    Some(excerpt)
}

/// `retrieve_date()`.
fn date(page: &Page<'_>, post: Option<&Post>) -> Option<String> {
    let site = page.site;
    let vars = &page.query.vars;
    if let Some(p) = post.filter(|p| !p.post_date.is_empty()) {
        return Some(format_translated(page, &p.post_date, &date_format(site)));
    }
    if vars.u64("day").unwrap_or(0) != 0 {
        // `get_the_date()` of the global post (`false` while a plugin hides the dates: no replacement).
        return page.global_post.filter(|_| !page.dates_hidden).map(|p| format_translated(page, &p.post_date, &date_format(site)));
    }
    if let Some(title) = single_month_title(page, " ") {
        return Some(title);
    }
    Some(vars.u64("year").unwrap_or(0).to_string())
}

/// `single_month_title( $prefix, false )`.
pub fn single_month_title(page: &Page<'_>, prefix: &str) -> Option<String> {
    let vars = &page.query.vars;
    let year = vars.u64("year").unwrap_or(0);
    let month = vars.u64("monthnum").unwrap_or(0);
    let m = vars.get("m").unwrap_or_default();
    let (my_year, my_month) = if month != 0 && year != 0 {
        (year.to_string(), month)
    } else if !m.is_empty() && m != "0" {
        (m.chars().take(4).collect(), m.chars().skip(4).take(2).collect::<String>().parse().unwrap_or(0))
    } else {
        return None;
    };
    // `$wp_locale->get_month()`.
    let start = chrono::NaiveDate::from_ymd_opt(2000, my_month as u32, 1)?.and_hms_opt(0, 0, 0)?;
    let name = page.site.date_utc("F", start.and_utc().timestamp());
    Some(format!("{prefix}{name}{prefix}{my_year}"))
}

/// `determine_pagenumbering()`: the current page and the number of pages.
fn pagenumbering(page: &Page<'_>) -> (u64, u64) {
    let q = page.query;
    if !q.flags.is_singular() {
        let nr = q.vars.u64("paged").filter(|n| *n != 0).unwrap_or(1);
        let max = if q.max_pages != 0 { q.max_pages } else { 1 };
        (nr, max)
    } else {
        let nr = q.vars.u64("page").filter(|n| *n != 0).unwrap_or(1);
        let max = page.global_post.map_or(1, |p| p.post_content.matches("<!--nextpage-->").count() as u64 + 1);
        (nr, max)
    }
}

/// `determine_pt_names()`.
fn post_type_name(page: &Page<'_>, source: Source<'_>, single: bool) -> Option<String> {
    let from_query = page.query.vars.get("post_type").filter(|t| !t.is_empty()).map(|t| t.split(',').next().unwrap_or(t).to_owned());
    let from_source = match source {
        Source::Post(p) => Some(p.post_type.clone()),
        Source::PostType(t) => Some(t.to_owned()),
        _ => None,
    };
    let from_queried = match &page.query.queried {
        rustpress_core::Queried::Post(p) => Some(p.post_type.clone()),
        _ => None,
    };
    let name = from_query.or(from_source).or(from_queried)?;
    let label = if single { post_type_singular_label(page.site, &name) } else { page.site.post_type_label(&name) };
    Some(label).filter(|l| !l.is_empty())
}

/// `retrieve_userid()`.
fn userid(page: &Page<'_>, source: Source<'_>) -> u64 {
    match source.post_author() {
        // `get_query_var( 'author' )`: `WP_Query::get_posts()` sets it from `author_name` too.
        0 => page.query.vars.u64("author").filter(|id| *id != 0).or(match &page.query.queried {
            rustpress_core::Queried::User(u) if page.query.flags.is_author => Some(u.id),
            _ => None,
        })
        .unwrap_or(0),
        id => id,
    }
}

/// `get_the_author_meta( $field, $user_id )`.
pub async fn author_meta(page: &Page<'_>, user_id: u64, field: &str) -> Result<String> {
    if user_id == 0 {
        return Ok(String::new());
    }
    let Some(user) = page.site.db.user(user_id).await? else { return Ok(String::new()) };
    Ok(match field {
        "display_name" => user.display_name,
        "user_login" => user.user_login,
        "user_nicename" => user.user_nicename,
        "user_email" => user.user_email,
        "user_url" => user.user_url,
        _ => page.site.db.user_meta(user_id, field).await?.unwrap_or_default(),
    })
}

/// `get_the_terms( $id, $taxonomy )`: the terms of a post ordered by name.
pub async fn post_terms(page: &Page<'_>, post_id: u64, taxonomy: &str) -> Result<Vec<Term>> {
    let mut terms = page.site.db.terms_for_post(post_id, Some(taxonomy)).await?;
    terms.sort_by_key(|t| (t.name.to_lowercase(), t.term_id));
    Ok(terms)
}

/// `WPSEO_Replace_Vars::get_terms()`.
async fn get_terms(page: &Page<'_>, source: Source<'_>, id: u64, taxonomy: &str, single: bool) -> Result<String> {
    if let Some(term) = source.term().filter(|t| t.term_id != 0) {
        return Ok(page.site.term_name(term));
    }
    let terms = post_terms(page, id, taxonomy).await?;
    if terms.is_empty() {
        return Ok(String::new());
    }
    let mut output = String::new();
    for term in &terms {
        if single {
            output = page.site.term_name(term);
            break;
        }
        output.push_str(&page.site.term_name(term));
        output.push_str(", ");
    }
    Ok(php_trim(&output).trim_end_matches(',').to_owned())
}

/// `retrieve_cf_custom_field_name()`.
async fn custom_field(page: &Page<'_>, source: Source<'_>, field: &str) -> Result<Option<String>> {
    let sanitize = rustpress_content::sanitize::text_field;
    if let Source::Post(p) = source {
        let value = page.site.db.post_meta_value(p.id, field).await?.unwrap_or_default();
        let is_array = matches!(rustpress_core::options::maybe_unserialize(&value), rustpress_phpser::Value::Array(_));
        return Ok((!value.is_empty() && !is_array).then(|| sanitize(&value)));
    }
    if let Some(t) = source.term().filter(|t| t.term_id != 0) {
        let value = page.site.db.term_meta_pairs(t.term_id).await?.into_iter().find(|(k, _)| k.as_deref() == Some(field)).and_then(|(_, v)| v).unwrap_or_default();
        return Ok((!value.is_empty()).then(|| sanitize(&value)));
    }
    Ok(None)
}

/// `retrieve_ct_desc_custom_tax_name()`.
async fn custom_tax_description(page: &Page<'_>, source: Source<'_>, taxonomy: &str) -> Result<Option<String>> {
    let id = source.id();
    if id == 0 {
        return Ok(None);
    }
    let terms = post_terms(page, id, taxonomy).await?;
    Ok(terms.first().map(|t| wp_strip_all_tags(&t.description)).filter(|d| !d.is_empty()))
}

/// `retrieve_primary_category()`: `WPSEO_Primary_Term::get_primary_term()`, the stored primary category if the post
/// still has it.
async fn primary_category(page: &Page<'_>, post: &Post) -> Result<Option<String>> {
    let primary = super::php_int(&page.site.db.post_meta_value(post.id, "_yoast_wpseo_primary_category").await?.unwrap_or_default());
    if primary <= 0 {
        return Ok(None);
    }
    let terms = post_terms(page, post.id, "category").await?;
    Ok(terms.iter().find(|t| t.term_id == primary as u64).map(|t| t.name.clone()))
}

/// `retrieve_category_title()`: the first term of the first hierarchical taxonomy of the post.
async fn first_hierarchical_term(page: &Page<'_>, post: &Post) -> Result<Option<String>> {
    let mut taxonomies: Vec<String> = Vec::new();
    if post.post_type == "post" {
        taxonomies.push("category".into());
    }
    taxonomies.extend(page.site.types.taxonomies.iter().filter(|t| t.hierarchical && t.object_types.contains(&post.post_type)).map(|t| t.name.clone()));
    for taxonomy in taxonomies {
        if let Some(term) = post_terms(page, post.id, &taxonomy).await?.first() {
            return Ok(Some(term.name.clone()));
        }
    }
    Ok(None)
}

/// `get_term_parents_list( $term_id, $taxonomy, [ 'format' => 'name', 'link' => false, 'inclusive' => true ] )`
/// joined by the separator, without it at the end.
async fn term_hierarchy(page: &Page<'_>, term: &Term) -> Result<Option<String>> {
    let hierarchical = term.taxonomy == "category" || page.site.types.taxonomy(&term.taxonomy).is_some_and(|t| t.hierarchical);
    if !hierarchical {
        return Ok(Some(String::new()));
    }
    let separator = format!(" {} ", page.options.title_separator());
    let mut chain = vec![term.clone()];
    let mut parent = term.parent;
    while parent != 0 {
        let Some(t) = page.site.db.term_by_id(Some(&term.taxonomy), parent).await? else { break };
        parent = t.parent;
        chain.push(t);
    }
    let list: String = chain.iter().rev().map(|t| format!("{}{separator}", t.name)).collect();
    Ok(Some(list.trim_end_matches(|c| separator.contains(c)).to_owned()))
}

/// `get_post_type_object( $name )->labels->singular_name`.
fn post_type_singular_label(site: &rustpress_core::Site, name: &str) -> String {
    match name {
        "post" => site.trx("Post", "post type singular name"),
        "page" => site.trx("Page", "post type singular name"),
        // Attachments have no `singular_name` of their own: the default of a flat type.
        "attachment" => site.trx("Post", "post type singular name"),
        other => site.types.post_type(other).map_or_else(|| other.to_owned(), |t| t.singular_label()),
    }
}
