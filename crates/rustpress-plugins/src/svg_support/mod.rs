//! SVG Support 2.6.1: SVG uploads (sanitized), and in "advanced mode" a script that swaps `<img class="style-svg">`
//! for the inline SVG on the website.
//!
//! The settings live in the option `bodhi_svgs_settings`. The plugin writes its defaults into the option on every
//! load when they are missing (`sanitize_svg_front_end` on, uploads only for administrators, sanitizing for every
//! role); RustPress applies them in memory.

mod admin;
mod sanitizer;
mod upload;

use futures::future::BoxFuture;
use rustpress_blocks::php_json;
use rustpress_core::native::{ActivePlugin, AdminContext, AdminHelp, AdminPage, AjaxAnswer, Assets, MetaType, NativePlugin, RegisteredMeta, Uploader};
use rustpress_core::{Result, Site};
use rustpress_phpser::{Key, Value};

pub struct SvgSupport;

/// `BODHI_SVGS_VERSION`.
const VERSION: &str = "2.6.1";

impl NativePlugin for SvgSupport {
    fn file(&self) -> &'static str {
        "svg-support/svg-support.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move { Ok(Box::new(Active { settings: Settings::from_option(site.options.value("bodhi_svgs_settings")) }) as Box<dyn ActivePlugin>) })
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some(admin::DOMAIN)
    }
}

/// PHP's `empty()` for a value of the settings array.
fn php_empty(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null | Value::Bool(false)) => true,
        Some(Value::Int(n)) => *n == 0,
        Some(Value::Float(f)) => *f == 0.0,
        Some(Value::Str(s)) => s.is_empty() || s.as_slice() == b"0",
        Some(Value::Array(items)) => items.is_empty(),
        Some(_) => false,
    }
}

/// `$bodhi_svgs_options` after the plugin's defaults.
#[derive(Debug, Clone, Default)]
struct Settings {
    options: Vec<(String, Value)>,
}

impl Settings {
    fn from_option(value: Option<Value>) -> Self {
        let mut options: Vec<(String, Value)> = match value {
            Some(Value::Array(items)) => items
                .into_iter()
                .map(|(k, v)| {
                    let key = match k {
                        rustpress_phpser::Key::Int(n) => n.to_string(),
                        rustpress_phpser::Key::Str(s) => String::from_utf8_lossy(&s).into_owned(),
                    };
                    (key, v)
                })
                .collect(),
            _ => Vec::new(),
        };
        let list = |role: &str| Value::Array(vec![(Key::Int(0), Value::Str(role.as_bytes().to_vec()))]);
        // The defaults `svg-support.php` stores on every load, in its order.
        let current = |options: &[(String, Value)], key: &str| options.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        let snapshot = options.clone();
        let mut set = |key: &str, value: Value| match options.iter_mut().find(|(k, _)| k == key) {
            Some((_, slot)) => *slot = value,
            None => options.push((key.to_owned(), value)),
        };
        if current(&snapshot, "sanitize_svg_front_end").is_none_or(|v| matches!(v, Value::Null)) {
            set("sanitize_svg_front_end", Value::Str(b"on".to_vec()));
        }
        match current(&snapshot, "restrict") {
            None | Some(Value::Null) => set("restrict", list("administrator")),
            Some(v) if v.as_str() == Some("on") => set("restrict", list("administrator")),
            Some(v) if v.as_str() == Some("none") => set("restrict", list("none")),
            _ => {}
        }
        match current(&snapshot, "sanitize_on_upload_roles") {
            None | Some(Value::Null) => set("sanitize_on_upload_roles", Value::Array(Vec::new())),
            Some(v) if v.as_str() == Some("none") => set("sanitize_on_upload_roles", list("none")),
            _ => {}
        }
        Self { options }
    }

    pub(super) fn get(&self, key: &str) -> Option<&Value> {
        self.options.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn on(&self, key: &str) -> bool {
        !php_empty(self.get(key))
    }

    fn str(&self, key: &str) -> Option<String> {
        self.get(key).and_then(Value::as_str).map(str::to_owned)
    }

    /// `bodhi_svgs_advanced_mode()`.
    fn advanced_mode(&self) -> bool {
        self.on("advanced_mode")
    }

    /// The class that marks images to inline (`css_target`, else `style-svg`).
    fn css_target(&self) -> Option<String> {
        self.str("css_target").filter(|_| self.on("css_target"))
    }
}

struct Active {
    settings: Settings,
}

impl ActivePlugin for Active {
    fn admin_pages(&self, site: &Site) -> Vec<AdminPage> {
        admin::pages(site)
    }

