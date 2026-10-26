//! `wp/v2/users` for signed-in users (`WP_REST_Users_Controller`), including `users/me`.

use rustpress_content::content::UserData;
use rustpress_db::User;
use serde_json::{Map, Value, json};

use super::Args;
use crate::error::RestError;
use crate::util::{Links, rfc3339};
use crate::{Outcome, Resp, Rest, Result};

/// A PHP value as JSON, for the preferences stored in user meta.
pub(crate) fn php_to_json(value: &rustpress_phpser::Value) -> Value {
    rustpress_core::options::php_to_json(value)
}

/// JSON as a PHP value (objects become associative arrays).
pub(crate) fn json_to_php(value: &Value) -> rustpress_phpser::Value {
    use rustpress_phpser::{Key, Value as Php};
    match value {
        Value::Null => Php::Null,
        Value::Bool(b) => Php::Bool(*b),
        Value::Number(n) => n.as_i64().map_or_else(|| Php::Float(n.as_f64().unwrap_or(0.0)), Php::Int),
        Value::String(s) => Php::Str(s.as_bytes().to_vec()),
        Value::Array(items) => Php::Array(items.iter().enumerate().map(|(i, v)| (Key::Int(i as i64), json_to_php(v))).collect()),
        Value::Object(map) => Php::Array(map.iter().map(|(k, v)| (Key::Str(k.as_bytes().to_vec()), json_to_php(v))).collect()),
    }
}

