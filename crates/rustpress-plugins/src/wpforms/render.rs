//! The form on the website: `WPForms\Frontend\Frontend::output()` with its render engines `Classic` and `Modern`,
//! the honeypots (`WPForms\Forms\AntiSpam`, `WPForms\Forms\Honeypot`), the tokens (`WPForms\Forms\Token`) and the
//! CSS variables of the shortcode (`WPForms\Frontend\CSSVars::output_css_vars_for_shortcode()`).

use rustpress_core::formatting::{esc_attr, esc_html, esc_url, sanitize_html_class, wpautop};
use serde_json::{Value, json};

use super::fields;
use super::php::{self, empty, html_attributes, set, str_of, string};
use super::settings::Settings;

/// Where the form is printed and what the request brings along.
pub struct Env<'a> {
    pub settings: &'a Settings,
    /// `__( $text, 'wpforms-lite' )`.
    pub tr: &'a (dyn Fn(&str) -> String + Sync),
    /// `WPFORMS_PLUGIN_URL` (with the trailing slash).
    pub plugin_url: String,
    /// `esc_url_raw( remove_query_arg( 'wpforms' ) )` of the request.
    pub action: String,
    /// `Token::get( true, $form_data )`, `time()` and `Token::get_time_token()`.
    pub token: String,
    pub token_time: String,
    pub time_token: String,
    /// `wp_create_nonce( "wpforms::form_{$id}" )` for a signed-in visitor.
    pub nonce: Option<String>,
    /// The smart tags `{page_title}`, `{page_url}` and `{url_referer}` of the hidden inputs.
    pub page_title: String,
    pub page_url: String,
    pub url_referer: String,
    /// `get_the_ID()` when `is_singular()`.
    pub singular: Option<u64>,
    /// The field the honeypot is put in front of and its label (see [`honeypot_choice`]).
    pub honeypot: Option<(i64, String)>,
    /// The label of the old honeypot (`array_rand()` of six names).
    pub legacy_honeypot_label: String,
    /// The smart tags without a submission (`wpforms_process_smart_tags( $content, $form_data, [], '', $context )`).
    pub smart_tags: &'a (dyn Fn(&str, &Value, &str) -> String + Sync),
    /// `$_GET` for the dynamic population of the fields.
    pub get: &'a [(String, String)],
    /// What `wpforms_frontend_container_class` adds to the container's classes (the block's classes).
    pub container_classes: Vec<String>,
    /// After a submission without JavaScript: `$_POST['wpforms']`, the errors of the processed form and whether the
    /// form was processed without errors (the confirmation is printed instead).
    pub submitted: Option<&'a Submitted>,
}

/// A submission of the request (`wpforms()->obj( 'process' )`).
#[derive(Debug, Default, Clone)]
pub struct Submitted {
    pub form_id: i64,
    /// `$_POST['wpforms']`, as PHP builds it.
    pub post: Value,
    /// `$process->errors[ $form_id ]`.
    pub errors: Value,
    /// The confirmation message to print instead of the form, when it was processed without errors.
    pub confirmation: Option<String>,
    /// The confirmation's `message_scroll`.
    pub scroll: bool,
}

/// `wpforms_get_render_engine()`.
pub fn modern(settings: &Settings) -> bool {
    settings.set("modern-markup")
}

/// `(int) wpforms_setting( 'disable-css', '1' )`.
pub fn disable_css(settings: &Settings) -> i64 {
    php::leading_int(&settings.string("disable-css", "1"))
}

/// What a form leaves for the page: the honeypot field of the form (`AntiSpam::$forms_data`).
#[derive(Debug, Default, Clone)]
pub struct Rendered {
    pub html: String,
    /// The form was printed (`Frontend::$forms[ $form_id ]`).
    pub printed: bool,
    pub honeypot_field: Option<i64>,
}

