//! A request for a page in the preview of the Customizer: `_wp_customize_include()` reads its arguments, and
//! `WP_Customize_Manager::setup_theme()` decides whether the page may be shown (`wp_die()` otherwise).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode, header};
use rustpress_core::customize::Preview;

use crate::app::Loaded;

/// The arguments of `_wp_customize_include()` on the website.
#[derive(Debug, Clone)]
pub struct Request {
    /// `customize_changeset_uuid` as sent.
    pub uuid: String,
    /// `sanitize_key( $_REQUEST['customize_messenger_channel'] )`.
    pub channel: String,
    /// `customize_theme`.
    pub theme: Option<String>,
    /// `customize_autosaved`.
    pub autosaved: bool,
    /// The query string, for the nonces sent with it.
    pub query: String,
}

impl Request {
    /// The Customizer is loaded when the request names a changeset; the arguments of a POST win over the query's.
    pub fn parse(query: &str, form: &[(String, String)]) -> Option<Self> {
        let mut pairs = crate::admin::pairs_of_public(query);
        pairs.extend(form.iter().cloned());
        let get = |key: &str| pairs.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        let uuid = get("customize_changeset_uuid").filter(|uuid| !uuid.is_empty())?;
        Some(Self {
            uuid,
            channel: get("customize_messenger_channel").map(|c| sanitize_key(&c)).unwrap_or_default(),
            theme: get("customize_theme"),
            autosaved: get("customize_autosaved").is_some_and(|v| !v.is_empty() && v != "0"),
            query: query.to_owned(),
        })
    }
}

/// `sanitize_key()`.
pub fn sanitize_key(key: &str) -> String {
    key.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '-').collect()
}

