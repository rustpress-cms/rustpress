//! `WP_Error` as the REST API serializes it.
//!
//! Messages are WordPress message ids (the English texts) with their arguments; they are
//! translated into the site's language when the error is turned into JSON, so the endpoint code
//! never deals with languages.

use rustpress_core::Site;
use serde_json::{Map, Value, json};

/// One argument of a message: plain text, or a list for `wp_sprintf`'s `%l`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsgArg {
    Text(String),
    List(Vec<String>),
}

/// A translatable message with its `sprintf` arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Msg {
    id: String,
    args: Vec<MsgArg>,
}

impl Msg {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_owned(), args: Vec::new() }
    }

    /// Adds plain text arguments (`%s`, `%d`, `%1$s`, ...).
    pub fn with(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|a| MsgArg::Text((*a).to_owned())));
        self
    }

    /// Adds a list argument for `%l` (`a, b, and c`).
    pub fn with_list(mut self, items: &[&str]) -> Self {
        self.args.push(MsgArg::List(items.iter().map(|i| (*i).to_owned()).collect()));
        self
    }

    /// The message in the site's language.
    pub fn render(&self, site: &Site) -> String {
        let template = site.tr(&self.id);
        let args: Vec<String> = self
            .args
            .iter()
            .map(|a| match a {
                MsgArg::Text(t) => t.clone(),
                MsgArg::List(items) => list_and(site, items),
            })
            .collect();
        let template = template.replace("%2$l", "%2$s").replace("%1$l", "%1$s");
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        rustpress_core::i18n::sprintf(&template, &refs)
    }
}

impl From<&str> for Msg {
    fn from(id: &str) -> Self {
        Msg::new(id)
    }
}

impl From<&String> for Msg {
    fn from(id: &String) -> Self {
        Msg::new(id)
    }
}

/// `wp_sprintf_l()`: `a and b`, `a, b, and c`, with the separators of the language.
pub fn list_and(site: &Site, items: &[String]) -> String {
    let separator = |template: &str| rustpress_core::i18n::sprintf(&site.tr(template), &["", ""]);
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a}{}{b}", separator("%1$s and %2$s")),
        [rest @ .., last] => {
            let between = separator("%1$s, %2$s");
            format!("{}{}{last}", rest.join(&between), separator("%1$s, and %2$s"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParamError {
    param: String,
    message: Msg,
    code: String,
    /// The `WP_Error` data of the detail (the forbidden status case).
    data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestError {
    pub code: String,
    pub message: Msg,
    pub status: u16,
    param: Option<Box<ParamError>>,
    /// The error comes from the endpoint itself, after its permission check passed.
    after_permission: bool,
}

impl RestError {
    pub fn new(code: &str, message: impl Into<Msg>, status: u16) -> Self {
        Self { code: code.to_owned(), message: message.into(), status, param: None, after_permission: false }
    }

    /// `rest_invalid_param` for one parameter.
    pub fn invalid_param(param: &str, message: impl Into<Msg>, code: &str) -> Self {
        Self::invalid_param_with_data(param, message, code, Value::Null)
    }

    /// `rest_invalid_param` whose detail carries its own `WP_Error` data (the forbidden status case).
    pub fn invalid_param_with_data(param: &str, message: impl Into<Msg>, code: &str, data: Value) -> Self {
        let mut error = Self::new("rest_invalid_param", Msg::new("Invalid parameter(s): %s").with(&[param]), 400);
        error.param = Some(Box::new(ParamError { param: param.to_owned(), message: message.into(), code: code.to_owned(), data }));
        error
    }

    /// Whether the route''s permission check let the request through, i.e. the error comes from
    /// validating parameters or from the endpoint itself. WordPress then still sends `Allow: GET`
    /// (`rest_send_allow_header()`); errors of the permission check and unknown routes have none.
    pub fn passed_permission_check(&self) -> bool {
        self.after_permission
            || !matches!(
            self.code.as_str(),
            "rest_no_route"
                | "rest_forbidden_context"
                | "rest_cannot_read"
                | "rest_cannot_read_post"
                | "rest_post_invalid_id"
                | "rest_post_incorrect_password"
                | "rest_post_invalid_parent"
                | "rest_comment_invalid_id"
                | "rest_term_invalid"
                | "rest_user_invalid_id"
                | "rest_user_cannot_view"
                | "rest_not_logged_in"
                | "rest_cannot_view"
                | "rest_forbidden"
                | "rest_cannot_read_status"
                | "rest_status_invalid"
                | "rest_block_type_cannot_view"
                | "rest_cannot_view_active_theme"
                | "rest_cannot_view_themes"
                | "rest_cannot_view_plugins"
                | "rest_cannot_manage_widgets"
                | "rest_cannot_manage_templates"
                | "rest_cannot_read_global_styles"
                | "rest_block_directory_cannot_view"
                | "rest_pattern_directory_cannot_view"
        )
    }

    /// Marks an error that the endpoint callback returned (WordPress then still sends `Allow`).
    pub fn after_permission_check(mut self) -> Self {
        self.after_permission = true;
        self
    }

    pub fn to_value(&self, site: &Site) -> Value {
        let mut data = Map::new();
        data.insert("status".into(), Value::from(self.status));
        if let Some(p) = &self.param {
            let message = p.message.render(site);
            data.insert("params".into(), json!({ p.param.as_str(): message }));
            data.insert("details".into(), json!({ p.param.as_str(): {"code": p.code, "message": message, "data": p.data} }));
        }
        json!({"code": self.code, "message": self.message.render(site), "data": Value::Object(data)})
    }
}
