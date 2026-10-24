//! `core/search`.

use rustpress_core::formatting::{esc_attr, esc_url};
use serde_json::{Map, Value};

use crate::Result;
use crate::html::TagEditor;
use crate::render::BlockCtx;
use crate::supports::safecss_filter_attr;

fn non_empty<'a>(attrs: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    attrs.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn style_path<'a>(attrs: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let mut current = attrs.get("style")?;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current).filter(|v| !crate::style_engine::is_empty(v))
}

fn style_attr(styles: &[String]) -> String {
    if styles.is_empty() {
        return String::new();
    }
    format!(" style=\"{}\"", esc_attr(&safecss_filter_attr(&styles.join(" "))))
}

struct Styles {
    input: String,
    button: String,
    wrapper: String,
    label: String,
}

/// `styles_for_block_core_search` (borders, colors, typography, width).
fn styles(attrs: &Map<String, Value>) -> Styles {
    let (mut wrapper, mut button, mut input, mut label) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let button_inside = non_empty(attrs, "buttonPosition") == Some("button-inside");
    let show_label = attrs.get("showLabel").is_some_and(|v| v != &Value::Bool(false));

    if let (Some(width), Some(unit)) = (attrs.get("width").filter(|v| !crate::style_engine::is_empty(v)), non_empty(attrs, "widthUnit")) {
        let n = width.as_f64().unwrap_or(0.0) as i64;
        wrapper.push(format!("width: {}{};", esc_attr(&n.to_string()), esc_attr(unit)));
    }
    // Borders: `style.border.{width,color,style}` plus per-side values.
    for property in ["width", "color", "style"] {
        for side in [None, Some("top"), Some("right"), Some("bottom"), Some("left")] {
            let mut path = vec!["border"];
            if let Some(s) = side {
                path.push(s);
            }
            path.push(property);
            let Some(value) = style_path(attrs, &path).and_then(Value::as_str) else { continue };
            let mut value = value.to_owned();
            if property == "color" && side.is_some() && value.contains("var:preset|color|") {
                value = format!("var(--wp--preset--color--{})", value.rsplit('|').next().unwrap_or(""));
            }
            let name = match side {
                Some(s) => format!("{s}-{property}"),
                None => property.to_owned(),
            };
            let declaration = format!("border-{name}: {};", esc_attr(&value));
            if button_inside {
                wrapper.push(declaration);
            } else {
                button.push(declaration.clone());
                input.push(declaration);
            }
        }
    }
    if let Some(radius) = style_path(attrs, &["border", "radius"]) {
        let default_padding = "4px";
        let radius = match radius {
            Value::Number(n) => format!("{n}px"),
            Value::String(s) => s.clone(),
            _ => String::new(),
        };
        let radius = if radius.contains("var:preset|border-radius|") {
            format!("var(--wp--preset--border-radius--{})", radius.rsplit('|').next().unwrap_or(""))
        } else {
            radius
        };
        if !radius.is_empty() {
            let declaration = format!("border-radius: {};", esc_attr(&radius));
            input.push(declaration.clone());
            button.push(declaration);
            if button_inside && (radius.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%').parse::<f64>() != Ok(0.0)) {
                wrapper.push(format!("border-radius: calc({} + {default_padding});", esc_attr(&radius)));
            }
        }
    }
    let use_input = non_empty(attrs, "buttonPosition") == Some("no-button");
    let mut color_target = |declaration: String| if use_input { input.push(declaration) } else { button.push(declaration) };
    if let Some(c) = style_path(attrs, &["color", "text"]).and_then(Value::as_str) {
        color_target(format!("color: {c};"));
    }
    if let Some(c) = style_path(attrs, &["color", "background"]).and_then(Value::as_str) {
        color_target(format!("background-color: {c};"));
    }
    if let Some(c) = style_path(attrs, &["color", "gradient"]).and_then(Value::as_str) {
        color_target(format!("background: {c};"));
    }
    let typography = typography_styles(attrs);
    if !typography.is_empty() {
        let escaped = esc_attr(&typography);
        label.push(escaped.clone());
        button.push(escaped.clone());
        input.push(escaped);
    }
    if let Some(d) = style_path(attrs, &["typography", "textDecoration"]).and_then(Value::as_str) {
        let declaration = format!("text-decoration: {};", esc_attr(d));
        button.push(declaration.clone());
        if show_label {
            label.push(declaration);
        }
    }
    Styles { input: style_attr(&input), button: style_attr(&button), wrapper: style_attr(&wrapper), label: style_attr(&label) }
}

fn typography_styles(attrs: &Map<String, Value>) -> String {
    let mut out = String::new();
    for (key, property) in [
        ("fontSize", "font-size"),
        ("fontFamily", "font-family"),
        ("letterSpacing", "letter-spacing"),
        ("fontWeight", "font-weight"),
        ("fontStyle", "font-style"),
        ("lineHeight", "line-height"),
        ("textTransform", "text-transform"),
    ] {
        if let Some(value) = style_path(attrs, &["typography", key]).and_then(Value::as_str) {
            out.push_str(&format!("{property}: {value};"));
        }
    }
    out
}

