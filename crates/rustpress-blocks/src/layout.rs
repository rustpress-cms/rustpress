//! Layout block support (`wp-includes/block-supports/layout.php`): the
//! `is-layout-*` classes, container classes with a hash of the layout, and
//! the CSS rules that go with them.

use serde_json::{Map, Value, json};

use crate::env::{Env, State};
use crate::html::TagEditor;
use crate::php_json::unique_id_from_values;
use crate::registry::BlockType;
use crate::style_engine::{get_styles, spacing_preset_to_var};

type Decls = Vec<(String, String)>;

fn d(items: &[(&str, &str)]) -> Decls {
    items.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

/// `className` of a layout type (`wp_get_layout_definitions`).
pub fn layout_class_name(layout_type: &str) -> Option<&'static str> {
    match layout_type {
        "default" => Some("is-layout-flow"),
        "constrained" => Some("is-layout-constrained"),
        "flex" => Some("is-layout-flex"),
        "grid" => Some("is-layout-grid"),
        _ => None,
    }
}

/// `wp_sanitize_block_gap_value`.
pub fn sanitize_block_gap_value(gap: Option<&Value>) -> Option<Value> {
    let unsafe_value = |s: &str| !s.is_empty() && (s.contains(['\\', '(', '&', '=', '}']) || s.contains("/*"));
    match gap? {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                let clean = match v {
                    Value::String(s) if !unsafe_value(s) => v.clone(),
                    Value::Number(_) | Value::Bool(_) => v.clone(),
                    _ => Value::Null,
                };
                out.insert(k.clone(), clean);
            }
            Some(Value::Object(out))
        }
        Value::String(s) if unsafe_value(s) => None,
        Value::Null => None,
        other => Some(other.clone()),
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Combines a (possibly two-sided) gap value into `top left`.
fn combined_gap(gap_value: &Value, fallback: &Value) -> String {
    let sides: &[&str] = if gap_value.is_object() { &["top", "left"] } else { &["top"] };
    let mut combined = String::new();
    for side in sides {
        let mut process = gap_value.clone();
        if let Some(map) = gap_value.as_object() {
            let fallback_value = match fallback {
                Value::Object(f) => f.get(*side).or_else(|| f.values().next()).cloned().unwrap_or(Value::Null),
                other => other.clone(),
            };
            process = map.get(*side).filter(|v| !v.is_null()).cloned().unwrap_or(fallback_value);
        }
        combined.push_str(&spacing_preset_to_var(&value_to_string(&process)));
        combined.push(' ');
    }
    combined.trim().to_owned()
}

/// The `$options` of `wp_get_layout_style()`: the rules of one viewport print only what it changes.
#[derive(Default)]
pub struct LayoutOptions<'a> {
    pub rules_group: Option<&'a str>,
    pub viewport_overrides: Option<&'a Map<String, Value>>,
    pub has_block_gap_override: bool,
}

