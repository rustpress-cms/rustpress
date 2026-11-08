//! Site Kit by Google 1.191.0, the part visitors of the website get: the tags Site Kit places on the pages.
//!
//! - `Plugin::register()`: `<meta name="generator" content="Site Kit by Google 1.191.0" />` on `wp_head`.
//! - `GTag::register()`: the `wp_resource_hints` filter adds `//www.googletagmanager.com` to `dns-prefetch`.
//! - Analytics 4 (`googlesitekit_active_modules` holds `analytics-4`): on `template_redirect` the `Web_Tag` sets up
//!   `GTag`, which enqueues `https://www.googletagmanager.com/gtag/js?id=<tag id>` (`google_gtagjs`, async) with the
//!   inline `gtag()` calls after it, as long as `useSnippet` is on, a measurement ID is set and the environment is
//!   `production` (`Tag_Guard`, `Tag_Environment_Type_Guard`). For signed-in users that tracking is disabled for
//!   (`trackingDisabled`, by default `loggedinUsers`), `print_tracking_opt_out()` adds the opt-out snippet on
//!   `wp_head` at priority 0.
//! - Consent mode (`googlesitekit_consent_mode` with `enabled`): the default consent state on `wp_head` at priority 1
//!   and the plugin's `googlesitekit-consent-mode` script in the footer.
//! - Site verification (always active): the `google-site-verification` meta tags of the users who can set Site Kit up,
//!   and the verification file `/google<token>.html` on `init`.
//!
//! Not ported: the dashboard, the administration and everything that talks to Google's APIs, and with them the
//! answers of the REST routes (`GET /wp-json/` lists them); the other modules' tags
//! (Tag Manager, AdSense, Ads, Reader Revenue Manager, Sign in with Google); custom dimensions (`availableCustomDimensions`),
//! the Google tag gateway (a feature flag), conversion tracking of other plugins' events, `?tagverify=1`
//! (`Tag_Verify_Guard`), the `googlesitekit-setup` meta tag of a setup in progress and the translations of the
//! consent mode script. See the Site Kit section of PLAN.md.

use futures::future::BoxFuture;
use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_core::native::{ActivePlugin, Assets, Head, HeadContext, Init, NativePlugin, PluginRequest, PluginResponse, Robot};
use rustpress_core::{Result, Site};
use rustpress_phpser::Value;

pub struct SiteKit;

/// `GOOGLESITEKIT_VERSION`.
const VERSION: &str = "1.191.0";

/// `Regions::EU_USER_CONSENT_POLICY`.
const EU_REGIONS: [&str; 32] = ["AT", "BE", "BG", "CH", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GB", "GR", "HR", "HU", "IE", "IS", "IT", "LI", "LT", "LU", "LV", "MT", "NL", "NO", "PL", "PT", "RO", "SE", "SI", "SK"];

/// The consent mode script of `dist/manifest.php` (its hash is `null`, so it has no `ver`).
const CONSENT_MODE_SCRIPT: &str = "googlesitekit-consent-mode-5ef17bce870a966299fc.js";

impl NativePlugin for SiteKit {
    fn file(&self) -> &'static str {
        "google-site-kit/google-site-kit.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some("google-site-kit")
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move {
            let option = |name: &'static str| async move { Ok::<_, rustpress_core::Error>(site.options.fetch(&site.db, name).await?.map(|raw| rustpress_core::options::maybe_unserialize(&raw))) };
            let modules = active_modules(option("googlesitekit_active_modules").await?, option("googlesitekit-active-modules").await?);
            let analytics = if modules.iter().any(|m| m == "analytics-4") { Some(AnalyticsSettings::from_option(option("googlesitekit_analytics-4_settings").await?)) } else { None };
            let consent_mode = option("googlesitekit_consent_mode").await?.and_then(|v| v.get("enabled").map(truthy)).unwrap_or(false);
            let prefix = site.db.table_prefix().to_owned();
            let verification_tags = verification_tags(site, &prefix).await?;
            let home_domain = host_of(&site.settings.home);
            Ok(Box::new(Active { analytics, consent_mode, verification_tags, home_domain, file_meta_key: format!("{prefix}googlesitekit_site_verification_file") }) as Box<dyn ActivePlugin>)
        })
    }
}

