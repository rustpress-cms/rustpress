//! `WP_Customize_Nav_Menus`: the panel of the menus with a section for each menu and its items, the menu locations, the
//! data of `customize-nav-menus` and the list of what can be added (`available_items_template()`).

use rustpress_core::formatting::{esc_attr, esc_html, html_entity_decode};
use rustpress_core::i18n::sprintf;
use rustpress_db::{Order, OrderBy, PostQuery, Term};
use serde_json::{Map, Value, json};

use super::State;
use super::manager::{Control, Manager, Panel, Section, Setting};
use crate::screens::block_editor::inline_json;
use crate::screens::nav_menus as admin;

/// `wp_get_nav_menus()`.
pub(super) async fn menus(state: &State<'_>) -> Vec<Term> {
    state.ctx.env.db.terms("nav_menu").await.unwrap_or_default()
}

/// A menu of `wp_get_nav_menus()`: stored, or as the preview makes it (`term` is `None` then, its `count` is 0).
struct MenuRef {
    term_id: i64,
    name: String,
    description: String,
    parent: i64,
    term: Option<Term>,
}

/// `WP_Customize_Nav_Menu_Setting::filter_wp_get_nav_menus()` of each previewed menu, in the order of the values.
fn previewed_menus(state: &State<'_>, menus: &[Term]) -> Vec<MenuRef> {
    let mut list: Vec<MenuRef> = menus.iter().map(|t| MenuRef { term_id: t.term_id as i64, name: t.name.clone(), description: t.description.clone(), parent: t.parent as i64, term: Some(t.clone()) }).collect();
    let Some(preview) = state.previewed else { return list };
    for (term_id, value) in &preview.nav_menus {
        let index = list.iter().position(|m| m.term_id == *term_id);
        let Some(value) = value else {
            if let Some(index) = index {
                list.remove(index);
            }
            continue;
        };
        let text = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
        let menu = MenuRef { term_id: *term_id, name: text("name"), description: text("description"), parent: value.get("parent").and_then(Value::as_i64).unwrap_or(0), term: None };
        match index {
            Some(index) => list[index] = menu,
            // `array_splice( $menus, -1, 0, … )`: before the last menu.
            None => {
                let at = list.len().saturating_sub(1);
                list.insert(at, menu);
            }
        }
        // `wp_list_sort( $menus, array( 'name' => 'ASC' ) )`.
        list.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    }
    list
}

/// `wp_html_excerpt( $text, $count, $more )`.
fn html_excerpt(text: &str, count: usize, more: &str) -> String {
    let plain = rustpress_core::formatting::wp_strip_all_tags(text);
    let chars: Vec<char> = plain.chars().collect();
    if chars.len() <= count {
        return plain;
    }
    let mut cut: String = chars[..count].iter().collect();
    // A character reference cut in two is dropped.
    if let Some(at) = cut.rfind('&')
        && !cut[at..].contains(';')
    {
        cut.truncate(at);
    }
    cut + more
}

/// An item of a menu in the Customizer: stored, or one the preview changes or adds (its setting's value is the
/// previewed one).
enum Entry {
    Stored(Box<admin::EditItem>),
    Previewed { id: i64, position: i64, title: String },
}

impl Entry {
    fn id(&self) -> i64 {
        match self {
            Entry::Stored(item) => item.id as i64,
            Entry::Previewed { id, .. } => *id,
        }
    }

    fn position(&self) -> i64 {
        match self {
            Entry::Stored(item) => item.menu_order,
            Entry::Previewed { position, .. } => *position,
        }
    }

    fn set_position(&mut self, n: i64) {
        match self {
            Entry::Stored(item) => item.menu_order = n,
            Entry::Previewed { position, .. } => *position = n,
        }
    }
}

