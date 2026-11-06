//! `nav-menus.php` of a classic theme: the menu editor, the meta boxes that add items, and the menu locations
//! (`wp-admin/includes/nav-menu.php`, `Walker_Nav_Menu_Edit`, `Walker_Nav_Menu_Checklist`).
//!
//! Differs from WordPress: the filters of the screen (`nav_menu_meta_box_object`, `wp_edit_nav_menu_walker`,
//! `wp_nav_locations_listed_per_menu`, ...) have no native counterpart, and the registered locations are those of
//! Twenty Seventeen, the one classic theme that runs.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::{add_query_arg, remove_query_arg};
use rustpress_db::{Order, OrderBy, Post, PostQuery, Term};
use serde_json::json;

use super::meta_boxes::submit_button;
use super::notices::notice;
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen, ScreenOptions};
use crate::content::{Content, MenuItemData, PostData};

const ITEM: &str = include_str!("../data/nav_menu_item.tpl");

/// The arguments the links of the screen leave out (`$removed_args`).
const REMOVED_ARGS: [&str; 6] = ["action", "customlink-tab", "edit-menu-item", "menu-item", "page-tab", "_wpnonce"];

/// `(int) $value`.
fn php_int(value: &str) -> i64 {
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, digits) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
    let number = digits[..end].parse::<i64>().unwrap_or(0);
    if negative { -number } else { number }
}

/// `htmlspecialchars( $text, ENT_QUOTES )`.
fn special_chars(text: &str) -> String {
    crate::sanitize::esc_textarea(text)
}

/// `get_registered_nav_menus()`: the locations of Twenty Seventeen, with their translated names.
pub(super) fn registered_locations(ctx: &Ctx<'_>) -> Vec<(String, String)> {
    let i18n = ctx.site().i18n();
    vec![("top".to_owned(), i18n.gettext("twentyseventeen", "Top Menu")), ("social".to_owned(), i18n.gettext("twentyseventeen", "Social Links Menu"))]
}

/// `get_nav_menu_locations()`, from the table: a page after a save comes before the options are read again.
pub(super) async fn menu_locations(ctx: &Ctx<'_>) -> Vec<(String, i64)> {
    let name = format!("theme_mods_{}", ctx.site().settings.stylesheet);
    let stored = ctx.env.db.option(&name).await.ok().flatten().unwrap_or_default();
    let mods = rustpress_core::options::maybe_unserialize(&stored);
    let Some(locations) = mods.get("nav_menu_locations") else { return Vec::new() };
    locations
        .as_array()
        .unwrap_or_default()
        .iter()
        .map(|(key, value)| {
            let key = match key {
                rustpress_phpser::Key::Int(number) => number.to_string(),
                rustpress_phpser::Key::Str(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            };
            (key, value.as_i64().unwrap_or(0))
        })
        .collect()
}

/// `get_post_type_object( $name )` as far as the boxes use it.
struct BoxType {
    name: String,
    title: String,
    search_items: String,
    all_items: String,
    hierarchical: bool,
}

/// `get_post_types( [ 'show_in_nav_menus' => true ], 'object' )`.
fn box_post_types(ctx: &Ctx<'_>) -> Vec<BoxType> {
    let mut types = vec![
        BoxType { name: "post".into(), title: ctx.tr("Posts"), search_items: ctx.tr("Search Posts"), all_items: ctx.tr("All Posts"), hierarchical: false },
        BoxType { name: "page".into(), title: ctx.tr("Pages"), search_items: ctx.tr("Search Pages"), all_items: ctx.tr("All Pages"), hierarchical: true },
    ];
    let admin = ctx.site().types.admin_post_types();
    for registered in ctx.site().types.registered_types(false).into_iter().filter(|t| t.show_in_nav_menus) {
        let info = admin.iter().find(|t| t.name == registered.name);
        let label = |key: &str, default: &str| info.and_then(|t| t.label(key)).unwrap_or_else(|| ctx.tr(default));
        types.push(BoxType {
            title: label("name", "Posts"),
            search_items: label("search_items", "Search Posts"),
            all_items: label("all_items", "All Posts"),
            hierarchical: registered.hierarchical,
            name: registered.name,
        });
    }
    types
}

/// `get_taxonomies( [ 'show_in_nav_menus' => true ], 'object' )`.
struct BoxTaxonomy {
    name: String,
    title: String,
    most_used: String,
    search_items: String,
    all_items: String,
    hierarchical: bool,
}

fn box_taxonomies(ctx: &Ctx<'_>) -> Vec<BoxTaxonomy> {
    let mut taxonomies = vec![
        BoxTaxonomy { name: "category".into(), title: ctx.tr("Categories"), most_used: ctx.env.site.trx("Most Used", "categories"), search_items: ctx.tr("Search Categories"), all_items: ctx.tr("All Categories"), hierarchical: true },
        BoxTaxonomy { name: "post_tag".into(), title: ctx.tr("Tags"), most_used: ctx.env.site.trx("Most Used", "tags"), search_items: ctx.tr("Search Tags"), all_items: ctx.tr("All Tags"), hierarchical: false },
    ];
    if ctx.env.theme.adds_theme_support("post-formats") || !ctx.env.block_theme {
        taxonomies.push(BoxTaxonomy { name: "post_format".into(), title: ctx.env.site.trx("Formats", "post format"), most_used: ctx.env.site.trx("Most Used", "tags"), search_items: ctx.tr("Search Tags"), all_items: ctx.env.site.trx("Formats", "post format"), hierarchical: false });
    }
    for registered in ctx.site().types.registered_types(true).into_iter().filter(|t| t.show_in_nav_menus) {
        let title = registered.label.clone().unwrap_or_else(|| ctx.tr(if registered.hierarchical { "Categories" } else { "Tags" }));
        taxonomies.push(BoxTaxonomy {
            title,
            most_used: ctx.env.site.trx("Most Used", if registered.hierarchical { "categories" } else { "tags" }),
            search_items: ctx.tr(if registered.hierarchical { "Search Categories" } else { "Search Tags" }),
            all_items: ctx.tr(if registered.hierarchical { "All Categories" } else { "All Tags" }),
            hierarchical: registered.hierarchical,
            name: registered.name,
        });
    }
    taxonomies
}

/// One entry of a checklist: `wp_setup_nav_menu_item()` of a post or term.
#[derive(Debug, Clone, Default)]
struct ChecklistItem {
    /// `ID`.
    id: i64,
    /// The field `Walker::walk()` nests by.
    parent: i64,
    /// `isset( $menu_item->post_type )` and `'nav_menu_item'`.
    menu_item: bool,
    object_id: i64,
    object: String,
    kind: String,
    /// `esc_html()` of it is the label.
    label: String,
    /// `$menu_item->title`.
    title: String,
    url: String,
    front_or_home: bool,
    /// What `_post_states()` adds.
    states: String,
}

/// The state of the walkers: `$_nav_menu_placeholder`.
struct Boxes<'a, 'b> {
    ctx: &'a Ctx<'b>,
    selected: i64,
    placeholder: i64,
    one_location_no_menus: bool,
    /// In an Ajax request `$nav_menu_selected_id` is not set: `disabled()` compares `''` with `'0'`.
    ajax: bool,
}

/// `Walker::walk( $elements, 0, ... )`: top level first, children nested, what has no parent in the list at the end.
fn walk<T: Clone>(elements: &[T], parent: impl Fn(&T) -> i64, id: impl Fn(&T) -> i64, mut start_el: impl FnMut(&mut String, &T, usize), start_lvl: impl Fn(&mut String, usize), end_lvl: impl Fn(&mut String, usize)) -> String {
    let mut output = String::new();
    if elements.is_empty() {
        return output;
    }
    let mut top: Vec<T> = Vec::new();
    let mut children: Vec<(i64, Vec<T>)> = Vec::new();
    fn push<T>(children: &mut Vec<(i64, Vec<T>)>, key: i64, item: T) {
        match children.iter_mut().find(|(k, _)| *k == key) {
            Some((_, list)) => list.push(item),
            None => children.push((key, vec![item])),
        }
    }
    for element in elements {
        if parent(element) == 0 {
            top.push(element.clone());
        } else {
            push(&mut children, parent(element), element.clone());
        }
    }
    if top.is_empty() {
        let root = parent(&elements[0]);
        children.clear();
        for element in elements {
            if parent(element) == root {
                top.push(element.clone());
            } else {
                push(&mut children, parent(element), element.clone());
            }
        }
    }
    struct Walk<'w, T> {
        children: &'w mut Vec<(i64, Vec<T>)>,
    }
    #[allow(clippy::too_many_arguments)]
    fn display<T: Clone>(walk: &mut Walk<'_, T>, element: &T, depth: usize, max_depth: usize, output: &mut String, id: &impl Fn(&T) -> i64, start_el: &mut impl FnMut(&mut String, &T, usize), start_lvl: &impl Fn(&mut String, usize), end_lvl: &impl Fn(&mut String, usize)) {
        start_el(output, element, depth);
        let key = id(element);
        let mut opened = false;
        if (max_depth == 0 || max_depth > depth + 1)
            && let Some(at) = walk.children.iter().position(|(k, _)| *k == key)
        {
            let list = walk.children[at].1.clone();
            for child in &list {
                if !opened {
                    opened = true;
                    start_lvl(output, depth);
                }
                display(walk, child, depth + 1, max_depth, output, id, start_el, start_lvl, end_lvl);
            }
            if let Some(at) = walk.children.iter().position(|(k, _)| *k == key) {
                walk.children.remove(at);
            }
        }
        if opened {
            end_lvl(output, depth);
        }
        // `Walker_Nav_Menu::end_el()`.
        output.push_str("</li>\n");
    }
    let mut state = Walk { children: &mut children };
    for element in &top {
        display(&mut state, element, 0, 0, &mut output, &id, &mut start_el, &start_lvl, &end_lvl);
    }
    // Orphans.
    let orphans: Vec<T> = children.iter().flat_map(|(_, list)| list.iter().cloned()).collect();
    let mut none: Vec<(i64, Vec<T>)> = Vec::new();
    let mut state = Walk { children: &mut none };
    for orphan in &orphans {
        display(&mut state, orphan, 0, 1, &mut output, &id, &mut start_el, &start_lvl, &end_lvl);
    }
    output
}

impl Boxes<'_, '_> {
    /// `wp_nav_menu_disabled_check()`.
    fn disabled(&self) -> &'static str {
        if self.one_location_no_menus || self.selected != 0 || self.ajax { "" } else { " disabled='disabled'" }
    }

    fn next_placeholder(&mut self) -> i64 {
        self.placeholder = if self.placeholder < 0 { self.placeholder - 1 } else { -1 };
        self.placeholder
    }

    /// `Walker_Nav_Menu_Checklist::start_el()`.
    fn checklist_item(&mut self, output: &mut String, item: &ChecklistItem, depth: usize) {
        let placeholder = self.next_placeholder();
        let possible_object_id = if item.menu_item { item.object_id } else { placeholder };
        let possible_db_id = if item.id != 0 && possible_object_id > 0 { item.id } else { 0 };
        output.push_str(&"\t".repeat(depth));
        output.push_str("<li><label class=\"menu-item-title\">");
        output.push_str(&format!("<input type=\"checkbox\"{} class=\"menu-item-checkbox", self.disabled()));
        if item.front_or_home {
            output.push_str(" add-to-top");
        }
        output.push_str(&format!("\" name=\"menu-item[{possible_object_id}][menu-item-object-id]\" value=\"{}\" /> ", item.object_id));
        output.push_str(&esc_html(&item.label));
        output.push_str(&item.states);
        output.push_str("</label>");
        let field = |name: &str, class: &str, value: &str| format!("<input type=\"hidden\" class=\"menu-item-{class}\" name=\"menu-item[{possible_object_id}][menu-item-{name}]\" value=\"{value}\" />");
        output.push_str(&field("db-id", "db-id", &possible_db_id.to_string()));
        output.push_str(&field("object", "object", &esc_attr(&item.object)));
        output.push_str(&field("parent-id", "parent-id", if item.menu_item && item.id == 0 { "" } else { "0" }));
        output.push_str(&field("type", "type", &esc_attr(&item.kind)));
        output.push_str(&field("title", "title", &special_chars(&item.title)));
        output.push_str(&field("url", "url", &esc_url(&item.url)));
        output.push_str(&field("target", "target", ""));
        output.push_str(&field("attr-title", "attr-title", ""));
        output.push_str(&field("classes", "classes", ""));
        output.push_str(&field("xfn", "xfn", ""));
    }

    /// `walk_nav_menu_tree()` with `Walker_Nav_Menu_Checklist`.
    fn checklist(&mut self, items: &[ChecklistItem]) -> String {
        let start_lvl = |output: &mut String, depth: usize| output.push_str(&format!("\n{0}<ul class='children'>\n", "\t".repeat(depth)));
        let end_lvl = |output: &mut String, depth: usize| output.push_str(&format!("\n{}</ul>", "\t".repeat(depth)));
        let mut me = Some(self);
        walk(items, |i| i.parent, |i| i.id, |output, item, depth| me.as_mut().expect("present").checklist_item(output, item, depth), start_lvl, end_lvl)
    }

    /// `wp_nav_menu_item_link_meta_box()`.
    fn custom_links(&mut self) -> String {
        let id = self.next_placeholder();
        let disabled = self.disabled();
        let ctx = self.ctx;
        format!(
            "\t<div class=\"customlinkdiv\" id=\"customlinkdiv\">\n\t\t<input type=\"hidden\" value=\"custom\" name=\"menu-item[{id}][menu-item-type]\" />\n\t\t<p id=\"menu-item-url-wrap\" class=\"wp-clearfix\">\n\t\t\t<label for=\"custom-menu-item-url\">{url}</label>\n\t\t\t<input id=\"custom-menu-item-url\" name=\"menu-item[{id}][menu-item-url]\"\n\t\t\t\ttype=\"text\"{disabled}\t\t\t\tclass=\"code menu-item-textbox form-required\" placeholder=\"https://\"\n\t\t\t/>\n\t\t\t<span id=\"custom-url-error\" class=\"error-message\" style=\"display: none;\">{invalid}</span>\n\t\t</p>\n\n\t\t<p id=\"menu-item-name-wrap\" class=\"wp-clearfix\">\n\t\t\t<label for=\"custom-menu-item-name\">{text}</label>\n\t\t\t<input id=\"custom-menu-item-name\" name=\"menu-item[{id}][menu-item-title]\"\n\t\t\t\ttype=\"text\"{disabled}\t\t\t\tclass=\"regular-text menu-item-textbox\"\n\t\t\t/>\n\t\t</p>\n\n\t\t<p class=\"button-controls button-controls-customlinkdiv\">\n\t\t\t<span class=\"add-to-menu\">\n\t\t\t\t<input id=\"submit-customlinkdiv\" name=\"add-custom-menu-item\"\n\t\t\t\t\ttype=\"submit\"{disabled}\t\t\t\t\tclass=\"button submit-add-to-menu right\" value=\"{add}\"\n\t\t\t\t/>\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t</span>\n\t\t</p>\n\n\t</div><!-- /.customlinkdiv -->\n\t",
            url = ctx.tr("URL"),
            invalid = ctx.tr("Please provide a valid link."),
            text = ctx.tr("Link Text"),
            add = esc_attr(&ctx.tr("Add to Menu")),
        )
    }
}