/// `Modules::get_active_modules_option()`: the option, the legacy option, or `pagespeed-insights`.
fn active_modules(option: Option<Value>, legacy: Option<Value>) -> Vec<String> {
    let list = |value: &Value| value.as_array().map(|items| items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect::<Vec<_>>());
    option.as_ref().and_then(list).or_else(|| legacy.as_ref().and_then(list)).unwrap_or_else(|| vec!["pagespeed-insights".to_owned()])
}

/// A PHP value in a boolean context.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty() && s != b"0",
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

/// `empty()` of a value that is present.
fn non_empty(value: Option<&Value>) -> bool {
    value.is_some_and(truthy)
}

/// A value as PHP prints it in a string.
fn php_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Int(i)) => i.to_string(),
        Some(Value::Float(f)) => f.to_string(),
        Some(Value::Bool(true)) => "1".to_owned(),
        _ => String::new(),
    }
}

/// `PHP_URL_HOST` of the home address.
fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    host.split(':').next().unwrap_or_default().to_owned()
}

/// The settings of Analytics 4 the tags read (`googlesitekit_analytics-4_settings` with the defaults of
/// `Settings::get_default()` for missing keys).
#[derive(Debug, Clone, PartialEq)]
struct AnalyticsSettings {
    measurement_id: String,
    google_tag_id: String,
    use_snippet: bool,
    tracking_disabled: Vec<String>,
    /// `! empty( $settings['measurementID'] )`.
    measurement_set: bool,
}

impl AnalyticsSettings {
    /// The option after its `option_{$option}` filters: anything but an array gives the defaults, missing keys are
    /// filled in.
    fn from_option(option: Option<Value>) -> Self {
        let option = option.filter(|v| v.as_array().is_some());
        let get = |key: &str| option.as_ref().and_then(|o| o.get(key));
        let tracking_disabled = match get("trackingDisabled") {
            Some(value) => value.as_array().map(|items| items.iter().map(|(_, v)| php_string(Some(v))).collect()).unwrap_or_default(),
            None => vec!["loggedinUsers".to_owned()],
        };
        Self { measurement_id: php_string(get("measurementID")), google_tag_id: php_string(get("googleTagID")), use_snippet: get("useSnippet").is_none_or(truthy), tracking_disabled, measurement_set: non_empty(get("measurementID")) }
    }

    /// `Analytics_4::get_tag_id()`: the Google tag ID, or the measurement ID.
    fn tag_id(&self) -> &str {
        if self.google_tag_id.is_empty() || self.google_tag_id == "0" { &self.measurement_id } else { &self.google_tag_id }
    }

    /// `is_tracking_disabled()`: tracking is off for this signed-in user.
    fn tracking_disabled_for(&self, viewer: Option<&rustpress_core::Viewer>) -> bool {
        if !self.use_snippet {
            return false;
        }
        let Some(viewer) = viewer else { return false };
        self.tracking_disabled.iter().any(|d| d == "loggedinUsers") || (self.tracking_disabled.iter().any(|d| d == "contentCreators") && viewer.can("edit_posts"))
    }
}

/// `get_all_verification_tags()`: the stored tags of the users who can set Site Kit up (`manage_options`), unique and
/// not empty, in the order of the meta rows (or of the transient that caches them).
async fn verification_tags(site: &Site, prefix: &str) -> Result<Vec<String>> {
    let key = format!("{prefix}googlesitekit_site_verification_meta");
    let cached = site.options.fetch(&site.db, "_transient_googlesitekit_verification_meta_tags").await?.map(|raw| rustpress_core::options::maybe_unserialize(&raw));
    let user_tags: Vec<(u64, String)> = match cached.as_ref().and_then(|c| c.get("user_tags")).and_then(Value::as_array) {
        Some(items) => items
            .iter()
            .map(|(k, v)| {
                let id = match k {
                    rustpress_phpser::Key::Int(i) => u64::try_from(*i).unwrap_or(0),
                    rustpress_phpser::Key::Str(s) => String::from_utf8_lossy(s).parse().unwrap_or(0),
                };
                (id, php_string(Some(v)))
            })
            .collect(),
        None => {
            // `$user_tags[ (int) $user_id ] = $meta_value`: a later row of the same user replaces the earlier one in place.
            let mut tags: Vec<(u64, String)> = Vec::new();
            for (id, value) in site.db.user_meta_by_key(&key).await? {
                match tags.iter_mut().find(|(i, _)| *i == id) {
                    Some(entry) => entry.1 = value,
                    None => tags.push((id, value)),
                }
            }
            tags
        }
    };
    if user_tags.is_empty() {
        return Ok(Vec::new());
    }
    let roles = rustpress_auth::Roles::from_option(&site.options.fetch(&site.db, &format!("{prefix}user_roles")).await?.unwrap_or_default());
    let mut tags: Vec<String> = Vec::new();
    for (id, tag) in user_tags {
        if tag.is_empty() || tags.contains(&tag) || !can_set_up(site, &roles, id).await? {
            continue;
        }
        tags.push(tag);
    }
    Ok(tags)
}

