//! The installer of a new site: `wp-admin/setup-config.php` (the database settings, written to `wp-config.php`)
//! and `wp-admin/install.php` (title and first user, then `wp_install()`).

pub mod brand;
pub mod fetch;
pub mod populate;

use std::path::{Path, PathBuf};

use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::options::Options;

use crate::login::LoginResponse;
use crate::registry::{LoaderEnv, default_scripts};

pub use populate::{InstallData, Installed};

/// What both pages know about the site that is being set up.
pub struct InstallEnv {
    pub wp_root: PathBuf,
    /// `wp_guess_url()`, without a trailing slash.
    pub url: String,
    /// `$wp_version` of the WordPress files.
    pub version: String,
    /// `$wp_local_package`: the language of a localized download of WordPress.
    pub local_package: Option<String>,
}

/// A request to one of the pages.
pub struct InstallRequest<'a> {
    pub post: bool,
    pub query: &'a [(String, String)],
    pub form: &'a [(String, String)],
}

impl InstallRequest<'_> {
    fn get(&self, name: &str) -> Option<&str> {
        self.query.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    fn post_field(&self, name: &str) -> Option<&str> {
        if !self.post {
            return None;
        }
        self.form.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// `$_REQUEST`: the form wins over the query string.
    fn request(&self, name: &str) -> Option<&str> {
        self.post_field(name).or_else(|| self.get(name))
    }

    /// `(int) $_GET['step']`.
    fn step(&self, default: i64) -> i64 {
        self.get("step").map_or(default, |v| {
            let digits: String = v.trim_start().chars().enumerate().take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && *c == '-')).map(|(_, c)| c).collect();
            digits.parse().unwrap_or(0)
        })
    }
}

/// `nocache_headers()` and the type of the installer's pages.
fn headers(charset: &str) -> Vec<(String, String)> {
    vec![
        ("Expires".into(), "Wed, 11 Jan 1984 05:00:00 GMT".into()),
        ("Cache-Control".into(), "no-cache, must-revalidate, max-age=0, no-store, private".into()),
        ("Content-Type".into(), format!("text/html; charset={charset}")),
    ]
}

fn response(status: u16, body: String) -> LoginResponse {
    LoginResponse { status, headers: headers("utf-8"), cookies: Vec::new(), body, bytes: Vec::new() }
}

/// `wp_die()` of the installer.
fn die(site: &Site, status: u16, message: &str) -> LoginResponse {
    let body = crate::die::page_with_robots(site, &site.tr("WordPress &rsaquo; Error"), message, "noindex, nofollow");
    LoginResponse { status, headers: headers("UTF-8"), cookies: Vec::new(), body, bytes: Vec::new() }
}

/// The redirect of `wp-load.php` (no `wp-config.php`) and of `wp_not_installed()` (no site in the database).
pub fn redirect(location: &str, by_wordpress: bool) -> LoginResponse {
    let mut headers = if by_wordpress { headers("UTF-8")[..2].to_vec() } else { Vec::new() };
    if by_wordpress {
        headers.push(("X-Redirect-By".into(), "WordPress".into()));
    }
    headers.push(("Location".into(), location.to_owned()));
    headers.push(("Content-Type".into(), "text/html; charset=UTF-8".into()));
    LoginResponse { status: 302, headers, cookies: Vec::new(), body: String::new(), bytes: Vec::new() }
}

/// `get_available_languages()`: the languages whose translations of WordPress are installed.
pub fn available_languages(wp_root: &Path) -> Vec<String> {
    let dir = wp_root.join("wp-content").join("languages");
    let mut languages: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.strip_suffix(".mo").map(str::to_owned))
        .filter(|name| !name.starts_with("continents-cities") && !name.starts_with("ms-") && !name.starts_with("admin-"))
        .collect();
    languages.sort();
    languages
}

/// `preg_replace( '/[^a-zA-Z0-9_]/', '', $language )` and `sanitize_locale_name()`.
fn clean_language(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect()
}

impl InstallEnv {
    /// A site that only speaks: the options of a new site in `locale`, with its translations when installed.
    pub fn site(&self, locale: &str) -> Site {
        let locale = if locale.is_empty() { "en_US" } else { locale };
        let options = Options::from_pairs([("WPLANG", locale), ("siteurl", self.url.as_str()), ("home", self.url.as_str()), ("blog_public", "0")]);
        let db = rustpress_db::Db::connect_lazy("mysql://nobody@127.0.0.1:1/none", "wp_").expect("a lazy pool needs no server");
        let site = Site::from_parts(db, options);
        site.init_i18n(&self.wp_root, &[]);
        site
    }

    fn loader(&self, site: &Site) -> LoaderEnv {
        LoaderEnv {
            includes: self.wp_root.join("wp-includes"),
            site_url: self.url.clone(),
            content_url: format!("{}/wp-content", self.url),
            version: self.version.clone(),
            rtl: site.is_rtl(),
            separate_block_assets: false,
            compressed_tinymce: false,
            nonce: None,
        }
    }

