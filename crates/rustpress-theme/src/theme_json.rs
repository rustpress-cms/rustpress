//! `theme.json` loading and merging: WordPress' core defaults, the parent
//! theme, the child theme and the user's customizations (`wp_global_styles`),
//! in that order.

use std::path::Path;

use serde_json::{Map, Value};

/// Recursive merge: objects are merged key by key, everything else
/// (including arrays) is replaced by the overriding value.
pub fn deep_merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(base_map), Value::Object(over_map)) => {
            for (key, value) in over_map {
                match base_map.get_mut(key) {
                    Some(existing) => deep_merge(existing, value),
                    None => {
                        base_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (base, over) => *base = over.clone(),
    }
}

/// Sets `path` to `true` unless something is already there (`do_opt_in_into_settings`).
fn opt_in_path(context: &mut Map<String, Value>, path: &[&str]) {
    let mut current = context;
    for (i, key) in path.iter().enumerate() {
        if i == path.len() - 1 {
            current.entry((*key).to_owned()).or_insert(Value::Bool(true));
            return;
        }
        let entry = current.entry((*key).to_owned()).or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            return;
        }
        current = entry.as_object_mut().expect("object");
    }
}

/// `appearanceTools: true` turns on a group of settings that are otherwise off.
fn do_opt_in(settings: &mut Map<String, Value>) {
    if settings.get("appearanceTools") != Some(&Value::Bool(true)) {
        return;
    }
    for path in [
        &["border", "color"][..],
        &["border", "radius"],
        &["border", "style"],
        &["border", "width"],
        &["color", "link"],
        &["color", "heading"],
        &["color", "button"],
        &["color", "caption"],
        &["dimensions", "aspectRatio"],
        &["dimensions", "height"],
        &["dimensions", "minHeight"],
        &["dimensions", "minWidth"],
        &["dimensions", "width"],
        &["position", "sticky"],
        &["spacing", "blockGap"],
        &["spacing", "margin"],
        &["spacing", "padding"],
        &["typography", "lineHeight"],
    ] {
        // A literal `null` (the core default for blockGap) counts as "not set".
        if let Some(Value::Null) = lookup(settings, path) {
            remove(settings, path);
        }
        opt_in_path(settings, path);
    }
    settings.remove("appearanceTools");
}

fn lookup<'a>(map: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let mut current = map.get(path[0])?;
    for key in &path[1..] {
        current = current.get(*key)?;
    }
    Some(current)
}

fn remove(map: &mut Map<String, Value>, path: &[&str]) {
    if path.len() == 1 {
        map.shift_remove(path[0]);
        return;
    }
    if let Some(Value::Object(inner)) = map.get_mut(path[0]) {
        remove(inner, &path[1..]);
    }
}

/// `maybe_opt_in_into_settings` for the top-level settings and each block's settings.
pub fn opt_in_settings(theme_json: &mut Value) {
    let Some(settings) = theme_json.get_mut("settings").and_then(Value::as_object_mut) else { return };
    do_opt_in(settings);
    if let Some(blocks) = settings.get_mut("blocks").and_then(Value::as_object_mut) {
        for block in blocks.values_mut() {
            if let Some(block_settings) = block.as_object_mut() {
                do_opt_in(block_settings);
            }
        }
    }
}

/// `{ "ref": "styles.elements.button.border.radius" }` -> the value found at that path of the
/// styles tree (`WP_Theme_JSON::resolve_value_references`); an unresolvable reference is removed.
pub fn resolve_refs(styles: &mut Value) {
    for _ in 0..3 {
        let root = styles.clone();
        replace_refs(styles, &root);
    }
}