/// `user_can( $user_id, Permissions::SETUP )`: `googlesitekit_setup` stands for `manage_options`.
async fn can_set_up(site: &Site, roles: &rustpress_auth::Roles, user_id: u64) -> Result<bool> {
    let Some(user) = site.db.auth_user_by_id(user_id).await? else { return Ok(false) };
    Ok(rustpress_auth::CurrentUser::load(&site.db, roles, &user).await?.can("manage_options"))
}

/// `print_site_verification_meta()` for one stored tag.
fn verification_meta(tag: &str) -> String {
    let tag = rustpress_core::formatting::html_entity_decode(tag).replace("&#039;", "'").replace("&#39;", "'");
    let tag = if tag.starts_with("<meta ") { tag } else { format!("<meta name=\"google-site-verification\" content=\"{}\">", esc_attr(&tag)) };
    rustpress_core::kses::kses(&tag, &rustpress_core::kses::Allowed::tags(&[("meta", &["name", "content"])]))
}

struct Active {
    analytics: Option<AnalyticsSettings>,
    consent_mode: bool,
    verification_tags: Vec<String>,
    /// `URL::parse( get_canonical_home_url(), PHP_URL_HOST )` for the linker.
    home_domain: String,
    /// The user meta key of the verification file tokens.
    file_meta_key: String,
}

impl Active {
    /// Whether `Analytics_4::register_tag()` places the tag on this site.
    fn tag(&self, site: &Site) -> Option<&AnalyticsSettings> {
        // `Tag_Guard`: `useSnippet` and a measurement ID; `Tag_Environment_Type_Guard`: `production`.
        self.analytics.as_ref().filter(|s| s.use_snippet && s.measurement_set && site.environment_type() == "production")
    }
}

