//! Global styles: turns the merged `theme.json` into the stylesheet WordPress
//! prints as `global-styles-inline-css` (a port of the relevant parts of
//! `WP_Theme_JSON::get_stylesheet`).

use rustpress_blocks::Registry;
use rustpress_blocks::fluid::fluid_font_size;
use rustpress_blocks::style_engine::to_kebab_case;
use serde_json::{Map, Value};

use crate::theme_json::ThemeJson;

/// `(css property, path in a styles node)` in the order WordPress emits them.
const PROPERTIES: &[(&str, &[&str])] = &[
    ("aspect-ratio", &["dimensions", "aspectRatio"]),
    ("background", &["color", "gradient"]),
    ("background-color", &["color", "background"]),
    ("background-image", &["background", "backgroundImage"]),
    ("background-position", &["background", "backgroundPosition"]),
    ("background-repeat", &["background", "backgroundRepeat"]),
    ("background-size", &["background", "backgroundSize"]),
    ("background-attachment", &["background", "backgroundAttachment"]),
    ("border-radius", &["border", "radius"]),
    ("border-top-left-radius", &["border", "radius", "topLeft"]),
    ("border-top-right-radius", &["border", "radius", "topRight"]),
    ("border-bottom-left-radius", &["border", "radius", "bottomLeft"]),
    ("border-bottom-right-radius", &["border", "radius", "bottomRight"]),
    ("border-color", &["border", "color"]),
    ("border-width", &["border", "width"]),
    ("border-style", &["border", "style"]),
    ("border-top-color", &["border", "top", "color"]),
    ("border-top-width", &["border", "top", "width"]),
    ("border-top-style", &["border", "top", "style"]),
    ("border-right-color", &["border", "right", "color"]),
    ("border-right-width", &["border", "right", "width"]),
    ("border-right-style", &["border", "right", "style"]),
    ("border-bottom-color", &["border", "bottom", "color"]),
    ("border-bottom-width", &["border", "bottom", "width"]),
    ("border-bottom-style", &["border", "bottom", "style"]),
    ("border-left-color", &["border", "left", "color"]),
    ("border-left-width", &["border", "left", "width"]),
    ("border-left-style", &["border", "left", "style"]),
    ("color", &["color", "text"]),
    ("text-align", &["typography", "textAlign"]),
    ("column-count", &["typography", "textColumns"]),
    ("font-family", &["typography", "fontFamily"]),
    ("font-size", &["typography", "fontSize"]),
    ("font-style", &["typography", "fontStyle"]),
    ("font-weight", &["typography", "fontWeight"]),
    ("letter-spacing", &["typography", "letterSpacing"]),
    ("line-height", &["typography", "lineHeight"]),
    ("margin", &["spacing", "margin"]),
    ("margin-top", &["spacing", "margin", "top"]),
    ("margin-right", &["spacing", "margin", "right"]),
    ("margin-bottom", &["spacing", "margin", "bottom"]),
    ("margin-left", &["spacing", "margin", "left"]),
    ("min-height", &["dimensions", "minHeight"]),
    ("min-width", &["dimensions", "minWidth"]),
    ("outline-color", &["outline", "color"]),
    ("outline-offset", &["outline", "offset"]),
    ("outline-style", &["outline", "style"]),
    ("outline-width", &["outline", "width"]),
    ("padding", &["spacing", "padding"]),
    ("padding-top", &["spacing", "padding", "top"]),
    ("padding-right", &["spacing", "padding", "right"]),
    ("padding-bottom", &["spacing", "padding", "bottom"]),
    ("padding-left", &["spacing", "padding", "left"]),
    ("text-decoration", &["typography", "textDecoration"]),
    ("text-indent", &["typography", "textIndent"]),
    ("text-transform", &["typography", "textTransform"]),
    ("filter", &["filter", "duotone"]),
    ("box-shadow", &["shadow"]),
    ("writing-mode", &["typography", "writingMode"]),
    ("width", &["dimensions", "width"]),
    ("height", &["dimensions", "height"]),
];

struct Preset {
    path: &'static [&'static str],
    value_key: &'static str,
    var: &'static str,
    classes: &'static [(&'static str, &'static str)],
}

const PRESETS: &[Preset] = &[
    Preset { path: &["dimensions", "aspectRatios"], value_key: "ratio", var: "aspect-ratio", classes: &[] },
    Preset {
        path: &["color", "palette"],
        value_key: "color",
        var: "color",
        classes: &[("color", "color"), ("background-color", "background-color"), ("border-color", "border-color")],
    },
    Preset { path: &["color", "gradients"], value_key: "gradient", var: "gradient", classes: &[("gradient-background", "background")] },
    Preset { path: &["typography", "fontSizes"], value_key: "size", var: "font-size", classes: &[("font-size", "font-size")] },
    Preset { path: &["typography", "fontFamilies"], value_key: "fontFamily", var: "font-family", classes: &[("font-family", "font-family")] },
    Preset { path: &["spacing", "spacingSizes"], value_key: "size", var: "spacing", classes: &[] },
    Preset { path: &["shadow", "presets"], value_key: "shadow", var: "shadow", classes: &[] },
];

