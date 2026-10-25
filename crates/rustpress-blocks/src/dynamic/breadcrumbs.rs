//! `core/breadcrumbs`: port of `render_block_core_breadcrumbs()` and its helpers.

use rustpress_core::Queried;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url, wp_trim_words_with};
use rustpress_core::urls::untrailingslashit;
use rustpress_core::ShowOnFront;
use rustpress_db::Post;
use serde_json::Value;

use crate::Result;
use crate::content::the_title_for;
use crate::render::BlockCtx;

use super::global_post;

struct Item {
    label: String,
    url: Option<String>,
    allow_html: bool,
}

impl Item {
    fn plain(label: impl Into<String>) -> Self {
        Self { label: label.into(), url: None, allow_html: false }
    }

    fn link(label: impl Into<String>, url: String) -> Self {
        Self { label: label.into(), url: Some(url), allow_html: false }
    }
}

/// Days since the epoch of a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn query_var(bc: &BlockCtx<'_>, key: &str) -> u64 {
    bc.env.query.vars.u64(key).unwrap_or(0)
}

fn is_paged(bc: &BlockCtx<'_>) -> bool {
    query_var(bc, "paged") > 1
}

/// `get_pagenum_link( 1 )`.
fn first_page_link(bc: &BlockCtx<'_>) -> String {
    let q = bc.env.query;
    let request = format!("{}{}", q.url_path, if q.raw_query.is_empty() { String::new() } else { format!("?{}", q.raw_query) });
    bc.env.site.pagenum_link(&request, 1)
}

/// `block_core_breadcrumbs_create_item()`.
fn create_item(bc: &BlockCtx<'_>, label: impl Into<String>, paged: bool) -> Item {
    Item { label: label.into(), url: paged.then(|| first_page_link(bc)), allow_html: false }
}

/// `block_core_breadcrumbs_create_page_number_item()`.
fn page_number_item(bc: &BlockCtx<'_>, var: &str) -> Item {
    let site = bc.env.site;
    let number = site.number_i18n(query_var(bc, var));
    let template = if var == "cpage" { site.tr("Comments Page %s") } else { site.tr("Page %s") };
    Item::plain(rustpress_core::i18n::sprintf(&template, &[&number]))
}

/// `block_core_breadcrumbs_get_post_title()`.
fn post_title(bc: &BlockCtx<'_>, post: &Post) -> String {
    let title = the_title_for(bc.env.site, post);
    if title.is_empty() { bc.env.site.tr("(no title)") } else { title }
}

async fn post_title_of(bc: &BlockCtx<'_>, id: u64) -> Result<String> {
    Ok(match bc.env.site.db.post(id).await? {
        Some(post) => post_title(bc, &post),
        None => bc.env.site.tr("(no title)"),
    })
}

fn is_hierarchical_type(bc: &BlockCtx<'_>, post_type: &str) -> bool {
    match post_type {
        "page" => true,
        "post" | "attachment" => false,
        other => bc.env.site.types.post_type(other).is_some_and(|t| t.hierarchical),
    }
}

fn is_hierarchical_taxonomy(bc: &BlockCtx<'_>, taxonomy: &str) -> bool {
    match taxonomy {
        "category" => true,
        "post_tag" | "post_format" => false,
        other => bc.env.site.types.taxonomy(other).is_some_and(|t| t.hierarchical),
    }
}

/// `get_object_taxonomies( $post_type )`: the taxonomy names in registration order.
fn object_taxonomies(bc: &BlockCtx<'_>, post_type: &str) -> Vec<String> {
    let mut names = Vec::new();
    if post_type == "post" {
        names.extend(["category", "post_tag", "post_format"].map(str::to_owned));
    }
    for info in &bc.env.site.types.taxonomies {
        if info.object_types.iter().any(|t| t == post_type) && !names.contains(&info.name) {
            names.push(info.name.clone());
        }
    }
    names
}

/// `block_core_breadcrumbs_get_term_ancestors_items()`.
async fn term_ancestor_items(bc: &BlockCtx<'_>, term: &rustpress_db::Term, taxonomy: &str) -> Result<Vec<Item>> {
    let mut chain = Vec::new();
    if is_hierarchical_taxonomy(bc, taxonomy) {
        let mut parent = term.parent;
        while parent != 0 && !chain.iter().any(|t: &rustpress_db::Term| t.term_id == parent) {
            let Some(ancestor) = bc.env.site.db.term_by_id(Some(taxonomy), parent).await? else { break };
            parent = ancestor.parent;
            chain.push(ancestor);
        }
    }
    let mut items = Vec::new();
    for ancestor in chain.iter().rev() {
        items.push(Item::link(ancestor.name.clone(), bc.env.site.term_link(ancestor).await?));
    }
    Ok(items)
}

