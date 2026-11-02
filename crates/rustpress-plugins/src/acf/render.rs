//! The inputs of fields as ACF prints them in forms: `acf_render_fields()`, `acf_render_field_wrap()`, the input
//! helpers of `acf-input-functions.php` and the `render_field()` methods of the field types.
//!
//! The whitespace of the plugin's PHP templates (the indentation before `<?php`, the newline `?>` eats) is part of
//! the output and kept here.

use futures::future::BoxFuture;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::{Result, Site};
use rustpress_phpser::{Key, Value};

use super::fields::Field;
use super::php::{is_numeric, key, key_eq, loose_eq, string, strval, truthy};

/// What rendering needs from the request.
pub struct Ctx<'a> {
    pub site: &'a Site,
    /// `wp_create_nonce( $action )` for whoever looks at the form.
    pub nonce: &'a (dyn Fn(&str) -> String + Send + Sync),
    pub services: &'a dyn rustpress_core::native::PageServices,
    /// `is_admin()`: titles get no "Private: " prefix.
    pub admin: bool,
    /// `acf_get_setting( 'uploader' )`: `wp` (the media library) or `basic` (a file input).
    pub uploader: &'a str,
    /// A field asked for the media scripts (`acf_enqueue_uploader()`).
    pub wants_uploader: std::sync::atomic::AtomicBool,
    /// `current_user_can( $capability )`.
    pub can: &'a (dyn Fn(&str) -> bool + Send + Sync),
    /// `user_can_richedit()`.
    pub rich_edit: bool,
    /// `wp_default_editor()`: `tinymce` or `html`.
    pub default_editor: &'a str,
    /// `acf_get_user_setting( 'wysiwyg_height', 300 )`.
    pub wysiwyg_height: i64,
}

impl Ctx<'_> {
    /// `__( $text, 'acf' )`.
    fn acf(&self, text: &str) -> String {
        self.site.trd("acf", text)
    }

    /// `_x( $text, $context, 'acf' )`.
    fn acfx(&self, text: &str, context: &str) -> String {
        self.site.i18n().pgettext("acf", context, text)
    }
}

// ------------------------------------------------------------------ PHP helpers

/// `trim()` with PHP's default characters.
fn php_trim(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0b'])
}

/// `echo $float`: PHP prints floats with 14 significant digits (`precision`).
pub fn float_echo(f: f64) -> String {
    if f == 0.0 {
        return "0".to_owned();
    }
    if !f.is_finite() {
        return if f.is_nan() { "NAN".to_owned() } else if f > 0.0 { "INF".to_owned() } else { "-INF".to_owned() };
    }
    let sci = format!("{f:.13e}");
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let sign = if negative { "-" } else { "" };
    if !(-4..15).contains(&exponent) {
        let (first, rest) = digits.split_at(1);
        let rest = if rest.is_empty() { ".0".to_owned() } else { format!(".{rest}") };
        return format!("{sign}{first}{rest}E{}{}", if exponent < 0 { "-" } else { "+" }, exponent.abs());
    }
    if exponent < 0 {
        return format!("{sign}0.{}{digits}", "0".repeat((-exponent - 1) as usize));
    }
    let point = exponent as usize + 1;
    if digits.len() <= point {
        format!("{sign}{digits}{}", "0".repeat(point - digits.len()))
    } else {
        format!("{sign}{}.{}", &digits[..point], &digits[point..])
    }
}

/// `(string) $value` as `echo` and `esc_attr()` see it.
fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Float(f)) => float_echo(*f),
        other => strval(other),
    }
}

/// `acf_not_empty()`: true, or a number.
fn not_empty(value: &Value) -> bool {
    truthy(Some(value)) || is_numeric(value)
}

/// `acf_is_empty()`.
fn is_empty(value: &Value) -> bool {
    !not_empty(value)
}

/// `isset( $array[ $key ] )`.
fn isset(value: Option<&Value>) -> bool {
    !matches!(value, None | Some(Value::Null))
}

/// `acf_get_array()`: an array, nothing for an empty value, the value in an array otherwise.
fn get_array(value: Option<&Value>) -> Vec<(Key, Value)> {
    match value {
        Some(Value::Array(items)) => items.clone(),
        Some(v) if !is_empty(v) => vec![(Key::Int(0), v.clone())],
        _ => Vec::new(),
    }
}

/// An array key as a value (`foreach ( $array as $key => ... )`).
fn key_value(k: &Key) -> Value {
    match k {
        Key::Int(i) => Value::Int(*i),
        Key::Str(s) => Value::Str(s.clone()),
    }
}

fn key_text(k: &Key) -> String {
    match k {
        Key::Int(i) => i.to_string(),
        Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
    }
}

/// `in_array( $needle, $haystack )` (loose).
fn in_array(needle: &Value, haystack: &[Value]) -> bool {
    haystack.iter().any(|v| loose_eq(needle, v))
}

/// `json_encode( $value )` with PHP's default flags.
fn json_encode(value: &Value) -> String {
    rustpress_blocks::php_json::encode_with(&rustpress_core::options::php_to_json(value), rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: false })
}

/// `acf_esc_html()`: `wp_kses()` with the tags of posts; nothing for an array.
pub fn esc_html_acf(value: &Value) -> String {
    match value {
        Value::Array(_) | Value::Null => String::new(),
        other => rustpress_core::kses::kses_post(&text(Some(other))),
    }
}

fn kses(text: &str) -> String {
    rustpress_core::kses::kses_post(text)
}

/// `acf_idify()`.
pub fn idify(name: &str) -> String {
    name.to_lowercase().replace("][", "-").replace('[', "-").replace(']', "")
}

/// `acf_numval()`.
fn numval(value: &Value) -> Value {
    let n = super::php::numval(value);
    match n {
        Value::Float(f) if f.fract() == 0.0 && f.abs() < 9.2e18 => Value::Int(f as i64),
        Value::Int(_) | Value::Float(_) => n,
        _ => Value::Int(super::php::intval(Some(value))),
    }
}

fn as_f64(value: &Value) -> f64 {
    match super::php::numval(value) {
        Value::Int(i) => i as f64,
        Value::Float(f) => f,
        _ => super::php::intval(Some(value)) as f64,
    }
}

// ------------------------------------------------------------------ attributes

/// An array of attributes in PHP's order.
#[derive(Debug, Clone, Default)]
pub struct Attrs(pub Vec<(String, Value)>);

impl Attrs {
    pub fn new(items: &[(&str, Value)]) -> Self {
        let mut out = Self::default();
        for (k, v) in items {
            out.set(k, v.clone());
        }
        out
    }

    /// `$attrs[ $name ] = $value`.
    pub fn set(&mut self, name: &str, value: Value) {
        match self.0.iter_mut().find(|(k, _)| k == name) {
            Some(slot) => slot.1 = value,
            None => self.0.push((name.to_owned(), value)),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    /// `unset( $attrs[ $name ] )`, returning what was there.
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        let at = self.0.iter().position(|(k, _)| k == name)?;
        Some(self.0.remove(at).1)
    }

    /// `$attrs[ $name ] .= $suffix`.
    fn append(&mut self, name: &str, suffix: &str) {
        let current = text(self.get(name));
        self.set(name, string(&format!("{current}{suffix}")));
    }

    /// `wp_parse_args( $this, $defaults )`: the keys of the defaults first.
    fn with_defaults(self, defaults: &[(&str, Value)]) -> Self {
        let mut out = Self::new(defaults);
        for (k, v) in self.0 {
            out.set(&k, v);
        }
        out
    }
}

/// `acf_filter_attrs()` (`acf_clean_atts()`): empty values go, the boolean attributes come last as `name="name"`.
pub fn filter_attrs(attrs: &Attrs) -> Attrs {
    let mut filtered = Attrs(attrs.0.iter().filter(|(_, v)| not_empty(v)).cloned().collect());
    for name in ["required", "readonly", "disabled", "multiple"] {
        filtered.remove(name);
        if truthy(attrs.get(name)) {
            filtered.set(name, string(name));
        }
    }
    filtered
}

/// `acf_esc_attrs()`.
pub fn esc_attrs(attrs: &Attrs) -> String {
    let mut html = String::new();
    for (k, v) in &attrs.0 {
        let value = match v {
            Value::Str(s) if k != "value" => php_trim(&String::from_utf8_lossy(s)).to_owned(),
            Value::Bool(b) => if *b { "1" } else { "0" }.to_owned(),
            Value::Array(_) => json_encode(v),
            other => text(Some(other)),
        };
        html.push_str(&format!(" {}=\"{}\"", esc_attr(k), esc_attr(&value)));
    }
    php_trim(&html).to_owned()
}

/// `acf_get_hidden_input()`.
pub fn hidden_input(attrs: &Attrs) -> String {
    format!("<input type=\"hidden\" {}/>", esc_attrs(attrs))
}

/// `acf_get_text_input()`.
pub fn text_input(attrs: Attrs) -> String {
    let mut attrs = attrs.with_defaults(&[("type", string("text"))]);
    if let Some(Value::Str(s)) = attrs.get("value") {
        let escaped = rustpress_core::text_diff::htmlspecialchars(&String::from_utf8_lossy(s));
        attrs.set("value", string(&escaped));
    }
    format!("<input {}/>", esc_attrs(&attrs))
}

/// `acf_get_textarea_input()`.
fn textarea_input(mut attrs: Attrs) -> String {
    let value = attrs.remove("value").filter(|v| isset(Some(v)));
    format!("<textarea {}>{}</textarea>", esc_attrs(&attrs), rustpress_content::sanitize::esc_textarea(&text(value.as_ref())))
}

/// `acf_get_checkbox_input()`.
fn checkbox_input(attrs: Attrs) -> String {
    let mut attrs = attrs.with_defaults(&[("type", string("checkbox"))]);
    let label = attrs.remove("label").filter(|v| isset(Some(v))).unwrap_or_else(|| string(""));
    let checked = isset(attrs.get("checked"));
    let mut label_attrs = Attrs::default();
    if checked {
        label_attrs.set("class", string("selected"));
    }
    if truthy(attrs.get("button_group")) {
        attrs.remove("button_group");
        match attrs.remove("tabindex").filter(|v| isset(Some(v))) {
            Some(tabindex) => label_attrs.set("tabindex", string(&text(Some(&tabindex)))),
            None => label_attrs.set("tabindex", string(if checked { "0" } else { "-1" })),
        }
        label_attrs.set("role", string("radio"));
        label_attrs.set("aria-checked", string(if checked { "true" } else { "false" }));
    }
    let label_html = esc_attrs(&label_attrs);
    format!("<label{}><input {}/> {}</label>", if label_html.is_empty() { String::new() } else { format!(" {label_html}") }, esc_attrs(&attrs), esc_html_acf(&label))
}

/// `acf_get_radio_input()`.
fn radio_input(mut attrs: Attrs) -> String {
    attrs.set("type", string("radio"));
    checkbox_input(attrs)
}

/// `acf_get_select_input()`.
fn select_input(mut attrs: Attrs) -> String {
    let value = to_list(attrs.remove("value"));
    let choices = match attrs.remove("choices") {
        Some(Value::Array(items)) => items,
        None | Some(Value::Null) => Vec::new(),
        Some(other) => vec![(Key::Int(0), other)],
    };
    format!("<select {}>{}</select>", esc_attrs(&attrs), walk_select_input(&choices, &value, 0))
}

/// `(array) $value` as a list of the values.
fn to_list(value: Option<Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items.into_iter().map(|(_, v)| v).collect(),
        None | Some(Value::Null) => Vec::new(),
        Some(other) => vec![other],
    }
}

/// `acf_walk_select_input()`.
fn walk_select_input(choices: &[(Key, Value)], values: &[Value], depth: usize) -> String {
    let values: Vec<Value> = if depth == 0 { values.iter().map(|v| string(&esc_attr(&text(Some(v))))).collect() } else { values.to_vec() };
    let mut html = String::new();
    for (k, label) in choices {
        if let Value::Array(items) = label {
            html.push_str(&format!("<optgroup label=\"{}\">{}</optgroup>", esc_attr(&key_text(k)), walk_select_input(items, &values, depth + 1)));
        } else {
            let mut attrs = Attrs::new(&[("value", key_value(k))]);
            // `array_search()`: loose.
            if let Some(pos) = values.iter().position(|v| loose_eq(&string(&esc_attr(&key_text(k))), v)) {
                attrs.set("selected", string("selected"));
                attrs.set("data-i", Value::Int(pos as i64));
            }
            html.push_str(&format!("<option {}>{}</option>", esc_attrs(&attrs), esc_html(&text(Some(label)))));
        }
    }
    html
}

// ------------------------------------------------------------------ fields

/// A field as an array with all its settings, the way the render functions see `$field`.
#[derive(Debug, Clone)]
pub struct RenderField {
    pub items: Vec<(Key, Value)>,
    pub sub_fields: Vec<Field>,
}

impl RenderField {
    pub fn get(&self, name: &str) -> Option<&Value> {
        let k = key(name);
        self.items.iter().find(|(existing, _)| key_eq(existing, &k)).map(|(_, v)| v)
    }

    pub fn set(&mut self, name: &str, value: Value) {
        super::php::set(&mut self.items, name, value);
    }

    fn str(&self, name: &str) -> String {
        text(self.get(name))
    }

    fn truthy(&self, name: &str) -> bool {
        truthy(self.get(name))
    }

    fn kind(&self) -> String {
        self.str("type")
    }

    fn append(&mut self, name: &str, suffix: &str) {
        let current = self.str(name);
        self.set(name, string(&format!("{current}{suffix}")));
    }

    /// `$field['prepend'] !== ''`.
    fn not_blank(&self, name: &str) -> bool {
        !matches!(self.get(name), Some(Value::Str(s)) if s.is_empty())
    }

    /// The setting `name` if it is set (`isset()`).
    fn isset(&self, name: &str) -> Option<Value> {
        self.get(name).filter(|v| isset(Some(v))).cloned()
    }

    /// `acf_get_field()`: the stored field after `acf_validate_field()` (the generic defaults, the wrapper, `_name`),
    /// with the input prefix `acf` and the type's `load_field()`.
    pub fn load(field: &Field, ctx: &Ctx<'_>) -> Self {
        let items = match &field.settings {
            Value::Array(items) => items.clone(),
            _ => Vec::new(),
        };
        let mut f = Self::raw(items, field.sub_fields.clone());
        if f.kind() == "checkbox" && !isset(f.get("custom_choice_button_text")) {
            f.set("custom_choice_button_text", string(&ctx.acf("Add new choice")));
        }
        f.set("prefix", string("acf"));
        f.load_field();
        f
    }