/// The random choices of `AntiSpam::get_random_field()` and `AntiSpam::get_honeypot_label()`: the field the honeypot
/// goes in front of and three words of the labels in random order. `random(n)` is a number below `n`.
pub fn honeypot_choice(form_data: &Value, random: &mut dyn FnMut(usize) -> usize) -> (i64, String) {
    let fields: Vec<(&String, &Value)> = form_data.get("fields").and_then(Value::as_object).map(|m| m.iter().collect()).unwrap_or_default();
    let mut before = 1;
    if !fields.is_empty() {
        let key = fields[random(fields.len())].0;
        let id = php::leading_int(key);
        if key != "0" && !key.is_empty() {
            before = id;
        }
    }
    let labels: Vec<String> = fields.iter().filter(|(_, f)| set(f, "label")).map(|(_, f)| str_of(f, "label")).collect();
    let joined = labels.join(" ");
    let words: Vec<&str> = joined.split(' ').collect();
    let count = words.len().min(3);
    // array_rand() keeps the order of the array, shuffle() mixes it.
    let mut keys: Vec<usize> = Vec::new();
    let mut pool: Vec<usize> = (0..words.len()).collect();
    for _ in 0..count {
        let i = random(pool.len());
        keys.push(pool.remove(i));
    }
    keys.sort_unstable();
    for i in (1..keys.len()).rev() {
        let j = random(i + 1);
        keys.swap(i, j);
    }
    (before, keys.iter().map(|k| words[*k]).collect::<Vec<_>>().join(" "))
}

/// The form `$id` (its post, `post_status` and `post_content`), as `Frontend::output()` prints it.
pub struct Form<'a> {
    pub id: u64,
    pub status: &'a str,
    pub data: &'a Value,
}

/// `[wpforms id title description]`: `Frontend::shortcode()`.
pub fn shortcode(form: Option<&Form<'_>>, title: bool, description: bool, env: &Env<'_>, themes: &super::css_vars::Themes) -> Rendered {
    let mut out = String::new();
    if let Some(form) = form {
        out.push_str(&super::css_vars::for_shortcode(form.data, form.id, env.settings, themes));
    }
    let mut rendered = output(form, title, description, env);
    out.push_str(&rendered.html);
    rendered.html = out;
    rendered
}

/// `filter_var( $value, FILTER_VALIDATE_BOOLEAN )`.
pub fn validate_boolean(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes")
}

