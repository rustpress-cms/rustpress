//! ACF on the screens of its internal post types (`ACF_Admin::current_screen()`: the navigation of
//! `views/global/navigation.php` and `header.php`, the help tabs and the footer texts) and the lists of field groups,
//! post types and taxonomies (`ACF_Admin_Internal_Post_Type_List` with `ACF_Admin_Field_Groups`,
//! `ACF_Admin_Post_Types` and `ACF_Admin_Taxonomies`), with the email opt-in banner of the free version
//! (`ACF_Admin_Email_Opt_In_Banner`).
//!
//! Differs from the plugin: there is no local JSON (no "Sync available" view, no "Local JSON" column), the toolbar
//! has no tabs of other plugins (`acf/admin/toolbar`), and the `post_template` rule of the "Location" column counts
//! every template but "default" as one of pages, the only type RustPress knows templates of.

use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::native::{AdminContext, AdminHelp, Assets, HelpTab, ListStatus};
use rustpress_db::Post;
use rustpress_phpser::Value;

use super::fields::{FieldGroup, Rule};
use super::php::strval;

const PLUGIN_DIR: &str = "wp-content/plugins/advanced-custom-fields";
const PARENT: &str = "edit.php?post_type=acf-field-group";

/// `acf_get_internal_post_types()`.
const INTERNAL: [&str; 4] = ["acf-field-group", "acf-post-type", "acf-taxonomy", "acf-ui-options-page"];

fn tr(site: &Site, text: &str) -> String {
    site.trd("acf", text)
}

fn esc_tr(site: &Site, text: &str) -> String {
    esc_html(&tr(site, text))
}

/// `acf_get_url( $path )`.
fn acf_url(site: &Site, path: &str) -> String {
    site.site_url(&format!("{PLUGIN_DIR}/{path}"))
}

/// `acf_add_url_utm_tags( $url, $campaign, $content, $anchor, $source, $medium )` of the free version.
pub(super) fn utm(url: &str, campaign: &str, content: &str, anchor: &str, source: &str, medium: &str) -> String {
    let source = if source.is_empty() { "ACF Free" } else { source };
    let medium = if medium.is_empty() { "insideplugin" } else { medium };
    let query: Vec<String> = [("utm_source", source), ("utm_medium", medium), ("utm_campaign", campaign), ("utm_content", content)].iter().map(|(k, v)| format!("{k}={}", rustpress_core::canonical::php_urlencode(v))).collect();
    let separator = if url.contains('?') { '&' } else { '?' };
    let anchor = if anchor.is_empty() { String::new() } else { format!("#{anchor}") };
    esc_url(&format!("{url}{separator}{}{anchor}", query.join("&")))
}