    /// `acf_validate_field()` of a field array (one that does not come from `acf_get_field()`): the generic defaults,
    /// those of the type, the wrapper and `_name`.
    pub fn raw(mut items: Vec<(Key, Value)>, sub_fields: Vec<Field>) -> Self {
        let generic: [(&str, Value); 16] = [
            ("ID", Value::Int(0)),
            ("key", string("")),
            ("label", string("")),
            ("name", string("")),
            ("aria-label", string("")),
            ("prefix", string("")),
            ("type", string("text")),
            ("value", Value::Null),
            ("menu_order", Value::Int(0)),
            ("instructions", string("")),
            ("required", Value::Bool(false)),
            ("id", string("")),
            ("class", string("")),
            ("conditional_logic", Value::Bool(false)),
            ("parent", Value::Int(0)),
            ("wrapper", Value::Array(Vec::new())),
        ];
        let mut out: Vec<(Key, Value)> = generic.iter().map(|(k, v)| (key(k), v.clone())).collect();
        for (k, v) in items.drain(..) {
            match out.iter_mut().find(|(existing, _)| key_eq(existing, &k)) {
                Some(slot) => slot.1 = v,
                None => out.push((k, v)),
            }
        }
        let mut f = Self { items: out, sub_fields };
        for (name, default) in super::fields::type_defaults(&f.kind()) {
            if !isset(f.get(name)) {
                f.set(name, default);
            }
        }
        let wrapper = super::php::parse_args(f.get("wrapper").unwrap_or(&Value::Null), Value::Array(vec![(key("width"), string("")), (key("class"), string("")), (key("id"), string(""))]));
        f.set("wrapper", wrapper);
        let name = f.get("name").cloned().unwrap_or(Value::Null);
        f.set("_name", name);
        f.set("_valid", Value::Int(1));
        f
    }

    /// The types' `load_field()` (`acf/load_field`).
    fn load_field(&mut self) {
        let f = self;
        match f.kind().as_str() {
            "accordion" => {
                f.set("name", string(""));
                f.set("required", Value::Int(0));
                f.set("value", Value::Bool(false));
            }
            "message" | "tab" => {
                f.set("name", string(""));
                f.set("instructions", string(""));
                f.set("required", Value::Int(0));
                f.set("value", Value::Bool(false));
            }
            kind @ ("date_picker" | "date_time_picker" | "time_picker") => {
                let (display, ret) = match kind {
                    "date_picker" => ("d/m/Y", "d/m/Y"),
                    "date_time_picker" => ("d/m/Y g:i a", "d/m/Y g:i a"),
                    _ => ("g:i a", "g:i a"),
                };
                if !f.truthy("display_format") {
                    f.set("display_format", string(display));
                }
                if !f.truthy("return_format") {
                    f.set("return_format", string(ret));
                }
            }
            _ => {}
        }
    }

    /// `acf_prepare_field()`: the input name from the key and prefix, the id from the name.
    fn prepare(&mut self) {
        if self.truthy("_prepare") {
            return;
        }
        if self.truthy("key") {
            let k = self.get("key").cloned().unwrap_or(Value::Null);
            self.set("name", k);
        }
        if self.truthy("prefix") {
            let name = format!("{}[{}]", self.str("prefix"), self.str("name"));
            self.set("name", string(&name));
        }
        let id = idify(&self.str("name"));
        self.set("id", string(&id));
        self.set("_prepare", Value::Bool(true));
        // `acf_field_oembed::prepare_field()`.
        if self.kind() == "oembed" {
            if !self.truthy("width") {
                self.set("width", Value::Int(640));
            }
            if !self.truthy("height") {
                self.set("height", Value::Int(390));
            }
        }
    }
}

/// `acf_get_field_label()`.
fn field_label(field: &RenderField) -> String {
    let mut label = esc_html(&field.str("label"));
    if field.truthy("required") {
        label.push_str(" <span class=\"acf-required\">*</span>");
    }
    label
}

/// `acf_render_field_label()`.
fn render_label(field: &RenderField) -> String {
    let label = field_label(field);
    if !truthy(Some(&string(&label))) {
        return String::new();
    }
    let id = field.str("id");
    if matches!(field.kind().as_str(), "radio" | "checkbox" | "taxonomy" | "button_group") && field.truthy("id") {
        format!("<label id=\"{}-label\">{}</label>", esc_attr(&id), kses(&label))
    } else {
        format!("<label{}>{}</label>", if field.truthy("id") { format!(" for=\"{}\"", esc_attr(&id)) } else { String::new() }, kses(&label))
    }
}

/// `acf_render_field_instructions()`.
fn render_instructions(field: &RenderField, tooltip: bool) -> String {
    if field.truthy("instructions") {
        let instructions = esc_html_acf(field.get("instructions").unwrap_or(&Value::Null));
        if tooltip {
            format!("<div class=\"acf-tip\"><i tabindex=\"0\" class=\"acf-icon acf-icon-help acf-js-tooltip\" title=\"{}\">?</i></div>", esc_attr(&instructions))
        } else {
            format!("<p class=\"description\">{instructions}</p>")
        }
    } else if field.truthy("hint") {
        format!("<p class=\"description\">{}</p>", esc_html_acf(field.get("hint").unwrap_or(&Value::Null)))
    } else {
        String::new()
    }
}

/// `acf_render_field_wrap( $field, $element, $instruction )`.
pub fn render_field_wrap<'a>(ctx: &'a Ctx<'a>, mut field: RenderField, element: &'a str, instruction: &'a str) -> BoxFuture<'a, Result<String>> {
    Box::pin(async move {
        field.prepare();
        let (element, inner) = match element {
            "div" => ("div", "div"),
            "tr" => ("tr", "td"),
            "td" => ("td", "div"),
            "ul" => ("ul", "li"),
            "ol" => ("ol", "li"),
            "dl" => ("dl", "dt"),
            _ => ("div", "div"),
        };
        let kind = field.kind();
        let mut wrapper = Attrs::new(&[
            ("id", string("")),
            ("class", string("acf-field")),
            ("width", string("")),
            ("style", string("")),
            ("data-name", field.get("_name").cloned().unwrap_or(Value::Null)),
            ("data-type", string(&kind)),
            ("data-key", field.get("key").cloned().unwrap_or(Value::Null)),
        ]);
        wrapper.append("class", &format!(" acf-field-{kind}"));
        if field.truthy("key") {
            wrapper.append("class", &format!(" acf-field-{}", field.str("key")));
        }
        if field.truthy("required") {
            wrapper.append("class", " is-required");
            wrapper.set("data-required", Value::Int(1));
        }
        if let Some(Value::Array(data)) = field.get("data").filter(|v| truthy(Some(v))) {
            for (name, attr) in data {
                wrapper.set(&format!("data-{}", key_text(name)), attr.clone());
            }
        }
        let class = text(wrapper.get("class")).replace('_', "-").replace("field-field-", "field-");
        wrapper.set("class", string(&class));
        // `acf_merge_attributes()` with the `wrapper` setting.
        if let Some(Value::Array(extra)) = field.get("wrapper").filter(|v| truthy(Some(v))) {
            let before = wrapper.clone();
            for (k, v) in extra {
                wrapper.set(&key_text(k), v.clone());
            }
            for name in ["class", "style"] {
                if let (Some(a), Some(b)) = (before.get(name).filter(|v| isset(Some(v))), extra.iter().find(|(k, _)| key_text(k) == name).map(|(_, v)| v).filter(|v| isset(Some(v)))) {
                    wrapper.set(name, string(&format!("{} {}", php_trim(&text(Some(a))), php_trim(&text(Some(b))))));
                }
            }
        }
        let width = wrapper.remove("width").unwrap_or(Value::Null);
        if truthy(Some(&width)) {
            let width = numval(&width);
            if element != "tr" && element != "td" {
                let w = text(Some(&width));
                wrapper.set("data-width", width);
                wrapper.append("style", &format!(" width:{w}%;"));
            }
        }
        // `array_map( 'trim' )` and `array_filter()`.
        let mut wrapper = Attrs(wrapper.0.into_iter().map(|(k, v)| (k, string(php_trim(&text(Some(&v)))))).filter(|(_, v)| truthy(Some(v))).collect());
        if field.truthy("conditional_logic") {
            wrapper.set("data-conditions", field.get("conditional_logic").cloned().unwrap_or(Value::Null));
        }
        if field.truthy("conditions") {
            wrapper.set("data-conditions", field.get("conditions").cloned().unwrap_or(Value::Null));
        }
        let mut out = format!("<{element} {}>\n", esc_attrs(&wrapper));
        if element != "td" {
            out.push_str(&format!("<{inner} class=\"acf-label\">\n"));
            out.push_str(&render_label(&field));
            if instruction == "label" {
                out.push_str(&render_instructions(&field, false));
            }
            out.push_str(&format!("</{inner}>\n"));
        }
        out.push_str(&format!("<{inner} class=\"acf-input\">\n"));
        out.push_str(&render_field(ctx, field.clone()).await?);
        if instruction == "field" {
            out.push_str(&render_instructions(&field, false));
        }
        out.push_str(&format!("</{inner}>\n"));
        out.push_str(&format!("</{element}>\n"));
        Ok(out)
    })
}

/// `acf_render_field()`: the type's `render_field()`.
async fn render_field(ctx: &Ctx<'_>, mut field: RenderField) -> Result<String> {
    field.prepare();
    Ok(match field.kind().as_str() {
        "text" | "password" => render_text(field),
        "textarea" => render_textarea(field),
        "number" => render_number(field),
        "range" => render_range(field),
        "email" => render_email(field),
        "url" => render_url(field),
        "true_false" => render_true_false(ctx, field),
        "select" => render_select(ctx, field, "select"),
        "checkbox" => render_checkbox(ctx, field),
        "radio" => render_radio(field),
        "button_group" => render_button_group(field),
        "color_picker" => render_color_picker(field),
        "date_picker" => render_date_picker(ctx, field),
        "date_time_picker" => render_date_time_picker(ctx, field),
        "time_picker" => render_time_picker(ctx, field),
        "message" => render_message(ctx, field),
        "link" => render_link(ctx, field),
        "oembed" => render_oembed(ctx, field),
        "google_map" => render_google_map(ctx, field),
        "accordion" => render_accordion(field),
        "tab" => render_tab(field),
        "group" => render_group(ctx, field).await?,
        "image" => render_image(ctx, field).await?,
        "file" => render_file(ctx, field).await?,
        "post_object" => render_post_object(ctx, field).await?,
        "page_link" => render_page_link(ctx, field).await?,
        "user" => render_user(ctx, field).await?,
        "taxonomy" => render_taxonomy(ctx, field).await?,
        "relationship" => render_relationship(ctx, field).await?,
        "icon_picker" => render_icon_picker(ctx, field).await?,
        "wysiwyg" => render_wysiwyg(ctx, field),
        _ => String::new(),
    })
}

/// The settings `keys` that are set, and `keys2` as `name="name"` when not empty.
fn pick(field: &RenderField, keys: &[&str], keys2: &[&str]) -> Attrs {
    let mut atts = Attrs::default();
    for k in keys {
        if let Some(v) = field.isset(k) {
            atts.set(k, v);
        }
    }
    for k in keys2 {
        if field.truthy(k) {
            atts.set(k, string(k));
        }
    }
    atts
}

/// The prepend and append boxes of text-like inputs; the input's class says they are there.
fn affixes(field: &mut RenderField) -> String {
    let mut html = String::new();
    if field.not_blank("prepend") {
        field.append("class", " acf-is-prepended");
        html.push_str(&format!("<div class=\"acf-input-prepend\">{}</div>", esc_html_acf(field.get("prepend").unwrap_or(&Value::Null))));
    }
    if field.not_blank("append") {
        field.append("class", " acf-is-appended");
        html.push_str(&format!("<div class=\"acf-input-append\">{}</div>", esc_html_acf(field.get("append").unwrap_or(&Value::Null))));
    }
    html
}

/// `acf_field_text::render_field()` (also the password field's).
fn render_text(mut field: RenderField) -> String {
    let mut html = affixes(&mut field);
    let mut input = Attrs::default();
    for k in ["type", "id", "class", "name", "value", "placeholder", "maxlength", "pattern", "readonly", "disabled", "required"] {
        if let Some(v) = field.isset(k) {
            input.set(k, v);
        }
    }
    if let Some(Value::Array(data)) = field.get("input-data") {
        for (name, attr) in data {
            input.set(&format!("data-{}", key_text(name)), attr.clone());
        }
    }
    html.push_str(&format!("<div class=\"acf-input-wrap\">{}</div>", text_input(filter_attrs(&input))));
    html
}

/// `acf_field_textarea::render_field()`.
fn render_textarea(mut field: RenderField) -> String {
    if !field.truthy("rows") {
        field.set("rows", Value::Int(8));
    }
    textarea_input(filter_attrs(&pick(&field, &["id", "class", "name", "value", "placeholder", "rows", "maxlength"], &["readonly", "disabled", "required"])))
}

/// `acf_field_number::render_field()`.
fn render_number(mut field: RenderField) -> String {
    if !field.truthy("step") {
        field.set("step", string("any"));
    }
    let mut html = affixes(&mut field);
    let atts = filter_attrs(&pick(&field, &["type", "id", "class", "name", "value", "min", "max", "step", "placeholder", "pattern"], &["readonly", "disabled", "required"]));
    html.push_str(&format!("<div class=\"acf-input-wrap\">{}</div>", text_input(atts)));
    html
}

/// PHP's `max()`/`min()` of two values compared as numbers: the first one when they are equal.
fn pick_number(a: &Value, b: &Value, larger: bool) -> Value {
    let (x, y) = (as_f64(a), as_f64(b));
    let take_b = if larger { y > x } else { y < x };
    if take_b { b.clone() } else { a.clone() }
}

/// `acf_field_range::render_field()`.
fn render_range(mut field: RenderField) -> String {
    if !field.truthy("step") {
        field.set("step", Value::Int(1));
    }
    if !field.truthy("min") {
        field.set("min", Value::Int(0));
    }
    if !field.truthy("max") {
        field.set("max", Value::Int(100));
    }
    if !field.get("value").is_some_and(is_numeric) {
        field.set("value", Value::Int(0));
    }
    let value = pick_number(field.get("value").unwrap_or(&Value::Null), field.get("min").unwrap_or(&Value::Null), true);
    let value = pick_number(&value, field.get("max").unwrap_or(&Value::Null), false);
    field.set("value", value);
    let atts = filter_attrs(&pick(&field, &["type", "id", "class", "name", "value", "min", "max", "step"], &["readonly", "disabled", "required"]));
    let mut html = "<div class=\"acf-range-wrap\">".to_owned();
    if field.not_blank("prepend") {
        html.push_str(&format!("<div class=\"acf-prepend\">{}</div>", esc_html_acf(field.get("prepend").unwrap_or(&Value::Null))));
    }
    html.push_str(&text_input(atts.clone()));
    let mut len = field.str("min").len().max(field.str("max").len()) as f64;
    let step = atts.get("step").cloned().unwrap_or(Value::Null);
    if as_f64(&step) < 1.0 {
        len += field.str("step").len() as f64 - 1.5;
    }
    html.push_str(&text_input(Attrs::new(&[
        ("type", string("number")),
        ("id", string(&format!("{}-alt", text(atts.get("id"))))),
        ("value", atts.get("value").cloned().unwrap_or(Value::Null)),
        ("step", step),
        ("style", string(&format!("width: {}em;", float_echo(1.8 + len * 0.7)))),
    ])));
    if field.not_blank("append") {
        html.push_str(&format!("<div class=\"acf-append\">{}</div>", esc_html_acf(field.get("append").unwrap_or(&Value::Null))));
    }
    html.push_str("</div>");
    html
}

