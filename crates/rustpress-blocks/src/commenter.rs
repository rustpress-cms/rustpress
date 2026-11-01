//! Who is writing comments: the cookies `comment_author_*` of a visitor (`wp_get_current_commenter()`) and the
//! pending comment somebody has just sent (`wp_get_unapproved_comment_author_email()`).

use std::sync::Arc;

use crate::Result;
use crate::env::Env;

/// The commenter of the request being answered: the cleaned values of the three cookies, and how `wp_hash()` is made.
#[derive(Clone)]
pub struct Commenter {
    pub author: String,
    pub email: String,
    pub url: String,
    /// `wp_hash( $data, $scheme )`.
    hash: Arc<dyn Fn(&str, &str) -> String + Send + Sync>,
}

impl Commenter {
    pub fn new(author: String, email: String, url: String, hash: impl Fn(&str, &str) -> String + Send + Sync + 'static) -> Self {
        Self { author, email, url, hash: Arc::new(hash) }
    }
}

/// `wp_hash( $data, $scheme )` with the salts of the site; empty outside a request or without the salts.
pub fn wp_hash(data: &str, scheme: &str) -> String {
    COMMENTER.try_with(|c| (c.hash)(data, scheme)).unwrap_or_default()
}

tokio::task_local! {
    pub static COMMENTER: Commenter;
}

/// `wp_get_current_commenter()`: `(author, email, url)`, empty without the cookies.
pub fn current_commenter() -> (String, String, String) {
    COMMENTER.try_with(|c| (c.author.clone(), c.email.clone(), c.url.clone())).unwrap_or_default()
}

/// Which pending comments the page shows besides the approved ones (`include_unapproved` of `comments_template()`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IncludeUnapproved {
    pub user: Option<u64>,
    pub email: Option<String>,
}

/// `wp_get_unapproved_comment_author_email()`: the address of the comment named by `?unapproved=ID&moderation-hash=`
/// (for ten minutes after it was sent), otherwise the address of the cookie.
pub async fn unapproved_author_email(env: &Env<'_>, now: i64) -> Result<String> {
    let vars = rustpress_core::QueryVars::parse(&env.query.raw_query);
    let id = vars.u64("unapproved").unwrap_or(0);
    let given = vars.get("moderation-hash").unwrap_or_default().to_owned();
    if id != 0 && !given.is_empty()
        && let Some(comment) = env.site.db.comment_any(id).await? {
            let expected = wp_hash(&comment.comment_date_gmt, "auth");
            if !expected.is_empty() && rustpress_core::hash_equals(&expected, &given) {
                let sent = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&comment.comment_date_gmt).unwrap_or(0);
                if now < sent + 600 && !comment.comment_author_email.is_empty() {
                    return Ok(comment.comment_author_email);
                }
            }
        }
    Ok(current_commenter().1)
}

/// What `comments_template()` asks for: the pending comments of the signed-in user, or of the commenter.
pub async fn include_unapproved(env: &Env<'_>) -> Result<IncludeUnapproved> {
    if let Some(viewer) = env.query.viewer.as_ref().filter(|v| v.id != 0) {
        return Ok(IncludeUnapproved { user: Some(viewer.id), email: None });
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let email = unapproved_author_email(env, now).await?;
    Ok(IncludeUnapproved { user: None, email: Some(email).filter(|e| !e.is_empty()) })
}