/// `acf_slugify( $str )`.
fn slugify(site: &Site, text: &str) -> String {
    rustpress_core::slug::remove_accents(text, &site.locale())
        .to_ascii_lowercase()
        .chars()
        .map(|c| if matches!(c, '_' | '-' | '/' | ' ') { '-' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

/// `get_current_screen()->post_type` is one of ACF's: `ACF_Admin::current_screen()` adds its frame.
pub(super) fn is_acf_screen(ctx: &AdminContext<'_>) -> bool {
    INTERNAL.contains(&ctx.typenow)
}

/// The internal post type whose list the screen is (`acf_is_screen( "edit-{$post_type}" )`).
fn list_of(ctx: &AdminContext<'_>) -> Option<&'static str> {
    let listed = ctx.screen_id.strip_prefix("edit-")?;
    ["acf-field-group", "acf-post-type", "acf-taxonomy"].into_iter().find(|t| *t == listed)
}

/// `$this->view`: the `post_status` of the request.
fn view(ctx: &AdminContext<'_>) -> String {
    rustpress_content::sanitize::text_field(ctx.get("post_status").unwrap_or_default())
}

/// `$_GET['page']` of a plugin page (`$plugin_page`).
fn plugin_page<'a>(ctx: &'a AdminContext<'a>) -> Option<&'a str> {
    if matches!(ctx.hook_suffix, "edit.php" | "post.php" | "post-new.php") { None } else { ctx.get("page") }
}

struct NavItem {
    text: String,
    url: String,
    class: String,
    active: bool,
    target: &'static str,
    aria_label: String,
    li_class: &'static str,
}

/// `acf_print_menu_section()`.
fn menu_section(site: &Site, items: &[NavItem], core: bool) -> String {
    let mut out = String::new();
    for item in items {
        let class = if item.class.is_empty() { item.text.as_str() } else { item.class.as_str() };
        let target = if item.target.is_empty() { String::new() } else { format!(" target=\"{}\"", esc_attr(item.target)) };
        let aria = if item.aria_label.is_empty() { String::new() } else { format!(" aria-label=\"{}\"", esc_attr(&item.aria_label)) };
        let html = format!(
            "<a class=\"acf-tab{} {}\" href=\"{}\"{target}{aria}><i class=\"acf-icon\"></i>{}</a>",
            if item.active { " is-active" } else { "" },
            format_args!("acf-header-tab-{}", esc_attr(&slugify(site, class))),
            esc_url(&item.url),
            rustpress_core::kses::kses_post(&item.text)
        );
        if core {
            out.push_str(&html);
        } else if item.li_class.is_empty() {
            out.push_str(&format!("<li>{html}</li>"));
        } else {
            out.push_str(&format!("<li class=\"{}\">{html}</li>", item.li_class));
        }
    }
    out
}

/// `views/global/navigation.php`, with `views/global/header.php` on the screens that are not an internal post type's
/// own (`$screen->id`).
fn navigation(ctx: &AdminContext<'_>, page_title: Option<&str>) -> String {
    let site = ctx.site;
    let submenu_file = format!("edit.php?post_type={}", ctx.typenow);
    let plugin_page = plugin_page(ctx);
    let (mut core_tabs, mut more): (Vec<NavItem>, Vec<NavItem>) = (Vec::new(), Vec::new());
    for page in super::admin::admin_pages(site).into_iter().filter(|p| p.parent == PARENT) {
        if !(ctx.can)(page.capability) {
            continue;
        }
        let (url, class) = if page.slug.contains(".php") { (page.slug.to_owned(), page.slug.replace("edit.php?post_type=", "")) } else { (rustpress_core::urls::add_query_arg(PARENT, "page", page.slug), page.slug.to_owned()) };
        let active = match plugin_page {
            Some(current) => current == page.slug,
            None => submenu_file == page.slug,
        };
        let item = || NavItem { text: page.menu_title.clone(), url: url.clone(), class: class.clone(), active, target: "", aria_label: String::new(), li_class: "" };
        match class.as_str() {
            "acf-field-group" => core_tabs.push(item()),
            "acf-post-type" | "acf-taxonomy" => {
                core_tabs.push(item());
                more.push(item());
            }
            "acf-ui-options-page" | "acf-tools" | "acf-settings-updates" => more.push(item()),
            _ => {}
        }
    }
    more.push(NavItem {
        text: format!("{}<span class=\"acf-requires-pro\">{}</span>", tr(site, "Options Pages"), tr(site, "PRO")),
        url: format!("{PARENT}&page=acf_options_preview"),
        class: String::new(),
        active: false,
        target: "_self",
        aria_label: String::new(),
        li_class: "",
    });
    let wpengine = NavItem {
        text: format!("<span><img class=\"acf-wp-engine-pro\" src=\"{}\" alt=\"WP Engine\" /></span><span class=\"acf-wp-engine-upsell-pill\">{}</span>", acf_url(site, "assets/images/wp-engine-horizontal-black.svg"), tr(site, "4 Months Free")),
        url: utm("https://wpengine.com/plans/?coupon=freedomtocreate", "bx_prod_referral", "acf_free_plugin_topbar_dropdown_cta", "", "acf_plugin", "referral"),
        class: String::new(),
        active: false,
        target: "_blank",
        aria_label: tr(site, "Get 4 months free on any WP Engine plan"),
        li_class: "acf-wp-engine",
    };
    let mut out = format!(
        "<div class=\"acf-admin-toolbar\">\n\t<div class=\"acf-admin-toolbar-inner\">\n\t\t<div class=\"acf-nav-wrap\">\n\t\t\t\t\t\t\t<a href=\"{}\" class=\"acf-logo\" aria-label=\"{}\">\n\t\t\t\t\t<img src=\"{}\" alt=\"{}\">\n\t\t\t\t</a>\n\t\t\t\n\t\t\t<h2>Advanced Custom Fields</h2>\n\t\t\t{}\t\t\t\t\t\t\t<div class=\"acf-more acf-header-tab-acf-more\" tabindex=\"0\">\n\t\t\t\t\t<span class=\"acf-tab acf-more-tab\"><i class=\"acf-icon acf-icon-more\"></i>{} <i class=\"acf-icon acf-icon-dropdown\"></i></span>\n\t\t\t\t\t<ul>\n\t\t\t\t\t\t{}{}\t\t\t\t\t</ul>\n\t\t\t\t</div>\n\t\t\t\t\t</div>\n",
        esc_url(&site.site_url(&format!("wp-admin/{PARENT}"))),
        esc_attr(&tr(site, "Edit ACF Field Groups")),
        esc_url(&acf_url(site, "assets/images/acf-logo.svg")),
        esc_attr(&tr(site, "Advanced Custom Fields logo")),
        menu_section(site, &core_tabs, true),
        esc_tr(site, "More"),
        menu_section(site, &more, false),
        menu_section(site, std::slice::from_ref(&wpengine), false),
    );
    out.push_str(&format!(
        "\t\t<div class=\"acf-nav-upgrade-wrap\">\n\t\t\t\t\t\t\t<a target=\"_blank\" href=\"{}\" class=\"btn-upgrade acf-admin-toolbar-upgrade-btn\">\n\t\t\t\t\t<i class=\"acf-icon acf-icon-stars\"></i>\n\t\t\t\t\t<p>{}</p>\n\t\t\t\t</a>\n\t\t\t\t\t\t\t<a href=\"{}\" target=\"_blank\" class=\"acf-nav-wpengine-logo\" aria-label=\"{}\">\t\t\t\t<img src=\"{}\" alt=\"{}\" />\n\t\t\t</a>\n\t\t</div>\n\t</div>\n</div>\n\n",
        esc_url(&utm("https://www.advancedcustomfields.com/pro/", "ACF upgrade", "header", "", "", "")),
        esc_tr(site, "Unlock Extra Features with ACF PRO"),
        utm("https://wpengine.com/", "bx_prod_referral", "acf_free_plugin_topbar_logo", "", "acf_plugin", "referral"),
        esc_attr(&tr(site, "WP Engine")),
        esc_url(&acf_url(site, "assets/images/wp-engine-horizontal-white.svg")),
        esc_tr(site, "WP Engine logo"),
    ));
    if !INTERNAL.contains(&ctx.screen_id) {
        out.push_str(&header(ctx, page_title));
    }
    out
}

/// `views/global/header.php`: the title of the screen and its "Add New" button.
fn header(ctx: &AdminContext<'_>, page_title: Option<&str>) -> String {
    let site = ctx.site;
    let options_preview = ctx.get("page") == Some("acf_options_preview");
    // `$post_type_object` is a global of `edit.php`, `post.php` and `post-new.php` only.
    let post_type_object = !ctx.typenow.is_empty() && matches!(ctx.hook_suffix, "edit.php" | "post.php" | "post-new.php");
    let labels = || site.types.admin_post_types().into_iter().find(|t| t.name == ctx.typenow);
    let title = match page_title {
        Some(title) => title.to_owned(),
        None if post_type_object => labels().and_then(|t| t.label("name")).unwrap_or_default(),
        None => String::new(),
    };
    if title.is_empty() {
        return String::new();
    }
    let mut out = format!("<div class=\"acf-headerbar\">\n\n\t<h1 class=\"acf-page-title\">\n\t{}\t", esc_html(&title));
    if options_preview {
        out.push_str(&format!("\t\t\t<div class=\"acf-pro-label\"><img src=\"{}\" alt=\"{}\"></div>\n\t\t", esc_url(&acf_url(site, "assets/images/pro-chip.svg")), esc_attr(&tr(site, "ACF PRO logo"))));
    }
    out.push_str("\t</h1>\n\t");
    if options_preview {
        out.push_str(&format!("\t\t<a href=\"#\" class=\"acf-btn acf-btn-sm disabled\">\n\t\t\t\t<i class=\"acf-icon acf-icon-plus\"></i>\n\t\t\t\t{}\t\t\t</a>\n\t", esc_tr(site, "Add Options Page")));
    }
    out.push('\t');
    if post_type_object && (ctx.can)("manage_options") {
        let add_new = labels().and_then(|t| t.label("add_new")).unwrap_or_default();
        out.push_str(&format!(
            "<a href=\"{}\" class=\"acf-btn acf-btn-sm\"><i class=\"acf-icon acf-icon-plus\"></i>{}</a>",
            esc_url(&site.site_url(&format!("wp-admin/post-new.php?post_type={}", ctx.typenow))),
            esc_html(&add_new)
        ));
    }
    out.push_str("\n</div>\n");
    out
}

/// `ACF_Admin::ensure_menu_selection()`: every screen of ACF's types belongs to its menu.
pub fn parent_file(parent_file: String) -> String {
    if parent_file.starts_with("edit.php?post_type=acf-") { PARENT.to_owned() } else { parent_file }
}

/// `ACF_Admin::ensure_submenu_selection()`: "Add New" highlights the list.
pub fn submenu_file(submenu_file: String) -> String {
    if submenu_file.starts_with("post-new.php?post_type=acf-") { submenu_file.replace("post-new", "edit") } else { submenu_file }
}

/// `ACF_Admin::in_admin_header()`.
pub fn in_admin_header(ctx: &AdminContext<'_>) -> String {
    if !is_acf_screen(ctx) {
        return String::new();
    }
    let page_title = match plugin_page(ctx) {
        Some("acf-tools") => Some(tr(ctx.site, "Tools")),
        Some("acf-settings-updates") => Some(tr(ctx.site, "Updates")),
        Some("acf_options_preview") => Some(tr(ctx.site, "Options Pages")),
        _ => None,
    };
    navigation(ctx, page_title.as_deref())
}

/// `ACF_Admin::setup_help_tab()`.
pub fn help(ctx: &AdminContext<'_>) -> Option<AdminHelp> {
    if !is_acf_screen(ctx) {
        return None;
    }
    let site = ctx.site;
    let t = |text: &str| tr(site, text);
    let sprintf = |template: &str, url: &str| rustpress_core::i18n::sprintf(&t(template), &[url]);
    let overview = format!(
        "<p><strong>{}</strong></p><p>{}</p><p>{}</p><p>{}</p>",
        t("Overview"),
        t("The Advanced Custom Fields plugin provides a visual form builder to customize WordPress edit screens with extra fields, and an intuitive API to display custom field values in any theme template file."),
        sprintf("Before creating your first Field Group, we recommend first reading our <a href=\"%s\" target=\"_blank\">Getting started</a> guide to familiarize yourself with the plugin's philosophy and best practises.", &utm("https://www.advancedcustomfields.com/resources/getting-started-with-acf/", "docs", "help-tab", "", "", "")),
        t("Please use the Help & Support tab to get in touch should you find yourself requiring assistance."),
    );
    let support = format!(
        "<p><strong>{}</strong></p><p>{}</p><ul><li>{}</li><li>{}</li><li>{}</li></ul>",
        t("Help & Support"),
        t("We are fanatical about support, and want you to get the best out of your website with ACF. If you run into any difficulties, there are several places you can find help:"),
        sprintf("<a href=\"%s\" target=\"_blank\">Documentation</a>. Our extensive documentation contains references and guides for most situations you may encounter.", &utm("https://www.advancedcustomfields.com/resources/", "docs", "help-tab", "", "", "")),
        sprintf("<a href=\"%s\" target=\"_blank\">Discussions</a>. We have an active and friendly community on our Community Forums who may be able to help you figure out the 'how-tos' of the ACF world.", &utm("https://support.advancedcustomfields.com/", "docs", "help-tab", "", "", "")),
        sprintf("<a href=\"%s\" target=\"_blank\">Help Desk</a>. The support professionals on our Help Desk will assist with your more in depth, technical challenges.", &utm("https://www.advancedcustomfields.com/support/", "docs", "help-tab", "", "", "")),
    );
    let sidebar = format!(
        "<p><strong>{}</strong></p><p><span class=\"dashicons dashicons-admin-plugins\"></span> {}</p><p><span class=\"dashicons dashicons-wordpress\"></span> <a href=\"https://wordpress.org/plugins/advanced-custom-fields/\" target=\"_blank\">{}</a></p><p><span class=\"dashicons dashicons-admin-home\"></span> <a href=\"{}\" target=\"_blank\" target=\"_blank\">{}</a></p>",
        t("Information"),
        rustpress_core::i18n::sprintf(&t("Version %s"), &[super::VERSION]),
        t("View details"),
        utm("https://www.advancedcustomfields.com/", "docs", "help-tab", "", "", ""),
        t("Visit website"),
    );
    Some(AdminHelp {
        tabs: vec![HelpTab { id: "overview".into(), title: t("Overview"), content: overview }, HelpTab { id: "help".into(), title: t("Help & Support"), content: support }],
        sidebar,
    })
}

/// `ACF_Admin::admin_footer_text()`.
pub fn footer_text(ctx: &AdminContext<'_>) -> Option<String> {
    if !is_acf_screen(ctx) {
        return None;
    }
    Some(format!(
        "<a href=\"{}\" target=\"_blank\">ACF&#174;</a> {} <a href=\"{}\" target=\"_blank\">WP Engine</a>.",
        utm("https://www.advancedcustomfields.com/", "footer", "footer", "", "", ""),
        tr(ctx.site, "is developed and maintained by"),
        utm("https://wpengine.com/", "bx_prod_referral", "acf_free_plugin_footer_text", "", "acf_plugin", "referral"),
    ))
}

/// `ACF_Admin::admin_footer_version_text()`.
pub fn update_footer(ctx: &AdminContext<'_>) -> Option<String> {
    if !is_acf_screen(ctx) {
        return None;
    }
    let site = ctx.site;
    let link = |url: &str| utm(url, "footer", "footer", "", "", "");
    Some(format!(
        "<a href=\"{}\" target=\"_blank\">{}</a> &#8729; <a href=\"{}\" target=\"_blank\">{}</a> &#8729; <a href=\"{}\" target=\"_blank\">{}</a> &#8729; <a href=\"{}\" target=\"_blank\">{} {}</a>",
        link("https://www.advancedcustomfields.com/resources/"),
        tr(site, "Documentation"),
        link("https://www.advancedcustomfields.com/support/"),
        tr(site, "Support"),
        link("https://www.advancedcustomfields.com/feedback/"),
        tr(site, "Feedback"),
        link("https://www.advancedcustomfields.com/changelog/"),
        tr(site, "ACF"),
        super::VERSION
    ))
}

/// `ACF::register_post_status()`.
pub fn list_statuses() -> Vec<ListStatus> {
    vec![ListStatus { name: "acf-disabled", show_in_admin_all_list: true, show_in_admin_status_list: true }]
}

/// The `label_count` of `acf-disabled`, and of `publish` on ACF's lists (`current_screen()` relabels it).
pub fn status_label_count(ctx: &AdminContext<'_>, status: &str, n: u64) -> Option<String> {
    let label = match status {
        "acf-disabled" => "Inactive <span class=\"count\">(%s)</span>",
        "publish" if list_of(ctx).is_some() => "Active <span class=\"count\">(%s)</span>",
        _ => return None,
    };
    Some(ctx.site.i18n().ngettext("acf", label, label, n as i64))
}

/// `ACF_Admin_Internal_Post_Type_List::admin_body_class()`.
pub fn admin_body_class(ctx: &AdminContext<'_>, classes: String) -> String {
    let Some(post_type) = list_of(ctx) else { return classes };
    let own = match post_type {
        "acf-field-group" => "acf-admin-field-groups",
        "acf-post-type" => "acf-admin-post-types",
        _ => "acf-admin-taxonomies",
    };
    let mut classes = format!("{classes} acf-admin-page acf-internal-post-type {own}");
    let view = view(ctx);
    if !view.is_empty() {
        classes.push_str(&format!(" view-{}", esc_attr(&view)));
    }
    classes
}

/// `wp_localize_script( $handle, $name, $data )`: the strings with their entities decoded.
pub(super) fn localize(name: &str, data: &[(&str, String)]) -> String {
    let map: serde_json::Map<String, serde_json::Value> = data.iter().map(|(k, v)| ((*k).to_owned(), serde_json::Value::String(rustpress_core::formatting::html_entity_decode(v)))).collect();
    let json = rustpress_blocks::php_json::encode_with(&serde_json::Value::Object(map), rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E");
    format!("var {name} = {json};")
}

/// `ACF_Admin_Email_Opt_In_Banner::should_show()`.
fn shows_banner(ctx: &AdminContext<'_>) -> bool {
    (ctx.can)("manage_options")
        && (list_of(ctx).is_some() || ctx.get("page") == Some("acf_options_preview"))
        && !matches!(ctx.site.options.get("acf_email_opt_in_banner_state"), Some("dismissed" | "submitted"))
}

/// `admin_enqueue_scripts` of the lists (`acf_enqueue_script( 'acf' )` with its texts) and of the banner.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    if list_of(ctx).is_none() {
        return;
    }
    let site = ctx.site;
    let acf = assets.script("acf", acf_url(site, "assets/build/js/acf.min.js"), &["jquery"], super::VERSION, false);
    let texts = ["Review local JSON changes", "Loading diff", "Sync changes", "Please activate your ACF PRO license to edit this options page.", "Please activate your ACF PRO license to edit field groups assigned to an ACF Block."];
    let translated: Vec<(&str, String)> = texts.iter().map(|t| (*t, esc_tr(site, t))).filter(|(k, v)| *k != v.as_str()).collect();
    if !translated.is_empty() {
        acf.extra = Some(localize("acfL10n", &translated));
    }
    if shows_banner(ctx) {
        let banner = assets.script("acf-email-opt-in-banner", acf_url(site, "assets/build/js/acf-email-opt-in-banner.min.js"), &["jquery", "acf"], super::VERSION, true);
        banner.extra = Some(localize(
            "acf_email_opt_in_banner",
            &[("empty_email", tr(site, "Email address is required.")), ("invalid_email", tr(site, "Please enter a valid email address.")), ("generic_error", tr(site, "Something went wrong on our end. Please try again."))],
        ));
    }
}

/// `admin_footer` of the lists: the script of `admin_footer()` (priority 1), `views/acf-field-group/pro-features.php`
/// (`include_pro_features()`) and the banner (`views/email-opt-in-banner.php`).
pub fn admin_footer(ctx: &AdminContext<'_>) -> String {
    if list_of(ctx).is_none() {
        return String::new();
    }
    let mut out = include_str!("list_footer.html").to_owned();
    out.push_str(&pro_features(ctx.site));
    if shows_banner(ctx) {
        out.push_str(&banner(ctx.site));
    }
    out
}

/// `views/acf-field-group/pro-features.php` of the free version.
fn pro_features(site: &Site) -> String {
    let title = esc_tr(site, "Unlock Advanced Features and Build Even More with ACF PRO");
    let chip = format!("<div class=\"acf-pro-label\"><img src=\"{}\" alt=\"{}\"></div>", esc_url(&acf_url(site, "assets/images/pro-chip.svg")), esc_attr(&tr(site, "ACF PRO logo")));
    let feature = |icon: &str, label: &str| format!("\t\t\t<div class=\"acf-field-group-pro-feature\">\n\t\t\t\t<i class=\"field-type-icon {icon}\"></i>\n\t\t\t\t<span class=\"field-type-label\">{}</span>\n\t\t\t</div>\n", esc_tr(site, label));
    let logo = format!(
        "<a href=\"{}\" target=\"_blank\"><img class=\"acf-field-group-pro-features-wpengine-logo\" src=\"{}\" alt=\"WP Engine\" /></a>",
        utm("https://wpengine.com/", "bx_prod_referral", "acf_free_plugin_cta_panel_logo", "", "acf_plugin", "referral"),
        acf_url(site, "assets/images/wp-engine-horizontal-white.svg")
    );
    let made_for = rustpress_core::kses::kses_post(&rustpress_core::i18n::sprintf(&tr(site, "Built for those that build with WordPress, by the team at %s"), &[&logo]));
    format!(
        "<div id=\"tmpl-acf-field-group-pro-features\">\n\t<div class=\"acf-field-group-pro-features-wrapper\">\n\t\t<h1 class=\"acf-field-group-pro-features-title-sm\">{title} {chip}</h1>\n\t\t<div class=\"acf-field-group-pro-features-content\">\n\t\t\t<h1 class=\"acf-field-group-pro-features-title\">{title} {chip}</h1>\n\t\t\t<p class=\"acf-field-group-pro-features-desc\">{}</p>\n\t\t\t<div class=\"acf-field-group-pro-features-actions\">\n\t\t\t\t<a target=\"_blank\" href=\"{}\" class=\"acf-btn acf-btn-muted acf-pro-features-learn-more\">\t\t\t\t\t{} <i class=\"acf-icon acf-icon-arrow-up-right\"></i>\n\t\t\t\t</a>\n\t\t\t\t<a target=\"_blank\" href=\"{}\" class=\"acf-btn acf-pro-features-upgrade\">\t\t\t\t\t{} <i class=\"acf-icon acf-icon-arrow-up-right\"></i>\n\t\t\t\t</a>\n\t\t\t</div>\n\t\t</div>\n\n\t\t<div class=\"acf-field-group-pro-features-grid\">\n{}\t\t</div>\n\t</div>\n\t<div class=\"acf-field-group-pro-features-footer-wrap\">\n\t\t<div class=\"acf-field-group-pro-features-footer\">\n\t\t\t<div class=\"acf-for-the-builders\">\n\t\t\t\t{made_for}\t\t\t</div>\n\t\t\t<div class=\"acf-more-tools-from-wpengine\">\n\t\t\t\t<a href=\"{}\" target=\"_blank\">{} <i class=\"acf-icon acf-icon-arrow-up-right\"></i></a>\t\t\t</div>\n\t\t</div>\n\t</div>\n</div>\n\n<script type=\"text/javascript\">\n\t( function ( $, undefined ) {{\n\t\t$( document ).ready( function() {{\n\t\t\tif ( 'field_group' === acf.get( 'screen' ) ) {{\n\t\t\t\t$( '#acf-field-group-options' ).after( $( '#tmpl-acf-field-group-pro-features' ).css( 'display', 'block' ) );\n\t\t\t}} else {{\n\t\t\t\t$( '#tmpl-acf-field-group-pro-features' ).appendTo( '#wpbody-content .wrap' ).css( 'display', 'block' );\n\t\t\t}}\n\t\t}} );\n\t}} )( jQuery );\n</script>\n",
        esc_tr(site, "Speed up your workflow and develop better websites with features like ACF Blocks and Options Pages, and sophisticated field types like Repeater, Flexible Content, Clone, and Gallery."),
        utm("https://www.advancedcustomfields.com/pro/", "ACF upgrade", "metabox", "", "", ""),
        esc_tr(site, "Learn More"),
        utm("https://www.advancedcustomfields.com/pro/", "ACF upgrade", "metabox_pricing", "pricing-table", "", ""),
        esc_tr(site, "View Pricing & Upgrade"),
        [
            ("field-type-icon-flexible-content", "Flexible Content Field"),
            ("field-type-icon-repeater", "Repeater Field"),
            ("field-type-icon-clone", "Clone Field"),
            ("pro-feature-blocks", "ACF Blocks"),
            ("pro-feature-options-pages", "Options Pages"),
            ("field-type-icon-gallery", "Gallery Field"),
        ]
        .iter()
        .map(|(icon, label)| feature(icon, label))
        .collect::<String>(),
        utm("https://wpengine.com/developer/", "bx_prod_referral", "acf_free_plugin_cta_panel_logo", "", "acf_plugin", "referral"),
        esc_tr(site, "More Tools from WP Engine"),
    )
}

/// `views/email-opt-in-banner.php`.
fn banner(site: &Site) -> String {
    let privacy = format!("<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>", esc_url("https://wpengine.com/legal/privacy/"), esc_tr(site, "Privacy Policy"));
    let fine_print = rustpress_core::kses::kses_post(&rustpress_core::i18n::sprintf(&tr(site, "We respect your inbox. Unsubscribe anytime. By signing up, you agree to our %s."), &[&privacy]));
    format!(
        "<div id=\"acf-email-opt-in-banner-wrap\" class=\"acf-email-opt-in-banner-wrap\" style=\"display: none;\">\n\t<section\n\t\tid=\"acf-email-opt-in-banner\"\n\t\tclass=\"acf-email-opt-in-banner\"\n\t\taria-labelledby=\"acf-email-opt-in-banner-heading\"\n\t>\n\t\t<div class=\"acf-email-opt-in-banner__content\">\n\t\t\t<h2 id=\"acf-email-opt-in-banner-heading\" class=\"acf-email-opt-in-banner__heading\">\n\t\t\t\t{}\t\t\t</h2>\n\t\t\t<p class=\"acf-email-opt-in-banner__body\">\n\t\t\t\t{}\t\t\t</p>\n\t\t\t<hr class=\"acf-email-opt-in-banner__divider\" />\n\t\t\t<p class=\"acf-email-opt-in-banner__fine-print\">\n\t\t\t\t{fine_print}\t\t\t</p>\n\t\t</div>\n\n\t\t<div class=\"acf-email-opt-in-banner__states\">\n\t\t\t<div class=\"acf-email-opt-in-banner__form\">\n\t\t\t\t<div class=\"acf-email-opt-in-banner__form-row\">\n\t\t\t\t\t<label for=\"acf-email-opt-in-banner-email\" class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</label>\n\t\t\t\t\t<div class=\"acf-email-opt-in-banner__input-wrap\">\n\t\t\t\t\t\t<span class=\"acf-email-opt-in-banner__input-icon\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t<input\n\t\t\t\t\t\t\ttype=\"email\"\n\t\t\t\t\t\t\tid=\"acf-email-opt-in-banner-email\"\n\t\t\t\t\t\t\tclass=\"acf-email-opt-in-banner__input\"\n\t\t\t\t\t\t\tname=\"email\"\n\t\t\t\t\t\t\tplaceholder=\"{}\"\n\t\t\t\t\t\t\tautocomplete=\"email\"\n\t\t\t\t\t\t\tmaxlength=\"254\"\n\t\t\t\t\t\t\taria-describedby=\"acf-email-opt-in-banner-error\"\n\t\t\t\t\t\t/>\n\t\t\t\t\t</div>\n\t\t\t\t\t<button type=\"button\" class=\"acf-btn acf-email-opt-in-banner__submit\">\n\t\t\t\t\t\t{}\t\t\t\t\t</button>\n\t\t\t\t</div>\n\t\t\t\t<p\n\t\t\t\t\tid=\"acf-email-opt-in-banner-error\"\n\t\t\t\t\tclass=\"acf-email-opt-in-banner__error\"\n\t\t\t\t\trole=\"alert\"\n\t\t\t\t\thidden\n\t\t\t\t>\n\t\t\t\t\t<span\n\t\t\t\t\t\tclass=\"acf-email-opt-in-banner__error-icon\"\n\t\t\t\t\t\taria-hidden=\"true\"\n\t\t\t\t\t></span>\n\t\t\t\t\t<span class=\"acf-email-opt-in-banner__error-text\"></span>\n\t\t\t\t</p>\n\t\t\t</div>\n\n\t\t\t<div\n\t\t\t\tclass=\"acf-email-opt-in-banner__success\"\n\t\t\t\trole=\"status\"\n\t\t\t\taria-live=\"polite\"\n\t\t\t\thidden\n\t\t\t>\n\t\t\t\t<span class=\"acf-email-opt-in-banner__success-icon\" aria-hidden=\"true\"></span>\n\t\t\t\t<span class=\"acf-email-opt-in-banner__success-text\">\n\t\t\t\t\t{}\t\t\t\t</span>\n\t\t\t</div>\n\t\t</div>\n\n\t\t<button\n\t\t\ttype=\"button\"\n\t\t\tclass=\"acf-email-opt-in-banner__dismiss\"\n\t\t\taria-label=\"{}\"\n\t\t>\n\t\t\t<span class=\"acf-email-opt-in-banner__dismiss-icon\" aria-hidden=\"true\"></span>\n\t\t</button>\n\t</section>\n</div>\n",
        esc_tr(site, "Join the ACF community."),
        esc_tr(site, "Get critical ACF security updates, releases, news, and workflow improvements."),
        esc_tr(site, "Email address"),
        esc_attr(&tr(site, "you@example.com")),
        esc_tr(site, "Join the list"),
        esc_tr(site, "You're on the list! Keep an eye out for our next update."),
        esc_attr(&tr(site, "Dismiss email opt-in banner")),
    )
}

/// `acf.data` of `ACF_Assets::print_footer_scripts()` on the lists, which enqueue `acf`.
pub fn footer_scripts(ctx: &AdminContext<'_>) -> Option<String> {
    list_of(ctx)?;
    Some(super::form::footer(ctx.site, &super::form::FormData { bare: true, ..Default::default() }, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, super::form::Uploader::No))
}

/// `admin_table_columns()` of the three lists.
pub fn posts_columns(ctx: &AdminContext<'_>, post_type: &str, columns: Vec<(String, String)>) -> Vec<(String, String)> {
    if list_of(ctx) != Some(post_type) {
        return columns;
    }
    let site = ctx.site;
    let get = |key: &str| columns.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap_or_default();
    let mut out = vec![("cb".to_owned(), get("cb")), ("title".to_owned(), get("title")), ("acf-description".to_owned(), tr(site, "Description")), ("acf-key".to_owned(), tr(site, "Key"))];
    let own: [(&str, &str); 3] = match post_type {
        "acf-field-group" => [("acf-location", "Location"), ("acf-count", "Fields"), ("", "")],
        "acf-post-type" => [("acf-taxonomies", "Taxonomies"), ("acf-field-groups", "Field Groups"), ("acf-count", "Posts")],
        _ => [("acf-post-types", "Post Types"), ("acf-field-groups", "Field Groups"), ("acf-count", "Terms")],
    };
    out.extend(own.iter().filter(|(k, _)| !k.is_empty()).map(|(k, label)| ((*k).to_owned(), tr(site, label))));
    out
}

/// `get_not_found_html()`: `views/{$post_type}/list-empty.php`, when the request searches nothing.
pub fn list_not_found(ctx: &AdminContext<'_>, post_type: &str) -> Option<String> {
    if list_of(ctx) != Some(post_type) || ctx.get("s").is_some_and(|s| !s.is_empty()) {
        return None;
    }
    let site = ctx.site;
    match post_type {
        "acf-field-group" => Some(format!(
            "<script>document.body.classList.add('acf-no-field-groups');</script>\n<div class=\"acf-no-field-groups-wrapper\">\n\t<div class=\"acf-no-field-groups-inner\">\n\t\t<img src=\"{}\" />\n\t\t<h2>{}</h2>\n\t\t<p>\n\t\t{}\t\t</p>\n\t\t<a href=\"{}\" class=\"acf-btn\"><i class=\"acf-icon acf-icon-plus\"></i> {}</a>\n\t\t<p class=\"acf-small\">\n\t\t\t{}\t\t</p>\n\t</div>\n</div>\n",
            esc_url(&acf_url(site, "assets/images/empty-group.svg")),
            esc_tr(site, "Add Your First Field Group"),
            rustpress_core::kses::kses_post(&rustpress_core::i18n::sprintf(&tr(site, "ACF uses <a href=\"%s\" target=\"_blank\">field groups</a> to group custom fields together, and then attach those fields to edit screens."), &[&utm("https://www.advancedcustomfields.com/resources/creating-a-field-group/", "docs", "no-field-groups", "", "", "")])),
            esc_url(&site.site_url("wp-admin/post-new.php?post_type=acf-field-group")),
            esc_tr(site, "Add Field Group"),
            rustpress_core::kses::kses_post(&rustpress_core::i18n::sprintf(&tr(site, "New to ACF? Take a look at our <a href=\"%s\" target=\"_blank\">getting started guide</a>."), &[&utm("https://www.advancedcustomfields.com/resources/getting-started-with-acf/", "docs", "no-field-groups", "", "", "")])),
        )),
        _ => None,
    }
}

/// `display_post_states()`: inactive items say so.
pub fn display_post_states(ctx: &AdminContext<'_>, post: &Post, mut states: Vec<(String, String)>) -> Vec<(String, String)> {
    if list_of(ctx).is_some() && post.post_status == "acf-disabled" {
        states.push(("acf-disabled".into(), format!("<span class=\"dashicons dashicons-hidden\"></span> {}", ctx.site.i18n().pgettext("acf", "post status", "Inactive"))));
    }
    states
}

/// `get_admin_url( $params )`: the list with `paged` of the request.
fn list_url(ctx: &AdminContext<'_>, post_type: &str, params: &str) -> String {
    let mut params = params.to_owned();
    if let Some(paged) = ctx.get("paged") {
        params.push_str(&format!("&paged={}", super::php::intval(Some(&super::php::string(paged)))));
    }
    ctx.site.site_url(&format!("wp-admin/edit.php?post_type={post_type}{params}"))
}

/// `page_row_actions()`: no Quick Edit, but Duplicate and Activate or Deactivate, in ACF's order.
pub fn post_row_actions(ctx: &AdminContext<'_>, post: &Post, hierarchical: bool, actions: Vec<(String, String)>) -> Vec<(String, String)> {
    let Some(post_type) = list_of(ctx) else { return actions };
    if !hierarchical || view(ctx) == "trash" {
        return actions;
    }
    let site = ctx.site;
    let mut actions: Vec<(String, String)> = actions.into_iter().filter(|(k, _)| k != "inline" && k != "inline hide-if-no-js").collect();
    let nonce = (ctx.nonce)("bulk-posts");
    let duplicate_url = match post_type {
        "acf-field-group" => list_url(ctx, post_type, &format!("&acfduplicate={}&_wpnonce={nonce}", post.id)),
        "acf-post-type" => site.site_url(&format!("wp-admin/post-new.php?post_type=acf-post-type&use_post_type={}&_wpnonce={}", post.id, (ctx.nonce)(&format!("acfduplicate-{}", post.id)))),
        _ => site.site_url(&format!("wp-admin/post-new.php?post_type=acf-taxonomy&use_taxonomy={}&_wpnonce={}", post.id, (ctx.nonce)(&format!("acfduplicate-{}", post.id)))),
    };
    actions.push(("acfduplicate".into(), format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&duplicate_url), esc_attr(&tr(site, "Duplicate this item")), tr(site, "Duplicate"))));
    let toggle = if post.post_status == "acf-disabled" {
        let url = list_url(ctx, post_type, &format!("&acfactivate={}&_wpnonce={nonce}", post.id));
        actions.push(("acfactivate".into(), format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&url), esc_attr(&tr(site, "Activate this item")), tr(site, "Activate"))));
        "acfactivate"
    } else {
        let url = list_url(ctx, post_type, &format!("&acfdeactivate={}&_wpnonce={nonce}", post.id));
        actions.push(("acfdeactivate".into(), format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&url), esc_attr(&tr(site, "Deactivate this item")), tr(site, "Deactivate"))));
        "acfdeactivate"
    };
    // `array_merge( array_flip( $order ), $actions )`: a key of the order without an action keeps its index.
    let order = ["edit", "acfduplicate", toggle, "trash"];
    let mut out: Vec<(String, String)> = order.iter().enumerate().map(|(i, key)| ((*key).to_owned(), actions.iter().find(|(k, _)| k == key).map_or_else(|| i.to_string(), |(_, v)| v.clone()))).collect();
    out.extend(actions.into_iter().filter(|(k, _)| !order.contains(&k.as_str())));
    out
}

