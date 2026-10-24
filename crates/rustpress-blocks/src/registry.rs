//! Block type metadata, read from the `block.json` files that ship with
//! WordPress (`wp-includes/blocks/<name>/block.json`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct BlockType {
    pub name: String,
    pub attributes: Map<String, Value>,
    pub supports: Value,
    pub uses_context: Vec<String>,
    pub provides_context: Map<String, Value>,
    /// `selectors` from block.json (feature specific CSS selectors).
    pub selectors: Value,
    /// Absolute path of the block's directory (for `style.min.css` etc.).
    pub dir: PathBuf,
    /// `style` assets declared in block.json (`file:./style.css`).
    pub has_style: bool,
    /// Frontend style handles from block.json `style` (string or list).
    pub styles: Vec<String>,
}

impl BlockType {
    /// `block_has_support( $block_type, $feature, $default )`.
    pub fn has_support(&self, path: &[&str], default: bool) -> bool {
        match self.support(path) {
            Some(Value::Bool(true)) => true,
            Some(Value::Array(_) | Value::Object(_)) => true,
            Some(_) => false,
            None => default,
        }
    }

    /// The raw value at `supports.<path>`.
    pub fn support(&self, path: &[&str]) -> Option<&Value> {
        let mut current = &self.supports;
        for key in path {
            current = current.get(*key)?;
        }
        Some(current)
    }

    /// `wp_should_skip_block_supports_serialization`.
    pub fn skips_serialization(&self, feature_set: &str, feature: Option<&str>) -> bool {
        match self.support(&[feature_set, "__experimentalSkipSerialization"]) {
            Some(Value::Array(list)) => feature.is_some_and(|f| list.iter().any(|v| v.as_str() == Some(f))),
            Some(Value::Bool(b)) => *b,
            _ => false,
        }
    }

    /// `WP_Block_Type::prepare_attributes_for_render`: invalid values fall back
    /// to the default, and missing attributes get their default.
    pub fn prepare_attributes(&self, attrs: &Map<String, Value>) -> Map<String, Value> {
        let mut out = attrs.clone();
        for (name, value) in attrs {
            let Some(schema) = self.attributes.get(name) else { continue };
            if validates(value, schema) {
                continue;
            }
            match schema.get("default") {
                Some(default) if !default.is_null() => {
                    out.insert(name.clone(), default.clone());
                }
                _ => {
                    out.shift_remove(name);
                }
            }
        }
        for (name, schema) in &self.attributes {
            if !out.contains_key(name)
                && let Some(default) = schema.get("default").filter(|d| !d.is_null()) {
                    out.insert(name.clone(), default.clone());
                }
        }
        out
    }
}

/// A small subset of `rest_validate_value_from_schema`: type and enum.
fn validates(value: &Value, schema: &Value) -> bool {
    if let Some(options) = schema.get("enum").and_then(Value::as_array)
        && !options.contains(value) {
            return false;
        }
    let matches_type = |t: &str| match t {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_f64().is_some_and(|f| f.fract() == 0.0),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "null" => value.is_null(),
        _ => true,
    };
    match schema.get("type") {
        Some(Value::String(t)) => matches_type(t),
        Some(Value::Array(types)) => types.iter().filter_map(Value::as_str).any(matches_type),
        _ => true,
    }
}

/// Loads and caches block types.
pub struct Registry {
    blocks_dir: PathBuf,
    cache: Mutex<HashMap<String, Option<Arc<BlockType>>>>,
}

impl Registry {
    /// `blocks_dir` is `<wordpress>/wp-includes/blocks`.
    pub fn new(blocks_dir: impl Into<PathBuf>) -> Self {
        Self { blocks_dir: blocks_dir.into(), cache: Mutex::new(HashMap::new()) }
    }

    pub fn blocks_dir(&self) -> &Path {
        &self.blocks_dir
    }

    /// The core blocks in registration order: the static blocks of `require-static-blocks.php`,
    /// then the dynamic ones of `require-dynamic-blocks.php`.
    pub fn registration_order(&self) -> Vec<String> {
        let mut names = Vec::new();
        if let Ok(text) = std::fs::read_to_string(self.blocks_dir.join("require-static-blocks.php")) {
            for line in text.lines() {
                if let Some(name) = line.trim().strip_prefix('\'').and_then(|l| l.split('\'').next()) {
                    names.push(format!("core/{name}"));
                }
            }
        }
        if let Ok(text) = std::fs::read_to_string(self.blocks_dir.join("require-dynamic-blocks.php")) {
            for line in text.lines() {
                if let Some(file) = line.split("/blocks/").nth(1).and_then(|l| l.split(".php").next()) {
                    names.push(format!("core/{file}"));
                }
            }
        }
        names
    }

