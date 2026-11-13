//! `cmplz_get_option()`: the option `cmplz_options` over the defaults of the fields (`COMPLIANZ::$config->fields`,
//! after the `cmplz_default_value` filter `cmplz_set_default()`), and the small helpers of `functions.php` that only
//! read options.

use rustpress_core::Site;
use rustpress_phpser::{Key, Value};

pub(super) fn s(v: &str) -> Value {
    Value::Str(v.as_bytes().to_vec())
}

fn key_string(key: &Key) -> String {
    match key {
        Key::Int(n) => n.to_string(),
        Key::Str(b) => String::from_utf8_lossy(b).into_owned(),
    }
}

/// `(string) $value`.
pub(super) fn php_string(value: &Value) -> String {
    match value {
        Value::Str(b) => String::from_utf8_lossy(b).into_owned(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => rustpress_blocks::general::php_float(*f),
        Value::Bool(true) => "1".to_owned(),
        _ => String::new(),
    }
}

/// PHP's truthiness.
pub(super) fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(b) => !(b.is_empty() || b == b"0"),
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

/// `$a == $b` for a value and a string (the loose comparisons of the plugin: `== 1`, `!= 1`).
pub(super) fn loose_eq_int(value: &Value, n: i64) -> bool {
    match value {
        Value::Int(v) => *v == n,
        Value::Bool(b) => *b == (n != 0),
        Value::Float(f) => *f == n as f64,
        Value::Str(b) => String::from_utf8_lossy(b).trim().parse::<f64>().is_ok_and(|f| f == n as f64),
        Value::Null => n == 0,
        _ => false,
    }
}

/// The strings of an array value (`is_array( $v ) ? $v : array()`), in order.
pub(super) fn strings(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().map(|(_, v)| php_string(v)).collect(),
        _ => Vec::new(),
    }
}

/// The value of `key` of an array (`$value[ $key ] ?? null`).
pub(super) fn item<'v>(value: &'v Value, key: &str) -> Option<&'v Value> {
    match value {
        Value::Array(items) => items.iter().find(|(k, _)| key_string(k) == key).map(|(_, v)| v),
        _ => None,
    }
}