/// `acf_field_email::render_field()`.
fn render_email(mut field: RenderField) -> String {
    let mut html = affixes(&mut field);
    let atts = filter_attrs(&pick(&field, &["type", "id", "class", "name", "value", "placeholder", "pattern"], &["readonly", "disabled", "required", "multiple"]));
    html.push_str(&format!("<div class=\"acf-input-wrap\">{}</div>", text_input(atts)));
    html
}

/// `acf_field_url::render_field()`.
fn render_url(field: RenderField) -> String {
    let atts = filter_attrs(&pick(&field, &["type", "id", "class", "name", "value", "placeholder", "pattern"], &["readonly", "disabled", "required"]));
    format!("<div class=\"acf-input-wrap acf-url\"><i class=\"acf-icon -globe -small\"></i>{}</div>", text_input(atts))
}

/// `acf_field_true_false::render_field()`.
fn render_true_false(ctx: &Ctx<'_>, mut field: RenderField) -> String {
    let mut input = Attrs::new(&[
        ("type", string("checkbox")),
        ("id", field.get("id").cloned().unwrap_or(Value::Null)),
        ("name", field.get("name").cloned().unwrap_or(Value::Null)),
        ("value", string("1")),
        ("class", field.get("class").cloned().unwrap_or(Value::Null)),
        ("autocomplete", string("off")),
    ]);
    let hidden = Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", Value::Int(0))]);
    let active = field.truthy("value");
    let mut switch = String::new();
    if active {
        input.set("checked", string("checked"));
    }
    if field.truthy("ui") {
        if matches!(field.get("ui_on_text"), Some(Value::Str(s)) if s.is_empty()) {
            field.set("ui_on_text", string(&ctx.acf("Yes")));
        }
        if matches!(field.get("ui_off_text"), Some(Value::Str(s)) if s.is_empty()) {
            field.set("ui_off_text", string(&ctx.acf("No")));
        }
        input.append("class", " acf-switch-input");
        switch.push_str(&format!("<div class=\"acf-switch{}\">", if active { " -on" } else { "" }));
        switch.push_str(&format!("<span class=\"acf-switch-on\">{}</span>", field.str("ui_on_text")));
        switch.push_str(&format!("<span class=\"acf-switch-off\">{}</span>", field.str("ui_off_text")));
        switch.push_str("<div class=\"acf-switch-slider\"></div>");
        switch.push_str("</div>");
    }
    let switch = if switch.is_empty() { String::new() } else { kses(&switch) };
    let message = if field.truthy("message") { format!("\t\t\t\t<span class=\"message\">{}</span>", esc_html_acf(field.get("message").unwrap_or(&Value::Null))) } else { String::new() };
    format!("<div class=\"acf-true-false\">\n\t\t\t{}\t<label>\n\t\t<input {}/>\n\t\t\t{switch}\t\t\t{message}\t</label>\n</div>\n\t\t\t", hidden_input(&hidden), esc_attrs(&input))
}

/// `acf_field_select::render_field()`; `name` is the type's name for the nonce of its AJAX requests.
fn render_select(ctx: &Ctx<'_>, mut field: RenderField, name: &str) -> String {
    let mut value = get_array(field.get("value"));
    let mut choices = get_array(field.get("choices"));
    if !field.truthy("placeholder") {
        field.set("placeholder", string(&ctx.acfx("Select", "verb")));
    }
    if value.is_empty() {
        value = vec![(Key::Int(0), string(""))];
    }
    if field.truthy("allow_null") && !field.truthy("multiple") {
        // `array( '' => ... ) + $choices`: the empty key once, first.
        let placeholder = format!("- {} -", field.str("placeholder"));
        let rest: Vec<(Key, Value)> = choices.into_iter().filter(|(k, _)| !key_eq(k, &key(""))).collect();
        choices = std::iter::once((key(""), string(&placeholder))).chain(rest).collect();
    }
    if field.truthy("ui") && field.truthy("ajax") {
        let mut minimal: Vec<(Key, Value)> = Vec::new();
        for (_, v) in &value {
            let k = super::php::key(&text(Some(v)));
            if let Some((ck, cv)) = choices.iter().find(|(ck, cv)| key_eq(ck, &k) && isset(Some(cv))) {
                match minimal.iter_mut().find(|(mk, _)| key_eq(mk, ck)) {
                    Some(slot) => slot.1 = cv.clone(),
                    None => minimal.push((ck.clone(), cv.clone())),
                }
            }
        }
        choices = minimal;
    }
    let mut select = Attrs::new(&[
        ("id", field.get("id").cloned().unwrap_or(Value::Null)),
        ("class", field.get("class").cloned().unwrap_or(Value::Null)),
        ("name", field.get("name").cloned().unwrap_or(Value::Null)),
        ("data-ui", field.get("ui").cloned().unwrap_or(Value::Null)),
        ("data-ajax", field.get("ajax").cloned().unwrap_or(Value::Null)),
        ("data-multiple", field.get("multiple").cloned().unwrap_or(Value::Null)),
        ("data-placeholder", field.get("placeholder").cloned().unwrap_or(Value::Null)),
        ("data-allow_null", field.get("allow_null").cloned().unwrap_or(Value::Null)),
    ]);
    if field.truthy("aria-label") {
        select.set("aria-label", field.get("aria-label").cloned().unwrap_or(Value::Null));
    }
    if field.truthy("multiple") {
        select.set("multiple", string("multiple"));
        select.set("size", Value::Int(5));
        select.append("name", "[]");
        if field.truthy("ui") {
            select.set("size", Value::Int(1));
        }
    }
    if field.truthy("create_options") && field.truthy("ui") {
        select.set("data-create_options", Value::Bool(true));
    }
    if field.truthy("readonly") {
        select.set("readonly", string("readonly"));
    }
    if field.truthy("disabled") {
        select.set("disabled", string("disabled"));
    }
    if field.truthy("ajax_action") {
        select.set("data-ajax_action", field.get("ajax_action").cloned().unwrap_or(Value::Null));
    }
    if field.truthy("nonce") {
        select.set("data-nonce", field.get("nonce").cloned().unwrap_or(Value::Null));
    }
    if field.truthy("ajax") && !field.truthy("nonce") && field.str("key").starts_with("field_") && matches!(field.get("key"), Some(Value::Str(_))) {
        select.set("data-nonce", string(&(ctx.nonce)(&format!("acf_field_{name}_{}", field.str("key")))));
    }
    if field.truthy("hide_search") {
        select.set("data-minimum-results-for-search", string("-1"));
    }
    let mut html = String::new();
    if field.truthy("multiple") || field.truthy("ui") {
        html.push_str(&hidden_input(&Attrs::new(&[("id", string(&format!("{}-input", field.str("id")))), ("name", field.get("name").cloned().unwrap_or(Value::Null))])));
    }
    let mut all_choices = choices;
    if field.truthy("create_options") && field.truthy("ui") && let Some(Value::Array(values)) = field.get("value") {
        let field_choices = get_array(field.get("choices"));
        for (_, v) in values {
            let k = key(&text(Some(v)));
            if field_choices.iter().any(|(ck, cv)| key_eq(ck, &k) && isset(Some(cv))) {
                continue;
            }
            let option = esc_attr(&text(Some(v)));
            let ok = key(&option);
            match all_choices.iter_mut().find(|(ck, _)| key_eq(ck, &ok)) {
                Some(slot) => slot.1 = string(&option),
                None => all_choices.push((ok, string(&option))),
            }
        }
    }
    select.set("value", Value::Array(value));
    select.set("choices", Value::Array(all_choices));
    html.push_str(&select_input(select));
    html
}

/// The values of a list as their `esc_attr()` texts.
fn esc_values(items: &[(Key, Value)]) -> Vec<Value> {
    items.iter().map(|(_, v)| string(&esc_attr(&text(Some(v))))).collect()
}

/// `acf_field_checkbox::walk()`: the list items; `values` holds the escaped values that were seen, `all_checked`
/// whether every choice is checked.
fn walk_checkboxes(choices: &[(Key, Value)], field: &RenderField, selected: &[Value], disabled: &[Value], seen: &mut Vec<Value>, all_checked: &mut bool) -> String {
    let mut html = String::new();
    for (k, label) in choices {
        html.push_str("<li>");
        if let Value::Array(items) = label {
            html.push_str("<ul>\n");
            html.push_str(&walk_checkboxes(items, field, selected, disabled, seen, all_checked));
            html.push_str("</ul>");
        } else {
            let esc_value = string(&esc_attr(&key_text(k)));
            let mut atts = Attrs::new(&[
                ("id", string(&format!("{}-{}", field.str("id"), key_text(k).replace(' ', "-")))),
                ("type", string("checkbox")),
                ("name", field.get("name").cloned().unwrap_or(Value::Null)),
                ("value", key_value(k)),
                ("label", label.clone()),
            ]);
            if in_array(&esc_value, selected) {
                atts.set("checked", string("checked"));
            } else {
                *all_checked = false;
            }
            if in_array(&esc_value, disabled) {
                atts.set("disabled", string("disabled"));
            }
            seen.push(esc_value);
            html.push_str(&checkbox_input(atts));
        }
        html.push_str("</li>\n");
    }
    html
}

/// `acf_field_checkbox::render_field()`.
fn render_checkbox(ctx: &Ctx<'_>, mut field: RenderField) -> String {
    let value = get_array(field.get("value"));
    let choices = get_array(field.get("choices"));
    let mut html = hidden_input(&Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null))]));
    let mut li = String::new();
    let mut ul = Attrs::new(&[("class", string("acf-checkbox-list")), ("role", string("group"))]);
    if field.truthy("id") {
        ul.set("aria-labelledby", string(&format!("{}-label", field.str("id"))));
    }
    ul.append("class", if field.str("layout") == "horizontal" { " acf-hl" } else { " acf-bl" });
    ul.append("class", &format!(" {}", field.str("class")));
    field.append("name", "[]");
    let selected = esc_values(&value);
    let disabled = match field.get("disabled") {
        Some(Value::Array(items)) => esc_values(items),
        _ => Vec::new(),
    };
    let mut seen = Vec::new();
    let mut all_checked = true;
    if !choices.is_empty() {
        li.push_str(&walk_checkboxes(&choices, &field, &selected, &disabled, &mut seen, &mut all_checked));
        if field.truthy("toggle") {
            let mut atts = Attrs::new(&[("type", string("checkbox")), ("class", string("acf-checkbox-toggle")), ("label", string(&ctx.acf("Toggle All")))]);
            if let Some(Value::Str(label)) = field.get("toggle") {
                atts.set("label", Value::Str(label.clone()));
            }
            if all_checked {
                atts.set("checked", string("checked"));
            }
            li = format!("<li>{}</li>\n{li}", checkbox_input(atts));
        }
    }
    if field.truthy("allow_custom") {
        for (_, v) in &value {
            let k = key(&text(Some(v)));
            if choices.iter().any(|(ck, cv)| key_eq(ck, &k) && isset(Some(cv))) {
                continue;
            }
            let esc_value = string(&esc_attr(&text(Some(v))));
            if in_array(&esc_value, &seen) {
                continue;
            }
            li.push_str(&format!(
                "<li><input class=\"acf-checkbox-custom\" type=\"checkbox\" checked=\"checked\" />{}</li>\n",
                text_input(Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", v.clone())]))
            ));
        }
        li.push_str(&format!("<li><a href=\"#\" class=\"button acf-add-checkbox\">{}</a></li>\n", esc_attr(&field.str("custom_choice_button_text"))));
    }
    html.push_str(&format!("<ul {}>\n{li}</ul>\n", esc_attrs(&ul)));
    html
}

/// `acf_field_radio::render_field()`.
fn render_radio(mut field: RenderField) -> String {
    let mut ul = Attrs::new(&[
        ("class", string("acf-radio-list")),
        ("data-allow_null", field.get("allow_null").cloned().unwrap_or(Value::Null)),
        ("data-other_choice", field.get("other_choice").cloned().unwrap_or(Value::Null)),
        ("role", string("radiogroup")),
    ]);
    if field.truthy("id") {
        ul.set("aria-labelledby", string(&format!("{}-label", field.str("id"))));
    }
    ul.append("class", if field.str("layout") == "horizontal" { " acf-hl" } else { " acf-bl" });
    ul.append("class", &format!(" {}", field.str("class")));
    let value = field.str("value");
    let mut choices = get_array(field.get("choices")).into_iter().filter(|_| true).collect::<Vec<_>>();
    if !matches!(field.get("choices"), Some(Value::Array(_))) {
        choices = Vec::new();
    }
    let has = |choices: &[(Key, Value)], k: &str| choices.iter().any(|(ck, cv)| key_eq(ck, &key(k)) && isset(Some(cv)));
    let checked = if has(&choices, &value) {
        value.clone()
    } else if field.truthy("other_choice") && !value.is_empty() {
        "other".to_owned()
    } else if field.truthy("allow_null") {
        String::new()
    } else {
        choices.first().map(|(k, _)| key_text(k)).unwrap_or_default()
    };
    let mut other_input: Option<Attrs> = None;
    if field.truthy("other_choice") {
        let mut other = Attrs::new(&[("type", string("text")), ("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", string("")), ("disabled", string("disabled")), ("class", string("acf-disabled"))]);
        if checked == "other" {
            other.remove("disabled");
            other.set("value", field.get("value").cloned().unwrap_or(Value::Null));
        }
        other_input = Some(other);
        if !has(&choices, "other") {
            match choices.iter_mut().find(|(k, _)| key_eq(k, &key("other"))) {
                Some(slot) => slot.1 = string(""),
                None => choices.push((key("other"), string(""))),
            }
        }
        field.set("choices", Value::Array(choices.clone()));
    }
    if choices.is_empty() {
        return String::new();
    }
    let mut e = hidden_input(&Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null))]));
    e.push_str(&format!("<ul {}>", esc_attrs(&ul)));
    let disabled = field.get("disabled").cloned();
    for (k, label) in &choices {
        let value = key_text(k);
        let mut attrs = Attrs::new(&[
            ("type", string("radio")),
            ("id", string(&rustpress_core::slug::sanitize_title(&format!("{}-{value}", field.str("id"))))),
            ("name", field.get("name").cloned().unwrap_or(Value::Null)),
            ("value", string(&value)),
        ]);
        let selected = esc_attr(&value) == esc_attr(&checked);
        if selected {
            attrs.set("checked", string("checked"));
        }
        if let Some(Value::Array(items)) = &disabled
            && in_array(&string(&value), &items.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>())
        {
            attrs.set("disabled", string("disabled"));
        }
        let additional = match (&other_input, value.as_str()) {
            (Some(other), "other") => format!(" {}", text_input(other.clone())),
            _ => String::new(),
        };
        e.push_str(&format!("<li><label{}><input {}/>{}</label>{additional}</li>", if selected { " class=\"selected\"" } else { "" }, esc_attrs(&attrs), esc_html_acf(label)));
    }
    e.push_str("</ul>");
    e
}