/// `wp_get_nav_menu_items()` in the preview: the stored items with the previewed ones
/// (`filter_wp_get_nav_menu_items()`, `sort_wp_get_nav_menu_items()`).
async fn previewed_items(state: &State<'_>, menu: &MenuRef) -> Vec<Entry> {
    let mut items: Vec<Entry> = match &menu.term {
        Some(term) => menu_items(state, term).await.into_iter().map(|item| Entry::Stored(Box::new(item))).collect(),
        None => Vec::new(),
    };
    let Some(preview) = state.previewed.filter(|p| !p.nav_menu_items.is_empty()) else { return items };
    let db = state.ctx.env.db;
    for (post_id, value) in &preview.nav_menu_items {
        let original = if *post_id > 0 {
            match db.post(*post_id as u64).await.ok().flatten() {
                Some(post) if post.post_type == "nav_menu_item" => db.terms_for_post(post.id, Some("nav_menu")).await.unwrap_or_default().first().map_or(0, |t| t.term_id as i64),
                _ => 0,
            }
        } else {
            0
        };
        let current = value.as_ref().and_then(|v| v.get("nav_menu_term_id")).and_then(Value::as_i64);
        if menu.term_id != original && Some(menu.term_id) != current {
            continue;
        }
        let invalid = value.as_ref().is_some_and(|v| v.get("_invalid") == Some(&Value::Bool(true)));
        if value.is_none() || invalid || (original == menu.term_id && current != Some(original)) {
            items.retain(|item| item.id() != *post_id);
            continue;
        }
        let Some(value) = value else { continue };
        let entry = Entry::Previewed { id: *post_id, position: value.get("position").and_then(Value::as_i64).unwrap_or(0), title: value.get("title").and_then(Value::as_str).unwrap_or_default().to_owned() };
        match items.iter_mut().find(|item| item.id() == *post_id) {
            Some(slot) => *slot = entry,
            None => items.push(entry),
        }
    }
    items.sort_by_key(Entry::position);
    for (n, item) in items.iter_mut().enumerate() {
        item.set_position(n as i64 + 1);
    }
    items
}

/// The items of a menu as `wp_get_nav_menu_items()` gives them: published, numbered by `menu_order`.
async fn menu_items(state: &State<'_>, menu: &Term) -> Vec<admin::EditItem> {
    let ctx = state.items_ctx.unwrap_or(state.ctx);
    if menu.count <= 0 {
        return Vec::new();
    }
    let query = PostQuery { post_types: vec!["nav_menu_item".into()], statuses: vec!["publish".into()], tax_groups: vec![vec![menu.term_taxonomy_id]], order_by: OrderBy::MenuOrder, order: Order::Asc, ..PostQuery::default() };
    let mut items = Vec::new();
    for post in ctx.env.db.posts(&query).await.unwrap_or_default() {
        let mut item = admin::setup_item(ctx, &post).await;
        if post.post_title.is_empty() {
            item.title = String::new();
        }
        // `get_post_states()` exists only in the administration: on the website the label is the type's name.
        if state.preview && item.kind == "post_type" && !item.invalid {
            item.type_label = admin::box_post_types(ctx).into_iter().find(|t| t.name == item.object).map_or_else(|| item.object.clone(), |t| admin::singular_of(ctx, &t.name));
        }
        items.push(item);
    }
    items.sort_by_key(|item| item.menu_order);
    for (index, item) in items.iter_mut().enumerate() {
        item.menu_order = index as i64 + 1;
    }
    items
}

/// `php_int()` of a numeric meta value.
fn int(text: &str) -> i64 {
    let digits = text.trim();
    let (negative, digits) = match digits.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, digits.strip_prefix('+').unwrap_or(digits)),
    };
    let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
    let number = digits[..end].parse::<i64>().unwrap_or(0);
    if negative { -number } else { number }
}

/// `WP_Customize_Nav_Menu_Item_Setting::value()` with `js_value()`.
fn item_value(item: &admin::EditItem, menu_id: i64) -> Value {
    let original = match item.kind.as_str() {
        "post_type" | "taxonomy" => item.original_title.clone().unwrap_or_default(),
        _ => String::new(),
    };
    let mut value = Map::new();
    value.insert("menu_item_parent".into(), json!(int(&item.parent)));
    value.insert("object_id".into(), json!(int(&item.object_id)));
    value.insert("object".into(), json!(item.object));
    value.insert("type".into(), json!(item.kind));
    value.insert("type_label".into(), json!(item.type_label));
    // A link of an object gets its address first; any other item its title first.
    if matches!(item.kind.as_str(), "post_type" | "taxonomy") {
        value.insert("url".into(), json!(item.url));
        value.insert("title".into(), json!(item.title));
    } else {
        value.insert("title".into(), json!(item.title));
        value.insert("url".into(), json!(item.url));
    }
    let rest = json!({
        "target": item.target,
        "attr_title": item.post_excerpt,
        "description": item.description,
        "classes": item.classes.join(" "),
        "xfn": item.xfn,
        "nav_menu_term_id": menu_id,
        "position": item.menu_order,
        "status": item.post_status,
        "_invalid": item.invalid,
        "original_title": html_entity_decode(&original),
    });
    if let Value::Object(rest) = rest {
        value.extend(rest);
    }
    Value::Object(value)
}

