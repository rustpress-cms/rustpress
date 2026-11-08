//! Complianz – GDPR/CCPA Cookie Consent 7.5.6, the website part: the cookie banner and its settings
//! (`cmplz_banner_loader`), the statistics scripts, the cookie blocker (`cmplz_cookie_blocker`), the public REST
//! routes and the small shortcodes of `cmplz_document`.
//!
//! The settings live in the option `cmplz_options`, the banners in the table `{prefix}cmplz_cookiebanners`, the
//! plugin's own transients in the option `cmplz_transients`. The generated legal documents (`[cmplz-document]`, the
//! `complianz/document` block, the cookie policy) and the consent area are not ported yet (see PLAN.md), nor is the
//! admin side: the settings are made in a WordPress installation.

mod banner;
mod blocker;
#[cfg(test)]
mod golden;
mod options;
mod regions;
mod services;
mod stats;

use std::path::{Path, PathBuf};

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, Assets, Head, HeadContext, NativePlugin, RestAnswer, RestRequest, ShortcodeContext};
use rustpress_core::shortcodes::Atts;
use rustpress_core::{QueryResult, Result, Site};
use rustpress_db::Post;
use rustpress_phpser::Value;
use serde_json::{json, Map, Value as Json};

use banner::Banner;
use blocker::{Blocker, Placeholders};
use options::{item, option, php_string, truthy, Options};

pub struct Complianz;

/// `CMPLZ_VERSION`.
const VERSION: &str = "7.5.6";
const DOMAIN: &str = "complianz-gdpr";
const PLUGIN: &str = "complianz-gdpr";

/// The routes of `register_rest_route( 'complianz/v1', ... )` as the reference lists them.
const REST_INDEX: &str = include_str!("rest_index.json");

impl NativePlugin for Complianz {
    fn file(&self) -> &'static str {
        "complianz-gdpr/complianz-gpdr.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move { Ok(Box::new(Active::load(site, Path::new(".")).await?) as Box<dyn ActivePlugin>) })
    }

    fn load_at<'a>(&'a self, site: &'a Site, wp_root: &'a Path) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move { Ok(Box::new(Active::load(site, wp_root).await?) as Box<dyn ActivePlugin>) })
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some(DOMAIN)
    }
}

/// A transient of `cmplz_get_transient()`: its value and when it expires.
struct Transient {
    name: String,
    value: Value,
    expires: i64,
}

/// The document types of `cmplz_config::$pages`, by region, in their order.
const PAGES: &[(&str, &[&str])] = &[
    ("eu", &["cookie-statement", "privacy-statement"]),
    ("us", &["cookie-statement", "privacy-statement"]),
    ("uk", &["cookie-statement", "privacy-statement"]),
    ("ca", &["cookie-statement", "privacy-statement"]),
    ("au", &["cookie-statement", "privacy-statement"]),
    ("za", &["cookie-statement", "privacy-statement"]),
    ("br", &["cookie-statement", "privacy-statement"]),
    ("all", &["impressum"]),
];

/// The page of a document as `get_shortcode_page_id()` finds it.
struct DocumentPage {
    kind: &'static str,
    region: &'static str,
    /// `None`: no page (the plugin then links to `get_permalink( false )`).
    page: Option<(u64, String, String)>,
}

/// A custom page of a document type (`cmplz_{type}_custom_page`): its permalink and title.
struct CustomPage {
    kind: &'static str,
    id: u64,
    permalink: Option<String>,
    title: String,
}

struct Active {
    options: Options,
    wizard_completed: bool,
    /// `site_needs_cookie_warning()` before the excluded pages are looked at.
    needs_warning: bool,
    /// The end of `site_shares_data()`: a cookie of the plugin's list belongs to a service that shares data.
    shares_cookies: bool,
    excluded_posts: Vec<String>,
    banner: Option<Banner>,
    banner_id: String,
    blocker: Blocker,
    custom_scripts: Value,
    transients: Vec<Transient>,
    policy_id: String,
    documents: Vec<DocumentPage>,
    custom_pages: Vec<CustomPage>,
    /// The options `cmplz_{type}_custom_page_url`.
    custom_urls: Vec<(&'static str, String)>,
    /// The posts with the meta `cmplz_shortcode`.
    shortcode_meta: Vec<u64>,
    plugin_dir: PathBuf,
    plugin_url: String,
    theme_dir: PathBuf,
    upload_dir: PathBuf,
    upload_url: String,
    /// `filemtime()` of the banner script and of the cookie blocker's stylesheet.
    script_version: String,
    general_version: String,
    document_version: String,
}

fn filemtime(path: &Path) -> String {
    path.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs().to_string()).unwrap_or_default()
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// The query arguments of the request (`$_GET`), decoded.
fn query_args(query: &QueryResult) -> Vec<(String, String)> {
    let decode = |s: &str| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.replace('+', " ").as_bytes())).into_owned();
    query.raw_query.split('&').filter(|p| !p.is_empty()).map(|p| p.split_once('=').map_or((decode(p), String::new()), |(k, v)| (decode(k), decode(v)))).collect()
}

/// `global $post` while the page is printed: the post of a single view, else the first post of the main query.
fn global_post(query: &QueryResult) -> Option<&Post> {
    query.posts.first()
}

