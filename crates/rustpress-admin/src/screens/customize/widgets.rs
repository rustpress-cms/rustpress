//! `WP_Customize_Widgets` with the block editor of the widget areas: a setting for each registered widget and widget
//! area, a section with the editor for each area, the data of `customize-widgets` and the forms of the widgets one can
//! add (`output_widget_control_templates()`).

use rustpress_core::formatting::{esc_attr, esc_html, esc_url, wp_strip_all_tags};
use rustpress_core::i18n::sprintf;
use rustpress_phpser::{Key, Value as Php};
use rustpress_theme::classic::widgets::RegisteredSidebar;
use serde_json::{Map, Value, json};

use super::State;
use super::manager::{Control, Manager, Panel, Section, Setting};
use crate::screens::block_editor::inline_json;

const FORMS: &str = include_str!("../../data/customize_widget_forms.html");

/// A widget of `$wp_registered_widgets`.
#[derive(Debug, Clone)]
pub(super) struct Registered {
    pub id: String,
    pub id_base: &'static str,
    /// `params[0]['number']`: `-1` for the template of a type without instances.
    pub number: i64,
}

/// What the widgets of the site are.
pub(super) struct Widgets {
    pub sidebars: Vec<RegisteredSidebar>,
    pub registered: Vec<Registered>,
    /// `wp_get_sidebars_widgets()`.
    pub sidebars_widgets: Vec<(String, Vec<String>)>,
    /// `get_option( 'widget_{id_base}' )` of each type.
    pub instances: Vec<(&'static str, Php)>,
}

/// The name, description and class name of a core widget, translated.
fn type_info(state: &State<'_>, id_base: &str) -> (String, String, &'static str) {
    let site = state.ctx.site();
    let (_, name, description, classname) = rustpress_rest::edit::WIDGET_TYPES.iter().find(|(base, ..)| *base == id_base).copied().unwrap_or(("", "", "", ""));
    let name = if id_base == "search" { site.trx(name, "Search widget") } else { site.tr(name) };
    (name, site.tr(description), classname)
}

/// The control's width and height (`$control_ops`).
fn control_size(id_base: &str) -> (u64, u64) {
    match id_base {
        "text" | "custom_html" | "block" => (400, 350),
        "rss" => (400, 200),
        _ => (250, 200),
    }
}

fn mime_type(id_base: &str) -> Option<&'static str> {
    match id_base {
        "media_audio" => Some("audio"),
        "media_image" | "media_gallery" => Some("image"),
        "media_video" => Some("video"),
        _ => None,
    }
}

/// Reads the registered widgets: each type registers its instances, or once with a generic template (`_register()`).
pub(super) async fn load(state: &State<'_>) -> Widgets {
    let ctx = state.ctx;
    let site = ctx.site();
    let mut names: Vec<String> = rustpress_rest::edit::WIDGET_TYPES.iter().map(|(base, ..)| format!("widget_{base}")).collect();
    names.push("sidebars_widgets".into());
    let rows = ctx.env.db.options_named(&names.iter().map(String::as_str).collect::<Vec<_>>()).await.unwrap_or_default();
    // The autoloaded options as the site has them (the preview changes them), the others from the database.
    let option = |name: &str| site.options.get(name).map(rustpress_core::options::maybe_unserialize).or_else(|| rows.iter().find(|(n, _)| n == name).map(|(_, value)| rustpress_core::options::maybe_unserialize(value)));
    let mut registered = Vec::new();
    let mut instances = Vec::new();
    for (id_base, ..) in rustpress_rest::edit::WIDGET_TYPES {
        let value = option(&format!("widget_{id_base}")).unwrap_or(Php::Bool(false));
        let numbers: Vec<i64> = match &value {
            Php::Array(items) => items.iter().filter_map(|(key, _)| if let Key::Int(n) = key { Some(*n) } else { None }).collect(),
            _ => Vec::new(),
        };
        if numbers.is_empty() {
            registered.push(Registered { id: format!("{id_base}-1"), id_base, number: -1 });
        }
        for n in numbers {
            registered.push(Registered { id: format!("{id_base}-{n}"), id_base, number: n });
        }
        instances.push((id_base, value));
    }
    let mut sidebars_widgets = Vec::new();
    if let Some(Php::Array(items)) = option("sidebars_widgets") {
        for (key, value) in items {
            let Key::Str(key) = key else { continue };
            let key = String::from_utf8_lossy(&key).into_owned();
            if key == "array_version" {
                continue;
            }
            let ids = match value {
                Php::Array(ids) => ids.iter().map(|(_, id)| super::register::php_text(id)).collect(),
                _ => Vec::new(),
            };
            sidebars_widgets.push((key, ids));
        }
    }
    let sidebars = rustpress_theme::classic::widgets::registered_sidebars(ctx.env.theme, site);
    Widgets { sidebars, registered, sidebars_widgets, instances }
}