/// `block_core_breadcrumbs_get_archive_breadcrumbs()`.
async fn archive_items(bc: &BlockCtx<'_>) -> Result<Vec<Item>> {
    let q = bc.env.query;
    let site = bc.env.site;
    let mut items = Vec::new();
    if q.flags.is_date {
        let mut year = query_var(bc, "year");
        let mut month = query_var(bc, "monthnum");
        let mut day = query_var(bc, "day");
        if year == 0
            && let Some(m) = q.vars.get("m").filter(|m| !m.is_empty())
        {
            year = m.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(0);
            month = m.get(4..6).and_then(|s| s.parse().ok()).unwrap_or(0);
            day = m.get(6..8).and_then(|s| s.parse().ok()).unwrap_or(0);
        }
        let paged = is_paged(bc);
        let month_name = |month: u64| site.date_utc("F", days_from_civil(year as i64, month as i64, 1) * 86400);
        if year != 0 {
            if month != 0 {
                items.push(Item::link(year.to_string(), site.date_link(year as u32, 0, 0)));
                if day != 0 {
                    items.push(Item::link(month_name(month), site.date_link(year as u32, month as u32, 0)));
                    items.push(create_item(bc, day.to_string(), paged));
                } else {
                    items.push(create_item(bc, month_name(month), paged));
                }
            } else {
                items.push(create_item(bc, year.to_string(), paged));
            }
        }
        if paged {
            items.push(page_number_item(bc, "paged"));
        }
        return Ok(items);
    }

    let paged = is_paged(bc);
    match &q.queried {
        Queried::None => return Ok(Vec::new()),
        Queried::Term(term) => {
            items.extend(term_ancestor_items(bc, term, &term.taxonomy).await?);
            items.push(create_item(bc, term.name.clone(), paged));
        }
        Queried::PostType(post_type) if q.flags.is_post_type_archive => {
            let title = site.post_type_archives_label(post_type);
            items.push(create_item(bc, title, paged));
        }
        Queried::User(user) if q.flags.is_author => items.push(create_item(bc, user.display_name.clone(), paged)),
        _ => {}
    }
    if paged {
        items.push(page_number_item(bc, "paged"));
    }
    Ok(items)
}

/// `block_core_breadcrumbs_get_terms_breadcrumbs()`.
async fn terms_items(bc: &BlockCtx<'_>, post_id: u64, post_type: &str) -> Result<Vec<Item>> {
    let site = bc.env.site;
    let mut taxonomies = Vec::new();
    if post_type == "post" {
        taxonomies.extend(["category", "post_tag"].map(str::to_owned));
    }
    for info in &site.types.taxonomies {
        if info.show_in_rest && info.queryable && info.object_types.iter().any(|t| t == post_type) {
            taxonomies.push(info.name.clone());
        }
    }
    for taxonomy in taxonomies {
        let mut terms = site.db.terms_for_post(post_id, Some(&taxonomy)).await?;
        if terms.is_empty() {
            continue;
        }
        terms.sort_by_key(|t| (t.name.to_lowercase(), t.term_id));
        let term = &terms[0];
        let mut items = term_ancestor_items(bc, term, &taxonomy).await?;
        items.push(Item::link(term.name.clone(), site.term_link(term).await?));
        return Ok(items);
    }
    Ok(Vec::new())
}

/// `get_post_ancestors()`, nearest parent first.
async fn post_ancestors(bc: &BlockCtx<'_>, post_id: u64) -> Result<Vec<Post>> {
    let mut out: Vec<Post> = Vec::new();
    let mut parent = match bc.env.site.db.post(post_id).await? {
        Some(post) => post.post_parent,
        None => 0,
    };
    while parent != 0 && !out.iter().any(|p| p.id == parent) && parent != post_id {
        let Some(ancestor) = bc.env.site.db.post(parent).await? else { break };
        parent = ancestor.post_parent;
        out.push(ancestor);
    }
    Ok(out)
}