    /// `wp_admin_css( 'install', true )` and its dependencies.
    fn styles(&self) -> String {
        let mut out = String::new();
        for (id, path) in [
            ("dashicons", "wp-includes/css/dashicons.min.css"),
            ("buttons", "wp-includes/css/buttons.min.css"),
            ("forms", "wp-admin/css/forms.min.css"),
            ("l10n", "wp-admin/css/l10n.min.css"),
            ("wp-base-styles", "wp-includes/css/dist/base-styles/admin-schemes.min.css"),
            ("install", "wp-admin/css/install.min.css"),
        ] {
            out.push_str(&format!("<link rel='stylesheet' id='{id}-css' href='{}/{path}?ver={}' media='all' />\n", self.url, self.version));
        }
        out
    }

    fn lang_attrs(site: &Site) -> String {
        let lang = esc_attr(&site.language_tag());
        if site.is_rtl() { format!("dir=\"rtl\" lang=\"{lang}\"") } else { format!("lang=\"{lang}\"") }
    }

    /// `wp_print_scripts()` calls that share one queue.
    fn scripts(&self, site: &Site, batches: &[&[&str]]) -> Vec<String> {
        let mut scripts = default_scripts(&self.loader(site));
        batches
            .iter()
            .map(|handles| {
                for handle in *handles {
                    scripts.enqueue(handle);
                }
                scripts.print_footer_scripts(false)
            })
            .collect()
    }

    // ------------------------------------------------------------------ setup-config.php

    /// `setup_config_display_header()`.
    fn setup_header(&self, site: &Site, body_classes: &[&str]) -> String {
        let mut classes: Vec<&str> = body_classes.to_vec();
        classes.extend(["wp-core-ui", "admin-color-modern"]);
        if site.is_rtl() {
            classes.push("rtl");
        }
        format!(
            "<!DOCTYPE html>\n<html {}>\n<head>\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n\t<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" />\n\t<meta name=\"robots\" content=\"noindex,nofollow\" />\n\t<title>{}</title>\n\t{}</head>\n<body class=\"{}\">\n<p id=\"logo\">{}</p>\n\t",
            Self::lang_attrs(site),
            site.tr("WordPress &rsaquo; Setup Configuration File"),
            self.styles(),
            classes.join(" "),
            site.tr("WordPress")
        )
    }

