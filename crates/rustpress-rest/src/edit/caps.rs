//! `map_meta_cap()` for posts and the capabilities of a post type.

use rustpress_db::Post;

use crate::Rest;

/// The capability names of a post type (`get_post_type_capabilities()`).
pub(crate) struct TypeCaps {
    pub edit_posts: String,
    pub edit_others: String,
    pub publish: String,
    pub read_private: String,
    pub delete_posts: String,
    pub delete_others: String,
    pub delete_published: String,
    pub delete_private: String,
    pub edit_private: String,
    pub edit_published: String,
}

impl TypeCaps {
    pub fn of(post_type: &str) -> Self {
        let plural = if post_type == "page" { "pages" } else { "posts" };
        let name = |prefix: &str| format!("{prefix}_{plural}");
        Self {
            edit_posts: name("edit"),
            edit_others: format!("edit_others_{plural}"),
            publish: format!("publish_{plural}"),
            read_private: format!("read_private_{plural}"),
            delete_posts: name("delete"),
            delete_others: format!("delete_others_{plural}"),
            delete_published: format!("delete_published_{plural}"),
            delete_private: format!("delete_private_{plural}"),
            edit_private: format!("edit_private_{plural}"),
            edit_published: format!("edit_published_{plural}"),
        }
    }
}

impl Rest<'_> {
    pub(crate) fn can_create(&self, post_type: &str) -> bool {
        if post_type == "attachment" {
            return self.can("upload_files");
        }
        self.can(&TypeCaps::of(post_type).edit_posts)
    }

    /// `current_user_can( 'edit_post', $post )`.
    pub(crate) fn can_edit_post(&self, post: &Post) -> bool {
        let Some(user) = self.user() else { return false };
        let caps = TypeCaps::of(&post.post_type);
        let status = if post.post_status == "trash" { "draft" } else { post.post_status.as_str() };
        let own = post.post_author != 0 && post.post_author == user.id;
        if post.post_type == "attachment" && !own && !user.can("edit_others_posts") {
            return false;
        }
        let needed: Vec<&str> = if own {
            let mut list = vec![caps.edit_posts.as_str()];
            match status {
                "publish" | "future" => list.push(caps.edit_published.as_str()),
                "private" => list.push(caps.edit_private.as_str()),
                _ => {}
            }
            list
        } else {
            let mut list = vec![caps.edit_others.as_str()];
            match status {
                "publish" | "future" => list.push(caps.edit_published.as_str()),
                "private" => list.push(caps.edit_private.as_str()),
                _ => {}
            }
            list
        };
        needed.iter().all(|c| user.can(c))
    }

    /// `current_user_can( 'delete_post', $post )`.
    pub(crate) fn can_delete_post(&self, post: &Post) -> bool {
        let Some(user) = self.user() else { return false };
        let caps = TypeCaps::of(&post.post_type);
        let own = post.post_author != 0 && post.post_author == user.id;
        let mut needed: Vec<&str> = vec![if own { caps.delete_posts.as_str() } else { caps.delete_others.as_str() }];
        match post.post_status.as_str() {
            "publish" | "future" => needed.push(caps.delete_published.as_str()),
            "private" => needed.push(caps.delete_private.as_str()),
            _ => {}
        }
        needed.iter().all(|c| user.can(c))
    }

    /// `current_user_can( 'read_post', $post )`.
    pub(crate) fn can_read_post(&self, post: &Post) -> bool {
        if post.post_status == "publish" || (post.post_type == "attachment" && post.post_status == "inherit") {
            return true;
        }
        let Some(user) = self.user() else { return false };
        match post.post_status.as_str() {
            "private" => (post.post_author == user.id && user.can("read")) || user.can(&TypeCaps::of(&post.post_type).read_private),
            _ => self.can_edit_post(post),
        }
    }

    pub(crate) fn can_publish(&self, post_type: &str) -> bool {
        self.can(&TypeCaps::of(post_type).publish)
    }
}
