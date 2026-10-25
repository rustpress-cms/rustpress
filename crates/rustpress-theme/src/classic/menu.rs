//! Classic navigation menus: `wp_get_nav_menu_items()`, `_wp_menu_item_classes_by_context()`,
//! `Walker_Nav_Menu` and `wp_nav_menu()`.

use rustpress_core::formatting::{capital_p_dangit, convert_chars, esc_attr, esc_url};
use rustpress_core::{Queried, QueryResult, Site};
use rustpress_db::{OrderBy, Order, Post, PostMeta, PostQuery};
use rustpress_phpser::Value;

use crate::Result;

/// A menu item as `wp_setup_nav_menu_item()` leaves it.
#[derive(Debug, Clone, Default)]
pub struct MenuItem {
    /// `ID` and `db_id`.
    pub id: u64,
    pub parent: u64,
    pub object_id: u64,
    pub object: String,
    pub kind: String,
    pub title: String,
    pub url: String,
    pub target: String,
    pub attr_title: String,
    pub xfn: String,
    pub classes: Vec<String>,
    pub current: bool,
    pub menu_order: i64,
    /// `post_parent` of the linked page, for `current-page-parent`.
    pub post_parent: u64,
}

/// The menu assigned to a theme location.
#[derive(Debug, Clone)]
pub struct Menu {
    pub term_id: u64,
    pub slug: String,
    pub items: Vec<MenuItem>,
}

/// The `title` filters of `the_title` that apply to a stored title.
pub(super) fn the_title_filters(site: &Site, title: &str) -> String {
    capital_p_dangit(convert_chars(&site.texturize(title)).trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\0' | '\x0B')), true)
}

fn meta_value<'a>(meta: &'a [PostMeta], key: &str) -> Option<&'a str> {
    meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.as_deref())
}

/// `get_nav_menu_locations()` for the theme: location -> menu term id.
pub fn theme_mod(site: &Site, stylesheet: &str) -> Value {
    site.options.value(&format!("theme_mods_{stylesheet}")).unwrap_or(Value::Null)
}

pub fn menu_location(mods: &Value, location: &str) -> Option<u64> {
    mods.get("nav_menu_locations")?.get(location)?.as_i64().filter(|id| *id > 0).map(|id| id as u64)
}

/// `has_nav_menu()`: a menu is assigned to the location and the menu exists.
pub async fn has_nav_menu(site: &Site, mods: &Value, location: &str) -> Result<bool> {
    let Some(id) = menu_location(mods, location) else { return Ok(false) };
    Ok(site.db.term_by_id(Some("nav_menu"), id).await?.is_some())
}

/// `wp_get_nav_menu_items()` with `wp_setup_nav_menu_item()`; invalid items are left out.
pub async fn load_menu(site: &Site, term_id: u64) -> Result<Option<Menu>> {
    let Some(term) = site.db.term_by_id(Some("nav_menu"), term_id).await? else { return Ok(None) };
    let mut items = Vec::new();
    if term.count > 0 {
        let query = PostQuery {
            post_types: vec!["nav_menu_item".into()],
            tax_groups: vec![vec![term.term_taxonomy_id]],
            order_by: OrderBy::MenuOrder,
            order: Order::Asc,
            ..PostQuery::default()
        };
        for post in site.db.posts(&query).await? {
            if let Some(item) = setup_item(site, &post).await? {
                items.push(item);
            }
        }
    }
    // `wp_list_sort( 'menu_order' )` is stable, then the items are numbered from 1.
    items.sort_by_key(|i| i.menu_order);
    for (n, item) in items.iter_mut().enumerate() {
        item.menu_order = n as i64 + 1;
    }
    Ok(Some(Menu { term_id, slug: term.slug, items }))
}

