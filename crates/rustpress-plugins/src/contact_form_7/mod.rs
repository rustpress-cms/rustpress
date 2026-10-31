//! Contact Form 7 6.2.4.1 (`contact-form-7/wp-contact-form-7.php`): contact forms stored as posts of the type
//! `wpcf7_contact_form`, printed by the shortcode `[contact-form-7]` (also inside the block
//! `contact-form-7/contact-form-selector`) and sent through the plugin's REST API.

pub mod controls;
pub mod form;
pub mod form_tag;
pub mod formatter;

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, Assets, NativePlugin, ShortcodeContext};
use rustpress_core::shortcodes::Atts;
use rustpress_core::{Result, Site};
use rustpress_db::{Order, OrderBy, PostQuery};

use controls::{Render, UserProps};
use form::{ContactForm, HtmlOptions, Place, form_html};

/// `WPCF7_VERSION`.
pub const VERSION: &str = "6.2.4.1";

const DOMAIN: &str = "contact-form-7";

pub struct ContactForm7;

impl NativePlugin for ContactForm7 {
    fn file(&self) -> &'static str {
        "contact-form-7/wp-contact-form-7.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some(DOMAIN)
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move {
            let query = PostQuery {
                post_types: vec![form::POST_TYPE.into()],
                statuses: ["publish", "future", "draft", "pending", "private", "trash", "auto-draft", "inherit"].map(String::from).to_vec(),
                order_by: OrderBy::Id,
                order: Order::Asc,
                limit: None,
                ..PostQuery::default()
            };
            let mut forms = Vec::new();
            for post in site.db.posts(&query).await? {
                let meta: Vec<(String, String)> = site.db.post_meta(post.id).await?.into_iter().filter_map(|m| Some((m.meta_key?, m.meta_value.unwrap_or_default()))).collect();
                forms.push(ContactForm::from_post(&post, &meta));
            }
            Ok(Box::new(Loaded { forms }) as Box<dyn ActivePlugin>)
        })
    }
}

/// The plugin with the contact forms of the site.
pub struct Loaded {
    pub forms: Vec<ContactForm>,
}

