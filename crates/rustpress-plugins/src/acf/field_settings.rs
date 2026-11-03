//! The settings of the field types on the field group editor: the `render_field_settings()`,
//! `render_field_validation_settings()`, `render_field_presentation_settings()` and `render_field_advanced_settings()`
//! methods of ACF's field types (free), as the settings `acf_render_field_setting()` gets, with the markup some of
//! them print in between.

use rustpress_core::Site;
use rustpress_phpser::{Key, Value};

use super::php::{key, string, strval, truthy};
use super::render::RenderField;

/// What a settings method needs from the request.
pub struct Env<'a> {
    pub site: &'a Site,
    /// The renderer (the titles of terms).
    pub render: &'a super::render::Ctx<'a>,
    /// The label of a field by its key (`get_field_object()`), for the targets of bidirectional fields.
    pub field_label: &'a (dyn Fn(&str) -> Option<String> + Send + Sync),
    /// `time()`, for the examples of the date formats (`date_i18n()`).
    pub now: i64,
    /// `current_user_can( 'upload_files' )` (the tabs of the icon picker).
    pub can_upload: bool,
    /// `wp_theme_has_theme_json()`.
    pub theme_json: bool,
}

/// One thing a settings method prints.
pub enum Part {
    /// `acf_render_field_setting( $field, $setting )`.
    Setting(Vec<(Key, Value)>),
    /// Markup it echoes.
    Html(String),
    /// The sub fields of a group (`acf-field-setting-sub_fields`).
    SubFields,
}

/// The tabs of a field's settings (`acf_get_combined_field_type_settings_tabs()`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    General,
    Validation,
    Presentation,
    ConditionalLogic,
    Advanced,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::General, Tab::Validation, Tab::Presentation, Tab::ConditionalLogic, Tab::Advanced];

    pub fn key(self) -> &'static str {
        match self {
            Tab::General => "general",
            Tab::Validation => "validation",
            Tab::Presentation => "presentation",
            Tab::ConditionalLogic => "conditional_logic",
            Tab::Advanced => "advanced",
        }
    }

    pub fn label(self, site: &Site) -> String {
        t_str(
            site,
            match self {
                Tab::General => "General",
                Tab::Validation => "Validation",
                Tab::Presentation => "Presentation",
                Tab::ConditionalLogic => "Conditional Logic",
                Tab::Advanced => "Advanced",
            },
        )
    }
}

fn t_str(site: &Site, text: &str) -> String {
    site.trd("acf", text)
}

/// `__( $text, 'acf' )`.
fn t(site: &Site, text: &str) -> Value {
    string(&t_str(site, text))
}

/// `_x( $text, $context, 'acf' )`.
fn tx(site: &Site, text: &str, context: &str) -> Value {
    string(&site.i18n().pgettext("acf", context, text))
}

/// `esc_html__( $text, 'acf' )`.
fn esc_t(site: &Site, text: &str) -> Value {
    string(&rustpress_core::formatting::esc_html(&t_str(site, text)))
}

/// An array with string keys.
fn arr(items: Vec<(&str, Value)>) -> Value {
    Value::Array(items.into_iter().map(|(k, v)| (key(k), v)).collect())
}

/// A list.
fn list(items: Vec<Value>) -> Value {
    Value::Array(items.into_iter().enumerate().map(|(i, v)| (Key::Int(i as i64), v)).collect())
}

/// `.` of strings.
fn cat(parts: Vec<Value>) -> Value {
    string(&parts.iter().map(|p| strval(Some(p))).collect::<String>())
}

fn set(items: Vec<(&str, Value)>) -> Part {
    Part::Setting(items.into_iter().map(|(k, v)| (key(k), v)).collect())
}

fn html(text: &str) -> Part {
    Part::Html(text.to_owned())
}

/// The instructions of the choices settings.
fn choices_instructions(site: &Site) -> Value {
    cat(vec![t(site, "Enter each choice on a new line."), string("<br />"), t(site, "For more control, you may specify both a value and label like this:"), string("<br /><span class=\"acf-field-setting-example\">"), t(site, "red : Red"), string("</span>")])
}

/// `acf_encode_choices( $array, $show_keys )`.
pub fn encode_choices(value: &Value, show_keys: bool) -> Value {
    let Value::Array(items) = value else { return value.clone() };
    if items.is_empty() {
        return string("");
    }
    let lines: Vec<String> = items
        .iter()
        .map(|(k, v)| {
            let k = super::render::key_text(k);
            let v = strval(Some(v));
            if show_keys && k != v { format!("{k} : {v}") } else { v }
        })
        .collect();
    string(&lines.join("\n"))
}

/// `empty( $field[ $k ] )` makes the setting `''`.
fn clear(field: &mut RenderField, names: &[&str]) {
    for name in names {
        if !truthy(field.get(name)) {
            field.set(name, string(""));
        }
    }
}

/// `date_i18n( $format )` now.
fn date(env: &Env<'_>, format: &str) -> Value {
    string(&env.site.date(format, env.now))
}

/// The radio choices of a date format: an example and the format.
fn format_choice(env: &Env<'_>, format: &str) -> (String, Value) {
    (format.to_owned(), cat(vec![string("<span>"), date(env, format), string(&format!("</span><code>{format}</code>"))]))
}

fn format_choices(env: &Env<'_>, formats: &[&str]) -> Value {
    let mut items: Vec<(Key, Value)> = formats.iter().map(|f| format_choice(env, f)).map(|(k, v)| (key(&k), v)).collect();
    items.push((key("other"), cat(vec![string("<span>"), t(env.site, "Custom:"), string("</span>")])));
    Value::Array(items)
}

/// `array_values( $wp_locale->weekday )`.
fn weekdays(site: &Site) -> Value {
    list(site.locale_data().weekday.iter().map(|d| string(d)).collect())
}