    /// `wp-admin/setup-config.php` of a site without `wp-config.php`. `write_config` saves the file and says
    /// whether it could.
    pub async fn setup_config(&self, request: &InstallRequest<'_>, write_config: impl FnOnce(&str) -> bool) -> LoginResponse {
        let english = self.site("en_US");
        let Ok(sample) = std::fs::read_to_string(self.wp_root.join("wp-config-sample.php")) else {
            let message = sprintf(&english.tr("Sorry, I need a %s file to work from. Please re-upload this file to your WordPress installation."), &["<code>wp-config-sample.php</code>"]);
            return die(&english, 500, &message);
        };
        let language = match request.request("language").filter(|l| !l.is_empty()) {
            Some(raw) => clean_language(raw),
            None => self.local_package.clone().unwrap_or_default(),
        };
        let available = available_languages(&self.wp_root);
        match request.step(-1) {
            -1 | 0 => {
                // The language chooser needs the translations API; without it WordPress goes on in English.
                let loaded = (!language.is_empty() && available.contains(&language)).then(|| language.clone());
                let site = self.site(loaded.as_deref().unwrap_or("en_US"));
                let mut step_1 = "setup-config.php?step=1".to_owned();
                if request.request("noapi").is_some() {
                    step_1.push_str("&amp;noapi");
                }
                if let Some(loaded) = &loaded {
                    step_1.push_str(&format!("&amp;language={loaded}"));
                }
                let tr = |t: &str| site.tr(t);
                let mut body = self.setup_header(&site, &[]);
                body.push_str(&format!(
                    "<h1 class=\"screen-reader-text\">\n\t\t{}</h1>\n<p>{}</p>\n<ol>\n\t<li>{}</li>\n\t<li>{}</li>\n\t<li>{}</li>\n\t<li>{}</li>\n\t<li>{}</li>\n</ol>\n<p>\n\t\t{}\t<strong>\n\t\t{}\t</strong>\n\t\t{}</p>\n<p>{}</p>\n\n<p class=\"step\"><a href=\"{step_1}\" class=\"button button-large\">{}</a></p>\n\t\t",
                    tr("Before getting started"),
                    tr("Welcome to WordPress. Before getting started, you will need to know the following items."),
                    tr("Database name"),
                    tr("Database username"),
                    tr("Database password"),
                    tr("Database host"),
                    tr("Table prefix (if you want to run more than one WordPress in a single database)"),
                    sprintf(&tr("This information is being used to create a %s file."), &["<code>wp-config.php</code>"]),
                    sprintf(
                        &tr("If for any reason this automatic file creation does not work, do not worry. All this does is fill in the database information to a configuration file. You may also simply open %1$s in a text editor, fill in your information, and save it as %2$s."),
                        &["<code>wp-config-sample.php</code>", "<code>wp-config.php</code>"]
                    ),
                    sprintf(&tr("Need more help? <a href=\"%1$s\">Read the support article on %2$s</a>."), &[&tr("https://developer.wordpress.org/advanced-administration/wordpress/wp-config/"), "<code>wp-config.php</code>"]),
                    tr("In all likelihood, these items were supplied to you by your web host. If you do not have this information, then you will need to contact them before you can continue. If you are ready&hellip;"),
                    tr("Let&#8217;s go!"),
                ));
                body.push_str(&self.scripts(&site, &[&["language-chooser"]]).concat());
                body.push_str("</body>\n</html>\n");
                response(200, body)
            }
            1 => {
                let site = self.site(&language);
                let tr = |t: &str| site.tr(t);
                let mut body = self.setup_header(&site, &[]);
                let noapi = if request.get("noapi").is_some() { "<input name=\"noapi\" type=\"hidden\" value=\"1\" />" } else { "" };
                body.push_str(&format!(
                    "<h1 class=\"screen-reader-text\">\n\t\t{}</h1>\n<form method=\"post\" action=\"setup-config.php?step=2\">\n\t<p>{}</p>\n\t<table class=\"form-table\" role=\"presentation\">\n\t\t<tr>\n\t\t\t<th scope=\"row\"><label for=\"dbname\">{}</label></th>\n\t\t\t<td><input name=\"dbname\" id=\"dbname\" type=\"text\" aria-describedby=\"dbname-desc\" size=\"25\" placeholder=\"wordpress\" autofocus />\n\t\t\t<p id=\"dbname-desc\">{}</p></td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<th scope=\"row\"><label for=\"uname\">{}</label></th>\n\t\t\t<td><input name=\"uname\" id=\"uname\" type=\"text\" aria-describedby=\"uname-desc\" size=\"25\" placeholder=\"{}\" />\n\t\t\t<p id=\"uname-desc\">{}</p></td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<th scope=\"row\"><label for=\"pwd\">{}</label></th>\n\t\t\t<td>\n\t\t\t\t<div class=\"wp-pwd\">\n\t\t\t\t\t<input name=\"pwd\" id=\"pwd\" type=\"password\" class=\"regular-text\" data-reveal=\"1\" aria-describedby=\"pwd-desc\" size=\"25\" placeholder=\"{}\" autocomplete=\"off\" spellcheck=\"false\" />\n\t\t\t\t\t<button type=\"button\" class=\"button wp-hide-pw user-new-password-toggle pwd-toggle hide-if-no-js\" data-toggle=\"0\" data-start-masked=\"1\" aria-label=\"{}\">\n\t\t\t\t\t\t<span class=\"dashicons dashicons-visibility\"></span>\n\t\t\t\t\t\t<span class=\"text\">{}</span>\n\t\t\t\t\t</button>\n\t\t\t\t</div>\n\t\t\t\t<p id=\"pwd-desc\">{}</p>\n\t\t\t</td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<th scope=\"row\"><label for=\"dbhost\">{}</label></th>\n\t\t\t<td><input name=\"dbhost\" id=\"dbhost\" type=\"text\" aria-describedby=\"dbhost-desc\" size=\"25\" value=\"localhost\" />\n\t\t\t<p id=\"dbhost-desc\">\n\t\t\t{}\t\t\t</p></td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<th scope=\"row\"><label for=\"prefix\">{}</label></th>\n\t\t\t<td><input name=\"prefix\" id=\"prefix\" type=\"text\" aria-describedby=\"prefix-desc\" value=\"wp_\" size=\"25\" />\n\t\t\t<p id=\"prefix-desc\">{}</p></td>\n\t\t</tr>\n\t</table>\n\t\t{noapi}\t<input type=\"hidden\" name=\"language\" value=\"{}\" />\n\t<p class=\"step\"><input name=\"submit\" type=\"submit\" value=\"{}\" class=\"button button-large\" /></p>\n</form>\n\t\t",
                    tr("Set up your database connection"),
                    tr("Below you should enter your database connection details. If you are not sure about these, contact your host."),
                    tr("Database Name"),
                    tr("The name of the database you want to use with WordPress."),
                    tr("Username"),
                    esc_attr(&site.trx("username", "example username")),
                    tr("Your database username."),
                    tr("Password"),
                    esc_attr(&site.trx("password", "example password")),
                    esc_attr(&tr("Show password")),
                    tr("Show"),
                    tr("Your database password."),
                    tr("Database Host"),
                    sprintf(&tr("You should be able to get this info from your web host, if %s does not work."), &["<code>localhost</code>"]),
                    tr("Table Prefix"),
                    tr("If you want to run multiple WordPress installations in a single database, change this."),
                    esc_attr(&language),
                    esc_attr(&tr("Submit")),
                ));
                body.push_str(&self.scripts(&site, &[&["password-toggle"], &["language-chooser"]]).concat());
                body.push_str("</body>\n</html>\n");
                response(200, body)
            }
            2 => self.setup_config_save(request, &language, &sample, write_config).await,
            _ => {
                let site = self.site(&language);
                let mut body = self.setup_header(&site, &[]);
                body.push_str(&self.scripts(&site, &[&["language-chooser"]]).concat());
                body.push_str("</body>\n</html>\n");
                response(200, body)
            }
        }
    }

