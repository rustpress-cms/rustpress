//! `wp-comments-post.php`: a visitor (or a signed-in user) sends a comment. This follows
//! `wp_handle_comment_submission()`, `wp_new_comment()`, `wp_allow_comment()` and the notifications that follow.

use rustpress_auth::{Auth, CurrentUser, Roles};
use rustpress_core::Site;
use rustpress_core::formatting::{esc_html, esc_url, esc_url_raw};
use rustpress_db::{Db, Val, Writer};

use crate::login::Mail;
use rustpress_auth::setcookie::SetCookie as SetCookieOut;
use crate::mail::MailCtx;
use crate::mail_comments::PostAuthor;

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

/// `comments_open()`: open, and not closed for old posts.
pub fn comments_open(site: &Site, post: &rustpress_db::Post, now: i64) -> bool {
    if post.comment_status != "open" {
        return false;
    }
    if site.options.string("close_comments_for_old_posts", "0") != "0" && post.post_type == "post" {
        let days = site.options.string("close_comments_days_old", "14").parse::<i64>().unwrap_or(0);
        if days != 0
            && post.post_date_gmt != "0000-00-00 00:00:00"
            && let Some(posted) = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date_gmt)
            && now - posted > days * 86_400
        {
            return false;
        }
    }
    true
}

