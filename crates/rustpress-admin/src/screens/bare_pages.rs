//! The files of `wp-admin` that are not screens: the installer and updater pages of a site that is installed, the
//! includes that print nothing, and the old redirects.

use rustpress_core::formatting::esc_attr;

use super::{AdminEnv, AdminResponse, page_headers, redirect};

/// A response with the headers of an admin page and no body.
pub fn empty() -> AdminResponse {
    AdminResponse { status: 200, headers: page_headers(), cookies: Vec::new(), body: String::new(), bytes: Vec::new() }
}

/// The styles of the installer (`wp_admin_css( 'install', true )`).
fn install_styles(env: &AdminEnv<'_>) -> String {
    let siteurl = env.site.settings.siteurl.trim_end_matches('/');
    let version = &env.loader.version;
    let mut out = String::new();
    for (id, path) in [
        ("dashicons", "wp-includes/css/dashicons.min.css"),
        ("buttons", "wp-includes/css/buttons.min.css"),
        ("forms", "wp-admin/css/forms.min.css"),
        ("l10n", "wp-admin/css/l10n.min.css"),
        ("wp-base-styles", "wp-includes/css/dist/base-styles/admin-schemes.min.css"),
        ("install", "wp-admin/css/install.min.css"),
    ] {
        out.push_str(&format!("<link rel='stylesheet' id='{id}-css' href='{siteurl}/{path}?ver={version}' media='all' />\n"));
    }
    out
}

fn lang_attrs(env: &AdminEnv<'_>) -> String {
    let lang = env.site.language_tag();
    if env.site.is_rtl() { format!("dir=\"rtl\" lang=\"{}\"", esc_attr(&lang)) } else { format!("lang=\"{}\"", esc_attr(&lang)) }
}

/// `upgrade.php` of a site whose database is up to date.
pub fn upgrade(env: &AdminEnv<'_>) -> AdminResponse {
    let home = env.site.settings.home.trim_end_matches('/');
    let body = format!(
        "<!DOCTYPE html>\n<html {}>\n<head>\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n\t<meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\" />\n\t<meta name=\"robots\" content=\"noindex,nofollow\" />\n\t<title>{}</title>\n\t{}</head>\n<body class=\"wp-core-ui admin-color-modern\">\n<p id=\"logo\">WordPress</p>\n\n\n<h1>{}</h1>\n<p>{}</p>\n<p class=\"step\"><a class=\"button button-large\" href=\"{home}/\">{}</a></p>\n\n\t</body>\n</html>\n",
        lang_attrs(env),
        env.site.tr("WordPress &rsaquo; Update"),
        install_styles(env),
        env.site.tr("No Update Required"),
        env.site.tr("Your WordPress database is already up to date!"),
        env.site.tr("Continue")
    );
    AdminResponse { status: 200, headers: page_headers(), cookies: Vec::new(), body, bytes: Vec::new() }
}

/// `install.php` of a site that is installed.
pub fn install(env: &AdminEnv<'_>) -> AdminResponse {
    let siteurl = env.site.settings.siteurl.trim_end_matches('/');
    let body = format!(
        "<!DOCTYPE html>\n<html {}>\n<head>\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n\t<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" />\n\t<meta name=\"robots\" content=\"noindex,nofollow\" />\n\t<title>{}</title>\n\t{}</head>\n<body class=\"wp-core-ui admin-color-modern\">\n<p id=\"logo\">WordPress</p>\n\n\t<h1>{}</h1><p>{}</p><p class=\"step\"><a href=\"{siteurl}/wp-login.php\">{}</a></p></body></html>",
        lang_attrs(env),
        env.site.tr("WordPress &rsaquo; Installation"),
        install_styles(env),
        env.site.tr("Already Installed"),
        env.site.tr("You appear to have already installed WordPress. To reinstall please clear your old database tables first."),
        env.site.tr("Log In")
    );
    let mut headers = page_headers();
    for header in headers.iter_mut().filter(|(name, _)| name == "Content-Type") {
        header.1 = "text/html; charset=utf-8".into();
    }
    AdminResponse { status: 200, headers, cookies: Vec::new(), body, bytes: Vec::new() }
}

/// `setup-config.php` of a site that has its `wp-config.php`.
pub fn setup_config(env: &AdminEnv<'_>) -> AdminResponse {
    let message = format!(
        "<p>{}</p>",
        rustpress_core::i18n::sprintf(
            &env.site.tr("The file %1$s already exists. If you need to reset any of the configuration items in this file, please delete it first. You may try <a href='%2$s'>installing now</a>."),
            &["<code>wp-config.php</code>", "install.php"]
        )
        .replace('\'', "\"")
    );
    let body = crate::die::page_with_robots(env.site, &env.site.tr("WordPress &rsaquo; Error"), &message, "noindex, nofollow");
    AdminResponse { status: 409, headers: page_headers(), cookies: Vec::new(), body, bytes: Vec::new() }
}

/// `media.php`: the media editing screen that was removed.
pub fn media(admin_url: &str) -> AdminResponse {
    redirect(&format!("{admin_url}upload.php?error=deprecated"))
}
