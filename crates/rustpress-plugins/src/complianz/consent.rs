//! The consent area of `cmplz_document` (`[cmplz-consent-area]`), its block `complianz/consent-area` and the REST
//! route `consent-area/<post>/<block>` that gives the consented content to the script of the placeholder.

use rustpress_core::formatting::esc_attr;
use serde_json::Value as Json;

/// `cmplz_sanitize_category()`.
pub fn sanitize_category(category: &str) -> &str {
    match category {
        "functional" | "preferences" | "statistics" | "marketing" => category,
        _ => "marketing",
    }
}

/// `stripslashes()`.
fn stripslashes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('0') => out.push('\0'),
                Some(next) => out.push(next),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The leading numeric part of a PHP string (whitespace, sign, digits, fraction, exponent).
fn numeric_prefix(text: &str) -> Option<regex::Match<'_>> {
    static NUMBER: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    NUMBER.get_or_init(|| regex::Regex::new(r"^[ \t\n\r\x0B\x0C]*[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?").expect("valid regex")).find(text)
}

/// `1 == $value` for a value of `json_decode()`: numbers and numeric strings equal to one, `true`, and objects
/// (which PHP converts to 1).
fn loose_eq_one(value: &Json) -> bool {
    match value {
        Json::Bool(b) => *b,
        Json::Number(n) => n.as_f64() == Some(1.0),
        Json::String(s) => numeric_prefix(s).is_some_and(|m| s[m.end()..].trim_matches([' ', '\t', '\n', '\r', '\x0B', '\x0C']).is_empty() && m.as_str().trim_start().parse::<f64>().is_ok_and(|n| n == 1.0)),
        Json::Object(_) => true,
        Json::Null | Json::Array(_) => false,
    }
}

/// `cmplz_has_consent( $category )` with the default consent type of the site and the cookie `cmplz_{category}`.
pub fn has_consent(consent_type: &str, cookie: Option<&str>) -> bool {
    if consent_type.is_empty() {
        return true;
    }
    if consent_type.contains("optout") && cookie.is_none() {
        return true;
    }
    cookie == Some("allow")
}

/// `cmplz_has_service_consent( $service )` with the cookie `cmplz_consented_services`.
pub fn has_service_consent(consent_type: &str, cookie: Option<&str>, service: &str) -> bool {
    if consent_type.is_empty() || consent_type.contains("optout") {
        return true;
    }
    let Some(cookie) = cookie else { return false };
    match serde_json::from_str::<Json>(&stripslashes(cookie)) {
        Ok(Json::Object(services)) => services.get(service).is_some_and(loose_eq_one),
        _ => false,
    }
}

/// PHP's truthiness of an attribute: absent, `""` and `"0"` are false.
fn truthy(value: Option<&str>) -> bool {
    value.is_some_and(|v| !v.is_empty() && v != "0")
}

/// The attributes of `show_consent_area()` after `shortcode_atts()`.
pub struct Atts<'a> {
    pub id: Option<&'a str>,
    pub clientside: Option<&'a str>,
    pub cache_redirect: Option<&'a str>,
    pub scroll_into_view: Option<&'a str>,
    pub service: Option<&'a str>,
    pub category: Option<&'a str>,
    /// The text, already `sanitize_text_field()`ed; the default is `Click to accept the cookies for this service`.
    pub text: String,
}

/// What the consent area shows: the placeholder, or (for the server-side variant with consent) the content.
pub enum Area {
    Html(String),
    /// The consent is given: `prefix`, then `do_shortcode( $content )`.
    Consented { prefix: String },
}