/// `acf_field_button_group::render_field()`.
fn render_button_group(field: RenderField) -> String {
    let value = esc_attr(&field.str("value"));
    let choices = match field.get("choices") {
        Some(Value::Array(items)) if !items.is_empty() => items.clone(),
        _ => return String::new(),
    };
    let mut selected = false;
    let mut buttons: Vec<Attrs> = Vec::new();
    for (k, label) in &choices {
        let checked = value == esc_attr(&key_text(k));
        selected |= checked;
        buttons.push(Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", key_value(k)), ("label", label.clone()), ("checked", Value::Bool(checked)), ("button_group", Value::Bool(true))]));
    }
    if !field.truthy("allow_null") && !selected {
        buttons[0].set("checked", Value::Bool(true));
    }
    if field.truthy("allow_null") && !selected {
        buttons[0].set("tabindex", string("0"));
    }
    let mut div = Attrs::new(&[("class", string("acf-button-group")), ("role", string("radiogroup"))]);
    if field.truthy("id") {
        div.set("aria-labelledby", string(&format!("{}-label", field.str("id"))));
    }
    if field.str("layout") == "vertical" {
        div.append("class", " -vertical");
    }
    if field.truthy("class") {
        div.append("class", &format!(" {}", field.str("class")));
    }
    if field.truthy("allow_null") {
        div.set("data-allow_null", Value::Int(1));
    }
    let mut html = hidden_input(&Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null))]));
    html.push_str(&format!("<div {}>", esc_attrs(&div)));
    for mut button in buttons {
        if truthy(button.get("checked")) {
            button.set("checked", string("checked"));
        } else {
            button.remove("checked");
        }
        html.push_str(&radio_input(button));
    }
    html.push_str("</div>");
    html
}

/// `acf_field_color_picker::render_field()`. A block theme has `theme.json`; the palette `themejson` (the colors of
/// the theme's palette) is not read.
fn render_color_picker(field: RenderField) -> String {
    let mut text_attrs = Attrs::default();
    for k in ["id", "class", "name", "value"] {
        text_attrs.set(k, field.get(k).cloned().unwrap_or(Value::Null));
    }
    let hidden = Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", field.get("value").cloned().unwrap_or(Value::Null))]);
    text_attrs.set("data-alpha-skip-debounce", Value::Bool(true));
    if field.truthy("enable_opacity") {
        text_attrs.set("data-alpha-enabled", Value::Bool(true));
    }
    let source = field.str("custom_palette_source");
    if source == "themejson" {
        text_attrs.set("data-acf-palette-type", string("custom"));
        text_attrs.set("data-acf-palette-colors", string(""));
    } else if (source == "custom" || source.is_empty()) && field.truthy("palette_colors") {
        text_attrs.set("data-acf-palette-type", string("custom"));
        text_attrs.set("data-acf-palette-colors", field.get("palette_colors").cloned().unwrap_or(Value::Null));
    } else {
        text_attrs.set("data-acf-palette-type", string("default"));
    }
    let wheel = if field.truthy("show_color_wheel") { "" } else { " acf-hide-color-picker-color-wheel" };
    format!("\t\t<div class=\"acf-color-picker{}\">\n\t\t\t{}\t\t\t{}\t\t</div>\n\t\t\t", esc_attr(wheel), hidden_input(&hidden), text_input(text_attrs))
}

/// `acf_str_replace()` with ACF's table of PHP to JavaScript date (or time) characters.
fn convert_format(format: &str, table: &[(&str, &str)]) -> String {
    let mut out = format.to_owned();
    let mut ignore: Vec<&str> = Vec::new();
    for (search, replace) in table {
        if ignore.contains(search) || !out.contains(search) {
            continue;
        }
        out = out.replace(search, replace);
        ignore.push(replace);
    }
    out
}

const DATE_TO_JS: [(&str, &str); 11] = [("Y", "yy"), ("y", "y"), ("m", "mm"), ("n", "m"), ("F", "MM"), ("M", "M"), ("l", "DD"), ("D", "D"), ("d", "dd"), ("j", "d"), ("S", "")];
const TIME_TO_JS: [(&str, &str); 8] = [("a", "tt"), ("A", "TT"), ("h", "hh"), ("g", "h"), ("H", "HH"), ("G", "H"), ("i", "mm"), ("s", "ss")];

/// `acf_split_date_time()`.
fn split_date_time(format: &str) -> (String, String) {
    let (mut date, mut time) = (String::new(), String::new());
    let mut is_time = false;
    for c in format.chars() {
        let s = c.to_string();
        if DATE_TO_JS.iter().any(|(k, _)| *k == s) {
            is_time = false;
        } else if TIME_TO_JS.iter().any(|(k, _)| *k == s) {
            is_time = true;
        }
        if is_time { time.push(c) } else { date.push(c) }
    }
    (php_trim(&date).to_owned(), php_trim(&time).to_owned())
}

fn format_date(ctx: &Ctx<'_>, value: &Value, format: &str) -> Value {
    super::format::format_date(ctx.site, value.clone(), format)
}

/// The readonly/disabled settings on both inputs of a picker.
fn picker_states(field: &RenderField, hidden: &mut Attrs, text_attrs: &mut Attrs) {
    for k in ["readonly", "disabled"] {
        if field.truthy(k) {
            hidden.set(k, string(k));
            text_attrs.set(k, string(k));
        }
    }
}

fn picker(div: &Attrs, hidden: &Attrs, text_attrs: Attrs) -> String {
    format!("\t\t<div {}>\n\t\t\t{}\t\t\t{}\t\t</div>\n\t\t\t", esc_attrs(div), hidden_input(hidden), text_input(text_attrs))
}

/// `acf_field_date_picker::render_field()`.
fn render_date_picker(ctx: &Ctx<'_>, field: RenderField) -> String {
    let value = field.get("value").cloned().unwrap_or(Value::Null);
    let (hidden_value, display_value) = if truthy(Some(&value)) { (format_date(ctx, &value, "Ymd"), format_date(ctx, &value, &field.str("display_format"))) } else { (string(""), string("")) };
    let mut div = Attrs::new(&[
        ("class", string("acf-date-picker acf-input-wrap")),
        ("data-date_format", string(&convert_format(&field.str("display_format"), &DATE_TO_JS))),
        ("data-first_day", field.get("first_day").cloned().unwrap_or(Value::Null)),
    ]);
    let mut hidden = Attrs::new(&[("id", field.get("id").cloned().unwrap_or(Value::Null)), ("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", hidden_value)]);
    let mut text_attrs = Attrs::new(&[
        ("class", string(&format!("{} input", field.str("class")))),
        ("value", display_value),
        ("data-default-to-today", field.get("default_to_current_date").cloned().unwrap_or(Value::Null)),
    ]);
    picker_states(&field, &mut hidden, &mut text_attrs);
    if field.truthy("save_format") {
        div.set("data-save_format", field.get("save_format").cloned().unwrap_or(Value::Null));
        hidden.set("value", value);
        text_attrs.set("value", string(""));
    }
    picker(&div, &hidden, text_attrs)
}

/// `acf_field_date_and_time_picker::render_field()`.
fn render_date_time_picker(ctx: &Ctx<'_>, field: RenderField) -> String {
    let value = field.get("value").cloned().unwrap_or(Value::Null);
    let display_format = field.str("display_format");
    let (hidden_value, display_value) = if truthy(Some(&value)) { (format_date(ctx, &value, "Y-m-d H:i:s"), format_date(ctx, &value, &display_format)) } else { (string(""), string("")) };
    let (date, time) = split_date_time(&display_format);
    let div = Attrs::new(&[
        ("class", string("acf-date-time-picker acf-input-wrap")),
        ("data-date_format", string(&convert_format(&date, &DATE_TO_JS))),
        ("data-time_format", string(&convert_format(&time, &TIME_TO_JS))),
        ("data-first_day", field.get("first_day").cloned().unwrap_or(Value::Null)),
    ]);
    let mut hidden = Attrs::new(&[("id", field.get("id").cloned().unwrap_or(Value::Null)), ("class", string("input-alt")), ("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", hidden_value)]);
    let mut text_attrs = Attrs::new(&[
        ("class", string(&format!("{} input", field.str("class")))),
        ("value", display_value),
        ("data-default-to-today", field.get("default_to_current_date").cloned().unwrap_or(Value::Null)),
    ]);
    picker_states(&field, &mut hidden, &mut text_attrs);
    picker(&div, &hidden, text_attrs)
}

/// `acf_field_time_picker::render_field()`.
fn render_time_picker(ctx: &Ctx<'_>, field: RenderField) -> String {
    let value = field.get("value").cloned().unwrap_or(Value::Null);
    let display_format = field.str("display_format");
    let display_value = if truthy(Some(&value)) { format_date(ctx, &value, &display_format) } else { string("") };
    let div = Attrs::new(&[("class", string("acf-time-picker acf-input-wrap")), ("data-time_format", string(&convert_format(&display_format, &TIME_TO_JS)))]);
    let mut hidden = Attrs::new(&[("id", field.get("id").cloned().unwrap_or(Value::Null)), ("class", string("input-alt")), ("type", string("hidden")), ("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", value)]);
    let mut text_attrs = Attrs::new(&[("class", string(&format!("{} input", field.str("class")))), ("type", string("text")), ("value", display_value)]);
    picker_states(&field, &mut hidden, &mut text_attrs);
    picker(&div, &hidden, text_attrs)
}

/// `acf_field_message::render_field()`.
fn render_message(ctx: &Ctx<'_>, field: RenderField) -> String {
    let mut m = ctx.site.texturize(&field.str("message"));
    if field.truthy("esc_html") {
        m = esc_html(&m);
    }
    match field.str("new_lines").as_str() {
        "wpautop" => m = rustpress_core::formatting::wpautop(&m),
        "br" => m = super::format::nl2br(&m),
        _ => {}
    }
    kses(&m)
}

/// `acf_field_link::get_link()`: a stored array, or a URL (ACF < 5.6.0).
fn get_link(value: &Value) -> Vec<(Key, Value)> {
    let mut link = vec![(key("title"), string("")), (key("url"), string("")), (key("target"), string(""))];
    match value {
        Value::Array(items) => {
            for (k, v) in items {
                // `array_merge()`: string keys replace, numeric keys are appended.
                match k {
                    Key::Str(_) => super::php::set(&mut link, &key_text(k), v.clone()),
                    Key::Int(_) => link.push((Key::Int(link.iter().filter(|(k, _)| matches!(k, Key::Int(_))).count() as i64), v.clone())),
                }
            }
        }
        Value::Str(_) if !is_numeric(value) => super::php::set(&mut link, "url", value.clone()),
        _ => {}
    }
    link
}

/// `acf_field_link::render_field()`. The media scripts of `acf_enqueue_uploader()` are left to the caller.
fn render_link(ctx: &Ctx<'_>, field: RenderField) -> String {
    let link = get_link(field.get("value").unwrap_or(&Value::Null));
    let get = |name: &str| link.iter().find(|(k, _)| key_eq(k, &key(name))).map(|(_, v)| text(Some(v))).unwrap_or_default();
    let mut div = Attrs::new(&[("id", field.get("id").cloned().unwrap_or(Value::Null)), ("class", string(&format!("{} acf-link", field.str("class"))))]);
    if truthy(link.iter().find(|(k, _)| key_eq(k, &key("url"))).map(|(_, v)| v)) {
        div.append("class", " -value");
    }
    if get("target") == "_blank" {
        div.append("class", " -external");
    }
    let mut hidden = String::new();
    for (k, v) in &link {
        let k = key_text(k);
        hidden.push_str(&format!("\t\t\t\t{}\t\t", hidden_input(&Attrs::new(&[("class", string(&format!("input-{k}"))), ("name", string(&format!("{}[{k}]", field.str("name")))), ("value", v.clone())]))));
    }
    format!(
        "<div {}>\n\t\n\t<div class=\"acf-hidden\">\n\t\t<a class=\"link-node\" href=\"{}\" target=\"{}\">{}</a>\n\t\t\t{hidden}\t</div>\n\t\n\t<a href=\"#\" class=\"button\" data-name=\"add\" target=\"\">{}</a>\n\t\n\t<div class=\"link-wrap\">\n\t\t<span class=\"link-title\">{}</span>\n\t\t<a class=\"link-url\" href=\"{}\" target=\"_blank\">{}</a>\n\t\t<i class=\"acf-icon -link-ext acf-js-tooltip\" title=\"{}\"></i><a class=\"acf-icon -pencil -clear acf-js-tooltip\" data-name=\"edit\" href=\"#\" title=\"{}\"></a><a class=\"acf-icon -cancel -clear acf-js-tooltip\" data-name=\"remove\" href=\"#\" title=\"{}\"></a>\n\t</div>\n\t\n</div>\n\t\t\t",
        esc_attrs(&div),
        esc_url(&get("url")),
        esc_attr(&get("target")),
        esc_html(&get("title")),
        esc_html(&ctx.acf("Select Link")),
        esc_html(&get("title")),
        esc_url(&get("url")),
        esc_html(&get("url")),
        esc_attr(&ctx.acf("Opens in a new window/tab")),
        esc_attr(&ctx.acf("Edit")),
        esc_attr(&ctx.acf("Remove")),
    )
}

/// `acf_field_oembed::render_field()`. Without network access the embed of a value is its link, as
/// `WP_Embed::maybe_make_link()` makes it.
fn render_oembed(ctx: &Ctx<'_>, field: RenderField) -> String {
    let mut atts = Attrs::new(&[("class", string("acf-oembed")), ("data-nonce", string(&(ctx.nonce)(&format!("acf_field_oembed_{}", field.str("key")))))]);
    let value = field.get("value").cloned().unwrap_or(Value::Null);
    if truthy(Some(&value)) {
        atts.append("class", " has-value");
    }
    let hidden = hidden_input(&Attrs::new(&[("class", string("input-value")), ("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", value.clone())]));
    let search = text_input(Attrs::new(&[("class", string("input-search")), ("value", value.clone()), ("placeholder", string(&ctx.acf("Enter URL"))), ("autocomplete", string("off"))]));
    let embed = if truthy(Some(&value)) {
        let url = text(Some(&value));
        format!("<a href=\"{}\">{}</a>", esc_url(&url), esc_html(&url))
    } else {
        String::new()
    };
    format!(
        "<div {}>\n\n\t\t\t{hidden}\n\t<div class=\"title\">\n\t\t\t{search}\t\t<div class=\"acf-actions -hover\">\n\t\t\t<a data-name=\"clear-button\" href=\"#\" class=\"acf-icon -cancel grey\"></a>\n\t\t</div>\n\t</div>\n\n\t<div class=\"canvas\">\n\t\t<div class=\"canvas-media\">\n\t\t\t{embed}\t\t</div>\n\t\t<i class=\"acf-icon -picture hide-if-value\"></i>\n\t</div>\n\n</div>\n\t\t\t",
        esc_attrs(&atts)
    )
}

