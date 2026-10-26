//! The screens of `wp-admin`: request handling, sign-in check and the pages themselves.

use std::sync::Arc;

use rustpress_auth::setcookie::{CookiePaths, SetCookie};
use rustpress_auth::{Auth, CurrentUser, Roles, Scheme};
use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_db::{Db, Writer};

use crate::adminbar::BarContext;
use crate::chrome::{self, Assets, ChromeEnv, Screen};
use crate::login::LoginResponse;
use crate::menu::{AdminMenu, MenuContext};
use crate::registry::LoaderEnv;

pub mod ajax;
pub mod ajax_comments;
mod ajax_legacy;
pub mod ajax_media;
mod ajax_misc;
mod ajax_privacy;
pub mod ajax_terms;
mod bare_pages;
mod file_editor;
mod privacy_requests;
mod site_health_info;
mod attachment_edit;
mod authorize_app;
pub mod block_editor;
pub mod comments;
pub mod media_lib;
pub mod media_ui;
pub mod common;
pub mod dashboard;
pub mod editor;
pub mod export;
pub mod general;
mod image_edit;
mod install;
pub mod notices;
pub mod options;
pub mod pages;
mod permalink;
pub mod post_edit;
pub mod plugins;
mod inline_edit;
mod quick_edit;
pub mod posts;
pub mod privacy;
pub mod settings;
pub mod boot_page;
pub mod revision;
pub mod site_editor;
mod site_health;
pub mod terms;
mod update_core;
pub mod themes;
pub mod user_edit;
pub mod users;
pub mod timezones;

/// The answer of a screen is the one of the sign-in page: status, headers, cookies, body.
pub type AdminResponse = LoginResponse;

/// What the screens need from the site.
pub struct AdminEnv<'a> {
    pub site: &'a Site,
    pub db: &'a Db,
    /// Present when writing is allowed.
    pub writer: Option<&'a Writer>,
    pub auth: &'a Auth,
    pub roles: &'a Roles,
    pub loader: LoaderEnv,
    pub https: bool,
    pub now: i64,
    /// The installed languages of the language drop-down.
    pub languages: Vec<crate::login::LanguageOption>,
    /// The languages `wp_get_available_translations()` knows: `(locale, native name, lang)`.
    pub available_translations: Vec<(String, String, String)>,
    /// Whether the active theme is a block theme (the Appearance menu differs).
    pub block_theme: bool,
    /// The active theme and the block registry (the block editor needs them).
    pub theme: &'a rustpress_theme::Theme,
    pub registry: &'a rustpress_blocks::Registry,
    /// Sends e-mail; without it every message fails (`wp_mail()` on a host that cannot send).
    pub mailer: Option<std::sync::Arc<dyn crate::login::Mail>>,
}

impl AdminEnv<'_> {
    /// What the messages WordPress sends on its own need.
    pub fn mail_ctx(&self) -> crate::mail::MailCtx<'_> {
        crate::mail::MailCtx { site: self.site, db: self.db, writer: self.writer, now: self.now, mailer: self.mailer.as_ref() }
    }

    /// `wp_mail()`: a plain text message; `false` when it could not be sent.
    pub async fn send_mail(&self, to: &str, subject: &str, message: &str) -> bool {
        match &self.mailer {
            Some(mailer) => mailer.send(to, subject, message).await,
            None => false,
        }
    }
}

/// A request to a file in `wp-admin`.
#[derive(Debug, Clone, Default)]
pub struct AdminRequest {
    pub post: bool,
    /// `options-general.php`; `wp-admin/` is `index.php`.
    pub file: String,
    pub query: Vec<(String, String)>,
    pub form: Vec<(String, String)>,
    /// Uploaded files of a multipart form.
    pub files: Vec<crate::multipart::FilePart>,
    pub cookies: Vec<(String, String)>,
    /// `$_SERVER['REQUEST_URI']`: path and query string.
    pub request_uri: String,
    pub remote_addr: Option<String>,
    pub user_agent: Option<String>,
}

