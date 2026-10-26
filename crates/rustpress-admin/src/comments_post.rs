//! `wp-comments-post.php`: a visitor (or a signed-in user) sends a comment. This follows
//! `wp_handle_comment_submission()`, `wp_new_comment()`, `wp_allow_comment()` and the notifications that follow.

use rustpress_core::OrLog;
use rustpress_auth::{Auth, CurrentUser, Roles};
use rustpress_core::Site;
use rustpress_core::formatting::{esc_html, esc_url, esc_url_raw};
use rustpress_db::{Db, Val, Writer};

use crate::login::Mail;
use rustpress_auth::setcookie::SetCookie as SetCookieOut;
use crate::mail::MailCtx;
pub use rustpress_content::comment_rules::{comments_open, filter_email, filter_text, filter_url};
use rustpress_content::comment_rules::{self, NewComment};

/// A request to `wp-comments-post.php`.
#[derive(Debug, Clone, Default)]
pub struct CommentRequest {
    pub form: Vec<(String, String)>,
    pub remote_addr: Option<String>,
    pub user_agent: Option<String>,
}

impl CommentRequest {
    fn field(&self, name: &str) -> Option<&str> {
        self.form.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

/// What the script needs from the site.
pub struct CommentEnv<'a> {
    pub site: &'a Site,
    pub db: &'a Db,
    pub writer: Option<&'a Writer>,
    pub auth: &'a Auth,
    pub roles: &'a Roles,
    pub mailer: Option<std::sync::Arc<dyn Mail>>,
    pub now: i64,
    /// The signed-in user and the token of their session.
    pub user: Option<(CurrentUser, String)>,
    pub https: bool,
}

/// The answer: a page or a redirect.
#[derive(Debug, Clone, Default)]
pub struct CommentReply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub cookies: Vec<SetCookieOut>,
    pub body: String,
}

/// One entry of a `WP_Error` with its status.
struct Failure {
    message: String,
    status: u16,
}

fn failure(message: String, status: u16) -> Failure {
    Failure { message, status }
}

fn nocache() -> Vec<(String, String)> {
    vec![
        ("Expires".into(), "Wed, 11 Jan 1984 05:00:00 GMT".into()),
        ("Cache-Control".into(), "no-cache, must-revalidate, max-age=0, no-store, private".into()),
    ]
}

/// `PHP strip_tags` + `trim`.
fn strip_trim(text: &str) -> String {
    rustpress_core::formatting::strip_tags(text).trim().to_owned()
}



