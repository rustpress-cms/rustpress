//! The jobs `wp-cron.php` runs for content: publishing scheduled posts and cleaning up auto-drafts and the trash.

use rustpress_auth::CurrentUser;
use rustpress_core::Site;
use rustpress_db::{Db, Val, Writer};

use crate::content::{Content, Result};

/// What a run did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub published: u64,
    pub auto_drafts_deleted: u64,
    pub trash_purged: u64,
}

const DAY: i64 = 86_400;
/// `EMPTY_TRASH_DAYS`.
const TRASH_DAYS: i64 = 30;

/// Runs the due jobs once.
pub async fn run(site: &Site, db: &Db, writer: &Writer, now: i64) -> Result<Report> {
    let user = CurrentUser::system();
    let content = Content { site, db, writer, user: &user, now };
    let mut report = Report::default();

    // `publish_future_post`: scheduled posts whose time has come.
    let gmt_now = rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", now);
    for id in db.due_future_post_ids(&gmt_now).await? {
        writer.update_rows("posts", "ID", id, &[("post_status", Val::Str("publish".into()))]).await?;
        let terms: Vec<u64> = db.terms_for_post(id, None).await?.into_iter().map(|t| t.term_taxonomy_id).collect();
        content.update_term_counts(&terms).await?;
        report.published += 1;
    }

    // `wp_delete_auto_drafts()`: auto-drafts older than a week.
    let week_ago = rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", now - 7 * DAY);
    for id in db.old_auto_draft_ids(&week_ago).await? {
        if content.delete_post(id, true).await.unwrap_or(false) {
            report.auto_drafts_deleted += 1;
        }
    }

    // `wp_scheduled_delete()`: what has been in the trash for 30 days.
    for id in db.trashed_before(now - TRASH_DAYS * DAY).await? {
        if content.delete_post(id, true).await.unwrap_or(false) {
            report.trash_purged += 1;
        }
    }
    Ok(report)
}