/// `admin_table_bulk_actions()`.
pub fn bulk_actions(ctx: &AdminContext<'_>, mut actions: Vec<(String, String)>) -> Vec<(String, String)> {
    let Some(post_type) = list_of(ctx) else { return actions };
    if !matches!(view(ctx).as_str(), "sync" | "trash") {
        let site = ctx.site;
        let mut set = |key: &str, label: String| match actions.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = label,
            None => actions.push((key.to_owned(), label)),
        };
        if post_type == "acf-field-group" {
            set("acfduplicate", tr(site, "Duplicate"));
        }
        set("acfactivate", tr(site, "Activate"));
        set("acfdeactivate", tr(site, "Deactivate"));
    }
    actions
}

/// `$wp_post_statuses['trash']` moves behind ACF's status on its lists.
pub fn list_views(ctx: &AdminContext<'_>, mut views: Vec<(String, String)>) -> Vec<(String, String)> {
    if list_of(ctx).is_some()
        && let Some(at) = views.iter().position(|(k, _)| k == "trash")
    {
        let trash = views.remove(at);
        views.push(trash);
    }
    views
}

/// `acf_get_location_rule( $param )->get_object_type()`.
fn rule_object_type(param: &str) -> &'static str {
    super::rest_schema::object_type_of(param)
}