/// `acf_field_google_map::render_field()`.
fn render_google_map(ctx: &Ctx<'_>, mut field: RenderField) -> String {
    for (k, v) in [("height", "400"), ("center_lat", "-37.81411"), ("center_lng", "144.96328"), ("zoom", "14")] {
        if !field.truthy(k) {
            field.set(k, string(v));
        }
    }
    let mut attrs = Attrs::new(&[
        ("id", field.get("id").cloned().unwrap_or(Value::Null)),
        ("class", string(&format!("acf-google-map {}", field.str("class")))),
        ("data-lat", field.get("center_lat").cloned().unwrap_or(Value::Null)),
        ("data-lng", field.get("center_lng").cloned().unwrap_or(Value::Null)),
        ("data-zoom", field.get("zoom").cloned().unwrap_or(Value::Null)),
    ]);
    let mut search = String::new();
    if field.truthy("value") {
        attrs.append("class", " -value");
        search = text(field.get("value").and_then(|v| v.get("address")));
    } else {
        field.set("value", string(""));
    }
    let hidden = hidden_input(&Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", field.get("value").cloned().unwrap_or(Value::Null))]));
    format!(
        "<div {}>\n\n\t\t\t{hidden}\n\t<div class=\"title\">\n\n\t\t<input class=\"search\" type=\"text\" placeholder=\"{}\" value=\"{}\" />\n\n\t\t<div class=\"acf-actions -hover\">\n\t\t\t<button type=\"button\" data-name=\"search\" class=\"acf-icon -search grey\" aria-label=\"{}\"></button>\n\t\t\t<button type=\"button\" data-name=\"clear\" class=\"acf-icon -cancel grey\" aria-label=\"{}\"></button>\n\t\t\t<button type=\"button\" data-name=\"locate\" class=\"acf-icon -location grey\" aria-label=\"{}\"></button>\n\t\t</div>\n\n\t\t<i class=\"acf-loading\"></i>\n\n\t</div>\n\n\t<div class=\"canvas\" style=\"{}\"></div>\n\n</div>\n\t\t\t",
        esc_attrs(&attrs),
        esc_attr(&ctx.acf("Search for address...")),
        esc_attr(&search),
        esc_attr(&ctx.acf("Search")),
        esc_attr(&ctx.acf("Clear location")),
        esc_attr(&ctx.acf("Find current location")),
        esc_attr(&format!("height: {}px", field.str("height"))),
    )
}

/// `acf_field_accordion::render_field()`.
fn render_accordion(field: RenderField) -> String {
    let atts = Attrs::new(&[
        ("class", string("acf-fields")),
        ("data-open", field.get("open").cloned().unwrap_or(Value::Null)),
        ("data-multi_expand", field.get("multi_expand").cloned().unwrap_or(Value::Null)),
        ("data-endpoint", field.get("endpoint").cloned().unwrap_or(Value::Null)),
    ]);
    format!("\t\t<div {}></div>\n\t\t\t", esc_attrs(&atts))
}

/// `acf_field_tab::render_field()`.
fn render_tab(field: RenderField) -> String {
    let mut atts = Attrs::new(&[
        ("href", string("")),
        ("class", string("acf-tab-button")),
        ("data-placement", field.get("placement").cloned().unwrap_or(Value::Null)),
        ("data-endpoint", field.get("endpoint").cloned().unwrap_or(Value::Null)),
        ("data-key", field.get("key").cloned().unwrap_or(Value::Null)),
        ("data-selected", field.get("selected").cloned().unwrap_or(Value::Null)),
    ]);
    if field.truthy("unique_tab_key") {
        atts.set("data-unique-tab-key", field.get("unique_tab_key").cloned().unwrap_or(Value::Null));
    }
    if field.truthy("class") {
        atts.append("class", &format!(" {}", field.str("class")));
    }
    format!("\t\t\t<a {}>{}</a>\n\t\t\t", esc_attrs(&atts), esc_html_acf(field.get("label").unwrap_or(&Value::Null)))
}

/// `acf_field_group::render_field()`: the sub fields with the group's values, named inside the group's input.
async fn render_group(ctx: &Ctx<'_>, field: RenderField) -> Result<String> {
    if field.sub_fields.is_empty() {
        return Ok(String::new());
    }
    let values = field.get("value").cloned().unwrap_or(Value::Null);
    let mut subs = Vec::new();
    for sub in &field.sub_fields {
        let mut sub_field = RenderField::load(sub, ctx);
        let value = match &values {
            Value::Array(items) => items.iter().find(|(k, v)| key_eq(k, &key(&sub.key)) && isset(Some(v))).map(|(_, v)| v.clone()),
            _ => None,
        };
        if let Some(value) = value {
            sub_field.set("value", value);
        } else if let Some(default) = sub_field.isset("default_value") {
            sub_field.set("value", default);
        }
        let prefix = field.get("name").cloned().unwrap_or(Value::Null);
        sub_field.set("prefix", prefix);
        if field.truthy("required") {
            sub_field.set("required", Value::Int(0));
        }
        subs.push(sub_field);
    }
    let mut out = String::new();
    if field.str("layout") == "table" {
        out.push_str("<table class=\"acf-table\">\n\t<thead>\n\t\t<tr>\n\t\t\t");
        for sub in &subs {
            let mut sub = sub.clone();
            sub.prepare();
            let mut atts = Attrs::new(&[("class", string("acf-th")), ("data-name", sub.get("_name").cloned().unwrap_or(Value::Null)), ("data-type", string(&sub.kind())), ("data-key", sub.get("key").cloned().unwrap_or(Value::Null))]);
            let width = sub.get("wrapper").and_then(|w| w.get("width")).cloned().unwrap_or(Value::Null);
            if truthy(Some(&width)) {
                atts.set("data-width", width.clone());
                atts.set("style", string(&format!("width: {}%;", text(Some(&width)))));
            }
            out.push_str(&format!("\t\t\t<th {}>\n\t\t\t\t{}\t\t\t\t{}\t\t\t</th>\n\t\t\t", esc_attrs(&atts), render_label(&sub), render_instructions(&sub, false)));
        }
        out.push_str("\t\t</tr>\n\t</thead>\n\t<tbody>\n\t\t<tr class=\"acf-row\">\n\t\t\t");
        for sub in subs {
            out.push_str(&render_field_wrap(ctx, sub, "td", "label").await?);
        }
        out.push_str("\t\t</tr>\n\t</tbody>\n</table>\n\t\t\t");
    } else {
        let placement = if field.str("layout") == "block" { "top" } else { "left" };
        out.push_str(&format!("<div class=\"acf-fields -{} -border\">", esc_attr(placement)));
        for sub in subs {
            out.push_str(&render_field_wrap(ctx, sub, "div", "label").await?);
        }
        out.push_str("</div>");
    }
    Ok(out)
}

/// `acf_get_image_size( $size )`: the width and height of a registered size (`{size}_size_w` for the sizes of the
/// settings), each `None` when unset or 0.
fn image_size(site: &Site, size: &str) -> (Option<String>, Option<String>) {
    let (w, h) = match size {
        "1536x1536" => ("1536".to_owned(), "1536".to_owned()),
        "2048x2048" => ("2048".to_owned(), "2048".to_owned()),
        _ => (site.options.get(&format!("{size}_size_w")).unwrap_or_default().to_owned(), site.options.get(&format!("{size}_size_h")).unwrap_or_default().to_owned()),
    };
    let set = |v: String| truthy(Some(&string(&v))).then_some(v);
    (set(w), set(h))
}

/// `wp_get_attachment_image_src( $id, $size )` of an image attachment.
async fn attachment_image_src(site: &Site, id: i64, size: &str) -> Result<Option<String>> {
    if id <= 0 {
        return Ok(None);
    }
    let Some(post) = site.db.post(id as u64).await? else { return Ok(None) };
    if post.post_type != "attachment" || !post.post_mime_type.starts_with("image/") {
        return Ok(None);
    }
    let (meta, _, url) = super::format::attachment_url(site, &post).await?;
    Ok(Some(super::format::image_src(site, meta.as_ref(), &url, size).0))
}

/// `acf_get_file_input()`: the file input and the nonce of its upload.
fn file_input(ctx: &Ctx<'_>, attrs: &Attrs) -> String {
    let mut field_key = match attrs.get("key") {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        _ => String::new(),
    };
    if field_key.is_empty() {
        let name = text(attrs.get("name"));
        let parts: Vec<&str> = name.split('[').collect();
        if parts.len() > 1 && !parts[1].is_empty() && parts[1] != "0" {
            let last = parts[parts.len() - 1];
            field_key = last[..last.len().saturating_sub(1)].to_owned();
        }
    }
    let nonce = if field_key.is_empty() {
        String::new()
    } else {
        let attrs = Attrs::new(&[("name", string(&format!("acf[{field_key}_file_nonce]"))), ("value", string(&(ctx.nonce)(&format!("acf/file_uploader_nonce/{field_key}"))))]);
        format!("<input type=\"hidden\" {} />", esc_attrs(&attrs))
    };
    format!("<input type=\"file\" {} />{nonce}", esc_attrs(attrs))
}

/// The error of a basic upload that failed: the value is a message, not an attachment.
fn basic_error(field: &RenderField) -> String {
    let value = field.get("value").cloned().unwrap_or(Value::Null);
    if truthy(Some(&value)) && !is_numeric(&value) { format!("\t\t\t\t<div class=\"acf-error-message\"><p>{}</p></div>\n\t\t\t", esc_html_acf(&value)) } else { String::new() }
}

/// `acf_field_image::render_field()`.
async fn render_image(ctx: &Ctx<'_>, field: RenderField) -> Result<String> {
    let uploader = ctx.uploader;
    if uploader == "wp" {
        ctx.wants_uploader.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let mut value = string("");
    let mut div = Attrs::new(&[
        ("class", string("acf-image-uploader")),
        ("data-preview_size", field.get("preview_size").cloned().unwrap_or(Value::Null)),
        ("data-library", field.get("library").cloned().unwrap_or(Value::Null)),
        ("data-mime_types", field.get("mime_types").cloned().unwrap_or(Value::Null)),
        ("data-uploader", string(uploader)),
    ]);
    let mut img = Attrs::new(&[("src", string("")), ("alt", string("")), ("data-name", string("image"))]);
    let raw = field.get("value").cloned().unwrap_or(Value::Null);
    let preview_size = field.str("preview_size");
    if truthy(Some(&raw)) && is_numeric(&raw) {
        let id = super::php::intval(Some(&raw));
        if let Some(src) = attachment_image_src(ctx.site, id, &preview_size).await? {
            value = raw.clone();
            img.set("src", string(&src));
            let alt = ctx.site.db.acf_meta(rustpress_db::MetaKind::Post, id as u64, "_wp_attachment_image_alt").await?.unwrap_or_default();
            img.set("alt", string(&alt));
            div.append("class", " has-value");
        }
    }
    let (w, h) = image_size(ctx.site, &preview_size);
    let size_w = w.map_or_else(|| "100%".to_owned(), |w| format!("{w}px"));
    let size_h = h.map_or_else(|| "100%".to_owned(), |h| format!("{h}px"));
    img.set("style", string(&format!("max-height: {size_h};")));
    let hidden = hidden_input(&Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", value)]));
    let edit = if uploader == "basic" { String::new() } else { format!("\t\t\t<a class=\"acf-icon -pencil dark\" data-name=\"edit\" href=\"#\" title=\"{}\" aria-label=\"{}\"></a>\n\t\t\t", esc_attr(&ctx.acf("Edit")), esc_attr(&ctx.acf("Edit image"))) };
    let empty = if uploader == "basic" {
        let mut args = Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("id", field.get("id").cloned().unwrap_or(Value::Null)), ("key", field.get("key").cloned().unwrap_or(Value::Null))]);
        if field.truthy("mime_types") {
            let extensions = field.str("mime_types").replace([' ', '.'], "");
            let accept: Vec<String> = extensions.split(',').filter(|e| truthy(Some(&string(e)))).map(|e| format!(".{e}")).collect();
            args.set("accept", string(&accept.join(",")));
        } else {
            args.set("accept", string("image/*"));
        }
        format!("\t\t\t\t{}\t\t\t<label class=\"acf-basic-uploader\">\n\t\t\t\t{}\t\t\t</label>\n\t\t", basic_error(&field), file_input(ctx, &args))
    } else {
        format!("\t\t\t<p>{} <a data-name=\"add\" class=\"acf-button button\" href=\"#\">{}</a></p>\n\t\t", esc_html(&ctx.acf("No image selected")), esc_html(&ctx.acf("Add Image")))
    };
    Ok(format!(
        "<div {}>\n\t\t\t{hidden}\t<div class=\"show-if-value image-wrap\" style=\"max-width: {}\" tabindex=\"0\" role=\"button\" aria-label=\"{}\">\n\t\t<img {} />\n\t\t<div class=\"acf-actions -hover\">\n\t\t\t{edit}\t\t\t<a class=\"acf-icon -cancel dark\" data-name=\"remove\" href=\"#\" title=\"{}\" aria-label=\"{}\"></a>\n\t\t</div>\n\t</div>\n\t<div class=\"hide-if-value\">\n\t\t\t{empty}\t</div>\n</div>\n\t\t\t",
        esc_attrs(&div),
        esc_attr(&size_w),
        esc_attr(&ctx.acf("Selected image. Press tab to access image options.")),
        esc_attrs(&img),
        esc_attr(&ctx.acf("Remove")),
        esc_attr(&ctx.acf("Remove image")),
    ))
}

/// `acf_field_file::render_field()`.
async fn render_file(ctx: &Ctx<'_>, field: RenderField) -> Result<String> {
    let uploader = field.isset("uploader").map_or_else(|| ctx.uploader.to_owned(), |v| text(Some(&v)));
    if uploader == "wp" {
        ctx.wants_uploader.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let mut div = Attrs::new(&[
        ("class", string("acf-file-uploader")),
        ("data-library", field.get("library").cloned().unwrap_or(Value::Null)),
        ("data-mime_types", field.get("mime_types").cloned().unwrap_or(Value::Null)),
        ("data-uploader", string(&uploader)),
    ]);
    let (mut icon, mut title, mut url, mut filename, mut filesize) = (String::new(), String::new(), String::new(), String::new(), String::new());
    let raw = field.get("value").cloned().unwrap_or(Value::Null);
    if truthy(Some(&raw)) {
        let formatter = super::format::Formatter { site: ctx.site, services: ctx.services, global_post: None };
        let attachment = formatter.attachment(super::php::intval(Some(&raw))).await?;
        if let Value::Array(_) = &attachment {
            div.append("class", " has-value");
            icon = text(attachment.get("icon"));
            title = text(attachment.get("title"));
            url = text(attachment.get("url"));
            filename = text(attachment.get("filename"));
            if truthy(attachment.get("filesize")) {
                filesize = rustpress_core::health::size_format(ctx.site, super::php::intval(attachment.get("filesize")).max(0) as u64, 0);
            }
        }
    }
    let hidden = hidden_input(&Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", raw.clone()), ("data-name", string("id"))]));
    let edit = if uploader == "basic" { String::new() } else { format!("\t\t\t<a class=\"acf-icon -pencil dark\" data-name=\"edit\" href=\"#\" title=\"{}\" aria-label=\"{}\"></a>\n\t\t\t", esc_attr(&ctx.acf("Edit")), esc_attr(&ctx.acf("Edit file"))) };
    let empty = if uploader == "basic" {
        let mut args = Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("id", field.get("id").cloned().unwrap_or(Value::Null)), ("key", field.get("key").cloned().unwrap_or(Value::Null))]);
        if field.truthy("mime_types") {
            args.set("accept", field.get("mime_types").cloned().unwrap_or(Value::Null));
        }
        format!("\t\t\t\n\t\t\t\t{}\t\t\t\n\t\t\t<label class=\"acf-basic-uploader\">\n\t\t\t\t{}\t\t\t</label>\n\t\t\t\n\t\t", basic_error(&field), file_input(ctx, &args))
    } else {
        format!("\t\t\t\n\t\t\t<p>{} <a data-name=\"add\" class=\"acf-button button\" href=\"#\">{}</a></p>\n\t\t\t\n\t\t", esc_html(&ctx.acf("No file selected")), esc_html(&ctx.acf("Add File")))
    };
    Ok(format!(
        "<div {}>\n\t\t\t{hidden}\t<div class=\"show-if-value file-wrap\" tabindex=\"0\" role=\"button\" aria-label=\"{}\">\n\t\t<div class=\"file-icon\">\n\t\t\t<img data-name=\"icon\" src=\"{}\" alt=\"\"/>\n\t\t</div>\n\t\t<div class=\"file-info\">\n\t\t\t<p>\n\t\t\t\t<strong data-name=\"title\">{}</strong>\n\t\t\t</p>\n\t\t\t<p>\n\t\t\t\t<strong>{}:</strong>\n\t\t\t\t<a data-name=\"filename\" href=\"{}\" target=\"_blank\">{}</a>\n\t\t\t</p>\n\t\t\t<p>\n\t\t\t\t<strong>{}:</strong>\n\t\t\t\t<span data-name=\"filesize\">{}</span>\n\t\t\t</p>\n\t\t</div>\n\t\t<div class=\"acf-actions -hover\">\n\t\t\t{edit}\t\t\t<a class=\"acf-icon -cancel dark\" data-name=\"remove\" href=\"#\" title=\"{}\" aria-label=\"{}\"></a>\n\t\t</div>\n\t</div>\n\t<div class=\"hide-if-value\">\n\t\t\t{empty}\t\t\n\t</div>\n</div>\n\t\t\t",
        esc_attrs(&div),
        esc_attr(&ctx.acf("Selected file. Press tab to access file options.")),
        esc_url(&icon),
        esc_html(&title),
        esc_html(&ctx.acf("File name")),
        esc_url(&url),
        esc_html(&filename),
        esc_html(&ctx.acf("File size")),
        esc_html(&filesize),
        esc_attr(&ctx.acf("Remove")),
        esc_attr(&ctx.acf("Remove file")),
    ))
}

/// The post types of a relational field (`acf_get_posts()` with `post_type`): the setting, or every type ACF lists.
fn post_types_of(site: &Site, field: &RenderField) -> Vec<String> {
    match field.get("post_type") {
        Some(Value::Array(items)) if !items.is_empty() => items.iter().map(|(_, v)| text(Some(v))).collect(),
        Some(v) if truthy(Some(v)) && !matches!(v, Value::Array(_)) => vec![text(Some(v))],
        _ => super::format::all_post_types(site),
    }
}

/// `acf_get_post_title()`: the title (with `(no title)`), the status when not published, a dash per ancestor.
async fn post_title(ctx: &Ctx<'_>, post: &rustpress_db::Post) -> Result<String> {
    let mut title = if ctx.admin { rustpress_blocks::content::the_title_in_admin(ctx.site, post) } else { rustpress_blocks::content::the_title_for(ctx.site, post) };
    if title.is_empty() {
        title = ctx.acf("(no title)");
    }
    let status = post_status(ctx.site, post).await?;
    if status != "publish" {
        title.push_str(&format!(" ({status})"));
    }
    let mut prepend = String::new();
    if post.post_type != "attachment" {
        let mut parent = post.post_parent;
        let mut seen = vec![post.id];
        while parent != 0 && !seen.contains(&parent) {
            seen.push(parent);
            let Some(p) = ctx.site.db.post(parent).await? else { break };
            prepend.push_str("- ");
            parent = p.post_parent;
        }
    }
    Ok(format!("{prepend}{title}"))
}

/// `get_post_status()`: an attachment takes the status of its parent (`publish` without one).
async fn post_status(site: &Site, post: &rustpress_db::Post) -> Result<String> {
    if post.post_type == "attachment" && post.post_status == "inherit" {
        if post.post_parent == 0 || post.post_parent == post.id {
            return Ok("publish".to_owned());
        }
        return Ok(match site.db.post(post.post_parent).await? {
            Some(parent) => parent.post_status,
            None => "publish".to_owned(),
        });
    }
    Ok(post.post_status.clone())
}

/// A relational field as the select of its AJAX search, with the chosen items as choices.
fn as_ajax_select(ctx: &Ctx<'_>, field: &mut RenderField, name: &str, choices: Vec<(Key, Value)>) {
    field.set("type", string("select"));
    field.set("ui", Value::Int(1));
    field.set("ajax", Value::Int(1));
    field.set("nonce", string(&(ctx.nonce)(&format!("acf_field_{name}_{}", field.str("key")))));
    field.set("choices", Value::Array(choices));
}

/// `acf_field_post_object::render_field()`.
async fn render_post_object(ctx: &Ctx<'_>, mut field: RenderField) -> Result<String> {
    let value = field.get("value").cloned().unwrap_or(Value::Null);
    let ids: Vec<u64> = match &value {
        Value::Array(items) => items.iter().filter(|(_, v)| is_numeric(v)).map(|(_, v)| super::php::intval(Some(v)).max(0) as u64).collect(),
        v if is_numeric(v) => vec![super::php::intval(Some(v)).max(0) as u64],
        _ => Vec::new(),
    };
    let mut choices = Vec::new();
    for post in ctx.site.db.acf_get_posts(&ids, &post_types_of(ctx.site, &field)).await? {
        choices.push((Key::Int(post.post.id as i64), string(&post_title(ctx, &post.post).await?)));
    }
    as_ajax_select(ctx, &mut field, "post_object", choices);
    Ok(render_select(ctx, field, "select"))
}

/// `acf_field_page_link::render_field()`: chosen posts by title, other values (archive links) as they are.
async fn render_page_link(ctx: &Ctx<'_>, mut field: RenderField) -> Result<String> {
    let value = get_array(field.get("value"));
    let mut choices: Vec<(Key, Value)> = Vec::new();
    if field.truthy("value") {
        let ids: Vec<u64> = value.iter().filter(|(_, v)| is_numeric(v)).map(|(_, v)| super::php::intval(Some(v)).max(0) as u64).collect();
        let mut posts = if ids.is_empty() { Vec::new() } else { ctx.site.db.acf_get_posts(&ids, &post_types_of(ctx.site, &field)).await? }.into_iter();
        for (_, v) in &value {
            if is_numeric(v) {
                if let Some(post) = posts.next() {
                    let title = post_title(ctx, &post.post).await?;
                    super::php::set(&mut choices, &post.post.id.to_string(), string(&title));
                }
            } else {
                super::php::set(&mut choices, &text(Some(v)), v.clone());
            }
        }
    }
    as_ajax_select(ctx, &mut field, "page_link", choices);
    Ok(render_select(ctx, field, "select"))
}

/// `acf_field_user::render_field()`: the chosen users by login and name (`acf_get_user_result()`).
async fn render_user(ctx: &Ctx<'_>, mut field: RenderField) -> Result<String> {
    let mut choices: Vec<(Key, Value)> = Vec::new();
    if field.truthy("value") {
        let ids: Vec<u64> = match field.get("value") {
            Some(Value::Array(items)) => items.iter().map(|(_, v)| super::php::intval(Some(v)).max(0) as u64).collect(),
            other => vec![super::php::intval(other).max(0) as u64],
        };
        let mut users = ctx.site.db.users_by_ids(&ids).await?;
        users.sort_by_key(|u| ids.iter().position(|id| *id == u.id));
        for user in users {
            let first = ctx.site.db.user_meta(user.id, "first_name").await?.unwrap_or_default();
            let last = ctx.site.db.user_meta(user.id, "last_name").await?.unwrap_or_default();
            let mut label = user.user_login.clone();
            if truthy(Some(&string(&first))) && truthy(Some(&string(&last))) {
                label.push_str(&format!(" ({first} {last})"));
            } else if truthy(Some(&string(&first))) {
                label.push_str(&format!(" ({first})"));
            }
            choices.push((Key::Int(user.id as i64), string(&label)));
        }
    }
    as_ajax_select(ctx, &mut field, "user", choices);
    Ok(render_select(ctx, field, "select"))
}

/// `get_taxonomy( $name )`: whether the taxonomy exists, and its plural name and "Add" label.
fn taxonomy_labels(site: &Site, name: &str) -> Option<(String, String, bool)> {
    let core = matches!(name, "category" | "post_tag" | "post_format" | "nav_menu" | "link_category" | "wp_pattern_category");
    let info = site.types.taxonomy(name);
    if !core && info.is_none() {
        return None;
    }
    let hierarchical = match name {
        "category" | "wp_pattern_category" => true,
        "post_tag" | "post_format" | "nav_menu" | "link_category" => false,
        _ => info.is_some_and(|t| t.hierarchical),
    };
    let add = match name {
        "post_tag" => site.tr("Add Tag"),
        "category" => site.tr("Add Category"),
        _ => site.tr(if hierarchical { "Add Category" } else { "Add Tag" }),
    };
    Some((site.taxonomy_label(name), add, hierarchical))
}

/// `acf_get_term_title()`: the name (or `(no title)`) after a dash per ancestor.
async fn term_title(ctx: &Ctx<'_>, term: &rustpress_db::Term, hierarchical: bool) -> Result<String> {
    let mut title = term.name.clone();
    if title.is_empty() {
        title = ctx.acf("(no title)");
    }
    if hierarchical {
        let mut parent = term.parent;
        let mut seen = vec![term.term_id];
        let mut prefix = String::new();
        while parent != 0 && !seen.contains(&parent) {
            seen.push(parent);
            let Some(p) = ctx.site.db.term_by_id(Some(&term.taxonomy), parent).await? else { break };
            prefix.push_str("- ");
            parent = p.parent;
        }
        title = format!("{prefix}{title}");
    }
    Ok(title)
}

/// `ACF_Taxonomy_Field_Walker` walked like `Walker::walk( $terms, 0 )`.
struct TermWalker<'a> {
    kind: String,
    name: Value,
    selected: &'a [Value],
}

