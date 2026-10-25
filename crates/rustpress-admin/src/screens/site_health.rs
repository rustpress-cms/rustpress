//! `wp-admin/site-health.php`: the status tab with the tests that make sense without PHP.
//!
//! The markup is baked; the results of the direct tests go to the script as `SiteHealth`
//! (`WP_Site_Health::enqueue_scripts()`). Tests of the PHP runtime (version, modules, sessions,
//! opcode cache, `php.ini`) do not exist here; the checks that call other servers run in the browser
//! only where RustPress has the REST routes for them.

use rustpress_core::formatting::esc_url;
use rustpress_core::i18n::sprintf;
use serde_json::{Value as Json, json};

use super::{AdminResponse, Ctx, die, render};
use crate::chrome::{Assets, Screen};
use crate::themes::ThemeInfo;

/// The default theme of a fresh installation (`WP_DEFAULT_THEME`).
const DEFAULT_THEME: &str = "twentytwentyfive";

struct Test {
    test: &'static str,
    status: &'static str,
    badge: (String, &'static str),
    label: String,
    description: String,
    actions: String,
}

impl Test {
    fn json(self) -> Json {
        json!({"label": self.label, "status": self.status, "badge": {"label": self.badge.0, "color": self.badge.1}, "description": self.description, "actions": self.actions, "test": self.test})
    }
}

fn active_plugins(ctx: &Ctx<'_>) -> Vec<String> {
    match ctx.site().options.value("active_plugins") {
        Some(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

fn wordpress_version(ctx: &Ctx<'_>, upgrade: Option<String>) -> Test {
    let tr = |s: &str| ctx.tr(s);
    let current = ctx.env.loader.version.clone();
    let mut test = Test { test: "wordpress_version", status: "good", badge: (tr("Performance"), "blue"), label: String::new(), description: String::new(), actions: String::new() };
    match upgrade {
        None => {
            test.label = sprintf(&tr("Your version of WordPress (%s) is up to date"), &[&current]);
            test.description = format!("<p>{}</p>", tr("You are currently running the latest version of WordPress available, keep it up!"));
        }
        Some(new) => {
            let major = |v: &str| v.split('.').take(2).collect::<Vec<_>>().join(".");
            test.label = sprintf(&tr("WordPress update available (%s)"), &[&new]);
            test.actions = format!("<a href=\"{}\">{}</a>", esc_url(&ctx.admin_url("update-core.php")), tr("Install the latest version of WordPress"));
            if major(&current) != major(&new) {
                test.status = "recommended";
                test.description = format!("<p>{}</p>", tr("A new version of WordPress is available."));
            } else {
                test.status = "critical";
                test.badge.0 = tr("Security");
                test.description = format!("<p>{}</p>", tr("A new minor update is available for your site. Because minor updates often address security, it&#8217;s important to install them."));
            }
        }
    }
    test
}

fn plugin_version(ctx: &Ctx<'_>, outdated: u64) -> Test {
    let (tr, site) = (|s: &str| ctx.tr(s), ctx.site());
    let plugins = crate::plugins::installed(&super::update_core::wp_root(ctx));
    let active = active_plugins(ctx);
    let total = plugins.len() as i64;
    let active_count = plugins.iter().filter(|p| active.contains(&p.file)).count() as i64;
    let mut test = Test {
        test: "plugin_version",
        status: "good",
        badge: (tr("Security"), "blue"),
        label: tr("Your plugins are all up to date"),
        description: format!("<p>{}</p>", tr("Plugins extend your site&#8217;s functionality with things like contact forms, ecommerce and much more. That means they have deep access to your site, so it&#8217;s vital to keep them up to date.")),
        actions: format!("<p><a href=\"{}\">{}</a></p>", esc_url(&ctx.admin_url("plugins.php")), tr("Manage your plugins")),
    };
    if outdated > 0 {
        test.status = "critical";
        test.label = tr("You have plugins waiting to be updated");
        test.description.push_str(&format!("<p>{}</p>", sprintf(&site.trn("Your site has %d plugin waiting to be updated.", "Your site has %d plugins waiting to be updated.", outdated as i64), &[&outdated.to_string()])));
        test.actions.push_str(&format!("<p><a href=\"{}\">{}</a></p>", esc_url(&ctx.admin_url("plugins.php?plugin_status=upgrade")), tr("Update your plugins")));
    } else if active_count == 1 {
        test.description.push_str(&format!("<p>{}</p>", tr("Your site has 1 active plugin, and it is up to date.")));
    } else if active_count > 0 {
        test.description.push_str(&format!("<p>{}</p>", sprintf(&site.trn("Your site has %d active plugin, and it is up to date.", "Your site has %d active plugins, and they are all up to date.", active_count), &[&active_count.to_string()])));
    } else {
        test.description.push_str(&format!("<p>{}</p>", tr("Your site does not have any active plugins.")));
    }
    if total > active_count {
        let unused = total - active_count;
        test.status = "recommended";
        test.label = tr("You should remove inactive plugins");
        test.description.push_str(&format!(
            "<p>{} {}</p>",
            sprintf(&site.trn("Your site has %d inactive plugin.", "Your site has %d inactive plugins.", unused), &[&unused.to_string()]),
            tr("Inactive plugins are tempting targets for attackers. If you are not going to use a plugin, you should consider removing it.")
        ));
        test.actions.push_str(&format!("<p><a href=\"{}\">{}</a></p>", esc_url(&ctx.admin_url("plugins.php?plugin_status=inactive")), tr("Manage inactive plugins")));
    }
    test
}

/// `WP_Theme::get_core_default_theme()`: the newest `twenty*` theme.
fn default_theme<'a>(themes: &'a [ThemeInfo]) -> Option<&'a ThemeInfo> {
    themes.iter().find(|t| t.stylesheet == DEFAULT_THEME).or_else(|| {
        let year = |t: &&ThemeInfo| t.stylesheet.strip_prefix("twenty").map(str::to_owned).filter(|rest| rest.len() >= 2 && rest.chars().all(char::is_alphabetic)).map(|_| t.stylesheet.clone());
        themes.iter().filter(|t| year(t).is_some()).max_by_key(|t| (t.version.clone(), t.stylesheet.clone()))
    })
}

fn theme_version(ctx: &Ctx<'_>, outdated: u64) -> Test {
    let (tr, site) = (|s: &str| ctx.tr(s), ctx.site());
    let themes = crate::themes::installed(&super::update_core::wp_root(ctx));
    let active = themes.iter().find(|t| t.stylesheet == site.settings.stylesheet);
    let parent = active.filter(|t| t.template != t.stylesheet).and_then(|t| themes.iter().find(|p| p.stylesheet == t.template));
    let default = default_theme(&themes);
    let active_name = active.map_or_else(|| site.settings.stylesheet.clone(), |t| t.name.clone());
    let using_default = default.is_some_and(|d| d.stylesheet == site.settings.stylesheet || (parent.is_some() && active.is_some_and(|a| a.template == d.template)));
    let total = themes.len() as i64;
    let mut allowed = 1;
    if active.is_some_and(|t| t.template != t.stylesheet) {
        allowed += 1;
    }
    if default.is_some() && !using_default {
        allowed += 1;
    }
    let mut test = Test {
        test: "theme_version",
        status: "good",
        badge: (tr("Security"), "blue"),
        label: tr("Your themes are all up to date"),
        description: format!("<p>{}</p>", tr("Themes add your site&#8217;s look and feel. It&#8217;s important to keep them up to date, to stay consistent with your brand and keep your site secure.")),
        actions: format!("<p><a href=\"{}\">{}</a></p>", esc_url(&ctx.admin_url("themes.php")), tr("Manage your themes")),
    };
    if outdated > 0 {
        test.status = "critical";
        test.label = tr("You have themes waiting to be updated");
        test.description.push_str(&format!("<p>{}</p>", sprintf(&site.trn("Your site has %d theme waiting to be updated.", "Your site has %d themes waiting to be updated.", outdated as i64), &[&outdated.to_string()])));
    } else if total == 1 {
        test.description.push_str(&format!("<p>{}</p>", tr("Your site has 1 installed theme, and it is up to date.")));
    } else if total > 0 {
        test.description.push_str(&format!("<p>{}</p>", sprintf(&site.trn("Your site has %d installed theme, and it is up to date.", "Your site has %d installed themes, and they are all up to date.", total), &[&total.to_string()])));
    } else {
        test.description.push_str(&format!("<p>{}</p>", tr("Your site does not have any installed themes.")));
    }
    if total > allowed {
        let inactive = total - allowed;
        let count = inactive.to_string();
        let default_name = default.map_or_else(|| DEFAULT_THEME.to_owned(), |d| d.name.clone());
        test.status = "recommended";
        test.label = tr("You should remove inactive themes");
        let line = if let Some(parent) = parent {
            let first = sprintf(&site.trn("Your site has %d inactive theme.", "Your site has %d inactive themes.", inactive), &[&count]);
            let second = if using_default {
                sprintf(&tr("To enhance your site&#8217;s security, you should consider removing any themes you are not using. You should keep your active theme, %1$s, and %2$s, its parent theme."), &[&active_name, &parent.name])
            } else {
                sprintf(&tr("To enhance your site&#8217;s security, you should consider removing any themes you are not using. You should keep %1$s, the default WordPress theme, %2$s, your active theme, and %3$s, its parent theme."), &[&default_name, &active_name, &parent.name])
            };
            format!("{first} {second}")
        } else if using_default {
            let first = sprintf(&site.trn("Your site has %1$d inactive theme, other than %2$s, your active theme.", "Your site has %1$d inactive themes, other than %2$s, your active theme.", inactive), &[&count, &active_name]);
            format!("{first} {}", tr("You should consider removing any unused themes to enhance your site&#8217;s security."))
        } else {
            let first = sprintf(&site.trn("Your site has %1$d inactive theme, other than %2$s, the default WordPress theme, and %3$s, your active theme.", "Your site has %1$d inactive themes, other than %2$s, the default WordPress theme, and %3$s, your active theme.", inactive), &[&count, &default_name, &active_name]);
            format!("{first} {}", tr("You should consider removing any unused themes to enhance your site&#8217;s security."))
        };
        test.description.push_str(&format!("<p>{line}</p>"));
    }
    if default.is_none() {
        test.status = "recommended";
        test.label = tr("Have a default theme available");
        test.description.push_str(&format!("<p>{}</p>", tr("Your site does not have any default theme. Default themes are used by WordPress automatically if anything is wrong with your chosen theme.")));
    }
    test
}

fn external_link(ctx: &Ctx<'_>, url: &str, text: &str) -> String {
    format!("<p><a href=\"{}\" target=\"_blank\">{}<span class=\"screen-reader-text\"> {}</span><span aria-hidden=\"true\" class=\"dashicons dashicons-external\"></span></a></p>", esc_url(url), ctx.tr(text), ctx.tr("(opens in a new tab)"))
}

fn simple(ctx: &Ctx<'_>, test: &'static str, badge: &str, label: &str, description: &str, actions: String) -> Test {
    Test { test, status: "good", badge: (ctx.tr(badge), "blue"), label: ctx.tr(label), description: format!("<p>{}</p>", ctx.tr(description)), actions }
}

fn insecure_registration(ctx: &Ctx<'_>) -> Test {
    let site = ctx.site();
    let mut test = simple(ctx, "insecure_registration", "Security", "Open Registration with privileged default role", "The combination of open registration setting and the default user role may lead to security issues.", String::new());
    let open = site.options.string("users_can_register", "0") == "1";
    if open && matches!(site.options.string("default_role", "subscriber").as_str(), "editor" | "administrator") {
        test.status = "critical";
        test.description = ctx.tr("Registration is open to anyone, and the default role is set to a privileged role.");
        test.actions = format!("<p><a href=\"{}\">{}</a></p>", esc_url(&ctx.admin_url("options-general.php")), ctx.tr("Change these settings"));
    }
    test
}

fn search_engine_visibility(ctx: &Ctx<'_>) -> Test {
    let mut test = simple(
        ctx,
        "search_engine_visibility",
        "Privacy",
        "Search engine indexing is enabled.",
        "Search engines can crawl and index your site. No action needed.",
        format!("<p><a href=\"{}\">{}</a></p>", esc_url(&ctx.admin_url("options-reading.php#blog_public")), ctx.tr("Review your visibility settings")),
    );
    if ctx.site().options.string("blog_public", "1") == "0" {
        test.status = "recommended";
        test.label = ctx.tr("Search engines are discouraged from indexing this site.");
        test.description = format!("<p>{}</p>", ctx.tr("Your site is hidden from search engines. Consider enabling indexing if this is a public site."));
    }
    test
}

async fn direct_tests(ctx: &Ctx<'_>) -> Vec<Json> {
    let counts = crate::updates::counts(ctx.site()).await;
    let upgrade = super::update_core::core_upgrade_version(ctx).await;
    let tests = vec![
        wordpress_version(ctx, upgrade),
        plugin_version(ctx, counts.plugins),
        theme_version(ctx, counts.themes),
        simple(
            ctx,
            "sql_server",
            "Performance",
            "SQL server is up to date",
            "The SQL server is a required piece of software for the database WordPress uses to store all your site&#8217;s content and settings.",
            external_link(ctx, "https://wordpress.org/about/requirements/", "Learn more about what WordPress requires to run."),
        ),
        simple(ctx, "rest_availability", "Performance", "The REST API is available", "The REST API is one way that WordPress and other applications communicate with the server. For example, the block editor screen relies on the REST API to display and save your posts and pages.", String::new()),
        simple(
            ctx,
            "is_in_debug_mode",
            "Security",
            "Your site is not set to output debug information",
            "Debug mode is often enabled to gather more details about an error or site failure, but may contain sensitive information which should not be available on a publicly available website.",
            external_link(ctx, "https://developer.wordpress.org/advanced-administration/debug/debug-wordpress/", "Learn more about debugging in WordPress."),
        ),
        insecure_registration(ctx),
        search_engine_visibility(ctx),
        simple(
            ctx,
            "persistent_object_cache",
            "Performance",
            "A persistent object cache is not required",
            "A persistent object cache makes your site&#8217;s database more efficient, resulting in faster load times because WordPress can retrieve your site&#8217;s content and settings much more quickly.",
            external_link(ctx, "https://developer.wordpress.org/advanced-administration/performance/optimization/#object-caching", "Learn more about persistent object caching."),
        ),
    ];
    tests.into_iter().map(Test::json).collect()
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("view_site_health_checks") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to access site health information.")));
    }
    let direct = direct_tests(ctx).await;
    let mut issues = json!({"good": 0, "recommended": 0, "critical": 0});
    for test in &direct {
        if let Some(count) = test["status"].as_str().and_then(|s| issues.get_mut(s)) {
            *count = json!(count.as_u64().unwrap_or(0) + 1);
        }
    }
    let data = json!({
        "screen": "site-health",
        "nonce": {"site_status": ctx.create_nonce("health-check-site-status"), "site_status_result": ctx.create_nonce("health-check-site-status-result")},
        "site_status": {"direct": direct, "async": [], "issues": issues},
    });
    let baked = super::pages::load(ctx, "site-health");
    let debug_tab = ctx.request.get("tab") == Some("debug");
    let content = if debug_tab {
        // The header is the one of the status tab with the other tab marked; the body is the information.
        let end = baked.content.find("<div class=\"health-check-body").unwrap_or(baked.content.len());
        let header = baked.content[..end].replace("class=\"health-check-tab active\">Status</a>", "class=\"health-check-tab \">Status</a>").replace("class=\"health-check-tab \">Info</a>", "class=\"health-check-tab active\">Info</a>");
        format!("{header}{}", super::site_health_info::body(ctx).await)
    } else {
        format!("{}\n", baked.content.trim_end_matches('\n'))
    };
    let assets = Assets { scripts: vec!["site-health".into()], styles: vec!["site-health".into()], localize: vec![("site-health".into(), "SiteHealth".into(), data)], ..Default::default() };
    let screen = Screen { file: "site-health.php".into(), parent_file: "tools.php".into(), submenu_file: Some("site-health.php".into()), title: sprintf(&tr("Site Health - %s"), &[&if ctx.request.get("tab") == Some("debug") { tr("Info") } else { tr("Status") }]), help: Some(baked.help), early_classes: vec!["site-health".into()], ..Default::default() };
    render(ctx, &screen, &content, &assets).await
}
