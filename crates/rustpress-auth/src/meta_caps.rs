//! `map_meta_cap()`: the capabilities about one object (a post, a comment, a user) and what they need of the
//! primitive capabilities of the user's roles. Every screen and every REST route asks these instead of building the
//! rule again.

use rustpress_db::Post;

use crate::CurrentUser;

/// The capability names of a post type (`get_post_type_capabilities()`).
#[derive(Debug, Clone)]
pub struct TypeCaps {
    pub read: String,
    pub create_posts: String,
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
        // The types of the site editor and the font library: everything is `edit_theme_options`.
        if matches!(post_type, "wp_navigation" | "wp_template" | "wp_template_part" | "wp_global_styles" | "wp_font_family" | "wp_font_face") {
            let cap = || "edit_theme_options".to_owned();
            let read = if post_type == "wp_global_styles" { "edit_posts".to_owned() } else { cap() };
            return Self {
                read,
                create_posts: cap(),
                edit_posts: cap(),
                edit_others: cap(),
                publish: cap(),
                read_private: cap(),
                delete_posts: cap(),
                delete_others: cap(),
                delete_published: cap(),
                delete_private: cap(),
                edit_private: cap(),
                edit_published: cap(),
            };
        }
        let plural = if post_type == "page" { "pages" } else { "posts" };
        let create_posts = match post_type {
            "attachment" => "upload_files".to_owned(),
            "wp_block" => "publish_posts".to_owned(),
            _ => format!("edit_{plural}"),
        };
        Self {
            read: if post_type == "wp_block" { "edit_posts".to_owned() } else { "read".to_owned() },
            create_posts,
            edit_posts: format!("edit_{plural}"),
            edit_others: format!("edit_others_{plural}"),
            publish: format!("publish_{plural}"),
            read_private: format!("read_private_{plural}"),
            delete_posts: format!("delete_{plural}"),
            delete_others: format!("delete_others_{plural}"),
            delete_published: format!("delete_published_{plural}"),
            delete_private: format!("delete_private_{plural}"),
            edit_private: format!("edit_private_{plural}"),
            edit_published: format!("edit_published_{plural}"),
        }
    }
}

/// What `map_meta_cap()` reads from the options of the site.
#[derive(Debug, Clone, Default)]
pub struct SiteCaps {
    /// `wp_page_for_privacy_policy`: editing or deleting it also needs `manage_privacy_options`.
    pub privacy_page: u64,
    /// `page_on_front` and `page_for_posts`: deleting them needs `manage_options`.
    pub page_on_front: u64,
    pub page_for_posts: u64,
    /// `link_manager_enabled`: without it `manage_links` is `do_not_allow`.
    pub link_manager_enabled: bool,
}

impl CurrentUser {
    fn all(&self, caps: &[&str]) -> bool {
        caps.iter().all(|c| self.can(c))
    }

    /// The caps of `edit_post`/`delete_post` for a post of the user or of somebody else (`map_meta_cap()`).
    /// A trashed post counts as not published (WordPress reads `_wp_trash_meta_status` here; only a contributor
    /// whose published post somebody else trashed would be answered differently).
    fn post_caps<'a>(&self, post: &Post, own_plain: &'a str, own_published: &'a str, others: &'a str, others_published: &'a str, others_private: &'a str) -> Vec<&'a str> {
        let own = post.post_author != 0 && post.post_author == self.id;
        let published = matches!(post.post_status.as_str(), "publish" | "future");
        if own {
            vec![if published { own_published } else { own_plain }]
        } else {
            let mut list = vec![others];
            if published {
                list.push(others_published);
            } else if post.post_status == "private" {
                list.push(others_private);
            }
            list
        }
    }

    /// `current_user_can( 'edit_post', $post )`. For a revision pass its parent.
    pub fn can_edit_post(&self, post: &Post) -> bool {
        if post.post_type == "revision" {
            return false;
        }
        let caps = TypeCaps::of(&post.post_type);
        let needed = self.post_caps(post, &caps.edit_posts, &caps.edit_published, &caps.edit_others, &caps.edit_published, &caps.edit_private);
        self.all(&needed) && (post.id == 0 || post.id != self.site_caps.privacy_page || self.can("manage_privacy_options"))
    }

    /// `current_user_can( 'delete_post', $post )`.
    pub fn can_delete_post(&self, post: &Post) -> bool {
        if post.post_type == "revision" {
            return false;
        }
        if post.id != 0 && (post.id == self.site_caps.page_on_front || post.id == self.site_caps.page_for_posts) {
            return self.can("manage_options");
        }
        let caps = TypeCaps::of(&post.post_type);
        let needed = self.post_caps(post, &caps.delete_posts, &caps.delete_published, &caps.delete_others, &caps.delete_published, &caps.delete_private);
        self.all(&needed) && (post.id == 0 || post.id != self.site_caps.privacy_page || self.can("manage_privacy_options"))
    }

    /// `current_user_can( 'read_post', $post )`. For a revision pass its parent.
    pub fn can_read_post(&self, post: &Post) -> bool {
        let caps = TypeCaps::of(&post.post_type);
        // Public statuses (`publish`, and `inherit` of attachments) need only `read`.
        if post.post_status == "publish" || (post.post_type == "attachment" && post.post_status == "inherit") {
            return self.can(&caps.read);
        }
        if post.post_author != 0 && post.post_author == self.id {
            return self.can(&caps.read);
        }
        if post.post_status == "private" {
            return self.can(&caps.read_private);
        }
        self.can_edit_post(post)
    }

    /// `current_user_can( 'publish_post', $post )` and `$post_type->cap->publish_posts`.
    pub fn can_publish_type(&self, post_type: &str) -> bool {
        self.can(&TypeCaps::of(post_type).publish)
    }

    /// `$post_type->cap->create_posts`.
    pub fn can_create_type(&self, post_type: &str) -> bool {
        self.can(&TypeCaps::of(post_type).create_posts)
    }

    /// `current_user_can( 'edit_comment', $comment )`: `edit_post` of the post the comment is on, `edit_posts`
    /// for a comment without a post.
    pub fn can_edit_comment(&self, post: Option<&Post>) -> bool {
        match post {
            Some(post) => self.can_edit_post(post),
            None => self.can("edit_posts"),
        }
    }

    /// `current_user_can( 'edit_user', $id )`: oneself, or with `edit_users`.
    pub fn can_edit_user(&self, user_id: u64) -> bool {
        self.id != 0 && (user_id == self.id || self.can("edit_users"))
    }
}
