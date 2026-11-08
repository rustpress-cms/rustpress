//! The actions of `admin-ajax.php` (for visitors and signed-in users alike): `wpforms_submit`
//! (`WPForms_Process::ajax_submit()`), `wpforms_get_token` (`Token::ajax_get_token()`) and `wpforms_restricted_email`
//! (`WPForms_Field_Email::ajax_check_restricted_email()`), with what a saved entry changes in the database
//! (`WPForms_Lite::update_entry_count()`, `EntriesWindow::record()`, `Cache::invalidate()`,
//! `DashboardWidget::clear_widget_cache()`).

use rustpress_core::native::{AdminContext, AjaxAnswer};
use rustpress_core::{Result, Site};
use rustpress_phpser::{Key, Value as Php};
use serde_json::Value;

use super::submit::{self, Form, Request, User};
use super::{DOMAIN, Loaded, php};

/// The answer to one of the plugin's actions, `None` for any other action.
pub async fn handle(loaded: &Loaded, action: &str, ctx: &AdminContext<'_>) -> Result<Option<AjaxAnswer>> {
    if !matches!(action, "wpforms_submit" | "wpforms_get_token" | "wpforms_restricted_email") {
        return Ok(None);
    }
    let site = ctx.site;
    let post = submit::post_tree(ctx.form);
    let get = submit::post_tree(ctx.query);
    let form_id = php::int(post.get("wpforms").and_then(|w| w.get("id"))).max(0) as u64;
    let form = loaded.form(form_id);
    let needs_users = action == "wpforms_submit" && form.is_some_and(|f| !php::empty(f.data.get("settings").and_then(|s| s.get("notification_enable"))));
    let facts = Facts::load(site, ctx, &post, form.map(|f| &f.data), needs_users).await?;
    let tr = |text: &str| site.trd(DOMAIN, text);
    let trn = |singular: &str, plural: &str, n: i64| site.i18n().ngettext(DOMAIN, singular, plural, n);
    let date = |format: &str, at: i64| site.date_utc(format, at);
    let verify = |nonce: &str, action: &str| (ctx.verify_nonce)(nonce, action);
    let shortcodes = site.shortcode_tags();
    let request = Request {
        settings: &loaded.settings,
        tr: &tr,
        trn: &trn,
        post: &post,
        get: &get,
        user: facts.user.clone(),
        can_manage: (ctx.can)("manage_options"),
        super_admin: (ctx.can)("delete_users"),
        verify_nonce: &verify,
        now: ctx.now,
        remote_addr: ctx.remote_addr,
        origin: facts.origin.clone(),
        request_uri: ctx.request_uri.to_owned(),
        admin_email: site.options.string("admin_email", ""),
        blogname: site.options.string("blogname", ""),
        home_url: site.home_url(""),
        home_host: facts.home_host.clone(),
        login_url: site.site_url("wp-login.php"),
        charset: site.options.string("blog_charset", "UTF-8"),
        language_attributes: facts.language_attributes.clone(),
        rtl: site.is_rtl(),
        date: &date,
        gmt_offset: facts.gmt_offset,
        date_format: site.options.string("date_format", ""),
        secret: &loaded.secret,
        permalinks: facts.permalinks.clone(),
        shortcodes: &shortcodes,
        page_author: facts.page_author.clone(),
        manager_emails: facts.manager_emails.clone(),
        plugin_dir: Some(super::notify::plugin_dir(&ctx.wp_root.join("wp-content"))),
    };
    let forms = |id: u64| loaded.form(id).map(|f| Form { status: &f.status, data: &f.data, post_date: &f.date });
    let body = match action {
        "wpforms_get_token" => submit::ajax_get_token(&request),
        "wpforms_restricted_email" => submit::ajax_restricted_email(&request, &forms),
        _ => {
            let outcome = submit::ajax_submit(&forms, &request);
            // `wp_mail()`; WPForms does not look at the result.
            if let Some(mailer) = ctx.mailer {
                for email in &outcome.emails {
                    mailer.send(email).await;
                }
            }
            if outcome.complete
                && let Some(writer) = ctx.writer
            {
                saved(site, writer, &loaded.settings, form_id, outcome.saved, ctx.now).await?;
            }
            outcome.body
        }
    };
    Ok(Some(AjaxAnswer::json(200, body)))
}

/// What the request needs from the database and the site before it is processed.
#[derive(Default)]
struct Facts {
    user: Option<User>,
    origin: String,
    home_host: String,
    language_attributes: String,
    gmt_offset: f64,
    permalinks: Vec<(i64, String)>,
    page_author: Option<(u64, String)>,
    manager_emails: Vec<String>,
}

