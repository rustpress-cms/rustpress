//! Pages that `@wordpress/boot` renders (`wp-includes/build/pages`): the font library and the connectors.
//! Their routes are listed in `wp-includes/build/routes/registry.php`; the page itself is WordPress' JavaScript.

use serde_json::{Value, json};

use super::block_editor::{preload_paths, shared_inline};
use super::{AdminResponse, Ctx, die, render};
use crate::chrome::{Assets, Boot, BootRoute, Screen};

/// The routes the registry lists for a page, in its order.
fn routes_of(ctx: &Ctx<'_>, page: &str) -> Vec<BootRoute> {
    let registry = std::fs::read_to_string(ctx.env.loader.includes.join("build/routes/registry.php")).unwrap_or_default();
    let field = |entry: &str, key: &str| -> Option<String> {
        let after = entry.split(&format!("'{key}'")).nth(1)?;
        let value = after.split("=>").nth(1)?.split(',').next()?.trim();
        Some(value.trim_matches('\'').to_owned())
    };
    registry
        .split("array(")
        .skip(2)
        .filter_map(|entry| {
            let entry = entry.split(')').next().unwrap_or_default();
            (field(entry, "page")? == page).then(|| BootRoute {
                path: field(entry, "path").unwrap_or_default(),
                name: field(entry, "name").unwrap_or_default(),
                has_route: field(entry, "has_route").is_some_and(|v| v == "true"),
                has_content: field(entry, "has_content").is_some_and(|v| v == "true"),
            })
        })
        .collect()
}

/// `wp_<page>_wp_admin_render_page()`.
const CRITICAL_STYLES: &str = "\t<style>\n\t\t/* Critical styles to prevent layout shifts - inlined for immediate application */\n\n\t\t#wpwrap {\n\t\t\toverflow-y: auto;\n\t\t}\n\t\tbody.js {\n\t\t\tbackground: #fff;\n\t\t}\n\n\t\t/* Reset wp-admin padding */\n\t\tbody.js #wpcontent {\n\t\t\tpadding-inline-start: 0;\n\t\t}\n\t\tbody.js #wpbody-content {\n\t\t\tpadding-bottom: 0;\n\t\t}\n\n\t\t/* Hide legacy admin elements */\n\t\tbody.js #wpbody-content > div:not(.boot-layout-container):not(#screen-meta) {\n\t\t\tdisplay: none;\n\t\t}\n\t\tbody.js #wpfooter {\n\t\t\tdisplay: none;\n\t\t}\n\n\t\t/* Accessibility regions */\n\t\t.a11y-speak-region {\n\t\t\tinset-inline-start: -1px;\n\t\t\ttop: -1px;\n\t\t}\n\n\t\t/* Admin menu indicators */\n\t\tul#adminmenu a.wp-has-current-submenu::after,\n\t\tul#adminmenu > li.current > a.current::after {\n\t\t\tborder-inline-end-color: #fff;\n\t\t}\n\n\t\t/* Media frame fix */\n\t\t.media-frame select.attachment-filters:last-of-type {\n\t\t\twidth: auto;\n\t\t\tmax-width: 100%;\n\t\t}\n\n\t\t/* Responsive overflow fix for #wpwrap */\n\t\t@media (min-width: 782px) {\n\t\t\t#wpwrap {\n\t\t\t\toverflow-y: initial;\n\t\t\t}\n\t\t}\n\t</style>\n";

/// A boot page: `page` is the slug, `file` the screen, `title` the heading shown without JavaScript.
async fn render_boot(ctx: &Ctx<'_>, page: &str, file: &str, parent_file: &str, title: String, no_js: String) -> AdminResponse {
    let routes = routes_of(ctx, page);
    let paths: Vec<(String, String)> = vec![
        ("GET".into(), "/?_fields=description,gmt_offset,home,image_max_bit_depth,image_sizes,image_size_threshold,image_strip_meta,name,site_icon,site_icon_url,site_logo,timezone_string,url,page_for_posts,page_on_front,show_on_front".into()),
        ("OPTIONS".into(), "/wp/v2/settings".into()),
    ];
    let preloaded = preload_paths(ctx, paths).await;
    let mut assets = Assets { boot: Some(Boot { page: page.to_owned(), routes }), ..Default::default() };
    shared_inline(ctx, &mut assets, &preloaded).await;
    assets.inline.retain(|(handle, _, position)| !(handle == "wp-block-library" && position == "before") && handle != "wp-blocks");
    let content = format!(
        "<div class=\"wrap hide-if-js\">\n\t<h1 class=\"wp-heading-inline\">{}</h1>\n\t<div class=\"notice notice-error hide-if-js\"><p>{}</p></div></div>\n{CRITICAL_STYLES}\t<div id=\"{page}-wp-admin-app\" class=\"boot-layout-container\"></div>\n\t\n",
        rustpress_core::formatting::esc_html(&title),
        no_js
    );
    let screen = Screen { file: file.to_owned(), parent_file: parent_file.to_owned(), submenu_file: Some(file.to_owned()), title, help: Some(crate::chrome::Help::default()), ..Default::default() };
    let _ = (json!(Value::Null), die);
    render(ctx, &screen, &content, &assets).await
}

/// `wp-admin/font-library.php`.
pub async fn font_library(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("edit_theme_options") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("You need a higher level of permission."), tr("Sorry, you are not allowed to manage fonts on this site.")));
    }
    render_boot(ctx, "font-library", "font-library.php", "themes.php", tr("Fonts"), tr("The Fonts screen requires JavaScript. Please enable JavaScript in your browser settings to install and manage fonts.")).await
}