/// `Frontend::output( $id, $title, $description )`.
pub fn output(form: Option<&Form<'_>>, title: bool, description: bool, env: &Env<'_>) -> Rendered {
    let mut r = Rendered::default();
    let Some(form) = form else { return r };
    if form.status != "publish" {
        return r;
    }
    let mut form_data = form.data.clone();
    if empty(Some(&form_data)) {
        return r;
    }
    // `InternalInformation::remove_internal_fields_on_front_end()` on `wpforms_frontend_form_data`.
    if let Some(Value::Object(fields)) = form_data.get_mut("fields") {
        fields.retain(|_, f| str_of(f, "type") != "internal-information");
    }
    let form_id = form.id;
    let modern = modern(env.settings);
    let out = &mut r.html;
    let errors = env.submitted.filter(|s| s.form_id == form_id as i64).map(|s| s.errors.clone()).unwrap_or(Value::Null);

    // stop_output()
    if empty(form_data.get("fields")) {
        out.push_str("<!-- WPForms: no fields, form hidden -->");
        return r;
    }
    let mut action = env.action.clone();
    let has_upload = form_data["fields"].as_object().is_some_and(|m| m.values().any(|f| str_of(f, "type") == "file-upload"));
    if has_upload {
        action = rustpress_core::urls::add_query_arg(&action, "wpforms_form_id", &form_id.to_string());
    }
    if let Some(sub) = env.submitted.filter(|s| s.form_id == form_id as i64 && empty(Some(&s.errors)))
        && let Some(message) = &sub.confirmation {
            container_open(out, &form_data, form_id, env, modern);
            confirmation(out, message, sub.scroll, form_id, env);
            container_close(out);
            return r;
        }
    r.printed = true;
    let settings = &form_data["settings"];
    let mut form_classes = vec!["wpforms-validate", "wpforms-form"];
    if set(settings, "ajax_submit") {
        form_classes.push("wpforms-ajax-form");
    }
    container_open(out, &form_data, form_id, env, modern);
    let atts = json!({
        "method": "post",
        "enctype": "multipart/form-data",
        "action": esc_url(&action),
        "data-token": env.token,
        "data-token-time": env.token_time,
    });
    out.push_str(&format!("<form {}>", html_attributes(&format!("wpforms-form-{form_id}"), &json!(form_classes), &json!({"formid": form_id}), &atts)));

    // `wpforms_frontend_output`: head (5), fields (10), the old honeypot (15), reCAPTCHA (20), foot (25).
    if title || description {
        head_container(out, &form_data, title, description, env);
    }
    let noscript = (env.tr)("Please enable JavaScript in your browser to complete this form.");
    if modern {
        out.push_str(&format!("<noscript class=\"wpforms-error-noscript\">{0}</noscript><div id=\"wpforms-error-noscript\" style=\"display: none;\">{0}</div>", esc_html(&noscript)));
    } else {
        out.push_str(&format!("<noscript class=\"wpforms-error-noscript\">{}</noscript>", esc_html(&noscript)));
    }
    if set(&errors, "header") {
        form_error(out, "header", &string(&errors["header"]), form_id, env, modern);
    }
    let mut rendered_fields = Vec::new();
    let honeypot_enabled = set(settings, "antispam_v3");
    out.push_str("<div class=\"wpforms-field-container\">");
    let fields: Vec<Value> = form_data["fields"].as_object().map(|m| m.values().cloned().collect()).unwrap_or_default();
    for field in fields {
        if let Some(id) = render_field(out, &form_data, field, env, modern, &errors, honeypot_enabled) {
            rendered_fields.push(id);
        }
    }
    // `AntiSpam::maybe_insert_honeypot_init_js()` on `wpforms_display_fields_after`.
    if honeypot_enabled
        && let Some((before, _)) = &env.honeypot
            && rendered_fields.contains(before) {
                let hp = honeypot_field_id(&form_data);
                r.honeypot_field = Some(hp);
                let ids = format!("#wpforms-{form_id}-field_{hp}-container");
                let styles = format!(
                    "{0} {{ position: absolute !important; overflow: hidden !important; display: inline !important; height: 1px !important; width: 1px !important; z-index: -1000 !important; padding: 0 !important; }} {0} input {{ visibility: hidden; }} #wpforms-conversational-form-page {0} label {{ counter-increment: none; }}",
                    esc_attr(&ids)
                );
                out.push_str(&format!(
                    "<script>\n\t\t\t\t( function() {{\n\t\t\t\t\tconst style = document.createElement( 'style' );\n\t\t\t\t\tstyle.appendChild( document.createTextNode( '{}' ) );\n\t\t\t\t\tdocument.head.appendChild( style );\n\t\t\t\t\tdocument.currentScript?.remove();\n\t\t\t\t}} )();\n\t\t\t</script>",
                    php::esc_js(&styles)
                ));
            }
    out.push_str("</div><!-- .wpforms-field-container -->");
    // `Honeypot::render()`.
    if str_of(settings, "honeypot") == "1" {
        out.push_str("<div class=\"wpforms-field wpforms-field-hp\">");
        out.push_str(&format!("<label for=\"wpforms-{0}-field-hp\" class=\"wpforms-field-label\">{1}</label>", str_of(&form_data, "id"), env.legacy_honeypot_label));
        out.push_str(&format!("<input type=\"text\" name=\"wpforms[hp]\" id=\"wpforms-{}-field-hp\" class=\"wpforms-field-medium\">", str_of(&form_data, "id")));
        out.push_str("</div>");
    }
    if !rendered_fields.is_empty() {
        foot(out, &form_data, form_id, env, modern, &errors);
    }
    out.push_str("</form>");
    container_close(out);
    r
}

/// `AntiSpam::get_honeypot_field_id()`: the first free field ID.
pub fn honeypot_field_id(form_data: &Value) -> i64 {
    let keys: Vec<i64> = form_data["fields"].as_object().map(|m| m.keys().map(|k| php::leading_int(k)).collect()).unwrap_or_default();
    let max = keys.iter().copied().max().unwrap_or(0);
    (1..=max).find(|i| !keys.contains(i)).unwrap_or(max + 1)
}