/// `wp_setup_nav_menu_item()` of a post that is no menu item.
async fn item_of_post(ctx: &Ctx<'_>, post: &Post, nested: bool) -> ChecklistItem {
    let site = ctx.site();
    let mut labelled = post.clone();
    if labelled.post_title.is_empty() {
        labelled.post_title = sprintf(&ctx.tr("#%d (no title)"), &[&post.id.to_string()]);
    }
    let states = if post.post_type == "page" { super::posts::post_states(ctx, post, "", false).await } else { String::new() };
    ChecklistItem {
        id: post.id as i64,
        parent: if nested { post.post_parent as i64 } else { 0 },
        object_id: post.id as i64,
        object: post.post_type.clone(),
        kind: "post_type".to_owned(),
        label: rustpress_blocks::content::the_title_in_admin(site, &labelled),
        title: labelled.post_title.clone(),
        url: site.permalink(post).await.unwrap_or_default(),
        states,
        ..Default::default()
    }
}

/// `_post_format_get_term()`: the name of a format is `get_post_format_string()`.
fn term_name(ctx: &Ctx<'_>, term: &Term) -> String {
    if term.taxonomy != "post_format" {
        return term.name.clone();
    }
    let format = term.slug.strip_prefix("post-format-").unwrap_or(&term.slug);
    let label = match format {
        "aside" => "Aside",
        "chat" => "Chat",
        "gallery" => "Gallery",
        "link" => "Link",
        "image" => "Image",
        "quote" => "Quote",
        "status" => "Status",
        "video" => "Video",
        "audio" => "Audio",
        _ => "Standard",
    };
    ctx.env.site.trx(label, "Post format")
}

/// `wp_setup_nav_menu_item()` of a term.
async fn item_of_term(ctx: &Ctx<'_>, term: &Term, nested: bool) -> ChecklistItem {
    let name = term_name(ctx, term);
    ChecklistItem {
        id: term.term_id as i64,
        parent: if nested { term.parent as i64 } else { 0 },
        object_id: term.term_id as i64,
        object: term.taxonomy.clone(),
        kind: "taxonomy".to_owned(),
        label: name.clone(),
        title: name,
        url: ctx.site().term_link(term).await.unwrap_or_default(),
        ..Default::default()
    }
}

/// `paginate_links()` of a box.
fn box_page_links(ctx: &Ctx<'_>, tab_name: &str, item_type: &str, item_object: &str, total: u64, current: u64) -> String {
    if total < 2 {
        return String::new();
    }
    let mut base = ctx.request.request_uri.clone();
    for (key, value) in [(tab_name, "all"), ("paged", "%#%"), ("item-type", item_type), ("item-object", item_object)] {
        base = add_query_arg(&base, key, value);
    }
    let link = |n: u64| esc_url(&base.replace("%#%", &n.to_string()));
    let page = format!("<span class=\"screen-reader-text\">{}</span> ", ctx.tr("Page"));
    let number = |n: u64| ctx.site().number_i18n(n);
    let mut out = Vec::new();
    if current > 1 {
        out.push(format!("<a class=\"prev page-numbers\" href=\"{}\"><span aria-label=\"{}\">{}</span></a>", link(current - 1), esc_attr(&ctx.tr("Previous page")), ctx.tr("&laquo;")));
    }
    let mut dots = false;
    for n in 1..=total {
        if n == current {
            out.push(format!("<span aria-current=\"page\" class=\"page-numbers current\">{page}{}</span>", number(n)));
            dots = true;
        } else if n <= 1 || (n + 2 >= current && n <= current + 2) || n > total - 1 {
            out.push(format!("<a class=\"page-numbers\" href=\"{}\">{page}{}</a>", link(n), number(n)));
            dots = true;
        } else if dots {
            out.push("<span class=\"page-numbers dots\">&hellip;</span>".to_owned());
            dots = false;
        }
    }
    if current < total {
        out.push(format!("<a class=\"next page-numbers\" href=\"{}\"><span aria-label=\"{}\">{}</span></a>", link(current + 1), esc_attr(&ctx.tr("Next page")), ctx.tr("&raquo;")));
    }
    out.join("\n")
}

/// The links of the tabs: nothing while no menu is selected.
fn tab_url(ctx: &Ctx<'_>, selected: i64, tab_name: &str, value: &str) -> String {
    if selected == 0 {
        return String::new();
    }
    let mut url = ctx.request.request_uri.clone();
    for arg in REMOVED_ARGS {
        url = remove_query_arg(&url, arg);
    }
    add_query_arg(&url, tab_name, value)
}

/// `wp_nav_menu_item_post_type_meta_box()`.
async fn post_type_box(boxes: &mut Boxes<'_, '_>, post_type: &BoxType) -> String {
    let ctx = boxes.ctx;
    let site = ctx.site();
    let db = ctx.env.db;
    let tr = |s: &str| ctx.tr(s);
    let name = post_type.name.as_str();
    let tab_name = format!("{name}-tab");
    let per_page = 50;
    let pagenum = if ctx.request.request(&tab_name).is_some() { ctx.request.request("paged").map_or(1, |p| php_int(p).unsigned_abs()) } else { 1 };
    let offset = if pagenum > 0 { per_page * (pagenum - 1) } else { 0 };
    let nested = post_type.hierarchical;

    // The important pages.
    let mut important: Vec<ChecklistItem> = Vec::new();
    let mut suppressed: Vec<u64> = Vec::new();
    if name == "page" {
        let static_front = site.options.string("show_on_front", "") == "page";
        let front = if static_front { site.options.u64("page_on_front", 0) } else { 0 };
        let front_post = if front != 0 { db.post(front).await.ok().flatten() } else { None };
        match front_post {
            Some(post) => {
                let mut item = item_of_post(ctx, &post, nested).await;
                item.front_or_home = true;
                important.push(item);
                suppressed.push(post.id);
            }
            None => {
                let placeholder = boxes.next_placeholder();
                important.push(ChecklistItem {
                    menu_item: true,
                    object_id: placeholder,
                    kind: "custom".to_owned(),
                    label: rustpress_blocks::content::the_title_in_admin(site, &Post { post_title: site.trx("Home", "nav menu home label"), ..Default::default() }),
                    title: site.trx("Home", "nav menu home label"),
                    url: site.home_url("/"),
                    front_or_home: true,
                    ..Default::default()
                });
            }
        }
        let posts_page = if static_front { site.options.u64("page_for_posts", 0) } else { 0 };
        if posts_page != 0
            && let Ok(Some(post)) = db.post(posts_page).await
        {
            important.push(item_of_post(ctx, &post, nested).await);
            suppressed.push(post.id);
        }
        let privacy = site.options.u64("wp_page_for_privacy_policy", 0);
        if privacy != 0
            && let Ok(Some(post)) = db.post(privacy).await
            && post.post_status == "publish"
        {
            important.push(item_of_post(ctx, &post, nested).await);
            suppressed.push(post.id);
        }
    }
    let base_query = |order_by: OrderBy, order: Order, limit: u64, exclude: &[u64]| PostQuery {
        post_types: vec![name.to_owned()],
        statuses: vec!["publish".to_owned()],
        order_by,
        order,
        limit: Some(limit),
        offset,
        exclude_ids: exclude.to_vec(),
        ..PostQuery::default()
    };
    let order_all = if name == "page" { OrderBy::MenuOrderTitle } else { OrderBy::Title };
    let mut query = base_query(order_all, Order::Asc, per_page, &suppressed);
    let mut found = db.count_posts(&query).await.unwrap_or(0);
    let mut listed = db.posts(&query).await.unwrap_or_default();
    // Only suppress and insert when more than just suppression pages available.
    if listed.is_empty() {
        if suppressed.is_empty() {
            return format!("<p>{}</p>", tr("No items."));
        }
        query.exclude_ids.clear();
        found = db.count_posts(&query).await.unwrap_or(0);
        listed = db.posts(&query).await.unwrap_or_default();
        important.clear();
    }
    let num_pages = found.div_ceil(per_page);
    let page_links = box_page_links(ctx, &tab_name, "post_type", name, num_pages, pagenum);

    let mut current_tab = "most-recent";
    if let Some(tab) = ctx.request.request(&tab_name).filter(|t| matches!(*t, "all" | "search")) {
        current_tab = if tab == "all" { "all" } else { "search" };
    }
    let search_key = format!("quick-search-posttype-{name}");
    let searched_raw = ctx.request.request(&search_key);
    if searched_raw.is_some_and(|s| !s.is_empty() && s != "0") {
        current_tab = "search";
    }
    let selected = boxes.selected;
    let tab = |value: &str, label: &str, data: &str| {
        format!(
            "\t\t\t<li {}>\n\t\t\t\t<a class=\"nav-tab-link\"\n\t\t\t\t\tdata-type=\"{data}\"\n\t\t\t\t\thref=\"{}\"\n\t\t\t\t>\n\t\t\t\t\t{label}\t\t\t\t</a>\n\t\t\t</li>\n",
            if current_tab == value { " class=\"tabs\"" } else { "" },
            esc_url(&format!("{}#{data}", tab_url(ctx, selected, &tab_name, value))),
        )
    };
    let active = |value: &str| if current_tab == value { "tabs-panel-active" } else { "tabs-panel-inactive" };
    let mut h = format!("\t<div id=\"posttype-{name}\" class=\"posttypediv\">\n\t\t<ul id=\"posttype-{name}-tabs\" class=\"posttype-tabs add-menu-item-tabs\">\n");
    h.push_str(&tab("most-recent", &tr("Most Recent"), &format!("tabs-panel-posttype-{name}-most-recent")));
    h.push_str(&tab("all", &tr("View All"), &format!("{name}-all")));
    h.push_str(&tab("search", &tr("Search"), &format!("tabs-panel-posttype-{name}-search")));
    h.push_str("\t\t</ul><!-- .posttype-tabs -->\n\n");
    h.push_str(&format!(
        "\t\t<div id=\"tabs-panel-posttype-{name}-most-recent\"\n\t\t\tclass=\"tabs-panel {}\"\n\t\t\trole=\"region\" aria-label=\"{}\" tabindex=\"0\"\n\t\t>\n\t\t\t<ul id=\"{name}checklist-most-recent\"\n\t\t\t\tclass=\"categorychecklist form-no-clear\"\n\t\t\t>\n\t\t\t\t",
        active("most-recent"),
        esc_attr(&tr("Most Recent"))
    ));
    let recent_posts = db.posts(&base_query(OrderBy::Date, Order::Desc, 15, &suppressed)).await.unwrap_or_default();
    let mut recent = Vec::with_capacity(recent_posts.len());
    for post in &recent_posts {
        recent.push(item_of_post(ctx, post, nested).await);
    }
    h.push_str(&boxes.checklist(&recent));
    h.push_str("\t\t\t</ul>\n\t\t</div><!-- /.tabs-panel -->\n\n");
    let searched = searched_raw.map_or_else(String::new, esc_attr);
    let found_posts = if searched_raw.is_some() {
        let query = PostQuery {
            post_types: vec![name.to_owned()],
            search: rustpress_core::query::parse_search_terms(&searched, false),
            search_phrase: Some(searched.clone()),
            order_by: OrderBy::Date,
            order: Order::Desc,
            limit: Some(5),
            ..PostQuery::default()
        };
        db.posts(&query).await.unwrap_or_default()
    } else {
        Vec::new()
    };
    h.push_str(&format!(
        "\t\t<div id=\"tabs-panel-posttype-{name}-search\"\n\t\t\tclass=\"tabs-panel {}\"\n\t\t\trole=\"region\" aria-label=\"{}\" tabindex=\"0\"\n\t\t>\n\t\t\t\t\t\t<p class=\"quick-search-wrap\">\n\t\t\t\t<label for=\"{search_key}\" class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</label>\n\t\t\t\t<input type=\"search\"{}\t\t\t\t\tclass=\"quick-search\" value=\"{searched}\"\n\t\t\t\t\tname=\"{search_key}\"\n\t\t\t\t\tid=\"{search_key}\"\n\t\t\t\t/>\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t{}\t\t\t</p>\n\n\t\t\t<ul id=\"{name}-search-checklist\"\n\t\t\t\tdata-wp-lists=\"list:{name}\"\n\t\t\t\tclass=\"categorychecklist form-no-clear\"\n\t\t\t>\n\t\t\t",
        active("search"),
        esc_attr(&post_type.search_items),
        tr("Search"),
        boxes.disabled(),
        submit_button(&tr("Search"), "small quick-search-submit hide-if-js", "submit", false, Some(&format!("submit-quick-search-posttype-{name}")), &[]),
    ));
    if !found_posts.is_empty() {
        let mut results = Vec::with_capacity(found_posts.len());
        for post in &found_posts {
            results.push(item_of_post(ctx, post, nested).await);
        }
        h.push_str("\t\t\t\t");
        h.push_str(&boxes.checklist(&results));
        h.push_str("\t\t\t");
    } else if !searched.is_empty() {
        h.push_str(&format!("\t\t\t\t<li>{}</li>\n\t\t\t", tr("No results found.")));
    }
    h.push_str("\t\t\t</ul>\n\t\t</div><!-- /.tabs-panel -->\n\n");
    let pager = if page_links.is_empty() { String::new() } else { format!("\t\t\t\t<div class=\"add-menu-item-pagelinks\">\n\t\t\t\t\t{page_links}\t\t\t\t</div>\n\t\t\t") };
    h.push_str(&format!(
        "\t\t<div id=\"{name}-all\"\n\t\t\tclass=\"tabs-panel tabs-panel-view-all {}\"\n\t\t\trole=\"region\" aria-label=\"{}\" tabindex=\"0\"\n\t\t>\n\t\t\t{pager}\n\t\t\t<ul id=\"{name}checklist\"\n\t\t\t\tdata-wp-lists=\"list:{name}\"\n\t\t\t\tclass=\"categorychecklist form-no-clear\"\n\t\t\t>\n\t\t\t\t",
        active("all"),
        esc_attr(&post_type.all_items),
    ));
    let mut all = important;
    for post in &listed {
        all.push(item_of_post(ctx, post, nested).await);
    }
    h.push_str(&boxes.checklist(&all));
    h.push_str(&format!("\t\t\t</ul>\n\n\t\t\t{pager}\t\t</div><!-- /.tabs-panel -->\n\n"));
    let disabled = boxes.disabled();
    h.push_str(&format!(
        "\t\t<p class=\"button-controls\" data-items-type=\"posttype-{name}\">\n\t\t\t<span class=\"list-controls hide-if-no-js\">\n\t\t\t\t<input type=\"checkbox\"{disabled}\t\t\t\t\tid=\"{tab_name}\" class=\"select-all\"\n\t\t\t\t/>\n\t\t\t\t<label for=\"{tab_name}\">{}</label>\n\t\t\t</span>\n\n\t\t\t<span class=\"add-to-menu\">\n\t\t\t\t<input type=\"submit\"{disabled}\t\t\t\t\tclass=\"button submit-add-to-menu right\" value=\"{}\"\n\t\t\t\t\tname=\"add-post-type-menu-item\" id=\"submit-posttype-{name}\"\n\t\t\t\t/>\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t</span>\n\t\t</p>\n\n\t</div><!-- /.posttypediv -->\n\t",
        tr("Select All"),
        esc_attr(&tr("Add to Menu")),
    ));
    h
}

