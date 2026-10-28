//! `wp_install()`: the tables, options, roles, first user and first content of a new site
//! (`wp-admin/includes/upgrade.php` and `schema.php`).

use rustpress_core::Site;
use rustpress_core::slug::sanitize_title_save;
use rustpress_db::{Val, Writer};
use rustpress_phpser::{Key, Value};

/// `wp_get_db_schema( 'all' )` of a single site, one statement per table.
pub fn schema(writer: &Writer, charset_collate: &str) -> Vec<String> {
    let t = |name: &str| writer.table(name);
    let max = 191;
    vec![
        format!("CREATE TABLE IF NOT EXISTS {} (\n\tID bigint(20) unsigned NOT NULL auto_increment,\n\tuser_login varchar(60) NOT NULL default '',\n\tuser_pass varchar(255) NOT NULL default '',\n\tuser_nicename varchar(50) NOT NULL default '',\n\tuser_email varchar(100) NOT NULL default '',\n\tuser_url varchar(100) NOT NULL default '',\n\tuser_registered datetime NOT NULL default '0000-00-00 00:00:00',\n\tuser_activation_key varchar(255) NOT NULL default '',\n\tuser_status int(11) NOT NULL default '0',\n\tdisplay_name varchar(250) NOT NULL default '',\n\tPRIMARY KEY  (ID),\n\tKEY user_login_key (user_login),\n\tKEY user_nicename (user_nicename),\n\tKEY user_email (user_email)\n) {charset_collate}", t("users")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n\tumeta_id bigint(20) unsigned NOT NULL auto_increment,\n\tuser_id bigint(20) unsigned NOT NULL default '0',\n\tmeta_key varchar(255) default NULL,\n\tmeta_value longtext,\n\tPRIMARY KEY  (umeta_id),\n\tKEY user_id (user_id),\n\tKEY meta_key (meta_key({max}))\n) {charset_collate}", t("usermeta")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n\tmeta_id bigint(20) unsigned NOT NULL auto_increment,\n\tterm_id bigint(20) unsigned NOT NULL default '0',\n\tmeta_key varchar(255) default NULL,\n\tmeta_value longtext,\n\tPRIMARY KEY  (meta_id),\n\tKEY term_id (term_id),\n\tKEY meta_key (meta_key({max}))\n) {charset_collate}", t("termmeta")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n term_id bigint(20) unsigned NOT NULL auto_increment,\n name varchar(200) NOT NULL default '',\n slug varchar(200) NOT NULL default '',\n term_group bigint(10) NOT NULL default 0,\n PRIMARY KEY  (term_id),\n KEY slug (slug({max})),\n KEY name (name({max}))\n) {charset_collate}", t("terms")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n term_taxonomy_id bigint(20) unsigned NOT NULL auto_increment,\n term_id bigint(20) unsigned NOT NULL default 0,\n taxonomy varchar(32) NOT NULL default '',\n description longtext NOT NULL,\n parent bigint(20) unsigned NOT NULL default 0,\n count bigint(20) NOT NULL default 0,\n PRIMARY KEY  (term_taxonomy_id),\n UNIQUE KEY term_id_taxonomy (term_id,taxonomy),\n KEY taxonomy (taxonomy)\n) {charset_collate}", t("term_taxonomy")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n object_id bigint(20) unsigned NOT NULL default 0,\n term_taxonomy_id bigint(20) unsigned NOT NULL default 0,\n term_order int(11) NOT NULL default 0,\n PRIMARY KEY  (object_id,term_taxonomy_id),\n KEY term_taxonomy_id (term_taxonomy_id)\n) {charset_collate}", t("term_relationships")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n\tmeta_id bigint(20) unsigned NOT NULL auto_increment,\n\tcomment_id bigint(20) unsigned NOT NULL default '0',\n\tmeta_key varchar(255) default NULL,\n\tmeta_value longtext,\n\tPRIMARY KEY  (meta_id),\n\tKEY comment_id (comment_id),\n\tKEY meta_key (meta_key({max}))\n) {charset_collate}", t("commentmeta")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n\tcomment_ID bigint(20) unsigned NOT NULL auto_increment,\n\tcomment_post_ID bigint(20) unsigned NOT NULL default '0',\n\tcomment_author tinytext NOT NULL,\n\tcomment_author_email varchar(100) NOT NULL default '',\n\tcomment_author_url varchar(200) NOT NULL default '',\n\tcomment_author_IP varchar(100) NOT NULL default '',\n\tcomment_date datetime NOT NULL default '0000-00-00 00:00:00',\n\tcomment_date_gmt datetime NOT NULL default '0000-00-00 00:00:00',\n\tcomment_content text NOT NULL,\n\tcomment_karma int(11) NOT NULL default '0',\n\tcomment_approved varchar(20) NOT NULL default '1',\n\tcomment_agent varchar(255) NOT NULL default '',\n\tcomment_type varchar(20) NOT NULL default 'comment',\n\tcomment_parent bigint(20) unsigned NOT NULL default '0',\n\tuser_id bigint(20) unsigned NOT NULL default '0',\n\tPRIMARY KEY  (comment_ID),\n\tKEY comment_post_ID (comment_post_ID),\n\tKEY comment_approved_date_gmt (comment_approved,comment_date_gmt),\n\tKEY comment_date_gmt (comment_date_gmt),\n\tKEY comment_parent (comment_parent),\n\tKEY comment_author_email (comment_author_email(10))\n) {charset_collate}", t("comments")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n\tlink_id bigint(20) unsigned NOT NULL auto_increment,\n\tlink_url varchar(255) NOT NULL default '',\n\tlink_name varchar(255) NOT NULL default '',\n\tlink_image varchar(255) NOT NULL default '',\n\tlink_target varchar(25) NOT NULL default '',\n\tlink_description varchar(255) NOT NULL default '',\n\tlink_visible varchar(20) NOT NULL default 'Y',\n\tlink_owner bigint(20) unsigned NOT NULL default '1',\n\tlink_rating int(11) NOT NULL default '0',\n\tlink_updated datetime NOT NULL default '0000-00-00 00:00:00',\n\tlink_rel varchar(255) NOT NULL default '',\n\tlink_notes mediumtext NOT NULL,\n\tlink_rss varchar(255) NOT NULL default '',\n\tPRIMARY KEY  (link_id),\n\tKEY link_visible (link_visible)\n) {charset_collate}", t("links")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n\toption_id bigint(20) unsigned NOT NULL auto_increment,\n\toption_name varchar(191) NOT NULL default '',\n\toption_value longtext NOT NULL,\n\tautoload varchar(20) NOT NULL default 'yes',\n\tPRIMARY KEY  (option_id),\n\tUNIQUE KEY option_name (option_name),\n\tKEY autoload (autoload)\n) {charset_collate}", t("options")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n\tmeta_id bigint(20) unsigned NOT NULL auto_increment,\n\tpost_id bigint(20) unsigned NOT NULL default '0',\n\tmeta_key varchar(255) default NULL,\n\tmeta_value longtext,\n\tPRIMARY KEY  (meta_id),\n\tKEY post_id (post_id),\n\tKEY meta_key (meta_key({max}))\n) {charset_collate}", t("postmeta")),
        format!("CREATE TABLE IF NOT EXISTS {} (\n\tID bigint(20) unsigned NOT NULL auto_increment,\n\tpost_author bigint(20) unsigned NOT NULL default '0',\n\tpost_date datetime NOT NULL default '0000-00-00 00:00:00',\n\tpost_date_gmt datetime NOT NULL default '0000-00-00 00:00:00',\n\tpost_content longtext NOT NULL,\n\tpost_title text NOT NULL,\n\tpost_excerpt text NOT NULL,\n\tpost_status varchar(20) NOT NULL default 'publish',\n\tcomment_status varchar(20) NOT NULL default 'open',\n\tping_status varchar(20) NOT NULL default 'open',\n\tpost_password varchar(255) NOT NULL default '',\n\tpost_name varchar(200) NOT NULL default '',\n\tto_ping text NOT NULL,\n\tpinged text NOT NULL,\n\tpost_modified datetime NOT NULL default '0000-00-00 00:00:00',\n\tpost_modified_gmt datetime NOT NULL default '0000-00-00 00:00:00',\n\tpost_content_filtered longtext NOT NULL,\n\tpost_parent bigint(20) unsigned NOT NULL default '0',\n\tguid varchar(255) NOT NULL default '',\n\tmenu_order int(11) NOT NULL default '0',\n\tpost_type varchar(20) NOT NULL default 'post',\n\tpost_mime_type varchar(100) NOT NULL default '',\n\tcomment_count bigint(20) NOT NULL default '0',\n\tPRIMARY KEY  (ID),\n\tKEY post_name (post_name({max})),\n\tKEY type_status_date (post_type,post_status,post_date,ID),\n\tKEY post_parent (post_parent),\n\tKEY post_author (post_author),\n\tKEY type_status_author (post_type,post_status,post_author)\n) {charset_collate}", t("posts")),
    ]
}

