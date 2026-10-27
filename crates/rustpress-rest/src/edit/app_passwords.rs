//! `wp/v2/users/<id|me>/application-passwords` (`WP_REST_Application_Passwords_Controller`).

use rustpress_core::OrLog;
use rustpress_auth::app_passwords::{self, AppPassword};
use serde_json::{Value, json};

use super::Args;
use crate::error::{Msg, RestError};
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

/// `sanitize_text_field()`: no tags, no line breaks, trimmed.
fn sanitize_text_field(text: &str) -> String {
    let stripped = rustpress_core::formatting::strip_tags(text);
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn mysql_time(timestamp: i64) -> String {
    rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d\\TH:i:s", timestamp)
}

/// The schema of `name` (`minLength` 1, `pattern` `.*\S.*`) as `rest_validate_value_from_schema()` checks it.
fn invalid_name(name: &str) -> Option<RestError> {
    if name.is_empty() {
        return Some(RestError::invalid_param("name", Msg::new("%1$s must be at least %2$s character long.").with(&["name", "1"]), "rest_too_short"));
    }
    // `\S` of PCRE: anything but ASCII white space.
    if name.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')) {
        return Some(RestError::invalid_param("name", Msg::new("%1$s does not match pattern %2$s.").with(&["name", ".*\\S.*"]), "rest_invalid_pattern"));
    }
    None
}

