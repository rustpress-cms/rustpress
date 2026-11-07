//! What `customize.php` enqueues, in the order WordPress does: `heartbeat` and `customize-controls`, then
//! `customize_controls_enqueue_scripts` (the widgets with `admin_enqueue_scripts( 'widgets.php' )` and the block editor,
//! the menus, each control, the theme), then what the widgets enqueue on `admin_print_scripts-widgets.php` while the head
//! is printed, and what `wp_enqueue_editor()` leaves for the footer.

use serde_json::json;

use super::State;
use super::manager::Manager;
use crate::chrome::{Assets, CUSTOMIZE_EDITOR_PLUGIN_SCRIPTS as EDITOR_PLUGIN_SCRIPTS, CUSTOMIZE_PLUGIN_SCRIPTS as PLUGIN_SCRIPTS};
use crate::screens::block_editor::{EditorPage, SettingsFor, block_styles_script, editor_head_styles, editor_settings, inline_json, shared_inline};

fn push(list: &mut Vec<String>, handles: &[&str]) {
    list.extend(handles.iter().map(|h| (*h).to_owned()));
}

pub(super) async fn assets(state: &State<'_>, m: &Manager, widgets_data: Option<String>, menus_data: (String, serde_json::Value), header_data: Option<serde_json::Value>) -> Assets {
    let ctx = state.ctx;
    let env = ctx.env;
    let widgets = widgets_data.is_some();
    let mut a = Assets { editor_modules: widgets, ..Default::default() };
    push(&mut a.styles, &["customize-controls"]);
    push(&mut a.scripts, &["heartbeat", "customize-controls"]);

    // `WP_Customize_Widgets::enqueue_scripts()`.
    if let Some(data) = widgets_data {
        push(&mut a.styles, &["customize-widgets"]);
        push(&mut a.scripts, &["customize-widgets"]);
        // `admin_enqueue_scripts( 'widgets.php' )`: the styles of the blocks, the command palette, then the plugins.
        push(&mut a.styles, &["wp-block-library"]);
        if env.theme.adds_theme_support("wp-block-styles") {
            push(&mut a.styles, &["wp-block-library-theme"]);
        }
        push(&mut a.styles, &["classic-theme-styles", "wp-commands"]);
        push(&mut a.scripts, &["wp-commands", "wp-core-commands", PLUGIN_SCRIPTS]);
        a.route_at_start = true;
        a.raw_data.push(("customize-widgets".into(), data));
        let info = crate::screens::posts::TypeInfo { name: String::new(), hierarchical: false, label: String::new(), singular: String::new(), labels: crate::screens::posts::ListLabels::defaults(ctx, false) };
        let settings = editor_settings(ctx, &rustpress_db::Post::default(), &info, SettingsFor::CustomizeWidgets).await;
        a.inline.push(("wp-customize-widgets".into(), format!("wp.domReady( function() {{\n\t\t\t\t\t   wp.customizeWidgets.initialize( \"widgets-customizer\", {} );\n\t\t\t\t\t}} );", inline_json(&settings)), "after".into()));
        push(&mut a.scripts, &["wp-customize-widgets"]);
        push(&mut a.styles, &["wp-customize-widgets"]);
        // `enqueue_block_editor_assets`, of WordPress and of the plugins.
        push(&mut a.scripts, &["wp-block-styles", "wp-block-directory", "wp-format-library", EDITOR_PLUGIN_SCRIPTS]);
        {
            let parts = ctx.plugin_parts();
            let mut plugin_ctx = ctx.plugin_context(&parts, "widgets.php");
            plugin_ctx.block_editor = true;
            for script in ctx.site().plugins.block_editor_assets(&plugin_ctx).scripts {
                if script.deps.iter().any(|dep| dep == "wp-edit-post") {
                    a.route_first = true;
                }
                a.plugin_page_scripts.push(script);
            }
        }
        push(&mut a.styles, &["wp-block-directory", "wp-format-library"]);
        editor_head_styles(ctx, &mut a);
        // Enqueued above, with the styles of the blocks.
        a.late_styles.clear();
        a.inline.push(("wp-block-styles".into(), block_styles_script(ctx), "after".into()));
        shared_inline(ctx, &mut a, &json!({}), EditorPage::Site).await;
        // Nothing is preloaded.
        for (handle, code, _) in &mut a.inline {
            if handle == "wp-api-fetch"
                && let Some(at) = code.find("\nwp.apiFetch.use( wp.apiFetch.createPreloadingMiddleware(")
            {
                code.truncate(at);
            }
        }
        a.inline.retain(|(handle, _, position)| !(handle == "wp-block-library" && position == "before"));
    }

    // `WP_Customize_Nav_Menus::enqueue_scripts()`.
    push(&mut a.styles, &["customize-nav-menus"]);
    push(&mut a.scripts, &["customize-nav-menus"]);
    a.raw_data.push(("customize-nav-menus".into(), menus_data.0));
    a.localize.push(("nav-menu".into(), "menus".into(), menus_data.1));

    // `enqueue()` of each control.
    let mut media = false;
    for control in &m.controls {
        match control.kind.as_str() {
            "color" => {
                push(&mut a.scripts, &["wp-color-picker"]);
                push(&mut a.styles, &["wp-color-picker"]);
            }
            "media" | "cropped_image" | "site_icon" | "header" => {
                if control.kind != "media" {
                    push(&mut a.scripts, &["customize-views"]);
                }
                if !media {
                    media = true;
                    let fetch = a.inline.iter().position(|(handle, _, _)| handle == "wp-api-fetch");
                    crate::screens::media_ui::enqueue(ctx, &mut a, 0).await;
                    a.pre_footer.clear();
                    // The fetch layer is set up once.
                    if fetch.is_some()
                        && let Some(last) = a.inline.iter().rposition(|(handle, _, _)| handle == "wp-api-fetch")
                    {
                        a.inline.remove(last);
                    }
                }
                if control.kind == "header"
                    && let Some(data) = &header_data
                {
                    push(&mut a.scripts, &["customize-views"]);
                    a.localize.push(("customize-views".into(), "_wpCustomizeHeader".into(), data.clone()));
                }
            }
            // `wp_enqueue_code_editor()` for CSS: the linter stays off.
            "code_editor" if state.syntax_highlighting => {
                push(&mut a.scripts, &["code-editor"]);
                push(&mut a.styles, &["code-editor"]);
                let mut code = crate::screens::file_editor::code_editor_settings(ctx, "custom.css");
                code["codemirror"]["indentUnit"] = json!(2);
                code["codemirror"]["tabSize"] = json!(2);
                a.inline.push(("code-editor".into(), format!("jQuery.extend( wp.codeEditor.defaultSettings, {} );", inline_json(&code)), "after".into()));
            }
            _ => {}
        }
    }
    if ctx.can("install_themes") || ctx.can("update_themes") || ctx.can("delete_themes") {
        push(&mut a.scripts, &["updates"]);
        let updates = crate::updates::counts(env.site).await;
        a.localize.push(("updates".into(), "_wpUpdatesItemCounts".into(), json!({"totals": crate::updates::update_data(ctx.site(), updates)})));
    }
    // `twentyseventeen_panels_js()`.
    if env.theme.template == "twentyseventeen" {
        let src = format!("{}/assets/js/customize-controls.js", state.template_uri);
        a.plugin_scripts.push(rustpress_core::native::Asset { handle: "twentyseventeen-customize-controls".into(), src, ver: Some("20161020".into()), in_footer: true, registered_only: true, ..Default::default() });
        push(&mut a.scripts, &["twentyseventeen-customize-controls"]);
    }

    // `admin_print_scripts-widgets.php`: the media, text and custom HTML widgets.
    if widgets {
        crate::screens::widgets::admin_scripts(ctx, &mut a).await;
        a.scripts.append(&mut a.late_scripts);
        // `wp_enqueue_editor()` while the head is printed.
        push(&mut a.post_header_styles, &["editor-buttons"]);
    }
    a
}
