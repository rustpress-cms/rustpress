//! `wp/v2/icons`: the icons of the core collection (`wp-includes/assets/icon-library-manifest.php`).

use serde_json::{Value, json};

use super::Args;
use crate::error::RestError;
use crate::{Outcome, Resp, Rest};

/// `WP_Icons_Registry::sanitize_icon_content()`: `wp_kses()` with `svg`, `path` and `polygon`.
fn sanitize(svg: &str) -> String {
    fn allowed(tag: &str) -> Option<&'static [&'static str]> {
        match tag {
            "svg" => Some(&["class", "xmlns", "width", "height", "viewbox", "aria-hidden", "role", "focusable"]),
            "path" => Some(&["fill", "fill-rule", "d", "transform"]),
            "polygon" => Some(&["fill", "fill-rule", "points", "transform", "focusable"]),
            _ => None,
        }
    }
    let mut out = String::new();
    let mut rest = svg;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else { break };
        let inner = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        if let Some(name) = inner.strip_prefix('/') {
            if allowed(&name.trim().to_ascii_lowercase()).is_some() {
                out.push_str(&format!("</{}>", name.trim().to_ascii_lowercase()));
            }
            continue;
        }
        let self_closing = inner.trim_end().ends_with('/');
        let inner = inner.trim_end().trim_end_matches('/');
        let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
        let name = inner[..name_end].to_ascii_lowercase();
        let Some(attrs) = allowed(&name) else { continue };
        out.push('<');
        out.push_str(&name);
        let mut tail = inner[name_end..].trim_start();
        while !tail.is_empty() {
            let key_end = tail.find(|c: char| c == '=' || c.is_whitespace()).unwrap_or(tail.len());
            let key = tail[..key_end].to_ascii_lowercase();
            tail = tail[key_end..].trim_start();
            let mut value = String::new();
            if let Some(after) = tail.strip_prefix('=') {
                let after = after.trim_start();
                if let Some(quoted) = after.strip_prefix('"') {
                    let end = quoted.find('"').unwrap_or(quoted.len());
                    value = quoted[..end].to_owned();
                    tail = quoted.get(end + 1..).unwrap_or("").trim_start();
                } else {
                    let end = after.find(char::is_whitespace).unwrap_or(after.len());
                    value = after[..end].to_owned();
                    tail = after[end..].trim_start();
                }
            }
            if attrs.contains(&key.as_str()) {
                out.push_str(&format!(" {key}=\"{}\"", value.replace('"', "&quot;")));
            }
        }
        out.push_str(if self_closing { " />" } else { ">" });
    }
    out.push_str(rest);
    out
}

impl Rest<'_> {
    /// The icons in manifest order, as `WP_REST_Icons_Controller` returns them.
    fn icons(&self) -> Vec<Value> {
        let manifest = std::fs::read_to_string(self.theme.wp_root.join("wp-includes/assets/icon-library-manifest.php")).unwrap_or_default();
        let dir = self.theme.wp_root.join("wp-includes/images/icon-library");
        let mut out = Vec::new();
        let (mut name, mut label) = (String::new(), String::new());
        for line in manifest.lines() {
            let line = line.trim();
            if let Some((key, rest)) = line.split_once("=> array(") {
                if rest.trim().is_empty() {
                    name = key.trim().trim_matches('\'').to_owned();
                }
            } else if let Some(rest) = line.strip_prefix("'label'") {
                label = rest.split('\'').nth(1).unwrap_or_default().to_owned();
            } else if let Some(rest) = line.strip_prefix("'filePath'") {
                let file = rest.split('\'').nth(1).unwrap_or_default();
                let Ok(svg) = std::fs::read_to_string(dir.join(file)) else { continue };
                out.push(json!({
                    "name": format!("core/{name}"),
                    "label": self.site.trx(&label, "icon label"),
                    "content": sanitize(&svg),
                    "collection": "core",
                }));
            }
        }
        out
    }

    pub(super) fn icons_route(&self, segments: &[&str], args: &Args) -> Outcome {
        if !self.can("edit_posts") {
            return Err(self.forbidden("rest_cannot_view", &self.site.tr("Sorry, you are not allowed to view the registered icons.")));
        }
        let icons = self.icons();
        match segments {
            ["wp", "v2", "icon-collections"] => Ok(Resp::item(json!([self.icon_collection()]))),
            ["wp", "v2", "icon-collections", slug] => {
                if *slug == "core" {
                    Ok(Resp::item(self.icon_collection()))
                } else {
                    Err(RestError::new("rest_icon_collection_not_found", &self.site.tr("Icon collection not found: \"%s\".").replace("%s", slug), 404))
                }
            }
            ["wp", "v2", "icons"] => Ok(Resp::item(Value::Array(self.filter_icons(icons, None, args.str("search").as_deref())))),
            ["wp", "v2", "icons", collection] => {
                if *collection != "core" {
                    return Err(RestError::new("rest_icon_collection_not_found", &self.site.tr("Icon collection not found: \"%s\".").replace("%s", collection), 404));
                }
                Ok(Resp::item(Value::Array(self.filter_icons(icons, Some(collection), args.str("search").as_deref()))))
            }
            ["wp", "v2", "icons", collection, name] => {
                let full = format!("{collection}/{name}");
                match icons.into_iter().find(|i| i["name"] == full) {
                    Some(icon) => Ok(Resp::item(icon)),
                    None => Err(RestError::new("rest_icon_not_found", &self.site.tr("Icon not found: \"%s\".").replace("%s", &full), 404)),
                }
            }
            _ => Err(RestError::new("rest_no_route", &self.site.tr("No route was found matching the URL and request method."), 404)),
        }
    }

    fn icon_collection(&self) -> Value {
        json!({"slug": "core", "label": self.site.tr("WordPress"), "description": self.site.tr("Default icon collection.")})
    }

    fn filter_icons(&self, icons: Vec<Value>, collection: Option<&str>, search: Option<&str>) -> Vec<Value> {
        let search = search.map(str::to_lowercase).filter(|s| !s.is_empty());
        icons
            .into_iter()
            .filter(|i| collection.is_none_or(|c| i["collection"] == c))
            .filter(|i| search.as_ref().is_none_or(|s| i["name"].as_str().unwrap_or_default().to_lowercase().contains(s) || i["label"].as_str().unwrap_or_default().to_lowercase().contains(s)))
            .collect()
    }
}
