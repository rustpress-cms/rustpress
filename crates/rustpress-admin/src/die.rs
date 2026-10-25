//! `wp_die()`: the plain error page of WordPress.

use rustpress_core::Site;
use rustpress_core::formatting::esc_attr;

const STYLE: &str = include_str!("die_style.css");

/// `wp_robots_no_robots()` as `wp_die()` prints it.
fn robots(site: &Site) -> String {
    let public = site.options.get("blog_public").is_some_and(|v| v != "0");
    if public { "max-image-preview:large, noindex, follow".to_owned() } else { "noindex, nofollow".to_owned() }
}

/// The page `wp_die( $message, $title )` sends. `message` is HTML as given.
pub fn page(site: &Site, title: &str, message: &str) -> String {
    let rtl = site.is_rtl();
    let lang = site.language_tag();
    let lang_attrs = if rtl { format!("dir=\"rtl\" lang=\"{}\"", esc_attr(&lang)) } else { format!("lang=\"{}\"", esc_attr(&lang)) };
    format!(
        "<!DOCTYPE html>\n<html {lang_attrs}>\n<head>\n\t<meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\" />\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\t\t<meta name='robots' content='{}' />\n\t<title>{title}</title>\n\t<style>\n{STYLE}</style>\n</head>\n<body id=\"error-page\">\n\t<div class=\"wp-die-message\">{message}</div></body>\n</html>\n\t",
        robots(site)
    )
}