impl Widgets {
    /// The instance of a widget (`$option[ $number ]`), an empty array if there is none.
    fn instance(&self, id_base: &str, number: i64) -> Php {
        self.instances.iter().find(|(base, _)| *base == id_base).and_then(|(_, value)| value.as_array()).and_then(|items| items.iter().find(|(key, _)| *key == Key::Int(number))).map_or(Php::Array(Vec::new()), |(_, value)| value.clone())
    }

    /// `next_widget_id_number()`: the pattern has no start, so it matches the end of any widget id.
    fn next_number(&self, id_base: &str) -> i64 {
        let suffix = format!("{id_base}-");
        let highest = self.registered.iter().filter_map(|w| w.id.rsplit_once('-').filter(|(base, n)| format!("{base}-").ends_with(&suffix) && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))?.1.parse::<i64>().ok()).max();
        highest.unwrap_or(1).max(1) + 1
    }
}

/// `sanitize_widget_js_instance()` of an instance.
fn js_instance(state: &State<'_>, instance: &Php) -> Value {
    let serialized = rustpress_phpser::serialize(instance);
    let title = match instance.get("title") {
        Some(Php::Str(text)) if !text.is_empty() && text != b"0" => String::from_utf8_lossy(text).into_owned(),
        _ => String::new(),
    };
    let empty = matches!(instance, Php::Array(items) if items.is_empty());
    let raw = if empty { rustpress_blocks::php_json::empty_object() } else { rustpress_core::options::php_to_json(instance) };
    use base64::Engine;
    json!({
        "encoded_serialized_instance": base64::engine::general_purpose::STANDARD.encode(&serialized),
        "title": title,
        "is_widget_customizer_js_value": true,
        "instance_hash_key": state.ctx.env.auth.salts().hash(&String::from_utf8_lossy(&serialized), "auth").unwrap_or_default(),
        "raw_instance": raw,
    })
}