impl TermWalker<'_> {
    fn start_el(&self, out: &mut String, term: &rustpress_db::Term) {
        let id = Value::Int(term.term_id as i64);
        let is_selected = in_array(&id, self.selected);
        let mut attrs = Attrs::new(&[("type", string(&self.kind)), ("name", self.name.clone()), ("value", id)]);
        if is_selected {
            attrs.set("checked", Value::Bool(true));
        }
        out.push_str(&format!(
            "\n<li data-id=\"{}\"><label{}><input {}/> <span>{}</span></label>",
            term.term_id,
            if is_selected { " class=\"selected\"" } else { "" },
            esc_attrs(&attrs),
            esc_html_acf(&string(&term.name))
        ));
    }

    fn display(&self, out: &mut String, term: &rustpress_db::Term, children: &mut std::collections::HashMap<u64, Vec<rustpress_db::Term>>, max_depth: usize, depth: usize) {
        self.start_el(out, term);
        if (max_depth == 0 || max_depth > depth + 1)
            && let Some(kids) = children.remove(&term.term_id)
        {
            let indent = "\t".repeat(depth);
            out.push_str(&format!("{indent}<ul class='children acf-bl'>\n"));
            for child in &kids {
                self.display(out, child, children, max_depth, depth + 1);
            }
            out.push_str(&format!("{indent}</ul>\n"));
        }
        out.push_str("</li>\n");
    }

    fn walk(&self, terms: &[rustpress_db::Term]) -> String {
        let mut out = String::new();
        if terms.is_empty() {
            return out;
        }
        let mut top: Vec<rustpress_db::Term> = terms.iter().filter(|t| t.parent == 0).cloned().collect();
        let mut children: std::collections::HashMap<u64, Vec<rustpress_db::Term>> = std::collections::HashMap::new();
        let mut order: Vec<u64> = Vec::new();
        let root_parent = if top.is_empty() { Some(terms[0].parent) } else { None };
        if let Some(root) = root_parent {
            top = terms.iter().filter(|t| t.parent == root).cloned().collect();
        }
        for t in terms {
            let is_top = match root_parent {
                Some(root) => t.parent == root,
                None => t.parent == 0,
            };
            if !is_top {
                if !children.contains_key(&t.parent) {
                    order.push(t.parent);
                }
                children.entry(t.parent).or_default().push(t.clone());
            }
        }
        for t in &top {
            self.display(&mut out, t, &mut children, 0, 0);
        }
        // Orphans whose parents are not in the list.
        for parent in order {
            if let Some(orphans) = children.remove(&parent) {
                let mut none = std::collections::HashMap::new();
                for t in &orphans {
                    self.display(&mut out, t, &mut none, 1, 0);
                }
            }
        }
        out
    }
}