const BLOCK_PRESETS: &[Preset] = &[Preset { path: &["dimensions", "dimensionSizes"], value_key: "size", var: "dimension", classes: &[] }];

/// Element selectors (`WP_Theme_JSON::ELEMENTS`) in output order.
const ELEMENTS: &[(&str, &str)] = &[
    ("link", "a:where(:not(.wp-element-button))"),
    ("heading", "h1, h2, h3, h4, h5, h6"),
    ("h1", "h1"),
    ("h2", "h2"),
    ("h3", "h3"),
    ("h4", "h4"),
    ("h5", "h5"),
    ("h6", "h6"),
    ("button", ".wp-element-button, .wp-block-button__link"),
    (
        "caption",
        ".wp-element-caption, .wp-block-audio figcaption, .wp-block-embed figcaption, .wp-block-gallery figcaption, .wp-block-image figcaption, .wp-block-table figcaption, .wp-block-video figcaption",
    ),
    ("cite", "cite"),
];

const PSEUDO_ORDER: &[&str] = &[":link", ":any-link", ":visited", ":hover", ":focus", ":focus-visible", ":active"];

const LAYOUT_TYPES: &[(&str, &str)] = &[("default", "is-layout-flow"), ("constrained", "is-layout-constrained"), ("flex", "is-layout-flex"), ("grid", "is-layout-grid")];

pub struct GlobalStyles<'a> {
    json: &'a ThemeJson,
    registry: &'a Registry,
}

impl<'a> GlobalStyles<'a> {
    pub fn new(json: &'a ThemeJson, registry: &'a Registry) -> Self {
        Self { json, registry }
    }

    fn settings(&self) -> Value {
        self.json.settings()
    }