/// `wp_nav_menu_item_taxonomy_meta_box()`.
async fn taxonomy_box(boxes: &mut Boxes<'_, '_>, taxonomy: &BoxTaxonomy) -> String {
    let ctx = boxes.ctx;
    let db = ctx.env.db;
    let tr = |s: &str| ctx.tr(s);
    let name = taxonomy.name.as_str();
    let tab_name = format!("{name}-tab");
    let per_page = 50usize;
    let pagenum = if ctx.request.request(&tab_name).is_some() { ctx.request.request("paged").map_or(1, |p| php_int(p).unsigned_abs()) } else { 1 };
    let offset = if pagenum > 0 { per_page * (pagenum as usize - 1) } else { 0 };
    let terms_all = db.terms(name).await.unwrap_or_default();
    let terms: Vec<Term> = terms_all.iter().skip(offset).take(per_page).cloned().collect();
    if terms.is_empty() {
        return format!("<p>{}</p>", tr("No items."));
    }
    let num_pages = (terms_all.len() as u64).div_ceil(per_page as u64);
    let page_links = box_page_links(ctx, &tab_name, "taxonomy", name, num_pages, pagenum);
    let nested = taxonomy.hierarchical;
    let mut current_tab = "most-used";
    if let Some(tab) = ctx.request.request(&tab_name) {
        current_tab = match tab {
            "all" => "all",
            "search" => "search",
            _ => "most-used",
        };
    }
    let search_key = format!("quick-search-taxonomy-{name}");
    let searched_raw = ctx.request.request(&search_key);
    if searched_raw.is_some_and(|s| !s.is_empty() && s != "0") {
        current_tab = "search";
    }
    let selected = boxes.selected;
    let tab = |value: &str, label: &str, data: &str| {
        format!(
            "\t\t\t<li {}>\n\t\t\t\t<a class=\"nav-tab-link\"\n\t\t\t\t\tdata-type=\"{data}\"\n\t\t\t\t\thref=\"{}\"\n\t\t\t\t>\n\t\t\t\t\t{label}\t\t\t\t</a>\n\t\t\t</li>\n",
            if current_tab == value { " class=\"tabs\"" } else { "" },
            esc_url(&format!("{}#{data}", tab_url(ctx, selected, &tab_name, value))),
        )
    };
    let active = |value: &str| if current_tab == value { "tabs-panel-active" } else { "tabs-panel-inactive" };
    let mut h = format!("\t<div id=\"taxonomy-{name}\" class=\"taxonomydiv\">\n\t\t<ul id=\"taxonomy-{name}-tabs\" class=\"taxonomy-tabs add-menu-item-tabs\">\n");
    h.push_str(&tab("most-used", &esc_html(&taxonomy.most_used), &format!("tabs-panel-{name}-pop")));
    h.push_str(&tab("all", &tr("View All"), &format!("tabs-panel-{name}-all")));
    h.push_str(&tab("search", &tr("Search"), &format!("tabs-panel-search-taxonomy-{name}")));
    h.push_str("\t\t</ul><!-- .taxonomy-tabs -->\n\n");
    h.push_str(&format!(
        "\t\t<div id=\"tabs-panel-{name}-pop\"\n\t\t\tclass=\"tabs-panel {}\"\n\t\t\trole=\"region\" aria-label=\"{}\" tabindex=\"0\"\n\t\t>\n\t\t\t<ul id=\"{name}checklist-pop\"\n\t\t\t\tclass=\"categorychecklist form-no-clear\"\n\t\t\t>\n\t\t\t\t",
        active("most-used"),
        esc_attr(&taxonomy.most_used)
    ));
    let mut popular = Vec::new();
    for term in db.popular_terms(name, 10).await.unwrap_or_default() {
        popular.push(item_of_term(ctx, &term, false).await);
    }
    h.push_str(&boxes.checklist(&popular));
    h.push_str("\t\t\t</ul>\n\t\t</div><!-- /.tabs-panel -->\n\n");
    let pager = if page_links.is_empty() { String::new() } else { format!("\t\t\t\t<div class=\"add-menu-item-pagelinks\">\n\t\t\t\t\t{page_links}\t\t\t\t</div>\n\t\t\t") };
    h.push_str(&format!(
        "\t\t<div id=\"tabs-panel-{name}-all\"\n\t\t\tclass=\"tabs-panel tabs-panel-view-all {}\"\n\t\t\trole=\"region\" aria-label=\"{}\" tabindex=\"0\"\n\t\t>\n\t\t\t{pager}\n\t\t\t<ul id=\"{name}checklist\"\n\t\t\t\tdata-wp-lists=\"list:{name}\"\n\t\t\t\tclass=\"categorychecklist form-no-clear\"\n\t\t\t>\n\t\t\t\t",
        active("all"),
        esc_attr(&taxonomy.all_items),
    ));
    let mut all = Vec::with_capacity(terms.len());
    for term in &terms {
        all.push(item_of_term(ctx, term, nested).await);
    }
    h.push_str(&boxes.checklist(&all));
    h.push_str(&format!("\t\t\t</ul>\n\n\t\t\t{pager}\t\t</div><!-- /.tabs-panel -->\n\n"));
    let searched = searched_raw.map_or_else(String::new, esc_attr);
    h.push_str(&format!(
        "\t\t<div id=\"tabs-panel-search-taxonomy-{name}\"\n\t\t\tclass=\"tabs-panel {}\"\n\t\t\trole=\"region\" aria-label=\"{}\" tabindex=\"0\">\n\t\t\t\t\t\t<p class=\"quick-search-wrap\">\n\t\t\t\t<label for=\"{search_key}\" class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</label>\n\t\t\t\t<input type=\"search\"\n\t\t\t\t\tclass=\"quick-search\" value=\"{searched}\"\n\t\t\t\t\tname=\"{search_key}\"\n\t\t\t\t\tid=\"{search_key}\"\n\t\t\t\t/>\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t{}\t\t\t</p>\n\n\t\t\t<ul id=\"{name}-search-checklist\"\n\t\t\t\tdata-wp-lists=\"list:{name}\"\n\t\t\t\tclass=\"categorychecklist form-no-clear\"\n\t\t\t>\n\t\t\t",
        active("search"),
        esc_attr(&taxonomy.search_items),
        tr("Search"),
        submit_button(&tr("Search"), "small quick-search-submit hide-if-js", "submit", false, Some(&format!("submit-quick-search-taxonomy-{name}")), &[]),
    ));
    if searched_raw.is_some() {
        let needle = searched.to_lowercase();
        let mut found: Vec<Term> = terms_all.iter().filter(|t| t.name.to_lowercase().contains(&needle) && t.count > 0).cloned().collect();
        found.sort_by_key(|t| std::cmp::Reverse(t.count));
        if !found.is_empty() {
            let mut results = Vec::with_capacity(found.len());
            for term in &found {
                results.push(item_of_term(ctx, term, false).await);
            }
            h.push_str("\t\t\t\t");
            h.push_str(&boxes.checklist(&results));
            h.push_str("\t\t\t");
        } else if !searched.is_empty() {
            h.push_str(&format!("\t\t\t\t<li>{}</li>\n\t\t\t", tr("No results found.")));
        }
    }
    h.push_str("\t\t\t</ul>\n\t\t</div><!-- /.tabs-panel -->\n\n");
    let disabled = boxes.disabled();
    h.push_str(&format!(
        "\t\t<p class=\"button-controls\" data-items-type=\"taxonomy-{name}\">\n\t\t\t<span class=\"list-controls hide-if-no-js\">\n\t\t\t\t<input type=\"checkbox\"{disabled}\t\t\t\t\tid=\"{tab_name}\" class=\"select-all\"\n\t\t\t\t/>\n\t\t\t\t<label for=\"{tab_name}\">{}</label>\n\t\t\t</span>\n\n\t\t\t<span class=\"add-to-menu\">\n\t\t\t\t<input type=\"submit\"{disabled}\t\t\t\t\tclass=\"button submit-add-to-menu right\" value=\"{}\"\n\t\t\t\t\tname=\"add-taxonomy-menu-item\" id=\"submit-taxonomy-{name}\"\n\t\t\t\t/>\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t</span>\n\t\t</p>\n\n\t</div><!-- /.taxonomydiv -->\n\t",
        tr("Select All"),
        esc_attr(&tr("Add to Menu")),
    ));
    h
}

/// `do_accordion_sections( 'nav-menus', 'side', null )`.
fn accordion(sections: &[(String, String, String)], hidden: &[String]) -> String {
    let mut h = "\t<div id=\"side-sortables\" class=\"accordion-container\">\n\t\t<ul class=\"outer-border\">\n\t".to_owned();
    let mut first_open = false;
    for (id, title, body) in sections {
        let hidden_class = if hidden.contains(id) { "hide-if-js" } else { "" };
        let open = !first_open && hidden_class.is_empty();
        first_open |= open;
        h.push_str(&format!(
            "\t\t\t\t\t<li class=\"control-section accordion-section {hidden_class} {} {id}\" id=\"{id}\">\n\t\t\t\t\t\t<h3 class=\"accordion-section-title hndle\">\n\t\t\t\t\t\t\t<button type=\"button\" class=\"accordion-trigger\" aria-expanded=\"{}\" aria-controls=\"{id}-content\">\n\t\t\t\t\t\t\t\t<span class=\"accordion-title\">\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t<span class=\"dashicons dashicons-arrow-down\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t</button>\n\t\t\t\t\t\t</h3>\n\t\t\t\t\t\t<div class=\"accordion-section-content \" id=\"{id}-content\">\n\t\t\t\t\t\t\t<div class=\"inside\">\n\t\t\t\t\t\t\t\t{body}\t\t\t\t\t\t\t</div><!-- .inside -->\n\t\t\t\t\t\t</div><!-- .accordion-section-content -->\n\t\t\t\t\t</li><!-- .accordion-section -->\n\t\t\t\t\t",
            if open { "open" } else { "" },
            if open { "true" } else { "false" },
            esc_html(title),
        ));
    }
    h.push_str("\t\t</ul><!-- .outer-border -->\n\t</div><!-- .accordion-container -->\n\t");
    h
}

/// A menu item as `wp_setup_nav_menu_item()` leaves it for the editor.
#[derive(Debug, Clone, Default)]
pub(super) struct EditItem {
    pub id: u64,
    pub parent: String,
    pub object_id: String,
    pub object: String,
    pub kind: String,
    pub type_label: String,
    pub title: String,
    pub post_excerpt: String,
    pub post_status: String,
    pub url: String,
    pub target: String,
    pub description: String,
    pub classes: Vec<String>,
    pub xfn: String,
    pub invalid: bool,
    /// `$menu_item->label`: Ajax-added items do not show "(Pending)".
    pub label: Option<String>,
    /// `$original_title` of the walker: `false` for custom links and what is gone.
    pub original_title: Option<String>,
    pub menu_order: i64,
}

/// `wp_setup_nav_menu_item()` of a stored item, under `is_admin()`.
pub(super) async fn setup_item(ctx: &Ctx<'_>, post: &Post) -> EditItem {
    let site = ctx.site();
    let db = ctx.env.db;
    let meta = db.post_meta(post.id).await.unwrap_or_default();
    let get = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone()).unwrap_or_default();
    let mut item = EditItem {
        id: post.id,
        parent: get("_menu_item_menu_item_parent"),
        object_id: get("_menu_item_object_id"),
        object: get("_menu_item_object"),
        kind: get("_menu_item_type"),
        post_excerpt: post.post_excerpt.clone(),
        post_status: post.post_status.clone(),
        target: get("_menu_item_target"),
        xfn: get("_menu_item_xfn"),
        description: rustpress_core::formatting::wp_trim_words(&post.post_content, 200, &ctx.tr("&hellip;")),
        menu_order: i64::from(post.menu_order),
        ..Default::default()
    };
    item.classes = match meta.iter().find(|m| m.meta_key.as_deref() == Some("_menu_item_classes")).and_then(|m| m.meta_value.as_deref()).map(rustpress_core::options::maybe_unserialize) {
        Some(rustpress_phpser::Value::Array(list)) => list.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        Some(rustpress_phpser::Value::Str(s)) => vec![String::from_utf8_lossy(&s).into_owned()],
        _ => vec![String::new()],
    };
    let object_id = php_int(&item.object_id).unsigned_abs();
    match item.kind.as_str() {
        "post_type" => {
            let known = matches!(item.object.as_str(), "post" | "page") || site.types.post_type_exists(&item.object);
            let target = if object_id != 0 { db.post(object_id).await.ok().flatten() } else { None };
            if known {
                item.type_label = box_post_types(ctx).into_iter().find(|t| t.name == item.object).map_or_else(|| item.object.clone(), |t| singular_of(ctx, &t.name));
                if let Some(target) = &target {
                    let states = super::posts::state_labels(ctx, target, "", false).await;
                    if !states.is_empty() {
                        item.type_label = rustpress_core::formatting::wp_strip_all_tags(&states.join(", "));
                    }
                }
            } else {
                item.type_label = item.object.clone();
                item.invalid = true;
            }
            let mut original = String::new();
            match &target {
                Some(target) => {
                    if target.post_status == "trash" {
                        item.invalid = true;
                    }
                    item.url = site.permalink(target).await.unwrap_or_default();
                    original = rustpress_blocks::content::the_title_in_admin(site, &Post { post_password: String::new(), post_status: "publish".into(), ..target.clone() });
                    item.original_title = Some(rustpress_blocks::content::the_title_in_admin(site, target));
                }
                None => item.invalid = true,
            }
            if original.is_empty() {
                original = sprintf(&ctx.tr("#%d (no title)"), &[&item.object_id]);
            }
            item.title = if post.post_title.is_empty() { original } else { post.post_title.clone() };
        }
        "post_type_archive" => {
            item.title = post.post_title.clone();
            item.type_label = ctx.tr("Post Type Archive");
            item.url = site.home_url("/");
        }
        "taxonomy" => {
            let terms = box_taxonomies(ctx);
            match terms.iter().find(|t| t.name == item.object) {
                Some(taxonomy) => item.type_label = singular_of_taxonomy(ctx, &taxonomy.name),
                None => {
                    item.type_label = item.object.clone();
                    item.invalid = true;
                }
            }
            let term = db.term_by_id(Some(&item.object), object_id).await.ok().flatten();
            let mut original = String::new();
            match &term {
                Some(term) => {
                    item.url = site.term_link(term).await.unwrap_or_default();
                    original = term.name.clone();
                    item.original_title = Some(term.name.clone());
                }
                None => item.invalid = true,
            }
            if original.is_empty() {
                original = sprintf(&ctx.tr("#%d (no title)"), &[&item.object_id]);
            }
            item.title = if post.post_title.is_empty() { original } else { post.post_title.clone() };
        }
        _ => {
            item.type_label = ctx.tr("Custom Link");
            item.title = post.post_title.clone();
            item.url = get("_menu_item_url");
        }
    }
    item
}