    fn admin_page_help(&self, _slug: &str, site: &Site) -> AdminHelp {
        admin::help(site)
    }

    fn admin_enqueue(&self, ctx: &AdminContext<'_>, assets: &mut Assets) {
        admin::enqueue(ctx, &self.settings, assets);
    }

    fn admin_footer_text(&self, ctx: &AdminContext<'_>) -> Option<String> {
        admin::footer_text(ctx)
    }

    fn admin_page<'a>(&'a self, _slug: &'a str, ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        // `get_option( 'bodhi_svgs_settings' )` as the page reads it, after the defaults of this request.
        Box::pin(async move { Ok(admin::settings_page(ctx, &self.settings, &ctx.settings_fields("bodhi_svgs_settings_group"))) })
    }

    fn plugin_action_links(&self, site: &Site, links: Vec<String>) -> Vec<String> {
        admin::action_links(site, links)
    }

    fn plugin_row_meta(&self, site: &Site, file: &str, links: Vec<String>) -> Vec<String> {
        admin::row_meta(site, file, links)
    }

    /// `bodhi_svgs_block_editor()`: the checkbox for an inline featured image in the block editor.
    fn block_editor_assets(&self, ctx: &AdminContext<'_>, assets: &mut Assets) {
        if self.settings.advanced_mode() {
            // The plugin's URL ends with a slash and the path starts with one.
            let src = format!("{}/js/min/gutenberg-filters-min.js", ctx.site.site_url("wp-content/plugins/svg-support/"));
            assets.script("bodhi-svgs-gutenberg-filters", src, &["wp-edit-post"], VERSION, true);
        }
    }