/// `WP_Customize_Nav_Menus::customize_register()`.
pub(super) async fn register(state: &State<'_>, m: &mut Manager, menus: &[Term], locations: &[(String, String)], widgets: bool) {
    let ctx = state.ctx;
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    let mut description = format!("<p>{}</p>", tr("This panel is used for managing navigation menus for content you have already published on your site. You can create menus and add items for existing content such as pages, posts, categories, tags, formats, or custom links."));
    if widgets {
        description.push_str(&format!("<p>{}</p>", sprintf(&tr("Menus can be displayed in locations defined by your theme or in <a href=\"%s\">widget areas</a> by adding a &#8220;Navigation Menu&#8221; widget."), &["javascript:wp.customize.panel( 'widgets' ).focus();"])));
    } else {
        description.push_str(&format!("<p>{}</p>", tr("Menus can be displayed in locations defined by your theme.")));
    }
    // The menus and items of the values are previewed first, so that their sections and controls are added.
    if let Some(preview) = state.previewed {
        preview_values(state, m, preview).await;
    }
    let menus = previewed_menus(state, menus);
    let instance = m.next_panel();
    // `_add_default_theme_supports()`: a block theme without menus and widgets has no panel for menus.
    let active = !(ctx.env.block_theme && locations.is_empty() && !widgets);
    m.add_panel(Panel { id: "nav_menus".into(), title: tr("Menus"), description, priority: 100, kind: "nav_menus", allowed: true, active, instance, auto_expand_sole_section: false });

    // Menu locations.
    let count = locations.len() as i64;
    let mut description = if count == 1 {
        format!("<p>{}</p>", tr("Your theme can display menus in one location. Select which menu you would like to use."))
    } else {
        format!("<p>{}</p>", sprintf(&site.trn("Your theme can display menus in %s location. Select which menu you would like to use.", "Your theme can display menus in %s locations. Select which menu appears in each location.", count), &[&count.to_string()]))
    };
    if widgets {
        description.push_str(&format!("<p>{}</p>", sprintf(&tr("If your theme has widget areas, you can also add menus there. Visit the <a href=\"%s\">Widgets panel</a> and add a &#8220;Navigation Menu widget&#8221; to display a menu in a sidebar or footer."), &["javascript:wp.customize.panel( 'widgets' ).focus();"])));
    }
    let instance = m.next_section();
    m.add_section(Section {
        id: "menu_locations".into(),
        title: if count == 1 { site.trx("View Location", "menu locations") } else { site.trx("View All Locations", "menu locations") },
        description,
        priority: 30,
        panel: "nav_menus".into(),
        kind: "default",
        description_hidden: false,
        allowed: true,
        active: true,
        instance,
        extra: Vec::new(),
    });
    let mut choices: Vec<(String, String)> = vec![("0".into(), tr("&mdash; Select &mdash;"))];
    for menu in &menus {
        choices.push((menu.term_id.to_string(), html_excerpt(&menu.name, 40, "&hellip;")));
    }
    let assigned = state.mods.get("nav_menu_locations");
    for (location, label) in locations {
        let id = format!("nav_menu_locations[{location}]");
        let value = assigned.and_then(|a| a.get(location)).map_or(json!(0), super::register::php_json);
        let mut setting = Setting::new(&id, "theme_mod", "postMessage", value.clone());
        setting.raw = value.clone();
        m.add_setting(setting);
        let mut control = Control::new(&id, "nav_menu_location", "menu_locations", m.next_control());
        control.label = label.clone();
        control.extra = vec![("locationId", json!(location))];
        control.content = location_content(state, &id, location, label, &choices, &value);
        m.add_control(control);
    }

    // A section for each menu, a setting and control for each item.
    for menu in &menus {
        let section_id = format!("nav_menu[{}]", menu.term_id);
        let instance = m.next_section();
        m.add_section(Section {
            id: section_id.clone(),
            title: html_entity_decode(&menu.name),
            description: String::new(),
            priority: 10,
            panel: "nav_menus".into(),
            kind: "nav_menu",
            description_hidden: false,
            allowed: true,
            active: true,
            instance,
            extra: vec![("menu_id", json!(menu.term_id))],
        });
        let auto_add = site.options.value("nav_menu_options").and_then(|options| options.get("auto_add").and_then(|list| list.as_array().map(|items| items.iter().any(|(_, id)| id.as_i64() == Some(menu.term_id))))).unwrap_or(false);
        // A previewed menu keeps the value of its setting (`preview()` runs again on `wp_loaded`).
        if m.setting_mut(&section_id).is_none_or(|_| previewed_menus_has(state, menu.term_id).is_none()) {
            m.add_setting(Setting::new(&section_id, "nav_menu", "postMessage", json!({"name": menu.name, "description": menu.description, "parent": menu.parent, "auto_add": auto_add})));
        }
        for (i, entry) in previewed_items(state, menu).await.into_iter().enumerate() {
            let id = format!("nav_menu_item[{}]", entry.id());
            let mut control = Control::new(&id, "nav_menu_item", &section_id, m.next_control());
            match &entry {
                Entry::Stored(item) => {
                    m.add_setting(Setting::new(&id, "nav_menu_item", "postMessage", item_value(item, menu.term_id)));
                    // `$item->title`: the original title for an item without one.
                    control.label = if item.title.is_empty() { item.original_title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| sprintf(&tr("#%d (no title)"), &[&item.object_id])) } else { item.title.clone() };
                }
                Entry::Previewed { title, .. } => control.label = title.clone(),
            }
            control.priority = 10 + i as i64;
            control.json_extra = vec![("menu_item_id", json!(entry.id()))];
            m.add_control(control);
        }
    }
    let instance = m.next_section();
    m.add_section(Section { id: "add_menu".into(), title: tr("New Menu"), description: String::new(), priority: 20, panel: "nav_menus".into(), kind: "new_menu", description_hidden: false, allowed: true, active: true, instance, extra: Vec::new() });
    let created = site.options.value("nav_menus_created_posts").map_or(json!([]), |value| super::register::php_json(&value));
    m.add_setting(Setting::new("nav_menus_created_posts", "option", "postMessage", created));
}

