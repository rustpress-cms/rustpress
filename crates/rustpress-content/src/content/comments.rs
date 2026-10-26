//! Comments: status changes, trash, delete, update and inserting a reply.

use super::{Content, ContentError, Result};
use crate::sanitize;
use rustpress_db::Val;

/// The values of `wp_update_comment()`; `None` keeps the stored value.
#[derive(Debug, Clone, Default)]
pub struct CommentUpdate {
    pub author: Option<String>,
    pub author_email: Option<String>,
    pub author_url: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
    /// The date in the site's time zone (`Y-m-d H:i:s`).
    pub date: Option<String>,
}

impl Content<'_> {
    /// `wp_set_comment_status()`: `approve`, `hold`, `spam`, `trash`.
    pub async fn set_comment_status(&self, id: u64, status: &str) -> Result<bool> {
        let Some(comment) = self.db.comment_any(id).await? else { return Ok(false) };
        let value = match status {
            "approve" | "1" => "1",
            "hold" | "0" => "0",
            "spam" => "spam",
            "trash" => "trash",
            _ => return Ok(false),
        };
        if comment.comment_approved == value {
            return Ok(true);
        }
        self.writer.update_rows("comments", "comment_ID", id, &[("comment_approved", Val::Str(value.to_owned()))]).await?;
        self.update_comment_count(comment.comment_post_id).await?;
        Ok(true)
    }

    /// `wp_trash_comment()`.
    pub async fn trash_comment(&self, id: u64) -> Result<bool> {
        let Some(comment) = self.db.comment_any(id).await? else { return Ok(false) };
        if comment.comment_approved == "trash" {
            return Ok(false);
        }
        self.writer.delete_rows_where("commentmeta", &format!("comment_id = {id} AND meta_key IN ('_wp_trash_meta_status', '_wp_trash_meta_time')")).await?;
        self.writer.insert_row("commentmeta", &[("comment_id", Val::Int(id as i64)), ("meta_key", Val::Str("_wp_trash_meta_status".into())), ("meta_value", Val::Str(comment.comment_approved.clone()))]).await?;
        self.writer.insert_row("commentmeta", &[("comment_id", Val::Int(id as i64)), ("meta_key", Val::Str("_wp_trash_meta_time".into())), ("meta_value", Val::Str(self.now.to_string()))]).await?;
        self.set_comment_status(id, "trash").await
    }

    /// `wp_untrash_comment()`.
    pub async fn untrash_comment(&self, id: u64) -> Result<bool> {
        let Some(comment) = self.db.comment_any(id).await? else { return Ok(false) };
        if comment.comment_approved != "trash" {
            return Ok(false);
        }
        let previous = self.db.comment_meta_value(id, "_wp_trash_meta_status").await?.filter(|s| !s.is_empty()).unwrap_or_else(|| "0".to_owned());
        self.writer.delete_rows_where("commentmeta", &format!("comment_id = {id} AND meta_key IN ('_wp_trash_meta_status', '_wp_trash_meta_time')")).await?;
        self.writer.update_rows("comments", "comment_ID", id, &[("comment_approved", Val::Str(previous))]).await?;
        self.update_comment_count(comment.comment_post_id).await?;
        Ok(true)
    }

    /// `wp_spam_comment()`.
    pub async fn spam_comment(&self, id: u64) -> Result<bool> {
        let Some(comment) = self.db.comment_any(id).await? else { return Ok(false) };
        if comment.comment_approved == "spam" {
            return Ok(false);
        }
        self.writer.delete_rows_where("commentmeta", &format!("comment_id = {id} AND meta_key = '_wp_trash_meta_status'")).await?;
        self.writer.insert_row("commentmeta", &[("comment_id", Val::Int(id as i64)), ("meta_key", Val::Str("_wp_trash_meta_status".into())), ("meta_value", Val::Str(comment.comment_approved.clone()))]).await?;
        self.set_comment_status(id, "spam").await
    }

    /// `wp_unspam_comment()`.
    pub async fn unspam_comment(&self, id: u64) -> Result<bool> {
        let Some(comment) = self.db.comment_any(id).await? else { return Ok(false) };
        if comment.comment_approved != "spam" {
            return Ok(false);
        }
        let previous = self.db.comment_meta_value(id, "_wp_trash_meta_status").await?.filter(|s| !s.is_empty()).unwrap_or_else(|| "0".to_owned());
        self.writer.delete_rows_where("commentmeta", &format!("comment_id = {id} AND meta_key = '_wp_trash_meta_status'")).await?;
        self.writer.update_rows("comments", "comment_ID", id, &[("comment_approved", Val::Str(previous))]).await?;
        self.update_comment_count(comment.comment_post_id).await?;
        Ok(true)
    }

    /// `wp_delete_comment( $id, true )`: replies move up to the parent.
    pub async fn delete_comment(&self, id: u64) -> Result<bool> {
        let Some(comment) = self.db.comment_any(id).await? else { return Ok(false) };
        for child in self.db.comment_children(id).await? {
            self.writer.update_rows("comments", "comment_ID", child, &[("comment_parent", Val::Int(comment.comment_parent as i64))]).await?;
        }
        self.writer.delete_rows("commentmeta", "comment_id", id).await?;
        self.writer.delete_rows("comments", "comment_ID", id).await?;
        self.update_comment_count(comment.comment_post_id).await?;
        Ok(true)
    }

    /// `wp_update_comment()`.
    pub async fn update_comment(&self, id: u64, update: &CommentUpdate) -> Result<()> {
        let Some(comment) = self.db.comment_any(id).await? else { return Err(ContentError::Message(self.site.tr("Invalid comment ID."))) };
        let mut fields: Vec<(&str, Val)> = Vec::new();
        if let Some(author) = &update.author {
            fields.push(("comment_author", Val::Str(sanitize::text_field(author))));
        }
        if let Some(email) = &update.author_email {
            fields.push(("comment_author_email", Val::Str(crate::sanitize::sanitize_email(email))));
        }
        if let Some(url) = &update.author_url {
            fields.push(("comment_author_url", Val::Str(if url.trim().is_empty() { String::new() } else { rustpress_core::formatting::esc_url_raw(url.trim()) })));
        }
        // The caller has run the `pre_comment_content` filters.
        if let Some(content) = &update.content {
            fields.push(("comment_content", Val::Str(content.clone())));
        }
        if let Some(date) = &update.date {
            fields.push(("comment_date_gmt", Val::Str(self.gmt_from_local(date))));
            fields.push(("comment_date", Val::Str(date.clone())));
        }
        self.writer.update_rows("comments", "comment_ID", id, &fields).await?;
        if let Some(status) = &update.status {
            self.set_comment_status(id, status).await?;
        }
        self.update_comment_count(comment.comment_post_id).await?;
        Ok(())
    }

    /// `wp_new_comment()` for a reply written by a signed-in user in the administration area. `content` has been
    /// through the `pre_comment_content` filters; `ip` and `agent` are those of the request.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_reply(&self, post_id: u64, parent: u64, content: &str, comment_type: &str, approved: &str, ip: &str, agent: &str) -> Result<u64> {
        let content = content.to_owned();
        let user_row = self.db.user(self.user.id).await?;
        let fields: Vec<(&str, Val)> = vec![
            ("comment_post_ID", Val::Int(post_id as i64)),
            ("comment_author", Val::Str(self.user.display_name.clone())),
            ("comment_author_email", Val::Str(self.user.email.clone())),
            ("comment_author_url", Val::Str(user_row.map(|u| u.user_url).unwrap_or_default())),
            ("comment_author_IP", Val::Str(ip.chars().filter(|c| c.is_ascii_hexdigit() || matches!(c, ':' | '.' | ',' | ' ')).collect())),
            ("comment_date", Val::Str(self.local_now())),
            ("comment_date_gmt", Val::Str(self.gmt_now())),
            ("comment_content", Val::Str(content)),
            ("comment_karma", Val::Int(0)),
            ("comment_approved", Val::Str(approved.to_owned())),
            ("comment_agent", Val::Str(agent.chars().take(254).collect())),
            ("comment_type", Val::Str(comment_type.to_owned())),
            ("comment_parent", Val::Int(parent as i64)),
            ("user_id", Val::Int(self.user.id as i64)),
        ];
        let id = self.writer.insert_row("comments", &fields).await?;
        self.update_comment_count(post_id).await?;
        Ok(id)
    }
}