async fn setup_item(site: &Site, post: &Post) -> Result<Option<MenuItem>> {
    let meta = site.db.post_meta(post.id).await?;
    let get = |key: &str| meta_value(&meta, key).unwrap_or("").to_owned();
    let kind = get("_menu_item_type");
    let object = get("_menu_item_object");
    let object_id: u64 = get("_menu_item_object_id").parse().unwrap_or(0);
    let mut item = MenuItem {
        id: post.id,
        parent: get("_menu_item_menu_item_parent").parse().unwrap_or(0),
        object_id,
        object: object.clone(),
        kind: kind.clone(),
        target: get("_menu_item_target"),
        attr_title: post.post_excerpt.clone(),
        xfn: get("_menu_item_xfn"),
        menu_order: i64::from(post.menu_order),
        ..MenuItem::default()
    };
    item.classes = match meta_value(&meta, "_menu_item_classes").map(rustpress_core::options::maybe_unserialize) {
        Some(Value::Array(list)) => list.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        Some(Value::Str(s)) => vec![String::from_utf8_lossy(&s).into_owned()],
        _ => Vec::new(),
    };
    match kind.as_str() {
        "post_type" => {
            let Some(target) = site.db.post(object_id).await? else { return Ok(None) };
            if target.post_status == "trash" {
                return Ok(None);
            }
            item.url = site.permalink(&target).await?;
            item.post_parent = target.post_parent;
            let mut original = the_title_filters(site, &target.post_title);
            if original.is_empty() {
                original = rustpress_core::i18n::sprintf(&site.tr("#%d (no title)"), &[&object_id.to_string()]);
            }
            item.title = if post.post_title.is_empty() { original } else { post.post_title.clone() };
        }
        "taxonomy" => {
            let Some(term) = site.db.term_by_id(Some(&object), object_id).await? else { return Ok(None) };
            item.url = site.term_link(&term).await?;
            let original = term.name.clone();
            item.title = if post.post_title.is_empty() { original } else { post.post_title.clone() };
        }
        "post_type_archive" => {
            item.title = post.post_title.clone();
            item.url = site.home_url("/");
        }
        _ => {
            item.title = post.post_title.clone();
            item.url = get("_menu_item_url");
        }
    }
    Ok(Some(item))
}

/// The queried object, as far as menu classes need it.
struct QueriedObject<'a> {
    id: u64,
    post: Option<&'a Post>,
    term: Option<&'a rustpress_db::Term>,
}

fn queried_of(q: &QueryResult) -> QueriedObject<'_> {
    match &q.queried {
        Queried::Post(p) => QueriedObject { id: p.id, post: Some(p), term: None },
        Queried::Term(t) => QueriedObject { id: t.term_id, post: None, term: Some(t) },
        Queried::User(u) => QueriedObject { id: u.id, post: None, term: None },
        Queried::None | Queried::PostType(_) => QueriedObject { id: 0, post: None, term: None },
    }
}