/// `acf_get_pretty_post_types()`.
pub(super) fn pretty_post_types(site: &Site) -> Value {
    let items: Vec<(String, String)> = super::format::all_post_types(site).into_iter().map(|t| (t.clone(), super::render::post_type_singular(site, &t))).collect();
    Value::Array(super::render::disambiguate(items).into_iter().map(|(k, v)| (key(&k), string(&v))).collect())
}

/// `acf_get_pretty_post_statuses()`: the statuses that are not internal, except ACF's.
fn pretty_post_statuses(site: &Site) -> Value {
    let statuses = [("publish", "Published"), ("future", "Scheduled"), ("draft", "Draft"), ("pending", "Pending"), ("private", "Private")];
    let items: Vec<(String, String)> = statuses.iter().map(|(k, label)| ((*k).to_owned(), site.trx(label, "post status"))).collect();
    Value::Array(super::render::disambiguate(items).into_iter().map(|(k, v)| (key(&k), string(&v))).collect())
}

/// `acf_get_taxonomy_labels()` of every taxonomy.
pub(super) fn taxonomy_labels(site: &Site) -> Value {
    let taxonomies = super::render::acf_taxonomies(site, None);
    Value::Array(super::render::taxonomy_singulars(site, &taxonomies).into_iter().map(|(k, v)| (key(&k), string(&v))).collect())
}

/// `acf_get_user_role_labels()`.
fn user_roles(site: &Site) -> Value {
    Value::Array(super::render::user_role_names(site).into_iter().map(|(k, v)| (key(&k), string(&v))).collect())
}

/// `acf_get_image_sizes()`.
fn image_sizes(site: &Site) -> Value {
    Value::Array(super::render::image_size_choices(site).into_iter().map(|(k, v)| (key(&k), string(&v))).collect())
}

/// `acf_build_bidirectional_target_current_choices()`: the chosen targets with their labels.
fn bidirectional_choices(env: &Env<'_>, field: &RenderField) -> Value {
    let Some(Value::Array(targets)) = field.get("bidirectional_target") else { return Value::Array(Vec::new()) };
    Value::Array(
        targets
            .iter()
            .filter_map(|(_, v)| match v {
                Value::Str(_) if truthy(Some(v)) => Some(strval(Some(v))),
                _ => None,
            })
            .map(|target| {
                let label = (env.field_label)(&target).filter(|l| !l.is_empty()).unwrap_or_else(|| target.clone());
                (key(&target), string(&label))
            })
            .collect(),
    )
}

/// `acf_render_bidirectional_field_settings()` (`enable_bidirection` is on).
fn bidirectional(env: &Env<'_>, field: &mut RenderField, out: &mut Vec<Part>) {
    let site = env.site;
    out.push(set(vec![("label", t(site, "Bidirectional")), ("instructions", t(site, "Update a field on the selected values, referencing back to this ID")), ("type", string("true_false")), ("name", string("bidirectional")), ("ui", Value::Int(1))]));
    let message = format!(
        "<p class=\"acf-feature-notice with-warning-icon\">{}</p>",
        rustpress_core::i18n::sprintf(
            &t_str(site, "Enabling the bidirectional setting allows you to update a value in the target fields for each value selected for this field, adding or removing the Post ID, Taxonomy ID or User ID of the item being updated. For more information, please read the <a href=\"%s\" target=\"_blank\">documentation</a>."),
            &[&super::admin_lists::utm("https://www.advancedcustomfields.com/resources/bidirectional-relationships/", "docs", "bidirectional", "", "", "")]
        )
    );
    out.push(set(vec![("name", string("bidirectional_notes")), ("type", string("message")), ("message", string(&message)), ("conditions", arr(vec![("field", string("bidirectional")), ("operator", string("==")), ("value", Value::Int(1))]))]));
    let choices = bidirectional_choices(env, field);
    out.push(set(vec![
        ("type", string("select")),
        ("name", string("bidirectional_target")),
        ("label", t(site, "Target Field")),
        ("instructions", t(site, "Select field(s) to store the reference back to the item being updated. You may select this field. Target fields must be compatible with where this field is being displayed. For example, if this field is displayed on a Taxonomy, your target field should be of type Taxonomy")),
        ("class", string("bidrectional_target")),
        ("choices", choices),
        ("conditions", arr(vec![("field", string("bidirectional")), ("operator", string("==")), ("value", Value::Int(1))])),
        ("ui", Value::Int(1)),
        ("multiple", Value::Int(1)),
        ("ajax", Value::Int(1)),
    ]));
}

/// The filters of the relational types: post type, post status and taxonomy.
async fn post_filters(env: &Env<'_>, out: &mut Vec<Part>) -> rustpress_core::Result<()> {
    let site = env.site;
    out.push(set(vec![("label", t(site, "Filter by Post Type")), ("instructions", string("")), ("type", string("select")), ("name", string("post_type")), ("choices", pretty_post_types(site)), ("multiple", Value::Int(1)), ("ui", Value::Int(1)), ("allow_null", Value::Int(1)), ("placeholder", t(site, "All post types"))]));
    out.push(set(vec![("label", t(site, "Filter by Post Status")), ("instructions", string("")), ("type", string("select")), ("name", string("post_status")), ("choices", pretty_post_statuses(site)), ("multiple", Value::Int(1)), ("ui", Value::Int(1)), ("allow_null", Value::Int(1)), ("placeholder", t(site, "Any post status"))]));
    out.push(set(vec![("label", t(site, "Filter by Taxonomy")), ("instructions", string("")), ("type", string("select")), ("name", string("taxonomy")), ("choices", super::render::taxonomy_terms(env.render).await?), ("multiple", Value::Int(1)), ("ui", Value::Int(1)), ("allow_null", Value::Int(1)), ("placeholder", t(site, "All taxonomies"))]));
    Ok(())
}

fn allow_null(site: &Site, out: &mut Vec<Part>) {
    out.push(set(vec![("label", t(site, "Allow Null")), ("instructions", string("")), ("name", string("allow_null")), ("type", string("true_false")), ("ui", Value::Int(1))]));
}

