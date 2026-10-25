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

/// `wp_get_layout_style` (without viewport overrides): stores the rules and
/// reports whether there are any.
#[allow(clippy::too_many_arguments)]
pub fn get_layout_style(
    state: &mut State,
    selector: &str,
    layout: &Map<String, Value>,
    has_block_gap_support: bool,
    gap_value: Option<&Value>,
    skip_gap_serialization: bool,
    fallback_gap: &Value,
    block_spacing: Option<&Value>,
) -> bool {
    let layout_type = layout.get("type").and_then(Value::as_str).unwrap_or("default");
    let mut rules: Vec<(String, Decls)> = Vec::new();
    let lget = |k: &str| layout.get(k).filter(|v| !v.is_null());
    let lstr = |k: &str| lget(k).and_then(Value::as_str).unwrap_or("");

    let zero_gap_rules = |rules: &mut Vec<(String, Decls)>, gap: &Value| {
        let gap = spacing_preset_to_var(&value_to_string(gap));
        rules.push((format!("{selector} > *"), d(&[("margin-block-start", "0"), ("margin-block-end", "0")])));
        rules.push((format!("{selector} > * + *"), vec![("margin-block-start".into(), gap), ("margin-block-end".into(), "0".into())]));
    };

    match layout_type {
        "default" => {
            if has_block_gap_support {
                let gap = gap_value.map(|g| if let Value::Object(m) = g { m.get("top").cloned().unwrap_or(Value::Null) } else { g.clone() });
                if let Some(gap) = gap.filter(|g| !g.is_null())
                    && !skip_gap_serialization {
                        zero_gap_rules(&mut rules, &gap);
                    }
            }
        }
        "constrained" => {
            let content_size = lstr("contentSize");
            let wide_size = lstr("wideSize");
            let justify = lget("justifyContent").and_then(Value::as_str).unwrap_or("center");
            let all_max = if !content_size.is_empty() {
                content_size.to_owned()
            } else if !wide_size.is_empty() {
                wide_size.to_owned()
            } else {
                "var(--wp--style--global--content-size, none)".to_owned()
            };
            let wide_max = if !wide_size.is_empty() {
                wide_size.to_owned()
            } else if !content_size.is_empty() {
                content_size.to_owned()
            } else {
                "var(--wp--style--global--wide-size, none)".to_owned()
            };
            let first_rule = |s: &str| s.split(';').next().unwrap_or("").trim().to_owned();
            let margin_left = if justify == "left" { "0 !important" } else { "auto !important" };
            let margin_right = if justify == "right" { "0 !important" } else { "auto !important" };
            if !content_size.is_empty() || !wide_size.is_empty() {
                rules.push((
                    format!("{selector} > :where(:not(.alignleft):not(.alignright):not(.alignfull))"),
                    vec![
                        ("max-width".into(), first_rule(&all_max)),
                        ("margin-left".into(), margin_left.into()),
                        ("margin-right".into(), margin_right.into()),
                    ],
                ));
                rules.push((format!("{selector} > .alignwide"), vec![("max-width".into(), first_rule(&wide_max))]));
                rules.push((format!("{selector} .alignfull"), d(&[("max-width", "none")])));
            }
            if let Some(spacing) = block_spacing {
                let styles = get_styles(&json!({ "spacing": spacing }), false);
                for (side, property) in [("padding-right", "margin-right"), ("padding-left", "margin-left")] {
                    if let Some(padding) = styles.declaration(side) {
                        let padding = if padding == "0" { "0px" } else { padding };
                        rules.push((format!("{selector} > .alignfull"), vec![(property.into(), format!("calc({padding} * -1)"))]));
                    }
                }
            }
            if justify == "left" {
                rules.push((format!("{selector} > :where(:not(.alignleft):not(.alignright):not(.alignfull))"), d(&[("margin-left", "0 !important")])));
            }
            if justify == "right" {
                rules.push((format!("{selector} > :where(:not(.alignleft):not(.alignright):not(.alignfull))"), d(&[("margin-right", "0 !important")])));
            }
            if has_block_gap_support {
                let gap = gap_value.map(|g| if let Value::Object(m) = g { m.get("top").cloned().unwrap_or(Value::Null) } else { g.clone() });
                if let Some(gap) = gap.filter(|g| !g.is_null())
                    && !skip_gap_serialization {
                        zero_gap_rules(&mut rules, &gap);
                    }
            }
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

            if lstr("flexWrap") == "nowrap" {
                rules.push((selector.to_owned(), d(&[("flex-wrap", "nowrap")])));
            }
            if has_block_gap_support
                && let Some(gap) = gap_value.filter(|g| !g.is_null())
                    && !skip_gap_serialization {
                        rules.push((selector.to_owned(), vec![("gap".into(), combined_gap(gap, fallback_gap))]));
                    }
            let justify = lget("justifyContent").and_then(Value::as_str).unwrap_or("");
            let vertical = lget("verticalAlignment").and_then(Value::as_str).unwrap_or("");
            if horizontal {
                if let Some(v) = option(justify_options, justify) {
                    rules.push((selector.to_owned(), d(&[("justify-content", v)])));
                }
                if let Some(v) = option(vertical_options, vertical) {
                    rules.push((selector.to_owned(), d(&[("align-items", v)])));
                }
            } else {
                rules.push((selector.to_owned(), d(&[("flex-direction", "column")])));
                match option(justify_options, justify) {
                    Some(v) if !justify.is_empty() => rules.push((selector.to_owned(), d(&[("align-items", v)]))),
                    _ => rules.push((selector.to_owned(), d(&[("align-items", "flex-start")]))),
                }
                if let Some(v) = option(vertical_options, vertical) {
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
                && let Some(gap) = gap_value.filter(|g| !g.is_null()) {
                    let combined = combined_gap(gap, fallback_gap);
                    responsive_gap = combined.clone();
                    gap_string = Some(combined);
                }
            if responsive_gap == "0" {
                responsive_gap = "0px".into();
            }
            let column_count = lget("columnCount").map(value_to_string).filter(|c| !c.is_empty() && c != "0");
            let min_width = lget("minimumColumnWidth").map(value_to_string).filter(|c| !c.is_empty());
            let auto_placement = if lget("autoFit").is_some_and(|v| !crate::style_engine::is_empty(v)) { "auto-fit" } else { "auto-fill" };
            let mut grid: Decls = Vec::new();
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
            if column_count.is_none() || min_width.is_some() {
                grid.push(("container-type".into(), "inline-size".into()));
            }
            rules.push((selector.to_owned(), grid));
            if let (Some(count), Some(rows)) = (&column_count, lget("rowCount").map(value_to_string).filter(|r| !r.is_empty())) {
                let _ = count;
                rules.push((selector.to_owned(), vec![("grid-template-rows".into(), format!("repeat({rows}, minmax(1rem, auto))"))]));
            }
            if let Some(gap) = gap_string.filter(|_| !skip_gap_serialization) {
                rules.push((selector.to_owned(), vec![("gap".into(), gap)]));
            }
        }
        _ => {}
    }

    let any = !rules.is_empty();
    for (sel, decls) in rules {
        state.block_supports_css.add(&sel, &decls, None);
    }
    any
}

/// `wp_get_child_layout_style_rules()`: child layout rules (`selfStretch`, grid placement) and, for a child of a
/// responsive grid, the container query that resets the placement. The third element is the `@container` group.
fn child_layout_rules(selector: &str, child: &Map<String, Value>, parent: &Map<String, Value>) -> Vec<(String, Decls, Option<String>)> {
    let mut decls: Decls = Vec::new();
    let self_stretch = child.get("selfStretch").and_then(Value::as_str);
    let flex_size = child.get("flexSize").filter(|v| !v.is_null());
    match (self_stretch, flex_size) {
        (Some(s @ ("fixed" | "fixedNoShrink")), Some(size)) => {
            decls.push(("flex-basis".into(), value_to_string(size)));
            if s == "fixedNoShrink" {
                decls.push(("flex-shrink".into(), "0".into()));
            }
            decls.push(("box-sizing".into(), "border-box".into()));
        }
        (Some("fill"), _) => decls.push(("flex-grow".into(), "1".into())),
        _ => {}
    }
    let place = |start: Option<String>, span: Option<String>| match (start, span) {
        (Some(s), Some(n)) => Some(format!("{s} / span {n}")),
        (Some(s), None) => Some(s),
        (None, Some(n)) => Some(format!("span {n}")),
        _ => None,
    };
    let get = |k: &str| child.get(k).filter(|v| !crate::style_engine::is_empty(v)).map(value_to_string);
    if let Some(v) = place(get("columnStart"), get("columnSpan")) {
        decls.push(("grid-column".into(), v));
    }
    if let Some(v) = place(get("rowStart"), get("rowSpan")) {
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
    if (column_start.is_some() || column_span.is_some()) && (minimum.is_some() || !has_count) {
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
    let child_layout = style_attr.and_then(|s| s.get("layout")).and_then(Value::as_object).filter(|l| !l.is_empty());
    if !supports_layout && child_layout.is_none() {
        return content;
    }

    let mut outer_class_names: Vec<String> = Vec::new();
    if let Some(child) = child_layout {
        let base: Map<String, Value> = child.iter().filter(|(k, _)| CHILD_KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        let parent_subset: Map<String, Value> = parent_layout
            .and_then(Value::as_object)
            .map(|p| p.iter().filter(|(k, _)| matches!(k.as_str(), "minimumColumnWidth" | "columnCount")).map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let class = unique_id_from_values(&json!({ "layout": base, "parentLayout": parent_subset }), "wp-container-content-");
        let rules = child_layout_rules(&format!(".{class}"), &base, &parent_subset);
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
    let global_block_gap = styles
        .get("blocks")
        .and_then(|b| b.get(&bt.name))
        .and_then(|b| b.get("spacing"))
        .and_then(|s| s.get("blockGap"))
        .filter(|v| !v.is_null())
        .or_else(|| styles.get("spacing").and_then(|s| s.get("blockGap")).filter(|v| !v.is_null()));
    if let Some(g) = global_block_gap {
        fallback_gap = g.clone();
    }

    let hash_input = json!([
        Value::Object(used_layout.clone()),
        has_block_gap_support,
        gap_value.clone().unwrap_or(Value::Null),
        skip_gap,
        fallback_gap.clone(),
        block_spacing.cloned().unwrap_or(Value::Null),
    ]);
    let block_name = sanitize(&attrs_block_name(bt));
    let container_class = unique_id_from_values(&hash_input, &format!("wp-container-{block_name}-is-layout-"));
    let has_style = env.with_state(|s| {
        get_layout_style(
            s,
            &format!(".{container_class}"),
            &used_layout,
            has_block_gap_support,
            gap_value.as_ref(),
            skip_gap,
            &fallback_gap,
            block_spacing,
        )
    });
    if has_style {
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
        let any = get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), None);
        assert!(any);
        assert_eq!(rules_of(&state), ".c{flex-wrap:nowrap;justify-content:space-between;align-items:center;}");
    }

    #[test]
    fn flex_gap_uses_both_sides() {
        let mut state = State::default();
        let layout = json!({"type": "flex"});
        let gap = json!({"top": "var:preset|spacing|50", "left": "10px"});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), true, Some(&gap), false, &json!("0.5em"), None);
        assert_eq!(rules_of(&state), ".c{gap:var(--wp--preset--spacing--50) 10px;}");
    }

    #[test]
    fn vertical_flex() {
        let mut state = State::default();
        let layout = json!({"type": "flex", "orientation": "vertical"});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), None);
        assert_eq!(rules_of(&state), ".c{flex-direction:column;align-items:flex-start;}");
    }

    #[test]
    fn constrained_with_sizes_and_padding() {
        let mut state = State::default();
        let layout = json!({"type": "constrained", "contentSize": "600px", "wideSize": "1000px"});
        let spacing = json!({"padding": {"right": "10px"}});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), Some(&spacing));
        assert_eq!(
            rules_of(&state),
            ".c > :where(:not(.alignleft):not(.alignright):not(.alignfull)){max-width:600px;margin-left:auto !important;margin-right:auto !important;}.c > .alignwide{max-width:1000px;}.c .alignfull{max-width:none;}.c > .alignfull{margin-right:calc(10px * -1);}"
        );
    }

    #[test]
    fn default_layout_without_block_gap_has_no_rules() {
        let mut state = State::default();
        let layout = json!({"type": "default"});
        assert!(!get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), None));
        assert!(state.block_supports_css.is_empty());
    }

    #[test]
    fn block_gap_in_default_and_constrained_layouts() {
        let mut state = State::default();
        let layout = json!({"type": "default"});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), true, Some(&json!("2rem")), false, &json!("0.5em"), None);
        assert_eq!(rules_of(&state), ".c > *{margin-block-start:0;margin-block-end:0;}.c > * + *{margin-block-start:2rem;margin-block-end:0;}");
    }

    #[test]
    fn grid_with_minimum_column_width() {
        let mut state = State::default();
        let layout = json!({"type": "grid", "minimumColumnWidth": "20rem"});
        get_layout_style(&mut state, ".c", layout.as_object().unwrap(), false, None, false, &json!("0.5em"), None);
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
        let rules = super::child_layout_rules(".k", child.as_object().unwrap(), &Map::new());
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