/// The previewed value of a menu, if the preview has one.
fn previewed_menus_has(state: &State<'_>, term_id: i64) -> Option<()> {
    state.previewed?.nav_menus.iter().any(|(id, _)| *id == term_id).then_some(())
}

/// `WP_Customize_Nav_Menu_Item_Setting::get_type_label()`.
fn type_label(state: &State<'_>, kind: &str, object: &str) -> String {
    let ctx = state.ctx;
    let site = ctx.site();
    match kind {
        "post_type" if matches!(object, "post" | "page") || site.types.post_type_exists(object) => admin::singular_of(ctx, object),
        "taxonomy" if matches!(object, "category" | "post_tag" | "post_format") || site.types.taxonomy(object).is_some() => admin::singular_of_taxonomy(ctx, object),
        "post_type" | "taxonomy" => object.to_owned(),
        "post_type_archive" => ctx.tr("Post Type Archive"),
        _ => ctx.tr("Custom Link"),
    }
}

/// `WP_Customize_Nav_Menu_Item_Setting::get_original_title()`.
async fn original_title(state: &State<'_>, kind: &str, object: &str, object_id: i64) -> String {
    let ctx = state.ctx;
    let site = ctx.site();
    if object_id == 0 {
        return String::new();
    }
    match kind {
        "post_type" => match ctx.env.db.post(object_id.unsigned_abs()).await.ok().flatten() {
            Some(post) => {
                let title = rustpress_blocks::content::the_title_in_admin(site, &post);
                if title.is_empty() { sprintf(&ctx.tr("#%d (no title)"), &[&post.id.to_string()]) } else { title }
            }
            None => String::new(),
        },
        "taxonomy" => ctx.env.db.term_by_id(Some(object), object_id.unsigned_abs()).await.ok().flatten().map(|term| term.name).unwrap_or_default(),
        _ => String::new(),
    }
}

