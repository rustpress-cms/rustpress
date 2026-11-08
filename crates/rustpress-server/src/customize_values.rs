//! The values a page in the preview of the Customizer shows instead of the stored ones:
//! `WP_Customize_Manager::unsanitized_post_values()` (the changeset, then `$_POST['customized']`) and what
//! `WP_Customize_Setting::preview()` makes of them for the settings of WordPress, of the ported theme, of the widgets
//! and of the menu locations: the options and the theme's `theme_mods_{$stylesheet}` as `get_option()` and
//! `get_theme_mod()` return them in the preview, sanitized with the setting's callback.

use rustpress_phpser::{Key, Value as Php};
use serde_json::Value;

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
}

/// `json_decode( $changeset_post->post_content, true )` of the changeset with this UUID, if there is one.
pub async fn changeset_data(loaded: &Loaded, uuid: &str) -> Vec<(String, Value)> {
    let query = rustpress_db::PostQuery {
        post_types: vec!["customize_changeset".into()],
        statuses: super::customize::STATI.iter().map(|s| (*s).to_owned()).collect(),
        name: Some(uuid.to_owned()),
        limit: Some(1),
        ..Default::default()
    };
    let Some(post) = loaded.site.db.posts(&query).await.ok().and_then(|posts| posts.into_iter().next()) else { return Vec::new() };
    match serde_json::from_str::<Value>(&post.post_content) {
        Ok(Value::Object(map)) => map.into_iter().collect(),
        _ => Vec::new(),
    }
}

/// `unsanitized_post_values()`: the values of the changeset (theme mods only of the active theme), then the posted ones.
pub fn merge(changeset: &[(String, Value)], stylesheet: &str, posted: &[(String, Value)]) -> Vec<(String, Value)> {
    let mut values: Vec<(String, Value)> = Vec::new();
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

/// The options and theme mods of the previewed values.
pub fn apply(loaded: &Loaded, values: &[(String, Value)]) -> Previewed {
    let site = &loaded.site;
    let stylesheet = site.settings.stylesheet.clone();
    let seventeen = loaded.theme.template == "twentyseventeen";
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
        let known = matches!(id.as_str(), "blogname" | "blogdescription" | "show_on_front" | "page_on_front" | "page_for_posts" | "site_icon" | "custom_logo" | "header_image" | "header_image_data" | "header_textcolor" | "header_video" | "external_header_video")
            || (seventeen && matches!(id.as_str(), "colorscheme" | "colorscheme_hue" | "page_layout" | "panel_1" | "panel_2" | "panel_3" | "panel_4"))
            || id.starts_with("nav_menu_locations[")
            || id == &format!("custom_css[{stylesheet}]")
            || id.starts_with("sidebars_widgets[")
            || id.starts_with("widget_");
        if known {
            out.known.push(id.clone());
        }
        match id.as_str() {
            "blogname" | "blogdescription" | "show_on_front" | "page_on_front" | "page_for_posts" | "site_icon" => out.options.push((id.clone(), text(value))),
            "custom_logo" | "header_image" | "header_image_data" => set_mod(&mut mods, &mut mods_changed, id, to_php(value)),
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
        let merged = merge(&changeset, "twentyseventeen", &posted);
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
