//! Native modules for WordPress plugins (see [`rustpress_core::native`]). Each module follows one release of its
//! plugin and is compared against the real plugin in the reference WordPress.

pub mod acf;
pub mod classic_editor;
pub mod complianz;
pub mod contact_form_7;
pub mod disable_comments;
pub mod redirection;
pub mod site_kit;
pub mod svg_support;
pub mod wordpress_seo;
pub mod wp_date_remover;
pub mod wp_mail_smtp;
pub mod wpcode;
pub mod wpforms;

/// Makes every module known to [`rustpress_core::native`]; call once at startup.
pub fn register() {
    rustpress_core::native::register(vec![&acf::Acf, &classic_editor::ClassicEditor, &complianz::Complianz, &contact_form_7::ContactForm7, &disable_comments::DisableComments, &redirection::Redirection, &site_kit::SiteKit, &svg_support::SvgSupport, &wordpress_seo::WordpressSeo, &wp_date_remover::WpDateRemover, &wp_mail_smtp::WpMailSmtp, &wpcode::Wpcode, &wpforms::WpForms]);
}
