//! Native modules for WordPress plugins (see [`rustpress_core::native`]). Each module follows one release of its
//! plugin and is compared against the real plugin in the reference WordPress.

pub mod svg_support;
pub mod wp_date_remover;

/// Makes every module known to [`rustpress_core::native`]; call once at startup.
pub fn register() {
    rustpress_core::native::register(vec![&svg_support::SvgSupport, &wp_date_remover::WpDateRemover]);
}