impl Active {
    async fn load(site: &Site, wp_root: &Path) -> Result<Self> {
        let transients = match option(site, "cmplz_transients").await {
            Some(Value::Array(items)) => items
                .iter()
                .map(|(k, v)| Transient {
                    name: php_string(&match k {
                        rustpress_phpser::Key::Int(n) => Value::Int(*n),
                        rustpress_phpser::Key::Str(b) => Value::Str(b.clone()),
                    }),
                    value: item(v, "value").cloned().unwrap_or(Value::Bool(false)),
                    expires: item(v, "expires").map(php_string).and_then(|e| e.parse().ok()).unwrap_or(0),
                })
                .collect(),
            _ => Vec::new(),
        };
        let current = |name: &str| transients.iter().find(|t| t.name == name && t.expires >= now()).map(|t| t.value.clone()).filter(truthy);
        let blocked_transient = current("cmplz_blocked_scripts");
        let options = Options::load(site, blocked_transient.as_ref().is_some_and(|v| matches!(v, Value::Array(items) if !items.is_empty()))).await;
        let plugin_dir = wp_root.join("wp-content/plugins").join(PLUGIN);
        let plugin_url = format!("{}/", site.site_url(&format!("wp-content/plugins/{PLUGIN}")));
        let theme_dir = wp_root.join("wp-content/themes").join(&site.settings.stylesheet);
        let theme_url = site.site_url(&format!("wp-content/themes/{}", site.settings.stylesheet));
        let uploads = site.options.string("upload_url_path", "");
        let uploads = if uploads.is_empty() { site.site_url("wp-content/uploads") } else { uploads.trim_end_matches('/').to_owned() };
        let upload_url = format!("{uploads}/complianz/");
        let upload_dir = wp_root.join("wp-content/uploads/complianz");
        let custom_scripts = option(site, "complianz_options_custom-scripts").await.unwrap_or(Value::Bool(false));
        let cf7 = site.active_plugins().iter().any(|f| f == "contact-form-7/wp-contact-form-7.php");
        let disabled = options::strings(&option(site, "cmplz_disabled_placeholders").await.unwrap_or(Value::Null));
        let placeholder_transients = transients
            .iter()
            .filter(|t| t.expires >= now() && (t.name.starts_with("cmplz_youtube_") || t.name.starts_with("cmplz_vimeo_image_") || t.name.starts_with("cmplz_dailymotion_image_") || t.name.starts_with("cmplz_googlemaps_image_")))
            .map(|t| (t.name.clone(), php_string(&t.value)))
            .collect();
        let placeholders = Placeholders {
            url: format!("{plugin_url}assets/images/placeholders/"),
            dir: plugin_dir.join("assets/images/placeholders"),
            theme_dir: theme_dir.clone(),
            theme_url,
            upload_url: upload_url.clone(),
            upload_dir: format!("{}/", upload_dir.display()),
            style: options.string("placeholder_style"),
            maps_format: options.string("google-maps-format"),
            dont_use: options.truthy("dont_use_placeholders"),
            disabled,
            loaded: services::loaded(&options),
            transients: placeholder_transients,
        };
        let blocker = Blocker::new(&options, &custom_scripts, blocked_transient.as_ref(), cf7, placeholders);

        // `cmplz_get_default_banner_id()` and `cmplz_get_cookiebanner()`.
        let has_table = option(site, "cmplz_cbdb_version").await.is_some_and(|v| truthy(&v));
        let rows = if has_table { site.db.complianz_banners().await.unwrap_or_default() } else { Vec::new() };
        let banner_id = match current("cmplz_default_banner_id") {
            Some(id) => php_string(&id),
            None => {
                let default = rows.iter().find(|row| row.iter().any(|(c, v)| c == "default" && v.as_deref().is_some_and(|v| v != "0" && !v.is_empty())));
                default.or(rows.first()).and_then(|row| row.iter().find(|(c, _)| c == "ID")).and_then(|(_, v)| v.clone()).unwrap_or_else(|| if has_table { String::new() } else { "0".into() })
            }
        };
        let banner = rows.iter().find(|row| row.iter().any(|(c, v)| c == "ID" && v.as_deref() == Some(banner_id.as_str()))).map(|row| Banner::from_row(row, &options));

        let shares_cookies = if option(site, "cmplz_cookietable_version").await.is_some_and(|v| truthy(&v)) { site.db.complianz_cookies_share_data().await.unwrap_or(false) } else { false };
        let documents = document_pages(site, &transients).await?;
        let mut custom_pages = Vec::new();
        for kind in ["cookie-statement", "privacy-statement", "impressum"] {
            if !options.is(kind, "custom") {
                continue;
            }
            let id: u64 = option(site, &format!("cmplz_{kind}_custom_page")).await.map(|v| php_string(&v)).unwrap_or_default().trim().parse().unwrap_or(0);
            let post = if id > 0 { site.db.post(id).await? } else { None };
            let permalink = match &post {
                Some(post) => Some(site.permalink(post).await?),
                None => None,
            };
            let title = post.as_ref().map(|p| rustpress_blocks::content::the_title_for(site, p)).unwrap_or_default();
            custom_pages.push(CustomPage { kind, id, permalink, title });
        }
        let mut custom_urls = Vec::new();
        for kind in ["cookie-statement", "privacy-statement", "impressum"] {
            custom_urls.push((kind, option(site, &format!("cmplz_{kind}_custom_page_url")).await.map(|v| php_string(&v)).unwrap_or_default()));
        }
        let wizard_completed = option(site, "cmplz_wizard_completed_once").await.is_some_and(|v| truthy(&v));
        let excluded_posts = options::strings(&option(site, "cmplz_excluded_posts_array").await.unwrap_or(Value::Null));
        let policy_id = option(site, "complianz_active_policy_id").await.map_or("1".to_owned(), |v| php_string(&v));
        let mut active = Self {
            wizard_completed,
            needs_warning: false,
            shares_cookies,
            excluded_posts,
            banner,
            banner_id,
            blocker,
            custom_scripts,
            transients,
            policy_id,
            documents,
            custom_pages,
            custom_urls,
            shortcode_meta: site.db.complianz_shortcode_posts().await.unwrap_or_default(),
            script_version: filemtime(&plugin_dir.join("cookiebanner/js/complianz.min.js")),
            general_version: filemtime(&plugin_dir.join("assets/css/cookieblocker.min.css")),
            document_version: String::new(),
            plugin_dir,
            plugin_url,
            theme_dir,
            upload_dir,
            upload_url,
            options,
        };
        active.document_version = filemtime(&active.plugin_dir.join(if active.options.truthy("use_document_css") { "assets/css/document.min.css" } else { "assets/css/document-grid.min.css" }));
        active.needs_warning = active.site_needs_cookie_warning();
        Ok(active)
    }

    fn transient(&self, name: &str) -> Option<&Value> {
        self.transients.iter().find(|t| t.name == name && t.expires >= now()).map(|t| &t.value).filter(|v| truthy(v))
    }

