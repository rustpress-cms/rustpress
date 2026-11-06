//! `block-supports/states.php` and `block-supports/block-visibility.php`: the styles of a block for pseudo-states
//! (`style[':hover']`) and viewports (`style['@mobile']`), and hiding a block.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value, json};

use crate::env::Env;
use crate::html::{TagEditor, tags};
use crate::registry::BlockType;
use crate::style_engine::get_styles;

type Declarations = Vec<(String, String)>;

/// `WP_Theme_JSON::ELEMENTS`.
const ELEMENTS: [(&str, &str); 13] = [
    ("link", "a:where(:not(.wp-element-button))"),
    ("heading", "h1, h2, h3, h4, h5, h6"),
    ("h1", "h1"),
    ("h2", "h2"),
    ("h3", "h3"),
    ("h4", "h4"),
    ("h5", "h5"),
    ("h6", "h6"),
    ("button", ".wp-element-button, .wp-block-button__link"),
    ("caption", ".wp-element-caption, .wp-block-audio figcaption, .wp-block-embed figcaption, .wp-block-gallery figcaption, .wp-block-image figcaption, .wp-block-table figcaption, .wp-block-video figcaption"),
    ("cite", "cite"),
    ("textInput", "textarea, input:where([type=email],[type=number],[type=password],[type=search],[type=text],[type=tel],[type=url])"),
    ("select", "select"),
];

/// `WP_Theme_JSON::VALID_BLOCK_PSEUDO_SELECTORS`.
fn block_pseudo_states(block_name: &str) -> &'static [&'static str] {
    match block_name {
        "core/button" | "core/navigation-link" => &[":hover", ":focus", ":focus-visible", ":active"],
        _ => &[],
    }
}

/// `WP_Theme_JSON::VALID_ELEMENT_PSEUDO_SELECTORS`.
fn element_pseudo_states(element: &str) -> &'static [&'static str] {
    match element {
        "link" | "button" => &[":link", ":any-link", ":visited", ":hover", ":focus", ":focus-visible", ":active"],
        _ => &[],
    }
}

/// `get_viewport_breakpoint_value_in_pixels()`: `None` for anything but a length in `px`, `em` or `rem`.
fn breakpoint_pixels(value: &str) -> Option<f64> {
    let value = value.trim();
    let (number, factor) = match (value.strip_suffix("rem"), value.strip_suffix("em"), value.strip_suffix("px")) {
        (Some(number), _, _) => (number, 16.0),
        (_, Some(number), _) => (number, 16.0),
        (_, _, Some(number)) => (number, 1.0),
        _ => return None,
    };
    let valid = match number.split_once('.') {
        Some((whole, fraction)) => whole.bytes().all(|b| b.is_ascii_digit()) && !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()),
        None => !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()),
    };
    valid.then(|| number.parse::<f64>().unwrap_or(0.0) * factor)
}

/// `WP_Theme_JSON::sanitize_viewport_settings()`: the mobile and the tablet breakpoint.
fn viewport_breakpoints(settings: Option<&Value>) -> (Option<String>, Option<String>) {
    let defaults = (Some("480px".to_owned()), Some("782px".to_owned()));
    let Some(settings) = settings.and_then(Value::as_object) else { return defaults };
    let read = |key: &str| settings.get(key).and_then(Value::as_str).and_then(|value| Some((value.trim().to_owned(), breakpoint_pixels(value)?)));
    match (read("mobile"), read("tablet")) {
        (None, None) => defaults,
        (Some((mobile, _)), None) => (Some(mobile), None),
        (None, Some((tablet, _))) => (None, Some(tablet)),
        (Some((mobile, low)), Some((tablet, high))) => (Some(mobile), (low < high).then_some(tablet)),
    }
}

