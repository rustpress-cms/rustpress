//! `map_meta_cap()` for the signed-in user of a request: the rules live in `rustpress_auth::meta_caps`.

use rustpress_db::Post;

pub(crate) use rustpress_auth::TypeCaps;

use crate::Rest;

impl Rest<'_> {
    pub(crate) fn can_create(&self, post_type: &str) -> bool {
        self.user().is_some_and(|u| u.can_create_type(post_type))
    }

    /// `current_user_can( 'edit_post', $post )`.
    pub(crate) fn can_edit_post(&self, post: &Post) -> bool {
        self.user().is_some_and(|u| u.can_edit_post(post))
    }

    /// `current_user_can( 'delete_post', $post )`.
    pub(crate) fn can_delete_post(&self, post: &Post) -> bool {
        self.user().is_some_and(|u| u.can_delete_post(post))
    }

    /// `current_user_can( 'read_post', $post )`: published posts for everybody.
    pub(crate) fn can_read_post(&self, post: &Post) -> bool {
        match self.user() {
            Some(user) => user.can_read_post(post),
            None => post.post_status == "publish" || (post.post_type == "attachment" && post.post_status == "inherit"),
        }
    }

    /// `get_post_status()`: an attachment that inherits has the status of its parent (`publish` without one).
    pub(crate) async fn post_status(&self, post: &Post) -> String {
        if post.post_type != "attachment" || post.post_status != "inherit" {
            return post.post_status.clone();
        }
        if post.post_parent == 0 || post.post_parent == post.id {
            return "publish".to_owned();
        }
        match self.site.db.post(post.post_parent).await.ok().flatten() {
            None => "publish".to_owned(),
            Some(parent) if parent.post_status == "trash" => {
                let meta = self.site.db.post_meta(parent.id).await.unwrap_or_default();
                meta.iter().find(|m| m.meta_key.as_deref() == Some("_wp_trash_meta_status")).and_then(|m| m.meta_value.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "publish".to_owned())
            }
            Some(parent) => parent.post_status,
        }
    }

    /// `check_read_permission()` of the posts controller: published, readable, or inheriting from a readable parent.
    pub(crate) async fn check_read_permission(&self, post: &Post) -> bool {
        let mut current = post.clone();
        for _ in 0..32 {
            if current.post_status == "publish" {
                return true;
            }
            let mut effective = current.clone();
            effective.post_status = self.post_status(&current).await;
            if self.can_read_post(&effective) {
                return true;
            }
            if current.post_status != "inherit" {
                return false;
            }
            if current.post_parent == 0 {
                return true;
            }
            match self.site.db.post(current.post_parent).await.ok().flatten() {
                Some(parent) => current = parent,
                None => return true,
            }
        }
        false
    }
}