impl Facts {
    async fn load(site: &Site, ctx: &AdminContext<'_>, post: &Value, form: Option<&Value>, needs_users: bool) -> Result<Self> {
        let mut facts = Facts::default();
        if ctx.user != 0
            && let Some(u) = site.db.user(ctx.user).await?
        {
            let first_name = site.db.user_meta(u.id, "first_name").await?.unwrap_or_default();
            let last_name = site.db.user_meta(u.id, "last_name").await?.unwrap_or_default();
            facts.user = Some(User { id: u.id, display_name: u.display_name, email: u.user_email, first_name, last_name });
        }
        let home = site.home_url("");
        let host = home.split_once("://").map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or_default()).unwrap_or_default();
        facts.origin = home.split_once("://").map(|(scheme, _)| format!("{scheme}://{host}")).unwrap_or_default();
        facts.home_host = host.split(':').next().unwrap_or_default().to_ascii_lowercase();
        // `get_language_attributes()`.
        let mut attributes = Vec::new();
        if site.is_rtl() {
            attributes.push("dir=\"rtl\"".to_owned());
        }
        let lang = site.language_tag();
        if !lang.is_empty() {
            attributes.push(format!("lang=\"{}\"", rustpress_core::formatting::esc_attr(&lang)));
        }
        facts.language_attributes = attributes.join(" ");
        // `get_option( 'gmt_offset' )`, which follows the time zone of the site.
        let offset = site.timezone.format("Z", ctx.now).trim().parse::<f64>().unwrap_or(0.0);
        facts.gmt_offset = offset / 3600.0;
        // `get_permalink()` of the pages the confirmations of the form go to.
        if let Some(settings) = form.and_then(|f| f.get("settings")) {
            let mut pages = Vec::new();
            if let Some(Value::Object(confirmations)) = settings.get("confirmations") {
                pages.extend(confirmations.values().map(|c| php::str_of(c, "page")));
            }
            pages.push(php::str_of(settings, "confirmation_page"));
            for page in pages {
                let id = php::leading_int(&page);
                if id <= 0 || facts.permalinks.iter().any(|(p, _)| *p == id) {
                    continue;
                }
                if let Some(p) = site.db.post(id as u64).await? {
                    facts.permalinks.push((id, site.permalink(&p).await?));
                }
            }
        }
        // The author of the page the form is on (`AuthorId`, `AuthorDisplay`).
        let page_id = php::int(post.get("page_id")).unsigned_abs();
        if page_id != 0 {
            let author = match site.db.post(page_id).await? {
                Some(p) => site.db.user(p.post_author).await?,
                None => None,
            };
            facts.page_author = Some(author.map_or((0, String::new()), |a| (a.id, a.display_name)));
        }
        if needs_users {
            facts.manager_emails = manager_emails(site).await?;
        }
        Ok(facts)
    }
}

/// The e-mail addresses of the users who can `manage_options`, lowercase.
async fn manager_emails(site: &Site) -> Result<Vec<String>> {
    let prefix = site.db.table_prefix();
    let roles = site.options.fetch(&site.db, &format!("{prefix}user_roles")).await?.unwrap_or_default();
    let roles = rustpress_auth::caps::Roles::from_option(&roles);
    let granted = |caps: &Php| -> Option<bool> {
        let items = caps.as_array()?;
        let mut role_grants = false;
        for (key, flag) in items {
            let Key::Str(name) = key else { continue };
            let name = String::from_utf8_lossy(name);
            let on = !matches!(flag, Php::Bool(false) | Php::Null) && flag.as_i64() != Some(0);
            if name == "manage_options" {
                return Some(on);
            }
            if on && roles.capabilities(&name).is_some_and(|c| c.get("manage_options").copied().unwrap_or(false)) {
                role_grants = true;
            }
        }
        Some(role_grants)
    };
    let ids: Vec<u64> = site
        .db
        .user_capability_rows()
        .await?
        .into_iter()
        .filter(|(_, caps)| caps.as_deref().and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok()).and_then(|c| granted(&c)).unwrap_or(false))
        .map(|(id, _)| id)
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(site.db.users_by_ids(&ids).await?.into_iter().map(|u| u.user_email.to_lowercase()).collect())
}