    /// `wp_get_global_stylesheet()`: variables, base layout rules, styles and preset classes.
    /// Block rules are limited to `used_blocks` when given (WordPress only
    /// prints rules for blocks that appear on the page).
    pub fn stylesheet(&self, used_blocks: Option<&[String]>) -> String {
        let handles: Option<Vec<String>> = used_blocks.map(|names| names.iter().map(|n| format!("wp-block-{}", n.trim_start_matches("core/"))).collect());
        self.stylesheet_chunks(handles.as_deref()).join("
")
    }

    /// The inline styles of the `global-styles` handle, in order: the main sheet, then one chunk
    /// per block or element node whose style handle is on the page
    /// (`wp_add_global_styles_for_blocks()`). Empty chunks are dropped; `print_inline_style()`
    /// joins the rest with newlines.
    pub fn stylesheet_chunks(&self, handles: Option<&[String]>) -> Vec<String> {
        let settings = self.settings();
        let styles = self.json.styles();
        let mut main = String::new();
        main.push_str(&self.variables(&settings));
        main.push_str(&self.block_variables(&settings));
        main.push_str(&self.root_rules(&settings, &styles));
        main.push_str(&self.root_styles(&settings, &styles));
        main.push_str(&self.preset_classes(&settings));
        // The theme's custom CSS comes last.
        if let Some(extra) = styles.get("css").and_then(Value::as_str) {
            main.push_str(extra);
        }
        let mut chunks = vec![main];
        chunks.extend(self.block_chunks(&settings, &styles, handles));
        chunks.retain(|c| !c.is_empty());
        chunks
    }

    /// `wp_get_global_stylesheet()` for a theme without `theme.json`: the variables, the layout
    /// rules WordPress needs without block gap support, the root styles of the core defaults and
    /// the preset classes, then the chunks of the blocks on the page.
    pub fn classic_chunks(&self, handles: Option<&[String]>) -> Vec<String> {
        let settings = self.settings();
        let styles = self.json.styles();
        let mut main = String::new();
        main.push_str(&self.variables(&settings));
        main.push_str(&self.block_variables(&settings));
        main.push_str(":where(body) { margin: 0; }");
        main.push_str(":where(.is-layout-flex){gap: 0.5em;}:where(.is-layout-grid){gap: 0.5em;}");
        main.push_str("body .is-layout-flex{display: flex;}.is-layout-flex{flex-wrap: wrap;align-items: center;}.is-layout-flex > :is(*, div){margin: 0;}");
        main.push_str("body .is-layout-grid{display: grid;}.is-layout-grid > :is(*, div){margin: 0;}");
        main.push_str(&self.root_styles(&settings, &styles));
        main.push_str(&self.preset_classes(&settings));
        let mut chunks = vec![main];
        chunks.extend(self.block_chunks(&settings, &styles, handles));
        // Blocks with a default block gap of their own (the gallery) get layout rules for it.
        for handle in handles.unwrap_or_default() {
            let Some(short) = handle.strip_prefix("wp-block-") else { continue };
            let Some(block) = self.registry.get(&format!("core/{short}")) else { continue };
            let gap = block.supports.get("spacing").and_then(|s| s.get("blockGap")).and_then(|g| g.get("__experimentalDefault")).and_then(Value::as_str);
            if let Some(gap) = gap {
                chunks.push(format!(":where(.wp-block-{short}.is-layout-flex){{gap: {gap};}}:where(.wp-block-{short}.is-layout-grid){{gap: {gap};}}"));
            }
        }
        chunks.retain(|c| !c.is_empty());
        chunks
    }

    // ----------------------------------------------------------- presets

    /// Preset items merged across origins by slug (core, theme, user).
    fn presets(&self, settings_path: &[&str], block: Option<&str>) -> Vec<(String, Value)> {
        let mut merged: Vec<(String, Value)> = Vec::new();
        for layer in [&self.json.core, &self.json.theme, &self.json.user] {
            let mut node = layer.get("settings");
            if let Some(block) = block {
                node = node.and_then(|s| s.get("blocks")).and_then(|b| b.get(block));
            }
            for key in settings_path {
                node = node.and_then(|n| n.get(*key));
            }
            let items = match node {
                Some(Value::Array(items)) => items.clone(),
                Some(Value::Object(map)) => {
                    // Origin-keyed form: {"theme": [...], "default": [...]}.
                    ["default", "theme", "custom"].iter().filter_map(|o| map.get(*o).and_then(Value::as_array)).flatten().cloned().collect()
                }
                _ => continue,
            };
            for item in items {
                let Some(slug) = item.get("slug").and_then(Value::as_str) else { continue };
                match merged.iter_mut().find(|(s, _)| s == slug) {
                    Some(slot) => slot.1 = item.clone(),
                    None => merged.push((slug.to_owned(), item.clone())),
                }
            }
        }
        merged
    }

    fn preset_value(&self, preset: &Preset, item: &Value, settings: &Value) -> Option<String> {
        let raw = item.get(preset.value_key)?;
        let text = match raw {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return None,
        };
        if preset.var == "font-size" {
            return Some(fluid_font_size(&text, item.get("fluid"), settings));
        }
        Some(text)
    }

    fn variables(&self, settings: &Value) -> String {
        let mut vars = String::new();
        for preset in PRESETS {
            for (slug, item) in self.presets(preset.path, None) {
                if let Some(value) = self.preset_value(preset, &item, settings) {
                    vars.push_str(&format!("--wp--preset--{}--{}: {};", preset.var, to_kebab_case(&slug), value));
                }
            }
        }
        if let Some(custom) = settings.get("custom").and_then(Value::as_object) {
            flatten_custom(custom, "--wp--custom", &mut vars);
        }
        if vars.is_empty() { String::new() } else { format!(":root{{{vars}}}") }
    }

    fn block_variables(&self, settings: &Value) -> String {
        let mut out = String::new();
        let Some(blocks) = self.json.merged.get("settings").and_then(|s| s.get("blocks")).and_then(Value::as_object) else { return out };
        for name in blocks.keys() {
            let mut vars = String::new();
            for preset in BLOCK_PRESETS {
                for (slug, item) in self.presets(preset.path, Some(name)) {
                    if let Some(value) = self.preset_value(preset, &item, settings) {
                        vars.push_str(&format!("--wp--preset--{}--{}: {};", preset.var, to_kebab_case(&slug), value));
                    }
                }
            }
            if !vars.is_empty() {
                out.push_str(&format!("{}{{{vars}}}", block_class(name)));
            }
        }
        out
    }

    fn preset_classes(&self, settings: &Value) -> String {
        let mut out = String::new();
        for preset in PRESETS {
            for (suffix, property) in preset.classes {
                for (slug, item) in self.presets(preset.path, None) {
                    if self.preset_value(preset, &item, settings).is_none() {
                        continue;
                    }
                    let slug = to_kebab_case(&slug);
                    out.push_str(&format!(".has-{slug}-{suffix}{{{property}: var(--wp--preset--{}--{slug}) !important;}}", preset.var));
                }
            }
        }
        out
    }

    // --------------------------------------------------------- root layout

    fn root_rules(&self, settings: &Value, styles: &Value) -> String {
        let mut css = String::new();
        let layout = settings.get("layout");
        let content = layout.and_then(|l| l.get("contentSize")).and_then(Value::as_str).filter(|s| !s.is_empty());
        let wide = layout.and_then(|l| l.get("wideSize")).and_then(Value::as_str).filter(|s| !s.is_empty());
        if content.is_some() || wide.is_some() {
            css.push_str(":root {");
            if let Some(c) = content {
                css.push_str(&format!(" --wp--style--global--content-size: {c};"));
            }
            if let Some(w) = wide {
                css.push_str(&format!("--wp--style--global--wide-size: {w};"));
            }
            css.push_str(" }");
        }
        css.push_str(":where(body) { margin: 0; }");

        let aware = settings.get("useRootPaddingAwareAlignments").and_then(Value::as_bool).unwrap_or(false);
        if aware {
            css.push_str(".wp-site-blocks { padding-top: var(--wp--style--root--padding-top); padding-bottom: var(--wp--style--root--padding-bottom); }");
            css.push_str(".has-global-padding { padding-right: var(--wp--style--root--padding-right); padding-left: var(--wp--style--root--padding-left); }");
            css.push_str(".has-global-padding > .alignfull { margin-right: calc(var(--wp--style--root--padding-right) * -1); margin-left: calc(var(--wp--style--root--padding-left) * -1); }");
            css.push_str(".has-global-padding :where(:not(.alignfull.is-layout-flow) > .has-global-padding:not(.wp-block-block, .alignfull)) { padding-right: 0; padding-left: 0; }");
            css.push_str(".has-global-padding :where(:not(.alignfull.is-layout-flow) > .has-global-padding:not(.wp-block-block, .alignfull)) > .alignfull { margin-left: 0; margin-right: 0; }");
        }
        css.push_str(".wp-site-blocks > .alignleft { float: left; margin-right: 2em; }");
        css.push_str(".wp-site-blocks > .alignright { float: right; margin-left: 2em; }");
        css.push_str(".wp-site-blocks > .aligncenter { justify-content: center; margin-left: auto; margin-right: auto; }");

        let has_gap_support = settings.get("spacing").and_then(|s| s.get("blockGap")).is_some_and(|v| !v.is_null());
        let gap = styles.get("spacing").and_then(|s| s.get("blockGap")).and_then(Value::as_str);
        if let (true, Some(gap)) = (has_gap_support, gap) {
            css.push_str(&format!(":where(.wp-site-blocks) > * {{ margin-block-start: {gap}; margin-block-end: 0; }}"));
            css.push_str(":where(.wp-site-blocks) > :first-child { margin-block-start: 0; }");
            css.push_str(":where(.wp-site-blocks) > :last-child { margin-block-end: 0; }");
            css.push_str(&format!(":root {{ --wp--style--block-gap: {gap}; }}"));
            for (kind, class) in LAYOUT_TYPES {
                css.push_str(&layout_spacing_rules(kind, &format!(".{class}"), gap));
            }
        }
        // Base styles of each layout type.
        css.push_str(&base_layout_styles());
        css
    }

    // -------------------------------------------------------------- styles

    fn root_styles(&self, settings: &Value, styles: &Value) -> String {
        let mut css = String::new();
        let aware = settings.get("useRootPaddingAwareAlignments").and_then(Value::as_bool).unwrap_or(false);

        // The root node (`body`) without elements/blocks.
        let mut declarations = self.declarations(styles, settings, aware);
        if aware {
            let padding = styles.get("spacing").and_then(|s| s.get("padding")).and_then(Value::as_object);
            let side = |name: &str| padding.and_then(|p| p.get(name)).and_then(Value::as_str).unwrap_or("0px").to_owned();
            for name in ["top", "right", "bottom", "left"] {
                declarations.push((format!("--wp--style--root--padding-{name}"), side(name)));
            }
        }
        if !declarations.is_empty() {
            css.push_str(&format!("body{{{}}}", ruleset(&declarations)));
        }

        // Root elements.
        if let Some(elements) = styles.get("elements").and_then(Value::as_object) {
            for (name, selector) in ELEMENTS {
                let Some(node) = elements.get(*name).filter(|n| n.is_object()) else { continue };
                css.push_str(&self.element_rules(node, selector, settings, name_is_plain(name), None));
            }
        }
        css
    }

    /// Rules for one element node: its own declarations, pseudo-classes and nested `css`.
    fn element_rules(&self, node: &Value, selector: &str, settings: &Value, plain: bool, scope: Option<&str>) -> String {
        let scoped = |sel: &str| match scope {
            Some(scope) => sel.split(',').map(|s| format!("{scope} {}", s.trim())).collect::<Vec<_>>().join(", "),
            None => sel.to_owned(),
        };
        let wrap = |sel: String| if plain && scope.is_none() { sel } else { format!(":root :where({sel})") };
        let mut css = String::new();
        let declarations = self.declarations(node, settings, false);
        if !declarations.is_empty() {
            css.push_str(&format!("{}{{{}}}", wrap(scoped(selector)), ruleset(&declarations)));
        }
        for pseudo in PSEUDO_ORDER {
            let Some(pseudo_node) = node.get(*pseudo) else { continue };
            let declarations = self.declarations(pseudo_node, settings, false);
            if declarations.is_empty() {
                continue;
            }
            let with_pseudo = scoped(selector).split(',').map(|s| format!("{}{pseudo}", s.trim())).collect::<Vec<_>>().join(", ");
            css.push_str(&format!(":root :where({with_pseudo}){{{}}}", ruleset(&declarations)));
        }
        if let Some(extra) = node.get("css").and_then(Value::as_str) {
            css.push_str(&process_custom_css(extra, &scoped(selector)));
        }
        css
    }

    fn block_selector(&self, name: &str) -> (String, Map<String, Value>) {
        let bt = self.registry.get(name);
        let selectors = bt.as_ref().and_then(|b| b.selectors.as_object().cloned()).unwrap_or_default();
        let root = selectors
            .get("root")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| bt.as_ref().and_then(|b| b.support(&["__experimentalSelector"])).and_then(Value::as_str).map(str::to_owned))
            .unwrap_or_else(|| block_class(name));
        (root, selectors)
    }