    /// `statistics_privacy_friendly()`.
    fn statistics_privacy_friendly(&self) -> bool {
        let statistics = self.options.string("compile_statistics");
        match statistics.as_str() {
            "no" => return true,
            "yes" | "clicky" | "matomo" => return false,
            _ => {}
        }
        let google = statistics == "google-analytics";
        if google || statistics == "google-tag-manager" {
            let info = self.options.get(if google { "compile_statistics_more_info" } else { "compile_statistics_more_info_tag_manager" });
            let has = |v: &str| matches!(&info, Value::Array(items) if items.iter().any(|(_, x)| matches!(x, Value::Str(b) if b == v.as_bytes())));
            if !has("accepted") || !has("ip-addresses-blocked") || !has("no-sharing") {
                return false;
            }
        }
        true
    }

    /// `consent_required_for_anonymous_stats()`; `array_search()` finds the first active region at index 0, which
    /// counts as not found.
    fn consent_required_for_anonymous_stats(&self) -> bool {
        let active: Vec<&regions::Region> = regions::REGIONS.iter().filter(|r| self.options.regions().iter().any(|c| c == r.code)).collect();
        if active.iter().any(|r| r.statistics_consent == "always") {
            return true;
        }
        let when_not_anonymous = active.iter().position(|r| r.statistics_consent == "when_not_anonymous").is_some_and(|i| i > 0);
        let statistics = self.options.string("compile_statistics");
        let google = statistics == "google-analytics" || statistics == "google-tag-manager";
        when_not_anonymous && google && self.options.is("consent_for_anonymous_stats", "yes")
    }

    /// `cookie_warning_required_stats( $region )`.
    fn cookie_warning_required_stats(&self, region: Option<&str>) -> bool {
        if self.options.is("consent_for_anonymous_stats", "yes") {
            return true;
        }
        if let Some(region) = region.and_then(regions::region) {
            return match region.statistics_consent {
                "no" => false,
                "always" => true,
                "when_not_anonymous" => self.options.is("eu_consent_regions", "yes") || !self.statistics_privacy_friendly(),
                _ => false,
            };
        }
        if self.statistics_privacy_friendly() && self.consent_required_for_anonymous_stats() {
            return true;
        }
        !self.statistics_privacy_friendly()
    }

    /// `site_shares_data()`.
    fn site_shares_data(&self) -> bool {
        if self.options.is("compile_statistics", "google-tag-manager") || self.options.uses_marketing_cookies() || self.blocker.has_scripts() {
            return true;
        }
        if self.options.is("uses_ad_cookies", "yes") || self.options.is("uses_social_media", "yes") || self.options.is("uses_thirdparty_services", "yes") {
            return true;
        }
        !self.statistics_privacy_friendly() || self.shares_cookies
    }

    /// `site_needs_cookie_warning()` without a region, before the excluded pages.
    fn site_needs_cookie_warning(&self) -> bool {
        self.options.has_region("us") || self.site_shares_data() || self.cookie_warning_required_stats(None)
    }

    /// `site_needs_cookie_warning()` for the request: no warning on a page of `cmplz_excluded_posts_array`.
    fn needs_warning_for(&self, query: &QueryResult) -> bool {
        let uri = sanitize_text_field(&request_uri(query));
        self.needs_warning && !self.excluded_posts.iter().any(|slug| uri.contains(slug.as_str()))
    }

    /// `cmplz_is_pagebuilder_preview()`.
    fn pagebuilder_preview(&self, query: &QueryResult) -> bool {
        const BUILDERS: &[&str] = &[
            "fb-edit", "builder_id", "et_pb_preview", "et_fb", "elementor-preview", "elementor_library", "elementor-app", "vc_action", "vc_editable", "vcv-action", "zion_builder_active", "zionbuilder-preview", "tb-preview", "tb-id", "fl_builder", "tve", "bricks",
            "ct_builder", "tatsu", "tatsu-header", "tatsu-footer",
        ];
        let args = query_args(query);
        let has = |name: &str| args.iter().any(|(k, _)| k == name);
        query.customize.is_some()
            || BUILDERS.iter().any(|b| has(b))
            || args.iter().any(|(k, v)| (k == "action" && v == "ms_get_preview") || (k == "context" && v == "edit"))
            || request_uri(query).contains("cornerstone/edit")
    }

    /// Whether the banner loader's hooks are on for the request (the constructor of `cmplz_banner_loader`).
    fn banner_active(&self, query: &QueryResult) -> bool {
        self.wizard_completed && self.needs_warning_for(query) && !self.pagebuilder_preview(query)
    }

    /// `cmplz_get_used_consenttypes()`.
    fn used_consenttypes(&self, query: &QueryResult) -> Vec<String> {
        let mut types: Vec<String> = Vec::new();
        for region in self.options.regions() {
            if let Some(r) = regions::region(&region) {
                types.push(self.filter_consenttype(r.kind, &region));
            }
        }
        if query_args(query).iter().any(|(k, _)| k == "cmplz_user_region") {
            types.push("optin".into());
            types.push("optout".into());
        }
        let mut unique: Vec<String> = Vec::new();
        for t in types {
            if !unique.contains(&t) {
                unique.push(t);
            }
        }
        unique
    }

    /// The `cmplz_consenttype` filter `maybe_filter_consenttype()`.
    fn filter_consenttype(&self, kind: &str, region: &str) -> String {
        let shares = self.site_shares_data();
        let sensitive = self.options.is("sensitive_information_processed", "yes");
        if region == "ca" && ((shares && sensitive) || self.options.is("ca_targets_quebec", "yes")) {
            return "optin".into();
        }
        if region == "au" && shares && sensitive && self.options.uses_marketing_cookies() {
            return "optin".into();
        }
        if region == "us" && self.options.is("us_california_optin", "yes") {
            return "optin".into();
        }
        kind.to_owned()
    }

    /// `COMPLIANZ::$company->get_default_region()`.
    fn default_region(&self) -> String {
        let country = self.options.string("country_company");
        let country = if regions::COUNTRIES.contains(&country.as_str()) { country } else { "US".into() };
        let company = regions::region_for_country(&country).unwrap_or("us");
        let regions = self.options.regions();
        if regions.len() == 1 {
            return regions[0].clone();
        }
        if regions.iter().any(|r| r == company) {
            return company.to_owned();
        }
        // `key( $regions )` of a list is 0, which is falsy: the company's region follows.
        company.to_owned()
    }