/// `wp_get_layout_style`: stores the rules and reports whether there are any.
#[allow(clippy::too_many_arguments)]
pub fn get_layout_style(
    state: &mut State,
    selector: &str,
    base_layout: &Map<String, Value>,
    has_block_gap_support: bool,
    gap_value: Option<&Value>,
    skip_gap_serialization: bool,
    fallback_gap: &Value,
    block_spacing: Option<&Value>,
    options: &LayoutOptions<'_>,
) -> bool {
    let overrides = options.viewport_overrides;
    let is_viewport = overrides.is_some();
    let merged;
    let layout = match overrides {
        Some(overrides) => {
            let mut all = base_layout.clone();
            all.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
            merged = all;
            &merged
        }
        None => base_layout,
    };
    let layout_type = base_layout.get("type").and_then(Value::as_str).unwrap_or("default");
    let output_block_gap = !is_viewport || options.has_block_gap_override;
    let overridden = |key: &str| overrides.is_some_and(|o| o.contains_key(key));
    let not_empty = |map: &Map<String, Value>, key: &str| map.get(key).is_some_and(|v| !crate::style_engine::is_empty(v));
    let mut rules: Vec<(String, Decls)> = Vec::new();
    let lget = |k: &str| layout.get(k).filter(|v| !v.is_null());
    let lstr = |k: &str| lget(k).and_then(Value::as_str).unwrap_or("");

    let zero_gap_rules = |rules: &mut Vec<(String, Decls)>, gap: &Value| {
        let gap = spacing_preset_to_var(&value_to_string(gap));
        rules.push((format!("{selector} > *"), d(&[("margin-block-start", "0"), ("margin-block-end", "0")])));
        rules.push((format!("{selector} > * + *"), vec![("margin-block-start".into(), gap), ("margin-block-end".into(), "0".into())]));
    };
    let block_gap_rules = |rules: &mut Vec<(String, Decls)>| {
        if has_block_gap_support && output_block_gap {
            let gap = gap_value.map(|g| if let Value::Object(m) = g { m.get("top").cloned().unwrap_or(Value::Null) } else { g.clone() });
            if let Some(gap) = gap.filter(|g| !g.is_null())
                && !skip_gap_serialization
            {
                zero_gap_rules(rules, &gap);
            }
        }
    };

    match layout_type {
        "default" => block_gap_rules(&mut rules),
        "constrained" => {
            let content_size = lstr("contentSize");
            let wide_size = lstr("wideSize");
            let justify = lget("justifyContent").and_then(Value::as_str).unwrap_or("center");
            let content_override = overridden("contentSize");
            let wide_override = overridden("wideSize");
            let justify_override = overridden("justifyContent");
            let output_sizes = !is_viewport || content_override || wide_override;
            let resetting = is_viewport && ((content_override && content_size.is_empty()) || (wide_override && wide_size.is_empty()));
            let all_max = if !content_size.is_empty() {
                content_size.to_owned()
            } else if !wide_size.is_empty() && !content_override {
                wide_size.to_owned()
            } else {
                "var(--wp--style--global--content-size, none)".to_owned()
            };
            let wide_max = if !wide_size.is_empty() {
                wide_size.to_owned()
            } else if !content_size.is_empty() && !wide_override {
                content_size.to_owned()
            } else {
                "var(--wp--style--global--wide-size, none)".to_owned()
            };
            let first_rule = |s: &str| s.split(';').next().unwrap_or("").trim().to_owned();
            let margin_left = if justify == "left" { "0 !important" } else { "auto !important" };
            let margin_right = if justify == "right" { "0 !important" } else { "auto !important" };
            let inner = format!("{selector} > :where(:not(.alignleft):not(.alignright):not(.alignfull))");
            if output_sizes && (!content_size.is_empty() || !wide_size.is_empty() || resetting) {
                let mut declarations: Decls = vec![("max-width".into(), first_rule(&all_max))];
                if !is_viewport || justify_override {
                    declarations.push(("margin-left".into(), margin_left.into()));
                    declarations.push(("margin-right".into(), margin_right.into()));
                }
                rules.push((inner.clone(), declarations));
                rules.push((format!("{selector} > .alignwide"), vec![("max-width".into(), first_rule(&wide_max))]));
                rules.push((format!("{selector} .alignfull"), d(&[("max-width", "none")])));
            }
            if let (false, Some(spacing)) = (is_viewport, block_spacing) {
                let styles = get_styles(&json!({ "spacing": spacing }), false);
                for (side, property) in [("padding-right", "margin-right"), ("padding-left", "margin-left")] {
                    if let Some(padding) = styles.declaration(side) {
                        let padding = if padding == "0" { "0px" } else { padding };
                        rules.push((format!("{selector} > .alignfull"), vec![(property.into(), format!("calc({padding} * -1)"))]));
                    }
                }
            }
            if justify_override && !output_sizes {
                rules.push((inner, vec![("margin-left".into(), margin_left.into()), ("margin-right".into(), margin_right.into())]));
            } else if !is_viewport {
                if justify == "left" {
                    rules.push((inner.clone(), d(&[("margin-left", "0 !important")])));
                }
                if justify == "right" {
                    rules.push((inner, d(&[("margin-right", "0 !important")])));
                }
            }
            block_gap_rules(&mut rules);
        }
        "flex" => {
            let orientation = lget("orientation").and_then(Value::as_str).unwrap_or("horizontal");
            let horizontal = orientation == "horizontal";
            let justify_options: &[(&str, &str)] = if horizontal {
                &[("left", "flex-start"), ("right", "flex-end"), ("center", "center"), ("space-between", "space-between")]
            } else {
                &[("left", "flex-start"), ("right", "flex-end"), ("center", "center"), ("stretch", "stretch")]
            };
            let vertical_options: &[(&str, &str)] = if horizontal {
                &[("top", "flex-start"), ("center", "center"), ("bottom", "flex-end"), ("stretch", "stretch")]
            } else {
                &[("top", "flex-start"), ("center", "center"), ("bottom", "flex-end"), ("space-between", "space-between")]
            };
            fn option<'a>(list: &[(&str, &'a str)], key: &str) -> Option<&'a str> {
                list.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
            }

            let output_wrap = !is_viewport || overridden("flexWrap");
            let output_orientation = !is_viewport || overridden("orientation");
            let output_justification = !is_viewport || overridden("justifyContent") || overridden("orientation");
            let output_alignment = !is_viewport || overridden("verticalAlignment") || overridden("orientation");

            if output_wrap && lstr("flexWrap") == "nowrap" {
                rules.push((selector.to_owned(), d(&[("flex-wrap", "nowrap")])));
            }
            if has_block_gap_support
                && output_block_gap
                && let Some(gap) = gap_value.filter(|g| !g.is_null())
                && !skip_gap_serialization
            {
                rules.push((selector.to_owned(), vec![("gap".into(), combined_gap(gap, fallback_gap))]));
            }
            let justify = lget("justifyContent").and_then(Value::as_str).unwrap_or("");
            let vertical = lget("verticalAlignment").and_then(Value::as_str).unwrap_or("");
            if horizontal {
                if let (true, Some(v)) = (output_justification, option(justify_options, justify)) {
                    rules.push((selector.to_owned(), d(&[("justify-content", v)])));
                }
                if let (true, Some(v)) = (output_alignment, option(vertical_options, vertical)) {
                    rules.push((selector.to_owned(), d(&[("align-items", v)])));
                }
            } else {
                if output_orientation {
                    rules.push((selector.to_owned(), d(&[("flex-direction", "column")])));
                }
                if output_justification {
                    match option(justify_options, justify) {
                        Some(v) if !justify.is_empty() => rules.push((selector.to_owned(), d(&[("align-items", v)]))),
                        _ => rules.push((selector.to_owned(), d(&[("align-items", "flex-start")]))),
                    }
                }
                if let (true, Some(v)) = (output_alignment, option(vertical_options, vertical)) {
                    rules.push((selector.to_owned(), d(&[("justify-content", v)])));
                }
            }
        }
        "grid" => {
            let mut responsive_gap = match fallback_gap {
                Value::Object(f) => f.get("left").or_else(|| f.values().next()).map(value_to_string).unwrap_or_default(),
                other => value_to_string(other),
            };
            let mut gap_string = None;
            if has_block_gap_support
                && let Some(gap) = gap_value.filter(|g| !g.is_null())
            {
                let combined = combined_gap(gap, fallback_gap);
                responsive_gap = combined.clone();
                gap_string = Some(combined);
            }
            if responsive_gap == "0" {
                responsive_gap = "0px".into();
            }
            let column_count = lget("columnCount").map(value_to_string).filter(|c| !c.is_empty() && c != "0");
            let min_width = lget("minimumColumnWidth").map(value_to_string).filter(|c| !c.is_empty() && c != "0");
            let mut output_columns = !is_viewport || overridden("minimumColumnWidth") || overridden("columnCount") || overridden("autoFit");
            if options.has_block_gap_override && column_count.is_some() && min_width.is_some() {
                output_columns = true;
            }
            let rows = lget("rowCount").map(value_to_string).filter(|r| !r.is_empty() && r != "0");
            let output_rows = (!is_viewport || overridden("rowCount")) && column_count.is_some() && rows.is_some();
            let auto_placement = if lget("autoFit").is_some_and(|v| !crate::style_engine::is_empty(v)) { "auto-fit" } else { "auto-fill" };
            let mut grid: Decls = Vec::new();
            if output_columns {
                match (&column_count, &min_width) {
                    (Some(count), Some(min)) => {
                        let max_value = format!("max(min({min}, 100%), (100% - ({responsive_gap} * ({count} - 1))) /{count})");
                        grid.push(("grid-template-columns".into(), format!("repeat({auto_placement}, minmax({max_value}, 1fr))")));
                    }
                    (Some(count), None) => grid.push(("grid-template-columns".into(), format!("repeat({count}, minmax(0, 1fr))"))),
                    (None, min) => {
                        let min = min.clone().unwrap_or_else(|| "12rem".into());
                        grid.push(("grid-template-columns".into(), format!("repeat({auto_placement}, minmax(min({min}, 100%), 1fr))")));
                    }
                }
            }
            if !grid.is_empty() {
                let base_has_container_type = !not_empty(base_layout, "columnCount") || not_empty(base_layout, "minimumColumnWidth");
                if (column_count.is_none() || min_width.is_some()) && (!is_viewport || !base_has_container_type) {
                    grid.push(("container-type".into(), "inline-size".into()));
                }
                rules.push((selector.to_owned(), grid));
            }
            if let (true, Some(rows)) = (output_rows, rows) {
                rules.push((selector.to_owned(), vec![("grid-template-rows".into(), format!("repeat({rows}, minmax(1rem, auto))"))]));
            }
            if let Some(gap) = gap_string.filter(|_| output_block_gap && !skip_gap_serialization) {
                rules.push((selector.to_owned(), vec![("gap".into(), gap)]));
            }
        }
        _ => {}
    }

    let any = !rules.is_empty();
    for (sel, decls) in rules {
        state.block_supports_css.add(&sel, &decls, options.rules_group);
    }
    any
}

