//! A page of the website in the preview of the Customizer (`is_customize_preview()`): what
//! `WP_Customize_Manager::setup_theme()` established for the request, and what the templates record while they print
//! (the widgets and sidebars of `WP_Customize_Widgets`, the `wp_nav_menu()` calls of `WP_Customize_Nav_Menus`), which
//! the exports at the end of `wp_footer()` need.

use std::sync::Mutex;

/// The Customizer of one request on the website.
#[derive(Debug)]
pub struct Preview {
    /// `changeset_uuid()`.
    pub uuid: String,
    /// `$messenger_channel`: `sanitize_key( $_REQUEST['customize_messenger_channel'] )`, empty without one. Inside the
    /// frame of the Customizer the page has no toolbar.
    pub channel: String,
    /// The `widgets` component is loaded: the theme is no block theme.
    pub widgets: bool,
    /// `current_theme_supports( 'customize-selective-refresh-widgets' )`: the widgets get the attributes of their partials.
    pub selective_refresh_widgets: bool,
    /// Whether the user may `customize`; a visitor may only look at a changeset that exists.
    pub can_customize: bool,
    /// `autosaved()`: `customize_autosaved` was sent.
    pub autosaved: bool,
    /// The settings of `$_POST['customized']`, in their order (`_dirty`).
    pub dirty: Vec<String>,
    /// The settings of `unsanitized_post_values()`: those of the changeset, then the posted ones.
    pub previewed: Vec<String>,
    /// Those of them that are settings of the Customizer.
    pub known: Vec<String>,
    /// `wp_get_custom_css()` in the preview, when the Additional CSS is previewed.
    pub custom_css: Option<String>,
    /// The stylesheet of the previewed theme when it is not the active one (`customize_theme`).
    pub theme: Option<String>,
    /// The previewed `nav_menu[…]` settings in their order: the term ID (negative for a menu the Customizer adds) and
    /// the sanitized value, `None` for a deleted menu.
    pub nav_menus: Vec<(i64, Option<serde_json::Value>)>,
    /// The previewed `nav_menu_item[…]` settings: the post ID and the sanitized value, `None` for a deleted item.
    pub nav_menu_items: Vec<(i64, Option<serde_json::Value>)>,
    /// The values a setting refuses, with the error `validate_setting_values()` gives for them.
    pub invalid: Vec<(String, serde_json::Value)>,
    rendered: Mutex<Rendered>,
}

/// What the page printed.
#[derive(Debug, Default, Clone)]
pub struct Rendered {
    /// `WP_Customize_Widgets::$rendered_sidebars`, in the order they were seen.
    pub sidebars: Vec<String>,
    /// `$rendered_widgets`.
    pub widgets: Vec<String>,
    /// `$sidebar_instance_count`.
    pub sidebar_instances: Vec<(String, u32)>,
    /// `WP_Customize_Nav_Menus::$preview_nav_menu_instance_args`: the hash and the JSON of the exported arguments.
    pub nav_menu_args: Vec<(String, serde_json::Value)>,
}

impl Preview {
    pub fn new(uuid: &str, channel: &str, widgets: bool, selective_refresh_widgets: bool, can_customize: bool, autosaved: bool) -> Self {
        Self { uuid: uuid.to_owned(), channel: channel.to_owned(), widgets, selective_refresh_widgets, can_customize, autosaved, dirty: Vec::new(), previewed: Vec::new(), known: Vec::new(), custom_css: None, theme: None, nav_menus: Vec::new(), nav_menu_items: Vec::new(), invalid: Vec::new(), rendered: Mutex::default() }
    }

    /// The preview hooks of `WP_Customize_Widgets`, which it adds for a user who may `edit_theme_options` only: the
    /// widgets are counted, marked and exported.
    pub fn widgets_previewed(&self) -> bool {
        self.widgets && self.can_customize
    }

    /// The preview hooks of `WP_Customize_Nav_Menus`, for a user who may `edit_theme_options` only: the menus get the
    /// attributes of their partials and are exported.
    pub fn menus_previewed(&self) -> bool {
        self.can_customize
    }

    /// `validate_setting_values()` of a value: its error, or `true`.
    pub fn validity(&self, id: &str) -> serde_json::Value {
        self.invalid.iter().find(|(i, _)| i == id).map_or(serde_json::Value::Bool(true), |(_, error)| error.clone())
    }

    /// What was recorded so far.
    pub fn rendered(&self) -> Rendered {
        self.rendered.lock().map(|r| r.clone()).unwrap_or_default()
    }

    /// `tally_sidebars_via_is_active_sidebar_calls()` and `tally_sidebars_via_dynamic_sidebar_calls()`.
    pub fn tally_sidebar(&self, id: &str) {
        if let Ok(mut rendered) = self.rendered.lock()
            && !rendered.sidebars.iter().any(|s| s == id)
        {
            rendered.sidebars.push(id.to_owned());
        }
    }

    /// `tally_rendered_widgets()`.
    pub fn tally_widget(&self, id: &str) {
        if let Ok(mut rendered) = self.rendered.lock()
            && !rendered.widgets.iter().any(|w| w == id)
        {
            rendered.widgets.push(id.to_owned());
        }
    }

    /// `start_dynamic_sidebar()`: the number of this call of `dynamic_sidebar()` for the sidebar.
    pub fn start_sidebar(&self, id: &str) -> u32 {
        let Ok(mut rendered) = self.rendered.lock() else { return 1 };
        match rendered.sidebar_instances.iter_mut().find(|(s, _)| s == id) {
            Some((_, count)) => {
                *count += 1;
                *count
            }
            None => {
                rendered.sidebar_instances.push((id.to_owned(), 1));
                1
            }
        }
    }

    /// `filter_wp_nav_menu_args()`: keeps the arguments of a menu that can be refreshed on its own.
    pub fn add_nav_menu_args(&self, hmac: &str, args: serde_json::Value) {
        if let Ok(mut rendered) = self.rendered.lock() {
            match rendered.nav_menu_args.iter_mut().find(|(h, _)| h == hmac) {
                Some(slot) => slot.1 = args,
                None => rendered.nav_menu_args.push((hmac.to_owned(), args)),
            }
        }
    }
}

/// `start_dynamic_sidebar()` and `end_dynamic_sidebar()`: the comments around the widgets of a sidebar.
pub fn sidebar_marker(edge: &str, id: &str, instance: u32) -> String {
    format!("\n<!--dynamic_sidebar_{edge}:{}:{instance}-->\n", crate::formatting::esc_html(id))
}