    /// `COMPLIANZ::$company->get_default_consenttype()`.
    fn default_consenttype(&self) -> String {
        let mut region = self.default_region();
        let selected = self.options.regions();
        if !selected.is_empty() && !selected.contains(&region) {
            region = selected[0].clone();
        }
        let kind = regions::region(&region).map_or("optin", |r| r.kind);
        self.filter_consenttype(kind, &region)
    }

    /// `get_statistics_category()`.
    fn statistics_category(&self) -> &'static str {
        let statistics = self.options.string("compile_statistics");
        let mut category = "statistics";
        let tagmanager = statistics == "google-tag-manager";
        if (!tagmanager && !self.cookie_warning_required_stats(None)) || tagmanager {
            category = "functional";
        }
        if statistics == "matomo" && self.options.is("matomo_anonymized", "yes") {
            category = "functional";
        }
        if statistics == "clarity" && self.options.is("clarity_consent_mode", "yes") {
            category = "functional";
        }
        if self.options.consent_mode() {
            category = "functional";
        }
        category
    }

    fn stats(&self) -> stats::Context<'_> {
        stats::Context { options: &self.options, category: self.statistics_category(), plugin_dir: &self.plugin_dir, theme_dir: &self.theme_dir, custom_scripts: &self.custom_scripts, cookie_prefix: "cmplz_" }
    }

    /// `get_cookie_path()`.
    fn cookie_path(&self, site: &Site) -> String {
        if self.options.truthy("set_cookies_on_root") {
            return "/".into();
        }
        let home = site.home_url("");
        let home = home.trim_end_matches('/');
        let host = home.split("://").nth(1).unwrap_or(home).split(['/', ':']).next().unwrap_or("").to_owned();
        let path = home.replace("http://", "").replace("https://", "").replace(&host, "");
        rustpress_core::urls::trailingslashit(&path)
    }

    /// `get_shortcode_page_id( $type, $region )`.
    fn document_page(&self, kind: &str, region: &str) -> Option<&(u64, String, String)> {
        self.documents.iter().find(|d| d.kind == kind && d.region == region).and_then(|d| d.page.as_ref())
    }

    /// `cmplz_page_is_of_type( $type )` for `global $post`.
    fn page_is_of_type(&self, kind: &str, post: Option<&Post>) -> bool {
        let Some(post) = post else { return false };
        self.options.regions().iter().any(|region| self.document_page(kind, region).is_some_and(|(id, _, _)| *id == post.id))
    }

    /// `get_page_links()` (or the transient `page_links_{banner}_{locale}`); `global_link` is
    /// `get_permalink( false )`, what a document without a page links to.
    fn page_links(&self, global_link: Option<&str>) -> Json {
        let name = format!("page_links_{}_{}", self.banner_id, self.options.locale());
        if let Some(cached) = self.transient(&name) {
            return php_to_json(cached);
        }
        let mut out: Vec<(String, Vec<(String, Json)>)> = Vec::new();
        for (region, kinds) in PAGES {
            for kind in *kinds {
                let Some((title, url)) = self.page_link(kind, region, global_link) else { continue };
                if url == "#" {
                    continue;
                }
                let entry = json!({ "title": title, "url": url });
                match out.iter_mut().find(|(r, _)| r == region) {
                    Some((_, list)) => list.push(((*kind).to_owned(), entry)),
                    None => out.push(((*region).to_owned(), vec![((*kind).to_owned(), entry)])),
                }
            }
        }
        if let Some(position) = out.iter().position(|(r, _)| r == "all") {
            let (_, general) = out.remove(position);
            for (region, _) in PAGES.iter().filter(|(r, _)| *r != "all") {
                for (kind, entry) in &general {
                    match out.iter_mut().find(|(r, _)| r == region) {
                        Some((_, list)) => match list.iter_mut().find(|(k, _)| k == kind) {
                            Some((_, slot)) => *slot = entry.clone(),
                            None => list.push((kind.clone(), entry.clone())),
                        },
                        None => out.push(((*region).to_owned(), vec![(kind.clone(), entry.clone())])),
                    }
                }
            }
        }
        if out.is_empty() {
            return Json::Array(Vec::new());
        }
        Json::Object(out.into_iter().map(|(r, list)| (r, Json::Object(list.into_iter().collect()))).collect())
    }

    /// `get_page_title()` and `get_page_url()` of a document.
    fn page_link(&self, kind: &str, region: &str, global_link: Option<&str>) -> Option<(String, String)> {
        if !self.options.has_region(region) {
            return Some((String::new(), "#".into()));
        }
        let setting = self.options.string(kind);
        let strip = |title: &str| {
            static PARENS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            PARENS.get_or_init(|| regex::Regex::new(r"(\(.*\))").expect("valid regex")).replace_all(title, "").into_owned()
        };
        match setting.as_str() {
            "none" => Some(("#".into(), "#".into())),
            "custom" => {
                let page = self.custom_pages.iter().find(|p| p.kind == kind)?;
                let title = if page.id == 0 { String::new() } else { page.title.clone() };
                let url = match &page.permalink {
                    Some(link) if page.id != 0 => rustpress_core::formatting::esc_url_raw(link),
                    _ => "#".into(),
                };
                Some((strip(&title), url))
            }
            "url" => {
                let url = rustpress_core::formatting::esc_url_raw(&self.custom_url(kind));
                let title = match kind {
                    "cookie-statement" => self.options.tr("Cookie Policy"),
                    "privacy-statement" => self.options.tr("Privacy Statement"),
                    _ => self.options.tr("Imprint"),
                };
                Some((strip(&title), url))
            }
            _ => match self.document_page(kind, region) {
                Some((_, permalink, title)) => Some((strip(title), permalink.clone())),
                None => Some((String::new(), global_link.map_or("#".into(), str::to_owned))),
            },
        }
    }

    fn custom_url(&self, kind: &str) -> String {
        self.custom_urls.iter().find(|(k, _)| *k == kind).map(|(_, v)| v.clone()).unwrap_or_default()
    }

    /// `blocked_content_text()`.
    fn blocked_content_text(&self) -> String {
        if self.options.is("consent_per_service", "yes") {
            let agree = self.options.string("agree_text_per_service");
            let text = self.options.string("blocked_content_text_per_service");
            return format!("<div class=\"cmplz-blocked-content-notice-body\">{text}&nbsp;<div class=\"cmplz-links\"><a href=\"#\" class=\"cmplz-link cookie-statement\">{{title}}</a></div></div><button class=\"cmplz-accept-service\">{agree}</button>");
        }
        self.options.string("blocked_content_text")
    }

    /// `get_front_end_settings()` with what `enqueue_assets()` adds, as `wp_localize_script()` passes it on.
    fn front_end_settings(&self, site: &Site, query: &QueryResult, global_link: Option<&str>) -> Map<String, Json> {
        let banner = self.banner.as_ref();
        let value = |name: &str| banner.map_or(Value::Null, |b| b.value(name));
        let consent_types = self.used_consenttypes(query);
        let css_exists = consent_types.iter().all(|t| self.upload_dir.join(format!("css/banner-{}-{t}.css", self.banner_id)).exists());
        let css_file = if css_exists { format!("{}css/banner-{{banner_id}}-{{type}}.css", self.upload_url) } else { format!("{}cookiebanner/css/defaults/banner-{{type}}.css", self.plugin_url) };
        let region = self.default_region();
        let args = query_args(query);
        let banner_version = banner.map_or("0".to_owned(), |b| b.banner_version.clone());
        let dismiss_timeout = if truthy(&value("dismiss_on_timeout")) { Value::Int(1000 * php_string(&value("dismiss_timeout")).parse::<i64>().unwrap_or(0)) } else { Value::Bool(false) };
        let disable = truthy(&value("disable_cookiebanner")) || query.flags.is_preview || self.pagebuilder_preview(query) || args.iter().any(|(k, _)| k == "cmplz_safe_mode");
        let cookie_domain = {
            let domain = self.options.string("cookie_domain").replace("http://", "").replace("https://", "");
            domain.strip_suffix('/').map(str::to_owned).unwrap_or(domain)
        };
        let locale = self.options.locale().to_owned();
        let tm = self.options.is("compile_statistics", "google-tag-manager") || self.options.is("compile_statistics", "matomo-tag-manager");
        let aria = if self.options.is("consent_per_service", "yes") { self.options.tr("Click button to enable {service}") } else { self.options.string("blocked_content_text") };
        let scalars: Vec<(&str, Value)> = vec![
            ("prefix", s("cmplz_")),
            ("debug", Value::Bool(false)),
            ("user_banner_id", s(&self.banner_id)),
        ];
        let mut out = Map::new();
        let put = |out: &mut Map<String, Json>, key: &str, value: Value| {
            out.insert(key.to_owned(), localized(&value));
        };
        for (k, v) in scalars {
            put(&mut out, k, v);
        }
        out.insert("set_cookies".into(), Json::Array(Vec::new()));
        put(&mut out, "block_ajax_content", self.options.get("enable_cookieblocker_ajax"));
        put(&mut out, "banner_version", s(&banner_version));
        put(&mut out, "version", s(VERSION));
        put(&mut out, "store_consent", Value::Bool(self.options.truthy("a_b_testing") || self.options.is("records_of_consent", "yes")));
        put(&mut out, "do_not_track_enabled", Value::Bool(!self.options.is("respect_dnt", "no")));
        put(&mut out, "consenttype", s(&self.default_consenttype()));
        put(&mut out, "region", s(&region));
        put(&mut out, "geoip", Value::Bool(false));
        put(&mut out, "dismiss_timeout", dismiss_timeout);
        put(&mut out, "disable_cookiebanner", Value::Bool(disable));
        put(&mut out, "soft_cookiewall", Value::Bool(truthy(&value("soft_cookiewall"))));
        put(&mut out, "dismiss_on_scroll", Value::Bool(truthy(&value("dismiss_on_scroll"))));
        put(&mut out, "cookie_expiry", self.options.get("cookie_expiry"));
        put(&mut out, "url", s(&site.rest_url("complianz/v1/")));
        put(&mut out, "locale", s(&format!("lang={}&locale={locale}", locale.get(..2).unwrap_or(&locale))));
        put(&mut out, "set_cookies_on_root", self.options.get("set_cookies_on_root"));
        put(&mut out, "cookie_domain", s(&cookie_domain));
        put(&mut out, "current_policy_id", s(&self.policy_id));
        put(&mut out, "cookie_path", s(&self.cookie_path(site)));
        out.insert(
            "categories".into(),
            json!({
                "statistics": self.options.tr_x("statistics", "as in: click to accept statistics cookies"),
                "marketing": self.options.tr_x("marketing", "as in: click to accept marketing cookies"),
            }),
        );
        put(&mut out, "tcf_active", Value::Bool(false));
        put(&mut out, "placeholdertext", s(&self.blocked_content_text()));
        put(&mut out, "css_file", s(&format!("{css_file}?v={banner_version}")));
        out.insert("page_links".into(), self.page_links(global_link));
        put(&mut out, "tm_categories", Value::Bool(tm));
        put(&mut out, "forceEnableStats", Value::Bool(!self.cookie_warning_required_stats(Some(&region))));
        put(&mut out, "preview", Value::Bool(false));
        put(&mut out, "clean_cookies", Value::Bool(!options::loose_eq_int(&self.options.get("safe_mode"), 1) && self.options.is("consent_per_service", "yes")));
        put(&mut out, "aria_label", s(&aria));
        let module = self.options.truthy("enable_iapi_navigation");
        put(&mut out, "iapi_nav", s(if module { "module" } else { "fallback" }));
        let singular = query.flags.is_singular();
        put(&mut out, "is_cookiepolicy_page", Value::Bool(singular && self.page_is_of_type("cookie-statement", global_post(query))));
        out
    }

    /// `cmplz_document::is_complianz_page()` of `global $post`.
    fn is_complianz_page(&self, post: Option<&Post>) -> bool {
        let Some(post) = post else { return false };
        let content = &post.post_content;
        if self.shortcode_meta.contains(&post.id) {
            return true;
        }
        if content.contains("[cmplz-terms-conditions") || content.contains("[cmplz-consent-area") {
            return false;
        }
        content.contains("<!-- wp:complianz/document") || has_shortcode(content, "cmplz-document") || has_shortcode(content, "cmplz-cookies") || content.contains("[cmplz-")
    }

    /// The document type of `global $post` as `cookiebanner_html()` reads it from the content.
    fn document_type(post: Option<&Post>) -> String {
        static GUTENBERG: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        static CLASSIC: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        static LEGACY: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let Some(post) = post.filter(|p| p.id > 0) else { return String::new() };
        let gutenberg = GUTENBERG.get_or_init(|| regex::Regex::new(r#"(?i)<!-- wp:complianz/document \{.*?"selectedDocument":"([^"].*?)".*?\} /-->"#).expect("valid regex"));
        let classic = CLASSIC.get_or_init(|| regex::Regex::new(r#"(?i)\[cmplz-document.*?type="(.*?)".*?region="(.*?)".*?\]"#).expect("valid regex"));
        let legacy = LEGACY.get_or_init(|| regex::Regex::new(r#"(?i)\[cmplz-document.*?type="(.*?)".*?\]"#).expect("valid regex"));
        let without_region = |kind: &str| {
            let region = PAGES.iter().map(|(r, _)| *r).find(|r| kind.ends_with(&format!("-{r}"))).unwrap_or("eu");
            kind.replace(&format!("-{region}"), "")
        };
        if let Some(c) = gutenberg.captures(&post.post_content) {
            return without_region(&c[1]);
        }
        if let Some(c) = classic.captures(&post.post_content) {
            return c[1].to_owned();
        }
        if let Some(c) = legacy.captures(&post.post_content) {
            return without_region(&c[1]);
        }
        String::new()
    }

    /// `cookiebanner_html()`.
    fn cookiebanner_html(&self, query: &QueryResult) -> String {
        if Self::document_type(global_post(query)) == "cookie-statement" {
            return String::new();
        }
        let mut banners = String::new();
        let mut manage = String::new();
        if let Some(banner) = &self.banner {
            for consent_type in self.used_consenttypes(query) {
                let (b, m) = banner.html(&consent_type, &self.options, &self.plugin_dir);
                banners.push_str(&b);
                manage.push_str(&m);
            }
        }
        format!("\n<!-- Consent Management powered by Complianz | GDPR/CCPA Cookie Consent https://wordpress.org/plugins/complianz-gdpr -->\n<div id=\"cmplz-cookiebanner-container\">{banners}</div>\n\t\t\t\t\t<div id=\"cmplz-manage-consent\" data-nosnippet=\"true\">{manage}</div>")
    }

    /// `cmplz_can_run_cookie_blocker()`.
    fn can_run_cookie_blocker(&self, query: &QueryResult) -> bool {
        if !self.needs_warning_for(query) || !self.options.is("enable_cookie_blocker", "yes") || query.flags.is_feed {
            return false;
        }
        if query.flags.is_preview || self.pagebuilder_preview(query) || query_args(query).iter().any(|(k, _)| k == "cmplz_safe_mode") {
            return false;
        }
        !self.options.truthy("safe_mode")
    }
}

/// `$_SERVER['REQUEST_URI']`.
fn request_uri(query: &QueryResult) -> String {
    if query.raw_query.is_empty() {
        query.url_path.clone()
    } else {
        format!("{}?{}", query.url_path, query.raw_query)
    }
}

/// `has_shortcode( $content, $tag )`: the tag as a shortcode (`[tag]`, `[tag ...]`, `[tag/]`).
fn has_shortcode(content: &str, tag: &str) -> bool {
    let open = format!("[{tag}");
    content.match_indices(&open).any(|(i, _)| matches!(content[i + open.len()..].chars().next(), Some(']' | ' ' | '/' | '\t' | '\n' | '\r')))
}

/// A value as `wp_localize_script()` passes it on: scalars as strings after `html_entity_decode()`.
fn localized(value: &Value) -> Json {
    match value {
        Value::Array(_) => php_to_json(value),
        Value::Null => Json::Null,
        _ => Json::String(html_entity_decode_quotes(&php_string(value))),
    }
}

/// `html_entity_decode( $text, ENT_QUOTES, 'UTF-8' )`.
fn html_entity_decode_quotes(text: &str) -> String {
    rustpress_core::formatting::html_entity_decode(text).replace("&#039;", "'").replace("&#39;", "'").replace("&apos;", "'")
}

/// A PHP value as `wp_json_encode()` writes it (a list as an array, other arrays as objects).
fn php_to_json(value: &Value) -> Json {
    rustpress_core::options::php_to_json(value)
}

fn s(text: &str) -> Value {
    options::s(text)
}

/// `wp_json_encode( $l10n, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )` of `WP_Scripts::localize()`.
fn localize(name: &str, value: &Json) -> String {
    let json = rustpress_blocks::php_json::encode_with(value, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E");
    format!("var {name} = {json};")
}

/// The pages of the documents (`get_shortcode_page_id()` for each type and region of `cmplz_config::$pages`), from
/// the transients `cmplz_shortcode_{type}-{region}` or the pages that hold a document.
async fn document_pages(site: &Site, transients: &[Transient]) -> Result<Vec<DocumentPage>> {
    let mut pages: Option<Vec<(u64, String, Option<String>)>> = None;
    let mut out = Vec::new();
    for (region, kinds) in PAGES {
        for kind in *kinds {
            let name = format!("cmplz_shortcode_{kind}-{region}");
            let cached = transients.iter().find(|t| t.name == name && t.expires >= now()).map(|t| php_string(&t.value));
            let id = match cached.as_deref() {
                Some("none") => None,
                Some(id) if !id.is_empty() && id != "0" => id.parse::<u64>().ok(),
                _ => {
                    if pages.is_none() {
                        pages = Some(site.db.complianz_document_pages().await.unwrap_or_default());
                    }
                    find_document_page(pages.as_deref().unwrap_or_default(), kind, region)
                }
            };
            let page = match id {
                Some(id) => match site.db.post(id).await? {
                    Some(post) => Some((id, site.permalink(&post).await?, rustpress_blocks::content::the_title_for(site, &post))),
                    None => None,
                },
                None => None,
            };
            out.push(DocumentPage { kind, region, page });
        }
    }
    Ok(out)
}

/// The search of `get_shortcode_page_id()` among the pages that hold a document.
fn find_document_page(pages: &[(u64, String, Option<String>)], kind: &str, region: &str) -> Option<u64> {
    let type_region = if region == "eu" { kind.to_owned() } else { format!("{kind}-{region}") };
    let html = |(_, content, meta): &(u64, String, Option<String>)| meta.clone().filter(|m| !m.is_empty() && m != "0").unwrap_or_else(|| content.clone());
    static SELECTED: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let selected = SELECTED.get_or_init(|| regex::Regex::new(r#"(?i)"selectedDocument":"(.*?)""#).expect("valid regex"));
    for page in pages {
        if selected.captures(&html(page)).is_some_and(|c| c[1] == *type_region) {
            return Some(page.0);
        }
    }
    for page in pages {
        let html = html(page);
        if has_shortcode(&html, "cmplz-document") && html.contains(&format!("type=\"{kind}\"")) && html.contains(&format!("region=\"{region}\"")) {
            return Some(page.0);
        }
    }
    for page in pages {
        let html = html(page);
        if region == "eu" && html.contains(" region=\"") {
            continue;
        }
        if has_shortcode(&html, "cmplz-document") && html.contains(&format!("type=\"{type_region}\"")) {
            return Some(page.0);
        }
    }
    None
}

impl ActivePlugin for Active {
    fn enqueue_query<'a>(&'a self, site: &'a Site, query: &'a QueryResult, assets: &'a mut Assets) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            // cmplz_document::enqueue_assets() on wp_enqueue_scripts at priority 10.
            if self.is_complianz_page(global_post(query)) {
                let (handle, file) = if self.options.truthy("use_document_css") { ("cmplz-document", "document.min.css") } else { ("cmplz-document-grid", "document-grid.min.css") };
                assets.style(handle, format!("{}assets/css/{file}", self.plugin_url), &[], &self.document_version, "all");
            }
            if !matches!(self.options.get("disable_cookie_block"), Value::Int(1)) {
                assets.style("cmplz-general", format!("{}assets/css/cookieblocker.min.css", self.plugin_url), &[], &self.general_version, "all");
            }
            if !self.banner_active(query) {
                return Ok(());
            }
            // cmplz_banner_loader::enqueue_assets() at PHP_INT_MAX - 50, then the services at PHP_INT_MAX.
            let global_link = match global_post(query) {
                Some(post) => Some(site.permalink(post).await?),
                None => None,
            };
            let settings = self.front_end_settings(site, query, global_link.as_deref());
            let script = assets.script("cmplz-cookiebanner", format!("{}cookiebanner/js/complianz.min.js", self.plugin_url), &[], &self.script_version, true);
            script.priority = i32::MAX - 50;
            script.defer_attr = true;
            script.extra = Some(localize("complianz", &Json::Object(settings)));
            script.after.extend(services::inline_scripts(&self.options));
            // enqueue_banner_css(): only with one consent type and a generated stylesheet.
            let types = self.used_consenttypes(query);
            if types.len() == 1 {
                let relative = format!("css/banner-{}-{}.css", self.banner_id.parse::<i64>().unwrap_or(0), types[0]);
                let file = self.upload_dir.join(&relative);
                if file.exists() {
                    let handle = format!("cmplz-banner-{}-{}", self.banner_id.parse::<i64>().unwrap_or(0), types[0]);
                    let style = assets.style(&handle, format!("{}{relative}", self.upload_url), &[], &filemtime(&file), "all");
                    style.priority = i32::MAX - 50;
                }
            }
            Ok(())
        })
    }

    fn wp_head<'a>(&'a self, ctx: &'a HeadContext<'a>, _robots: &'a [(String, rustpress_core::native::Robot)]) -> BoxFuture<'a, Result<Option<Head>>> {
        Box::pin(async move {
            if !self.banner_active(ctx.query) {
                return Ok(None);
            }
            let mut head = Head { robots: None, ..Head::default() };
            // dynamic_gtm_enqueue(): the consent scripts of Google Tag Gateway, and the statistics in the head.
            head.first.push_str(&self.stats().gtg_consent_script());
            if self.options.is("gtm_code_head", "yes") {
                head.first.push_str(&self.stats().inline_cookie_script());
            }
            // cookiebanner_css().
            head.late.push_str("\t\t\t<style>.cmplz-hidden {\n\t\t\t\t\tdisplay: none !important;\n\t\t\t\t}</style>\n\t\t\t\t");
            Ok(Some(head))
        })
    }

    fn wp_footer_query<'a>(&'a self, _site: &'a Site, query: &'a QueryResult) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            if !self.banner_active(query) {
                return Ok(String::new());
            }
            let mut out = self.cookiebanner_html(query);
            out.push_str(&services::footer_css(&self.options));
            Ok(out)
        })
    }

    fn print_footer_scripts(&self, _site: &Site, query: &QueryResult) -> String {
        if self.pagebuilder_preview(query) {
            return String::new();
        }
        if self.wizard_completed && self.needs_warning_for(query) {
            if self.options.is("gtm_code_head", "yes") {
                return String::new();
            }
            return self.stats().inline_cookie_script();
        }
        self.stats().inline_cookie_script_no_warning()
    }

    fn body_class<'a>(&'a self, _site: &'a Site, query: &'a QueryResult) -> BoxFuture<'a, Result<Vec<String>>> {
        Box::pin(async move { Ok(if self.is_complianz_page(global_post(query)) { vec!["cmplz-document".to_owned()] } else { Vec::new() }) })
    }

    fn template_output(&self, _site: &Site, query: &QueryResult, html: String) -> String {
        if !self.can_run_cookie_blocker(query) {
            return html;
        }
        self.blocker.replace_tags(html, &self.blocked_content_text())
    }

    fn shortcode_tags(&self) -> &'static [&'static str] {
        &["cmplz-manage-consent", "cmplz-revoke-link", "cmplz-accept-link"]
    }

    fn shortcode<'a>(&'a self, tag: &'a str, atts: &'a Atts, _content: Option<&'a str>, _ctx: &'a ShortcodeContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let text = atts.named.iter().rev().find(|(k, _)| k.eq_ignore_ascii_case("text")).map(|(_, v)| v.clone()).filter(|v| !v.is_empty() && v != "0");
            Ok(match tag {
                "cmplz-manage-consent" => format!(
                    "<div id=\"cmplz-manage-consent-container-nojavascript\">{}&nbsp;{}</div><div id=\"cmplz-manage-consent-container\" name=\"cmplz-manage-consent-container\" class=\"cmplz-manage-consent-container\"></div>",
                    self.options.tr_x("You have loaded the Cookie Policy without javascript support.", "cookie policy"),
                    self.options.tr_x("On AMP, you can use the manage consent button on the bottom of the page.", "cookie policy"),
                ),
                "cmplz-revoke-link" => {
                    let text = text.map(|t| sanitize_text_field(&t)).filter(|t| !t.is_empty() && t != "0").unwrap_or_else(|| self.options.tr("Revoke"));
                    let status = self.options.tr("Current status: %s");
                    format!(
                        "{REVOKE_CSS}<button class=\"cmplz-deny cmplz-revoke-custom\">{}</button>&nbsp;<span class=\"cmplz-status-accepted\">{}</span><span class=\"cmplz-status-denied\">{}</span>",
                        rustpress_core::formatting::esc_html(&text),
                        status.replacen("%s", &self.options.tr("Accepted"), 1),
                        status.replacen("%s", &self.options.tr("Denied"), 1),
                    )
                }
                _ => {
                    let text = text.map(|t| sanitize_text_field(&t)).unwrap_or_else(|| self.options.tr("Click to accept marketing cookies"));
                    format!("<div class=\"cmplz-custom-accept-btn cmplz-accept\"><a href=\"#\">{}</a></div>", rustpress_core::formatting::esc_html(&text))
                }
            })
        })
    }

    fn rest_index(&self) -> &'static str {
        REST_INDEX
    }

    fn rest<'a>(&'a self, request: &'a RestRequest<'a>) -> BoxFuture<'a, Result<Option<RestAnswer>>> {
        Box::pin(async move {
            let route = request.route.trim_end_matches('/');
            Ok(match (request.method, route) {
                ("GET", "/complianz/v1/banner") => {
                    let region = self.default_region();
                    let banner_version = self.banner.as_ref().map_or("0".to_owned(), |b| b.banner_version.clone());
                    Some(RestAnswer::ok(json!({
                        "consenttype": self.default_consenttype(),
                        "region": region,
                        "version": VERSION,
                        "forceEnableStats": !self.cookie_warning_required_stats(Some(&region)),
                        "banner_version": banner_version,
                    })))
                }
                ("POST", "/complianz/v1/track") => Some(RestAnswer::ok(json!({ "success": true }))),
                ("GET", "/complianz/v1/cookie_data") => {
                    // load_cookie_data(): the list of the cookie shredder, only with consent per service.
                    let list = if !options::loose_eq_int(&self.options.get("safe_mode"), 1) && self.options.is("consent_per_service", "yes") { self.transient("cmplz_cookie_shredder_list").map(php_to_json) } else { None };
                    Some(RestAnswer::ok(list.unwrap_or(Json::Array(Vec::new()))))
                }
                ("GET", "/complianz/v1/manage_consent_html") => Some(RestAnswer::ok(Json::String(self.manage_consent_html()))),
                _ => None,
            })
        })
    }
}