/// `wp_get_child_layout_style_rules()`: child layout rules (`selfStretch`, grid placement) and, for a child of a
/// responsive grid, the container query that resets the placement. The third element is the `@container` group.
/// `viewport` holds the values of one viewport, which only print what they change.
fn child_layout_rules(selector: &str, base: &Map<String, Value>, parent: &Map<String, Value>, viewport: Option<&Map<String, Value>>) -> Vec<(String, Decls, Option<String>)> {
    let merged;
    let child = match viewport {
        Some(overrides) => {
            let mut all = base.clone();
            all.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
            merged = all;
            &merged
        }
        None => base,
    };
    let changed = |keys: &[&str]| viewport.is_none_or(|overrides| keys.iter().any(|key| overrides.contains_key(*key)));
    let mut decls: Decls = Vec::new();
    let set = |decls: &mut Decls, property: &str, value: String| match decls.iter_mut().find(|(p, _)| p == property) {
        Some(slot) => slot.1 = value,
        None => decls.push((property.to_owned(), value)),
    };
    let self_stretch = child.get("selfStretch").and_then(Value::as_str);
    let base_stretch = base.get("selfStretch").and_then(Value::as_str);
    let is_fixed = |value: Option<&str>| matches!(value, Some("fixed" | "fixedNoShrink"));
    let flex_size = child.get("flexSize").filter(|v| !v.is_null());
    if changed(&["selfStretch", "flexSize"]) {
        if viewport.is_some() && matches!(self_stretch, Some("fit" | "fill")) && is_fixed(base_stretch) && base.get("flexSize").is_some_and(|v| !v.is_null()) {
            set(&mut decls, "flex-basis", "unset".into());
            if base_stretch == Some("fixedNoShrink") {
                set(&mut decls, "flex-shrink", "unset".into());
            }
        }
        if let (true, Some(size)) = (is_fixed(self_stretch), flex_size) {
            set(&mut decls, "flex-basis", value_to_string(size));
            if self_stretch == Some("fixedNoShrink") {
                set(&mut decls, "flex-shrink", "0".into());
            } else if viewport.is_some() && base_stretch == Some("fixedNoShrink") {
                set(&mut decls, "flex-shrink", "unset".into());
            }
            set(&mut decls, "box-sizing", "border-box".into());
        } else if self_stretch == Some("fill") {
            set(&mut decls, "flex-grow", "1".into());
        }
    }
    let place = |start: Option<String>, span: Option<String>| match (start, span) {
        (Some(s), Some(n)) => Some(format!("{s} / span {n}")),
        (Some(s), None) => Some(s),
        (None, Some(n)) => Some(format!("span {n}")),
        _ => None,
    };
    let get = |k: &str| child.get(k).filter(|v| !crate::style_engine::is_empty(v)).map(value_to_string);
    if changed(&["columnStart", "columnSpan"]) && let Some(v) = place(get("columnStart"), get("columnSpan")) {
        decls.push(("grid-column".into(), v));
    }
    if changed(&["rowStart", "rowSpan"]) && let Some(v) = place(get("rowStart"), get("rowSpan")) {
        decls.push(("grid-row".into(), v));
    }
    let mut rules = Vec::new();
    if !decls.is_empty() {
        rules.push((selector.to_owned(), decls, None));
    }
    // A span or start of a responsive grid (minimum column width, or no column count) is dropped when the grid is narrower.
    let column_start = child.get("columnStart").filter(|v| !crate::style_engine::is_empty(v));
    let column_span = child.get("columnSpan").filter(|v| !crate::style_engine::is_empty(v));
    let minimum = parent.get("minimumColumnWidth").filter(|v| !crate::style_engine::is_empty(v)).and_then(Value::as_str);
    let has_count = parent.get("columnCount").is_some_and(|v| !crate::style_engine::is_empty(v));
    if viewport.is_none() && (column_start.is_some() || column_span.is_some()) && (minimum.is_some() || !has_count) {
        let number = |v: Option<&Value>| v.map_or(0.0, |v| match v {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            Value::String(s) => crate::general::floatval(s),
            _ => 0.0,
        });
        let (span_number, start_number) = (number(column_span), number(column_start));
        let width = minimum.unwrap_or("12rem");
        let value = crate::general::floatval(width);
        let unit = crate::general::php_float(value);
        let (parent_value, parent_unit) = match width.split_once(unit.as_str()) {
            Some((_, rest)) if matches!(rest, "px" | "rem" | "em") => (value, rest.to_owned()),
            _ => (12.0, "rem".to_owned()),
        };
        let break_at = if span_number != 0.0 && start_number != 0.0 {
            start_number + span_number - 1.0
        } else if span_number != 0.0 {
            span_number
        } else {
            start_number
        };
        let gap = if parent_unit == "px" { 24.0 } else { 1.5 };
        let query = (break_at * parent_value + (break_at - 1.0) * gap).max(parent_value * 2.0 + gap - 1.0);
        let grid_column = if span_number > 1.0 { "1/-1" } else { "auto" };
        rules.push((
            selector.to_owned(),
            vec![("grid-column".to_owned(), grid_column.to_owned()), ("grid-row".to_owned(), "auto".to_owned())],
            Some(format!("@container (max-width: {}{parent_unit} )", crate::general::php_float(query))),
        ));
    }
    rules
}