    /// The block nodes of `styles.blocks` as separate chunks: the block itself, then each element
    /// and each pseudo-class of an element (`get_styles_block_nodes()`).
    fn block_chunks(&self, settings: &Value, styles: &Value, handles: Option<&[String]>) -> Vec<String> {
        let mut chunks = Vec::new();
        let Some(blocks) = styles.get("blocks").and_then(Value::as_object) else { return chunks };
        for (name, node) in blocks {
            if !node.is_object() {
                continue;
            }
            let on_page = match (handles, name.strip_prefix("core/")) {
                (Some(handles), Some(short)) => handles.iter().any(|h| h.strip_prefix("wp-block-") == Some(short)),
                _ => true,
            };
            if !on_page {
                continue;
            }
            let (selector, _) = self.block_selector(name);
            chunks.push(self.block_rules(name, node, settings, None));
            let Some(elements) = node.get("elements").and_then(Value::as_object) else { continue };
            for (element, element_node) in elements {
                let Some((_, element_selector)) = ELEMENTS.iter().find(|(n, _)| n == element) else { continue };
                if !element_node.is_object() {
                    continue;
                }
                let combined = element_selector.split(',').map(|s| format!("{selector} {}", s.trim())).collect::<Vec<_>>().join(", ");
                chunks.extend(self.element_chunks(element_node, &combined, settings));
            }
        }
        chunks
    }