/// `Frontend::get_container_classes()` and the render engine's `form_container_open()`.
fn container_open(out: &mut String, form_data: &Value, form_id: u64, env: &Env<'_>, modern: bool) {
    let mut classes: Vec<String> = if disable_css(env.settings) == 1 { vec!["wpforms-container-full".into()] } else { Vec::new() };
    for class in &env.container_classes {
        if !classes.contains(class) {
            classes.push(class.clone());
        }
    }
    let form_class = str_of(&form_data["settings"], "form_class");
    if set(&form_data["settings"], "form_class") {
        classes.extend(form_class.split(' ').map(str::to_owned));
    }
    if modern {
        classes.push("wpforms-render-modern".into());
    }
    out.push_str(&format!("<div class=\"wpforms-container {}\" id=\"wpforms-{}\">", php::sanitize_classes(&json!(classes)), form_id));
}

fn container_close(out: &mut String) {
    out.push_str("</div>  <!-- .wpforms-container -->");
}

/// `Classic::confirmation()` from `Frontend::confirmation()`.
fn confirmation(out: &mut String, message: &str, scroll: bool, form_id: u64, env: &Env<'_>) {
    let mut class = if disable_css(env.settings) == 1 { "wpforms-confirmation-container-full".to_owned() } else { "wpforms-confirmation-container".to_owned() };
    if scroll {
        class.push_str(" wpforms-confirmation-scroll");
    }
    out.push_str(&format!("<div class=\"{}\" id=\"wpforms-confirmation-{}\" tabindex=\"-1\">{}</div>", php::sanitize_class_string(&class), form_id, message));
}

/// `Classic::form_head_container()`.
fn head_container(out: &mut String, form_data: &Value, title: bool, description: bool, env: &Env<'_>) {
    let settings = &form_data["settings"];
    out.push_str("<div class=\"wpforms-head-container\">");
    if title && set(settings, "form_title") {
        out.push_str(&format!("<div class=\"wpforms-title\">{}</div>", esc_html(&str_of(settings, "form_title"))));
    }
    if description && set(settings, "form_desc") {
        out.push_str("<div class=\"wpforms-description\">");
        out.push_str(&(env.smart_tags)(&str_of(settings, "form_desc"), form_data, "form-description"));
        out.push_str("</div>");
    }
    out.push_str("</div>");
}

/// `wpforms_sanitize_error()`.
pub fn sanitize_error(error: &str) -> String {
    use rustpress_core::kses::{Allowed, kses};
    static ALLOWED: std::sync::OnceLock<Allowed> = std::sync::OnceLock::new();
    let allowed = ALLOWED.get_or_init(|| {
        Allowed::tags(&[
            ("a", &["href", "title", "target", "rel"]),
            ("br", &[]),
            ("em", &[]),
            ("strong", &[]),
            ("del", &[]),
            ("p", &["style"]),
            ("blockquote", &[]),
            ("ul", &[]),
            ("ol", &[]),
            ("li", &[]),
            ("span", &["style"]),
        ])
    });
    kses(error, allowed)
}

/// The render engine's `form_error( $type, $error )`.
pub fn form_error(out: &mut String, kind: &str, error: &str, form_id: u64, env: &Env<'_>, modern: bool) {
    out.push_str(&form_error_html(kind, error, &form_id.to_string(), env.tr, modern));
}