impl ActivePlugin for Active {
    /// The routes `REST_Routes` and the modules register under `google-site-kit/v1`, recorded from the plugin.
    fn rest_index(&self) -> &'static str {
        include_str!("rest_index.json")
    }

    /// `Email_Log::register_post_statuses()`: the states of the email reports.
    fn internal_post_statuses(&self) -> &'static [&'static str] {
        &["email_sent", "email_failed", "email_scheduled"]
    }

    fn resource_hints(&self, relation: &str) -> Vec<String> {
        if relation == "dns-prefetch" { vec!["//www.googletagmanager.com".to_owned()] } else { Vec::new() }
    }

    fn enqueue(&self, site: &Site, assets: &mut Assets) {
        if let Some(settings) = self.tag(site) {
            // `GTag::enqueue_gtag_script()` on `wp_enqueue_scripts` at priority 20.
            let tag_id = settings.tag_id();
            let src = format!("https://www.googletagmanager.com/gtag/js?id={}", rustpress_core::canonical::rawurlencode(tag_id));
            let script = assets.script("google_gtagjs", src, &[], "", false);
            script.ver = None;
            script.priority = 20;
            let linker = serde_json::json!({ "domains": [self.home_domain] });
            script.after = vec![
                "window.dataLayer = window.dataLayer || [];function gtag(){dataLayer.push(arguments);}".to_owned(),
                format!("gtag(\"set\",\"linker\",{});", rustpress_blocks::php_json::encode(&linker)),
                "gtag(\"js\", new Date());".to_owned(),
                "gtag(\"set\", \"developer_id.dZTNiMT\", true);".to_owned(),
                format!("gtag(\"config\", \"{}\");", rustpress_core::formatting::esc_js(tag_id)),
            ];
            // `script_loader_tag`: `add_async_defer_attribute()`, the comment of `Web_Tag` (10), then the one of `GTag` (20).
            script.tag_attributes = " async".to_owned();
            script.tag_prefix = format!("\n<!-- {} -->\n<!-- {} -->\n", esc_html(&site.trd("google-site-kit", "Google tag (gtag.js) snippet added by Site Kit")), esc_html(&site.trd("google-site-kit", "Google Analytics snippet added by Site Kit")));
        }
        if self.consent_mode {
            let script = assets.script("googlesitekit-consent-mode", site.site_url(&format!("wp-content/plugins/google-site-kit/dist/assets/js/{CONSENT_MODE_SCRIPT}")), &[], "", true);
            script.ver = None;
            // Hooked on `init` at the default priority: after the theme's callback of the same priority.
            script.priority = 11;
        }
    }

    fn wp_head<'a>(&'a self, ctx: &'a HeadContext<'a>, _robots: &'a [(String, Robot)]) -> BoxFuture<'a, Result<Option<Head>>> {
        Box::pin(async move {
            let site = ctx.site;
            let mut head = Head::default();
            // `print_tracking_opt_out()` at priority 0.
            if let Some(settings) = &self.analytics
                && settings.tracking_disabled_for(ctx.query.viewer.as_ref())
            {
                // The tabs are the indentation of the PHP template (`\t<?php endif;` after the closing comment).
                head.first.push_str(&format!("<!-- {} -->\n\t", esc_html(&site.trd("google-site-kit", "Google Analytics opt-out snippet added by Site Kit"))));
                if !settings.measurement_id.is_empty() {
                    head.first.push_str(&format!("<script>\nwindow[\"ga-disable-{}\"] = true;\n</script>\n", esc_attr(&settings.measurement_id)));
                }
                head.first.push_str(&format!("\t<!-- {} -->\n\t", esc_html(&site.trd("google-site-kit", "End Google Analytics opt-out snippet added by Site Kit"))));
            }
            // `render_gtag_consent_data_layer_snippet()` at priority 1.
            if self.consent_mode {
                head.html.push_str(&consent_mode_snippet(site));
            }
            // Priority 10: the generator is hooked when the plugin loads, the verification tags when the modules
            // register on `init`.
            head.late.push_str(&format!("<meta name=\"generator\" content=\"Site Kit by Google {}\" />", esc_attr(VERSION)));
            for tag in &self.verification_tags {
                head.late_init.push_str(&verification_meta(tag));
            }
            Ok(Some(head))
        })
    }

    /// The generator and the verification tags are also hooked on `login_head`.
    fn login_head(&self, _site: &Site) -> String {
        let mut out = format!("<meta name=\"generator\" content=\"Site Kit by Google {}\" />", esc_attr(VERSION));
        for tag in &self.verification_tags {
            out.push_str(&verification_meta(tag));
        }
        out
    }

    fn init<'a>(&'a self, site: &'a Site, request: &'a PluginRequest<'a>) -> BoxFuture<'a, Result<Option<Init>>> {
        Box::pin(async move {
            // `Site_Verification::register()`: `GET /google<token>.html` of a user who can set Site Kit up.
            if !request.method.eq_ignore_ascii_case("GET") {
                return Ok(None);
            }
            let Some(token) = request.uri.strip_prefix("/google").and_then(|rest| rest.strip_suffix(".html")) else { return Ok(None) };
            if token.is_empty() || !token.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) {
                return Ok(None);
            }
            let users = site.db.user_meta_by_key(&self.file_meta_key).await?;
            let Some(&(user_id, _)) = users.iter().filter(|(_, v)| v == token).min_by_key(|(id, _)| *id) else { return Ok(None) };
            let prefix = site.db.table_prefix().to_owned();
            let roles = rustpress_auth::Roles::from_option(&site.options.fetch(&site.db, &format!("{prefix}user_roles")).await?.unwrap_or_default());
            if !can_set_up(site, &roles, user_id).await? {
                return Ok(None);
            }
            let body = format!("google-site-verification: google{}.html", esc_html(token));
            Ok(Some(Init::Response(PluginResponse { status: 200, headers: vec![("Content-Type".to_owned(), "text/html; charset=UTF-8".to_owned())], body })))
        })
    }
}

