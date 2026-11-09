//! The preview of the Customizer on the website: what `WP_Customize_Manager::customize_preview_settings()`,
//! `WP_Customize_Widgets::export_preview_data()` and `WP_Customize_Selective_Refresh::export_preview_data()` print at
//! the end of `wp_footer()`.
//!
//! The settings are registered as on the website: `customize_register` runs on `wp_loaded`, but the widgets register
//! theirs on `wp`, after `customize_preview_init()` prepared the controls. Their settings, panel, sections and controls
//! come last and unsorted. The active callbacks run with the query of the page.

use rustpress_core::QueryResult;
use rustpress_core::customize::Preview;
use serde_json::{Map, Value, json};

use super::manager::Manager;
use super::{State, nav_menus, register, widgets};
use crate::screens::Ctx;

/// `wp_json_encode( $data, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )`.
fn hex_tag(value: &Value) -> String {
    rustpress_theme::customize_preview::json_hex_tag(value)
}

/// The three exports, each in the tag of `wp_print_inline_script_tag()`.
pub async fn exports<'a>(ctx: &'a Ctx<'a>, stored: &'a Ctx<'a>, q: &QueryResult, preview: &Preview) -> String {
    let mut state = super::state(ctx).await;
    state.preview = true;
    state.items_ctx = Some(stored);
    state.previewed = Some(preview);
    if let Some(css) = &preview.custom_css {
        state.custom_css = css.clone();
    }
    let widgets_on = preview.widgets;
    let menus = nav_menus::menus(&state).await;
    let seventeen = ctx.env.theme.template == "twentyseventeen";
    let locations = if seventeen { crate::screens::nav_menus::registered_locations(ctx) } else { Vec::new() };
    let mut m = Manager::default();
    if widgets_on {
        for id in preview.previewed.iter().filter(|id| widgets::is_widget_setting(id)) {
            m.add_setting(super::manager::Setting::new(id, "option", "postMessage", Value::Null));
        }
    }
    register::core(&state, &mut m).await;
    if seventeen {
        register::twentyseventeen(&state, &mut m);
    }
    nav_menus::register(&state, &mut m, &menus, &locations, widgets_on).await;
    // `check_capabilities()`: all of them need `edit_theme_options`, which a visitor does not have.
    let allowed = ctx.can("edit_theme_options");
    let restrict = |m: &mut Manager| {
        if !allowed {
            m.settings.iter_mut().for_each(|s| s.allowed = false);
            m.sections.iter_mut().for_each(|s| s.allowed = false);
            m.panels.iter_mut().for_each(|p| p.allowed = false);
        }
    };
    restrict(&mut m);
    m.prepare(&|capability| ctx.can(capability));
    // `prepare_controls()` keeps the sections of panels with their panel only.
    let prepared_sections = m.sections.len();
    let widget_state = if widgets_on { Some(widgets::load(&state).await) } else { None };
    if let Some(w) = &widget_state {
        widgets::register(&state, &mut m, w);
        restrict(&mut m);
    }

    let mut out = settings_script(&state, &m, prepared_sections, q, preview);
    if let Some(w) = widget_state.as_ref().filter(|_| preview.widgets_previewed()) {
        out.push_str(&widgets_script(&state, w, preview));
    }
    out.push_str(&partials_script(&state, &m, seventeen));
    out
}