fn select_multiple(out: &mut Vec<Part>) {
    out.push(set(vec![("label", string("Select Multiple")), ("instructions", string("Allow content editors to select multiple values")), ("name", string("multiple")), ("type", string("true_false")), ("ui", Value::Int(1))]));
}

fn default_value(site: &Site, kind: &str, out: &mut Vec<Part>) {
    out.push(set(vec![("label", t(site, "Default Value")), ("instructions", t(site, "Appears when creating a new post")), ("type", string(kind)), ("name", string("default_value"))]));
}

fn placeholder(site: &Site, out: &mut Vec<Part>) {
    out.push(set(vec![("label", t(site, "Placeholder Text")), ("instructions", t(site, "Appears within the input")), ("type", string("text")), ("name", string("placeholder"))]));
}

fn prepend_append(site: &Site, out: &mut Vec<Part>) {
    out.push(set(vec![("label", t(site, "Prepend")), ("instructions", t(site, "Appears before the input")), ("type", string("text")), ("name", string("prepend"))]));
    out.push(set(vec![("label", t(site, "Append")), ("instructions", t(site, "Appears after the input")), ("type", string("text")), ("name", string("append"))]));
}

fn return_value_radio(site: &Site, out: &mut Vec<Part>) {
    out.push(set(vec![("label", t(site, "Return Value")), ("instructions", t(site, "Specify the returned value on front end")), ("type", string("radio")), ("name", string("return_format")), ("layout", string("horizontal")), ("choices", arr(vec![("value", t(site, "Value")), ("label", t(site, "Label")), ("array", t(site, "Both (Array)"))]))]));
}

fn layout_radio(site: &Site, first: (&str, &str), second: (&str, &str), out: &mut Vec<Part>) {
    out.push(set(vec![("label", t(site, "Layout")), ("instructions", string("")), ("type", string("radio")), ("name", string("layout")), ("layout", string("horizontal")), ("choices", arr(vec![(first.0, t(site, first.1)), (second.0, t(site, second.1))]))]));
}

fn character_limit(site: &Site, out: &mut Vec<Part>) {
    out.push(set(vec![("label", t(site, "Character Limit")), ("instructions", t(site, "Leave blank for no limit")), ("type", string("number")), ("name", string("maxlength"))]));
}