/// The previewed menus and menu items: `value()` and `js_value()` of their settings in the preview, and the dynamic
/// settings of those that are not registered yet (`register_dynamic_settings()`, in the order of the values).
pub(super) async fn preview_values(state: &State<'_>, m: &mut Manager, preview: &rustpress_core::customize::Preview) {
    for id in &preview.previewed {
        let value = if let Some((_, menu)) = preview.nav_menus.iter().find(|(term_id, _)| *id == format!("nav_menu[{term_id}]")) {
            (menu.clone().unwrap_or(json!(false)), "nav_menu")
        } else if let Some((_, item)) = preview.nav_menu_items.iter().find(|(post_id, _)| *id == format!("nav_menu_item[{post_id}]")) {
            let value = match item {
                Some(Value::Object(item)) => {
                    let mut item = item.clone();
                    let text = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
                    let (kind, object) = (text("type"), text("object"));
                    let object_id = item.get("object_id").and_then(Value::as_i64).unwrap_or(0);
                    item.insert("type_label".into(), json!(type_label(state, &kind, &object)));
                    item.insert("original_title".into(), json!(html_entity_decode(&original_title(state, &kind, &object, object_id).await)));
                    Value::Object(item)
                }
                _ => json!(false),
            };
            (value, "nav_menu_item")
        } else if id.starts_with("nav_menu[") || id.starts_with("nav_menu_item[") {
            // A value that does not sanitize is not previewed: the setting is there, with the value it has without.
            if m.setting_mut(id).is_none() {
                m.add_setting(Setting::new(id, if id.starts_with("nav_menu[") { "nav_menu" } else { "nav_menu_item" }, "postMessage", json!(false)));
            }
            continue;
        } else {
            continue;
        };
        match m.setting_mut(id) {
            Some(setting) => {
                setting.raw = value.0.clone();
                setting.value = value.0;
            }
            None => m.add_setting(Setting::new(id, value.1, "postMessage", value.0)),
        }
    }
}

/// `WP_Customize_Nav_Menu_Location_Control::render_content()`.
fn location_content(state: &State<'_>, setting_id: &str, location: &str, label: &str, choices: &[(String, String)], value: &Value) -> String {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let current = match value {
        Value::Number(n) => n.to_string(),
        Value::String(text) => text.clone(),
        Value::Bool(true) => "1".into(),
        _ => String::new(),
    };
    let set = !matches!(current.as_str(), "" | "0");
    let mut options = String::new();
    for (choice, text) in choices {
        let selected = if *choice == current { " selected='selected'" } else { "" };
        options.push_str(&format!("<option value=\"{}\"{selected}>{}</option>", esc_attr(choice), esc_html(text)));
    }
    let title = if label.is_empty() { String::new() } else { format!("\t\t\t<span class=\"customize-control-title\">{}</span>\n\t\t\t", esc_html(label)) };
    format!(
        "\t\t<label>\n\t\t\t{title}\n\t\t\t\n\t\t\t<select {}>\n\t\t\t\t{options}\t\t\t</select>\n\t\t</label>\n\t\t<button type=\"button\" class=\"button-link create-menu{}\" data-location-id=\"{}\" aria-label=\"{}\">{}</button>\n\t\t<button type=\"button\" class=\"button-link edit-menu{}\" aria-label=\"{}\">{}</button>\n\t\t",
        super::controls::link(setting_id),
        if set { " hidden" } else { "" },
        esc_attr(location),
        esc_attr(&tr("Create a menu for this location")),
        tr("+ Create New Menu"),
        if set { "" } else { " hidden" },
        esc_attr(&tr("Edit selected menu")),
        tr("Edit Menu")
    )
}

