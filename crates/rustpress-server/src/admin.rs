//! `wp-login.php`, `wp-admin/load-styles.php` and `wp-admin/load-scripts.php`: the parts of the
//! administration area the server hands to `rustpress-admin`.

use std::path::Path;

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode, header};
use rustpress_admin::login::{self, LanguageOption, LoginEnv, LoginRequest};
use rustpress_admin::registry::{LoaderEnv, default_scripts, default_styles};
use rustpress_auth::{Auth, Roles, Salts};
use rustpress_core::Site;
use rustpress_core::rewrite::urldecode;

use crate::app::Loaded;
use crate::config::Config;

/// The seconds since the epoch.
pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// `$wp_version` of the WordPress files.
pub fn wp_version(includes: &Path) -> String {
    let text = std::fs::read_to_string(includes.join("version.php")).unwrap_or_default();
    text.split("$wp_version = '").nth(1).and_then(|rest| rest.split('\'').next()).unwrap_or("0").to_owned()
}

/// Everything the administration area derives from the site once per snapshot.
pub struct AdminParts {
    pub auth: Auth,
    pub roles: Roles,
    pub loader: LoaderEnv,
    pub languages: Vec<LanguageOption>,
    pub privacy_policy: Option<(String, String)>,
}

fn config_path(config: &Config) -> Option<std::path::PathBuf> {
    if let Some(path) = &config.wp_config {
        return Some(path.clone());
    }
    let inside = config.wp_root.join("wp-config.php");
    if inside.is_file() {
        return Some(inside);
    }
    let above = config.wp_root.parent()?.join("wp-config.php");
    above.is_file().then_some(above)
}

impl AdminParts {
    pub async fn load(config: &Config, site: &Site) -> AdminParts {
        let constants = config_path(config).and_then(|path| std::fs::read_to_string(path).ok()).map(|text| rustpress_auth::salts::parse_config_constants(&text)).unwrap_or_default();
        let mut options = std::collections::HashMap::new();
        for name in Salts::option_names() {
            if let Ok(Some(value)) = site.options.fetch(&site.db, &name).await {
                options.insert(name, value);
            }
        }
        let auth = Auth::new(Salts::new(constants, options), &site.settings.siteurl);
        let prefix = site.db.table_prefix();
        let roles = Roles::from_option(&site.options.fetch(&site.db, &format!("{prefix}user_roles")).await.ok().flatten().unwrap_or_default());
        let includes = config.wp_root.join("wp-includes");
        let siteurl = site.settings.siteurl.trim_end_matches('/').to_owned();
        let loader = LoaderEnv {
            version: wp_version(&includes),
            includes,
            content_url: format!("{siteurl}/wp-content"),
            site_url: siteurl,
            rtl: site.is_rtl(),
            separate_block_assets: false,
            compressed_tinymce: false,
            nonce: None,
        };
        let languages = installed_languages(config, site).await;
        let privacy_policy = privacy_policy(site).await;
        AdminParts { auth, roles, loader, languages, privacy_policy }
    }
}

/// `get_available_languages()` with the names `wp_get_available_translations()` has cached.
async fn installed_languages(config: &Config, site: &Site) -> Vec<LanguageOption> {
    let dir = config.wp_root.join("wp-content").join("languages");
    let mut locales: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut names: Vec<String> = entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        for name in names {
            let stem = name.strip_suffix(".mo").or_else(|| name.strip_suffix(".l10n.php"));
            let Some(stem) = stem else { continue };
            if stem.starts_with("continents-cities") || stem.starts_with("ms-") || stem.starts_with("admin-") || locales.iter().any(|l| l == stem) {
                continue;
            }
            locales.push(stem.to_owned());
        }
    }
    let translations = site.options.fetch(&site.db, "_site_transient_available_translations").await.ok().flatten().and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok());
    locales
        .into_iter()
        .map(|locale| {
            let entry = translations.as_ref().and_then(|t| t.get(&locale));
            let native_name = entry.and_then(|e| e.get("native_name")).and_then(|v| v.as_str()).map_or_else(|| locale.clone(), str::to_owned);
            let lang = entry
                .and_then(|e| e.get("iso"))
                .and_then(|iso| iso.as_array())
                .and_then(|items| items.first())
                .and_then(|(_, v)| v.as_str())
                .map(str::to_owned)
                .unwrap_or_default();
            LanguageOption { locale, native_name, lang }
        })
        .collect()
}

/// The privacy policy page, when it is published: `(title, address)`.
async fn privacy_policy(site: &Site) -> Option<(String, String)> {
    let id = site.options.u64("wp_page_for_privacy_policy", 0);
    if id == 0 {
        return None;
    }
    let page = site.db.post(id).await.ok().flatten()?;
    if page.post_status != "publish" {
        return None;
    }
    let title = rustpress_blocks::content::the_title_for(site, &page);
    let url = site.permalink(&page).await.ok()?;
    (!title.is_empty()).then_some((title, url))
}