/// `labels->singular_name` of a post type.
fn singular_of(ctx: &Ctx<'_>, name: &str) -> String {
    match name {
        "post" => ctx.env.site.trx("Post", "post type singular name"),
        "page" => ctx.env.site.trx("Page", "post type singular name"),
        _ => ctx.site().types.admin_post_types().into_iter().find(|t| t.name == name).and_then(|t| t.label("singular_name")).unwrap_or_else(|| ctx.tr("Post")),
    }
}

/// `labels->singular_name` of a taxonomy.
fn singular_of_taxonomy(ctx: &Ctx<'_>, name: &str) -> String {
    match name {
        "category" => ctx.env.site.trx("Category", "taxonomy singular name"),
        "post_tag" => ctx.env.site.trx("Tag", "taxonomy singular name"),
        "post_format" => ctx.env.site.trx("Format", "post format"),
        _ => ctx.site().types.registered_types(true).into_iter().find(|t| t.name == name).and_then(|t| t.singular).unwrap_or_else(|| ctx.env.site.trx("Tag", "taxonomy singular name")),
    }
}

/// `wp_get_nav_menu_items( $id, [ 'post_status' => 'any' ] )`, numbered by `menu_order`.
pub(super) async fn menu_items(ctx: &Ctx<'_>, menu: &Term) -> Vec<EditItem> {
    if menu.count <= 0 {
        return Vec::new();
    }
    let query = PostQuery {
        post_types: vec!["nav_menu_item".into()],
        // `any`.
        statuses: ["publish", "draft", "pending", "private", "future"].map(str::to_owned).to_vec(),
        tax_groups: vec![vec![menu.term_taxonomy_id]],
        order_by: OrderBy::MenuOrder,
        order: Order::Asc,
        ..PostQuery::default()
    };
    let mut items = Vec::new();
    for post in ctx.env.db.posts(&query).await.unwrap_or_default() {
        items.push(setup_item(ctx, &post).await);
    }
    items.sort_by_key(|item| item.menu_order);
    for (index, item) in items.iter_mut().enumerate() {
        item.menu_order = index as i64 + 1;
    }
    items
}

/// `Walker_Nav_Menu_Edit::start_el()`.
fn edit_item(ctx: &Ctx<'_>, item: &EditItem, depth: usize) -> String {
    let tr = |s: &str| ctx.tr(s);
    let id = item.id.to_string();
    let editing = ctx.request.get("edit-menu-item") == Some(id.as_str());
    let nonce_url = |action: &str, nonce_action: &str| format!("{}?action={action}&amp;menu-item={id}&amp;_wpnonce={}", ctx.admin_url("nav-menus.php"), ctx.create_nonce(nonce_action));
    let mut classes = vec![format!("menu-item menu-item-depth-{depth}"), format!("menu-item-{}", esc_attr(&item.object)), format!("menu-item-edit-{}", if editing { "active" } else { "inactive" })];
    let mut title = item.title.clone();
    if item.invalid {
        classes.push("menu-item-invalid".to_owned());
        title = sprintf(&tr("%s (Invalid)"), &[&item.title]);
    } else if item.post_status == "draft" {
        classes.push("pending".to_owned());
        title = sprintf(&tr("%s (Pending)"), &[&item.title]);
    }
    if let Some(label) = item.label.as_ref().filter(|label| !label.is_empty()) {
        title = label.clone();
    }
    let url_field = if item.kind == "custom" {
        format!(
            "\t\t\t\t\t<p class=\"field-url description description-wide\">\n\t\t\t\t\t\t<label for=\"edit-menu-item-url-{id}\">\n\t\t\t\t\t\t\t{}<br />\n\t\t\t\t\t\t\t<input type=\"text\" id=\"edit-menu-item-url-{id}\" class=\"widefat code edit-menu-item-url\" name=\"menu-item-url[{id}]\" value=\"{}\" />\n\t\t\t\t\t\t</label>\n\t\t\t\t\t</p>\n\t\t\t\t",
            tr("URL"),
            esc_url(&item.url)
        )
    } else {
        String::new()
    };
    let original = match item.original_title.as_ref().filter(|_| item.kind != "custom") {
        Some(original) => format!(
            "\t\t\t\t\t\t<p class=\"link-to-original\">\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</p>\n\t\t\t\t\t",
            sprintf(&tr("Original: %s"), &[&format!("<a href=\"{}\">{}</a>", esc_url(&item.url), esc_html(original))])
        ),
        None => String::new(),
    };
    let edit_url = if editing { ctx.admin_url("nav-menus.php") } else { format!("{}?edit-menu-item={id}#menu-item-settings-{id}", ctx.admin_url("nav-menus.php")) };
    let cancel_url = esc_url(&format!("{}?edit-menu-item={id}&cancel={}", ctx.admin_url("nav-menus.php"), ctx.env.now));
    // The walker ends with the indentation of its closing `<?php`.
    let template = format!("{ITEM}\t\t");
    template
        .replace("{t5}", "\t\t\t\t\t")
        .replace("{t4}", "\t\t\t\t")
        .replace("{classes}", &classes.join(" "))
        .replace("{title}", &esc_html(&title))
        .replace("{submenu_style}", if depth == 0 { "style=\"display: none;\"" } else { "" })
        .replace("{sub_item}", &tr("sub item"))
        .replace("{type_label}", &esc_html(&item.type_label))
        .replace("{move_up_url}", &nonce_url("move-up-menu-item", "move-menu_item"))
        .replace("{move_up}", &esc_attr(&tr("Move up")))
        .replace("{move_down_url}", &nonce_url("move-down-menu-item", "move-menu_item"))
        .replace("{move_down}", &esc_attr(&tr("Move down")))
        .replace("{edit_url}", &esc_url(&edit_url))
        .replace("{edit_item}", &esc_attr(&tr("Edit menu item")))
        .replace("{edit}", &tr("Edit"))
        .replace("{url_field}", &url_field)
        .replace("{navigation_label}", &tr("Navigation Label"))
        .replace("{title_value}", &special_chars(&item.title))
        .replace("{title_attribute}", &tr("Title Attribute"))
        .replace("{attr_title}", &special_chars(&item.post_excerpt))
        .replace("{target_checked}", if item.target == "_blank" { " checked='checked'" } else { "" })
        .replace("{new_tab}", &tr("Open link in a new tab"))
        .replace("{css_classes}", &tr("CSS Classes (optional)"))
        .replace("{classes_value}", &special_chars(&item.classes.join(" ")))
        .replace("{xfn_label}", &tr("Link Relationship (XFN)"))
        .replace("{xfn}", &special_chars(&item.xfn))
        .replace("{description_label}", &tr("Description"))
        .replace("{description}", &crate::sanitize::esc_textarea(&item.description))
        .replace("{description_help}", &tr("The description will be displayed in the menu if the active theme supports it."))
        .replace("{menu_parent}", &tr("Menu Parent"))
        .replace("{menu_order}", &tr("Menu Order"))
        .replace("{move}", &tr("Move"))
        .replace("{up_one}", &tr("Up one"))
        .replace("{down_one}", &tr("Down one"))
        .replace("{to_top}", &tr("To the top"))
        .replace("{original}", &original)
        .replace("{delete_url}", &nonce_url("delete-menu-item", &format!("delete-menu_item_{id}")))
        .replace("{remove}", &tr("Remove"))
        .replace("{cancel_url}", &cancel_url)
        .replace("{cancel}", &tr("Cancel"))
        .replace("{object_id}", &esc_attr(&item.object_id))
        .replace("{object}", &esc_attr(&item.object))
        .replace("{parent}", &esc_attr(&item.parent))
        .replace("{position}", &item.menu_order.to_string())
        .replace("{kind}", &esc_attr(&item.kind))
        .replace("{id}", &id)
}

/// `wp_get_nav_menu_to_edit()`, and the deepest level of the menu.
fn menu_to_edit(ctx: &Ctx<'_>, items: &[EditItem]) -> (String, usize) {
    let mut result = format!("<div id=\"menu-instructions\" class=\"post-body-plain{}\"><p>{}</p></div>", if items.is_empty() { "" } else { " menu-instructions-inactive" }, ctx.tr("Add menu items from the column on the left."));
    if items.is_empty() {
        result.push_str(" <ul class=\"menu\" id=\"menu-to-edit\"> </ul>");
        return (result, 0);
    }
    if items.iter().any(|item| item.post_status == "draft") {
        result.push_str(&notice(&ctx.tr("Click Save Menu to make pending menu items public."), "info", false, "", &["notice-alt", "inline"], true));
    }
    if items.iter().any(|item| item.invalid) {
        result.push_str(&notice(&ctx.tr("There are some invalid menu items. Please check or delete them."), "error", false, "", &["notice-alt", "inline"], true));
    }
    let mut deepest = 0;
    let tree = walk(
        items,
        |item| php_int(&item.parent),
        |item| item.id as i64,
        |output, item, depth| {
            deepest = deepest.max(depth);
            output.push_str(&edit_item(ctx, item, depth));
        },
        |_, _| {},
        |_, _| {},
    );
    result.push_str("<ul class=\"menu\" id=\"menu-to-edit\"> ");
    result.push_str(&tree);
    result.push_str(" </ul> ");
    (result, deepest)
}

/// The strings of `$nav_menus_l10n`.
fn l10n(ctx: &Ctx<'_>, one_location_no_menus: bool) -> serde_json::Value {
    let tr = |s: &str| ctx.tr(s);
    json!({
        "oneThemeLocationNoMenus": one_location_no_menus,
        "moveUp": tr("Move up one"),
        "moveDown": tr("Move down one"),
        "moveToTop": tr("Move to the top"),
        "moveUnder": tr("Move under %s"),
        "moveOutFrom": tr("Move out from under %s"),
        "under": tr("Under %s"),
        "outFrom": tr("Out from under %s"),
        "menuFocus": tr("Edit %1$s (%2$s, %3$d of %4$d)"),
        "subMenuFocus": tr("Edit %1$s (%2$s, sub-item %3$d of %4$d under %5$s)"),
        "subMenuMoreDepthFocus": tr("Edit %1$s (%2$s, sub-item %3$d of %4$d under %5$s, level %6$d)"),
        "menuItemDeletion": tr("item %s"),
        "itemsDeleted": tr("Deleted menu item: %s."),
        "itemAdded": tr("Menu item added"),
        "itemRemoved": tr("Menu item removed"),
        "movedUp": tr("Menu item moved up"),
        "movedDown": tr("Menu item moved down"),
        "movedTop": tr("Menu item moved to the top"),
        "movedLeft": tr("Menu item moved out of submenu"),
        "movedRight": tr("Menu item is now a sub-item"),
        "parentUpdated": tr("Menu parent updated"),
        "orderUpdated": tr("Menu order updated"),
    })
}

/// The help tabs of the two views.
fn help(ctx: &Ctx<'_>, locations_screen: bool) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let tab = |id: &str, title: String, content: String| HelpTab { id: id.to_owned(), title, content };
    let widgets = ctx.admin_url("widgets.php");
    let tabs = if locations_screen {
        vec![tab(
            "locations-overview",
            tr("Overview"),
            format!(
                "<p>{}</p><ul><li>{}</li><li>{}</li><li>{}</li></ul>",
                tr("This screen is used for globally assigning menus to locations defined by your theme."),
                tr("To assign menus to one or more theme menu locations, <strong>select a menu from each location&#8217;s dropdown</strong>. When you are finished, <strong>click Save Changes</strong>"),
                tr("To edit a menu currently assigned to a theme menu location, <strong>click the adjacent &#8217;Edit&#8217; link</strong>"),
                tr("To add a new menu instead of assigning an existing one, <strong>click the &#8217;Use new menu&#8217; link</strong>. Your new menu will be automatically assigned to that location in the theme."),
            ),
        )]
    } else {
        vec![
            tab(
                "overview",
                tr("Overview"),
                format!(
                    "<p>{}</p><p>{}</p><p>{}</p><ul><li>{}</li><li>{}</li></ul>",
                    tr("This screen is used for managing your navigation menus."),
                    sprintf(&tr("Menus can be displayed in locations defined by your theme, even used in sidebars by adding a &#8220;Navigation Menu&#8221; widget on the <a href=\"%1$s\">Widgets</a> screen. If your theme does not support the navigation menus feature (the default themes, %2$s and %3$s, do), you can learn about adding this support by following the documentation link to the side."), &[&widgets, "Twenty Twenty", "Twenty Twenty-One"]),
                    tr("From this screen you can:"),
                    tr("Create, edit, and delete menus"),
                    tr("Add, organize, and modify individual menu items"),
                ),
            ),
            tab(
                "menu-management",
                tr("Menu Management"),
                format!(
                    "<p>{}</p><ul><li>{}</li><li>{}</li></ul><p>{}</p>",
                    tr("The menu management box at the top of the screen is used to control which menu is opened in the editor below."),
                    tr("To edit an existing menu, <strong>choose a menu from the dropdown and click Select</strong>"),
                    tr("If you have not yet created any menus, <strong>click the &#8217;create a new menu&#8217; link</strong> to get started"),
                    tr("You can assign individual menus to the theme&#8217;s menu locations by <strong>selecting the desired settings</strong> at the bottom of the menu editor. To assign menus to all theme menu locations at once, <strong>visit the Manage Locations tab</strong> at the top of the screen."),
                ),
            ),
            tab(
                "editing-menus",
                tr("Editing Menus"),
                format!(
                    "<p>{}</p><p>{}</p><ul><li>{}</li><li>{}</li><li>{}</li><li>{}</li></ul>",
                    tr("Each navigation menu may contain a mix of links to pages, categories, custom URLs or other content types. Menu links are added by selecting items from the expanding boxes in the left-hand column below."),
                    tr("<strong>Clicking the arrow to the right of any menu item</strong> in the editor will reveal a standard group of settings. Additional settings such as link target, CSS classes, link relationships, and link descriptions can be enabled and disabled via the Screen Options tab."),
                    tr("Add one or several items at once by <strong>selecting the checkbox next to each item and clicking Add to Menu</strong>"),
                    tr("To add a custom link, <strong>expand the Custom Links section, enter a URL and link text, and click Add to Menu</strong>"),
                    tr("To reorganize menu items, <strong>drag and drop items with your mouse or use your keyboard</strong>. Drag or move a menu item a little to the right to make it a submenu"),
                    tr("Delete a menu item by <strong>expanding it and clicking the Remove link</strong>"),
                ),
            ),
        ]
    };
    let sidebar = format!(
        "<p><strong>{}</strong></p><p>{}</p><p>{}</p>",
        tr("For more information:"),
        tr("<a href=\"https://wordpress.org/documentation/article/appearance-menus-screen/\">Documentation on Menus</a>"),
        tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>"),
    );
    Help { tabs, sidebar }
}

/// `wp_html_excerpt( $name, 40, '&hellip;' )`.
fn truncated(name: &str) -> String {
    static ENTITY_END: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"&[^;\s]{0,6}$").expect("valid pattern"));
    let text = rustpress_core::formatting::wp_strip_all_tags(name);
    let excerpt: String = text.chars().take(40).collect();
    let cut = ENTITY_END.replace(&excerpt, "").into_owned();
    if cut != text { format!("{cut}&hellip;") } else { cut }
}

