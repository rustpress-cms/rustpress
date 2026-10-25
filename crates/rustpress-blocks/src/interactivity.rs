//! The server-side part of the Interactivity API (`WP_Interactivity_API::process_directives()`):
//! the directives `data-wp-interactive`, `-context`, `-bind`, `-class`, `-style` and `-text`
//! are evaluated against the state the blocks registered and the context in the markup.
//! `data-wp-each` and `data-wp-router-region` are left alone.

use serde_json::{Map, Value};

use crate::html::{Tag, TagEditor, is_void, tags};

/// What `wp_interactivity_state()` and the derived state callbacks leave behind.
#[derive(Debug, Clone, Default)]
pub struct Interactivity {
    /// Namespace -> state.
    state: Map<String, Value>,
    /// Derived state closures that were registered: `(namespace, path)`.
    closures: Vec<(String, String)>,
    /// The closures that were evaluated, by namespace (`derivedStateClosures`).
    derived: Vec<(String, Vec<String>)>,
}

impl Interactivity {
    /// `wp_interactivity_state( $ns, $data )`: merges recursively into the namespace.
    pub fn state(&mut self, namespace: &str, data: Value) {
        let slot = self.state.entry(namespace.to_owned()).or_insert_with(|| Value::Object(Map::new()));
        merge(slot, data);
    }

    /// `wp_interactivity_state( $ns, array( $name => function () {...} ) )` for a closure the
    /// renderer implements itself (see [`run_closure`]). It prints as `{}`.
    pub fn state_closure(&mut self, namespace: &str, name: &str) {
        let mut data = Map::new();
        data.insert(name.to_owned(), Value::Object(Map::new()));
        self.state(namespace, Value::Object(data));
        if !self.closures.iter().any(|(n, p)| n == namespace && p == name) {
            self.closures.push((namespace.to_owned(), name.to_owned()));
        }
    }

    /// The JSON of the `wp-script-module-data-@wordpress/interactivity` script, if there is any data.
    pub fn data_json(&self) -> Option<String> {
        let mut data = Map::new();
        let state: Map<String, Value> = self.state.iter().filter(|(_, v)| !is_empty(v)).map(|(k, v)| (k.clone(), v.clone())).collect();
        if !state.is_empty() {
            data.insert("state".into(), Value::Object(state));
        }
        let derived: Map<String, Value> = self.derived.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (k.clone(), Value::from(v.clone()))).collect();
        if !derived.is_empty() {
            data.insert("derivedStateClosures".into(), Value::Object(derived));
        }
        if data.is_empty() {
            return None;
        }
        // JSON_HEX_TAG | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE.
        Some(serde_json::to_string(&Value::Object(data)).ok()?.replace('<', "\\u003C").replace('>', "\\u003E"))
    }

    /// The `<script>` element that carries the data, if there is any.
    pub fn data_script(&self) -> Option<String> {
        self.data_json().map(|json| format!("<script id=\"wp-script-module-data-@wordpress/interactivity\" type=\"application/json\">\n{json}\n</script>\n"))
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// `array_replace_recursive()`.
fn merge(target: &mut Value, source: Value) {
    match (target, source) {
        (Value::Object(into), Value::Object(from)) => {
            for (key, value) in from {
                match into.get_mut(&key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        into.insert(key, value);
                    }
                }
            }
        }
        (target, source) => *target = source,
    }
}