/// The cookies of a request, decoded the way PHP fills `$_COOKIE`.
fn cookies_of(headers: &HeaderMap) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for value in headers.get_all(header::COOKIE) {
        let Ok(text) = value.to_str() else { continue };
        for pair in text.split(';') {
            let Some((name, value)) = pair.trim().split_once('=') else { continue };
            out.push((name.trim().to_owned(), String::from_utf8_lossy(&urldecode(value.trim().as_bytes())).into_owned()));
        }
    }
    out
}

/// `a=1&b=2` as decoded pairs.
fn pairs_of(text: &str) -> Vec<(String, String)> {
    text.split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (String::from_utf8_lossy(&urldecode(name.as_bytes())).into_owned(), String::from_utf8_lossy(&urldecode(value.as_bytes())).into_owned())
        })
        .collect()
}

/// `wp-login.php`.
pub async fn login_page(loaded: &Loaded, writer: Option<&rustpress_db::Writer>, method_post: bool, query: &str, body: &str, headers: &HeaderMap) -> Response<Body> {
    let Some(parts) = &loaded.admin else { return plain(StatusCode::NOT_FOUND, "Not Found") };
    let mut loader = parts.loader.clone();
    // Nobody is signed in on this page: the nonces belong to user 0 without a session.
    let auth = parts.auth.clone();
    let at = now();
    loader.nonce = Some(std::sync::Arc::new(move |action: &str| auth.create_nonce(action, 0, "", at).unwrap_or_default()));
    let env = LoginEnv {
        site: &loaded.site,
        db: &loaded.site.db,
        writer,
        auth: &parts.auth,
        roles: &parts.roles,
        loader,
        languages: parts.languages.clone(),
        privacy_policy: parts.privacy_policy.clone(),
        https: loaded.site.settings.siteurl.starts_with("https"),
        now: now(),
    };
    let request = LoginRequest {
        post: method_post,
        query: pairs_of(query),
        form: if method_post { pairs_of(body) } else { Vec::new() },
        cookies: cookies_of(headers),
        remote_addr: None,
        user_agent: headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_owned),
    };
    let reply = login::handle(&env, &request).await;
    let mut response = Response::new(Body::from(reply.body));
    *response.status_mut() = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::OK);
    for (name, value) in &reply.headers {
        if let (Ok(name), Ok(value)) = (header::HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            response.headers_mut().append(name, value);
        }
    }
    for cookie in &reply.cookies {
        if let Ok(value) = HeaderValue::from_str(&cookie.header_value(env.now)) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }
    response
}

/// A file in `wp-admin` (`index.php`, `options-general.php`, ...).
#[allow(clippy::too_many_arguments)]
pub async fn admin_page(loaded: &Loaded, config: &Config, writer: Option<&rustpress_db::Writer>, file: &str, method_post: bool, request_uri: &str, query: &str, body: &str, headers: &HeaderMap) -> Response<Body> {
    let Some(parts) = &loaded.admin else { return plain(StatusCode::NOT_FOUND, "Not Found") };
    let themes = config.wp_root.join("wp-content").join("themes");
    let block_theme = [&loaded.site.settings.stylesheet, &loaded.site.settings.template].iter().any(|t| themes.join(t.as_str()).join("templates").join("index.html").is_file());
    let env = rustpress_admin::screens::AdminEnv {
        site: &loaded.site,
        db: &loaded.site.db,
        writer,
        auth: &parts.auth,
        roles: &parts.roles,
        loader: parts.loader.clone(),
        https: loaded.site.settings.siteurl.starts_with("https"),
        now: now(),
        languages: parts.languages.clone(),
        available_translations: Vec::new(),
        block_theme,
    };
    let request = rustpress_admin::screens::AdminRequest {
        post: method_post,
        file: file.to_owned(),
        query: pairs_of(query),
        form: if method_post { pairs_of(body) } else { Vec::new() },
        cookies: cookies_of(headers),
        request_uri: request_uri.to_owned(),
        remote_addr: None,
        user_agent: headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_owned),
    };
    let reply = rustpress_admin::screens::handle(&env, &request).await;
    let mut response = Response::new(Body::from(reply.body));
    *response.status_mut() = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::OK);
    for (name, value) in &reply.headers {
        if let (Ok(name), Ok(value)) = (header::HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            response.headers_mut().append(name, value);
        }
    }
    for cookie in &reply.cookies {
        if let Ok(value) = HeaderValue::from_str(&cookie.header_value(env.now)) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }
    response
}

fn plain(status: StatusCode, text: &'static str) -> Response<Body> {
    let mut response = Response::new(Body::from(text));
    *response.status_mut() = status;
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=UTF-8"));
    response
}