/// `wpdb::get_charset_collate()` for `utf8mb4` and an empty `DB_COLLATE`: the 5.20 collation where the server
/// knows it (MySQL 5.6 and later, every MariaDB).
pub fn charset_collate(server_version: &str) -> String {
    let mut parts = server_version.split(['.', '-']).map(|p| p.parse::<u32>().unwrap_or(0));
    let (major, minor) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    let collate = if (major, minor) >= (5, 6) { "utf8mb4_unicode_520_ci" } else { "utf8mb4_unicode_ci" };
    format!("DEFAULT CHARACTER SET utf8mb4 COLLATE {collate}")
}

/// `$wp_db_version` of the WordPress files.
pub fn db_version(wp_root: &std::path::Path) -> u64 {
    let text = std::fs::read_to_string(wp_root.join("wp-includes").join("version.php")).unwrap_or_default();
    text.split("$wp_db_version = ").nth(1).and_then(|rest| rest.split(';').next()).and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

/// What `wp-admin/install.php` hands to `wp_install()`.
pub struct InstallData<'a> {
    pub blog_title: &'a str,
    /// Sanitized already (`sanitize_user( $user_name, true )` equals it).
    pub user_name: &'a str,
    pub user_email: &'a str,
    pub public: i64,
    pub user_password: &'a str,
    /// The language of the installer; stored when its translations are installed.
    pub language: &'a str,
    /// `wp_guess_url()`.
    pub url: &'a str,
}