impl AdminRequest {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.query.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn post_field(&self, name: &str) -> Option<&str> {
        self.form.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// `$_REQUEST`: the form wins over the query string.
    pub fn request(&self, name: &str) -> Option<&str> {
        self.post_field(name).or_else(|| self.get(name))
    }

    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.cookies.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

/// Everything a screen works with.
pub struct Ctx<'a> {
    pub env: &'a AdminEnv<'a>,
    pub request: &'a AdminRequest,
    pub user: CurrentUser,
    pub token: String,
}

impl Ctx<'_> {
    pub fn site(&self) -> &Site {
        self.env.site
    }

    pub fn tr(&self, text: &str) -> String {
        self.env.site.tr(text)
    }

    pub fn can(&self, capability: &str) -> bool {
        self.user.can(capability)
    }

    pub fn admin_url(&self, path: &str) -> String {
        self.env.site.site_url(&format!("wp-admin/{path}"))
    }

    pub fn create_nonce(&self, action: &str) -> String {
        self.env.auth.create_nonce(action, self.user.id, &self.token, self.env.now).unwrap_or_default()
    }

    /// `check_admin_referer()` / `wp_verify_nonce()` on `_wpnonce`.
    pub fn verify_nonce(&self, action: &str) -> bool {
        let nonce = self.request.request("_wpnonce").unwrap_or_default();
        self.env.auth.verify_nonce(nonce, action, self.user.id, &self.token, self.env.now).is_some()
    }

    /// `wp_nonce_field( $action, $name )`.
    pub fn nonce_field_named(&self, action: &str, name: &str) -> String {
        format!(
            "<input type=\"hidden\" id=\"{name}\" name=\"{name}\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />",
            self.create_nonce(action),
            rustpress_core::formatting::esc_url(&self.request.request_uri)
        )
    }

    /// `wp_nonce_field( $action )`.
    pub fn nonce_field(&self, action: &str) -> String {
        format!(
            "<input type=\"hidden\" id=\"_wpnonce\" name=\"_wpnonce\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />",
            self.create_nonce(action),
            rustpress_core::formatting::esc_url(&self.request.request_uri)
        )
    }

    /// `settings_fields( $group )`.
    pub fn settings_fields(&self, group: &str) -> String {
        format!("<input type='hidden' name='option_page' value='{}' /><input type=\"hidden\" name=\"action\" value=\"update\" />{}", esc_attr(group), self.nonce_field(&format!("{group}-options")))
    }
}

/// `nocache_headers()` and the frame headers of the administration area.
pub fn page_headers() -> Vec<(String, String)> {
    vec![
        ("Expires".into(), "Wed, 11 Jan 1984 05:00:00 GMT".into()),
        ("Cache-Control".into(), "no-cache, must-revalidate, max-age=0, no-store, private".into()),
        ("Referrer-Policy".into(), "strict-origin-when-cross-origin".into()),
        ("X-Frame-Options".into(), "SAMEORIGIN".into()),
        ("Content-Security-Policy".into(), "frame-ancestors 'self';".into()),
        ("Content-Type".into(), "text/html; charset=UTF-8".into()),
    ]
}

fn paths(site: &Site) -> CookiePaths {
    CookiePaths::from_urls(&site.settings.home, &site.settings.siteurl)
}

/// `wp_user_settings()`: the cookies that carry the user's interface settings.
async fn user_settings_cookies(env: &AdminEnv<'_>, request: &AdminRequest, user_id: u64) -> Vec<SetCookie> {
    let key = format!("{}user-settings", env.db.table_prefix());
    let stored = env.db.user_meta(user_id, &key).await.ok().flatten().unwrap_or_default();
    let stored_time: i64 = env.db.user_meta(user_id, &format!("{}user-settings-time", env.db.table_prefix())).await.ok().flatten().and_then(|t| t.parse().ok()).unwrap_or(0);
    if let Some(cookie) = request.cookie(&format!("wp-settings-{user_id}")) {
        let cleaned: String = cookie.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '=' | '&' | '-' | '_')).collect();
        if cleaned == stored {
            return Vec::new();
        }
        let current: i64 = request.cookie(&format!("wp-settings-time-{user_id}")).map(|t| t.chars().filter(char::is_ascii_digit).collect::<String>()).and_then(|t| t.parse().ok()).unwrap_or(0);
        if current > stored_time {
            return Vec::new();
        }
    }
    let site_path = paths(env.site).site;
    let secure = env.site.settings.siteurl.starts_with("https");
    let year = 31_536_000;
    let value = if stored.is_empty() { "deleted".to_owned() } else { stored.clone() };
    let first_expires = if stored.is_empty() { 1 } else { env.now + year };
    vec![
        SetCookie { name: format!("wp-settings-{user_id}"), value, expires: Some(first_expires), path: site_path.clone(), domain: None, secure, http_only: false },
        SetCookie { name: format!("wp-settings-time-{user_id}"), value: env.now.to_string(), expires: Some(env.now + year), path: site_path, domain: None, secure, http_only: false },
    ]
}