/// The built-in taxonomies' `object_type`.
fn taxonomy_object_types(site: &Site, taxonomy: &str) -> Option<Vec<String>> {
    let builtin: &[&str] = match taxonomy {
        "category" | "post_tag" | "post_format" => &["post"],
        "nav_menu" => &["nav_menu_item"],
        "link_category" => &["link"],
        "wp_theme" => &["wp_template", "wp_template_part", "wp_global_styles"],
        "wp_template_part_area" => &["wp_template_part"],
        "wp_pattern_category" => &["wp_block"],
        other => return site.types.admin_taxonomies().into_iter().find(|t| t.name == other).map(|t| t.object_types),
    };
    Some(builtin.iter().map(|s| (*s).to_owned()).collect())
}

/// `get_object_subtype( $rule )` of the location types: a list of subtypes, empty for none.
fn rule_object_subtypes(site: &Site, rule: &Rule) -> Vec<String> {
    let value = strval(Some(&rule.value));
    let equal = rule.operator == "==";
    match rule.param.as_str() {
        "page_parent" | "page_template" | "page_type" | "page" => vec!["page".to_owned()],
        "post_type" | "taxonomy" if equal => vec![value],
        "post_category" | "post_taxonomy" if equal => match value.split_once(':') {
            Some((taxonomy, _)) if value.find(':').is_some_and(|at| at > 0) => taxonomy_object_types(site, taxonomy).unwrap_or_default(),
            _ => Vec::new(),
        },
        // `acf_get_post_templates()`: pages always; other templates are taken as pages' (see the module comment).
        "post_template" if equal => vec!["page".to_owned()],
        _ => Vec::new(),
    }
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect()
}

