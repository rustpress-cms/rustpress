//! Redirection 5.10.1 (`redirection/redirection.php`) on the website: the redirects of the WordPress module
//! (`WordPress_Module`), the canonical domain settings (`Redirection_Canonical`), the custom headers, the hit
//! counters and the logs of redirects and 404 pages.
//!
//! Differs from the plugin: the logs, counters and hits are only written when RustPress may write to the database;
//! a pass-through to another server (`Pass_Action::process_external()`) and custom filters (`Custom_Match`) do not
//! run; redirects on `wp-login.php` and the source list of `log_external` (a PHP backtrace) are missing; the
//! object cache of `Redirect_Cache` is not used (it only stores what the table holds).

mod item;
mod options;
pub(crate) mod php;
mod regex;
mod request;
mod url;

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, Init, NativePlugin, PluginRequest, PluginResponse, Redirect, Redirected};
use rustpress_core::{QueryResult, Result, Site};

use item::{Action, Ctx, Item};
use options::Options;

pub const VERSION: &str = "5.10.1";

pub struct Redirection;

impl NativePlugin for Redirection {
    fn file(&self) -> &'static str {
        "redirection/redirection.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some("redirection")
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move { Ok(Box::new(Loaded { options: Options::load(site) }) as Box<dyn ActivePlugin>) })
    }
}

pub struct Loaded {
    options: Options,
}

/// The request state: a redirect matched on `init` (`WordPress_Module::$matched`).
const MATCHED: &str = "redirection:matched";

const NO_CACHE: [(&str, &str); 2] = [("Expires", "Wed, 11 Jan 1984 05:00:00 GMT"), ("Cache-Control", "no-cache, must-revalidate, max-age=0, no-store, private")];

/// `wp_sanitize_redirect()`.
fn sanitize_redirect(location: &str) -> String {
    let mut out = String::new();
    for c in location.chars() {
        match c {
            ' ' => out.push_str("%20"),
            c if !c.is_ascii() => out.push_str(&php::urlencode(c.encode_utf8(&mut [0; 4]))),
            c if c.is_ascii_alphanumeric() || "-~+_.?#=&;,/:%!*[]()@".contains(c) => out.push(c),
            _ => {}
        }
    }
    // `_deep_replace()` of `%0d` and `%0a`.
    loop {
        let next = ["%0d", "%0a", "%0D", "%0A"].iter().fold(out.clone(), |s, strip| s.replace(strip, ""));
        if next == out {
            return out;
        }
        out = next;
    }
}

