//! What the preview of the Customizer adds to a page of the website (`WP_Customize_Manager::customize_preview_init()`
//! and the `customize_preview_init` of its components): the styles and scripts of the head, the scripts of the footer
//! in the order WordPress queues them, and the place at the end of `wp_footer()` where the server puts the exports of
//! the settings (`customize_preview_settings()`, `WP_Customize_Widgets::export_preview_data()` and
//! `WP_Customize_Selective_Refresh::export_preview_data()`).

use std::path::Path;

use rustpress_core::Site;
use rustpress_core::customize::Preview;

use crate::script_loader::deps::{Arg, Extra, Ver};
use crate::script_loader::scripts::Scripts;

/// Where the exports of the settings go; the server replaces it.
pub const FOOTER_EXPORTS: &str = "<!--rustpress-customize-preview-exports-->";

/// `WP_Customize_Widgets::print_preview_css()` on `wp_print_styles` at priority 1, with what its `?>` leaves.
pub const WIDGETS_PREVIEW_CSS: &str = "\t\t<style>\n\t\t.widget-customizer-highlighted-widget {\n\t\t\toutline: none;\n\t\t\t-webkit-box-shadow: 0 0 2px rgba(30, 140, 190, 0.8);\n\t\t\tbox-shadow: 0 0 2px rgba(30, 140, 190, 0.8);\n\t\t\tposition: relative;\n\t\t\tz-index: 1;\n\t\t}\n\t\t</style>\n\t\t";

/// `customize_preview_loading_style()` and `remove_frameless_preview_messenger_channel()` on `wp_head`.
pub fn loading_style(preview: &Preview) -> String {
    let mut out = String::from(
        "\t\t<style>\n\t\t\tbody.wp-customizer-unloading {\n\t\t\t\topacity: 0.25;\n\t\t\t\tcursor: progress !important;\n\t\t\t\t-webkit-transition: opacity 0.5s;\n\t\t\t\ttransition: opacity 0.5s;\n\t\t\t}\n\t\t\tbody.wp-customizer-unloading * {\n\t\t\t\tpointer-events: none !important;\n\t\t\t}\n\t\t\tform.customize-unpreviewable,\n\t\t\tform.customize-unpreviewable input,\n\t\t\tform.customize-unpreviewable select,\n\t\t\tform.customize-unpreviewable button,\n\t\t\ta.customize-unpreviewable,\n\t\t\tarea.customize-unpreviewable {\n\t\t\t\tcursor: not-allowed !important;\n\t\t\t}\n\t\t</style>\n\t\t",
    );
    if !preview.channel.is_empty() {
        out.push_str("<script>\n\t\t( function() {\n\t\t\tif ( parent !== window ) {\n\t\t\t\treturn;\n\t\t\t}\n\t\t\tconst url = new URL( location.href );\n\t\t\tif ( url.searchParams.has( 'customize_messenger_channel' ) ) {\n\t\t\t\turl.searchParams.delete( 'customize_messenger_channel' );\n\t\t\t\tlocation.replace( url );\n\t\t\t}\n\t\t} )();\n\t\t\n//# sourceURL=WP_Customize_Manager%3A%3Aremove_frameless_preview_messenger_channel\n</script>\n");
    }
    out
}

/// The stylesheets the preview enqueues on `wp_loaded`, `customize-preview` after its `dashicons`, and the toolbar's
/// after them (it is enqueued on `template_redirect`).
pub fn styles(includes_url: &str, version: &str, rtl: bool, toolbar: bool) -> String {
    let variant = if rtl { "-rtl" } else { "" };
    let mut out = format!(
        "<link rel='stylesheet' id='dashicons-css' href='{includes_url}/css/dashicons.min.css?ver={version}' media='all' />\n<link rel='stylesheet' id='customize-preview{variant}-css' href='{includes_url}/css/customize-preview{variant}.min.css?ver={version}' media='all' />\n"
    );
    if toolbar {
        let bar = crate::document::toolbar_styles(includes_url, version, rtl);
        out.push_str(bar.split_once('\n').map_or("", |(_, rest)| rest));
    }
    out
}

/// The stylesheets of the media widgets' `enqueue_preview_scripts()` and of `wp_playlist_scripts()`.
pub fn media_styles(includes_url: &str, version: &str) -> String {
    format!(
        "<link rel='stylesheet' id='mediaelement-css' href='{includes_url}/js/mediaelement/mediaelementplayer-legacy.min.css?ver=4.2.17' media='all' />\n<link rel='stylesheet' id='wp-mediaelement-css' href='{includes_url}/js/mediaelement/wp-mediaelement.min.css?ver={version}' media='all' />\n"
    )
}

/// A script a theme registers for the preview: its handle, address, dependencies, version and loading strategy.
pub struct ThemeScript<'a> {
    pub handle: &'a str,
    pub src: String,
    pub deps: &'a [&'a str],
    pub ver: &'a str,
    pub strategy: Option<&'a str>,
}