/// What `wp_install()` returns.
pub struct Installed {
    pub user_id: u64,
    pub password: String,
    pub password_message: String,
}

fn ser(value: &Value) -> String {
    String::from_utf8_lossy(&rustpress_phpser::serialize(value)).into_owned()
}

fn s(text: &str) -> Value {
    Value::Str(text.as_bytes().to_vec())
}

fn k(text: &str) -> Key {
    Key::Str(text.as_bytes().to_vec())
}

/// `WP_Theme::get_core_default_theme()` after `WP_DEFAULT_THEME`: the newest default theme that is there.
fn default_theme(wp_root: &std::path::Path) -> String {
    const CORE: [&str; 16] = [
        "twentytwentyfive", "twentytwentyfour", "twentytwentythree", "twentytwentytwo", "twentytwentyone", "twentytwenty", "twentynineteen", "twentyseventeen", "twentysixteen", "twentyfifteen",
        "twentyfourteen", "twentythirteen", "twentytwelve", "twentyeleven", "twentyten", "classic",
    ];
    let themes = wp_root.join("wp-content").join("themes");
    CORE.iter().find(|theme| themes.join(theme).join("style.css").is_file()).copied().unwrap_or(CORE[0]).to_owned()
}

/// The cron array `wp_install()` leaves: the update checks one, one and a half and two hours from now.
fn cron(now: i64) -> String {
    // `md5( serialize( array() ) )`: the key of an event without arguments.
    const NO_ARGS: &str = "40cd750bba9870f18aada2478b24840a";
    let event = |hook: &str| -> Value {
        Value::Array(vec![(
            k(hook),
            Value::Array(vec![(k(NO_ARGS), Value::Array(vec![(k("schedule"), s("twicedaily")), (k("args"), Value::Array(Vec::new())), (k("interval"), Value::Int(43200))]))]),
        )])
    };
    ser(&Value::Array(vec![
        (Key::Int(now + 3600), event("wp_version_check")),
        (Key::Int(now + 5400), event("wp_update_plugins")),
        (Key::Int(now + 7200), event("wp_update_themes")),
        (k("version"), Value::Int(2)),
    ]))
}

