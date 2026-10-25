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
pub mod ajax_media;
pub mod ajax_terms;
pub mod block_editor;
pub mod comments;
pub mod media_lib;
pub mod media_ui;
pub mod common;
pub mod dashboard;
pub mod editor;
pub mod general;
pub mod notices;
pub mod options;
pub mod post_edit;
pub mod posts;
pub mod settings;
pub mod terms;
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
    AdminResponse { status, headers: page_headers(), cookies: Vec::new(), body: crate::die::page(env.site, title, message) }
}

/// Handles a request to a file in `wp-admin`.
pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest) -> AdminResponse {
    if request.file == "admin-ajax.php" || request.file == "async-upload.php" {
        return ajax::handle(env, request).await;
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
        other => unavailable(&ctx, other).await,
    };
    let mut cookies = user_settings_cookies(env, request, ctx.user.id).await;
    cookies.extend(std::mem::take(&mut response.cookies));
    response.cookies = cookies;
    response
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
        bar: BarContext { updates: updates.total(), pending_comments: pending, logout_nonce: ctx.create_nonce("log-out"), view_item: None, command_palette: assets.scripts.iter().any(|s| s == "wp-core-commands") },
        stylesheet: env.site.settings.stylesheet.clone(),
        cookie_path: paths(env.site).cookie,
        now: env.now,
        secure: env.https,
    };
    let body = chrome::render(&chrome_env, screen, content, assets);
    AdminResponse { status: 200, headers: page_headers(), cookies: Vec::new(), body }
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
    }
}
