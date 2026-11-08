//! WPForms Lite 2.0.4 (`wpforms-lite/wpforms.php`): forms stored as posts of the type `wpforms` (the form data as
//! JSON in `post_content`), printed by the shortcode `[wpforms]` and the block `wpforms/form-selector`, and sent
//! through `admin-ajax.php` (`wpforms_submit`).

pub mod ajax;
pub mod css_vars;
pub mod dom;
pub mod fields;
#[cfg(test)]
mod golden;
pub mod inline;
pub mod notify;
pub mod page;
pub mod php;
pub mod punycode;
pub mod render;
pub mod settings;
pub mod smart_tags;
pub mod template;
pub mod submit;
pub mod token;

pub(crate) use crate::redirection::php as redirection_php;

/// `wp_send_json()`: `json_encode()` without flags.
fn encode(value: &Value) -> String {
    rustpress_blocks::php_json::encode(value)
}

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, BlockContext, NativePlugin, ShortcodeContext};
use rustpress_core::shortcodes::Atts;
use rustpress_core::{Result, Site};
use rustpress_db::{Order, OrderBy, PostQuery};
use serde_json::Value;

use render::{Env, Form};
use settings::Settings;

/// `WPFORMS_VERSION`.
pub const VERSION: &str = "2.0.4";

const DOMAIN: &str = "wpforms-lite";

/// The plugin file.
const FILE: &str = "wpforms-lite/wpforms.php";

pub struct WpForms;

impl NativePlugin for WpForms {
    fn file(&self) -> &'static str {
        FILE
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some(DOMAIN)
    }

    fn activate<'a>(&'a self, site: &'a Site, writer: &'a rustpress_db::Writer, _user: u64, _now: i64) -> BoxFuture<'a, Result<()>> {
        Box::pin(activate(site, writer))
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move {
            let query = PostQuery {
                post_types: vec!["wpforms".into()],
                statuses: ["publish", "future", "draft", "pending", "private", "trash", "auto-draft", "inherit"].map(String::from).to_vec(),
                order_by: OrderBy::Id,
                order: Order::Asc,
                limit: None,
                ..PostQuery::default()
            };
            let forms = site.db.posts(&query).await?.into_iter().map(|post| FormPost { id: post.id, status: post.post_status.clone(), date: post.post_date.clone(), data: decode(&post.post_content) }).collect();
            let settings = Settings::parse(site.options.fetch(&site.db, "wpforms_settings").await?.as_deref());
            let secret = token::secret_key(site.options.fetch(&site.db, "wpforms_crypto_secret_key").await?.as_deref());
            Ok(Box::new(Loaded { forms, settings, secret, themes: std::sync::OnceLock::new() }) as Box<dyn ActivePlugin>)
        })
    }
}

/// What the activation of the plugin stores that the forms need: the secret key of the tokens
/// (`Crypto::get_secret_key()`) and the Modern markup of `Frontend::init_style_settings()` for a site without forms.
async fn activate(site: &Site, writer: &rustpress_db::Writer) -> Result<()> {
    use base64::Engine;
    use rand::RngCore;
    if site.options.fetch(&site.db, "wpforms_crypto_secret_key").await?.is_none() {
        let mut key = [0u8; 32];
        rand::rng().fill_bytes(&mut key);
        writer.update_option("wpforms_crypto_secret_key", &base64::engine::general_purpose::STANDARD.encode(key)).await?;
    }
    let raw = site.options.fetch(&site.db, "wpforms_settings").await?;
    let settings = Settings::parse(raw.as_deref());
    if !settings.set("modern-markup-is-set") {
        let table = format!("{}posts", site.db.table_prefix());
        let counted = writer.column_u64(&format!("SELECT ID FROM {table} WHERE post_type = 'wpforms' AND post_status IN ('publish', 'trash') LIMIT 1")).await?;
        let mut pairs: Vec<(rustpress_phpser::Key, rustpress_phpser::Value)> = match raw.as_deref().map(|r| rustpress_phpser::unserialize(r.as_bytes())) {
            Some(Ok(rustpress_phpser::Value::Array(pairs))) => pairs,
            _ => Vec::new(),
        };
        let modern = if counted.is_empty() { "1" } else { "0" };
        let mut put = |key: &str, value: rustpress_phpser::Value| {
            let key = rustpress_phpser::Key::Str(key.as_bytes().to_vec());
            match pairs.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = value,
                None => pairs.push((key, value)),
            }
        };
        put("modern-markup", rustpress_phpser::Value::Str(modern.as_bytes().to_vec()));
        put("modern-markup-is-set", rustpress_phpser::Value::Bool(true));
        if modern == "1" {
            put("modern-markup-hide-setting", rustpress_phpser::Value::Bool(true));
        }
        writer.update_option("wpforms_settings", &String::from_utf8_lossy(&rustpress_phpser::serialize(&rustpress_phpser::Value::Array(pairs)))).await?;
    }
    Ok(())
}