/// `available_item_types()`.
fn item_types(state: &State<'_>) -> Vec<(String, String, &'static str, String)> {
    item_types_with_hierarchy(state).into_iter().map(|(title, label, kind, object, _)| (title, label, kind, object)).collect()
}

/// The item types, each post type with whether it is hierarchical.
fn item_types_with_hierarchy(state: &State<'_>) -> Vec<(String, String, &'static str, String, bool)> {
    let ctx = state.ctx;
    let mut out = Vec::new();
    for post_type in admin::box_post_types(ctx) {
        let singular = admin::singular_of(ctx, &post_type.name);
        out.push((post_type.title, singular, "post_type", post_type.name, post_type.hierarchical));
    }
    for taxonomy in admin::box_taxonomies(ctx) {
        let singular = admin::singular_of_taxonomy(ctx, &taxonomy.name);
        out.push((taxonomy.title, singular, "taxonomy", taxonomy.name, false));
    }
    out
}

/// The data of `customize-nav-menus` and the strings of `nav-menu`.
pub(super) fn scripts_data(state: &State<'_>, menus: &[Term], locations: &[(String, String)], panel_title: &str) -> (String, Value) {
    let ctx = state.ctx;
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    let trx = |text: &str, context: &str| site.trx(text, context);
    let count = locations.len() as i64;
    let all_menus: Vec<Value> = menus
        .iter()
        .map(|menu| json!({"term_id": menu.term_id, "name": menu.name, "slug": menu.slug, "term_group": 0, "term_taxonomy_id": menu.term_taxonomy_id, "taxonomy": "nav_menu", "description": menu.description, "parent": menu.parent, "count": menu.count, "filter": "raw"}))
        .collect();
    let types: Vec<Value> = item_types(state).into_iter().map(|(title, label, kind, object)| json!({"title": title, "type_label": label, "type": kind, "object": object})).collect();
    let mut mapped = Map::new();
    for (location, name) in locations {
        mapped.insert(location.clone(), json!(name));
    }
    let settings = json!({
        "allMenus": all_menus,
        "itemTypes": types,
        "l10n": {
            "untitled": trx("(no label)", "missing menu item navigation label"),
            "unnamed": trx("(unnamed)", "Missing menu name."),
            "custom_label": tr("Custom Link"),
            "page_label": trx("Page", "post type singular name"),
            "menuLocation": trx("(Currently set to: %s)", "menu"),
            "locationsTitle": if count == 1 { tr("Menu Location") } else { tr("Menu Locations") },
            "locationsDescription": if count == 1 { tr("Your theme can display menus in one location.") } else { sprintf(&site.trn("Your theme can display menus in %s location.", "Your theme can display menus in %s locations.", count), &[&count.to_string()]) },
            "menuNameLabel": tr("Menu Name"),
            "newMenuNameDescription": tr("If your theme has multiple menus, giving them clear names will help you manage them."),
            "itemAdded": tr("Menu item added"),
            "itemDeleted": tr("Menu item deleted"),
            "menuAdded": tr("Menu created"),
            "menuDeleted": tr("Menu deleted"),
            "movedUp": tr("Menu item moved up"),
            "movedDown": tr("Menu item moved down"),
            "movedLeft": tr("Menu item moved out of submenu"),
            "movedRight": tr("Menu item is now a sub-item"),
            "customizingMenus": sprintf(&tr("Customizing &#9656; %s"), &[&esc_html(panel_title)]),
            "invalidTitleTpl": tr("%s (Invalid)"),
            "pendingTitleTpl": tr("%s (Pending)"),
            "itemsFound": tr("Number of items found: %d"),
            "itemsFoundMore": tr("Additional items found: %d"),
            "itemsLoadingMore": tr("Loading more results... please wait."),
            "reorderModeOn": tr("Reorder mode enabled"),
            "reorderModeOff": tr("Reorder mode closed"),
            "reorderLabelOn": esc_attr(&tr("Reorder menu items")),
            "reorderLabelOff": esc_attr(&tr("Close reorder mode")),
        },
        "settingTransport": "postMessage",
        "phpIntMax": i64::MAX,
        "defaultSettingValues": {
            "nav_menu": {"name": "", "description": "", "parent": 0, "auto_add": false},
            "nav_menu_item": {"object_id": 0, "object": "", "menu_item_parent": 0, "position": 0, "type": "custom", "title": "", "url": "", "target": "", "attr_title": "", "description": "", "classes": "", "xfn": "", "status": "publish", "nav_menu_term_id": 0, "_invalid": false},
        },
        "locationSlugMappedToName": if mapped.is_empty() { json!([]) } else { Value::Object(mapped) },
    });
    let menus_l10n = json!({
        "oneThemeLocationNoMenus": null,
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
    });
    (format!("var _wpCustomizeNavMenusSettings = {};", inline_json(&settings)), menus_l10n)
}

