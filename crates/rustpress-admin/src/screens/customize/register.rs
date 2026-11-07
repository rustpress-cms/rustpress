//! What `customize_register` adds on `customize.php`: `WP_Customize_Manager::register_controls()` and
//! `twentyseventeen_customize_register()`.

use rustpress_core::formatting::{esc_url, html_entity_decode};
use rustpress_core::i18n::sprintf;
use rustpress_phpser::Value as Php;
use serde_json::{Value, json};

use super::controls::{Field, header_image, link};
use super::manager::{Control, Manager, Panel, Section, Setting};
use super::State;

/// A PHP value as `wp_json_encode()` writes it.
pub(super) fn php_json(value: &Php) -> Value {
    rustpress_core::options::php_to_json(value)
}

/// How PHP prints a scalar (`echo`, `esc_attr()`).
pub(super) fn php_text(value: &Php) -> String {
    match value {
        Php::Str(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        Php::Int(n) => n.to_string(),
        Php::Float(f) => f.to_string(),
        Php::Bool(true) => "1".to_owned(),
        _ => String::new(),
    }
}

/// `js_value()`: strings lose their character references.
fn js_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(html_entity_decode(&text)),
        other => other,
    }
}

impl State<'_> {
    /// `get_theme_mod()`: the stored value, else the default (a string default is formatted with the directories of
    /// the theme).
    pub fn theme_mod(&self, name: &str, default: Value) -> Value {
        match self.mods.get(name).filter(|v| !matches!(v, Php::Null)) {
            Some(value) => php_json(value),
            None => match default {
                Value::String(text) => Value::String(sprintf(&text, &[&self.template_uri, &self.stylesheet_uri])),
                other => other,
            },
        }
    }

    /// `get_option()` of an option the Customizer shows: a string, `false` when it is not set.
    pub fn option(&self, name: &str) -> Value {
        match self.ctx.site().options.get(name) {
            Some(value) => php_json(&rustpress_core::options::maybe_unserialize(value)),
            None => Value::Bool(false),
        }
    }
}

/// A setting of the type `theme_mod` with its default.
fn theme_mod_setting(state: &State<'_>, id: &str, default: Value, transport: &'static str) -> Setting {
    let raw = state.theme_mod(id, default);
    let mut setting = Setting::new(id, "theme_mod", transport, js_value(raw.clone()));
    setting.raw = raw;
    setting
}

fn option_setting(state: &State<'_>, id: &str, transport: &'static str) -> Setting {
    let raw = state.option(id);
    let mut setting = Setting::new(id, "option", transport, js_value(raw.clone()));
    setting.raw = raw;
    setting.allowed = state.ctx.can("manage_options");
    setting
}

fn section(m: &mut Manager, id: &str, title: String, priority: i64, description: String) -> Section {
    Section { id: id.to_owned(), title, description, priority, panel: String::new(), kind: "default", description_hidden: false, allowed: true, active: true, instance: m.next_section(), extra: Vec::new() }
}

/// The markup of a control of `WP_Customize_Control` for a setting.
fn field_content(control: &Control, value: &Value, choices: &[(String, String)]) -> String {
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(true) => "1".to_owned(),
        _ => String::new(),
    };
    let field = Field {
        id: &control.id,
        kind: &control.kind,
        label: &control.label,
        description: &control.description,
        link: link(&control.settings[0].1),
        value: text,
        value_true: matches!(value, Value::Bool(true)),
        choices,
        input_attrs: &[],
    };
    field.render()
}

/// `$this->value()` of a control: what the markup prints.
fn raw_value(m: &Manager, id: &str) -> Value {
    m.settings.iter().find(|s| s.id == id).map(|s| s.raw.clone()).unwrap_or(Value::Null)
}

