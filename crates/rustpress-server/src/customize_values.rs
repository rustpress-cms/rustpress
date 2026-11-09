//! The values a page in the preview of the Customizer shows instead of the stored ones:
//! `WP_Customize_Manager::unsanitized_post_values()` (the changeset, then `$_POST['customized']`) and what
//! `WP_Customize_Setting::preview()` makes of them for the settings of WordPress, of the ported theme, of the widgets
//! and of the menu locations: the options and the theme's `theme_mods_{$stylesheet}` as `get_option()` and
//! `get_theme_mod()` return them in the preview, sanitized with the setting's callback.

use rustpress_phpser::{Key, Value as Php};
use serde_json::{Value, json};

use crate::app::Loaded;

/// What the preview changes.
#[derive(Debug, Default)]
pub struct Previewed {
    /// The options, as their stored text.
    pub options: Vec<(String, String)>,
    /// `wp_get_custom_css()` (`WP_Customize_Custom_CSS_Setting::filter_previewed_wp_get_custom_css()`).
    pub custom_css: Option<String>,
    /// The values that belong to a setting of the Customizer.
    pub known: Vec<String>,
    /// The menus and menu items (`WP_Customize_Nav_Menu_Setting::preview()`, `WP_Customize_Nav_Menu_Item_Setting::preview()`).
    pub nav_menus: Vec<(i64, Option<Value>)>,
    pub nav_menu_items: Vec<(i64, Option<Value>)>,
    /// The values a setting's `sanitize()` refuses, with the error of `validate_setting_values()`.
    pub invalid: Vec<(String, Value)>,
}

/// The error of `validate_setting_values()` for a value that does not sanitize.
fn invalid(site: &rustpress_core::Site, code: &str) -> Value {
    let message = if code == "invalid_url" { site.tr("Invalid URL.") } else { site.tr("Invalid value.") };
    json!({code: {"message": message, "data": null}})
}

/// Whether one of the addresses of a value is one `sanitize_url()` empties.
fn has_invalid_url(value: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| value.get(*key).and_then(Value::as_str).is_some_and(|url| !url.is_empty() && rustpress_core::formatting::esc_url_raw(url).is_empty()))
}

/// `json_decode( $post->post_content, true )` as an array, or nothing (`get_changeset_post_data()`).
fn post_data(content: &str) -> Option<Vec<(String, Value)>> {
    match serde_json::from_str::<Value>(content) {
        Ok(Value::Object(map)) => Some(map.into_iter().collect()),
        Ok(Value::Array(items)) => Some(items.into_iter().enumerate().map(|(i, v)| (i.to_string(), v)).collect()),
        _ => None,
    }
}

/// `changeset_data()` of the changeset with this UUID, if there is one: with `customize_autosaved` the autosave
/// revision of the signed-in user (`wp_get_post_autosave()`) when it has one.
pub async fn changeset_data(loaded: &Loaded, uuid: &str, autosave_of: Option<u64>) -> Vec<(String, Value)> {
    let query = rustpress_db::PostQuery {
        post_types: vec!["customize_changeset".into()],
        statuses: super::customize::STATI.iter().map(|s| (*s).to_owned()).collect(),
        name: Some(uuid.to_owned()),
        limit: Some(1),
        ..Default::default()
    };
    let Some(post) = loaded.site.db.posts(&query).await.ok().and_then(|posts| posts.into_iter().next()) else { return Vec::new() };
    if let Some(user) = autosave_of {
        let query = rustpress_db::PostQuery {
            post_types: vec!["revision".into()],
            statuses: vec!["inherit".into()],
            parent: Some(post.id),
            name: Some(format!("{}-autosave-v1", post.id)),
            author: (user != 0).then_some(user),
            order_by: rustpress_db::OrderBy::Date,
            order: rustpress_db::Order::Desc,
            limit: Some(1),
            ..Default::default()
        };
        if let Some(data) = loaded.site.db.posts(&query).await.ok().and_then(|posts| posts.into_iter().next()).and_then(|autosave| post_data(&autosave.post_content)) {
            return data;
        }
    }
    post_data(&post.post_content).unwrap_or_default()
}