impl Rest<'_> {
    /// `get_user()` of the controller: the user the route is about, or the error WordPress answers with.
    async fn app_password_user(&self, id: &str) -> std::result::Result<u64, RestError> {
        let Some(me) = self.user() else { return Err(RestError::new("rest_not_logged_in", "You are not currently logged in.", 401)) };
        if !self.site.app_passwords_available() {
            return Err(RestError::new("application_passwords_disabled", "Application passwords are not available.", 501));
        }
        let target = if id == "me" {
            me.id
        } else {
            let Ok(number) = id.parse::<u64>() else { return Err(RestError::new("rest_user_invalid_id", "Invalid user ID.", 404)) };
            if self.site.db.user(number).await.map_err(|e| RestError::new("rest_error", &e.to_string(), 500))?.is_none() {
                return Err(RestError::new("rest_user_invalid_id", "Invalid user ID.", 404));
            }
            number
        };
        Ok(target)
    }

    fn app_password_allowed(&self, target: u64) -> bool {
        self.user().is_some_and(|me| me.id == target) || self.can("edit_users")
    }

    fn prepare_app_password(&self, user: u64, item: &AppPassword, with_links: bool) -> Value {
        let mut data = json!({
            "uuid": item.uuid,
            "app_id": item.app_id,
            "name": item.name,
            "created": mysql_time(item.created),
            "last_used": item.last_used.map(mysql_time),
            "last_ip": item.last_ip,
        });
        if with_links {
            let mut self_link = serde_json::Map::new();
            self_link.insert("href".into(), json!(self.rest_url(&format!("wp/v2/users/{user}/application-passwords/{}", item.uuid))));
            self_link.insert("targetHints".into(), json!({"allow": ["GET", "POST", "PUT", "PATCH", "DELETE"]}));
            let mut links = Links::new();
            links.push_raw("self", Value::Object(self_link));
            data["_links"] = links.finish();
        }
        data
    }

    /// The routes below `users/<id>/application-passwords`.
    pub(crate) async fn app_passwords_route(&self, user_id: &str, uuid: Option<&str>, method: &str, args: &Args) -> Result<Outcome> {
        let target = match self.app_password_user(user_id).await {
            Ok(target) => target,
            Err(e) => return Ok(Err(e)),
        };
        let forbidden = |code: &str, message: &str| Ok(Err(self.forbidden(code, message)));
        let db = &self.site.db;
        let mut list = app_passwords::list(db, target).await;

        if uuid == Some("introspect") {
            if method != "GET" {
                return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
            }
            let current = self.access.as_ref().and_then(|a| a.app_password.clone());
            let Some(current) = current else {
                return Ok(Err(RestError::new("rest_no_authenticated_app_password", "Cannot introspect application password.", 404).after_permission_check()));
            };
            let Some(item) = list.iter().find(|p| p.uuid == current) else {
                return Ok(Err(RestError::new("rest_application_password_not_found", "Application password not found.", 404).after_permission_check()));
            };
            return Ok(Ok(Resp::item(self.prepare_app_password(target, item, true))));
        }

        match (uuid, method) {
            (None, "GET") => {
                if !self.app_password_allowed(target) {
                    return forbidden("rest_cannot_list_application_passwords", "Sorry, you are not allowed to list application passwords for this user.");
                }
                let items: Vec<Value> = list.iter().map(|p| self.prepare_app_password(target, p, true)).collect();
                Ok(Ok(Resp::item(Value::Array(items))))
            }
            (None, "POST") => {
                if !self.app_password_allowed(target) {
                    return forbidden("rest_cannot_create_application_passwords", "Sorry, you are not allowed to create application passwords for this user.");
                }
                let name = args.str("name").unwrap_or_default();
                if args.get("name").is_none() {
                    return Ok(Err(RestError::missing_params(&["name"])));
                }
                if let Some(error) = invalid_name(&name) {
                    return Ok(Err(error));
                }
                let app_id = args.str("app_id").unwrap_or_default();
                let writer = match self.writer() {
                    Ok(writer) => writer,
                    Err(e) => return Ok(Err(e)),
                };
                let plain = app_passwords::new_password();
                let item = AppPassword { uuid: app_passwords::new_uuid(), app_id, name: sanitize_text_field(&name), password: app_passwords::hash(&plain), created: self.access.as_ref().map_or(0, |a| a.now), last_used: None, last_ip: None };
                list.push(item.clone());
                if let Err(e) = app_passwords::save(writer, target, &list).await {
                    return Ok(Err(RestError::new("rest_cannot_create_application_password", &e.to_string(), 500)));
                }
                let mut data = self.prepare_app_password(target, &item, true);
                data["password"] = json!(app_passwords::display(&plain));
                // WordPress puts the password before the links.
                if let Value::Object(map) = &mut data {
                    let links = map.remove("_links");
                    if let Some(links) = links {
                        map.insert("_links".into(), links);
                    }
                }
                let mut resp = Resp::item(data);
                resp.status = Some(201);
                resp.location = Some(self.rest_url(&format!("wp/v2/users/{target}/application-passwords/{}", item.uuid)));
                Ok(Ok(resp))
            }
            (None, "DELETE") => {
                if !self.app_password_allowed(target) {
                    return forbidden("rest_cannot_delete_application_passwords", "Sorry, you are not allowed to delete application passwords for this user.");
                }
                let writer = match self.writer() {
                    Ok(writer) => writer,
                    Err(e) => return Ok(Err(e)),
                };
                let count = list.len();
                if count > 0 {
                    writer.delete_user_meta(target, app_passwords::META_KEY).await.or_log();
                }
                Ok(Ok(Resp::item(json!({"deleted": true, "count": count}))))
            }
            (Some(uuid), "GET") => {
                if !self.app_password_allowed(target) {
                    return forbidden("rest_cannot_read_application_password", "Sorry, you are not allowed to read this application password.");
                }
                let Some(item) = list.iter().find(|p| p.uuid == uuid) else {
                    return Ok(Err(RestError::new("rest_application_password_not_found", "Application password not found.", 404).after_permission_check()));
                };
                Ok(Ok(Resp::item(self.prepare_app_password(target, item, true))))
            }
            (Some(uuid), "POST" | "PUT" | "PATCH") => {
                if !self.app_password_allowed(target) {
                    return forbidden("rest_cannot_edit_application_password", "Sorry, you are not allowed to edit this application password.");
                }
                let writer = match self.writer() {
                    Ok(writer) => writer,
                    Err(e) => return Ok(Err(e)),
                };
                let Some(item) = list.iter_mut().find(|p| p.uuid == uuid) else {
                    return Ok(Err(RestError::new("rest_application_password_not_found", "Application password not found.", 404).after_permission_check()));
                };
                if let Some(name) = args.str("name") {
                    if let Some(error) = invalid_name(&name) {
                        return Ok(Err(error));
                    }
                    item.name = sanitize_text_field(&name);
                }
                if let Some(app_id) = args.str("app_id") {
                    item.app_id = app_id;
                }
                let updated = item.clone();
                if let Err(e) = app_passwords::save(writer, target, &list).await {
                    return Ok(Err(RestError::new("rest_cannot_update_application_password", &e.to_string(), 500)));
                }
                Ok(Ok(Resp::item(self.prepare_app_password(target, &updated, true))))
            }
            (Some(uuid), "DELETE") => {
                if !self.app_password_allowed(target) {
                    return forbidden("rest_cannot_delete_application_password", "Sorry, you are not allowed to delete this application password.");
                }
                let writer = match self.writer() {
                    Ok(writer) => writer,
                    Err(e) => return Ok(Err(e)),
                };
                let Some(index) = list.iter().position(|p| p.uuid == uuid) else {
                    return Ok(Err(RestError::new("rest_application_password_not_found", "Application password not found.", 404).after_permission_check()));
                };
                let previous = list.remove(index);
                if let Err(e) = app_passwords::save(writer, target, &list).await {
                    return Ok(Err(RestError::new("rest_cannot_delete_application_password", &e.to_string(), 500)));
                }
                Ok(Ok(Resp::item(json!({"deleted": true, "previous": self.prepare_app_password(target, &previous, false)}))))
            }
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }
}