/// `WP_Customize_Manager::register_controls()`.
pub(super) async fn core(state: &State<'_>, m: &mut Manager) {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let trx = |text: &str, context: &str| ctx.site().trx(text, context);
    let theme_name = state.theme_name.clone();

    // Themes (the controls are loaded through Ajax).
    let instance = m.next_panel();
    m.add_panel(Panel {
        id: "themes".into(),
        title: theme_name,
        description: format!(
            "<p>{}</p><p>{}</p>",
            tr("Looking for a theme? You can search or browse the WordPress.org theme directory, install and preview themes, then activate them right here."),
            tr("While previewing a new theme, you can continue to tailor things like widgets and menus, and explore theme-specific options.")
        ),
        priority: 0,
        kind: "themes",
        allowed: ctx.can("switch_themes"),
        active: true,
        instance,
        auto_expand_sole_section: false,
    });
    let instance = m.next_section();
    m.add_section(Section {
        id: "installed_themes".into(),
        title: tr("Installed themes"),
        description: String::new(),
        priority: 0,
        panel: "themes".into(),
        kind: "themes",
        description_hidden: false,
        allowed: ctx.can("switch_themes"),
        active: true,
        instance,
        extra: vec![("action", json!("installed")), ("filter_type", json!("local"))],
    });
    let instance = m.next_section();
    m.add_section(Section {
        id: "wporg_themes".into(),
        title: tr("WordPress.org themes"),
        description: String::new(),
        priority: 5,
        panel: "themes".into(),
        kind: "themes",
        description_hidden: false,
        allowed: ctx.can("install_themes"),
        active: true,
        instance,
        extra: vec![("action", json!("wporg")), ("filter_type", json!("remote"))],
    });
    let mut active_theme = theme_mod_setting(state, "active_theme", json!(""), "refresh");
    active_theme.allowed = ctx.can("switch_themes");
    m.add_setting(active_theme);

    // Site Identity.
    let identity = section(m, "title_tagline", tr("Site Identity"), 20, String::new());
    m.add_section(identity);
    m.add_setting(option_setting(state, "blogname", "refresh"));
    let mut control = Control::new("blogname", "text", "title_tagline", m.next_control());
    control.label = tr("Site Title");
    m.add_control(control);
    m.add_setting(option_setting(state, "blogdescription", "refresh"));
    let mut control = Control::new("blogdescription", "text", "title_tagline", m.next_control());
    control.label = tr("Tagline");
    m.add_control(control);
    if !state.support.as_ref().is_some_and(|s| s.header_text) {
        let mut setting = theme_mod_setting(state, "header_text", json!(1), "refresh");
        setting.allowed = state.custom_logo.is_some();
        m.add_setting(setting);
        let mut control = Control::new("header_text", "checkbox", "title_tagline", m.next_control());
        control.label = tr("Display Site Title and Tagline");
        m.add_control(control);
    }
    m.add_setting(option_setting(state, "site_icon", "postMessage"));
    let mut control = Control::new("site_icon", "site_icon", "title_tagline", m.next_control());
    control.label = tr("Site Icon");
    control.description = format!("<p>{}</p>", sprintf(&tr("The Site Icon is what you see in browser tabs, bookmark bars, and within the WordPress mobile apps. It should be square and at least <strong>%1$s by %2$s</strong> pixels."), &["512", "512"]));
    control.priority = 60;
    m.add_control(control);
    let mut setting = theme_mod_setting(state, "custom_logo", Value::Null, "postMessage");
    setting.allowed = state.custom_logo.is_some();
    m.add_setting(setting);
    let mut control = Control::new("custom_logo", "cropped_image", "title_tagline", m.next_control());
    control.label = tr("Logo");
    control.priority = 8;
    m.add_control(control);

    // Colors.
    let colors = section(m, "colors", tr("Colors"), 40, String::new());
    m.add_section(colors);
    let header_text = state.support.as_ref().is_some_and(|s| s.header_text);
    let default_text_color = state.support.as_ref().map(|s| s.default_text_color.clone()).unwrap_or_default();
    let mut setting = theme_mod_setting(state, "header_textcolor", json!(default_text_color), "refresh");
    setting.value = match setting.value {
        // `maybe_hash_hex_color()`.
        Value::String(color) if matches!(color.len(), 3 | 6) && color.bytes().all(|b| b.is_ascii_hexdigit()) => json!(format!("#{color}")),
        other => other,
    };
    setting.allowed = header_text;
    m.add_setting(setting);
    let mut control = Control::new("display_header_text", "checkbox", "title_tagline", m.next_control());
    control.settings = vec![("default".into(), "header_textcolor".into())];
    control.label = tr("Display Site Title and Tagline");
    control.priority = 40;
    m.add_control(control);
    let mut control = Control::new("header_textcolor", "color", "colors", m.next_control());
    control.label = tr("Header Text Color");
    control.extra = vec![("statuses", json!({"": tr("Default")})), ("defaultValue", json!(default_text_color)), ("mode", json!("full"))];
    m.add_control(control);
    let mut setting = theme_mod_setting(state, "background_color", json!(""), "refresh");
    setting.allowed = false;
    m.add_setting(setting);
    let control = Control::new("background_color", "color", "colors", m.next_control());
    m.add_control(control);

    // Custom Header.
    let video = state.support.as_ref().is_some_and(|s| s.video);
    let (width, height) = state.support.as_ref().map_or((0, 0), |s| (s.width, s.height));
    let (title, description, control_description) = if video {
        let description = if width > 0 && height > 0 {
            sprintf(&tr("Upload your video in %1$s format and minimize its file size for best results. Your theme recommends dimensions of %2$s pixels."), &["<code>.mp4</code>", &format!("<strong>{width} &times; {height}</strong>")])
        } else if width > 0 {
            sprintf(&tr("Upload your video in %1$s format and minimize its file size for best results. Your theme recommends a width of %2$s pixels."), &["<code>.mp4</code>", &format!("<strong>{width}</strong>")])
        } else {
            sprintf(&tr("Upload your video in %1$s format and minimize its file size for best results. Your theme recommends a height of %2$s pixels."), &["<code>.mp4</code>", &format!("<strong>{height}</strong>")])
        };
        (tr("Header Media"), format!("<p>{}</p>", tr("If you add a video, the image will be used as a fallback while the video loads.")), description)
    } else {
        (tr("Header Image"), String::new(), String::new())
    };
    let mut header = section(m, "header_image", title, 60, description);
    header.allowed = state.support.is_some();
    m.add_section(header);
    let header_transport = if video { "postMessage" } else { "refresh" };
    for id in ["header_video", "external_header_video"] {
        let mut setting = theme_mod_setting(state, id, json!(""), "postMessage");
        setting.allowed = video;
        m.add_setting(setting);
    }
    let default_image = state.support.as_ref().map(|s| s.default_image.clone()).unwrap_or_default();
    let mut setting = theme_mod_setting(state, "header_image", json!(default_image), header_transport);
    setting.allowed = state.support.is_some();
    m.add_setting(setting);
    let mut setting = theme_mod_setting(state, "header_image_data", Value::Null, header_transport);
    setting.allowed = state.support.is_some();
    m.add_setting(setting);
    let mut control = Control::new("header_video", "media", "header_image", m.next_control());
    control.label = tr("Header Video");
    control.description = control_description;
    // `is_header_video_active()`: only on the front page.
    control.active = false;
    m.add_control(control);
    let mut control = Control::new("external_header_video", "url", "header_image", m.next_control());
    control.description = tr("Or, enter a YouTube URL:");
    control.active = false;
    m.add_control(control);
    let mut control = Control::new("header_image", "header", "header_image", m.next_control());
    control.settings = vec![("default".into(), "header_image".into()), ("data".into(), "header_image_data".into())];
    control.label = tr("Header Image");
    // `get_current_image_src()` asks `$this->get_url`, which the class does not declare: there is never a current image.
    control.content = header_image(&tr, &trx, &control.label, video, width, height, false, ctx.can("upload_files"));
    m.add_control(control);

    // Custom Background: the section and its controls are made, and dropped for a theme without support.
    let mut background = section(m, "background_image", tr("Background Image"), 80, String::new());
    background.allowed = false;
    m.add_section(background);
    for id in ["background_image", "background_image_thumb"] {
        let mut setting = theme_mod_setting(state, id, json!(""), "refresh");
        setting.allowed = false;
        m.add_setting(setting);
    }
    let control = Control::new("background_image", "background", "background_image", m.next_control());
    m.add_control(control);
    for (id, kind) in [("background_preset", "select"), ("background_position", "background_position"), ("background_size", "select"), ("background_repeat", "checkbox"), ("background_attachment", "checkbox")] {
        if id != "background_position" {
            let mut setting = theme_mod_setting(state, id, json!(""), "refresh");
            setting.allowed = false;
            m.add_setting(setting);
        } else {
            for axis in ["background_position_x", "background_position_y"] {
                let mut setting = theme_mod_setting(state, axis, json!(""), "refresh");
                setting.allowed = false;
                m.add_setting(setting);
            }
        }
        let control = Control::new(id, kind, "background_image", m.next_control());
        m.add_control(control);
    }

    // Static Front Page.
    let mut front = section(m, "static_front_page", tr("Homepage Settings"), 120, tr("You can choose what&#8217;s displayed on the homepage of your site. It can be posts in reverse chronological order (classic blog), or a fixed/static page. To set a static homepage, you first need to create two Pages. One will become the homepage, and the other will be where your posts are displayed."));
    front.active = state.has_published_pages;
    m.add_section(front);
    m.add_setting(option_setting(state, "show_on_front", "refresh"));
    let mut control = Control::new("show_on_front", "radio", "static_front_page", m.next_control());
    control.label = tr("Your homepage displays");
    m.add_control(control);
    for (id, label) in [("page_on_front", "Homepage"), ("page_for_posts", "Posts page")] {
        m.add_setting(option_setting(state, id, "refresh"));
        let mut control = Control::new(id, "dropdown-pages", "static_front_page", m.next_control());
        control.label = tr(label);
        control.allow_addition = true;
        m.add_control(control);
    }

    // Custom CSS.
    let mut description = format!(
        "<p>{} <a href=\"{}\" class=\"external-link\" target=\"_blank\">{}<span class=\"screen-reader-text\"> {}</span></a></p>",
        tr("Add your own CSS code here to customize the appearance and layout of your site."),
        esc_url(&tr("https://developer.wordpress.org/advanced-administration/wordpress/css/")),
        tr("Learn more about CSS"),
        tr("(opens in a new tab)")
    );
    description.push_str(&format!("<p id=\"editor-keyboard-trap-help-1\">{}</p>", tr("When using a keyboard to navigate:")));
    description.push_str(&format!(
        "<ul><li id=\"editor-keyboard-trap-help-2\">{}</li><li id=\"editor-keyboard-trap-help-3\">{}</li><li id=\"editor-keyboard-trap-help-4\">{}</li></ul>",
        tr("In the editing area, the Tab key enters a tab character."),
        tr("To move away from this area, press the Esc key followed by the Tab key."),
        tr("Screen reader users: when in forms mode, you may need to press the Esc key twice.")
    ));
    if state.syntax_highlighting {
        description.push_str(&format!(
            "<p>{}</p>",
            sprintf(
                &tr("The edit field automatically highlights code syntax. You can disable this in your <a href=\"%1$s\" %2$s>user profile%3$s</a> to work in plain text mode."),
                &[&esc_url(&ctx.admin_url("profile.php")), "class=\"external-link\" target=\"_blank\"", &format!("<span class=\"screen-reader-text\"> {}</span>", tr("(opens in a new tab)"))]
            )
        ));
    }
    description.push_str(&format!("<p class=\"section-description-buttons\"><button type=\"button\" class=\"button-link section-description-close\">{}</button></p>", tr("Close")));
    let mut css = section(m, "custom_css", tr("Additional CSS"), 200, description);
    css.description_hidden = true;
    m.add_section(css);
    let css_id = format!("custom_css[{}]", state.stylesheet);
    let mut setting = Setting::new(&css_id, "custom_css", "postMessage", json!(state.custom_css));
    // `edit_css` maps to `unfiltered_html` on a single site.
    setting.allowed = ctx.can("unfiltered_html");
    m.add_setting(setting);
    let mut control = Control::new("custom_css", "code_editor", "custom_css", m.next_control());
    control.settings = vec![("default".into(), css_id)];
    control.label = tr("CSS code");
    m.add_control(control);
}

