//! The frame of every administration screen: head, toolbar, main menu, footer and the scripts.

use rustpress_auth::CurrentUser;
use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

use crate::adminbar::{self, BarContext};
use crate::menu::AdminMenu;
use crate::registry::{LoaderEnv, default_scripts, default_styles};

/// The screen being shown (`$pagenow`, `$parent_file`, `$submenu_file`, `$typenow`).
#[derive(Debug, Clone, Default)]
pub struct Screen {
    /// `options-general.php`
    pub file: String,
    /// The top-level entry the screen belongs to; defaults to `file`.
    pub parent_file: String,
    /// The highlighted submenu entry; defaults to `file`.
    pub submenu_file: Option<String>,
    pub post_type: String,
    /// `get_current_screen()->id` when the file does not tell it (`edit-category`).
    pub id: String,
    /// `$title`, in plain text.
    pub title: String,
    /// The contextual help (`get_current_screen()->add_help_tab()`).
    pub help: Option<Help>,
    /// "Screen Options": the columns that can be hidden, the items per page, the view mode.
    pub options: Option<ScreenOptions>,
    /// The "View Post" entry of the toolbar: `(label, address)`.
    pub view_item: Option<(String, String)>,
    /// `view_item` is the archive of the post type.
    pub view_archive: bool,
    /// The block editor takes the whole window (`is-fullscreen-mode`).
    pub fullscreen: bool,
    /// More classes of the `body` element.
    pub body_classes: Vec<String>,
    /// Classes of the `admin_body_class` filter: they come right after `no-js`.
    pub early_classes: Vec<String>,
    /// The content has no indentation of its own: the frame leaves none either.
    pub flush_content: bool,
    /// The page's markup follows the help and the screen options on the same line (`?>` right before it in the
    /// PHP file); otherwise a line break comes first.
    pub content_inline: bool,
    /// `$hook_suffix` of a plugin's page (`settings_page_svg-support`): it is the screen's ID, `adminpage` and body
    /// class.
    pub hook_suffix: Option<String>,
}

impl Screen {
    /// `$current_screen->id`, the `pagenow` of the page.
    pub fn screen_id(&self) -> String {
        if !self.id.is_empty() {
            return self.id.clone();
        }
        if let Some(hook) = &self.hook_suffix {
            return hook.clone();
        }
        let base = self.file.trim_end_matches(".php");
        let post_type = if self.post_type.is_empty() { "post" } else { self.post_type.as_str() };
        match base {
            "index" => "dashboard".to_owned(),
            "edit" => format!("edit-{post_type}"),
            "post" | "post-new" => post_type.to_owned(),
            "media-new" => "media".to_owned(),
            "user-new" => "user".to_owned(),
            other => other.to_owned(),
        }
    }

    /// `$typenow`.
    pub fn typenow(&self) -> String {
        if self.post_type.is_empty() && self.file == "upload.php" { "attachment".to_owned() } else { self.post_type.clone() }
    }
}

/// One tab of the contextual help.
#[derive(Debug, Clone)]
pub struct HelpTab {
    pub id: String,
    pub title: String,
    /// HTML.
    pub content: String,
}

/// The contextual help of a screen.
#[derive(Debug, Clone, Default)]
pub struct Help {
    pub tabs: Vec<HelpTab>,
    /// HTML of the sidebar ("For more information").
    pub sidebar: String,
}

/// What `WP_Screen::render_screen_options()` shows for a list table.
#[derive(Debug, Clone, Default)]
pub struct ScreenOptions {
    /// `(column, title, hidden)` of the columns that can be hidden (`render_list_table_columns_preferences()`).
    pub columns: Vec<(String, String, bool)>,
    /// `(option, value)` of "Number of items per page" (`render_per_page_options()`).
    pub per_page: Option<(String, u64)>,
    /// The view mode (`list` or `excerpt`) of the posts and comments lists (`render_view_mode()`).
    pub view_mode: Option<String>,
    /// `(id, title, hidden)` of the meta boxes that can be hidden (`meta_box_prefs()`); `None` without meta boxes.
    pub meta_boxes: Option<Vec<(String, String, bool)>>,
    /// Whether the welcome panel of the dashboard shows; `None` where there is none.
    pub welcome: Option<bool>,
    /// `(max, current)` of the `layout_columns` screen option (`render_screen_layout()`).
    pub layout_columns: Option<(u32, u32)>,
    /// The "Additional settings" of an edit screen: whether the full-height editor is on (`editor_expand`).
    pub editor_expand: Option<bool>,
}