/// The default of a field as the fields of `settings/config/fields` define it; `T` marks a text WordPress translates
/// (`__( ..., 'complianz-gdpr' )`).
enum Default {
    Str(&'static str),
    T(&'static str),
    False,
    True,
    Int(i64),
}

/// The defaults of the fields the website reads (`'default'` of each field, `null` reads as `false`).
const DEFAULTS: &[(&str, Default)] = &[
    ("regions", Default::False),
    ("other_region_behaviour", Default::Str("none")),
    ("eu_consent_regions", Default::Str("no")),
    ("uk_consent_regions", Default::Str("no")),
    ("ca_targets_quebec", Default::Str("")),
    ("us_states", Default::False),
    ("us_california_optin", Default::Str("no")),
    ("wp_admin_access_users", Default::Str("no")),
    ("cookie-statement", Default::Str("generated")),
    ("privacy-statement", Default::Str("custom")),
    ("impressum", Default::Str("none")),
    ("disclaimer", Default::Str("none")),
    ("country_company", Default::Str("NL")),
    ("records_of_consent", Default::Str("no")),
    ("datarequest", Default::Str("no")),
    ("respect_dnt", Default::Str("no")),
    ("sensitive_information_processed", Default::Str("")),
    ("compile_statistics", Default::Str("")),
    ("compile_statistics_more_info", Default::Str("")),
    ("compile_statistics_more_info_tag_manager", Default::Str("")),
    ("compile_statistics_more_info_gtg", Default::Str("")),
    ("matomo_anonymized", Default::Str("")),
    ("consent_for_anonymous_stats", Default::Str("yes")),
    ("configuration_by_complianz", Default::Str("yes")),
    ("consent-mode", Default::Str("no")),
    ("gtag-basic-consent-mode", Default::Str("no")),
    ("cmplz-gtag-urlpassthrough", Default::Str("no")),
    ("cmplz-gtag-ads_data_redaction", Default::Str("no")),
    ("cmplz-tm-template", Default::Str("no")),
    ("ua_code", Default::Str("")),
    ("additional_gtags_stats", Default::False),
    ("gtm_code", Default::Str("")),
    ("gtm_code_head", Default::Str("no")),
    ("aw_code", Default::Str("")),
    ("additional_gtags_marketing", Default::False),
    ("matomo_url", Default::False),
    ("matomo_site_id", Default::Str("")),
    ("matomo_tag_url", Default::False),
    ("matomo_container_id", Default::Str("")),
    ("clicky_site_id", Default::Str("")),
    ("yandex_id", Default::Str("")),
    ("clarity_consent_mode", Default::Str("yes")),
    ("clarity_id", Default::Str("")),
    ("yandex_ecommerce", Default::Str("no")),
    ("consent_per_service", Default::Str("no")),
    ("uses_thirdparty_services", Default::Str("no")),
    ("thirdparty_services_on_site", Default::Str("")),
    ("block_recaptcha_service", Default::Str("no")),
    ("self_host_google_fonts", Default::Str("no")),
    ("block_hubspot_service", Default::Str("no")),
    ("hotjar_privacyfriendly", Default::Str("")),
    ("uses_social_media", Default::Str("no")),
    ("socialmedia_on_site", Default::Str("")),
    ("uses_firstparty_marketing_cookies", Default::Str("no")),
    ("uses_ad_cookies", Default::Str("no")),
    ("uses_ad_cookies_personalized", Default::Str("no")),
    ("enable_amazon_consent_signal", Default::Str("no")),
    ("uses_wordpress_comments", Default::Str("no")),
    ("block_wordpress_comment_cookies", Default::Str("yes")),
    ("enable_cookie_banner", Default::False),
    ("enable_cookie_blocker", Default::False),
    ("cookie_expiry", Default::Int(365)),
    ("enable_cookieblocker_ajax", Default::False),
    ("set_cookies_on_root", Default::False),
    ("cookie_domain", Default::False),
    ("enable_iapi_navigation", Default::False),
    ("a_b_testing", Default::False),
    ("dont_use_placeholders", Default::False),
    ("blocked_content_text", Default::T("Click to accept {category} cookies and enable this content")),
    ("blocked_content_text_per_service", Default::T("Click 'I agree' to enable {service}")),
    ("agree_text_per_service", Default::T("I agree")),
    ("placeholder_style", Default::Str("minimal")),
    ("google-maps-format", Default::Str("1280x920")),
    ("safe_mode", Default::False),
    ("use_cdb_api", Default::Str("yes")),
    ("use_cdb_links", Default::Str("yes")),
    ("organisation_name", Default::Str("")),
    ("address_company", Default::Str("")),
    ("email_company", Default::Str("")),
    ("telephone_company", Default::Str("")),
    ("purpose_personaldata", Default::Str("")),
    ("use_document_css", Default::True),
    ("use_custom_document_css", Default::False),
];

/// The options of the plugin as the website reads them.
pub struct Options {
    stored: Vec<(String, Value)>,
    /// `get_locale()`.
    locale: String,
    /// The options the scans of the plugin wrote (`cmplz_detected_stats`, `cmplz_detected_social_media`,
    /// `cmplz_detected_thirdparty_services`).
    detected_stats: Vec<String>,
    detected_social_media: Vec<String>,
    detected_thirdparty: Vec<String>,
    /// `cmplz_get_transient( 'cmplz_blocked_scripts' )` is not empty.
    cached_blocked_scripts: bool,
    translate: Box<dyn Fn(&str) -> String + Send + Sync>,
    translate_x: Box<dyn Fn(&str, &str) -> String + Send + Sync>,
    translate_n: Box<dyn Fn(&str, &str, i64) -> String + Send + Sync>,
    /// `__( $text )` of the default text domain.
    translate_core: Box<dyn Fn(&str) -> String + Send + Sync>,
}

/// `get_option( $name )`, also of an option that is not autoloaded; `None` when it does not exist.
pub(super) async fn option(site: &Site, name: &str) -> Option<Value> {
    site.options.fetch(&site.db, name).await.ok().flatten().map(|raw| rustpress_core::options::maybe_unserialize(&raw))
}

/// The values of a scanned list (`array_filter()` of the option, a string as a list of one).
async fn detected(site: &Site, name: &str) -> Vec<String> {
    match option(site, name).await {
        Some(Value::Array(items)) => items.iter().map(|(_, v)| v).filter(|v| truthy(v)).map(php_string).collect(),
        Some(v) if truthy(&v) => vec![php_string(&v)],
        _ => Vec::new(),
    }
}

impl Options {
    pub async fn load(site: &Site, cached_blocked_scripts: bool) -> Self {
        let stored = match option(site, "cmplz_options").await {
            Some(Value::Array(items)) => items.iter().map(|(k, v)| (key_string(k), v.clone())).collect(),
            _ => Vec::new(),
        };
        let i18n = site.i18n();
        Self {
            stored,
            locale: site.locale(),
            detected_stats: detected(site, "cmplz_detected_stats").await,
            detected_social_media: detected(site, "cmplz_detected_social_media").await,
            detected_thirdparty: detected(site, "cmplz_detected_thirdparty_services").await,
            cached_blocked_scripts,
            translate: Box::new({
                let i18n = i18n.clone();
                move |text| i18n.gettext("complianz-gdpr", text)
            }),
            translate_x: Box::new({
                let i18n = i18n.clone();
                move |text, context| i18n.pgettext("complianz-gdpr", context, text)
            }),
            translate_n: Box::new({
                let i18n = i18n.clone();
                move |single, plural, n| i18n.ngettext("complianz-gdpr", single, plural, n)
            }),
            translate_core: Box::new(move |text| i18n.gettext("default", text)),
        }
    }

    /// Options from the stored values alone (untranslated, `en_US`), for the golden tests.
    #[cfg(test)]
    pub fn from_values(stored: Vec<(String, Value)>) -> Self {
        Self {
            stored,
            locale: "en_US".into(),
            detected_stats: Vec::new(),
            detected_social_media: Vec::new(),
            detected_thirdparty: Vec::new(),
            cached_blocked_scripts: false,
            translate: Box::new(str::to_owned),
            translate_x: Box::new(|text, _| text.to_owned()),
            translate_n: Box::new(|single, plural, n| if n == 1 { single.to_owned() } else { plural.to_owned() }),
            translate_core: Box::new(str::to_owned),
        }
    }

    /// `cmplz_get_option( $id )`: the stored value; a missing one or `false` takes the field's default.
    pub fn get(&self, id: &str) -> Value {
        let value = self.stored.iter().find(|(k, _)| k == id).map(|(_, v)| v.clone()).unwrap_or(Value::Bool(false));
        if !matches!(value, Value::Bool(false)) {
            return value;
        }
        let Some((_, default)) = DEFAULTS.iter().find(|(k, _)| *k == id) else { return value };
        let default = match default {
            Default::Str(v) => s(v),
            Default::T(v) => s(&(self.translate)(v)),
            Default::False => Value::Bool(false),
            Default::True => Value::Bool(true),
            Default::Int(n) => Value::Int(*n),
        };
        self.set_default(id, default)
    }

    /// `cmplz_get_option( $id, false )`: the stored value, `false` when there is none (no default).
    pub fn stored(&self, id: &str) -> Value {
        self.stored.iter().find(|(k, _)| k == id).map(|(_, v)| v.clone()).unwrap_or(Value::Bool(false))
    }

    /// `cmplz_set_default()`, the `cmplz_default_value` filter, for the fields the website reads.
    fn set_default(&self, id: &str, value: Value) -> Value {
        let list = |items: &[String]| Value::Array(items.iter().enumerate().map(|(i, v)| (Key::Int(i as i64), s(v))).collect());
        match id {
            "compile_statistics" if !self.detected_stats.is_empty() => s(&self.detected_stats[0]),
            "country_company" => {
                let code = self.locale.get(3..5).unwrap_or_default();
                if super::regions::COUNTRIES.contains(&code) { s(code) } else { value }
            }
            "uses_social_media" if !self.detected_social_media.is_empty() => s("yes"),
            "socialmedia_on_site" if !self.detected_social_media.is_empty() => list(&self.detected_social_media),
            "uses_thirdparty_services" if !self.detected_thirdparty.is_empty() || self.cached_blocked_scripts => s("yes"),
            "thirdparty_services_on_site" if !self.detected_thirdparty.is_empty() => list(&self.detected_thirdparty),
            _ => value,
        }
    }

    /// `cmplz_get_option( $id )` as a string.
    pub fn string(&self, id: &str) -> String {
        php_string(&self.get(id))
    }

    /// `cmplz_get_option( $id ) === $value` for a string.
    pub fn is(&self, id: &str, value: &str) -> bool {
        matches!(self.get(id), Value::Str(b) if b == value.as_bytes())
    }

    /// The truthiness of `cmplz_get_option( $id )`.
    pub fn truthy(&self, id: &str) -> bool {
        truthy(&self.get(id))
    }

    /// `cmplz_get_regions()`: the selected regions (a single string in the free plugin), empty ones dropped.
    pub fn regions(&self) -> Vec<String> {
        match self.get("regions") {
            Value::Array(items) => items.iter().map(|(_, v)| v).filter(|v| truthy(v)).map(php_string).collect(),
            v if truthy(&v) => vec![php_string(&v)],
            _ => Vec::new(),
        }
    }

    /// `cmplz_has_region( $code )` (`cmplz_get_regions( true )` also lists `all`).
    pub fn has_region(&self, code: &str) -> bool {
        code == "all" || self.regions().iter().any(|r| r == code)
    }

    /// `cmplz_uses_thirdparty( $name )`.
    pub fn uses_thirdparty(&self, name: &str) -> bool {
        if self.is("uses_thirdparty_services", "yes") && in_array(&self.get("thirdparty_services_on_site"), name) {
            return true;
        }
        self.is("uses_social_media", "yes") && in_array(&self.get("socialmedia_on_site"), name)
    }

    /// `cmplz_consent_mode()`.
    pub fn consent_mode(&self) -> bool {
        self.is("consent-mode", "yes")
    }

    /// `cmplz_uses_marketing_cookies()`.
    pub fn uses_marketing_cookies(&self) -> bool {
        self.is("uses_ad_cookies", "yes") || self.is("uses_firstparty_marketing_cookies", "yes") || self.is("uses_thirdparty_services", "yes") || self.is("uses_social_media", "yes")
    }

    /// `__( $text, 'complianz-gdpr' )`.
    pub fn tr(&self, text: &str) -> String {
        (self.translate)(text)
    }

    /// `_x( $text, $context, 'complianz-gdpr' )`.
    pub fn tr_x(&self, text: &str, context: &str) -> String {
        (self.translate_x)(text, context)
    }

    /// `_n( $single, $plural, $number, 'complianz-gdpr' )`.
    pub fn tr_n(&self, single: &str, plural: &str, n: i64) -> String {
        (self.translate_n)(single, plural, n)
    }

    /// `__( $text )` of WordPress itself.
    pub fn tr_core(&self, text: &str) -> String {
        (self.translate_core)(text)
    }

    /// `in_array( $value, cmplz_get_option( $id ) )` of an option that holds a list.
    pub fn in_list(&self, id: &str, value: &str) -> bool {
        in_array(&self.get(id), value)
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }
}

/// `in_array( $name, $list )` with `is_array()` first (loose: strings only here).
fn in_array(list: &Value, name: &str) -> bool {
    match list {
        Value::Array(items) => items.iter().any(|(_, v)| php_string(v) == name),
        _ => false,
    }
}