/// `show_consent_area()` (and `show_consent_area_clientside()`), `post_id` being `global $post`'s ID or 0.
/// `locale` is `get_locale()` (for the `remove_accents()` of `sanitize_title()`).
pub fn show_consent_area(atts: &Atts, post_id: u64, consent_type: &str, locale: &str, cookie: &dyn Fn(&str) -> Option<String>) -> Area {
    let block_id = rustpress_core::slug::sanitize_title_save(atts.id.unwrap_or("default"), locale);
    if truthy(atts.clientside) {
        // shortcode_atts() a second time: `service` is 'general' by then, `category` still false.
        let category = atts.category.filter(|c| truthy(Some(c))).map_or("marketing", sanitize_category);
        let service = super::blocker::sanitize_service_name(atts.service.unwrap_or("general"));
        let category = esc_attr(category);
        return Area::Html(format!(
            "\t\t\t<div class=\"cmplz-consent-area cmplz-placeholder\" data-post_id=\"{post_id}\" data-block_id=\"{}\" data-category=\"{category}\" data-service=\"{}\">\n\t\t\t\t<a href=\"#\" class=\"cmplz_{category}_consentarea\" >{}</a>\n\t\t\t</div>\n\t\t\t",
            esc_attr(&block_id),
            esc_attr(&service),
            rustpress_core::kses::kses_post(&atts.text),
        ));
    }
    let cache_redirect = matches!(atts.cache_redirect, Some("true" | "1"));
    let scroll_into_view = atts.scroll_into_view.is_none_or(|v| v == "true");
    let has_category = truthy(atts.category);
    let category = if has_category { sanitize_category(atts.category.unwrap_or_default()).to_owned() } else { "no-category".to_owned() };
    let service = if has_category { "no-service".to_owned() } else { super::blocker::sanitize_service_name(atts.service.unwrap_or("general")) };

    let consented = has_service_consent(consent_type, cookie("cmplz_consented_services").as_deref(), &service) || has_consent(consent_type, cookie(&format!("cmplz_{category}")).as_deref());
    let (service, category) = (esc_attr(&service), esc_attr(&category));
    if consented {
        let mut prefix = String::new();
        if cache_redirect {
            prefix.push_str("\t\t\t\t\t<script>\n\t\t\t\t\t\tvar url = window.location.href;\n\t\t\t\t\t\tif (url.indexOf('cmplz_consent=1') === -1) {\n\t\t\t\t\t\t\tif (url.indexOf('?') !== -1) {url += '&';} else {url += '?';}\n\t\t\t\t\t\t\turl += 'cmplz_consent=1';\n\t\t\t\t\t\t\twindow.location.replace(url);\n\t\t\t\t\t\t} else{\n\t\t\t\t\t\t\tconsole.log(\"already on redirect url\");\n\t\t\t\t\t\t}\n\t\t\t\t\t</script>\n\t\t\t\t\t");
        }
        prefix.push_str("<a id=\"cmplz_consent_area_anchor\"></a>");
        return Area::Consented { prefix };
    }
    let redirect_uri = if scroll_into_view { "cmplz_consent=1#cmplz_consent_area_anchor" } else { "cmplz_consent=1" };
    let mut out = String::new();
    if cache_redirect {
        out.push_str("\t\t\t\t\t<script>\n\t\t\t\t\t\tvar url = window.location.href;\n\t\t\t\t\t\tvar consented_area_visible = document.getElementById('cmplz_consent_area_anchor');\n\t\t\t\t\t\tif (url.indexOf('cmplz_consent=1') !== -1 && !consented_area_visible ) {\n\t\t\t\t\t\t\turl = url.replace('cmplz_consent=1', '');\n\t\t\t\t\t\t\turl = url.replace('#cmplz_consent_area_anchor', '');\n\t\t\t\t\t\t\turl = url.replace('?&', '?');\n\t\t\t\t\t\t\turl = url.replace('&&', '?');\n\t\t\t\t\t\t\t//if last character is ? or &, drop it\n\t\t\t\t\t\t\tif (url.substring(url.length-1) === \"&\" || url.substring(url.length-1) === \"?\")\n\t\t\t\t\t\t\t{\n\t\t\t\t\t\t\t\turl = url.substring(0, url.length-1);\n\t\t\t\t\t\t\t}\n\t\t\t\t\t\t\twindow.location.replace(url);\n\t\t\t\t\t\t}\n\n\t\t\t\t\t\tdocument.addEventListener(\"cmplz_enable_category\", cmplzEnableCustomBlockedContent);\n\t\t\t\t\t\tfunction cmplzEnableCustomBlockedContent(e) {\n\t\t\t\t\t\t\tif ( cmplz_has_service_consent('");
        out.push_str(&format!("{service}' ) || cmplz_has_consent('{category}' )){{\n\t\t\t\t\t\t\t\tif (url.indexOf('cmplz_consent=1') === -1 ) {{\n\t\t\t\t\t\t\t\t\tif (url.indexOf('?') !== -1) {{url += '&';}} else {{url += '?';}}\n\t\t\t\t\t\t\t\t\turl += '{redirect_uri}';\n\t\t\t\t\t\t\t\t\twindow.location.replace(url);\n\t\t\t\t\t\t\t\t}}\n\t\t\t\t\t\t\t}}\n\t\t\t\t\t\t}}\n\t\t\t\t\t</script>\n\t\t\t\t"));
    } else {
        out.push_str(&format!("\t\t\t\t\t<script>\n\t\t\t\t\t\tdocument.addEventListener(\"cmplz_enable_category\", cmplzEnableCustomBlockedContent);\n\t\t\t\t\t\tfunction cmplzEnableCustomBlockedContent(e) {{\n\t\t\t\t\t\t\tif ( cmplz_has_service_consent('{service}', 'marketing' ) && !document.getElementById(\"cmplz_consent_area_anchor\") ){{\n\t\t\t\t\t\t\t\tlocation.reload();\n\t\t\t\t\t\t\t}}\n\t\t\t\t\t\t\tif ( e.detail.category === '{category}' && !document.getElementById(\"cmplz_consent_area_anchor\") ){{\n\t\t\t\t\t\t\t\tlocation.reload();\n\t\t\t\t\t\t\t}}\n\t\t\t\t\t\t}}\n\t\t\t\t\t</script>\n\t\t\t\t"));
    }
    out.push_str("\t\t\t\t<div class=\"cmplz-consent-area\">\n\t\t\t\t\t");
    let text = rustpress_core::kses::kses_post(&atts.text);
    if has_category {
        out.push_str(&format!("\t\t\t\t\t\t<a href=\"#\" data-category=\"{category}\" class=\"cmplz-accept-category  cmplz_{category}_consentarea\" >{text}</a>\n\t\t\t\t\t"));
    } else {
        out.push_str(&format!("\t\t\t\t\t\t<a href=\"#\" data-service=\"{service}\" class=\"cmplz-accept-service  cmplz_{service}_consentarea\" >{text}</a>\n\t\t\t\t\t"));
    }
    out.push_str("\t\t\t\t</div>\n\t\t\t\t");
    Area::Html(out)
}