/// `Classic::form_error()` and `Modern::form_error()` of the form with the ID `$form_data['id']`.
pub fn form_error_html(kind: &str, error: &str, form_id: &str, tr: &(dyn Fn(&str) -> String + Sync), modern: bool) -> String {
    let styled = matches!(kind, "header_styled" | "footer_styled");
    if !styled && !matches!(kind, "header" | "footer") {
        return String::new();
    }
    if !modern {
        return if styled {
            format!("<div class=\"wpforms-error-container wpforms-error-styled-container\"><div class=\"wpforms-error\">{}</div></div>", wpautop(&sanitize_error(error)))
        } else {
            format!("<div class=\"wpforms-error-container\">{}</div>", wpautop(&sanitize_error(error)))
        };
    }
    let (class, open, close) = if styled { (" wpforms-error-styled-container", "<div class=\"wpforms-error\">", "</div>") } else { ("", "", "") };
    format!(
        "<div id=\"wpforms-{}-{}-error\" class=\"wpforms-error-container{class}\" role=\"alert\">\n\t\t\t\t\t\t{open}<span class=\"wpforms-hidden\" aria-hidden=\"false\">{}</span>{}{close}\n\t\t\t\t\t</div>",
        esc_attr(form_id),
        esc_attr(kind),
        esc_html(&tr("Form error message")),
        wpautop(&sanitize_error(error))
    )
}

/// `Frontend::get_field_properties( $field, $form_data )` outside the printing of a form (an AJAX request without
/// `$_GET` and without errors).
pub fn field_props(field: &Value, form_data: &Value, settings: &Settings, tr: &(dyn Fn(&str) -> String + Sync)) -> Value {
    let smart_tags = |content: &str, _: &Value, _: &str| content.to_owned();
    let env = Env {
        settings,
        tr,
        plugin_url: String::new(),
        action: String::new(),
        token: String::new(),
        token_time: String::new(),
        time_token: String::new(),
        nonce: None,
        page_title: String::new(),
        page_url: String::new(),
        url_referer: String::new(),
        singular: None,
        honeypot: None,
        legacy_honeypot_label: String::new(),
        smart_tags: &smart_tags,
        get: &[],
        container_classes: Vec::new(),
        submitted: None,
    };
    let Some(field) = fields::field_data(field.clone(), settings) else { return Value::Null };
    let ctx = Ctx { env: &env, form_data, form_id: php::int(form_data.get("id")), modern: modern(settings), errors: &Value::Null };
    let attributes = field_attributes(&field, &ctx);
    field_properties(&field, &attributes, &ctx)
}

/// `wpforms_get_field_required_label()` of the render engine.
pub fn required_label(modern: bool) -> &'static str {
    if modern { " <span class=\"wpforms-required-label\" aria-hidden=\"true\">*</span>" } else { " <span class=\"wpforms-required-label\">*</span>" }
}

/// What the field functions see of the form being printed.
pub struct Ctx<'a, 'b> {
    pub env: &'a Env<'b>,
    pub form_data: &'a Value,
    pub form_id: i64,
    pub modern: bool,
    pub errors: &'a Value,
}

impl Ctx<'_, '_> {
    pub fn tr(&self, text: &str) -> String {
        (self.env.tr)(text)
    }
}