/// A list of strings as PHP stores it.
fn serialized_list(items: &[&str]) -> String {
    let body: String = items.iter().enumerate().map(|(index, item)| format!("i:{index};s:{}:\"{item}\";", item.len())).collect();
    format!("a:{}:{{{body}}}", items.len())
}

/// The tab of the locations.
fn locations_view(ctx: &Ctx<'_>, locations: &[(String, String)], assigned: &[(String, i64)], menus: &[Term], selected: i64) -> String {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let count = locations.len();
    let mut h = if count == 1 {
        format!("<p>{}</p>", tr("Your theme supports one menu. Select which menu you would like to use."))
    } else {
        format!("<p>{}</p>", sprintf(&site.trn("Your theme supports %s menu. Select which menu appears in each location.", "Your theme supports %s menus. Select which menu appears in each location.", count as i64), &[&site.number_i18n(count as u64)]))
    };
    let locations_url = format!("{}?action=locations", ctx.admin_url("nav-menus.php"));
    h.push_str(&format!(
        "\t<div id=\"menu-locations-wrap\">\n\t\t<form method=\"post\" action=\"{}\">\n\t\t\t<table class=\"widefat fixed\" id=\"menu-locations-table\">\n\t\t\t\t<thead>\n\t\t\t\t<tr>\n\t\t\t\t\t<th scope=\"col\" class=\"manage-column column-locations\">{}</th>\n\t\t\t\t\t<th scope=\"col\" class=\"manage-column column-menus\">{}</th>\n\t\t\t\t</tr>\n\t\t\t\t</thead>\n\t\t\t\t<tbody class=\"menu-locations\">\n\t\t\t\t",
        esc_url(&locations_url),
        tr("Menu Location"),
        tr("Assigned Menu"),
    ));
    for (location, name) in locations {
        let current = assigned.iter().find(|(key, _)| key == location).map(|(_, id)| *id);
        h.push_str(&format!(
            "\t\t\t\t\t<tr class=\"menu-locations-row\">\n\t\t\t\t\t\t<td class=\"menu-location-title\"><label for=\"locations-{location}\">{name}</label></td>\n\t\t\t\t\t\t<td class=\"menu-location-menus\">\n\t\t\t\t\t\t\t<select name=\"menu-locations[{location}]\" id=\"locations-{location}\">\n\t\t\t\t\t\t\t\t<option value=\"0\">&mdash; {} &mdash;</option>\n\t\t\t\t\t\t\t\t",
            esc_html(&tr("Select a Menu")),
        ));
        for menu in menus {
            let chosen = current == Some(menu.term_id as i64);
            h.push_str(&format!(
                "\t\t\t\t\t\t\t\t\t<option {} {} value=\"{}\">\n\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t</option>\n\t\t\t\t\t\t\t\t",
                if chosen { "data-orig=\"true\"" } else { "" },
                if chosen { " selected='selected'" } else { "" },
                menu.term_id,
                truncated(&menu.name),
            ));
        }
        h.push_str("\t\t\t\t\t\t\t</select>\n\t\t\t\t\t\t\t<div class=\"locations-row-links\">\n\t\t\t\t\t\t\t\t");
        if let Some(id) = current.filter(|id| *id != 0) {
            h.push_str(&format!(
                "\t\t\t\t\t\t\t\t<span class=\"locations-edit-menu-link\">\n\t\t\t\t\t\t\t\t\t<a href=\"{}\">\n\t\t\t\t\t\t\t\t\t\t\t<span aria-hidden=\"true\">{}</span>\n\t\t\t\t\t\t\t\t\t\t\t<span class=\"screen-reader-text\">{}</span>\n\t\t\t\t\t\t\t\t\t\t</a>\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t\t",
                esc_url(&format!("{}?action=edit&menu={id}", ctx.admin_url("nav-menus.php"))),
                site.trx("Edit", "menu"),
                tr("Edit selected menu"),
            ));
        }
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t<span class=\"locations-add-menu-link\">\n\t\t\t\t\t\t\t\t\t<a href=\"{}\">{}</a>\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t</div><!-- .locations-row-links -->\n\t\t\t\t\t\t</td><!-- .menu-location-menus -->\n\t\t\t\t\t</tr><!-- .menu-locations-row -->\n\t\t\t\t",
            esc_url(&format!("{}?action=edit&menu=0&use-location={location}", ctx.admin_url("nav-menus.php"))),
            site.trx("Use new menu", "menu"),
        ));
    }
    h.push_str(&format!(
        "\t\t\t\t</tbody>\n\t\t\t</table>\n\t\t\t<p class=\"button-controls wp-clearfix\">{}</p>\n\t\t\t{}\t\t\t<input type=\"hidden\" name=\"menu\" id=\"nav-menu-meta-object-id\" value=\"{selected}\" />\n\t\t</form>\n\t</div><!-- #menu-locations-wrap -->\n\t\t",
        submit_button(&tr("Save Changes"), "primary left", "nav-menu-locations", false, None, &[]),
        ctx.nonce_field("save-menu-locations"),
    ));
    h
}

/// `wp_ajax_add_menu_item()`: the items are saved without a menu and come back as they are edited.
pub(super) async fn ajax_add_menu_item(ctx: &Ctx<'_>) -> String {
    let Some(writer) = ctx.env.writer else { return "0".to_owned() };
    let db = ctx.env.db;
    let content = Content { site: ctx.site(), db, writer, user: &ctx.user, now: ctx.env.now };
    let mut rows = Vec::new();
    for (index, (_, mut fields)) in posted_items(ctx).into_iter().enumerate() {
        let field = |name: &str| fields.iter().rev().find(|(k, _)| k == name).map_or_else(String::new, |(_, v)| v.clone());
        let (kind, object, object_id) = (field("menu-item-type"), field("menu-item-object"), field("menu-item-object-id"));
        if !kind.is_empty() && kind != "custom" && !object_id.is_empty() && object_id != "0" {
            // "Restore the missing menu item properties."
            let description = if kind == "taxonomy" { db.term_by_id(Some(&object), php_int(&object_id).unsigned_abs()).await.ok().flatten().map_or_else(String::new, |term| term.description) } else { String::new() };
            fields.push(("menu-item-description".to_owned(), description));
        }
        rows.push((index.to_string(), fields));
    }
    let ids = save_menu_items(&content, 0, rows).await;
    let mut items = Vec::new();
    for id in ids {
        if let Ok(Some(post)) = db.post(id).await {
            let mut item = setup_item(ctx, &post).await;
            if item.title.is_empty() {
                item.title = ctx.tr("Menu Item");
            }
            item.label = Some(item.title.clone());
            items.push(item);
        }
    }
    if items.is_empty() {
        return String::new();
    }
    walk(&items, |item| php_int(&item.parent), |item| item.id as i64, |output, item, depth| output.push_str(&edit_item(ctx, item, depth)), |_, _| {}, |_, _| {})
}

/// `wp_ajax_menu_locations_save()` after its checks.
pub(super) async fn ajax_locations_save(ctx: &Ctx<'_>) -> &'static str {
    let list: Vec<(String, i64)> = form_list(ctx, "menu-locations").into_iter().map(|(key, value)| (key, php_int(&value).abs())).collect();
    if list.is_empty() {
        return "0";
    }
    let Some(writer) = ctx.env.writer else { return "0" };
    let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    if content.set_nav_menu_locations(&list).await.is_err() {
        return "0";
    }
    "1"
}

/// `wp_ajax_menu_get_metabox()`.
pub(super) async fn ajax_metabox(ctx: &Ctx<'_>) -> String {
    let request = ctx.request;
    let object = request.post_field("item-object").unwrap_or_default();
    let mut boxes = Boxes { ctx, selected: 0, placeholder: 0, one_location_no_menus: false, ajax: true };
    let (kind, markup) = match request.post_field("item-type") {
        Some("post_type") => match box_post_types(ctx).iter().find(|t| t.name == object) {
            Some(post_type) => ("posttype", post_type_box(&mut boxes, post_type).await),
            None => return String::new(),
        },
        Some("taxonomy") => match box_taxonomies(ctx).iter().find(|t| t.name == object) {
            Some(taxonomy) => ("taxonomy", taxonomy_box(&mut boxes, taxonomy).await),
            None => return String::new(),
        },
        _ => return String::new(),
    };
    rustpress_blocks::php_json::encode(&json!({"replace-id": format!("{kind}-{object}"), "markup": markup}))
}

/// `_wp_ajax_menu_quick_search()`.
pub(super) async fn ajax_quick_search(ctx: &Ctx<'_>) -> String {
    let request = ctx.request;
    let site = ctx.site();
    let db = ctx.env.db;
    let kind = request.post_field("type").unwrap_or_default();
    let object_type = request.post_field("object_type").unwrap_or_default();
    let query = request.post_field("q").unwrap_or_default();
    let markup = request.post_field("response-format") == Some("markup");
    let mut boxes = Boxes { ctx, selected: 0, placeholder: 0, one_location_no_menus: false, ajax: true };
    let line = |id: u64, title: &str, post_type: &str| format!("{}\n", rustpress_blocks::php_json::encode(&json!({"ID": id, "post_title": title, "post_type": post_type})));
    let is_post_type = |name: &str| matches!(name, "post" | "page") || site.types.post_type_exists(name);
    let is_taxonomy = |name: &str| matches!(name, "category" | "post_tag" | "post_format") || site.types.is_taxonomy(name);
    let mut out = String::new();
    if kind == "get-post-item" {
        let id = request.post_field("ID").map(|v| php_int(v).unsigned_abs());
        let Some(id) = id else { return out };
        if is_post_type(object_type) {
            let Ok(Some(post)) = db.post(id).await else { return out };
            if markup {
                let item = item_of_post(ctx, &post, false).await;
                out.push_str(&boxes.checklist(&[item]));
            } else {
                out.push_str(&line(id, &rustpress_blocks::content::the_title_in_admin(site, &post), &post.post_type));
            }
        } else if is_taxonomy(object_type)
            && let Ok(Some(term)) = db.term_by_id(Some(object_type), id).await
        {
            if markup {
                let item = item_of_term(ctx, &term, false).await;
                out.push_str(&boxes.checklist(&[item]));
            } else {
                out.push_str(&line(id, &term_name(ctx, &term), object_type));
            }
        }
        return out;
    }
    static SEARCH: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"quick-search-(posttype|taxonomy)-([a-zA-Z0-9_-]*\b)").expect("valid pattern"));
    let Some(found) = SEARCH.captures(kind) else { return out };
    let name = found[2].to_owned();
    if &found[1] == "posttype" {
        if !is_post_type(&name) {
            return out;
        }
        // `search_columns`: only the title.
        let terms = rustpress_core::query::parse_search_terms(query, false);
        let order_by = if name == "page" { OrderBy::MenuOrderTitle } else { OrderBy::Date };
        let posts = db.posts_by_title(&name, &terms, order_by, Order::Desc, 10).await.unwrap_or_default();
        for post in &posts {
            if markup {
                let item = item_of_post(ctx, post, false).await;
                out.push_str(&boxes.checklist(&[item]));
            } else {
                out.push_str(&line(post.id, &rustpress_blocks::content::the_title_in_admin(site, post), &name));
            }
        }
        return out;
    }
    let needle = query.to_lowercase();
    let terms: Vec<Term> = db.terms(&name).await.unwrap_or_default().into_iter().filter(|term| term.name.to_lowercase().contains(&needle)).take(10).collect();
    for term in &terms {
        if markup {
            let item = item_of_term(ctx, term, false).await;
            out.push_str(&boxes.checklist(&[item]));
        } else {
            out.push_str(&line(term.term_id, &term_name(ctx, term), &name));
        }
    }
    out
}

/// What the view needs.
struct View<'a> {
    menus: &'a [Term],
    selected: i64,
    title: String,
    add_new_screen: bool,
    one_location_no_menus: bool,
    locations: &'a [(String, String)],
    assigned: &'a [(String, i64)],
    edit_markup: Option<String>,
    items: Option<usize>,
    side: String,
    auto_add: Vec<i64>,
    messages: Vec<String>,
}

/// The part of the page above the columns.
fn manage_menus(ctx: &Ctx<'_>, view: &View<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let create_url = esc_url(&format!("{}?action=edit&menu=0", ctx.admin_url("nav-menus.php")));
    let count = view.menus.len();
    let mut h = "\t<div class=\"manage-menus\">\n\t\t".to_owned();
    if count < 1 {
        h.push_str(&format!(
            "<span class=\"first-menu-message\">\n\t\t\t{}\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t{}\t\t\t</span>\n\t\t</span><!-- /first-menu-message -->\n\t\t",
            tr("Create your first menu below."),
            tr("Fill in the Menu Name and click the Create Menu button to create your first menu."),
        ));
    } else if count < 2 {
        h.push_str(&format!(
            "\t\t<span class=\"add-edit-menu-action\">\n\t\t\t{}\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t{}\t\t\t</span>\n\t\t</span><!-- /add-edit-menu-action -->\n\t\t",
            sprintf(&tr("Edit your menu below, or <a href=\"%s\">create a new menu</a>. Do not forget to save your changes!"), &[&create_url]),
            tr("Click the Save Menu button to save your changes."),
        ));
    } else {
        h.push_str(&format!(
            "\t\t\t<form method=\"get\" action=\"{}\">\n\t\t\t<input type=\"hidden\" name=\"action\" value=\"edit\" />\n\t\t\t<label for=\"select-menu-to-edit\" class=\"selected-menu\">{}</label>\n\t\t\t<select name=\"menu\" id=\"select-menu-to-edit\">\n\t\t\t\t",
            esc_url(&ctx.admin_url("nav-menus.php")),
            tr("Select a menu to edit:"),
        ));
        if view.add_new_screen {
            h.push_str(&format!("\t\t\t\t\t<option value=\"0\" selected=\"selected\">{}</option>\n\t\t\t\t", tr("&mdash; Select &mdash;")));
        }
        h.push_str("\t\t\t\t");
        for menu in view.menus {
            h.push_str(&format!(
                "\t\t\t\t\t<option value=\"{id}\" {selected}>\n\t\t\t\t\t\t{name}",
                id = menu.term_id,
                selected = if menu.term_id as i64 == view.selected { " selected='selected'" } else { "" },
                name = esc_html(&truncated(&menu.name)),
            ));
            let at: Vec<&str> = view.assigned.iter().filter(|(_, id)| *id == menu.term_id as i64).filter_map(|(key, _)| view.locations.iter().find(|(location, _)| location == key).map(|(_, name)| name.as_str())).collect();
            if !at.is_empty() {
                let listed = &at[..at.len().min(3)];
                h.push_str(&format!(" ({}{})", listed.join(", "), if at.len() > listed.len() { " &hellip;" } else { "" }));
            }
            h.push_str("\t\t\t\t\t</option>\n\t\t\t\t");
        }
        h.push_str(&format!(
            "\t\t\t</select>\n\t\t\t<span class=\"submit-btn\"><input type=\"submit\" class=\"button\" value=\"{}\"></span>\n\t\t\t<span class=\"add-new-menu-action\">\n\t\t\t\t{}\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span>\n\t\t\t</span><!-- /add-new-menu-action -->\n\t\t</form>\n\t\t\t",
            esc_attr(&tr("Select")),
            sprintf(&tr("or <a href=\"%s\">create a new menu</a>. Do not forget to save your changes!"), &[&create_url]),
            tr("Click the Save Menu button to save your changes."),
        ));
    }
    h.push_str("\t</div><!-- /manage-menus -->\n");
    h
}