/// The settings of the preview (`customize_preview_settings()`).
fn settings_script(state: &State<'_>, m: &Manager, prepared_sections: usize, q: &QueryResult, preview: &Preview) -> String {
    let ctx = state.ctx;
    let site = ctx.site();
    let home = site.home_url("/");
    let rendered = preview.rendered();
    let sidebar_rendered = |id: &str| rendered.sidebars.iter().any(|s| s == id);
    let section_active = |section: &super::manager::Section| match section.extra.iter().find(|(key, _)| *key == "sidebarId").and_then(|(_, v)| v.as_str()) {
        // `WP_Customize_Sidebar_Section::active_callback()`.
        Some(sidebar) => sidebar_rendered(sidebar),
        None => section.active,
    };
    let mut panels = Map::new();
    let mut sections = Map::new();
    for panel in m.panels.iter().filter(|p| p.allowed) {
        panels.insert(panel.id.clone(), json!(panel.active));
        for section in m.sections[..prepared_sections].iter().filter(|s| s.panel == panel.id) {
            sections.insert(section.id.clone(), json!(section_active(section)));
        }
    }
    for (i, section) in m.sections.iter().enumerate() {
        if i < prepared_sections && !section.panel.is_empty() {
            continue;
        }
        if section.allowed && !sections.contains_key(&section.id) {
            sections.insert(section.id.clone(), json!(section_active(section)));
        }
    }
    let flags = &q.flags;
    let mut controls = Map::new();
    for control in &m.controls {
        if !m.control_allowed(control, &|capability| ctx.can(capability)) {
            continue;
        }
        let active = match control.id.as_str() {
            // `is_header_video_active()` with the default callback `is_front_page()`.
            "header_video" | "external_header_video" => flags.is_front_page,
            // `twentyseventeen_is_view_with_layout_option()`; its `is_active_sidebar()` counts the sidebar.
            "page_layout" => {
                flags.is_page
                    || (flags.is_archive && {
                        preview.tally_sidebar("sidebar-1");
                        !rustpress_theme::classic::widgets::is_active_sidebar(site, "sidebar-1")
                    })
            }
            // `twentyseventeen_is_frontpage()`.
            "panel_1" | "panel_2" | "panel_3" | "panel_4" if ctx.env.theme.template == "twentyseventeen" => flags.is_front_page && !flags.is_home,
            _ => control.active,
        };
        controls.insert(control.id.clone(), json!(active));
    }
    let mut self_url = q.url_path.clone();
    if !q.raw_query.is_empty() {
        self_url.push('?');
        self_url.push_str(&q.raw_query);
    }
    let mut self_url = rustpress_core::formatting::esc_url_raw(&self_url);
    for key in ["customize_theme", "customize_changeset_uuid", "customize_messenger_channel"] {
        self_url = rustpress_core::urls::remove_query_arg(&self_url, key);
    }
    let host = home.split_once("://").map_or("", |(_, rest)| rest.split('/').next().unwrap_or_default()).to_owned();
    // `validate_setting_values()` of the posted values: the settings that exist are valid.
    let valid: Map<String, Value> = preview.dirty.iter().filter(|id| m.settings.iter().any(|s| s.id == **id)).map(|id| (id.clone(), preview.validity(id))).collect();
    let validities = if valid.is_empty() { json!([]) } else { Value::Object(valid) };
    let nonce = if ctx.can("customize") { Value::Object(super::nonces(state)) } else { json!([]) };
    let settings = json!({
        "changeset": {"uuid": preview.uuid, "autosaved": preview.autosaved},
        "timeouts": {"selectiveRefresh": 250, "keepAliveSend": 1000},
        "theme": {"stylesheet": state.stylesheet, "active": preview.theme.is_none(), "isBlockTheme": ctx.env.block_theme},
        "url": {
            "self": self_url,
            "allowed": [rustpress_core::formatting::esc_url_raw(&home)],
            "allowedHosts": [host],
            "isCrossDomain": false,
        },
        "channel": if preview.channel.is_empty() { Value::Null } else { json!(preview.channel) },
        "activePanels": if panels.is_empty() { json!([]) } else { Value::Object(panels) },
        "activeSections": if sections.is_empty() { json!([]) } else { Value::Object(sections) },
        "activeControls": if controls.is_empty() { json!([]) } else { Value::Object(controls) },
        "settingValidities": validities,
        "nonce": nonce,
        "l10n": {
            "shiftClickToEdit": ctx.tr("Shift-click to edit this element."),
            "linkUnpreviewable": ctx.tr("This link is not live-previewable."),
            "formUnpreviewable": ctx.tr("This form is not live-previewable."),
        },
        "_dirty": preview.dirty,
    });
    let mut values = String::new();
    for setting in m.settings.iter().filter(|s| s.allowed) {
        values.push_str(&format!("v[{}] = {};\n", hex_tag(&json!(setting.id)), hex_tag(&setting.value)));
    }
    format!(
        "<script>\n\t\t\tvar _wpCustomizeSettings = {};\n\t\t\t_wpCustomizeSettings.values = {{}};\n\t\t\t(function( v ) {{\n\t\t\t\t{values}\t\t\t}})( _wpCustomizeSettings.values );\n\t\t\n//# sourceURL=WP_Customize_Manager%3A%3Acustomize_preview_settings\n</script>\n",
        hex_tag(&settings)
    )
}

