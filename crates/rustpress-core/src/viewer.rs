//! The person who looks at the site: who they are and what they may do. The main query asks for it to show
//! private posts and previews of posts that are not published (`current_user_can()` in `WP_Query::get_posts()`);
//! the frontend shows the toolbar and the comment form of a signed-in user from it.

use std::sync::Arc;

use rustpress_db::Post;

#[derive(Clone)]
pub struct Viewer {
    pub id: u64,
    pub login: String,
    pub display_name: String,
    pub email: String,
    /// `show_admin_bar()` of the user: the toolbar is printed.
    pub show_admin_bar: bool,
    /// The page of the privacy policy: only who manages the privacy options may edit it.
    pub privacy_page: u64,
    can: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    nonce: Arc<dyn Fn(&str) -> String + Send + Sync>,
    verify: Arc<dyn Fn(&str, &str) -> bool + Send + Sync>,
}

impl std::fmt::Debug for Viewer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Viewer").field("id", &self.id).finish()
    }
}

impl Viewer {
    pub fn new(id: u64, can: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        Self { id, login: String::new(), display_name: String::new(), email: String::new(), show_admin_bar: false, privacy_page: 0, can: Arc::new(can), nonce: Arc::new(|_| String::new()), verify: Arc::new(|_, _| false) }
    }

    /// Who the viewer is, and how a nonce of their session is made.
    pub fn with_identity(mut self, login: &str, display_name: &str, email: &str, show_admin_bar: bool, nonce: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        self.login = login.to_owned();
        self.display_name = display_name.to_owned();
        self.email = email.to_owned();
        self.show_admin_bar = show_admin_bar;
        self.nonce = Arc::new(nonce);
        self
    }

    /// How a nonce of the viewer's session is checked (`wp_verify_nonce()`).
    pub fn with_verifier(mut self, privacy_page: u64, verify: impl Fn(&str, &str) -> bool + Send + Sync + 'static) -> Self {
        self.privacy_page = privacy_page;
        self.verify = Arc::new(verify);
        self
    }

    /// `wp_verify_nonce( $nonce, $action )`.
    pub fn check_nonce(&self, nonce: &str, action: &str) -> bool {
        (self.verify)(nonce, action)
    }

    /// `current_user_can( $capability )`.
    pub fn can(&self, capability: &str) -> bool {
        (self.can)(capability)
    }

    /// `wp_create_nonce( $action )`.
    pub fn create_nonce(&self, action: &str) -> String {
        (self.nonce)(action)
    }

    /// The suffix of the capabilities of a post type: `posts`, or `pages` for pages.
    fn suffix(post_type: &str) -> &'static str {
        if post_type == "page" { "pages" } else { "posts" }
    }

    /// Whether the viewer may read the private posts of the type (`read_private_posts`).
    pub fn can_read_private(&self, post_type: &str) -> bool {
        self.can(&format!("read_private_{}", Self::suffix(post_type)))
    }

    /// `current_user_can( 'edit_post', $post->ID )`, the capabilities `map_meta_cap()` asks for.
    pub fn can_edit_post(&self, post: &Post) -> bool {
        if post.post_type == "page" && self.privacy_page == post.id && post.id != 0 {
            return self.can("manage_privacy_options");
        }
        let s = Self::suffix(&post.post_type);
        let published = matches!(post.post_status.as_str(), "publish" | "future");
        if post.post_author == self.id {
            if published { self.can(&format!("edit_published_{s}")) } else { self.can(&format!("edit_{s}")) }
        } else {
            let mut ok = self.can(&format!("edit_others_{s}"));
            if published {
                ok = ok && self.can(&format!("edit_published_{s}"));
            } else if post.post_status == "private" {
                ok = ok && self.can(&format!("edit_private_{s}"));
            }
            ok
        }
    }

    /// `current_user_can( 'read_post', $post->ID )` for a private post.
    pub fn can_read_post(&self, post: &Post) -> bool {
        post.post_author == self.id || self.can_read_private(&post.post_type)
    }
}