/// The editor: the two columns.
fn editor(ctx: &Ctx<'_>, view: &View<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let selected = view.selected;
    let disabled = if ctx.request.get("menu").is_some_and(|m| php_int(m) == 0) { " metabox-holder-disabled" } else { "" };
    let save_label = if selected == 0 { tr("Create Menu") } else { tr("Save Menu") };
    let save = |id: &str| submit_button(&save_label, "primary large menu-save", "save_menu", false, Some(id), &[]);
    let mut h = format!(
        "\t<div id=\"nav-menus-frame\" class=\"wp-clearfix\">\n\t<div id=\"menu-settings-column\" class=\"metabox-holder{disabled}\">\n\n\t\t<div class=\"clear\"></div>\n\n\t\t<form id=\"nav-menu-meta\" class=\"nav-menu-meta\" method=\"post\" enctype=\"multipart/form-data\">\n\t\t\t<input type=\"hidden\" name=\"menu\" id=\"nav-menu-meta-object-id\" value=\"{selected}\" />\n\t\t\t<input type=\"hidden\" name=\"action\" value=\"add-menu-item\" />\n\t\t\t{}\t\t\t<h2>{}</h2>\n\t\t\t{}\t\t</form>\n\n\t</div><!-- /#menu-settings-column -->\n\t<div id=\"menu-management-liquid\">\n\t\t<div id=\"menu-management\">\n\t\t\t<form id=\"update-nav-menu\" method=\"post\" enctype=\"multipart/form-data\">\n\t\t\t\t<h2>{}</h2>\n\t\t\t\t<div class=\"menu-edit\">\n\t\t\t\t\t<input type=\"hidden\" name=\"nav-menu-data\">\n\t\t\t\t\t",
        ctx.nonce_field_named("add-menu_item", "menu-settings-column-nonce"),
        tr("Add menu items"),
        view.side,
        tr("Menu structure"),
    );
    h.push_str(&format!(
        "<input type=\"hidden\" id=\"closedpostboxesnonce\" name=\"closedpostboxesnonce\" value=\"{}\" /><input type=\"hidden\" id=\"meta-box-order-nonce\" name=\"meta-box-order-nonce\" value=\"{}\" />{}",
        ctx.create_nonce("closedpostboxes"),
        ctx.create_nonce("meta-box-order"),
        ctx.nonce_field_named("update-nav_menu", "update-nav-menu-nonce"),
    ));
    let name_value = if view.one_location_no_menus { esc_attr(&tr("Menu 1")) } else { esc_attr(&view.title) };
    if view.one_location_no_menus {
        h.push_str("\t\t\t\t\t\t<input type=\"hidden\" name=\"zero-menu-state\" value=\"true\" />\n\t\t\t\t\t\t");
    }
    h.push_str(&format!(
        "\t\t\t\t\t<input type=\"hidden\" name=\"action\" value=\"update\" />\n\t\t\t\t\t<input type=\"hidden\" name=\"menu\" id=\"menu\" value=\"{selected}\" />\n\t\t\t\t\t<div id=\"nav-menu-header\">\n\t\t\t\t\t\t<div class=\"major-publishing-actions wp-clearfix\">\n\t\t\t\t\t\t\t<label class=\"menu-name-label\" for=\"menu-name\">{}</label>\n\t\t\t\t\t\t\t<input name=\"menu-name\" id=\"menu-name\" type=\"text\" class=\"menu-name regular-text menu-item-textbox form-required\" required=\"required\" value=\"{name_value}\"{} />\n\t\t\t\t\t\t\t<div class=\"publishing-action\">\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</div><!-- END .publishing-action -->\n\t\t\t\t\t\t</div><!-- END .major-publishing-actions -->\n\t\t\t\t\t</div><!-- END .nav-menu-header -->\n\t\t\t\t\t<div id=\"post-body\">\n\t\t\t\t\t\t<div id=\"post-body-content\" class=\"wp-clearfix\">\n\t\t\t\t\t\t\t",
        tr("Menu Name"),
        if view.add_new_screen { " aria-describedby=\"menu-name-desc\"" } else { "" },
        save("save_menu_header"),
    ));
    let hide_style = if view.items == Some(0) { "style=\"display: none;\"" } else { "" };
    if !view.add_new_screen {
        let starter = if view.one_location_no_menus {
            tr("Edit your default menu by adding or removing items. Drag the items into the order you prefer. Click Create Menu to save your changes.")
        } else {
            tr("Drag the items into the order you prefer. Click the arrow on the right of the item to reveal additional configuration options.")
        };
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<div class=\"drag-instructions post-body-plain\" {hide_style}>\n\t\t\t\t\t\t\t\t\t<p>{starter}</p>\n\t\t\t\t\t\t\t\t</div>\n\n\t\t\t\t\t\t\t\t<div id=\"nav-menu-bulk-actions-top\" class=\"bulk-actions\" {hide_style}>\n\t\t\t\t\t\t\t\t\t<label class=\"bulk-select-button\" for=\"bulk-select-switcher-top\">\n\t\t\t\t\t\t\t\t\t\t<input type=\"checkbox\" id=\"bulk-select-switcher-top\" name=\"bulk-select-switcher-top\" class=\"bulk-select-switcher\">\n\t\t\t\t\t\t\t\t\t\t<span class=\"bulk-select-button-label\">{}</span>\n\t\t\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t\t\t</div>\n\n\t\t\t\t\t\t\t\t",
            tr("Bulk Select"),
        ));
        match &view.edit_markup {
            Some(markup) => h.push_str(&format!("{markup}\n\t\t\t\t\t\t\t\n\t\t\t\t\t\t\t")),
            None => h.push_str("\t\t\t\t\t\t\t\t\t<ul class=\"menu\" id=\"menu-to-edit\"></ul>\n\t\t\t\t\t\t\t\t\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t"),
        }
    }
    if view.add_new_screen {
        h.push_str(&format!("\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<p class=\"post-body-plain\" id=\"menu-name-desc\">{}</p>\n\t\t\t\t\t\t\t\t", tr("Give your menu a name, then click Create Menu.")));
        if let Some(location) = ctx.request.get("use-location") {
            h.push_str(&format!("\t\t\t\t\t\t\t\t\t<input type=\"hidden\" name=\"use-location\" value=\"{}\" />\n\t\t\t\t\t\t\t\t", esc_attr(location)));
        }
        h.push_str("\n\t\t\t\t\t\t\t\t");
    }
    h.push_str("\n\t\t\t\t\t\t\t");
    if !view.add_new_screen {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t<div id=\"nav-menu-bulk-actions-bottom\" class=\"bulk-actions\" {hide_style}>\n\t\t\t\t\t\t\t\t\t<label class=\"bulk-select-button\" for=\"bulk-select-switcher-bottom\">\n\t\t\t\t\t\t\t\t\t\t<input type=\"checkbox\" id=\"bulk-select-switcher-bottom\" name=\"bulk-select-switcher-top\" class=\"bulk-select-switcher\">\n\t\t\t\t\t\t\t\t\t\t<span class=\"bulk-select-button-label\">{}</span>\n\t\t\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t\t\t\t<input type=\"button\" class=\"deletion menu-items-delete disabled\" value=\"{}\">\n\t\t\t\t\t\t\t\t\t<div id=\"pending-menu-items-to-delete\">\n\t\t\t\t\t\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t\t\t\t\t\t<ul></ul>\n\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t",
            tr("Bulk Select"),
            esc_attr(&tr("Remove Selected Items")),
            tr("List of menu items selected for deletion:"),
        ));
    }
    let no_menus_style = if view.one_location_no_menus { "style=\"display: none;\"" } else { "" };
    let auto_add = view.auto_add.contains(&selected);
    h.push_str(&format!(
        "\n\t\t\t\t\t\t\t<div class=\"menu-settings\" {no_menus_style}>\n\t\t\t\t\t\t\t\t<h3>{}</h3>\n\t\t\t\t\t\t\t\t\n\t\t\t\t\t\t\t\t<fieldset class=\"menu-settings-group auto-add-pages\">\n\t\t\t\t\t\t\t\t\t<legend class=\"menu-settings-group-name howto\">{}</legend>\n\t\t\t\t\t\t\t\t\t<div class=\"menu-settings-input checkbox-input\">\n\t\t\t\t\t\t\t\t\t\t<input type=\"checkbox\"{} name=\"auto-add-pages\" id=\"auto-add-pages\" value=\"1\" /> <label for=\"auto-add-pages\">{}</label>\n\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t</fieldset>\n\n\t\t\t\t\t\t\t\t\n\t\t\t\t\t\t\t\t\t<fieldset class=\"menu-settings-group menu-theme-locations\">\n\t\t\t\t\t\t\t\t\t\t<legend class=\"menu-settings-group-name howto\">{}</legend>\n\t\t\t\t\t\t\t\t\t\t",
        tr("Menu Settings"),
        tr("Auto add pages"),
        if auto_add { " checked='checked'" } else { "" },
        tr("Automatically add new top-level pages to this menu"),
        tr("Menu location"),
    ));
    for (location, description) in view.locations {
        let current = view.assigned.iter().find(|(key, _)| key == location).map(|(_, id)| *id);
        let checked = current == Some(selected) && selected != 0;
        let other = current.filter(|id| *id != 0 && *id != selected);
        let location = esc_attr(location);
        let described = if other.is_some() { format!("\t\t\t\t\t\t\t\t\t\t\t\t\t\taria-describedby=\"theme-location-set-{location}\"\n\t\t\t\t\t\t\t\t\t\t\t\t\t") } else { String::new() };
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t<div class=\"menu-settings-input checkbox-input\">\n\t\t\t\t\t\t\t\t\t\t\t\t<input type=\"checkbox\"{}\t\t\t\t\t\t\t\t\t\t\t\t\tname=\"menu-locations[{location}]\"\n\t\t\t\t\t\t\t\t\t\t\t\t\tid=\"locations-{location}\"\n\t\t\t\t\t\t\t\t\t\t\t\t\tvalue=\"{selected}\"\n\t\t\t\t\t\t\t\t\t\t\t\t\t{described}\t\t\t\t\t\t\t\t\t\t\t\t/>\n\t\t\t\t\t\t\t\t\t\t\t\t<label for=\"locations-{location}\">\n\t\t\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t\t\t\t\t\t\t",
            if checked { " checked='checked'" } else { "" },
            esc_html(description),
        ));
        if let Some(id) = other {
            let name = view.menus.iter().find(|menu| menu.term_id as i64 == id).map_or_else(|| tr("an unknown menu"), |menu| esc_html(&menu.name));
            h.push_str(&format!(
                "\t\t\t\t\t\t\t\t\t\t\t\t\t<span class=\"theme-location-set\" id=\"theme-location-set-{location}\">\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t\t\t\t\t\t",
                sprintf(&ctx.env.site.trx("(Currently set to: %s)", "menu location"), &[&name]),
            ));
        }
        h.push_str("\t\t\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t\t");
    }
    h.push_str(&format!(
        "\t\t\t\t\t\t\t\t\t</fieldset>\n\n\t\t\t\t\t\t\t\t\n\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t</div><!-- /#post-body-content -->\n\t\t\t\t\t</div><!-- /#post-body -->\n\t\t\t\t\t<div id=\"nav-menu-footer\">\n\t\t\t\t\t\t<div class=\"major-publishing-actions\">\n\t\t\t\t\t\t\t<div class=\"publishing-action\">\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</div><!-- END .publishing-action -->\n\t\t\t\t\t\t\t",
        save("save_menu_footer"),
    ));
    if !view.menus.is_empty() {
        h.push_str("\n\t\t\t\t\t\t\t\t");
        if view.add_new_screen {
            h.push_str(&format!(
                "\t\t\t\t\t\t\t\t<span class=\"cancel-action\">\n\t\t\t\t\t\t\t\t\t<a class=\"submitcancel cancellation menu-cancel\" href=\"{}\">{}</a>\t\t\t\t\t\t\t\t</span><!-- END .cancel-action -->\n\t\t\t\t\t\t\t\t",
                esc_url(&ctx.admin_url("nav-menus.php")),
                tr("Cancel"),
            ));
        } else {
            let url = format!("{}?action=delete&amp;menu={selected}&amp;_wpnonce={}", ctx.admin_url("nav-menus.php"), ctx.create_nonce(&format!("delete-nav_menu-{selected}")));
            h.push_str(&format!(
                "\t\t\t\t\t\t\t\t<span class=\"delete-action\">\n\t\t\t\t\t\t\t\t\t<a class=\"submitdelete deletion menu-delete\" href=\"{}\">{}</a>\t\t\t\t\t\t\t\t</span><!-- END .delete-action -->\n\t\t\t\t\t\t\t\t",
                esc_url(&url),
                tr("Delete Menu"),
            ));
        }
        h.push_str("\n\t\t\t\t\t\t\t");
    }
    h.push_str("\t\t\t\t\t\t</div><!-- END .major-publishing-actions -->\n\t\t\t\t\t</div><!-- /#nav-menu-footer -->\n\t\t\t\t</div><!-- /.menu-edit -->\n\t\t\t</form><!-- /#update-nav-menu -->\n\t\t</div><!-- /#menu-management -->\n\t</div><!-- /#menu-management-liquid -->\n\t</div><!-- /#nav-menus-frame -->\n\t");
    h
}

/// `$nav_menu_option['auto_add']`.
async fn auto_add_list(ctx: &Ctx<'_>) -> Vec<i64> {
    let stored = ctx.env.db.option("nav_menu_options").await.ok().flatten().unwrap_or_default();
    let options = rustpress_core::options::maybe_unserialize(&stored);
    let Some(list) = options.get("auto_add") else { return Vec::new() };
    list.as_array().unwrap_or_default().iter().filter_map(|(_, value)| value.as_i64()).collect()
}

/// `wp_get_admin_notice()` of the screen.
fn message(text: &str, class: &str) -> String {
    notice(text, "", true, "message", &[class], true)
}

/// A field `$name[$key]` of a form.
fn form_list(ctx: &Ctx<'_>, name: &str) -> Vec<(String, String)> {
    let prefix = format!("{name}[");
    ctx.request.form.iter().filter_map(|(key, value)| key.strip_prefix(&prefix).and_then(|rest| rest.strip_suffix(']')).map(|rest| (rest.to_owned(), value.clone()))).collect()
}

/// The rows of `$_REQUEST['menu-item']`: `$key => [ $field => $value ]`.
fn posted_items(ctx: &Ctx<'_>) -> Vec<(String, Vec<(String, String)>)> {
    let mut rows: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for (key, value) in &ctx.request.form {
        let Some(rest) = key.strip_prefix("menu-item[") else { continue };
        let Some((id, field)) = rest.split_once("][") else { continue };
        let Some(field) = field.strip_suffix(']') else { continue };
        let entry = (field.to_owned(), value.clone());
        match rows.iter_mut().find(|(known, _)| known == id) {
            Some((_, fields)) => fields.push(entry),
            None => rows.push((id.to_owned(), vec![entry])),
        }
    }
    rows
}