/// `wp_render_position_support()`: sticky (and fixed) positioning gets a container class and its rules.
pub fn render_position_support(env: &Env<'_>, bt: &BlockType, attrs: &Map<String, Value>, content: String) -> String {
    let position = attrs.get("style").and_then(|s| s.get("position")).filter(|p| !crate::style_engine::is_empty(p));
    if !bt.has_support(&["position"], false) || position.is_none() {
        return content;
    }
    let position = position.and_then(Value::as_object).cloned().unwrap_or_default();
    let settings = env.theme.global_settings();
    let allowed = |kind: &str| settings.get("position").and_then(|p| p.get(kind)) == Some(&Value::Bool(true));
    let position_type = position.get("type").and_then(Value::as_str).unwrap_or_default().to_owned();
    if !(position_type == "sticky" && allowed("sticky") || position_type == "fixed" && allowed("fixed")) {
        // The class name is taken even if nothing is output.
        env.with_state(|s| s.unique_id("wp-container-"));
        return content;
    }
    let class = env.with_state(|s| s.unique_id("wp-container-"));
    let selector = format!(".{class}");
    env.with_state(|s| {
        for side in ["top", "right", "bottom", "left"] {
            let Some(value) = position.get(side).filter(|v| !v.is_null()) else { continue };
            let mut value = value_to_string(value);
            if side == "top" {
                if value == "0" {
                    value = "0px".into();
                }
                value = format!("calc({value} + var(--wp-admin--admin-bar--position-offset, 0px))");
            }
            s.block_supports_css.add(&selector, &[(side.to_owned(), value)], None);
        }
        s.block_supports_css.add(&selector, &[("position".to_owned(), position_type.clone()), ("z-index".to_owned(), "10".to_owned())], None);
    });
    match TagEditor::first(&content) {
        Some(mut editor) => {
            editor.add_class(&class);
            editor.add_class(&format!("is-position-{position_type}"));
            editor.finish()
        }
        None => content,
    }
}