/// `WP_Theme_JSON::get_viewport_media_queries()`.
pub fn viewport_media_queries(settings: Option<&Value>, include_desktop: bool) -> Vec<(&'static str, String)> {
    let (mobile, tablet) = viewport_breakpoints(settings);
    let mut queries = Vec::new();
    if let Some(mobile) = &mobile {
        queries.push(("@mobile", format!("@media (width <= {mobile})")));
    }
    if let Some(tablet) = &tablet {
        queries.push(("@tablet", match &mobile {
            Some(mobile) => format!("@media ({mobile} < width <= {tablet})"),
            None => format!("@media (width <= {tablet})"),
        }));
    }
    if include_desktop && let Some(desktop) = tablet.as_ref().or(mobile.as_ref()) {
        queries.push(("@desktop", format!("@media (width > {desktop})")));
    }
    queries
}

/// `wp_render_block_visibility_support()`.
pub fn render_visibility_support(env: &Env<'_>, bt: &BlockType, attrs: &Map<String, Value>, content: String) -> String {
    let visibility = attrs.get("metadata").and_then(|m| m.get("blockVisibility"));
    if visibility == Some(&Value::Bool(false)) {
        return String::new();
    }
    if !bt.has_support(&["visibility"], true) {
        return content;
    }
    let Some(viewports) = visibility.and_then(|v| v.get("viewport")).and_then(Value::as_object).filter(|v| !v.is_empty()) else { return content };
    let queries = viewport_media_queries(env.theme.global_settings().get("viewport"), true);
    let mut hidden: Vec<(&str, &str)> = Vec::new();
    for (size, visible) in viewports {
        if *visible == Value::Bool(false)
            && let Some((state, query)) = queries.iter().find(|(state, _)| state.trim_start_matches('@') == size)
        {
            hidden.push((state.trim_start_matches('@'), query));
        }
    }
    if hidden.is_empty() {
        return content;
    }
    hidden.sort();
    let classes: Vec<String> = hidden.iter().map(|(size, _)| format!("wp-block-hidden-{size}")).collect();
    env.with_state(|s| {
        for (class, (_, query)) in classes.iter().zip(&hidden) {
            s.block_supports_css.add(&format!(".{class}"), &[("display".to_owned(), "none !important".to_owned())], Some(query));
        }
    });
    let Some(mut editor) = TagEditor::first(&content) else { return content };
    editor.add_class(&classes.join(" "));
    let mut html = editor.finish();
    // `wp_get_loading_optimization_attributes()` must not count these images.
    let mut from = 0;
    while let Some(img) = tags(&html, from).into_iter().find(|t| !t.is_closer && t.name == "img") {
        let mut editor = TagEditor::from_tag(&html, img);
        editor.set_attribute("fetchpriority", "auto");
        let (start, _, tag) = editor.tag_replacement();
        from = start + tag.len();
        html = editor.finish();
    }
    html
}

/// `wp_normalize_state_preset_vars()`.
fn normalize_preset_vars(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.iter().map(|(k, v)| (k.clone(), normalize_preset_vars(v))).collect()),
        Value::Array(list) => Value::Array(list.iter().map(normalize_preset_vars).collect()),
        Value::String(text) if text.starts_with("var:preset|") => Value::String(format!("var(--wp--{})", text["var:".len()..].replace('|', "--"))),
        other => other.clone(),
    }
}

/// `empty()` of a state: anything but a non-empty array.
fn state_map(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value.and_then(Value::as_object).filter(|map| !map.is_empty())
}