/// `acf_field_taxonomy::render_field()`: a select of the chosen terms (`select`, `multi_select`), or every term of the
/// taxonomy as a checkbox or radio list (`wp_list_categories()` with ACF's walker).
async fn render_taxonomy(ctx: &Ctx<'_>, mut field: RenderField) -> Result<String> {
    let value = get_array(field.get("value"));
    field.set("value", Value::Array(value.clone()));
    let nonce = (ctx.nonce)(&format!("acf_field_taxonomy_{}", field.str("key")));
    let div = Attrs::new(&[
        ("class", string("acf-taxonomy-field")),
        ("data-save", field.get("save_terms").cloned().unwrap_or(Value::Null)),
        ("data-ftype", field.get("field_type").cloned().unwrap_or(Value::Null)),
        ("data-taxonomy", field.get("taxonomy").cloned().unwrap_or(Value::Null)),
        ("data-allow_null", field.get("allow_null").cloned().unwrap_or(Value::Null)),
        ("data-nonce", string(&nonce)),
    ]);
    let taxonomy = field.str("taxonomy");
    let Some((label, add_label, hierarchical)) = taxonomy_labels(ctx.site, &taxonomy) else { return Ok(String::new()) };
    let mut out = format!("<div {}>\n\t\t\t", esc_attrs(&div));
    if field.truthy("add_term") && (ctx.can)("manage_categories") {
        out.push_str(&format!("\t<div class=\"acf-actions -hover\">\n\t\t<a href=\"#\" class=\"acf-icon -plus acf-js-tooltip small\" data-name=\"add\" title=\"{}\"></a>\n\t</div>\n\t\t\t\t", esc_attr(&add_label)));
    }
    let field_type = field.str("field_type");
    match field_type.as_str() {
        "select" | "multi_select" => {
            field.set("multiple", Value::Int(i64::from(field_type == "multi_select")));
            let mut choices: Vec<(Key, Value)> = Vec::new();
            if !value.is_empty() {
                for (_, v) in &value {
                    let id = super::php::intval(Some(v));
                    if id <= 0 {
                        continue;
                    }
                    if let Some(term) = ctx.site.db.term_by_id(Some(&taxonomy), id as u64).await? {
                        let title = term_title(ctx, &term, hierarchical).await?;
                        super::php::set(&mut choices, &term.term_id.to_string(), string(&title));
                    }
                }
            }
            field.set("type", string("select"));
            field.set("ui", Value::Int(1));
            field.set("ajax", Value::Int(1));
            field.set("nonce", string(&nonce));
            field.set("choices", Value::Array(choices));
            out.push_str(&render_select(ctx, field, "select"));
        }
        "radio" | "checkbox" => {
            out.push_str(&hidden_input(&Attrs::new(&[("type", string("hidden")), ("name", field.get("name").cloned().unwrap_or(Value::Null))])));
            if field_type == "checkbox" {
                field.append("name", "[]");
            }
            let mut ul = Attrs::new(&[("class", string("acf-checkbox-list acf-bl")), ("role", string(if field_type == "radio" { "radiogroup" } else { "group" }))]);
            if field.truthy("id") {
                ul.set("aria-labelledby", string(&format!("{}-label", field.str("id"))));
            }
            let terms = ctx.site.db.terms(&taxonomy).await?;
            let list = if terms.is_empty() {
                rustpress_core::i18n::sprintf(&ctx.acfx("No %s", "No Terms"), &[&label])
            } else {
                let selected: Vec<Value> = value.iter().map(|(_, v)| v.clone()).collect();
                TermWalker { kind: field_type.clone(), name: field.get("name").cloned().unwrap_or(Value::Null), selected: &selected }.walk(&terms)
            };
            out.push_str(&format!("<div class=\"categorychecklist-holder\">\n\t<ul {}>\n\t\t\t{list}\t</ul>\n</div>\n\t\t\t", esc_attrs(&ul)));
        }
        _ => {}
    }
    out.push_str("</div>\n\t\t\t");
    Ok(out)
}

/// `acf_get_post_type_label()`: the singular name, the name itself for a type that does not exist.
fn post_type_singular(site: &Site, name: &str) -> String {
    match name {
        "post" => site.trx("Post", "post type singular name"),
        "page" => site.trx("Page", "post type singular name"),
        // No `singular_name`: `_get_custom_object_labels()` copies the name.
        "attachment" => site.trx("Media", "post type general name"),
        other => match site.types.post_type(other).or_else(|| site.types.rewrite_post_types().iter().find(|t| t.name == other)) {
            Some(t) => t.singular_label(),
            None => site.types.registered_post_types.iter().find(|r| r.name == other).and_then(|r| r.singular.clone().or_else(|| r.label.clone())).unwrap_or_else(|| other.to_owned()),
        },
    }
}

/// Labels that more than one entry shares get the entry's name added (`acf_get_pretty_post_types()`,
/// `acf_get_taxonomy_labels()`).
fn disambiguate(items: Vec<(String, String)>) -> Vec<(String, String)> {
    let count = |label: &str| items.iter().filter(|(_, l)| l == label).count();
    items.iter().map(|(name, label)| (name.clone(), if count(label) > 1 { format!("{label} ({name})") } else { label.clone() })).collect()
}