    /// Step 2: checks the settings, then writes `wp-config.php` from `wp-config-sample.php`.
    async fn setup_config_save(&self, request: &InstallRequest<'_>, language: &str, sample: &str, write_config: impl FnOnce(&str) -> bool) -> LoginResponse {
        let site = self.site(language);
        let tr = |t: &str| site.tr(t);
        let field = |name: &str| request.post_field(name).unwrap_or("").trim().to_owned();
        let (dbname, uname, pwd, dbhost, prefix) = (field("dbname"), field("uname"), field("pwd"), field("dbhost"), field("prefix"));
        let mut step_1 = "setup-config.php?step=1".to_owned();
        let mut install = "install.php".to_owned();
        if request.request("noapi").is_some() {
            step_1.push_str("&amp;noapi");
        }
        if language.is_empty() {
            install.push_str("?language=en_US");
        } else {
            step_1.push_str(&format!("&amp;language={language}"));
            install.push_str(&format!("?language={language}"));
        }
        let try_again = format!("</p><p class=\"step\"><a href=\"{step_1}\" onclick=\"javascript:history.go(-1);return false;\" class=\"button button-large\">{}</a>", tr("Try Again"));
        if prefix.is_empty() {
            return die(&site, 500, &format!("{}{try_again}", tr("<strong>Error:</strong> \"Table Prefix\" must not be empty.")));
        }
        if !prefix.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return die(&site, 500, &format!("{}{try_again}", tr("<strong>Error:</strong> \"Table Prefix\" can only contain numbers, letters, and underscores.")));
        }
        match rustpress_db::probe(&server_url(&uname, &pwd, &dbhost), &dbname, &prefix).await {
            rustpress_db::Probe::Ready { .. } => {}
            rustpress_db::Probe::NoConnection => {
                let message = format!(
                    "<h1>{}</h1>\n<p>{}</p>\n<ul>\n<li>{}</li>\n<li>{}</li>\n<li>{}</li>\n</ul>\n<p>{}</p>\n{try_again}",
                    tr("Error establishing a database connection"),
                    sprintf(
                        &tr("This either means that the username and password information in your %1$s file is incorrect or that contact with the database server at %2$s could not be established. This could mean your host&#8217;s database server is down."),
                        &["<code>wp-config.php</code>", &format!("<code>{}</code>", esc_html(&dbhost))]
                    ),
                    tr("Are you sure you have the correct username and password?"),
                    tr("Are you sure you have typed the correct hostname?"),
                    tr("Are you sure the database server is running?"),
                    sprintf(&tr("If you are unsure what these terms mean you should probably contact your host. If you still need help you can always visit the <a href=\"%s\">WordPress support forums</a>."), &[&tr("https://wordpress.org/support/forums/")]),
                );
                return die(&site, 500, &message);
            }
            rustpress_db::Probe::NoDatabase => {
                let db = format!("<code>{}</code>", esc_html(&dbname));
                let message = format!(
                    "<h1>{}</h1>\n<p>{}</p>\n<ul>\n<li>{}</li>\n<li>{}</li>\n<li>{}</li>\n</ul>\n<p>{}</p>\n{try_again}",
                    tr("Cannot select database"),
                    sprintf(&tr("The database server could be connected to (which means your username and password is okay) but the %s database could not be selected."), &[&db]),
                    tr("Are you sure it exists?"),
                    sprintf(&tr("Does the user %1$s have permission to use the %2$s database?"), &[&format!("<code>{}</code>", esc_html(&uname)), &db]),
                    sprintf(&tr("On some systems the name of your database is prefixed with your username, so it would be like <code>username_%1$s</code>. Could that be the problem?"), &[&esc_html(&dbname)]),
                    sprintf(&tr("If you do not know how to set up a database you should <strong>contact your host</strong>. If all else fails you may find help at the <a href=\"%s\">WordPress support forums</a>."), &[&tr("https://wordpress.org/support/forums/")]),
                );
                return die(&site, 500, &message);
            }
            rustpress_db::Probe::PrefixParses => return die(&site, 500, &tr("<strong>Error:</strong> \"Table Prefix\" is invalid.")),
        }

