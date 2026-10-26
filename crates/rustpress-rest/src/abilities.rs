//! `wp-abilities/v1` (the Abilities API): the abilities WordPress registers itself, their categories, and running
//! them. The descriptions come from the reference (`dev/gen-abilities.py`); the results are worked out here.

use serde_json::{Map, Value, json};

use crate::edit::Args;
use crate::error::{Msg, RestError};
use crate::params::Params;
use crate::util::{LinkParams, link_header_with};
use crate::{Outcome, Resp, Rest, Result};

fn registry() -> &'static Value {
    static DATA: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let mut data: Value = serde_json::from_str(include_str!("abilities.json")).expect("the abilities are valid JSON");
        cast_empty_objects(&mut data);
        data
    })
}

/// An empty object is printed as `{}`, not as an empty PHP array.
fn cast_empty_objects(value: &mut Value) {
    match value {
        Value::Object(map) if map.is_empty() => *value = rustpress_blocks::php_json::empty_object(),
        Value::Object(map) => map.values_mut().for_each(cast_empty_objects),
        Value::Array(items) => items.iter_mut().for_each(cast_empty_objects),
        _ => {}
    }
}

fn items(key: &str) -> &'static [Value] {
    registry().get(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

/// The `input` of a request in the query string: `input[fields][]=name` and the like, as a tree.
fn query_input(raw: &str) -> Option<Value> {
    let mut root: Option<Value> = None;
    for pair in raw.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(key.replace('+', " ").as_bytes())).into_owned();
        let value = String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(value.replace('+', " ").as_bytes())).into_owned();
        if key == "input" {
            root = Some(Value::String(value));
        } else if let Some(rest) = key.strip_prefix("input[") {
            let path: Vec<&str> = rest.split('[').map(|p| p.trim_end_matches(']')).collect();
            let node = root.get_or_insert_with(|| Value::Object(Map::new()));
            if !node.is_object() {
                *node = Value::Object(Map::new());
            }
            insert(node, &path, value);
        }
    }
    root
}

fn insert(node: &mut Value, path: &[&str], value: String) {
    let Some((first, rest)) = path.split_first() else { return };
    if first.is_empty() {
        // `[]`: append.
        if !node.is_array() {
            *node = Value::Array(Vec::new());
        }
        if let Value::Array(items) = node {
            if rest.is_empty() {
                items.push(Value::String(value));
            } else {
                items.push(Value::Object(Map::new()));
                if let Some(last) = items.last_mut() {
                    insert(last, rest, value);
                }
            }
        }
        return;
    }
    if !node.is_object() {
        *node = Value::Object(Map::new());
    }
    if let Value::Object(map) = node {
        if rest.is_empty() {
            map.insert((*first).to_owned(), Value::String(value));
        } else {
            let child = map.entry((*first).to_owned()).or_insert(Value::Null);
            insert(child, rest, value);
        }
    }
}