/// `_wp_menu_item_classes_by_context()`.
pub async fn classes_by_context(site: &Site, q: &QueryResult, request_uri: &str, items: &mut [MenuItem]) -> Result<()> {
    let queried = queried_of(q);
    let flags = &q.flags;
    let page_for_posts = site.settings.page_for_posts;
    let page_on_front = site.settings.page_on_front;
    let privacy_page: u64 = site.options.u64("wp_page_for_privacy_policy", 0);
    let home = site.home_url("");
    let front_page_url = home.clone();
    let origin = match site.settings.home.split_once("://") {
        Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or("")),
        None => String::new(),
    };

    let mut active_ancestor_items: Vec<u64> = Vec::new();
    let mut active_parent_items: Vec<u64> = Vec::new();
    let mut active_parent_objects: Vec<u64> = Vec::new();
    let mut active_object = String::new();

    // Categories of the queried post (a flat taxonomy lookup of the hierarchical ones).
    let mut possible_object_parents: Vec<u64> = Vec::new();
    let mut possible_taxonomy_ancestors: Vec<(String, u64)> = Vec::new();
    let hierarchical_taxonomies = ["category"];
    if flags.is_singular() && queried.post.is_some_and(|p| p.post_type != "page") {
        for taxonomy in hierarchical_taxonomies {
            let terms = site.db.terms_for_post(queried.id, Some(taxonomy)).await?;
            let all = site.db.terms(taxonomy).await?;
            for term in &terms {
                possible_object_parents.push(term.term_id);
                let mut at = term.term_id;
                let mut guard = 0;
                loop {
                    possible_taxonomy_ancestors.push((taxonomy.to_owned(), at));
                    guard += 1;
                    match all.iter().find(|t| t.term_id == at).map(|t| t.parent) {
                        Some(parent) if parent != 0 && guard < 50 => at = parent,
                        _ => break,
                    }
                }
            }
        }
    } else if let Some(term) = queried.term
        && hierarchical_taxonomies.contains(&term.taxonomy.as_str())
    {
        let all = site.db.terms(&term.taxonomy).await?;
        let mut at = term.term_id;
        let mut guard = 0;
        loop {
            possible_taxonomy_ancestors.push((term.taxonomy.clone(), at));
            guard += 1;
            match all.iter().find(|t| t.term_id == at).map(|t| t.parent) {
                Some(parent) if parent != 0 && guard < 50 => at = parent,
                _ => break,
            }
        }
    }

    // Ancestors of the queried page.
    let mut page_ancestors: Vec<u64> = Vec::new();
    if let Some(post) = queried.post
        && post.post_type == "page"
    {
        let mut parent = post.post_parent;
        while parent != 0 && !page_ancestors.contains(&parent) {
            page_ancestors.push(parent);
            parent = site.db.post(parent).await?.map_or(0, |p| p.post_parent);
        }
    }

    // `_menu_item_menu_item_parent` of every item, to walk up the menu tree.
    let parents: Vec<(u64, u64)> = items.iter().map(|i| (i.id, i.parent)).collect();
    let parent_of = |id: u64| parents.iter().find(|(i, _)| *i == id).map_or(0, |(_, p)| *p);

    for item in items.iter_mut() {
        item.current = false;
        let mut classes = item.classes.clone();
        classes.push("menu-item".into());
        classes.push(format!("menu-item-type-{}", item.kind));
        classes.push(format!("menu-item-object-{}", item.object));
        if item.kind == "post_type" && page_on_front == item.object_id {
            classes.push("menu-item-home".into());
        }
        if item.kind == "post_type" && privacy_page == item.object_id {
            classes.push("menu-item-privacy-policy".into());
        }
        let walk_ancestors = |start: u64, out: &mut Vec<u64>| {
            let mut ancestor = start;
            loop {
                ancestor = parent_of(ancestor);
                if ancestor == 0 || out.contains(&ancestor) {
                    break;
                }
                out.push(ancestor);
            }
        };
        if flags.is_singular() && item.kind == "taxonomy" && possible_object_parents.contains(&item.object_id) {
            active_parent_objects.push(item.object_id);
            active_parent_items.push(item.id);
            active_object = queried.post.map(|p| p.post_type.clone()).unwrap_or_default();
        } else if item.object_id == queried.id
            && ((page_for_posts != 0 && item.kind == "post_type" && flags.is_home && page_for_posts == item.object_id)
                || (item.kind == "post_type" && flags.is_singular())
                || (item.kind == "taxonomy" && (flags.is_category || flags.is_tag || flags.is_tax) && queried.term.is_some_and(|t| t.taxonomy == item.object)))
        {
            classes.push("current-menu-item".into());
            item.current = true;
            walk_ancestors(item.id, &mut active_ancestor_items);
            if item.kind == "post_type" && item.object == "page" {
                classes.push("page_item".into());
                classes.push(format!("page-item-{}", item.object_id));
                classes.push("current_page_item".into());
            }
            active_parent_items.push(item.parent);
            active_parent_objects.push(item.post_parent);
            active_object = item.object.clone();
        } else if item.object == "custom" {
            let current_url = format!("{origin}{}", request_uri.trim_end_matches('/'));
            let raw_item_url = item.url.split('#').next().unwrap_or("").to_owned();
            let item_url = raw_item_url.trim_end_matches('/').to_owned();
            let indexless = current_url.trim_end_matches("index.php").trim_end_matches('/').to_owned();
            let root_relative = request_uri.trim_end_matches('/').to_owned();
            let candidates = [current_url.clone(), indexless.clone(), root_relative];
            if !raw_item_url.is_empty() && candidates.contains(&item_url) {
                classes.push("current-menu-item".into());
                item.current = true;
                walk_ancestors(item.id, &mut active_ancestor_items);
                if home.trim_end_matches('/') == current_url.trim_end_matches('/') || home.trim_end_matches('/') == indexless.trim_end_matches('/') {
                    classes.push("current_page_item".into());
                }
                active_parent_items.push(item.parent);
                active_parent_objects.push(item.post_parent);
                active_object = item.object.clone();
            } else if item_url == front_page_url.trim_end_matches('/') && flags.is_front_page {
                classes.push("current-menu-item".into());
            }
            if item_url == home.trim_end_matches('/') {
                classes.push("menu-item-home".into());
            }
        }
        if page_for_posts != 0 && item.kind == "post_type" && !flags.is_page && page_for_posts == item.object_id {
            classes.push("current_page_parent".into());
        }
        dedup(&mut classes);
        item.classes = classes;
    }

    active_ancestor_items.retain(|i| *i != 0);
    active_parent_items.retain(|i| *i != 0);
    active_parent_objects.retain(|i| *i != 0);
    dedup(&mut active_ancestor_items);
    dedup(&mut active_parent_items);
    dedup(&mut active_parent_objects);

    for item in items.iter_mut() {
        let mut classes = item.classes.clone();
        let post_ancestor = item.kind == "post_type"
            && queried.post.is_some_and(|p| p.post_type == "page")
            && page_ancestors.contains(&item.object_id)
            && item.object_id != queried.id;
        let tax_ancestor = item.kind == "taxonomy"
            && possible_taxonomy_ancestors.iter().any(|(tax, id)| *tax == item.object && *id == item.object_id)
            && queried.term.is_none_or(|t| t.term_id != item.object_id);
        if post_ancestor || tax_ancestor {
            if let Some(term) = queried.term {
                classes.push(format!("current-{}-ancestor", term.taxonomy));
            } else if let Some(post) = queried.post {
                classes.push(format!("current-{}-ancestor", post.post_type));
            }
        }
        let is_ancestor = active_ancestor_items.contains(&item.id);
        let is_parent = active_parent_items.contains(&item.id);
        if is_ancestor {
            classes.push("current-menu-ancestor".into());
        }
        if is_parent {
            classes.push("current-menu-parent".into());
        }
        if active_parent_objects.contains(&item.object_id) {
            classes.push(format!("current-{active_object}-parent"));
        }
        if item.kind == "post_type" && item.object == "page" {
            if is_parent {
                classes.push("current_page_parent".into());
            }
            if is_ancestor {
                classes.push("current_page_ancestor".into());
            }
        }
        dedup(&mut classes);
        item.classes = classes;
    }
    Ok(())
}