/// `Consent_Mode::render_gtag_consent_data_layer_snippet()`.
fn consent_mode_snippet(site: &Site) -> String {
    let regions: Vec<serde_json::Value> = EU_REGIONS.iter().map(|r| serde_json::Value::String((*r).to_owned())).collect();
    let mut defaults = serde_json::Map::new();
    for key in ["ad_personalization", "ad_storage", "ad_user_data", "analytics_storage", "functionality_storage", "security_storage", "personalization_storage"] {
        defaults.insert(key.to_owned(), "denied".into());
    }
    defaults.insert("region".to_owned(), serde_json::Value::Array(regions));
    defaults.insert("wait_for_update".to_owned(), 500.into());
    let defaults = rustpress_blocks::php_json::encode(&serde_json::Value::Object(defaults));
    let map = serde_json::json!({
        "statistics": ["analytics_storage"],
        "marketing": ["ad_storage", "ad_user_data", "ad_personalization"],
        "functional": ["functionality_storage", "security_storage"],
        "preferences": ["personalization_storage"],
    });
    let map = rustpress_blocks::php_json::encode(&map);
    format!(
        "<!-- {} -->\n<script id=\"google_gtagjs-js-consent-mode-data-layer\">\nwindow.dataLayer = window.dataLayer || [];function gtag(){{dataLayer.push(arguments);}}\ngtag('consent', 'default', {defaults});\nwindow._googlesitekitConsentCategoryMap = {map};\nwindow._googlesitekitConsents = {defaults};\n</script>\n<!-- {} -->\n",
        esc_html(&site.trd("google-site-kit", "Google tag (gtag.js) consent mode dataLayer added by Site Kit")),
        esc_html(&site.trd("google-site-kit", "End Google tag (gtag.js) consent mode dataLayer added by Site Kit")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(serialized: &str) -> Value {
        rustpress_core::options::maybe_unserialize(serialized)
    }

    #[test]
    fn home_host() {
        assert_eq!(host_of("https://example.org/blog"), "example.org");
        assert_eq!(host_of("http://user:pw@example.org:8080/?x"), "example.org");
        assert_eq!(host_of("http://localhost"), "localhost");
    }

    #[test]
    fn active_modules_fall_back() {
        let list = value(r#"a:2:{i:0;s:11:"analytics-4";i:1;s:14:"search-console";}"#);
        assert_eq!(active_modules(Some(list.clone()), None), ["analytics-4", "search-console"]);
        assert_eq!(active_modules(None, Some(list)), ["analytics-4", "search-console"]);
        assert_eq!(active_modules(None, None), ["pagespeed-insights"]);
    }

    #[test]
    fn analytics_defaults() {
        let settings = AnalyticsSettings::from_option(None);
        assert!(settings.use_snippet && !settings.measurement_set);
        assert_eq!(settings.tracking_disabled, ["loggedinUsers"]);
        let settings = AnalyticsSettings::from_option(Some(value(r#"a:4:{s:13:"measurementID";s:9:"G-ABC1234";s:11:"googleTagID";s:0:"";s:10:"useSnippet";b:0;s:16:"trackingDisabled";a:0:{}}"#)));
        assert!(!settings.use_snippet && settings.measurement_set);
        assert!(settings.tracking_disabled.is_empty());
        assert_eq!(settings.tag_id(), "G-ABC1234");
        let settings = AnalyticsSettings::from_option(Some(value(r#"a:2:{s:13:"measurementID";s:9:"G-ABC1234";s:11:"googleTagID";s:11:"GT-XYZ98765";}"#)));
        assert_eq!(settings.tag_id(), "GT-XYZ98765");
    }

    #[test]
    fn verification_meta_matches_the_plugin() {
        // Captured from `print_site_verification_meta()` of the plugin's code in the reference.
        assert_eq!(verification_meta("abc123"), r#"<meta name="google-site-verification" content="abc123">"#);
        assert_eq!(verification_meta("a&amp;b\"c"), r#"<meta name="google-site-verification" content="a&amp;b&quot;c">"#);
        assert_eq!(verification_meta(r#"&lt;meta name="google-site-verification" content="xyz" /&gt;"#), r#"<meta name="google-site-verification" content="xyz" />"#);
        assert_eq!(verification_meta(r#"<meta name="google-site-verification" content="q" onload="x">"#), r#"<meta name="google-site-verification" content="q">"#);
        assert_eq!(verification_meta("it&#039;s"), r#"<meta name="google-site-verification" content="it&apos;s">"#);
    }
}
