//! `wp/v2/plugins` (`WP_REST_Plugins_Controller`): the installed plugins and whether they are active.
//! Installing needs WordPress.org, which RustPress does not talk to; deleting is left to the plugins screen.

use rustpress_core::plugins::{self, PluginInfo, kses_description};
use serde_json::{Value, json};

use super::Args;
use crate::error::{Msg, RestError};
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

/// `plugins/akismet/akismet`: the plugin file without `.php`.
fn plugin_id(info: &PluginInfo) -> String {
    info.file.strip_suffix(".php").unwrap_or(&info.file).to_owned()
}

impl Rest<'_> {
    fn active_plugins(&self) -> Vec<String> {
        match self.site.options.value("active_plugins") {
            Some(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
            _ => Vec::new(),
        }
    }

    fn prepare_plugin(&self, info: &PluginInfo, active: bool) -> Value {
        let description = kses_description(&info.description);
        let author = if info.author_uri.is_empty() { info.author.clone() } else { format!("<a href=\"{}\">{}</a>", info.author_uri, info.author) };
        let rendered = if info.author.is_empty() { description.clone() } else { format!("{description} <cite>By {author}.</cite>") };
        let mut links = Links::new();
        let allow = if self.can("activate_plugins") { vec!["GET", "POST", "PUT", "PATCH", "DELETE"] } else { vec!["GET"] };
        links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/plugins/{}", plugin_id(info))), "targetHints": {"allow": allow}}));
        json!({
            "plugin": plugin_id(info),
            "status": if active { "active" } else { "inactive" },
            "name": info.name,
            "plugin_uri": info.plugin_uri,
            "author": info.author,
            "author_uri": info.author_uri,
            "description": {"raw": description, "rendered": rendered},
            "version": info.version,
            "network_only": info.network,
            "requires_wp": info.requires_wp,
            "requires_php": info.requires_php,
            "textdomain": info.text_domain,
            "_links": links.finish(),
        })
    }

    pub(crate) async fn plugins_route(&self, segments: &[&str], method: &str, args: &Args) -> Result<Outcome> {
        let id = segments[3..].join("/");
        let single = !id.is_empty();
        let denied = if single { "rest_cannot_view_plugin" } else { "rest_cannot_view_plugins" };
        let managing = matches!(method, "POST" | "PUT" | "PATCH" | "DELETE");
        if !self.can("activate_plugins") {
            let code = if managing { "rest_cannot_manage_plugins" } else { denied };
            return Ok(Err(self.forbidden(code, "Sorry, you are not allowed to manage plugins for this site.")));
        }
        let installed = plugins::installed(&self.theme.wp_root);
        let mut active = self.active_plugins();
        match (method, single) {
            ("GET", false) => {
                let search = args.str("search").map(|s| s.to_lowercase()).filter(|s| !s.is_empty());
                let wanted = args.list("status").unwrap_or_default();
                for status in &wanted {
                    if !matches!(status.as_str(), "active" | "inactive") {
                        return Ok(Err(RestError::invalid_param_with_data("status", Msg::new("%1$s is not one of %2$s.").with(&["status[0]", "inactive and active"]), "rest_not_in_enum", Value::Null)));
                    }
                }
                let items: Vec<Value> = installed
                    .iter()
                    .filter(|p| search.as_ref().is_none_or(|s| [&p.name, &p.description, &p.author, &p.version].iter().any(|f| f.to_lowercase().contains(s))))
                    .filter(|p| wanted.is_empty() || wanted.contains(&(if active.contains(&p.file) { "active" } else { "inactive" }).to_owned()))
                    .map(|p| self.prepare_plugin(p, active.contains(&p.file)))
                    .collect();
                let mut resp = Resp::item(Value::Array(items));
                resp.allow = Some("GET, POST".to_owned());
                Ok(Ok(resp))
            }
            ("GET", true) | ("PUT" | "PATCH" | "POST", true) => {
                let Some(info) = installed.iter().find(|p| plugin_id(p) == id) else { return Ok(Err(RestError::new("rest_plugin_not_found", "Plugin not found.", 404))) };
                if method != "GET" {
                    if let Some(status) = args.str("status") {
                        if !matches!(status.as_str(), "active" | "inactive") {
                            return Ok(Err(RestError::invalid_param_with_data("status", Msg::new("%1$s is not one of %2$s.").with(&["status", "inactive and active"]), "rest_not_in_enum", Value::Null)));
                        }
                        let writer = match self.writer() {
                            Ok(w) => w,
                            Err(e) => return Ok(Err(e)),
                        };
                        let now_active = active.contains(&info.file);
                        if status == "active" && !now_active {
                            active.push(info.file.clone());
                        } else if status == "inactive" && now_active {
                            active.retain(|f| f != &info.file);
                        } else {
                            return Ok(Ok(Resp::item(self.prepare_plugin(info, now_active))));
                        }
                        let value = rustpress_phpser::Value::Array(active.iter().enumerate().map(|(i, s)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Str(s.as_bytes().to_vec()))).collect());
                        let text = String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned();
                        if writer.update_option("active_plugins", &text).await.is_err() {
                            return Ok(Err(RestError::new("rest_cannot_edit", "The plugin status could not be changed.", 500)));
                        }
                    }
                }
                Ok(Ok(Resp::item(self.prepare_plugin(info, active.contains(&info.file)))))
            }
            ("POST", false) => Ok(Err(RestError::new("rest_cannot_install_plugin", "Sorry, you are not allowed to install plugins on this site.", 403))),
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }
}
