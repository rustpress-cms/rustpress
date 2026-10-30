//! The administration of SVG Support: "Settings > SVG Support" (with its help), saving the settings through
//! `options.php` and by Ajax, the plugin's assets on its own screens, the rating in the footer and its links in the
//! list of plugins.

use rustpress_blocks::php_json;
use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::native::{AdminContext, AdminHelp, AdminPage, AjaxAnswer, Assets, HelpTab};
use rustpress_phpser::{Key, Value};

use super::{Settings, VERSION};

/// The text domain; its translations come from `wp-content/languages/plugins`.
pub(super) const DOMAIN: &str = "svg-support";

/// The `$menu_slug` of the settings page.
const SLUG: &str = "svg-support";

/// `$hook_suffix` of the settings page.
const HOOK: &str = "settings_page_svg-support";

fn t(site: &Site, text: &str) -> String {
    site.trd(DOMAIN, text)
}

/// `bodhi_svgs_admin_menu()`.
pub(super) fn pages(site: &Site) -> Vec<AdminPage> {
    vec![AdminPage { parent: "options-general.php", slug: SLUG, page_title: t(site, "SVG Support Settings and Usage"), menu_title: t(site, "SVG Support"), capability: "manage_options" }]
}

fn plugin_url(site: &Site, path: &str) -> String {
    site.site_url(&format!("wp-content/plugins/svg-support/{path}"))
}

/// `bodhi_svgs_admin_css()` and `bodhi_svgs_admin_settings_assets()`.
pub(super) fn enqueue(ctx: &AdminContext<'_>, settings: &Settings, assets: &mut Assets) {
    let site = ctx.site;
    let settings_page = ctx.hook_suffix == HOOK;
    let media_library = ctx.hook_suffix == "upload.php";
    if settings_page || media_library {
        assets.style("bodhi-svgs-admin", plugin_url(site, "css/svgs-admin.css"), &[], VERSION, "all");
    }
    if settings_page && !settings.advanced_mode() {
        assets.style("bodhi-svgs-admin-simple-mode", plugin_url(site, "css/svgs-admin-simple-mode.css"), &[], VERSION, "all");
    }
    if matches!(ctx.hook_suffix, "post.php" | "post-new.php") {
        assets.style("bodhi-svgs-admin-edit-post", plugin_url(site, "css/svgs-admin-edit-post.css"), &[], VERSION, "all");
    }
    if !settings_page {
        return;
    }
    assets.style("bodhi-svgs-settings", plugin_url(site, "css/svgs-settings.css"), &[], VERSION, "all");
    let data = serde_json::json!({
        "ajaxUrl": site.site_url("wp-admin/admin-ajax.php"),
        "nonce": (ctx.nonce)("bodhi_svgs_autosave"),
        "iconsUrl": plugin_url(site, "admin/img/icons.svg"),
        "i18n": {
            "idle": t(site, "All changes saved"),
            "saving": t(site, "Saving…"),
            "saved": t(site, "Changes saved"),
            "error": t(site, "Couldn't save — retrying on next change"),
            "btnIdle": t(site, "Save changes"),
            "btnSaving": t(site, "Saving…"),
            "btnSaved": t(site, "Saved"),
        },
    });
    let script = assets.script("bodhi-svgs-settings", plugin_url(site, "js/svgs-settings.js"), &[], VERSION, true);
    script.before.push(format!("var SvgsSettings = {};", php_json::encode(&data)));
}

/// `bodhi_svgs_admin_footer_text()` on the settings page and in the media library: it prints the rating instead of
/// returning the thanks.
pub(super) fn footer_text(ctx: &AdminContext<'_>) -> Option<String> {
    if ctx.hook_suffix != HOOK && ctx.hook_suffix != "upload.php" {
        return None;
    }
    let text = esc_html(&t(ctx.site, "If you like %1$sSVG Support%2$s please leave a %3$s★★★★★%4$s rating. A huge thanks in advance!"));
    Some(rustpress_core::i18n::sprintf(&text, &["<strong>", "</strong>", "<a href=\"https://wordpress.org/support/view/plugin-reviews/svg-support?filter=5#postform\" target=\"_blank\" class=\"svgs-rating-link\">", "</a>"]))
}

/// `bodhi_svgs_plugin_action_links()`.
pub(super) fn action_links(site: &Site, links: Vec<String>) -> Vec<String> {
    let mut all = vec![format!("<a href=\"{}\" title=\"{}\">{}</a>", site.site_url("wp-admin/options-general.php?page=svg-support"), t(site, "SVG Support Settings"), t(site, "Settings"))];
    all.extend(links);
    all
}

/// `bodhi_svgs_plugin_meta_links()`.
pub(super) fn row_meta(site: &Site, file: &str, mut links: Vec<String>) -> Vec<String> {
    if file == "svg-support/svg-support.php" {
        links.push(format!("<a target=\"_blank\" href=\"https://wordpress.org/support/plugin/svg-support\">{}</a>", t(site, "Get Support")));
        links.push(format!("<a target=\"_blank\" href=\"https://wordpress.org/support/plugin/svg-support/reviews/\">{}</a>", t(site, "Leave a Review")));
        links.push(format!("<a target=\"_blank\" href=\"https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=Z9R7JERS82EQQ&source=url\">{}</a>", t(site, "Donate to author")));
    }
    links
}