fn replace_refs(value: &mut Value, root: &Value) {
    match value {
        Value::Object(map) => {
            if let Some(path) = map.get("ref").and_then(Value::as_str).filter(|_| map.len() == 1) {
                let mut found = Some(root);
                for key in path.strip_prefix("styles.").unwrap_or(path).split('.') {
                    found = found.and_then(|v| v.get(key));
                }
                *value = found.filter(|v| !v.is_object() || v.get("ref").is_none()).cloned().unwrap_or(Value::Null);
                return;
            }
            for child in map.values_mut() {
                replace_refs(child, root);
            }
            map.retain(|_, v| !v.is_null());
        }
        Value::Array(items) => items.iter_mut().for_each(|v| replace_refs(v, root)),
        _ => {}
    }
}
/// `var:preset|spacing|50` -> `var(--wp--preset--spacing--50)` everywhere in a
/// styles tree, as `wp_get_global_styles()` returns it.
pub fn resolve_custom_vars(value: &mut Value) {
    match value {
        Value::String(s) => {
            if let Some(rest) = s.strip_prefix("var:") {
                *s = format!("var(--wp--{})", rest.replace('|', "--"));
            }
        }
        Value::Array(items) => items.iter_mut().for_each(resolve_custom_vars),
        Value::Object(map) => map.values_mut().for_each(resolve_custom_vars),
        _ => {}
    }
}

/// Registers a partial under `styles.blocks.<type>.variations.<slug>`.
fn add_variation_partial(theme: &mut Value, partial: &Value) {
    let Some(styles) = partial.get("styles").filter(|s| s.is_object()) else { return };
    let slug = partial
        .get("slug")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| partial.get("title").and_then(Value::as_str).map(rustpress_blocks::style_engine::to_kebab_case));
    let (Some(slug), Some(types)) = (slug, partial.get("blockTypes").and_then(Value::as_array)) else { return };
    for block_type in types.iter().filter_map(Value::as_str) {
        let mut patch = Value::Object(Map::new());
        patch["styles"]["blocks"][block_type]["variations"][&slug] = styles.clone();
        deep_merge(theme, &patch);
    }
}

/// The variation `slug` of `block_name` from the merged styles.
pub fn find_variation<'a>(styles: &'a Value, block_name: &str, slug: &str) -> Option<&'a Value> {
    styles.get("blocks")?.get(block_name)?.get("variations")?.get(slug).filter(|v| v.as_object().is_some_and(|o| !o.is_empty()))
}

/// Reads and parses a JSON file; `None` if missing or invalid.
pub fn read_file(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Value>(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "invalid theme.json");
            None
        }
    }
}

/// The resolved layers.
#[derive(Debug, Clone, Default)]
pub struct ThemeJson {
    /// Core defaults, the theme(s) and the user merged: what blocks consult.
    pub merged: Value,
    pub core: Value,
    pub theme: Value,
    pub user: Value,
}

impl ThemeJson {
    pub fn new(core: Value, parent: Option<Value>, child: Option<Value>, user: Option<Value>) -> Self {
        Self::with_partials(core, parent, child, user, &[])
    }

    /// Like [`ThemeJson::new`], with block style variation partials
    /// (`styles/blocks/*.json`: `{"slug", "blockTypes": [...], "styles": {...}}`).
    pub fn with_partials(core: Value, parent: Option<Value>, child: Option<Value>, user: Option<Value>, partials: &[Value]) -> Self {
        Self::with_layers(core, Value::Null, parent, child, user, partials)
    }

    /// Core defaults, then the styles blocks declare themselves (`blocks`), the theme and the user.
    pub fn with_layers(core: Value, blocks: Value, parent: Option<Value>, child: Option<Value>, user: Option<Value>, partials: &[Value]) -> Self {
        let mut theme = Value::Object(Map::new());
        for layer in [parent, child].into_iter().flatten() {
            let mut layer = layer;
            opt_in_settings(&mut layer);
            set_spacing_sizes(&mut layer, "theme");
            deep_merge(&mut theme, &layer);
        }
        for partial in partials {
            add_variation_partial(&mut theme, partial);
        }
        let mut core = core;
        opt_in_settings(&mut core);
        set_spacing_sizes(&mut core, "default");
        let mut user = user.unwrap_or(Value::Object(Map::new()));
        opt_in_settings(&mut user);
        set_spacing_sizes(&mut user, "custom");

        let mut merged = core.clone();
        if blocks.is_object() {
            deep_merge(&mut merged, &blocks);
        }
        deep_merge(&mut merged, &theme);
        deep_merge(&mut merged, &user);
        Self { merged, core, theme, user }
    }