/// `wp_list_pluck( $stashed_theme_mods[ $stylesheet ], 'value' )` of the option `customize_stashed_theme_mods`.
pub fn stashed_theme_mods(site: &rustpress_core::Site, stylesheet: &str) -> Vec<(String, Value)> {
    let Some(Php::Array(themes)) = site.options.get("customize_stashed_theme_mods").map(rustpress_core::options::maybe_unserialize) else { return Vec::new() };
    let Some((_, Php::Array(mods))) = themes.iter().find(|(key, _)| *key == Key::Str(stylesheet.as_bytes().to_vec())) else { return Vec::new() };
    mods.iter()
        .filter_map(|(key, params)| {
            let id = match key {
                Key::Int(n) => n.to_string(),
                Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
            };
            let value = params.get("value")?;
            Some((id, rustpress_core::options::php_to_json(value)))
        })
        .collect()
}

/// `unsanitized_post_values()`: the stashed theme mods of a previewed theme, the values of the changeset (theme mods
/// only of the previewed theme), then the posted ones.
pub fn merge(stashed: &[(String, Value)], changeset: &[(String, Value)], stylesheet: &str, posted: &[(String, Value)]) -> Vec<(String, Value)> {
    let mut values: Vec<(String, Value)> = stashed.to_vec();
    let mut set = |id: String, value: Value| match values.iter_mut().find(|(k, _)| *k == id) {
        Some(slot) => slot.1 = value,
        None => values.push((id, value)),
    };
    for (id, params) in changeset {
        let Some(value) = params.get("value") else { continue };
        if params.get("type").and_then(Value::as_str) == Some("theme_mod") {
            if let Some((theme, setting)) = id.split_once("::")
                && theme == stylesheet
            {
                set(setting.to_owned(), value.clone());
            }
        } else {
            set(id.clone(), value.clone());
        }
    }
    for (id, value) in posted {
        set(id.clone(), value.clone());
    }
    values
}

/// `intval()` of a value from JSON.
fn intval(value: &Value) -> i64 {
    match value {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)).unwrap_or(0),
        Value::Bool(b) => i64::from(*b),
        Value::String(s) => {
            let text = s.trim_start_matches([' ', '\t', '\n', '\r', '\x0B', '\x0C']);
            let (negative, digits) = match text.strip_prefix('-') {
                Some(rest) => (true, rest),
                None => (false, text.strip_prefix('+').unwrap_or(text)),
            };
            let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
            let number = digits[..end].parse::<i64>().unwrap_or(0);
            if negative { -number } else { number }
        }
        Value::Array(items) => i64::from(!items.is_empty()),
        Value::Object(items) => i64::from(!items.is_empty()),
        Value::Null => 0,
    }
}

/// `(string)` of a scalar value.
fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(true) => "1".into(),
        _ => String::new(),
    }
}

/// A value from JSON as PHP keeps it after `json_decode( $json, true )`.
pub fn to_php(value: &Value) -> Php {
    match value {
        Value::Null => Php::Null,
        Value::Bool(b) => Php::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => Php::Int(i),
            None => Php::Float(n.as_f64().unwrap_or(0.0)),
        },
        Value::String(s) => Php::Str(s.as_bytes().to_vec()),
        Value::Array(items) => Php::Array(items.iter().enumerate().map(|(i, v)| (Key::Int(i as i64), to_php(v))).collect()),
        Value::Object(map) => Php::Array(
            map.iter()
                .map(|(k, v)| {
                    // PHP turns keys that are decimal integers into integer keys.
                    let key = match k.parse::<i64>() {
                        Ok(n) if n.to_string() == *k => Key::Int(n),
                        _ => Key::Str(k.as_bytes().to_vec()),
                    };
                    (key, to_php(v))
                })
                .collect(),
        ),
    }
}

/// Sets `$array[ $key ]`, keeping the place of a key that exists.
fn set_key(array: &mut Php, key: Key, value: Php) {
    if !matches!(array, Php::Array(_)) {
        *array = Php::Array(Vec::new());
    }
    if let Php::Array(items) = array {
        match items.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => items.push((key, value)),
        }
    }
}

fn str_key(key: &str) -> Key {
    match key.parse::<i64>() {
        Ok(n) if n.to_string() == key => Key::Int(n),
        _ => Key::Str(key.as_bytes().to_vec()),
    }
}

/// `set_theme_mod()` in the previewed copy of the theme mods.
fn set_mod(mods: &mut Php, changed: &mut bool, key: &str, value: Php) {
    set_key(mods, str_key(key), value);
    *changed = true;
}