/// A block attribute as PHP reads it into a string: `null` is absent, booleans become `"1"`/`""`.
fn attr_text(attrs: &serde_json::Map<String, Json>, key: &str) -> Option<String> {
    match attrs.get(key)? {
        Json::Null => None,
        Json::String(s) => Some(s.clone()),
        Json::Bool(b) => Some(if *b { "1".to_owned() } else { String::new() }),
        Json::Number(n) => Some(n.to_string()),
        other => Some(other.to_string()),
    }
}

/// `(int)` of a block attribute.
fn attr_int(attrs: &serde_json::Map<String, Json>, key: &str) -> i64 {
    match attrs.get(key) {
        Some(Json::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)).unwrap_or(0),
        Some(Json::String(s)) => numeric_prefix(s).and_then(|m| m.as_str().trim_start().parse::<f64>().ok()).map_or(0, |f| f as i64),
        Some(Json::Bool(true)) => 1,
        _ => 0,
    }
}

/// `cmplz_render_consent_area_block()`.
pub fn render_block(attrs: &serde_json::Map<String, Json>, locale: &str) -> String {
    let category = match attrs.get("category") {
        None | Some(Json::Null) => "marketing",
        Some(Json::String(s)) => sanitize_category(s),
        Some(_) => "marketing",
    };
    let service = match attr_text(attrs, "service") {
        None => "general".to_owned(),
        Some(s) => super::blocker::sanitize_service_name(&s),
    };
    let post_id = attr_int(attrs, "postId");
    let block_id = rustpress_core::slug::sanitize_title_save(&attr_text(attrs, "blockId").unwrap_or_default(), locale);
    let placeholder = attr_text(attrs, "placeholderContent").unwrap_or_default();
    format!(
        "<div class=\"cmplz-consent-area cmplz-placeholder\" data-post_id=\"{post_id}\" data-block_id=\"{}\" data-category=\"{}\" data-service=\"{}\">\n\t\t{}\t</div>\n\t",
        esc_attr(&block_id),
        esc_attr(category),
        esc_attr(&service),
        rustpress_core::kses::kses_post(&placeholder),
    )
}

/// The consented content of `cmplz_rest_consented_content()` before `do_shortcode()`: the attribute
/// `consentedContent` of the first `complianz/consent-area` block with the id (breadth first), or the content of the
/// shortcode with the id.
pub fn consented_content(content: &str, block_id: &str) -> String {
    if content.contains("<!-- wp:complianz/consent-area") {
        let mut queue: std::collections::VecDeque<rustpress_blocks::parser::Block> = rustpress_blocks::parser::parse_blocks(content).into();
        while let Some(block) = queue.pop_front() {
            if block.name.as_deref() == Some("complianz/consent-area") && matches!(block.attrs.get("blockId"), Some(Json::String(id)) if id == block_id) {
                return attr_text(&block.attrs, "consentedContent").unwrap_or_default();
            }
            queue.extend(block.inner_blocks);
        }
        String::new()
    } else if content.contains("[cmplz-consent-area") {
        let pattern = if block_id == "default" {
            r"(?is)\[cmplz-consent-area[^\]]*?\](.*?)\[/cmplz-consent-area\]".to_owned()
        } else {
            format!(r#"(?is)\[cmplz-consent-area[^\]]*?id=["']{}["'][^\]]*?\](.*?)\[/cmplz-consent-area\]"#, regex::escape(block_id))
        };
        regex::Regex::new(&pattern).ok().and_then(|re| re.captures(content).map(|c| c[1].to_owned())).unwrap_or_default()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_consent_reads_the_cookie() {
        assert!(has_service_consent("optin", Some(r#"{\"youtube\":1}"#), "youtube"));
        assert!(has_service_consent("optin", Some(r#"{"youtube":"1"}"#), "youtube"));
        assert!(!has_service_consent("optin", Some(r#"{"youtube":0}"#), "youtube"));
        assert!(!has_service_consent("optin", None, "youtube"));
        assert!(has_service_consent("optout", None, "youtube"));
    }

    #[test]
    fn category_consent_reads_the_cookie() {
        assert!(has_consent("optout", None));
        assert!(!has_consent("optout", Some("deny")));
        assert!(has_consent("optin", Some("allow")));
        assert!(!has_consent("optin", None));
    }
}
