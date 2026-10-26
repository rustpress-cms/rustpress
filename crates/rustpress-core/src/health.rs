//! The Site Health tests that run behind the screen (`wp-site-health/v1`) and the sizes of the directories.
//! RustPress has no background updates, makes no requests to itself or to WordPress.org, and has no page cache
//! plugin, so those tests answer what is true for it.

use std::path::Path;

use serde_json::{Map, Value, json};

use crate::formatting::esc_url;
use crate::i18n::sprintf;
use crate::site::Site;

/// `size_format( $bytes, $decimals )`.
pub fn size_format(site: &Site, bytes: u64, decimals: usize) -> String {
    let locale = site.locale_data();
    let number = |value: f64| {
        let factor = 10f64.powi(decimals as i32);
        let rounded = (value * factor).round() / factor;
        let text = format!("{rounded:.decimals$}");
        let (whole, fraction) = text.split_once('.').map_or((text.as_str(), ""), |(w, f)| (w, f));
        let mut out = String::new();
        for (i, c) in whole.chars().enumerate() {
            if i > 0 && (whole.len() - i) % 3 == 0 {
                out.push_str(&locale.thousands_sep);
            }
            out.push(c);
        }
        if decimals > 0 {
            out.push_str(&locale.decimal_point);
            out.push_str(fraction);
        }
        out
    };
    if bytes == 0 {
        return format!("{} B", number(0.0));
    }
    const UNITS: [(&str, u64); 5] = [("TB", 1 << 40), ("GB", 1 << 30), ("MB", 1 << 20), ("KB", 1 << 10), ("B", 1)];
    for (unit, magnitude) in UNITS {
        if bytes >= magnitude {
            return format!("{} {unit}", number(bytes as f64 / magnitude as f64));
        }
    }
    format!("{} B", number(0.0))
}

/// `recurse_dirsize()`: the size of a directory with everything below it.
pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    std::fs::read_dir(path).map(|entries| entries.flatten().map(|e| dir_size(&e.path())).sum()).unwrap_or(0)
}

/// `WP_Debug_Data::get_sizes()` as `WP_REST_Site_Health_Controller::get_directory_sizes()` answers it.
pub async fn directory_sizes(site: &Site, root: &Path) -> Value {
    let content = root.join("wp-content");
    let configured = site.options.string("upload_path", "");
    let uploads = if configured.is_empty() { content.join("uploads") } else if Path::new(&configured).is_absolute() { configured.into() } else { root.join(configured) };
    let fonts = uploads.join("fonts");
    let themes = content.join("themes");
    let plugins = content.join("plugins");
    let exclude = [&themes, &plugins, &uploads, &fonts];
    let inside: u64 = [&themes, &plugins, &uploads].iter().map(|p| dir_size(p)).sum();
    let _ = exclude;
    let paths: Vec<(&str, std::path::PathBuf, Option<u64>)> = vec![
        ("wordpress_size", root.to_path_buf(), Some(dir_size(root).saturating_sub(inside))),
        ("themes_size", themes.clone(), None),
        ("plugins_size", plugins.clone(), None),
        ("uploads_size", uploads.clone(), None),
        ("fonts_size", fonts.clone(), None),
    ];
    let mut out = Map::new();
    out.insert("raw".into(), json!(0));
    let mut total = 0u64;
    for (name, path, known) in paths {
        if !path.is_dir() {
            out.insert(name.into(), json!({"size": site.tr("The directory does not exist.")}));
            continue;
        }
        let bytes = known.unwrap_or_else(|| dir_size(&path));
        total += bytes;
        let size = size_format(site, bytes, 2);
        let mut entry = Map::new();
        entry.insert("size".into(), json!(size));
        entry.insert("debug".into(), json!(format!("{size} ({bytes} bytes)")));
        if bytes != 0 {
            entry.insert("raw".into(), json!(bytes));
        }
        out.insert(name.into(), Value::Object(entry));
    }
    let database = site.db.database_size().await.unwrap_or(0);
    total += database;
    for (name, bytes) in [("database_size", database), ("total_size", total)] {
        let size = size_format(site, bytes, 2);
        let mut entry = Map::new();
        entry.insert("size".into(), json!(size));
        entry.insert("debug".into(), json!(format!("{size} ({bytes} bytes)")));
        if bytes != 0 {
            entry.insert("raw".into(), json!(bytes));
        }
        out.insert(name.into(), Value::Object(entry));
    }
    // `raw` of the whole answer stays 0 in WordPress; it comes first.
    let mut ordered = Map::new();
    ordered.insert("raw".into(), out.remove("raw").unwrap_or(json!(0)));
    ordered.extend(out);
    Value::Object(ordered)
}

fn badge(site: &Site, label: &str) -> Value {
    json!({"label": site.tr(label), "color": "blue"})
}

fn external(site: &Site, url: &str, label: &str) -> String {
    format!(
        "<p><a href=\"{}\" target=\"_blank\">{}<span class=\"screen-reader-text\"> {}</span><span aria-hidden=\"true\" class=\"dashicons dashicons-external\"></span></a></p>",
        esc_url(url),
        site.tr(label),
        site.tr("(opens in a new tab)")
    )
}

