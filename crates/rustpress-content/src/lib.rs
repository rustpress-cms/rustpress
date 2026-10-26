//! Writing content: posts, terms, users, comments, attachments and uploads, ported from
//! `wp-includes/post.php`, `taxonomy.php`, `user.php`, `comment.php` and `media.php`. Used by the
//! administration screens and by the REST API.

pub mod comment_rules;
pub mod content;
pub mod media;
pub mod sanitize;