/// `twentyseventeen_customize_register()`.
pub(super) fn twentyseventeen(state: &State<'_>, m: &mut Manager) {
    let ctx = state.ctx;
    let tt = |text: &str| ctx.site().i18n().gettext("twentyseventeen", text);
    for id in ["blogname", "blogdescription", "header_textcolor"] {
        if let Some(setting) = m.setting_mut(id) {
            setting.transport = "postMessage";
        }
    }
    m.add_setting(theme_mod_setting(state, "colorscheme", json!("light"), "postMessage"));
    m.add_setting(theme_mod_setting(state, "colorscheme_hue", json!(250), "postMessage"));
    let mut control = Control::new("colorscheme", "radio", "colors", m.next_control());
    control.label = tt("Color Scheme");
    control.priority = 5;
    m.add_control(control);
    let mut control = Control::new("colorscheme_hue", "color", "colors", m.next_control());
    control.priority = 6;
    control.extra = vec![("statuses", json!({"": ctx.tr("Default")})), ("defaultValue", json!(250)), ("mode", json!("hue"))];
    m.add_control(control);
    let options = section(m, "theme_options", tt("Theme Options"), 130, String::new());
    m.add_section(options);
    m.add_setting(theme_mod_setting(state, "page_layout", json!("two-column"), "postMessage"));
    let mut control = Control::new("page_layout", "radio", "theme_options", m.next_control());
    control.label = tt("Page Layout");
    control.description = tt("When the two-column layout is assigned, the page title is in one column and content is in the other.");
    // `twentyseventeen_is_view_with_layout_option()`: a page or an archive without a sidebar.
    control.active = false;
    m.add_control(control);
    for i in 1..=4 {
        let id = format!("panel_{i}");
        m.add_setting(theme_mod_setting(state, &id, json!(false), "postMessage"));
        let mut control = Control::new(&id, "dropdown-pages", "theme_options", m.next_control());
        control.label = sprintf(&tt("Front Page Section %d Content"), &[&i.to_string()]);
        if i == 1 {
            control.description = tt("Select pages to feature in each area from the dropdowns. Add an image to a section by setting a featured image in the page editor. Empty sections will not be displayed.");
        }
        control.allow_addition = true;
        // `twentyseventeen_is_frontpage()`.
        control.active = false;
        m.add_control(control);
    }
}