        let config = config_from_sample(sample, &dbname, &uname, &pwd, &dbhost, &prefix);
        let mut body = self.setup_header(&site, &[]);
        if write_config(&config) {
            body.push_str(&format!(
                "<h1 class=\"screen-reader-text\">\n\t\t\t\t{}</h1>\n<p>{}</p>\n\n<p class=\"step\"><a href=\"{install}\" class=\"button button-large\">{}</a></p>\n\t\t\t\t",
                tr("Successful database connection"),
                tr("All right, sparky! You&#8217;ve made it through this part of the installation. WordPress can now communicate with your database. If you are ready, time now to&hellip;"),
                tr("Run the installation"),
            ));
        } else {
            let text: String = config.split_inclusive('\n').map(rustpress_core::formatting::esc_html).collect();
            body.push_str(&format!(
                "<p>\n\t\t\t{}</p>\n<p id=\"wp-config-description\">\n\t\t\t{}</p>\n<p class=\"configuration-rules-label\"><label for=\"wp-config\">\n\t\t\t{}\t</label></p>\n<textarea id=\"wp-config\" cols=\"98\" rows=\"15\" class=\"code\" readonly=\"readonly\" aria-describedby=\"wp-config-description\">{text}</textarea>\n<p>{}</p>\n<p class=\"step\"><a href=\"{install}\" class=\"button button-large\">{}</a></p>\n<script>\n(function(){{\nif ( ! /iPad|iPod|iPhone/.test( navigator.userAgent ) ) {{\n\tvar el = document.getElementById('wp-config');\n\tel.focus();\n\tel.select();\n}}\n}})();\n</script>\n\t\t\t",
                sprintf(&tr("Unable to write to %s file."), &["<code>wp-config.php</code>"]),
                sprintf(&tr("You can create the %s file manually and paste the following text into it."), &["<code>wp-config.php</code>"]),
                sprintf(&tr("Configuration rules for %s:"), &["<code>wp-config.php</code>"]),
                tr("After you&#8217;ve done that, click &#8220;Run the installation&#8221;."),
                tr("Run the installation"),
            ));
        }
        body.push_str(&self.scripts(&site, &[&["language-chooser"]]).concat());
        body.push_str("</body>\n</html>\n");
        response(200, body)
    }

    /// `setup-config.php` and `wp-admin/install.php` when `wp-config.php` exists already.
    pub fn config_exists(&self) -> LoginResponse {
        let site = self.site("en_US");
        let message = format!(
            "<p>{}</p>",
            sprintf(
                &site.tr("The file %1$s already exists. If you need to reset any of the configuration items in this file, please delete it first. You may try <a href=\"%2$s\">installing now</a>."),
                &["<code>wp-config.php</code>", "install.php"]
            )
        );
        die(&site, 409, &message)
    }

    // ------------------------------------------------------------------ install.php

    /// `display_header()` of `install.php`.
    fn install_header(&self, site: &Site, body_classes: &str) -> String {
        let mut classes = body_classes.to_owned();
        if site.is_rtl() {
            classes.push_str("rtl");
        }
        if !classes.is_empty() {
            classes.insert(0, ' ');
        }
        format!(
            "<!DOCTYPE html>\n<html {}>\n<head>\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n\t<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" />\n\t<meta name=\"robots\" content=\"noindex,nofollow\" />\n\t<title>{}</title>\n\t{}</head>\n<body class=\"wp-core-ui admin-color-modern{classes}\">\n<p id=\"logo\">{}</p>\n\n\t",
            Self::lang_attrs(site),
            site.tr("WordPress &rsaquo; Installation"),
            self.styles(),
            site.tr("WordPress")
        )
    }

    /// `display_setup_form()`. `user_table`: the users table exists already (it is shared with another site).
    fn setup_form(&self, site: &Site, request: &InstallRequest<'_>, error: Option<&str>, user_table: bool) -> String {
        let tr = |t: &str| site.tr(t);
        let posted = request.post_field("weblog_title").is_some();
        let blog_public = if posted { request.post_field("blog_public").map_or(1, |v| v.trim().parse::<i64>().unwrap_or(0)) } else { 1 };
        let field = |name: &str| request.post_field(name).unwrap_or("").trim().to_owned();
        let (weblog_title, user_name, admin_email) = (field("weblog_title"), field("user_name"), field("admin_email"));
        let mut out = String::new();
        if let Some(error) = error {
            out.push_str(&format!("<h1>{}</h1>\n<p class=\"message\">{error}</p>\n", site.trx("Welcome", "Howdy")));
        }
        out.push_str(&format!(
            "<form id=\"setup\" method=\"post\" action=\"install.php?step=2\" novalidate=\"novalidate\">\n\t<table class=\"form-table\" role=\"presentation\">\n\t\t<tr>\n\t\t\t<th scope=\"row\"><label for=\"weblog_title\">{}</label></th>\n\t\t\t<td><input name=\"weblog_title\" type=\"text\" id=\"weblog_title\" size=\"25\" value=\"{}\" /></td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<th scope=\"row\"><label for=\"user_login\">{}</label></th>\n\t\t\t<td>\n\t\t\t",
            tr("Site Title"),
            esc_attr(&weblog_title),
            tr("Username"),
        ));
        if user_table {
            out.push_str(&tr("User(s) already exists."));
            out.push_str("<input name=\"user_name\" type=\"hidden\" value=\"admin\" />");
        } else {
            out.push_str(&format!(
                "\t\t\t\t<input name=\"user_name\" type=\"text\" id=\"user_login\" size=\"25\" aria-describedby=\"user-name-desc\" value=\"{}\" />\n\t\t\t\t<p id=\"user-name-desc\">{}</p>\n\t\t\t\t",
                esc_attr(&crate::sanitize::user_login(&user_name, true)),
                tr("Usernames can have only alphanumeric characters, spaces, underscores, hyphens, periods, and the @ symbol."),
            ));
        }
        out.push_str("\t\t\t</td>\n\t\t</tr>\n\t\t");
        if !user_table {
            // `stripslashes()` undoes the slashes WordPress adds to `$_POST`; the form here has none.
            let initial = match request.post_field("admin_password") {
                Some(password) => password.to_owned(),
                None => crate::sanitize::random_password(18, true),
            };
            out.push_str(&format!(
                "\t\t<tr class=\"form-field form-required user-pass1-wrap\">\n\t\t\t<th scope=\"row\">\n\t\t\t\t<label for=\"pass1\">\n\t\t\t\t\t{}\t\t\t\t</label>\n\t\t\t</th>\n\t\t\t<td>\n\t\t\t\t<div class=\"wp-pwd\">\n\t\t\t\t\t\t\t\t\t\t<div class=\"password-input-wrapper\">\n\t\t\t\t\t\t<input type=\"password\" name=\"admin_password\" id=\"pass1\" class=\"regular-text\" autocomplete=\"new-password\" spellcheck=\"false\" data-reveal=\"1\" data-pw=\"{}\" aria-describedby=\"pass-strength-result admin-password-desc\" />\n\t\t\t\t\t\t<div id=\"pass-strength-result\" aria-live=\"polite\"></div>\n\t\t\t\t\t</div>\n\t\t\t\t\t<button type=\"button\" class=\"button wp-hide-pw user-new-password-toggle hide-if-no-js\" data-start-masked=\"{}\" data-toggle=\"0\" aria-label=\"{}\">\n\t\t\t\t\t\t<span class=\"dashicons dashicons-hidden\"></span>\n\t\t\t\t\t\t<span class=\"text\">{}</span>\n\t\t\t\t\t</button>\n\t\t\t\t</div>\n\t\t\t\t<p id=\"admin-password-desc\"><span class=\"description important hide-if-no-js\">\n\t\t\t\t<strong>{}</strong>\n\t\t\t\t\t\t\t\t{}</span></p>\n\t\t\t</td>\n\t\t</tr>\n\t\t<tr class=\"form-field form-required user-pass2-wrap hide-if-js\">\n\t\t\t<th scope=\"row\">\n\t\t\t\t<label for=\"pass2\">{}\t\t\t\t\t<span class=\"description\">{}</span>\n\t\t\t\t</label>\n\t\t\t</th>\n\t\t\t<td>\n\t\t\t\t<input type=\"password\" name=\"admin_password2\" id=\"pass2\" autocomplete=\"new-password\" spellcheck=\"false\" />\n\t\t\t</td>\n\t\t</tr>\n\t\t<tr class=\"pw-weak\">\n\t\t\t<th scope=\"row\">{}</th>\n\t\t\t<td>\n\t\t\t\t<label>\n\t\t\t\t\t<input type=\"checkbox\" name=\"pw_weak\" class=\"pw-checkbox\" />\n\t\t\t\t\t{}\t\t\t\t</label>\n\t\t\t</td>\n\t\t</tr>\n\t\t",
                tr("Password"),
                esc_attr(&initial),
                i64::from(request.post_field("admin_password").is_some()),
                esc_attr(&tr("Hide password")),
                tr("Hide"),
                tr("Important:"),
                tr("You will need this password to log&nbsp;in. Please store it in a secure location."),
                tr("Repeat Password"),
                tr("(required)"),
                tr("Confirm Password"),
                tr("Confirm use of weak password"),
            ));
        }
        let visibility = tr("Search engine visibility");
        out.push_str(&format!(
            "\t\t<tr>\n\t\t\t<th scope=\"row\"><label for=\"admin_email\">{}</label></th>\n\t\t\t<td><input name=\"admin_email\" type=\"email\" id=\"admin_email\" size=\"25\" aria-describedby=\"admin-email-desc\" value=\"{}\" />\n\t\t\t<p id=\"admin-email-desc\">{}</p></td>\n\t\t</tr>\n\t\t\t\t<tr>\n\t\t\t<th scope=\"row\">{visibility}</th>\n\t\t\t<td>\n\t\t\t\t<fieldset>\n\t\t\t\t\t<legend class=\"screen-reader-text\"><span>{visibility}</span></legend>\n\t\t\t\t\t\t\t\t\t\t\t<label for=\"blog_public\"><input name=\"blog_public\" type=\"checkbox\" id=\"blog_public\" aria-describedby=\"privacy-desc\" value=\"0\" {} />\n\t\t\t\t\t\t{}</label>\n\t\t\t\t\t\t<p id=\"privacy-desc\" class=\"description\">{}</p>\n\t\t\t\t\t\t\t\t\t</fieldset>\n\t\t\t</td>\n\t\t</tr>\n\t</table>\n\t<p class=\"step\"><input type=\"submit\" name=\"Submit\" id=\"submit\" class=\"button button-large\" value=\"{}\"  /></p>\n\t<input type=\"hidden\" name=\"language\" value=\"{}\" />\n</form>\n\t",
            tr("Your Email"),
            esc_attr(&admin_email),
            tr("Double-check your email address before continuing."),
            if blog_public == 0 { " checked='checked'" } else { "" },
            tr("Discourage search engines from indexing this site"),
            tr("It is up to search engines to honor this request."),
            esc_attr(&tr("Install WordPress")),
            esc_attr(request.request("language").unwrap_or("")),
        ));
        out
    }

    /// The end of every page of `install.php`.
    fn install_footer(&self, site: &Site) -> String {
        let scripts = self.scripts(site, &[&["jquery", "user-profile"]]).concat();
        format!(
            "<script>var t = document.getElementById('weblog_title'); if (t){{ t.focus(); }}</script>\n\t{scripts}<script>\njQuery( function( $ ) {{\n\t$( '.hide-if-no-js' ).removeClass( 'hide-if-no-js' );\n}} );\n</script>\n</body>\n</html>\n"
        )
    }

    /// `install.php` of a site that is configured but has no tables yet. `writer` reaches its database;
    /// `server_version` is `wpdb::db_version()`.
    pub async fn install(&self, request: &InstallRequest<'_>, writer: &rustpress_db::Writer, server_version: &str, now: i64) -> LoginResponse {
        let english = self.site("en_US");
        let installed = writer.column_string(&format!("SELECT option_value FROM {} WHERE option_name = 'siteurl' LIMIT 1", writer.table("options"))).await.is_ok_and(|rows| !rows.is_empty());
        if installed {
            let mut body = self.install_header(&english, "");
            body.push_str(&format!(
                "<h1>{}</h1><p>{}</p><p class=\"step\"><a href=\"{}\">{}</a></p></body></html>",
                english.tr("Already Installed"),
                english.tr("You appear to have already installed WordPress. To reinstall please clear your old database tables first."),
                esc_url(&format!("{}/wp-login.php", self.url)),
                english.tr("Log In")
            ));
            return response(200, body);
        }
        let language = match request.request("language").filter(|l| !l.is_empty()) {
            Some(raw) => clean_language(raw),
            None => self.local_package.clone().unwrap_or_default(),
        };
        let available = available_languages(&self.wp_root);
        let installed_language = (!language.is_empty() && available.contains(&language)).then(|| language.clone());
        let user_table = writer.column_string(&format!("SHOW TABLES LIKE '{}users'", writer.table_prefix().replace('_', "\\_"))).await.is_ok_and(|rows| !rows.is_empty());
        match request.step(0) {
            2 => {
                let loaded = installed_language.unwrap_or_else(|| "en_US".to_owned());
                let site = self.site(&loaded);
                let tr = |t: &str| site.tr(t);
                let mut body = self.install_header(&site, "");
                let field = |name: &str| request.post_field(name).unwrap_or("").to_owned();
                let weblog_title = field("weblog_title").trim().to_owned();
                let user_name = field("user_name").trim().to_owned();
                let admin_password = field("admin_password");
                let admin_password_check = field("admin_password2");
                let admin_email = field("admin_email").trim().to_owned();
                let public = request.post_field("blog_public").map_or(1, |v| v.trim().parse::<i64>().unwrap_or(0));
                let error = if user_name.is_empty() {
                    Some(tr("Please provide a valid username."))
                } else if crate::sanitize::user_login(&user_name, true) != user_name {
                    Some(tr("The username you provided has invalid characters."))
                } else if admin_password != admin_password_check {
                    Some(tr("Your passwords do not match. Please try again."))
                } else if admin_email.is_empty() {
                    Some(tr("You must provide an email address."))
                } else if !crate::sanitize::is_email(&admin_email) {
                    Some(tr("Sorry, that is not a valid email address. Email addresses look like <code>username@example.com</code>."))
                } else {
                    None
                };
                if let Some(error) = error {
                    body.push_str(&self.setup_form(&site, request, Some(&error), user_table));
                    body.push_str(&self.install_footer(&site));
                    return response(200, body);
                }
                let data = InstallData { blog_title: &weblog_title, user_name: &user_name, user_email: &admin_email, public, user_password: &admin_password, language: &loaded, url: &self.url };
                let result = match populate::wp_install(writer, &site, &self.wp_root, server_version, &data, now).await {
                    Ok(result) => result,
                    Err(e) => return die(&site, 500, &format!("<p>{}</p>", esc_html(&e.to_string()))),
                };
                let shown = if !result.password.is_empty() && admin_password_check.is_empty() { format!("<code>{}</code><br />\n\t\t\t\t", esc_html(&result.password)) } else { String::new() };
                body.push_str(&format!(
                    "\n<h1>{}</h1>\n\n<p>{}</p>\n\n<table class=\"form-table install-success\">\n\t<tr>\n\t\t<th>{}</th>\n\t\t<td>{}</td>\n\t</tr>\n\t<tr>\n\t\t<th>{}</th>\n\t\t<td>\n\t\t\t\t\t\t{shown}<p>{}</p>\n\t\t</td>\n\t</tr>\n</table>\n\n<p class=\"step\"><a href=\"{}\">{}</a></p>\n\n\t\t\t",
                    tr("Success!"),
                    tr("WordPress has been installed. Thank you, and enjoy!"),
                    tr("Username"),
                    esc_html(&crate::sanitize::user_login(&user_name, true)),
                    tr("Password"),
                    result.password_message,
                    esc_url(&format!("{}/wp-login.php", self.url)),
                    tr("Log In"),
                ));
                body.push_str(&self.install_footer(&site));
                response(200, body)
            }
            0 | 1 => {
                let site = self.site(installed_language.as_deref().unwrap_or("en_US"));
                let tr = |t: &str| site.tr(t);
                let mut body = self.install_header(&site, "");
                body.push_str(&format!(
                    "<h1>{}</h1>\n<p>{}</p>\n\n<h2>{}</h2>\n<p>{}</p>\n\n\t\t",
                    site.trx("Welcome", "Howdy"),
                    tr("Welcome to the famous five-minute WordPress installation process! Just fill in the information below and you&#8217;ll be on your way to using the most extendable and powerful personal publishing platform in the world."),
                    tr("Information needed"),
                    tr("Please provide the following information. Do not worry, you can always change these settings later."),
                ));
                body.push_str(&self.setup_form(&site, request, None, user_table));
                body.push_str(&self.install_footer(&site));
                response(200, body)
            }
            _ => {
                let site = self.site("en_US");
                let mut body = self.install_header(&site, "");
                body.push_str(&self.install_footer(&site));
                response(200, body)
            }
        }
    }
}