/// `filter_var( $url, FILTER_SANITIZE_URL )` and `wp_parse_url()` of `red_parse_domain_only()`.
fn domain_only(url: &str, https: bool) -> String {
    let url: String = url.chars().filter(|c| c.is_ascii_alphanumeric() || "$-_.+!*'(),{}|\\^~[]`<>#%\";/?:@&=".contains(*c)).collect();
    let url = if url.starts_with("http:") || url.starts_with("https:") { url } else { format!("{}{url}", if https { "https://" } else { "http://" }) };
    let rest = url.split_once("://").map_or("", |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    host.split(':').next().unwrap_or_default().to_owned()
}

impl Loaded {
    /// `red_get_rest_api()` as a path (the REST API is never redirected).
    fn rest_api_path(&self, site: &Site) -> String {
        let url = match self.options.get("rest_api") {
            rustpress_phpser::Value::Int(1) => site.home_url("/?rest_route=/"),
            _ => site.rest_url("/"),
        };
        let rest = url.split_once("://").map_or(url.as_str(), |(_, r)| r);
        match rest.find('/') {
            Some(i) => {
                let path = &rest[i..];
                if self.options.is_int("rest_api", 3) && path == "/index.php" { "/?rest_route=/".to_owned() } else { path.to_owned() }
            }
            None => String::new(),
        }
    }

    /// The headers of Redirection's `wp_redirect` filter.
    fn redirect_headers(&self, status: u16) -> Vec<(String, String)> {
        if status == 307 {
            return NO_CACHE.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        }
        let mut headers = options::run(&options::redirect_headers(&self.options.headers()));
        if !self.options.is_int("redirect_cache", 0) && status == 301 {
            if self.options.is_int("redirect_cache", -1) {
                headers.extend(NO_CACHE.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())));
            } else {
                let seconds = (self.options.number("redirect_cache") * 3600.0) as i64;
                let expires = chrono::DateTime::from_timestamp(chrono::Utc::now().timestamp() + seconds, 0).unwrap_or_default();
                headers.push(("Expires".to_owned(), expires.format("%a, %d %b %Y %H:%M:%S GMT").to_string()));
                headers.push(("Cache-Control".to_owned(), format!("max-age={seconds}")));
            }
        }
        headers
    }

    /// `wp_redirect( $target, $code, 'redirection' )` and `die()`.
    fn redirect(&self, location: &str, status: i64) -> Option<Init> {
        if location.is_empty() || !(300..=399).contains(&status) {
            return None;
        }
        let status = status as u16;
        let mut headers = Vec::new();
        for (name, value) in self.redirect_headers(status) {
            headers.retain(|(n, _): &(String, String)| !n.eq_ignore_ascii_case(&name));
            headers.push((name, value));
        }
        headers.push(("X-Redirect-By".to_owned(), "redirection".to_owned()));
        headers.push(("Location".to_owned(), sanitize_redirect(location)));
        headers.push(("Content-Type".to_owned(), "text/html; charset=UTF-8".to_owned()));
        Some(Init::Response(PluginResponse { status, headers, body: String::new() }))
    }

    /// `WordPress_Module::get_canonical_target()`.
    fn canonical_target(&self, site: &Site, request: &PluginRequest<'_>) -> Option<String> {
        let url = request::request_url(request);
        let relocate = self.options.string("relocate");
        if !relocate.is_empty() {
            let relocate = relocate.trim_end_matches('/');
            let protected = ["/wp-admin", "/wp-login.php", "/wp-json/"].iter().any(|base| url.starts_with(base));
            return (request::server_name(request) != domain_only(relocate, request.https) && !protected).then(|| format!("{relocate}{url}"));
        }
        let server = request::request_server_name(request);
        let preferred = self.options.string("preferred_domain");
        let has_www = |s: &str| s.starts_with("www.");
        let force_www = |s: &str| preferred == "www" && !has_www(s);
        let remove_www = |s: &str| preferred == "nowww" && has_www(s);
        let mut aliases: Vec<String> = if force_www(&server) || remove_www(&server) { vec![server.clone()] } else { Vec::new() };
        aliases.extend(self.options.strings("aliases"));
        if self.options.truthy("https") && !request.https {
            aliases.push(server.clone());
        }
        if !aliases.contains(&server) {
            return None;
        }
        // `get_canonical_target( get_bloginfo( 'url' ) )`.
        let home = site.home_url("");
        let mut canonical = domain_only(&home, request.https).trim_end_matches('/').to_owned();
        if force_www(&home) {
            canonical = format!("www.{}", canonical.trim_start_matches(['w', '.']));
        } else if remove_www(&home) {
            canonical = canonical.trim_start_matches(['w', '.']).to_owned();
        }
        let mut canonical = format!("{}{canonical}", if request.https { "https://" } else { "http://" });
        if self.options.truthy("https") {
            canonical = canonical.replace("http://", "https://");
        }
        (canonical == home).then(|| format!("{}{url}", rustpress_core::formatting::esc_url_raw(&canonical)))
    }

    /// `WordPress_Module::init()` (and `canonical_domain()` before it).
    async fn on_init(&self, site: &Site, request: &PluginRequest<'_>) -> Result<Option<Init>> {
        if !self.started() {
            return Ok(None);
        }
        if let Some(target) = self.canonical_target(site, request)
            && let Some(init) = self.redirect(&target, 301)
        {
            return Ok(Some(init));
        }
        let url = url::RequestUrl::new(&request::request_url(request));
        if !url.is_valid() || url.is_protected(&self.rest_api_path(site)) {
            return Ok(None);
        }
        let ctx = Ctx { site, request, options: &self.options, is_404: false };
        for item in Item::for_url(site, &self.options, &url.decoded).await? {
            let Some(fired) = item.get_match(&ctx, &url.decoded, &url.original).await? else { continue };
            request.state.lock().expect("plugin state").flags.insert(MATCHED);
            return Ok(Some(self.run(site, fired.action, fired.target).await?));
        }
        Ok(None)
    }

    /// `Red_Action::run()`.
    async fn run(&self, site: &Site, action: Action, target: Option<String>) -> Result<Init> {
        Ok(match action {
            Action::Url(code) => target.and_then(|t| self.redirect(&t, code)).unwrap_or(Init::Matched),
            Action::Random(code) => {
                let mut init = None;
                if let Some(id) = site.db.random_public_post_id().await?
                    && let Some(post) = site.db.post(id).await?
                {
                    init = self.redirect(&site.permalink(&post).await?, code);
                }
                init.unwrap_or(Init::Matched)
            }
            Action::Error(code) => Init::Error { status: u16::try_from(code).unwrap_or(404), by: "redirection" },
            Action::Nothing => Init::Matched,
            // `Pass_Action::process_internal()`; an external target is not fetched.
            Action::Pass => match target {
                Some(t) if !(t.starts_with("http://") || t.starts_with("https://")) => Init::Rewrite(t),
                _ => Init::Matched,
            },
        })
    }

    /// `Status::needs_installing()` and the database version the website code reads.
    fn started(&self) -> bool {
        !self.options.needs_installing() && self.options.supports_match_url()
    }

    /// `WordPress_Module::template_redirect()`: redirects of the 404 page type, then the 404 log.
    async fn on_not_found(&self, site: &Site, query: &QueryResult, request: &PluginRequest<'_>) -> Result<Option<Redirect>> {
        if !self.started() || !query.flags.is_404 || request.state.lock().expect("plugin state").flags.contains(MATCHED) {
            return Ok(None);
        }
        let url = url::RequestUrl::new(&request::request_url(request));
        if url.is_valid() && !url.is_protected(&self.rest_api_path(site)) {
            let ctx = Ctx { site, request, options: &self.options, is_404: true };
            for item in Item::for_url(site, &self.options, &url.decoded).await? {
                if !item.matcher.as_ref().is_some_and(item::Match::is_page) {
                    continue;
                }
                if let Some(fired) = item.get_match(&ctx, &url.decoded, &url.original).await? {
                    if let (Action::Url(code), Some(target)) = (fired.action, fired.target)
                        && (300..=399).contains(&code)
                    {
                        return Ok(Some(Redirect { location: target, status: code as u16, by: "redirection" }));
                    }
                    break;
                }
            }
        }
        if self.options.number("expire_404") >= 0.0 {
            let details = request::Details { target: None, http_code: 404, redirect_id: None, redirect_by: None, log_headers: self.options.truthy("log_header") };
            request::log(site, request, &self.options, "redirection_404", &request::request_url(request), details).await?;
        }
        Ok(None)
    }

    /// `wp_redirect` and `x_redirect_by` for a redirect of WordPress or another plugin.
    async fn on_wp_redirect(&self, site: &Site, request: &PluginRequest<'_>, redirect: &Redirected<'_>) -> Result<Vec<(String, String)>> {
        if !self.started() {
            return Ok(Vec::new());
        }
        let headers = self.redirect_headers(redirect.status);
        // `record_redirect_by()`.
        let matched = request.state.lock().expect("plugin state").flags.contains(MATCHED);
        if !matched && redirect.by != "redirection" && self.options.truthy("log_external") {
            let by = if redirect.by.is_empty() { "wordpress" } else { redirect.by };
            let details = request::Details {
                target: Some(redirect.location.to_owned()),
                http_code: i64::from(redirect.status),
                redirect_id: None,
                redirect_by: Some(by.to_owned()),
                log_headers: self.options.truthy("log_header"),
            };
            request::log(site, request, &self.options, "redirection_logs", &request::request_url(request), details).await?;
        }
        Ok(headers)
    }
}