impl Rest<'_> {
    pub(crate) async fn abilities_route(&self, segments: &[&str], method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let rest = &segments[2..];
        let denied = || self.forbidden("rest_forbidden", "Sorry, you are not allowed to do that.");
        match rest {
            ["abilities"] if method == "GET" => {
                if !self.can("read") {
                    return Ok(Err(denied()));
                }
                self.abilities_list(params)
            }
            ["abilities", parts @ ..] if !parts.is_empty() => {
                // The name has a slash in it: `core/get-site-info`, and `/run` after it.
                let (name_parts, run) = match parts.split_last() {
                    Some((&"run", before)) if !before.is_empty() => (before, true),
                    _ => (parts, false),
                };
                let name = name_parts.join("/");
                if run {
                    return self.ability_run(&name, method, args, params).await;
                }
                if method != "GET" {
                    return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
                }
                if !self.can("read") {
                    return Ok(Err(denied()));
                }
                match items("abilities").iter().find(|a| a["name"] == name.as_str()) {
                    Some(ability) => {
                        let context = params.get("context").unwrap_or("view");
                        let mut resp = Resp::item(self.ability_json(ability, context));
                        resp.allow = Some("GET".to_owned());
                        Ok(Ok(resp))
                    }
                    None => Ok(Err(RestError::new("rest_ability_not_found", "Ability not found.", 404))),
                }
            }
            ["categories"] if method == "GET" => {
                if !self.can("read") {
                    return Ok(Err(denied()));
                }
                let list: Vec<Value> = items("categories").iter().map(|c| self.category_json(c, params.get("context").unwrap_or("view"))).collect();
                let mut resp = Resp::item(Value::Array(list.clone()));
                resp.total = Some(list.len() as u64);
                resp.total_pages = Some(1);
                resp.allow = Some("GET".to_owned());
                Ok(Ok(resp))
            }
            ["categories", slug] if method == "GET" => {
                if !self.can("read") {
                    return Ok(Err(denied()));
                }
                match items("categories").iter().find(|c| c["slug"] == *slug) {
                    Some(category) => {
                        let mut resp = Resp::item(self.category_json(category, params.get("context").unwrap_or("view")));
                        resp.allow = Some("GET".to_owned());
                        Ok(Ok(resp))
                    }
                    None => Ok(Err(RestError::new("rest_ability_category_not_found", "Ability category not found.", 404))),
                }
            }
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    fn abilities_list(&self, params: &Params) -> Result<Outcome> {
        let page = match params.integer("page", 1, 1, None) {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let per_page = match params.integer("per_page", 50, 1, Some(100)) {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let category = params.get("category").unwrap_or_default();
        let namespace = params.get("namespace").unwrap_or_default();
        let selected: Vec<&Value> = items("abilities")
            .iter()
            .filter(|a| (category.is_empty() || a["category"] == category) && (namespace.is_empty() || a["name"].as_str().is_some_and(|n| n.split('/').next() == Some(namespace))))
            .collect();
        let total = selected.len() as u64;
        let max_pages = total.div_ceil(per_page);
        let context = params.get("context").unwrap_or("view");
        let list: Vec<Value> = selected.into_iter().skip(((page - 1).saturating_mul(per_page)) as usize).take(per_page as usize).map(|a| self.ability_json(a, context)).collect();
        let mut resp = Resp::item(Value::Array(list));
        resp.total = Some(total);
        resp.total_pages = Some(max_pages);
        resp.allow = Some("GET".to_owned());
        resp.link = link_header_with(self, "/wp-abilities/v1/abilities", params, &LinkParams { arrays: &[], booleans: &[] }, page, max_pages, false);
        Ok(Ok(resp))
    }

    /// `prepare_item_for_response()` with the fields of the context and the links.
    fn ability_json(&self, ability: &Value, context: &str) -> Value {
        let fields: &[&str] = match context {
            "view" | "edit" => &["name", "label", "description", "category", "input_schema", "output_schema", "meta"],
            "embed" => &["name", "label", "category"],
            _ => &[],
        };
        let mut data = Map::new();
        for field in fields {
            if let Some(value) = ability.get(*field) {
                data.insert((*field).to_owned(), value.clone());
            }
        }
        let name = ability["name"].as_str().unwrap_or_default();
        let mut links = Map::new();
        links.insert("self".into(), json!([{"href": self.rest_url(&format!("wp-abilities/v1/abilities/{name}")), "targetHints": {"allow": ["GET"]}}]));
        links.insert("collection".into(), json!([{"href": self.rest_url("wp-abilities/v1/abilities")}]));
        links.insert("wp:action-run".into(), json!([{"href": self.rest_url(&format!("wp-abilities/v1/abilities/{name}/run"))}]));
        data.insert("_links".into(), Value::Object(links));
        Value::Object(data)
    }

    fn category_json(&self, category: &Value, context: &str) -> Value {
        let fields: &[&str] = match context {
            "view" | "edit" => &["slug", "label", "description", "meta"],
            "embed" => &["slug", "label"],
            _ => &[],
        };
        let mut data = Map::new();
        for field in fields {
            if let Some(value) = category.get(*field) {
                data.insert((*field).to_owned(), value.clone());
            }
        }
        let slug = category["slug"].as_str().unwrap_or_default();
        data.insert(
            "_links".into(),
            json!({
                "self": [{"href": self.rest_url(&format!("wp-abilities/v1/categories/{slug}")), "targetHints": {"allow": ["GET"]}}],
                "collection": [{"href": self.rest_url("wp-abilities/v1/categories")}],
                "abilities": [{"href": self.rest_url(&format!("wp-abilities/v1/abilities?category={slug}"))}],
            }),
        );
        Value::Object(data)
    }

    /// `execute_ability()` with the checks of `check_ability_permissions()` before it.
    async fn ability_run(&self, name: &str, method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let Some(ability) = items("abilities").iter().find(|a| a["name"] == name) else {
            return Ok(Err(RestError::new("rest_ability_not_found", "Ability not found.", 404)));
        };
        let annotations = &ability["meta"]["annotations"];
        let flag = |key: &str| annotations.get(key).and_then(Value::as_bool).unwrap_or(false);
        let expected = if flag("readonly") {
            "GET"
        } else if flag("destructive") && flag("idempotent") {
            "DELETE"
        } else {
            "POST"
        };
        if expected != method {
            let message = match expected {
                "GET" => "Read-only abilities require GET method.",
                "DELETE" => "Abilities that perform destructive actions require DELETE method.",
                _ => "Abilities that perform updates require POST method.",
            };
            return Ok(Err(RestError::new("rest_ability_invalid_method", message, 405)));
        }
        let input = if matches!(method, "GET" | "DELETE") { query_input(params.raw()) } else { args.map.get("input").cloned() };
        let fields = match self.validate_ability_input(name, ability, input) {
            Ok(fields) => fields,
            Err(reason) => {
                let message = Msg::new("Ability \"%1$s\" has invalid input. Reason: %2$s").with(&[name, &reason]);
                return Ok(Err(RestError::new("ability_invalid_input", message, 400)));
            }
        };
        let allowed = match name {
            "core/get-user-info" => self.user().is_some(),
            _ => self.can("manage_options"),
        };
        if !allowed {
            return Ok(Err(self.forbidden("rest_ability_cannot_execute", "Sorry, you are not allowed to execute this ability.")));
        }
        let all: Vec<(String, Value)> = match name {
            "core/get-site-info" => self.site_info(),
            "core/get-user-info" => self.user_info().await?,
            "core/get-environment-info" => self.environment_info().await?,
            _ => return Ok(Err(RestError::new("rest_ability_not_found", "Ability not found.", 404))),
        };
        let mut out = Map::new();
        if fields.is_empty() {
            out.extend(all);
        } else {
            for field in &fields {
                if let Some((_, value)) = all.iter().find(|(k, _)| k == field) {
                    out.insert(field.clone(), value.clone());
                }
            }
        }
        let mut resp = Resp::item(Value::Object(out));
        resp.allow = Some("GET, POST, PUT, PATCH, DELETE".to_owned());
        Ok(Ok(resp))
    }

    /// `WP_Ability::validate_input()` for the schemas of the core abilities (`fields`, a list of the names of the
    /// output): the fields asked for, or the reason the input is not valid.
    fn validate_ability_input(&self, _name: &str, ability: &Value, input: Option<Value>) -> std::result::Result<Vec<String>, String> {
        let schema = &ability["input_schema"];
        let Some(input) = input else { return Ok(Vec::new()) };
        let Value::Object(map) = &input else { return Err("input is not of type object.".to_owned()) };
        let properties = schema["properties"].as_object();
        let mut fields = Vec::new();
        for (key, value) in map {
            if !properties.is_some_and(|p| p.contains_key(key)) {
                return Err(format!("{key} is not a valid property of Object."));
            }
            if key == "fields" {
                let list: Vec<String> = match value {
                    Value::Array(items) => items.iter().map(|v| v.as_str().unwrap_or_default().to_owned()).collect(),
                    Value::String(s) => s.split([',', ' ']).filter(|p| !p.is_empty()).map(str::to_owned).collect(),
                    _ => return Err("input[fields] is not of type array.".to_owned()),
                };
                let allowed: Vec<&str> = schema["properties"]["fields"]["items"]["enum"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
                for (i, item) in list.iter().enumerate() {
                    if !allowed.contains(&item.as_str()) {
                        let param = format!("input[fields][{i}]");
                        return Err(Msg::new("%1$s is not one of %2$l.").with(&[&param]).with_list(&allowed).render(self.site));
                    }
                }
                fields = list;
            }
        }
        Ok(fields)
    }

    fn site_info(&self) -> Vec<(String, Value)> {
        let s = &self.site.settings;
        vec![
            ("name".into(), json!(s.blogname)),
            ("description".into(), json!(s.blogdescription)),
            ("url".into(), json!(s.home.trim_end_matches('/'))),
            ("wpurl".into(), json!(s.siteurl.trim_end_matches('/'))),
            ("admin_email".into(), json!(self.site.options.string("admin_email", ""))),
            ("charset".into(), json!("UTF-8")),
            ("language".into(), json!(self.site.locale().replace('_', "-"))),
            ("version".into(), json!(self.wp_version())),
        ]
    }

    async fn user_info(&self) -> Result<Vec<(String, Value)>> {
        let Some(user) = self.user() else { return Ok(Vec::new()) };
        let db = &self.site.db;
        let record = db.user(user.id).await?;
        let meta = |key: &'static str| async move { db.user_meta(user.id, key).await.map(Option::unwrap_or_default) };
        let locale = meta("locale").await?;
        Ok(vec![
            ("id".into(), json!(user.id)),
            ("display_name".into(), json!(user.display_name)),
            ("user_nicename".into(), json!(record.as_ref().map(|r| r.user_nicename.clone()).unwrap_or_default())),
            ("user_login".into(), json!(user.login)),
            ("roles".into(), json!(user.roles)),
            ("locale".into(), json!(if locale.is_empty() { self.site.locale() } else { locale })),
            ("first_name".into(), json!(meta("first_name").await?)),
            ("last_name".into(), json!(meta("last_name").await?)),
            ("nickname".into(), json!(meta("nickname").await?)),
            ("description".into(), json!(meta("description").await?)),
            ("user_url".into(), json!(record.map(|r| r.user_url).unwrap_or_default())),
        ])
    }

    async fn environment_info(&self) -> Result<Vec<(String, Value)>> {
        let server = self.site.db.server_info().await?;
        Ok(vec![
            ("environment".into(), json!(self.site.environment_type())),
            // There is no PHP here.
            ("php_version".into(), json!("N/A")),
            ("db_server_info".into(), json!(server.version)),
            ("wp_version".into(), json!(self.wp_version())),
        ])
    }
}