fn typography_classes(attrs: &Map<String, Value>) -> String {
    let mut classes = Vec::new();
    if let Some(size) = non_empty(attrs, "fontSize") {
        classes.push(format!("has-{}-font-size", esc_attr(size)));
    }
    if let Some(family) = non_empty(attrs, "fontFamily") {
        classes.push(format!("has-{}-font-family", esc_attr(family)));
    }
    classes.join(" ")
}

fn border_color_classes(attrs: &Map<String, Value>) -> String {
    let custom = style_path(attrs, &["border", "color"]).is_some();
    let named = non_empty(attrs, "borderColor");
    let mut classes = Vec::new();
    if custom || named.is_some() {
        classes.push("has-border-color".to_owned());
    }
    if let Some(n) = named {
        classes.push(format!("has-{}-border-color", esc_attr(n)));
    }
    classes.join(" ")
}

fn color_classes(attrs: &Map<String, Value>) -> String {
    let mut classes = Vec::new();
    if let Some(c) = non_empty(attrs, "textColor") {
        classes.push(format!("has-text-color has-{c}-color"));
    } else if style_path(attrs, &["color", "text"]).is_some() {
        classes.push("has-text-color".to_owned());
    }
    let named_bg = non_empty(attrs, "backgroundColor");
    let named_gradient = non_empty(attrs, "gradient");
    if named_bg.is_some() || style_path(attrs, &["color", "background"]).is_some() || named_gradient.is_some() || style_path(attrs, &["color", "gradient"]).is_some() {
        classes.push("has-background".to_owned());
    }
    if let Some(c) = named_bg {
        classes.push(format!("has-{c}-background-color"));
    }
    if let Some(g) = named_gradient {
        classes.push(format!("has-{g}-gradient-background"));
    }
    classes.join(" ")
}