impl ActivePlugin for Loaded {
    fn init<'a>(&'a self, site: &'a Site, request: &'a PluginRequest<'a>) -> BoxFuture<'a, Result<Option<Init>>> {
        Box::pin(self.on_init(site, request))
    }

    /// `WordPress_Module::send_headers()`: the site headers of the settings.
    fn send_headers(&self, _site: &Site, _request: &PluginRequest<'_>) -> Vec<(String, String)> {
        if !self.started() {
            return Vec::new();
        }
        options::run(&options::site_headers(&self.options.headers()))
    }

    fn wp_redirect<'a>(&'a self, site: &'a Site, request: &'a PluginRequest<'a>, redirect: &'a Redirected<'a>) -> BoxFuture<'a, Result<Vec<(String, String)>>> {
        Box::pin(self.on_wp_redirect(site, request, redirect))
    }

    fn not_found<'a>(&'a self, site: &'a Site, query: &'a QueryResult, request: &'a PluginRequest<'a>) -> BoxFuture<'a, Result<Option<Redirect>>> {
        Box::pin(self.on_not_found(site, query, request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_redirects() {
        assert_eq!(sanitize_redirect("/hello-world/?q=ä b"), "/hello-world/?q=%C3%A4%20b");
        assert_eq!(sanitize_redirect("/a<b>%0%0aDd\"'"), "/abd");
    }
}