/// `WP_Site_Health::get_test_https_status()`. RustPress does not probe whether HTTPS would work.
pub fn https_status(site: &Site, is_ssl: bool) -> Value {
    let using_https = |url: &str| url.starts_with("https://");
    let admin = |anchor: &str| esc_url(&format!("{}/wp-admin/options-general.php#{anchor}", site.settings.siteurl.trim_end_matches('/')));
    let mut result = json!({
        "label": site.tr("Your website is using an active HTTPS connection"),
        "status": "good",
        "badge": badge(site, "Security"),
        "description": format!("<p>{}</p>", site.tr("An HTTPS connection is a more secure way of browsing the web. Many services now have HTTPS as a requirement. HTTPS allows you to take advantage of new features that can increase site speed, improve search rankings, and gain the trust of your visitors by helping to protect their online privacy.")),
        "actions": external(site, "https://developer.wordpress.org/advanced-administration/security/https/", "Learn more about why you should use HTTPS"),
        "test": "https_status",
    });
    if using_https(&site.settings.home) && using_https(&site.settings.siteurl) {
        return result;
    }
    let description = if using_https(&site.settings.siteurl) {
        if is_ssl {
            sprintf(&site.tr("You are accessing this website using HTTPS, but your <a href=\"%s\">Site Address</a> is not set up to use HTTPS by default."), &[&admin("home")])
        } else {
            sprintf(&site.tr("Your <a href=\"%s\">Site Address</a> is not set up to use HTTPS."), &[&admin("home")])
        }
    } else if is_ssl {
        sprintf(&site.tr("You are accessing this website using HTTPS, but your <a href=\"%1$s\">WordPress Address</a> and <a href=\"%2$s\">Site Address</a> are not set up to use HTTPS by default."), &[&admin("siteurl"), &admin("home")])
    } else {
        sprintf(&site.tr("Your <a href=\"%1$s\">WordPress Address</a> and <a href=\"%2$s\">Site Address</a> are not set up to use HTTPS."), &[&admin("siteurl"), &admin("home")])
    };
    result["status"] = json!("recommended");
    result["label"] = json!(site.tr("Your website does not use HTTPS"));
    result["description"] = json!(format!("<p>{description}</p><p>{}</p>", site.tr("Talk to your web host about supporting HTTPS for your website.")));
    result
}

/// `WP_Site_Health::get_test_authorization_header()`: `credentials` are the user and password of a Basic header.
pub fn authorization_header(site: &Site, credentials: Option<(&str, &str)>) -> Value {
    let mut result = json!({
        "label": site.tr("The Authorization header is working as expected"),
        "status": "good",
        "badge": badge(site, "Security"),
        "description": format!("<p>{}</p>", site.tr("The Authorization header is used by third-party applications you have approved for this site. Without this header, those apps cannot connect to your site.")),
        "actions": "",
        "test": "authorization_header",
    });
    let label = match credentials {
        None => site.tr("The authorization header is missing"),
        Some(("user", "pwd")) => return result,
        Some(_) => site.tr("The authorization header is invalid"),
    };
    result["label"] = json!(label);
    result["status"] = json!("recommended");
    let description = format!(
        "{}<p>{}</p>",
        result["description"].as_str().unwrap_or_default(),
        site.tr("If you are still seeing this warning after having tried the actions below, you may need to contact your hosting provider for further assistance.")
    );
    result["description"] = json!(description);
    result["actions"] = json!(format!(
        "<p><a href=\"{}\" target=\"_blank\">{}<span class=\"screen-reader-text\"> {}</span><span aria-hidden=\"true\" class=\"dashicons dashicons-external\"></span></a></p>",
        site.tr("https://developer.wordpress.org/rest-api/frequently-asked-questions/#why-is-authentication-not-working"),
        site.tr("Learn how to configure the Authorization header."),
        site.tr("(opens in a new tab)")
    ));
    result
}

/// `WP_Site_Health::get_test_dotorg_communication()`: RustPress makes no requests to WordPress.org and needs none.
pub fn dotorg_communication(site: &Site) -> Value {
    json!({
        "label": site.tr("RustPress does not communicate with WordPress.org"),
        "status": "good",
        "badge": badge(site, "Security"),
        "description": format!(
            "<p>{}</p><p>{}</p>",
            site.tr("Communicating with the WordPress servers is used to check for new versions, and to both install and update WordPress core, themes or plugins."),
            site.tr("RustPress does not contact the WordPress servers: updates are installed by replacing its program, and themes and plugins are installed from ZIP files.")
        ),
        "actions": "",
        "test": "dotorg_communication",
    })
}

/// `WP_Site_Health::get_test_background_updates()`: there are no background updates.
pub fn background_updates(site: &Site) -> Value {
    json!({
        "label": site.tr("RustPress does not use background updates"),
        "status": "good",
        "badge": badge(site, "Security"),
        "description": format!(
            "<p>{}</p><p>{}</p>",
            site.tr("Background updates ensure that WordPress can auto-update if a security update is released for the version you are currently using."),
            site.tr("RustPress is a single program and does not update itself; install a new version to update it.")
        ),
        "actions": "",
        "test": "background_updates",
    })
}