/// `sanitize_hex_color_no_hash()`.
fn hex_color_no_hash(color: &str) -> Option<String> {
    let color = color.trim_start_matches('#');
    let valid = (color.len() == 3 || color.len() == 6) && color.bytes().all(|b| b.is_ascii_hexdigit());
    valid.then(|| color.to_owned())
}

/// `ID_PATTERN` of the menu and menu item settings: `nav_menu[-12]` gives -12.
fn setting_number(id: &str, base: &str) -> Option<i64> {
    let number = id.strip_prefix(base)?.strip_prefix('[')?.strip_suffix(']')?;
    let digits = number.strip_prefix('-').unwrap_or(number);
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then(|| number.parse().ok()).flatten()
}

/// `(bool)` of a value from JSON.
fn boolval(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty() && s != "0",
        Value::Array(items) => !items.is_empty(),
        Value::Object(items) => !items.is_empty(),
    }
}

/// `array_merge( $default, $value )` and `wp_array_slice_assoc( $value, array_keys( $default ) )`: the keys of the
/// defaults in their order, with the values that were sent (a `null` one is not `isset()`).
fn merged(value: &Value, defaults: &[(&str, Value)]) -> serde_json::Map<String, Value> {
    defaults.iter().map(|(key, default)| ((*key).to_owned(), value.get(*key).filter(|v| !v.is_null()).cloned().unwrap_or_else(|| default.clone()))).collect()
}

/// `WP_Customize_Nav_Menu_Setting::sanitize()`: `Some(None)` deletes the menu, `None` is invalid.
fn sanitize_nav_menu(site: &rustpress_core::Site, value: &Value) -> Option<Option<Value>> {
    if *value == Value::Bool(false) {
        return Some(None);
    }
    if !value.is_object() && !value.is_array() {
        return None;
    }
    let mut menu = merged(value, &[("name", json!("")), ("description", json!("")), ("parent", json!(0)), ("auto_add", json!(false))]);
    let name = rustpress_core::formatting::php_trim(&rustpress_core::formatting::esc_html(&text(&menu["name"]))).to_owned();
    let name = if name.is_empty() { site.trx("(unnamed)", "Missing menu name.") } else { name };
    menu.insert("name".into(), json!(name));
    menu.insert("description".into(), json!(rustpress_content::sanitize::text_field(&text(&menu["description"]))));
    menu.insert("parent".into(), json!(intval(&menu["parent"]).max(0)));
    menu.insert("auto_add".into(), json!(boolval(&menu["auto_add"])));
    Some(Some(Value::Object(menu)))
}

/// `WP_Customize_Nav_Menu_Item_Setting::sanitize()`: `Some(None)` deletes the item, `None` is invalid (also an
/// address that `sanitize_url()` empties).
fn sanitize_nav_menu_item(value: &Value) -> Option<Option<Value>> {
    if *value == Value::Bool(false) {
        return Some(None);
    }
    if !value.is_object() && !value.is_array() {
        return None;
    }
    let defaults = [
        ("object_id", json!(0)),
        ("object", json!("")),
        ("menu_item_parent", json!(0)),
        ("position", json!(0)),
        ("type", json!("custom")),
        ("title", json!("")),
        ("url", json!("")),
        ("target", json!("")),
        ("attr_title", json!("")),
        ("description", json!("")),
        ("classes", json!("")),
        ("xfn", json!("")),
        ("status", json!("publish")),
        ("original_title", json!("")),
        ("nav_menu_term_id", json!(0)),
        ("_invalid", json!(false)),
    ];
    let mut item = merged(value, &defaults);
    for key in ["position", "object_id", "menu_item_parent", "nav_menu_term_id"] {
        let number = intval(&item[key]);
        item.insert(key.into(), json!(number));
    }
    for key in ["type", "object", "target"] {
        let key_value = super::customize::sanitize_key(&text(&item[key]));
        item.insert(key.into(), json!(key_value));
    }
    for key in ["xfn", "classes"] {
        let parts: Vec<String> = match &item[key] {
            Value::Array(list) => list.iter().map(text).collect(),
            Value::Object(map) => map.values().map(text).collect(),
            other => text(other).split(' ').map(str::to_owned).collect(),
        };
        let joined = parts.iter().map(|part| rustpress_core::formatting::sanitize_html_class(part)).collect::<Vec<_>>().join(" ");
        item.insert(key.into(), json!(joined));
    }
    // `title_save_pre` (`trim()`), `excerpt_save_pre` and `content_save_pre` (`convert_invalid_entities()`).
    let title = rustpress_core::formatting::php_trim(&text(&item["title"])).to_owned();
    item.insert("title".into(), json!(title));
    for key in ["attr_title", "description"] {
        let converted = rustpress_core::kses::convert_invalid_entities(&text(&item[key]));
        item.insert(key.into(), json!(converted));
    }
    let url = text(&item["url"]);
    if !url.is_empty() {
        let url = rustpress_core::formatting::esc_url_raw(&url);
        if url.is_empty() {
            return None;
        }
        item.insert("url".into(), json!(url));
    }
    if text(&item["status"]) != "publish" {
        item.insert("status".into(), json!("draft"));
    }
    let invalid = boolval(&item["_invalid"]);
    item.insert("_invalid".into(), json!(invalid));
    Some(Some(Value::Object(item)))
}

