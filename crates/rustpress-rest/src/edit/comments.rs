//! Writes of `wp/v2/comments` (`WP_REST_Comments_Controller::create_item()`, `update_item()`, `delete_item()`) with
//! their permission checks. Notifications by mail (notes) are not sent: the REST API has no mailer.

use rustpress_content::comment_rules::{self, NewComment};
use rustpress_db::{Comment, Val};
use serde_json::{Value, json};

use super::Args;
use crate::error::{Msg, RestError};
use crate::params::Params;
use crate::{Outcome, Resp, Rest, Result};

/// `sanitize_key()`.
fn sanitize_key(value: &str) -> String {
    value.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-')).collect()
}

/// `rest_is_ip_address()`.
fn is_ip_address(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>().is_ok()
}

/// What `prepare_item_for_database()` makes of the request.
#[derive(Default)]
struct Prepared {
    content: Option<String>,
    post_id: Option<u64>,
    parent: Option<u64>,
    user_id: Option<u64>,
    author: Option<String>,
    email: Option<String>,
    url: Option<String>,
    ip: String,
    agent: Option<String>,
    /// `(local, gmt)`.
    date: Option<(String, String)>,
}

macro_rules! bail {
    ($e:expr) => {
        return Ok(Err($e))
    };
}

impl Rest<'_> {
    /// `POST /wp/v2/comments`, `POST|PUT|PATCH|DELETE /wp/v2/comments/<id>`.
    pub(crate) async fn comments_write(&self, id: Option<&str>, method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        // The schema's checks of the arguments come first (`author_email`: an address or nothing).
        if let Some(email) = args.str("author_email").filter(|e| !e.is_empty())
            && !rustpress_content::sanitize::is_email(&email)
        {
            bail!(RestError::invalid_param("author_email", "Invalid email address.", "rest_invalid_email"));
        }
        for name in ["date", "date_gmt"] {
            if args.str(name).is_some_and(|d| !d.is_empty() && !Self::valid_date(&d)) {
                bail!(RestError::invalid_param(name, "Invalid date.", "rest_invalid_date"));
            }
        }
        match id {
            None if method == "POST" => self.create_comment(args, params).await,
            None => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
            Some(id) if method == "DELETE" => self.delete_comment(id, args).await,
            Some(id) => self.update_comment(id, args, params).await,
        }
    }

    /// `check_edit_permission()`: moderators, and those who may edit the comment's post.
    async fn check_edit_comment(&self, comment: &Comment) -> Result<bool> {
        let Some(user) = self.user().filter(|u| u.id != 0) else { return Ok(false) };
        if user.can("moderate_comments") {
            return Ok(true);
        }
        let post = self.site.db.post(comment.comment_post_id).await?;
        Ok(user.can_edit_comment(post.as_ref()))
    }

    /// `check_target_post_permission()`: the post a comment or note is put on.
    async fn check_target_post(&self, post_id: u64, params: &Params, is_note: bool) -> Result<Option<RestError>> {
        let no_post = || RestError::new("rest_comment_invalid_post_id", "Sorry, you are not allowed to create this comment without a post.", 403);
        if post_id == 0 {
            return Ok(Some(no_post()));
        }
        let post = self.site.db.post(post_id).await?;
        let can_edit = post.as_ref().is_some_and(|p| self.can_edit_post(p));
        let allowed = if is_note { can_edit } else { self.can("moderate_comments") || can_edit };
        if !allowed {
            return Ok(Some(if is_note {
                self.forbidden("rest_cannot_create_note", "Sorry, you are not allowed to create notes for this post.")
            } else {
                self.forbidden("rest_cannot_edit", "Sorry, you are not allowed to edit this comment.")
            }));
        }
        let Some(post) = post else { return Ok(Some(no_post())) };
        if !is_note {
            return Ok(None);
        }
        if !matches!(post.post_type.as_str(), "post" | "page") {
            return Ok(Some(RestError::new("rest_comment_not_supported_post_type", "Sorry, this post type does not support notes.", 403)));
        }
        if post.post_status == "trash" {
            return Ok(Some(RestError::new("rest_comment_trash_post", "Sorry, you are not allowed to create a comment on this post.", 403)));
        }
        if !self.check_read_post_permission(&post, params, false, false).await {
            return Ok(Some(self.forbidden("rest_cannot_read_post", "Sorry, you are not allowed to read the post for this comment.")));
        }
        Ok(None)
    }

    /// `prepare_item_for_database()`.
    async fn prepare_comment_for_database(&self, args: &Args) -> Result<std::result::Result<Prepared, RestError>> {
        let content = match args.get("content") {
            Some(Value::String(content)) => Some(content.trim().to_owned()),
            Some(Value::Object(object)) => object.get("raw").and_then(Value::as_str).map(|raw| raw.trim().to_owned()),
            _ => None,
        };
        let mut prepared = Prepared { content, post_id: args.u64("post"), parent: args.u64("parent"), ..Prepared::default() };
        if args.has("author") {
            let user = match args.u64("author").filter(|id| *id != 0) {
                Some(id) => self.site.db.user(id).await?,
                None => None,
            };
            let Some(user) = user else { return Ok(Err(RestError::new("rest_comment_author_invalid", "Invalid comment author ID.", 400))) };
            prepared.user_id = Some(user.id);
            prepared.author = Some(user.display_name.clone());
            prepared.email = Some(user.user_email.clone());
            prepared.url = Some(user.user_url.clone());
        }
        // `author_name` and `author_user_agent` go through `sanitize_text_field`.
        if let Some(name) = args.str("author_name") {
            prepared.author = Some(rustpress_content::sanitize::text_field(&name));
        }
        if let Some(email) = args.str("author_email") {
            prepared.email = Some(email);
        }
        if let Some(url) = args.str("author_url") {
            prepared.url = Some(url);
        }
        let remote = self.request.as_ref().and_then(|r| r.remote_addr.clone()).unwrap_or_default();
        prepared.ip = match args.str("author_ip") {
            Some(ip) if self.can("moderate_comments") => ip,
            _ if !remote.is_empty() && is_ip_address(&remote) => remote,
            _ => "127.0.0.1".into(),
        };
        prepared.agent = match args.str("author_user_agent").filter(|a| !a.is_empty()) {
            Some(agent) => Some(rustpress_content::sanitize::text_field(&agent)),
            None => self.request.as_ref().and_then(|r| r.user_agent.clone()).filter(|a| !a.is_empty()),
        };
        // `rest_get_date_with_gmt()`
        if let Some(date) = args.str("date").filter(|d| !d.is_empty()) {
            let local: String = date.replace('T', " ").chars().take(19).collect();
            prepared.date = Some((local.clone(), self.gmt_from_local(&local)));
        } else if let Some(date) = args.str("date_gmt").filter(|d| !d.is_empty()) {
            let gmt: String = date.replace('T', " ").chars().take(19).collect();
            if let Some(ts) = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&gmt) {
                prepared.date = Some((self.site.date("Y-m-d H:i:s", ts), gmt));
            }
        }
        Ok(Ok(prepared))
    }

    /// `wp_check_comment_data_max_lengths()`.
    fn check_max_lengths(author: &str, email: &str, url: &str, content: &str) -> Option<RestError> {
        let code = if author.len() > 245 {
            "comment_author_column_length"
        } else if email.len() > 100 {
            "comment_author_email_column_length"
        } else if url.len() > 200 {
            "comment_author_url_column_length"
        } else if content.len() > 65_525 {
            "comment_content_column_length"
        } else {
            return None;
        };
        Some(RestError::new(code, "Comment field exceeds maximum length allowed.", 400))
    }

    /// `check_is_comment_content_allowed()`: no empty comment, except a note that resolves or reopens.
    fn content_allowed(content: Option<&str>, is_note: bool, args: &Args) -> bool {
        let Some(content) = content else { return true };
        if is_note && args.get("meta").and_then(|m| m.get("_wp_note_status")).and_then(Value::as_str).is_some_and(|s| matches!(s, "resolved" | "reopen")) {
            return true;
        }
        !content.is_empty()
    }

    /// The `pre_comment_content` filters: `wp_filter_kses` unless the comment's user may post unfiltered HTML.
    async fn filter_comment_content(&self, content: &str, user_id: u64) -> Result<String> {
        let unfiltered = if user_id == 0 {
            false
        } else if self.user().is_some_and(|u| u.id == user_id) {
            self.can("unfiltered_html")
        } else {
            self.user_can(user_id, "unfiltered_html").await?
        };
        Ok(rustpress_core::kses::comment_content_filter(content, &self.site.settings.home, !unfiltered))
    }

    /// `user_can( $user_id, $capability )` for another user.
    pub(crate) async fn user_can(&self, user_id: u64, capability: &str) -> Result<bool> {
        let Some(roles) = self.access.as_ref().and_then(|a| a.roles) else { return Ok(false) };
        let Some(author) = self.site.db.auth_user_by_id(user_id).await? else { return Ok(false) };
        Ok(rustpress_auth::CurrentUser::load(&self.site.db, roles, &author).await.is_ok_and(|u| u.can(capability)))
    }

    /// `meta[_wp_note_status]`: `resolved` or `reopen`, for those who may edit the comment.
    async fn update_comment_meta(&self, comment: &Comment, args: &Args) -> Result<Option<RestError>> {
        let Some(meta) = args.get("meta").and_then(Value::as_object) else { return Ok(None) };
        let Some(value) = meta.get("_wp_note_status") else { return Ok(None) };
        let writer = match self.writer() {
            Ok(w) => w,
            Err(e) => return Ok(Some(e)),
        };
        match value {
            Value::Null => {
                writer.delete_rows_where("commentmeta", &format!("comment_id = {} AND meta_key = '_wp_note_status'", comment.comment_id)).await?;
            }
            Value::String(status) if matches!(status.as_str(), "resolved" | "reopen") => {
                writer.delete_rows_where("commentmeta", &format!("comment_id = {} AND meta_key = '_wp_note_status'", comment.comment_id)).await?;
                writer
                    .insert_row("commentmeta", &[("comment_id", Val::Int(comment.comment_id as i64)), ("meta_key", Val::Str("_wp_note_status".into())), ("meta_value", Val::Str(status.clone()))])
                    .await?;
            }
            _ => return Ok(Some(RestError::invalid_param("meta", Msg::new("%1$s is not one of %2$l.").with(&["meta[_wp_note_status]"]).with_list(&["resolved", "reopen"]), "rest_not_in_enum"))),
        }
        Ok(None)
    }

    /// `handle_status_param()`: the status a request asks for.
    async fn handle_comment_status(&self, status: &str, comment: &Comment) -> Result<bool> {
        let old = match comment.comment_approved.as_str() {
            "1" => "approved",
            "0" => "unapproved",
            other => other,
        };
        if status == old {
            return Ok(false);
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(_) => return Ok(false),
        };
        let id = comment.comment_id;
        Ok(match status {
            "approved" | "approve" | "1" => {
                let changed = comment.comment_approved != "1" && content.set_comment_status(id, "approve").await?;
                if changed {
                    self.comment_event(super::CommentEvent::Approved { comment: id, by: self.user().map_or(0, |u| u.id) }).await;
                }
                changed
            }
            "hold" | "0" => comment.comment_approved != "0" && content.set_comment_status(id, "hold").await?,
            "spam" => content.spam_comment(id).await?,
            "unspam" => content.unspam_comment(id).await?,
            "trash" => content.trash_comment(id).await?,
            "untrash" => content.untrash_comment(id).await?,
            _ => false,
        })
    }

    /// `create_item_permissions_check()` and `create_item()`.
    async fn create_comment(&self, args: &Args, params: &Params) -> Result<Outcome> {
        let kind = args.str("type").map_or_else(|| "comment".to_owned(), |t| sanitize_key(&t));
        let is_note = kind == "note";
        let Some(user) = self.user().filter(|u| u.id != 0) else {
            // `rest_allow_anonymous_comments` is off.
            bail!(RestError::new("rest_comment_login_required", "Sorry, you must be logged in to comment.", 401));
        };
        let moderator = user.can("moderate_comments");
        let post_id = args.u64("post").unwrap_or(0);
        if args.has("author") && args.u64("author") != Some(user.id) && !moderator {
            bail!(self.forbidden("rest_comment_invalid_author", "Sorry, you are not allowed to edit 'author' for comments."));
        }
        if let Some(ip) = args.str("author_ip")
            && !moderator
        {
            let remote = self.request.as_ref().and_then(|r| r.remote_addr.clone()).unwrap_or_default();
            if remote.is_empty() || ip != remote {
                bail!(self.forbidden("rest_comment_invalid_author_ip", "Sorry, you are not allowed to edit 'author_ip' for comments."));
            }
        }
        if is_note
            && post_id != 0
            && let Some(error) = self.check_target_post(post_id, params, true).await?
        {
            bail!(error);
        }
        if args.has("status") {
            let allowed = if is_note {
                match self.site.db.post(post_id).await? {
                    Some(post) => self.can_edit_post(&post),
                    None => false,
                }
            } else {
                moderator
            };
            if !allowed {
                bail!(self.forbidden("rest_comment_invalid_status", "Sorry, you are not allowed to edit 'status' for comments."));
            }
        }
        if post_id == 0 {
            bail!(RestError::new("rest_comment_invalid_post_id", "Sorry, you are not allowed to create this comment without a post.", 403));
        }
        if !is_note {
            let Some(post) = self.site.db.post(post_id).await? else {
                bail!(RestError::new("rest_comment_invalid_post_id", "Sorry, you are not allowed to create this comment without a post.", 403));
            };
            if post.post_status == "draft" {
                bail!(RestError::new("rest_comment_draft_post", "Sorry, you are not allowed to create a comment on this post.", 403));
            }
            if post.post_status == "trash" {
                bail!(RestError::new("rest_comment_trash_post", "Sorry, you are not allowed to create a comment on this post.", 403));
            }
            if !self.check_read_post_permission(&post, params, true, false).await {
                bail!(self.forbidden("rest_cannot_read_post", "Sorry, you are not allowed to read the post for this comment."));
            }
            if !rustpress_content::comment_rules::comments_open(self.site, &post, self.now()) {
                bail!(RestError::new("rest_comment_closed", "Sorry, comments are closed for this item.", 403));
            }
        }

        // `create_item()`
        if args.u64("id").is_some_and(|id| id != 0) {
            bail!(RestError::new("rest_comment_exists", "Cannot create existing comment.", 400));
        }
        if !matches!(kind.as_str(), "comment" | "note") {
            bail!(RestError::new("rest_invalid_comment_type", "Cannot create a comment with that type.", 400));
        }
        let prepared = match self.prepare_comment_for_database(args).await? {
            Ok(p) => p,
            Err(e) => bail!(e),
        };
        if !Self::content_allowed(Some(prepared.content.as_deref().unwrap_or_default()), is_note, args) {
            bail!(RestError::new("rest_comment_content_invalid", "Invalid comment content.", 400));
        }
        let now = self.now();
        let (date, date_gmt) = prepared.date.clone().unwrap_or_else(|| (self.site.date("Y-m-d H:i:s", now), rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", now)));
        // The signed-in user is the author unless the request names one.
        let missing_author = prepared.user_id.is_none() && prepared.author.as_deref().is_none_or(str::is_empty) && prepared.email.as_deref().is_none_or(str::is_empty) && prepared.url.as_deref().is_none_or(str::is_empty);
        let mut c = NewComment {
            post_id,
            author: prepared.author.clone().unwrap_or_default(),
            email: prepared.email.clone().unwrap_or_default(),
            url: prepared.url.clone().unwrap_or_default(),
            content: prepared.content.clone().unwrap_or_default(),
            comment_type: kind.clone(),
            parent: prepared.parent.unwrap_or(0),
            user_id: prepared.user_id,
            ip: prepared.ip.clone(),
            agent: prepared.agent.clone().unwrap_or_default(),
        };
        if missing_author {
            let row = self.site.db.user(user.id).await?;
            c.user_id = Some(user.id);
            c.author = user.display_name.clone();
            c.email = user.email.clone();
            c.url = row.map(|u| u.user_url).unwrap_or_default();
        }
        if self.site.options.string("require_name_email", "1") == "1" && (c.author.is_empty() || c.email.is_empty()) {
            bail!(RestError::new("rest_comment_author_data_required", "Creating a comment requires valid author name and email values.", 400));
        }
        if let Some(error) = Self::check_max_lengths(&c.author, &c.email, &c.url, &c.content) {
            bail!(error);
        }
        // `wp_allow_comment()`; notes are never duplicates or floods.
        let mut approved = "1".to_owned();
        if !is_note {
            if comment_rules::is_duplicate(&self.site.db, &c).await {
                bail!(RestError::new("comment_duplicate", "Duplicate comment detected; it looks as though you&#8217;ve already said that!", 409));
            }
            if comment_rules::is_flood(self.site, &self.site.db, Some(user), &c.ip, &c.email, now).await {
                bail!(RestError::new("comment_flood", "You are posting comments too quickly. Slow down.", 400));
            }
            approved = match self.access.as_ref().and_then(|a| a.roles) {
                Some(roles) => comment_rules::check_comment_data(self.site, &self.site.db, roles, &c).await,
                None => "0".into(),
            };
        }
        // `wp_filter_comment()` and `wp_insert_comment()`.
        let content = self.filter_comment_content(&c.content, c.user_id.unwrap_or(0)).await?;
        let writer = match self.writer() {
            Ok(w) => w,
            Err(e) => bail!(e),
        };
        let ip: String = c.ip.chars().filter(|ch| ch.is_ascii_hexdigit() || matches!(ch, ':' | '.' | ',' | ' ')).collect();
        let fields: Vec<(&str, Val)> = vec![
            ("comment_post_ID", Val::Int(post_id as i64)),
            ("comment_author", Val::Str(comment_rules::filter_text(&c.author))),
            ("comment_author_email", Val::Str(comment_rules::filter_email(&c.email))),
            ("comment_author_url", Val::Str(comment_rules::filter_url(&c.url))),
            ("comment_author_IP", Val::Str(ip)),
            ("comment_date", Val::Str(date)),
            ("comment_date_gmt", Val::Str(date_gmt)),
            ("comment_content", Val::Str(content)),
            ("comment_karma", Val::Int(0)),
            ("comment_approved", Val::Str(approved.clone())),
            ("comment_agent", Val::Str(c.agent.chars().take(254).collect())),
            ("comment_type", Val::Str(kind.clone())),
            ("comment_parent", Val::Int(c.parent as i64)),
            ("user_id", Val::Int(c.user_id.unwrap_or(0) as i64)),
        ];
        let id = writer.insert_row("comments", &fields).await?;
        if id == 0 {
            bail!(RestError::new("rest_comment_failed_create", "Creating comment failed.", 500));
        }
        if approved == "1"
            && let Ok(content) = self.content()
        {
            content.update_comment_count(post_id).await?;
        }
        let Some(mut comment) = self.site.db.comment_any(id).await? else { bail!(RestError::new("rest_comment_failed_create", "Creating comment failed.", 500)) };
        if let Some(status) = args.str("status") {
            self.handle_comment_status(&sanitize_key(&status), &comment).await?;
        }
        self.comment_event(super::CommentEvent::Inserted { comment: id, by: user.id, creating: true }).await;
        if let Some(error) = self.update_comment_meta(&comment, args).await? {
            bail!(error);
        }
        comment = self.site.db.comment_any(id).await?.unwrap_or(comment);
        let context = if moderator { "edit" } else { "view" };
        let resp = Resp::item(self.prepare_comment(&comment, context).await?);
        Ok(Ok(self.created(resp, self.rest_url(&format!("wp/v2/comments/{id}")))))
    }

    /// `update_item_permissions_check()` and `update_item()`.
    async fn update_comment(&self, id: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let comment = match self.rest_comment(id).await? {
            Ok(c) => c,
            Err(e) => bail!(e),
        };
        if !self.check_edit_comment(&comment).await? {
            bail!(self.forbidden("rest_cannot_edit", "Sorry, you are not allowed to edit this comment."));
        }
        if let Some(post) = args.u64("post")
            && post != comment.comment_post_id
            && let Some(error) = self.check_target_post(post, params, comment.comment_type == "note").await?
        {
            bail!(error);
        }
        let current_type = if comment.comment_type.is_empty() { "comment" } else { comment.comment_type.as_str() };
        if args.str("type").is_some_and(|t| t != current_type) {
            bail!(RestError::new("rest_comment_invalid_type", "Sorry, you are not allowed to change the comment type.", 404));
        }
        let prepared = match self.prepare_comment_for_database(args).await? {
            Ok(p) => p,
            Err(e) => bail!(e),
        };
        if let Some(post) = prepared.post_id.filter(|p| *p != 0)
            && self.site.db.post(post).await?.is_none()
        {
            bail!(RestError::new("rest_comment_invalid_post_id", "Invalid post ID.", 403));
        }
        // The prepared values always hold the address, so the comment is always rewritten (`wp_update_comment()`).
        // Unlike when creating, the note status of the request does not allow empty content here.
        if !Self::content_allowed(prepared.content.as_deref(), false, args) {
            bail!(RestError::new("rest_comment_content_invalid", "Invalid comment content.", 400));
        }
        let author = prepared.author.clone().unwrap_or_else(|| comment.comment_author.clone());
        let email = prepared.email.clone().unwrap_or_else(|| comment.comment_author_email.clone());
        let url = prepared.url.clone().unwrap_or_else(|| comment.comment_author_url.clone());
        let content = prepared.content.clone().unwrap_or_else(|| comment.comment_content.clone());
        if let Some(error) = Self::check_max_lengths(&author, &email, &url, &content) {
            bail!(error);
        }
        let user_id = prepared.user_id.unwrap_or(comment.user_id);
        // `wp_filter_comment()` runs over the merged values; the GMT date follows the local one.
        let content = self.filter_comment_content(&content, comment.user_id).await?;
        let date = prepared.date.as_ref().map_or_else(|| comment.comment_date.clone(), |(local, _)| local.clone());
        let date_gmt = self.gmt_from_local(&date);
        let agent = prepared.agent.clone().unwrap_or_else(|| comment.comment_agent.clone());
        let ip: String = prepared.ip.chars().filter(|ch| ch.is_ascii_hexdigit() || matches!(ch, ':' | '.' | ',' | ' ')).collect();
        let fields: Vec<(&str, Val)> = vec![
            ("comment_post_ID", Val::Int(prepared.post_id.filter(|p| *p != 0).unwrap_or(comment.comment_post_id) as i64)),
            ("comment_author", Val::Str(comment_rules::filter_text(&author))),
            ("comment_author_email", Val::Str(comment_rules::filter_email(&email))),
            ("comment_author_url", Val::Str(comment_rules::filter_url(&url))),
            ("comment_author_IP", Val::Str(ip)),
            ("comment_date", Val::Str(date)),
            ("comment_date_gmt", Val::Str(date_gmt)),
            ("comment_content", Val::Str(content)),
            ("comment_agent", Val::Str(agent.chars().take(254).collect())),
            ("comment_parent", Val::Int(prepared.parent.unwrap_or(comment.comment_parent) as i64)),
            ("user_id", Val::Int(user_id as i64)),
        ];
        let writer = match self.writer() {
            Ok(w) => w,
            Err(e) => bail!(e),
        };
        writer.update_rows("comments", "comment_ID", comment.comment_id, &fields).await?;
        if let Ok(content) = self.content() {
            content.update_comment_count(comment.comment_post_id).await?;
        }
        let mut updated = self.site.db.comment_any(comment.comment_id).await?.unwrap_or(comment);
        if let Some(status) = args.str("status") {
            self.handle_comment_status(&sanitize_key(&status), &updated).await?;
        }
        if let Some(error) = self.update_comment_meta(&updated, args).await? {
            bail!(error);
        }
        self.comment_event(super::CommentEvent::Inserted { comment: updated.comment_id, by: self.user().map_or(0, |u| u.id), creating: false }).await;
        updated = self.site.db.comment_any(updated.comment_id).await?.unwrap_or(updated);
        Ok(Ok(Resp::item(self.prepare_comment(&updated, "edit").await?)))
    }

    /// `delete_item_permissions_check()` and `delete_item()`: to the trash, or for good with `force`.
    async fn delete_comment(&self, id: &str, args: &Args) -> Result<Outcome> {
        let comment = match self.rest_comment(id).await? {
            Ok(c) => c,
            Err(e) => bail!(e),
        };
        if !self.check_edit_comment(&comment).await? {
            bail!(self.forbidden("rest_cannot_delete", "Sorry, you are not allowed to delete this comment."));
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => bail!(e),
        };
        if args.bool("force").unwrap_or(false) {
            // `$previous->get_data()`: the fields without the links.
            let mut previous = self.prepare_comment(&comment, "edit").await?;
            if let Some(object) = previous.as_object_mut() {
                object.remove("_links");
            }
            if !content.delete_comment(comment.comment_id).await? {
                bail!(RestError::new("rest_cannot_delete", "The comment cannot be deleted.", 500));
            }
            return Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))));
        }
        if comment.comment_approved == "trash" {
            bail!(RestError::new("rest_already_trashed", "The comment has already been trashed.", 410));
        }
        if !content.trash_comment(comment.comment_id).await? {
            bail!(RestError::new("rest_cannot_delete", "The comment cannot be deleted.", 500));
        }
        let trashed = self.site.db.comment_any(comment.comment_id).await?.unwrap_or(comment);
        Ok(Ok(Resp::item(self.prepare_comment(&trashed, "edit").await?)))
    }
}