/// `bodhi_svgs_help_tab()`.
pub(super) fn help(site: &Site) -> AdminHelp {
    let t = |text: &str| t(site, text);
    let tab = |id: &str, title: &str, content: String| HelpTab { id: id.to_owned(), title: t(title), content };
    let overview = format!(
        "<h3>{}</h3><p>{}</p><p>{}</p><p>{}</p><p>{}</p>",
        t("Overview"),
        t("At it's core, SVG Support allows you to upload SVG files and use them as you would any regular image with the added benefit of being scalable and looking great on any screen size, no matter what size it's displayed. Additionally, SVG file sizes are (more often than not) much smaller than conventional image formats."),
        t("Even this most basic of usage is very powerful for modern websites, however, there's lots of cool stuff you can do with SVG files!"),
        t("SVG Support features an \"Advanced Mode\" which toggles extra features, allowing you to take more control of how your SVG files are used. By rendering SVG files inline, it opens up a huge number of possibilities including animations, embedded links within the SVG, odd shaped link areas, custom CSS targeting elements within the SVG and  whole lot more!"),
        t("So let's get into some more details! Simply click the tabs to the left to get more of an understanding of how powerful SVG Support is.")
    );
    let setting = |label: &str, text: String| format!("<p><strong>{}</strong><br>{text}</p>", t(label));
    let the_settings = format!(
        "<h3>{}</h3>{}{}{}{}{}{}{}",
        t("The Settings"),
        setting("Restrict To Administrators:", t("SVG files are actually XML code, so allowing regular users to upload them can pose serious security risks. Please leave this checked unless you really know what you're doing.")),
        setting("Enable Advanced Mode:", format!("{}<br>{}", t("When using SVG files like regular images just isn't enough ;)"), t("Enabling \"Advanced Mode\" displays options to give you more control over how you use SVG files on your site. It also includes extra JS on the front end, so leave this disabled unless you're actually using any of the advanced features."))),
        setting("Output JS in Footer:", t("This setting allows you to choose whether the SVG Support JS file is enqueued in the header or the footer of the site. Usually you would enqueue in the footer unless you need it to be loaded sooner for some reason.")),
        setting("Use Expanded JS:", t("This setting gives you the choice of JS file that is enqueued, the full expanded version or the minified version. You would usually enqueue the minified version, but if you want to bundle the JS file using a caching or minification plugin or similar, then you might want to enqueue the expanded, non-minified version.")),
        setting("CSS Class To Target:", t("This allows you to set your own custom class that you will use in your SVG source IMG tags that you would like rendered inline. For example, it might be easier for you to remember to add the class \"inline-svg\" or something, in which case you would use your desired class name in this field to be used across your site.")),
        setting("Automatically Insert Class:", t("When this is checked, you won't have to add the class to your SVG files during the embed process in the editor. When you pick your SVG, it will be placed in the editor with just the SVG Support class and others stripped. It does not change existing code, it's only a helper to allow you to quickly embed your SVG files and have them render inline without having to fiddle with the classes.")),
        setting("Force Inline SVG:", t("This feature is to force all SVG files that are found in your site to be rendered inline. This can help if you aren't able to set a custom class on your IMG tags for some reason, usually when used in theme options or page builder elements."))
    );
    let standard = format!(
        "<h3>{}</h3><p>{}</p><p>{}</p>",
        t("Standard Usage"),
        t("You can simply upload SVG files to your media library like any other image.<br>Make sure to select \"Restrict to Administrators\" if you only want to allow admins to upload SVG files."),
        t("If you want to enable sanitization or minification of uploaded SVG files, please enable advanced settings and then enable sanitization and/or minification.")
    );
    let inline = format!(
        "<h3>{}</h3><p>{}<br>{}</p><pre><code>&lt;img class=\"style-svg\" alt=\"alt-text\" src=\"image-source.svg\" /&gt;</code></pre>{}<pre><code>&lt;img class=\"your-custom-class\" alt=\"alt-text\" src=\"image-source.svg\" /&gt;</code></pre><p>{}<br>{}</p><p><em>{}</em><br><em>- {}</em><br><em>- {}</em><br><em>- {}</em></p>",
        t("Render SVG Inline"),
        t("You can embed your SVG image like any standard image with the addition of adding the class <code>style-svg</code> (or your custom class) to any IMG tags that you want this plugin to swap out with your actual SVG code."),
        t("For example:"),
        t("or"),
        t("The whole IMG tag element will now be dynamically replaced by the actual code of your SVG, making the inner content targetable."),
        t("This allows you to target elements within your SVG using CSS."),
        t("Please Note:"),
        t("You will likely need to set your own height and width in your CSS for SVG files to display correctly."),
        t("Your uploaded image needs to be an SVG file for this plugin to replace the img tag with the inline SVG code. It will not create SVG files for you."),
        t("You can set this target class on any element and the script will traverse all children of that target element looking for IMG tags with SVG in the src to replace.")
    );
    let featured = format!(
        "<h3>{}</h3><p>{}</p><p>{}</p><ol><li>{}</li><li>{}</li><li>{}</li><li>{}</li><li>{}</li></ol>",
        t("Featured Images"),
        t("You can use SVG files as featured images just like any other image format, with the addition of being able to render your featured SVG inline on a per-post basis."),
        t("To render your featured SVG inline:"),
        t("Make sure \"Advanced Mode\" is enabled."),
        t("Add your featured SVG like you would any regular featured image format."),
        t("Publish, Save Draft, or Update the post."),
        t("Once the screen reloads, click the new checkbox below the featured image to render your SVG inline."),
        t("Publish, Save Draft, or Update the post a final time to render the SVG inline.")
    );
    let animation = format!(
        "<h3>{}</h3><p>{}<br>{}</p><p><strong>{}</strong><br>{}</p><p>{}</p>",
        t("Animation"),
        t("So you want to animate your SVG?"),
        t("There's a number of ways you can animate an SVG. You could use CSS or JS to target elements within your SVG or even embed the animations in the file itself. Whichever way you choose, there is always a little bit of preparation required before uploading your SVG to your media library."),
        t("First, let's talk about using CSS or JS to target elements within your SVG."),
        t("Before you upload your SVG, you're going to need some classes to target inside your SVG. To do this, open your SVG file in the code editor of choice (I use Sublime Text). You will see each element within your SVG file written in XML code. Each little part of your SVG has it's own bit of code, so it's up to you which ones you want to target. It's in here that you'll place your new classes on each element you want to target."),
        t("Then there's the option of animating the SVG file itself. There is a number of online tools to do this, or you can use the software of your choice. Once your SVG is animated and ready to go, you then upload it like any other image to your WordPress media library. When you embed it on a page/post, you will need to make sure to add the class to the IMG tag so SVG Support can render it inline. This will ensure your animations are displayed.")
    );
    let donations = format!(
        "<h3>{}</h3><p>{}</p><p><strong>{}</strong></p><p><strong>{}</strong><a href=\"https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=Z9R7JERS82EQQ\" target=\"_blank\">Click Here</a><br/><strong>{}</strong><br/><strong>{}</strong><br/><strong>{}</strong></p><p>{}</br>{}</br>{}<br/><a href=\"https://www.coinbase.com/join/59be646cb87715012bbdcc6b\" target=\"_blank\">https://www.coinbase.com/join/59be646cb87715012bbdcc6b</a></p>",
        t("Donations"),
        t("SVG Support (this plugin) has grown to be used by over 1 million websites actively and is maintained solely by one person. I couldn't possibly tell you how many hours have gone into the development, maintenance and support of this plugin. If you find it useful and would like to donate to help keep it going, that would be amazing! I truly appreciate the support and how far this has come."),
        t("Donation Methods:"),
        t("PayPal: "),
        t("BTC: 1qF8r2HkTLifND7WLGfWmvxfXc9ze55DZ"),
        t("LTC: LUnQPJrSk6cVFmMqBMv5FAqweJbnzRUz4o"),
        t("ETH: 0x599695Eb51aFe2e5a0DAD60aD9c89Bc8f10B54f4"),
        t("Need to buy some crypto to donate?"),
        t("My Coinbase referral link will get $10 USD worth of BTC for free when you spend $100."),
        t("(You don't need to send me that much though, anything is appreciated!)")
    );
    AdminHelp {
        tabs: vec![
            tab("bodhi_svgs_help_tab-overview", "Overview", overview),
            tab("bodhi_svgs_help_tab-the_settings", "The Settings", the_settings),
            tab("bodhi_svgs_help_tab_usage-standard", "Standard Usage", standard),
            tab("bodhi_svgs_help_tab-inlining_svg", "Render SVG Inline", inline),
            tab("bodhi_svgs_help_tab-featured_images", "Featured Images", featured),
            tab("bodhi_svgs_help_tab-animation", "Animation", animation),
            tab("bodhi_svgs_help_tab-donations", "DONATIONS", donations),
        ],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p>", t("For more help, visit:"), t("<a target=\"_blank\" href=\"https://wordpress.org/support/plugin/svg-support\">SVG Support Forum</a>")),
    }
}