/// `acf_get_object_type( $type, $subtype )`: `(name, label, icon)`.
fn object_type(site: &Site, object_type: &str, subtype: &str) -> Option<(String, String, String)> {
    let name = if subtype.is_empty() { object_type.to_owned() } else { format!("{object_type}/{subtype}") };
    let (label, icon) = match object_type {
        "post" if subtype.is_empty() => (tr(site, "Posts"), "dashicons-admin-post".to_owned()),
        "post" => {
            let builtin_icon = match subtype {
                "post" => Some(Some("dashicons-admin-post")),
                "page" => Some(Some("dashicons-admin-page")),
                "attachment" => Some(Some("dashicons-admin-media")),
                "revision" | "nav_menu_item" | "custom_css" | "customize_changeset" | "oembed_cache" | "user_request" | "wp_block" | "wp_template" | "wp_template_part" | "wp_global_styles" | "wp_navigation" | "wp_font_family" | "wp_font_face" => Some(None),
                _ => None,
            };
            let icon = match builtin_icon {
                Some(icon) => icon.map(str::to_owned),
                None => {
                    site.types.any_post_type(subtype)?;
                    site.types.admin_post_types().into_iter().find(|t| t.name == subtype).and_then(|t| t.menu_icon)
                }
            };
            // A `menu_icon` that is no dashicon name counts as none.
            let icon = icon.filter(|i| !i.is_empty() && i.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')).unwrap_or_else(|| "dashicons-admin-post".to_owned());
            (site.post_type_label(subtype), icon)
        }
        "term" if subtype.is_empty() => (tr(site, "Taxonomies"), "dashicons-tag".to_owned()),
        "term" => {
            taxonomy_object_types(site, subtype)?;
            (site.taxonomy_label(subtype), "dashicons-tag".to_owned())
        }
        "attachment" => (tr(site, "Attachments"), "dashicons-admin-media".to_owned()),
        "comment" => (tr(site, "Comments"), "dashicons-admin-comments".to_owned()),
        "widget" => (tr(site, "Widgets"), "dashicons-screenoptions".to_owned()),
        "menu" => (tr(site, "Menus"), "dashicons-admin-appearance".to_owned()),
        "menu_item" => (tr(site, "Menu items"), "dashicons-admin-appearance".to_owned()),
        "user" => (tr(site, "Users"), "dashicons-admin-users".to_owned()),
        "option" => (tr(site, "Options"), "dashicons-admin-generic".to_owned()),
        "block" => (tr(site, "Blocks"), "dashicons-block-default".to_owned()),
        _ => return None,
    };
    Some((name, label, icon))
}

/// `render_admin_table_column_locations()`.
fn locations_column(site: &Site, location: &[Vec<Rule>]) -> String {
    let mut objects: Vec<(String, String, String)> = Vec::new();
    for rules in location {
        let Some(object_type_name) = rules.iter().map(|r| rule_object_type(&r.param)).find(|t| !t.is_empty()) else { continue };
        let subtypes: Vec<Vec<String>> = rules.iter().map(|r| rule_object_subtypes(site, r)).filter(|s| !s.is_empty()).collect();
        let subtypes: Vec<String> = match subtypes.as_slice() {
            [] => vec![String::new()],
            [one] => one.clone(),
            [first, rest @ ..] => first.iter().filter(|s| rest.iter().all(|other| other.contains(s))).cloned().collect(),
        };
        for subtype in subtypes {
            if let Some(object) = object_type(site, object_type_name, &subtype) {
                match objects.iter_mut().find(|(name, _, _)| *name == object.0) {
                    Some(slot) => *slot = object,
                    None => objects.push(object),
                }
            }
        }
    }
    let html = match objects.first() {
        Some((_, _, icon)) => {
            let labels: Vec<&str> = objects.iter().take(3).map(|(_, label, _)| label.as_str()).collect();
            format!("<span class=\"dashicons {icon}{}\"></span>{}{}", if objects.len() > 1 { " acf-multi-dashicon" } else { "" }, labels.join(", "), if objects.len() > 3 { ", ..." } else { "" })
        }
        None => format!("<span class=\"dashicons dashicons-businesswoman\"></span> {}", tr(site, "Various")),
    };
    rustpress_core::kses::kses_post(&html)
}

/// The "no description" dash.
fn emdash(site: &Site, text: &str) -> String {
    format!("<span class=\"acf-emdash\" aria-hidden=\"true\">—</span><span class=\"screen-reader-text\">{}</span>", esc_tr(site, text))
}

/// Up to three labels, the others in a tooltip.
fn label_list(labels: &[String]) -> String {
    let mut text = labels.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
    if labels.len() > 3 {
        text.push_str(&format!(", <span class=\"acf-more-items acf-js-tooltip\" title=\"{}\">+{}</span>", labels[3..].join(", "), labels.len() - 3));
    }
    rustpress_core::kses::kses_post(&text)
}

/// `get_post_type_object( $name )` exists.
fn post_type_exists(site: &Site, name: &str) -> bool {
    matches!(name, "post" | "page" | "attachment" | "revision" | "nav_menu_item" | "custom_css" | "customize_changeset" | "oembed_cache" | "user_request" | "wp_block" | "wp_template" | "wp_template_part" | "wp_global_styles" | "wp_navigation" | "wp_font_family" | "wp_font_face")
        || site.types.any_post_type(name).is_some()
        || site.types.admin_post_types().iter().any(|t| t.name == name)
}

/// The strings of an array setting.
fn string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items.iter().map(|(_, v)| strval(Some(v))).collect(),
        _ => Vec::new(),
    }
}