pub(super) fn search(bc: &BlockCtx<'_>) -> Result<String> {
    let mut attrs = bc.attrs.clone();
    attrs.entry("label".to_owned()).or_insert_with(|| Value::from("Search"));
    attrs.entry("buttonText".to_owned()).or_insert_with(|| Value::from("Search"));
    let attrs = &attrs;

    let input_id = bc.env.with_state(|s| s.unique_id("wp-block-search__input-"));
    let position = non_empty(attrs, "buttonPosition");
    let mut classnames: Vec<&str> = Vec::new();
    match position {
        Some("button-inside") => classnames.push("wp-block-search__button-inside"),
        Some("button-outside") => classnames.push("wp-block-search__button-outside"),
        Some("no-button") => classnames.push("wp-block-search__no-button"),
        Some("button-only") => classnames.push("wp-block-search__button-only wp-block-search__searchfield-hidden"),
        _ => {}
    }
    if let (Some(use_icon), Some(p)) = (attrs.get("buttonUseIcon"), position)
        && p != "no-button" {
            classnames.push(if use_icon.as_bool() == Some(true) { "wp-block-search__icon-button" } else { "wp-block-search__text-button" });
        }
    let classnames = classnames.join(" ");

    let show_label = attrs.get("showLabel").is_some_and(|v| !crate::style_engine::is_empty(v));
    let use_icon = attrs.get("buttonUseIcon").is_some_and(|v| !crate::style_engine::is_empty(v));
    let show_button = position != Some("no-button");
    let button_position = if show_button { position } else { None };
    let inline = styles(attrs);
    let color_classes = color_classes(attrs);
    let typography = typography_classes(attrs);
    let button_inside = position == Some("button-inside");
    let border_classes = border_color_classes(attrs);

    // <label>
    let label_html = non_empty(attrs, "label").unwrap_or("Search");
    let mut label = TagEditor::first(&format!("<label {}>{}</label>", inline.label, label_html)).expect("label tag");
    label.set_attribute("for", &input_id);
    label.add_class("wp-block-search__label");
    if show_label && non_empty(attrs, "label").is_some() {
        if !typography.is_empty() {
            label.add_class(&typography);
        }
    } else {
        label.add_class("screen-reader-text");
    }
    let label = label.finish();

    // <input>
    let mut input = TagEditor::first(&format!("<input type=\"search\" name=\"s\" required {}/>", inline.input)).expect("input tag");
    let mut input_classes = vec!["wp-block-search__input".to_owned()];
    if !button_inside && !border_classes.is_empty() {
        input_classes.push(border_classes.clone());
    }
    if !typography.is_empty() {
        input_classes.push(typography.clone());
    }
    if !show_button && !color_classes.is_empty() {
        input_classes.push(color_classes.clone());
    }
    input.add_class(&input_classes.join(" "));
    input.set_attribute("id", &input_id);
    input.set_attribute("value", bc.env.query.vars.get("s").unwrap_or(""));
    input.set_attribute("placeholder", attrs.get("placeholder").and_then(Value::as_str).unwrap_or(""));
    let expandable = button_position == Some("button-only");
    if expandable {
        bc.env.with_state(|s| s.script_modules.insert("@wordpress/block-library/search/view".to_owned()));
        input.set_attribute("data-wp-bind--aria-hidden", "!context.isSearchInputVisible");
        input.set_attribute("data-wp-bind--tabindex", "state.tabindex");
        input.set_attribute("aria-hidden", "true");
        input.set_attribute("tabindex", "-1");
    }
    let input = input.finish();

    let mut query_params = String::new();
    if let Some(params) = attrs.get("query").and_then(Value::as_object) {
        for (name, value) in params {
            let value = value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string());
            query_params.push_str(&format!("<input type=\"hidden\" name=\"{}\" value=\"{}\" />", esc_attr(name), esc_attr(&value)));
        }
    }

    // <button>
    let mut button = String::new();
    if show_button {
        let mut classes = vec!["wp-block-search__button".to_owned()];
        if !color_classes.is_empty() {
            classes.push(color_classes.clone());
        }
        if !typography.is_empty() {
            classes.push(typography.clone());
        }
        if !button_inside && !border_classes.is_empty() {
            classes.push(border_classes.clone());
        }
        let inner = if use_icon {
            classes.push("has-icon".to_owned());
            "<svg class=\"search-icon\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\">\n\t\t\t\t\t<path d=\"M13 5c-3.3 0-6 2.7-6 6 0 1.4.5 2.7 1.3 3.7l-3.8 3.8 1.1 1.1 3.8-3.8c1 .8 2.3 1.3 3.7 1.3 3.3 0 6-2.7 6-6S16.3 5 13 5zm0 10.5c-2.5 0-4.5-2-4.5-4.5s2-4.5 4.5-4.5 4.5 2 4.5 4.5-2 4.5-4.5 4.5z\"></path>\n\t\t\t\t</svg>".to_owned()
        } else {
            non_empty(attrs, "buttonText").unwrap_or_default().to_owned()
        };
        classes.push("wp-element-button".to_owned());
        let mut editor = TagEditor::first(&format!("<button type=\"submit\" {}>{inner}</button>", inline.button)).expect("button tag");
        editor.add_class(&classes.join(" "));
        if position == Some("button-only") {
            editor.set_attribute("data-wp-bind--aria-label", "state.ariaLabel");
            editor.set_attribute("data-wp-bind--aria-controls", "state.ariaControls");
            editor.set_attribute("data-wp-bind--aria-expanded", "context.isSearchInputVisible");
            editor.set_attribute("data-wp-bind--type", "state.type");
            editor.set_attribute("data-wp-on--click", "actions.openSearchInput");
            editor.set_attribute("aria-label", "Expand search field");
            editor.set_attribute("aria-controls", &format!("wp-block-search__input-{input_id}"));
            editor.set_attribute("aria-expanded", "false");
            editor.set_attribute("type", "button");
        } else {
            editor.set_attribute("aria-label", &strip_tags(attrs.get("buttonText").and_then(Value::as_str).unwrap_or("")));
        }
        button = editor.finish();
    }

    let mut field_classes = vec!["wp-block-search__inside-wrapper".to_owned()];
    if button_inside && !border_classes.is_empty() {
        field_classes.push(border_classes);
    }
    let field = format!("<div class=\"{}\" {}>{}</div>", esc_attr(&field_classes.join(" ")), inline.wrapper, format_args!("{input}{query_params}{button}"));

    let wrapper = bc.wrapper_attributes(&[("class", classnames)]);
    let mut directives = String::new();
    if expandable {
        let context = serde_json::json!({
            "isSearchInputVisible": false,
            "inputId": input_id,
            "ariaLabelExpanded": "Submit Search",
            "ariaLabelCollapsed": "Expand search field",
        });
        directives = format!(
            "\n\t\t data-wp-interactive=\"core/search\"\n\t\t data-wp-context='{}'\n\t\t data-wp-class--wp-block-search__searchfield-hidden=\"!context.isSearchInputVisible\"\n\t\t data-wp-on--keydown=\"actions.handleSearchKeydown\"\n\t\t data-wp-on--focusout=\"actions.handleSearchFocusout\"\n\t\t",
            crate::php_json::encode(&context)
        );
    }
    let action = esc_url(&bc.env.site.home_url("/"));
    let action = if action.ends_with('/') { action } else { format!("{action}/") };
    let tag_name = attrs.get("tagName").and_then(Value::as_str).unwrap_or("");
    if tag_name == "search" {
        return Ok(format!("<search {wrapper} {directives}><form method=\"get\" action=\"{action}\">{label}{field}</form></search>"));
    }
    Ok(format!("<form role=\"search\" method=\"get\" action=\"{action}\" {wrapper} {directives}>{label}{field}</form>"))
}

fn strip_tags(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_owned()
}