/// `WP_Scripts` of the footer of a page in the preview. The queue is filled in WordPress' order: what
/// `customize_preview_init()` enqueues, the theme's own scripts, the media widgets, the components, and what the
/// templates enqueued while printing.
pub struct FooterScripts {
    scripts: Scripts,
}

impl FooterScripts {
    /// The registry of the WordPress files, with the scripts the head printed marked as done.
    pub fn new(site: &Site, wp_root: &Path, theme_scripts: &[ThemeScript<'_>], printed: &[&str]) -> Self {
        let env = crate::plugin_assets::loader_env(site, wp_root).in_language_of(site);
        let mut scripts = crate::script_loader::registry::default_scripts(&env);
        for script in theme_scripts {
            scripts.add(script.handle, Some(&script.src), script.deps, Ver::Value(script.ver.to_owned()), Some(Arg::Int(1)));
            if let Some(strategy) = script.strategy {
                scripts.add_data(script.handle, "strategy", Extra::Text(strategy.to_owned()));
            }
        }
        // `wp_default_scripts()` adds the texts of the players on `init`.
        scripts.add_inline_script("mediaelement-core", &format!("var mejsL10n = {};", crate::document::mejs_l10n(site, wp_root)), "before");
        for handle in printed {
            scripts.deps.done.push((*handle).to_owned());
        }
        script_translations(site, wp_root, &mut scripts);
        Self { scripts }
    }

    pub fn enqueue(&mut self, handle: &str) {
        self.scripts.enqueue(handle);
    }

    /// `wp_localize_script()`.
    pub fn localize(&mut self, handle: &str, object_name: &str, l10n: &serde_json::Value) {
        self.scripts.localize(handle, object_name, l10n);
    }

    /// `print_footer_scripts()` of what is queued so far.
    pub fn print(&mut self) -> String {
        self.scripts.print_footer_scripts(false)
    }

    /// Whether a handle was printed (by this or as done).
    pub fn is_done(&self, handle: &str) -> bool {
        self.scripts.deps.done.iter().any(|h| h == handle)
    }

}

/// `load_script_textdomain()` for the scripts of WordPress.
fn script_translations(site: &Site, wp_root: &Path, scripts: &mut Scripts) {
    if site.locale() == "en_US" {
        return;
    }
    let languages = wp_root.join("wp-content").join("languages");
    let locale = site.locale();
    let sources: std::collections::HashMap<String, String> = scripts.deps.registered().iter().filter_map(|dep| Some((dep.handle.clone(), dep.src.clone()?))).collect();
    scripts.set_translation_loader(Box::new(move |handle, domain, _path| {
        let relative = sources.get(handle)?.strip_prefix('/')?;
        // The files are named after the script that is not minified.
        let relative = relative.strip_suffix(".min.js").map_or_else(|| relative.to_owned(), |base| format!("{base}.js"));
        let base = if domain == "default" { locale.clone() } else { format!("{domain}-{locale}") };
        rustpress_core::files::read(&languages.join(format!("{base}-{:x}.json", md5::compute(relative.as_bytes())))).map(|text| text.to_string())
    }));
}

/// `WP_Customize_Selective_Refresh::handle_render_partials_request()`: the `data` of the JSON response, without the
/// validities of the settings, which the server adds. A block theme has no partials of its own.
pub async fn render_partials(
    theme: &crate::Theme,
    site: &Site,
    q: &rustpress_core::QueryResult,
    registry: &rustpress_blocks::Registry,
    partials: &[(String, serde_json::Value)],
) -> crate::Result<serde_json::Map<String, serde_json::Value>> {
    let (contents, header) = if crate::classic::is_ported(theme) {
        crate::classic::render_partials(theme, site, q, registry, partials).await?
    } else {
        (partials.iter().map(|(id, _)| (id.clone(), serde_json::Value::Null)).collect(), None)
    };
    let mut data = serde_json::Map::new();
    data.insert("contents".into(), serde_json::Value::Object(contents));
    // `WP_DEBUG_DISPLAY` is on unless the configuration turns it off.
    data.insert("errors".into(), serde_json::Value::Array(Vec::new()));
    if let Some(header) = header {
        data.insert("custom_header_settings".into(), header);
    }
    Ok(data)
}

/// `WP_Customize_Nav_Menus::export_preview_data()` on `wp_footer` at priority 1.
pub fn nav_menus_exports(preview: &Preview) -> String {
    let rendered = preview.rendered();
    let args = if rendered.nav_menu_args.is_empty() {
        serde_json::Value::Array(Vec::new())
    } else {
        serde_json::Value::Object(rendered.nav_menu_args.into_iter().collect())
    };
    let exports = serde_json::json!({ "navMenuInstanceArgs": args });
    format!(
        "<script>\nvar _wpCustomizePreviewNavMenusExports = {};\n//# sourceURL=WP_Customize_Nav_Menus%3A%3Aexport_preview_data\n</script>\n",
        json_hex_tag(&exports)
    )
}

/// `wp_json_encode( $data, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )`.
pub fn json_hex_tag(value: &serde_json::Value) -> String {
    rustpress_blocks::php_json::encode(value).replace("\\/", "/").replace('<', "\\u003C").replace('>', "\\u003E")
}