const CHILD_KEYS: [&str; 6] = ["selfStretch", "flexSize", "columnStart", "columnSpan", "rowStart", "rowSpan"];

/// `wp_render_layout_support_flag`: adds the layout classes to the block's
/// outermost tag, or to the inner wrapper for blocks with inner blocks.
pub fn render_layout_support(
    env: &Env<'_>,
    bt: &BlockType,
    attrs: &Map<String, Value>,
    parent_layout: Option<&Value>,
    first_inner_chunk: Option<&str>,
    has_inner_blocks: bool,
    content: String,
) -> String {
    let supports_layout = bt.has_support(&["layout"], false) || bt.has_support(&["__experimentalLayout"], false);
    let style_attr = attrs.get("style").and_then(Value::as_object);
    if !supports_layout && style_attr.is_none_or(|s| s.is_empty()) {
        return content;
    }
    let queries = crate::states::viewport_media_queries(env.theme.global_settings().get("viewport"), false);
    let child_layout = style_attr.and_then(|s| s.get("layout")).and_then(Value::as_object).filter(|l| !l.is_empty());
    // Viewport child layouts, so that a block with only those is still processed.
    let child_values = |layout: Option<&Value>| -> Map<String, Value> { layout.and_then(Value::as_object).map(|l| l.iter().filter(|(k, _)| CHILD_KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default() };
    let viewport_children: Vec<(&str, &str, Map<String, Value>)> = queries
        .iter()
        .filter_map(|(breakpoint, query)| {
            let child = child_values(style_attr.and_then(|s| s.get(*breakpoint)).and_then(|v| v.get("layout")));
            (!child.is_empty()).then_some((*breakpoint, query.as_str(), child))
        })
        .collect();
    if !supports_layout && child_layout.is_none() && viewport_children.is_empty() {
        return content;
    }

    let mut outer_class_names: Vec<String> = Vec::new();
    if child_layout.is_some() || !viewport_children.is_empty() {
        let base = child_values(style_attr.and_then(|s| s.get("layout")));
        let parent_subset: Map<String, Value> = parent_layout
            .and_then(Value::as_object)
            .map(|p| p.iter().filter(|(k, _)| matches!(k.as_str(), "minimumColumnWidth" | "columnCount")).map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let mut hash_input = json!({ "layout": base, "parentLayout": parent_subset });
        for (breakpoint, _, child) in &viewport_children {
            hash_input[*breakpoint] = Value::Object(child.clone());
        }
        let class = unique_id_from_values(&hash_input, "wp-container-content-");
        let mut rules = child_layout_rules(&format!(".{class}"), &base, &parent_subset, None);
        for (_, query, child) in &viewport_children {
            rules.extend(child_layout_rules(&format!(".{class}"), &base, &parent_subset, Some(child)).into_iter().map(|(sel, decls, _)| (sel, decls, Some((*query).to_owned()))));
        }
        if !rules.is_empty() {
            env.with_state(|s| {
                for (sel, decls, group) in &rules {
                    s.block_supports_css.add(sel, decls, group.as_deref());
                }
            });
            outer_class_names.push(class);
        }
    }

    let Some(mut editor) = TagEditor::first(&content) else { return content };
    if !supports_layout && !outer_class_names.is_empty() {
        for class in &outer_class_names {
            editor.add_class(class);
        }
        return editor.finish();
    } else if !supports_layout {
        return content;
    }

    // Layout of this block.
    let fallback_layout = bt
        .support(&["layout", "default"])
        .or_else(|| bt.support(&["__experimentalLayout", "default"]))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut used_layout: Map<String, Value> = match attrs.get("layout") {
        Some(Value::Object(o)) => o.clone(),
        _ => fallback_layout,
    };
    if used_layout.get("inherit").is_some_and(|v| !crate::style_engine::is_empty(v))
        || used_layout.get("contentSize").is_some_and(|v| !crate::style_engine::is_empty(v))
    {
        used_layout.insert("type".into(), Value::from("constrained"));
    }

    let settings = env.theme.global_settings();
    let mut class_names: Vec<String> = Vec::new();
    let root_padding_aware = settings.get("useRootPaddingAwareAlignments").and_then(Value::as_bool).unwrap_or(false);
    if root_padding_aware && used_layout.get("type").and_then(Value::as_str) == Some("constrained") {
        class_names.push("has-global-padding".into());
    }
    let attr_layout = attrs.get("layout").and_then(Value::as_object);
    let sanitize = |s: &str| rustpress_core::slug::sanitize_title(s);
    if let Some(o) = attr_layout.and_then(|l| l.get("orientation")).and_then(Value::as_str).filter(|s| !s.is_empty()) {
        class_names.push(format!("is-{}", sanitize(o)));
    }
    if let Some(j) = attr_layout.and_then(|l| l.get("justifyContent")).and_then(Value::as_str).filter(|s| !s.is_empty()) {
        class_names.push(format!("is-content-justification-{}", sanitize(j)));
    }
    if attr_layout.and_then(|l| l.get("flexWrap")).and_then(Value::as_str) == Some("nowrap") {
        class_names.push("is-nowrap".into());
    }
    let layout_classname = used_layout
        .get("type")
        .and_then(Value::as_str)
        .map_or(layout_class_name("default"), layout_class_name)
        .unwrap_or("");
    if !layout_classname.is_empty() {
        class_names.push(sanitize(layout_classname));
    }

    // Container class and CSS.
    let gap_value = sanitize_block_gap_value(style_attr.and_then(|s| s.get("spacing")).and_then(|s| s.get("blockGap")));
    let mut fallback_gap = bt.support(&["spacing", "blockGap", "__experimentalDefault"]).cloned().unwrap_or(Value::from("0.5em"));
    let block_spacing = style_attr.and_then(|s| s.get("spacing")).filter(|v| !v.is_null());
    let skip_gap = bt.skips_serialization("spacing", Some("blockGap"));
    let has_block_gap_support = settings.get("spacing").and_then(|s| s.get("blockGap")).is_some_and(|v| !v.is_null());

    let styles = env.theme.global_styles();
    let block_styles = styles.get("blocks").and_then(|b| b.get(&bt.name));
    let gap_of = |of: Option<&Value>| of.and_then(|o| o.get("spacing")).and_then(|s| s.get("blockGap")).filter(|v| !v.is_null()).cloned();
    // The style variation of the block first (`wp_get_block_style_variation_name_from_registered_style()`).
    let variation_gap = attrs.get("className").and_then(Value::as_str).filter(|c| c.contains("is-style-")).and_then(|class_name| {
        let variations = block_styles?.get("variations")?.as_object()?;
        let name = class_name.split(' ').filter_map(|c| c.strip_prefix("is-style-")).find(|name| *name != "default" && variations.contains_key(*name))?;
        gap_of(variations.get(name))
    });
    if let Some(gap) = variation_gap.or_else(|| gap_of(block_styles)).or_else(|| gap_of(Some(styles))) {
        fallback_gap = gap;
    }

    let mut hash_list = vec![
        Value::Object(used_layout.clone()),
        Value::Bool(has_block_gap_support),
        gap_value.clone().unwrap_or(Value::Null),
        Value::Bool(skip_gap),
        fallback_gap.clone(),
        block_spacing.cloned().unwrap_or(Value::Null),
    ];
    let container_values = |viewport: &Value| -> Map<String, Value> { viewport.get("layout").and_then(Value::as_object).map(|l| l.iter().filter(|(k, _)| !CHILD_KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default() };
    let viewport_gap = |viewport: &Value| viewport.get("spacing").and_then(|s| s.get("blockGap")).filter(|v| !v.is_null()).cloned();
    let viewports: Vec<(&str, &str, &Value)> = queries.iter().filter_map(|(breakpoint, query)| Some((*breakpoint, query.as_str(), style_attr?.get(*breakpoint).filter(|v| v.is_object())?))).collect();
    for (breakpoint, _, viewport) in &viewports {
        let layout = container_values(viewport);
        if !layout.is_empty() {
            hash_list.push(json!({ "breakpoint": breakpoint, "layout": layout }));
        }
        if let Some(gap) = viewport_gap(viewport) {
            hash_list.push(json!({ "breakpoint": breakpoint, "blockGap": sanitize_block_gap_value(Some(&gap)) }));
        }
    }
    let block_name = sanitize(&attrs_block_name(bt));
    let container_class = unique_id_from_values(&Value::Array(hash_list), &format!("wp-container-{block_name}-is-layout-"));
    let selector = format!(".{container_class}");
    let has_style = env.with_state(|s| get_layout_style(s, &selector, &used_layout, has_block_gap_support, gap_value.as_ref(), skip_gap, &fallback_gap, block_spacing, &LayoutOptions::default()));
    let mut viewport_style = false;
    for (_, query, viewport) in &viewports {
        let layout = container_values(viewport);
        let own_gap = viewport_gap(viewport);
        if layout.is_empty() && own_gap.is_none() {
            continue;
        }
        let gap = match &own_gap {
            Some(gap) => sanitize_block_gap_value(Some(gap)),
            None => gap_value.clone(),
        };
        let options = LayoutOptions { rules_group: Some(query), viewport_overrides: Some(&layout), has_block_gap_override: own_gap.is_some() };
        viewport_style |= env.with_state(|s| get_layout_style(s, &selector, &used_layout, has_block_gap_support, gap.as_ref(), skip_gap, &fallback_gap, block_spacing, &options));
    }
    if viewport_style || has_style {
        class_names.push(container_class);
    }

    // `wp-block-group-is-layout-flow`
    let short_name = match bt.name.split_once('/') {
        Some(("core", rest)) => rest.to_owned(),
        _ => bt.name.replace('/', "-"),
    };
    class_names.push(format!("wp-block-{short_name}-{layout_classname}"));

    for class in &outer_class_names {
        editor.add_class(class);
    }

    // For blocks with inner blocks the classes belong on the element that wraps them.
    let wrapper_class = if has_inner_blocks { inner_wrapper_class(first_inner_chunk) } else { None };
    let mut target = editor;
    if let Some(wrapper) = wrapper_class {
        let html = target.tag().clone();
        let current = target.current_html();
        let mut from = html.start;
        let mut found = None;
        while let Some(tag) = crate::html::first_start_tag(&current, from) {
            if tag.attr("class").is_some_and(|c| c.contains(&wrapper)) {
                found = Some(tag);
                break;
            }
            from = tag.end;
        }
        if let Some(tag) = found {
            target = TagEditor::from_tag(&current, tag);
        }
    }
    for class in &class_names {
        target.add_class(class);
    }
    target.finish()
}

fn attrs_block_name(bt: &BlockType) -> String {
    bt.name.clone()
}

/// The class attribute of the last still-open element in the first HTML
/// chunk before the inner blocks.
fn inner_wrapper_class(first_chunk: Option<&str>) -> Option<String> {
    let chunk = first_chunk?;
    let mut stack: Vec<Option<String>> = Vec::new();
    for tag in crate::html::tags(chunk, 0) {
        if tag.is_closer {
            stack.pop();
        } else if !crate::html::is_void(&tag.name) {
            stack.push(tag.attr("class").map(str::to_owned));
        }
    }
    stack.into_iter().rev().flatten().find(|c| !c.is_empty())
}

/// `wp_restore_group_inner_container()` on `render_block_core/group`: themes without a `theme.json` get the
/// `wp-block-group__inner-container` wrapper back, with the layout classes moved onto it.
pub fn restore_group_inner_container(attrs: &serde_json::Map<String, Value>, content: String) -> String {
    use regex::bytes::Regex;
    let tag_name = attrs.get("tagName").and_then(Value::as_str).unwrap_or("div");
    let quoted = regex::escape(tag_name);
    // PCRE's `\s` without `/u`.
    const S: &str = r"[ \t\n\x0B\x0C\r]";
    let Ok(existing) = Regex::new(&format!(r#"(?-u)^{S}*<{quoted}\b[^>]*wp-block-group({S}|")[^>]*>{S}*<div\b[^>]*wp-block-group__inner-container({S}|")[^>]*>"#)) else { return content };
    let layout = attrs.get("layout").and_then(|l| l.get("type")).and_then(Value::as_str);
    if existing.is_match(content.as_bytes()) || matches!(layout, Some("flex" | "grid")) {
        return content;
    }
    let mut layout_classes: Vec<String> = Vec::new();
    let mut without = content.clone();
    if let Some(tag) = crate::html::tags(&content, 0).into_iter().find(|t| !t.is_closer && t.attr("class").is_some_and(|c| c.split_ascii_whitespace().any(|c| c == "wp-block-group"))) {
        let mut editor = crate::html::TagEditor::from_tag(&content, tag);
        let classes: Vec<String> = editor.get_attribute("class").unwrap_or_default().split_ascii_whitespace().map(str::to_owned).collect();
        for class in classes {
            if class.contains("is-layout-") && !layout_classes.contains(&class) {
                editor.remove_class(&class);
                layout_classes.push(class);
            }
        }
        without = editor.finish();
    }
    let Ok(replace) = Regex::new(&format!(r"(?ms-u)(^{S}*<{quoted}\b[^>]*wp-block-group[^>]*>)(.*)(</{quoted}>{S}*$)")) else { return content };
    let updated = replace.replace_all(without.as_bytes(), |caps: &regex::bytes::Captures<'_>| {
        let mut out = caps[1].to_vec();
        out.extend_from_slice(b"<div class=\"wp-block-group__inner-container\">");
        out.extend_from_slice(&caps[2]);
        out.extend_from_slice(b"</div>");
        out.extend_from_slice(&caps[3]);
        out
    });
    let updated = String::from_utf8_lossy(&updated).into_owned();
    if layout_classes.is_empty() {
        return updated;
    }
    let Some(tag) = crate::html::tags(&updated, 0).into_iter().find(|t| !t.is_closer && t.attr("class").is_some_and(|c| c.split_ascii_whitespace().any(|c| c == "wp-block-group__inner-container"))) else { return updated };
    let mut editor = crate::html::TagEditor::from_tag(&updated, tag);
    for class in &layout_classes {
        editor.add_class(class);
    }
    editor.finish()
}

/// `wp_restore_image_outer_container()` on `render_block_core/image`: themes without a `theme.json` get aligned
/// images wrapped in a `<div class="wp-block-image">` again.
pub fn restore_image_outer_container(attrs: &serde_json::Map<String, Value>, content: String) -> String {
    let Some(tag) = crate::html::tags(&content, 0).into_iter().find(|t| !t.is_closer && t.name == "figure") else { return content };
    let mut figure = crate::html::TagEditor::from_tag(&content, tag);
    if !figure.has_class("wp-block-image") || !["alignleft", "aligncenter", "alignright"].iter().any(|c| figure.has_class(c)) {
        return content;
    }
    let class = match attrs.get("className").and_then(Value::as_str) {
        Some(extra) => format!("wp-block-image {extra}"),
        None => "wp-block-image".to_owned(),
    };
    figure.remove_class("wp-block-image");
    let mut seen: Vec<&str> = Vec::new();
    for name in class.split_ascii_whitespace() {
        if !seen.contains(&name) {
            seen.push(name);
            figure.remove_class(name);
        }
    }
    format!("<div class=\"{}\">{}</div>", crate::html::esc_attr(&class), figure.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules_of(state: &State) -> String {
        state.block_supports_css.compile()
    }

    #[test]
    fn flex_layout_rules() {
        let mut state = State::default();
        let layout = json!({"type": "flex", "flexWrap": "nowrap", "justifyContent": "space-between", "verticalAlignment": "center"});
        let any = get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), None, &LayoutOptions::default());
        assert!(any);
        assert_eq!(rules_of(&state), ".c{flex-wrap:nowrap;justify-content:space-between;align-items:center;}");
    }

    #[test]
    fn flex_gap_uses_both_sides() {
        let mut state = State::default();
        let layout = json!({"type": "flex"});
        let gap = json!({"top": "var:preset|spacing|50", "left": "10px"});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), true, Some(&gap), false, &json!("0.5em"), None, &LayoutOptions::default());
        assert_eq!(rules_of(&state), ".c{gap:var(--wp--preset--spacing--50) 10px;}");
    }

    #[test]
    fn vertical_flex() {
        let mut state = State::default();
        let layout = json!({"type": "flex", "orientation": "vertical"});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), None, &LayoutOptions::default());
        assert_eq!(rules_of(&state), ".c{flex-direction:column;align-items:flex-start;}");
    }

    #[test]
    fn constrained_with_sizes_and_padding() {
        let mut state = State::default();
        let layout = json!({"type": "constrained", "contentSize": "600px", "wideSize": "1000px"});
        let spacing = json!({"padding": {"right": "10px"}});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), Some(&spacing), &LayoutOptions::default());
        assert_eq!(
            rules_of(&state),
            ".c > :where(:not(.alignleft):not(.alignright):not(.alignfull)){max-width:600px;margin-left:auto !important;margin-right:auto !important;}.c > .alignwide{max-width:1000px;}.c .alignfull{max-width:none;}.c > .alignfull{margin-right:calc(10px * -1);}"
        );
    }

    #[test]
    fn default_layout_without_block_gap_has_no_rules() {
        let mut state = State::default();
        let layout = json!({"type": "default"});
        assert!(!get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), None, &LayoutOptions::default()));
        assert!(state.block_supports_css.is_empty());
    }

    #[test]
    fn block_gap_in_default_and_constrained_layouts() {
        let mut state = State::default();
        let layout = json!({"type": "default"});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), true, Some(&json!("2rem")), false, &json!("0.5em"), None, &LayoutOptions::default());
        assert_eq!(rules_of(&state), ".c > *{margin-block-start:0;margin-block-end:0;}.c > * + *{margin-block-start:2rem;margin-block-end:0;}");
    }

    #[test]
    fn grid_with_minimum_column_width() {
        let mut state = State::default();
        let layout = json!({"type": "grid", "minimumColumnWidth": "20rem"});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), None, &LayoutOptions::default());
        assert_eq!(rules_of(&state), ".c{grid-template-columns:repeat(auto-fill, minmax(min(20rem, 100%), 1fr));container-type:inline-size;}");
    }

    #[test]
    fn gap_values_with_code_are_rejected() {
        assert_eq!(sanitize_block_gap_value(Some(&json!("1rem"))), Some(json!("1rem")));
        assert_eq!(sanitize_block_gap_value(Some(&json!("calc(1px)"))), None);
        assert_eq!(sanitize_block_gap_value(Some(&json!({"top": "1px", "left": "a}b"}))), Some(json!({"top": "1px", "left": null})));
    }

    #[test]
    fn child_layout_rules() {
        let child = json!({"selfStretch": "fixed", "flexSize": "50%", "columnSpan": 2});
        let rules = super::child_layout_rules(".k", child.as_object().unwrap(), &Map::new(), None);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[1].2.as_deref(), Some("@container (max-width: 25.5rem )"));
        assert_eq!(rules[0].1, d(&[("flex-basis", "50%"), ("box-sizing", "border-box"), ("grid-column", "span 2")]));
    }

    #[test]
    fn wrapper_is_the_last_open_element_with_a_class() {
        assert_eq!(inner_wrapper_class(Some("\n<div class=\"wp-block-group\">\n<p class=\"x\">a</p>")), Some("wp-block-group".into()));
        assert_eq!(inner_wrapper_class(Some("<div class=\"a\"><div class=\"b\">")), Some("b".into()));
        assert_eq!(inner_wrapper_class(Some("<div>")), None);
        assert_eq!(inner_wrapper_class(None), None);
    }
}