/// `WP_Site_Health::get_test_loopback_requests()`: scheduled events and the code editors run without requests to the site.
pub fn loopback_requests(site: &Site) -> Value {
    json!({
        "label": site.tr("Your site does not need loopback requests"),
        "status": "good",
        "badge": badge(site, "Performance"),
        "description": format!("<p>{}</p>", site.tr("Loopback requests are used to run scheduled events, and are also used by the built-in editors for themes and plugins to verify code stability.")),
        "actions": "",
        "test": "loopback_requests",
    })
}

/// The headers `WP_Site_Health::get_page_cache_headers()` looks for.
const PAGE_CACHE_HEADERS: [&str; 16] = [
    "cache-control",
    "expires",
    "age",
    "last-modified",
    "etag",
    "via",
    "x-cache-enabled",
    "x-cache-disabled",
    "cf-cache-status",
    "x-cache",
    "x-litespeed-cache",
    "x-srcache-store-status",
    "x-srcache-fetch-status",
    "x-cache-status",
    "x-proxy-cache",
    "x-varnish",
];

/// `WP_Site_Health::get_test_page_cache()`: `response_time` is the median time the home page took, in
/// milliseconds; `headers` are the client caching headers it came with.
pub fn page_cache(site: &Site, response_time: u64, headers: &[String]) -> Value {
    let threshold = 600u64;
    let mut description = format!(
        "<p>{}</p><p>{}</p><code>{}.</code>",
        site.tr("Page cache enhances the speed and performance of your site by saving and serving static pages instead of calling for a page every time a user visits."),
        site.tr("Page cache is detected by looking for an active page cache plugin as well as making three requests to the homepage and looking for one or more of the following HTTP client caching response headers:"),
        PAGE_CACHE_HEADERS.join("</code>, <code>")
    );
    let fast = response_time != 0 && response_time < threshold;
    let status = if fast { if headers.is_empty() { "recommended" } else { "good" } } else { "critical" };
    let label = match (status, headers.is_empty()) {
        ("recommended", _) => site.tr("Page cache is not detected but the server response time is OK"),
        ("good", _) => site.tr("Page cache is detected and the server response time is good"),
        (_, true) => site.tr("Page cache is not detected and the server response time is slow"),
        (_, false) => site.tr("Page cache is detected but the server response time is still slow"),
    };
    let mut summary: Vec<String> = Vec::new();
    let numbers = [site.number_i18n(response_time), site.number_i18n(threshold)];
    summary.push(if fast {
        format!("<span class=\"dashicons dashicons-yes-alt\" aria-hidden=\"true\"></span> {}", sprintf(&site.tr("Median server response time was %1$s milliseconds. This is less than the recommended %2$s milliseconds threshold."), &[&numbers[0], &numbers[1]]))
    } else {
        format!("<span class=\"dashicons dashicons-warning\" aria-hidden=\"true\"></span> {}", sprintf(&site.tr("Median server response time was %1$s milliseconds. It should be less than the recommended %2$s milliseconds threshold."), &[&numbers[0], &numbers[1]]))
    });
    if headers.is_empty() {
        summary.push(format!("<span class=\"dashicons dashicons-warning\" aria-hidden=\"true\"></span> {}", site.tr("No client caching response headers were detected.")));
        summary.push(format!("<span class=\"dashicons dashicons-warning\" aria-hidden=\"true\"></span> {}", site.tr("A page cache plugin was not detected.")));
    } else {
        let count = headers.len() as i64;
        summary.push(format!(
            "<span class=\"dashicons dashicons-yes-alt\" aria-hidden=\"true\"></span> {} <code>{}</code>.",
            sprintf(&site.trn("There was %d client caching response header detected:", "There were %d client caching response headers detected:", count), &[&count.to_string()]),
            headers.join("</code>, <code>")
        ));
    }
    description.push_str(&format!("<ul><li>{}</li></ul>", summary.join("</li><li>")));
    json!({
        "badge": badge(site, "Performance"),
        "description": description,
        "test": "page_cache",
        "status": status,
        "label": label,
        "actions": format!(
            "<p><a href=\"{}\" target=\"_blank\" rel=\"noreferrer\">{}<span class=\"screen-reader-text\"> {}</span><span aria-hidden=\"true\" class=\"dashicons dashicons-external\"></span></a></p>",
            site.tr("https://developer.wordpress.org/advanced-administration/performance/optimization/#caching"),
            site.tr("Learn more about page cache"),
            site.tr("(opens in a new tab)")
        ),
    })
}

/// The caching headers of a response, in the order of [`PAGE_CACHE_HEADERS`].
pub fn caching_headers(names: &[String]) -> Vec<String> {
    PAGE_CACHE_HEADERS.iter().filter(|h| names.iter().any(|n| n.eq_ignore_ascii_case(h))).map(|h| (*h).to_owned()).collect()
}