/// `acf_get_taxonomies()`: the public ones of WordPress and every registered one; with post types, the taxonomies
/// of those types (`acf_get_taxonomies_for_post_type()`).
fn acf_taxonomies(site: &Site, post_types: Option<&[String]>) -> Vec<String> {
    let registered: Vec<&rustpress_core::types::TaxonomyInfo> = site.types.rewrite_taxonomies().iter().chain(site.types.taxonomies.iter()).collect();
    if let Some(types) = post_types {
        let mut out: Vec<String> = Vec::new();
        for post_type in types {
            let core: &[&str] = if post_type == "post" { &["category", "post_tag", "post_format"] } else { &[] };
            for name in core.iter().map(|s| (*s).to_owned()).chain(registered.iter().filter(|t| t.object_types.iter().any(|o| o == post_type)).map(|t| t.name.clone())) {
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        return out;
    }
    let mut out: Vec<String> = ["category", "post_tag", "post_format"].iter().map(|s| (*s).to_owned()).collect();
    for t in registered {
        if !out.contains(&t.name) && !matches!(t.name.as_str(), "nav_menu" | "link_category" | "wp_theme" | "wp_template_part_area" | "wp_pattern_category") {
            out.push(t.name.clone());
        }
    }
    out
}

/// `acf_get_taxonomy_labels()` of a list of taxonomies.
fn taxonomy_singulars(site: &Site, taxonomies: &[String]) -> Vec<(String, String)> {
    disambiguate(taxonomies.iter().map(|t| (t.clone(), site.taxonomy_singular_label(t))).collect())
}

/// `_get_term_children( 0, $terms )`: the terms below `parent` depth first, in the order of the list.
fn term_children(parent: u64, terms: &[rustpress_db::Term], seen: &mut Vec<u64>) -> Vec<rustpress_db::Term> {
    let mut out = Vec::new();
    for term in terms {
        if term.term_id == parent || term.parent != parent || seen.contains(&term.term_id) {
            continue;
        }
        seen.push(term.term_id);
        out.push(term.clone());
        out.extend(term_children(term.term_id, terms, seen));
    }
    out
}

/// The choices of the taxonomy filter: `{taxonomy}:{slug}` by the taxonomy's label (`acf_get_choices_from_terms()`
/// and `acf_get_grouped_terms()`).
async fn term_filter_choices(ctx: &Ctx<'_>, field: &RenderField, post_types: &[String]) -> Result<Vec<(Key, Value)>> {
    let site = ctx.site;
    let mut groups: Vec<(Key, Value)> = Vec::new();
    let push = |groups: &mut Vec<(Key, Value)>, label: &str, id: String, text: String| {
        let k = key(label);
        match groups.iter_mut().find(|(gk, _)| key_eq(gk, &k)) {
            Some((_, Value::Array(items))) => super::php::set(items, &id, string(&text)),
            _ => groups.push((k, Value::Array(vec![(key(&id), string(&text))]))),
        }
    };
    let hierarchical = |name: &str| taxonomy_labels(site, name).is_some_and(|(_, _, h)| h);
    let chosen = get_array(field.get("taxonomy"));
    if !chosen.is_empty() {
        let labels = taxonomy_singulars(site, &acf_taxonomies(site, None));
        for (_, v) in chosen {
            let encoded = text(Some(&v));
            let Some((taxonomy, slug)) = encoded.split_once(':') else { continue };
            let Some(term) = site.db.term_by_slug(taxonomy, slug).await? else { continue };
            let label = labels.iter().find(|(t, _)| *t == term.taxonomy).map(|(_, l)| l.clone()).unwrap_or_default();
            let title = term_title(ctx, &term, hierarchical(&term.taxonomy)).await?;
            push(&mut groups, &label, format!("{}:{}", term.taxonomy, term.slug), title);
        }
        return Ok(groups);
    }
    let taxonomies = if post_types.is_empty() { acf_taxonomies(site, None) } else { acf_taxonomies(site, Some(post_types)) };
    for (taxonomy, label) in taxonomy_singulars(site, &taxonomies) {
        let mut terms = site.db.terms(&taxonomy).await?;
        if terms.is_empty() {
            continue;
        }
        if hierarchical(&taxonomy) {
            let ordered = term_children(0, &terms, &mut Vec::new());
            if ordered.len() == terms.len() {
                terms = ordered;
            }
        }
        for term in &terms {
            let title = term_title(ctx, term, hierarchical(&taxonomy)).await?;
            push(&mut groups, &label, format!("{}:{}", term.taxonomy, term.slug), title);
        }
    }
    Ok(groups)
}

/// `acf_field_relationship::render_field()`. The featured image of the `elements` setting is left out.
async fn render_relationship(ctx: &Ctx<'_>, field: RenderField) -> Result<String> {
    let site = ctx.site;
    let post_types: Vec<String> = get_array(field.get("post_type")).iter().map(|(_, v)| text(Some(v))).collect();
    let filters: Vec<Value> = get_array(field.get("filters")).into_iter().map(|(_, v)| v).collect();
    let has = |name: &str| in_array(&string(name), &filters);
    let mut post_type_choices: Vec<(Key, Value)> = Vec::new();
    if has("post_type") {
        post_type_choices.push((key(""), string(&ctx.acf("Select post type"))));
        let types = if post_types.is_empty() { super::format::all_post_types(site) } else { post_types.clone() };
        for (name, label) in disambiguate(types.iter().map(|t| (t.clone(), post_type_singular(site, t))).collect()) {
            super::php::set(&mut post_type_choices, &name, string(&label));
        }
    }
    let mut taxonomy_choices: Vec<(Key, Value)> = Vec::new();
    if has("taxonomy") {
        taxonomy_choices.push((key(""), string(&ctx.acf("Select taxonomy"))));
        for (k, v) in term_filter_choices(ctx, &field, &post_types).await? {
            if !taxonomy_choices.iter().any(|(existing, _)| key_eq(existing, &k)) {
                taxonomy_choices.push((k, v));
            }
        }
    }
    let atts = Attrs::new(&[
        ("id", field.get("id").cloned().unwrap_or(Value::Null)),
        ("class", string(&format!("acf-relationship {}", field.str("class")))),
        ("data-min", field.get("min").cloned().unwrap_or(Value::Null)),
        ("data-max", field.get("max").cloned().unwrap_or(Value::Null)),
        ("data-s", string("")),
        ("data-paged", Value::Int(1)),
        ("data-post_type", string("")),
        ("data-taxonomy", string("")),
        ("data-nonce", string(&(ctx.nonce)(&format!("acf_field_relationship_{}", field.str("key"))))),
    ]);
    let hidden = hidden_input(&Attrs::new(&[("name", field.get("name").cloned().unwrap_or(Value::Null)), ("value", string(""))]));
    let mut out = format!("<div {}>\n\t\n\t\t\t{hidden}\t\n\t\t\t", esc_attrs(&atts));
    if !filters.is_empty() {
        out.push_str(&format!("\t<div class=\"filters -f{}\">\n\t\t\t\t", filters.len()));
        if has("search") {
            out.push_str(&format!("\t\t<div class=\"filter -search\">\n\t\t\t\t\t{}\t\t</div>\n\t\t\t\t\t", text_input(Attrs::new(&[("placeholder", string(&ctx.acf("Search..."))), ("data-filter", string("s"))]))));
        }
        if has("post_type") {
            out.push_str(&format!("\t\t<div class=\"filter -post_type\">\n\t\t\t\t\t{}\t\t</div>\n\t\t\t\t\t", select_input(Attrs::new(&[("choices", Value::Array(post_type_choices)), ("data-filter", string("post_type"))]))));
        }
        if has("taxonomy") {
            out.push_str(&format!("\t\t<div class=\"filter -taxonomy\">\n\t\t\t\t\t{}\t\t</div>\n\t\t\t\t", select_input(Attrs::new(&[("choices", Value::Array(taxonomy_choices)), ("data-filter", string("taxonomy"))]))));
        }
        out.push_str("\t\t\n\t</div>\n\t\t\t");
    }
    out.push_str("\t\n\t<div class=\"selection\">\n\t\t<div class=\"choices\">\n\t\t\t<ul class=\"acf-bl list choices-list\"></ul>\n\t\t</div>\n\t\t<div class=\"values\">\n\t\t\t<ul class=\"acf-bl list values-list\">\n\t\t\t\t");
    if field.truthy("value") {
        let ids: Vec<u64> = get_array(field.get("value")).iter().map(|(_, v)| super::php::intval(Some(v)).max(0) as u64).collect();
        let types = if post_types.is_empty() { super::format::all_post_types(site) } else { post_types.clone() };
        for post in site.db.acf_get_posts(&ids, &types).await? {
            let title = esc_html(&post_title(ctx, &post.post).await?);
            out.push_str(&format!(
                "\t\t\t\t\t<li>\n\t\t\t\t\t\t{}\t\t\t\t\t\t<span tabindex=\"0\" data-id=\"{}\" class=\"acf-rel-item acf-rel-item-remove\">\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t<a href=\"#\" class=\"acf-icon -minus small dark\" data-name=\"remove_item\"></a>\n\t\t\t\t\t\t</span>\n\t\t\t\t\t</li>\n\t\t\t\t\t\t",
                hidden_input(&Attrs::new(&[("name", string(&format!("{}[]", field.str("name")))), ("value", Value::Int(post.post.id as i64))])),
                post.post.id,
                kses(&title)
            ));
        }
        out.push_str("\t\t\t\t");
    }
    out.push_str("\t\t\t</ul>\n\t\t</div>\n\t</div>\n</div>\n\t\t\t");
    Ok(out)
}

/// `acf_field_icon_picker::get_tabs()`.
fn icon_tabs(ctx: &Ctx<'_>) -> Vec<(&'static str, String)> {
    let mut tabs = vec![("dashicons", esc_html(&ctx.acf("Dashicons")))];
    if (ctx.can)("upload_files") {
        tabs.push(("media_library", esc_html(&ctx.acf("Media Library"))));
    }
    tabs.push(("url", esc_html(&ctx.acf("URL"))));
    tabs
}

/// `acf_field_icon_picker::render_icon_list_tab()` of the Dashicons.
fn icon_list_tab(ctx: &Ctx<'_>, tab: &str) -> String {
    let search = text_input(Attrs::new(&[("class", string("acf-icon-list-search-input")), ("placeholder", string(&esc_html(&ctx.acf("Search icons...")))), ("type", string("search"))]));
    let no_results = rustpress_core::i18n::sprintf(&esc_html(&ctx.acf("No search results for '%s'")), &["<span class=\"acf-invalid-icon-list-search-term\"></span>"]);
    format!(
        "\t\t\t<div class=\"acf-icon-list-search-wrap\">\n\t\t\t\t{search}\t\t\t</div>\n\t\t\t<div\n\t\t\t\tclass=\"acf-icon-list\"\n\t\t\t\trole=\"radiogroup\"\n\t\t\t\tdata-parent-tab=\"{}\"\n\t\t\t\t\t\t\t></div>\n\t\t\t<div class=\"acf-icon-list-empty\">\n\t\t\t\t<img src=\"{}\" />\n\t\t\t\t<p class=\"acf-no-results-text\">\n\t\t\t\t\t{no_results}\t\t\t\t</p>\n\t\t\t</div>\n\t\t\t",
        esc_attr(tab),
        esc_url(&ctx.site.site_url("wp-content/plugins/advanced-custom-fields/assets/images/face-sad.svg")),
    )
}

/// `acf_field_icon_picker::render_field()`.
async fn render_icon_picker(ctx: &Ctx<'_>, field: RenderField) -> Result<String> {
    if ctx.uploader == "wp" {
        ctx.wants_uploader.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let value = field.get("value").cloned().unwrap_or(Value::Null);
    let part = |name: &str| value.get(name).cloned().unwrap_or(Value::Null);
    let (kind, icon) = (text(Some(&part("type"))), part("value"));
    let div = Attrs::new(&[("id", field.get("id").cloned().unwrap_or(Value::Null)), ("class", string(&format!("{} acf-icon-picker", field.str("class"))))]);
    let mut out = format!("<div {}>", esc_attrs(&div));
    out.push_str(&hidden_input(&Attrs::new(&[("name", string(&format!("{}[type]", field.str("name")))), ("value", part("type")), ("data-hidden-type", string("type"))])));
    out.push_str(&hidden_input(&Attrs::new(&[("name", string(&format!("{}[value]", field.str("name")))), ("value", icon.clone()), ("data-hidden-type", string("value"))])));
    let wanted: Vec<String> = match field.get("tabs") {
        Some(Value::Array(items)) => items.iter().filter_map(|(_, v)| match v {
            Value::Str(s) => Some(String::from_utf8_lossy(s).into_owned()),
            _ => None,
        }).collect(),
        _ => Vec::new(),
    };
    let shown: Vec<(&str, String)> = icon_tabs(ctx).into_iter().filter(|(name, _)| wanted.iter().any(|w| w == name)).collect();
    for (name, label) in &shown {
        if shown.len() > 1 {
            let tab = RenderField::raw(
                vec![
                    (key("type"), string("tab")),
                    (key("label"), string(label)),
                    (key("key"), string("acf_icon_picker_tabs")),
                    (key("selected"), Value::Bool(*name == kind)),
                    (key("unique_tab_key"), string(name)),
                ],
                Vec::new(),
            );
            out.push_str(&render_field_wrap(ctx, tab, "div", "label").await?);
        }
        out.push_str(&format!("<div class=\"acf-icon-picker-tabs acf-icon-picker-{}-tabs\" data-tab=\"{}\">", esc_attr(&name.replace('_', "-")), esc_attr(name)));
        match *name {
            "dashicons" => out.push_str(&icon_list_tab(ctx, name)),
            "media_library" => {
                let shows = matches!(kind.as_str(), "media_library" | "dashicons") && truthy(Some(&icon));
                let img = if is_numeric(&icon) { attachment_image_src(ctx.site, super::php::intval(Some(&icon)), "thumbnail").await?.unwrap_or_default() } else { String::new() };
                out.push_str(&format!(
                    "\t\t\t\t\t\t<div class=\"acf-icon-picker-tab\" data-category=\"{}\">\n\t\t\t\t\t\t\t<div class=\"acf-icon-picker-media-library\">\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<button\n\t\t\t\t\t\t\t\t\taria-label=\"{}\"\n\t\t\t\t\t\t\t\t\tclass=\"acf-icon-picker-media-library-preview\"\n\t\t\t\t\t\t\t\t\tstyle=\"{}\"\n\t\t\t\t\t\t\t\t>\n\t\t\t\t\t\t\t\t\t<div class=\"acf-icon-picker-media-library-preview-img\" style=\"{}\">\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<img src=\"{}\" alt=\"{}\" />\n\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t<div class=\"acf-icon-picker-media-library-preview-dashicon\" style=\"{}\">\n\t\t\t\t\t\t\t\t\t\t<div class=\"dashicons {}\">\n\t\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t</button>\n\t\t\t\t\t\t\t\t<button class=\"acf-icon-picker-media-library-button\">\n\t\t\t\t\t\t\t\t\t<div class=\"acf-icon-picker-media-library-button-icon dashicons dashicons-admin-media\"></div>\n\t\t\t\t\t\t\t\t\t<span>{}</span>\n\t\t\t\t\t\t\t\t</button>\n\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t\t",
                    esc_attr(name),
                    esc_attr(&ctx.acf("Click to change the icon in the Media Library")),
                    esc_attr(if shows { "" } else { "display: none;" }),
                    esc_attr(if kind != "media_library" { "display: none;" } else { "" }),
                    esc_url(&img),
                    esc_attr(&ctx.acf("The currently selected image preview")),
                    esc_attr(if kind != "dashicons" { "display: none;" } else { "" }),
                    esc_attr(&text(Some(&icon))),
                    esc_html(&ctx.acf("Browse Media Library")),
                ));
            }
            _ => {
                let url = if kind == "url" { icon.clone() } else { string("") };
                out.push_str(&format!(
                    "<div class=\"acf-icon-picker-url\">{}\t\t\t\t\t\t<p class=\"description\">{}</p>\n\t\t\t\t\t\t</div>",
                    text_input(Attrs::new(&[("class", string("acf-icon_url")), ("value", url)])),
                    esc_html(&ctx.acf("The URL to the icon you'd like to use, or svg as Data URI"))
                ));
            }
        }
        out.push_str("</div>");
    }
    out.push_str("</div>");
    Ok(out)
}

/// `uniqid( $prefix )`: the time in seconds and microseconds as hex.
fn uniqid(prefix: &str) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{prefix}{:08x}{:05x}", now.as_secs(), now.subsec_micros())
}

/// `acf_field_wysiwyg::render_field()`. The editor's id comes from `uniqid()`, so it differs from request to request.
fn render_wysiwyg(ctx: &Ctx<'_>, mut field: RenderField) -> String {
    ctx.wants_uploader.store(true, std::sync::atomic::Ordering::Relaxed);
    let id = uniqid("acf-editor-");
    let mut default_editor = "html";
    let mut show_tabs = true;
    let tabs = field.str("tabs");
    if !ctx.rich_edit {
        show_tabs = false;
    } else if tabs == "visual" {
        default_editor = "tinymce";
        show_tabs = false;
    } else if tabs == "text" {
        show_tabs = false;
    } else if ctx.default_editor == "tinymce" {
        default_editor = "tinymce";
    }
    if !(ctx.can)("upload_files") {
        field.set("media_upload", Value::Int(0));
    }
    let switch_class = if default_editor == "html" { "html-active" } else { "tmce-active" };
    // `format_for_editor()` on `acf_the_editor_content`.
    let value = match field.get("value") {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        _ => String::new(),
    };
    let value = if value.is_empty() { value } else { value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") };
    let mut wrap = Attrs::new(&[("id", string(&format!("wp-{id}-wrap"))), ("class", string(&format!("acf-editor-wrap wp-core-ui wp-editor-wrap {switch_class}"))), ("data-toolbar", field.get("toolbar").cloned().unwrap_or(Value::Null))]);
    if field.truthy("delay") {
        wrap.append("class", " delay");
    }
    let height = ctx.wysiwyg_height.max(300);
    let textarea = textarea_input(Attrs::new(&[("id", string(&id)), ("class", string("wp-editor-area")), ("name", field.get("name").cloned().unwrap_or(Value::Null)), ("style", string(&format!("height:{height}px;"))), ("value", string("%s"))]));
    let textarea = textarea.replacen("%s", &value, 1);
    let eid = esc_attr(&id);
    let media = if field.truthy("media_upload") {
        format!(
            "\t\t\t\t<div id=\"wp-{eid}-media-buttons\" class=\"wp-media-buttons\">\n\t\t\t\t\t<button type=\"button\" id=\"insert-media-button\" class=\"button insert-media add_media\" data-editor=\"{eid}\" aria-haspopup=\"dialog\" aria-controls=\"wp-media-modal\"><span class=\"wp-media-buttons-icon\" aria-hidden=\"true\"></span> {}</button>\t\t\t\t</div>\n\t\t\t\t",
            ctx.site.tr("Add Media")
        )
    } else {
        String::new()
    };
    let tab_buttons = if ctx.rich_edit && show_tabs {
        format!(
            "\t\t\t\t\t<div class=\"wp-editor-tabs\">\n\t\t\t\t\t\t<button id=\"{eid}-tmce\" class=\"wp-switch-editor switch-tmce\" data-wp-editor-id=\"{eid}\" type=\"button\">{}</button>\n\t\t\t\t\t\t<button id=\"{eid}-html\" class=\"wp-switch-editor switch-html\" data-wp-editor-id=\"{eid}\" type=\"button\">{}</button>\n\t\t\t\t\t</div>\n\t\t\t\t",
            esc_html(&ctx.acf("Visual")),
            esc_html(&ctx.acfx("Text", "Name for the Text editor tab (formerly HTML)"))
        )
    } else {
        String::new()
    };
    let delay = if field.truthy("delay") { format!("\t\t\t\t\t<div class=\"acf-editor-toolbar\">{}</div>\n\t\t\t\t", esc_html(&ctx.acf("Click to initialize TinyMCE"))) } else { String::new() };
    format!(
        "\t\t<div {}>\n\t\t\t<div id=\"wp-{eid}-editor-tools\" class=\"wp-editor-tools hide-if-no-js\">\n\t\t\t\t{media}\t\t\t\t\t{tab_buttons}\t\t\t</div>\n\t\t\t<div id=\"wp-{eid}-editor-container\" class=\"wp-editor-container\">\n\t\t\t\t\t{delay}\t\t\t\t\t{textarea}\t\t\t</div>\n\t\t</div>\n\t\t\t",
        esc_attrs(&wrap)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_print_like_php() {
        assert_eq!(float_echo(1.8 + 3.0 * 0.7), "3.9");
        assert_eq!(float_echo(0.1 + 0.2), "0.3");
        assert_eq!(float_echo(2.5), "2.5");
        assert_eq!(float_echo(100.0), "100");
        assert_eq!(float_echo(-0.00001), "-1.0E-5");
        assert_eq!(float_echo(0.0001), "0.0001");
    }

    struct TestServices(std::path::PathBuf);

    impl rustpress_core::native::PageServices for TestServices {
        fn excerpt<'a>(&'a self, _post: &'a rustpress_db::Post) -> BoxFuture<'a, Result<String>> {
            Box::pin(async { Ok(String::new()) })
        }

        fn wp_root(&self) -> &std::path::Path {
            &self.0
        }
    }

    /// `json_decode( $json, true )`.
    fn php_value(json: &serde_json::Value) -> Value {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => n.as_i64().map_or_else(|| Value::Float(n.as_f64().unwrap_or(0.0)), Value::Int),
            serde_json::Value::String(s) => string(s),
            serde_json::Value::Array(items) => Value::Array(items.iter().enumerate().map(|(i, v)| (Key::Int(i as i64), php_value(v))).collect()),
            serde_json::Value::Object(map) => Value::Array(
                map.iter()
                    .map(|(k, v)| {
                        let key = match k.parse::<i64>() {
                            Ok(i) if i.to_string() == *k => Key::Int(i),
                            _ => Key::Str(k.as_bytes().to_vec()),
                        };
                        (key, php_value(v))
                    })
                    .collect(),
            ),
        }
    }

    fn field_of(json: &serde_json::Value) -> Field {
        let subs = json.get("sub_fields").and_then(|s| s.as_array()).map(|s| s.iter().map(field_of).collect()).unwrap_or_default();
        let mut settings = json.clone();
        if let Some(map) = settings.as_object_mut() {
            map.remove("sub_fields");
        }
        super::super::fields::from_settings(php_value(&settings), subs)
    }

    /// The inputs `dev/acf-render-golden.sh` had the plugin print.
    #[test]
    fn golden_inputs() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("acf-render");
        let cases: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).expect("cases")).expect("JSON");
        let options = rustpress_core::options::Options::from_pairs([("siteurl", "http://localhost:8080"), ("home", "http://localhost:8080"), ("date_format", "F j, Y"), ("time_format", "g:i a"), ("timezone_string", ""), ("gmt_offset", "0"), ("thumbnail_size_w", "150"), ("thumbnail_size_h", "150"), ("medium_size_w", "300"), ("medium_size_h", "300")]);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
        let _guard = runtime.enter();
        let site = Site::from_parts(rustpress_db::Db::connect_lazy("mysql://nobody@127.0.0.1:1/none", "wp_").expect("lazy pool"), options);
        let nonce = |_: &str| "NONCE".to_owned();
        let services = TestServices(std::path::PathBuf::from("."));
        let ctx = Ctx { site: &site, nonce: &nonce, services: &services, admin: false, uploader: "basic", wants_uploader: Default::default(), can: &|_| false, rich_edit: false, default_editor: "html", wysiwyg_height: 300 };
        let mut failures = Vec::new();
        let list = cases.as_array().expect("a list");
        for case in list {
            let name = case["name"].as_str().expect("name");
            let expected = std::fs::read_to_string(dir.join(format!("{name}.html"))).expect("expected output");
            let mut field = RenderField::load(&field_of(&case["field"]), &ctx);
            if let Some(value) = case.get("value").filter(|v| !v.is_null()) {
                field.set("value", php_value(value));
            }
            let element = case.get("element").and_then(|e| e.as_str()).unwrap_or("div");
            let instruction = case.get("instruction").and_then(|e| e.as_str()).unwrap_or("label");
            let actual = runtime.block_on(render_field_wrap(&ctx, field, element, instruction)).expect("render");
            if actual != expected {
                let at = actual.bytes().zip(expected.bytes()).position(|(a, b)| a != b).unwrap_or(actual.len().min(expected.len()));
                let from = at.saturating_sub(80);
                failures.push(format!(
                    "{name} (byte {at}):\n  expected …{:?}\n  actual   …{:?}",
                    expected.get(from..(at + 100).min(expected.len())).unwrap_or(""),
                    actual.get(from..(at + 100).min(actual.len())).unwrap_or("")
                ));
            }
        }
        assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), list.len(), failures.join("\n"));
    }

    #[test]
    fn ids_from_names() {
        assert_eq!(idify("acf[field_abc]"), "acf-field_abc");
        assert_eq!(idify("acf[field_G][field_S]"), "acf-field_g-field_s");
    }
}