/// `bodhi_svgs_settings_sanitize()`: the settings as the form (or the Ajax autosave) sent them, with `css_target`
/// cleaned and the switches that were not sent turned off.
pub(super) fn sanitize(value: Option<Value>) -> Option<Value> {
    let mut items: Vec<(Key, Value)> = match value {
        Some(Value::Array(items)) => items,
        None | Some(Value::Null) => Vec::new(),
        // Anything else is returned as it came.
        other => return other,
    };
    let set = |items: &mut Vec<(Key, Value)>, key: &str, new: Value| match items.iter_mut().find(|(k, _)| *k == Key::Str(key.as_bytes().to_vec())) {
        Some((_, slot)) => *slot = new,
        None => items.push((Key::Str(key.as_bytes().to_vec()), new)),
    };
    let get = |items: &[(Key, Value)], key: &str| items.iter().find(|(k, _)| *k == Key::Str(key.as_bytes().to_vec())).map(|(_, v)| v.clone()).filter(|v| !matches!(v, Value::Null));
    if let Some(target) = get(&items, "css_target") {
        let text = target.as_str().map(str::to_owned).unwrap_or_default();
        set(&mut items, "css_target", Value::Str(esc_attr(&rustpress_content::sanitize::text_field(&text)).into_bytes()));
    }
    if get(&items, "sanitize_svg_front_end").and_then(|v| v.as_str().map(str::to_owned)).as_deref() != Some("on") {
        set(&mut items, "sanitize_svg_front_end", Value::Bool(false));
    }
    for key in ["sanitize_on_upload_roles", "restrict"] {
        let roles = match get(&items, key) {
            None => Value::Array(vec![(Key::Int(0), Value::Str(b"none".to_vec()))]),
            Some(Value::Array(list)) => Value::Array(list),
            // `(array) $value`.
            Some(other) => Value::Array(vec![(Key::Int(0), other)]),
        };
        set(&mut items, key, roles);
    }
    Some(Value::Array(items))
}