/// The titles of the visible field groups of a screen (`acf_get_field_groups( $args )`).
async fn group_titles(groups: &[FieldGroup], ctx: &AdminContext<'_>, screen: super::fields::Screen<'_>) -> rustpress_core::Result<Vec<String>> {
    let matcher = super::fields::MatchContext { site: ctx.site, user: super::fields::MatchUser::Admin(ctx.can), admin: true };
    let mut out = Vec::new();
    for group in groups {
        if group.visible(&screen, &matcher).await? {
            out.push(group.title.clone());
        }
    }
    Ok(out)
}

/// `admin_table_columns_html()`: the item as `acf_get_internal_post_type()` reads it.
pub async fn posts_custom_column(groups: &[FieldGroup], ctx: &AdminContext<'_>, column: &str, post: &Post) -> rustpress_core::Result<String> {
    if list_of(ctx) != Some(post.post_type.as_str()) {
        return Ok(String::new());
    }
    let site = ctx.site;
    let settings = rustpress_core::options::maybe_unserialize(&post.post_content);
    let common = match column {
        "acf-key" => Some(format!("<i class=\"acf-icon acf-icon-key-solid\"></i>{}", esc_html(&post.post_name))),
        "acf-description" => Some(match settings.get("description") {
            Some(description @ (Value::Str(_) | Value::Int(_) | Value::Float(_))) if super::php::truthy(Some(description)) => format!("<span class=\"acf-description\">{}</span>", super::render::esc_html_acf(description)),
            _ => emdash(site, "No description"),
        }),
        _ => None,
    };
    if let Some(cell) = common {
        return Ok(cell);
    }
    match post.post_type.as_str() {
        "acf-post-type" => return post_type_column(groups, ctx, column, post, &settings).await,
        "acf-taxonomy" => return taxonomy_column(groups, ctx, column, post, &settings).await,
        _ => {}
    }
    Ok(match column {
        "acf-location" => {
            let location = match groups.iter().find(|g| g.id == post.id) {
                Some(group) => group.location.clone(),
                None => super::fields::parse_location(&settings),
            };
            locations_column(site, &location)
        }
        "acf-count" => {
            let count = match groups.iter().find(|g| g.id == post.id) {
                Some(group) => group.fields.len(),
                None => site.db.acf_fields().await?.iter().filter(|f| f.parent == post.id).count(),
            };
            if count == 0 {
                emdash(site, "No fields")
            } else if post.post_status == "trash" {
                esc_html(&site.number_i18n(count as u64))
            } else {
                format!("<a href=\"{}\">{}</a>", esc_url(&site.site_url(&format!("wp-admin/post.php?action=edit&post={}", post.id))), esc_html(&site.number_i18n(count as u64)))
            }
        }
        _ => String::new(),
    })
}

