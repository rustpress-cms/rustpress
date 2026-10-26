//! The REST API for signed-in users: reading with the `edit` context and writing (`POST`, `PUT`, `PATCH`,
//! `DELETE`). Cookie authentication with the `wp_rest` nonce is checked by the server; this module gets the
//! result as [`Access`].

mod app_passwords;
mod block_renderer;
mod caps;
mod editor;
mod fonts;
mod export;
mod health;
mod hints;
mod icons;
mod media;
mod navigation;
mod plugins;
mod posts;
mod settings;
mod templates;
pub use templates::default_template_types;
mod terms;
mod types;
mod users;
mod view_config;

use std::path::PathBuf;

use rustpress_auth::CurrentUser;
use rustpress_db::Writer;
use serde_json::{Map, Value};

use crate::error::RestError;
use crate::params::Params;
use crate::{Outcome, Rest, Result};

/// The signed-in user of a request.
pub struct Access<'a> {
    pub user: CurrentUser,
    /// The application password the request was authenticated with.
    pub app_password: Option<String>,
    pub writer: Option<&'a Writer>,
    pub now: i64,
    pub uploads: PathBuf,
}

/// A file of a multipart upload.
#[derive(Debug, Clone)]
pub struct UploadedPart {
    pub field: String,
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

/// The body of a request.
#[derive(Debug, Clone, Default)]
pub enum Body {
    #[default]
    None,
    Json(Value),
    Form(Vec<(String, String)>),
    Multipart { fields: Vec<(String, String)>, files: Vec<UploadedPart> },
    /// A raw upload (`POST /wp/v2/media` with `Content-Disposition: attachment; filename=...`).
    Raw { bytes: Vec<u8>, content_type: String, disposition: String },
}

/// The request as the server saw it.
#[derive(Debug, Clone, Default)]
pub struct Request {
    pub method: String,
    pub body: Body,
    /// The user and password of an `Authorization: Basic` header.
    pub basic: Option<(String, String)>,
}

/// The parameters of a request: the query string and the body in one map (the body wins).
#[derive(Debug, Clone, Default)]
pub struct Args {
    pub map: Map<String, Value>,
}

impl Args {
    fn new(params: &Params, body: &Body) -> Self {
        let mut map = Map::new();
        for key in params_keys(params) {
            if let Some(list) = params.list(&key).filter(|_| params.raw().contains(&format!("{key}[]")) || params.raw().contains(&format!("{key}%5B"))) {
                map.insert(key.clone(), Value::from(list));
            } else if let Some(value) = params.get(&key) {
                map.insert(key.clone(), Value::from(value));
            }
        }
        match body {
            Body::Json(Value::Object(object)) => {
                for (k, v) in object {
                    map.insert(k.clone(), v.clone());
                }
            }
            Body::Form(pairs) => {
                for (k, v) in pairs {
                    map.insert(k.trim_end_matches("[]").to_owned(), Value::from(v.clone()));
                }
            }
            Body::Multipart { fields, .. } => {
                for (k, v) in fields {
                    map.insert(k.clone(), Value::from(v.clone()));
                }
            }
            _ => {}
        }
        Self { map }
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.map.get(key)
    }

    /// A string parameter; numbers and booleans are turned into text.
    pub fn str(&self, key: &str) -> Option<String> {
        match self.map.get(key)? {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(if *b { "1".into() } else { String::new() }),
            _ => None,
        }
    }

    pub fn u64(&self, key: &str) -> Option<u64> {
        match self.map.get(key)? {
            Value::Number(n) => n.as_u64().or_else(|| n.as_i64().map(|i| i.max(0) as u64)),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn i64(&self, key: &str) -> Option<i64> {
        match self.map.get(key)? {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        match self.map.get(key)? {
            Value::Bool(b) => Some(*b),
            Value::Number(n) => Some(n.as_i64().unwrap_or(0) != 0),
            Value::String(s) => Some(!matches!(s.as_str(), "" | "0" | "false")),
            _ => None,
        }
    }

    /// A list of IDs: `[1,2]`, `"1,2"`.
    pub fn ids(&self, key: &str) -> Option<Vec<u64>> {
        match self.map.get(key)? {
            Value::Array(items) => Some(items.iter().filter_map(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).collect()),
            Value::String(s) => Some(s.split(',').filter_map(|p| p.trim().parse().ok()).collect()),
            Value::Number(n) => n.as_u64().map(|n| vec![n]),
            _ => None,
        }
    }

    pub fn list(&self, key: &str) -> Option<Vec<String>> {
        match self.map.get(key)? {
            Value::Array(items) => Some(items.iter().filter_map(|v| v.as_str().map(str::to_owned).or_else(|| v.as_u64().map(|n| n.to_string()))).collect()),
            Value::String(s) => Some(s.split(',').map(|p| p.trim().to_owned()).filter(|p| !p.is_empty()).collect()),
            _ => None,
        }
    }

    /// A text field that may be sent as `"x"` or as `{"raw": "x"}`.
    pub fn raw_text(&self, key: &str) -> Option<String> {
        match self.map.get(key)? {
            Value::String(s) => Some(s.clone()),
            Value::Object(o) => o.get("raw").and_then(Value::as_str).map(str::to_owned),
            _ => None,
        }
    }
}

fn params_keys(params: &Params) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for pair in params.raw().split('&').filter(|p| !p.is_empty()) {
        let key = pair.split('=').next().unwrap_or_default();
        let decoded = String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(key.replace('+', " ").as_bytes())).into_owned();
        let name = decoded.split('[').next().unwrap_or_default().to_owned();
        if !keys.contains(&name) {
            keys.push(name);
        }
    }
    keys
}

impl Rest<'_> {
    pub(crate) fn user(&self) -> Option<&CurrentUser> {
        self.access.as_ref().map(|a| &a.user)
    }