/// `wp_is_uuid()` of any version.
fn is_uuid(uuid: &str) -> bool {
    let parts: Vec<&str> = uuid.split('-').collect();
    parts.len() == 5 && parts.iter().zip([8, 4, 4, 4, 12]).all(|(part, len)| part.len() == len && part.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
}

/// `get_post_stati()`: a changeset is found in any status.
pub(crate) const STATI: [&str; 12] = ["publish", "future", "draft", "pending", "private", "trash", "auto-draft", "inherit", "request-pending", "request-confirmed", "request-failed", "request-completed"];

/// `WP_Customize_Manager::find_changeset_post_id()`.
async fn changeset_exists(loaded: &Loaded, uuid: &str) -> bool {
    let query = rustpress_db::PostQuery {
        post_types: vec!["customize_changeset".into()],
        statuses: STATI.iter().map(|s| (*s).to_owned()).collect(),
        name: Some(uuid.to_owned()),
        limit: Some(1),
        ..Default::default()
    };
    loaded.site.db.posts(&query).await.map(|posts| !posts.is_empty()).unwrap_or(false)
}

/// `WP_Customize_Manager::wp_die()` on the website: the error page of `wp_die()`. `setup_theme()` runs before the
/// translations are loaded. In the frame the page tells the Customizer about the error (`error`, the Ajax message).
fn die(loaded: &Loaded, request: &Request, message: &str, error: i64, translated: bool) -> Response<Body> {
    let site = &loaded.site;
    let mut message = message.to_owned();
    if !request.channel.is_empty() {
        let mut scripts = rustpress_theme::customize_preview::FooterScripts::new(site, &loaded.theme.wp_root, &[], &[]);
        scripts.enqueue("customize-base");
        message.push_str(&scripts.print());
        let settings = serde_json::json!({"messengerArgs": {"channel": request.channel, "url": site.site_url("wp-admin/customize.php")}, "error": error});
        message.push_str(&format!(
            "<script>\n\t\t\t( function( api, settings ) {{\n\t\t\t\tvar preview = new api.Messenger( settings.messengerArgs );\n\t\t\t\tpreview.send( 'iframe-loading-error', settings.error );\n\t\t\t}} )( wp.customize, {} );\n\t\t\t\n//# sourceURL=WP_Customize_Manager%3A%3Awp_die\n</script>\n",
            rustpress_theme::customize_preview::json_hex_tag(&settings)
        ));
    }
    let body = if translated { rustpress_admin::die::page(site, &site.tr("WordPress &rsaquo; Error"), &message) } else { rustpress_admin::die::page_before_translations(site, "WordPress &rsaquo; Error", &message) };
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=UTF-8"));
    headers.insert(header::EXPIRES, HeaderValue::from_static("Wed, 11 Jan 1984 05:00:00 GMT"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache, must-revalidate, max-age=0, no-store, private"));
    response
}

/// `setup_theme()` and the check of `customize_preview_init()`: the preview of the page, or the error page.
#[allow(clippy::result_large_err)]
pub async fn setup_theme(loaded: &Loaded, viewer: Option<&rustpress_core::Viewer>, request: &Request, form: &[(String, String)]) -> Result<(Arc<Preview>, Option<rustpress_core::Site>), Response<Body>> {
    let site = &loaded.site;
    let uuid = sanitize_key(&request.uuid);
    if !is_uuid(&uuid) {
        return Err(die(loaded, request, "Invalid changeset UUID", -1, false));
    }
    let can = viewer.is_some_and(|v| v.can("customize"));
    // Without the capability the UUID is the key: only a changeset that exists can be looked at.
    if !can && !changeset_exists(loaded, &uuid).await {
        return Err(die(loaded, request, "Non-existent changeset UUID.", if request.channel.is_empty() { -1 } else { 0 }, false));
    }
    // `customize_preview_init()` runs after the translations are loaded.
    if !request.channel.is_empty() && !can {
        let message = rustpress_core::i18n::sprintf(&site.tr("Unauthorized. You may remove the %s param to preview as frontend."), &["<code>customize_messenger_channel</code>"]);
        return Err(die(loaded, request, &message, -1, true));
    }
    let block_theme = rustpress_blocks::ThemeAccess::is_block_theme(&loaded.theme);
    // `add_theme_support( 'customize-selective-refresh-widgets' )` of the ported theme.
    let selective_refresh_widgets = loaded.theme.template == "twentyseventeen";
    let mut preview = Preview::new(&uuid, &request.channel, !block_theme, selective_refresh_widgets, can, request.autosaved);
    // `$_POST['customized']` counts with the capability and a nonce of the Customizer only.
    let stylesheet = &site.settings.stylesheet;
    let nonce_of = |key: &str| crate::admin::pairs_of_public(&request.query).into_iter().chain(form.iter().cloned()).rev().find(|(k, _)| k == key).map(|(_, v)| v);
    let has_nonce = viewer.is_some_and(|v| {
        let preview_action = format!("preview-customize_{stylesheet}");
        nonce_of("nonce").is_some_and(|n| v.check_nonce(&n, &preview_action) || v.check_nonce(&n, &format!("save-customize_{stylesheet}"))) || nonce_of("customize_preview_nonce").is_some_and(|n| v.check_nonce(&n, &preview_action))
    });
    let posted: Vec<(String, serde_json::Value)> = match form.iter().rev().find(|(k, _)| k == "customized") {
        Some((_, json)) if can && has_nonce => match serde_json::from_str::<serde_json::Value>(json) {
            Ok(serde_json::Value::Object(map)) => map.into_iter().collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    let changeset = crate::customize_values::changeset_data(loaded, &uuid).await;
    let values = crate::customize_values::merge(&changeset, stylesheet, &posted);
    let previewed = crate::customize_values::apply(loaded, &values);
    preview.dirty = posted.into_iter().map(|(id, _)| id).collect();
    preview.previewed = values.iter().map(|(id, _)| id.clone()).collect();
    preview.known = previewed.known.clone();
    preview.custom_css = previewed.custom_css;
    let site = (!previewed.options.is_empty()).then(|| loaded.site.with_options(&previewed.options));
    Ok((Arc::new(preview), site))
}

/// `wp_send_json()` with the headers of the preview.
fn json(status: StatusCode, value: &serde_json::Value) -> Response<Body> {
    let mut response = Response::new(Body::from(rustpress_blocks::php_json::encode(value)));
    *response.status_mut() = status;
    preview_headers(response.headers_mut());
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=UTF-8"));
    response
}

/// `WP_Customize_Selective_Refresh::handle_render_partials_request()`: the partials for their placements.
pub async fn render_partials(loaded: &Loaded, site: &rustpress_core::Site, result: &rustpress_core::QueryResult, preview: &Preview, form: &[(String, String)]) -> Response<Body> {
    let error = |status: StatusCode, code: &str| json(status, &serde_json::json!({"success": false, "data": code}));
    let Some((_, raw)) = form.iter().rev().find(|(k, _)| k == "partials") else { return error(StatusCode::BAD_REQUEST, "missing_partials") };
    let partials: Vec<(String, serde_json::Value)> = match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Object(map)) => map.into_iter().collect(),
        Ok(serde_json::Value::Array(list)) => list.into_iter().enumerate().map(|(i, v)| (i.to_string(), v)).collect(),
        _ => return error(StatusCode::OK, "malformed_partials"),
    };
    if partials.iter().any(|(_, contexts)| !contexts.is_array() && !contexts.is_object()) {
        return error(StatusCode::OK, "malformed_container_contexts");
    }
    let partials: Vec<(String, serde_json::Value)> = partials.into_iter().map(|(id, contexts)| (id, if let serde_json::Value::Object(map) = contexts { serde_json::Value::Array(map.into_values().collect()) } else { contexts })).collect();
    let data = match rustpress_theme::customize_preview::render_partials(&loaded.theme, site, result, &loaded.registry, &partials).await {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(error = %e, "rendering the partials failed");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "render_failed");
        }
    };
    let mut response = serde_json::Map::new();
    for key in ["contents", "errors"] {
        if let Some(value) = data.get(key) {
            response.insert(key.into(), value.clone());
        }
    }
    // `validate_setting_values( unsanitized_post_values() )`: what the preview knows is valid.
    let valid: serde_json::Map<String, serde_json::Value> = preview.known.iter().map(|id| (id.clone(), serde_json::Value::Bool(true))).collect();
    response.insert("setting_validities".into(), if valid.is_empty() { serde_json::json!([]) } else { serde_json::Value::Object(valid) });
    if let Some(header) = data.get("custom_header_settings") {
        response.insert("custom_header_settings".into(), header.clone());
    }
    // `export_partial_rendered_nav_menu_instances()`.
    let args = preview.rendered().nav_menu_args;
    response.insert("nav_menu_instance_args".into(), if args.is_empty() { serde_json::json!([]) } else { serde_json::Value::Object(args.into_iter().collect()) });
    json(StatusCode::OK, &serde_json::json!({"success": true, "data": response}))
}

/// `customize_preview_init()`: `nocache_headers()`, the robots headers and `filter_iframe_security_headers()`.
pub fn preview_headers(headers: &mut HeaderMap) {
    headers.insert(header::EXPIRES, HeaderValue::from_static("Wed, 11 Jan 1984 05:00:00 GMT"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache, must-revalidate, max-age=0, no-store, private"));
    headers.insert("x-robots", HeaderValue::from_static("noindex, nofollow, noarchive"));
    headers.insert("x-robots-tag", HeaderValue::from_static("noindex, nofollow, noarchive"));
    headers.insert("x-frame-options", HeaderValue::from_static("SAMEORIGIN"));
    headers.insert("content-security-policy", HeaderValue::from_static("frame-ancestors 'self'"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_names_a_changeset() {
        let request = Request::parse("customize_changeset_uuid=0f8c4a1e-2b3d-4c5e-9f60-718293a4b5c6&customize_messenger_channel=Preview-0", &[]).expect("a preview");
        assert_eq!(request.channel, "preview-0");
        assert!(Request::parse("p=1", &[]).is_none());
        let posted = Request::parse("", &[("customize_changeset_uuid".to_owned(), "x".to_owned()), ("customize_autosaved".to_owned(), "on".to_owned())]).expect("a posted preview");
        assert!(posted.autosaved);
    }

    #[test]
    fn uuids_of_any_version() {
        assert!(is_uuid("0f8c4a1e-2b3d-4c5e-9f60-718293a4b5c6"));
        assert!(!is_uuid("nope"));
        assert!(!is_uuid("0F8C4A1E-2b3d-4c5e-9f60-718293a4b5c6"));
        assert_eq!(sanitize_key("AB c-_9!"), "abc-_9");
    }
}