/// `map_deep( $value, 'sanitize_text_field' )`.
fn map_text_field(value: Value) -> Value {
    match value {
        Value::Str(text) => Value::Str(rustpress_content::sanitize::text_field(&String::from_utf8_lossy(&text)).into_bytes()),
        Value::Array(items) => Value::Array(items.into_iter().map(|(k, v)| (k, map_text_field(v))).collect()),
        other => other,
    }
}

/// `bodhi_svgs_autosave_settings()`.
pub(super) async fn autosave(ctx: &AdminContext<'_>) -> rustpress_core::Result<AjaxAnswer> {
    // `check_ajax_referer( 'bodhi_svgs_autosave', 'nonce' )`.
    let nonce = ctx.post_field("nonce").or_else(|| ctx.get("nonce")).unwrap_or_default();
    if !(ctx.verify_nonce)(nonce, "bodhi_svgs_autosave") {
        return Ok(AjaxAnswer::text(403, "-1"));
    }
    let error = |status: u16, message: &str| AjaxAnswer::json(status, php_json::encode(&serde_json::json!({"success": false, "data": {"message": esc_html(&t(ctx.site, message))}})));
    if !(ctx.can)("manage_options") {
        return Ok(error(403, "You can't play with this."));
    }
    let raw = rustpress_core::native::form_value(ctx.form, "bodhi_svgs_settings").unwrap_or(Value::Array(Vec::new()));
    if !matches!(raw, Value::Array(_)) {
        return Ok(error(400, "Invalid settings payload."));
    }
    let clean = sanitize(Some(map_text_field(raw))).unwrap_or(Value::Array(Vec::new()));
    if let Some(writer) = ctx.writer {
        writer.update_option("bodhi_svgs_settings", &String::from_utf8_lossy(&rustpress_phpser::serialize(&clean))).await?;
    }
    Ok(AjaxAnswer::json(200, "{\"success\":true}".to_owned()))
}

/// `bodhi_svgs_icon()`: an icon of the bundled sprite.
fn icon(icons: &str, name: &str, size: u32) -> String {
    format!("<svg class=\"svgs-icon\" width=\"{size}\" height=\"{size}\" aria-hidden=\"true\" focusable=\"false\"><use href=\"{icons}#{}\"></use></svg>", esc_attr(name))
}

/// `checked( $value )`.
fn checked(on: bool) -> &'static str {
    if on { " checked='checked'" } else { "" }
}

/// `wp_kses( $desc, array( 'code' => array(), 'strong' => array(), 'em' => array() ) )` for the plugin's own texts.
fn kses_desc(text: &str) -> String {
    rustpress_core::kses::kses(text, &rustpress_core::kses::Allowed::tags(&[("code", &[]), ("strong", &[]), ("em", &[])]))
}

/// `bodhi_svgs_toggle_row()`.
fn toggle_row(key: &str, label: &str, desc: &str, is_checked: bool, value: &str) -> String {
    let field = esc_attr(&format!("bodhi_svgs_settings[{key}]"));
    let value = if value.is_empty() { String::new() } else { format!(" value=\"{}\"", esc_attr(value)) };
    format!(
        "\t\t<div class=\"svgs-row\">\n\t\t\t<label class=\"svgs-toggle-row\" for=\"{field}\">\n\t\t\t\t<input id=\"{field}\" name=\"{field}\" type=\"checkbox\" class=\"svgs-switch-input\"{value} {} />\n\t\t\t\t<span class=\"svgs-switch\" aria-hidden=\"true\"></span>\n\t\t\t\t<span class=\"svgs-toggle-text\">\n\t\t\t\t\t<span class=\"svgs-toggle-label\">{}</span>\n\t\t\t\t\t<span class=\"svgs-toggle-desc\">{}</span>\n\t\t\t\t</span>\n\t\t\t</label>\n\t\t</div>\n\t\t",
        checked(is_checked),
        esc_html(label),
        kses_desc(desc)
    )
}

/// `bodhi_svgs_role_chips()`: a checkbox per role of the site.
fn role_chips(icons: &str, key: &str, roles: &[(String, String)], selected: &[String]) -> String {
    let mut out = String::from("<div class=\"svgs-chips\">");
    for (slug, name) in roles {
        let slug = esc_attr(slug);
        out.push_str(&format!(
            "\t\t\t<label class=\"svgs-chip\">\n\t\t\t\t<input type=\"checkbox\" name=\"bodhi_svgs_settings[{}][]\" value=\"{}\" {} />\n\t\t\t\t<span class=\"svgs-chip-check\">{}</span>\n\t\t\t\t<span>{}</span>\n\t\t\t</label>\n\t\t\t",
            esc_attr(key),
            esc_attr(&slug),
            checked(selected.contains(&slug)),
            icon(icons, "check", 13),
            esc_html(name)
        ));
    }
    out.push_str("</div>");
    out
}