/// `auth_redirect()` for somebody who is not signed in.
fn auth_redirect(env: &AdminEnv<'_>, request: &AdminRequest) -> AdminResponse {
    let origin = {
        let siteurl = &env.site.settings.siteurl;
        match siteurl.split_once("://") {
            Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or_default()),
            None => siteurl.clone(),
        }
    };
    let current = format!("{origin}{}", request.request_uri);
    let login = format!("{}?redirect_to={}&reauth=1", env.site.site_url("wp-login.php"), rustpress_core::canonical::php_urlencode(&current));
    redirect(&login)
}

/// `wp_die()` for the administration area.
pub fn die(env: &AdminEnv<'_>, status: u16, title: &str, message: &str) -> AdminResponse {
    AdminResponse { status, headers: page_headers(), cookies: Vec::new(), body: crate::die::page(env.site, title, message), bytes: Vec::new() }
}

/// Handles a request to a file in `wp-admin`.
pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest) -> AdminResponse {
    if request.file == "admin-ajax.php" || request.file == "async-upload.php" {
        return ajax::handle(env, request).await;
    }
    // Pages of an installed site that need no sign-in.
    match request.file.as_str() {
        "install.php" => return bare_pages::install(env),
        "upgrade.php" => return bare_pages::upgrade(env),
        "setup-config.php" => return bare_pages::setup_config(env),
        _ => {}
    }
    // The signed-in user: the `auth` cookie of this connection's scheme.
    let (scheme, name) = if env.https { (Scheme::SecureAuth, env.auth.cookie_name(Scheme::SecureAuth)) } else { (Scheme::Auth, env.auth.cookie_name(Scheme::Auth)) };
    let session = match request.cookie(&name) {
        Some(cookie) => env.auth.validate_cookie(env.db, cookie, scheme, env.now, false).await.ok().flatten(),
        None => None,
    };
    let Some(session) = session else { return auth_redirect(env, request) };
    let Ok(user) = CurrentUser::load(env.db, env.roles, &session.user).await else {
        return die(env, 500, "WordPress &rsaquo; Error", "<p>The user could not be loaded.</p>");
    };
    let ctx = Ctx { env, request, user, token: session.token.clone() };
    // `user_can_access_admin_page()`: the pages of the menu are only for those the menu is for.
    if let Some(capabilities) = menu_capabilities(&request.file) {
        if !capabilities.iter().any(|capability| ctx.can(capability)) {
            return die(env, 403, "WordPress &rsaquo; Error", &ctx.tr("Sorry, you are not allowed to access this page."));
        }
    }
    let mut response = match request.file.as_str() {
        "" | "index.php" => dashboard::page(&ctx).await,
        "options-general.php" => general::page(&ctx).await,
        "options-writing.php" => settings::writing(&ctx).await,
        "options-reading.php" => settings::reading(&ctx).await,
        "options-discussion.php" => settings::discussion(&ctx).await,
        "options-media.php" => settings::media(&ctx).await,
        "edit.php" => posts::page(&ctx).await,
        "post.php" => post_edit::post_php(&ctx).await,
        "post-new.php" => post_edit::post_new(&ctx).await,
        "options.php" => options::handle(&ctx).await,
        "edit-tags.php" => terms::page(&ctx).await,
        "term.php" => terms::edit_page(&ctx).await,
        "upload.php" => media_lib::upload_php(&ctx).await,
        "media-new.php" => media_lib::media_new(&ctx).await,
        "edit-comments.php" => comments::page(&ctx).await,
        "comment.php" => comments::comment_php(&ctx).await,
        "users.php" => users::page(&ctx).await,
        "profile.php" => user_edit::profile(&ctx).await,
        "user-edit.php" => user_edit::user_edit(&ctx).await,
        "user-new.php" => user_edit::user_new(&ctx).await,
        "tools.php" => pages::tools(&ctx).await,
        "theme-editor.php" => file_editor::theme_editor(&ctx).await,
        "plugin-editor.php" => file_editor::plugin_editor(&ctx).await,
        "import.php" => pages::import(&ctx).await,
        "export.php" => export::page(&ctx).await,
        "options-permalink.php" => permalink::page(&ctx).await,
        "options-privacy.php" => privacy::page(&ctx).await,
        "privacy-policy-guide.php" => privacy::guide(&ctx).await,
        "themes.php" => themes::page(&ctx).await,
        "site-editor.php" => site_editor::page(&ctx).await,
        "revision.php" => revision::page(&ctx).await,
        "font-library.php" => boot_page::font_library(&ctx).await,
        "options-connectors.php" => boot_page::connectors(&ctx).await,
        "plugins.php" => plugins::page(&ctx).await,
        "plugin-install.php" => install::plugin_install(&ctx).await,
        "theme-install.php" => install::theme_install(&ctx).await,
        "update.php" => {
            if !matches!(request.get("action"), Some("upload-plugin" | "upload-theme" | "install-plugin" | "install-theme" | "upgrade-plugin" | "upgrade-theme" | "update-selected" | "update-selected-themes" | "activate-plugin")) {
                bare_pages::empty()
            } else {
                install::update_php(&ctx).await
            }
        }
        "menu.php" | "admin-header.php" => bare_pages::empty(),
        "admin.php" => {
            // `admin.php?page=...`: a page of a plugin's menu, and there is none.
            if request.request("page").is_some_and(|p| !p.is_empty()) {
                die(env, 403, "WordPress &rsaquo; Error", &ctx.tr("Sorry, you are not allowed to access this page."))
            } else {
                bare_pages::empty()
            }
        }
        "media.php" => bare_pages::media(&ctx.admin_url("")),
        // The screens of themes that are not block themes.
        // `nav-menus.php` first says that the theme has neither menus nor widgets, then checks the capability.
        "nav-menus.php" => die(env, 500, "WordPress &rsaquo; Error", &ctx.tr("Your theme does not support navigation menus or widgets.")),
        "widgets.php" if !ctx.can("edit_theme_options") => higher_permission(&ctx, "Sorry, you are not allowed to edit theme options on this site.", 403),
        "widgets.php" => die(env, 500, "WordPress &rsaquo; Error", &sprintf_html(&ctx.tr("The theme you are currently using is not widget-aware, meaning that it has no sidebars that you are able to change. For information on making your theme widget-aware, please <a href=\"%s\">follow these instructions</a>."), "https://developer.wordpress.org/themes/functionality/widgets/")),
        "link-manager.php" | "link-add.php" | "link.php" if !(ctx.user.can_manage_links_if_enabled() && ctx.can("install_plugins")) => die(env, 500, "WordPress &rsaquo; Error", &ctx.tr("Sorry, you are not allowed to edit the links for this site.")),
        "network.php" if !ctx.can("setup_network") => die_message(env, 500, &ctx.tr("Sorry, you are not allowed to manage options for this site.")),
        "customize.php" if !ctx.can("edit_theme_options") => higher_permission(&ctx, "Sorry, you are not allowed to customize this site.", 403),
        "press-this.php" if !ctx.can("edit_posts") => die(env, 403, &ctx.tr("You need a higher level of permission."), &ctx.tr("Sorry, you are not allowed to create posts as this user.")),
        "press-this.php" if !ctx.can("activate_plugins") => die(env, 200, &ctx.tr("Installation Required"), &ctx.tr("Press This is not available. Please contact your site administrator.")),
        "link-manager.php" | "link-add.php" | "link.php" => {
            let url = ctx.admin_url(&format!("update.php?action=install-plugin&plugin=link-manager&_wpnonce={}", ctx.create_nonce("install-plugin_link-manager")));
            die(env, 500, "WordPress &rsaquo; Error", &sprintf_html(&ctx.tr("If you are looking to use the link manager, please install the <a href=\"%s\">Link Manager plugin</a>."), &rustpress_core::formatting::esc_url(&url)))
        }
        "ms-admin.php" | "ms-edit.php" => redirect(&ctx.admin_url("")),
        // Without a network the files of its administration send the user to the pages of the site.
        "ms-options.php" => redirect(&ctx.admin_url("settings.php")),
        "ms-sites.php" => redirect(&ctx.admin_url("sites.php")),
        "ms-themes.php" => redirect(&ctx.admin_url("themes.php")),
        "ms-upgrade-network.php" => redirect(&ctx.admin_url("upgrade.php")),
        "ms-users.php" => redirect(&ctx.admin_url("users.php")),
        "moderation.php" => redirect(&ctx.admin_url("edit-comments.php?comment_status=moderated")),
        "ms-delete-site.php" | "my-sites.php" => die_message(env, 500, &ctx.tr("Multisite support is not enabled.")),
        "network.php" => die_message(env, 500, &rustpress_core::i18n::sprintf(&ctx.tr("You must define the %1$s constant as true in your %2$s file to allow creation of a Network."), &["<code>WP_ALLOW_MULTISITE</code>", "<code>wp-config.php</code>"])),
        "press-this.php" => {
            let url = ctx.admin_url(&format!("update.php?action=install-plugin&plugin=press-this&from=press-this&_wpnonce={}", ctx.create_nonce("install-plugin_press-this")));
            let link = format!("<a href=\"{}\" class=\"install-now\" data-slug=\"press-this\" data-name=\"press-this\" aria-label=\"{}\">{}</a>", rustpress_core::formatting::esc_url(&url), esc_attr(&ctx.tr("Install Now")), ctx.tr("Install Now"));
            let mut response = die(env, 200, &ctx.tr("Installation Required"), &format!("{}<br />{link}", ctx.tr("The Press This plugin is required.")));
            response.status = 200;
            response
        }
        // Files that only define functions or print parts of a screen.
        "custom-background.php" | "custom-header.php" | "edit-form-advanced.php" | "edit-form-blocks.php" | "edit-form-comment.php" | "edit-link-form.php" | "edit-tag-form.php" | "install-helper.php" | "link-parse-opml.php" | "options-head.php" | "site-health-info.php" | "widgets-form.php" | "widgets-form-blocks.php" => bare_pages::include_only(200),
        "upgrade-functions.php" => bare_pages::include_only(500),
        "authorize-application.php" => authorize_app::page(&ctx).await,
        "about.php" | "credits.php" | "freedoms.php" | "contribute.php" | "privacy.php" => pages::about(&ctx, &request.file).await,
        "site-health.php" => site_health::page(&ctx).await,
        "export-personal-data.php" => privacy_requests::page(&ctx, privacy_requests::Kind::Export).await,
        "erase-personal-data.php" => privacy_requests::page(&ctx, privacy_requests::Kind::Erase).await,
        "update-core.php" => update_core::page(&ctx).await,
        other => unavailable(&ctx, other).await,
    };
    let mut cookies = user_settings_cookies(env, request, ctx.user.id).await;
    cookies.extend(std::mem::take(&mut response.cookies));
    response.cookies = cookies;
    response
}