/// `Frontend::render_field()`: the field with its container, label, description and error. The field's ID when it
/// was printed.
fn render_field(out: &mut String, form_data: &Value, field: Value, env: &Env<'_>, modern: bool, errors: &Value, honeypot_enabled: bool) -> Option<i64> {
    let ty = str_of(&field, "type");
    if !fields::is_known(&ty) {
        return None;
    }
    let field = fields::field_data(field, env.settings)?;
    let form_id = php::int(form_data.get("id"));
    let ctx = Ctx { env, form_data, form_id, modern, errors };
    let field_id = php::int(field.get("id"));
    let attributes = field_attributes(&field, &ctx);
    let mut field = field;
    let properties = field_properties(&field, &attributes, &ctx);
    field["properties"] = properties;

    // `wpforms_display_field_before`: the honeypot (1), the container (5), the fieldset (10), the label (15) and a
    // description before the field (20).
    if honeypot_enabled
        && let Some((before, label)) = &env.honeypot
            && *before == field_id {
                let hp = honeypot_field_id(form_data);
                let id_attr = format!("wpforms-{form_id}-field_{hp}");
                out.push_str(&format!(
                    "\t\t<div id=\"{0}-container\"\n\t\t\tclass=\"wpforms-field wpforms-field-text\"\n\t\t\tdata-field-type=\"text\"\n\t\t\tdata-field-id=\"{1}\"\n\t\t\t>\n\t\t\t<label class=\"wpforms-field-label\" for=\"{0}\" >{2}</label>\n\t\t\t<input type=\"text\" id=\"{0}\" class=\"wpforms-field-medium\" name=\"wpforms[fields][{1}]\" >\n\t\t</div>\n\t\t",
                    esc_attr(&id_attr),
                    esc_attr(&hp.to_string()),
                    esc_html(label)
                ));
            }
    let props = &field["properties"];
    let mut container = props["container"].clone();
    container["data"]["field-id"] = json!(field_id);
    out.push_str(&format!("<div {}>", html_attributes(&string(&container["id"]), &container["class"], &container["data"], &container["attr"])));
    let fieldset = modern && fields::requires_fieldset(&field);
    if fieldset {
        let mut label = props["label"].clone();
        if set(&field, "label_hide") {
            label["attr"]["aria-hidden"] = json!("false");
        }
        let required = if label["required"].as_bool() == Some(true) { required_label(modern) } else { "" };
        if let Value::Object(attr) = &mut label["attr"] {
            attr.remove("for");
        }
        out.push_str(&format!("<fieldset><legend {}>{}{}</legend>", html_attributes(&string(&label["id"]), &label["class"], &label["data"], &label["attr"]), esc_html(&string(&label["value"])), required));
    }
    let label = &props["label"];
    if !empty(label.get("value")) && label["disabled"].as_bool() != Some(true) && !fieldset {
        let mut label = label.clone();
        if modern && set(&field, "label_hide") {
            label["attr"]["aria-hidden"] = json!("false");
        }
        let required = if label["required"].as_bool() == Some(true) { required_label(modern) } else { "" };
        out.push_str(&format!("<label {}>{}{}</label>", html_attributes(&string(&label["id"]), &label["class"], &label["data"], &label["attr"]), esc_html(&string(&label["value"])), required));
    }
    let description = |out: &mut String, position: &str| {
        let d = &props["description"];
        if empty(d.get("value")) || str_of(d, "position") != position {
            return;
        }
        out.push_str(&format!("<div {}>{}</div>", html_attributes(&string(&d["id"]), &d["class"], &d["data"], &d["attr"]), string(&d["value"])));
    };
    description(out, "before");

    fields::display(out, &field, &ctx);

    // `wpforms_display_field_after`: the error (3), a description after the field (5), the fieldset (10) and the
    // container (15).
    let error = &props["error"];
    if !empty(error.get("value")) && !error["value"].is_array() && !error["value"].is_object() {
        let tag = if modern { "em" } else { "label" };
        out.push_str(&format!("<{tag} {}>{}</{tag}>", html_attributes(&string(&error["id"]), &error["class"], &error["data"], &error["attr"]), esc_html(&string(&error["value"]))));
    }
    description(out, "after");
    if fieldset {
        out.push_str("</fieldset>");
    }
    out.push_str("</div>");
    Some(field_id)
}

/// `Frontend::get_field_attributes()`.
fn field_attributes(field: &Value, ctx: &Ctx<'_, '_>) -> Value {
    let form_id = ctx.form_id;
    let field_id = php::int(field.get("id"));
    let ty = str_of(field, "type");
    let mut field_class = vec![json!("wpforms-field"), json!(format!("wpforms-field-{}", sanitize_html_class(&ty)))];
    if set(field, "css") {
        // `wpforms_sanitize_classes( $field['css'], true )` of a string: its classes as an array.
        let css = str_of(field, "css");
        let mut seen: Vec<String> = Vec::new();
        for c in css.trim().split(' ') {
            if !seen.iter().any(|s| s == c) {
                seen.push(c.to_owned());
            }
        }
        field_class.extend(seen.into_iter().filter(|c| !c.is_empty() && c != "0").map(|c| json!(sanitize_html_class(&c))));
    }
    match str_of(field, "input_columns").as_str() {
        "2" => field_class.push(json!("wpforms-list-2-columns")),
        "3" => field_class.push(json!("wpforms-list-3-columns")),
        "inline" => field_class.push(json!("wpforms-list-inline")),
        _ => {}
    }
    let mut label_class = vec![json!("wpforms-field-label")];
    if set(field, "label_hide") {
        label_class.push(json!("wpforms-label-hide"));
    }
    let mut input_class = Vec::new();
    if set(field, "size") {
        input_class.push(json!(format!("wpforms-field-{}", sanitize_html_class(&str_of(field, "size")))));
    }
    if set(field, "required") {
        input_class.push(json!("wpforms-field-required"));
    }
    if set(ctx.errors, &field_id.to_string()) {
        input_class.push(json!("wpforms-error"));
    }
    json!({
        "field_class": field_class,
        "field_id": [format!("wpforms-{form_id}-field_{field_id}-container")],
        "field_style": "",
        "label_class": label_class,
        "label_id": "",
        "description_class": ["wpforms-field-description"],
        "description_id": [],
        "input_id": [format!("wpforms-{form_id}-field_{field_id}")],
        "input_class": input_class,
        "input_data": {},
    })
}