/// A `mysql://` URL for `DB_HOST` (`host`, `host:port`, `host:/socket`) without a database.
pub fn server_url(user: &str, password: &str, host: &str) -> String {
    let host = if host.is_empty() { "localhost" } else { host };
    let (host, socket) = match host.split_once(':') {
        Some((host, socket)) if socket.starts_with('/') => (host, Some(socket)),
        _ => (host, None),
    };
    let mut url = format!("mysql://{}:{}@{}/?ssl-mode=disabled", encode(user), encode(password), host);
    if let Some(socket) = socket {
        url.push_str("&socket=");
        url.push_str(&encode(socket));
    }
    url
}

/// Percent-encoding for the parts of a URL.
fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `addcslashes( $value, "\\'" )`.
fn php_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

/// `wp-config.php` as `setup-config.php` writes it: the sample with the database settings, the table prefix and
/// eight new keys and salts.
pub fn config_from_sample(sample: &str, dbname: &str, uname: &str, pwd: &str, dbhost: &str, prefix: &str) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_ []{}<>~`+=,.;:/?|";
    let mut rng = rand::rng();
    let mut key = || -> String { (0..64).map(|_| CHARS[rng.random_range(0..CHARS.len())] as char).collect() };
    let mut out = String::new();
    for line in sample.split_inclusive('\n') {
        if line.starts_with("$table_prefix =") {
            out.push_str(&format!("$table_prefix = '{}';\r\n", php_quote(prefix)));
            continue;
        }
        let constant = line.strip_prefix("define(").and_then(|rest| {
            let rest = rest.trim_start_matches([' ', '\t']).strip_prefix('\'')?;
            let (name, after) = rest.split_once('\'')?;
            let padding = after.strip_prefix(',')?;
            let spaces = padding.len() - padding.trim_start_matches(' ').len();
            (spaces > 0 && name.chars().all(|c| c.is_ascii_uppercase() || c == '_')).then(|| (name.to_owned(), " ".repeat(spaces)))
        });
        let Some((name, padding)) = constant else {
            out.push_str(line);
            continue;
        };
        let value = match name.as_str() {
            "DB_NAME" => Some(php_quote(dbname)),
            "DB_USER" => Some(php_quote(uname)),
            "DB_PASSWORD" => Some(php_quote(pwd)),
            "DB_HOST" => Some(php_quote(dbhost)),
            "DB_CHARSET" => Some("utf8mb4".to_owned()),
            "AUTH_KEY" | "SECURE_AUTH_KEY" | "LOGGED_IN_KEY" | "NONCE_KEY" | "AUTH_SALT" | "SECURE_AUTH_SALT" | "LOGGED_IN_SALT" | "NONCE_SALT" => Some(key()),
            _ => None,
        };
        match value {
            Some(value) => out.push_str(&format!("define( '{name}',{padding}'{value}' );\r\n")),
            None => out.push_str(line),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_fills_in_the_sample() {
        let sample = "<?php\ndefine( 'DB_NAME', 'database_name_here' );\ndefine( 'DB_PASSWORD', 'password_here' );\ndefine( 'DB_CHARSET', 'utf8' );\ndefine( 'AUTH_KEY',         'put your unique phrase here' );\ndefine( 'WP_DEBUG', false );\n$table_prefix = 'wp_';\n";
        let config = config_from_sample(sample, "db1", "u", "it's", "localhost", "x_");
        let lines: Vec<&str> = config.split('\n').collect();
        assert_eq!(lines[1], "define( 'DB_NAME', 'db1' );\r");
        assert_eq!(lines[2], "define( 'DB_PASSWORD', 'it\\'s' );\r");
        assert_eq!(lines[3], "define( 'DB_CHARSET', 'utf8mb4' );\r");
        assert!(lines[4].starts_with("define( 'AUTH_KEY',         '") && lines[4].len() == "define( 'AUTH_KEY',         '' );\r".len() + 64);
        assert_eq!(lines[5], "define( 'WP_DEBUG', false );");
        assert_eq!(lines[6], "$table_prefix = 'x_';\r");
    }

    #[test]
    fn server_urls() {
        assert_eq!(server_url("u", "p@ss", "db.example:3307"), "mysql://u:p%40ss@db.example:3307/?ssl-mode=disabled");
        assert_eq!(server_url("u", "", "localhost:/tmp/my.sock"), "mysql://u:@localhost/?ssl-mode=disabled&socket=%2Ftmp%2Fmy.sock");
    }
}