/// `absint()` of a value from JSON.
fn absint(value: &Value) -> i64 {
    intval(value).abs()
}

/// `WP_Customize_Header_Image_Setting::sanitize()` with `sanitize_choice()`: a text, or the parts of an image the
/// header control sends, in their order. `None` is invalid (also an address that `sanitize_url()` empties).
async fn sanitize_header_image(site: &rustpress_core::Site, value: &Value) -> Option<Value> {
    async fn choice(site: &rustpress_core::Site, value: &Value) -> Option<Value> {
        let map = match value {
            Value::String(text) => return Some(json!(rustpress_content::sanitize::text_field(text))),
            Value::Object(map) => map.clone(),
            Value::Array(items) => items.iter().enumerate().map(|(i, v)| (i.to_string(), v.clone())).collect(),
            _ => return None,
        };
        let scalar = |v: &Value| matches!(v, Value::String(_) | Value::Number(_) | Value::Bool(_));
        let set = |key: &str| map.get(key).filter(|v| !v.is_null());
        let mut out = serde_json::Map::new();
        if let Some(id) = set("attachment_id") {
            if !scalar(id) {
                return None;
            }
            let id = absint(id);
            if id > 0 {
                let mime = site.db.post(id as u64).await.ok().flatten().map(|post| post.post_mime_type);
                if !mime.is_some_and(|mime| mime.starts_with("image/")) {
                    return None;
                }
            }
            out.insert("attachment_id".into(), json!(id));
        }
        for key in ["url", "thumbnail_url"] {
            if let Some(url) = set(key) {
                let Value::String(url) = url else { return None };
                let url = rustpress_core::formatting::esc_url_raw(url);
                if url.is_empty() {
                    return None;
                }
                out.insert(key.into(), json!(url));
            }
        }
        for key in ["timestamp", "width", "height"] {
            if let Some(number) = set(key) {
                if !scalar(number) {
                    return None;
                }
                out.insert(key.into(), json!(absint(number)));
            }
        }
        if let Some(alt) = set("alt_text") {
            let Value::String(alt) = alt else { return None };
            out.insert("alt_text".into(), json!(rustpress_content::sanitize::text_field(alt)));
        }
        if let Some(parent) = set("attachment_parent") {
            if !scalar(parent) {
                return None;
            }
            out.insert("attachment_parent".into(), json!(absint(parent)));
        }
        Some(Value::Object(out))
    }
    match value.get("choice").filter(|_| value.is_object()) {
        Some(inner) => Some(json!({"choice": Box::pin(choice(site, inner)).await?})),
        None => Box::pin(choice(site, value)).await,
    }
}