    pub fn settings(&self) -> Value {
        self.merged.get("settings").cloned().unwrap_or(Value::Object(Map::new()))
    }

    pub fn styles(&self) -> Value {
        let mut styles = self.merged.get("styles").cloned().unwrap_or(Value::Object(Map::new()));
        resolve_refs(&mut styles);
        resolve_custom_vars(&mut styles);
        styles
    }

    pub fn template_parts(&self) -> Vec<Value> {
        self.theme.get("templateParts").and_then(Value::as_array).cloned().unwrap_or_default()
    }

    pub fn custom_templates(&self) -> Vec<Value> {
        self.theme.get("customTemplates").and_then(Value::as_array).cloned().unwrap_or_default()
    }
}

/// `WP_Theme_JSON::compute_spacing_sizes()`: the sizes a `spacingScale` stands for.
fn compute_spacing_sizes(scale: &Value) -> Vec<Value> {
    let number = |key: &str| scale.get(key).and_then(Value::as_f64);
    let (Some(steps), Some(medium), Some(increment)) = (number("steps"), number("mediumStep"), number("increment")) else { return Vec::new() };
    let (Some(unit), Some(operator)) = (scale.get("unit").and_then(Value::as_str), scale.get("operator").and_then(Value::as_str)) else { return Vec::new() };
    if steps == 0.0 || !matches!(operator, "+" | "*") {
        return Vec::new();
    }
    let unit = if unit == "%" { "%".to_owned() } else { unit.to_lowercase() };
    let round2 = |v: f64| {
        let r = (v * 100.0).round() / 100.0;
        format!("{r}")
    };
    // PHP's `round()` rounds halves away from zero.
    let mid = (steps / 2.0).round();
    let mut sizes: Vec<(String, String)> = Vec::new();
    let mut current = medium;
    let mut slug = 40i64;
    let mut remainder = 0.0;
    let mut below_count = mid - 1.0;
    let mut below: Vec<(String, String)> = Vec::new();
    while steps > 1.0 && slug > 0 && below_count > 0.0 {
        if operator == "+" {
            current -= increment;
        } else if increment > 1.0 {
            current /= increment;
        } else {
            current *= increment;
        }
        if current <= 0.0 {
            remainder = below_count;
            break;
        }
        below.push((slug.to_string(), format!("{}{unit}", round2(current))));
        slug -= 10;
        below_count -= 1.0;
    }
    below.reverse();
    sizes.extend(below);
    sizes.push(("50".to_owned(), format!("{}{unit}", scale.get("mediumStep").map(|v| v.to_string()).unwrap_or_default())));
    let mut current = medium;
    let mut slug = 60i64;
    let steps_above = (steps - mid) + remainder;
    let mut index = 0.0;
    while index < steps_above {
        current = if operator == "+" {
            current + increment
        } else if increment >= 1.0 {
            current * increment
        } else {
            current / increment
        };
        sizes.push((slug.to_string(), format!("{}{unit}", round2(current))));
        slug += 10;
        index += 1.0;
    }
    // With seven steps or fewer the names are numbers.
    let numbered = steps <= 7.0;
    sizes
        .into_iter()
        .enumerate()
        .map(|(i, (slug, size))| serde_json::json!({"name": if numbered { (i + 1).to_string() } else { slug.clone() }, "slug": slug, "size": size}))
        .collect()
}