/// `WP_Screen::render_screen_meta()`: the help, the screen options and the buttons that open them.
fn screen_meta_with(help: Option<&Help>, options: Option<&ScreenOptions>, tr: &dyn Fn(&str) -> String, tr_x: &dyn Fn(&str, &str) -> String, nonce: &str) -> String {
    let empty = Help::default();
    let help = help.unwrap_or(&empty);
    let full = screen_meta(help, &tr("Help"));
    // The help part ends with the wrapper of the help; what follows depends on the options and the buttons.
    let mut help_part = if help.tabs.is_empty() {
        full.strip_suffix("\t\t\t\t</div>\n\t\t").unwrap_or(&full).to_owned()
    } else {
        full.split("\t\t\t\t</div>\n\t\t\t\t<div id=\"screen-meta-links\">").next().unwrap_or(&full).to_owned()
    };
    if !help.tabs.is_empty() && help.sidebar.is_empty() {
        help_part = help_part.replacen("id=\"contextual-help-wrap\" class=\"hidden\"", "id=\"contextual-help-wrap\" class=\"hidden no-sidebar\"", 1);
    }
    let mut out = help_part;
    out.push_str("\t\t");
    if let Some(options) = options {
        out.push_str(&format!("<div id=\"screen-options-wrap\" class=\"hidden\" tabindex=\"-1\" aria-label=\"{}\">\n<form id='adv-settings' method='post'>\n", esc_attr(&tr("Screen Options Tab"))));
        // `render_meta_boxes_preferences()`
        if let Some(boxes) = &options.meta_boxes {
            out.push_str(&format!(
                "\t\t<fieldset class=\"metabox-prefs\">\n\t\t<legend>{}</legend>\n\t\t<p>\n\t\t\t{}\t\t\t{}\t\t</p>\n\t\t<div class=\"metabox-prefs-container\">\n\t\t",
                tr("Screen elements"),
                tr("Some screen elements can be shown or hidden by using the checkboxes."),
                tr("Expand or collapse the elements by clicking on their headings, and arrange them by dragging their headings or by clicking on the up and down arrows.")
            ));
            for (id, title, hidden) in boxes {
                let id = esc_attr(id);
                out.push_str(&format!(
                    "<label for=\"{id}-hide\"><input class=\"hide-postbox-tog\" name=\"{id}-hide\" type=\"checkbox\" id=\"{id}-hide\" value=\"{id}\" {} />{title}</label>",
                    if *hidden { "" } else { " checked='checked'" }
                ));
            }
            if let Some(shown) = options.welcome {
                out.push_str(&format!(
                    "<label for=\"wp_welcome_panel-hide\"><input type=\"checkbox\" id=\"wp_welcome_panel-hide\"{} />{}</label>\n",
                    if shown { " checked='checked'" } else { "" },
                    tr_x("Welcome", "Welcome panel")
                ));
            }
            out.push_str("\t\t</div>\n\t\t</fieldset>\n\t\t");
        }
        if !options.columns.is_empty() {
            out.push_str(&format!("\t\t<fieldset class=\"metabox-prefs\">\n\t\t<legend>{}</legend>\n\t\t", tr("Columns")));
            for (column, title, hidden) in &options.columns {
                let id = format!("{column}-hide");
                out.push_str(&format!(
                    "<label><input class=\"hide-column-tog\" name=\"{id}\" type=\"checkbox\" id=\"{id}\" value=\"{column}\"{} />{}</label>\n",
                    if *hidden { "" } else { " checked='checked'" },
                    rustpress_core::formatting::wp_strip_all_tags(title)
                ));
            }
            out.push_str("\t\t</fieldset>\n\t\t");
        }
        if let Some((max, current)) = options.layout_columns {
            out.push_str(&format!("\t\t<fieldset class='columns-prefs'>\n\t\t<legend class=\"screen-layout\">{}</legend>\n\t\t", tr("Layout")));
            for i in 1..=max {
                let label = if i == 1 { tr("%s column") } else { tr("%s columns") };
                out.push_str(&format!(
                    "\t\t\t<label class=\"columns-prefs-{i}\">\n\t\t\t<input type='radio' name='screen_columns' value='{i}' {} />\n\t\t\t{}\t\t\t</label>\n\t\t",
                    if current == i { " checked='checked'" } else { "" },
                    label.replace("%s", &i.to_string())
                ));
            }
            out.push_str("\t\t</fieldset>\n\t\t");
        }
        if let Some((option, value)) = &options.per_page {
            out.push_str(&format!(
                "\t\t<fieldset class=\"screen-options\">\n\t\t<legend>{}</legend>\n\t\t\t\t\t\t\t<label for=\"{o}\">{}</label>\n\t\t\t\t<input type=\"number\" step=\"1\" min=\"1\" max=\"999\" class=\"screen-per-page small-text\" name=\"wp_screen_options[value]\"\n\t\t\t\t\tid=\"{o}\"\n\t\t\t\t\tvalue=\"{value}\" />\n\t\t\t\t\t\t\t<input type=\"hidden\" name=\"wp_screen_options[option]\" value=\"{o}\" />\n\t\t</fieldset>\n\t\t",
                tr("Pagination"),
                tr("Number of items per page:"),
                o = esc_attr(option)
            ));
        }
        if let Some(mode) = &options.view_mode {
            let checked = |value: &str| if mode == value { " checked='checked'" } else { "" };
            out.push_str(&format!(
                "\t\t<fieldset class=\"metabox-prefs view-mode\">\n\t\t\t<legend>{}</legend>\n\t\t\t<label for=\"list-view-mode\">\n\t\t\t\t<input id=\"list-view-mode\" type=\"radio\" name=\"mode\" value=\"list\" {} />\n\t\t\t\t{}\t\t\t</label>\n\t\t\t<label for=\"excerpt-view-mode\">\n\t\t\t\t<input id=\"excerpt-view-mode\" type=\"radio\" name=\"mode\" value=\"excerpt\" {} />\n\t\t\t\t{}\t\t\t</label>\n\t\t</fieldset>\n\t\t",
                tr("View mode"),
                checked("list"),
                tr("Compact view"),
                checked("excerpt"),
                tr("Extended view")
            ));
        }
        // `WP_Screen::$_screen_settings` of the edit screens.
        if let Some(expand) = options.editor_expand {
            out.push_str(&format!(
                "<fieldset class=\"editor-expand hidden\"><legend>{}</legend><label for=\"editor-expand-toggle\"><input type=\"checkbox\" id=\"editor-expand-toggle\"{} />{}</label></fieldset>",
                tr("Additional settings"),
                if expand { " checked='checked'" } else { "" },
                tr("Enable full-height editor and distraction-free functionality.")
            ));
        }
        // `screen_options_show_submit`: the items per page and the view mode need the button.
        if options.per_page.is_some() || options.view_mode.is_some() {
            out.push_str(&format!("<p class=\"submit\"><input type=\"submit\" name=\"screen-options-apply\" id=\"screen-options-apply\" class=\"button button-primary\" value=\"{}\"  /></p>", esc_attr(&tr("Apply"))));
        }
        out.push_str(&format!("\n<input type=\"hidden\" id=\"screenoptionnonce\" name=\"screenoptionnonce\" value=\"{nonce}\" />\n</form>\n</div>"));
    }
    out.push_str("\t\t</div>\n\t\t");
    if help.tabs.is_empty() && options.is_none() {
        return out;
    }
    out.push_str("\t\t<div id=\"screen-meta-links\">\n\t\t");
    if options.is_some() {
        out.push_str(&format!("\t\t\t<div id=\"screen-options-link-wrap\" class=\"hide-if-no-js screen-meta-toggle\">\n\t\t\t<button type=\"button\" id=\"show-settings-link\" class=\"button button-compact show-settings\" aria-controls=\"screen-options-wrap\" aria-expanded=\"false\">{}</button>\n\t\t\t</div>\n\t\t\t", tr("Screen Options")));
    }
    if !help.tabs.is_empty() {
        out.push_str(&format!("\t\t\t<div id=\"contextual-help-link-wrap\" class=\"hide-if-no-js screen-meta-toggle\">\n\t\t\t<button type=\"button\" id=\"contextual-help-link\" class=\"button button-compact show-settings\" aria-controls=\"contextual-help-wrap\" aria-expanded=\"false\">{}</button>\n\t\t\t</div>\n\t\t", esc_html(&tr("Help"))));
    }
    out.push_str("\t\t</div>\n\t\t\n");
    out
}