/// What `wpforms_process_entry_saved` and `wpforms_process_complete` change for a saved entry.
async fn saved(site: &Site, writer: &rustpress_db::Writer, settings: &super::settings::Settings, form_id: u64, counted: bool, now: i64) -> Result<()> {
    let options = writer.table("options");
    let option = |name: &str| format!("SELECT option_value FROM {options} WHERE option_name = '{name}' LIMIT 1");
    // `WPForms_Lite::update_entry_count()`.
    if counted && form_id != 0 {
        let postmeta = writer.table("postmeta");
        // `EntriesWindow::record()` (before the count changes).
        let stored = writer.column_string(&option("wpforms_entries_count_daily")).await?.into_iter().next();
        let stored = stored.and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok());
        let since = stored.as_ref().and_then(|v| v.get("since")).filter(|s| !matches!(s, Php::Null | Php::Bool(false)) && s.as_str() != Some("") && s.as_str() != Some("0") && s.as_i64() != Some(0));
        let (since, mut days): (i64, Vec<(String, i64)>) = match since {
            Some(since) => {
                let days = stored.as_ref().and_then(|v| v.get("days")).and_then(Php::as_array).unwrap_or_default();
                let days = days
                    .iter()
                    .filter_map(|(k, v)| {
                        let key = match k {
                            Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
                            Key::Int(i) => i.to_string(),
                        };
                        let count = v.as_i64().or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()).map(|f| f as i64))?;
                        Some((key, count))
                    })
                    .collect();
                (since.as_i64().or_else(|| since.as_str().map(php::leading_int)).unwrap_or(0), days)
            }
            None => {
                // `EntriesWindow::create()`, stored at once.
                let total = writer.column_u64(&format!("SELECT CAST(COALESCE(SUM(meta_value), 0) AS UNSIGNED) FROM {postmeta} WHERE meta_key = 'wpforms_entries_count'")).await?.into_iter().next().unwrap_or(0);
                let mut since = now;
                if total == 0 {
                    let activated = site.options.fetch(&site.db, "wpforms_activated").await?.and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok());
                    let lite = activated.as_ref().and_then(|a| a.get("lite")).and_then(|l| l.as_i64().or_else(|| l.as_str().map(php::leading_int))).unwrap_or(0);
                    if lite > 0 {
                        since = lite;
                    }
                }
                writer.update_option_with_autoload("wpforms_entries_count_daily", &window(since, &[]), "off").await?;
                (since, Vec::new())
            }
        };
        let today = gmdate(now);
        match days.iter_mut().find(|(d, _)| *d == today) {
            Some(slot) => slot.1 += 1,
            None => days.push((today, 1)),
        }
        let oldest = gmdate(now - 30 * 86_400);
        days.retain(|(d, _)| d.as_str() >= oldest.as_str());
        writer.update_option_with_autoload("wpforms_entries_count_daily", &window(since, &days), "off").await?;
        let existing = writer.column_u64(&format!("SELECT meta_id FROM {postmeta} WHERE post_id = {form_id} AND meta_key = 'wpforms_entries_count' LIMIT 1")).await?;
        if existing.is_empty() {
            writer.add_post_meta(form_id, "wpforms_entries_count", "1").await?;
        } else {
            writer.execute(format!("UPDATE {postmeta} SET meta_value = meta_value + 1 WHERE post_id = {form_id} AND meta_key = 'wpforms_entries_count'")).await?;
        }
    }
    // `Cache::invalidate()`.
    let generation = writer.column_string(&option("wpforms_dashboard_cache_generation")).await?.into_iter().next().map_or(0, |g| php::leading_int(&rustpress_phpser::unserialize(g.as_bytes()).ok().and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(str::to_owned))).unwrap_or(g)));
    writer.update_option_with_autoload("wpforms_dashboard_cache_generation", &(generation + 1).to_string(), "off").await?;
    let currency = settings.string("currency", "USD").to_lowercase();
    for preset in ["0", "1", "7", "30", "90", "365"] {
        let key = format!("dashboard_cache_{preset}_{currency}_lite");
        if writer.delete_option(&format!("_wpforms_transient_{key}")).await? {
            writer.delete_option(&format!("_wpforms_transient_timeout_{key}")).await?;
        }
    }
    // `DashboardWidget::clear_widget_cache()`: `delete_transient()`.
    if writer.delete_option("_transient_wpforms_dash_widget_lite_entries_by_form").await? {
        writer.delete_option("_transient_timeout_wpforms_dash_widget_lite_entries_by_form").await?;
    }
    Ok(())
}

/// `gmdate( 'Y-m-d', $timestamp )`.
fn gmdate(at: i64) -> String {
    rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d", at)
}

/// The option `wpforms_entries_count_daily` as `serialize()` writes it.
fn window(since: i64, days: &[(String, i64)]) -> String {
    let days: Vec<(Key, Php)> = days.iter().map(|(d, n)| (Key::Str(d.as_bytes().to_vec()), Php::Int(*n))).collect();
    let value = Php::Array(vec![(Key::Str(b"since".to_vec()), Php::Int(since)), (Key::Str(b"days".to_vec()), Php::Array(days))]);
    String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned()
}