    /// `WP_Theme_JSON_Resolver::get_block_data()`: the styles blocks declare themselves
    /// (`supports.__experimentalStyle`), and an empty block gap for blocks with a default one.
    pub fn block_data(&self) -> Value {
        let mut blocks = Map::new();
        for name in self.registration_order() {
            let Some(block) = self.get(&name) else { continue };
            if let Some(style) = block.support(&["__experimentalStyle"]) {
                blocks.insert(name.clone(), style.clone());
            }
            if block.support(&["spacing", "blockGap", "__experimentalDefault"]).is_some() {
                let entry = blocks.entry(name.clone()).or_insert_with(|| Value::Object(Map::new()));
                if let Some(entry) = entry.as_object_mut() {
                    let spacing = entry.entry("spacing").or_insert_with(|| Value::Object(Map::new()));
                    if let Some(spacing) = spacing.as_object_mut() {
                        spacing.entry("blockGap").or_insert(Value::Null);
                    }
                }
            }
        }
        if blocks.is_empty() {
            return Value::Object(Map::new());
        }
        let mut styles = Map::new();
        styles.insert("blocks".to_owned(), Value::Object(blocks));
        let mut root = Map::new();
        root.insert("styles".to_owned(), Value::Object(styles));
        Value::Object(root)
    }

    pub fn get(&self, name: &str) -> Option<Arc<BlockType>> {
        let mut cache = self.cache.lock().expect("registry lock");
        if let Some(found) = cache.get(name) {
            return found.clone();
        }
        let loaded = self.load(name).map(Arc::new);
        cache.insert(name.to_owned(), loaded.clone());
        loaded
    }

    fn load(&self, name: &str) -> Option<BlockType> {
        let slug = name.strip_prefix("core/")?;
        if slug.contains(['/', '\\', '.']) {
            return None;
        }
        let dir = self.blocks_dir.join(slug);
        let text = std::fs::read_to_string(dir.join("block.json")).ok()?;
        let json: Value = serde_json::from_str(&text).ok()?;
        let object = json.as_object()?;
        Some(BlockType {
            name: name.to_owned(),
            attributes: object.get("attributes").and_then(Value::as_object).cloned().unwrap_or_default(),
            supports: object.get("supports").cloned().unwrap_or(Value::Null),
            uses_context: object
                .get("usesContext")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
                .unwrap_or_default(),
            provides_context: object.get("providesContext").and_then(Value::as_object).cloned().unwrap_or_default(),
            selectors: object.get("selectors").cloned().unwrap_or(Value::Null),
            has_style: object.contains_key("style"),
            styles: match object.get("style") {
                Some(Value::String(s)) => vec![s.clone()],
                Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
                _ if dir.join("style.min.css").is_file() => vec![format!("wp-block-{slug}")],
                _ => Vec::new(),
            },
            dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn block(attrs: Value, supports: Value) -> BlockType {
        BlockType {
            name: "core/test".into(),
            attributes: attrs.as_object().cloned().unwrap_or_default(),
            supports,
            uses_context: Vec::new(),
            provides_context: Map::new(),
            selectors: Value::Null,
            dir: PathBuf::new(),
            has_style: false,
            styles: Vec::new(),
        }
    }

    #[test]
    fn defaults_fill_in_and_invalid_values_revert() {
        let b = block(
            json!({"level": {"type": "number", "default": 2}, "isLink": {"type": "boolean", "default": false}, "tag": {"type": "string", "enum": ["a", "b"]}}),
            Value::Null,
        );
        let attrs = json!({"level": "x", "tag": "c", "extra": 1});
        let prepared = b.prepare_attributes(attrs.as_object().unwrap());
        assert_eq!(prepared["level"], 2);
        assert_eq!(prepared["isLink"], false);
        assert!(prepared.get("tag").is_none());
        assert_eq!(prepared["extra"], 1);
    }

    #[test]
    fn support_lookup_follows_block_has_support() {
        let b = block(json!({}), json!({"color": {"text": true}, "align": ["wide"], "html": false, "anchor": true}));
        assert!(b.has_support(&["color"], false));
        assert!(b.has_support(&["color", "text"], false));
        assert!(b.has_support(&["align"], false));
        assert!(!b.has_support(&["html"], true) || !b.has_support(&["html"], false));
        assert!(b.has_support(&["anchor"], false));
        assert!(!b.has_support(&["typography"], false));
        assert!(b.has_support(&["customClassName"], true));
    }

    #[test]
    fn skip_serialization_can_be_boolean_or_list() {
        let b = block(json!({}), json!({"color": {"__experimentalSkipSerialization": true}, "spacing": {"__experimentalSkipSerialization": ["blockGap"]}}));
        assert!(b.skips_serialization("color", Some("text")));
        assert!(b.skips_serialization("spacing", Some("blockGap")));
        assert!(!b.skips_serialization("spacing", Some("padding")));
        assert!(!b.skips_serialization("border", None));
    }
}