/// `WP_Customize_Widgets::export_preview_data()`.
fn widgets_script(state: &State<'_>, w: &widgets::Widgets, preview: &Preview) -> String {
    let ctx = state.ctx;
    let rendered = preview.rendered();
    let fill = |ids: &[String]| if ids.is_empty() { json!([]) } else { Value::Object(ids.iter().map(|id| (id.clone(), json!(true))).collect()) };
    let settings = json!({
        "renderedSidebars": fill(&rendered.sidebars),
        "renderedWidgets": fill(&rendered.widgets),
        "registeredSidebars": widgets::sidebars_json(w),
        "registeredWidgets": widgets::registered_widgets_json(state, w),
        "l10n": {"widgetTooltip": ctx.tr("Shift-click to edit this widget.")},
        "selectiveRefreshableWidgets": widgets::refreshable_json(),
    });
    format!("<script>\nvar _wpWidgetCustomizerPreviewSettings = {};\n//# sourceURL=WP_Customize_Widgets%3A%3Aexport_preview_data\n</script>\n", hex_tag(&settings))
}

/// `WP_Customize_Selective_Refresh::export_preview_data()`: the partials that `register_controls()` and the theme add.
fn partials_script(state: &State<'_>, m: &Manager, seventeen: bool) -> String {
    let ctx = state.ctx;
    let allowed = |id: &str| m.settings.iter().any(|s| s.id == id && s.allowed);
    let mut partials = Map::new();
    let mut add = |id: &str, settings: &[&str], selector: &str, inclusive: bool| {
        if settings.iter().all(|s| allowed(s)) {
            partials.insert(
                id.to_owned(),
                json!({"settings": settings, "primarySetting": settings[0], "selector": selector, "type": "default", "fallbackRefresh": true, "containerInclusive": inclusive}),
            );
        }
    };
    if state.custom_logo.is_some() {
        add("custom_logo", &["custom_logo"], ".custom-logo-link", true);
    }
    if state.support.is_some() {
        add("custom_header", &["header_video", "external_header_video", "header_image"], "#wp-custom-header", true);
    }
    if seventeen {
        add("blogname", &["blogname"], ".site-title a", false);
        add("blogdescription", &["blogdescription"], ".site-description", false);
        for i in 1..=4 {
            add(&format!("panel_{i}"), &[&format!("panel_{i}")], &format!("#panel{i}"), true);
        }
    }
    let exports = json!({
        "partials": if partials.is_empty() { json!([]) } else { Value::Object(partials) },
        "renderQueryVar": "wp_customize_render_partials",
        "l10n": {
            "shiftClickToEdit": ctx.tr("Shift-click to edit this element."),
            "clickEditMenu": ctx.tr("Click to edit this menu."),
            "clickEditWidget": ctx.tr("Click to edit this widget."),
            "clickEditTitle": ctx.tr("Click to edit the site title."),
            "clickEditMisc": ctx.tr("Click to edit this element."),
            "badDocumentWrite": rustpress_core::i18n::sprintf(&ctx.tr("%s is forbidden"), &["document.write()"]),
        },
    });
    format!("<script>\nvar _customizePartialRefreshExports = {};\n//# sourceURL=WP_Customize_Selective_Refresh%3A%3Aexport_preview_data\n</script>\n", hex_tag(&exports))
}