/// `wp_split_selector_list()`: commas inside parentheses do not separate.
fn split_selector_list(selector: &str) -> Vec<String> {
    let mut selectors = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    for c in selector.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            ',' if depth == 0 => {
                selectors.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    selectors.push(current);
    selectors
}

/// `wp_build_state_selector()`.
fn build_state_selector(base: &str, block_selector: Option<&str>, state: &str) -> String {
    static LEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([.#]?[-_a-zA-Z0-9]+|\[[^\]]+\])").expect("the pattern is valid"));
    let Some(block_selector) = block_selector.filter(|s| !s.trim().is_empty()) else { return format!("{base}{state}") };
    let scoped: Vec<String> = split_selector_list(block_selector)
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|selector| match LEADING.find(selector) {
            Some(leading) => format!("{base}{}{state}", &selector[leading.end()..]),
            None => format!("{base}{state}"),
        })
        .collect();
    if scoped.is_empty() { format!("{base}{state}") } else { scoped.join(", ") }
}

/// `wp_add_state_style_group()`.
fn add_group(groups: &mut Vec<(Option<String>, Value)>, selector: Option<&str>, style: Value) {
    let key = selector.unwrap_or_default();
    match groups.iter_mut().find(|(s, _)| s.as_deref().unwrap_or_default() == key) {
        Some((_, group)) => crate::interactivity::merge(group, style),
        None => groups.push((selector.map(str::to_owned), style)),
    }
}

/// `wp_get_state_style_groups()`.
fn style_groups(state_style: &Map<String, Value>, selectors: &Value) -> Vec<(Option<String>, Value)> {
    let root = selectors.get("root").and_then(Value::as_str);
    let mut groups = Vec::new();
    for (feature, styles) in state_style {
        match selectors.get(feature) {
            Some(Value::String(selector)) => {
                add_group(&mut groups, Some(selector), json!({ feature.as_str(): styles }));
            }
            Some(Value::Object(feature_selectors)) if styles.is_object() => {
                let mut remaining = styles.as_object().cloned().unwrap_or_default();
                for (subfeature, selector) in feature_selectors {
                    let Some(selector) = selector.as_str().filter(|_| subfeature != "root") else { continue };
                    let Some(value) = styles.get(subfeature) else { continue };
                    add_group(&mut groups, Some(selector), json!({ feature.as_str(): { subfeature.as_str(): value } }));
                    remaining.remove(subfeature);
                }
                if !remaining.is_empty() {
                    let selector = feature_selectors.get("root").and_then(Value::as_str).or(root);
                    add_group(&mut groups, selector, json!({ feature.as_str(): remaining }));
                }
            }
            _ => add_group(&mut groups, root, json!({ feature.as_str(): styles })),
        }
    }
    groups
}

/// One entry of `$css_rules`.
struct Rule {
    state: String,
    selector: Option<String>,
    declarations: Declarations,
    rules_group: Option<String>,
}

/// `wp_add_block_state_style_rule()`.
fn add_rule(rules: &mut Vec<Rule>, state: &str, selector: Option<&str>, style: &Value, rules_group: Option<&str>) {
    if !style.as_object().is_some_and(|s| !s.is_empty()) {
        return;
    }
    let mut prepared = style.clone();
    if let Some(map) = prepared.as_object_mut() {
        // Layout is processed by `wp_render_layout_support_flag()`.
        map.shift_remove("layout");
    }
    let mut declarations = get_styles(&normalize_preset_vars(&prepared), false).declarations;
    // Base text alignment is class-based, so state styles need a declaration.
    if let Some(align) = style.get("typography").and_then(|t| t.get("textAlign")).and_then(Value::as_str).filter(|a| !a.trim().is_empty()) {
        match declarations.iter_mut().find(|(property, _)| property == "text-align") {
            Some(slot) => slot.1 = align.to_owned(),
            None => declarations.push(("text-align".to_owned(), align.to_owned())),
        }
    }
    if declarations.is_empty() {
        return;
    }
    rules.push(Rule { state: state.to_owned(), selector: selector.map(str::to_owned), declarations, rules_group: rules_group.map(str::to_owned) });
}

/// `wp_get_block_state_style_rules()` for one state.
fn add_state_rules(rules: &mut Vec<Rule>, state: &str, state_style: &Map<String, Value>, bt: &BlockType, rules_group: Option<&str>) {
    for (selector, style) in style_groups(state_style, &bt.selectors) {
        add_rule(rules, state, selector.as_deref(), &style, rules_group);
    }
}

/// `wp_get_block_css_selector( $block_type )`.
fn block_root_selector(bt: &BlockType) -> String {
    if let Some(root) = bt.selectors.get("root").and_then(Value::as_str) {
        return root.to_owned();
    }
    if let Some(selector) = bt.support(&["__experimentalSelector"]).and_then(Value::as_str) {
        return selector.to_owned();
    }
    format!(".wp-block-{}", bt.name.replace("core/", "").replace('/', "-"))
}

/// `wp_get_block_state_element_selectors()`: the selector of an element inside the block.
fn element_selector(root_selector: &str, element: &str) -> Option<String> {
    if root_selector.trim().is_empty() {
        return None;
    }
    let (_, element_selector) = ELEMENTS.iter().find(|(name, _)| *name == element)?;
    let mut selectors = Vec::new();
    for block_selector in split_selector_list(root_selector) {
        let block_selector = block_selector.trim();
        if block_selector.is_empty() {
            continue;
        }
        if block_selector == *element_selector {
            selectors = vec![(*element_selector).to_owned()];
            break;
        }
        if !element_selector.contains(',') {
            selectors.push(format!("{block_selector} {element_selector}"));
            continue;
        }
        selectors.push(split_selector_list(element_selector).iter().map(|s| format!("{block_selector} {s}")).collect::<Vec<_>>().join(","));
    }
    (!selectors.is_empty()).then(|| selectors.join(","))
}

/// `wp_get_state_declarations_with_background_resets()`.
fn with_background_resets(declarations: &Declarations) -> Declarations {
    let has = |property: &str| declarations.iter().any(|(p, v)| p == property && !v.is_empty());
    let mut result = declarations.clone();
    if has("background-color") && !has("background") && !has("background-image") {
        match result.iter_mut().find(|(p, _)| p == "background-image") {
            Some(slot) => slot.1 = "unset".to_owned(),
            None => result.push(("background-image".to_owned(), "unset".to_owned())),
        }
    }
    result
}

/// `wp_get_state_declarations_with_fallback_border_styles()`, without the declarations that were there.
fn fallback_border_styles(declarations: &Declarations) -> Declarations {
    let has = |property: &str| declarations.iter().any(|(p, v)| p == property && !v.is_empty());
    let has_style = has("border-style");
    let mut result = Vec::new();
    if !has_style && (has("border-color") || has("border-width")) {
        result.push(("border-style".to_owned(), "solid".to_owned()));
    }
    for side in ["top", "right", "bottom", "left"] {
        let style = format!("border-{side}-style");
        if !has_style && !has(&style) && (has(&format!("border-{side}-color")) || has(&format!("border-{side}-width"))) {
            result.push((style, "solid".to_owned()));
        }
    }
    result.retain(|(property, _)| !declarations.iter().any(|(p, _)| p == property));
    result
}

/// `wp_get_block_state_unique_class()`.
fn unique_class(block_name: &str, rules: &[Rule]) -> String {
    let rules: Vec<Value> = rules
        .iter()
        .map(|rule| {
            let declarations: Map<String, Value> = rule.declarations.iter().map(|(p, v)| (p.clone(), Value::String(v.clone()))).collect();
            let mut entry = json!({ "state": rule.state, "selector": rule.selector, "declarations": declarations });
            if let (Some(group), Some(map)) = (&rule.rules_group, entry.as_object_mut()) {
                map.insert("rules_group".to_owned(), Value::String(group.clone()));
            }
            entry
        })
        .collect();
    let digest = format!("{:x}", md5::compute(crate::php_json::encode(&json!({ "blockName": block_name, "rules": rules })).as_bytes()));
    format!("wp-states-{}", &digest[..8])
}

/// The rules of `wp_render_block_states_support()`.
fn state_rules(bt: &BlockType, style: &Map<String, Value>, queries: &[(&'static str, String)]) -> Vec<Rule> {
    let pseudo_states = block_pseudo_states(&bt.name);
    let mut rules = Vec::new();
    for state in pseudo_states {
        if let Some(state_style) = state_map(style.get(*state)) {
            add_state_rules(&mut rules, state, state_style, bt, None);
        }
    }
    for (breakpoint, query) in queries {
        let Some(viewport) = state_map(style.get(*breakpoint)) else { continue };
        let root: Map<String, Value> = viewport.iter().filter(|(key, _)| *key != "elements" && !pseudo_states.contains(&key.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        if !root.is_empty() {
            add_state_rules(&mut rules, "", &root, bt, Some(query));
        }
        if let Some(elements) = state_map(viewport.get("elements")) {
            let root_selector = block_root_selector(bt);
            for (element, element_style) in elements {
                let Some(element_style) = state_map(Some(element_style)) else { continue };
                let Some(selector) = element_selector(&root_selector, element) else { continue };
                let states = element_pseudo_states(element);
                let root_style: Map<String, Value> = element_style.iter().filter(|(key, _)| !states.contains(&key.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
                add_rule(&mut rules, "", Some(&selector), &Value::Object(root_style), Some(query));
                for state in states {
                    if let Some(state_style) = state_map(element_style.get(*state)) {
                        add_rule(&mut rules, state, Some(&selector), &Value::Object(state_style.clone()), Some(query));
                    }
                }
            }
        }
        for state in pseudo_states {
            if let Some(state_style) = state_map(viewport.get(*state)) {
                add_state_rules(&mut rules, state, state_style, bt, Some(query));
            }
        }
    }
    rules
}

/// `wp_render_block_states_support()`.
pub fn render_states_support(env: &Env<'_>, bt: &BlockType, attrs: &Map<String, Value>, content: String) -> String {
    if content.is_empty() || content == "0" {
        return content;
    }
    let Some(style) = state_map(attrs.get("style")) else { return content };
    let queries = viewport_media_queries(env.theme.global_settings().get("viewport"), false);
    let rules = state_rules(bt, style, &queries);
    if rules.is_empty() {
        return content;
    }
    let class = unique_class(&bt.name, &rules);
    env.with_state(|s| {
        for rule in &rules {
            let selector = build_state_selector(&format!(".{class}"), rule.selector.as_deref(), &rule.state);
            let important: Declarations = with_background_resets(&rule.declarations).into_iter().map(|(p, v)| (p, format!("{v} !important"))).collect();
            s.block_supports_css.add(&selector, &important, rule.rules_group.as_deref());
            let fallback = fallback_border_styles(&rule.declarations);
            if !fallback.is_empty() {
                s.block_supports_css.add(&selector, &fallback, rule.rules_group.as_deref());
            }
        }
    });
    let Some(mut editor) = TagEditor::first(&content) else { return content };
    editor.add_class(&class);
    editor.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_breakpoints_follow_the_settings() {
        let defaults = viewport_media_queries(None, false);
        assert_eq!(defaults[0], ("@mobile", "@media (width <= 480px)".to_owned()));
        assert_eq!(defaults[1], ("@tablet", "@media (480px < width <= 782px)".to_owned()));
        assert_eq!(viewport_media_queries(None, true)[2], ("@desktop", "@media (width > 782px)".to_owned()));
        assert_eq!(viewport_media_queries(Some(&json!({"tablet": " 50em "})), false), vec![("@tablet", "@media (width <= 50em)".to_owned())]);
        // A tablet width below the mobile one is dropped.
        assert_eq!(viewport_media_queries(Some(&json!({"mobile": "800px", "tablet": "700px"})), false), vec![("@mobile", "@media (width <= 800px)".to_owned())]);
        assert_eq!(viewport_media_queries(Some(&json!({"mobile": "wide"})), false), defaults);
    }

    #[test]
    fn selectors_are_scoped_to_the_class_of_the_state() {
        assert_eq!(build_state_selector(".c", None, ":hover"), ".c:hover");
        assert_eq!(build_state_selector(".c", Some(".wp-block-button .wp-block-button__link"), ":hover"), ".c .wp-block-button__link:hover");
        assert_eq!(build_state_selector(".c", Some(".a, .b:is(.x,.y)"), ""), ".c, .c:is(.x,.y)");
        assert_eq!(build_state_selector(".c", Some(":root .a"), ":hover"), ".c:hover");
    }

    #[test]
    fn preset_references_become_variables() {
        assert_eq!(normalize_preset_vars(&json!({"color": {"text": "var:preset|color|contrast"}})), json!({"color": {"text": "var(--wp--preset--color--contrast)"}}));
    }

    #[test]
    fn borders_get_a_style_only_where_none_is_set() {
        let declarations = vec![("border-color".to_owned(), "red".to_owned()), ("border-top-width".to_owned(), "1px".to_owned())];
        assert_eq!(fallback_border_styles(&declarations), vec![("border-style".to_owned(), "solid".to_owned()), ("border-top-style".to_owned(), "solid".to_owned())]);
    }
}