/// `$post['active']` of `ACF_Internal_Post_Type::get_raw_post()` (the status), and not in the trash.
fn active(post: &Post) -> bool {
    matches!(post.post_status.as_str(), "publish" | "auto-draft")
}

/// `ACF_Admin_Post_Types::render_admin_table_column()`.
async fn post_type_column(groups: &[FieldGroup], ctx: &AdminContext<'_>, column: &str, post: &Post, settings: &Value) -> rustpress_core::Result<String> {
    let site = ctx.site;
    let name = strval(settings.get("post_type"));
    Ok(match column {
        "acf-taxonomies" => {
            let mut taxonomies = string_list(settings.get("taxonomies"));
            for acf_taxonomy in site.db.acf_posts("acf-taxonomy").await? {
                let taxonomy = rustpress_core::options::maybe_unserialize(&acf_taxonomy.content);
                if string_list(taxonomy.get("object_type")).contains(&name) {
                    taxonomies.push(strval(taxonomy.get("taxonomy")));
                }
            }
            let mut labels: Vec<String> = Vec::new();
            let mut seen: Vec<String> = Vec::new();
            for taxonomy in taxonomies {
                if seen.contains(&taxonomy) {
                    continue;
                }
                seen.push(taxonomy.clone());
                if taxonomy_object_types(site, &taxonomy).is_none() {
                    continue;
                }
                let label = site.taxonomy_label(&taxonomy);
                if !label.is_empty() {
                    labels.push(label);
                }
            }
            if labels.is_empty() { emdash(site, "No taxonomies") } else { label_list(&labels) }
        }
        "acf-field-groups" => {
            let titles = group_titles(groups, ctx, super::fields::Screen::PostType { post_type: &name }).await?;
            if titles.is_empty() { emdash(site, "No field groups") } else { label_list(&titles) }
        }
        "acf-count" => {
            let none = rustpress_core::kses::kses_post(&emdash(site, "No posts"));
            if !active(post) {
                return Ok(none);
            }
            let published = site.db.post_status_counts(&name).await?.into_iter().find(|(s, _)| s == "publish").map_or(0, |(_, n)| n);
            if published == 0 {
                none
            } else {
                format!("<a href=\"{}\">{}</a>", esc_url(&site.site_url(&format!("wp-admin/edit.php?post_type={name}"))), esc_html(&site.number_i18n(published)))
            }
        }
        _ => String::new(),
    })
}