fn dedup<T: PartialEq + Clone>(list: &mut Vec<T>) {
    let mut seen: Vec<T> = Vec::new();
    list.retain(|v| {
        if seen.contains(v) {
            false
        } else {
            seen.push(v.clone());
            true
        }
    });
}

/// The arguments of `wp_nav_menu()` that the themes here use.
#[derive(Debug, Clone)]
pub struct NavArgs {
    pub theme_location: String,
    pub menu_id: String,
    pub menu_class: String,
    pub depth: usize,
    pub link_before: String,
    pub link_after: String,
    /// The container element (`div` by default) and, for `nav`, its `aria-label`.
    pub container: Option<(String, Option<String>)>,
    /// Whether the items get an `id="menu-item-N"` (the nav menu widget removes them).
    pub item_ids: bool,
}

impl NavArgs {
    pub fn new(location: &str) -> Self {
        Self { theme_location: location.to_owned(), menu_id: String::new(), menu_class: "menu".into(), depth: 0, link_before: String::new(), link_after: String::new(), container: Some(("div".into(), None)), item_ids: true }
    }
}

/// `wp_nav_menu()` for a menu with items; `title_hook` is the theme's `nav_menu_item_title`
/// filter and `link_hook` its `walker_nav_menu_start_el` filter.
pub fn wp_nav_menu(
    site: &Site,
    menu: &Menu,
    items: &[MenuItem],
    args: &NavArgs,
    title_hook: &dyn Fn(&str, &MenuItem, usize) -> String,
    link_hook: &dyn Fn(String, &MenuItem, usize) -> String,
) -> String {
    let mut sorted: Vec<MenuItem> = items.to_vec();
    let with_children: Vec<u64> = sorted.iter().filter(|i| i.parent != 0 && i.parent != i.id).map(|i| i.parent).collect();
    for item in sorted.iter_mut() {
        if item.parent == item.id {
            item.parent = 0;
        }
        if with_children.contains(&item.id) {
            item.classes.push("menu-item-has-children".into());
        }
    }
    let mut body = String::new();
    walk(site, &sorted, 0, 0, args.depth, args, title_hook, link_hook, &mut body);
    if body.is_empty() {
        return String::new();
    }
    let id = if args.menu_id.is_empty() { format!("menu-{}", menu.slug) } else { args.menu_id.clone() };
    let list = format!("<ul id=\"{}\" class=\"{}\">{body}</ul>", esc_attr(&id), esc_attr(&args.menu_class));
    match &args.container {
        Some((tag, label)) => {
            let aria = match (tag.as_str(), label) {
                ("nav", Some(label)) if !label.is_empty() => format!(" aria-label=\"{}\"", esc_attr(label)),
                _ => String::new(),
            };
            format!("<{tag} class=\"menu-{}-container\"{aria}>{list}</{tag}>", menu.slug)
        }
        None => list,
    }
}

