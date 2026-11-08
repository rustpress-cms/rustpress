//! What a page with forms gets besides the forms: the stylesheets and scripts of `Frontend::assets_css()`,
//! `Frontend::assets_js()` and the actions on `wpforms_wp_footer`, and the settings of the scripts that
//! `Frontend::footer_end()` prints (`var wpforms_settings`).

use rustpress_core::formatting::{esc_html, html_entity_decode};
use rustpress_core::native::Asset;
use serde_json::{Map, Value, json};

use super::fields::sanitize_key;
use super::php::{self, set, str_of};
use super::render::{disable_css, modern};
use super::settings::Settings;
use super::VERSION;

/// `wpforms_get_currencies()`: code, thousands separator, decimal separator, symbol, its position and the decimals.
const CURRENCIES: &[(&str, &str, &str, &str, &str, i64)] = &[
    ("USD", ",", ".", "&#36;", "left", 2),
    ("GBP", ",", ".", "&pound;", "left", 2),
    ("EUR", ".", ",", "&euro;", "right", 2),
    ("ARS", ".", ",", "&#36;", "left", 2),
    ("AUD", ",", ".", "&#36;", "left", 2),
    ("BRL", ".", ",", "R$", "left", 2),
    ("BGN", " ", ",", "лв", "right", 2),
    ("CAD", ",", ".", "&#36;", "left", 2),
    ("CRC", ",", ".", "&#8353;", "left", 2),
    ("XAF", ",", "", "F.CFA ", "left", 0),
    ("CLP", ".", "", "&#36;", "left", 0),
    ("COP", ".", "", "&#36;", "left", 0),
    ("CZK", ".", ",", "&#75;&#269;", "right", 2),
    ("DKK", ".", ",", "kr.", "right", 2),
    ("GHS", ",", ".", "GH&#8373;&nbsp;", "left", 2),
    ("HKD", ",", ".", "&#36;", "left", 2),
    ("HUF", ".", ",", "Ft", "right", 2),
    ("INR", ",", ".", "&#8377;", "left", 2),
    ("ILS", ",", ".", "&#8362;", "left", 2),
    ("JPY", ",", "", "&#165;", "left", 0),
    ("KES", ",", ".", "Ksh&nbsp;", "left", 2),
    ("MYR", ",", ".", "&#82;&#77;", "left", 2),
    ("MXN", ",", ".", "&#36;", "left", 2),
    ("NGN", ",", ".", "&#8358;", "left", 2),
    ("NOK", ".", ",", "Kr", "left", 2),
    ("NZD", ",", ".", "&#36;", "left", 2),
    ("PEN", ",", ".", "S/", "left", 2),
    ("PHP", ",", ".", "Php", "left", 2),
    ("PLN", ".", ",", "&#122;&#322;", "left", 2),
    ("RON", ",", ".", "lei", "right", 2),
    ("RUB", " ", ".", "pyб", "right", 2),
    ("SAR", ",", ".", "&#65020;", "right", 2),
    ("SGD", ",", ".", "&#36;", "left", 2),
    ("RSD", ",", ".", "din. ", "left", 2),
    ("ZAR", ",", ".", "R", "left", 2),
    ("KRW", ",", "", "&#8361;", "left", 0),
    ("LKR", ",", ".", "Rs ", "left", 2),
    ("SEK", ".", ",", "Kr", "right", 2),
    ("CHF", ",", ".", "CHF", "left", 2),
    ("TWD", ",", ".", "&#36;", "left", 2),
    ("THB", ",", ".", "&#3647;", "left", 2),
    ("TRY", ",", ".", "&#8378;", "left", 2),
    ("AED", ",", ".", "&#1583;.&#1573;", "right", 2),
    ("UYU", ".", ",", "$U", "left", 2),
];

/// `ReadOnlyField::read_only_frontend_strings()` of the fields that do not allow `read_only`.
const READ_ONLY_DISALLOWED: &[&str] = &["captcha", "map", "content", "divider", "hidden", "html", "entry-preview", "pagebreak", "payment-total"];

/// A form the page printed: its data and the field its honeypot took (`AntiSpam::$forms_data`).
pub struct Printed<'a> {
    pub id: u64,
    pub data: &'a Value,
    pub honeypot: Option<i64>,
}

