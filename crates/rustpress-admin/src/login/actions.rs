//! The actions of `wp-login.php` besides signing in and out: the cookie of a protected post (`postpass`), the lost
//! password form and the reset of a password (`lostpassword`, `rp`), registration, the message pages (`checkemail`,
//! `confirmaction`) and the check of the administration e-mail address.

use rustpress_core::OrLog;
use super::*;
use crate::content::{Content, UserData};
use rustpress_auth::password::{hash_portable, verify_fast_hash};
use rustpress_db::Val;

/// The password reset keys live for a day (`password_reset_expiration`).
const RESET_KEY_LIFETIME: i64 = 86_400;
/// `MONTH_IN_SECONDS`.
const MONTH: i64 = 30 * 86_400;

/// `wp_get_admin_notice()` of an informational message with extra classes.
fn info_notice(message: &str, classes: &[&str]) -> String {
    let extra: String = classes.iter().map(|c| format!(" {c}")).collect();
    format!("<div class=\"notice notice-info{extra}\"><p>{message}</p></div>")
}

/// `wp_specialchars_decode( $text, ENT_QUOTES )`.
fn specialchars_decode(text: &str) -> String {
    text.replace("&#039;", "'").replace("&#39;", "'").replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// `wp_generate_password( $length, $special_chars )`.
fn generate_password(length: usize, special: bool) -> String {
    crate::sanitize::random_password(length, special)
}

impl LoginEnv<'_> {
    /// The answer every action starts from: no caching, the test cookie, the frame headers.
    fn blank(&self) -> LoginResponse {
        let mut response = LoginResponse { status: 200, headers: page_headers(), cookies: self.test_cookies(), body: String::new(), bytes: Vec::new() };
        response.headers.extend(frame_headers());
        response
    }

    /// `wp_get_referer()`.
    fn referer(&self, request: &LoginRequest) -> Option<String> {
        let raw = request.request("_wp_http_referer").map(str::to_owned).or_else(|| request.referer.clone())?;
        if raw.is_empty() {
            return None;
        }
        let home = self.site.settings.home.trim_end_matches('/');
        if raw == request.request_uri || raw == format!("{home}{}", request.request_uri) {
            return None;
        }
        let valid = validate_redirect(self.site, &raw, "");
        (!valid.is_empty()).then_some(valid)
    }

    /// `wp_redirect()` or, with `safe`, `wp_safe_redirect()`. Without a location nothing is sent.
    fn redirect(&self, mut response: LoginResponse, request: &LoginRequest, location: &str, safe: bool) -> LoginResponse {
        let mut target = sanitize_redirect(location.trim());
        if target.is_empty() {
            return response;
        }
        if safe {
            // A relative address is made absolute from the directory of this script.
            if !target.contains("://") && !target.starts_with('/') && !target.starts_with('#') && !target.starts_with('?') {
                let dir = request.path.rsplit_once('/').map_or("", |(dir, _)| dir);
                target = format!("{}/{target}", dir.trim_end_matches('/'));
            }
            target = validate_redirect(self.site, &target, &self.admin_url());
        }
        response.status = 302;
        response.headers.push(("X-Redirect-By".into(), "WordPress".into()));
        response.headers.push(("Location".into(), sanitize_redirect(&target)));
        response
    }

    /// `wp_die( $message )` with the 500 of a call without a status.
    fn die(&self, status: u16, title: &str, html: &str) -> LoginResponse {
        let mut response = self.blank();
        response.status = status;
        response.body = crate::die::page(self.site, title, html);
        response
    }

    /// `wp_nonce_ays()`.
    fn nonce_ays(&self, request: &LoginRequest) -> LoginResponse {
        let mut html = self.tr("The link you followed has expired.");
        if let Some(referer) = self.referer(request) {
            html.push_str(&format!("</p><p><a href=\"{}\">{}</a>", esc_url(&referer), self.tr("Please try again.")));
        }
        self.die(403, &self.tr("An error occurred."), &html)
    }

    /// The person who is signed in by the `logged_in` cookie, with the token of the session.
    async fn signed_in(&self, request: &LoginRequest) -> Option<(AuthUser, String)> {
        let cookie = request.cookie(&self.auth.cookie_name(Scheme::LoggedIn))?;
        let session = self.auth.validate_cookie(self.db, cookie, Scheme::LoggedIn, self.now, request.post).await.ok().flatten()?;
        Some((session.user, session.token))
    }

    /// `wp_mail()`: e-mail is not sent by RustPress itself.
    async fn send_mail(&self, to: &str, subject: &str, message: &str) -> bool {
        match &self.mailer {
            Some(mailer) => mailer.send(to, subject, message).await,
            None => false,
        }
    }

    /// `wp_login_url( $redirect )`.
    pub(super) fn login_url_for(&self, redirect: &str) -> String {
        if redirect.is_empty() { self.login_url() } else { format!("{}?redirect_to={}", self.login_url(), rustpress_core::canonical::php_urlencode(redirect)) }
    }

    /// The page of an action: header, body, footer, scripts.
    #[allow(clippy::too_many_arguments)]
    fn render(&self, request: &LoginRequest, mut response: LoginResponse, action: &str, title: &str, message: &str, errors: &[LoginError], head_scripts: &[&str], body: &str, focus: &str, footer_scripts: &[&str]) -> LoginResponse {
        let mut page = self.header_with(action, title, message, errors, head_scripts, false);
        page.push_str(body);
        page.push_str(&self.footer_for(request, false));
        if !focus.is_empty() {
            page.push_str(&self.focus_script(focus));
        }
        if errors.first().is_some_and(|e| matches!(e.code, "empty_password" | "empty_email" | "invalid_email" | "invalidcombo" | "empty_username" | "invalid_username" | "incorrect_password" | "retrieve_password_email_failure")) {
            page.push_str(&inline_script_tag("document.querySelector('form').classList.add('shake');", &[]));
        }
        page.push_str(&self.footer_scripts_after(head_scripts, footer_scripts));
        page.push_str("\t</body>\n\t</html>\n\t");
        response.body = page;
        response
    }

    // ------------------------------------------------------------------ postpass

    /// The password form of a protected post: the cookie that unlocks it.
    pub(super) async fn postpass_action(&self, request: &LoginRequest) -> LoginResponse {
        let response = self.blank();
        let redirect_to = match request.post_field("redirect_to") {
            Some(redirect) => redirect.to_owned(),
            None => self.referer(request).unwrap_or_default(),
        };
        if !request.post {
            return self.redirect(response, request, &redirect_to, true);
        }
        let Some(password) = request.post_field("post_password") else { return self.redirect(response, request, &redirect_to, true) };
        let mut response = response;
        let secure = redirect_to.starts_with("https://");
        response.cookies.push(SetCookie {
            name: format!("wp-postpass_{}", self.auth.cookie_hash()),
            value: hash_portable(password),
            expires: Some(self.now + 10 * 86_400),
            path: self.paths().cookie,
            domain: None,
            secure,
            http_only: false,
        });
        self.redirect(response, request, &redirect_to, true)
    }

    // ------------------------------------------------------------------ the administration e-mail address

    pub(super) async fn confirm_admin_email_action(&self, request: &LoginRequest) -> LoginResponse {
        let response = self.blank();
        let Some((user, token)) = self.signed_in(request).await else { return self.redirect(response, request, &self.login_url(), true) };
        let redirect_to = request.request("redirect_to").filter(|r| !r.is_empty()).map(str::to_owned).unwrap_or_else(|| self.admin_url());
        let Ok(current) = CurrentUser::load(self.db, self.roles, &user).await else { return self.redirect(response, request, &redirect_to, true) };
        if !current.can("manage_options") {
            return self.redirect(response, request, &redirect_to, true);
        }
        let admin_email = self.site.options.string("admin_email", "");
        let remind_interval = 3 * 86_400;
        if let Some(nonce) = request.get("remind_me_later").filter(|n| !n.is_empty()) {
            if self.auth.verify_nonce(nonce, "remind_me_later_nonce", user.id, &token, self.now).is_none() {
                return self.redirect(response, request, &self.login_url(), true);
            }
            if let Some(writer) = self.writer {
                writer.update_option("admin_email_lifespan", &(self.now + remind_interval).to_string()).await.or_log();
            }
            let target = rustpress_core::urls::add_query_arg(&redirect_to, "admin_email_remind_later", "1");
            return self.redirect(response, request, &target, true);
        }
        if request.post && request.post_field("correct-admin-email").is_some_and(|v| !v.is_empty()) {
            let nonce = request.post_field("confirm_admin_email_nonce").unwrap_or_default();
            if self.auth.verify_nonce(nonce, "confirm_admin_email", user.id, &token, self.now).is_none() {
                return self.nonce_ays(request);
            }
            if let Some(writer) = self.writer {
                writer.update_option("admin_email_lifespan", &(self.now + 6 * MONTH).to_string()).await.or_log();
            }
            return self.redirect(response, request, &redirect_to, true);
        }
        let nonce = self.auth.create_nonce("confirm_admin_email", user.id, &token, self.now).unwrap_or_default();
        let remind_nonce = self.auth.create_nonce("remind_me_later_nonce", user.id, &token, self.now).unwrap_or_default();
        let change_link = rustpress_core::urls::add_query_arg(&self.site.site_url("wp-admin/options-general.php"), "highlight", "confirm_admin_email");
        let remind_link = format!(
            "{}?redirect_to={}&action=confirm_admin_email&remind_me_later={remind_nonce}",
            self.login_url(),
            rustpress_core::canonical::php_urlencode(&redirect_to)
        );
        let help_url = self.tr("https://wordpress.org/documentation/article/settings-general-screen/#email-address");
        let referer_field = format!("<input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />", esc_url(&rustpress_core::urls::remove_query_arg(&request.request_uri, "_wp_http_referer")));
        let form = format!(
            "\n\t\t<form class=\"admin-email-confirm-form\" name=\"admin-email-confirm-form\" action=\"{action}\" method=\"post\">\n\t\t\t<input type=\"hidden\" id=\"confirm_admin_email_nonce\" name=\"confirm_admin_email_nonce\" value=\"{nonce}\" />{referer_field}\t\t\t<input type=\"hidden\" name=\"redirect_to\" value=\"{redirect}\" />\n\n\t\t\t<h1 class=\"admin-email__heading\">\n\t\t\t\t{heading}\t\t\t</h1>\n\t\t\t<p class=\"admin-email__details\">\n\t\t\t\t{verify}\t\t\t\t<a href=\"{help}\" target=\"_blank\">{why}<span class=\"screen-reader-text\"> {new_tab}</span></a>\t\t\t</p>\n\t\t\t<p class=\"admin-email__details\">\n\t\t\t\t{current}\t\t\t</p>\n\t\t\t<p class=\"admin-email__details\">\n\t\t\t\t{different}\t\t\t</p>\n\n\t\t\t<div class=\"admin-email__actions\">\n\t\t\t\t<div class=\"admin-email__actions-primary\">\n\t\t\t\t\t\t\t\t\t\t<a class=\"button button-large\" href=\"{change}\">{update}</a>\n\t\t\t\t\t<input type=\"submit\" name=\"correct-admin-email\" id=\"correct-admin-email\" class=\"button button-primary button-large\" value=\"{correct}\" />\n\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t<div class=\"admin-email__actions-secondary\">\n\t\t\t\t\t\t\t\t\t\t\t\t<a href=\"{remind}\">{remind_label}</a>\n\t\t\t\t\t</div>\n\t\t\t\t\t\t\t</div>\n\t\t</form>\n\n\t\t",
            action = esc_url(&self.site.site_url("wp-login.php?action=confirm_admin_email")),
            redirect = esc_attr(&redirect_to),
            heading = self.tr("Administration email verification"),
            verify = self.tr("Please verify that the <strong>administration email</strong> for this website is still correct."),
            help = esc_url(&help_url),
            why = self.tr("Why is this important?"),
            new_tab = self.tr("(opens in a new tab)"),
            current = rustpress_core::i18n::sprintf(&self.tr("Current administration email: %s"), &[&format!("<strong>{}</strong>", esc_html(&admin_email))]),
            different = self.tr("This email may be different from your personal email address."),
            change = esc_url(&change_link),
            update = self.tr("Update"),
            correct = esc_attr(&self.tr("The email is correct")),
            remind = esc_url(&remind_link),
            remind_label = self.tr("Remind me later"),
        );
        self.render(request, response, "confirm_admin_email", &self.tr("Confirm your administration email"), "", &[], &[], &form, "", &[])
    }

    // ------------------------------------------------------------------ the lost password

    /// `retrieve_password()`: the key is saved and mailed; the errors it can end with.
    async fn retrieve_password(&self, request: &LoginRequest) -> Result<(), Vec<LoginError>> {
        let user_login = request.post_field("user_login").unwrap_or_default();
        self.mail_ctx().retrieve_password(user_login, request.remote_addr.as_deref()).await.map_err(|errors| errors.into_iter().map(|(code, message)| error(code, message)).collect())
    }

    pub(crate) fn mail_ctx(&self) -> crate::mail::MailCtx<'_> {
        crate::mail::MailCtx { site: self.site, db: self.db, writer: self.writer, now: self.now, mailer: self.mailer.as_ref() }
    }

    /// The "Log in" and "Register" links under the forms of the lost password and the reset.
    fn nav_login_register(&self) -> String {
        let register = if self.site.options.get("users_can_register").is_some_and(|v| v != "0" && !v.is_empty()) {
            format!(" | <a class=\"wp-login-register\" href=\"{}\">{}</a>", esc_url(&self.site.site_url("wp-login.php?action=register")), self.tr("Register"))
        } else {
            String::new()
        };
        format!("\n\t\t<p id=\"nav\">\n\t\t\t<a class=\"wp-login-log-in\" href=\"{}\">{}</a>\n\t\t\t{register}\t\t</p>\n\t\t", esc_url(&self.login_url()), self.tr("Log in"))
    }

    pub(super) async fn lostpassword_action(&self, request: &LoginRequest, action: &str) -> LoginResponse {
        let response = self.blank();
        let mut errors: Vec<LoginError> = Vec::new();
        if request.post {
            match self.retrieve_password(request).await {
                Ok(()) => {
                    let redirect = request.request("redirect_to").filter(|r| !r.is_empty()).unwrap_or("wp-login.php?checkemail=confirm").to_owned();
                    return self.redirect(response, request, &redirect, true);
                }
                Err(found) => errors = found,
            }
        }
        match request.get("error") {
            Some("invalidkey") => errors.push(error("invalidkey", self.tr("<strong>Error:</strong> Your password reset link appears to be invalid. Please request a new link below."))),
            Some("expiredkey") => errors.push(error("expiredkey", self.tr("<strong>Error:</strong> Your password reset link has expired. Please request a new link below."))),
            _ => {}
        }
        let redirect_to = request.request("redirect_to").unwrap_or_default();
        let user_login = if request.post { request.post_field("user_login").unwrap_or_default() } else { "" };
        let message = info_notice(&self.tr("Please enter your username or email address. You will receive an email message with instructions on how to reset your password."), &["message"]);
        let body = format!(
            "\n\t\t<form name=\"lostpasswordform\" id=\"lostpasswordform\" action=\"{action}\" method=\"post\">\n\t\t\t<p>\n\t\t\t\t<label for=\"user_login\">{label}</label>\n\t\t\t\t<input type=\"text\" name=\"user_login\" id=\"user_login\" class=\"input ltr\" value=\"{login}\" size=\"20\" autocapitalize=\"off\" autocomplete=\"username\" required=\"required\" />\n\t\t\t</p>\n\t\t\t\t\t\t<input type=\"hidden\" name=\"redirect_to\" value=\"{redirect}\" />\n\t\t\t<p class=\"submit\">\n\t\t\t\t<input type=\"submit\" name=\"wp-submit\" id=\"wp-submit\" class=\"button button-primary button-large\" value=\"{submit}\" />\n\t\t\t</p>\n\t\t</form>\n{nav}",
            action = esc_url(&self.site.site_url("wp-login.php?action=lostpassword")),
            label = self.tr("Username or Email Address"),
            login = esc_attr(user_login),
            redirect = esc_attr(redirect_to),
            submit = esc_attr(&self.tr("Get New Password")),
            nav = self.nav_login_register(),
        );
        self.render(request, response, action, &self.tr("Lost Password"), &message, &errors, &[], &body, "user_login", &[])
    }

    // ------------------------------------------------------------------ the reset of a password

    /// `check_password_reset_key()`: the user, or the code of the error.
    async fn check_password_reset_key(&self, key: &str, login: &str) -> Result<AuthUser, &'static str> {
        let key: String = key.chars().filter(char::is_ascii_alphanumeric).collect();
        if key.is_empty() || login.is_empty() {
            return Err("invalid_key");
        }
        let Some(user) = self.db.auth_user_by_login(&sanitize_user(login)).await.ok().flatten() else { return Err("invalid_key") };
        let stored = self.db.user_activation_key(user.id).await.unwrap_or_default();
        let (expiration, pass_key) = match stored.split_once(':') {
            Some((time, key)) => (time.parse::<i64>().ok().map(|t| t + RESET_KEY_LIFETIME), key.to_owned()),
            None => (None, stored.clone()),
        };
        if pass_key.is_empty() {
            return Err("invalid_key");
        }
        let correct = verify_fast_hash(&key, &pass_key);
        match expiration {
            Some(expires) if correct && self.now < expires => return Ok(user),
            Some(_) if correct => return Err("expired_key"),
            _ => {}
        }
        if rustpress_core::hash_equals(&stored, &key) || (correct && expiration.is_none()) {
            return Err("expired_key");
        }
        Err("invalid_key")
    }

    pub(super) async fn resetpass_action(&self, request: &LoginRequest, action: &str) -> LoginResponse {
        let response = self.blank();
        let rp_cookie = format!("wp-resetpass-{}", self.auth.cookie_hash());
        let rp_path = request.request_uri.split('?').next().unwrap_or_default().to_owned();
        let cookie = |value: &str, expires: Option<i64>| SetCookie { name: rp_cookie.clone(), value: value.to_owned(), expires, path: rp_path.clone(), domain: None, secure: self.https, http_only: true };
        // The link from the e-mail: keep the key in a cookie and go to the same page without it.
        if let (Some(key), Some(login)) = (request.get("key"), request.get("login")) {
            let mut response = response;
            response.cookies.push(cookie(&format!("{login}:{key}"), None));
            let mut target = request.request_uri.clone();
            for name in ["key", "login"] {
                target = rustpress_core::urls::remove_query_arg(&target, name);
            }
            return self.redirect(response, request, &target, true);
        }
        let (rp_login, rp_key, user) = match request.cookie(&rp_cookie).filter(|c| c.find(':').is_some_and(|i| i > 0)) {
            Some(value) => {
                let (login, key) = value.split_once(':').unwrap_or_default();
                let mut user = self.check_password_reset_key(key, login).await;
                if request.post_field("pass1").is_some() && request.post_field("rp_key") != Some(key) {
                    user = Err("invalid_key");
                }
                (login.to_owned(), key.to_owned(), user)
            }
            None => (String::new(), String::new(), Err("invalid_key")),
        };
        let user = match user {
            Ok(user) => user,
            Err(code) => {
                let mut response = response;
                response.cookies.push(cookie(" ", Some(self.now - 365 * 86_400)));
                let target = if code == "expired_key" { "wp-login.php?action=lostpassword&error=expiredkey" } else { "wp-login.php?action=lostpassword&error=invalidkey" };
                return self.redirect(response, request, &self.site.site_url(target), false);
            }
        };
        let mut errors: Vec<LoginError> = Vec::new();
        let mut pass1 = request.post_field("pass1").unwrap_or_default().to_owned();
        let pass2 = request.post_field("pass2").unwrap_or_default().to_owned();
        if !pass1.is_empty() {
            pass1 = pass1.trim().to_owned();
            if pass1.is_empty() {
                errors.push(error("password_reset_empty_space", self.tr("The password cannot be a space or all spaces.")));
            }
        }
        if !pass1.is_empty() && pass2.trim() != pass1 {
            errors.push(error("password_reset_mismatch", self.tr("<strong>Error:</strong> The passwords do not match.")));
        }
        if errors.is_empty() && !pass1.is_empty() {
            if let Some(writer) = self.writer {
                // `reset_password()`.
                let hash = rustpress_auth::password::hash_password(&pass1);
                writer.update_rows("users", "ID", user.id, &[("user_pass", Val::Str(hash)), ("user_activation_key", Val::Str(String::new()))]).await.or_log();
                writer.update_user_meta(user.id, "default_password_nag", "").await.or_log();
                self.mail_ctx().password_change_notification(&user.user_login, &user.user_email).await;
                let message = info_notice(&format!("{} <a href=\"{}\">{}</a>", self.tr("Your password has been reset."), esc_url(&self.login_url()), self.tr("Log in")), &["message", "reset-pass"]);
                let mut page = self.header_with(action, &self.tr("Password Reset"), &message, &[], &[], false);
                page.push_str(&self.footer_for(request, false));
                page.push_str(&self.footer_scripts(&[]));
                page.push_str("\t</body>\n\t</html>\n\t");
                let mut response = response;
                response.body = page;
                return response;
            }
            errors.push(error("no_writes", self.tr("<strong>Error:</strong> This site was started without write access.")));
        }
        let message = info_notice(&self.tr("Enter your new password below or generate one."), &["message", "reset-pass"]);
        let hint = self.tr("Hint: The password should be at least twelve characters long. To make it stronger, use upper and lower case letters, numbers, and symbols like ! \" ? $ % ^ &amp; ).");
        let body = format!(
            "\t\t<form name=\"resetpassform\" id=\"resetpassform\" action=\"{action}\" method=\"post\" autocomplete=\"off\">\n\t\t\t<input type=\"hidden\" id=\"user_login\" value=\"{login}\" autocomplete=\"off\" />\n\n\t\t\t<div class=\"user-pass1-wrap\">\n\t\t\t\t<p>\n\t\t\t\t\t<label for=\"pass1\">{new_password}</label>\n\t\t\t\t</p>\n\n\t\t\t\t<div class=\"wp-pwd\">\n\t\t\t\t\t<input type=\"password\" name=\"pass1\" id=\"pass1\" class=\"input password-input ltr\" size=\"24\" value=\"\" autocomplete=\"new-password\" spellcheck=\"false\" data-reveal=\"1\" data-pw=\"{generated}\" aria-describedby=\"pass-strength-result\" />\n\n\t\t\t\t\t<button type=\"button\" class=\"button button-secondary wp-hide-pw hide-if-no-js\" data-toggle=\"0\" aria-label=\"{hide}\">\n\t\t\t\t\t\t<span class=\"dashicons dashicons-hidden\" aria-hidden=\"true\"></span>\n\t\t\t\t\t</button>\n\t\t\t\t\t<div id=\"pass-strength-result\" class=\"hide-if-no-js\" aria-live=\"polite\">{strength}</div>\n\t\t\t\t</div>\n\t\t\t\t<div class=\"pw-weak\">\n\t\t\t\t\t<input type=\"checkbox\" name=\"pw_weak\" id=\"pw-weak\" class=\"pw-checkbox\" />\n\t\t\t\t\t<label for=\"pw-weak\">{weak}</label>\n\t\t\t\t</div>\n\t\t\t</div>\n\n\t\t\t<p class=\"user-pass2-wrap\">\n\t\t\t\t<label for=\"pass2\">{confirm}</label>\n\t\t\t\t<input type=\"password\" name=\"pass2\" id=\"pass2\" class=\"input\" size=\"20\" value=\"\" autocomplete=\"new-password\" spellcheck=\"false\" />\n\t\t\t</p>\n\n\t\t\t<p class=\"description indicator-hint\">{hint}</p>\n\n\t\t\t\t\t\t<input type=\"hidden\" name=\"rp_key\" value=\"{key}\" />\n\t\t\t<p class=\"submit reset-pass-submit\">\n\t\t\t\t<button type=\"button\" class=\"button wp-generate-pw hide-if-no-js skip-aria-expanded\">{generate}</button>\n\t\t\t\t<input type=\"submit\" name=\"wp-submit\" id=\"wp-submit\" class=\"button button-primary button-large\" value=\"{save}\" />\n\t\t\t</p>\n\t\t</form>\n{nav}",
            action = esc_url(&self.site.site_url("wp-login.php?action=resetpass")),
            login = esc_attr(&rp_login),
            new_password = self.tr("New password"),
            generated = esc_attr(&generate_password(16, true)),
            hide = esc_attr(&self.tr("Hide password")),
            strength = self.tr("Strength indicator"),
            weak = self.tr("Confirm use of weak password"),
            confirm = self.tr("Confirm new password"),
            key = esc_attr(&rp_key),
            generate = self.tr("Generate Password"),
            save = esc_attr(&self.tr("Save Password")),
            nav = self.nav_login_register(),
        );
        self.render(request, response, action, &self.tr("Reset Password"), &message, &errors, &["utils", "user-profile"], &body, "pass1", &[])
    }

    // ------------------------------------------------------------------ registration

    pub(super) async fn register_action(&self, request: &LoginRequest) -> LoginResponse {
        let response = self.blank();
        if !self.site.options.get("users_can_register").is_some_and(|v| v != "0" && !v.is_empty()) {
            return self.redirect(response, request, &self.site.site_url("wp-login.php?registration=disabled"), false);
        }
        let mut errors: Vec<LoginError> = Vec::new();
        let (mut user_login, mut user_email) = (String::new(), String::new());
        if request.post {
            user_login = request.post_field("user_login").unwrap_or_default().to_owned();
            user_email = request.post_field("user_email").unwrap_or_default().to_owned();
            match self.register_new_user(request, &user_login, &user_email).await {
                Ok(()) => {
                    let redirect = request.post_field("redirect_to").filter(|r| !r.is_empty()).unwrap_or("wp-login.php?checkemail=registered").to_owned();
                    return self.redirect(response, request, &redirect, true);
                }
                Err(found) => errors = found,
            }
        }
        let redirect_to = request.request("redirect_to").unwrap_or_default();
        let message = info_notice(&self.tr("Register For This Site"), &["message", "register"]);
        let body = format!(
            "\t\t<form name=\"registerform\" id=\"registerform\" action=\"{action}\" method=\"post\" novalidate=\"novalidate\">\n\t\t\t<p>\n\t\t\t\t<label for=\"user_login\">{username}</label>\n\t\t\t\t<input type=\"text\" name=\"user_login\" id=\"user_login\" class=\"input ltr\" value=\"{login}\" size=\"20\" autocapitalize=\"off\" autocomplete=\"username\" required=\"required\" />\n\t\t\t</p>\n\t\t\t<p>\n\t\t\t\t<label for=\"user_email\">{email_label}</label>\n\t\t\t\t<input type=\"email\" name=\"user_email\" id=\"user_email\" class=\"input\" value=\"{email}\" size=\"25\" autocomplete=\"email\" required=\"required\" />\n\t\t\t</p>\n\t\t\t\t\t\t<p id=\"reg_passmail\">\n\t\t\t\t{passmail}\t\t\t</p>\n\t\t\t<input type=\"hidden\" name=\"redirect_to\" value=\"{redirect}\" />\n\t\t\t<p class=\"submit\">\n\t\t\t\t<input type=\"submit\" name=\"wp-submit\" id=\"wp-submit\" class=\"button button-primary button-large\" value=\"{submit}\" />\n\t\t\t</p>\n\t\t</form>\n\n\t\t<p id=\"nav\">\n\t\t\t<a class=\"wp-login-log-in\" href=\"{login_url}\">{log_in}</a>\n\t\t\t | <a class=\"wp-login-lost-password\" href=\"{lost_url}\">{lost}</a>\t\t</p>\n\t\t",
            action = esc_url(&self.site.site_url("wp-login.php?action=register")),
            username = self.tr("Username"),
            login = esc_attr(&user_login),
            email_label = self.tr("Email"),
            email = esc_attr(&user_email),
            passmail = self.tr("Registration confirmation will be emailed to you."),
            redirect = esc_attr(redirect_to),
            submit = esc_attr(&self.tr("Register")),
            login_url = esc_url(&self.login_url()),
            log_in = self.tr("Log in"),
            lost_url = esc_url(&self.lostpassword_url()),
            lost = self.tr("Lost your password?"),
        );
        self.render(request, response, "register", &self.tr("Registration Form"), &message, &errors, &[], &body, "user_login", &[])
    }

    /// `register_new_user()`: the account is made, the person is told by e-mail how to choose a password.
    async fn register_new_user(&self, request: &LoginRequest, user_login: &str, user_email: &str) -> Result<(), Vec<LoginError>> {
        let mut errors: Vec<LoginError> = Vec::new();
        let sanitized = sanitize_user(user_login);
        let strict = crate::sanitize::user_login(user_login, true);
        if sanitized.is_empty() {
            errors.push(error("empty_username", self.tr("<strong>Error:</strong> Please enter a username.")));
        } else if strict != sanitized || strict.is_empty() {
            errors.push(error("invalid_username", self.tr("<strong>Error:</strong> This username is invalid because it uses illegal characters. Please enter a valid username.")));
        } else if self.db.user_by_login(&sanitized).await.ok().flatten().is_some() {
            errors.push(error("username_exists", self.tr("<strong>Error:</strong> This username is already registered. Please choose another one.")));
        }
        let user_email = user_email.to_owned();
        let mut email_shown = user_email.clone();
        if user_email.is_empty() {
            errors.push(error("empty_email", self.tr("<strong>Error:</strong> Please type your email address.")));
        } else if !crate::sanitize::is_email(&user_email) {
            errors.push(error("invalid_email", self.tr("<strong>Error:</strong> The email address is not correct.")));
            email_shown.clear();
        } else if self.db.user_by_email(&user_email).await.ok().flatten().is_some() {
            errors.push(error(
                "email_exists",
                rustpress_core::i18n::sprintf(&self.tr("<strong>Error:</strong> This email address is already registered. <a href=\"%s\">Log in</a> with this address or choose another one."), &[&esc_url(&self.login_url())]),
            ));
        }
        let _ = email_shown;
        if !errors.is_empty() {
            return Err(errors);
        }
        let fail = |this: &Self| {
            vec![error(
                "registerfail",
                rustpress_core::i18n::sprintf(&this.tr("<strong>Error:</strong> Could not register you&hellip; please contact the <a href=\"mailto:%s\">site admin</a>!"), &[&esc_attr(&this.site.options.string("admin_email", ""))]),
            )]
        };
        let Some(writer) = self.writer else { return Err(fail(self)) };
        let system = CurrentUser::system();
        let content = Content { site: self.site, db: self.db, writer, user: &system, now: self.now };
        let password = generate_password(12, false);
        let data = UserData { user_login: Some(sanitized.clone()), user_pass: Some(password), user_email: Some(user_email.clone()), ..Default::default() };
        let user_id = match content.insert_user(&data).await {
            Ok(id) if id != 0 => id,
            _ => return Err(fail(self)),
        };
        writer.update_user_meta(user_id, "default_password_nag", "1").await.or_log();
        if let Some(lang) = request.cookie("wp_lang").filter(|l| !l.is_empty()) {
            if self.languages.iter().any(|l| l.locale == lang) {
                writer.update_user_meta(user_id, "locale", lang).await.or_log();
            }
        }
        self.new_user_notification(user_id).await;
        Ok(())
    }

    /// `wp_new_user_notification()`: the site administrator hears about the new user, the new user how to log in.
    async fn new_user_notification(&self, user_id: u64) {
        self.mail_ctx().new_user_notification(user_id, "both").await;
    }

    // ------------------------------------------------------------------ messages

    pub(super) fn checkemail_action(&self, request: &LoginRequest) -> LoginResponse {
        let response = self.blank();
        let mut errors: Vec<LoginError> = Vec::new();
        let link = esc_url(&self.login_url());
        match request.get("checkemail") {
            Some("confirm") => errors.push(LoginError { code: "confirm", message: rustpress_core::i18n::sprintf(&self.tr("Check your email for the confirmation link, then visit the <a href=\"%s\">login page</a>."), &[&link]), is_message: true }),
            Some("registered") => errors.push(LoginError { code: "registered", message: rustpress_core::i18n::sprintf(&self.tr("Registration complete. Please check your email, then visit the <a href=\"%s\">login page</a>."), &[&link]), is_message: true }),
            _ => {}
        }
        self.render(request, response, "checkemail", &self.tr("Check your email"), "", &errors, &[], "", "", &[])
    }

    /// `confirmaction`: the link of a request for personal data.
    pub(super) async fn confirmaction_action(&self, request: &LoginRequest) -> LoginResponse {
        let response = self.blank();
        let Some(request_id) = request.get("request_id") else { return self.die(500, "WordPress &rsaquo; Error", &self.tr("Missing request ID.")) };
        let Some(key) = request.get("confirm_key") else { return self.die(500, "WordPress &rsaquo; Error", &self.tr("Missing confirm key.")) };
        let request_id = request_id.parse::<i64>().unwrap_or(0).max(0) as u64;
        let key = crate::sanitize::text_field(key);
        let fail = |message: String| self.die(500, "WordPress &rsaquo; Error", &message);
        let post = self.db.post(request_id).await.ok().flatten().filter(|p| p.post_type == "user_request");
        let Some(post) = post else { return fail(self.tr("Invalid personal data request.")) };
        let saved_key = post.post_password.clone();
        let modified = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_modified_gmt).unwrap_or(0);
        if saved_key.is_empty() || modified == 0 {
            return fail(self.tr("Invalid personal data request."));
        }
        if !matches!(post.post_status.as_str(), "request-pending" | "request-failed") {
            return fail(self.tr("This personal data request has expired."));
        }
        if key.is_empty() {
            return fail(self.tr("The confirmation key is missing from this personal data request."));
        }
        if !verify_fast_hash(&key, &saved_key) {
            return fail(self.tr("The confirmation key is invalid for this personal data request."));
        }
        if self.now > modified + 86_400 {
            return fail(self.tr("The confirmation key has expired for this personal data request."));
        }
        // `_wp_privacy_account_request_confirmed()`.
        if let Some(writer) = self.writer {
            let system = CurrentUser::system();
            let content = Content { site: self.site, db: self.db, writer, user: &system, now: self.now };
            writer.update_post_meta(request_id, "_wp_user_request_confirmed_timestamp", &self.now.to_string()).await.or_log();
            content.insert_post(&crate::content::PostData { id: Some(request_id), post_status: Some("request-confirmed".into()), ..Default::default() }).await.or_log();
        }
        let message = match post.post_name.as_str() {
            "export_personal_data" => format!("<p class=\"success\">{}</p><p>{}</p>", self.tr("Thanks for confirming your export request."), self.tr("The site administrator has been notified. You will receive a link to download your export via email when they fulfill your request.")),
            "remove_personal_data" => format!("<p class=\"success\">{}</p><p>{}</p>", self.tr("Thanks for confirming your erasure request."), self.tr("The site administrator has been notified. You will receive an email confirmation when they erase your data.")),
            _ => format!("<p class=\"success\">{}</p><p>{}</p>", self.tr("Action has been confirmed."), self.tr("The site administrator has been notified and will fulfill your request as soon as possible.")),
        };
        self.mail_ctx().request_confirmed_notification(request_id).await;
        let mut page = self.header_with("confirmaction", &self.tr("User action confirmed."), &message, &[], &[], false);
        page.push_str(&self.footer_for(request, false));
        page.push_str(&self.footer_scripts(&[]));
        page.push_str("\t</body>\n\t</html>\n\t");
        let mut response = response;
        response.body = page;
        response
    }
}

