//! `wp-login.php`: the sign-in form, signing in, signing out.

use rustpress_auth::password::check_password;
use rustpress_auth::setcookie::{CookiePaths, SetCookie};
use rustpress_auth::{Auth, CurrentUser, Roles, Scheme, session};
use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_db::{AuthUser, Db, Writer};

use crate::deps::Ver;
use crate::registry::{LoaderEnv, default_scripts, default_styles};
use crate::tags::{Attr, inline_script_tag};

/// One installed language for the switcher at the bottom of the page.
#[derive(Debug, Clone)]
pub struct LanguageOption {
    pub locale: String,
    pub native_name: String,
    /// `lang` attribute: the first ISO code, empty when the name list does not know the language.
    pub lang: String,
}

/// What the sign-in page needs from the site.
pub struct LoginEnv<'a> {
    pub site: &'a Site,
    pub db: &'a Db,
    /// Present when writing is allowed; without it nobody can sign in.
    pub writer: Option<&'a Writer>,
    pub auth: &'a Auth,
    pub roles: &'a Roles,
    pub loader: LoaderEnv,
    pub languages: Vec<LanguageOption>,
    /// The title of the privacy policy page and its address, if it is published.
    pub privacy_policy: Option<(String, String)>,
    /// Whether the page is served over TLS (`is_ssl()`).
    pub https: bool,
    pub now: i64,
}

/// A request to `wp-login.php`.
#[derive(Debug, Clone, Default)]
pub struct LoginRequest {
    pub post: bool,
    pub query: Vec<(String, String)>,
    pub form: Vec<(String, String)>,
    pub cookies: Vec<(String, String)>,
    pub remote_addr: Option<String>,
    pub user_agent: Option<String>,
}