/// The roles of the site in their order, with the translated names (`translate_user_role()`).
fn roles(site: &Site) -> Vec<(String, String)> {
    let Some(Value::Array(items)) = site.options.value(&format!("{}user_roles", site.db.table_prefix())) else { return Vec::new() };
    items
        .iter()
        .filter_map(|(key, role)| match key {
            Key::Str(slug) => Some((String::from_utf8_lossy(slug).into_owned(), site.trx(role.get("name").and_then(Value::as_str).unwrap_or_default(), "User role"))),
            Key::Int(_) => None,
        })
        .collect()
}

/// `(array) $options[ $key ]` as strings.
fn list(settings: &Settings, key: &str) -> Vec<String> {
    match settings.get(key) {
        Some(Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        Some(Value::Null) | None => Vec::new(),
        Some(other) => other.as_str().map(|s| vec![s.to_owned()]).unwrap_or_default(),
    }
}

/// `bodhi_svg_support_settings_page()`: `admin/svgs-settings-page.php`.
pub(super) fn settings_page(ctx: &AdminContext<'_>, settings: &Settings, settings_fields: &str) -> String {
    let site = ctx.site;
    let t = |text: &str| t(site, text);
    let h = |text: &str| esc_html(&t(text));
    let a = |text: &str| esc_attr(&t(text));
    let icons = esc_url(&plugin_url(site, "admin/img/icons.svg"));
    let icon = |name: &str, size: u32| icon(&icons, name, size);
    let isset = |key: &str| settings.get(key).is_some_and(|v| !matches!(v, Value::Null));
    let roles = roles(site);
    let css_target = settings.get("css_target").filter(|v| !matches!(v, Value::Null)).map(|v| esc_attr(v.as_str().unwrap_or_default())).unwrap_or_default();
    let mut out = String::new();
    out.push_str(&format!(
        "\n<div class=\"wrap svgs-ds\">\n\n\t<div class=\"svgs-header\">\n\t\t<img src=\"{}\" width=\"30\" height=\"30\" alt=\"\" />\n\t\t<h1>{} <span class=\"svgs-version\">v{}</span></h1>\n\t\t<span class=\"svgs-savestate\" id=\"svgs-savestate\" data-state=\"idle\">\n\t\t\t{}\t\t\t<span>{}</span>\n\t\t</span>\n\t</div>\n\t<hr class=\"wp-header-end\">\n\n",
        esc_url(&plugin_url(site, "admin/img/logo.svg")),
        h("SVG Support"),
        esc_html(VERSION),
        icon("cloud", 16),
        h("All changes saved")
    ));
    out.push_str(&format!(
        "\t<div class=\"svgs-callout\">\n\t\t{}\t\t<div>\n\t\t\t<strong>{}</strong>\n\t\t\t<p>\n\t\t\t\t{}\t\t\t\t<code>style-svg</code>\n\t\t\t\t{}\t\t\t\t<a href=\"https://svg.support/tutorials/\" target=\"_blank\">svg.support</a>.\n\t\t\t</p>\n\t\t</div>\n\t</div>\n\n",
        icon("info", 18),
        h("Style SVGs with CSS — easily."),
        h("Enable Advanced Mode to embed your full SVG inline via a simple"),
        h("class, then target its internal elements with plain CSS. For help, check the Help tab (top right) or the tutorials at")
    ));
    out.push_str(&format!(
        "\t<div class=\"svgs-layout\">\n\n\t\t<div class=\"svgs-main\">\n\n\t\t\t<form name=\"bodhi_svgs_settings_form\" id=\"svgs-settings-form\" method=\"post\" action=\"options.php\">\n\n\t\t\t\t{settings_fields}\n\t\t\t\t<section class=\"svgs-panel\">\n\t\t\t\t\t<header class=\"svgs-panel-head\">\n\t\t\t\t\t\t<div>\n\t\t\t\t\t\t\t<h2>{}</h2>\n\t\t\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t</header>\n\t\t\t\t\t<div class=\"svgs-panel-body\">\n\n",
        h("General"),
        h("Uploads, sanitization and performance.")
    ));
    for (key, label, hint) in [
        ("restrict", "Restrict SVG uploads to", "Select the user roles that are allowed to upload SVG files. Only give upload permission to roles you trust."),
        ("sanitize_on_upload_roles", "Do not sanitize for these roles", "Select the user roles that should bypass SVG sanitization on upload. Leave empty unless you fully trust a role's uploads."),
    ] {
        out.push_str(&format!(
            "\t\t\t\t\t\t<div class=\"svgs-row\">\n\t\t\t\t\t\t\t<div class=\"svgs-field-label\">{}</div>\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t<p class=\"svgs-hint\">{}</p>\n\t\t\t\t\t\t</div>\n\n",
            h(label),
            role_chips(&icons, key, &roles, &list(settings, key)),
            h(hint)
        ));
    }
    out.push_str("\t\t\t\t\t\t");
    out.push_str(&toggle_row(
        "sanitize_svg_front_end",
        &t("Sanitize SVG on front-end"),
        &t("Enhance security by sanitizing SVG images on the front-end. This helps prevent XSS and injection attacks."),
        settings.get("sanitize_svg_front_end").and_then(Value::as_str) == Some("on"),
        "",
    ));
    out.push_str(&toggle_row("minify_svg", &t("Minify SVG"), &t("Auto-minify all SVG uploads to keep file sizes small."), isset("minify_svg"), ""));
    out.push_str(&toggle_row("frontend_css", &t("Load front-end CSS"), &t("A very small piece of CSS that helps with displaying SVGs on the front-end in some cases."), isset("frontend_css"), ""));
    out.push_str(&format!(
        "\n\t\t\t\t\t</div>\n\t\t\t\t</section>\n\n\t\t\t\t<section class=\"svgs-panel\" id=\"svgs-advanced-panel\">\n\t\t\t\t\t<header class=\"svgs-panel-head\">\n\t\t\t\t\t\t<div>\n\t\t\t\t\t\t\t<h2>{}</h2>\n\t\t\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t\t<div class=\"svgs-head-right\">\n\t\t\t\t\t\t\t<label class=\"svgs-toggle-row\" for=\"bodhi_svgs_settings[advanced_mode]\">\n\t\t\t\t\t\t\t\t<input id=\"bodhi_svgs_settings[advanced_mode]\" name=\"bodhi_svgs_settings[advanced_mode]\" type=\"checkbox\" class=\"svgs-switch-input\" aria-label=\"{}\" {} />\n\t\t\t\t\t\t\t\t<span class=\"svgs-switch\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t</header>\n\t\t\t\t\t<div class=\"svgs-panel-body\">\n\n",
        h("Advanced Mode"),
        h("Inline rendering, styling and animation. Adds a small JS file to your front-end."),
        a("Enable Advanced Mode"),
        checked(isset("advanced_mode"))
    ));
    out.push_str(&format!(
        "\t\t\t\t\t\t<div class=\"svgs-locked\">\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t{}\t\t\t\t\t\t</div>\n\n\t\t\t\t\t\t<div class=\"svgs-advanced\">\n\n",
        icon("lock", 17),
        h("Turn on Advanced Mode to unlock inline rendering, styling and animation options.")
    ));
    out.push_str(&format!(
        "\t\t\t\t\t\t\t<div class=\"svgs-row\">\n\t\t\t\t\t\t\t\t<div class=\"svgs-field-label\">{}</div>\n\t\t\t\t\t\t\t\t<input id=\"bodhi_svgs_settings[css_target]\" class=\"svgs-input\" name=\"bodhi_svgs_settings[css_target]\" type=\"text\" placeholder=\"style-svg\" value=\"{css_target}\" />\n\t\t\t\t\t\t\t\t<p class=\"svgs-hint\">\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t<code>style-svg</code>.\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t\t<div class=\"svgs-code\">\n\t\t\t\t\t\t\t\t\t<div class=\"svgs-code-bar\">\n\t\t\t\t\t\t\t\t\t\t<span>{}</span>\n\t\t\t\t\t\t\t\t\t\t<span class=\"svgs-code-lang\">html</span>\n\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t<pre>&lt;img class=\"<span class=\"svgs-tok\">style-svg</span>\" src=\"graphic.svg\" /&gt;</pre>\n\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t</div>\n\n\t\t\t\t\t\t\t",
        h("CSS class to target"),
        h("The default target class is"),
        h("Set your own class here, or leave blank to use the default. The plugin traverses any depth below an element carrying the class and replaces child IMG tags with SVG sources."),
        h("in your content")
    ));
    let skip_nested = settings.get("skip_nested_svg").is_some_and(|v| match v {
        Value::Null => false,
        // `1 == $value`.
        other => other.as_str().map_or_else(|| other.as_i64() == Some(1), |s| s.trim().parse::<f64>().is_ok_and(|n| n == 1.0)),
    });
    out.push_str(&toggle_row("skip_nested_svg", &t("Skip nested SVGs"), &t("Only inline the first SVG in a target container — useful for Gutenberg Cover blocks with nested SVG images."), skip_nested, "1"));
    out.push_str(&toggle_row("js_foot_choice", &t("Output JS in footer"), &t("Place the inline-rendering script before the closing body tag instead of in the head. Requires your theme to call wp_footer()."), isset("js_foot_choice"), ""));
    out.push_str(&toggle_row("use_vanilla_js", &t("Use vanilla JS"), &t("Load the dependency-free script instead of the jQuery version."), isset("use_vanilla_js"), ""));
    out.push_str(&toggle_row("use_expanded_js", &t("Use expanded JS"), &t("Load the readable, non-minified script — useful if a caching plugin minifies your scripts externally."), isset("use_expanded_js"), ""));
    out.push_str(&format!("\n\t\t\t\t\t\t\t<p class=\"svgs-subhead\">{}</p>\n\n\t\t\t\t\t\t\t", h("Legacy settings")));
    out.push_str(&toggle_row(
        "force_inline_svg",
        &t("Force inline SVG"),
        &t("<strong>Use with caution!</strong> Automatically adds the target class to ALL image tags with SVG sources, rendering every SVG on your site inline. Useful when a page builder won't let you add a CSS class to an image."),
        isset("force_inline_svg"),
        "",
    ));
    out.push_str(&toggle_row(
        "auto_insert_class",
        &t("Automatically insert class"),
        &t("(Classic Editor only) Inserts the target class into IMG tags when you embed an SVG in a post, and strips the default WordPress classes. Only affects SVG files."),
        isset("auto_insert_class"),
        "",
    ));
    out.push_str(&format!(
        "\n\t\t\t\t\t\t</div>\n\n\t\t\t\t\t</div>\n\t\t\t\t</section>\n\n\t\t\t\t<section class=\"svgs-panel\">\n\t\t\t\t\t<header class=\"svgs-panel-head\">\n\t\t\t\t\t\t<div>\n\t\t\t\t\t\t\t<h2>{}</h2>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t</header>\n\t\t\t\t\t<div class=\"svgs-panel-body\">\n\t\t\t\t\t\t<div class=\"svgs-danger-row\">\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t<div class=\"svgs-danger-text\">\n\t\t\t\t\t\t\t\t<span class=\"svgs-toggle-label\">{}</span>\n\t\t\t\t\t\t\t\t<span class=\"svgs-toggle-desc\">{}</span>\n\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t<label class=\"svgs-toggle-row\" for=\"bodhi_svgs_settings[del_plugin_data]\">\n\t\t\t\t\t\t\t\t<input id=\"bodhi_svgs_settings[del_plugin_data]\" name=\"bodhi_svgs_settings[del_plugin_data]\" type=\"checkbox\" class=\"svgs-switch-input\" aria-label=\"{}\" {} />\n\t\t\t\t\t\t\t\t<span class=\"svgs-switch\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t</div>\n\t\t\t\t</section>\n\n\t\t\t\t<p class=\"svgs-save-row\">\n\t\t\t\t\t<input class=\"svgs-btn svgs-btn-primary\" type=\"submit\" name=\"bodhi_svgs_settings_submit\" value=\"{}\" />\n\t\t\t\t</p>\n\n\t\t\t</form>\n\n\t\t</div> <!-- .svgs-main -->\n\n",
        h("Danger zone"),
        icon("triangle-alert", 20),
        h("Delete plugin data on uninstall"),
        h("Removes all settings and metadata when the plugin is deleted. This cannot be undone."),
        a("Delete plugin data on uninstall"),
        checked(isset("del_plugin_data")),
        a("Save changes")
    ));
    out.push_str(&sidebar(site, &icons));
    out.push_str("\t</div> <!-- .svgs-layout -->\n\n</div> <!-- .wrap.svgs-ds -->\n");
    out
}

/// The sidebar of the settings page: donations, tutorials, support and the partner.
fn sidebar(site: &Site, icons: &str) -> String {
    let h = |text: &str| esc_html(&t(site, text));
    let a = |text: &str| esc_attr(&t(site, text));
    let icon = |name: &str, size: u32| icon(icons, name, size);
    let qr = |coin: &str, file: &str, alt: &str, address: &str| {
        format!(
            "\t\t\t\t\t<div class=\"svgs-crypto-item\">\n\t\t\t\t\t\t<details class=\"svgs-qr\">\n\t\t\t\t\t\t\t<summary title=\"{}\">\n\t\t\t\t\t\t\t\t<strong>{coin}</strong>\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</summary>\n\t\t\t\t\t\t\t<span class=\"svgs-qr-box\"><img src=\"{}\" width=\"124\" height=\"124\" alt=\"{}\" /></span>\n\t\t\t\t\t\t</details>\n\t\t\t\t\t\t<span>{address}</span>\n\t\t\t\t\t</div>\n",
            a("Show QR code"),
            icon("qr-code", 14),
            esc_url(&plugin_url(site, &format!("admin/img/{file}"))),
            a(alt)
        )
    };
    let mut out = format!(
        "\t\t<aside class=\"svgs-sidebar\">\n\n\t\t\t<div class=\"svgs-card svgs-card-ink\">\n\t\t\t\t<div class=\"svgs-stat\">1,000,000+</div>\n\t\t\t\t<div class=\"svgs-stat-caption\">{}</div>\n\t\t\t\t<p>{}<br />{}</p>\n\t\t\t\t<a class=\"svgs-btn svgs-btn-accent svgs-btn-sm svgs-btn-block\" target=\"_blank\" href=\"https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&amp;hosted_button_id=Z9R7JERS82EQQ&amp;source=url\">\n\t\t\t\t\t{}\t\t\t\t\t{}\t\t\t\t</a>\n\t\t\t\t<div class=\"svgs-crypto\">\n",
        h("active installs and counting"),
        h("Maintained by one person since 2013."),
        h("If it's useful, a donation keeps it going."),
        icon("heart", 15),
        h("Donate via PayPal")
    );
    out.push_str(&qr("BTC", "qr-btc.svg", "Bitcoin donation address QR code", "1qF8r2HkTLifND7WLGfWmvxfXc9ze55DZ"));
    out.push_str(&qr("ETH", "qr-eth.svg", "Ethereum donation address QR code", "0x599695Eb51aFe2e5a0DAD60aD9c89Bc8f10B54f4"));
    out.push_str(&format!(
        "\t\t\t\t</div>\n\t\t\t</div>\n\n\t\t\t<div class=\"svgs-card\">\n\t\t\t\t<h3>{}{}</h3>\n\t\t\t\t<p>{}</p>\n\t\t\t\t<a class=\"svgs-btn svgs-btn-secondary svgs-btn-sm svgs-btn-block\" target=\"_blank\" href=\"https://svg.support/tutorials/\">\n\t\t\t\t\t{}\t\t\t\t\t{}\t\t\t\t</a>\n\t\t\t\t<p class=\"svgs-card-links\">\n\t\t\t\t\t<a target=\"_blank\" href=\"https://wordpress.org/plugins/svg-support/\">{}</a> · <a target=\"_blank\" href=\"https://github.com/benbodhi/svg-support\">{}</a> · <a target=\"_blank\" href=\"https://twitter.com/svgsupport\">@SVGSupport</a><br />\n\t\t\t\t\t&copy; <a target=\"_blank\" href=\"https://benbodhi.com/\">Benbodhi</a> · <a target=\"_blank\" href=\"https://twitter.com/benbodhi\">@benbodhi</a> · <a target=\"_blank\" href=\"https://farcaster.xyz/benbodhi\">Farcaster</a>\n\t\t\t\t</p>\n\t\t\t</div>\n\n",
        icon("sparkles", 16),
        h("Tutorials & tools"),
        h("Guides for inline rendering, styling and animating SVGs — plus handy tools for animating and optimizing your files — all on the new site."),
        h("Visit svg.support"),
        icon("arrow-up-right", 14),
        h("WordPress.org"),
        h("GitHub")
    ));
    out.push_str(&format!(
        "\t\t\t<div class=\"svgs-card\">\n\t\t\t\t<h3>{}{}</h3>\n\t\t\t\t<p>{}</p>\n\t\t\t\t<div class=\"svgs-btn-stack\">\n\t\t\t\t\t<a class=\"svgs-btn svgs-btn-secondary svgs-btn-sm svgs-btn-block\" target=\"_blank\" href=\"https://wordpress.org/support/plugin/svg-support/\">\n\t\t\t\t\t\t{}\t\t\t\t\t\t{}\t\t\t\t\t</a>\n\t\t\t\t\t<a class=\"svgs-btn svgs-btn-secondary svgs-btn-sm svgs-btn-block\" target=\"_blank\" href=\"https://wordpress.org/support/view/plugin-reviews/svg-support?filter=5#postform\">\n\t\t\t\t\t\t<span class=\"svgs-btn-stars\">{}</span>\n\t\t\t\t\t\t{}\t\t\t\t\t</a>\n\t\t\t\t</div>\n\t\t\t</div>\n\n",
        icon("life-buoy", 16),
        h("Support & reviews"),
        h("Need a hand? Support is handled personally through the WordPress.org forums. And if you're enjoying SVG Support, a quick five-star review means the world."),
        h("Get support"),
        icon("arrow-up-right", 14),
        (0..5).map(|_| icon("star-filled", 12)).collect::<String>(),
        h("Leave a review")
    ));
    out.push_str(&format!(
        "\t\t\t<div class=\"svgs-card\">\n\t\t\t\t<h3>{}</h3>\n\t\t\t\t<a target=\"_blank\" class=\"svgs-partner-logo\" href=\"https://shortpixel.com/h/af/OLKMLXE207471\"><img src=\"{}\" alt=\"{}\" /></a>\t\t\t\t<p>{}</p>\n\t\t\t\t<a class=\"svgs-btn svgs-btn-secondary svgs-btn-sm svgs-btn-block\" target=\"_blank\" href=\"https://shortpixel.com/h/af/OLKMLXE207471\">\n\t\t\t\t\t{}\t\t\t\t\t{}\t\t\t\t</a>\n\t\t\t</div>\n\n\t\t</aside>\n\n",
        h("Optimize your images"),
        esc_url(&plugin_url(site, "admin/img/shortpixel.png")),
        a("ShortPixel logo"),
        h("ShortPixel reduces the size of your JPG and PNG images with no visible quality loss — originals are kept in a backup folder. If you upgrade to a paid plan, I'll receive a small commission."),
        h("Try ShortPixel for free"),
        icon("external-link", 14)
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::Str(text.as_bytes().to_vec())
    }

    #[test]
    fn sanitizing_turns_off_what_was_not_sent() {
        let form = Value::Array(vec![(Key::Str(b"advanced_mode".to_vec()), s("on")), (Key::Str(b"css_target".to_vec()), s(" my <b>svg</b> "))]);
        let Some(Value::Array(items)) = sanitize(Some(form)) else { panic!() };
        let keys: Vec<String> = items.iter().map(|(k, _)| if let Key::Str(k) = k { String::from_utf8_lossy(k).into_owned() } else { String::new() }).collect();
        assert_eq!(keys, ["advanced_mode", "css_target", "sanitize_svg_front_end", "sanitize_on_upload_roles", "restrict"]);
        assert_eq!(items[1].1, s("my svg"));
        assert_eq!(items[2].1, Value::Bool(false));
        assert_eq!(items[4].1, Value::Array(vec![(Key::Int(0), s("none"))]));
    }
}