/// The choices of the radio controls.
pub(super) fn choices(state: &State<'_>, id: &str) -> Vec<(String, String)> {
    let ctx = state.ctx;
    let tt = |text: &str| ctx.site().i18n().gettext("twentyseventeen", text);
    let pairs: Vec<(&str, String)> = match id {
        "show_on_front" => vec![("posts", ctx.tr("Your latest posts")), ("page", ctx.tr("A static page"))],
        "colorscheme" => vec![("light", tt("Light")), ("dark", tt("Dark")), ("custom", tt("Custom"))],
        "page_layout" => vec![("one-column", tt("One Column")), ("two-column", tt("Two Column"))],
        _ => Vec::new(),
    };
    pairs.into_iter().map(|(value, label)| (value.to_owned(), label)).collect()
}

/// Renders the controls whose markup the server prints, once every setting is known.
pub(super) async fn render_contents(state: &State<'_>, m: &mut Manager) {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let add_page = (ctx.can("publish_pages") && ctx.can("edit_theme_options")).then(|| tr("Add Page"));
    let indices: Vec<usize> = (0..m.controls.len()).collect();
    for index in indices {
        let control = m.controls[index].clone();
        let setting_id = control.settings.first().map(|(_, id)| id.clone()).unwrap_or_default();
        let value = raw_value(m, &setting_id);
        let content = match control.kind.as_str() {
            "text" | "url" | "checkbox" | "radio" | "select" | "textarea" => {
                let choices = choices(state, &control.id);
                field_content(&control, &value, &choices)
            }
            "dropdown-pages" => {
                let selected = match &value {
                    Value::String(text) => text.parse().unwrap_or(0),
                    Value::Number(n) => n.as_u64().unwrap_or(0),
                    _ => 0,
                };
                let name = format!("_customize-dropdown-pages-{}", control.id);
                let dropdown = super::super::common::dropdown_pages(ctx, &name, &name, selected, Some(&tr("&mdash; Select &mdash;"))).await;
                let field = Field { id: &control.id, kind: &control.kind, label: &control.label, description: &control.description, link: link(&setting_id), value: String::new(), value_true: false, choices: &[], input_attrs: &[] };
                let label = add_page.clone().unwrap_or_default();
                let tr_ref: &dyn Fn(&str) -> String = &tr;
                field.dropdown_pages(&dropdown, (control.allow_addition && add_page.is_some()).then_some((label.as_str(), tr_ref)))
            }
            _ => continue,
        };
        m.controls[index].content = content;
    }
}