/// `populate_options()`: the options of a new site in the order WordPress adds them, with their autoload value.
fn default_options(site: &Site, wp_root: &std::path::Path, url: &str, now: i64) -> Vec<(&'static str, String, &'static str)> {
    let theme = default_theme(wp_root);
    let offset_or_tz = site.trx("0", "default GMT offset or timezone string");
    let (gmt_offset, timezone_string) = if offset_or_tz.parse::<f64>().is_ok() {
        (offset_or_tz, String::new())
    } else if offset_or_tz.parse::<chrono_tz::Tz>().is_ok() {
        ("0".to_owned(), offset_or_tz)
    } else {
        ("0".to_owned(), String::new())
    };
    let db_version = db_version(wp_root).to_string();
    let empty = || "a:0:{}".to_owned();
    let fat = ["moderation_keys", "recently_edited", "disallowed_keys", "uninstall_plugins", "auto_plugin_theme_update_emails"];
    let options: Vec<(&'static str, String)> = vec![
        ("siteurl", url.to_owned()),
        ("home", url.to_owned()),
        ("blogname", site.tr("My Site")),
        ("blogdescription", String::new()),
        ("users_can_register", "0".into()),
        ("admin_email", "you@example.com".into()),
        ("start_of_week", site.trx("1", "start of week")),
        ("use_balanceTags", "0".into()),
        ("use_smilies", "1".into()),
        ("require_name_email", "1".into()),
        ("comments_notify", "1".into()),
        ("posts_per_rss", "10".into()),
        ("rss_use_excerpt", "0".into()),
        ("mailserver_url", "mail.example.com".into()),
        ("mailserver_login", "login@example.com".into()),
        ("mailserver_pass", String::new()),
        ("mailserver_port", "110".into()),
        ("default_category", "1".into()),
        ("default_comment_status", "open".into()),
        ("default_ping_status", "open".into()),
        ("default_pingback_flag", "1".into()),
        ("posts_per_page", "10".into()),
        ("date_format", site.tr("F j, Y")),
        ("time_format", site.tr("g:i a")),
        ("links_updated_date_format", site.tr("F j, Y g:i a")),
        ("comment_moderation", "0".into()),
        ("moderation_notify", "1".into()),
        ("permalink_structure", String::new()),
        ("rewrite_rules", String::new()),
        ("hack_file", "0".into()),
        ("blog_charset", "UTF-8".into()),
        ("moderation_keys", String::new()),
        ("active_plugins", empty()),
        ("category_base", String::new()),
        ("ping_sites", "https://rpc.pingomatic.com/".into()),
        ("comment_max_links", "2".into()),
        ("gmt_offset", gmt_offset),
        ("default_email_category", "1".into()),
        ("recently_edited", String::new()),
        ("template", theme.clone()),
        ("stylesheet", theme),
        ("comment_registration", "0".into()),
        ("html_type", "text/html".into()),
        ("use_trackback", "0".into()),
        ("default_role", "subscriber".into()),
        ("db_version", db_version.clone()),
        ("uploads_use_yearmonth_folders", "1".into()),
        ("upload_path", String::new()),
        ("blog_public", "1".into()),
        ("default_link_category", "2".into()),
        ("show_on_front", "posts".into()),
        ("tag_base", String::new()),
        ("show_avatars", "1".into()),
        ("avatar_rating", "G".into()),
        ("upload_url_path", String::new()),
        ("thumbnail_size_w", "150".into()),
        ("thumbnail_size_h", "150".into()),
        ("thumbnail_crop", "1".into()),
        ("medium_size_w", "300".into()),
        ("medium_size_h", "300".into()),
        ("avatar_default", "mystery".into()),
        ("large_size_w", "1024".into()),
        ("large_size_h", "1024".into()),
        ("image_default_link_type", "none".into()),
        ("image_default_size", String::new()),
        ("image_default_align", String::new()),
        ("close_comments_for_old_posts", "0".into()),
        ("close_comments_days_old", "14".into()),
        ("thread_comments", "1".into()),
        ("thread_comments_depth", "5".into()),
        ("page_comments", "0".into()),
        ("comments_per_page", "50".into()),
        ("default_comments_page", "newest".into()),
        ("comment_order", "asc".into()),
        ("sticky_posts", empty()),
        ("widget_categories", empty()),
        ("widget_text", empty()),
        ("widget_rss", empty()),
        ("uninstall_plugins", empty()),
        ("timezone_string", timezone_string),
        ("page_for_posts", "0".into()),
        ("page_on_front", "0".into()),
        ("default_post_format", "0".into()),
        ("link_manager_enabled", "0".into()),
        ("finished_splitting_shared_terms", "1".into()),
        ("site_icon", "0".into()),
        ("medium_large_size_w", "768".into()),
        ("medium_large_size_h", "0".into()),
        ("wp_page_for_privacy_policy", "0".into()),
        ("show_comments_cookies_opt_in", "1".into()),
        ("admin_email_lifespan", (now + 6 * 30 * 86400).to_string()),
        ("disallowed_keys", String::new()),
        ("comment_previously_approved", "1".into()),
        ("auto_plugin_theme_update_emails", empty()),
        ("auto_update_core_dev", "enabled".into()),
        ("auto_update_core_minor", "enabled".into()),
        ("auto_update_core_major", "enabled".into()),
        ("wp_force_deactivated_plugins", empty()),
        ("wp_attachment_pages_enabled", "0".into()),
        ("wp_notes_notify", "1".into()),
        ("initial_db_version", db_version),
    ];
    options.into_iter().map(|(name, value)| (name, value, if fat.contains(&name) { "off" } else { "on" })).collect()
}