/// The `load` parameter of `load-styles.php` / `load-scripts.php`: sorted chunks joined, cleaned, split.
fn load_list(query: &str) -> Vec<String> {
    let mut chunks: Vec<(String, String)> = pairs_of(query).into_iter().filter_map(|(k, v)| k.strip_prefix("load[").and_then(|r| r.strip_suffix(']')).map(|name| (name.to_owned(), v))).collect();
    if chunks.is_empty() {
        chunks = pairs_of(query).into_iter().filter(|(k, _)| k == "load").collect();
    }
    chunks.sort_by(|a, b| a.0.cmp(&b.0));
    let joined: String = chunks.into_iter().map(|(_, v)| v).collect();
    let cleaned: String = joined.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, ',' | '_' | '-')).collect();
    let mut out: Vec<String> = Vec::new();
    for handle in cleaned.split(',') {
        if !out.iter().any(|h| h == handle) {
            out.push(handle.to_owned());
        }
    }
    out
}

fn static_reply(content_type: &'static str, etag: &str, body: String) -> Response<Body> {
    let mut response = Response::new(Body::from(body));
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(etag) {
        headers.insert(header::ETAG, value);
    }
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    let expires = chrono_like_http_date(now() + 31_536_000);
    if let Ok(value) = HeaderValue::from_str(&expires) {
        headers.insert(header::EXPIRES, value);
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000"));
    response
}

/// An HTTP date (`Sun, 06 Nov 1994 08:49:37 GMT`).
fn chrono_like_http_date(timestamp: i64) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let days = timestamp.div_euclid(86_400);
    let secs = timestamp.rem_euclid(86_400);
    // civil from days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }
    format!("{}, {day:02} {} {year} {:02}:{:02}:{:02} GMT", DAYS[days.rem_euclid(7) as usize], MONTHS[(month - 1) as usize], secs / 3600, (secs % 3600) / 60, secs % 60)
}

fn not_modified(etag: &str, headers: &HeaderMap) -> Option<Response<Body>> {
    let sent = headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok())?;
    if sent.replace('\\', "") != etag {
        return None;
    }
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    Some(response)
}

fn read_file(root: &Path, relative: &str) -> String {
    let path = root.join(relative.trim_start_matches('/'));
    let Ok(canonical) = std::fs::canonicalize(&path) else { return String::new() };
    let Ok(base) = std::fs::canonicalize(root) else { return String::new() };
    if !canonical.starts_with(&base) {
        return String::new();
    }
    std::fs::read_to_string(canonical).unwrap_or_default()
}

/// `wp-admin/load-styles.php`.
pub fn load_styles(loaded: &Loaded, config: &Config, query: &str, headers: &HeaderMap) -> Response<Body> {
    let Some(parts) = &loaded.admin else { return plain(StatusCode::NOT_FOUND, "Not Found") };
    let load = load_list(query);
    if load.iter().all(String::is_empty) {
        return plain(StatusCode::BAD_REQUEST, "");
    }
    let rtl = pairs_of(query).iter().any(|(k, v)| k == "dir" && v == "rtl");
    let styles = default_styles(&parts.loader);
    let etag = styles.deps.etag(&load, &parts.loader.version);
    if let Some(response) = not_modified(&etag, headers) {
        return response;
    }
    let mut out = String::new();
    for handle in &load {
        let Some(style) = styles.deps.get(handle) else { continue };
        let Some(src) = style.src.as_deref().filter(|s| !s.is_empty()) else { continue };
        let mut path = src.to_owned();
        if rtl && style.extra.get("rtl").is_some_and(|e| e.truthy()) {
            path = path.replace(".min.css", "-rtl.min.css");
        }
        let content = format!("{}\n", read_file(&config.wp_root, &path));
        if src.starts_with("/wp-includes/css/") {
            out.push_str(&content.replace("../images/", "../wp-includes/images/").replace("../js/tinymce/", "../wp-includes/js/tinymce/").replace("../fonts/", "../wp-includes/fonts/"));
        } else {
            out.push_str(&content.replace("../images/", "images/"));
        }
    }
    static_reply("text/css; charset=UTF-8", &etag, out)
}

/// `wp-admin/load-scripts.php`.
pub fn load_scripts(loaded: &Loaded, config: &Config, query: &str, headers: &HeaderMap) -> Response<Body> {
    let Some(parts) = &loaded.admin else { return plain(StatusCode::NOT_FOUND, "Not Found") };
    let load = load_list(query);
    if load.iter().all(String::is_empty) {
        return plain(StatusCode::BAD_REQUEST, "");
    }
    let scripts = default_scripts(&parts.loader);
    let etag = scripts.deps.etag(&load, &parts.loader.version);
    if let Some(response) = not_modified(&etag, headers) {
        return response;
    }
    let mut out = String::new();
    for handle in &load {
        let Some(script) = scripts.deps.get(handle) else { continue };
        out.push_str(&read_file(&config.wp_root, script.src.as_deref().unwrap_or_default()));
        out.push('\n');
    }
    static_reply("application/javascript; charset=UTF-8", &etag, out)
}