/// `(int) $string`: the leading digits.
fn php_int(s: &str) -> u64 {
    let digits: String = s.trim_start().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

impl Loaded {
    /// `WP_Query` with `post_status => any`: what `WPCF7_ContactForm::find()` looks through.
    fn findable(&self) -> impl Iterator<Item = &ContactForm> {
        self.forms.iter().filter(|f| f.status != "trash" && f.status != "auto-draft")
    }

    /// `wpcf7_get_contact_form_by_hash()`.
    fn by_hash(&self, hash: &str) -> Option<&ContactForm> {
        let valid = hash.len() >= 7 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !valid {
            return None;
        }
        self.findable().find(|f| f.hash.starts_with(hash))
    }

    /// `wpcf7_contact_form( $id )`: `get_post()` of any status.
    fn by_id(&self, id: &str) -> Option<&ContactForm> {
        let id = php_int(id);
        (id != 0).then(|| self.forms.iter().find(|f| f.id == id)).flatten()
    }

    /// `wpcf7_get_contact_form_by_title()`.
    fn by_title(&self, title: &str) -> Option<&ContactForm> {
        if title.is_empty() {
            return None;
        }
        self.findable().find(|f| f.title == title)
    }

    /// `wpcf7_get_contact_form_by_old_id()`.
    fn by_old_id(&self, old: u64) -> Option<&ContactForm> {
        self.findable().find(|f| f.old_unit_id.as_deref().is_some_and(|v| v.trim().parse::<f64>().ok() == Some(old as f64)))
    }

    /// `wpcf7_contact_form_tag_func()`.
    async fn shortcode_html(&self, tag: &str, atts: &Atts, ctx: &ShortcodeContext<'_>) -> Result<String> {
        let site = ctx.site;
        if ctx.query.flags.is_feed {
            return Ok("[contact-form-7]".to_owned());
        }
        let get = |name: &str| atts.get(name).unwrap_or_default().to_owned();
        let (form, options, shortcode_atts) = if tag == "contact-form-7" {
            let id = get("id").trim().to_owned();
            let title = get("title").trim().to_owned();
            let form = self.by_hash(&id).or_else(|| self.by_id(&id)).or_else(|| self.by_title(&title));
            let options = HtmlOptions { html_id: get("html_id"), html_name: get("html_name"), html_title: get("html_title"), html_class: get("html_class"), raw_form: get("output") == "raw_form" };
            let names = ["id", "title", "html_id", "html_name", "html_title", "html_class", "output"];
            let shortcode_atts: Vec<(String, String)> = names.iter().map(|n| ((*n).to_owned(), get(n))).collect();
            (form, options, shortcode_atts)
        } else {
            let old = atts.positional.first().map_or(0, |id| php_int(id));
            (self.by_old_id(old), HtmlOptions::default(), Vec::new())
        };
        let Some(form) = form else {
            return Ok(format!(
                "<p class=\"wpcf7-contact-form-not-found\"><strong>{}</strong> {}</p>",
                rustpress_core::formatting::esc_html(&site.trd(DOMAIN, "Error:")),
                rustpress_core::formatting::esc_html(&site.trd(DOMAIN, "Contact form not found."))
            ));
        };
        // `wpcf7_switch_locale()`: the form speaks its own language.
        let site_locale = site.locale();
        let same_language = form.locale.is_empty() || form.locale == site_locale;
        let tr = move |text: &str| if same_language { site.trd(DOMAIN, text) } else { text.to_owned() };
        let viewer = ctx.query.viewer.as_ref();
        let user = match viewer {
            Some(v) if form.form.contains("user_") => match site.db.user(v.id).await? {
                Some(u) => {
                    let mut meta = Vec::new();
                    for key in ["first_name", "last_name", "nickname", "description"] {
                        meta.push((key.to_owned(), site.db.user_meta(u.id, key).await?.unwrap_or_default()));
                    }
                    Some(UserProps { login: u.user_login, email: u.user_email, url: u.user_url, display_name: u.display_name, nicename: u.user_nicename, meta })
                }
                None => None,
            },
            _ => None,
        };
        let post_meta: Option<Vec<(String, String)>> = match ctx.loop_post {
            Some(id) if form.form.contains("post_meta") => Some(site.db.post_meta(id).await?.into_iter().filter_map(|m| Some((m.meta_key?, m.meta_value.unwrap_or_default()))).collect()),
            _ => None,
        };
        let query: Vec<(String, String)> = rustpress_core::QueryVars::parse(&ctx.query.raw_query).iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect();
        let request_uri = request_uri(&ctx.query.url_path, &ctx.query.raw_query);
        let home = site.home_url("");
        let html_type = site.options.get("html_type").unwrap_or("text/html").to_owned();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        let today = rustpress_core::datetime::SiteTimezone::from_options(&site.options).format("Y-m-d", now);
        let today = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap_or_default();
        // `generate_unit_tag()` counts only the forms that get past the raw output and the subscribers-only notice.
        let counted = !options.raw_form && !(form.is_true("subscribers_only") && viewer.is_none());
        let (count, mut used) = {
            let mut state = ctx.state.lock().expect("plugin state");
            (if counted { state.next("wpcf7_unit_tag") } else { 0 }, state.lists.remove("wpcf7_used_ids").unwrap_or_default())
        };
        let place = Place {
            request_uri: &request_uri,
            home: &home,
            loop_post: ctx.loop_post,
            html_type: &html_type,
            count,
            nonce: viewer.map(|v| v.create_nonce("wp_rest")),
            logged_in: viewer.is_some(),
            version: VERSION,
        };
        let quiz_hash = |answer: &str| rustpress_blocks::commenter::wp_hash(answer, "wpcf7_quiz");
        let mut random = |n: usize| (rand_index() % n as u64) as usize;
        let template = form.form.clone();
        let mut render = Render {
            tr: &tr,
            quiz_hash: &quiz_hash,
            query: &query,
            user: user.as_ref(),
            post_meta: post_meta.as_deref(),
            shortcode_atts: &shortcode_atts,
            template: &template,
            today,
            used_ids: &mut used,
            random: &mut random,
            responses: 0,
        };
        let html = form_html(form, &options, &place, &mut render);
        ctx.state.lock().expect("plugin state").lists.insert("wpcf7_used_ids", used);
        Ok(html)
    }
}

/// A random number for `array_rand()` (the question a quiz asks).
fn rand_index() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    h.finish()
}

/// `urlencode()` of PHP.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `wpcf7_get_request_uri()`: `add_query_arg( array() )` of the request, rooted and cleaned by `sanitize_url()`.
/// The query is read by `parse_str()` and written again by `build_query()` with `urlencode()`d values; a key
/// without value loses its `=`.
pub fn request_uri(path: &str, query: &str) -> String {
    let decode = |s: &str| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.replace('+', " ").as_bytes())).into_owned();
    let mut pairs: Vec<(String, String)> = Vec::new();
    for part in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = part.split_once('=').unwrap_or((part, ""));
        // parse_str() turns spaces and dots of the key into underscores.
        let key = decode(k).replace([' ', '.'], "_");
        if key.is_empty() {
            continue;
        }
        let value = decode(v);
        match pairs.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => pairs.push((key, value)),
        }
    }
    let built: Vec<String> = pairs.iter().map(|(k, v)| if v.is_empty() { k.clone() } else { format!("{k}={}", urlencode(v)) }).collect();
    let uri = if built.is_empty() { path.to_owned() } else { format!("{path}?{}", built.join("&")) };
    rustpress_core::formatting::esc_url_raw(&format!("/{}", uri.trim_start_matches('/')))
}