    pub(crate) fn method(&self) -> &str {
        self.request.as_ref().map_or("GET", |r| r.method.as_str())
    }

    pub(crate) fn can(&self, capability: &str) -> bool {
        self.user().is_some_and(|u| u.can(capability))
    }

    /// The `Allow` header of an error answer.
    pub(crate) fn allow_for(&self, _route: &str) -> String {
        if self.access.is_some() { "GET, POST, PUT, PATCH, DELETE".to_owned() } else { "GET".to_owned() }
    }

    /// The not-logged-in or forbidden error WordPress sends for an action.
    pub(crate) fn forbidden(&self, code: &str, message: &str) -> RestError {
        RestError::new(code, message, if self.user().is_some() { 403 } else { 401 })
    }

    /// Routes a request that is not a plain anonymous read. `None` leaves it to the read-only endpoints.
    pub(crate) async fn dispatch_edit(&self, route: &str, params: &Params) -> Result<Option<Outcome>> {
        let route = route.trim_end_matches('/');
        let segments: Vec<&str> = route.trim_start_matches('/').split('/').collect();
        let method = self.method().to_owned();
        if method == "OPTIONS" {
            return Ok(Some(self.options(route)));
        }
        let body = self.request.as_ref().map(|r| r.body.clone()).unwrap_or_default();
        let args = Args::new(params, &body);
        let get = method == "GET";
        // The index of a namespace.
        if get && crate::namespaces().iter().any(|ns| *ns == segments.join("/")) {
            return Ok(Some(Ok(crate::Resp::item(self.namespace_index(&segments.join("/"))))));
        }
        if let ["wp", "v2", "font-families" | "font-collections", ..] = segments.as_slice() {
            return self.fonts_route(&segments, &method, &args, params).await.map(Some);
        }
        if self.access.is_none() && get {
            return Ok(None);
        }
        let edit_read = params.get("context") == Some("edit");
        match segments.as_slice() {
            ["wp", "v2", "users", id, "application-passwords"] => self.app_passwords_route(id, None, &method, &args).await.map(Some),
            ["wp", "v2", "users", id, "application-passwords", uuid] => self.app_passwords_route(id, Some(uuid), &method, &args).await.map(Some),
            ["wp", "v2", "users", "me"] => self.users_me(&method, &args).await.map(Some),
            ["wp", "v2", "users"] if get && (self.access.is_some()) => self.users_list(&args).await.map(Some),
            ["wp", "v2", "users", id] if get && self.access.is_some() && id.parse::<u64>().is_ok() => self.users_get(id, &args).await.map(Some),
            ["wp", "v2", "plugins", ..] => self.plugins_route(&segments, &method, &args).await.map(Some),
            ["wp", "v2", "settings"] => self.settings(&method, &args).await.map(Some),
            ["wp", "v2", "types"] if get && self.access.is_some() => Ok(Some(self.types_list_edit(&args))),
            ["wp", "v2", "types", name] if get && self.access.is_some() => Ok(Some(self.type_get_edit(name, &args))),
            ["wp", "v2", "taxonomies"] if get && self.access.is_some() => Ok(Some(self.taxonomies_list_edit(&args))),
            ["wp", "v2", "taxonomies", name] if get && self.access.is_some() => Ok(Some(self.taxonomy_get_edit(name, &args))),
            ["wp", "v2", base @ ("posts" | "pages")] | ["wp", "v2", base @ ("posts" | "pages"), _] if !get || edit_read || params.get("status").is_some() || self.access.is_some() => {
                let kind = if *base == "posts" { crate::posts::Kind::Post } else { crate::posts::Kind::Page };
                self.posts_route(kind, segments.get(3).copied(), &method, &args, params).await.map(Some)
            }
            ["wp", "v2", base @ ("posts" | "pages"), parent, sub @ ("autosaves" | "revisions")] | ["wp", "v2", base @ ("posts" | "pages"), parent, sub @ ("autosaves" | "revisions"), _] => {
                let kind = if *base == "posts" { crate::posts::Kind::Post } else { crate::posts::Kind::Page };
                self.revisions_route(kind, parent, sub, segments.get(5).copied(), &method, &args).await.map(Some)
            }
            ["wp", "v2", "media"] | ["wp", "v2", "media", _] if !get || self.access.is_some() => self.media_route(segments.get(3).copied(), &method, &args, params).await.map(Some),
            ["wp", "v2", base @ ("categories" | "tags" | "wp_pattern_category")] | ["wp", "v2", base @ ("categories" | "tags" | "wp_pattern_category"), _] if !get || edit_read => {
                let tax = match *base {
                    "categories" => crate::terms::Tax::category(),
                    "tags" => crate::terms::Tax::tag(),
                    _ => crate::terms::Tax::pattern_category(),
                };
                self.terms_route(&tax, segments.get(3).copied(), &method, &args, params).await.map(Some)
            }
            ["wp", "v2", base] | ["wp", "v2", base, _] if self.custom_route(base).is_some() && (!get || edit_read) => self.custom_edit_route(base, segments.get(3).copied(), &method, &args, params).await.map(Some),
            ["wp", "v2", "templates", ..] => self.templates_route(templates::TKind::Template, &segments, &method, &args, params).await.map(Some),
            ["wp", "v2", "template-parts", ..] => self.templates_route(templates::TKind::Part, &segments, &method, &args, params).await.map(Some),
            ["wp", "v2", "block-renderer", namespace, name] if get || method == "POST" => self.block_renderer(namespace, name, &args, params).await.map(Some),
            ["oembed", "1.0", "proxy"] if get && args.str("url").is_none() => Ok(Some(Err(RestError::missing_params(&["url"])))),
            ["oembed", "1.0", "proxy"] if get => Ok(Some(if self.can("edit_posts") { Err(RestError::new("oembed_invalid_url", &self.site.tr("Not Found"), 404).after_permission_check()) } else { Err(self.forbidden("rest_forbidden", &self.site.tr("Sorry, you are not allowed to make proxied oEmbed requests."))) })),
            ["wp-site-health", "v1", ..] if get => self.health_route(&segments).await.map(Some),
            ["wp", "v2", "view-config"] if get => self.view_config(&args).await.map(Some),
            ["wp", "v2", "icons", ..] | ["wp", "v2", "icon-collections", ..] if get => Ok(Some(self.icons_route(&segments, &args))),
            ["wp-block-editor", "v1", "navigation-fallback"] if get => self.navigation_fallback(&args).await.map(Some),
            _ => self.editor_route(&segments, &method, &args, params).await,
        }
    }