/// `current_time( 'mysql' )` and `current_time( 'mysql', true )` with the zone the new site starts with.
fn local_and_gmt(site: &Site, now: i64) -> (String, String) {
    let utc = chrono::DateTime::from_timestamp(now, 0).unwrap_or_default();
    let format = "%Y-%m-%d %H:%M:%S";
    let gmt = utc.format(format).to_string();
    let zone = site.trx("0", "default GMT offset or timezone string");
    let local = if let Ok(hours) = zone.parse::<f64>() {
        (utc + chrono::Duration::seconds((hours * 3600.0) as i64)).format(format).to_string()
    } else if let Ok(tz) = zone.parse::<chrono_tz::Tz>() {
        utc.with_timezone(&tz).format(format).to_string()
    } else {
        gmt.clone()
    };
    (local, gmt)
}

/// `wp_install()`. `site` speaks the language of the installer; the tables must not hold a site yet.
pub async fn wp_install(writer: &Writer, site: &Site, wp_root: &std::path::Path, server_version: &str, data: &InstallData<'_>, now: i64) -> rustpress_db::Result<Installed> {
    let collate = charset_collate(server_version);
    for statement in schema(writer, &collate) {
        writer.execute(statement).await?;
    }
    writer.update_option_with_autoload("cron", &cron(now), "on").await?;
    let options = default_options(site, wp_root, data.url, now);
    for (name, value, autoload) in &options {
        writer.update_option_with_autoload(name, value, autoload).await?;
    }
    // `populate_roles()`
    writer.update_option_with_autoload(&format!("{}user_roles", writer.table_prefix()), include_str!("user_roles.txt"), "on").await?;

    writer.update_option("blogname", data.blog_title).await?;
    writer.update_option("admin_email", data.user_email).await?;
    writer.update_option("blog_public", &data.public.to_string()).await?;
    writer.update_option_with_autoload("fresh_site", "1", "off").await?;
    // `sanitize_option( 'WPLANG' )` keeps only a language whose translations are installed.
    if !data.language.is_empty() && data.language != "en_US" && wp_root.join("wp-content").join("languages").join(format!("{}.mo", data.language)).is_file() {
        writer.update_option("WPLANG", data.language).await?;
    }
    writer.update_option("siteurl", data.url).await?;
    if data.public == 0 {
        writer.update_option("default_pingback_flag", "0").await?;
    }

    // The first user.
    let (local, gmt) = local_and_gmt(site, now);
    let mut password = data.user_password.trim().to_owned();
    let generated = password.is_empty();
    let message = if generated {
        password = crate::sanitize::random_password(12, false);
        site.tr("<strong><em>Note that password</em></strong> carefully! It is a <em>random</em> password that was generated just for you.")
    } else {
        format!("<em>{}</em>", site.tr("Your chosen password."))
    };
    let login = data.user_name;
    let nicename: String = sanitize_title_save(login, &site.locale()).chars().take(50).collect();
    let user_id = writer
        .insert_row(
            "users",
            &[
                ("user_login", Val::Str(login.to_owned())),
                ("user_pass", Val::Str(rustpress_auth::password::hash_password(&password))),
                ("user_nicename", Val::Str(nicename)),
                ("user_email", Val::Str(data.user_email.to_owned())),
                ("user_url", Val::Str(data.url.to_owned())),
                ("user_registered", Val::Str(gmt.clone())),
                ("user_activation_key", Val::Str(String::new())),
                ("user_status", Val::Int(0)),
                ("display_name", Val::Str(login.to_owned())),
            ],
        )
        .await?;
    let prefix = writer.table_prefix().to_owned();
    let mut meta: Vec<(String, String)> = [
        ("nickname", login),
        ("first_name", ""),
        ("last_name", ""),
        ("description", ""),
        ("rich_editing", "true"),
        ("syntax_highlighting", "true"),
        ("infinite_scrolling", "true"),
        ("comment_shortcuts", "false"),
        ("admin_color", "modern"),
        ("use_ssl", "0"),
        ("show_admin_bar_front", "true"),
        ("locale", ""),
    ]
    .iter()
    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
    .collect();
    meta.push((format!("{prefix}capabilities"), "a:1:{s:13:\"administrator\";b:1;}".to_owned()));
    meta.push((format!("{prefix}user_level"), "10".to_owned()));
    meta.push(("dismissed_wp_pointers".to_owned(), String::new()));
    if generated {
        meta.push(("default_password_nag".to_owned(), "1".to_owned()));
    }
    for (key, value) in &meta {
        writer.update_user_meta(user_id, key, value).await?;
    }
    writer.update_option_with_autoload("user_count", "1", "off").await?;

    install_defaults(writer, site, data.url, user_id, &local, &gmt).await?;
    Ok(Installed { user_id, password, password_message: message })
}