/// `WP_Screen::render_screen_meta()` with the help tabs and the Help button.
fn screen_meta(help: &Help, label: &str) -> String {
    if help.tabs.is_empty() {
        // A screen without help tabs: the empty wrapper and no Help button.
        return "		<div id=\"screen-meta\" class=\"metabox-prefs\">

			<div id=\"contextual-help-wrap\" class=\"hidden no-sidebar\" tabindex=\"-1\" aria-label=\"Contextual Help Tab\">
				<div id=\"contextual-help-back\"></div>
				<div id=\"contextual-help-columns\">
					<div class=\"contextual-help-tabs\">
						<ul>
												</ul>
					</div>

					
					<div class=\"contextual-help-tabs-wrap\">
											</div>
				</div>
			</div>
				</div>
		".to_owned();
    }
    let mut tabs = String::new();
    let mut panels = String::new();
    for (index, tab) in help.tabs.iter().enumerate() {
        let active = if index == 0 { " class=\"active\"" } else { "" };
        let panel_active = if index == 0 { " active" } else { "" };
        tabs.push_str(&format!(
            "
							<li id=\"tab-link-{id}\"{active}>
								<a href=\"#tab-panel-{id}\" aria-controls=\"tab-panel-{id}\">
									{title}								</a>
							</li>
							",
            id = esc_attr(&tab.id),
            title = esc_html(&tab.title)
        ));
        panels.push_str(&format!("
							<div id=\"tab-panel-{}\" class=\"help-tab-content{panel_active}\">
								{}							</div>
							", esc_attr(&tab.id), tab.content));
    }
    let sidebar = if help.sidebar.is_empty() { String::new() } else { format!("										<div class=\"contextual-help-sidebar\">
						{}					</div>
", help.sidebar) };
    format!(
        "		<div id=\"screen-meta\" class=\"metabox-prefs\">

			<div id=\"contextual-help-wrap\" class=\"hidden\" tabindex=\"-1\" aria-label=\"Contextual Help Tab\">
				<div id=\"contextual-help-back\"></div>
				<div id=\"contextual-help-columns\">
					<div class=\"contextual-help-tabs\">
						<ul>
						{tabs}						</ul>
					</div>

{sidebar}					
					<div class=\"contextual-help-tabs-wrap\">
						{panels}					</div>
				</div>
			</div>
				</div>
				<div id=\"screen-meta-links\">
					<div id=\"contextual-help-link-wrap\" class=\"hide-if-no-js screen-meta-toggle\">
			<button type=\"button\" id=\"contextual-help-link\" class=\"button button-compact show-settings\" aria-controls=\"contextual-help-wrap\" aria-expanded=\"false\">{}</button>
			</div>
				</div>
		
",
        esc_html(label)
    )
}

/// What the frame needs besides the screen.
pub struct ChromeEnv<'a> {
    pub site: &'a Site,
    pub loader: LoaderEnv,
    pub user: &'a CurrentUser,
    pub menu: &'a AdminMenu,
    pub bar: BarContext,
    /// `get_stylesheet()`, for the `wp-theme-…` body class.
    pub stylesheet: String,
    /// `COOKIEPATH`, `time()`, whether the sign-in is over TLS: `userSettings`.
    pub cookie_path: String,
    pub now: i64,
    pub secure: bool,
    /// `$_SERVER['REQUEST_URI']` and `$_SERVER['HTTP_HOST']`.
    pub request_uri: String,
    pub host: Option<String>,
    /// What `admin_print_styles` prints after the colors: view transitions, emoji styles and the `@font-face` rules.
    pub head_styles: String,
}

/// `wp_removable_query_args()`.
const REMOVABLE_QUERY_ARGS: &[&str] = &[
    "activate", "activated", "admin_email_remind_later", "approved", "core-major-auto-updates-saved", "deactivate", "delete_count", "deleted", "disabled", "doing_wp_cron", "enabled", "error", "hotkeys_highlight_first",
    "hotkeys_highlight_last", "ids", "locked", "message", "same", "saved", "settings-updated", "skipped", "spammed", "trashed", "unspammed", "untrashed", "update", "updated", "wp-post-new-reload",
];

/// Extra assets of a screen.
#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub styles: Vec<String>,
    pub scripts: Vec<String>,
    /// Scripts a page enqueues before `admin-header.php` runs: they come before `svg-painter` and `heartbeat`.
    pub early_scripts: Vec<String>,
    /// Scripts a screen enqueues on `admin_enqueue_scripts`: after those of every screen.
    pub late_scripts: Vec<String>,
    /// Enqueued by the page after `admin-header.php` has printed the head: they are printed in the footer.
    pub post_header_scripts: Vec<String>,
    pub post_header_styles: Vec<String>,
    /// Printed in the head after the scripts (`admin_head`).
    pub head: String,
    /// Printed after the scripts of the footer (`admin_print_footer_scripts`, late).
    pub footer: String,
    /// Printed right after the scripts of the footer, before the emoji script (`admin_print_footer_scripts` of a page).
    pub footer_early: String,
    /// Printed before the scripts of the footer (`admin_footer`: the media templates).
    pub pre_footer: String,
    /// `wp_localize_script()`: `(handle, object name, data)`.
    pub localize: Vec<(String, String, serde_json::Value)>,
    /// Code added to the data of a script as it is (`$wp_scripts->add_data( $handle, 'data', ... )`), after the
    /// localizations: `(handle, code)`.
    pub raw_data: Vec<(String, String)>,
    /// `wp_add_inline_script()`: `(handle, code, position)`.
    pub inline: Vec<(String, String, String)>,
    /// The screen runs the block editor: its script modules (math blocks, routing) are loaded too.
    pub editor_modules: bool,
    /// The site editor enqueues the router before the math module.
    pub route_first: bool,
    /// The post editor prints no emoji detection script.
    pub no_emoji: bool,
    /// The update screens print neither the heartbeat nor the session check (`wp_auth_check_load()`).
    pub no_auth_check: bool,
    /// The page emptied the global `$locale` (options-general.php when the site language is not installed): the
    /// session check asks for no language.
    pub empty_locale: bool,
    /// A page of `wp-includes/build/pages` (font library, connectors): started by `@wordpress/boot`.
    pub boot: Option<Boot>,
    /// `wp_add_inline_style()`: `(handle, code)`; a handle that is not registered gets one without a file.
    pub inline_styles: Vec<(String, String)>,
    /// Printed in the head after the styles, before the scripts (`admin_head` of the editor).
    pub head_early: String,
    /// The scripts plugins enqueue on `admin_enqueue_scripts`.
    pub plugin_scripts: Vec<rustpress_core::native::Asset>,
    /// The links of the stylesheets plugins enqueue: printed with the styles that are not concatenated, before the
    /// emoji styles.
    pub plugin_styles: String,
    /// What the plugins print on `admin_notices`, the end of `admin-header.php`.
    pub notices: String,
    /// What a plugin's `admin_footer_text` prints instead of the thanks for using WordPress.
    pub footer_text: Option<String>,
}

/// A page that `@wordpress/boot` renders, with the routes `wp-includes/build/routes/registry.php` lists for it.
#[derive(Debug, Clone, Default)]
pub struct Boot {
    /// The slug of the page (`font-library`).
    pub page: String,
    pub routes: Vec<BootRoute>,
    /// More script modules the page needs: `(id, path below script-modules, e.g. "connectors/index")`.
    pub extra_modules: Vec<(String, String)>,
    /// `script_module_data_<page>-wp-admin`: JSON.
    pub module_data: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BootRoute {
    pub path: String,
    pub name: String,
    pub has_route: bool,
    pub has_content: bool,
}

impl Boot {
    /// The module ids of a route: `(route module, content module)`.
    fn module_ids(route: &BootRoute) -> (Option<String>, Option<String>) {
        (route.has_route.then(|| format!("wp/routes/{}/route", route.name)), route.has_content.then(|| format!("wp/routes/{}/content", route.name)))
    }

    fn mount_id(&self) -> String {
        format!("{}-wp-admin-app", self.page)
    }

    fn prerequisites(&self) -> String {
        format!("{}-wp-admin-prerequisites", self.page)
    }
}

/// The strings of the first `array(...)` after `key` in an `*.asset.php` file.
fn asset_list(includes: &std::path::Path, file: &str, key: &str) -> Vec<String> {
    let asset = rustpress_core::files::read_string(&includes.join(file));
    let Some(rest) = asset.split(&format!("'{key}' => array(")).nth(1) else { return Vec::new() };
    let body = rest.split(')').next().unwrap_or_default();
    body.split('\'').skip(1).step_by(2).map(str::to_owned).collect()
}

fn asset_version(includes: &std::path::Path, file: &str) -> String {
    let asset = rustpress_core::files::read_string(&includes.join(file));
    asset.split("'version' => '").nth(1).and_then(|rest| rest.split('\'').next()).unwrap_or_default().to_owned()
}

/// The `module_dependencies` ids of an `*.asset.php` file.
fn asset_module_dependencies(includes: &std::path::Path, file: &str) -> Vec<String> {
    let asset = rustpress_core::files::read_string(&includes.join(file));
    let Some(rest) = asset.split("'module_dependencies' => array(").nth(1) else { return Vec::new() };
    let body = rest.split("), 'version'").next().unwrap_or_default();
    body.split("'id' => '").skip(1).filter_map(|part| part.split('\'').next()).map(str::to_owned).collect()
}

/// `print_emoji_detection_script()`: the settings and the detection script, inline.
fn emoji_scripts(env: &ChromeEnv<'_>, editor: bool) -> String {
    if editor {
        return String::new();
    }
    let root = env.loader.site_url.trim_end_matches('/');
    let settings = serde_json::json!({
        "baseUrl": "https://s.w.org/images/core/emoji/17.0.2/72x72/",
        "ext": ".png",
        "svgUrl": "https://s.w.org/images/core/emoji/17.0.2/svg/",
        "svgExt": ".svg",
        "source": {"concatemoji": format!("{root}/wp-includes/js/wp-emoji-release.min.js?ver={}", env.loader.version)}
    });
    let loader = rustpress_core::files::read_string(&env.loader.includes.join("js").join("wp-emoji-loader.min.js"));
    let json = rustpress_blocks::php_json::encode_with(&settings, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false });
    format!("<script id=\"wp-emoji-settings\" type=\"application/json\">\n{json}\n</script>\n<script type=\"module\">\n{}\n//# sourceURL={root}/wp-includes/js/wp-emoji-loader.min.js\n</script>\n", loader.trim_end())
}

/// The root-level text of an HTML label (`$extract_root_text()` of `wp_enqueue_command_palette_assets()`).
fn root_text(label: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    let mut rest = label;
    while !rest.is_empty() {
        match rest.find('<') {
            Some(at) => {
                if depth == 0 {
                    out.push_str(&rest[..at]);
                }
                let Some(end) = rest[at..].find('>') else { break };
                let tag = &rest[at + 1..at + end];
                if tag.starts_with('/') {
                    depth = depth.saturating_sub(1);
                } else if !tag.ends_with('/') {
                    depth += 1;
                }
                rest = &rest[at + end + 1..];
            }
            None => {
                if depth == 0 {
                    out.push_str(rest);
                }
                break;
            }
        }
    }
    out.trim().to_owned()
}

/// `wp.coreCommands.initializeCommandPalette( {...} )` with the entries of the menu.
fn command_palette_call(env: &ChromeEnv<'_>) -> String {
    let site = env.site;
    let url_of = |slug: &str| if slug.contains(".php") { slug.to_owned() } else { format!("admin.php?page={slug}") };
    let mut commands: Vec<serde_json::Value> = Vec::new();
    for item in &env.menu.items {
        let label = root_text(&item.title);
        if label.is_empty() {
            continue;
        }
        commands.push(serde_json::json!({"label": label, "url": url_of(&item.slug), "name": item.slug}));
        if let Some((_, list)) = env.menu.submenu.iter().find(|(parent, _)| *parent == item.slug) {
            for sub in list {
                let sub_label = root_text(&sub.title);
                if sub_label.is_empty() {
                    continue;
                }
                let url = sub.href.as_ref().map_or_else(|| url_of(&sub.slug), |href| site.site_url(&format!("wp-admin/{href}")));
                commands.push(serde_json::json!({"label": rustpress_core::i18n::sprintf(&site.tr("%1$s > %2$s"), &[&label, &sub_label]), "url": url, "name": format!("{}-{}", item.slug, sub.slug)}));
            }
        }
    }
    let json = rustpress_blocks::php_json::encode_with(&serde_json::json!({"is_network_admin": false, "menu_commands": commands}), rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false });
    format!("wp.coreCommands.initializeCommandPalette( {} );", json.replace('<', &format!("{}u003C", char::from(92u8))).replace('>', &format!("{}u003E", char::from(92u8))))
}

/// The `version` of a script module (`wp-includes/js/dist/script-modules/<path>.min.asset.php`).
fn module_version(includes: &std::path::Path, path: &str) -> String {
    let asset = rustpress_core::files::read_string(&includes.join("js/dist/script-modules").join(format!("{path}.min.asset.php")));
    asset.split("'version' => '").nth(1).and_then(|rest| rest.split('\'').next()).unwrap_or_default().to_owned()
}

/// `WP_Script_Modules::print_import_map()` and `print_enqueued_script_modules()`.
fn script_modules(env: &ChromeEnv<'_>, editor: bool, route_first: bool, boot: Option<&Boot>) -> String {
    let root = env.loader.site_url.trim_end_matches('/').to_owned();
    let includes = &env.loader.includes;
    let url = |path: &str| format!("{root}/wp-includes/js/dist/script-modules/{path}.min.js?ver={}", module_version(includes, path));
    let mut imports = vec![("@wordpress/video-conversion/worker".to_owned(), url("video-conversion/worker")), ("@wordpress/vips/worker".to_owned(), url("vips/worker"))];
    if editor {
        imports.push(("@wordpress/latex-to-mathml".to_owned(), url("latex-to-mathml/index")));
        let route = ("@wordpress/route".to_owned(), url("route/index"));
        if route_first {
            imports.insert(imports.len() - 1, route);
        } else {
            imports.push(route);
        }
    }
    // The modules of a boot page: the router first, the routes after the boot module, then what boot depends on.
    let route_url = |name: &str, kind: &str| {
        let version = asset_version(includes, &format!("build/routes/{name}/{kind}.min.asset.php"));
        format!("{root}/wp-includes/build/routes/{name}/{kind}.min.js?ver={version}")
    };
    let mut preloads = String::new();
    if let Some(boot) = boot {
        let preload = |id: &str, src: &str, low: bool| format!("<link rel=\"modulepreload\" href=\"{src}\" id=\"{id}-js-modulepreload\"{}>\n", if low { " data-wp-fetchpriority=\"low\"" } else { "" });
        imports.insert(0, ("@wordpress/route".to_owned(), url("route/index")));
        imports.push(("@wordpress/boot".to_owned(), url("boot/index")));
        for route in &boot.routes {
            if route.has_route {
                imports.push((format!("wp/routes/{}/route", route.name), route_url(&route.name, "route")));
            }
            if route.has_content {
                imports.push((format!("wp/routes/{}/content", route.name), route_url(&route.name, "content")));
            }
        }
        for dependency in asset_module_dependencies(includes, "js/dist/script-modules/boot/index.min.asset.php") {
            if dependency != "@wordpress/route" {
                imports.push((dependency.clone(), url(&format!("{}/index", dependency.trim_start_matches("@wordpress/")))));
            }
        }
        for (id, path) in &boot.extra_modules {
            imports.push((id.clone(), url(path)));
        }
        preloads.push_str(&preload("@wordpress/a11y", &url("a11y/index"), true));
        preloads.push_str(&preload("@wordpress/route", &url("route/index"), false));
        preloads.push_str(&preload("@wordpress/boot", &url("boot/index"), false));
        for route in &boot.routes {
            if route.has_route {
                preloads.push_str(&preload(&format!("wp/routes/{}/route", route.name), &route_url(&route.name, "route"), false));
            }
        }
        if let Some(data) = &boot.module_data {
            preloads.push_str(&format!("<script id=\"wp-script-module-data-{}-wp-admin\" type=\"application/json\">\n{data}\n</script>\n", boot.page));
        }
    }
    let map: serde_json::Map<String, serde_json::Value> = imports.into_iter().map(|(id, src)| (id, serde_json::Value::from(src))).collect();
    let json = rustpress_blocks::php_json::encode_with(&serde_json::json!({"imports": map}), rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: true });
    let mut out = format!("<script id=\"wp-importmap\" type=\"importmap\">\n{json}\n</script>\n");
    if editor {
        out.push_str(&format!("<script id=\"@wordpress/latex-to-mathml/loader-js-module\" src=\"{}\" type=\"module\"></script>\n", url("latex-to-mathml/loader")));
    }
    if let Some(boot) = boot {
        out.push_str(&format!("<script id=\"{0}-wp-admin-js-module\" src=\"{root}/wp-includes/build/pages/{0}/loader.js?ver={1}\" type=\"module\"></script>\n", boot.page, env.loader.version));
        out.push_str(&preloads);
    }
    out
}