    /// An element node followed by its pseudo-class nodes, one chunk each.
    fn element_chunks(&self, node: &Value, selector: &str, settings: &Value) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut base = String::new();
        let declarations = self.declarations(node, settings, false);
        if !declarations.is_empty() {
            base.push_str(&format!(":root :where({selector}){{{}}}", ruleset(&declarations)));
        }
        if let Some(extra) = node.get("css").and_then(Value::as_str) {
            base.push_str(&process_custom_css(extra, selector));
        }
        chunks.push(base);
        for pseudo in PSEUDO_ORDER {
            let Some(pseudo_node) = node.get(*pseudo) else { continue };
            let declarations = self.declarations(pseudo_node, settings, false);
            if declarations.is_empty() {
                continue;
            }
            let with_pseudo = selector.split(',').map(|s| format!("{}{pseudo}", s.trim())).collect::<Vec<_>>().join(", ");
            chunks.push(format!(":root :where({with_pseudo}){{{}}}", ruleset(&declarations)));
        }
        chunks
    }

    /// The rules of the block itself: declarations by feature selector, block gap layouts and custom CSS.
    fn block_rules(&self, name: &str, node: &Value, settings: &Value, scope: Option<&str>) -> String {
        let mut css = String::new();
        let (root_selector, feature_selectors) = self.block_selector(name);
        let selector = scope.map(str::to_owned).unwrap_or(root_selector);

        // Declarations grouped by the selector of their feature.
        let declarations = self.declarations_with_features(node, settings);
        let mut groups: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for (feature, property, value) in declarations {
            let own = feature_selectors.get(&feature).and_then(|s| s.as_str().map(str::to_owned).or_else(|| s.get("root").and_then(Value::as_str).map(str::to_owned)));
            let target = match (scope, own) {
                // A variation's instance class goes right behind the first compound of each feature selector.
                (Some(scope), Some(own)) => variation_selector(scope, &own),
                (Some(scope), None) => scope.to_owned(),
                (None, own) => own.unwrap_or_else(|| selector.clone()),
            };
            match groups.iter_mut().find(|(s, _)| *s == target) {
                Some((_, list)) => list.push((property, value)),
                None => groups.push((target, vec![(property, value)])),
            }
        }
        for (target, declarations) in groups {
            css.push_str(&format!(":root :where({target}){{{}}}", ruleset(&declarations)));
        }

        // Block gap becomes layout rules for the block's layout classes.
        if let Some(gap) = node.get("spacing").and_then(|s| s.get("blockGap")).and_then(Value::as_str) {
            let class_base = block_class(name);
            let base = class_base.trim_start_matches('.');
            for (kind, class) in LAYOUT_TYPES {
                css.push_str(&layout_spacing_rules(kind, &format!(".{base}-{class}"), gap));
            }
        }
        if let Some(extra) = node.get("css").and_then(Value::as_str) {
            css.push_str(&process_custom_css(extra, &selector));
        }
        css
    }

    /// All rules for one block node of `styles.blocks`: the block and its elements. `scope` replaces the
    /// block selector (used for style variations).
    pub fn block_node(&self, name: &str, node: &Value, settings: &Value, scope: Option<&str>) -> String {
        let mut css = self.block_rules(name, node, settings, scope);
        let (root_selector, _) = self.block_selector(name);
        let selector = scope.map(str::to_owned).unwrap_or(root_selector);
        if let Some(elements) = node.get("elements").and_then(Value::as_object) {
            for (element, element_selector) in ELEMENTS {
                let Some(element_node) = elements.get(*element).filter(|n| n.is_object()) else { continue };
                let combined = element_selector.split(',').map(|s| format!("{selector} {}", s.trim())).collect::<Vec<_>>().join(", ");
                css.push_str(&self.element_rules(element_node, &combined, settings, false, None));
            }
        }
        css
    }

    /// Rules for a style variation: elements below `scope` plus block nodes.
    pub fn variation_stylesheet(&self, block_name: &str, variation: &Value, scope: &str) -> String {
        let settings = self.settings();
        let mut css = String::new();
        let mut data = variation.clone();
        let elements = data.as_object_mut().and_then(|o| o.remove("elements"));
        let blocks = data.as_object_mut().and_then(|o| o.remove("blocks"));
        if let Some(Value::Object(elements)) = elements {
            for (name, selector) in ELEMENTS {
                let Some(node) = elements.get(*name).filter(|n| n.is_object()) else { continue };
                css.push_str(&self.element_rules(node, selector, &settings, false, Some(scope)));
            }
        }
        // The variation's own declarations apply to the scoped block.
        css.push_str(&self.block_node(block_name, &data, &settings, Some(scope)));
        if let Some(Value::Object(blocks)) = blocks {
            for (name, node) in &blocks {
                let (selector, _) = self.block_selector(name);
                let scoped = format!("{scope} {selector}");
                css.push_str(&self.block_node(name, node, &settings, Some(&scoped)));
            }
        }
        css
    }

    // ------------------------------------------------------- declarations

    fn declarations(&self, node: &Value, settings: &Value, aware_root: bool) -> Vec<(String, String)> {
        self.declarations_with_features(node, settings)
            .into_iter()
            .filter(|(_, property, _)| !(aware_root && matches!(property.as_str(), "padding-top" | "padding-right" | "padding-bottom" | "padding-left")))
            .map(|(_, p, v)| (p, v))
            .collect()
    }

    /// `(feature, property, value)` for every property set on the node.
    fn declarations_with_features(&self, node: &Value, settings: &Value) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for (property, path) in PROPERTIES {
            let mut current = Some(node);
            for key in *path {
                current = current.and_then(|c| c.get(*key));
            }
            let Some(value) = current else { continue };
            let text = match value {
                Value::String(s) if !s.is_empty() => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => continue,
            };
            let text = if *property == "font-size" && !text.starts_with("var(") && !text.starts_with("calc(") && !text.starts_with("clamp(") {
                fluid_font_size(&text, None, settings)
            } else {
                text
            };
            let feature = path[0].to_owned();
            out.push((feature, (*property).to_owned(), text));
        }
        out
    }
}

