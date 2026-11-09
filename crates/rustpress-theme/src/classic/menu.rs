//! Classic navigation menus: `wp_get_nav_menu_items()`, `_wp_menu_item_classes_by_context()`,
//! `Walker_Nav_Menu` and `wp_nav_menu()`.

use rustpress_core::formatting::{capital_p_dangit, convert_chars, esc_attr, esc_url};
use rustpress_core::{Queried, QueryResult, Site};
use rustpress_db::{OrderBy, Order, Post, PostMeta, PostQuery};
use rustpress_phpser::Value;

use rustpress_core::customize::Preview;

use crate::Result;

/// A menu item as `wp_setup_nav_menu_item()` leaves it.
#[derive(Debug, Clone, Default)]
pub struct MenuItem {
    /// `ID` and `db_id`; a negative one is an item the Customizer's preview adds.
    pub id: i64,
    pub parent: i64,
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
    /// `description`: `wp_trim_words( post_content, 200 )`.
    pub description: String,
}

/// The menu assigned to a theme location.
#[derive(Debug, Clone)]
pub struct Menu {
    /// Negative for a menu the Customizer's preview adds.
    pub term_id: i64,
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

/// The menu of a location; a negative one is a menu the Customizer's preview adds.
pub fn menu_location(mods: &Value, location: &str) -> Option<i64> {
    mods.get("nav_menu_locations")?.get(location)?.as_i64().filter(|id| *id != 0)
}

/// The previewed `nav_menu[…]` setting of a menu: `Some(None)` when the preview deletes it.
fn previewed_menu(preview: Option<&Preview>, term_id: i64) -> Option<Option<&serde_json::Value>> {
    preview?.nav_menus.iter().find(|(id, _)| *id == term_id).map(|(_, value)| value.as_ref())
}

/// `has_nav_menu()`: a menu is assigned to the location and the menu exists.
pub async fn has_nav_menu(site: &Site, preview: Option<&Preview>, mods: &Value, location: &str) -> Result<bool> {
    let Some(id) = menu_location(mods, location) else { return Ok(false) };
    if let Some(menu) = previewed_menu(preview, id) {
        return Ok(menu.is_some());
    }
    Ok(id > 0 && site.db.term_by_id(Some("nav_menu"), id as u64).await?.is_some())
}

/// `wp_get_nav_menu_items()` with `wp_setup_nav_menu_item()`; invalid items are left out. In the preview of the
/// Customizer the menu and its items are the previewed ones.
pub async fn load_menu(site: &Site, preview: Option<&Preview>, term_id: i64) -> Result<Option<Menu>> {
    let term = if term_id > 0 { site.db.term_by_id(Some("nav_menu"), term_id as u64).await? } else { None };
    // `WP_Customize_Nav_Menu_Setting::filter_wp_get_nav_menu_object()`: a previewed menu has no items in the
    // database (its `count` is 0).
    let (slug, term) = match previewed_menu(preview, term_id) {
        Some(None) => return Ok(None),
        Some(Some(value)) => (rustpress_core::slug::sanitize_title_save(value.get("name").and_then(serde_json::Value::as_str).unwrap_or_default(), &site.locale()), None),
        None => match term {
            Some(term) => (term.slug.clone(), Some(term)),
            None => return Ok(None),
        },
    };
    let mut items = Vec::new();
    if let Some(term) = term.as_ref().filter(|term| term.count > 0) {
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
    if let Some(preview) = preview.filter(|p| !p.nav_menu_items.is_empty()) {
        preview_items(site, preview, term_id, &mut items).await?;
        // `sort_wp_get_nav_menu_items()`.
        items.sort_by_key(|i| i.menu_order);
        for (n, item) in items.iter_mut().enumerate() {
            item.menu_order = n as i64 + 1;
        }
    }
    Ok(Some(Menu { term_id, slug, items }))
}

/// `WP_Customize_Nav_Menu_Item_Setting::filter_wp_get_nav_menu_items()` of each previewed item: it leaves the menu
/// it was in, is updated in the menu it is in now, or is added at its end.
async fn preview_items(site: &Site, preview: &Preview, term_id: i64, items: &mut Vec<MenuItem>) -> Result<()> {
    for (post_id, value) in &preview.nav_menu_items {
        // `$original_nav_menu_term_id`: the menu of the stored item.
        let original = if *post_id > 0 {
            match site.db.post(*post_id as u64).await? {
                Some(post) if post.post_type == "nav_menu_item" => site.db.terms_for_post(post.id, Some("nav_menu")).await?.first().map_or(0, |t| t.term_id as i64),
                _ => 0,
            }
        } else {
            0
        };
        let current = value.as_ref().and_then(|v| v.get("nav_menu_term_id")).and_then(serde_json::Value::as_i64);
        if term_id != original && Some(term_id) != current {
            continue;
        }
        let invalid = value.as_ref().is_some_and(|v| v.get("_invalid") == Some(&serde_json::Value::Bool(true)));
        if value.is_none() || invalid || (original == term_id && current != Some(original)) {
            items.retain(|item| item.id != *post_id);
            continue;
        }
        let Some(value) = value else { continue };
        let item = previewed_item(site, *post_id, value).await?;
        match items.iter_mut().find(|i| i.id == *post_id) {
            Some(slot) => *slot = item,
            None => items.push(item),
        }
    }
    Ok(())
}

/// `value_as_wp_post_nav_menu_item()`: the item of a previewed value, its address from the object it links.
async fn previewed_item(site: &Site, post_id: i64, value: &serde_json::Value) -> Result<MenuItem> {
    let text = |key: &str| value.get(key).and_then(serde_json::Value::as_str).unwrap_or_default().to_owned();
    let int = |key: &str| value.get(key).and_then(serde_json::Value::as_i64).unwrap_or(0);
    let kind = text("type");
    let object = text("object");
    let object_id = int("object_id").max(0) as u64;
    let mut item = MenuItem {
        id: post_id,
        parent: int("menu_item_parent"),
        object_id,
        object: object.clone(),
        kind: kind.clone(),
        url: text("url"),
        target: text("target"),
        // `nav_menu_attr_title`: `wptexturize()` and `strip_tags()`.
        attr_title: rustpress_core::formatting::strip_tags(&site.texturize(&text("attr_title"))),
        xfn: text("xfn"),
        classes: text("classes").split(' ').map(str::to_owned).collect(),
        menu_order: int("position"),
        ..MenuItem::default()
    };
    // `get_original_title()`, for an item without a title of its own.
    let mut original = String::new();
    match kind.as_str() {
        "post_type" if object_id != 0 => {
            let target = site.db.post(object_id).await?;
            if let Some(target) = &target {
                original = the_title_filters(site, &target.post_title);
                if original.is_empty() {
                    original = rustpress_core::i18n::sprintf(&site.tr("#%d (no title)"), &[&object_id.to_string()]);
                }
            }
            item.url = match &target {
                Some(target) => site.permalink(target).await?,
                None => String::new(),
            };
        }
        "taxonomy" if object_id != 0 && !object.is_empty() => match site.db.term_by_id(Some(&object), object_id).await? {
            Some(term) => {
                original = term.name.clone();
                item.url = site.term_link(&term).await?;
            }
            None => item.url = String::new(),
        },
        "post_type_archive" if !object.is_empty() => item.url = site.post_type_archive_link(&object).unwrap_or_default(),
        _ => {}
    }
    item.title = text("title");
    if item.title.is_empty() {
        item.title = original;
    }
    Ok(item)
}

async fn setup_item(site: &Site, post: &Post) -> Result<Option<MenuItem>> {
    let meta = site.db.post_meta(post.id).await?;
    let get = |key: &str| meta_value(&meta, key).unwrap_or("").to_owned();
    let kind = get("_menu_item_type");
    let object = get("_menu_item_object");
    let object_id: u64 = get("_menu_item_object_id").parse().unwrap_or(0);
    let mut item = MenuItem {
        id: post.id as i64,
        parent: get("_menu_item_menu_item_parent").parse().unwrap_or(0),
        object_id,
        object: object.clone(),
        kind: kind.clone(),
        target: get("_menu_item_target"),
        attr_title: post.post_excerpt.clone(),
        xfn: get("_menu_item_xfn"),
        description: rustpress_core::formatting::wp_trim_words(&post.post_content, 200, "&hellip;"),
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

    let mut active_ancestor_items: Vec<i64> = Vec::new();
    let mut active_parent_items: Vec<i64> = Vec::new();
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
    let parents: Vec<(i64, i64)> = items.iter().map(|i| (i.id, i.parent)).collect();
    let parent_of = |id: i64| parents.iter().find(|(i, _)| *i == id).map_or(0, |(_, p)| *p);

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
        let walk_ancestors = |start: i64, out: &mut Vec<i64>| {
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
            // `$matches`: each address also `urldecode()`d.
            let decode = |s: &str| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.as_bytes())).into_owned();
            let candidates = [current_url.clone(), decode(&current_url), indexless.clone(), decode(&indexless), root_relative.clone(), decode(&root_relative)];
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

/// A value of the arguments a partial sends back, as PHP has it after `json_decode( $json, true )`.
pub fn json_to_php(value: &serde_json::Value) -> rustpress_phpser::Value {
    use rustpress_phpser::{Key, Value as Php};
    match value {
        serde_json::Value::Null => Php::Null,
        serde_json::Value::Bool(b) => Php::Bool(*b),
        serde_json::Value::Number(n) => n.as_i64().map_or_else(|| Php::Float(n.as_f64().unwrap_or(0.0)), Php::Int),
        serde_json::Value::String(s) => Php::Str(s.as_bytes().to_vec()),
        serde_json::Value::Array(items) => Php::Array(items.iter().enumerate().map(|(i, v)| (Key::Int(i as i64), json_to_php(v))).collect()),
        serde_json::Value::Object(map) => Php::Array(map.iter().map(|(k, v)| (Key::Str(k.as_bytes().to_vec()), json_to_php(v))).collect()),
    }
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
    /// The items that printed their `id="menu-item-N"` earlier in the request (`_nav_menu_item_id_use_once()`).
    pub used_ids: std::collections::HashSet<i64>,
    /// The `menu` argument: a menu's term id, or none when the menu comes from the theme location.
    pub menu: Option<i64>,
    /// `fallback_cb`.
    pub fallback_cb: &'static str,
    /// The preview of the Customizer, which marks the menu as a partial (`WP_Customize_Nav_Menus`).
    pub customize: Option<std::sync::Arc<rustpress_core::customize::Preview>>,
    /// `container_class`: the class of the container instead of `menu-{slug}-container`.
    pub container_class: String,
    /// `items_wrap`: `%1$s` is the id of the list, `%2$s` its class, `%3$s` the items.
    pub items_wrap: String,
    /// The `nav_menu_item_args` filter of the theme: it may change `link_after` for the item, and the change stays for
    /// the items that follow (the arguments are one object).
    pub item_args: Option<fn(&mut String, &MenuItem, usize)>,
}

impl NavArgs {
    pub fn new(location: &str) -> Self {
        Self {
            theme_location: location.to_owned(),
            menu_id: String::new(),
            menu_class: "menu".into(),
            depth: 0,
            link_before: String::new(),
            link_after: String::new(),
            container: Some(("div".into(), None)),
            used_ids: Default::default(),
            menu: None,
            fallback_cb: "wp_page_menu",
            customize: None,
            container_class: String::new(),
            items_wrap: "<ul id=\"%1$s\" class=\"%2$s\">%3$s</ul>".into(),
            item_args: None,
        }
    }

    /// `WP_Customize_Nav_Menus::filter_wp_nav_menu_args()`: the arguments exported to the preview, sorted by key, and
    /// their `args_hmac` (`wp_hash( serialize( $args ) )`). The menus here are echoed, have a location or a menu and a
    /// container, so they can always be refreshed on their own.
    fn exported(&self) -> (String, serde_json::Value) {
        use rustpress_phpser::{Key, Value as Php};
        let text = |s: &str| Php::Str(s.as_bytes().to_vec());
        let (container, label) = match &self.container {
            Some((tag, label)) => (text(tag), label.clone().unwrap_or_default()),
            None => (Php::Bool(false), String::new()),
        };
        let pairs: Vec<(&str, Php)> = vec![
            ("after", text("")),
            ("before", text("")),
            ("can_partial_refresh", Php::Bool(true)),
            ("container", container),
            ("container_aria_label", text(&label)),
            ("container_class", text("")),
            ("container_id", text("")),
            ("depth", Php::Int(self.depth as i64)),
            ("echo", Php::Bool(true)),
            ("fallback_cb", text(self.fallback_cb)),
            ("item_spacing", text("preserve")),
            ("items_wrap", text("<ul id=\"%1$s\" class=\"%2$s\">%3$s</ul>")),
            ("link_after", text(&self.link_after)),
            ("link_before", text(&self.link_before)),
            ("menu", self.menu.map_or_else(|| text(""), Php::Int)),
            ("menu_class", text(&self.menu_class)),
            ("menu_id", text(&self.menu_id)),
            ("theme_location", text(&self.theme_location)),
            ("walker", text("")),
        ];
        let array = Php::Array(pairs.iter().map(|(k, v)| (Key::Str(k.as_bytes().to_vec()), v.clone())).collect());
        let hmac = rustpress_blocks::commenter::wp_hash(&String::from_utf8_lossy(&rustpress_phpser::serialize(&array)), "auth");
        let mut json = serde_json::Map::new();
        for (key, value) in pairs {
            let value = match value {
                Php::Bool(b) => serde_json::Value::Bool(b),
                Php::Int(i) => serde_json::json!(i),
                Php::Str(bytes) => serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned()),
                _ => serde_json::Value::Null,
            };
            json.insert(key.to_owned(), value);
        }
        json.insert("args_hmac".into(), serde_json::Value::String(hmac.clone()));
        (hmac, serde_json::Value::Object(json))
    }

    /// `_nav_menu_item_id_use_once()`: takes the ids used so far, and marks the ones of this menu.
    pub fn use_ids_once(&mut self, env: &rustpress_blocks::Env<'_>, items: &[MenuItem]) {
        self.used_ids = env.with_state(|state| {
            let used = state.nav_menu_item_ids.clone();
            state.nav_menu_item_ids.extend(items.iter().map(|item| item.id));
            used
        });
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
    let with_children: Vec<i64> = sorted.iter().filter(|i| i.parent != 0 && i.parent != i.id).map(|i| i.parent).collect();
    for item in sorted.iter_mut() {
        if item.parent == item.id {
            item.parent = 0;
        }
        if with_children.contains(&item.id) {
            item.classes.push("menu-item-has-children".into());
        }
    }
    let mut body = String::new();
    let mut link_after = args.link_after.clone();
    // `Walker::walk()`: without items at the top, the parent of the first item is the root.
    let root = if sorted.iter().any(|i| i.parent == 0) { 0 } else { sorted.first().map_or(0, |i| i.parent) };
    walk(site, &sorted, root, 0, args.depth, args, &mut link_after, title_hook, link_hook, &mut body);
    // The orphans, whose parent is not in the menu, follow at the top without their children.
    if args.depth == 0 {
        let mut shown: Vec<i64> = vec![root];
        let mut at = 0;
        while at < shown.len() {
            let parent = shown[at];
            shown.extend(sorted.iter().filter(|i| i.parent == parent && !shown.contains(&i.id)).map(|i| i.id).collect::<Vec<_>>());
            at += 1;
        }
        let mut groups: Vec<i64> = Vec::new();
        for item in sorted.iter().filter(|i| !shown.contains(&i.id) && i.parent != root) {
            if !groups.contains(&item.parent) {
                groups.push(item.parent);
            }
        }
        for parent in groups {
            for item in sorted.iter().filter(|i| i.parent == parent && !shown.contains(&i.id)) {
                walk(site, std::slice::from_ref(item), parent, 0, 1, args, &mut link_after, title_hook, link_hook, &mut body);
            }
        }
    }
    if body.is_empty() {
        return String::new();
    }
    let id = if args.menu_id.is_empty() { format!("menu-{}", menu.slug) } else { args.menu_id.clone() };
    let list = rustpress_core::i18n::sprintf(&args.items_wrap, &[&esc_attr(&id), &esc_attr(&args.menu_class), &body]);
    let html = match &args.container {
        Some((tag, label)) => {
            let aria = match (tag.as_str(), label) {
                ("nav", Some(label)) if !label.is_empty() => format!(" aria-label=\"{}\"", esc_attr(label)),
                _ => String::new(),
            };
            let class = if args.container_class.is_empty() { format!("menu-{}-container", menu.slug) } else { args.container_class.clone() };
            format!("<{tag} class=\"{}\"{aria}>{list}</{tag}>", esc_attr(&class))
        }
        None => list,
    };
    // `WP_Customize_Nav_Menus::filter_wp_nav_menu()`: the attributes of the partial on the first element.
    match &args.customize {
        Some(preview) => {
            let (hmac, exported) = args.exported();
            let attributes = format!(
                " data-customize-partial-id=\"{}\" data-customize-partial-type=\"nav_menu_instance\" data-customize-partial-placement-context=\"{}\"",
                esc_attr(&format!("nav_menu_instance[{hmac}]")),
                esc_attr(&rustpress_blocks::php_json::encode(&exported))
            );
            preview.add_nav_menu_args(&hmac, exported);
            let name_end = html.char_indices().skip(1).find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_')).map_or(html.len(), |(i, _)| i);
            if html.starts_with('<') && name_end > 1 { format!("{} {attributes}{}", &html[..name_end], &html[name_end..]) } else { html }
        }
        None => html,
    }
}

#[allow(clippy::too_many_arguments)]
fn walk(
    site: &Site,
    items: &[MenuItem],
    parent: i64,
    depth: usize,
    max_depth: usize,
    args: &NavArgs,
    link_after: &mut String,
    title_hook: &dyn Fn(&str, &MenuItem, usize) -> String,
    link_hook: &dyn Fn(String, &MenuItem, usize) -> String,
    out: &mut String,
) {
    for item in items.iter().filter(|i| i.parent == parent) {
        let indent = "\t".repeat(depth);
        let mut classes: Vec<String> = item.classes.iter().filter(|c| !c.is_empty()).cloned().collect();
        classes.push(format!("menu-item-{}", item.id));
        if !args.used_ids.contains(&item.id) {
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
        if let Some(filter) = args.item_args {
            filter(link_after, item, depth);
        }
        attribute("target", &item.target, false);
        attribute("rel", &item.xfn, false);
        attribute("href", &item.url, true);
        attribute("aria-current", if item.current { "page" } else { "" }, false);
        let same = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();
        if !item.attr_title.is_empty() && !same(&item.attr_title, &item.title) && !same(&item.attr_title, &filtered) && !same(&item.attr_title, &title) {
            attribute("title", &item.attr_title, false);
        }
        let link = format!("<a{atts}>{}{title}{link_after}</a>", args.link_before);
        out.push_str(&link_hook(link, item, depth));
        let has_children = (max_depth == 0 || depth + 1 < max_depth) && items.iter().any(|i| i.parent == item.id);
        if has_children {
            out.push_str(&format!("\n{indent}<ul class=\"sub-menu\">\n"));
            walk(site, items, item.id, depth + 1, max_depth, args, link_after, title_hook, link_hook, out);
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
        let item = |id: i64, parent: i64, title: &str| MenuItem { id, parent, title: title.into(), url: format!("http://x/?p={id}"), classes: vec!["menu-item".into()], menu_order: id, ..MenuItem::default() };
        let items = vec![item(1, 0, "A"), item(2, 1, "B"), item(3, 0, "C")];
        let mut out = String::new();
        let args = NavArgs::new("top");
        let hook = |t: &str, _: &MenuItem, _: usize| t.to_owned();
        let link = |l: String, _: &MenuItem, _: usize| l;
        // The site is only used for the title filters; texturizing plain words changes nothing.
        let site = rustpress_core::Site::from_parts(rustpress_db::Db::connect_lazy("mysql://nobody@127.0.0.1:1/none", "wp_").expect("lazy pool"), rustpress_core::options::Options::from_pairs([("home", "http://x")]));
        walk(&site, &items, 0, 0, 0, &args, &mut String::new(), &hook, &link, &mut out);
        assert_eq!(
            out,
            "<li id=\"menu-item-1\" class=\"menu-item menu-item-1\"><a href=\"http://x/?p=1\">A</a>\n<ul class=\"sub-menu\">\n\t<li id=\"menu-item-2\" class=\"menu-item menu-item-2\"><a href=\"http://x/?p=2\">B</a></li>\n</ul>\n</li>\n<li id=\"menu-item-3\" class=\"menu-item menu-item-3\"><a href=\"http://x/?p=3\">C</a></li>\n"
        );
    }
}