/// `wp-admin/admin-header.php` to `admin-footer.php` around `content` (the inside of `.wrap`, with
/// its own `<div class="wrap">`).
pub fn render(env: &ChromeEnv<'_>, screen: &Screen, content: &str, assets: &Assets) -> String {
    let site = env.site;
    let tr = |s: &str| site.tr(s);
    let version = env.loader.version.clone();
    let rtl = site.is_rtl();
    let page = screen.file.trim_end_matches(".php");
    let parent_file = if screen.parent_file.is_empty() { screen.file.clone() } else { screen.parent_file.clone() };
    let submenu_file = screen.submenu_file.clone().or_else(|| Some(screen.file.clone()));
    let blog_name = site.options.string("blogname", "");
    let admin_title = if blog_name.is_empty() { rustpress_core::formatting::wptexturize(&site.options.string("siteurl", "")) } else { rustpress_core::formatting::wptexturize(&blog_name) };
    let title = format!("{} &lsaquo; {} &#8212; WordPress", esc_html(&screen.title), esc_html(&admin_title).replace('\'', "&#039;"));

    // Styles of the head.
    let mut styles = default_styles(&env.loader);
    styles.add_inline_style("admin-bar", "@media print { #wpadminbar { display:none; } }");
    for (handle, code) in &assets.inline_styles {
        if !styles.is_registered(handle) {
            styles.add(handle, None, &[], crate::deps::Ver::None, None);
        }
        styles.add_inline_style(handle, code);
    }
    // The styles a screen asks for (`wp_enqueue_media()`) are enqueued before `admin-header.php` adds its own.
    if !assets.styles.is_empty() {
        styles.enqueue("admin-bar");
        for handle in &assets.styles {
            styles.enqueue(handle);
        }
    }
    for handle in [
        "dashicons", "admin-bar", "common", "forms", "admin-menu", "dashboard", "list-tables", "edit", "revisions", "media", "themes", "about", "nav-menus", "wp-pointer", "widgets", "site-icon", "l10n", "wp-base-styles", "wp-tooltip", "buttons",
        "wp-auth-check", "wp-theme", "wp-components", "wp-commands",
    ] {
        if handle == "wp-auth-check" && assets.no_auth_check {
            continue;
        }
        styles.enqueue(handle);
    }
    // `wp_register_style( '<page>-wp-admin-prerequisites', false, <the dependencies that have styles> )`.
    if let Some(boot) = &assets.boot {
        let deps = asset_list(&env.loader.includes, "js/dist/script-modules/boot/index.min.asset.php", "dependencies");
        let deps: Vec<&str> = deps.iter().map(String::as_str).filter(|handle| styles.is_registered(handle)).collect();
        let version = asset_version(&env.loader.includes, "js/dist/script-modules/boot/index.min.asset.php");
        styles.add(&boot.prerequisites(), None, &deps, crate::deps::Ver::Value(version), None);
        styles.enqueue(&boot.prerequisites());
    }
    for handle in &assets.styles {
        styles.enqueue(handle);
    }
    let admin_styles = styles.print_admin_styles(true);
    let colors = format!("<link rel='stylesheet' id='colors-css' href='{}/wp-admin/css/colors/modern/colors.min.css?ver={version}' media='all' />\n", env.loader.site_url);

    // Scripts of the head (`utils` with `userSettings`).
    let mut scripts = default_scripts(&env.loader);
    // `enqueue_editor_block_styles_assets()` registers a script without a file for the styles of the blocks.
    if assets.scripts.iter().chain(&assets.early_scripts).any(|h| h == "wp-block-styles") {
        scripts.add("wp-block-styles", None, &["wp-blocks"], crate::deps::Ver::Default, Some(crate::deps::Arg::Int(1)));
    }
    if let Some(boot) = &assets.boot {
        // `wp_register_script( '<page>-wp-admin-prerequisites', '', $asset['dependencies'], ... )` and its init call.
        let deps = asset_list(&env.loader.includes, "js/dist/script-modules/boot/index.min.asset.php", "dependencies");
        let deps: Vec<&str> = deps.iter().map(String::as_str).collect();
        let version = asset_version(&env.loader.includes, "js/dist/script-modules/boot/index.min.asset.php");
        scripts.add(&boot.prerequisites(), Some(""), &deps, crate::deps::Ver::Value(version), Some(crate::deps::Arg::Int(1)));
        let routes: Vec<serde_json::Value> = boot
            .routes
            .iter()
            .map(|route| {
                let (route_module, content_module) = Boot::module_ids(route);
                let mut entry = serde_json::Map::new();
                entry.insert("path".into(), route.path.clone().into());
                if let Some(id) = content_module {
                    entry.insert("content_module".into(), id.into());
                }
                if let Some(id) = route_module {
                    entry.insert("route_module".into(), id.into());
                }
                serde_json::Value::Object(entry)
            })
            .collect();
        let flags = rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false };
        let hex = |value: &serde_json::Value| rustpress_blocks::php_json::encode_with(value, flags).replace('<', &format!("{}u003C", char::from(92u8))).replace('>', &format!("{}u003E", char::from(92u8)));
        let init = "( mountId, routes, initModules ) => {\n\tconst run = async () => {\n\t\tconst mod = await import( \"@wordpress/boot\" );\n\t\tmod.initSinglePage( { mountId, routes, initModules } );\n\t};\n\tif ( document.readyState === \"loading\" ) {\n\t\tdocument.addEventListener( \"DOMContentLoaded\", run );\n\t} else {\n\t\trun();\n\t}\n}";
        scripts.add_inline_script(&boot.prerequisites(), &format!("( {init} )( {}, {}, [] );", hex(&serde_json::Value::from(boot.mount_id())), hex(&serde_json::Value::Array(routes))), "after");
    }
    scripts.relocalize(
        "utils",
        "userSettings",
        &serde_json::json!({"url": env.cookie_path, "uid": env.user.id, "time": env.now, "secure": if env.secure { "1" } else { "" }}),
    );
    // `wp_heartbeat_settings()` and `wp_heartbeat_set_suspension()`.
    let mut heartbeat = serde_json::Map::new();
    if let Some(nonce) = env.loader.nonce.as_ref() {
        heartbeat.insert("nonce".into(), serde_json::Value::String(nonce("heartbeat-nonce")));
    }
    if matches!(screen.file.as_str(), "post.php" | "post-new.php") {
        heartbeat.insert("suspension".into(), serde_json::Value::String("disable".into()));
    }
    scripts.relocalize("heartbeat", "heartbeatSettings", &serde_json::Value::Object(heartbeat));
    // A screen enqueues its scripts before `admin-header.php` adds the ones of every screen.
    for handle in ["common", "admin-bar"].into_iter().chain(assets.early_scripts.iter().map(String::as_str)).chain(assets.scripts.iter().map(String::as_str)) {
        scripts.enqueue(handle);
    }
    for (handle, object, data) in &assets.localize {
        scripts.localize_over(handle, object, data);
    }
    for (handle, code) in &assets.raw_data {
        scripts.append_data(handle, code);
    }
    for (handle, code, position) in &assets.inline {
        scripts.add_inline_script(handle, code, position);
    }
    let always: &[&str] = if assets.no_auth_check { &["svg-painter"] } else { &["svg-painter", "heartbeat", "wp-auth-check"] };
    for handle in always {
        scripts.enqueue(handle);
    }
    // `wp_enqueue_command_palette_assets()` runs for every screen, last.
    scripts.enqueue("wp-commands");
    scripts.enqueue("wp-core-commands");
    scripts.add_inline_script("wp-core-commands", &command_palette_call(env), "after");
    for script in &assets.plugin_scripts {
        let deps: Vec<&str> = script.deps.iter().map(String::as_str).collect();
        let ver = script.ver.clone().map_or(crate::deps::Ver::None, crate::deps::Ver::Value);
        scripts.add(&script.handle, Some(&script.src), &deps, ver, script.in_footer.then_some(crate::deps::Arg::Int(1)));
        if let Some(extra) = &script.extra {
            scripts.append_data(&script.handle, extra);
        }
        for code in &script.before {
            scripts.add_inline_script(&script.handle, code, "before");
        }
        for code in &script.after {
            scripts.add_inline_script(&script.handle, code, "after");
        }
        scripts.enqueue(&script.handle);
    }
    if let Some(boot) = &assets.boot {
        scripts.enqueue(&boot.prerequisites());
    }
    for handle in &assets.late_scripts {
        scripts.enqueue(handle);
    }
    let head_scripts = scripts.print_head_scripts(true);
    // What the page enqueues after the head is printed comes with the footer.
    for handle in &assets.post_header_styles {
        styles.enqueue(handle);
    }
    let footer_styles = styles.print_admin_styles(true);
    for handle in &assets.post_header_scripts {
        scripts.enqueue(handle);
    }
    let footer_scripts = scripts.print_footer_scripts(true);

    let mut classes = vec!["wp-admin".to_owned(), "wp-core-ui".to_owned(), "no-js".to_owned()];
    if screen.fullscreen {
        classes.push("is-fullscreen-mode".to_owned());
    }
    classes.extend(screen.early_classes.iter().cloned());
    classes.push(screen.hook_suffix.clone().unwrap_or_else(|| format!("{}-php", page)));
    classes.push("auto-fold".to_owned());
    classes.push("admin-bar".to_owned());
    if !screen.typenow().is_empty() {
        classes.push(format!("post-type-{}", rustpress_core::formatting::sanitize_html_class(&screen.typenow())));
    }
    if matches!(screen.file.as_str(), "edit-tags.php" | "term.php") && !screen.id.is_empty() {
        classes.push(format!("taxonomy-{}", rustpress_core::formatting::sanitize_html_class(screen.id.trim_start_matches("edit-"))));
    }
    let mut parts = version.split('.');
    let (major, minor) = (parts.next().unwrap_or("0"), parts.next().unwrap_or("0"));
    classes.push(format!("branch-{major}-{minor}"));
    classes.push(format!("version-{}", version.replace('.', "-")));
    classes.push("admin-color-modern".to_owned());
    if rtl {
        classes.push("rtl".to_owned());
    }
    classes.push(format!("locale-{}", rustpress_core::formatting::sanitize_html_class(&site.locale().replace('_', "-").to_lowercase())));
    classes.push("no-customize-support".to_owned());
    classes.push("svg".to_owned());
    classes.extend(screen.body_classes.iter().cloned());
    classes.push(format!("wp-theme-{}", rustpress_core::formatting::sanitize_html_class(&env.stylesheet)));

    let locale_data = site.locale_data();
    let thousands = (locale_data.thousands_sep.clone(), locale_data.decimal_point.clone());
    let ajax = esc_url(&site.site_url("wp-admin/admin-ajax.php"));
    let ajax_path = ajax.strip_prefix(&env.loader.site_url).unwrap_or(&ajax).to_owned();
    // `wp_admin_canonical_url()`: the address of the request without the arguments of a one-time notice.
    // `set_url_scheme( 'http://' . $_SERVER['HTTP_HOST'] . $_SERVER['REQUEST_URI'] )`.
    let origin = match env.host.as_deref().filter(|h| !h.is_empty()) {
        Some(host) => format!("{}://{host}", if env.secure { "https" } else { "http" }),
        None => match env.loader.site_url.split_once("://") {
            Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or_default()),
            None => env.loader.site_url.clone(),
        },
    };
    let mut canonical = if env.request_uri.is_empty() { site.site_url(&format!("wp-admin/{}", screen.file)) } else { format!("{origin}{}", env.request_uri) };
    for arg in REMOVABLE_QUERY_ARGS {
        canonical = rustpress_core::urls::remove_query_arg(&canonical, arg);
    }

    let pagenow = screen.screen_id();
    let mut out = String::new();
    out.push_str(&format!(
        "<!DOCTYPE html>\n<html class=\"wp-toolbar\"\n\t{}>\n<head>\n<meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\" />\n\t<title>{title}</title>\n",
        if rtl { format!("dir=\"rtl\" lang=\"{}\"", esc_attr(&site.language_tag())) } else { format!("lang=\"{}\"", esc_attr(&site.language_tag())) }
    ));
    out.push_str(&format!(
        "<script>\naddLoadEvent = function(func){{if(typeof jQuery!=='undefined')jQuery(function(){{func();}});else if(typeof wpOnload!=='function'){{wpOnload=func;}}else{{var oldonload=wpOnload;wpOnload=function(){{oldonload();func();}}}}}};\nvar ajaxurl = '{ajax_path}',\n\tpagenow = '{pagenow}',\n\ttypenow = '{}',\n\tadminpage = '{}',\n\tthousandsSeparator = '{}',\n\tdecimalPoint = '{}',\n\tisRtl = {};\n</script>\n",
        screen.typenow(),
        screen.hook_suffix.clone().unwrap_or_else(|| page.to_owned() + "-php"),
        thousands.0,
        thousands.1,
        i32::from(rtl)
    ));
    out.push_str(&admin_styles);
    out.push_str(&colors);
    match env.head_styles.split_once("<style id=\"wp-emoji-styles-inline-css\">") {
        Some((before, after)) if !assets.plugin_styles.is_empty() => out.push_str(&format!("{before}{}<style id=\"wp-emoji-styles-inline-css\">{after}", assets.plugin_styles)),
        _ => out.push_str(&env.head_styles),
    }
    out.push_str(&assets.head_early);
    out.push_str(&head_scripts);
    out.push_str("<script>var _wpColorScheme = {\"icons\":{\"base\":\"#f3f1f1\",\"focus\":\"#fff\",\"current\":\"#fff\"}};</script>\n");
    out.push_str(&format!(
        "\t<link id=\"wp-admin-canonical\" rel=\"canonical\" href=\"{}\" />\n\t<script>\n\t\tif ( window.history.replaceState ) {{\n\t\t\twindow.history.replaceState( null, null, document.getElementById( 'wp-admin-canonical' ).href + window.location.hash );\n\t\t}}\n\t</script>\n\t<meta name=\"viewport\" content=\"width=device-width,initial-scale=1.0\">",
        esc_url(&canonical)
    ));
    out.push_str(&assets.head);
    // The scripts of `admin_head` end with the indentation of their closing `<?php`.
    out.push_str(if assets.head.is_empty() { "</head>\n" } else { "\t</head>\n" });
    out.push_str(&format!("<body class=\"{}\">\n", esc_attr(&classes.join(" "))));
    out.push_str("<script>\n\tdocument.body.className = document.body.className.replace('no-js','js');\n</script>\n\n");
    // Only who may customize the site gets the script (`wp_admin_bar_customize_menu()`).
    if env.user.can("edit_theme_options") {
        out.push_str("<script>\n\t\t(function() {\n\t\t\tvar request, b = document.body, c = 'className', cs = 'customize-support', rcs = new RegExp('(^|\\\\s+)(no-)?'+cs+'(\\\\s+|$)');\n\n\t\t\t\trequest = true;\n\t\n\t\t\tb[c] = b[c].replace( rcs, ' ' );\n\t\t\t// The customizer requires postMessage and CORS (if the site is cross domain).\n\t\t\tb[c] += ( window.postMessage && request ? ' ' : ' no-' ) + cs;\n\t\t}());\n\t\n//# sourceURL=wp_customize_support_script\n</script>\n");
    }
    out.push('\n');
    out.push_str("<div id=\"wpwrap\">\n\n");
    out.push_str(&format!(
        "<div id=\"adminmenumain\" role=\"navigation\" aria-label=\"{}\">\n<a href=\"#wpbody-content\" class=\"screen-reader-shortcut\">{}</a>\n<a href=\"#wp-toolbar\" class=\"screen-reader-shortcut\">{}</a>\n<div id=\"adminmenuback\"></div>\n<div id=\"adminmenuwrap\">\n<ul id=\"adminmenu\">\n\n",
        esc_attr(&tr("Main menu")),
        tr("Skip to main content"),
        tr("Skip to toolbar")
    ));
    out.push_str(&env.menu.render(site, &screen.file, &parent_file, submenu_file.as_deref(), &screen.post_type));
    out.push_str("</ul>\n</div>\n</div>\n");
    out.push_str("<div id=\"wpcontent\">\n");
    let mut bar = env.bar.clone();
    bar.view_item = screen.view_item.clone();
    bar.view_archive = screen.view_archive;
    out.push_str(&adminbar::render_admin(site, env.user, &bar));
    out.push_str("\n<div id=\"wpbody\" role=\"main\">\n\n<div id=\"wpbody-content\">\n");
    let meta = match (&screen.help, &screen.options) {
        (help, Some(options)) => {
            let nonce = env.loader.nonce.as_ref().map(|n| n("screen-options-nonce")).unwrap_or_default();
            let tr_x = |text: &str, context: &str| env.site.trx(text, context);
            let mut meta = screen_meta_with(help.as_ref(), Some(options), &tr, &tr_x, &nonce);
            if screen.content_inline && meta.ends_with("\t\t\n") {
                meta.pop();
            }
            meta
        }
        (Some(help), None) => {
            let mut meta = screen_meta(help, &tr("Help"));
            if screen.content_inline && meta.ends_with("\t\t\n") {
                meta.pop();
            }
            // A page that starts at the left edge needs none of the indentation the frame leaves.
            if screen.flush_content && meta.ends_with("</div>
		") {
                meta.truncate(meta.len() - 2);
            }
            meta
        }
        (None, None) => "\t\t".to_owned(),
    };
    // The notices end the header; the line break before the page's markup is the page's own.
    match meta.strip_suffix('\n') {
        Some(head) => out.push_str(&format!("{head}{}\n", assets.notices)),
        None => out.push_str(&format!("{meta}{}", assets.notices)),
    }
    out.push_str(content);
    out.push_str("\n<div class=\"clear\"></div></div><!-- wpbody-content -->\n<div class=\"clear\"></div></div><!-- wpbody -->\n<div class=\"clear\"></div></div><!-- wpcontent -->\n\n");
    out.push_str(&format!(
        "<div id=\"wpfooter\" role=\"contentinfo\">\n\t\t<p id=\"footer-left\" class=\"alignleft\">\n\t\t{}\t</p>\n\t<p id=\"footer-upgrade\" class=\"alignright\">\n\t\t{}\t</p>\n\t<div class=\"clear\"></div>\n</div>\n",
        assets.footer_text.clone().unwrap_or_else(|| format!("<span id=\"footer-thankyou\">{}</span>", rustpress_core::i18n::sprintf(&tr("Thank you for creating with <a href=\"%s\">WordPress</a>."), &["https://wordpress.org/"]))),
        rustpress_core::i18n::sprintf(&tr("Version %s"), &[&version])
    ));
    out.push_str(&assets.pre_footer);
    if !assets.no_auth_check {
        // `wp_auth_check_html()`: the sign-in form is loaded in the dialog only from the domain of the request.
        let login_url = site.site_url("wp-login.php");
        let same_domain = env.host.as_deref().filter(|h| !h.is_empty()).is_none_or(|host| login_url.starts_with(&format!("{}://{host}", if env.secure { "https" } else { "http" })));
        let src = esc_url(&if assets.empty_locale { format!("{login_url}?interim-login=1&wp_lang") } else { format!("{login_url}?interim-login=1&wp_lang={}", site.locale()) });
        let (wrap_class, form) = if same_domain { ("hidden", format!("\t\t<div id=\"wp-auth-check-form\" class=\"loading\" data-src=\"{src}\"></div>\n\t\t")) } else { ("hidden fallback", String::new()) };
        out.push_str(&format!(
            "\t<div id=\"wp-auth-check-wrap\" class=\"{wrap_class}\">\n\t<div id=\"wp-auth-check-bg\"></div>\n\t<div id=\"wp-auth-check\">\n\t<button type=\"button\" class=\"wp-auth-check-close button-link\"><span class=\"screen-reader-text\">\n\t\t{close}\t</span></button>\n\t{form}\t<div class=\"wp-auth-fallback\">\n\t\t<p><b class=\"wp-auth-fallback-expired\" tabindex=\"0\">{expired}</b></p>\n\t\t<p><a href=\"{login}\" target=\"_blank\">{again}</a>\n\t\t{note}</p>\n\t</div>\n\t</div>\n\t</div>\n\t",
            close = tr("Close dialog"),
            expired = tr("Session expired"),
            login = esc_url(&site.site_url("wp-login.php")),
            again = tr("Please log in again."),
            note = tr("The login page will open in a new tab. After logging in you can close it and return to this page.")
        ));
    }
    let modules = script_modules(env, assets.editor_modules, assets.route_first, assets.boot.as_ref());
    // `print_a11y_script_module_html()`: the live regions, when `@wordpress/a11y` is in the import map.
    let a11y = modules.contains("\"@wordpress/a11y\":");
    out.push_str(&modules);
    out.push_str(&footer_styles);
    out.push_str(&footer_scripts);
    out.push_str(&assets.footer_early);
    out.push_str(&emoji_scripts(env, assets.no_emoji));
    if a11y {
        out.push_str(&format!(
            "<div style=\"position:absolute;margin:-1px;padding:0;height:1px;width:1px;overflow:hidden;clip-path:inset(50%);border:0;word-wrap:normal !important;\"><p id=\"a11y-speak-intro-text\" class=\"a11y-speak-intro-text\" hidden>{}</p><div id=\"a11y-speak-assertive\" class=\"a11y-speak-region\" aria-live=\"assertive\" aria-relevant=\"additions text\" aria-atomic=\"true\"></div><div id=\"a11y-speak-polite\" class=\"a11y-speak-region\" aria-live=\"polite\" aria-relevant=\"additions text\" aria-atomic=\"true\"></div></div>",
            esc_html(&tr("Notifications"))
        ));
    }
    out.push_str(&assets.footer);
    out.push_str("\n<div class=\"clear\"></div></div><!-- wpwrap -->\n<script>if(typeof wpOnload==='function')wpOnload();</script>\n</body>\n</html>\n");
    out
}
