//! The field types of ACF (free) as `acf()->fields` registers them, in the order of `acf.php`: what
//! `acf_register_field_type_info()` knows of each (`initialize()`), `acf_get_grouped_field_types()`,
//! `acf_get_field_types_info()`, `acf_get_field_categories_i18n()` and `acf_get_pro_field_types()`.

use rustpress_core::Site;
use rustpress_phpser::Value;
use serde_json::{Map, Value as Json};

use super::php::{key, string};

/// What `initialize()` sets.
pub struct TypeInfo {
    pub name: &'static str,
    /// The label and its context (`_x()`).
    pub label: (&'static str, Option<&'static str>),
    pub category: &'static str,
    /// The parts of the description, joined with a space.
    pub description: &'static [&'static str],
    /// Below `acf_get_url()`, with its leading slash (the URL gets two).
    pub preview: &'static str,
    /// `acf_add_url_utm_tags( $url, 'docs', 'field-type-selection', $anchor )`.
    pub doc: Option<(&'static str, &'static str)>,
    pub tutorial: Option<(&'static str, &'static str)>,
    pub public: bool,
    /// `supports['required']`.
    pub required: bool,
    /// `supports['bindings']`.
    pub bindings: bool,
}

pub const TYPES: &[TypeInfo] = &[
    TypeInfo { name: "text", label: ("Text", None), category: "basic", description: &["A basic text input, useful for storing single string values."], preview: "/assets/images/field-type-previews/field-preview-text.png", doc: Some(("https://www.advancedcustomfields.com/resources/text/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "textarea", label: ("Text Area", None), category: "basic", description: &["A basic textarea input for storing paragraphs of text."], preview: "/assets/images/field-type-previews/field-preview-textarea.png", doc: Some(("https://www.advancedcustomfields.com/resources/textarea/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "number", label: ("Number", None), category: "basic", description: &["An input limited to numerical values."], preview: "/assets/images/field-type-previews/field-preview-number.png", doc: Some(("https://www.advancedcustomfields.com/resources/number/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "range", label: ("Range", None), category: "basic", description: &["An input for selecting a numerical value within a specified range using a range slider element."], preview: "/assets/images/field-type-previews/field-preview-range.png", doc: Some(("https://www.advancedcustomfields.com/resources/range/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "email", label: ("Email", None), category: "basic", description: &["A text input specifically designed for storing email addresses."], preview: "/assets/images/field-type-previews/field-preview-email.png", doc: Some(("https://www.advancedcustomfields.com/resources/email/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "url", label: ("URL", None), category: "basic", description: &["A text input specifically designed for storing web addresses."], preview: "/assets/images/field-type-previews/field-preview-url.png", doc: Some(("https://www.advancedcustomfields.com/resources/url/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "password", label: ("Password", None), category: "basic", description: &["An input for providing a password using a masked field."], preview: "/assets/images/field-type-previews/field-preview-password.png", doc: Some(("https://www.advancedcustomfields.com/resources/password/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "image", label: ("Image", None), category: "content", description: &["Uses the native WordPress media picker to upload, or choose images."], preview: "/assets/images/field-type-previews/field-preview-image.png", doc: Some(("https://www.advancedcustomfields.com/resources/image/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "file", label: ("File", None), category: "content", description: &["Uses the native WordPress media picker to upload, or choose files."], preview: "/assets/images/field-type-previews/field-preview-file.png", doc: Some(("https://www.advancedcustomfields.com/resources/file/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "wysiwyg", label: ("WYSIWYG Editor", None), category: "content", description: &["Displays the WordPress WYSIWYG editor as seen in Posts and Pages allowing for a rich text-editing experience that also allows for multimedia content.", "We do not recommend using this field in ACF Blocks."], preview: "/assets/images/field-type-previews/field-preview-wysiwyg.png", doc: Some(("https://www.advancedcustomfields.com/resources/wysiwyg-editor/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "oembed", label: ("oEmbed", None), category: "content", description: &["An interactive component for embedding videos, images, tweets, audio and other content by making use of the native WordPress oEmbed functionality."], preview: "/assets/images/field-type-previews/field-preview-oembed.png", doc: Some(("https://www.advancedcustomfields.com/resources/oembed/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "select", label: ("Select", Some("noun")), category: "choice", description: &["A dropdown list with a selection of choices that you specify."], preview: "/assets/images/field-type-previews/field-preview-select.png", doc: Some(("https://www.advancedcustomfields.com/resources/select/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "checkbox", label: ("Checkbox", None), category: "choice", description: &["A group of checkbox inputs that allow the user to select one, or multiple values that you specify."], preview: "/assets/images/field-type-previews/field-preview-checkbox.png", doc: Some(("https://www.advancedcustomfields.com/resources/checkbox/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "radio", label: ("Radio Button", None), category: "choice", description: &["A group of radio button inputs that allows the user to make a single selection from values that you specify."], preview: "/assets/images/field-type-previews/field-preview-radio-button.png", doc: Some(("https://www.advancedcustomfields.com/resources/radio-button/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "button_group", label: ("Button Group", None), category: "choice", description: &["A group of buttons with values that you specify, users can choose one option from the values provided."], preview: "/assets/images/field-type-previews/field-preview-button-group.png", doc: Some(("https://www.advancedcustomfields.com/resources/button-group/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "true_false", label: ("True / False", None), category: "choice", description: &["A toggle that allows you to pick a value of 1 or 0 (on or off, true or false, etc). Can be presented as a stylized switch or checkbox."], preview: "/assets/images/field-type-previews/field-preview-true-false.png", doc: Some(("https://www.advancedcustomfields.com/resources/true-false/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "link", label: ("Link", None), category: "relational", description: &["Allows you to specify a link and its properties such as title and target using the WordPress native link picker."], preview: "/assets/images/field-type-previews/field-preview-link.png", doc: Some(("https://www.advancedcustomfields.com/resources/link/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "post_object", label: ("Post Object", None), category: "relational", description: &["An interactive and customizable UI for picking one or many posts, pages or post type items with the option to search. "], preview: "/assets/images/field-type-previews/field-preview-post-object.png", doc: Some(("https://www.advancedcustomfields.com/resources/post-object/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "page_link", label: ("Page Link", None), category: "relational", description: &["An interactive dropdown to select one or more posts, pages, custom post type items or archive URLs, with the option to search."], preview: "/assets/images/field-type-previews/field-preview-page-link.png", doc: Some(("https://www.advancedcustomfields.com/resources/page-link/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "relationship", label: ("Relationship", None), category: "relational", description: &["A dual-column interface to select one or more posts, pages, or custom post type items to create a relationship with the item that you're currently editing. Includes options to search and filter."], preview: "/assets/images/field-type-previews/field-preview-relationship.png", doc: Some(("https://www.advancedcustomfields.com/resources/relationship/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "taxonomy", label: ("Taxonomy", None), category: "relational", description: &["Allows the selection of one or more taxonomy terms based on the criteria and options specified in the fields settings."], preview: "/assets/images/field-type-previews/field-preview-taxonomy.png", doc: Some(("https://www.advancedcustomfields.com/resources/taxonomy/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "user", label: ("User", None), category: "relational", description: &["Allows the selection of one or more users which can be used to create relationships between data objects."], preview: "/assets/images/field-type-previews/field-preview-user.png", doc: Some(("https://www.advancedcustomfields.com/resources/user/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "google_map", label: ("Google Map", None), category: "advanced", description: &["An interactive UI for selecting a location using Google Maps. Requires a Google Maps API key and additional configuration to display correctly."], preview: "/assets/images/field-type-previews/field-preview-google-map.png", doc: Some(("https://www.advancedcustomfields.com/resources/google-map/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "date_picker", label: ("Date Picker", None), category: "advanced", description: &["An interactive UI for picking a date. The date return format can be customized using the field settings."], preview: "/assets/images/field-type-previews/field-preview-date-picker.png", doc: Some(("https://www.advancedcustomfields.com/resources/date-picker/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "date_time_picker", label: ("Date Time Picker", None), category: "advanced", description: &["An interactive UI for picking a date and time. The date return format can be customized using the field settings."], preview: "/assets/images/field-type-previews/field-preview-date-time.png", doc: Some(("https://www.advancedcustomfields.com/resources/date-time-picker/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "time_picker", label: ("Time Picker", None), category: "advanced", description: &["An interactive UI for picking a time. The time format can be customized using the field settings."], preview: "/assets/images/field-type-previews/field-preview-time.png", doc: Some(("https://www.advancedcustomfields.com/resources/time-picker/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "color_picker", label: ("Color Picker", None), category: "advanced", description: &["An interactive UI for selecting a color, or specifying a Hex value."], preview: "/assets/images/field-type-previews/field-preview-color-picker.png", doc: Some(("https://www.advancedcustomfields.com/resources/color-picker/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "icon_picker", label: ("Icon Picker", None), category: "advanced", description: &["An interactive UI for selecting an icon. Select from Dashicons, the media library, or a standalone URL input."], preview: "/assets/images/field-type-previews/field-preview-icon-picker.png", doc: Some(("https://www.advancedcustomfields.com/resources/icon-picker/", "")), tutorial: None, public: true, required: true, bindings: true },
    TypeInfo { name: "message", label: ("Message", None), category: "layout", description: &["Used to display a message to editors alongside other fields. Useful for providing additional context or instructions around your fields."], preview: "/assets/images/field-type-previews/field-preview-message.png", doc: None, tutorial: None, public: true, required: false, bindings: false },
    TypeInfo { name: "accordion", label: ("Accordion", None), category: "layout", description: &["Allows you to group and organize custom fields into collapsable panels that are shown while editing content. Useful for keeping large datasets tidy."], preview: "/assets/images/field-type-previews/field-preview-accordion.png", doc: Some(("https://www.advancedcustomfields.com/resources/accordion/", "")), tutorial: None, public: true, required: false, bindings: false },
    TypeInfo { name: "tab", label: ("Tab", None), category: "layout", description: &["Allows you to group fields into tabbed sections in the edit screen. Useful for keeping fields organized and structured."], preview: "/assets/images/field-type-previews/field-preview-tabs.png", doc: Some(("https://www.advancedcustomfields.com/resources/tab/", "")), tutorial: None, public: true, required: false, bindings: false },
    TypeInfo { name: "group", label: ("Group", None), category: "layout", description: &["Provides a way to structure fields into groups to better organize the data and the edit screen."], preview: "/assets/images/field-type-previews/field-preview-group.png", doc: Some(("https://www.advancedcustomfields.com/resources/group/", "")), tutorial: None, public: true, required: true, bindings: false },
];

fn tr(site: &Site, text: &str) -> String {
    site.trd("acf", text)
}

/// `acf_get_field_type( $name )`.
pub fn get(name: &str) -> Option<&'static TypeInfo> {
    TYPES.iter().find(|t| t.name == name)
}

impl TypeInfo {
    /// The translated label.
    pub fn label(&self, site: &Site) -> String {
        match self.label.1 {
            Some(context) => site.i18n().pgettext("acf", context, self.label.0),
            None => tr(site, self.label.0),
        }
    }
}

/// `acf_get_field_type_label( $name )`.
pub fn type_label(site: &Site, name: &str) -> String {
    match get(name) {
        Some(info) => info.label(site),
        None => format!("<span class=\"acf-js-tooltip\" title=\"{}\">{}</span>", tr(site, "Field type does not exist"), tr(site, "Unknown")),
    }
}

/// `acf_field_type_supports( $name, 'required', true )`.
pub fn supports_required(name: &str) -> bool {
    get(name).is_none_or(|t| t.required)
}

/// `acf_field_type_supports( $name, 'bindings', true )`.
pub fn supports_bindings(name: &str) -> bool {
    get(name).is_none_or(|t| t.bindings)
}

/// `acf_get_field_categories_i18n()`.
pub fn categories(site: &Site) -> Vec<(&'static str, String)> {
    [("basic", "Basic"), ("content", "Content"), ("choice", "Choice"), ("relational", "Relational"), ("advanced", "Advanced"), ("layout", "Layout"), ("pro", "PRO")].iter().map(|(k, v)| (*k, tr(site, v))).collect()
}

/// `acf_get_grouped_field_types()`: the choices of the type select, by the translated category.
pub fn grouped(site: &Site) -> Value {
    let l10n = categories(site);
    let mut groups: Vec<(rustpress_phpser::Key, Value)> = Vec::new();
    for info in TYPES.iter().filter(|t| t.public) {
        let category = l10n.iter().find(|(k, _)| *k == info.category).map_or_else(|| info.category.to_owned(), |(_, v)| v.clone());
        let k = key(&category);
        let entry = (key(info.name), string(&info.label(site)));
        match groups.iter_mut().find(|(existing, _)| *existing == k) {
            Some((_, Value::Array(items))) => items.push(entry),
            _ => groups.push((k, Value::Array(vec![entry]))),
        }
    }
    Value::Array(groups)
}

fn docs_url(url: &str, anchor: &str) -> String {
    super::admin_lists::utm(url, "docs", "field-type-selection", anchor, "", "")
}

/// `acf_get_field_types_info()`: `array_filter()` drops what is empty or false.
pub fn types_info(site: &Site) -> Json {
    let base = site.site_url("wp-content/plugins/advanced-custom-fields/");
    let mut out = Map::new();
    for info in TYPES.iter().filter(|t| t.public) {
        let mut item = Map::new();
        item.insert("label".into(), Json::String(info.label(site)));
        item.insert("name".into(), Json::String(info.name.into()));
        let description = info.description.iter().map(|d| tr(site, d)).collect::<Vec<_>>().join(" ");
        if !description.is_empty() {
            item.insert("description".into(), Json::String(description));
        }
        item.insert("category".into(), Json::String(info.category.into()));
        item.insert("public".into(), Json::Bool(true));
        if let Some((url, anchor)) = info.doc {
            item.insert("doc_url".into(), Json::String(docs_url(url, anchor)));
        }
        if let Some((url, anchor)) = info.tutorial {
            item.insert("tutorial_url".into(), Json::String(docs_url(url, anchor)));
        }
        if !info.preview.is_empty() {
            item.insert("preview_image".into(), Json::String(format!("{base}{}", info.preview)));
        }
        out.insert(info.name.into(), Json::Object(item));
    }
    Json::Object(out)
}

/// `acf_get_pro_field_types()`.
pub fn pro_types(site: &Site) -> Json {
    let base = site.site_url("wp-content/plugins/advanced-custom-fields/");
    let pro = |name: &str, label: String, doc: &str, preview: &str, description: &str, tutorial: &str, category: &str| {
        serde_json::json!({
            "name": name,
            "label": label,
            "doc_url": docs_url(doc, ""),
            "preview_image": format!("{base}/assets/images/field-type-previews/{preview}"),
            "description": tr(site, description),
            "tutorial_url": docs_url(tutorial, ""),
            "category": category,
            "pro": true,
        })
    };
    let mut out = Map::new();
    out.insert("clone".into(), pro("clone", site.i18n().pgettext("acf", "noun", "Clone"), "https://www.advancedcustomfields.com/resources/clone/", "field-preview-clone.png", "This allows you to select and display existing fields. It does not duplicate any fields in the database, but loads and displays the selected fields at run-time. The Clone field can either replace itself with the selected fields or display the selected fields as a group of subfields.", "https://www.advancedcustomfields.com/resources/how-to-use-the-clone-field/", "layout"));
    out.insert("flexible_content".into(), pro("flexible_content", tr(site, "Flexible Content"), "https://www.advancedcustomfields.com/resources/flexible-content/", "field-preview-flexible-content.png", "This provides a simple, structured, layout-based editor. The Flexible Content field allows you to define, create and manage content with total control by using layouts and subfields to design the available blocks.", "https://www.advancedcustomfields.com/resources/building-layouts-with-the-flexible-content-field-in-a-theme/", "layout"));
    out.insert("gallery".into(), pro("gallery", tr(site, "Gallery"), "https://www.advancedcustomfields.com/resources/gallery/", "field-preview-gallery.png", "This provides an interactive interface for managing a collection of attachments. Most settings are similar to the Image field type. Additional settings allow you to specify where new attachments are added in the gallery and the minimum/maximum number of attachments allowed.", "https://www.advancedcustomfields.com/resources/how-to-use-the-gallery-field/", "content"));
    out.insert("repeater".into(), pro("repeater", tr(site, "Repeater"), "https://www.advancedcustomfields.com/resources/repeater/", "field-preview-repeater.png", "This provides a solution for repeating content such as slides, team members, and call-to-action tiles, by acting as a parent to a set of subfields which can be repeated again and again.", "https://www.advancedcustomfields.com/resources/repeater/how-to-use-the-repeater-field/", "layout"));
    Json::Object(out)
}
