//! The e-mail WordPress sends on its own: new users, changed passwords, the requests for personal data. The texts
//! are those of WordPress (`mail_texts`); what is sent is decided by what the site has (an SMTP server or none).

use rustpress_core::OrLog;
use std::sync::Arc;

use rustpress_auth::password::fast_hash;
use rustpress_core::Site;
use rustpress_db::{Db, Val, Writer};

use crate::login::Mail;
use crate::mail_texts;

/// What the messages need: the site, the database, a way to send.
pub struct MailCtx<'a> {
    pub site: &'a Site,
    pub db: &'a Db,
    pub writer: Option<&'a Writer>,
    pub now: i64,
    pub mailer: Option<&'a Arc<dyn Mail>>,
}

/// `wp_specialchars_decode( $text, ENT_QUOTES )`.
pub fn specialchars_decode(text: &str) -> String {
    text.replace("&#039;", "'").replace("&#39;", "'").replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

impl MailCtx<'_> {
    pub(crate) fn tr(&self, text: &str) -> String {
        self.site.tr(text)
    }

    /// The title of the site as plain text.
    pub fn sitename(&self) -> String {
        specialchars_decode(&self.site.settings.blogname)
    }

    /// `wp_mail()`.
    pub async fn send(&self, to: &str, subject: &str, message: &str) -> bool {
        match self.mailer {
            Some(mailer) => mailer.send(to, &specialchars_decode(subject), message).await,
            None => false,
        }
    }

    /// `wp_mail()` with the header lines.
    pub async fn send_with_headers(&self, to: &str, subject: &str, message: &str, headers: &str) -> bool {
        match self.mailer {
            Some(mailer) => mailer.send_with_headers(to, &specialchars_decode(subject), message, headers).await,
            None => false,
        }
    }

    fn login_url(&self) -> String {
        self.site.site_url("wp-login.php")
    }

    /// `get_password_reset_key()`: a new key, saved hashed with its time.
    pub async fn password_reset_key(&self, user_id: u64) -> Option<String> {
        let writer = self.writer?;
        let key = crate::sanitize::random_password(20, false);
        let hashed = format!("{}:{}", self.now, fast_hash(&key));
        writer.update_rows("users", "ID", user_id, &[("user_activation_key", Val::Str(hashed))]).await.ok()?;
        Some(key)
    }

    /// `network_site_url( 'wp-login.php?login=...&key=...&action=rp' )`.
    pub fn reset_url(&self, login: &str, key: &str) -> String {
        format!("{}?login={}&key={key}&action=rp", self.login_url(), rustpress_core::canonical::rawurlencode(login))
    }

    /// `retrieve_password()`: the key is saved and mailed. The errors are `(code, message)`.
    pub async fn retrieve_password(&self, user_login: &str, requester_ip: Option<&str>) -> Result<(), Vec<(&'static str, String)>> {
        let tr = |s: &str| self.tr(s);
        let user_login = user_login.trim().to_owned();
        let mut errors: Vec<(&'static str, String)> = Vec::new();
        let by_login = |login: String| async move { self.db.auth_user_by_login(&crate::login::sanitize_user(&login)).await.ok().flatten() };
        let mut found = None;
        if user_login.is_empty() {
            errors.push(("empty_username", tr("<strong>Error:</strong> Please enter a username or email address.")));
        } else if user_login.find('@').is_some_and(|i| i > 0) {
            found = self.db.auth_user_by_email(&user_login).await.ok().flatten();
            if found.is_none() {
                found = by_login(user_login.clone()).await;
            }
            if found.is_none() {
                errors.push(("invalid_email", tr("<strong>Error:</strong> There is no account with that username or email address.")));
            }
        } else {
            found = by_login(user_login.clone()).await;
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let Some(user) = found else { return Err(vec![("invalidcombo", tr("<strong>Error:</strong> There is no account with that username or email address."))]) };
        let Some(key) = self.password_reset_key(user.id).await else { return Err(vec![("no_password_key_update", tr("Could not save password reset key to database."))]) };
        let site_name = self.sitename();
        let mut message = format!("{}\r\n\r\n", tr("Someone has requested a password reset for the following account:"));
        message.push_str(&format!("{}\r\n\r\n", rustpress_core::i18n::sprintf(&tr("Site Name: %s"), &[&site_name])));
        message.push_str(&format!("{}\r\n\r\n", rustpress_core::i18n::sprintf(&tr("Username: %s"), &[&user.user_login])));
        message.push_str(&format!("{}\r\n\r\n", tr("If this was a mistake, ignore this email and nothing will happen.")));
        message.push_str(&format!("{}\r\n\r\n", tr("To reset your password, visit the following address:")));
        message.push_str(&format!("{}&wp_lang={}\r\n\r\n", self.reset_url(&user.user_login, &key), self.site.locale()));
        if let Some(ip) = requester_ip.filter(|ip| !ip.is_empty()) {
            message.push_str(&format!("{}\r\n", rustpress_core::i18n::sprintf(&tr("This password reset request originated from the IP address %s."), &[ip])));
        }
        let title = rustpress_core::i18n::sprintf(&tr("[%s] Password Reset"), &[&site_name]);
        if !self.send(&user.user_email, &title, &message).await {
            return Err(vec![(
                "retrieve_password_email_failure",
                rustpress_core::i18n::sprintf(
                    &tr("<strong>Error:</strong> The email could not be sent. Your site may not be correctly configured to send emails. <a href=\"%s\">Get support for resetting your password</a>."),
                    &[&rustpress_core::formatting::esc_url(&tr("https://wordpress.org/documentation/article/reset-your-password/"))],
                ),
            )]);
        }
        Ok(())
    }

    /// `wp_new_user_notification()`: `notify` is `admin`, `user`, `both` or empty (the administrator only).
    pub async fn new_user_notification(&self, user_id: u64, notify: &str) {
        if !["user", "admin", "both", ""].contains(&notify) {
            return;
        }
        let Ok(Some(user)) = self.db.user(user_id).await else { return };
        let blogname = self.sitename();
        let tr = |s: &str| self.tr(s);
        if notify != "user" {
            let mut message = format!("{}\r\n\r\n", rustpress_core::i18n::sprintf(&tr("New user registration on your site %s:"), &[&blogname]));
            message.push_str(&format!("{}\r\n\r\n", rustpress_core::i18n::sprintf(&tr("Username: %s"), &[&user.user_login])));
            message.push_str(&format!("{}\r\n", rustpress_core::i18n::sprintf(&tr("Email: %s"), &[&user.user_email])));
            let admin_email = self.site.options.string("admin_email", "");
            let subject = rustpress_core::i18n::sprintf(&tr("[%s] New User Registration"), &[&blogname]);
            self.send(&admin_email, &subject, &message).await;
        }
        if notify == "admin" || notify.is_empty() {
            return;
        }
        let Some(key) = self.password_reset_key(user_id).await else { return };
        let message = format!("{}\r\n\r\n{}\r\n", tr("To set your password, visit the following address:"), self.reset_url(&user.user_login, &key));
        let subject = rustpress_core::i18n::sprintf(&tr("[%s] Login Details"), &[&blogname]);
        self.send(&user.user_email, &subject, &message).await;
    }

    /// `wp_password_change_notification()`: the administrator hears that a user chose a new password.
    pub async fn password_change_notification(&self, login: &str, email: &str) {
        let admin_email = self.site.options.string("admin_email", "");
        if email.eq_ignore_ascii_case(&admin_email) {
            return;
        }
        let message = format!("{}\r\n", rustpress_core::i18n::sprintf(&self.tr("Password changed for user: %s"), &[login]));
        let subject = rustpress_core::i18n::sprintf(&self.tr("[%s] Password Changed"), &[&self.sitename()]);
        self.send(&admin_email, &subject, &message).await;
    }

    /// `wp_user_request_action_description()`.
    fn action_description(&self, action: &str) -> String {
        match action {
            "export_personal_data" => self.tr("Export Personal Data"),
            "remove_personal_data" => self.tr("Erase Personal Data"),
            other => rustpress_core::i18n::sprintf(&self.tr("Confirm the \"%s\" action"), &[other]),
        }
    }

    /// `wp_send_user_request()`: a new confirmation key, and the e-mail with the link.
    pub async fn send_user_request(&self, request_id: u64) -> Result<(), String> {
        let Some(request) = self.db.post(request_id).await.ok().flatten().filter(|p| p.post_type == "user_request") else { return Err(self.tr("Invalid personal data request.")) };
        let writer = self.writer.ok_or_else(|| self.tr("Invalid personal data request."))?;
        // `wp_generate_user_request_key()`.
        let key = crate::sanitize::random_password(20, false);
        let system = rustpress_auth::CurrentUser::system();
        let content = crate::content::Content { site: self.site, db: self.db, writer, user: &system, now: self.now };
        let data = crate::content::PostData { id: Some(request_id), post_status: Some("request-pending".into()), post_password: Some(fast_hash(&key)), ..Default::default() };
        content.insert_post(&data).await.map_err(|e| e.to_string())?;
        let sitename = self.sitename();
        let description = self.action_description(&request.post_name);
        let confirm_url = format!("{}?action=confirmaction&request_id={request_id}&confirm_key={key}", self.login_url());
        let subject = rustpress_core::i18n::sprintf(&self.tr("[%1$s] Confirm Action: %2$s"), &[&sitename, &description]);
        let content = self
            .tr(mail_texts::CONFIRM_ACTION)
            .replace("###DESCRIPTION###", &description)
            .replace("###CONFIRM_URL###", &rustpress_core::formatting::esc_url_raw(&confirm_url))
            .replace("###EMAIL###", &request.post_title)
            .replace("###SITENAME###", &sitename)
            .replace("###SITEURL###", &rustpress_core::formatting::esc_url_raw(&self.site.settings.home));
        if self.send(&request.post_title, &subject, &content).await { Ok(()) } else { Err(self.tr("Unable to send personal data export confirmation email.")) }
    }

    /// `_wp_privacy_send_request_confirmation_notification()`: the administrator hears that a request was confirmed.
    pub async fn request_confirmed_notification(&self, request_id: u64) {
        let Some(request) = self.db.post(request_id).await.ok().flatten().filter(|p| p.post_type == "user_request" && p.post_status == "request-confirmed") else { return };
        if self.db.post_meta_value(request_id, "_wp_admin_notified").await.ok().flatten().is_some_and(|v| !v.is_empty() && v != "0") {
            return;
        }
        let manage_url = if request.post_name == "export_personal_data" { self.site.site_url("wp-admin/export-personal-data.php") } else { self.site.site_url("wp-admin/erase-personal-data.php") };
        let description = self.action_description(&request.post_name);
        let sitename = self.sitename();
        let subject = rustpress_core::i18n::sprintf(&self.tr("[%1$s] Action Confirmed: %2$s"), &[&sitename, &description]);
        let content = self
            .tr(mail_texts::REQUEST_CONFIRMED_ADMIN)
            .replace("###SITENAME###", &sitename)
            .replace("###USER_EMAIL###", &request.post_title)
            .replace("###DESCRIPTION###", &description)
            .replace("###MANAGE_URL###", &rustpress_core::formatting::esc_url_raw(&manage_url))
            .replace("###SITEURL###", &rustpress_core::formatting::esc_url_raw(&self.site.settings.home));
        let admin_email = self.site.options.string("admin_email", "");
        if self.send(&admin_email, &subject, &content).await
            && let Some(writer) = self.writer
        {
            writer.update_post_meta(request_id, "_wp_admin_notified", "1").await.or_log();
        }
    }

    /// `wp_privacy_send_personal_data_export_email()`.
    pub async fn export_email(&self, request_id: u64, exports_url: &str) -> Result<(), String> {
        let Some(request) = self.db.post(request_id).await.ok().flatten().filter(|p| p.post_type == "user_request" && p.post_name == "export_personal_data") else {
            return Err(self.tr("Invalid request ID when sending personal data export email."));
        };
        let expiration_date = self.site.date(&self.site.options.string("date_format", "F j, Y"), self.now + 3 * 86_400);
        let file = self.db.post_meta_value(request_id, "_export_file_name").await.ok().flatten().unwrap_or_default();
        let url = format!("{exports_url}{file}");
        let sitename = self.sitename();
        let subject = rustpress_core::i18n::sprintf(&self.tr("[%s] Personal Data Export"), &[&sitename]);
        let content = self
            .tr(mail_texts::EXPORT_EMAIL)
            .replace("###EXPIRATION###", &expiration_date)
            .replace("###LINK###", &rustpress_core::formatting::esc_url_raw(&url))
            .replace("###EMAIL###", &request.post_title)
            .replace("###SITENAME###", &sitename)
            .replace("###SITEURL###", &rustpress_core::formatting::esc_url_raw(&self.site.settings.home));
        if self.send(&request.post_title, &subject, &content).await { Ok(()) } else { Err(self.tr("Unable to send personal data export email.")) }
    }

    /// `_wp_privacy_send_erasure_fulfillment_notification()`.
    pub async fn erasure_fulfillment_notification(&self, request_id: u64) {
        let Some(request) = self.db.post(request_id).await.ok().flatten().filter(|p| p.post_type == "user_request" && p.post_status == "request-completed") else { return };
        if self.db.post_meta_value(request_id, "_wp_user_notified").await.ok().flatten().is_some_and(|v| !v.is_empty() && v != "0") {
            return;
        }
        let sitename = self.sitename();
        let policy_page: u64 = self.site.options.string("wp_page_for_privacy_policy", "0").parse().unwrap_or(0);
        let policy_url = match self.db.post(policy_page).await.ok().flatten().filter(|p| policy_page != 0 && p.post_status == "publish") {
            Some(page) => self.site.permalink(&page).await.unwrap_or_default(),
            None => String::new(),
        };
        let subject = rustpress_core::i18n::sprintf(&self.tr("[%s] Erasure Request Fulfilled"), &[&sitename]);
        let text = if policy_url.is_empty() { mail_texts::ERASURE_PLAIN } else { mail_texts::ERASURE_WITH_POLICY };
        let content = self
            .tr(text)
            .replace("###SITENAME###", &sitename)
            .replace("###PRIVACY_POLICY_URL###", &policy_url)
            .replace("###SITEURL###", &rustpress_core::formatting::esc_url_raw(&self.site.settings.home));
        if self.send(&request.post_title, &subject, &content).await
            && let Some(writer) = self.writer
        {
            writer.update_post_meta(request_id, "_wp_user_notified", "1").await.or_log();
        }
    }
}