/// `wp_save_nav_menu_items()`.
async fn save_menu_items(content: &Content<'_>, menu_id: u64, rows: Vec<(String, Vec<(String, String)>)>) -> Vec<u64> {
    let mut saved = Vec::new();
    for (key, fields) in rows {
        let field = |name: &str| fields.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.clone());
        let db_id = field("menu-item-db-id");
        let db_id_empty = db_id.as_deref().is_none_or(|v| v.is_empty() || v == "0");
        let object_id_empty = field("menu-item-object-id").is_none_or(|v| v.is_empty() || v == "0");
        let kind = field("menu-item-type");
        let url = field("menu-item-url").unwrap_or_default();
        if object_id_empty && (kind.is_none() || matches!(url.as_str(), "https://" | "http://" | "") || !(kind.as_deref() == Some("custom") && db_id.is_none()) || !db_id_empty) {
            continue;
        }
        let possible = php_int(&key);
        let actual = if db_id_empty || possible < 0 || db_id.as_deref().is_some_and(|v| php_int(v) != possible) { 0 } else { possible as u64 };
        let value = |name: &str| field(name).unwrap_or_default();
        let data = MenuItemData {
            object_id: value("menu-item-object-id"),
            object: value("menu-item-object"),
            parent_id: value("menu-item-parent-id"),
            position: value("menu-item-position"),
            kind: value("menu-item-type"),
            title: value("menu-item-title"),
            url: value("menu-item-url"),
            description: value("menu-item-description"),
            attr_title: value("menu-item-attr-title"),
            target: value("menu-item-target"),
            classes: value("menu-item-classes"),
            xfn: value("menu-item-xfn"),
            status: String::new(),
        };
        if let Ok(id) = content.update_nav_menu_item(menu_id, actual, &data).await {
            saved.push(id);
        }
    }
    saved
}

/// `wp_nav_menu_update_menu_items()`.
async fn update_menu_items(ctx: &Ctx<'_>, content: &Content<'_>, menu_id: u64, title: &str) -> Vec<String> {
    use rustpress_core::OrLog;
    let db = ctx.env.db;
    let mut messages = Vec::new();
    let mut left: Vec<u64> = match db.term_by_id(Some("nav_menu"), menu_id).await.ok().flatten() {
        Some(menu) if menu.count > 0 => {
            let query = PostQuery { post_types: vec!["nav_menu_item".into()], statuses: vec!["draft".into(), "publish".into()], tax_groups: vec![vec![menu.term_taxonomy_id]], ..PostQuery::default() };
            db.posts(&query).await.unwrap_or_default().into_iter().map(|post| post.id).collect()
        }
        _ => Vec::new(),
    };
    let lookup = |name: &str, key: &str| ctx.request.form.iter().rev().find(|(k, _)| *k == format!("{name}[{key}]")).map_or_else(String::new, |(_, value)| value.clone());
    for (key, db_id) in form_list(ctx, "menu-item-db-id") {
        // A menu item title can't be blank.
        let title_field = lookup("menu-item-title", &key);
        if title_field.is_empty() {
            continue;
        }
        let data = MenuItemData {
            object_id: lookup("menu-item-object-id", &key),
            object: lookup("menu-item-object", &key),
            parent_id: lookup("menu-item-parent-id", &key),
            position: lookup("menu-item-position", &key),
            kind: lookup("menu-item-type", &key),
            title: title_field,
            url: lookup("menu-item-url", &key),
            description: lookup("menu-item-description", &key),
            attr_title: lookup("menu-item-attr-title", &key),
            target: lookup("menu-item-target", &key),
            classes: lookup("menu-item-classes", &key),
            xfn: lookup("menu-item-xfn", &key),
            status: String::new(),
        };
        let id = if php_int(&db_id) != php_int(&key) || key.parse::<i64>().is_err() { 0 } else { php_int(&key).unsigned_abs() };
        match content.update_nav_menu_item(menu_id, id, &data).await {
            Ok(saved) => left.retain(|item| *item != saved),
            Err(error) => messages.push(notice(&error, "", false, "message", &["error"], true)),
        }
    }
    // Remove menu items from the menu that weren't in `$_POST`.
    for item in left {
        content.delete_post(item, false).await.or_log();
    }
    // Store 'auto-add' pages.
    let auto_add = ctx.request.post_field("auto-add-pages").is_some_and(|v| !v.is_empty() && v != "0");
    let mut list = auto_add_list(ctx).await;
    let id = menu_id as i64;
    if auto_add {
        if !list.contains(&id) {
            list.push(id);
        }
    } else {
        list.retain(|known| *known != id);
    }
    let menus = db.terms("nav_menu").await.unwrap_or_default();
    list.retain(|known| menus.iter().any(|menu| menu.term_id as i64 == *known));
    let stored = db.option("nav_menu_options").await.ok().flatten().unwrap_or_default();
    let mut options = match rustpress_core::options::maybe_unserialize(&stored) {
        rustpress_phpser::Value::Array(items) => items,
        _ => vec![(rustpress_phpser::Key::Int(0), rustpress_phpser::Value::Bool(false))],
    };
    let auto: Vec<(rustpress_phpser::Key, rustpress_phpser::Value)> = list.iter().enumerate().map(|(index, id)| (rustpress_phpser::Key::Int(index as i64), rustpress_phpser::Value::Int(*id))).collect();
    let key = rustpress_phpser::Key::Str(b"auto_add".to_vec());
    match options.iter_mut().find(|(k, _)| *k == key) {
        Some((_, value)) => *value = rustpress_phpser::Value::Array(auto),
        None => options.push((key, rustpress_phpser::Value::Array(auto))),
    }
    if let Some(writer) = ctx.env.writer {
        let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&rustpress_phpser::Value::Array(options))).into_owned();
        writer.update_option_with_autoload("nav_menu_options", &serialized, "off").await.or_log();
    }
    messages.push(message(&sprintf(&ctx.tr("%s has been updated."), &[&format!("<strong>{title}</strong>")]), "updated"));
    messages
}

/// What the switch at the top of `nav-menus.php` leaves.
enum ActionResult {
    Continue,
    Respond(Box<AdminResponse>),
}

fn expired(ctx: &Ctx<'_>) -> ActionResult {
    ActionResult::Respond(Box::new(die(ctx.env, 403, &ctx.tr("An error occurred."), &ctx.tr("The link you followed has expired."))))
}

/// A menu item for the moves: `(ID, parent)` in the order of `wp_get_nav_menu_items()`, numbered from 1.
async fn ordered_items(ctx: &Ctx<'_>, menu_id: u64) -> Vec<EditItem> {
    match ctx.env.db.term_by_id(Some("nav_menu"), menu_id).await.ok().flatten() {
        Some(menu) if menu.count > 0 => {
            let query = PostQuery { post_types: vec!["nav_menu_item".into()], tax_groups: vec![vec![menu.term_taxonomy_id]], order_by: OrderBy::MenuOrder, order: Order::Asc, ..PostQuery::default() };
            let mut items = Vec::new();
            for post in ctx.env.db.posts(&query).await.unwrap_or_default() {
                items.push(setup_item(ctx, &post).await);
            }
            items.sort_by_key(|item| item.menu_order);
            for (index, item) in items.iter_mut().enumerate() {
                item.menu_order = index as i64 + 1;
            }
            items
        }
        _ => Vec::new(),
    }
}

/// `case 'move-down-menu-item'` and `case 'move-up-menu-item'`.
async fn move_item(ctx: &Ctx<'_>, content: &Content<'_>, up: bool) {
    use rustpress_core::OrLog;
    let db = ctx.env.db;
    let Some(writer) = ctx.env.writer else { return };
    let item_id = ctx.request.request("menu-item").map_or(0, php_int).unsigned_abs();
    let Ok(Some(item_post)) = db.post(item_id).await else { return };
    if item_post.post_type != "nav_menu_item" {
        return;
    }
    let menu_id = match ctx.request.request("menu") {
        Some(menu) => php_int(menu).unsigned_abs(),
        None => db.terms_for_post(item_id, Some("nav_menu")).await.unwrap_or_default().first().map_or(0, |term| term.term_id),
    };
    if menu_id == 0 {
        return;
    }
    let items = ordered_items(ctx, menu_id).await;
    let order_of = |id: u64| items.iter().find(|item| item.id == id).map(|item| item.menu_order);
    let at_order = |order: i64| items.iter().find(|item| item.menu_order == order).map(|item| item.id);
    let contains = |id: i64| id > 0 && items.iter().any(|item| item.id == id as u64);
    let item = setup_item(ctx, &item_post).await;
    let parent = php_int(&item.parent);
    let reset = |value: i64| if value == item_id as i64 { 0 } else { value };
    let set_parent = |value: i64| async move {
        writer.update_post_meta(item_id, "_menu_item_menu_item_parent", &value.to_string()).await.or_log();
    };
    let set_order = |id: u64, order: i32| async move {
        content.insert_post(&PostData { id: Some(id), menu_order: Some(order), ..Default::default() }).await.or_log();
    };
    let own_order = i64::from(item_post.menu_order);
    if up {
        // If this menu item is not first.
        let Some(order) = order_of(item_id).filter(|order| *order != 0) else { return };
        let Some(previous) = at_order(order - 1) else { return };
        if parent != 0 && contains(parent) && parent as u64 == previous {
            let parent_post = db.post(parent as u64).await.ok().flatten();
            let Some(parent_post) = parent_post else { return };
            let parent_item = setup_item(ctx, &parent_post).await;
            let parent_order = order_of(parent as u64).unwrap_or(0);
            let before_parent = if parent_order != 0 { at_order(parent_order - 1) } else { None };
            let grandparent = php_int(&parent_item.parent);
            let new_parent = match before_parent {
                Some(_) if grandparent != 0 => grandparent,
                Some(before) => {
                    let possible = db.post_meta(before).await.unwrap_or_default().into_iter().find(|m| m.meta_key.as_deref() == Some("_menu_item_menu_item_parent")).and_then(|m| m.meta_value).map_or(0, |v| php_int(&v));
                    if contains(possible) { possible } else { 0 }
                }
                None => 0,
            };
            set_parent(new_parent).await;
            set_order(item_id, (own_order - 1) as i32).await;
            set_order(parent as u64, parent_post.menu_order + 1).await;
        } else if own_order == 0 || parent == 0 || !contains(parent) || previous != parent as u64 {
            // Just make it a child of the previous; keep the order.
            set_parent(reset(previous as i64)).await;
            set_order(item_id, item_post.menu_order).await;
        }
        return;
    }
    let order = order_of(item_id).unwrap_or(0);
    if let Some(next) = at_order(order + 1) {
        let Ok(Some(next_post)) = db.post(next).await else { return };
        let next_item = setup_item(ctx, &next_post).await;
        let next_parent = php_int(&next_item.parent);
        if parent != 0 && (next_parent == 0 || next_parent != parent) {
            // If not siblings of same parent, bubble menu item up but keep order.
            let parent_db_id = if contains(parent) { parent as u64 } else { 0 };
            let bubbled = match db.post(parent_db_id).await.ok().flatten() {
                Some(post) => php_int(&setup_item(ctx, &post).await.parent),
                None => 0,
            };
            set_parent(reset(bubbled)).await;
        } else {
            // Make menu item a child of its next sibling.
            set_parent(reset(next as i64)).await;
            set_order(item_id, item_post.menu_order + 1).await;
            set_order(next, next_post.menu_order - 1).await;
        }
    } else if parent != 0 && contains(parent) {
        // The item is last but still has a parent, so bubble up.
        let bubbled = db.post_meta(parent as u64).await.unwrap_or_default().into_iter().find(|m| m.meta_key.as_deref() == Some("_menu_item_menu_item_parent")).and_then(|m| m.meta_value).map_or(0, |v| php_int(&v));
        set_parent(reset(bubbled)).await;
    }
}

