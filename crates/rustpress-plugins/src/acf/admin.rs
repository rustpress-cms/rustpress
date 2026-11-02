//! ACF in the frame of every administration screen: its menu (`ACF_Admin::admin_menu()` and the `admin_menu` of the
//! field group, post type, taxonomy, options preview and tools screens), the global stylesheet and the script of the
//! escaped-HTML notice (`ACF_Admin::admin_enqueue_scripts()`), and the body classes (`ACF_Admin::admin_body_class()`).
//!
//! Differs from the plugin: the "Upgrade Database" entry of a pending database upgrade is not added.

use rustpress_core::Site;
use rustpress_core::native::{AdminContext, AdminPage, Assets};

/// `acf_get_setting( 'capability' )`.
const CAPABILITY: &str = "manage_options";
const PARENT: &str = "edit.php?post_type=acf-field-group";

fn tr(site: &Site, text: &str) -> String {
    site.trd("acf", text)
}

/// The menu and its entries.
pub fn admin_pages(site: &Site) -> Vec<AdminPage> {
    let page = |parent: &'static str, slug: &'static str, title: &str| AdminPage { parent, slug, page_title: tr(site, title), menu_title: tr(site, title), capability: CAPABILITY, ..AdminPage::default() };
    vec![
        AdminPage { icon: "dashicons-welcome-widgets-menus", position: Some(80), ..page("", PARENT, "ACF") },
        page(PARENT, PARENT, "Field Groups"),
        page(PARENT, "edit.php?post_type=acf-post-type", "Post Types"),
        page(PARENT, "edit.php?post_type=acf-taxonomy", "Taxonomies"),
        page(PARENT, "acf_options_preview", "Options Pages"),
        page(PARENT, "acf-tools", "Tools"),
    ]
}

/// `ACF_Admin::admin_enqueue_scripts()`: on every screen.
pub fn admin_enqueue(site: &Site, assets: &mut Assets) {
    let url = |path: &str| site.site_url(&format!("wp-content/plugins/advanced-custom-fields/{path}"));
    assets.style("acf-global", url("assets/build/css/acf-global.min.css"), &["dashicons"], super::VERSION, "all");
    let data = serde_json::json!({"show_details": tr(site, "Show&nbsp;details"), "hide_details": tr(site, "Hide&nbsp;details")});
    // `wp_localize_script()` decodes the entities of its strings.
    let data = match data {
        serde_json::Value::Object(map) => serde_json::Value::Object(map.into_iter().map(|(k, v)| (k, serde_json::Value::String(rustpress_core::formatting::html_entity_decode(v.as_str().unwrap_or_default())))).collect()),
        other => other,
    };
    let json = rustpress_blocks::php_json::encode_with(&data, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E");
    let script = assets.script("acf-escaped-html-notice", url("assets/build/js/acf-escaped-html-notice.min.js"), &["jquery"], super::VERSION, true);
    script.extra = Some(format!("var acf_escaped_html_notice = {json};"));
}

/// `ACF_Admin::admin_body_class()` (`acf-admin-7-0` for WordPress 7.0 and later, which `WORDPRESS_VERSION` is).
pub fn admin_body_class(ctx: &AdminContext<'_>, classes: String) -> String {
    format!("{classes} acf-admin-5-3 acf-admin-7-0 acf-browser-{}", super::form::browser(ctx.user_agent))
}