/// `wp_die( $message )` of a plain message: the title is the one of an error.
fn die_message(env: &AdminEnv<'_>, status: u16, message: &str) -> AdminResponse {
    die(env, status, "WordPress &rsaquo; Error", message)
}

/// `wp_die( '<h1>…permission</h1><p>message</p>', 403 )`.
fn higher_permission(ctx: &Ctx<'_>, message: &str, status: u16) -> AdminResponse {
    die(ctx.env, status, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", ctx.tr("You need a higher level of permission."), ctx.tr(message)))
}

/// `sprintf()` with one string, not escaped.
fn sprintf_html(format: &str, value: &str) -> String {
    rustpress_core::i18n::sprintf(format, &[value])
}

/// The capabilities (any of them) the menu of WordPress asks for a page.
fn menu_capabilities(file: &str) -> Option<&'static [&'static str]> {
    Some(match file {
        "options-general.php" | "options-writing.php" | "options-reading.php" | "options-discussion.php" | "options-media.php" | "options-permalink.php" | "options-connectors.php" => &["manage_options"],
        "options-privacy.php" => &["manage_privacy_options"],
        "plugins.php" => &["activate_plugins"],
        "plugin-install.php" => &["install_plugins"],
        "plugin-editor.php" => &["edit_plugins"],
        "themes.php" => &["switch_themes", "edit_theme_options"],
        "theme-editor.php" => &["edit_themes"],
        "site-editor.php" | "font-library.php" => &["edit_theme_options"],
        "edit.php" | "post-new.php" | "edit-comments.php" | "tools.php" => &["edit_posts"],
        "upload.php" | "media-new.php" => &["upload_files"],
        "import.php" => &["import"],
        "export.php" => &["export"],
        "site-health.php" => &["view_site_health_checks"],
        "update-core.php" => &["update_core"],
        "export-personal-data.php" => &["export_others_personal_data"],
        "erase-personal-data.php" => &["erase_others_personal_data"],
        _ => return None,
    })
}

