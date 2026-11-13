//! The jobs `wp-cron.php` runs for content: publishing scheduled posts and cleaning up auto-drafts and the trash.

use rustpress_auth::CurrentUser;
use rustpress_core::Site;
use rustpress_db::{Db, Val, Writer};

use crate::content::{Content, Result};
use rustpress_content::cron::{CronArray, post_args};

/// The event of a scheduled post.
const HOOK: &str = "publish_future_post";

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

    // `publish_future_post`: the events whose time has come, as `wp-cron.php` runs them (each is unscheduled, then
    // `check_and_publish_future_post()` runs).
    let mut cron = CronArray::load(db).await?;
    let due = cron.due(HOOK, now);
    if !due.is_empty() {
        for (timestamp, args) in &due {
            cron.unschedule_event(*timestamp, HOOK, args);
        }
        cron.save(writer).await?;
    }
    for (_, args) in &due {
        let Some(id) = args.as_array().and_then(|a| a.first()).and_then(|(_, v)| v.as_i64()).filter(|id| *id > 0) else { continue };
        if check_and_publish_future_post(&content, id as u64).await? {
            report.published += 1;
        }
    }
    // Scheduled posts without an event (saved by RustPress before it kept the events, or whose event was lost)
    // are published as well; WordPress would leave them as "Missed schedule".
    let gmt_now = rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", now);
    let cron = CronArray::load(db).await?;
    for id in db.due_future_post_ids(&gmt_now).await? {
        if !cron.has(HOOK, &post_args(id)) && check_and_publish_future_post(&content, id).await? {
            report.published += 1;
        }
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

/// Whether a `publish_future_post` event is due: the check `_wp_cron()` makes on every request, here only for the
/// one hook RustPress runs. Reads the autoloaded option of the snapshot, so it costs no query.
pub fn publish_due(site: &Site, now: i64) -> bool {
    site.options.get("cron").is_some_and(|raw| !CronArray::parse(raw).due(HOOK, now).is_empty())
}

/// `check_and_publish_future_post()`: publishes a scheduled post whose date has come, and schedules one that was
/// run too early again. Returns whether the post was published.
async fn check_and_publish_future_post(content: &Content<'_>, id: u64) -> Result<bool> {
    let Some(post) = content.db.post(id).await? else { return Ok(false) };
    if post.post_status != "future" {
        return Ok(false);
    }
    let time = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date_gmt).unwrap_or(0);
    if time > content.now {
        let mut cron = CronArray::load(content.db).await?;
        cron.clear_scheduled_hook(HOOK, &post_args(id));
        cron.schedule_single_event(time, HOOK, post_args(id), content.now);
        cron.save(content.writer).await?;
        return Ok(false);
    }
    publish_post(content, &post).await?;
    Ok(true)
}

/// `wp_publish_post()`: a post without a category gets the default one, then the status becomes `publish` and the
/// transition runs (the event is cleared, the term counts follow). The plugins' `save_post` is not called.
async fn publish_post(content: &Content<'_>, post: &rustpress_db::Post) -> Result<()> {
    let id = post.id;
    if content.object_taxonomies(&post.post_type).iter().any(|(name, _)| name == "category") && content.db.terms_for_post(id, Some("category")).await?.is_empty() {
        let default = content.site.options.u64("default_category", 0);
        if default != 0 {
            content.set_object_terms(id, &[rustpress_content::content::TermRef::Id(default)], "category", false).await?;
        }
    }
    content.writer.update_rows("posts", "ID", id, &[("post_status", Val::Str("publish".into()))]).await?;
    content.transition_post_status(id, "publish", &post.post_date).await?;
    let terms: Vec<u64> = content.db.terms_for_post(id, None).await?.into_iter().map(|t| t.term_taxonomy_id).collect();
    content.update_term_counts(&terms).await?;
    Ok(())
}