/// A word list of the settings (`moderation_keys`, `disallowed_keys`): one regular expression per line.
fn matches_list(list: &str, texts: &[&str]) -> bool {
    for word in list.lines().map(str::trim).filter(|w| !w.is_empty()) {
        let Ok(pattern) = regex::Regex::new(&format!("(?iu){}", regex::escape(word))) else { continue };
        if texts.iter().any(|text| pattern.is_match(text)) {
            return true;
        }
    }
    false
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

    /// `check_comment()`: whether a comment may appear at once.
    async fn check_comment(&self, author: &str, email: &str, url: &str, content: &str, ip: &str, agent: &str, comment_type: &str, user_id: Option<u64>) -> bool {
        let options = &self.site.options;
        if options.string("comment_moderation", "0") == "1" {
            return false;
        }
        // Links that were not written as links count, too.
        let rendered = rustpress_blocks::content::comment_text(self.site, content);
        let max_links = options.string("comment_max_links", "2").parse::<usize>().unwrap_or(0);
        if max_links > 0 {
            static LINK: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            let count = LINK.get_or_init(|| regex::Regex::new(r"(?i)<a [^>]*href").expect("pattern")).find_iter(&rendered).count();
            if count >= max_links {
                return false;
            }
        }
        let keys = options.string("moderation_keys", "");
        if !keys.trim().is_empty() && matches_list(keys.trim(), &[author, email, url, &rendered, ip, agent]) {
            return false;
        }
        if options.string("comment_previously_approved", "1") == "1" {
            if comment_type != "trackback" && comment_type != "pingback" && !author.is_empty() && !email.is_empty() {
                let known = self.db.user_by_email(&crate::sanitize::sanitize_email(email)).await.ok().flatten();
                let ok = match known {
                    Some(user) => self.db.has_approved_comment(Some(user.id), author, email).await.unwrap_or(false),
                    None => self.db.has_approved_comment(None, author, email).await.unwrap_or(false),
                };
                return ok && (keys.trim().is_empty() || !email.contains(keys.trim()));
            }
            return false;
        }
        let _ = user_id;
        false
    }

    /// `wp_check_comment_data()`: `1`, `0`, or `trash`/`spam` for a comment the disallowed list names.
    async fn check_comment_data(&self, c: &NewComment) -> String {
        if let Some(user_id) = c.user_id.filter(|id| *id != 0)
            && let Some(author) = self.db.auth_user_by_id(user_id).await.ok().flatten()
            && let Ok(user) = CurrentUser::load(self.db, self.roles, &author).await
        {
            let post_author = self.db.post(c.post_id).await.ok().flatten().map_or(0, |p| p.post_author);
            if user_id == post_author || user.can("moderate_comments") {
                return "1".into();
            }
        }
        let mut approved = if self.check_comment(&c.author, &c.email, &c.url, &c.content, &c.ip, &c.agent, &c.comment_type, c.user_id).await { "1".to_owned() } else { "0".to_owned() };
        let disallowed = self.site.options.string("disallowed_keys", "");
        if !disallowed.trim().is_empty() {
            let without_html = rustpress_core::formatting::strip_tags(&c.content);
            if matches_list(disallowed.trim(), &[&c.author, &c.email, &c.url, &c.content, &without_html, &c.ip, &c.agent]) {
                approved = "trash".into();
            }
        }
        approved
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
        if self.db.duplicate_comment(c.post_id, c.parent, &c.author, &c.email, &c.content).await.ok().flatten().is_some() {
            return Err(failure(tr("Duplicate comment detected; it looks as though you&#8217;ve already said that!"), 409));
        }
        let moderator = user.is_some_and(|u| u.can("manage_options") || u.can("moderate_comments"));
        if !moderator {
            let hour_ago = self.site.date_utc("Y-m-d H:i:s", self.now - 3600);
            let by_user = user.map(|u| u.id);
            let last = self.db.last_comment_since(by_user, &c.ip, &c.email, &hour_ago).await;
            if let Some(last) = last.ok().flatten() {
                let last = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&last).unwrap_or(0);
                if self.now - last < 15 {
                    return Err(failure(tr("You are posting comments too quickly. Slow down."), 429));
                }
            }
        }
        let mut approved = self.check_comment_data(&c).await;
        // `wp_filter_comment()`.
        let home = self.site.settings.home.clone();
        c.agent = c.agent.clone();
        c.author = filter_text(&c.author);
        c.content = if strict { rustpress_core::kses::comment_content_filter(&c.content, &home, true) } else { rustpress_core::kses::comment_content_filter(&c.content, &home, false) };
        c.url = filter_url(&c.url);
        c.email = filter_email(&c.email);
        if approved != "trash" && approved != "spam" {
            approved = self.check_comment_data(&c).await;
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
            let _ = content.update_comment_count(c.post_id).await;
        }
        // `comment_post`: the moderators and the author of the post are told.
        self.notify(id, &approved, user).await;
        match self.db.comment_any(id).await.ok().flatten() {
            Some(comment) => Ok((comment, true)),
            None => Err(failure(tr("<strong>Error:</strong> The comment could not be saved. Please try again later."), 500)),
        }
    }

    /// `wp_new_comment_notify_moderator()` and `wp_new_comment_notify_postauthor()`.
    async fn notify(&self, comment_id: u64, approved: &str, current: Option<&CurrentUser>) {
        let Some(comment) = self.db.comment_any(comment_id).await.ok().flatten() else { return };
        let Some(post) = self.db.post(comment.comment_post_id).await.ok().flatten() else { return };
        let author_user = self.db.user(post.post_author).await.ok().flatten();
        let author_caps = match self.db.auth_user_by_id(post.post_author).await.ok().flatten() {
            Some(auth_user) => CurrentUser::load(self.db, self.roles, &auth_user).await.ok(),
            None => None,
        };
        let edit_cap = if post.post_type == "page" { "edit_pages" } else { "edit_posts" };
        let post_author = author_user.as_ref().map(|user| PostAuthor {
            user,
            can_edit_comments: author_caps.as_ref().is_some_and(|c| c.can(edit_cap)),
            can_read: author_caps.as_ref().is_some_and(|c| c.can("read")),
        });
        let mail = self.mail_ctx();
        if approved == "0" {
            mail.notify_moderator(comment_id, post_author.as_ref()).await;
        }
        if approved == "1" && self.site.options.string("comments_notify", "1") != "0" {
            mail.notify_postauthor(comment_id, post_author.as_ref(), current.map_or(0, |u| u.id)).await;
        }
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

/// A comment on its way in.
struct NewComment {
    post_id: u64,
    author: String,
    email: String,
    url: String,
    content: String,
    comment_type: String,
    parent: u64,
    user_id: Option<u64>,
    ip: String,
    agent: String,
}

/// `pre_comment_author_name`: `sanitize_text_field`, `wp_filter_kses`, `_wp_specialchars`.
fn filter_text(text: &str) -> String {
    let text = rustpress_core::kses::kses_data(&crate::sanitize::text_field(text));
    rustpress_core::formatting::specialchars_noquotes(&text)
}

/// `pre_comment_author_email`: `trim`, `sanitize_email`, `wp_filter_kses`.
fn filter_email(email: &str) -> String {
    rustpress_core::kses::kses_data(&crate::sanitize::sanitize_email(email.trim()))
}

/// `pre_comment_author_url`: `wp_strip_all_tags`, `sanitize_url`, `wp_filter_kses`.
fn filter_url(url: &str) -> String {
    let url = rustpress_core::formatting::strip_tags(url);
    let url = if url.trim().is_empty() { String::new() } else { esc_url_raw(url.trim()) };
    rustpress_core::kses::kses_data(&url)
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