#[allow(clippy::too_many_arguments)]
fn walk(
    site: &Site,
    items: &[MenuItem],
    parent: u64,
    depth: usize,
    max_depth: usize,
    args: &NavArgs,
    title_hook: &dyn Fn(&str, &MenuItem, usize) -> String,
    link_hook: &dyn Fn(String, &MenuItem, usize) -> String,
    out: &mut String,
) {
    for item in items.iter().filter(|i| i.parent == parent) {
        let indent = "\t".repeat(depth);
        let mut classes: Vec<String> = item.classes.iter().filter(|c| !c.is_empty()).cloned().collect();
        classes.push(format!("menu-item-{}", item.id));
        if args.item_ids {
            out.push_str(&format!("{indent}<li id=\"menu-item-{}\" class=\"{}\">", item.id, esc_attr(&classes.join(" "))));
        } else {
            out.push_str(&format!("{indent}<li class=\"{}\">", esc_attr(&classes.join(" "))));
        }
        let filtered = the_title_filters(site, &item.title);
        let title = title_hook(&filtered, item, depth);
        let mut atts = String::new();
        let mut attribute = |name: &str, value: &str, url: bool| {
            if !value.is_empty() {
                atts.push_str(&format!(" {name}=\"{}\"", if url { esc_url(value) } else { esc_attr(value) }));
            }
        };
        attribute("target", &item.target, false);
        attribute("rel", &item.xfn, false);
        attribute("href", &item.url, true);
        attribute("aria-current", if item.current { "page" } else { "" }, false);
        let same = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();
        if !item.attr_title.is_empty() && !same(&item.attr_title, &item.title) && !same(&item.attr_title, &filtered) && !same(&item.attr_title, &title) {
            attribute("title", &item.attr_title, false);
        }
        let link = format!("<a{atts}>{}{title}{}</a>", args.link_before, args.link_after);
        out.push_str(&link_hook(link, item, depth));
        let has_children = (max_depth == 0 || depth + 1 < max_depth) && items.iter().any(|i| i.parent == item.id);
        if has_children {
            out.push_str(&format!("\n{indent}<ul class=\"sub-menu\">\n"));
            walk(site, items, item.id, depth + 1, max_depth, args, title_hook, link_hook, out);
            out.push_str(&format!("{indent}</ul>\n"));
        }
        out.push_str("</li>\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_keeps_the_first_of_each() {
        let mut classes = vec!["a".to_owned(), "b".to_owned(), "a".to_owned()];
        dedup(&mut classes);
        assert_eq!(classes, ["a", "b"]);
    }

    #[tokio::test]
    async fn nested_items_are_wrapped_like_walker_nav_menu() {
        let item = |id: u64, parent: u64, title: &str| MenuItem { id, parent, title: title.into(), url: format!("http://x/?p={id}"), classes: vec!["menu-item".into()], menu_order: id as i64, ..MenuItem::default() };
        let items = vec![item(1, 0, "A"), item(2, 1, "B"), item(3, 0, "C")];
        let mut out = String::new();
        let args = NavArgs::new("top");
        let hook = |t: &str, _: &MenuItem, _: usize| t.to_owned();
        let link = |l: String, _: &MenuItem, _: usize| l;
        // The site is only used for the title filters; texturizing plain words changes nothing.
        let site = rustpress_core::Site::from_parts(rustpress_db::Db::connect_lazy("mysql://nobody@127.0.0.1:1/none", "wp_").expect("lazy pool"), rustpress_core::options::Options::from_pairs([("home", "http://x")]));
        walk(&site, &items, 0, 0, 0, &args, &hook, &link, &mut out);
        assert_eq!(
            out,
            "<li id=\"menu-item-1\" class=\"menu-item menu-item-1\"><a href=\"http://x/?p=1\">A</a>\n<ul class=\"sub-menu\">\n\t<li id=\"menu-item-2\" class=\"menu-item menu-item-2\"><a href=\"http://x/?p=2\">B</a></li>\n</ul>\n</li>\n<li id=\"menu-item-3\" class=\"menu-item menu-item-3\"><a href=\"http://x/?p=3\">C</a></li>\n"
        );
    }
}