/// The switch at the top of `nav-menus.php`.
async fn run_action(ctx: &Ctx<'_>, selected: &mut i64, selected_title: &mut String, assigned: &mut Vec<(String, i64)>, messages: &mut Vec<String>, locations: &[(String, String)]) -> ActionResult {
    use rustpress_core::OrLog;
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    let action = request.request("action").unwrap_or("edit").to_owned();
    if action == "locations" && locations.is_empty() {
        return ActionResult::Respond(Box::new(redirect(&ctx.admin_url("nav-menus.php"))));
    }
    let writes = matches!(action.as_str(), "add-menu-item" | "move-down-menu-item" | "move-up-menu-item" | "delete-menu-item" | "delete" | "delete_menus" | "update") || (action == "locations" && !form_list(ctx, "menu-locations").is_empty());
    if !writes {
        return ActionResult::Continue;
    }
    let Some(writer) = ctx.env.writer else { return ActionResult::Respond(Box::new(super::common::read_only(ctx))) };
    let content = Content { site: ctx.site(), db, writer, user: &ctx.user, now: ctx.env.now };
    let form_locations = |current: &[(String, i64)]| {
        let mut merged = current.to_vec();
        for (key, value) in form_list(ctx, "menu-locations") {
            let value = php_int(&value).abs();
            match merged.iter_mut().find(|(known, _)| *known == key) {
                Some((_, id)) => *id = value,
                None => merged.push((key, value)),
            }
        }
        merged
    };
    match action.as_str() {
        "add-menu-item" => {
            if !ctx.verify_nonce_field("add-menu_item", "menu-settings-column-nonce") {
                return expired(ctx);
            }
            if request.request("nav-menu-locations").is_some() {
                let list: Vec<(String, i64)> = form_list(ctx, "menu-locations").into_iter().map(|(key, value)| (key, php_int(&value).abs())).collect();
                content.set_nav_menu_locations(&list).await.or_log();
            } else {
                save_menu_items(&content, (*selected).unsigned_abs(), posted_items(ctx)).await;
            }
        }
        "move-down-menu-item" | "move-up-menu-item" => {
            if !ctx.verify_nonce("move-menu_item") {
                return expired(ctx);
            }
            move_item(ctx, &content, action == "move-up-menu-item").await;
        }
        "delete-menu-item" => {
            let item_id = request.request("menu-item").map_or(0, php_int).unsigned_abs();
            if !ctx.verify_nonce(&format!("delete-menu_item_{item_id}")) {
                return expired(ctx);
            }
            let item = db.post(item_id).await.ok().flatten().filter(|post| post.post_type == "nav_menu_item");
            if item.is_some() && content.delete_post(item_id, true).await.unwrap_or(false) {
                messages.push(message(&tr("The menu item has been successfully deleted."), "updated"));
            }
        }
        "delete" => {
            if !ctx.verify_nonce(&format!("delete-nav_menu-{selected}")) {
                return expired(ctx);
            }
            let exists = db.term_by_id(Some("nav_menu"), (*selected).unsigned_abs()).await.ok().flatten().is_some();
            if exists {
                content.delete_nav_menu((*selected).unsigned_abs(), assigned).await.or_log();
                *assigned = menu_locations(ctx).await;
                messages.push(message(&tr("The menu has been successfully deleted."), "updated"));
            } else {
                *selected = 0;
            }
        }
        "delete_menus" => {
            if !ctx.verify_nonce("nav_menus_bulk_actions") {
                return expired(ctx);
            }
            for (_, id) in form_list(ctx, "delete_menus") {
                let id = php_int(&id).unsigned_abs();
                if db.term_by_id(Some("nav_menu"), id).await.ok().flatten().is_some() {
                    content.delete_nav_menu(id, assigned).await.or_log();
                    *assigned = menu_locations(ctx).await;
                }
            }
            messages.push(message(&tr("Selected menus have been successfully deleted."), "updated"));
        }
        "update" => {
            if !ctx.verify_nonce_field("update-nav_menu", "update-nav-menu-nonce") {
                return expired(ctx);
            }
            let new_locations = form_list(ctx, "menu-locations");
            let merged = form_locations(assigned);
            let posted_name = rustpress_core::formatting::php_trim(&esc_html(request.post_field("menu-name").unwrap_or_default())).to_owned();
            if *selected == 0 {
                if posted_name.is_empty() {
                    messages.push(message(&tr("Please enter a valid menu name."), "error"));
                    return ActionResult::Continue;
                }
                match content.update_nav_menu_object(0, &posted_name).await {
                    Err(error) => messages.push(message(&error, "error")),
                    Ok(id) => {
                        let name = db.term_by_id(Some("nav_menu"), id).await.ok().flatten().map_or_else(String::new, |menu| menu.name);
                        *selected = id as i64;
                        *selected_title = name.clone();
                        if request.request("menu-item").is_some() {
                            save_menu_items(&content, id, posted_items(ctx)).await;
                        }
                        if request.request("zero-menu-state").is_some() || request.post_field("auto-add-pages").is_some_and(|v| !v.is_empty() && v != "0") {
                            update_menu_items(ctx, &content, id, &name).await;
                        }
                        let mut list = assigned.clone();
                        if request.request("zero-menu-state").is_some() {
                            if let Some(first) = list.first_mut() {
                                first.1 = id as i64;
                            }
                            content.set_nav_menu_locations(&list).await.or_log();
                        } else if !new_locations.is_empty() {
                            for (location, _) in &new_locations {
                                match list.iter_mut().find(|(known, _)| known == location) {
                                    Some((_, menu)) => *menu = id as i64,
                                    None => list.push((location.clone(), id as i64)),
                                }
                            }
                            content.set_nav_menu_locations(&list).await.or_log();
                        }
                        if let Some(location) = request.request("use-location") {
                            if locations.iter().any(|(known, _)| known == location) {
                                match list.iter_mut().find(|(known, _)| known == location) {
                                    Some((_, menu)) => *menu = id as i64,
                                    None => list.push((location.to_owned(), id as i64)),
                                }
                            }
                            content.set_nav_menu_locations(&list).await.or_log();
                        }
                        return ActionResult::Respond(Box::new(redirect(&ctx.admin_url(&format!("nav-menus.php?menu={id}")))));
                    }
                }
                return ActionResult::Continue;
            }
            // Remove menu locations that have been unchecked.
            let mut list = merged;
            for (location, _) in locations {
                let posted = new_locations.iter().any(|(key, value)| key == location && !value.is_empty() && value != "0");
                if !posted && list.iter().any(|(key, id)| key == location && *id == *selected) {
                    list.retain(|(key, _)| key != location);
                }
            }
            content.set_nav_menu_locations(&list).await.or_log();
            *assigned = list;
            let current = db.term_by_id(Some("nav_menu"), (*selected).unsigned_abs()).await.ok().flatten();
            let mut name = posted_name;
            if name.is_empty() {
                messages.push(message(&tr("Please enter a valid menu name."), "error"));
                name = current.as_ref().map_or_else(String::new, |menu| menu.name.clone());
            }
            match content.update_nav_menu_object((*selected).unsigned_abs(), &name).await {
                Err(error) => messages.push(message(&error, "error")),
                Ok(id) => {
                    *selected_title = db.term_by_id(Some("nav_menu"), id).await.ok().flatten().map_or_else(String::new, |menu| menu.name);
                    let updates = update_menu_items(ctx, &content, id, selected_title).await;
                    messages.extend(updates);
                }
            }
        }
        _ => {
            if !ctx.verify_nonce("save-menu-locations") {
                return expired(ctx);
            }
            let list = form_locations(assigned);
            content.set_nav_menu_locations(&list).await.or_log();
            *assigned = list;
            messages.push(message(&tr("Menu locations updated."), "updated"));
        }
    }
    ActionResult::Continue
}

/// `nav-menus.php`.
pub async fn nav_menus(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    // `current_theme_supports( 'menus' )`: Twenty Seventeen registers locations, a block theme has this screen only
    // when it asks for it.
    if ctx.env.block_theme {
        return die(ctx.env, 500, "WordPress &rsaquo; Error", &tr("Your theme does not support navigation menus or widgets."));
    }
    if !ctx.can("edit_theme_options") {
        return super::common::forbidden_heading(ctx, "Sorry, you are not allowed to edit theme options on this site.");
    }
    let locations = registered_locations(ctx);
    let mut selected = request.request("menu").map_or(0, php_int);
    let mut assigned = menu_locations(ctx).await;
    let mut messages: Vec<String> = Vec::new();
    let mut selected_title = String::new();
    match run_action(ctx, &mut selected, &mut selected_title, &mut assigned, &mut messages, &locations).await {
        ActionResult::Continue => {}
        ActionResult::Respond(response) => return *response,
    }
    let menus = db.terms("nav_menu").await.unwrap_or_default();
    let menu_count = menus.len();
    let add_new_screen = request.get("menu").is_some_and(|m| php_int(m) == 0);
    let locations_screen = request.get("action") == Some("locations");
    let published_pages = db.count_posts(&PostQuery { post_types: vec!["page".into()], statuses: vec!["publish".into()], ..PostQuery::default() }).await.unwrap_or(0);
    let one_location_no_menus = locations.len() == 1 && !add_new_screen && menus.is_empty() && published_pages > 0;
    if menu_count == 0 && !add_new_screen && !one_location_no_menus {
        return redirect(&ctx.admin_url("nav-menus.php?action=edit&menu=0"));
    }
    let is_menu = |id: i64| menus.iter().any(|menu| menu.term_id as i64 == id);
    let stored_recent = super::common::user_option(ctx, "nav_menu_recently_edited").await.map_or(0, |value| php_int(&value).abs());
    let recently_edited = if stored_recent == 0 && is_menu(selected) { selected } else { stored_recent };
    if selected == 0 && request.get("menu").is_none() && is_menu(recently_edited) {
        selected = recently_edited;
    }
    if !add_new_screen && menu_count > 0 && request.get("action") == Some("delete") {
        selected = menus[0].term_id as i64;
    }
    if one_location_no_menus {
        selected = 0;
    } else if selected == 0 && !menus.is_empty() && !add_new_screen {
        selected = menus[0].term_id as i64;
    }
    if selected != recently_edited && is_menu(selected) && let Some(writer) = ctx.env.writer {
        use rustpress_core::OrLog;
        writer.update_user_meta(ctx.user.id, "nav_menu_recently_edited", &selected.to_string()).await.or_log();
    }
    let current = menus.iter().find(|menu| menu.term_id as i64 == selected);
    let title = if selected_title.is_empty() { current.map_or_else(String::new, |menu| menu.name.clone()) } else { selected_title };
    let assigned = menu_locations(ctx).await;
    let items = match current {
        Some(menu) => Some(menu_items(ctx, menu).await),
        None => None,
    };
    let (edit_markup, depth) = match &items {
        Some(list) => {
            let (markup, depth) = menu_to_edit(ctx, list);
            (Some(markup), depth)
        }
        None => (None, 0),
    };

    // `wp_nav_menu_setup()` and `wp_initial_nav_menu_meta_boxes()`.
    let post_types = box_post_types(ctx);
    let taxonomies = box_taxonomies(ctx);
    // `_wp_delete_orphaned_draft_menu_items()` on `admin_head-nav-menus.php`.
    if let Some(writer) = ctx.env.writer {
        use rustpress_core::OrLog;
        let content = Content { site: ctx.site(), db, writer, user: &ctx.user, now: ctx.env.now };
        for id in db.orphaned_menu_items(ctx.env.now - 30 * 86_400).await.unwrap_or_default() {
            content.delete_post(id, true).await.or_log();
        }
    }
    let mut hidden_columns = super::common::user_option(ctx, "managenav-menuscolumnshidden").await;
    if hidden_columns.is_none() {
        let defaults = serialized_list(&["link-target", "css-classes", "xfn", "description", "title-attribute"]);
        if let Some(writer) = ctx.env.writer {
            use rustpress_core::OrLog;
            writer.update_user_meta(ctx.user.id, "managenav-menuscolumnshidden", &defaults).await.or_log();
        }
        hidden_columns = Some(defaults);
    }
    let hidden_columns: Vec<String> = hidden_columns.and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok()).map_or_else(Vec::new, |value| value.as_array().unwrap_or_default().iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect());
    let mut box_ids: Vec<String> = Vec::new();
    box_ids.push("add-post-type-page".to_owned());
    box_ids.extend(post_types.iter().filter(|t| t.name != "page").map(|t| format!("add-post-type-{}", t.name)));
    box_ids.push("add-custom-links".to_owned());
    box_ids.extend(taxonomies.iter().map(|t| format!("add-{}", t.name)));
    let hidden_boxes = match super::common::user_option(ctx, "metaboxhidden_nav-menus").await {
        Some(raw) => rustpress_phpser::unserialize(raw.as_bytes()).ok().map_or_else(Vec::new, |value| value.as_array().unwrap_or_default().iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect()),
        None => {
            let initial = ["add-post-type-page", "add-post-type-post", "add-custom-links", "add-category"];
            let hidden: Vec<String> = box_ids.iter().filter(|id| !initial.contains(&id.as_str())).cloned().collect();
            if let Some(writer) = ctx.env.writer {
                use rustpress_core::OrLog;
                writer.update_user_meta(ctx.user.id, "metaboxhidden_nav-menus", &serialized_list(&hidden.iter().map(String::as_str).collect::<Vec<_>>())).await.or_log();
            }
            hidden
        }
    };

    // The boxes, in the order they print, for `$_nav_menu_placeholder`.
    let mut boxes = Boxes { ctx, selected, placeholder: 0, one_location_no_menus, ajax: false };
    let mut sections: Vec<(String, String, String)> = Vec::new();
    if !locations_screen {
        if let Some(page) = post_types.iter().find(|t| t.name == "page") {
            sections.push(("add-post-type-page".to_owned(), page.title.clone(), post_type_box(&mut boxes, page).await));
        }
        for post_type in post_types.iter().filter(|t| t.name != "page") {
            sections.push((format!("add-post-type-{}", post_type.name), post_type.title.clone(), post_type_box(&mut boxes, post_type).await));
        }
        sections.push(("add-custom-links".to_owned(), tr("Custom Links"), boxes.custom_links()));
        for taxonomy in &taxonomies {
            sections.push((format!("add-{}", taxonomy.name), taxonomy.title.clone(), taxonomy_box(&mut boxes, taxonomy).await));
        }
    }
    let view = View {
        menus: &menus,
        selected,
        title,
        add_new_screen,
        one_location_no_menus,
        locations: &locations,
        assigned: &assigned,
        items: items.as_ref().map(Vec::len),
        edit_markup,
        side: accordion(&sections, &hidden_boxes),
        auto_add: auto_add_list(ctx).await,
        messages,
    };

    // The page.
    let mut live_preview = String::new();
    if ctx.can("edit_theme_options") {
        let focus = if locations_screen { ("autofocus%5Bsection%5D", "menu_locations") } else { ("autofocus%5Bpanel%5D", "nav_menus") };
        let mut back = request.request_uri.clone();
        for arg in crate::chrome::REMOVABLE_QUERY_ARGS {
            back = remove_query_arg(&back, arg);
        }
        let url = format!("{}?{}={}&return={}", ctx.admin_url("customize.php"), focus.0, focus.1, rustpress_core::canonical::php_urlencode(&back));
        live_preview = format!(" <a class=\"page-title-action hide-if-no-customize\" href=\"{}\">{}</a>", esc_url(&url), tr("Manage with Live Preview"));
    }
    let edit_tab_active = !request.get("action").is_some_and(|a| a == "locations");
    let mut h = format!(
        "<div class=\"wrap\">\n\t<h1 class=\"wp-heading-inline\">{}</h1>\n\t{live_preview}\n\t<hr class=\"wp-header-end\">\n\n\t<nav class=\"nav-tab-wrapper wp-clearfix\" aria-label=\"{}\">\n\t\t<a href=\"{}\" class=\"nav-tab{}\"{}>{}</a>\n\t\t",
        esc_html(&tr("Menus")),
        esc_attr(&tr("Secondary menu")),
        esc_url(&ctx.admin_url("nav-menus.php")),
        if edit_tab_active { " nav-tab-active" } else { "" },
        if edit_tab_active { " aria-current=\"page\"" } else { "" },
        esc_html(&tr("Edit Menus")),
    );
    if !locations.is_empty() && menu_count > 0 {
        h.push_str(&format!(
            "\t\t\t<a href=\"{}\" class=\"nav-tab{}\"{}>{}</a>\n\t\t\t",
            esc_url(&format!("{}?action=locations", ctx.admin_url("nav-menus.php"))),
            if locations_screen { " nav-tab-active" } else { "" },
            if locations_screen { " aria-current=\"page\"" } else { "" },
            esc_html(&tr("Manage Locations")),
        ));
    }
    h.push_str("\t</nav>\n\t");
    for message in &view.messages {
        h.push_str(message);
    }
    h.push('\t');
    if locations_screen {
        h.push_str(&locations_view(ctx, &locations, &assigned, &menus, selected));
        h.push('\t');
    } else {
        h.push_str(&manage_menus(ctx, &view));
        h.push_str(&editor(ctx, &view));
    }
    h.push_str("</div><!-- /.wrap-->\n");

    let mut assets = Assets { scripts: vec!["nav-menu".to_owned()], post_header_scripts: vec!["accordion".to_owned()], ..Default::default() };
    assets.localize.push(("nav-menu".to_owned(), "menus".to_owned(), l10n(ctx, one_location_no_menus)));
    let options = if locations_screen {
        None
    } else {
        let columns: Vec<(String, String, bool)> = [("link-target", "Link Target"), ("title-attribute", "Title Attribute"), ("css-classes", "CSS Classes"), ("xfn", "Link Relationship (XFN)"), ("description", "Description")]
            .iter()
            .map(|(key, label)| ((*key).to_owned(), tr(label), hidden_columns.iter().any(|hidden| hidden == key)))
            .collect();
        let box_prefs: Vec<(String, String, bool)> = sections.iter().map(|(id, title, _)| (id.clone(), title.clone(), hidden_boxes.contains(id))).collect();
        Some(ScreenOptions { columns, columns_title: Some(tr("Show advanced menu properties")), meta_boxes: Some(box_prefs), ..Default::default() })
    };
    let screen = Screen {
        file: "nav-menus.php".into(),
        parent_file: "themes.php".into(),
        submenu_file: Some("nav-menus.php".into()),
        title: tr("Menus"),
        help: Some(help(ctx, locations_screen)),
        options,
        early_classes: vec![format!("menu-max-depth-{depth}")],
        content_inline: true,
        ..Default::default()
    };
    render(ctx, &screen, &h, &assets).await
}