impl ActivePlugin for Loaded {
    fn enqueue(&self, site: &Site, assets: &mut Assets) {
        let base = site.site_url("wp-content/plugins/contact-form-7");
        assets.style("contact-form-7", format!("{base}/includes/css/styles.css"), &[], VERSION, "all");
        if site.is_rtl() {
            assets.style("contact-form-7-rtl", format!("{base}/includes/css/styles-rtl.css"), &["contact-form-7"], VERSION, "all");
        }
        assets.script("swv", format!("{base}/includes/swv/js/index.js"), &[], VERSION, true).registered_only = true;
        let root = rustpress_core::formatting::esc_url_raw(&site.rest_url(""));
        let object = format!("var wpcf7 = {{\n    \"api\": {{\n        \"root\": {},\n        \"namespace\": \"contact-form-7\\/v1\"\n    }}\n}};", json_string(&root));
        let script = assets.script("contact-form-7", format!("{base}/includes/js/index.js"), &["wp-i18n", "swv"], VERSION, true);
        script.translations = Some(DOMAIN.to_owned());
        script.before.push(object);
    }

    fn shortcode_tags(&self) -> &'static [&'static str] {
        &["contact-form-7", "contact-form"]
    }

    fn shortcode<'a>(&'a self, tag: &'a str, atts: &'a Atts, _content: Option<&'a str>, ctx: &'a ShortcodeContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(self.shortcode_html(tag, atts, ctx))
    }
}

/// A string as `wp_json_encode()` writes it (slashes escaped).
fn json_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default().replace('/', "\\/")
}

#[cfg(test)]
mod tests {
    use super::controls::Render;
    use super::form::{ContactForm, HtmlOptions, Place, form_html};

    fn decode(s: &str) -> String {
        String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.replace('+', " ").as_bytes())).into_owned()
    }

    #[test]
    fn request_uri_is_rooted() {
        assert_eq!(super::request_uri("/cf7-test/", ""), "/cf7-test/");
        assert_eq!(super::request_uri("/a/", "x=1&y=2"), "/a/?x=1&y=2");
        assert_eq!(super::request_uri("/", "rest_route=/wp/v2/pages&_embed&per_page=10"), "/?rest_route=%2Fwp%2Fv2%2Fpages&_embed&per_page=10");
        assert_eq!(super::request_uri("/", "s=a+b&s=c%20d&a.b=1"), "/?s=c+d&a_b=1");
    }

    /// The forms `dev/cf7-form-golden.sh` had the plugin print.
    #[test]
    fn golden_forms() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("cf7-forms");
        let mut cases = 0;
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("golden directory") {
            let path = entry.expect("entry").path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".form")).map(str::to_owned) else { continue };
            let template = std::fs::read_to_string(&path).expect("form");
            let expected = std::fs::read_to_string(dir.join(format!("{name}.html"))).expect("expected output");
            let settings = std::fs::read_to_string(dir.join(format!("{name}.settings"))).unwrap_or_default();
            let atts: Vec<(String, String)> = std::fs::read_to_string(dir.join(format!("{name}.atts")))
                .unwrap_or_default()
                .trim()
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap_or((p, ""));
                    (decode(k), decode(v))
                })
                .collect();
            let att = |k: &str| atts.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()).unwrap_or_default();
            let options = HtmlOptions { html_id: att("html_id"), html_name: att("html_name"), html_title: att("html_title"), html_class: att("html_class"), raw_form: att("output") == "raw_form" };
            let form = ContactForm { locale: "en_US".into(), form: template.clone(), additional_settings: settings, ..ContactForm::default() };
            let place = Place { request_uri: "/contact/?ref=golden", home: "http://localhost:8080", loop_post: None, html_type: "text/html", count: 1, nonce: None, logged_in: false, version: super::VERSION };
            let tr = |s: &str| s.to_owned();
            let hash = |_: &str| "HASH".to_owned();
            let query = vec![("ref".to_owned(), "golden".to_owned())];
            let mut used = Vec::new();
            let mut random = |_: usize| 0;
            let mut render = Render {
                tr: &tr,
                quiz_hash: &hash,
                query: &query,
                user: None,
                post_meta: None,
                shortcode_atts: &atts,
                template: &template,
                today: chrono::NaiveDate::from_ymd_opt(2026, 10, 31).unwrap(),
                used_ids: &mut used,
                random: &mut random,
                responses: 0,
            };
            let actual = form_html(&form, &options, &place, &mut render);
            cases += 1;
            if actual != expected {
                let at = actual.bytes().zip(expected.bytes()).position(|(a, b)| a != b).unwrap_or(actual.len().min(expected.len()));
                let from = at.saturating_sub(60);
                failures.push(format!(
                    "{name} (byte {at}):\n  expected …{:?}\n  actual   …{:?}",
                    expected.get(from..(at + 100).min(expected.len())).unwrap_or(""),
                    actual.get(from..(at + 100).min(actual.len())).unwrap_or("")
                ));
            }
        }
        assert!(cases > 0, "no golden cases");
        assert!(failures.is_empty(), "{} of {cases} differ:\n{}", failures.len(), failures.join("\n"));
    }
}