/// What the page knows about the request.
pub struct Page<'a> {
    pub settings: &'a Settings,
    pub plugin_url: &'a str,
    /// `__( $text, 'wpforms-lite' )`.
    pub tr: &'a dyn Fn(&str) -> String,
    /// `admin_url( 'admin-ajax.php' )`.
    pub ajax_url: String,
    /// `is_ssl()`.
    pub ssl: bool,
    /// `wp_create_nonce( 'wpforms' )`.
    pub nonce: String,
    /// `wpforms_get_language_code()`.
    pub language: String,
    /// `Analytics::should_track_user()`: no signed-in user who may publish posts.
    pub tracked: bool,
}

/// `TextField::frontend_js()` and `Textarea::frontend_js()`: a field of the type with a length limit.
fn has_limit(forms: &[Printed<'_>], ty: &str) -> bool {
    forms.iter().flat_map(|f| f.data["fields"].as_object().into_iter().flat_map(|m| m.values())).any(|field| {
        str_of(field, "type") == ty && field.get("limit_enabled").is_some_and(|v| !v.is_null()) && (ty == "textarea" || set(field, "limit_count") || set(field, "limit_min"))
    })
}

/// `Select::is_field_style( $form, 'modern' )` for one of the forms.
fn has_modern_select(forms: &[Printed<'_>]) -> bool {
    forms.iter().flat_map(|f| f.data["fields"].as_object().into_iter().flat_map(|m| m.values())).any(|field| str_of(field, "type") == "select" && set(field, "style") && sanitize_key("modern") == str_of(field, "style"))
}

/// `wpforms_has_field_type( $types, $forms, true )`.
fn has_field_type(forms: &[Printed<'_>], types: &[&str]) -> bool {
    forms.iter().flat_map(|f| f.data["fields"].as_object().into_iter().flat_map(|m| m.values())).any(|field| types.contains(&str_of(field, "type").as_str()))
}

/// `wpforms_has_field_setting( 'input_mask', $forms, true )`.
fn has_input_mask(forms: &[Printed<'_>]) -> bool {
    forms.iter().flat_map(|f| f.data["fields"].as_object().into_iter().flat_map(|m| m.values())).any(|field| set(field, "input_mask"))
}

fn style(assets: &mut Vec<Asset>, handle: &str, src: String, ver: &str) {
    assets.push(Asset { handle: handle.to_owned(), src, ver: Some(ver.to_owned()), media: "all".into(), priority: 10, ..Asset::default() });
}

fn script(assets: &mut Vec<Asset>, handle: &str, src: String, deps: &[&str], ver: &str) -> usize {
    assets.push(Asset { handle: handle.to_owned(), src, deps: deps.iter().map(|d| (*d).to_owned()).collect(), ver: Some(ver.to_owned()), in_footer: true, priority: 10, ..Asset::default() });
    assets.len() - 1
}

/// The stylesheets of `wpforms_frontend_css` and `Frontend::assets_css()`.
pub fn styles(page: &Page<'_>, forms: &[Printed<'_>]) -> Vec<Asset> {
    let mut out = Vec::new();
    let url = page.plugin_url;
    // `Select::enqueue_frontend_css()`.
    if has_modern_select(forms) || page.settings.set("global-assets") {
        style(&mut out, "wpforms-choicesjs", format!("{url}assets/css/choices.min.css"), "10.2.0");
    }
    let disable = disable_css(page.settings);
    let engine = if modern(page.settings) { "modern" } else { "classic" };
    if disable == 3 {
        style(&mut out, "wpforms-no-styles", format!("{url}assets/css/frontend/wpforms-no-styles.min.css"), VERSION);
    } else {
        let name = if disable == 1 { "full" } else { "base" };
        let handle = format!("wpforms-{engine}-{name}");
        style(&mut out, &handle, format!("{url}assets/css/frontend/{engine}/wpforms-{name}.min.css"), VERSION);
        if engine == "modern" && name == "full" {
            out.last_mut().expect("just pushed").after.push(super::css_vars::root_css());
        }
    }
    out
}

/// The scripts of `wpforms_frontend_js`, `Frontend::assets_js()` and `wpforms_wp_footer`, all in the footer.
pub fn scripts(page: &Page<'_>, forms: &[Printed<'_>]) -> Vec<Asset> {
    let mut out = Vec::new();
    let url = page.plugin_url;
    let global = page.settings.set("global-assets");
    let engine = if modern(page.settings) { "modern" } else { "classic" };
    // `wpforms_frontend_js`: the text limit of the text fields and text areas, Choices.js of the modern selects.
    if has_limit(forms, "text") || has_limit(forms, "textarea") {
        script(&mut out, "wpforms-text-limit", format!("{url}assets/js/frontend/fields/text-limit.es5.min.js"), &["wpforms-validation"], VERSION);
    }
    if has_modern_select(forms) || global {
        let i = script(&mut out, "wpforms-choicesjs", format!("{url}assets/lib/choices.min.js"), &[], "10.2.0");
        let tr = |s: &str| esc_html(&(page.tr)(s));
        let config = json!({
            "removeItemButton": "1",
            "shouldSort": "",
            "fuseOptions": {"threshold": 0.1, "distance": 1000},
            "loadingText": tr("Loading..."),
            "noResultsText": tr("No results found"),
            "noChoicesText": tr("No choices to choose from"),
            "uniqueItemText": tr("Only unique values can be added"),
            "customAddItemText": tr("Only values matching specific conditions can be added"),
        });
        out[i].extra = Some(format!("var wpforms_choicesjs_config = {};", localize(&config)));
    }
    script(&mut out, "wpforms-validation", format!("{url}assets/lib/jquery.validate.min.js"), &["jquery"], "1.22.1");
    if global || has_field_type(forms, &["phone", "address"]) || has_input_mask(forms) {
        script(&mut out, "wpforms-maskedinput", format!("{url}assets/lib/jquery.inputmask.min.js"), &["jquery"], "5.0.9");
    }
    if global || has_field_type(forms, &["email"]) {
        script(&mut out, "wpforms-mailcheck", format!("{url}assets/lib/mailcheck.min.js"), &[], "1.1.2");
        script(&mut out, "wpforms-punycode", format!("{url}assets/lib/punycode.min.js"), &[], "1.0.0");
        script(&mut out, "wpforms-mailcheck-tlds", format!("{url}assets/js/frontend/mailcheck-tlds.min.js"), &[], VERSION);
    }
    script(&mut out, "wpforms-generic-utils", format!("{url}assets/js/share/utils.min.js"), &["jquery"], VERSION);
    script(&mut out, "wpforms", format!("{url}assets/js/frontend/wpforms.min.js"), &["jquery"], VERSION);
    if engine == "modern" {
        script(&mut out, "wpforms-modern", format!("{url}assets/js/frontend/wpforms-modern.min.js"), &["wpforms"], VERSION);
    }
    // `wpforms_wp_footer`: `Analytics::enqueue_footer_assets()` (10) and `Address\Frontend::assets_footer()` (15).
    if !forms.is_empty() && page.tracked {
        script(&mut out, "wpforms-analytics", format!("{url}assets/js/frontend/analytics.min.js"), &["jquery"], VERSION);
    }
    script(&mut out, "wpforms-address-field", format!("{url}assets/js/frontend/fields/address.min.js"), &["wpforms"], VERSION);
    out
}

/// `wp_localize_script()`: the scalars become strings, the rest stays as it is.
fn localize(value: &Value) -> String {
    rustpress_blocks::php_json::encode_with(value, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E")
}

/// `wpforms_setting( $key, $default )` as `get_strings()` uses it.
fn setting(page: &Page<'_>, key: &str, default: String) -> Value {
    Value::String(page.settings.string(key, &default))
}

/// `Frontend::get_strings()` with the filters of `wpforms_frontend_strings` of the Lite release.
pub fn strings(page: &Page<'_>, forms: &[Printed<'_>]) -> Map<String, Value> {
    let tr = |s: &str| esc_html(&(page.tr)(s));
    let sprintf = |template: &str, args: &[&str]| rustpress_core::i18n::sprintf(template, args);
    let mut s = Map::new();
    let mut put = |k: &str, v: Value| {
        s.insert(k.to_owned(), v);
    };
    put("val_required", setting(page, "validation-required", tr("This field is required.")));
    put("val_email", setting(page, "validation-email", tr("Please enter a valid email address.")));
    put("val_email_suggestion", setting(page, "validation-email-suggestion", sprintf(&tr("Did you mean %s?"), &["{suggestion}"])));
    put("val_email_suggestion_title", json!(rustpress_core::formatting::esc_attr(&(page.tr)("Click to accept this suggestion."))));
    put("val_email_restricted", setting(page, "validation-email-restricted", tr("This email address is not allowed.")));
    put("val_number", setting(page, "validation-number", tr("Please enter a valid number.")));
    put("val_number_positive", setting(page, "validation-number-positive", tr("Please enter a valid positive number.")));
    put("val_minimum_price", setting(page, "validation-minimum-price", tr("Amount entered is less than the required minimum.")));
    put("val_confirm", setting(page, "validation-confirm", tr("Field values do not match.")));
    put("val_checklimit", setting(page, "validation-check-limit", tr("You have exceeded the number of allowed selections: {#}.")));
    put("val_limit_characters", setting(page, "validation-character-limit", sprintf(&tr("%1$s of %2$s max characters."), &["{count}", "{limit}"])));
    put("val_limit_words", setting(page, "validation-word-limit", sprintf(&tr("%1$s of %2$s max words."), &["{count}", "{limit}"])));
    put("val_limit_min_characters", setting(page, "validation-character-minimum", sprintf(&tr("Minimum is %1$s characters. Characters needed: %2$s."), &["{min}", "{needed}"])));
    put("val_limit_min_words", setting(page, "validation-word-minimum", sprintf(&tr("Minimum is %1$s words. Words needed: %2$s."), &["{min}", "{needed}"])));
    put("val_limit_between_characters_needed", json!(sprintf(&tr("Between %1$s and %2$s characters. Characters needed: %3$s"), &["{min}", "{max}", "{needed}"])));
    put("val_limit_between_characters_remaining", json!(sprintf(&tr("Between %1$s and %2$s characters. Characters remaining: %3$s"), &["{min}", "{max}", "{remaining}"])));
    put("val_limit_between_words_needed", json!(sprintf(&tr("Between %1$s and %2$s words. Words needed: %3$s"), &["{min}", "{max}", "{needed}"])));
    put("val_limit_between_words_remaining", json!(sprintf(&tr("Between %1$s and %2$s words. Words remaining: %3$s"), &["{min}", "{max}", "{remaining}"])));
    put("val_min", setting(page, "validation-min", tr("Please enter a value greater than or equal to {0}.")));
    put("val_max", setting(page, "validation-max", tr("Please enter a value less than or equal to {0}.")));
    put("val_recaptcha_fail_msg", setting(page, "recaptcha-fail-msg", tr("Google reCAPTCHA verification failed, please try again later.")));
    put("val_turnstile_fail_msg", setting(page, "turnstile-fail-msg", tr("Cloudflare Turnstile verification failed, please try again later.")));
    put("val_inputmask_incomplete", setting(page, "validation-inputmask-incomplete", tr("Please fill out the field in required format.")));
    put("uuid_cookie", json!(false));
    put("locale", json!(page.language));
    put("country", json!(false));
    put("country_list_label", json!(tr("Country list")));
    put("wpforms_plugin_url", json!(page.plugin_url));
    put("gdpr", page.settings.value("gdpr"));
    put("ajaxurl", json!(page.ajax_url));
    put("mailcheck_enabled", json!(true));
    put("mailcheck_domains", json!([]));
    put("mailcheck_toplevel_domains", json!(["dev"]));
    put("is_ssl", json!(page.ssl));
    put("nonce", json!(page.nonce));
    // `get_payment_strings()`.
    let code = page.settings.string("currency", "");
    let currency = CURRENCIES.iter().find(|c| c.0 == code).or(CURRENCIES.first()).expect("USD");
    put("currency_code", json!(currency.0));
    put("currency_thousands", json!(currency.1));
    put("currency_decimals", json!(currency.5));
    put("currency_decimal", json!(currency.2));
    put("currency_symbol", json!(currency.3));
    put("currency_symbol_pos", json!(currency.4));
    put("val_requiredpayment", setting(page, "validation-requiredpayment", tr("Payment is required.")));
    put("val_creditcard", setting(page, "validation-creditcard", tr("Please enter a valid credit card number.")));
    // `get_css_vars_strings()`.
    let modern = modern(page.settings);
    if modern {
        put("css_vars", Value::Array(super::css_vars::ROOT_VARS.iter().map(|(k, _)| json!(k)).collect()));
    }
    // `wpforms_frontend_strings`: `Modern::frontend_strings()`, the read-only fields, the token, the honeypot,
    // the analytics and the address field.
    if modern {
        put("isModernMarkupEnabled", json!(true));
        put("formErrorMessagePrefix", json!(tr("Form error message")));
        put("submitBtnDisabled", json!(tr("Submit button is disabled during form submission.")));
    }
    put("readOnlyDisallowedFields", json!(READ_ONLY_DISALLOWED));
    put("error_updating_token", json!(tr("Error updating token. Please try again or contact support if the issue persists.")));
    put("network_error", json!(tr("Network error or server is unreachable. Check your connection or try again later.")));
    put("token_cache_lifetime", json!(86400));
    let mut hn = Map::new();
    for form in forms {
        if let Some(field) = form.honeypot {
            hn.insert(form.id.to_string(), json!(field));
        }
    }
    put("hn_data", if hn.is_empty() { json!([]) } else { Value::Object(hn) });
    put("analytics_composite_field_types", json!(["name", "address", "date-time", "email", "password", "paypal-commerce"]));
    put("analytics_base64", json!(true));
    put("address_field", json!({"list_countries_without_states": ["GB", "DE", "CH", "NL"]}));
    // Every scalar is decoded and escaped as a string.
    for value in s.values_mut() {
        let text = match value {
            Value::String(t) => t.clone(),
            Value::Bool(b) => (if *b { "1" } else { "" }).to_owned(),
            Value::Number(_) => php::string(value),
            _ => continue,
        };
        *value = Value::String(esc_html(&html_entity_decode(&text)));
    }
    s
}

/// `Frontend::footer_end()`: the settings of the scripts after the footer scripts.
pub fn footer_end(page: &Page<'_>, forms: &[Printed<'_>]) -> String {
    let strings = rustpress_blocks::php_json::encode(&Value::Object(strings(page, forms)));
    format!("<script type='text/javascript'>\n/* <![CDATA[ */\nvar wpforms_settings = {strings}\n/* ]]> */\n</script>\n")
}

/// `Frontend::missing_assets_error_js()` for a user who may manage the forms.
pub fn missing_assets_error_js(tr: &dyn Fn(&str) -> String) -> String {
    let mut message = rustpress_core::i18n::sprintf(
        &tr("Heads up! WPForms has detected an issue with JavaScript on this page. JavaScript is required for this form to work properly, so this form may not work as expected. See our <a href=\"%s\" target=\"_blank\" rel=\"noopener noreferrer\">troubleshooting guide</a> to learn more or contact support."),
        &["https://wpforms.com/docs/getting-support-wpforms/"],
    );
    message.push_str("<p>");
    message.push_str(&esc_html(&tr("This message is only displayed to site administrators.")));
    message.push_str("</p>");
    MISSING_ASSETS_SCRIPT.replacen("%s", &message, 1)
}

/// `Frontend::get_missing_assets_error_script()`.
const MISSING_ASSETS_SCRIPT: &str = "<script>
				( function() {
					function wpforms_js_error_loading() {

						if ( typeof window.wpforms !== 'undefined' ) {
							return;
						}

						const forms = document.querySelectorAll( '.wpforms-form' );

						if ( ! forms.length ) {
							return;
						}

						const error = document.createElement( 'div' );

						error.classList.add( 'wpforms-error-container' );
						error.setAttribute( 'role', 'alert' );
						error.innerHTML = '%s';

						forms.forEach( function( form ) {
							if ( form.querySelector( '.wpforms-error-container' ) ) {
								return;
							}

							const formError = error.cloneNode( true ),
								formErrorId = form.id + '-error';

							formError.setAttribute( 'id', formErrorId );

							form.insertBefore( formError, form.firstChild );
							form.setAttribute( 'aria-invalid', 'true' );
							form.setAttribute( 'aria-errormessage', formErrorId );
						} );
					}

					if ( document.readyState === 'loading' ) {
						document.addEventListener( 'DOMContentLoaded', wpforms_js_error_loading );
					} else {
						wpforms_js_error_loading();
					}
				}() );
			</script>";