/// `wpforms_decode()`: the JSON of `post_content` with `wp_unslash()` on its strings.
pub fn decode(content: &str) -> Value {
    fn unslash(value: &mut Value) {
        match value {
            Value::String(s) => *s = fields::stripslashes(s),
            Value::Array(list) => list.iter_mut().for_each(unslash),
            Value::Object(map) => map.values_mut().for_each(unslash),
            _ => {}
        }
    }
    if content.is_empty() {
        return Value::Bool(false);
    }
    match serde_json::from_str::<Value>(content) {
        Ok(mut value) => {
            unslash(&mut value);
            value
        }
        Err(_) => Value::Array(Vec::new()),
    }
}

/// A form post.
pub struct FormPost {
    pub id: u64,
    pub status: String,
    /// `post_date`.
    pub date: String,
    pub data: Value,
}

/// The plugin with the forms and settings of the site.
pub struct Loaded {
    pub forms: Vec<FormPost>,
    pub settings: Settings,
    pub secret: Vec<u8>,
    /// The themes of the block and the form settings, read once they are needed.
    themes: std::sync::OnceLock<css_vars::Themes>,
}

/// `time()`.
fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// A number below `n` for `array_rand()` and `shuffle()`.
fn random(n: usize) -> usize {
    use rand::Rng;
    if n == 0 { 0 } else { rand::rng().random_range(0..n) }
}

/// `remove_query_arg( 'wpforms' )` of the request: the query as `add_query_arg()` writes it again, rooted.
fn action(path: &str, query: &str) -> String {
    let kept: Vec<&str> = query
        .split('&')
        .filter(|part| {
            let key = part.split_once('=').map_or(*part, |(k, _)| k);
            let key = String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(key.replace('+', " ").as_bytes())).into_owned();
            key != "wpforms" && !key.starts_with("wpforms[")
        })
        .collect();
    crate::contact_form_7::request_uri(path, &kept.join("&"))
}

/// The smart tag `{page_title}` of a page without a submission (`PageTitle::get_value()`): the title of
/// `wp_title( '', false )` without the filters of other code, as `get_wp_title()` cleans it.
fn page_title(site: &Site, query: &rustpress_core::QueryResult, global_post: Option<&rustpress_db::Post>) -> String {
    use rustpress_core::formatting::{capital_p_dangit, convert_chars, esc_html, wp_strip_all_tags, wptexturize};
    let neutralize = |s: &str| esc_html(&wp_strip_all_tags(s));
    let flags = &query.flags;
    if flags.is_front_page {
        return neutralize(&if flags.is_page { global_post.map(|p| p.post_title.clone()).unwrap_or_default() } else { site.options.get("blogname").unwrap_or_default().to_owned() });
    }
    // `wp_title()`: the title of a single post, a term archive or the error page.
    let title = match &query.queried {
        rustpress_core::query::Queried::Post(post) if flags.is_singular() || flags.is_home => wp_strip_all_tags(&wptexturize(&post.post_title)),
        rustpress_core::query::Queried::Term(term) if flags.is_category || flags.is_tag || flags.is_tax => wp_strip_all_tags(&wptexturize(&term.name)),
        _ if flags.is_404 => site.tr("Page not found"),
        _ => String::new(),
    };
    neutralize(&capital_p_dangit(&esc_html(&convert_chars(&wptexturize(title.trim()))), false))
}

impl Loaded {
    /// `wpforms()->obj( 'form' )->get( $id )`.
    fn form(&self, id: u64) -> Option<&FormPost> {
        self.forms.iter().find(|f| f.id == id)
    }

    /// `Frontend::shortcode()` with the assets and the footer settings of the page.
    async fn shortcode_html(&self, atts: &Atts, ctx: &ShortcodeContext<'_>) -> Result<String> {
        let id = atts.get("id").unwrap_or_default();
        let title = render::validate_boolean(atts.get("title").unwrap_or_default());
        let description = render::validate_boolean(atts.get("description").unwrap_or_default());
        let id = if id.is_empty() || id == "0" { 0 } else { php::leading_int(id).max(0) as u64 };
        self.render(id, title, description, ctx, None).await
    }