/// `ACF_Admin_Taxonomies::render_admin_table_column()`.
async fn taxonomy_column(groups: &[FieldGroup], ctx: &AdminContext<'_>, column: &str, post: &Post, settings: &Value) -> rustpress_core::Result<String> {
    let site = ctx.site;
    let name = strval(settings.get("taxonomy"));
    Ok(match column {
        "acf-post-types" => {
            let mut labels: Vec<String> = string_list(settings.get("object_type")).iter().filter(|t| post_type_exists(site, t)).map(|t| site.post_type_label(t)).filter(|l| !l.is_empty()).collect();
            for acf_post_type in site.db.acf_posts("acf-post-type").await? {
                let post_type = rustpress_core::options::maybe_unserialize(&acf_post_type.content);
                if string_list(post_type.get("taxonomies")).contains(&name) {
                    labels.push(acf_post_type.title.clone());
                }
            }
            if labels.is_empty() {
                emdash(site, "No post types")
            } else {
                let mut unique: Vec<String> = Vec::new();
                for label in labels {
                    if !unique.contains(&label) {
                        unique.push(label);
                    }
                }
                label_list(&unique)
            }
        }
        "acf-field-groups" => {
            let titles = group_titles(groups, ctx, super::fields::Screen::Term { taxonomy: &name }).await?;
            if titles.is_empty() { emdash(site, "No field groups") } else { label_list(&titles) }
        }
        "acf-count" => {
            let none = rustpress_core::kses::kses_post(&emdash(site, "No terms"));
            if !active(post) {
                return Ok(none);
            }
            let top_level = site.db.terms(&name).await?.iter().filter(|t| t.parent == 0).count() as u64;
            if top_level == 0 {
                none
            } else {
                format!("<a href=\"{}\">{}</a>", esc_url(&site.site_url(&format!("wp-admin/edit-tags.php?taxonomy={name}"))), esc_html(&site.number_i18n(top_level)))
            }
        }
        _ => String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utm_tags() {
        assert_eq!(utm("https://www.advancedcustomfields.com/pro/", "ACF upgrade", "header", "", "", ""), "https://www.advancedcustomfields.com/pro/?utm_source=ACF+Free&#038;utm_medium=insideplugin&#038;utm_campaign=ACF+upgrade&#038;utm_content=header");
        assert_eq!(
            utm("https://wpengine.com/plans/?coupon=freedomtocreate", "bx_prod_referral", "acf_free_plugin_topbar_dropdown_cta", "", "acf_plugin", "referral"),
            "https://wpengine.com/plans/?coupon=freedomtocreate&#038;utm_source=acf_plugin&#038;utm_medium=referral&#038;utm_campaign=bx_prod_referral&#038;utm_content=acf_free_plugin_topbar_dropdown_cta"
        );
    }
}