/// `get_setting_type()`: the setting of a widget's instance or of a widget area.
pub(super) fn is_widget_setting(id: &str) -> bool {
    if let Some(area) = id.strip_prefix("sidebars_widgets[").and_then(|rest| rest.strip_suffix(']')) {
        return !area.is_empty();
    }
    id.strip_prefix("widget_").and_then(|rest| rest.strip_suffix(']')).and_then(|rest| rest.split_once('[')).is_some_and(|(base, number)| !base.is_empty() && !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

/// `WP_Customize_Widgets::customize_register()` with the block editor.
pub(super) fn register(state: &State<'_>, m: &mut Manager, w: &Widgets) {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    for widget in &w.registered {
        let id = format!("widget_{}[{}]", widget.id_base, widget.id.rsplit_once('-').map_or("", |(_, n)| n));
        let value = js_instance(state, &w.instance(widget.id_base, widget.number));
        m.add_setting(Setting::new(&id, "option", "postMessage", value));
    }
    let instance = m.next_panel();
    m.add_panel(Panel {
        id: "widgets".into(),
        title: tr("Widgets"),
        description: tr("Widgets are independent sections of content that can be placed into widgetized areas provided by your theme (commonly called sidebars)."),
        priority: 110,
        kind: "widgets",
        allowed: true,
        active: !w.sidebars.is_empty(),
        instance,
        auto_expand_sole_section: true,
    });
    // `array_merge()` of the inactive widgets, the registered areas and what is stored.
    let mut areas: Vec<String> = vec!["wp_inactive_widgets".into()];
    areas.extend(w.sidebars.iter().map(|s| s.id.clone()));
    for (id, _) in &w.sidebars_widgets {
        if !areas.contains(id) {
            areas.push(id.clone());
        }
    }
    for area in areas {
        let registered = w.sidebars.iter().position(|s| s.id == area);
        if registered.is_none() && area != "wp_inactive_widgets" {
            continue;
        }
        let ids = w.sidebars_widgets.iter().find(|(id, _)| *id == area).map(|(_, ids)| ids.clone()).unwrap_or_default();
        let setting_id = format!("sidebars_widgets[{area}]");
        m.add_setting(Setting::new(&setting_id, "option", "postMessage", json!(ids)));
        let Some(priority) = registered else { continue };
        let sidebar = &w.sidebars[priority];
        let section_id = format!("sidebar-widgets-{area}");
        let instance = m.next_section();
        m.add_section(Section {
            id: section_id.clone(),
            title: sidebar.name.clone(),
            description: String::new(),
            priority: priority as i64,
            panel: "widgets".into(),
            kind: "sidebar",
            description_hidden: false,
            allowed: true,
            // `is_sidebar_rendered()`: nothing was rendered.
            active: false,
            instance,
            extra: vec![("sidebarId", json!(area))],
        });
        let mut control = Control::new(&setting_id, "sidebar_block_editor", &section_id, m.next_control());
        control.label = sidebar.name.clone();
        m.add_control(control);
    }
}

/// The JSON of a registered widget, without its callback.
fn registered_json(state: &State<'_>, widget: &Registered) -> Map<String, Value> {
    let (name, description, classname) = type_info(state, widget.id_base);
    let mut out = Map::new();
    out.insert("name".into(), json!(name));
    out.insert("id".into(), json!(widget.id));
    out.insert("params".into(), json!([{"number": widget.number}]));
    out.insert("classname".into(), json!(classname));
    out.insert("customize_selective_refresh".into(), json!(true));
    out.insert("description".into(), json!(description));
    out.insert("show_instance_in_rest".into(), json!(true));
    if let Some(mime) = mime_type(widget.id_base) {
        out.insert("mime_type".into(), json!(mime));
    }
    out
}

/// `get_available_widgets()`: one widget of each type, by name (`strnatcasecmp()`).
fn available(state: &State<'_>, w: &Widgets) -> Vec<(Registered, Map<String, Value>)> {
    let mut sorted: Vec<(String, &Registered)> = w.registered.iter().map(|widget| (type_info(state, widget.id_base).0, widget)).collect();
    sorted.sort_by(|a, b| rustpress_blocks::general::strnatcasecmp(&a.0, &b.0));
    let mut done: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for (_, widget) in sorted {
        if done.contains(&widget.id_base) {
            continue;
        }
        done.push(widget.id_base);
        let mut item = registered_json(state, widget);
        let (width, height) = control_size(widget.id_base);
        item.insert("temp_id".into(), json!(format!("{}-__i__", widget.id_base)));
        item.insert("is_multi".into(), json!(true));
        item.insert("multi_number".into(), json!(w.next_number(widget.id_base)));
        item.insert("is_disabled".into(), json!(false));
        item.insert("id_base".into(), json!(widget.id_base));
        item.insert("transport".into(), json!("postMessage"));
        item.insert("width".into(), json!(width));
        item.insert("height".into(), json!(height));
        item.insert("is_wide".into(), json!(false));
        out.push((widget.clone(), item));
    }
    out
}

/// `var _wpCustomizeWidgetsSettings = ...;`.
/// `array_values( $wp_registered_sidebars )`.
pub(super) fn sidebars_json(w: &Widgets) -> Value {
    let sidebars: Vec<Value> = w
        .sidebars
        .iter()
        .map(|s| {
            json!({
                "name": s.name, "id": s.id, "description": s.description, "class": s.class, "before_widget": s.before_widget, "after_widget": s.after_widget,
                "before_title": s.before_title, "after_title": s.after_title, "before_sidebar": "", "after_sidebar": "", "show_in_rest": s.show_in_rest,
            })
        })
        .collect();
    Value::Array(sidebars)
}

/// `$wp_registered_widgets` without the callbacks.
pub(super) fn registered_widgets_json(state: &State<'_>, w: &Widgets) -> Value {
    let mut registered = Map::new();
    for widget in &w.registered {
        registered.insert(widget.id.clone(), Value::Object(registered_json(state, widget)));
    }
    Value::Object(registered)
}

/// `get_selective_refreshable_widgets()`: every core widget can.
pub(super) fn refreshable_json() -> Value {
    let mut refreshable = Map::new();
    for (id_base, ..) in rustpress_rest::edit::WIDGET_TYPES {
        refreshable.insert(id_base.to_owned(), json!(true));
    }
    Value::Object(refreshable)
}

pub(super) fn settings_data(state: &State<'_>, w: &Widgets) -> String {
    let ctx = state.ctx;
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    let decode = rustpress_core::formatting::html_entity_decode;
    let sidebars = sidebars_json(w);
    let registered = registered_widgets_json(state, w);
    let available: Vec<Value> = available(state, w).into_iter().map(|(_, item)| Value::Object(item)).collect();
    let count = w.sidebars.len() as i64;
    let mut some_shown = Map::new();
    some_shown.insert("1".into(), json!(decode(&tr("Your theme has 1 other widget area, but this particular page does not display it."))));
    for n in 2..count {
        some_shown.insert(n.to_string(), json!(decode(&sprintf(&site.trn("Your theme has %s other widget area, but this particular page does not display it.", "Your theme has %s other widget areas, but this particular page does not display them.", n), &[&n.to_string()]))));
    }
    let none_shown = if count == 1 {
        decode(&tr("Your theme has 1 widget area, but this particular page does not display it."))
    } else {
        decode(&sprintf(&site.trn("Your theme has %s widget area, but this particular page does not display it.", "Your theme has %s widget areas, but this particular page does not display them.", count), &[&count.to_string()]))
    };
    let reorder_nav = format!(
        "<div class=\"widget-reorder-nav\"><span class=\"move-widget\" tabindex=\"0\">{}</span><span class=\"move-widget-down\" tabindex=\"0\">{}</span><span class=\"move-widget-up\" tabindex=\"0\">{}</span></div>",
        tr("Move to another area&hellip;"),
        tr("Move down"),
        tr("Move up")
    );
    let move_area = format!(
        "<div class=\"move-widget-area\">\n\t\t\t\t<p class=\"description\">{}</p>\n\t\t\t\t<ul class=\"widget-area-select\">\n\t\t\t\t\t<% _.each( sidebars, function ( sidebar ){{ %>\n\t\t\t\t\t\t<li class=\"\" data-id=\"<%- sidebar.id %>\" tabindex=\"0\">\n\t\t\t\t\t\t\t<div><strong><%- sidebar.name %></strong></div>\n\t\t\t\t\t\t\t<div><%- sidebar.description %></div>\n\t\t\t\t\t\t</li>\n\t\t\t\t\t<% }}); %>\n\t\t\t\t</ul>\n\t\t\t\t<div class=\"move-widget-actions\">\n\t\t\t\t\t<button class=\"move-widget-btn button\" type=\"button\">{}</button>\n\t\t\t\t</div>\n\t\t\t</div>",
        tr("Select an area to move this widget into:"),
        site.trx("Move", "Move widget")
    );
    let refreshable = refreshable_json();
    let settings = json!({
        "registeredSidebars": sidebars,
        "registeredWidgets": registered,
        "availableWidgets": available,
        "l10n": {
            "saveBtnLabel": tr("Apply"),
            "saveBtnTooltip": tr("Save and preview changes before publishing them."),
            "removeBtnLabel": tr("Remove"),
            "removeBtnTooltip": tr("Keep widget settings and move it to the inactive widgets"),
            "error": tr("An error has occurred. Please reload the page and try again."),
            "widgetMovedUp": tr("Widget moved up"),
            "widgetMovedDown": tr("Widget moved down"),
            "navigatePreview": tr("You can navigate to other pages on your site while using the Customizer to view and edit the widgets displayed on those pages."),
            "someAreasShown": some_shown,
            "noAreasShown": none_shown,
            "reorderModeOn": tr("Reorder mode enabled"),
            "reorderModeOff": tr("Reorder mode closed"),
            "reorderLabelOn": esc_attr(&tr("Reorder widgets")),
            "widgetsFound": tr("Number of widgets found: %d"),
            "noWidgetsFound": tr("No widgets found."),
        },
        "tpl": {"widgetReorderNav": reorder_nav, "moveWidgetArea": move_area},
        "selectiveRefreshableWidgets": refreshable,
    });
    format!("var _wpCustomizeWidgetsSettings = {};", inline_json(&settings))
}

/// The form of a widget type as `form_callback( -1 )` prints it.
fn form(state: &State<'_>, id_base: &str, menus: &[rustpress_db::Term]) -> String {
    let ctx = state.ctx;
    if id_base == "nav_menu" {
        return nav_menu_form(state, menus);
    }
    let start = format!("⟨{id_base}⟩");
    let Some(at) = FORMS.find(&start) else { return String::new() };
    let rest = &FORMS[at + start.len()..];
    let end = rest.find('⟨').unwrap_or(rest.len());
    let form = super::super::template_text::translate(ctx.site(), &rest[..end]);
    if id_base != "tag_cloud" {
        return form;
    }
    // The taxonomies with `show_tagcloud`, the tags selected.
    let site = ctx.site();
    let mut taxonomies = vec![
        ("category".to_owned(), site.trx("Categories", "taxonomy general name")),
        ("post_tag".to_owned(), site.trx("Tags", "taxonomy general name")),
        ("link_category".to_owned(), site.tr("Link Categories")),
    ];
    for registered in site.types.registered_types(true).into_iter().filter(|t| t.public) {
        let label = registered.label.clone().unwrap_or_else(|| site.trx("Tags", "taxonomy general name"));
        taxonomies.push((registered.name, label));
    }
    let options: String = taxonomies
        .iter()
        .map(|(name, label)| format!("\t\t\t\t\t\t\t\t\t\t\t<option value=\"{}\" {}>\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</option>\n", esc_attr(name), if name == "post_tag" { " selected='selected'" } else { "" }, esc_html(label)))
        .collect();
    form.replace("{{tag_cloud_taxonomies}}", &options)
}

/// `WP_Nav_Menu_Widget::form()` of a new widget in the Customizer.
fn nav_menu_form(state: &State<'_>, menus: &[rustpress_db::Term]) -> String {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let (empty_style, not_empty_style) = if menus.is_empty() { (" style=\"display:none\" ", "") } else { ("", " style=\"display:none\" ") };
    let mut options = String::new();
    for menu in menus {
        options.push_str(&format!("\t\t\t\t\t\t<option value=\"{}\" >\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</option>\n\t\t\t\t\t", menu.term_id, esc_html(&menu.name)));
    }
    format!(
        "\t\t<p class=\"nav-menu-widget-no-menus-message\" {not_empty_style}>\n\t\t\t{}\t\t</p>\n\t\t<div class=\"nav-menu-widget-form-controls\" {empty_style}>\n\t\t\t<p>\n\t\t\t\t<label for=\"widget-nav_menu-__i__-title\">{}</label>\n\t\t\t\t<input type=\"text\" class=\"widefat\" id=\"widget-nav_menu-__i__-title\" name=\"widget-nav_menu[__i__][title]\" value=\"\" />\n\t\t\t</p>\n\t\t\t<p>\n\t\t\t\t<label for=\"widget-nav_menu-__i__-nav_menu\">{}</label>\n\t\t\t\t<select id=\"widget-nav_menu-__i__-nav_menu\" name=\"widget-nav_menu[__i__][nav_menu]\">\n\t\t\t\t\t<option value=\"0\">{}</option>\n\t\t\t\t\t{options}\t\t\t\t</select>\n\t\t\t</p>\n\t\t\t\t\t\t\t<p class=\"edit-selected-nav-menu\" style=\"display: none;\">\n\t\t\t\t\t<button type=\"button\" class=\"button\">{}</button>\n\t\t\t\t</p>\n\t\t\t\t\t</div>\n\t\t",
        sprintf(&tr("No menus have been created yet. <a href=\"%s\">Create some</a>."), &[&esc_attr("javascript: wp.customize.panel( \"nav_menus\" ).focus();")]),
        tr("Title:"),
        tr("Select Menu:"),
        tr("&mdash; Select &mdash;"),
        tr("Edit Menu")
    )
}

/// `wp_widget_control()` of an available widget: `index` is the counter of `wp_list_widget_controls_dynamic_sidebar()`.
fn widget_control(state: &State<'_>, widget: &Registered, index: usize, multi_number: i64, menus: &[rustpress_db::Term]) -> String {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let trx = |text: &str, context: &str| ctx.site().trx(text, context);
    let (name, description, _) = type_info(state, widget.id_base);
    let title = esc_html(&wp_strip_all_tags(&name));
    let id_format = format!("{}-__i__", widget.id_base);
    let (width, height) = control_size(widget.id_base);
    let mut href = ctx.request.request_uri.clone();
    for (key, value) in [("editwidget", widget.id.clone()), ("addnew", "1".to_owned()), ("num", multi_number.to_string()), ("base", widget.id_base.to_owned())] {
        href = rustpress_core::urls::add_query_arg(&href, key, &value);
    }
    let description = esc_html(&description);
    format!(
        "<div id='widget-{index}_{id_format}' class='widget'>\t<div class=\"widget-top\">\n\t<div class=\"widget-title-action\">\n\t\t<button type=\"button\" class=\"widget-action hide-if-no-js\" aria-expanded=\"false\">\n\t\t\t<span class=\"screen-reader-text edit\">\n\t\t\t\t{edit_widget}\t\t\t</span>\n\t\t\t<span class=\"screen-reader-text add\">\n\t\t\t\t{add_widget}\t\t\t</span>\n\t\t\t<span class=\"toggle-indicator\" aria-hidden=\"true\"></span>\n\t\t</button>\n\t\t<a class=\"widget-control-edit hide-if-js\" href=\"{href}\">\n\t\t\t<span class=\"edit\">{edit}</span>\n\t\t\t<span class=\"add\">{add}</span>\n\t\t\t<span class=\"screen-reader-text\">{title}</span>\n\t\t</a>\n\t</div>\n\t<div class=\"widget-title\"><h3>{title}<span class=\"in-widget-title\"></span></h3></div>\n\t</div>\n\n\t<div class=\"widget-inside\">\n\t<div class=\"form\">\t<div class=\"widget-content\">\t{form}\t</div><!-- .widget-content -->\t<input type=\"hidden\" name=\"widget-id\" class=\"widget-id\" value=\"{id_format}\" />\n\t<input type=\"hidden\" name=\"id_base\" class=\"id_base\" value=\"{id_base}\" />\n\t<input type=\"hidden\" name=\"widget-width\" class=\"widget-width\" value=\"{width}\" />\n\t<input type=\"hidden\" name=\"widget-height\" class=\"widget-height\" value=\"{height}\" />\n\t<input type=\"hidden\" name=\"widget_number\" class=\"widget_number\" value=\"{number}\" />\n\t<input type=\"hidden\" name=\"multi_number\" class=\"multi_number\" value=\"{multi_number}\" />\n\t<input type=\"hidden\" name=\"add_new\" class=\"add_new\" value=\"multi\" />\n\n\t<div class=\"widget-control-actions\">\n\t\t<div class=\"alignleft\">\n\t\t\t<button type=\"button\" class=\"button-link button-link-delete widget-control-remove\">{delete}</button>\n\t\t\t<span class=\"widget-control-close-wrapper\">\n\t\t\t\t| <button type=\"button\" class=\"button-link widget-control-close\">{done}</button>\n\t\t\t</span>\n\t\t</div>\n\t\t<div class=\"alignright\">\n\t\t\t<input type=\"submit\" name=\"savewidget\" id=\"widget-{id_format}-savewidget\" class=\"button button-primary widget-control-save right\" value=\"{save}\"  />\t\t\t<span class=\"spinner\"></span>\n\t\t</div>\n\t</div>\n\t</div><!-- .form -->\t</div>\n\n\t<div class=\"widget-description\">\n\t{description}\n\t</div>\n\t</div>",
        edit_widget = sprintf(&tr("Edit widget: %s"), &[&title]),
        add_widget = sprintf(&tr("Add widget: %s"), &[&title]),
        href = esc_url(&href),
        edit = trx("Edit", "widget"),
        add = trx("Add", "widget"),
        form = form(state, widget.id_base, menus),
        id_base = esc_attr(widget.id_base),
        number = widget.number,
        delete = tr("Delete"),
        done = tr("Done"),
        save = esc_attr(&tr("Save")),
    )
}

/// `output_widget_control_templates()`.
pub(super) fn control_templates(state: &State<'_>, w: &Widgets, menus: &[rustpress_db::Term], panel_title: &str) -> String {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let mut out = format!(
        "\t\t<div id=\"widgets-left\"><!-- compatibility with JS which looks for widget templates here -->\n\t\t<div id=\"available-widgets\">\n\t\t\t<div class=\"customize-section-title\">\n\t\t\t\t<button class=\"customize-section-back\" tabindex=\"-1\">\n\t\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</span>\n\t\t\t\t</button>\n\t\t\t\t<h3>\n\t\t\t\t\t<span class=\"customize-action\">\n\t\t\t\t\t\t{}\t\t\t\t\t</span>\n\t\t\t\t\t{}\t\t\t\t</h3>\n\t\t\t</div>\n\t\t\t<div id=\"available-widgets-filter\">\n\t\t\t\t<label for=\"widgets-search\">\n\t\t\t\t\t{}\t\t\t\t</label>\n\t\t\t\t<input type=\"text\" id=\"widgets-search\" aria-describedby=\"widgets-search-desc\" />\n\t\t\t\t<div class=\"search-icon\" aria-hidden=\"true\"></div>\n\t\t\t\t<button type=\"button\" class=\"clear-results\"><span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span></button>\n\t\t\t\t<p class=\"screen-reader-text\" id=\"widgets-search-desc\">\n\t\t\t\t\t{}\t\t\t\t</p>\n\t\t\t</div>\n\t\t\t<div id=\"available-widgets-list\">\n\t\t\t",
        tr("Back"),
        sprintf(&tr("Customizing &#9656; %s"), &[&esc_html(panel_title)]),
        tr("Add a Widget"),
        tr("Search Widgets"),
        tr("Clear Results"),
        tr("The search results will be updated as you type.")
    );
    for (index, (widget, item)) in available(state, w).into_iter().enumerate() {
        let multi = item.get("multi_number").and_then(Value::as_i64).unwrap_or(0);
        let id = esc_attr(&widget.id);
        out.push_str(&format!("\t\t\t\t<div id=\"widget-tpl-{id}\" data-widget-id=\"{id}\" class=\"widget-tpl {id}\" tabindex=\"0\">\n\t\t\t\t\t{}\t\t\t\t</div>\n\t\t\t", widget_control(state, &widget, index + 1, multi, menus)));
    }
    out.push_str(&format!("\t\t\t<p class=\"no-widgets-found-message\">{}</p>\n\t\t\t</div><!-- #available-widgets-list -->\n\t\t</div><!-- #available-widgets -->\n\t\t</div><!-- #widgets-left -->\n\t\t", tr("No widgets found.")));
    out
}
