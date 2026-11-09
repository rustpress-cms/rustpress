//! The `functions.php` of Extendable 2.2.0.
//!
//! Its stylesheet (`extendable-style`) and `add_theme_support( 'wp-block-styles' )` are literal calls that
//! `functions_php.rs` reads. Not ported, because they need a plugin or a signed-in viewer: the WooCommerce,
//! Contact Form 7 and WPForms styles and filters, the duotone of Extendify demo logos (`render_block_data`, only for
//! logo files named `extendify-demo-*`/`ext-custom-logo-*`), the animations of `inc/animations.php` (only for
//! signed-in viewers or with the option `extendify_animation_settings`), the pattern categories and the editor
//! filters.

use rustpress_core::native::Asset;
use serde_json::json;

use super::{BlockThemePort, ScriptContext};
use crate::script_loader::scripts::Scripts;

pub struct Extendable;

impl BlockThemePort for Extendable {
    fn version(&self) -> &'static str {
        "2.2.0"
    }

    /// `extendable_enqueue_header_scripts()` and `extendable_enqueue_navigation_customizations()`, both in the
    /// footer (`get_bloginfo( 'name' )` is the raw option); the second one has no version (`null`) and the data `ExtendableNavData`.
    fn scripts(&self, ctx: &ScriptContext<'_>) -> Vec<Asset> {
        let header = Asset {
            handle: "extendable-header".to_owned(),
            src: format!("{}/assets/js/header.js", ctx.template_uri),
            ver: Some(ctx.version.clone()),
            in_footer: true,
            priority: 10,
            ..Asset::default()
        };
        let data = json!({ "logoUrl": ctx.custom_logo_url, "siteTitle": ctx.site.settings.blogname });
        let navigation = Asset {
            handle: "extendable-navigation_customizations".to_owned(),
            src: format!("{}/assets/js/navigation-customization.js", ctx.template_uri),
            ver: None,
            in_footer: true,
            priority: 10,
            extra: Some(Scripts::l10n_script("ExtendableNavData", &data)),
            ..Asset::default()
        };
        vec![header, navigation]
    }
}