/// The derived state callbacks of core blocks. `context` is the context of the namespace.
fn run_closure(namespace: &str, path: &str, context: &Value) -> Value {
    match (namespace, path) {
        ("core/accordion", "isOpen") => context.get("openByDefault").cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// PHP truthiness.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty() && s != "0",
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

const DIRECTIVES: [&str; 8] = ["interactive", "router-region", "context", "bind", "class", "style", "text", "each"];

/// Tags whose closer the tag processor does not visit.
fn visits_closer(name: &str) -> bool {
    !is_void(name) && !matches!(name, "script" | "iframe" | "noembed" | "noframes" | "style" | "textarea" | "title" | "xmp")
}

#[derive(Debug)]
struct Parsed {
    prefix: String,
    suffix: Option<String>,
    unique_id: Option<String>,
}

/// `parse_directive_name()` for an attribute name that starts with `data-wp-`.
fn parse_directive_name(attribute: &str) -> Option<Parsed> {
    let name = attribute.get(8..)?;
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    let suffix_index = name.find("--");
    if suffix_index == Some(0) || name.starts_with('-') {
        return None;
    }
    let Some(index) = suffix_index else { return Some(Parsed { prefix: name.to_owned(), suffix: None, unique_id: None }) };
    let prefix = name[..index].to_owned();
    let remaining = &name[index..];
    if remaining.bytes().take_while(|b| *b == b'-').count() == 3 {
        let unique = &remaining[3..];
        return Some(Parsed { prefix, suffix: None, unique_id: (!unique.is_empty()).then(|| unique.to_owned()) });
    }
    let suffix = &remaining[2..];
    if let Some(at) = suffix.find("---")
        && suffix.as_bytes().get(at + 3) != Some(&b'-')
    {
        let unique = &suffix[at + 3..];
        let own = &suffix[..at];
        return Some(Parsed { prefix, suffix: (!own.is_empty()).then(|| own.to_owned()), unique_id: (!unique.is_empty()).then(|| unique.to_owned()) });
    }
    Some(Parsed { prefix, suffix: (!suffix.is_empty()).then(|| suffix.to_owned()), unique_id: None })
}

#[derive(Debug)]
struct Entry {
    namespace: Option<String>,
    value: Value,
    suffix: Option<String>,
    unique_id: Option<String>,
}

/// `extract_directive_value()`: the namespace prefix and the decoded value.
fn extract_value(raw: Option<&str>, default_namespace: Option<&str>) -> (Option<String>, Value) {
    let default = default_namespace.map(str::to_owned);
    let Some(raw) = raw.filter(|r| !r.is_empty()) else { return (default, Value::Null) };
    let mut namespace = default;
    let mut value = raw;
    if let Some((head, tail)) = raw.split_once("::")
        && !tail.is_empty()
        && !head.is_empty()
        && head.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
    {
        namespace = Some(head.to_owned());
        value = tail;
    }
    match serde_json::from_str::<Value>(value) {
        Ok(decoded) => (namespace, decoded),
        Err(_) => (namespace, Value::String(value.to_owned())),
    }
}

struct Processor<'a> {
    st: &'a mut Interactivity,
    namespaces: Vec<Option<String>>,
    contexts: Vec<Map<String, Value>>,
}

impl Processor<'_> {
    fn entries(&self, editor: &TagEditor, prefix: &str) -> Vec<Entry> {
        let default = self.namespaces.last().cloned().flatten();
        let mut out = Vec::new();
        for attr in &editor.tag().attrs {
            if !attr.name.starts_with(&format!("data-wp-{prefix}")) {
                continue;
            }
            let Some(parsed) = parse_directive_name(&attr.name) else { continue };
            if parsed.prefix != prefix {
                continue;
            }
            let Some(value) = editor.get_attribute(&attr.name) else { continue };
            let raw = (attr.value.is_some() || !value.is_empty()).then_some(value);
            let (namespace, value) = extract_value(raw, default.as_deref());
            out.push(Entry { namespace, value, suffix: parsed.suffix, unique_id: parsed.unique_id });
        }
        out.sort_by(|a, b| (a.suffix.as_deref().unwrap_or(""), a.unique_id.as_deref().unwrap_or("")).cmp(&(b.suffix.as_deref().unwrap_or(""), b.unique_id.as_deref().unwrap_or(""))));
        out
    }

    /// `evaluate()`.
    fn evaluate(&mut self, entry: &Entry) -> Value {
        let (Some(namespace), Value::String(path)) = (entry.namespace.as_deref().filter(|n| !n.is_empty()), &entry.value) else { return Value::Null };
        if path.is_empty() {
            return Value::Null;
        }
        let context = self.contexts.last().and_then(|c| c.get(namespace)).cloned().unwrap_or(Value::Object(Map::new()));
        let state = self.st.state.get(namespace).cloned().unwrap_or(Value::Object(Map::new()));
        let negate = path.starts_with('!');
        let path = path.strip_prefix('!').unwrap_or(path);
        let segments: Vec<&str> = path.split('.').collect();
        let mut current = {
            let mut store = Map::new();
            store.insert("state".into(), state);
            store.insert("context".into(), context.clone());
            Value::Object(store)
        };
        for (index, segment) in segments.iter().enumerate() {
            if *segment == "length" {
                match &current {
                    Value::Array(items) => {
                        current = Value::from(items.len());
                        break;
                    }
                    Value::String(text) => {
                        current = Value::from(text.len());
                        break;
                    }
                    _ => {}
                }
            }
            let next = match &current {
                Value::Object(map) => map.get(*segment).filter(|v| !v.is_null()).cloned(),
                Value::Array(items) => segment.parse::<usize>().ok().and_then(|i| items.get(i)).filter(|v| !v.is_null()).cloned(),
                _ => None,
            };
            let Some(next) = next else {
                current = Value::Null;
                break;
            };
            current = next;
            // A derived state closure: `state.<path>` registered by a block.
            let closure_path = segments[..=index].join(".");
            let state_path = closure_path.strip_prefix("state.");
            if segments[0] == "state"
                && let Some(state_path) = state_path
                && self.st.closures.iter().any(|(n, p)| n == namespace && p == state_path)
            {
                current = run_closure(namespace, state_path, &context);
                match self.st.derived.iter_mut().find(|(n, _)| n == namespace) {
                    Some((_, paths)) => {
                        if !paths.iter().any(|p| p == &closure_path) {
                            paths.push(closure_path.clone());
                        }
                    }
                    None => self.st.derived.push((namespace.to_owned(), vec![closure_path.clone()])),
                }
            }
        }
        if negate { Value::Bool(!truthy(&current)) } else { current }
    }

    fn enter(&mut self, editor: &mut TagEditor, prefixes: &[&'static str], text: &mut Option<String>) {
        for prefix in DIRECTIVES {
            if !prefixes.contains(&prefix) {
                continue;
            }
            match prefix {
                "interactive" => self.interactive(editor),
                "context" => self.context(editor),
                "bind" => self.bind(editor),
                "class" => self.class(editor),
                "style" => self.style(editor),
                "text" => *text = self.text(editor),
                _ => {}
            }
        }
    }

    fn exit(&mut self, prefixes: &[&'static str]) {
        for prefix in DIRECTIVES.iter().rev() {
            if !prefixes.contains(prefix) {
                continue;
            }
            match *prefix {
                "interactive" => {
                    self.namespaces.pop();
                }
                "context" => {
                    self.contexts.pop();
                }
                _ => {}
            }
        }
    }

    fn interactive(&mut self, editor: &TagEditor) {
        let value = editor.get_attribute("data-wp-interactive").filter(|v| !v.is_empty());
        let new_namespace = value.and_then(|v| match serde_json::from_str::<Value>(v) {
            Ok(Value::Object(map)) => map.get("namespace").and_then(Value::as_str).map(str::to_owned),
            Ok(Value::Array(_)) => None,
            _ => Some(v.to_owned()),
        });
        let valid = new_namespace.filter(|n| n.chars().next().is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/')));
        let namespace = valid.or_else(|| self.namespaces.last().cloned().flatten());
        self.namespaces.push(namespace);
    }

    fn context(&mut self, editor: &TagEditor) {
        let mut context = Value::Object(self.contexts.last().cloned().unwrap_or_default());
        for entry in self.entries(editor, "context") {
            if entry.suffix.is_some() {
                continue;
            }
            let Some(namespace) = entry.namespace else { continue };
            let value = if entry.value.is_object() || entry.value.is_array() { entry.value } else { Value::Object(Map::new()) };
            let mut patch = Map::new();
            patch.insert(namespace, value);
            merge(&mut context, Value::Object(patch));
        }
        self.contexts.push(match context {
            Value::Object(map) => map,
            _ => Map::new(),
        });
    }

    fn bind(&mut self, editor: &mut TagEditor) {
        for entry in self.entries(editor, "bind") {
            let Some(name) = entry.suffix.clone().filter(|s| !s.is_empty()) else { continue };
            if entry.unique_id.is_some() || name.starts_with("on") {
                continue;
            }
            let result = self.evaluate(&entry);
            let result = match result {
                Value::Array(_) | Value::Object(_) => Value::Null,
                other => other,
            };
            let dashed = name.len() > 5 && name.as_bytes()[4] == b'-';
            match &result {
                Value::Null => editor.remove_attribute(&name),
                Value::Bool(false) if !dashed => editor.remove_attribute(&name),
                Value::Bool(true) if !dashed => editor.set_boolean_attribute(&name),
                Value::Bool(b) => editor.set_attribute(&name, if *b { "true" } else { "false" }),
                Value::String(s) => editor.set_attribute(&name, s),
                Value::Number(n) => editor.set_attribute(&name, &n.to_string()),
                _ => {}
            }
        }
    }

    fn class(&mut self, editor: &mut TagEditor) {
        for entry in self.entries(editor, "class") {
            let Some(suffix) = entry.suffix.clone().filter(|s| !s.is_empty()) else { continue };
            let class_name = match &entry.unique_id {
                Some(id) => format!("{suffix}---{id}"),
                None => suffix,
            };
            if truthy(&self.evaluate(&entry)) {
                editor.add_class(&class_name);
            } else {
                editor.remove_class(&class_name);
            }
        }
    }

    fn style(&mut self, editor: &mut TagEditor) {
        for entry in self.entries(editor, "style") {
            let Some(property) = entry.suffix.clone().filter(|s| !s.is_empty()) else { continue };
            if entry.unique_id.is_some() {
                continue;
            }
            let value = self.evaluate(&entry);
            let current = editor.get_attribute("style").unwrap_or("").to_owned();
            if truthy(&value) || !current.is_empty() {
                let merged = merge_style_property(&current, &property, &value);
                if merged.is_empty() {
                    editor.remove_attribute("style");
                } else {
                    editor.set_attribute("style", &merged);
                }
            }
        }
    }

    fn text(&mut self, editor: &TagEditor) -> Option<String> {
        let entry = self.entries(editor, "text").into_iter().find(|e| e.suffix.is_none() && e.unique_id.is_none() && !is_empty_value(&e.value))?;
        Some(match self.evaluate(&entry) {
            Value::String(s) => rustpress_core::formatting::esc_html(&s),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
    }
}

fn is_empty_value(value: &Value) -> bool {
    !truthy(value)
}

/// `merge_style_property()`.
fn merge_style_property(style: &str, name: &str, value: &Value) -> String {
    let value = match value {
        Value::String(s) if truthy(value) => Some(s.trim().trim_end_matches(';').to_owned()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    };
    let mut result = String::new();
    for assignment in style.split(';') {
        if assignment.trim().is_empty() {
            continue;
        }
        let (key, val) = assignment.split_once(':').unwrap_or((assignment, ""));
        if key.trim() != name {
            result.push_str(&format!("{}:{};", key.trim(), val.trim()));
        }
    }
    if let Some(value) = value.filter(|v| !v.is_empty()) {
        result.push_str(&format!("{name}:{value};"));
    }
    result
}

/// Evaluates the directives of `html` (`wp_interactivity_process_directives()`). Markup whose tags do not
/// balance is returned as it was.
pub fn process_directives(html: &str, st: &mut Interactivity) -> String {
    if !html.contains("data-wp-") {
        return html.to_owned();
    }
    run(html, st).unwrap_or_else(|| html.to_owned())
}

fn run(html: &str, st: &mut Interactivity) -> Option<String> {
    let all = tags(html, 0);
    let mut processor = Processor { st, namespaces: Vec::new(), contexts: Vec::new() };
    let mut stack: Vec<(String, Vec<&'static str>)> = Vec::new();
    // Replacements `(start, end, text)` in document order.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    // `data-wp-text` bodies: the opener's index and the text.
    let mut pending_text: Vec<(usize, String)> = Vec::new();
    let mut index = 0;
    while index < all.len() {
        let tag = &all[index];
        let name = tag.name.as_str();
        if name == "svg" || name == "math" {
            if !tag.is_closer {
                index = skip_to_closer(&all, index);
            }
            index += 1;
            continue;
        }
        if tag.is_closer {
            if !visits_closer(name) {
                index += 1;
                continue;
            }
            let (open_name, prefixes) = stack.pop()?;
            if open_name != name {
                return None;
            }
            processor.exit(&prefixes);
            index += 1;
            continue;
        }
        if tag.attrs.iter().any(|a| a.name.starts_with("data-wp-each-child")) {
            index = skip_balanced(&all, index);
            index += 1;
            continue;
        }
        let mut prefixes: Vec<&'static str> = Vec::new();
        for attr in tag.attrs.iter().filter(|a| a.name.starts_with("data-wp-")) {
            if let Some(parsed) = parse_directive_name(&attr.name)
                && let Some(known) = DIRECTIVES.iter().find(|d| **d == parsed.prefix)
                && !prefixes.contains(known)
            {
                prefixes.push(known);
            }
        }
        let closes = visits_closer(name);
        if closes {
            stack.push((name.to_owned(), prefixes.clone()));
        }
        if prefixes.is_empty() {
            index += 1;
            continue;
        }
        let mut editor = TagEditor::from_tag(html, tag.clone());
        let mut text = None;
        processor.enter(&mut editor, &prefixes, &mut text);
        if editor.is_modified() {
            edits.push(editor.tag_replacement());
        }
        if let Some(text) = text {
            pending_text.push((index, text));
        }
        if !closes {
            processor.exit(&prefixes);
        }
        index += 1;
    }
    if !stack.is_empty() {
        return None;
    }

    // The bodies of `data-wp-text` elements replace everything between the tags.
    let mut cuts: Vec<(usize, usize, String)> = Vec::new();
    for (open, text) in pending_text {
        let close = skip_balanced(&all, open);
        if close > open && all[close].is_closer {
            cuts.push((all[open].end, all[close].start, text));
        }
    }
    edits.retain(|(start, _, _)| !cuts.iter().any(|(from, to, _)| start >= from && start < to));
    edits.extend(cuts);
    edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut out = html.to_owned();
    for (start, end, text) in edits {
        out.replace_range(start..end, &text);
    }
    Some(out)
}

/// The index of the tag that closes the (non-void) element opened at `open`, or `open` itself.
fn skip_balanced(all: &[Tag], open: usize) -> usize {
    let name = &all[open].name;
    if !visits_closer(name) {
        return open;
    }
    let mut depth = 0usize;
    for (offset, tag) in all[open + 1..].iter().enumerate() {
        if &tag.name != name {
            continue;
        }
        if !tag.is_closer {
            depth += 1;
        } else if depth == 0 {
            return open + 1 + offset;
        } else {
            depth -= 1;
        }
    }
    open
}

/// `skip_to_tag_closer()` for `svg` and `math`: the index of the closing tag.
fn skip_to_closer(all: &[Tag], open: usize) -> usize {
    let name = &all[open].name;
    let mut depth = 1usize;
    for (offset, tag) in all[open + 1..].iter().enumerate() {
        if &tag.name == name {
            if tag.is_closer {
                depth -= 1;
            } else {
                depth += 1;
            }
            if depth == 0 {
                return open + 1 + offset;
            }
        }
    }
    all.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_one(html: &str) -> String {
        process_directives(html, &mut Interactivity::default())
    }

    #[test]
    fn bound_attributes_without_state_are_removed() {
        let html = "<button data-wp-bind--aria-expanded=\"state.isSubmenuOpen\" aria-label=\"x\" aria-expanded=\"false\" class=\"c\" ><svg></svg></button>";
        assert_eq!(run_one(html), "<button data-wp-bind--aria-expanded=\"state.isSubmenuOpen\" aria-label=\"x\"  class=\"c\" ><svg></svg></button>");
        let html = "<a class=\"c\" data-wp-bind--aria-expanded=\"state.x\" aria-expanded=\"false\">t</a>";
        assert_eq!(run_one(html), "<a class=\"c\" data-wp-bind--aria-expanded=\"state.x\" >t</a>");
    }

    #[test]
    fn markup_without_directives_is_unchanged() {
        assert_eq!(run_one("<p aria-expanded=\"false\">x</p>"), "<p aria-expanded=\"false\">x</p>");
    }

    #[test]
    fn state_context_and_derived_closures() {
        let mut st = Interactivity::default();
        st.state_closure("core/accordion", "isOpen");
        let html = "<div data-wp-interactive=\"core/accordion\"><div data-wp-context='{ \"openByDefault\": true }' data-wp-class--is-open=\"state.isOpen\"><button data-wp-bind--aria-expanded=\"state.isOpen\"></button></div></div>";
        let out = process_directives(html, &mut st);
        assert_eq!(
            out,
            "<div data-wp-interactive=\"core/accordion\"><div class=\"is-open\" data-wp-context='{ \"openByDefault\": true }' data-wp-class--is-open=\"state.isOpen\"><button aria-expanded=\"true\" data-wp-bind--aria-expanded=\"state.isOpen\"></button></div></div>"
        );
        assert_eq!(st.data_json().unwrap(), "{\"state\":{\"core/accordion\":{\"isOpen\":{}}},\"derivedStateClosures\":{\"core/accordion\":[\"state.isOpen\"]}}");
    }

    #[test]
    fn unbalanced_markup_is_left_alone() {
        let html = "<div data-wp-class--x=\"state.y\"><p>open";
        assert_eq!(run_one(html), html);
    }

    #[test]
    fn directive_names() {
        let p = parse_directive_name("data-wp-bind--aria-expanded").unwrap();
        assert_eq!((p.prefix.as_str(), p.suffix.as_deref(), p.unique_id.as_deref()), ("bind", Some("aria-expanded"), None));
        let p = parse_directive_name("data-wp-on--click---a").unwrap();
        assert_eq!((p.prefix.as_str(), p.suffix.as_deref(), p.unique_id.as_deref()), ("on", Some("click"), Some("a")));
        assert!(parse_directive_name("data-wp---x").is_none());
    }
}
