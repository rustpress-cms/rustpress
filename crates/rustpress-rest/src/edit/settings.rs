//! `wp/v2/settings` (`WP_REST_Settings_Controller`).

use serde_json::{Map, Value, json};

use super::Args;
use crate::error::RestError;
use crate::{Outcome, Resp, Rest, Result};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Text,
    Int,
    Bool,
}

/// `(REST name, option name, type)` of the settings WordPress registers for the REST API.
const SETTINGS: &[(&str, &str, Kind)] = &[
    ("title", "blogname", Kind::Text),
    ("description", "blogdescription", Kind::Text),
    ("url", "siteurl", Kind::Text),
    ("email", "admin_email", Kind::Text),
    ("timezone", "timezone_string", Kind::Text),
    ("date_format", "date_format", Kind::Text),
    ("time_format", "time_format", Kind::Text),
    ("start_of_week", "start_of_week", Kind::Int),
    ("language", "WPLANG", Kind::Text),
    ("use_smilies", "use_smilies", Kind::Bool),
    ("default_category", "default_category", Kind::Int),
    ("default_post_format", "default_post_format", Kind::Text),
    ("posts_per_page", "posts_per_page", Kind::Int),
    ("show_on_front", "show_on_front", Kind::Text),
    ("page_on_front", "page_on_front", Kind::Int),
    ("page_for_posts", "page_for_posts", Kind::Int),
    ("default_ping_status", "default_ping_status", Kind::Text),
    ("default_comment_status", "default_comment_status", Kind::Text),
    ("site_logo", "site_logo", Kind::Int),
    ("site_icon", "site_icon", Kind::Int),
];

impl Rest<'_> {
    fn settings_value(&self, option: &str, kind: Kind) -> Value {
        let raw = self.site.options.string(option, "");
        match kind {
            Kind::Text => {
                if option == "siteurl" {
                    return json!(raw.trim_end_matches('/'));
                }
                if option == "WPLANG" && raw == "en_US" {
                    return json!("");
                }
                json!(raw)
            }
            Kind::Int => json!(raw.trim().parse::<i64>().unwrap_or(0)),
            Kind::Bool => json!(matches!(raw.as_str(), "1" | "true")),
        }
    }

    fn settings_body(&self) -> Value {
        let mut out = Map::new();
        for (name, option, kind) in SETTINGS {
            let mut value = self.settings_value(option, *kind);
            if *option == "default_post_format" && value == json!("") {
                value = json!("0");
            }
            if *option == "use_smilies" && self.site.options.get("use_smilies").is_none() {
                value = json!(true);
            }
            out.insert((*name).to_owned(), value);
        }
        Value::Object(out)
    }

    pub(crate) async fn settings(&self, method: &str, args: &Args) -> Result<Outcome> {
        if !self.can("manage_options") {
            return Ok(Err(self.forbidden("rest_cannot_view", "Sorry, you are not allowed to do that.")));
        }
        if matches!(method, "POST" | "PUT" | "PATCH") {
            let writer = match self.writer() {
                Ok(w) => w,
                Err(e) => return Ok(Err(e)),
            };
            for (name, option, kind) in SETTINGS {
                let Some(value) = args.get(name) else { continue };
                let stored = match (kind, value) {
                    (Kind::Bool, Value::Bool(b)) => (if *b { "1" } else { "0" }).to_owned(),
                    (Kind::Bool, other) => (if args.bool(name).unwrap_or(false) || other == &json!(1) { "1" } else { "0" }).to_owned(),
                    (Kind::Int, _) => args.i64(name).unwrap_or(0).to_string(),
                    (Kind::Text, Value::Null) => String::new(),
                    (Kind::Text, _) => args.str(name).unwrap_or_default(),
                };
                let stored = if *option == "blogname" || *option == "blogdescription" { rustpress_core::formatting::esc_html(&stored) } else { stored };
                if let Err(e) = writer.update_option(option, &stored).await {
                    return Ok(Err(RestError::new("rest_cannot_update", &e.to_string(), 500)));
                }
            }
        }
        // The options are cached; a write is visible after the next reload, so the answer reflects the request.
        let mut body = self.settings_body();
        if matches!(method, "POST" | "PUT" | "PATCH")
            && let Value::Object(map) = &mut body
        {
            for (name, option, kind) in SETTINGS {
                if let Some(value) = args.get(name) {
                    let shown = match kind {
                        Kind::Int => json!(args.i64(name).unwrap_or(0)),
                        Kind::Bool => json!(args.bool(name).unwrap_or(false)),
                        Kind::Text => json!(args.str(name).unwrap_or_default()),
                    };
                    let _ = (option, value);
                    map.insert((*name).to_owned(), shown);
                }
            }
        }
        let mut resp = Resp::item(body);
        resp.allow = Some("GET, POST, PUT, PATCH".to_owned());
        Ok(Ok(resp))
    }
}