/// The parts of `render_field_{$tab}_settings()` of the field's type.
pub async fn settings(env: &Env<'_>, field: &mut RenderField, tab: Tab) -> rustpress_core::Result<Vec<Part>> {
    let site = env.site;
    let mut out: Vec<Part> = Vec::new();
    let o = &mut out;
    let kind = strval(field.get("type"));
    match (kind.as_str(), tab) {
        ("accordion", Tab::General) => {
            o.push(set(vec![("label", t(site, "Open")), ("instructions", t(site, "Display this accordion as open on page load.")), ("name", string("open")), ("type", string("true_false")), ("ui", Value::Int(1))]));
            o.push(set(vec![("label", t(site, "Multi-Expand")), ("instructions", t(site, "Allow this accordion to open without closing others.")), ("name", string("multi_expand")), ("type", string("true_false")), ("ui", Value::Int(1))]));
            o.push(set(vec![("label", t(site, "Endpoint")), ("instructions", t(site, "Define an endpoint for the previous accordion to stop. This accordion will not be visible.")), ("name", string("endpoint")), ("type", string("true_false")), ("ui", Value::Int(1))]));
        }
        ("button_group", Tab::General) => {
            let choices = encode_choices(field.get("choices").unwrap_or(&Value::Null), true);
            field.set("choices", choices);
            o.push(set(vec![("label", t(site, "Choices")), ("instructions", choices_instructions(site)), ("type", string("textarea")), ("name", string("choices"))]));
            o.push(set(vec![("label", t(site, "Default Value")), ("instructions", t(site, "Appears when creating a new post")), ("type", string("text")), ("name", string("default_value"))]));
            return_value_radio(site, o);
        }
        ("button_group", Tab::Validation) => allow_null(site, o),
        ("button_group", Tab::Presentation) => layout_radio(site, ("horizontal", "Horizontal"), ("vertical", "Vertical"), o),
        ("checkbox", Tab::General) => {
            let choices = encode_choices(field.get("choices").unwrap_or(&Value::Null), true);
            field.set("choices", choices);
            let defaults = encode_choices(field.get("default_value").unwrap_or(&Value::Null), false);
            field.set("default_value", defaults);
            o.push(set(vec![("label", t(site, "Choices")), ("instructions", choices_instructions(site)), ("type", string("textarea")), ("name", string("choices"))]));
            o.push(set(vec![("label", t(site, "Default Value")), ("instructions", t(site, "Enter each default value on a new line")), ("type", string("textarea")), ("name", string("default_value"))]));
            return_value_radio(site, o);
        }
        ("checkbox", Tab::Validation) => {
            o.push(set(vec![("label", t(site, "Allow Custom Values")), ("name", string("allow_custom")), ("type", string("true_false")), ("ui", Value::Int(1)), ("instructions", t(site, "Allow 'custom' values to be added"))]));
            o.push(set(vec![("label", t(site, "Save Custom Values")), ("name", string("save_custom")), ("type", string("true_false")), ("ui", Value::Int(1)), ("instructions", t(site, "Save 'custom' values to the field's choices")), ("conditions", arr(vec![("field", string("allow_custom")), ("operator", string("==")), ("value", Value::Int(1))]))]));
        }
        ("checkbox", Tab::Presentation) => {
            layout_radio(site, ("vertical", "Vertical"), ("horizontal", "Horizontal"), o);
            o.push(set(vec![("label", t(site, "Add Toggle All")), ("instructions", t(site, "Prepend an extra checkbox to toggle all choices")), ("name", string("toggle")), ("type", string("true_false")), ("ui", Value::Int(1))]));
        }
        ("color_picker", Tab::General) => {
            o.push(set(vec![("label", t(site, "Default Value")), ("instructions", string("")), ("type", string("text")), ("name", string("default_value")), ("placeholder", string("#FFFFFF"))]));
            o.push(set(vec![("label", t(site, "Enable Transparency")), ("instructions", string("")), ("type", string("true_false")), ("name", string("enable_opacity")), ("ui", Value::Int(1))]));
            o.push(set(vec![("label", t(site, "Return Format")), ("instructions", string("")), ("type", string("radio")), ("name", string("return_format")), ("layout", string("horizontal")), ("choices", arr(vec![("string", t(site, "Hex String")), ("array", t(site, "RGBA Array"))]))]));
        }
        ("color_picker", Tab::Presentation) => {
            o.push(set(vec![("label", t(site, "Show Custom Palette")), ("instructions", string("")), ("type", string("true_false")), ("name", string("show_custom_palette")), ("ui", Value::Int(1))]));
            let mut conditions = arr(vec![("field", string("show_custom_palette")), ("operator", string("==")), ("value", Value::Int(1))]);
            if env.theme_json {
                o.push(set(vec![("label", t(site, "Custom Palette Source")), ("instructions", string("")), ("type", string("radio")), ("name", string("custom_palette_source")), ("layout", string("vertical")), ("choices", arr(vec![("custom", t(site, "Specify custom colors")), ("themejson", t(site, "Use colors from theme.json"))])), ("conditions", conditions)]));
                conditions = arr(vec![("field", string("custom_palette_source")), ("operator", string("==")), ("value", string("custom"))]);
            }
            o.push(set(vec![("label", t(site, "Custom Palette")), ("instructions", t(site, "Use a custom color palette by entering comma separated hex or rgba values")), ("type", string("text")), ("name", string("palette_colors")), ("conditions", conditions)]));
            o.push(set(vec![("label", t(site, "Show Color Wheel")), ("instructions", string("")), ("type", string("true_false")), ("name", string("show_color_wheel")), ("default_value", Value::Int(1)), ("ui", Value::Int(1))]));
        }
        (kind @ ("date_picker" | "date_time_picker"), Tab::General) => {
            let (display, returned): (&[&str], &[&str]) = if kind == "date_picker" { (&["d/m/Y", "m/d/Y", "F j, Y"], &["d/m/Y", "m/d/Y", "F j, Y", "Ymd"]) } else { (&["d/m/Y g:i a", "m/d/Y g:i a", "F j, Y g:i a", "Y-m-d H:i:s"], &["d/m/Y g:i a", "m/d/Y g:i a", "F j, Y g:i a", "Y-m-d H:i:s"]) };
            o.push(html("<div class=\"acf-field-settings-split\">"));
            o.push(set(vec![("label", t(site, "Display Format")), ("hint", t(site, "The format displayed when editing a post")), ("type", string("radio")), ("name", string("display_format")), ("other_choice", Value::Int(1)), ("choices", format_choices(env, display))]));
            // `save_format` of fields from ACF before 5.0.0.
            if kind == "date_picker" && truthy(field.get("save_format")) {
                o.push(set(vec![("label", t(site, "Save Format")), ("hint", t(site, "The format used when saving a value")), ("type", string("text")), ("name", string("save_format"))]));
            } else {
                o.push(set(vec![("label", t(site, "Return Format")), ("hint", t(site, "The format returned via template functions")), ("type", string("radio")), ("name", string("return_format")), ("other_choice", Value::Int(1)), ("choices", format_choices(env, returned))]));
            }
            o.push(html("</div>"));
            o.push(set(vec![("label", t(site, "Week Starts On")), ("instructions", string("")), ("type", string("select")), ("name", string("first_day")), ("choices", weekdays(site))]));
            o.push(set(vec![("label", t(site, "Default to the current date")), ("instructions", t(site, "Use the current date as the default value for this field.")), ("type", string("true_false")), ("name", string("default_to_current_date")), ("ui", Value::Int(1))]));
        }
        ("email", Tab::General) | ("text", Tab::General) | ("number", Tab::General) | ("url", Tab::General) => default_value(site, "text", o),
        ("email", Tab::Presentation) | ("text", Tab::Presentation) | ("password", Tab::Presentation) => {
            placeholder(site, o);
            prepend_append(site, o);
        }
        ("file", Tab::General) => {
            o.push(set(vec![("label", t(site, "Return Value")), ("instructions", t(site, "Specify the returned value on front end")), ("type", string("radio")), ("name", string("return_format")), ("layout", string("horizontal")), ("choices", arr(vec![("array", t(site, "File Array")), ("url", t(site, "File URL")), ("id", t(site, "File ID"))]))]));
            o.push(set(vec![("label", t(site, "Library")), ("instructions", t(site, "Limit the media library choice")), ("type", string("radio")), ("name", string("library")), ("layout", string("horizontal")), ("choices", arr(vec![("all", t(site, "All")), ("uploadedTo", t(site, "Uploaded to post"))]))]));
        }
        ("file", Tab::Validation) => {
            clear(field, &["min_size", "max_size"]);
            o.push(set(vec![("label", t(site, "Minimum")), ("instructions", t(site, "Restrict which files can be uploaded")), ("type", string("text")), ("name", string("min_size")), ("prepend", t(site, "File size")), ("append", string("MB"))]));
            o.push(set(vec![("label", t(site, "Maximum")), ("instructions", t(site, "Restrict which files can be uploaded")), ("type", string("text")), ("name", string("max_size")), ("prepend", t(site, "File size")), ("append", string("MB"))]));
            o.push(set(vec![("label", t(site, "Allowed File Types")), ("hint", t(site, "Comma separated list. Leave blank for all types")), ("type", string("text")), ("name", string("mime_types"))]));
        }
        ("google_map", Tab::General) => {
            o.push(set(vec![("label", t(site, "Center")), ("hint", t(site, "Center the initial map")), ("type", string("text")), ("name", string("center_lat")), ("prepend", string("lat")), ("placeholder", string("-37.81411"))]));
            o.push(set(vec![("label", t(site, "Center")), ("hint", t(site, "Center the initial map")), ("type", string("text")), ("name", string("center_lng")), ("prepend", string("lng")), ("placeholder", string("144.96328")), ("_append", string("center_lat"))]));
            o.push(set(vec![("label", t(site, "Zoom")), ("instructions", t(site, "Set the initial zoom level")), ("type", string("text")), ("name", string("zoom")), ("placeholder", string("14"))]));
            o.push(set(vec![("label", t(site, "Height")), ("instructions", t(site, "Customize the map height")), ("type", string("text")), ("name", string("height")), ("append", string("px")), ("placeholder", string("400"))]));
        }
        ("group", Tab::General) => {
            o.push(Part::SubFields);
            o.push(set(vec![("label", t(site, "Layout")), ("instructions", t(site, "Specify the style used to render the selected fields")), ("type", string("radio")), ("name", string("layout")), ("layout", string("horizontal")), ("choices", arr(vec![("block", t(site, "Block")), ("table", t(site, "Table")), ("row", t(site, "Row"))]))]));
        }
        ("icon_picker", Tab::General) => {
            let mut tabs = vec![("dashicons", esc_t(site, "Dashicons"))];
            if env.can_upload {
                tabs.push(("media_library", esc_t(site, "Media Library")));
            }
            tabs.push(("url", esc_t(site, "URL")));
            o.push(set(vec![("label", t(site, "Tabs")), ("instructions", t(site, "Select where content editors can choose the icon from.")), ("type", string("checkbox")), ("name", string("tabs")), ("choices", arr(tabs))]));
            let doc = format!("<a href=\"{}\" target=\"_blank\">{}</a>", super::admin_lists::utm("https://www.advancedcustomfields.com/resources/icon-picker/", "docs", "icon-picker-return-format", "", "", ""), t_str(site, "Learn More"));
            let instructions = rustpress_core::i18n::sprintf(&t_str(site, "Specify the return format for the icon. %s"), &[&doc]);
            o.push(set(vec![("label", t(site, "Return Format")), ("instructions", string(&instructions)), ("type", string("radio")), ("name", string("return_format")), ("choices", arr(vec![("string", t(site, "String")), ("array", t(site, "Array"))])), ("layout", string("horizontal"))]));
        }
        ("image", Tab::General) => {
            o.push(set(vec![("label", t(site, "Return Format")), ("instructions", string("")), ("type", string("radio")), ("name", string("return_format")), ("layout", string("horizontal")), ("choices", arr(vec![("array", t(site, "Image Array")), ("url", t(site, "Image URL")), ("id", t(site, "Image ID"))]))]));
            o.push(set(vec![("label", t(site, "Library")), ("instructions", t(site, "Limit the media library choice")), ("type", string("radio")), ("name", string("library")), ("layout", string("horizontal")), ("choices", arr(vec![("all", t(site, "All")), ("uploadedTo", t(site, "Uploaded to post"))]))]));
        }
        ("image", Tab::Validation) => {
            clear(field, &["min_width", "min_height", "min_size", "max_width", "max_height", "max_size"]);
            o.push(set(vec![("label", t(site, "Minimum")), ("hint", t(site, "Restrict which images can be uploaded")), ("type", string("text")), ("name", string("min_width")), ("prepend", t(site, "Width")), ("append", string("px"))]));
            o.push(set(vec![("label", string("")), ("type", string("text")), ("name", string("min_height")), ("prepend", t(site, "Height")), ("append", string("px")), ("_append", string("min_width"))]));
            o.push(set(vec![("label", string("")), ("type", string("text")), ("name", string("min_size")), ("prepend", t(site, "File size")), ("append", string("MB")), ("_append", string("min_width"))]));
            o.push(set(vec![("label", t(site, "Maximum")), ("hint", t(site, "Restrict which images can be uploaded")), ("type", string("text")), ("name", string("max_width")), ("prepend", t(site, "Width")), ("append", string("px"))]));
            o.push(set(vec![("label", string("")), ("type", string("text")), ("name", string("max_height")), ("prepend", t(site, "Height")), ("append", string("px")), ("_append", string("max_width"))]));
            o.push(set(vec![("label", string("")), ("type", string("text")), ("name", string("max_size")), ("prepend", t(site, "File size")), ("append", string("MB")), ("_append", string("max_width"))]));
            o.push(set(vec![("label", t(site, "Allowed File Types")), ("instructions", t(site, "Comma separated list. Leave blank for all types")), ("type", string("text")), ("name", string("mime_types"))]));
        }
        ("image", Tab::Presentation) => {
            o.push(set(vec![("label", t(site, "Preview Size")), ("instructions", string("")), ("type", string("select")), ("name", string("preview_size")), ("choices", image_sizes(site))]));
        }
        ("link", Tab::General) => {
            o.push(set(vec![("label", t(site, "Return Value")), ("instructions", t(site, "Specify the returned value on front end")), ("type", string("radio")), ("name", string("return_format")), ("layout", string("horizontal")), ("choices", arr(vec![("array", t(site, "Link Array")), ("url", t(site, "Link URL"))]))]));
        }
        ("message", Tab::General) => {
            o.push(set(vec![("label", t(site, "Message")), ("instructions", string("")), ("type", string("textarea")), ("name", string("message"))]));
            o.push(set(vec![("label", t(site, "New Lines")), ("instructions", t(site, "Controls how new lines are rendered")), ("type", string("select")), ("name", string("new_lines")), ("choices", arr(vec![("wpautop", t(site, "Automatically add paragraphs")), ("br", t(site, "Automatically add &lt;br&gt;")), ("", t(site, "No Formatting"))]))]));
            o.push(set(vec![("label", t(site, "Escape HTML")), ("instructions", t(site, "Allow HTML markup to display as visible text instead of rendering")), ("name", string("esc_html")), ("type", string("true_false")), ("ui", Value::Int(1))]));
        }
        ("number", Tab::Validation) => {
            o.push(set(vec![("label", t(site, "Minimum Value")), ("instructions", string("")), ("type", string("number")), ("name", string("min"))]));
            o.push(set(vec![("label", t(site, "Maximum Value")), ("instructions", string("")), ("type", string("number")), ("name", string("max"))]));
        }
        ("number", Tab::Presentation) => {
            placeholder(site, o);
            o.push(set(vec![("label", t(site, "Step Size")), ("instructions", string("")), ("type", string("number")), ("name", string("step"))]));
            prepend_append(site, o);
        }
        ("oembed", Tab::General) => {
            o.push(set(vec![("label", t(site, "Embed Size")), ("type", string("text")), ("name", string("width")), ("prepend", t(site, "Width")), ("append", string("px")), ("placeholder", Value::Int(640))]));
            o.push(set(vec![("label", t(site, "Embed Size")), ("type", string("text")), ("name", string("height")), ("prepend", t(site, "Height")), ("append", string("px")), ("placeholder", Value::Int(390)), ("_append", string("width"))]));
        }
        ("page_link", Tab::General) => {
            post_filters(env, o).await?;
            o.push(set(vec![("label", t(site, "Allow Archives URLs")), ("instructions", string("")), ("name", string("allow_archives")), ("type", string("true_false")), ("ui", Value::Int(1))]));
            select_multiple(o);
        }
        ("page_link", Tab::Validation) | ("post_object", Tab::Validation) | ("select", Tab::Validation) | ("user", Tab::Validation) => allow_null(site, o),
        ("post_object", Tab::General) => {
            post_filters(env, o).await?;
            o.push(set(vec![("label", t(site, "Return Format")), ("instructions", string("")), ("type", string("radio")), ("name", string("return_format")), ("choices", arr(vec![("object", t(site, "Post Object")), ("id", t(site, "Post ID"))])), ("layout", string("horizontal"))]));
            select_multiple(o);
        }
        ("post_object" | "relationship" | "taxonomy" | "user", Tab::Advanced) => bidirectional(env, field, o),
        ("radio", Tab::General) => {
            let choices = encode_choices(field.get("choices").unwrap_or(&Value::Null), true);
            field.set("choices", choices);
            o.push(set(vec![("label", t(site, "Choices")), ("instructions", choices_instructions(site)), ("type", string("textarea")), ("name", string("choices"))]));
            o.push(set(vec![("label", t(site, "Default Value")), ("instructions", t(site, "Appears when creating a new post")), ("type", string("text")), ("name", string("default_value"))]));
            return_value_radio(site, o);
        }
        ("radio", Tab::Validation) => {
            allow_null(site, o);
            o.push(set(vec![("label", t(site, "Allow Other Choice")), ("name", string("other_choice")), ("type", string("true_false")), ("ui", Value::Int(1)), ("instructions", t(site, "Add 'other' choice to allow for custom values"))]));
            o.push(set(vec![("label", t(site, "Save Other Choice")), ("name", string("save_other_choice")), ("type", string("true_false")), ("ui", Value::Int(1)), ("instructions", t(site, "Save 'other' values to the field's choices")), ("conditions", arr(vec![("field", string("other_choice")), ("operator", string("==")), ("value", Value::Int(1))]))]));
        }
        ("radio", Tab::Presentation) => layout_radio(site, ("vertical", "Vertical"), ("horizontal", "Horizontal"), o),
        ("range", Tab::General) => default_value(site, "number", o),
        ("range", Tab::Validation) => {
            o.push(set(vec![("label", t(site, "Minimum Value")), ("instructions", string("")), ("type", string("number")), ("name", string("min")), ("placeholder", string("0"))]));
            o.push(set(vec![("label", t(site, "Maximum Value")), ("instructions", string("")), ("type", string("number")), ("name", string("max")), ("placeholder", string("100"))]));
        }
        ("range", Tab::Presentation) => {
            o.push(set(vec![("label", t(site, "Step Size")), ("instructions", string("")), ("type", string("number")), ("name", string("step")), ("placeholder", string("1"))]));
            prepend_append(site, o);
        }
        ("relationship", Tab::General) => {
            post_filters(env, o).await?;
            o.push(set(vec![("label", t(site, "Filters")), ("instructions", string("")), ("type", string("checkbox")), ("name", string("filters")), ("choices", arr(vec![("search", t(site, "Search")), ("post_type", t(site, "Post Type")), ("taxonomy", t(site, "Taxonomy"))]))]));
            o.push(set(vec![("label", t(site, "Return Format")), ("instructions", string("")), ("type", string("radio")), ("name", string("return_format")), ("choices", arr(vec![("object", t(site, "Post Object")), ("id", t(site, "Post ID"))])), ("layout", string("horizontal"))]));
        }
        ("relationship", Tab::Validation) => {
            clear(field, &["min", "max"]);
            o.push(set(vec![("label", t(site, "Minimum Posts")), ("instructions", string("")), ("type", string("number")), ("name", string("min"))]));
            o.push(set(vec![("label", t(site, "Maximum Posts")), ("instructions", string("")), ("type", string("number")), ("name", string("max"))]));
        }
        ("relationship", Tab::Presentation) => {
            o.push(set(vec![("label", t(site, "Elements")), ("instructions", t(site, "Selected elements will be displayed in each result")), ("type", string("checkbox")), ("name", string("elements")), ("choices", arr(vec![("featured_image", t(site, "Featured Image"))]))]));
        }
        ("select", Tab::General) => {
            let choices = encode_choices(field.get("choices").unwrap_or(&Value::Null), true);
            field.set("choices", choices);
            let defaults = encode_choices(field.get("default_value").unwrap_or(&Value::Null), false);
            field.set("default_value", defaults);
            o.push(set(vec![("label", t(site, "Choices")), ("instructions", choices_instructions(site)), ("name", string("choices")), ("type", string("textarea"))]));
            o.push(set(vec![("label", t(site, "Default Value")), ("instructions", t(site, "Enter each default value on a new line")), ("name", string("default_value")), ("type", string("textarea"))]));
            o.push(set(vec![("label", t(site, "Return Format")), ("instructions", t(site, "Specify the value returned")), ("type", string("radio")), ("name", string("return_format")), ("layout", string("horizontal")), ("choices", arr(vec![("value", t(site, "Value")), ("label", t(site, "Label")), ("array", t(site, "Both (Array)"))]))]));
            select_multiple(o);
        }
        ("select", Tab::Presentation) => {
            let ui = arr(vec![("field", string("ui")), ("operator", string("==")), ("value", Value::Int(1))]);
            let multiple = arr(vec![("field", string("multiple")), ("operator", string("==")), ("value", Value::Int(1))]);
            o.push(set(vec![("label", t(site, "Stylized UI")), ("instructions", t(site, "Use a stylized checkbox using select2")), ("name", string("ui")), ("type", string("true_false")), ("ui", Value::Int(1))]));
            o.push(set(vec![("label", t(site, "Use AJAX to lazy load choices?")), ("instructions", string("")), ("name", string("ajax")), ("type", string("true_false")), ("ui", Value::Int(1)), ("conditions", ui.clone())]));
            o.push(set(vec![("label", t(site, "Create Options")), ("instructions", t(site, "Allow content editors to create new options by typing in the Select input. Multiple options can be created from a comma separated string.")), ("name", string("create_options")), ("type", string("true_false")), ("ui", Value::Int(1)), ("conditions", list(vec![ui.clone(), multiple.clone()]))]));
            o.push(set(vec![("label", t(site, "Save Options")), ("instructions", t(site, "Save created options back to the \"Choices\" setting in the field definition.")), ("name", string("save_options")), ("type", string("true_false")), ("ui", Value::Int(1)), ("conditions", list(vec![ui, multiple, arr(vec![("field", string("create_options")), ("operator", string("==")), ("value", Value::Int(1))])]))]));
        }
        ("tab", Tab::General) => {
            o.push(set(vec![("label", t(site, "Placement")), ("type", string("select")), ("name", string("placement")), ("choices", arr(vec![("top", t(site, "Top aligned")), ("left", t(site, "Left aligned"))]))]));
            o.push(set(vec![("label", t(site, "New Tab Group")), ("instructions", t(site, "Start a new group of tabs at this tab.")), ("name", string("endpoint")), ("type", string("true_false")), ("ui", Value::Int(1))]));
        }
        ("taxonomy", Tab::General) => {
            o.push(set(vec![("label", t(site, "Taxonomy")), ("instructions", t(site, "Select the taxonomy to be displayed")), ("type", string("select")), ("name", string("taxonomy")), ("choices", taxonomy_labels(site))]));
            o.push(set(vec![("label", t(site, "Create Terms")), ("instructions", t(site, "Allow new terms to be created whilst editing")), ("name", string("add_term")), ("type", string("true_false")), ("ui", Value::Int(1))]));
            o.push(set(vec![("label", t(site, "Save Terms")), ("instructions", t(site, "Connect selected terms to the post")), ("name", string("save_terms")), ("type", string("true_false")), ("ui", Value::Int(1))]));
            o.push(set(vec![("label", t(site, "Load Terms")), ("instructions", t(site, "Load value from posts terms")), ("name", string("load_terms")), ("type", string("true_false")), ("ui", Value::Int(1))]));
            o.push(set(vec![("label", t(site, "Return Value")), ("instructions", string("")), ("type", string("radio")), ("name", string("return_format")), ("choices", arr(vec![("object", t(site, "Term Object")), ("id", t(site, "Term ID"))])), ("layout", string("horizontal"))]));
            let appearance = Value::Array(vec![
                (key(&t_str(site, "Multiple Values")), arr(vec![("checkbox", t(site, "Checkbox")), ("multi_select", t(site, "Multi Select"))])),
                (key(&t_str(site, "Single Value")), arr(vec![("radio", t(site, "Radio Buttons")), ("select", tx(site, "Select", "noun"))])),
            ]);
            o.push(set(vec![("label", t(site, "Appearance")), ("instructions", t(site, "Select the appearance of this field")), ("type", string("select")), ("name", string("field_type")), ("optgroup", Value::Bool(true)), ("choices", appearance)]));
            o.push(set(vec![("label", t(site, "Allow Null")), ("instructions", string("")), ("name", string("allow_null")), ("type", string("true_false")), ("ui", Value::Int(1)), ("conditions", arr(vec![("field", string("field_type")), ("operator", string("!=")), ("value", string("checkbox"))]))]));
        }
        ("text", Tab::Validation) | ("textarea", Tab::Validation) => character_limit(site, o),
        ("textarea", Tab::General) | ("wysiwyg", Tab::General) => default_value(site, "textarea", o),
        ("textarea", Tab::Presentation) => {
            o.push(set(vec![("label", t(site, "Rows")), ("instructions", t(site, "Sets the textarea height")), ("type", string("number")), ("name", string("rows")), ("placeholder", Value::Int(8))]));
            placeholder(site, o);
            o.push(set(vec![("label", t(site, "New Lines")), ("instructions", t(site, "Controls how new lines are rendered")), ("type", string("select")), ("name", string("new_lines")), ("choices", arr(vec![("wpautop", t(site, "Automatically add paragraphs")), ("br", t(site, "Automatically add &lt;br&gt;")), ("", t(site, "No Formatting"))]))]));
        }
        ("time_picker", Tab::General) => {
            o.push(html("<div class=\"acf-field-settings-split\">"));
            o.push(set(vec![("label", t(site, "Display Format")), ("hint", t(site, "The format displayed when editing a post")), ("type", string("radio")), ("name", string("display_format")), ("other_choice", Value::Int(1)), ("choices", format_choices(env, &["g:i a", "H:i:s"]))]));
            o.push(set(vec![("label", t(site, "Return Format")), ("hint", t(site, "The format returned via template functions")), ("type", string("radio")), ("name", string("return_format")), ("other_choice", Value::Int(1)), ("choices", format_choices(env, &["g:i a", "H:i:s"]))]));
            o.push(html("</div>"));
        }
        ("true_false", Tab::General) => {
            o.push(set(vec![("label", t(site, "Message")), ("instructions", t(site, "Displays text alongside the checkbox")), ("type", string("text")), ("name", string("message"))]));
            o.push(set(vec![("label", t(site, "Default Value")), ("instructions", string("")), ("type", string("true_false")), ("name", string("default_value"))]));
        }
        ("true_false", Tab::Presentation) => {
            let ui = arr(vec![("field", string("ui")), ("operator", string("==")), ("value", Value::Int(1))]);
            o.push(set(vec![("label", t(site, "On Text")), ("instructions", t(site, "Text shown when active")), ("type", string("text")), ("name", string("ui_on_text")), ("placeholder", t(site, "Yes")), ("conditions", ui.clone())]));
            o.push(set(vec![("label", t(site, "Off Text")), ("instructions", t(site, "Text shown when inactive")), ("type", string("text")), ("name", string("ui_off_text")), ("placeholder", t(site, "No")), ("conditions", ui)]));
            o.push(set(vec![("label", t(site, "Stylized UI")), ("instructions", t(site, "Use a stylized checkbox using select2")), ("type", string("true_false")), ("name", string("ui")), ("ui", Value::Int(1)), ("class", string("acf-field-object-true-false-ui"))]));
        }
        ("url", Tab::Presentation) => placeholder(site, o),
        ("user", Tab::General) => {
            o.push(set(vec![("label", t(site, "Filter by Role")), ("instructions", string("")), ("type", string("select")), ("name", string("role")), ("choices", user_roles(site)), ("multiple", Value::Int(1)), ("ui", Value::Int(1)), ("allow_null", Value::Int(1)), ("placeholder", t(site, "All user roles"))]));
            o.push(set(vec![("label", t(site, "Return Format")), ("instructions", string("")), ("type", string("radio")), ("name", string("return_format")), ("choices", arr(vec![("array", t(site, "User Array")), ("object", t(site, "User Object")), ("id", t(site, "User ID"))])), ("layout", string("horizontal"))]));
            select_multiple(o);
        }
        ("wysiwyg", Tab::Presentation) => {
            let not_text = arr(vec![("field", string("tabs")), ("operator", string("!=")), ("value", string("text"))]);
            o.push(set(vec![("label", t(site, "Tabs")), ("instructions", string("")), ("type", string("select")), ("name", string("tabs")), ("choices", arr(vec![("all", t(site, "Visual & Text")), ("visual", t(site, "Visual Only")), ("text", t(site, "Text Only"))]))]));
            // `get_toolbars()`: "Full" and "Basic", keyed by `sanitize_title()` of the label.
            o.push(set(vec![("label", t(site, "Toolbar")), ("instructions", string("")), ("type", string("select")), ("name", string("toolbar")), ("choices", arr(vec![("full", string("Full")), ("basic", string("Basic"))])), ("conditions", not_text.clone())]));
            o.push(set(vec![("label", t(site, "Show Media Upload Buttons")), ("instructions", string("")), ("name", string("media_upload")), ("type", string("true_false")), ("ui", Value::Int(1))]));
            o.push(set(vec![("label", t(site, "Delay Initialization")), ("instructions", t(site, "TinyMCE will not be initialized until field is clicked")), ("name", string("delay")), ("type", string("true_false")), ("ui", Value::Int(1)), ("conditions", not_text)]));
        }
        _ => {}
    }
    Ok(out)
}