/// `wp_install_defaults()`: the category, the first post with its comment, the sample page, the draft of the
/// privacy policy and the widgets of the classic default themes.
async fn install_defaults(writer: &Writer, site: &Site, url: &str, user_id: u64, now: &str, now_gmt: &str) -> rustpress_db::Result<()> {
    let tr = |text: &str| site.tr(text);
    let locale = site.locale();
    writer
        .insert_row(
            "terms",
            &[
                ("term_id", Val::Int(1)),
                ("name", Val::Str(tr("Uncategorized"))),
                ("slug", Val::Str(sanitize_title_save(&site.trx("Uncategorized", "Default category slug"), &locale))),
                ("term_group", Val::Int(0)),
            ],
        )
        .await?;
    let tt_id = writer
        .insert_row("term_taxonomy", &[("term_id", Val::Int(1)), ("taxonomy", Val::Str("category".into())), ("description", Val::Str(String::new())), ("parent", Val::Int(0)), ("count", Val::Int(1))])
        .await?;

    let post = |content: String, title: String, name: String, guid: String| -> Vec<(&'static str, Val)> {
        vec![
            ("post_author", Val::Int(user_id as i64)),
            ("post_date", Val::Str(now.to_owned())),
            ("post_date_gmt", Val::Str(now_gmt.to_owned())),
            ("post_content", Val::Str(content)),
            ("post_excerpt", Val::Str(String::new())),
            ("post_title", Val::Str(title)),
            ("post_name", Val::Str(name)),
            ("post_modified", Val::Str(now.to_owned())),
            ("post_modified_gmt", Val::Str(now_gmt.to_owned())),
            ("guid", Val::Str(guid)),
        ]
    };
    let paragraph = |text: String| format!("<!-- wp:paragraph -->\n<p>{text}</p>\n<!-- /wp:paragraph -->");
    let quote = |text: String| format!("<!-- wp:quote -->\n<blockquote class=\"wp-block-quote\">\n<!-- wp:paragraph -->\n<p>{text}</p>\n<!-- /wp:paragraph -->\n</blockquote>\n<!-- /wp:quote -->");
    let tail = [("to_ping", Val::Str(String::new())), ("pinged", Val::Str(String::new())), ("post_content_filtered", Val::Str(String::new()))];

    // The first post.
    let mut fields = post(
        paragraph(tr("Welcome to WordPress. This is your first post. Edit or delete it, then start writing!")),
        tr("Hello world!"),
        sanitize_title_save(&site.trx("hello-world", "Default post slug"), &locale),
        format!("{url}/?p=1"),
    );
    fields.push(("comment_count", Val::Int(1)));
    fields.extend(tail.iter().cloned());
    writer.insert_row("posts", &fields).await?;
    writer.insert_row("term_relationships", &[("term_taxonomy_id", Val::Int(tt_id as i64)), ("object_id", Val::Int(1))]).await?;

    // Its comment.
    let comment = rustpress_core::i18n::sprintf(
        &tr("Hi, this is a comment.\nTo get started with moderating, editing, and deleting comments, please visit the Comments screen in the dashboard.\nCommenter avatars come from <a href=\"%s\">Gravatar</a>."),
        &[&rustpress_core::formatting::esc_url(&tr("https://gravatar.com/"))],
    );
    writer
        .insert_row(
            "comments",
            &[
                ("comment_post_ID", Val::Int(1)),
                ("comment_author", Val::Str(tr("A WordPress Commenter"))),
                ("comment_author_email", Val::Str("wapuu@wordpress.example".into())),
                ("comment_author_url", Val::Str(rustpress_core::formatting::esc_url(&tr("https://wordpress.org/")))),
                ("comment_date", Val::Str(now.to_owned())),
                ("comment_date_gmt", Val::Str(now_gmt.to_owned())),
                ("comment_content", Val::Str(comment)),
                ("comment_type", Val::Str("comment".into())),
            ],
        )
        .await?;

    // The sample page.
    let page = [
        paragraph(tr("This is an example page. It's different from a blog post because it will stay in one place and will show up in your site navigation (in most themes). Most people start with an About page that introduces them to potential site visitors. It might say something like this:")),
        quote(tr("Hi there! I'm a bike messenger by day, aspiring actor by night, and this is my website. I live in Los Angeles, have a great dog named Jack, and I like pi&#241;a coladas. (And gettin' caught in the rain.)")),
        paragraph(tr("...or something like this:")),
        quote(tr("The XYZ Doohickey Company was founded in 1971, and has been providing quality doohickeys to the public ever since. Located in Gotham City, XYZ employs over 2,000 people and does all kinds of awesome things for the Gotham community.")),
        paragraph(rustpress_core::i18n::sprintf(&tr("As a new WordPress user, you should go to <a href=\"%s\">your dashboard</a> to delete this page and create new pages for your content. Have fun!"), &[&format!("{url}/wp-admin/")])),
    ]
    .join("\n\n");
    let mut fields = post(page, tr("Sample Page"), tr("sample-page"), format!("{url}/?page_id=2"));
    fields.push(("comment_status", Val::Str("closed".into())));
    fields.push(("post_type", Val::Str("page".into())));
    fields.extend(tail.iter().cloned());
    writer.insert_row("posts", &fields).await?;
    writer.insert_row("postmeta", &[("post_id", Val::Int(2)), ("meta_key", Val::Str("_wp_page_template".into())), ("meta_value", Val::Str("default".into()))]).await?;

    // The privacy policy, as a draft.
    let mut fields = post(crate::privacy_text::default_content(site, url), tr("Privacy Policy"), tr("privacy-policy"), format!("{url}/?page_id=3"));
    fields.push(("comment_status", Val::Str("closed".into())));
    fields.push(("post_type", Val::Str("page".into())));
    fields.push(("post_status", Val::Str("draft".into())));
    fields.extend(tail.iter().cloned());
    writer.insert_row("posts", &fields).await?;
    writer.insert_row("postmeta", &[("post_id", Val::Int(3)), ("meta_key", Val::Str("_wp_page_template".into())), ("meta_value", Val::Str("default".into()))]).await?;
    writer.update_option("wp_page_for_privacy_policy", "3").await?;

    // The widgets of the classic themes.
    let block = |content: String| Value::Array(vec![(k("content"), s(&content))]);
    let group = |title: &str, inner: &str| format!("<!-- wp:group --><div class=\"wp-block-group\"><!-- wp:heading --><h2>{}</h2><!-- /wp:heading -->{inner}</div><!-- /wp:group -->", tr(title));
    let widgets = Value::Array(vec![
        (Key::Int(2), block("<!-- wp:search /-->".into())),
        (Key::Int(3), block(group("Recent Posts", "<!-- wp:latest-posts /-->"))),
        (Key::Int(4), block(group("Recent Comments", "<!-- wp:latest-comments {\"displayAvatar\":false,\"displayDate\":false,\"displayExcerpt\":false} /-->"))),
        (Key::Int(5), block(group("Archives", "<!-- wp:archives /-->"))),
        (Key::Int(6), block(group("Categories", "<!-- wp:categories /-->"))),
        (k("_multiwidget"), Value::Int(1)),
    ]);
    writer.update_option("widget_block", &ser(&widgets)).await?;
    let list = |ids: &[&str]| Value::Array(ids.iter().enumerate().map(|(i, id)| (Key::Int(i as i64), s(id))).collect());
    let sidebars = Value::Array(vec![
        (k("wp_inactive_widgets"), Value::Array(Vec::new())),
        (k("sidebar-1"), list(&["block-2", "block-3", "block-4"])),
        (k("sidebar-2"), list(&["block-5", "block-6"])),
        (k("array_version"), Value::Int(3)),
    ]);
    writer.update_option("sidebars_widgets", &ser(&sidebars)).await?;
    writer.update_user_meta(user_id, "show_welcome_panel", "1").await?;

    // The options the widgets of WordPress add when they are first registered.
    for name in [
        "pages", "calendar", "archives", "media_audio", "media_image", "media_gallery", "media_video", "meta", "search", "recent-posts", "recent-comments", "tag_cloud", "nav_menu", "custom_html",
    ] {
        writer.update_option(&format!("widget_{name}"), "a:1:{s:12:\"_multiwidget\";i:1;}").await?;
    }
    Ok(())
}