    /// `OPTIONS`: the methods the user may use. The schema stays empty.
    fn options(&self, route: &str) -> Outcome {
        let mut methods = vec!["GET"];
        let writable = self.access.is_some();
        let route_ends_with_id = route.rsplit('/').next().is_some_and(|s| s.parse::<u64>().is_ok());
        if writable && self.can("edit_posts") {
            methods.push("POST");
            if route_ends_with_id {
                methods.extend(["PUT", "PATCH", "DELETE"]);
            }
        }
        let mut resp = crate::Resp::item(serde_json::json!({"namespace": "wp/v2", "methods": methods, "endpoints": [{"methods": methods, "args": {}}], "schema": {}}));
        resp.allow = Some(methods.join(", "));
        Ok(resp)
    }

    /// `rest_ensure_response()` of a created item.
    pub(crate) fn created(&self, mut resp: crate::Resp, location: String) -> crate::Resp {
        resp.status = Some(201);
        resp.location = Some(location);
        resp
    }

    pub(crate) fn writer(&self) -> std::result::Result<&Writer, RestError> {
        self.access.as_ref().and_then(|a| a.writer).ok_or_else(|| RestError::new("rest_cannot_edit", "RustPress was started without write access.", 500))
    }

    pub(crate) fn content(&self) -> std::result::Result<rustpress_content::content::Content<'_>, RestError> {
        let access = self.access.as_ref().ok_or_else(|| RestError::new("rest_forbidden", "Sorry, you are not allowed to do that.", 401))?;
        let writer = access.writer.ok_or_else(|| RestError::new("rest_cannot_edit", "RustPress was started without write access.", 500))?;
        Ok(rustpress_content::content::Content { site: self.site, db: &self.site.db, writer, user: &access.user, now: access.now })
    }

    pub(crate) fn now(&self) -> i64 {
        self.access.as_ref().map_or(0, |a| a.now)
    }
}
