//! Writes of `wp/v2/comments` and `wp/v2/users/<id>`: the checks WordPress makes before it writes (missing
//! parameters, the comment or user, the permission). Who passes them gets `rest_not_implemented` (501) until the
//! writes themselves are ported.

use super::Args;
use crate::error::RestError;
use crate::{Outcome, Rest, Result};

impl Rest<'_> {
    fn not_implemented(&self) -> Outcome {
        Err(RestError::new("rest_not_implemented", "This request is not supported by RustPress yet.", 501))
    }

    /// `POST /wp/v2/comments`, `POST|PUT|PATCH|DELETE /wp/v2/comments/<id>`.
    pub(crate) async fn comments_write(&self, id: Option<&str>, method: &str) -> Result<Outcome> {
        let Some(id) = id else {
            if method != "POST" {
                return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
            }
            // `create_item_permissions_check()`: anonymous comments are off (`rest_allow_anonymous_comments`).
            if self.user().is_none() {
                return Ok(Err(RestError::new("rest_comment_login_required", "Sorry, you must be logged in to comment.", 401)));
            }
            return Ok(self.not_implemented());
        };
        let comment = match id.parse::<u64>() {
            Ok(id) => self.site.db.comment_any(id).await?,
            Err(_) => None,
        };
        let Some(comment) = comment else {
            return Ok(Err(RestError::new("rest_comment_invalid_id", "Invalid comment ID.", 404)));
        };
        // `check_edit_permission()`
        let post = self.site.db.post(comment.comment_post_id).await?;
        let may_edit = self.user().is_some_and(|u| u.id != 0 && (u.can("moderate_comments") || u.can_edit_comment(post.as_ref())));
        if !may_edit {
            return Ok(Err(if method == "DELETE" {
                self.forbidden("rest_cannot_delete", "Sorry, you are not allowed to delete this comment.")
            } else {
                self.forbidden("rest_cannot_edit", "Sorry, you are not allowed to edit this comment.")
            }));
        }
        Ok(self.not_implemented())
    }

    /// `POST|PUT|PATCH|DELETE /wp/v2/users/<id>`.
    pub(crate) async fn users_write(&self, id: &str, method: &str, args: &Args) -> Result<Outcome> {
        if method == "DELETE" && args.get("reassign").is_none() {
            return Ok(Err(RestError::missing_params(&["reassign"])));
        }
        let user = match id.parse::<u64>() {
            Ok(id) => self.site.db.user(id).await?,
            Err(_) => None,
        };
        let Some(user) = user else {
            return Ok(Err(RestError::new("rest_user_invalid_id", "Invalid user ID.", 404)));
        };
        if method == "DELETE" {
            if !self.can("delete_users") {
                return Ok(Err(self.forbidden("rest_user_cannot_delete", "Sorry, you are not allowed to delete this user.")));
            }
            return Ok(self.not_implemented());
        }
        if args.list("roles").is_some_and(|r| !r.is_empty()) && !self.can("promote_users") {
            return Ok(Err(self.forbidden("rest_cannot_edit_roles", "Sorry, you are not allowed to edit roles of this user.")));
        }
        if !self.user().is_some_and(|u| u.can_edit_user(user.id)) {
            return Ok(Err(self.forbidden("rest_cannot_edit", "Sorry, you are not allowed to edit this user.")));
        }
        Ok(self.not_implemented())
    }
}