impl CommentEnv<'_> {
    fn tr(&self, text: &str) -> String {
        self.site.tr(text)
    }

    fn mail_ctx(&self) -> MailCtx<'_> {
        MailCtx { site: self.site, db: self.db, writer: self.writer, now: self.now, mailer: self.mailer.as_ref() }
    }

    fn die(&self, status: u16, html: &str) -> CommentReply {
        let back = format!("\n<p><a href='javascript:history.back()'>{}</a></p>", self.tr("&laquo; Back"));
        let mut headers = nocache();
        headers.push(("Content-Type".into(), "text/html; charset=UTF-8".into()));
        CommentReply { status, headers, cookies: Vec::new(), body: crate::die::page_with_back_link(self.site, &self.tr("Comment Submission Failure"), html, &back) }
    }


    /// `wp_handle_comment_submission()`.
    async fn submit(&self, request: &CommentRequest) -> Result<(rustpress_db::Comment, bool), Failure> {
        let tr = |s: &str| self.tr(s);
        let post_id = request.field("comment_post_ID").and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0).max(0) as u64;
        let mut author = request.field("author").map(strip_trim).unwrap_or_default();
        let mut email = request.field("email").map(|e| e.trim().to_owned()).unwrap_or_default();
        let mut url = request.field("url").map(|u| u.trim().to_owned()).unwrap_or_default();
        let content = request.field("comment").map(|c| c.trim().to_owned()).unwrap_or_default();
        let parent = request.field("comment_parent").and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0).unsigned_abs();
        let mut user_id = 0u64;
        if request.field("comment_parent").is_some() && parent != 0 {
            let reply_ok = self.db.comment_any(parent).await.ok().flatten().is_some_and(|c| c.comment_approved != "0" && c.comment_approved != "spam" && c.comment_approved != "trash");
            if !reply_ok {
                return Err(failure(tr("Sorry, replies to unapproved comments are not allowed."), 403));
            }
        }
        let Some(post) = self.db.post(post_id).await.ok().flatten().filter(|p| !p.comment_status.is_empty()) else { return Err(failure(String::new(), 0)) };
        // The status of an attachment is the one of its parent.
        let status = if post.post_type == "attachment" { self.db.post(post.post_parent).await.ok().flatten().map_or_else(|| post.post_status.clone(), |p| p.post_status) } else { post.post_status.clone() };
        let can_read = |user: &CurrentUser| -> bool {
            let private = if post.post_type == "page" { "read_private_pages" } else { "read_private_posts" };
            post.post_author == user.id || user.can(private) || (status == "publish" && user.can("read"))
        };
        let user = self.user.as_ref().map(|(u, _)| u);
        if status == "private" && !user.is_some_and(can_read) {
            return Err(failure(String::new(), 0));
        }
        if !comments_open(self.site, &post, self.now) {
            return Err(failure(tr("Sorry, comments are closed for this item."), 403));
        } else if status == "trash" {
            return Err(failure(String::new(), 0));
        } else if status != "publish" && status != "private" {
            // A draft: only somebody who may read it is told why.
            return Err(if user.is_some_and(can_read) { failure(tr("Sorry, comments are not allowed for this item."), 403) } else { failure(String::new(), 0) });
        } else if rustpress_blocks::content::post_password_required(&post) {
            return Err(failure(String::new(), 0));
        }
        let mut strict = true;
        if let Some((user, _)) = &self.user {
            author = if user.display_name.is_empty() { user.login.clone() } else { user.display_name.clone() };
            email = user.email.clone();
            url = self.db.user(user.id).await.ok().flatten().map(|u| u.user_url).unwrap_or_default();
            user_id = user.id;
            if user.can("unfiltered_html") {
                // Only with the nonce of the form the comment goes in as written.
                let nonce = request.field("_wp_unfiltered_html_comment").unwrap_or_default();
                let token = self.user.as_ref().map(|(_, t)| t.as_str()).unwrap_or_default();
                strict = self.auth.verify_nonce(nonce, &format!("unfiltered-html-comment_{post_id}"), user.id, token, self.now).is_none();
            }
        } else if self.site.options.string("comment_registration", "0") != "0" {
            return Err(failure(tr("Sorry, you must be logged in to comment."), 403));
        }
        if self.site.options.string("require_name_email", "1") != "0" && user_id == 0 {
            if email.is_empty() || author.is_empty() {
                return Err(failure(tr("<strong>Error:</strong> Please fill the required fields."), 200));
            } else if !crate::sanitize::is_email(&email) {
                return Err(failure(tr("<strong>Error:</strong> Please enter a valid email address."), 200));
            }
        }
        if content.is_empty() {
            return Err(failure(tr("<strong>Error:</strong> Please type your comment text."), 200));
        }
        if author.len() > 245 {
            return Err(failure(tr("<strong>Error:</strong> Your name is too long."), 200));
        }
        if email.len() > 100 {
            return Err(failure(tr("<strong>Error:</strong> Your email address is too long."), 200));
        }
        if url.len() > 200 {
            return Err(failure(tr("<strong>Error:</strong> Your URL is too long."), 200));
        }
        if content.len() > 65_525 {
            return Err(failure(tr("<strong>Error:</strong> Your comment is too long."), 200));
        }
        let new = NewComment {
            post_id,
            author,
            email,
            url,
            content,
            comment_type: "comment".into(),
            parent,
            user_id: Some(user_id),
            ip: request.remote_addr.clone().unwrap_or_default().chars().filter(|c| c.is_ascii_hexdigit() || matches!(c, ':' | '.' | ',' | ' ')).collect(),
            agent: request.user_agent.clone().unwrap_or_default().chars().take(254).collect(),
        };
        self.new_comment(new, strict, user).await
    }

    /// `wp_new_comment()`: the comment is checked, filtered and saved; the notifications follow.
    async fn new_comment(&self, mut c: NewComment, strict: bool, user: Option<&CurrentUser>) -> Result<(rustpress_db::Comment, bool), Failure> {
        let tr = |s: &str| self.tr(s);
        // A reply goes to a parent that is approved or waits.
        if c.parent != 0 {
            let parent_ok = self.db.comment_any(c.parent).await.ok().flatten().is_some_and(|p| p.comment_approved == "1" || p.comment_approved == "0");
            if !parent_ok {
                c.parent = 0;
            }
        }
        let date = self.site.date("Y-m-d H:i:s", self.now);
        let date_gmt = self.site.date_utc("Y-m-d H:i:s", self.now);
        // `wp_allow_comment()`: a duplicate, a flood.
        if comment_rules::is_duplicate(self.db, &c).await {
            return Err(failure(tr("Duplicate comment detected; it looks as though you&#8217;ve already said that!"), 409));
        }
        if comment_rules::is_flood(self.site, self.db, user, &c.ip, &c.email, self.now).await {
            return Err(failure(tr("You are posting comments too quickly. Slow down."), 429));
        }
        let mut approved = comment_rules::check_comment_data(self.site, self.db, self.roles, &c).await;
        // `wp_filter_comment()`.
        let home = self.site.settings.home.clone();
        c.agent = c.agent.clone();
        c.author = filter_text(&c.author);
        c.content = if strict { rustpress_core::kses::comment_content_filter(&c.content, &home, true) } else { rustpress_core::kses::comment_content_filter(&c.content, &home, false) };
        c.url = filter_url(&c.url);
        c.email = filter_email(&c.email);
        if approved != "trash" && approved != "spam" {
            approved = comment_rules::check_comment_data(self.site, self.db, self.roles, &c).await;
        }
        let Some(writer) = self.writer else { return Err(failure(tr("<strong>Error:</strong> The comment could not be saved. Please try again later."), 500)) };
        let fields: Vec<(&str, Val)> = vec![
            ("comment_post_ID", Val::Int(c.post_id as i64)),
            ("comment_author_IP", Val::Str(c.ip.clone())),
            ("comment_author", Val::Str(c.author.clone())),
            ("comment_author_email", Val::Str(c.email.clone())),
            ("comment_author_url", Val::Str(c.url.clone())),
            ("comment_date", Val::Str(date)),
            ("comment_date_gmt", Val::Str(date_gmt)),
            ("comment_content", Val::Str(c.content.clone())),
            ("comment_karma", Val::Int(0)),
            ("comment_approved", Val::Str(approved.clone())),
            ("comment_agent", Val::Str(c.agent.clone())),
            ("comment_type", Val::Str(c.comment_type.clone())),
            ("comment_parent", Val::Int(c.parent as i64)),
            ("user_id", Val::Int(c.user_id.unwrap_or(0) as i64)),
        ];
        let id = match writer.insert_row("comments", &fields).await {
            Ok(id) if id != 0 => id,
            _ => return Err(failure(tr("<strong>Error:</strong> The comment could not be saved. Please try again later."), 500)),
        };
        if approved == "1" {
            let system = CurrentUser::system();
            let content = crate::content::Content { site: self.site, db: self.db, writer, user: &system, now: self.now };
            content.update_comment_count(c.post_id).await.or_log();
        }
        // `comment_post`: the moderators and the author of the post are told.
        self.notify(id, user).await;
        match self.db.comment_any(id).await.ok().flatten() {
            Some(comment) => Ok((comment, true)),
            None => Err(failure(tr("<strong>Error:</strong> The comment could not be saved. Please try again later."), 500)),
        }
    }

    /// `wp_new_comment_notify_moderator()` and `wp_new_comment_notify_postauthor()`.
    async fn notify(&self, comment_id: u64, current: Option<&CurrentUser>) {
        self.mail_ctx().new_comment_notify(self.roles, comment_id, true, current.map_or(0, |u| u.id)).await;
    }

    /// The reply for a comment that was saved: the cookies and where to go.
    async fn done(&self, request: &CommentRequest, comment: &rustpress_db::Comment) -> CommentReply {
        let consent = request.field("wp-comment-cookies-consent").is_some();
        let mut cookies: Vec<SetCookieOut> = Vec::new();
        // `wp_set_comment_cookies()`: only for visitors.
        if self.user.is_none() {
            let hash = self.auth.cookie_hash().to_owned();
            let paths = rustpress_auth::setcookie::CookiePaths::from_urls(&self.site.settings.home, &self.site.settings.siteurl);
            let mk = |name: String, value: String, expires: i64| SetCookieOut { name, value, expires: Some(expires), path: paths.cookie.clone(), domain: None, secure: self.site.settings.home.starts_with("https"), http_only: false };
            if consent {
                let lifetime = self.now + 365 * 86_400;
                cookies.push(mk(format!("comment_author_{hash}"), comment.comment_author.clone(), lifetime));
                cookies.push(mk(format!("comment_author_email_{hash}"), comment.comment_author_email.clone(), lifetime));
                cookies.push(mk(format!("comment_author_url_{hash}"), esc_url(&comment.comment_author_url), lifetime));
            } else {
                let past = self.now - 365 * 86_400;
                for name in ["comment_author_", "comment_author_email_", "comment_author_url_"] {
                    cookies.push(mk(format!("{name}{hash}"), " ".into(), past));
                }
            }
        }
        let link = crate::screens::comments::comment_link_for(self.site, self.db, comment).await;
        let mut location = match request.field("redirect_to").filter(|r| !r.is_empty()) {
            Some(redirect) => format!("{redirect}#comment-{}", comment.comment_id),
            None => link,
        };
        // Without the cookies the commenter sees their own waiting comment for a moment, with this link.
        if !consent && comment.comment_approved == "0" && !comment.comment_author_email.is_empty() {
            let hash = self.auth.salts().hash(&comment.comment_date_gmt, "auth").unwrap_or_default();
            location = rustpress_core::urls::add_query_arg(&rustpress_core::urls::add_query_arg(&location, "unapproved", &comment.comment_id.to_string()), "moderation-hash", &hash);
        }
        let mut headers = nocache();
        headers.push(("Content-Type".into(), "text/html; charset=UTF-8".into()));
        let target = crate::login::sanitize_redirect(&crate::login::validate_redirect(self.site, &esc_url_raw(&location), &self.site.site_url("wp-admin/")));
        headers.push(("X-Redirect-By".into(), "WordPress".into()));
        headers.push(("Location".into(), target));
        CommentReply { status: 302, headers, cookies, body: String::new() }
    }
}

/// Handles a request to `wp-comments-post.php`.
pub async fn handle(env: &CommentEnv<'_>, request: &CommentRequest) -> CommentReply {
    match env.submit(request).await {
        Ok((comment, _)) => env.done(request, &comment).await,
        Err(failure) if failure.status != 0 => env.die(failure.status, &format!("<p>{}</p>", failure.message)),
        // Errors without a status end the script without a word.
        Err(_) => CommentReply { status: 200, headers: nocache(), cookies: Vec::new(), body: String::new() },
    }
}

#[allow(dead_code)]
fn unused(_: &str) -> String {
    esc_html("")
}