/// `WP_Theme_JSON::set_spacing_sizes()`: sizes computed from the layer's own `spacingScale` are
/// its `default` origin; sizes it lists itself stay in `origin`.
fn set_spacing_sizes(layer: &mut Value, origin: &str) {
    let Some(scale) = layer.get("settings").and_then(|s| s.get("spacing")).and_then(|s| s.get("spacingScale")).cloned() else { return };
    let computed = compute_spacing_sizes(&scale);
    if computed.is_empty() {
        return;
    }
    let Some(spacing) = layer.get_mut("settings").and_then(|s| s.get_mut("spacing")).and_then(Value::as_object_mut) else { return };
    let mut by_origin = match spacing.remove("spacingSizes") {
        Some(Value::Array(items)) => {
            let mut map = Map::new();
            map.insert(origin.to_owned(), Value::Array(items));
            map
        }
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    by_origin.insert("default".to_owned(), Value::Array(computed));
    spacing.insert("spacingSizes".to_owned(), Value::Object(by_origin));
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn objects_merge_and_arrays_replace() {
        let mut base = json!({"a": {"x": 1, "y": [1, 2]}, "b": 1});
        deep_merge(&mut base, &json!({"a": {"y": [3]}, "c": 2}));
        assert_eq!(base, json!({"a": {"x": 1, "y": [3]}, "b": 1, "c": 2}));
    }

    #[test]
    fn appearance_tools_opt_in_unset_settings() {
        let mut tj = json!({"settings": {"appearanceTools": true, "spacing": {"blockGap": null, "margin": false}}});
        opt_in_settings(&mut tj);
        let s = &tj["settings"];
        assert_eq!(s["spacing"]["blockGap"], true);
        assert_eq!(s["spacing"]["margin"], false, "explicit values are kept");
        assert_eq!(s["spacing"]["padding"], true);
        assert_eq!(s["border"]["radius"], true);
        assert!(s.get("appearanceTools").is_none());
    }

    #[test]
    fn preset_references_are_resolved_in_styles() {
        let tj = ThemeJson::new(json!({}), Some(json!({"styles": {"blocks": {"core/columns": {"spacing": {"blockGap": "var:preset|spacing|50"}}}, "css": "a|b"}})), None, None);
        assert_eq!(tj.styles()["blocks"]["core/columns"]["spacing"]["blockGap"], "var(--wp--preset--spacing--50)");
        assert_eq!(tj.styles()["css"], "a|b");
    }

    #[test]
    fn layers_are_merged_in_order() {
        let core = json!({"settings": {"layout": {"contentSize": "1"}, "useRootPaddingAwareAlignments": false}});
        let parent = json!({"settings": {"layout": {"contentSize": "2", "wideSize": "3"}}, "templateParts": [{"name": "header"}]});
        let child = json!({"settings": {"layout": {"wideSize": "4"}}});
        let user = json!({"settings": {"useRootPaddingAwareAlignments": true}});
        let tj = ThemeJson::new(core, Some(parent), Some(child), Some(user));
        assert_eq!(tj.settings()["layout"], json!({"contentSize": "2", "wideSize": "4"}));
        assert_eq!(tj.settings()["useRootPaddingAwareAlignments"], true);
        assert_eq!(tj.template_parts().len(), 1);
    }

    #[test]
    fn the_default_spacing_scale_yields_seven_sizes() {
        let sizes = compute_spacing_sizes(&serde_json::json!({"operator": "*", "increment": 1.5, "steps": 7, "mediumStep": 1.5, "unit": "rem"}));
        let pairs: Vec<(String, String)> = sizes.iter().map(|s| (s["slug"].as_str().unwrap().to_owned(), s["size"].as_str().unwrap().to_owned())).collect();
        let expected = [("20", "0.44rem"), ("30", "0.67rem"), ("40", "1rem"), ("50", "1.5rem"), ("60", "2.25rem"), ("70", "3.38rem"), ("80", "5.06rem")];
        assert_eq!(pairs, expected.map(|(a, b)| (a.to_owned(), b.to_owned())));
        assert!(compute_spacing_sizes(&serde_json::json!({"steps": 0})).is_empty());
    }

    #[test]
    fn refs_point_to_other_style_values() {
        let mut styles = serde_json::json!({"elements": {"button": {"border": {"radius": "4px"}}}, "blocks": {"core/search": {"elements": {"button": {"border": {"radius": {"ref": "styles.elements.button.border.radius"}}}}}}});
        resolve_refs(&mut styles);
        assert_eq!(styles["blocks"]["core/search"]["elements"]["button"]["border"]["radius"], "4px");
    }}