pub(super) async fn breadcrumbs(bc: &BlockCtx<'_>) -> Result<String> {
    let q = bc.env.query;
    let site = bc.env.site;
    let is_front_page = q.flags.is_front_page;
    if !bc.attr_bool("showOnHomePage") && is_front_page {
        return Ok(String::new());
    }
    let page_for_posts = site.settings.page_for_posts;
    let mut items: Vec<Item> = Vec::new();

    if bc.attr_bool("showHomeItem") {
        let home = site.tr("Home");
        if !is_front_page || (site.settings.show_on_front == ShowOnFront::Page && query_var(bc, "page") > 1) {
            items.push(Item::link(home, site.home_url("/")));
        } else {
            items.push(create_item(bc, home, is_paged(bc)));
        }
    }

    if q.flags.is_home {
        if page_for_posts != 0 {
            let title = post_title_of(bc, page_for_posts).await?;
            items.push(create_item(bc, title, is_paged(bc)));
        }
        if is_paged(bc) {
            items.push(page_number_item(bc, "paged"));
        }
    } else if is_front_page {
        if query_var(bc, "page") > 1 {
            items.push(page_number_item(bc, "page"));
        }
    } else if q.flags.is_search {
        let paged = is_paged(bc);
        let query = esc_attr(q.vars.get("s").unwrap_or(""));
        let trimmed = wp_trim_words_with(&query, 10, "&hellip;", site.locale_data().word_count_type);
        items.push(create_item(bc, rustpress_core::i18n::sprintf(&site.tr("Search results for: \"%s\""), &[&trimmed]), paged));
        if paged {
            items.push(page_number_item(bc, "paged"));
        }
    } else if q.flags.is_404 {
        items.push(Item::plain(site.tr("Page not found")));
    } else if q.flags.is_archive {
        items.extend(archive_items(bc).await?);
    } else {
        let (Some(context_id), Some(context_type)) = (bc.context.post_id(), bc.context.get("postType").and_then(Value::as_str)) else { return Ok(String::new()) };
        let mut post_id = context_id;
        let mut post_type = context_type.to_owned();
        let Some(post) = (match global_post(bc).await? {
            Some(post) if post.id == post_id => Some(post),
            _ => site.db.post(post_id).await?.map(std::sync::Arc::new),
        }) else {
            return Ok(String::new());
        };
        let mut post_parent = post.post_parent;
        if !is_hierarchical_type(bc, &post_type)
            && post_parent != 0
            && let Some(parent) = site.db.post(post_parent).await?
        {
            post_id = parent.id;
            post_type = parent.post_type.clone();
            post_parent = parent.post_parent;
        }
        let show_terms = if !is_hierarchical_type(bc, &post_type) && post_parent == 0 {
            true
        } else if object_taxonomies(bc, &post_type).is_empty() {
            false
        } else {
            bc.attr_bool("prefersTaxonomy")
        };

        // The archive of the post type.
        let archive_link = if post_type == "post" {
            if site.settings.show_on_front == ShowOnFront::Page && page_for_posts != 0 {
                match site.db.post(page_for_posts).await? {
                    Some(page) => Some(site.permalink(&page).await?),
                    None => None,
                }
            } else {
                Some(site.home_url(""))
            }
        } else {
            site.post_type_archive_link(&post_type)
        };
        if let Some(link) = archive_link.filter(|l| untrailingslashit(&site.home_url("")) != untrailingslashit(l)) {
            let label = if post_type == "post" && page_for_posts != 0 { post_title_of(bc, page_for_posts).await? } else { site.post_type_archives_label(&post_type) };
            items.push(Item::link(label, link));
        }

        if !show_terms {
            for ancestor in post_ancestors(bc, post_id).await?.iter().rev() {
                items.push(Item { label: post_title(bc, ancestor), url: Some(site.permalink(ancestor).await?), allow_html: true });
            }
        } else {
            items.extend(terms_items(bc, post_id, &post_type).await?);
        }

        let page = query_var(bc, "page");
        let cpage = query_var(bc, "cpage");
        let title = post_title(bc, &post);
        if page > 1 || cpage > 1 {
            items.push(Item { label: title, url: Some(site.permalink(&post).await?), allow_html: true });
            items.push(page_number_item(bc, if cpage > 1 { "cpage" } else { "page" }));
        } else {
            items.push(Item { label: title, url: None, allow_html: true });
        }
    }

    if !bc.attr_bool("showCurrentItem") && !items.is_empty() {
        items.pop();
    }
    if items.is_empty() {
        return Ok(String::new());
    }

    let separator = bc.attr_str("separator").unwrap_or("/");
    let escaped: String = separator.chars().flat_map(|c| if c == '\\' || c == '"' { vec!['\\', c] } else { vec![c] }).collect();
    let wrapper = bc.wrapper_attributes(&[("style", format!("--separator: \"{escaped}\";")), ("aria-label", site.tr("Breadcrumbs"))]);
    let list: String = items
        .iter()
        .map(|item| {
            let label = if item.allow_html { item.label.clone() } else { esc_html(&item.label) };
            match item.url.as_deref().filter(|u| !u.is_empty()) {
                Some(url) => format!("<li><a href=\"{}\">{label}</a></li>", esc_url(url)),
                None => format!("<li><span aria-current=\"page\">{label}</span></li>"),
            }
        })
        .collect();
    Ok(format!("<nav {wrapper}><ol>{list}</ol></nav>"))
}