    /// `Frontend::output()` of the form `id` on the page (with the CSS variables of `css_vars`), and what the page
    /// needs for it.
    async fn render(&self, id: u64, title: bool, description: bool, ctx: &ShortcodeContext<'_>, block: Option<&serde_json::Map<String, Value>>) -> Result<String> {
        let site = ctx.site;
        let form = self.form(id).filter(|f| !php::empty(Some(&f.data)) && id != 0);
        let now = now();
        let form_key = form.map(|f| php::str_of(&f.data, "id")).unwrap_or_default();
        let form_id = form.map(|f| php::int(f.data.get("id"))).unwrap_or(0);
        let tr = move |text: &str| site.trd(DOMAIN, text);
        let global = match ctx.global_post {
            Some(id) => site.db.post(id).await?,
            None => None,
        };
        let request_uri = if ctx.query.raw_query.is_empty() { ctx.query.url_path.clone() } else { format!("{}?{}", ctx.query.url_path, ctx.query.raw_query) };
        let home = site.home_url("");
        let origin = home.split_once("://").map(|(scheme, rest)| format!("{scheme}://{}", rest.split('/').next().unwrap_or_default())).unwrap_or_default();
        let current = rustpress_core::formatting::esc_url_raw(&format!("{origin}{request_uri}"));
        let decode = |s: &str| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.as_bytes())).into_owned();
        let page_url = rustpress_core::formatting::esc_url(&decode(&current));
        let referer = ctx.query.referer.as_deref().map(|r| rustpress_core::formatting::esc_url(&decode(r))).unwrap_or_default();
        let get: Vec<(String, String)> = rustpress_core::QueryVars::parse(&ctx.query.raw_query).iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect();
        let smart = |text: &str, _: &Value, _: &str| text.to_owned();
        let honeypot = form.map(|f| render::honeypot_choice(&f.data, &mut random));
        let names = ["Name", "Phone", "Comment", "Message", "Email", "Website"];
        // `FormSelector::add_class_callback()`: the block's own classes.
        let mut container_classes = Vec::new();
        if let Some(attr) = block {
            let class = attr.get("className").map(php::string).unwrap_or_default();
            if !class.is_empty() {
                container_classes.extend(class.split(' ').map(rustpress_core::formatting::esc_attr));
            }
            container_classes.push("wpforms-block".to_owned());
            let client = attr.get("clientId").map(php::string).unwrap_or_default();
            if !client.is_empty() {
                container_classes.push(format!("wpforms-block-{client}"));
            }
        }
        let env = Env {
            settings: &self.settings,
            tr: &tr,
            plugin_url: site.site_url("wp-content/plugins/wpforms-lite/"),
            action: action(&ctx.query.url_path, &ctx.query.raw_query),
            token: token::get(now, &form_key, &self.secret),
            token_time: now.to_string(),
            time_token: token::time_token(now, form_id.max(0), &self.secret),
            nonce: ctx.query.viewer.as_ref().map(|v| v.create_nonce(&format!("wpforms::form_{id}"))),
            page_title: page_title(site, ctx.query, global.as_ref()),
            page_url,
            url_referer: referer,
            singular: if ctx.query.flags.is_singular() { ctx.global_post } else { None },
            honeypot,
            legacy_honeypot_label: names[random(names.len())].to_owned(),
            smart_tags: &smart,
            get: &get,
            container_classes,
            submitted: None,
        };
        let form_ref = form.map(|f| Form { id: f.id, status: &f.status, data: &f.data });
        let themes = self.themes.get_or_init(|| css_vars::Themes::load(ctx.services.wp_root()));
        let rendered = match block {
            None => render::shortcode(form_ref.as_ref(), title, description, &env, themes),
            Some(attr) => {
                let first = {
                    let mut state = ctx.state.lock().expect("plugin state");
                    let done = state.lists.entry("wpforms_custom_css").or_default();
                    let first = !done.contains(&id.to_string());
                    done.push(id.to_string());
                    first
                };
                let data = form.map(|f| &f.data).unwrap_or(&Value::Null);
                let attr = css_vars::override_block_attributes(attr.clone(), data, themes);
                let css = css_vars::for_block(&attr, data, id, &self.settings, first);
                let mut r = render::output(form_ref.as_ref(), title, description, &env);
                r.html = format!("{css}{}", r.html);
                r
            }
        };
        let content = global.as_ref().map(|p| p.post_content.as_str()).unwrap_or_default();
        let head_css = ctx.query.flags.is_singular() && (rustpress_core::shortcodes::tags_in(content, &["wpforms"]).contains(&"wpforms") || content.contains("<!-- wp:wpforms/form-selector "));
        self.printed(rendered.printed.then_some((id, rendered.honeypot_field)), head_css, ctx);
        Ok(rendered.html)
    }

    /// Remembers a printed form (`Frontend::$forms`) and puts the assets and the footer settings of all forms of the
    /// page so far in the plugin state.
    fn printed(&self, form: Option<(u64, Option<i64>)>, head_css: bool, ctx: &ShortcodeContext<'_>) {
        let site = ctx.site;
        let mut state = ctx.state.lock().expect("plugin state");
        let list = state.lists.entry("wpforms_forms").or_default();
        if let Some((id, honeypot)) = form
            && !list.iter().any(|e| e.split(':').next() == Some(&id.to_string()))
        {
            list.push(format!("{id}:{}", honeypot.map(|h| h.to_string()).unwrap_or_default()));
        }
        let entries = list.clone();
        let printed: Vec<page::Printed<'_>> = entries
            .iter()
            .filter_map(|e| {
                let (id, hp) = e.split_once(':')?;
                let form = self.form(id.parse().ok()?)?;
                Some(page::Printed { id: form.id, data: &form.data, honeypot: hp.parse().ok() })
            })
            .collect();
        let tr = |text: &str| site.trd(DOMAIN, text);
        let viewer = ctx.query.viewer.as_ref();
        let page = page::Page {
            settings: &self.settings,
            plugin_url: &site.site_url("wp-content/plugins/wpforms-lite/"),
            tr: &tr,
            ajax_url: site.site_url("wp-admin/admin-ajax.php"),
            ssl: site.home_url("").starts_with("https://"),
            nonce: ctx.query.create_nonce("wpforms"),
            language: site.locale().split('_').next().unwrap_or("en").to_lowercase(),
            tracked: !viewer.is_some_and(|v| v.can("publish_posts")),
        };
        // `Frontend::assets_header()`: on a single post whose content has the shortcode or the block, the stylesheets
        // are enqueued for the head; otherwise `assets_footer()` prints them late, with the scripts.
        if head_css {
            state.flags.insert("wpforms_head_css");
        }
        let head = state.flags.contains("wpforms_head_css");
        let mut styles = if head || !printed.is_empty() { page::styles(&page, &printed) } else { Vec::new() };
        for style in &mut styles {
            style.in_footer = !head;
        }
        let ours = |a: &rustpress_core::native::Asset| a.handle.starts_with("wpforms");
        state.assets.styles.retain(|a| !ours(a));
        state.assets.scripts.retain(|a| !ours(a));
        state.assets.styles.extend(styles);
        if printed.is_empty() {
            return;
        }
        state.assets.scripts.extend(page::scripts(&page, &printed));
        let marker = "<script type='text/javascript'>\n/* <![CDATA[ */\nvar wpforms_settings = ";
        state.footer_scripts.retain(|s| !s.starts_with(marker) && !s.starts_with(MISSING_MARKER));
        if viewer.is_some_and(|v| v.can("manage_options")) {
            state.footer_scripts.push(page::missing_assets_error_js(&tr));
        }
        state.footer_scripts.push(page::footer_end(&page, &printed));
    }
}

