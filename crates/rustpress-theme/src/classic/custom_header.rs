//! The custom header of a ported classic theme: what it passes to `add_theme_support( 'custom-header' )` and
//! `register_default_headers()`, and the functions of `wp-includes/theme.php` that read it with the theme mods.

use rustpress_core::Site;
use rustpress_core::formatting::esc_url_raw;
use rustpress_phpser::{Key, Value};

use crate::Result;

/// `get_theme_support( 'custom-header' )` after `add_theme_support()` merged the defaults.
#[derive(Debug, Clone)]
pub struct Support {
    pub default_image: String,
    pub width: u64,
    pub height: u64,
    pub flex_width: bool,
    pub flex_height: bool,
    pub uploads: bool,
    pub header_text: bool,
    pub default_text_color: String,
    pub random_default: bool,
    pub max_width: u64,
    /// `video`: the header can show a video, and the Customizer calls it header media.
    pub video: bool,
}

/// `add_theme_support( 'custom-header', ... )` of the theme; `template_uri` is `get_template_directory_uri()`.
pub fn support(template: &str, template_uri: &str) -> Option<Support> {
    match template {
        // `twentyseventeen_custom_header_setup()`.
        "twentyseventeen" => Some(Support {
            default_image: format!("{template_uri}/assets/images/header.jpg"),
            width: 2000,
            height: 1200,
            flex_width: false,
            flex_height: true,
            uploads: true,
            header_text: true,
            default_text_color: String::new(),
            random_default: false,
            max_width: 0,
            video: true,
        }),
        _ => None,
    }
}

/// One of `$_wp_default_headers`, with its addresses formatted (`Custom_Image_Header::process_default_headers()`).
#[derive(Debug, Clone)]
pub struct DefaultHeader {
    pub key: String,
    pub url: String,
    pub thumbnail_url: String,
    pub description: String,
}

impl DefaultHeader {
    /// The array `set_theme_mod( 'header_image_data', ... )` stores.
    pub fn to_php(&self) -> Value {
        let text = |s: &str| Value::Str(s.as_bytes().to_vec());
        Value::Array(vec![
            (key("url"), text(&esc_url_raw(&self.url))),
            (key("thumbnail_url"), text(&esc_url_raw(&self.thumbnail_url))),
            (key("description"), text(&self.description)),
        ])
    }
}

fn key(name: &str) -> Key {
    Key::Str(name.as_bytes().to_vec())
}

/// `register_default_headers()` of the theme, formatted with the directories of the template and the stylesheet.
pub fn default_headers(site: &Site, template: &str, template_uri: &str) -> Vec<DefaultHeader> {
    match template {
        "twentyseventeen" => {
            let url = format!("{template_uri}/assets/images/header.jpg");
            vec![DefaultHeader {
                key: "default-image".into(),
                url: url.clone(),
                thumbnail_url: url,
                description: site.i18n().gettext("twentyseventeen", "Default Header Image"),
            }]
        }
        _ => Vec::new(),
    }
}

/// One of `get_uploaded_header_images()`.
#[derive(Debug, Clone)]
pub struct Uploaded {
    pub attachment_id: u64,
    pub url: String,
    pub alt_text: String,
    pub attachment_parent: Value,
    pub width: Option<Value>,
    pub height: Option<Value>,
}

impl Uploaded {
    /// The array `set_theme_mod( 'header_image_data', ... )` stores.
    pub fn to_php(&self) -> Value {
        let text = |s: &str| Value::Str(s.as_bytes().to_vec());
        let mut items = vec![
            (key("attachment_id"), Value::Int(self.attachment_id as i64)),
            (key("url"), text(&esc_url_raw(&self.url))),
            (key("thumbnail_url"), text(&esc_url_raw(&self.url))),
            (key("alt_text"), text(&self.alt_text)),
            (key("attachment_parent"), self.attachment_parent.clone()),
        ];
        if let Some(width) = &self.width {
            items.push((key("width"), width.clone()));
        }
        if let Some(height) = &self.height {
            items.push((key("height"), height.clone()));
        }
        Value::Array(items)
    }
}

/// `get_uploaded_header_images()`: the attachments marked for the stylesheet, by ID.
pub async fn uploaded_header_images(site: &Site, stylesheet: &str) -> Result<Vec<Uploaded>> {
    let mut out = Vec::new();
    for id in site.db.custom_header_attachments(stylesheet).await? {
        let Some(post) = site.db.post(id).await? else { continue };
        let meta = site.db.post_meta(id).await?;
        let value = |name: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(name)).and_then(|m| m.meta_value.clone());
        let url = match value("_wp_attached_file").filter(|file| !file.is_empty()) {
            Some(file) => rustpress_blocks::images::attachment_url(site, &file),
            None => post.guid.clone(),
        };
        let data = value("_wp_attachment_metadata").map(|raw| rustpress_core::options::maybe_unserialize(&raw)).unwrap_or(Value::Null);
        out.push(Uploaded {
            attachment_id: id,
            url: esc_url_raw(&url),
            alt_text: value("_wp_attachment_image_alt").unwrap_or_default(),
            attachment_parent: data.get("attachment_parent").filter(|v| !matches!(v, Value::Null)).cloned().unwrap_or(Value::Str(Vec::new())),
            width: data.get("width").filter(|v| !matches!(v, Value::Null)).cloned(),
            height: data.get("height").filter(|v| !matches!(v, Value::Null)).cloned(),
        });
    }
    Ok(out)
}