/// `acf_field::render_required_setting()` (validation, priority 5).
pub fn required_setting(site: &Site, kind: &str) -> Option<Part> {
    super::field_types::supports_required(kind).then(|| set(vec![("label", t(site, "Required")), ("instructions", string("")), ("type", string("true_false")), ("name", string("required")), ("ui", Value::Int(1)), ("class", string("field-required"))]))
}

/// `acf_field::render_bindings_setting()` (presentation, priority 5): a stored field without the setting allows it,
/// except in `wp_ajax_acf/field_group/render_field_settings` (`in_ajax`).
pub fn bindings_setting(site: &Site, field: &mut RenderField, in_ajax: bool) -> Option<Part> {
    if !super::field_types::supports_bindings(&strval(field.get("type"))) {
        return None;
    }
    let link = format!("<a target=\"_blank\" href=\"{}\">{}</a>", super::admin_lists::utm("https://www.advancedcustomfields.com/resources/bindings-security/", "docs", "field-settings", "", "", ""), rustpress_core::formatting::esc_html(&t_str(site, "Learn more.")));
    let instructions = rustpress_core::i18n::sprintf(&rustpress_core::formatting::esc_html(&t_str(site, "Allow content editors to access and display the field value in the editor UI using Block Bindings or the ACF Shortcode. %s")), &[&link]);
    if field.get("allow_in_bindings").is_none_or(|v| matches!(v, Value::Null)) {
        let stored = truthy(field.get("ID")) && !in_ajax;
        field.set("allow_in_bindings", Value::Bool(stored));
    }
    Some(set(vec![("label", t(site, "Allow Access to Value in Editor UI")), ("instructions", string(&instructions)), ("type", string("true_false")), ("name", string("allow_in_bindings")), ("ui", Value::Int(1)), ("class", string("field-show-in-bindings"))]))
}