/// `Frontend::get_field_properties()` with the filters `wpforms_field_properties_{$type}` (the field) and
/// `wpforms_field_properties` (the Modern engine, the population, read-only).
fn field_properties(field: &Value, attributes: &Value, ctx: &Ctx<'_, '_>) -> Value {
    let form_id = ctx.form_id;
    let field_id = php::int(field.get("id"));
    let error = ctx.errors.get(field_id.to_string()).filter(|e| !empty(Some(e))).cloned().unwrap_or(json!(""));
    let default_value = if field.get("default_value").is_some_and(|v| !v.is_null()) { (ctx.env.smart_tags)(&str_of(field, "default_value"), ctx.form_data, "field-properties") } else { String::new() };
    let description = if set(field, "description") { (ctx.env.smart_tags)(&str_of(field, "description"), ctx.form_data, "field-properties") } else { String::new() };
    let mut properties = json!({
        "container": {
            "attr": {"style": attributes["field_style"]},
            "class": attributes["field_class"],
            "data": {},
            "id": attributes["field_id"].as_array().map(|a| a.iter().map(string).collect::<String>()).unwrap_or_default(),
        },
        "label": {
            "attr": {"for": format!("wpforms-{form_id}-field_{field_id}")},
            "class": attributes["label_class"],
            "data": {},
            "disabled": set(field, "label_disable"),
            "hidden": set(field, "label_hide"),
            "id": attributes["label_id"],
            "required": set(field, "required"),
            "value": if set(field, "label") { field["label"].clone() } else { json!("") },
        },
        "inputs": {
            "primary": {
                "attr": {
                    "name": format!("wpforms[fields][{field_id}]"),
                    "value": default_value,
                    "placeholder": field.get("placeholder").filter(|v| !v.is_null()).cloned().unwrap_or(json!("")),
                },
                "class": attributes["input_class"],
                "data": attributes["input_data"],
                "id": attributes["input_id"].as_array().map(|a| a.iter().map(string).collect::<String>()).unwrap_or_default(),
                "required": if set(field, "required") { "required" } else { "" },
            },
        },
        "error": {
            "attr": {"for": format!("wpforms-{form_id}-field_{field_id}")},
            "class": ["wpforms-error"],
            "data": {},
            "id": "",
            "value": error,
        },
        "description": {
            "attr": {},
            "class": attributes["description_class"],
            "data": {},
            "id": attributes["description_id"].as_array().map(|a| a.iter().map(string).collect::<String>()).unwrap_or_default(),
            "position": "after",
            "value": description,
        },
    });
    properties = fields::properties(properties, field, ctx);
    // `Modern::field_properties()` (10).
    if ctx.modern {
        let id = format!("wpforms-{form_id}-field_{}", str_of(field, "id"));
        let desc_id = format!("{id}-description");
        properties["description"]["id"] = json!(desc_id);
        properties["error"]["attr"]["role"] = json!("alert");
        let for_ = string(&properties["error"]["attr"]["for"]);
        properties["error"]["attr"]["id"] = json!(format!("{for_}-error"));
        properties["error"]["attr"]["for"] = json!("");
        let ty = str_of(field, "type");
        if let Some(Value::Object(inputs)) = properties.get_mut("inputs") {
            for input in inputs.values_mut() {
                let input_id = string(&input["id"]);
                if !empty(input.get("id")) && ty != "hidden" {
                    php::entry(input, "attr")["aria-errormessage"] = json!(format!("{input_id}-error"));
                }
                if set(field, "description") {
                    php::entry(input, "attr")["aria-describedby"] = json!(desc_id);
                }
            }
        }
    }
    // `WPForms_Field::field_prefill_value_property()` (10).
    properties = fields::prefill(properties, field, ctx);
    // `ReadOnlyField::read_only_field_properties()` (100).
    if set(field, "read_only") && fields::allows_read_only(&str_of(field, "type")) {
        php::push(php::entry(&mut properties["container"], "class"), "wpforms-field-readonly");
    }
    properties
}

