//! The e-mail about comments: the author of a post hears about a new comment (`wp_notify_postauthor()`), the
//! moderators about a comment that waits for approval (`wp_notify_moderator()`).

use crate::mail::{MailCtx, specialchars_decode};

/// The facts about the author of a post the comment e-mail depends on.
pub struct PostAuthor<'a> {
    pub user: &'a rustpress_db::User,
    /// Whether they may edit the comments of the post (`user_can( $author, 'edit_comment' )`).
    pub can_edit_comments: bool,
    /// Whether they may read the post (`user_can( $author, 'read_post' )`).
    pub can_read: bool,
}

impl MailCtx<'_> {
    fn admin_url(&self, path: &str) -> String {
        self.site.site_url(&format!("wp-admin/{path}"))
    }

    /// The host of the home address without `www.`: the sender `wordpress@host`.
    fn wp_email(&self) -> String {
        let host = self.site.settings.home.split_once("://").map_or("localhost", |(_, rest)| rest.split(['/', ':']).next().unwrap_or("localhost"));
        format!("wordpress@{}", host.trim_start_matches("www."))
    }

    /// `wp_notify_postauthor()` for a comment: the author of the post hears about it. `current_user` is who is
    /// signed in (0 for a visitor).
    pub async fn notify_postauthor(&self, comment_id: u64, post_author: Option<&PostAuthor<'_>>, current_user: u64) -> bool {
        let Some(comment) = self.db.comment_any(comment_id).await.ok().flatten() else { return false };
        let Some(post) = self.db.post(comment.comment_post_id).await.ok().flatten() else { return false };
        let Some(author) = post_author else { return false };
        let email = author.user.user_email.clone();
        if email.is_empty() || comment.user_id == post.post_author || current_user == post.post_author || !author.can_read {
            return false;
        }
        let (ip, _agent) = self.db.comment_extra(comment_id).await.ok().flatten().unwrap_or_default();
        let blogname = self.sitename();
        let content = specialchars_decode(&comment.comment_content);
        let wp_email = self.wp_email();
        let mut headers = if comment.comment_author.is_empty() { format!("From: \"{blogname}\" <{wp_email}>\n") } else { format!("From: \"{}\" <{wp_email}>\n", comment.comment_author) };
        headers.push_str("Content-Type: text/plain; charset=\"UTF-8\"\n");
        if !comment.comment_author_email.is_empty() {
            headers.push_str(&if comment.comment_author.is_empty() { format!("Reply-To: {}\n", comment.comment_author_email) } else { format!("Reply-To: \"{0}\" <{0}>\n", comment.comment_author_email) });
        }
        let tr = |s: &str| self.tr(s);
        let sprintf = rustpress_core::i18n::sprintf;
        let title = &post.post_title;
        // No reverse lookup of the address: like `gethostbyaddr()` that finds no name, the address is the name.
        let domain = ip.clone();
        let (mut message, subject) = match comment.comment_type.as_str() {
            "trackback" | "pingback" => {
                let (kind, all) = if comment.comment_type == "trackback" {
                    (tr("New trackback on your post \"%s\""), tr("You can see all trackbacks on this post here:"))
                } else {
                    (tr("New pingback on your post \"%s\""), tr("You can see all pingbacks on this post here:"))
                };
                let mut m = format!("{}\r\n", sprintf(&kind, &[title]));
                m.push_str(&format!("{}\r\n", sprintf(&tr("Website: %1$s (IP address: %2$s, %3$s)"), &[&comment.comment_author, &ip, &domain])));
                m.push_str(&format!("{}\r\n", sprintf(&tr("URL: %s"), &[&comment.comment_author_url])));
                m.push_str(&format!("{}\r\n\r\n", sprintf(&tr("Comment: %s"), &[&format!("\r\n{content}")])));
                m.push_str(&format!("{all}\r\n"));
                let subject = if comment.comment_type == "trackback" { sprintf(&tr("[%1$s] Trackback: \"%2$s\""), &[&blogname, title]) } else { sprintf(&tr("[%1$s] Pingback: \"%2$s\""), &[&blogname, title]) };
                (m, subject)
            }
            _ => {
                let mut m = format!("{}\r\n", sprintf(&tr("New comment on your post \"%s\""), &[title]));
                m.push_str(&format!("{}\r\n", sprintf(&tr("Author: %1$s (IP address: %2$s, %3$s)"), &[&comment.comment_author, &ip, &domain])));
                m.push_str(&format!("{}\r\n", sprintf(&tr("Email: %s"), &[&comment.comment_author_email])));
                m.push_str(&format!("{}\r\n", sprintf(&tr("URL: %s"), &[&comment.comment_author_url])));
                if comment.comment_parent != 0 && author.can_edit_comments {
                    m.push_str(&format!("{}\r\n", sprintf(&tr("In reply to: %s"), &[&self.admin_url(&format!("comment.php?action=editcomment&c={}#wpbody-content", comment.comment_parent))])));
                }
                m.push_str(&format!("{}\r\n\r\n", sprintf(&tr("Comment: %s"), &[&format!("\r\n{content}")])));
                m.push_str(&format!("{}\r\n", tr("You can see all comments on this post here:")));
                (m, sprintf(&tr("[%1$s] Comment: \"%2$s\""), &[&blogname, title]))
            }
        };
        let permalink = self.site.permalink(&post).await.unwrap_or_default();
        message.push_str(&format!("{permalink}#comments\r\n\r\n"));
        let link = crate::screens::comments::comment_link_for(self.site, self.db, &comment).await;
        message.push_str(&format!("{}\r\n", sprintf(&tr("Permalink: %s"), &[&link])));
        if author.can_edit_comments {
            message.push_str(&format!("{}\r\n", sprintf(&tr("Trash it: %s"), &[&self.admin_url(&format!("comment.php?action=trash&c={comment_id}#wpbody-content"))])));
            message.push_str(&format!("{}\r\n", sprintf(&tr("Spam it: %s"), &[&self.admin_url(&format!("comment.php?action=spam&c={comment_id}#wpbody-content"))])));
        }
        self.send_with_headers(&email, &subject, &message, &headers).await
    }

    /// `wp_notify_moderator()`: the administrator (and the author of the post) hear about a comment that waits.
    pub async fn notify_moderator(&self, comment_id: u64, post_author: Option<&PostAuthor<'_>>) -> bool {
        let notify = self.site.options.string("moderation_notify", "1");
        if notify.is_empty() || notify == "0" {
            return true;
        }
        let Some(comment) = self.db.comment_any(comment_id).await.ok().flatten() else { return true };
        let Some(post) = self.db.post(comment.comment_post_id).await.ok().flatten() else { return true };
        let admin_email = self.site.options.string("admin_email", "");
        let mut emails = vec![admin_email.clone()];
        if let Some(author) = post_author
            && author.can_edit_comments
            && !author.user.user_email.is_empty()
            && !author.user.user_email.eq_ignore_ascii_case(&admin_email)
        {
            emails.push(author.user.user_email.clone());
        }
        let (ip, _agent) = self.db.comment_extra(comment_id).await.ok().flatten().unwrap_or_default();
        let waiting = self.db.count_comments_with_status("0").await.unwrap_or(0);
        let blogname = self.sitename();
        let content = specialchars_decode(&comment.comment_content);
        let tr = |s: &str| self.tr(s);
        let sprintf = rustpress_core::i18n::sprintf;
        let title = &post.post_title;
        let permalink = self.site.permalink(&post).await.unwrap_or_default();
        let domain = ip.clone();
        let mut message = match comment.comment_type.as_str() {
            "trackback" | "pingback" => {
                let (kind, excerpt) = if comment.comment_type == "trackback" {
                    (tr("A new trackback on the post \"%s\" is waiting for your approval"), tr("Trackback excerpt: %s"))
                } else {
                    (tr("A new pingback on the post \"%s\" is waiting for your approval"), tr("Pingback excerpt: %s"))
                };
                let mut m = format!("{}\r\n{permalink}\r\n\r\n", sprintf(&kind, &[title]));
                m.push_str(&format!("{}\r\n", sprintf(&tr("Website: %1$s (IP address: %2$s, %3$s)"), &[&comment.comment_author, &ip, &domain])));
                m.push_str(&format!("{}\r\n", sprintf(&tr("URL: %s"), &[&comment.comment_author_url])));
                m.push_str(&format!("{}\r\n\r\n", sprintf(&excerpt, &[&format!("\r\n{content}")])));
                m
            }
            _ => {
                let mut m = format!("{}\r\n{permalink}\r\n\r\n", sprintf(&tr("A new comment on the post \"%s\" is waiting for your approval"), &[title]));
                m.push_str(&format!("{}\r\n", sprintf(&tr("Author: %1$s (IP address: %2$s, %3$s)"), &[&comment.comment_author, &ip, &domain])));
                m.push_str(&format!("{}\r\n", sprintf(&tr("Email: %s"), &[&comment.comment_author_email])));
                m.push_str(&format!("{}\r\n", sprintf(&tr("URL: %s"), &[&comment.comment_author_url])));
                if comment.comment_parent != 0 {
                    m.push_str(&format!("{}\r\n", sprintf(&tr("In reply to: %s"), &[&self.admin_url(&format!("comment.php?action=editcomment&c={}#wpbody-content", comment.comment_parent))])));
                }
                m.push_str(&format!("{}\r\n\r\n", sprintf(&tr("Comment: %s"), &[&format!("\r\n{content}")])));
                m
            }
        };
        message.push_str(&format!("{}\r\n", sprintf(&tr("Approve it: %s"), &[&self.admin_url(&format!("comment.php?action=approve&c={comment_id}#wpbody-content"))])));
        message.push_str(&format!("{}\r\n", sprintf(&tr("Trash it: %s"), &[&self.admin_url(&format!("comment.php?action=trash&c={comment_id}#wpbody-content"))])));
        message.push_str(&format!("{}\r\n", sprintf(&tr("Spam it: %s"), &[&self.admin_url(&format!("comment.php?action=spam&c={comment_id}#wpbody-content"))])));
        message.push_str(&format!(
            "{}\r\n",
            sprintf(&self.site.trn("Currently %s comment is waiting for approval. Please visit the moderation panel:", "Currently %s comments are waiting for approval. Please visit the moderation panel:", waiting as i64), &[&self.site.number_i18n(waiting)])
        ));
        message.push_str(&format!("{}\r\n", self.admin_url("edit-comments.php?comment_status=moderated#wpbody-content")));
        let subject = sprintf(&tr("[%1$s] Please moderate: \"%2$s\""), &[&blogname, title]);
        for email in emails {
            self.send(&email, &subject, &message).await;
        }
        true
    }
}