/// A string value of the theme mods, `None` when it is not set (`isset()`).
pub fn mod_value<'v>(mods: &'v Value, name: &str) -> Option<&'v Value> {
    mods.get(name).filter(|v| !matches!(v, Value::Null))
}

fn php_string(value: &Value) -> Option<String> {
    match value {
        Value::Str(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Value::Int(n) => Some(n.to_string()),
        _ => None,
    }
}

/// PHP's `empty()` of a value.
fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) | Value::Int(0) => true,
        Value::Float(f) => *f == 0.0,
        Value::Str(bytes) => bytes.is_empty() || bytes == b"0",
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

/// `get_theme_mod( 'header_image', get_theme_support( 'custom-header', 'default-image' ) )`.
pub fn header_image_mod(mods: &Value, support: &Support) -> Value {
    mod_value(mods, "header_image").cloned().unwrap_or_else(|| Value::Str(support.default_image.as_bytes().to_vec()))
}

/// `get_header_textcolor()`.
pub fn header_textcolor(mods: &Value, support: &Support) -> String {
    mod_value(mods, "header_textcolor").and_then(php_string).unwrap_or_else(|| support.default_text_color.clone())
}

/// `display_header_text()`.
pub fn display_header_text(mods: &Value, support: &Support) -> bool {
    support.header_text && header_textcolor(mods, support) != "blank"
}

/// What `_get_random_header_data()` picked.
#[derive(Debug, Clone)]
pub struct RandomHeader {
    pub url: String,
    /// The attachment of an uploaded header; 0 for one of the theme.
    pub attachment_id: u64,
    pub width: Option<Value>,
    pub height: Option<Value>,
}

/// `_get_random_header_data()`: `array_rand()` picks at random, and so does this. WordPress keeps the pick in a static
/// variable, so a request asks once and hands it to the functions below.
pub fn random_header(mods: &Value, support: &Support, defaults: &[DefaultHeader], uploaded: &[Uploaded]) -> Option<RandomHeader> {
    let stored = mod_value(mods, "header_image").and_then(php_string).unwrap_or_default();
    let candidates: Vec<RandomHeader> = if stored == "random-uploaded-image" {
        uploaded.iter().map(|u| RandomHeader { url: u.url.clone(), attachment_id: u.attachment_id, width: u.width.clone(), height: u.height.clone() }).collect()
    } else if stored == "random-default-image" || support.random_default {
        defaults.iter().map(|d| RandomHeader { url: d.url.clone(), attachment_id: 0, width: None, height: None }).collect()
    } else {
        Vec::new()
    };
    if candidates.is_empty() {
        return None;
    }
    let pick = rand::random_range(0..candidates.len());
    candidates.into_iter().nth(pick)
}

/// `get_random_header_image()`.
fn random_header_image(random: Option<&RandomHeader>) -> String {
    random.map(|r| r.url.clone()).unwrap_or_default()
}

/// `is_random_header_image( $type )`: `any`, `default` or `uploaded`.
pub fn is_random_header_image(kind: &str, mods: &Value, support: &Support, random: Option<&RandomHeader>) -> bool {
    let stored = header_image_mod(mods, support);
    let text = php_string(&stored).unwrap_or_default();
    let fallback = || is_empty(&stored) && !random_header_image(random).is_empty();
    match kind {
        "any" => text == "random-default-image" || text == "random-uploaded-image" || fallback(),
        _ => text == format!("random-{kind}-image") || (kind == "default" && fallback()),
    }
}

/// `set_url_scheme()` with the scheme of the site.
fn set_url_scheme(site: &Site, url: &str) -> String {
    let scheme = if site.settings.siteurl.starts_with("https://") { "https" } else { "http" };
    let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")).or_else(|| url.strip_prefix("//"));
    rest.map_or(url.to_owned(), |rest| format!("{scheme}://{rest}"))
}

/// `get_header_image()`: `None` for `false`.
pub fn header_image(site: &Site, mods: &Value, support: &Support, random: Option<&RandomHeader>) -> Option<String> {
    let stored = header_image_mod(mods, support);
    let mut url = match &stored {
        Value::Str(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        // `is_string()` fails for anything else.
        _ if !is_random_header_image("any", mods, support, random) => return None,
        _ => String::new(),
    };
    if url == "remove-header" {
        return None;
    }
    if is_random_header_image("any", mods, support, random) {
        url = random_header_image(random);
    }
    Some(esc_url_raw(&set_url_scheme(site, url.trim())))
}

/// The width and height of `get_custom_header()`.
pub fn custom_header_size(mods: &Value, support: &Support, random: Option<&RandomHeader>) -> (u64, u64) {
    let data = mod_value(mods, "header_image_data").filter(|data| !is_empty(data));
    let picked = is_random_header_image("any", mods, support, random);
    let prop = |name: &str| -> Option<&Value> {
        if picked {
            let random = random?;
            return if name == "width" { random.width.as_ref() } else { random.height.as_ref() };
        }
        match data? {
            Value::Object { props, .. } => props.iter().find_map(|(k, v)| (*k == key(name)).then_some(v)),
            other @ Value::Array(_) => other.get(name),
            _ => None,
        }
    };
    let absint = |value: Option<&Value>, default: u64| match value {
        None => default,
        Some(Value::Int(n)) => n.unsigned_abs(),
        Some(Value::Float(f)) => f.trunc().abs() as u64,
        Some(Value::Str(bytes)) => String::from_utf8_lossy(bytes).trim().parse::<f64>().map_or(0, |f| f.trunc().abs() as u64),
        Some(Value::Bool(true)) => 1,
        Some(_) => 0,
    };
    (absint(prop("width"), support.width), absint(prop("height"), support.height))
}