/// The footer of the form: `Frontend::foot()`.
fn foot(out: &mut String, form_data: &Value, form_id: u64, env: &Env<'_>, modern: bool, errors: &Value) {
    let settings = &form_data["settings"];
    let submit = if set(settings, "submit_text") { str_of(settings, "submit_text") } else { (env.tr)("Submit") };
    let mut attrs = serde_json::Map::new();
    attrs.insert("aria-live".into(), json!("assertive"));
    attrs.insert("value".into(), json!("wpforms-submit"));
    let mut data = serde_json::Map::new();
    let mut classes = vec![json!("wpforms-submit")];
    if set(settings, "submit_text_processing") {
        data.insert("alt-text".into(), settings["submit_text_processing"].clone());
        data.insert("submit-text".into(), json!(submit));
    }
    if set(settings, "submit_class") {
        match &settings["submit_class"] {
            Value::Array(list) => classes.extend(list.iter().cloned()),
            other => classes.extend(string(other).split(' ').filter(|c| !c.is_empty() && *c != "0").map(|c| json!(c))),
        }
    }
    if set(errors, "footer") {
        form_error(out, "footer", &string(&errors["footer"]), form_id, env, modern);
    }
    out.push_str("<div class=\"wpforms-submit-container\" >");
    out.push_str(&format!("<input type=\"hidden\" name=\"wpforms[id]\" value=\"{form_id}\">"));
    out.push_str(&format!("<input type=\"hidden\" name=\"wpforms[time_token]\" value=\"{}\">", esc_attr(&env.time_token)));
    if let Some(nonce) = &env.nonce {
        out.push_str(&format!("\t\t\t<input\n\t\t\t\ttype=\"hidden\"\n\t\t\t\tname=\"wpforms[nonce]\"\n\t\t\t\tvalue=\"{}\"\n\t\t\t/>\n\t\t\t", esc_attr(nonce)));
    }
    out.push_str(&format!("<input type=\"hidden\" name=\"page_title\" value=\"{}\">", esc_attr(&env.page_title)));
    out.push_str(&format!("<input type=\"hidden\" name=\"page_url\" value=\"{}\">", esc_url(&env.page_url)));
    out.push_str(&format!("<input type=\"hidden\" name=\"url_referer\" value=\"{}\">", esc_url(&env.url_referer)));
    if let Some(id) = env.singular {
        out.push_str(&format!("<input type=\"hidden\" name=\"page_id\" value=\"{id}\">"));
        out.push_str(&format!("<input type=\"hidden\" name=\"wpforms[post_id]\" value=\"{id}\">"));
    }
    out.push_str(&format!(
        "<button type=\"submit\" name=\"wpforms[submit]\" {}>{}</button>",
        html_attributes(&format!("wpforms-submit-{form_id}"), &Value::Array(classes), &Value::Object(data), &Value::Object(attrs)),
        esc_html(&submit)
    ));
    if set(settings, "ajax_submit") {
        out.push_str(&format!(
            "<img src=\"{}\" class=\"wpforms-submit-spinner\" style=\"display: none;\" width=\"26\" height=\"26\" alt=\"{}\">",
            esc_url(&format!("{}assets/images/submit-spin.svg", env.plugin_url)),
            esc_attr(&(env.tr)("Loading"))
        ));
    }
    out.push_str("</div>");
}
