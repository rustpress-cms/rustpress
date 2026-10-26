//! The rules a new comment goes through before it is saved, shared by `wp-comments-post.php` and the REST API:
//! `wp_allow_comment()` (duplicates, floods), `wp_check_comment_data()`, `check_comment()` and the filters of
//! `wp_filter_comment()`.

use rustpress_auth::{CurrentUser, Roles};
use rustpress_core::Site;
use rustpress_core::formatting::esc_url_raw;
use rustpress_db::Db;

/// A comment as it is about to be saved.
#[derive(Debug, Clone, Default)]
pub struct NewComment {
    pub post_id: u64,
    pub author: String,
    pub email: String,
    pub url: String,
    pub content: String,
    pub comment_type: String,
    pub parent: u64,
    pub user_id: Option<u64>,
    pub ip: String,
    pub agent: String,
}

/// A word list of the settings (`moderation_keys`, `disallowed_keys`): one regular expression per line.
pub fn matches_list(list: &str, texts: &[&str]) -> bool {
    for word in list.lines().map(str::trim).filter(|w| !w.is_empty()) {
        let Ok(pattern) = regex::Regex::new(&format!("(?iu){}", regex::escape(word))) else { continue };
        if texts.iter().any(|text| pattern.is_match(text)) {
            return true;
        }
    }
    false
}

/// `check_comment()`: whether a comment may appear at once.
pub async fn check_comment(site: &Site, db: &Db, c: &NewComment) -> bool {
    let options = &site.options;
    if options.string("comment_moderation", "0") == "1" {
        return false;
    }
    // Links that were not written as links count, too.
    let rendered = rustpress_blocks::content::comment_text(site, &c.content);
    let max_links = options.string("comment_max_links", "2").parse::<usize>().unwrap_or(0);
    if max_links > 0 {
        static LINK: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let count = LINK.get_or_init(|| regex::Regex::new(r"(?i)<a [^>]*href").expect("pattern")).find_iter(&rendered).count();
        if count >= max_links {
            return false;
        }
    }
    let keys = options.string("moderation_keys", "");
    if !keys.trim().is_empty() && matches_list(keys.trim(), &[&c.author, &c.email, &c.url, &rendered, &c.ip, &c.agent]) {
        return false;
    }
    if options.string("comment_previously_approved", "1") == "1" {
        if c.comment_type != "trackback" && c.comment_type != "pingback" && !c.author.is_empty() && !c.email.is_empty() {
            let known = db.user_by_email(&crate::sanitize::sanitize_email(&c.email)).await.ok().flatten();
            let ok = match known {
                Some(user) => db.has_approved_comment(Some(user.id), &c.author, &c.email).await.unwrap_or(false),
                None => db.has_approved_comment(None, &c.author, &c.email).await.unwrap_or(false),
            };
            return ok && (keys.trim().is_empty() || !c.email.contains(keys.trim()));
        }
        return false;
    }
    false
}

/// `wp_check_comment_data()`: `1`, `0`, or `trash` for a comment the disallowed list names.
pub async fn check_comment_data(site: &Site, db: &Db, roles: &Roles, c: &NewComment) -> String {
    if let Some(user_id) = c.user_id.filter(|id| *id != 0)
        && let Some(author) = db.auth_user_by_id(user_id).await.ok().flatten()
        && let Ok(user) = CurrentUser::load(db, roles, &author).await
    {
        let post_author = db.post(c.post_id).await.ok().flatten().map_or(0, |p| p.post_author);
        if user_id == post_author || user.can("moderate_comments") {
            return "1".into();
        }
    }
    let mut approved = if check_comment(site, db, c).await { "1".to_owned() } else { "0".to_owned() };
    let disallowed = site.options.string("disallowed_keys", "");
    if !disallowed.trim().is_empty() {
        let without_html = rustpress_core::formatting::strip_tags(&c.content);
        if matches_list(disallowed.trim(), &[&c.author, &c.email, &c.url, &c.content, &without_html, &c.ip, &c.agent]) {
            approved = "trash".into();
        }
    }
    approved
}

/// The duplicate check of `wp_allow_comment()`: the same author (and address) said the same under the same parent.
pub async fn is_duplicate(db: &Db, c: &NewComment) -> bool {
    db.duplicate_comment(c.post_id, c.parent, &c.author, &c.email, &c.content).await.ok().flatten().is_some()
}

/// `wp_check_comment_flood()`: a comment of the same user (or address without one), or with the same email address,
/// less than 15 seconds ago. Administrators and moderators are never too quick.
pub async fn is_flood(site: &Site, db: &Db, user: Option<&CurrentUser>, ip: &str, email: &str, now: i64) -> bool {
    if user.is_some_and(|u| u.can("manage_options") || u.can("moderate_comments")) {
        return false;
    }
    let hour_ago = site.date_utc("Y-m-d H:i:s", now - 3600);
    let last = db.last_comment_since(user.map(|u| u.id).filter(|id| *id != 0), ip, email, &hour_ago).await;
    match last.ok().flatten() {
        Some(last) => now - rustpress_core::datetime::SiteTimezone::gmt_timestamp(&last).unwrap_or(0) < 15,
        None => false,
    }
}

/// `pre_comment_author_name`: `sanitize_text_field`, `wp_filter_kses`, `_wp_specialchars`.
pub fn filter_text(text: &str) -> String {
    let text = rustpress_core::kses::kses_data(&crate::sanitize::text_field(text));
    rustpress_core::formatting::specialchars_noquotes(&text)
}

/// `pre_comment_author_email`: `trim`, `sanitize_email`, `wp_filter_kses`.
pub fn filter_email(email: &str) -> String {
    rustpress_core::kses::kses_data(&crate::sanitize::sanitize_email(email.trim()))
}

/// `pre_comment_author_url`: `wp_strip_all_tags`, `sanitize_url`, `wp_filter_kses`.
pub fn filter_url(url: &str) -> String {
    let url = rustpress_core::formatting::strip_tags(url);
    let url = if url.trim().is_empty() { String::new() } else { esc_url_raw(url.trim()) };
    rustpress_core::kses::kses_data(&url)
}

/// `comments_open()`: open, and not closed for old posts.
pub fn comments_open(site: &Site, post: &rustpress_db::Post, now: i64) -> bool {
    if post.comment_status != "open" {
        return false;
    }
    if site.options.string("close_comments_for_old_posts", "0") != "0" && post.post_type == "post" {
        let days = site.options.string("close_comments_days_old", "14").parse::<i64>().unwrap_or(0);
        if days != 0
            && post.post_date_gmt != "0000-00-00 00:00:00"
            && let Some(posted) = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date_gmt)
            && now - posted > days * 86_400
        {
            return false;
        }
    }
    true
}