fn name_is_plain(name: &str) -> bool {
    matches!(name, "link" | "heading" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

/// `.wp-block-post-title` for `core/post-title`.
fn block_class(name: &str) -> String {
    let short = match name.split_once('/') {
        Some(("core", rest)) => rest.to_owned(),
        _ => name.replace('/', "-"),
    };
    format!(".wp-block-{short}")
}

/// `WP_Theme_JSON::get_block_style_variation_selector()`: `class` is appended to the first
/// compound selector (up to a space or `:`) of each part of a selector list.
fn variation_selector(class: &str, selector: &str) -> String {
    selector
        .split(',')
        .map(|part| {
            let part = part.trim();
            let start = part.find(|c: char| !(c.is_whitespace() || c == ':')).unwrap_or(part.len());
            let end = part[start..].find(|c: char| c.is_whitespace() || c == ':').map_or(part.len(), |i| start + i);
            format!("{}{class}{}", &part[..end], &part[end..])
        })
        .collect::<Vec<_>>()
        .join(", ")
}
/// `prop: value;prop: value;`
fn ruleset(declarations: &[(String, String)]) -> String {
    declarations.iter().map(|(p, v)| format!("{p}: {v};")).collect()
}

fn flatten_custom(map: &Map<String, Value>, prefix: &str, out: &mut String) {
    for (key, value) in map {
        let name = format!("{prefix}--{}", to_kebab_case(key));
        match value {
            Value::Object(inner) => flatten_custom(inner, &name, out),
            Value::String(s) => out.push_str(&format!("{name}: {s};")),
            Value::Number(n) => out.push_str(&format!("{name}: {n};")),
            _ => {}
        }
    }
}

/// Spacing rules of one layout type below `selector` (e.g. `.is-layout-flow`).
fn layout_spacing_rules(kind: &str, selector: &str, gap: &str) -> String {
    let wrap = |extra: &str| format!(":root :where({selector}){extra}");
    match kind {
        "default" | "constrained" => format!(
            "{}{{margin-block-start: 0;}}{}{{margin-block-end: 0;}}{}{{margin-block-start: {gap};margin-block-end: 0;}}",
            wrap(" > :first-child"),
            wrap(" > :last-child"),
            wrap(" > *")
        ),
        _ => format!("{}{{gap: {gap};}}", wrap("")),
    }
}

fn base_layout_styles() -> String {
    let mut css = String::new();
    for class in ["is-layout-flow", "is-layout-constrained"] {
        css.push_str(&format!(".{class} > .alignleft{{float: left;margin-inline-start: 0;margin-inline-end: 2em;}}"));
        css.push_str(&format!(".{class} > .alignright{{float: right;margin-inline-start: 2em;margin-inline-end: 0;}}"));
        css.push_str(&format!(".{class} > .aligncenter{{margin-left: auto !important;margin-right: auto !important;}}"));
        if class == "is-layout-constrained" {
            css.push_str(".is-layout-constrained > :where(:not(.alignleft):not(.alignright):not(.alignfull)){max-width: var(--wp--style--global--content-size);margin-left: auto !important;margin-right: auto !important;}");
            css.push_str(".is-layout-constrained > .alignwide{max-width: var(--wp--style--global--wide-size);}");
        }
    }
    css.push_str("body .is-layout-flex{display: flex;}.is-layout-flex{flex-wrap: wrap;align-items: center;}.is-layout-flex > :is(*, div){margin: 0;}");
    css.push_str("body .is-layout-grid{display: grid;}.is-layout-grid > :is(*, div){margin: 0;}");
    css
}

/// `process_blocks_custom_css`: nested rules with `&` become scoped rules.
fn process_custom_css(css: &str, selector: &str) -> String {
    let mut out = String::new();
    for part in css.split('&') {
        if part.is_empty() || part == "0" {
            continue;
        }
        if !part.contains('{') {
            // Plain declarations apply to the block itself.
            out.push_str(&format!(":root :where({}){{{}}}", selector.trim(), part.trim()));
            continue;
        }
        let cleaned = part.replace('}', "");
        let pieces: Vec<&str> = cleaned.split('{').collect();
        let [nested, value] = pieces[..] else { continue };
        // A pseudo-element (`::before`, with the combinators in front of it) goes behind the `:where()`.
        let (nested, pseudo) = match nested.find("::") {
            Some(at) => {
                let start = nested[..at].trim_end_matches(|c: char| matches!(c, '>' | '+' | '~') || c.is_whitespace()).len();
                let tail = &nested[at + 2..];
                let end = at + 2 + tail.find(|c: char| !(c.is_ascii_alphabetic() || c == '-')).unwrap_or(tail.len());
                (format!("{}{}", &nested[..start], &nested[end..]), nested[start..end].to_owned())
            }
            None => (nested.to_owned(), String::new()),
        };
        // A nested selector that starts with a space is a descendant of the block (each comma part);
        // otherwise it is appended to the block selector as a whole.
        let scoped = if nested.starts_with(' ') {
            nested.split(',').map(|s| format!("{selector} {}", s.trim())).collect::<Vec<_>>().join(", ")
        } else {
            format!("{selector}{nested}")
        };
        out.push_str(&format!(":root :where({scoped}){pseudo}{{{}}}", value.trim()));
    }
    out
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn fluid_sizes_use_the_wide_size_as_maximum_viewport() {
        let settings = json!({"typography": {"fluid": true}, "layout": {"wideSize": "1340px"}});
        let fluid = json!({"min": "1rem", "max": "1.125rem"});
        assert_eq!(fluid_font_size("1rem", Some(&fluid), &settings), "clamp(1rem, 1rem + ((1vw - 0.2rem) * 0.196), 1.125rem)");
        let fluid = json!({"min": "1.125rem", "max": "1.375rem"});
        assert_eq!(fluid_font_size("1.38rem", Some(&fluid), &settings), "clamp(1.125rem, 1.125rem + ((1vw - 0.2rem) * 0.392), 1.375rem)");
        let fluid = json!({"min": "2.15rem", "max": "3rem"});
        assert_eq!(fluid_font_size("2.15rem", Some(&fluid), &settings), "clamp(2.15rem, 2.15rem + ((1vw - 0.2rem) * 1.333), 3rem)");
    }

    #[test]
    fn fluid_can_be_switched_off_per_size() {
        let settings = json!({"typography": {"fluid": true}});
        assert_eq!(fluid_font_size("0.875rem", Some(&json!(false)), &settings), "0.875rem");
        assert_eq!(fluid_font_size("1rem", None, &json!({"typography": {}})), "1rem");
    }

    #[test]
    fn small_sizes_stay_fixed_and_large_ones_get_a_range() {
        let settings = json!({"typography": {"fluid": true}});
        assert_eq!(fluid_font_size("13px", None, &settings), "13px");
        assert!(fluid_font_size("36px", None, &settings).starts_with("clamp("));
    }

    #[test]
    fn custom_css_is_scoped_like_wordpress() {
        assert_eq!(process_custom_css("& li{margin-top: 0.5rem;}", ".wp-block-list"), ":root :where(.wp-block-list li){margin-top: 0.5rem;}");
        assert_eq!(
            process_custom_css("&.has-text-align-right { border-width: 0 2px 0 0; } &.has-text-align-center { border-width: 0; }", ".q"),
            ":root :where(.q.has-text-align-right ){border-width: 0 2px 0 0;}:root :where(.q.has-text-align-center ){border-width: 0;}"
        );
        assert_eq!(
            process_custom_css("& textarea, input:not([type=submit]){a:b;} & label {c:d; }", ".f"),
            ":root :where(.f textarea, .f input:not([type=submit])){a:b;}:root :where(.f label){c:d;}"
        );
    }

    #[test]
    fn layout_spacing_rules_per_type() {
        assert_eq!(
            layout_spacing_rules("default", ".is-layout-flow", "1.2rem"),
            ":root :where(.is-layout-flow) > :first-child{margin-block-start: 0;}:root :where(.is-layout-flow) > :last-child{margin-block-end: 0;}:root :where(.is-layout-flow) > *{margin-block-start: 1.2rem;margin-block-end: 0;}"
        );
        assert_eq!(layout_spacing_rules("flex", ".is-layout-flex", "1rem"), ":root :where(.is-layout-flex){gap: 1rem;}");
    }

    #[test]
    fn block_class_names() {
        assert_eq!(block_class("core/post-title"), ".wp-block-post-title");
        assert_eq!(block_class("my/plugin"), ".wp-block-my-plugin");
    }

    #[test]
    fn variation_classes_follow_the_first_compound_selector() {
        assert_eq!(variation_selector(".is-style-r--2", ".wp-block-image img, .wp-block-image .crop"), ".wp-block-image.is-style-r--2 img, .wp-block-image.is-style-r--2 .crop");
        assert_eq!(variation_selector(".is-style-x", ".wp-block-button:hover"), ".wp-block-button.is-style-x:hover");
    }

    #[test]
    fn nested_custom_css_appends_or_scopes_and_handles_pseudo_elements() {
        // A selector list that does not start with a space is appended as a whole.
        assert_eq!(
            process_custom_css("&.a, .b .c{x:y;}", ".q"),
            ":root :where(.q.a, .b .c){x:y;}"
        );
        assert_eq!(process_custom_css("& li::before{a:b;}", ".l"), ":root :where(.l li)::before{a:b;}");
        // Text without braces applies to the block itself, even when it is only whitespace.
        assert_eq!(process_custom_css(" &:not(.w){width:1px}", ".s"), ":root :where(.s){}:root :where(.s:not(.w)){width:1px}");
    }}