/// The start of `Frontend::get_missing_assets_error_script()`.
const MISSING_MARKER: &str = "<script>\n\t\t\t\t( function() {\n\t\t\t\t\tfunction wpforms_js_error_loading()";

impl ActivePlugin for Loaded {
    fn shortcode_tags(&self) -> &'static [&'static str] {
        &["wpforms"]
    }

    fn block_names(&self) -> &'static [&'static str] {
        &["wpforms/form-selector"]
    }

    /// `FormSelector::get_form_html()`.
    fn render_block<'a>(&'a self, _name: &'a str, attrs: &'a serde_json::Map<String, Value>, ctx: &'a BlockContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let id = attrs.get("formId").filter(|v| !php::empty(Some(v))).map_or(0, |v| php::int(Some(v)).max(0) as u64);
            if id == 0 {
                return Ok(String::new());
            }
            let shortcode_ctx = ShortcodeContext {
                site: ctx.site,
                query: ctx.query,
                loop_post: ctx.post.map(|p| p.id),
                global_post: ctx.post.map(|p| p.id),
                in_the_content: false,
                block_theme: false,
                state: ctx.state,
                services: ctx.services,
            };
            let title = !php::empty(attrs.get("displayTitle"));
            let description = !php::empty(attrs.get("displayDesc"));
            self.render(id, title, description, &shortcode_ctx, Some(attrs)).await
        })
    }

    fn shortcode<'a>(&'a self, _tag: &'a str, atts: &'a Atts, _content: Option<&'a str>, ctx: &'a ShortcodeContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(self.shortcode_html(atts, ctx))
    }

    /// `wp_ajax_wpforms_submit`, `wp_ajax_wpforms_get_token` and `wp_ajax_wpforms_restricted_email`.
    fn admin_ajax<'a>(&'a self, action: &'a str, ctx: &'a rustpress_core::native::AdminContext<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::AjaxAnswer>>> {
        Box::pin(ajax::handle(self, action, ctx))
    }

    /// The same actions for visitors (`wp_ajax_nopriv_*`).
    fn admin_ajax_nopriv<'a>(&'a self, action: &'a str, ctx: &'a rustpress_core::native::AdminContext<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::AjaxAnswer>>> {
        Box::pin(ajax::handle(self, action, ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::render::{Env, Form, shortcode};
    use super::settings::Settings;
    use serde_json::Value;

    /// The forms `dev/wpforms-form-golden.sh` had the plugin print.
    #[test]
    fn golden_forms() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("wpforms-forms");
        let mut cases = 0;
        let mut failures = Vec::new();
        let mut names: Vec<_> = std::fs::read_dir(&dir).expect("golden directory").filter_map(|e| e.ok()?.path().file_name()?.to_str()?.strip_suffix(".json").map(str::to_owned)).collect();
        names.sort();
        for name in names {
            let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{name}.json"))).expect("case")).expect("json");
            let expected = std::fs::read_to_string(dir.join(format!("{name}.html"))).expect("expected output");
            let mut pairs = vec![("modern-markup".to_owned(), "1".to_owned())];
            if let Some(m) = case["wpforms_settings"].as_object() {
                for (k, v) in m {
                    pairs.retain(|(n, _)| n != k);
                    pairs.push((k.clone(), v.as_str().unwrap_or_default().to_owned()));
                }
            }
            let settings = Settings(pairs.into_iter().map(|(k, v)| (k, Value::String(v))).collect());
            let hp = std::fs::read_to_string(dir.join(format!("{name}.hp"))).ok();
            let honeypot = hp.as_deref().and_then(|s| s.split_once('|')).map(|(id, label)| (id.parse().unwrap_or(0), label.to_owned()));
            let legacy = std::fs::read_to_string(dir.join(format!("{name}.hp-legacy"))).unwrap_or_default();
            let mut data = case["form"].clone();
            data["id"] = Value::String("9000".into());
            let tr = |s: &str| s.to_owned();
            let smart = |s: &str, _: &Value, _: &str| s.to_owned();
            let env = Env {
                settings: &settings,
                tr: &tr,
                plugin_url: "http://localhost:8080/wp-content/plugins/wpforms-lite/".into(),
                action: "/golden/?ref=golden".into(),
                token: "TOKEN".into(),
                token_time: "TIME".into(),
                time_token: "TIMETOKEN".into(),
                nonce: None,
                page_title: String::new(),
                page_url: "http://localhost:8080/golden/?ref=golden".into(),
                url_referer: String::new(),
                singular: None,
                honeypot,
                legacy_honeypot_label: legacy,
                smart_tags: &smart,
                get: &[],
                container_classes: Vec::new(),
                submitted: None,
            };
            let form = Form { id: 9000, status: "publish", data: &data };
            let themes = super::css_vars::Themes { wpforms: Default::default(), custom: Default::default() };
            let actual = shortcode(Some(&form), case["title"].as_bool().unwrap_or(false), case["description"].as_bool().unwrap_or(false), &env, &themes).html;
            cases += 1;
            if actual != expected {
                let at = actual.bytes().zip(expected.bytes()).position(|(a, b)| a != b).unwrap_or(actual.len().min(expected.len()));
                let from = at.saturating_sub(80);
                failures.push(format!(
                    "{name} (byte {at}):\n  expected …{:?}\n  actual   …{:?}",
                    expected.get(from..(at + 120).min(expected.len())).unwrap_or(""),
                    actual.get(from..(at + 120).min(actual.len())).unwrap_or("")
                ));
            }
        }
        assert!(cases > 0, "no golden cases");
        assert!(failures.is_empty(), "{} of {cases} differ:\n{}", failures.len(), failures.join("\n"));
    }
}