/// A screen of WordPress that RustPress does not have yet.
async fn unavailable(ctx: &Ctx<'_>, file: &str) -> AdminResponse {
    let exists = ctx.env.loader.includes.parent().is_some_and(|root| root.join("wp-admin").join(file).is_file());
    if !exists {
        return die(ctx.env, 404, "WordPress &rsaquo; Error", &format!("<p>{}</p>", ctx.tr("The requested page does not exist.")));
    }
    let content = format!(
        "<div class=\"wrap\">\n<h1>{}</h1>\n{}\n</div>\n\n",
        esc_html(file),
        notices::admin_notice("This screen is not available in RustPress yet.", &["notice-warning"])
    );
    let mut response = render(ctx, &Screen { file: file.to_owned(), title: file.to_owned(), ..Default::default() }, &content, &Assets::default()).await;
    response.status = 501;
    response
}

/// The frame around `content` for the signed-in user.
pub async fn render(ctx: &Ctx<'_>, screen: &Screen, content: &str, assets: &Assets) -> AdminResponse {
    let env = ctx.env;
    let updates = crate::updates::counts(env.site).await;
    let pending = env.db.count_comments_with_status("0").await.unwrap_or(0);
    let menu_context = MenuContext { block_theme: env.block_theme, pending_comments: pending, updates: (updates.total(), updates.plugins, updates.themes), site_title: env.site.settings.blogname.clone() };
    let menu = AdminMenu::core(env.site, &ctx.user, &menu_context).prepare(&ctx.user);
    let mut loader = env.loader.clone();
    {
        let auth: Arc<Auth> = Arc::new(env.auth.clone());
        let (id, token, now) = (ctx.user.id, ctx.token.clone(), env.now);
        loader.nonce = Some(Arc::new(move |action: &str| auth.create_nonce(action, id, &token, now).unwrap_or_default()));
    }
    let chrome_env = ChromeEnv {
        site: env.site,
        loader,
        user: &ctx.user,
        menu: &menu,
        bar: BarContext { updates: updates.total(), pending_comments: pending, logout_nonce: ctx.create_nonce("log-out"), view_item: None, view_archive: false, command_palette: true },
        stylesheet: env.site.settings.stylesheet.clone(),
        cookie_path: paths(env.site).cookie,
        now: env.now,
        secure: env.https,
        request_uri: ctx.request.request_uri.clone(),
        head_styles: format!("{}{}{}", include_str!("../data/head_styles.html"), rustpress_theme::document::font_faces(env.theme), rustpress_theme::document::font_faces_from_style_variations(env.theme)),
    };
    // What the command palette's scripts need on every screen: the fetch layer, the data and preferences persistence.
    let mut assets = assets.clone();
    if !assets.inline.iter().any(|(handle, _, _)| handle == "wp-api-fetch") {
        let nonce = ctx.create_nonce("wp_rest");
        assets.inline.push((
            "wp-api-fetch".into(),
            format!(
                "wp.apiFetch.use( wp.apiFetch.createRootURLMiddleware( {} ) );\nwp.apiFetch.nonceMiddleware = wp.apiFetch.createNonceMiddleware( {} );\nwp.apiFetch.use( wp.apiFetch.nonceMiddleware );\nwp.apiFetch.use( wp.apiFetch.mediaUploadMiddleware );\nwp.apiFetch.nonceEndpoint = {};",
                serde_json::json!(env.site.site_url("wp-json/")),
                serde_json::json!(nonce),
                serde_json::json!(env.site.site_url("wp-admin/admin-ajax.php?action=rest-nonce"))
            ),
            "after".into(),
        ));
    }
    if !assets.inline.iter().any(|(handle, _, _)| handle == "mediaelement-core") {
        // `var mejsL10n`: the strings of MediaElement.js, read from the script loader of WordPress and translated.
        let mut strings = serde_json::Map::new();
        if let Ok(php) = std::fs::read_to_string(env.loader.includes.join("script-loader.php"))
            && let Ok(re) = regex::Regex::new(r"'(mejs\.[a-z-]+)'\s*=>\s*__\(\s*'((?:[^'\\]|\\.)*)'\s*\)")
        {
            for found in re.captures_iter(&php) {
                strings.insert(found[1].to_owned(), serde_json::Value::from(ctx.tr(&found[2].replace("\\'", "'"))));
            }
        }
        let language = env.site.locale().split(['_', '-']).next().unwrap_or("en").to_lowercase();
        let json = rustpress_blocks::php_json::encode_with(&serde_json::json!({"language": language, "strings": strings}), rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false });
        let json = json.replace('<', &format!("{}u003C", char::from(92u8))).replace('>', &format!("{}u003E", char::from(92u8)));
        assets.inline.push(("mediaelement-core".into(), format!("var mejsL10n = {json};"), "before".into()));
    }
    if !assets.inline.iter().any(|(handle, _, _)| handle == "moment") {
        // `moment.updateLocale()` and `wp.date.setSettings()` (`wp_default_packages_inline_scripts()`).
        let site = env.site;
        let locale = site.locale_data();
        let flags = rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false };
        let hex_tags = |json: String| json.replace('<', &format!("{}u003C", char::from(92u8))).replace('>', &format!("{}u003E", char::from(92u8)));
        let time_format = site.options.string("time_format", &ctx.tr("g:i a"));
        let date_format = site.options.string("date_format", &ctx.tr("F j, Y"));
        let start_of_week = site.options.string("start_of_week", "0").parse::<i64>().unwrap_or(0);
        let user_locale = site.locale();
        let moment = serde_json::json!({
            "months": locale.month, "monthsShort": locale.month_abbrev, "weekdays": locale.weekday, "weekdaysShort": locale.weekday_abbrev,
            "week": {"dow": start_of_week},
            "longDateFormat": {"LT": time_format, "LTS": null, "L": null, "LL": date_format, "LLL": ctx.tr("F j, Y g:i a"), "LLLL": null}
        });
        assets.inline.push(("moment".into(), format!("moment.updateLocale( '{user_locale}', {} );", rustpress_blocks::php_json::encode_with(&moment, flags)), "after".into()));
        let gmt_offset = site.options.string("gmt_offset", "0");
        let offset: f64 = gmt_offset.parse().unwrap_or(0.0);
        let timezone_string = site.options.string("timezone_string", "UTC");
        let abbr = if timezone_string.is_empty() { String::new() } else { site.date("T", env.now) };
        let offset_value = if offset.fract() == 0.0 { serde_json::json!(offset as i64) } else { serde_json::json!(offset) };
        let relative = serde_json::json!({
            "future": ctx.tr("%s from now"), "past": ctx.tr("%s ago"), "s": ctx.tr("a second"), "ss": ctx.tr("%d seconds"), "m": ctx.tr("a minute"), "mm": ctx.tr("%d minutes"),
            "h": ctx.tr("an hour"), "hh": ctx.tr("%d hours"), "d": ctx.tr("a day"), "dd": ctx.tr("%d days"), "M": ctx.tr("a month"), "MM": ctx.tr("%d months"), "y": ctx.tr("a year"), "yy": ctx.tr("%d years")
        });
        let date = serde_json::json!({
            "l10n": {
                "locale": user_locale, "months": locale.month, "monthsShort": locale.month_abbrev, "weekdays": locale.weekday, "weekdaysShort": locale.weekday_abbrev,
                "meridiem": {"am": locale.meridiem[0], "pm": locale.meridiem[1], "AM": locale.meridiem[2], "PM": locale.meridiem[3]},
                "relative": relative, "startOfWeek": start_of_week
            },
            "formats": {"time": time_format, "date": date_format, "datetime": ctx.tr("F j, Y g:i a"), "datetimeAbbreviated": ctx.tr("M j, Y g:i a")},
            "timezone": {"offset": offset_value, "offsetFormatted": gmt_offset.replace(".25", ":15").replace(".5", ":30").replace(".75", ":45"), "string": timezone_string, "abbr": abbr}
        });
        assets.inline.push(("wp-date".into(), hex_tags(format!("wp.date.setSettings( {} );", rustpress_blocks::php_json::encode_with(&date, flags))), "after".into()));
    }
    if !assets.inline.iter().any(|(handle, _, _)| handle == "wp-data") {
        assets.inline.push((
            "wp-data".into(),
            format!("( function() {{\n\tvar userId = {};\n\tvar storageKey = \"WP_DATA_USER_\" + userId;\n\twp.data\n\t\t.use( wp.data.plugins.persistence, {{ storageKey: storageKey }} );\n}} )();", ctx.user.id),
            "after".into(),
        ));
    }
    if !assets.inline.iter().any(|(handle, _, _)| handle == "wp-preferences") {
        let preferences = env
            .db
            .user_meta(ctx.user.id, &format!("{}persisted_preferences", env.db.table_prefix()))
            .await
            .ok()
            .flatten()
            .and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok())
            .map_or_else(|| serde_json::json!(""), |value| rustpress_core::options::php_to_json(&value));
        let preferences = rustpress_blocks::php_json::encode_with(&preferences, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: true });
        assets.inline.push((
            "wp-preferences".into(),
            format!("( function() {{\n\t\t\t\tvar serverData = {};\n\t\t\t\tvar userId = \"{}\";\n\t\t\t\tvar persistenceLayer = wp.preferencesPersistence.__unstableCreatePersistenceLayer( serverData, userId );\n\t\t\t\tvar preferencesStore = wp.preferences.store;\n\t\t\t\twp.data.dispatch( preferencesStore ).setPersistenceLayer( persistenceLayer );\n\t\t\t}} ) ();", preferences, ctx.user.id),
            "after".into(),
        ));
    }
    let body = chrome::render(&chrome_env, screen, content, &assets);
    AdminResponse { status: 200, headers: page_headers(), cookies: Vec::new(), body, bytes: Vec::new() }
}

/// A redirect (`wp_redirect()` and `exit`).
pub fn redirect(location: &str) -> AdminResponse {
    AdminResponse {
        status: 302,
        headers: vec![
            ("Expires".into(), "Wed, 11 Jan 1984 05:00:00 GMT".into()),
            ("Cache-Control".into(), "no-cache, must-revalidate, max-age=0, no-store, private".into()),
            ("X-Redirect-By".into(), "WordPress".into()),
            ("Location".into(), location.to_owned()),
            ("Content-Type".into(), "text/html; charset=UTF-8".into()),
        ],
        cookies: Vec::new(),
        body: String::new(),
        bytes: Vec::new(),
    }
}