/// The JSON a control of a subclass adds (`to_json()` and `json()`).
pub(super) async fn control_json(state: &State<'_>, m: &mut Manager) {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let can_upload = ctx.can("upload_files");
    for index in 0..m.controls.len() {
        let control = m.controls[index].clone();
        let setting_id = control.settings.first().map(|(_, id)| id.clone()).unwrap_or_default();
        let value = raw_value(m, &setting_id);
        let mut extra: Vec<(&'static str, Value)> = Vec::new();
        match control.kind.as_str() {
            "media" | "cropped_image" | "site_icon" | "header" => {
                let image = control.kind != "media";
                let mut labels = if image {
                    vec![
                        ("select", tr("Select image")),
                        ("site_icon", tr("Select Site Icon")),
                        ("change", tr("Change image")),
                        ("default", tr("Default")),
                        ("remove", tr("Remove")),
                        ("placeholder", tr("No image selected")),
                        ("frame_title", tr("Select image")),
                        ("frame_button", tr("Choose image")),
                    ]
                } else {
                    vec![
                        ("select", tr("Select video")),
                        ("change", tr("Change video")),
                        ("default", tr("Default")),
                        ("remove", tr("Remove")),
                        ("placeholder", tr("No video selected")),
                        ("frame_title", tr("Select video")),
                        ("frame_button", tr("Choose video")),
                    ]
                };
                if control.id == "custom_logo" {
                    for (key, text) in [("select", "Select logo"), ("change", "Change logo"), ("remove", "Remove"), ("default", "Default"), ("placeholder", "No logo selected"), ("frame_title", "Select logo"), ("frame_button", "Choose logo")] {
                        if let Some(slot) = labels.iter_mut().find(|(k, _)| *k == key) {
                            slot.1 = tr(text);
                        }
                    }
                }
                let labels: serde_json::Map<String, Value> = labels.into_iter().map(|(k, v)| (k.to_owned(), json!(v))).collect();
                extra.push(("mime_type", json!(if image { "image" } else { "video" })));
                extra.push(("button_labels", Value::Object(labels)));
                extra.push(("canUpload", json!(can_upload)));
                let id = match &value {
                    Value::String(text) => text.parse::<u64>().unwrap_or(0),
                    Value::Number(n) => n.as_u64().unwrap_or(0),
                    _ => 0,
                };
                if control.kind != "header" && id > 0
                    && let Some(post) = ctx.env.db.post(id).await.ok().flatten()
                    && let Some(attachment) = crate::screens::ajax_media::attachment_js(ctx, &post).await
                {
                    extra.push(("attachment", attachment));
                }
                match control.kind.as_str() {
                    "cropped_image" => {
                        let (width, height, flex_width, flex_height) = state.custom_logo.unwrap_or((0, 0, false, false));
                        extra.extend([("width", json!(width)), ("height", json!(height)), ("flex_width", json!(u8::from(flex_width))), ("flex_height", json!(u8::from(flex_height)))]);
                    }
                    "site_icon" => extra.extend([("width", json!(512)), ("height", json!(512)), ("flex_width", json!(0)), ("flex_height", json!(0))]),
                    _ => {}
                }
                m.controls[index].label = html_entity_decode(&control.label);
            }
            "code_editor" => {
                let mut code = super::super::file_editor::code_editor_settings(ctx, "custom.css");
                code["codemirror"]["indentUnit"] = json!(2);
                code["codemirror"]["tabSize"] = json!(2);
                m.controls[index].json_extra = vec![
                    ("editor_settings", if state.syntax_highlighting { code } else { json!(false) }),
                    ("input_attrs", json!({"aria-describedby": "editor-keyboard-trap-help-1 editor-keyboard-trap-help-2 editor-keyboard-trap-help-3 editor-keyboard-trap-help-4"})),
                ];
            }
            _ => {}
        }
        if !extra.is_empty() {
            m.controls[index].extra.extend(extra);
        }
    }
}