/// The options and theme mods of the previewed values.
pub async fn apply(site: &rustpress_core::Site, template: &str, values: &[(String, Value)]) -> Previewed {
    let stylesheet = site.settings.stylesheet.clone();
    let seventeen = template == "twentyseventeen";
    let mut out = Previewed::default();
    let mods_name = format!("theme_mods_{stylesheet}");
    let mut mods = site.options.get(&mods_name).map(rustpress_core::options::maybe_unserialize).unwrap_or(Php::Bool(false));
    let mut mods_changed = false;
    let mut options: Vec<(String, Php)> = Vec::new();
    let option_of = |name: &str, options: &mut Vec<(String, Php)>| -> usize {
        if let Some(i) = options.iter().position(|(n, _)| n == name) {
            return i;
        }
        options.push((name.to_owned(), site.options.get(name).map(rustpress_core::options::maybe_unserialize).unwrap_or(Php::Bool(false))));
        options.len() - 1
    };
    for (id, value) in values {
        if id == "header_image" || id == "header_image_data" {
            match sanitize_header_image(site, value).await {
                Some(image) => {
                    out.known.push(id.clone());
                    set_mod(&mut mods, &mut mods_changed, id, to_php(&image));
                }
                None => out.invalid.push((id.clone(), invalid(site, if has_invalid_url(value.get("choice").unwrap_or(value), &["url", "thumbnail_url"]) { "invalid_url" } else { "invalid_value" }))),
            }
            continue;
        }
        let known = matches!(id.as_str(), "blogname" | "blogdescription" | "show_on_front" | "page_on_front" | "page_for_posts" | "site_icon" | "custom_logo" | "header_textcolor" | "header_video" | "external_header_video")
            || (seventeen && matches!(id.as_str(), "colorscheme" | "colorscheme_hue" | "page_layout" | "panel_1" | "panel_2" | "panel_3" | "panel_4"))
            || id.starts_with("nav_menu_locations[")
            || id == &format!("custom_css[{stylesheet}]")
            || id.starts_with("sidebars_widgets[")
            || id.starts_with("widget_");
        if known {
            out.known.push(id.clone());
        }
        // The dynamic settings of the menus; a value that does not sanitize is not previewed.
        if let Some(term_id) = setting_number(id, "nav_menu") {
            match sanitize_nav_menu(site, value) {
                Some(menu) => {
                    out.known.push(id.clone());
                    out.nav_menus.push((term_id, menu));
                }
                None => out.invalid.push((id.clone(), invalid(site, "invalid_value"))),
            }
            continue;
        }
        if let Some(post_id) = setting_number(id, "nav_menu_item") {
            match sanitize_nav_menu_item(value) {
                Some(item) => {
                    out.known.push(id.clone());
                    out.nav_menu_items.push((post_id, item));
                }
                None => out.invalid.push((id.clone(), invalid(site, if has_invalid_url(value, &["url"]) { "invalid_url" } else { "invalid_value" }))),
            }
            continue;
        }
        match id.as_str() {
            "blogname" | "blogdescription" | "show_on_front" | "page_on_front" | "page_for_posts" | "site_icon" => out.options.push((id.clone(), text(value))),
            "custom_logo" => set_mod(&mut mods, &mut mods_changed, id, to_php(value)),
            "header_textcolor" => {
                let color = text(value);
                // An invalid color becomes the theme's `default-text-color`, which Twenty Seventeen does not set.
                let color = if color == "blank" { color } else { hex_color_no_hash(&color).unwrap_or_default() };
                set_mod(&mut mods, &mut mods_changed, id, Php::Str(color.into_bytes()));
            }
            "header_video" => set_mod(&mut mods, &mut mods_changed, id, Php::Int(intval(value).abs())),
            "external_header_video" => set_mod(&mut mods, &mut mods_changed, id, Php::Str(rustpress_core::formatting::esc_url_raw(text(value).trim()).into_bytes())),
            "colorscheme" if seventeen => {
                let scheme = text(value);
                let scheme = if ["light", "dark", "custom"].contains(&scheme.as_str()) { scheme } else { "light".to_owned() };
                set_mod(&mut mods, &mut mods_changed, id, Php::Str(scheme.into_bytes()));
            }
            "colorscheme_hue" if seventeen => set_mod(&mut mods, &mut mods_changed, id, Php::Int(intval(value).abs())),
            "page_layout" if seventeen => {
                let layout = text(value);
                let layout = if layout == "one-column" || layout == "two-column" { layout } else { String::new() };
                set_mod(&mut mods, &mut mods_changed, id, Php::Str(layout.into_bytes()));
            }
            "panel_1" | "panel_2" | "panel_3" | "panel_4" if seventeen => set_mod(&mut mods, &mut mods_changed, id, Php::Int(intval(value).abs())),
            _ => {
                if let Some(location) = id.strip_prefix("nav_menu_locations[").and_then(|rest| rest.strip_suffix(']')) {
                    // `intval_base10()` into the array of the locations.
                    let mut locations = match &mods {
                        Php::Array(items) => items.iter().find(|(k, _)| *k == Key::Str(b"nav_menu_locations".to_vec())).map(|(_, v)| v.clone()).unwrap_or(Php::Array(Vec::new())),
                        _ => Php::Array(Vec::new()),
                    };
                    set_key(&mut locations, str_key(location), Php::Int(intval(value)));
                    set_mod(&mut mods, &mut mods_changed, "nav_menu_locations", locations);
                } else if let Some(theme) = id.strip_prefix("custom_css[").and_then(|rest| rest.strip_suffix(']')) {
                    if theme == stylesheet {
                        out.custom_css = Some(text(value));
                    }
                } else if let Some(area) = id.strip_prefix("sidebars_widgets[").and_then(|rest| rest.strip_suffix(']')) {
                    // `sanitize_sidebar_widgets()`.
                    let ids: Vec<Php> = match value {
                        Value::Array(items) => items.iter().map(|v| Php::Str(text(v).into_bytes())).collect(),
                        _ => Vec::new(),
                    };
                    let i = option_of("sidebars_widgets", &mut options);
                    set_key(&mut options[i].1, str_key(area), Php::Array(ids.into_iter().enumerate().map(|(n, v)| (Key::Int(n as i64), v)).collect()));
                } else if let Some((base, number)) = id.strip_prefix("widget_").and_then(|rest| rest.strip_suffix(']')).and_then(|rest| rest.split_once('[')) {
                    // `sanitize_widget_instance()` with the block editor: the raw instance of a widget shown in REST.
                    let Ok(number) = number.parse::<i64>() else { continue };
                    let instance = match value.get("raw_instance") {
                        Some(raw) => to_php(raw),
                        None if value.as_array().is_some_and(Vec::is_empty) => Php::Array(Vec::new()),
                        None => continue,
                    };
                    let i = option_of(&format!("widget_{base}"), &mut options);
                    set_key(&mut options[i].1, Key::Int(number), instance);
                }
            }
        }
    }
    if mods_changed {
        out.options.push((mods_name, String::from_utf8_lossy(&rustpress_phpser::serialize(&mods)).into_owned()));
    }
    for (name, value) in options {
        out.options.push((name, String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posted_values_win_and_theme_mods_need_the_active_theme() {
        let changeset = vec![
            ("blogname".to_owned(), serde_json::json!({"value": "Stored", "type": "option"})),
            ("twentyseventeen::colorscheme".to_owned(), serde_json::json!({"value": "dark", "type": "theme_mod"})),
            ("other::colorscheme".to_owned(), serde_json::json!({"value": "custom", "type": "theme_mod"})),
            ("no_value".to_owned(), serde_json::json!({"type": "option"})),
        ];
        let posted = vec![("blogname".to_owned(), serde_json::json!("Posted")), ("blogdescription".to_owned(), serde_json::json!("New"))];
        let merged = merge(&[], &changeset, "twentyseventeen", &posted);
        assert_eq!(merged, vec![("blogname".to_owned(), serde_json::json!("Posted")), ("colorscheme".to_owned(), serde_json::json!("dark")), ("blogdescription".to_owned(), serde_json::json!("New"))]);
    }

    #[test]
    fn intval_reads_like_php() {
        assert_eq!(intval(&serde_json::json!(" 42abc")), 42);
        assert_eq!(intval(&serde_json::json!("-7")), -7);
        assert_eq!(intval(&serde_json::json!("abc")), 0);
        assert_eq!(intval(&serde_json::json!(3.9)), 3);
        assert_eq!(intval(&serde_json::json!(true)), 1);
        assert_eq!(intval(&serde_json::json!([])), 0);
    }

    #[test]
    fn decimal_keys_become_integers() {
        let php = to_php(&serde_json::json!({"2": "a", "02": "b", "x": [1]}));
        assert_eq!(php, Php::Array(vec![(Key::Int(2), Php::Str(b"a".to_vec())), (Key::Str(b"02".to_vec()), Php::Str(b"b".to_vec())), (Key::Str(b"x".to_vec()), Php::Array(vec![(Key::Int(0), Php::Int(1))]))]));
        assert_eq!(hex_color_no_hash("#ABCDEF"), Some("ABCDEF".to_owned()));
        assert_eq!(hex_color_no_hash("zz"), None);
    }
}