    /// `bodhi_svgs_add_class_to_thumbnail()`: on a singular page, an inlined featured image gets the target class.
    fn post_thumbnail_html<'a>(&'a self, site: &'a Site, post_id: u64, singular: bool, html: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            if !self.settings.advanced_mode() || !singular {
                return Ok(html);
            }
            let rows = site.db.post_meta(post_id).await?;
            // `in_array( 1, get_post_meta( ... ) )`: a value that equals 1 loosely.
            let inline = rows.iter().filter(|m| m.meta_key.as_deref() == Some("inline_featured_image")).filter_map(|m| m.meta_value.as_deref()).any(|v| v.trim().parse::<f64>().is_ok_and(|n| n == 1.0));
            if !inline {
                return Ok(html);
            }
            let target = self.settings.str("css_target").filter(|c| !c.is_empty() && c != "0").unwrap_or_else(|| "style-svg".to_owned());
            Ok(html.replace("attachment-", &format!("{target} attachment-")))
        })
    }

    fn upload_mimes(&self, uploader: &Uploader<'_>, mimes: &mut Vec<(String, String)>) {
        upload::upload_mimes(&self.settings, uploader, mimes);
    }

    fn upload_prefilter(&self, uploader: &Uploader<'_>, name: &mut String, bytes: &mut Vec<u8>) -> std::result::Result<(), String> {
        upload::prefilter(uploader.site, &self.settings, uploader, name, bytes)
    }

    fn accept_filetype(&self, name: &str, mimes: &[(String, String)]) -> Option<(String, String)> {
        upload::accept_filetype(name, mimes)
    }

    fn attachment_metadata(&self, site: &Site, mime: &str, path: &std::path::Path, relative: &str, metadata: Value) -> Value {
        upload::metadata(site, mime, path, relative, metadata)
    }

    fn prepare_attachment_for_js(&self, mime: &str, path: Option<&std::path::Path>, response: &mut serde_json::Map<String, serde_json::Value>) {
        upload::prepare_attachment_for_js(mime, path, response);
    }

    /// `bodhi_svgs_display_thumbs()`: the output buffer of the media library.
    fn admin_page_output(&self, ctx: &AdminContext<'_>, html: String) -> String {
        if ctx.hook_suffix == "upload.php" { upload::media_templates(html) } else { html }
    }

    fn registered_settings(&self) -> Vec<(&'static str, &'static str)> {
        vec![("bodhi_svgs_settings_group", "bodhi_svgs_settings")]
    }

    fn sanitize_setting(&self, _option: &str, value: Option<Value>) -> Option<Value> {
        admin::sanitize(value)
    }

    fn admin_ajax<'a>(&'a self, action: &'a str, ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<Option<AjaxAnswer>>> {
        Box::pin(async move {
            match action {
                "bodhi_svgs_autosave" => Ok(Some(admin::autosave(ctx).await?)),
                _ => Ok(None),
            }
        })
    }

    /// `bodhi_svgs_register_meta()`: whether the featured image is inlined, for every post type.
    fn registered_post_meta(&self) -> Vec<RegisteredMeta> {
        vec![RegisteredMeta { key: "inline_featured_image", post_type: None, kind: MetaType::Boolean }]
    }

    fn enqueue(&self, site: &Site, assets: &mut Assets) {
        let base = site.site_url("wp-content/plugins/svg-support");
        let settings = &self.settings;
        let in_footer = settings.on("js_foot_choice");
        // `bodhi_svgs_frontend_js()` at priority 9.
        if settings.str("sanitize_svg_front_end").as_deref() == Some("on") && settings.advanced_mode() {
            assets.script("bodhi-dompurify-library", format!("{base}/vendor/DOMPurify/DOMPurify.min.js"), &[], "2.5.9", in_footer).priority = 9;
        }
        // `bodhi_svgs_frontend_css()`.
        if settings.on("frontend_css") {
            assets.style("bodhi-svgs-attachment", format!("{base}/css/svgs-attachment.css"), &[], VERSION, "all");
        }
        // `bodhi_svgs_inline()`.
        if settings.advanced_mode() {
            let (bodhi, force) = match settings.css_target() {
                Some(class) => {
                    let class = rustpress_core::formatting::esc_attr(&class);
                    (format!("img.{class}"), class)
                }
                None => ("img.style-svg".to_owned(), "style-svg".to_owned()),
            };
            let (folder, suffix) = if settings.on("use_expanded_js") { ("", "") } else { ("min/", "-min") };
            let vanilla = settings.on("use_vanilla_js");
            let path = format!("js/{folder}svgs-inline{}{suffix}.js", if vanilla { "-vanilla" } else { "" });
            let deps: &[&str] = if vanilla { &[] } else { &["jquery"] };
            let script = assets.script("bodhi_svg_inline", format!("{base}/{path}"), deps, VERSION, in_footer);
            // `wp_localize_script()` turns the boolean into a string.
            let skip_nested = if settings.on("skip_nested_svg") { "1" } else { "" };
            script.extra = Some(format!("var svgSettings = {};", localize(&serde_json::json!({ "skipNested": skip_nested }))));
            let css_target = serde_json::json!({ "Bodhi": bodhi, "ForceInlineSVG": force });
            let force_active = if settings.on("force_inline_svg") { "true" } else { "false" };
            let sanitize = settings.get("sanitize_svg_front_end").map_or(serde_json::Value::Null, php_to_json);
            script.after.push(format!(
                "cssTarget={};ForceInlineSVGActive={};frontSanitizationEnabled={};",
                php_json::encode(&css_target),
                php_json::encode(&serde_json::Value::String(force_active.into())),
                php_json::encode(&sanitize)
            ));
        }
    }
}

/// `wp_json_encode( $l10n, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )` of `WP_Scripts::localize()`.
fn localize(value: &serde_json::Value) -> String {
    php_json::encode_with(value, php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E")
}

/// A scalar of the settings as `wp_json_encode()` sees it.
fn php_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::Value::from(*n),
        Value::Float(f) => serde_json::Value::from(*f),
        Value::Str(_) => serde_json::Value::String(value.as_str().unwrap_or_default().to_owned()),
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(serialized: &str) -> Settings {
        Settings::from_option(rustpress_phpser::unserialize(serialized.as_bytes()).ok())
    }

    #[test]
    fn front_end_sanitizing_is_on_by_default() {
        let s = settings("a:0:{}");
        assert_eq!(s.str("sanitize_svg_front_end").as_deref(), Some("on"));
        assert!(!s.advanced_mode());
    }

    #[test]
    fn empty_css_target_means_the_default_class() {
        let s = settings("a:2:{s:13:\"advanced_mode\";s:2:\"on\";s:10:\"css_target\";s:0:\"\";}");
        assert!(s.advanced_mode());
        assert_eq!(s.css_target(), None);
    }
}