impl LoginRequest {
    fn get(&self, name: &str) -> Option<&str> {
        self.query.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    fn post_field(&self, name: &str) -> Option<&str> {
        self.form.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// `$_REQUEST`: the form wins over the query string.
    fn request(&self, name: &str) -> Option<&str> {
        self.post_field(name).or_else(|| self.get(name))
    }

    fn cookie(&self, name: &str) -> Option<&str> {
        self.cookies.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

/// The answer: a page or a redirect, with headers and cookies.
#[derive(Debug, Clone, Default)]
pub struct LoginResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub cookies: Vec<SetCookie>,
    pub body: String,
}

/// One entry of a `WP_Error`.
#[derive(Debug, Clone)]
struct LoginError {
    code: &'static str,
    message: String,
    /// `message` shows as information, anything else as an error.
    is_message: bool,
}

fn error(code: &'static str, message: String) -> LoginError {
    LoginError { code, message, is_message: false }
}

/// `nocache_headers()` and the frame headers of the sign-in page.
fn page_headers() -> Vec<(String, String)> {
    vec![
        ("Expires".into(), "Wed, 11 Jan 1984 05:00:00 GMT".into()),
        ("Cache-Control".into(), "no-cache, must-revalidate, max-age=0, no-store, private".into()),
    ]
}

fn frame_headers() -> Vec<(String, String)> {
    vec![
        ("X-Frame-Options".into(), "SAMEORIGIN".into()),
        ("Content-Security-Policy".into(), "frame-ancestors 'self';".into()),
        ("Referrer-Policy".into(), "strict-origin-when-cross-origin".into()),
        ("Content-Type".into(), "text/html; charset=UTF-8".into()),
    ]
}

/// `sanitize_user()` without strict mode: tags and a few characters go, the rest is trimmed.
fn sanitize_user(raw: &str) -> String {
    let stripped = rustpress_core::formatting::strip_tags(raw);
    let mut out = String::new();
    for c in stripped.chars() {
        if c.is_control() || matches!(c, '%' | '<' | '>' | '&') {
            continue;
        }
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_owned()
}

/// `wp_validate_redirect()`.
fn validate_redirect(site: &Site, location: &str, fallback: &str) -> String {
    let mut location = location.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{8}', '\u{b}']).to_owned();
    if location.starts_with("//") {
        location = format!("http:{location}");
    }
    let test = location.split('?').next().unwrap_or_default();
    let (scheme, host) = match test.split_once("://") {
        Some((scheme, rest)) => (Some(scheme.to_owned()), Some(rest.split(['/', '?', '#']).next().unwrap_or_default().to_owned())),
        None => (None, None),
    };
    if let Some(scheme) = &scheme
        && scheme != "http"
        && scheme != "https"
    {
        return fallback.to_owned();
    }
    if scheme.is_none() && test.contains(':') && !test.starts_with('/') && test.split(':').next().is_some_and(|s| s.chars().all(|c| c.is_ascii_alphabetic())) {
        return fallback.to_owned();
    }
    if let Some(host) = &host {
        let bare = host.rsplit('@').next().unwrap_or(host);
        let bare = bare.split(':').next().unwrap_or(bare);
        let home = site.settings.home.split_once("://").map_or("", |(_, r)| r.split(['/', ':']).next().unwrap_or(""));
        if !bare.eq_ignore_ascii_case(home) || host.contains('@') {
            return fallback.to_owned();
        }
    }
    location
}

/// `wp_sanitize_redirect()`: only characters that belong in an address stay.
fn sanitize_redirect(location: &str) -> String {
    location.chars().filter(|c| c.is_ascii_alphanumeric() || "-~+_.?#=!&;,/:%@$|*'()[]\\".contains(*c) || (*c as u32) >= 0x80).collect::<String>().replace(' ', "%20")
}

impl LoginEnv<'_> {
    fn tr(&self, text: &str) -> String {
        self.site.tr(text)
    }

    fn login_url(&self) -> String {
        self.site.site_url("wp-login.php")
    }

    fn lostpassword_url(&self) -> String {
        format!("{}?action=lostpassword", self.login_url())
    }

    fn admin_url(&self) -> String {
        self.site.site_url("wp-admin/")
    }

    fn paths(&self) -> CookiePaths {
        CookiePaths::from_urls(&self.site.settings.home, &self.site.settings.siteurl)
    }

    fn secure_cookie_default(&self) -> bool {
        self.https
    }

    fn test_cookies(&self) -> Vec<SetCookie> {
        let paths = self.paths();
        let make = |path: &str| SetCookie { name: "wordpress_test_cookie".into(), value: "WP Cookie check".into(), expires: None, path: path.to_owned(), domain: None, secure: self.login_url().starts_with("https"), http_only: true };
        let mut cookies = vec![make(&paths.cookie)];
        if paths.site != paths.cookie {
            cookies.push(make(&paths.site));
        }
        cookies
    }

    // ------------------------------------------------------------------ authentication

    /// `wp_authenticate()` through the filters of a default install.
    async fn authenticate(&self, login: &str, password: &str) -> Result<AuthUser, Vec<LoginError>> {
        let username = sanitize_user(login);
        let password = password.trim();
        // `wp_authenticate_username_password()`
        if username.is_empty() || password.is_empty() {
            let mut errors = Vec::new();
            if username.is_empty() {
                errors.push(error("empty_username", self.tr("<strong>Error:</strong> The username field is empty.")));
            }
            if password.is_empty() {
                errors.push(error("empty_password", self.tr("<strong>Error:</strong> The password field is empty.")));
            }
            return Err(errors);
        }
        let by_login = self.db.auth_user_by_login(&username).await.ok().flatten();
        let by_email = if by_login.is_none() && username.contains('@') { self.db.auth_user_by_email(&username).await.ok().flatten() } else { None };
        let is_email = by_login.is_none() && username.contains('@');
        let Some(user) = by_login.or(by_email) else {
            if is_email {
                return Err(vec![error("invalid_email", self.tr("Unknown email address. Check again or try your username."))]);
            }
            return Err(vec![error(
                "invalid_username",
                rustpress_core::i18n::sprintf(&self.tr("<strong>Error:</strong> The username <strong>%s</strong> is not registered on this site. If you are unsure of your username, try your email address instead."), &[&esc_html(&username)]),
            )]);
        };
        if !check_password(password, &user.user_pass) {
            let subject = if is_email {
                rustpress_core::i18n::sprintf(&self.tr("<strong>Error:</strong> The password you entered for the email address %s is incorrect."), &[&format!("<strong>{}</strong>", esc_html(&username))])
            } else {
                rustpress_core::i18n::sprintf(&self.tr("<strong>Error:</strong> The password you entered for the username %s is incorrect."), &[&format!("<strong>{}</strong>", esc_html(&username))])
            };
            return Err(vec![error("incorrect_password", format!("{subject} <a href=\"{}\">{}</a>", self.lostpassword_url(), self.tr("Lost your password?")))]);
        }
        Ok(user)
    }

    /// `wp_validate_auth_cookie()` for the sign-in cookie, as `wp_authenticate_cookie()` uses it.
    async fn user_of_auth_cookie(&self, request: &LoginRequest) -> Option<AuthUser> {
        let (scheme, name) = if self.secure_cookie_default() { (Scheme::SecureAuth, self.auth.cookie_name(Scheme::SecureAuth)) } else { (Scheme::Auth, self.auth.cookie_name(Scheme::Auth)) };
        let cookie = request.cookie(&name)?;
        let session = self.auth.validate_cookie(self.db, cookie, scheme, self.now, request.post).await.ok().flatten()?;
        Some(session.user)
    }

    // ------------------------------------------------------------------ the page

    fn login_title(&self, title: &str) -> String {
        let site = &self.site.settings.blogname;
        rustpress_core::i18n::sprintf(&self.tr("%1$s &lsaquo; %2$s &#8212; WordPress"), &[title, site])
    }

    fn robots(&self) -> String {
        let mut directives: Vec<&str> = Vec::new();
        if self.site.options.get("blog_public").is_some_and(|v| v != "0") {
            directives.push("max-image-preview:large");
        } else {
            directives.push("noindex");
            directives.push("nofollow");
        }
        for extra in ["noindex", "noarchive"] {
            if !directives.contains(&extra) {
                directives.push(extra);
            }
        }
        format!("<meta name='robots' content='{}' />\n", directives.join(", "))
    }

    fn header(&self, action: &str, title: &str, message: &str, errors: &[LoginError]) -> String {
        let rtl = self.site.is_rtl();
        let mut styles = default_styles(&self.loader);
        styles.enqueue("login");
        let admin_styles = styles.print_admin_styles(true);
        let language = self.site.language_tag();
        let mut lang_attrs = format!("lang=\"{}\"", esc_attr(&language));
        if rtl {
            lang_attrs = format!("dir=\"rtl\" {lang_attrs}");
        }
        let mut classes = vec![format!("login-action-{action}"), "wp-core-ui".to_owned(), "admin-color-modern".to_owned()];
        if rtl {
            classes.push("rtl".to_owned());
        }
        classes.push(format!("locale-{}", rustpress_core::formatting::sanitize_html_class(&self.site.locale().replace('_', "-").to_lowercase())));
        let logo_text = self.tr("Powered by WordPress");
        let logo_url = self.tr("https://wordpress.org/");

        let mut out = String::new();
        out.push_str(&format!("<!DOCTYPE html>\n\t<html {lang_attrs}>\n\t<head>\n\t<meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\" />\n\t<title>{}</title>\n\t", self.login_title(title)));
        out.push_str(&self.robots());
        out.push_str(&admin_styles);
        out.push_str("\t<meta name='referrer' content='strict-origin-when-cross-origin' />\n\t");
        out.push_str("\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n\t");
        out.push_str("\t</head>\n");
        out.push_str(&format!("\t<body class=\"login no-js {}\">\n\t", esc_attr(&classes.join(" "))));
        out.push_str(&inline_script_tag("document.body.className = document.body.className.replace('no-js','js');", &[]));
        out.push('\n');
        if action != "confirm_admin_email" && !title.is_empty() {
            out.push_str(&format!("\t\t\t\t<h1 class=\"screen-reader-text\">{title}</h1>\n"));
        }
        out.push_str(&format!("\t\t\t<div id=\"login\">\n\t\t<h1 role=\"presentation\" class=\"wp-login-logo\"><a href=\"{}\">{logo_text}</a></h1>\n\t", esc_url(&logo_url)));
        if !message.is_empty() {
            out.push_str(&format!("{message}\n"));
        }
        out.push_str(&self.notices(errors));
        out
    }

    /// The error and message notices of `login_header()`.
    fn notices(&self, errors: &[LoginError]) -> String {
        let mut out = String::new();
        let list: Vec<&LoginError> = errors.iter().filter(|e| !e.is_message && !(e.code.is_empty() && e.message.is_empty())).collect();
        let messages: String = errors.iter().filter(|e| e.is_message).map(|e| format!("<p>{}</p>", e.message)).collect();
        if !list.is_empty() {
            let inner = if list.len() > 1 {
                format!("<ul class=\"login-error-list\">{}</ul>", list.iter().map(|e| format!("<li>{}</li>", e.message)).collect::<String>())
            } else {
                format!("<p>{}</p>", list[0].message)
            };
            out.push_str(&format!("<div id=\"login_error\" class=\"notice notice-error\">{inner}</div>"));
        }
        if !messages.is_empty() {
            out.push_str(&format!("<div id=\"login-message\" class=\"notice notice-info message\">{messages}</div>"));
        }
        out
    }

    fn footer(&self) -> String {
        let mut out = String::new();
        let blogname = &self.site.settings.blogname;
        let link = format!("<a href=\"{}\">{}</a>", esc_url(&self.site.home_url("/")), rustpress_core::i18n::sprintf(&self.site.trx("&larr; Go to %s", "site"), &[blogname]));
        out.push_str(&format!("\t\t<p id=\"backtoblog\">\n\t\t\t{link}\t\t</p>\n\t\t"));
        if let Some((title, url)) = &self.privacy_policy {
            out.push_str(&format!("<div class=\"privacy-policy-page-link\"><a class=\"privacy-policy-link\" href=\"{}\" rel=\"privacy-policy\">{}</a></div>", esc_url(url), esc_html(title)));
        }
        out.push_str("\t</div>");
        out.push_str("\n\t");
        if !self.languages.is_empty() {
            out.push_str(&self.language_switcher());
        }
        out.push_str("\n\t");
        out
    }

    fn language_switcher(&self) -> String {
        let current = self.site.locale();
        // WordPress compares the empty string with the selected locale here, so this entry is never selected.
        let mut options = String::from("<option value=\"en_US\" lang=\"en\" data-installed=\"1\">English (United States)</option>\n");
        for language in &self.languages {
            options.push_str(&format!(
                "<option value=\"{}\" lang=\"{}\"{} data-installed=\"1\">{}</option>\n",
                esc_attr(&language.locale),
                esc_attr(&language.lang),
                if language.locale == current { " selected='selected'" } else { "" },
                esc_html(&language.native_name)
            ));
        }
        let options = options.trim_end_matches('\n');
        format!(
            "\t\t\t<div class=\"language-switcher\">\n\t\t\t\t<form id=\"language-switcher\" method=\"get\">\n\n\t\t\t\t\t<label for=\"language-switcher-locales\">\n\t\t\t\t\t\t{}<span class=\"dashicons dashicons-translation\" aria-hidden=\"true\"></span>\n\t\t\t\t\t</label>\n\n\t\t\t\t\t<select name=\"wp_lang\" id=\"language-switcher-locales\">{options}</select>\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t\t\t<input type=\"submit\" class=\"button\" value=\"{}\">\n\n\t\t\t\t\t</form>\n\t\t\t\t</div>\n\t\t\t",
            self.tr("Language"),
            esc_attr(&self.tr("Change"))
        )
    }

    /// The scripts the page prints at the end (`wp_print_footer_scripts()`).
    fn footer_scripts(&self, enqueue: &[&str]) -> String {
        let mut scripts = default_scripts(&self.loader);
        for handle in enqueue {
            scripts.enqueue(handle);
        }
        scripts.print_footer_scripts(true)
    }

    /// The form of the default action.
    #[allow(clippy::too_many_arguments)]
    fn form(&self, errors: &[LoginError], user_login: &str, remember: bool, redirect_to: &str) -> String {
        let has_errors = errors.iter().any(|e| !e.code.is_empty() || !e.message.is_empty());
        let describe = if has_errors {
            if errors.first().is_some_and(|e| e.is_message) { " aria-describedby=\"login-message\"" } else { " aria-describedby=\"login_error\"" }
        } else {
            ""
        };
        let help = self.tr("Selecting \"Remember Me\" increases the length of time until you&#8217;re asked to log in again on this device. To keep your account secure, use this option only on your personal devices.");
        let toggletip = toggletip(&help, "rememberme-help-toggletip", &self.tr("Help"), &self.tr("Close"));
        let register = if self.site.options.get("users_can_register").is_some_and(|v| v != "0" && !v.is_empty()) {
            format!("<a class=\"wp-login-register\" href=\"{}\">{}</a>{}", esc_url(&self.site.site_url("wp-login.php?action=register")), self.tr("Register"), esc_html(" | "))
        } else {
            String::new()
        };
        format!(
            "\n\t\t<form name=\"loginform\" id=\"loginform\" action=\"{action}\" method=\"post\">\n\t\t\t<p>\n\t\t\t\t<label for=\"user_login\">{label_login}</label>\n\t\t\t\t<input type=\"text\" name=\"log\" id=\"user_login\"{describe} class=\"input ltr\" value=\"{login}\" size=\"20\" autocapitalize=\"off\" autocomplete=\"username\" required=\"required\" />\n\t\t\t</p>\n\n\t\t\t<div class=\"user-pass-wrap\">\n\t\t\t\t<label for=\"user_pass\">{label_pass}</label>\n\t\t\t\t<div class=\"wp-pwd\">\n\t\t\t\t\t<input type=\"password\" name=\"pwd\" id=\"user_pass\"{describe} class=\"input password-input ltr\" value=\"\" size=\"20\" autocomplete=\"current-password\" spellcheck=\"false\" required=\"required\" />\n\t\t\t\t\t<button type=\"button\" class=\"button button-secondary wp-hide-pw hide-if-no-js\" data-toggle=\"0\" aria-label=\"{show}\">\n\t\t\t\t\t\t<span class=\"dashicons dashicons-visibility\" aria-hidden=\"true\"></span>\n\t\t\t\t\t</button>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t\t\t\t\t\t\t<p class=\"forgetmenot\">\n\t\t\t\t<input name=\"rememberme\" type=\"checkbox\" id=\"rememberme\" value=\"forever\" {checked} />\n\t\t\t\t<label for=\"rememberme\">{remember_label}</label>\n\t\t\t\t{toggletip}\t\t\t</p>\n\t\t\t<p class=\"submit\">\n\t\t\t\t<input type=\"submit\" name=\"wp-submit\" id=\"wp-submit\" class=\"button button-primary button-large\" value=\"{submit}\" />\n\t\t\t\t\t\t\t\t\t<input type=\"hidden\" name=\"redirect_to\" value=\"{redirect}\" />\n\t\t\t\t\t\t\t\t\t<input type=\"hidden\" name=\"testcookie\" value=\"1\" />\n\t\t\t</p>\n\t\t</form>\n\n\t\t\t\t\t<p id=\"nav\">\n\t\t\t\t{register}<a class=\"wp-login-lost-password\" href=\"{lost}\">{lost_label}</a>\t\t\t</p>\n",
            action = esc_url(&self.login_url()),
            label_login = self.tr("Username or Email Address"),
            label_pass = self.tr("Password"),
            show = esc_attr(&self.tr("Show password")),
            checked = if remember { "checked='checked'" } else { "" },
            remember_label = esc_html(&self.tr("Remember Me")),
            submit = esc_attr(&self.tr("Log In")),
            login = esc_attr(user_login),
            redirect = esc_attr(redirect_to),
            lost = esc_url(&self.lostpassword_url()),
            lost_label = self.tr("Lost your password?"),
        )
    }

    /// The default action: the form, with the outcome of a sign-in attempt.
    async fn login_action(&self, request: &LoginRequest) -> LoginResponse {
        let mut response = LoginResponse { status: 200, headers: page_headers(), cookies: self.test_cookies(), body: String::new() };
        response.headers.extend(frame_headers());

        let redirect_to_requested = request.request("redirect_to").map(str::to_owned);
        let redirect_to = redirect_to_requested.clone().unwrap_or_else(|| self.admin_url());
        let reauth = request.request("reauth").is_some_and(|v| !v.is_empty());

        let login = request.post_field("log").filter(|l| !l.is_empty()).unwrap_or_default();
        let password = request.post_field("pwd").filter(|p| !p.is_empty()).unwrap_or_default();
        let remember = request.post_field("rememberme").is_some_and(|v| !v.is_empty());

        let mut outcome = self.authenticate(login, password).await;
        // `wp_authenticate_cookie()`: no credentials, but a valid sign-in cookie.
        if outcome.is_err() && login.is_empty() && password.is_empty() && !reauth
            && let Some(user) = self.user_of_auth_cookie(request).await
        {
            outcome = Ok(user);
        }
        let credentials_given = !login.is_empty() || !password.is_empty();
        let mut signed_in: Option<AuthUser> = None;
        let mut errors: Vec<LoginError> = Vec::new();
        match outcome {
            Ok(user) => {
                if credentials_given {
                    // Sign in: a session and the cookies.
                    let Some(writer) = self.writer else {
                        errors.push(error("writes_disabled", self.tr("<strong>Error:</strong> Signing in is not possible: this site was started without write access.")));
                        return self.login_page(request, response, errors, login, remember, &redirect_to, false);
                    };
                    let expiration = Auth::login_expiration(remember, self.now);
                    match session::create_session(self.db, writer, user.id, expiration, request.remote_addr.as_deref(), request.user_agent.as_deref(), self.now).await {
                        Ok(token) => {
                            let secure = self.secure_cookie_default();
                            let home_https = self.site.settings.home.starts_with("https");
                            response.cookies.extend(self.auth.login_cookies(&user, remember, secure, home_https, &token, self.now, expiration, &self.paths()));
                            signed_in = Some(user);
                        }
                        Err(e) => {
                            tracing_error(&e);
                            errors.push(error("session", self.tr("<strong>Error:</strong> Could not start a session.")));
                        }
                    }
                } else {
                    signed_in = Some(user);
                }
            }
            Err(e) => errors = e,
        }

        // Cookies must work: the sign-in cookie, or at least the test cookie.
        if signed_in.is_none() || (signed_in.is_some() && !credentials_given) {
            // The logged-in cookie of this request counts as set after a sign-in.
        }
        let logged_in_cookie_present = request.cookie(&self.auth.cookie_name(Scheme::LoggedIn)).is_some_and(|c| !c.is_empty()) || (signed_in.is_some() && credentials_given);
        if !logged_in_cookie_present && request.post_field("testcookie").is_some() && request.cookie("wordpress_test_cookie").is_none_or(str::is_empty) {
            signed_in = None;
            errors = vec![error(
                "test_cookie",
                rustpress_core::i18n::sprintf(
                    &self.tr("<strong>Error:</strong> Cookies are blocked or not supported by your browser. You must <a href=\"%s\">enable cookies</a> to use WordPress."),
                    &[&self.tr("https://developer.wordpress.org/advanced-administration/wordpress/cookies/#enable-cookies-in-your-browser")],
                ),
            )];
        }

        if let Some(user) = &signed_in
            && !reauth
        {
            // Where to go: `wp_redirect()` for the admin, `wp_safe_redirect()` for anything else.
            let mut target = redirect_to.clone();
            let admin_url = self.admin_url();
            if target.is_empty() || target == "wp-admin/" || target == admin_url {
                let current = CurrentUser::load(self.db, self.roles, user).await.ok();
                if let Some(current) = &current
                    && !current.can("edit_posts")
                {
                    target = if current.can("read") { self.site.site_url("wp-admin/profile.php") } else { self.site.home_url("/") };
                }
                response.status = 302;
                response.headers.push(("X-Redirect-By".into(), "WordPress".into()));
                response.headers.push(("Location".into(), sanitize_redirect(&target)));
                return response;
            }
            let safe = validate_redirect(self.site, &target, &admin_url);
            response.status = 302;
            response.headers.push(("X-Redirect-By".into(), "WordPress".into()));
            response.headers.push(("Location".into(), sanitize_redirect(&safe)));
            return response;
        }
        if signed_in.is_some() && reauth {
            errors.clear();
        }
        self.login_page(request, response, errors, login, remember, &redirect_to, reauth)
    }

    /// Builds the form page with `errors`.
    #[allow(clippy::too_many_arguments)]
    fn login_page(&self, request: &LoginRequest, mut response: LoginResponse, mut errors: Vec<LoginError>, login: &str, remember: bool, redirect_to: &str, reauth: bool) -> LoginResponse {
        if request.get("loggedout").is_some_and(|v| !v.is_empty()) || reauth {
            errors.retain(|e| e.is_message);
        }
        let only_empty = errors.len() == 2 && errors[0].code == "empty_username" && errors[1].code == "empty_password";
        if !request.post && only_empty {
            errors.clear();
        }
        if request.get("loggedout").is_some_and(|v| !v.is_empty()) {
            errors.push(LoginError { code: "loggedout", message: self.tr("You are now logged out."), is_message: true });
        } else if request.get("registration") == Some("disabled") {
            errors.push(error("registerdisabled", self.tr("<strong>Error:</strong> User registration is currently not allowed.")));
        }
        let user_login = if request.post_field("log").is_some() && errors.first().is_some_and(|e| e.code == "incorrect_password" || e.code == "empty_password") { request.post_field("log").unwrap_or_default() } else { "" };
        let _ = login;
        let title = self.tr("Log In");
        let mut body = self.header("login", &title, "", &errors);
        body.push_str(&self.form(&errors, user_login, remember, redirect_to));
        // The focus script.
        let mut script = String::from("function wp_attempt_focus() {setTimeout( function() {try {");
        if !user_login.is_empty() {
            script.push_str("d = document.getElementById( \"user_pass\" ); d.value = \"\";");
        } else {
            script.push_str("d = document.getElementById( \"user_login\" );");
            if errors.first().is_some_and(|e| e.code == "invalid_username") {
                script.push_str("d.value = \"\";");
            }
        }
        script.push_str("d.focus(); d.select();} catch( er ) {}}, 200);}\n");
        script.push_str("wp_attempt_focus();\n");
        script.push_str("if ( typeof wpOnload === 'function' ) { wpOnload() }");
        body.push_str(&format!("\t\t\t{}", inline_script_tag(&script, &[])));
        body.push_str(&self.footer());
        if errors.first().is_some_and(|e| matches!(e.code, "empty_password" | "empty_email" | "invalid_email" | "invalidcombo" | "empty_username" | "invalid_username" | "incorrect_password")) {
            // `wp_shake_js()`
            body.push_str(&inline_script_tag("document.querySelector('form').classList.add('shake');", &[]));
        }
        body.push_str(&self.footer_scripts(&["user-profile", "wp-tooltip"]));
        body.push_str("\t</body>\n\t</html>\n\t");
        response.body = body;
        response
    }

    // ------------------------------------------------------------------ other actions

    /// `wp-login.php?action=logout`.
    async fn logout_action(&self, request: &LoginRequest) -> LoginResponse {
        let mut response = LoginResponse { status: 200, headers: page_headers(), cookies: self.test_cookies(), body: String::new() };
        response.headers.extend(frame_headers());
        // The session of the logged-in cookie, and the nonce of the sign-out link.
        let cookie_name = self.auth.cookie_name(Scheme::LoggedIn);
        let session = match request.cookie(&cookie_name) {
            Some(cookie) => self.auth.validate_cookie(self.db, cookie, Scheme::LoggedIn, self.now, request.post).await.ok().flatten(),
            None => None,
        };
        let (user_id, token) = session.as_ref().map(|s| (s.user.id, s.token.clone())).unwrap_or((0, String::new()));
        let nonce = request.request("_wpnonce").unwrap_or_default();
        if self.auth.verify_nonce(nonce, "log-out", user_id, &token, self.now).is_none() {
            // `wp_nonce_ays( 'log-out' )`
            let blogname = &self.site.settings.blogname;
            let title = rustpress_core::i18n::sprintf(&self.tr("You are attempting to log out of %s"), &[blogname]);
            let mut url = self.site.site_url("wp-login.php?action=logout");
            if let Some(redirect) = request.request("redirect_to").filter(|r| !r.is_empty()) {
                url = format!("{url}&redirect_to={}", rustpress_core::canonical::php_urlencode(redirect));
            }
            let link = esc_html(&format!("{url}&_wpnonce={}", self.auth.create_nonce("log-out", user_id, &token, self.now).unwrap_or_default()));
            let html = format!("{title}</p><p>{}", rustpress_core::i18n::sprintf(&self.tr("Do you really want to <a href=\"%s\">log out</a>?"), &[&link]));
            response.status = 403;
            response.body = crate::die::page(self.site, &title, &html);
            return response;
        }
        // `wp_logout()`: end the session, clear the cookies.
        if let (Some(session), Some(writer)) = (&session, self.writer) {
            let _ = session::destroy_session(self.db, writer, session.user.id, &session.token, self.now).await;
        }
        response.cookies.extend(self.auth.clear_cookies(user_id, self.now, self.auth.cookie_hash(), &self.paths()));
        let target = match request.request("redirect_to").filter(|r| !r.is_empty()) {
            Some(redirect) => redirect.to_owned(),
            None => format!("{}?loggedout=true&wp_lang={}", self.login_url(), self.site.locale()),
        };
        let safe = validate_redirect(self.site, &target, &self.admin_url());
        response.status = 302;
        response.headers.push(("X-Redirect-By".into(), "WordPress".into()));
        response.headers.push(("Location".into(), sanitize_redirect(&safe)));
        response
    }
}

/// `wp_get_toggletip()` for the default button.
fn toggletip(content: &str, id: &str, label: &str, close: &str) -> String {
    let text = esc_html(content);
    format!(
        "<span class=\"wp-tooltip wp-is-toggletip\">\n\t\t\t\t<button aria-haspopup=\"dialog\" aria-label=\"{label}\" class=\"wp-tooltip__toggle\" popovertarget=\"{id}\" type=\"button\"><span aria-hidden=\"true\" class=\"dashicons dashicons-editor-help\"></span></button>\n\t\t\t\t<span popover=\"auto\" id=\"{id}\" class=\"wp-tooltip__bubble\" role=\"dialog\" aria-label=\"{label}\" tabindex=\"-1\" autofocus><span id=\"{id}-text\" class=\"wp-tooltip__text\">{text}</span><button type=\"button\" class=\"wp-tooltip__close\" popovertarget=\"{id}\" popovertargetaction=\"hide\" aria-label=\"{close}\"><span class=\"dashicons dashicons-no-alt\" aria-hidden=\"true\"></span></button></span></span>"
    )
}

fn tracing_error(error: &rustpress_db::Error) {
    eprintln!("login: {error}");
}

/// Handles a request to `wp-login.php`.
pub async fn handle(env: &LoginEnv<'_>, request: &LoginRequest) -> LoginResponse {
    let action = request.request("action").unwrap_or("login");
    match action {
        "logout" => env.logout_action(request).await,
        _ => env.login_action(request).await,
    }
}

#[allow(dead_code)]
fn unused(_: Ver, _: &[(&str, Attr)]) {}