/// `available_items_template()`.
pub(super) fn available_items(state: &State<'_>, panel_title: &str) -> String {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let mut out = format!(
        "\t\t<div id=\"available-menu-items\" class=\"accordion-container\">\n\t\t\t<div class=\"customize-section-title\">\n\t\t\t\t<button type=\"button\" class=\"customize-section-back\" tabindex=\"-1\">\n\t\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</span>\n\t\t\t\t</button>\n\t\t\t\t<h3>\n\t\t\t\t\t<span class=\"customize-action\">\n\t\t\t\t\t\t{}\t\t\t\t\t</span>\n\t\t\t\t\t{}\t\t\t\t</h3>\n\t\t\t</div>\n\t\t\t<div id=\"available-menu-items-search\" class=\"accordion-section cannot-expand\">\n\t\t\t\t<div class=\"accordion-section-title\">\n\t\t\t\t\t<label for=\"menu-items-search\">{}</label>\n\t\t\t\t\t<input type=\"text\" id=\"menu-items-search\" aria-describedby=\"menu-items-search-desc\" />\n\t\t\t\t\t<p class=\"screen-reader-text\" id=\"menu-items-search-desc\">\n\t\t\t\t\t\t{}\t\t\t\t\t</p>\n\t\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t\t<div class=\"search-icon\" aria-hidden=\"true\"></div>\n\t\t\t\t\t<button type=\"button\" class=\"clear-results\"><span class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</span></button>\n\t\t\t\t</div>\n\t\t\t\t<ul class=\"accordion-section-content available-menu-items-list\" data-type=\"search\"></ul>\n\t\t\t</div>\n\t\t\t",
        tr("Back"),
        sprintf(&tr("Customizing &#9656; %s"), &[&esc_html(panel_title)]),
        tr("Add Menu Items"),
        tr("Search Menu Items"),
        tr("The search results will be updated as you type."),
        tr("Clear Results")
    );
    // `print_custom_links_available_menu_item()`.
    out.push_str(&format!(
        "\t\t<div id=\"new-custom-menu-item\" class=\"accordion-section\">\n\t\t\t<h4 class=\"accordion-section-title\">\n\t\t\t\t<button type=\"button\" class=\"accordion-trigger\" aria-expanded=\"false\" aria-controls=\"new-custom-menu-item-content\">\n\t\t\t\t\t{}\t\t\t\t\t<span class=\"toggle-indicator\" aria-hidden=\"true\"></span>\n\t\t\t\t</button>\n\t\t\t</h4>\n\t\t\t<div class=\"accordion-section-content customlinkdiv\" id=\"new-custom-menu-item-content\">\n\t\t\t\t<input type=\"hidden\" value=\"custom\" id=\"custom-menu-item-type\" name=\"menu-item[-1][menu-item-type]\" />\n\t\t\t\t<p id=\"menu-item-url-wrap\" class=\"wp-clearfix\">\n\t\t\t\t\t<label class=\"howto\" for=\"custom-menu-item-url\">{}</label>\n\t\t\t\t\t<input id=\"custom-menu-item-url\" name=\"menu-item[-1][menu-item-url]\" type=\"text\" class=\"code menu-item-textbox\" placeholder=\"https://\">\n\t\t\t\t\t<span id=\"custom-url-error\" class=\"error-message\" style=\"display: none;\">{}</span>\n\t\t\t\t</p>\n\t\t\t\t<p id=\"menu-item-name-wrap\" class=\"wp-clearfix\">\n\t\t\t\t\t<label class=\"howto\" for=\"custom-menu-item-name\">{}</label>\n\t\t\t\t\t<input id=\"custom-menu-item-name\" name=\"menu-item[-1][menu-item-title]\" type=\"text\" class=\"regular-text menu-item-textbox\">\n\t\t\t\t\t<span id=\"custom-name-error\" class=\"error-message\" style=\"display: none;\">{}</span>\n\t\t\t\t</p>\n\t\t\t\t<p class=\"button-controls\">\n\t\t\t\t\t<span class=\"add-to-menu\">\n\t\t\t\t\t\t<input type=\"submit\" class=\"button submit-add-to-menu right\" value=\"{}\" name=\"add-custom-menu-item\" id=\"custom-menu-item-submit\">\n\t\t\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t\t</span>\n\t\t\t\t</p>\n\t\t\t</div>\n\t\t</div>\n\t\t",
        tr("Custom Links"),
        tr("URL"),
        tr("Please provide a valid link."),
        tr("Link Text"),
        tr("The link text cannot be empty."),
        esc_attr(&tr("Add to Menu"))
    ));
    // The pages come first.
    let mut types = item_types_with_hierarchy(state);
    if let Some(at) = types.iter().position(|(_, _, kind, object, _)| *kind == "post_type" && object == "page") {
        let page = types.remove(at);
        types.insert(0, page);
    }
    for (title, type_label, kind, object, hierarchical) in types {
        let id = esc_attr(&format!("available-menu-items-{kind}-{object}"));
        let mut create = String::new();
        if kind == "post_type" {
            let can = match object.as_str() {
                "page" => ctx.can("edit_pages") && ctx.can("publish_pages"),
                _ => ctx.can("edit_posts") && ctx.can("publish_posts"),
            };
            if can {
                let add_new = match object.as_str() {
                    "page" => tr("Add Page"),
                    "post" => tr("Add Post"),
                    other => ctx.site().types.admin_post_types().into_iter().find(|t| t.name == other).and_then(|t| t.label("add_new_item")).unwrap_or_else(|| tr(if hierarchical { "Add Page" } else { "Add Post" })),
                };
                let object = esc_attr(&object);
                create = format!(
                    "\t\t\t\t\t\t\t\t\t\t\t<div class=\"new-content-item-wrapper\">\n\t\t\t\t\t\t\t<label for=\"create-item-input-{object}\">{}</label>\n\t\t\t\t\t\t\t<div class=\"new-content-item\">\n\t\t\t\t\t\t\t\t<input type=\"text\" id=\"create-item-input-{object}\" class=\"create-item-input form-required\">\n\t\t\t\t\t\t\t\t<button type=\"button\" class=\"button add-content\">{}</button>\n\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t<span id=\"create-input-{object}-error\" class=\"create-item-error error-message\" style=\"display: none;\">{}</span>\n\n\t\t\t\t\t\t</div>\n\t\t\t\t\t",
                    esc_html(&add_new),
                    tr("Add"),
                    tr("Please enter an item title")
                );
            }
            create = format!("\t\t\t\t\t{create}\t\t\t\t");
        }
        out.push_str(&format!(
            "\t\t<div id=\"{id}\" class=\"accordion-section\">\n\t\t\t<h4 class=\"accordion-section-title\">\n\t\t\t\t<button type=\"button\" class=\"accordion-trigger\" aria-expanded=\"false\" aria-controls=\"{id}-content\">\n\t\t\t\t\t{}\t\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t\t<span class=\"no-items\">{}</span>\n\t\t\t\t\t<span class=\"toggle-indicator\" aria-hidden=\"true\"></span>\n\t\t\t\t</button>\n\t\t\t</h4>\n\t\t\t<div class=\"accordion-section-content\" id=\"{id}-content\">\n\t\t\t\t{create}\t\t\t\t<ul class=\"available-menu-items-list\" data-type=\"{}\" data-object=\"{}\" data-type_label=\"{}\"></ul>\n\t\t\t</div>\n\t\t</div>\n\t\t",
            esc_html(&title),
            tr("No items"),
            esc_attr(kind),
            esc_attr(&object),
            esc_attr(&type_label)
        ));
    }
    out.push_str("\t\t</div><!-- #available-menu-items -->\n\t\t");
    out
}