impl Rest<'_> {
    pub(crate) fn user_allow(&self, id: u64) -> Vec<&'static str> {
        let me = self.user().map_or(0, |u| u.id);
        let mut allow = vec!["GET"];
        if self.can("edit_users") || id == me {
            allow.extend(["POST", "PUT", "PATCH"]);
        }
        if self.can("delete_users") || id == me && self.can("read") && false {
            allow.push("DELETE");
        } else if id == me {
            allow.push("DELETE");
        }
        allow
    }

    /// `prepare_item_for_response()` for the `edit` (and `view`) context of a signed-in user.
    pub(crate) async fn prepare_user_edit(&self, user: &User, context: &str) -> Result<Value> {
        let db = &self.site.db;
        let prefix = db.table_prefix().to_owned();
        let meta = |key: String| async move { db.user_meta(user.id, &key).await };
        let mut data = Map::new();
        data.insert("id".into(), json!(user.id));
        let can_see_private = context == "edit";
        if can_see_private {
            data.insert("username".into(), json!(user.user_login));
        }
        data.insert("name".into(), json!(user.display_name));
        if can_see_private {
            data.insert("first_name".into(), json!(meta("first_name".into()).await?.unwrap_or_default()));
            data.insert("last_name".into(), json!(meta("last_name".into()).await?.unwrap_or_default()));
            data.insert("email".into(), json!(user.user_email));
        }
        data.insert("url".into(), json!(user.user_url));
        data.insert("description".into(), json!(meta("description".into()).await?.unwrap_or_default()));
        data.insert("link".into(), json!(self.site.author_url(user)));
        if can_see_private {
            let locale = meta("locale".into()).await?.filter(|l| !l.is_empty()).unwrap_or_else(|| self.site.locale());
            data.insert("locale".into(), json!(locale));
            data.insert("nickname".into(), json!(meta("nickname".into()).await?.unwrap_or_default()));
        }
        data.insert("slug".into(), json!(user.user_nicename));
        if can_see_private {
            let caps_raw = meta(format!("{prefix}capabilities")).await?.unwrap_or_default();
            let caps_value = rustpress_phpser::unserialize(caps_raw.as_bytes()).ok();
            let mut roles: Vec<String> = Vec::new();
            let mut extra = Map::new();
            if let Some(rustpress_phpser::Value::Array(items)) = &caps_value {
                for (key, value) in items {
                    if let rustpress_phpser::Key::Str(name) = key {
                        let name = String::from_utf8_lossy(name).into_owned();
                        let granted = value.as_i64() == Some(1);
                        extra.insert(name.clone(), Value::Bool(granted));
                        if granted && self.role_exists(&name) {
                            roles.push(name);
                        }
                    }
                }
            }
            data.insert("roles".into(), json!(roles));
            let registered = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&user.user_registered).and_then(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)).map(|d| d.format("%Y-%m-%dT%H:%M:%S+00:00").to_string()).unwrap_or_else(|| rfc3339(&user.user_registered));
            data.insert("registered_date".into(), json!(registered));
            if self.user().is_some_and(|u| u.id == user.id) {
                let me = self.user().expect("user");
                let mut caps = Map::new();
                for (name, granted) in &me.allcaps {
                    caps.insert(name.clone(), Value::Bool(*granted));
                }
                data.insert("capabilities".into(), Value::Object(caps));
            } else {
                data.insert("capabilities".into(), Value::Object(Map::new()));
            }
            data.insert("extra_capabilities".into(), Value::Object(extra));
        }
        if self.site.show_avatars() {
            data.insert("avatar_urls".into(), self.avatar_urls(&user.user_email));
        }
        // The meta the editor stores for the user.
        let preferences = if context == "edit" { meta(format!("{prefix}persisted_preferences")).await? } else { None };
        let mut meta_object = Map::new();
        if let Some(raw) = preferences.filter(|r| !r.is_empty())
            && let Ok(value) = rustpress_phpser::unserialize(raw.as_bytes())
        {
            meta_object.insert("persisted_preferences".into(), php_to_json(&value));
        }
        data.insert("meta".into(), if meta_object.is_empty() { Value::Array(Vec::new()) } else { Value::Object(meta_object) });
        let mut links = Links::new();
        let allow: Vec<Value> = self.user_allow(user.id).into_iter().map(Value::from).collect();
        let mut self_link = Map::new();
        self_link.insert("href".into(), json!(self.rest_url(&format!("wp/v2/users/{}", user.id))));
        self_link.insert("targetHints".into(), json!({"allow": allow}));
        links.push_raw("self", Value::Object(self_link));
        links.add("collection", vec![], self.rest_url("wp/v2/users"));
        data.insert("_links".into(), links.finish());
        Ok(Value::Object(data))
    }

    fn role_exists(&self, name: &str) -> bool {
        let key = format!("{}user_roles", self.site.db.table_prefix());
        self.site.options.get(&key).is_some_and(|raw| rustpress_auth::Roles::from_option(raw).contains(name))
    }

    pub(crate) async fn users_me(&self, method: &str, args: &Args) -> Result<Outcome> {
        let Some(me) = self.user() else {
            return Ok(Err(RestError::new("rest_not_logged_in", "You are not currently logged in.", 401)));
        };
        let Some(user) = self.site.db.user(me.id).await? else {
            return Ok(Err(RestError::new("rest_user_invalid_id", "Invalid user ID.", 404)));
        };
        if matches!(method, "POST" | "PUT" | "PATCH") {
            if let Err(e) = self.update_user_fields(user.id, args).await {
                return Ok(Err(e));
            }
        }
        let user = self.site.db.user(me.id).await?.unwrap_or(user);
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        let mut resp = Resp::item(self.prepare_user_edit(&user, &context).await?);
        resp.allow = Some(self.user_allow(user.id).join(", "));
        Ok(Ok(resp))
    }

    async fn update_user_fields(&self, id: u64, args: &Args) -> std::result::Result<(), RestError> {
        let content = self.content()?;
        let existing = self.site.db.user(id).await.map_err(|e| RestError::new("rest_error", &e.to_string(), 500))?;
        let Some(_) = existing else { return Err(RestError::new("rest_user_invalid_id", "Invalid user ID.", 404)) };
        let data = UserData {
            id: Some(id),
            user_pass: args.str("password"),
            user_email: args.str("email"),
            user_url: args.str("url"),
            display_name: args.str("name"),
            nickname: args.str("nickname"),
            first_name: args.str("first_name"),
            last_name: args.str("last_name"),
            description: args.str("description"),
            locale: args.str("locale"),
            ..Default::default()
        };
        content.insert_user(&data).await.map_err(|e| RestError::new("rest_user_invalid_argument", &e.to_string(), 400))?;
        if let Some(Value::Object(meta)) = args.get("meta")
            && let Some(preferences) = meta.get("persisted_preferences")
        {
            let writer = self.writer()?;
            let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&json_to_php(preferences))).into_owned();
            writer.update_user_meta(id, &format!("{}persisted_preferences", self.site.db.table_prefix()), &serialized).await.map_err(|e| RestError::new("rest_error", &e.to_string(), 500))?;
        }
        Ok(())
    }

    pub(crate) async fn users_list(&self, args: &Args) -> Result<Outcome> {
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        if context == "edit" && !self.can("list_users") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to list users.", 403)));
        }
        let per_page = args.u64("per_page").unwrap_or(10).clamp(1, 100);
        let page = args.u64("page").unwrap_or(1).max(1);
        let search = args.str("search").unwrap_or_default().to_lowercase();
        let who_authors = args.str("who").as_deref() == Some("authors");
        let include = args.ids("include").unwrap_or_default();
        let exclude = args.ids("exclude").unwrap_or_default();
        let roles = args.list("roles").unwrap_or_default();
        let all = self.site.db.all_users().await?;
        let mut users: Vec<User> = Vec::new();
        for user in all {
            if !include.is_empty() && !include.contains(&user.id) || exclude.contains(&user.id) {
                continue;
            }
            if !search.is_empty() && ![&user.user_login, &user.user_nicename, &user.display_name, &user.user_email, &user.user_url].iter().any(|f| f.to_lowercase().contains(&search)) {
                continue;
            }
            if who_authors || !roles.is_empty() {
                let caps_raw = self.site.db.user_meta(user.id, &format!("{}capabilities", self.site.db.table_prefix())).await?.unwrap_or_default();
                let held: Vec<String> = match rustpress_phpser::unserialize(caps_raw.as_bytes()) {
                    Ok(rustpress_phpser::Value::Array(items)) => items.iter().filter(|(_, v)| v.as_i64() == Some(1)).filter_map(|(k, _)| match k {
                        rustpress_phpser::Key::Str(s) => Some(String::from_utf8_lossy(s).into_owned()),
                        _ => None,
                    }).collect(),
                    _ => Vec::new(),
                };
                if !roles.is_empty() && !roles.iter().any(|r| held.contains(r)) {
                    continue;
                }
                if who_authors && !held.iter().any(|r| matches!(r.as_str(), "administrator" | "editor" | "author")) {
                    continue;
                }
            }
            users.push(user);
        }
        match args.str("orderby").as_deref() {
            Some("id") => users.sort_by_key(|u| u.id),
            Some("registered_date") => users.sort_by(|a, b| a.user_registered.cmp(&b.user_registered)),
            Some("email") => users.sort_by(|a, b| a.user_email.to_lowercase().cmp(&b.user_email.to_lowercase())),
            Some("slug") => users.sort_by(|a, b| a.user_nicename.cmp(&b.user_nicename)),
            _ => users.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()).then(a.id.cmp(&b.id))),
        }
        if args.str("order").as_deref() == Some("desc") {
            users.reverse();
        }
        let total = users.len() as u64;
        let start = ((page - 1) * per_page) as usize;
        let mut items = Vec::new();
        for user in users.iter().skip(start).take(per_page as usize) {
            items.push(self.prepare_user_edit(user, &context).await?);
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(total.div_ceil(per_page));
        resp.allow = Some("GET, POST".to_owned());
        Ok(Ok(resp))
    }

    pub(crate) async fn users_get(&self, id: &str, args: &Args) -> Result<Outcome> {
        let id: u64 = id.parse().unwrap_or(0);
        let Some(user) = self.site.db.user(id).await? else {
            return Ok(Err(RestError::new("rest_user_invalid_id", "Invalid user ID.", 404)));
        };
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        if context == "edit" && !(self.can("edit_users") || self.user().is_some_and(|u| u.id == id)) {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit this user.", 403)));
        }
        let mut resp = Resp::item(self.prepare_user_edit(&user, &context).await?);
        resp.allow = Some(self.user_allow(id).join(", "));
        Ok(Ok(resp))
    }
}