impl Active {
    /// `cmplz_rest_api_manage_consent_html()` (the Do Not Track signal of the browser is not looked at).
    fn manage_consent_html(&self) -> String {
        let Some(banner) = &self.banner else { return String::new() };
        banner.categories_html(&self.default_consenttype(), &self.options, &self.plugin_dir)
    }
}

/// The style and script of `cmplz_revoke_link()`.
const REVOKE_CSS: &str = "<style>.cmplz-status-accepted,.cmplz-status-denied {display: none;}</style>\n\t\t\t\t<script>\n\t\t\t\tdocument.addEventListener('cmplz_before_cookiebanner', function(){\n                    if (cmplz_has_consent('marketing')) {\n\t\t\t\t        document.querySelector('.cmplz-status-accepted').style.display = 'block';\n\t\t\t\t        document.querySelector('.cmplz-status-denied').style.display = 'none';\n\t\t\t\t    } else {\n\t\t\t\t\t\tdocument.querySelector('.cmplz-status-accepted').style.display = 'none';\n\t\t\t\t        document.querySelector('.cmplz-status-denied').style.display = 'block';\n\t\t\t\t\t\tdocument.querySelector('.cmplz-revoke-custom').setAttribute('disabled', true);\n\n\t\t\t\t    }\n                    document.addEventListener('click', e => {\n\t\t\t\t\t\tif ( e.target.closest('.cmplz-revoke-custom') ) {\n\t\t\t\t\t\t\tdocument.querySelector('.cmplz-revoke-custom').setAttribute('disabled', true);\n                            cmplz_set_banner_status('dismissed');\n\t\t\t\t\t\t}\n\t\t\t\t\t});\n                    document.addEventListener('click', e => {\n\t\t\t\t\t\tif ( e.target.closest('.cmplz-accept') ) {\n                            document.querySelector('.cmplz-status-accepted').style.display = 'block';\n\t\t\t\t        \tdocument.querySelector('.cmplz-status-denied').style.display = 'none';\n\t\t\t\t\t\t\tdocument.querySelector('.cmplz-revoke-custom').removeAttribute('disabled');\n\t\t\t\t\t\t}\n\t\t\t\t\t});\n\t\t\t\t});\n\t\t\t</script>";

/// `sanitize_text_field()` for a short text: tags stripped, line breaks and tabs collapsed, percent-encoded octets
/// removed, trimmed (no check of the UTF-8).
fn sanitize_text_field(text: &str) -> String {
    static TAGS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static SPACE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static OCTETS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let mut text = if text.contains('<') { TAGS.get_or_init(|| regex::Regex::new(r"(?s)<[^>]*?>").expect("valid regex")).replace_all(text, "").into_owned() } else { text.to_owned() };
    let octets = OCTETS.get_or_init(|| regex::Regex::new(r"(?i)%[a-f0-9]{2}").expect("valid regex"));
    while octets.is_match(&text) {
        text = octets.replace_all(&text, "").into_owned();
    }
    SPACE.get_or_init(|| regex::Regex::new(r"[\r\n\t ]+").expect("valid regex")).replace_all(&text, " ").trim().to_owned()
}
