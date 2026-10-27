//! `wp_editor()`: the classic editor of posts: TinyMCE (Visual) and Quicktags (Code).

use rustpress_core::formatting::esc_attr;

use super::Ctx;
use crate::registry::default_scripts;

const MCE_INIT: &str = include_str!("../tinymce_init.tpl");

/// What the editor adds to a screen.
pub struct Editor {
    /// The editor wrapper with its tools and the textarea.
    pub html: String,
    /// `tinyMCEPreInit`: printed before the scripts.
    pub preinit: String,
    /// Starts TinyMCE and Quicktags: printed after the scripts.
    pub init: String,
    /// Scripts the editor needs.
    pub scripts: Vec<String>,
    pub rich: bool,
}

/// `user_can_richedit()`: the browser can edit rich text and the user did not turn it off.
pub(crate) fn can_richedit(ctx: &Ctx<'_>, rich_editing: Option<&str>) -> bool {
    if rich_editing != Some("true") {
        return false;
    }
    let agent = ctx.request.user_agent.as_deref().unwrap_or_default();
    let mobile = ["Mobile", "Android", "Silk/", "Kindle", "BlackBerry", "Opera Mini", "Opera Mobi"].iter().any(|m| agent.contains(m));
    let ie = agent.contains("MSIE") || agent.contains("Trident/");
    let safari = agent.contains("Safari") && !agent.contains("Chrome") && !agent.contains("Chromium");
    if safari {
        return !mobile || agent.split("AppleWebKit/").nth(1).and_then(|r| r.split('.').next()).and_then(|v| v.parse::<u32>().ok()).is_some_and(|v| v >= 534);
    }
    !ie && (agent.contains("Gecko") || agent.contains("Chrome") || agent.contains("Edg/") || (agent.contains("Opera") && !mobile))
}

/// `get_user_setting( 'editor' )` from the `wp-settings-{id}` cookie.
fn user_setting(ctx: &Ctx<'_>, name: &str) -> Option<String> {
    let cookie = ctx.request.cookie(&format!("wp-settings-{}", ctx.user.id))?;
    cookie.split('&').find_map(|pair| pair.split_once('=').filter(|(k, _)| *k == name).map(|(_, v)| v.to_owned()))
}

/// `add_editor_style()` calls with literal file names in the theme's PHP files.
fn editor_styles(ctx: &Ctx<'_>) -> Vec<String> {
    let site = ctx.site();
    let Some(root) = ctx.env.loader.includes.parent() else { return Vec::new() };
    let themes = root.join("wp-content").join("themes");
    let mut out: Vec<String> = Vec::new();
    let mut slugs = vec![site.settings.template.clone()];
    if site.settings.stylesheet != site.settings.template {
        slugs.push(site.settings.stylesheet.clone());
    }
    for slug in slugs {
        let dir = themes.join(&slug);
        let mut files = vec![dir.join("functions.php")];
        if let Ok(entries) = std::fs::read_dir(dir.join("inc")) {
            files.extend(entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "php")));
        }
        for file in files {
            let Ok(text) = std::fs::read_to_string(&file) else { continue };
            let mut rest = text.as_str();
            while let Some(at) = rest.find("add_editor_style(") {
                rest = &rest[at + "add_editor_style(".len()..];
                let args = rest.split(';').next().unwrap_or_default();
                for quoted in args.split(['\'', '"']).skip(1).step_by(2) {
                    if quoted.is_empty() || quoted.contains('$') || quoted.contains('(') {
                        continue;
                    }
                    let url = if quoted.starts_with("http") { quoted.to_owned() } else { format!("{}/wp-content/themes/{slug}/{}", ctx.env.loader.site_url, quoted.trim_start_matches('/')) };
                    if !out.contains(&url) {
                        out.push(url);
                    }
                }
            }
        }
    }
    out
}

/// `wp_editor( $content, 'content' )` for the post body.
pub fn classic_editor(ctx: &Ctx<'_>, content: &str, post_type: &str, post_status: &str, rich_editing: Option<&str>) -> Editor {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let rich = can_richedit(ctx, rich_editing);
    let default_editor = if rich { user_setting(ctx, "editor").filter(|e| e == "html" || e == "tinymce").unwrap_or_else(|| "tinymce".to_owned()) } else { "html".to_owned() };
    let tmce_active = rich && default_editor == "tinymce";
    let site_url = &ctx.env.loader.site_url;
    let version = {
        let scripts = default_scripts(&ctx.env.loader);
        scripts.deps.get("wp-tinymce-root").map(|s| match &s.ver {
            crate::deps::Ver::Value(v) => v.clone(),
            _ => String::new(),
        })
    }
    .unwrap_or_default();

    let escaped = if tmce_active { content.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") } else { super::post_edit::esc_textarea_pub(content) };
    let mut tools = String::from("<div id=\"wp-content-editor-tools\" class=\"wp-editor-tools hide-if-no-js\"><div class=\"wp-editor-tabs\">");
    if rich {
        tools.push_str(&format!(
            "<button type=\"button\" id=\"content-tmce\"{} class=\"wp-switch-editor switch-tmce\" data-wp-editor-id=\"content\">{}</button>\n<button type=\"button\" id=\"content-html\"{} class=\"wp-switch-editor switch-html\" data-wp-editor-id=\"content\">{}</button>\n",
            if tmce_active { " aria-pressed=\"true\"" } else { "" },
            site.trx("Visual", "Name for the Visual editor tab"),
            if tmce_active { "" } else { " aria-pressed=\"true\"" },
            site.trx("Code", "Name for the Code editor tab (formerly Text)")
        ));
    }
    tools.push_str("</div>\n</div>\n");
    let html = format!(
        "\t<div id=\"postdivrich\" class=\"postarea\">\n\t<div id=\"wp-content-wrap\" class=\"wp-core-ui wp-editor-wrap {}\"><link rel='stylesheet' id='editor-buttons-css' href='{site_url}/wp-includes/css/editor.min.css?ver={}' media='all' />\n{tools}<div id=\"wp-content-editor-container\" class=\"wp-editor-container\"><div id=\"ed_toolbar\" class=\"quicktags-toolbar hide-if-no-js\"></div><textarea class=\"wp-editor-area\" style=\"height: 300px\"{} cols=\"40\" name=\"content\" id=\"content\">{escaped}</textarea></div>\n</div>\n",
        if tmce_active { "tmce-active" } else { "html-active" },
        ctx.env.loader.version,
        if rich { " autocomplete=\"off\"" } else { "" }
    );

    let qt = "'content':{id:\"content\",buttons:\"strong,em,link,block,del,ins,img,ul,ol,li,code,more,close\"}";
    let mut mce_init = String::from("{}");
    let mut plugins = String::new();
    if rich {
        let mut css = vec![format!("{site_url}/wp-includes/css/dashicons.min.css?ver={}", ctx.env.loader.version), format!("{site_url}/wp-includes/js/tinymce/skins/wordpress/wp-content.css?ver={}", ctx.env.loader.version)];
        css.extend(editor_styles(ctx));
        let body_class = format!(
            "content post-type-{post_type} post-status-{post_status} post-format-standard page-template-default locale-{}",
            rustpress_core::formatting::sanitize_html_class(&site.locale().replace('_', "-").to_lowercase())
        );
        let lang = site.locale().split('_').next().unwrap_or("en").to_owned();
        mce_init = MCE_INIT
            .replace("{SITE}", site_url)
            .replace("{TINYMCE_VERSION}", &version)
            .replace("{CONTENT_CSS}", &css.join(","))
            .replace("{BODY_CLASS}", &body_class)
            .replace("{LANG}", &lang)
            .replace("{LANG_ATTR}", &site.language_tag());
        plugins = "charmap,colorpicker,hr,lists,media,paste,tabfocus,textcolor,fullscreen,wordpress,wpautoresize,wpeditimage,wpemoji,wpgallery,wplink,wpdialogs,wptextpattern,wpview".to_owned();
    }
    // `editor_js()`: the settings, then (with TinyMCE) its scripts, then the start of the editors.
    let preinit = format!(
        "\n\t\t<script>\n\t\ttinyMCEPreInit = {{\n\t\t\tbaseURL: \"{site_url}/wp-includes/js/tinymce\",\n\t\t\tsuffix: \".min\",\n\t\t\tdragDropUpload: true,\t\t\tmceInit: {mce_init},\n\t\t\tqtInit: {{{qt}}},\n\t\t\tref: {{plugins:\"{plugins}\",theme:\"modern\",language:\"{}\"}},\n\t\t\tload_ext: function(url,lang){{var sl=tinymce.ScriptLoader;sl.markDone(url+'/langs/'+lang+'.js');sl.markDone(url+'/langs/'+lang+'_dlg.js');}}\n\t\t}};\n\t\t</script>\n\t\t",
        if rich { site.locale().split('_').next().unwrap_or("en").to_owned() } else { String::new() }
    );
    let init = String::from(
        "\t\t<script>\n\t\t\n\t\t( function() {\n\t\t\tvar initialized = [];\n\t\t\tvar initialize  = function() {\n\t\t\t\tvar init, id, inPostbox, $wrap;\n\t\t\t\tvar readyState = document.readyState;\n\n\t\t\t\tif ( readyState !== 'complete' && readyState !== 'interactive' ) {\n\t\t\t\t\treturn;\n\t\t\t\t}\n\n\t\t\t\tfor ( id in tinyMCEPreInit.mceInit ) {\n\t\t\t\t\tif ( initialized.indexOf( id ) > -1 ) {\n\t\t\t\t\t\tcontinue;\n\t\t\t\t\t}\n\n\t\t\t\t\tinit      = tinyMCEPreInit.mceInit[id];\n\t\t\t\t\t$wrap     = tinymce.$( '#wp-' + id + '-wrap' );\n\t\t\t\t\tinPostbox = $wrap.parents( '.postbox' ).length > 0;\n\n\t\t\t\t\tif (\n\t\t\t\t\t\t! init.wp_skip_init &&\n\t\t\t\t\t\t( $wrap.hasClass( 'tmce-active' ) || ! tinyMCEPreInit.qtInit.hasOwnProperty( id ) ) &&\n\t\t\t\t\t\t( readyState === 'complete' || ( ! inPostbox && readyState === 'interactive' ) )\n\t\t\t\t\t) {\n\t\t\t\t\t\ttinymce.init( init );\n\t\t\t\t\t\tinitialized.push( id );\n\n\t\t\t\t\t\tif ( ! window.wpActiveEditor ) {\n\t\t\t\t\t\t\twindow.wpActiveEditor = id;\n\t\t\t\t\t\t}\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\n\t\t\tif ( typeof tinymce !== 'undefined' ) {\n\t\t\t\tif ( tinymce.Env.ie && tinymce.Env.ie < 11 ) {\n\t\t\t\t\ttinymce.$( '.wp-editor-wrap ' ).removeClass( 'tmce-active' ).addClass( 'html-active' );\n\t\t\t\t} else {\n\t\t\t\t\tif ( document.readyState === 'complete' ) {\n\t\t\t\t\t\tinitialize();\n\t\t\t\t\t} else {\n\t\t\t\t\t\tdocument.addEventListener( 'readystatechange', initialize );\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\n\t\t\tif ( typeof quicktags !== 'undefined' ) {\n\t\t\t\tfor ( id in tinyMCEPreInit.qtInit ) {\n\t\t\t\t\tquicktags( tinyMCEPreInit.qtInit[id] );\n\n\t\t\t\t\tif ( ! window.wpActiveEditor ) {\n\t\t\t\t\t\twindow.wpActiveEditor = id;\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}());\n\t\t</script>\n",
    );
    // The quicktags have a link button: `editor_js()` prints the link dialog after the script.
    let init = format!("{init}\t\t{}", link_dialog(ctx));
    let mut scripts = vec!["quicktags".to_owned(), "editor".to_owned(), "wplink".to_owned()];
    if rich {
        scripts.push("wp-tinymce".to_owned());
    }
    let _ = (esc_attr(""), tr(""));
    Editor { html, preinit, init, scripts, rich }
}

/// `_WP_Editors::print_default_editor_scripts()` of `wp_enqueue_editor()`: the settings `wp.editor.initialize()`
/// starts with, then the link dialog.
pub fn default_editor_scripts(ctx: &Ctx<'_>, rich: bool) -> String {
    let site = ctx.site();
    let site_url = &ctx.env.loader.site_url;
    let version = &ctx.env.loader.version;
    let settings = if rich {
        // `default_settings()` with the toolbar and the plugins of the default editor (the same keys as the post
        // editor's, without the parts the post editor adds).
        let lang = site.locale().split('_').next().unwrap_or("en").to_owned();
        let common = MCE_INIT
            .split_once("{'content':{")
            .and_then(|(_, rest)| rest.split_once(",content_css:"))
            .map(|(common, _)| common.replace("resize:false", "resize:\"vertical\""))
            .unwrap_or_default()
            .replace("{LANG}", &lang)
            .replace("{LANG_ATTR}", &site.language_tag())
            .replace("{TINYMCE_VERSION}", &tinymce_version(ctx));
        format!(
            "{{{common},content_css:\"{site_url}/wp-includes/css/dashicons.min.css?ver={version},{site_url}/wp-includes/js/tinymce/skins/wordpress/wp-content.css?ver={version}\",toolbar1:\"bold,italic,bullist,numlist,link\",wpautop:false,indent:true,elementpath:false,plugins:\"charmap,colorpicker,hr,lists,paste,tabfocus,textcolor,fullscreen,wordpress,wpautoresize,wpeditimage,wpemoji,wpgallery,wplink,wptextpattern\"}}"
        )
    } else {
        "{}".to_owned()
    };
    let mut out = format!(
        "\t\t<script>\n\t\twindow.wp = window.wp || {{}};\n\t\twindow.wp.editor = window.wp.editor || {{}};\n\t\twindow.wp.editor.getDefaultSettings = function() {{\n\t\t\treturn {{\n\t\t\t\ttinymce: {settings},\n\t\t\t\tquicktags: {{\n\t\t\t\t\tbuttons: 'strong,em,link,ul,ol,li,code'\n\t\t\t\t}}\n\t\t\t}};\n\t\t}};\n\n\t\t"
    );
    if rich {
        out.push_str(&format!(
            "\t\t\tvar tinyMCEPreInit = {{\n\t\t\t\tbaseURL: \"{site_url}/wp-includes/js/tinymce\",\n\t\t\t\tsuffix: \".min\",\n\t\t\t\tmceInit: {{}},\n\t\t\t\tqtInit: {{}},\n\t\t\t\tload_ext: function(url,lang){{var sl=tinymce.ScriptLoader;sl.markDone(url+'/langs/'+lang+'.js');sl.markDone(url+'/langs/'+lang+'_dlg.js');}}\n\t\t\t}};\n\t\t\t"
        ));
    }
    out.push_str("\t\t</script>\n\t\t");
    out.push_str(&link_dialog(ctx));
    out
}

/// The version TinyMCE's files are cached with (`$tinymce_version`, the version of `wp-tinymce-root`).
fn tinymce_version(ctx: &Ctx<'_>) -> String {
    default_scripts(&ctx.env.loader)
        .deps
        .get("wp-tinymce-root")
        .map(|s| match &s.ver {
            crate::deps::Ver::Value(v) => v.clone(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

/// `_WP_Editors::wp_link_dialog()`.
pub fn link_dialog(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    format!(
        "\t\t<div id=\"wp-link-backdrop\" style=\"display: none\"></div>\n\t\t<div id=\"wp-link-wrap\" class=\"wp-core-ui\" style=\"display: none\" role=\"dialog\" aria-modal=\"true\" aria-labelledby=\"link-modal-title\">\n\t\t<form id=\"wp-link\" tabindex=\"-1\">\n\t\t<input type=\"hidden\" id=\"_ajax_linking_nonce\" name=\"_ajax_linking_nonce\" value=\"{}\" />\t\t<h1 id=\"link-modal-title\">{}</h1>\n\t\t<button type=\"button\" id=\"wp-link-close\"><span class=\"screen-reader-text\">\n\t\t\t{}\t\t</span></button>\n\t\t<div id=\"link-selector\">\n\t\t\t<div id=\"link-options\">\n\t\t\t\t<p class=\"howto\" id=\"wplink-enter-url\">{}</p>\n\t\t\t\t<div>\n\t\t\t\t\t<label><span>{}</span>\n\t\t\t\t\t<input id=\"wp-link-url\" type=\"text\" aria-describedby=\"wplink-enter-url\" /></label>\n\t\t\t\t</div>\n\t\t\t\t<div class=\"wp-link-text-field\">\n\t\t\t\t\t<label><span>{}</span>\n\t\t\t\t\t<input id=\"wp-link-text\" type=\"text\" /></label>\n\t\t\t\t</div>\n\t\t\t\t<div class=\"link-target\">\n\t\t\t\t\t<label><span></span>\n\t\t\t\t\t<input type=\"checkbox\" id=\"wp-link-target\" /> {}</label>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t<p class=\"howto\" id=\"wplink-link-existing-content\">{}</p>\n\t\t\t<div id=\"search-panel\">\n\t\t\t\t<div class=\"link-search-wrapper\">\n\t\t\t\t\t<label>\n\t\t\t\t\t\t<span class=\"search-label\">{}</span>\n\t\t\t\t\t\t<input type=\"search\" id=\"wp-link-search\" class=\"link-search-field\" autocomplete=\"off\" aria-describedby=\"wplink-link-existing-content\" />\n\t\t\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t\t</label>\n\t\t\t\t</div>\n\t\t\t\t<div id=\"search-results\" class=\"query-results\" tabindex=\"0\">\n\t\t\t\t\t<ul></ul>\n\t\t\t\t\t<div class=\"river-waiting\">\n\t\t\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t\t</div>\n\t\t\t\t</div>\n\t\t\t\t<div id=\"most-recent-results\" class=\"query-results\" tabindex=\"0\">\n\t\t\t\t\t<div class=\"query-notice\" id=\"query-notice-message\">\n\t\t\t\t\t\t<em class=\"query-notice-default\">{}</em>\n\t\t\t\t\t\t<em class=\"query-notice-hint screen-reader-text\">\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</em>\n\t\t\t\t\t</div>\n\t\t\t\t\t<ul></ul>\n\t\t\t\t\t<div class=\"river-waiting\">\n\t\t\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t\t</div>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t</div>\n\t\t<div class=\"submitbox\">\n\t\t\t<div id=\"wp-link-cancel\">\n\t\t\t\t<button type=\"button\" class=\"button\">{}</button>\n\t\t\t</div>\n\t\t\t<div id=\"wp-link-update\">\n\t\t\t\t<input type=\"submit\" value=\"{}\" class=\"button button-primary\" id=\"wp-link-submit\" name=\"wp-link-submit\">\n\t\t\t</div>\n\t\t</div>\n\t\t</form>\n\t\t</div>\n\t\t",
        ctx.create_nonce("internal-linking"),
        tr("Insert/edit link"),
        tr("Close"),
        tr("Enter the destination URL"),
        tr("URL"),
        tr("Link Text"),
        tr("Open link in a new tab"),
        tr("Or link to existing content"),
        tr("Search"),
        tr("No search term specified. Showing recent items."),
        tr("Search or use up and down arrow keys to select an item."),
        tr("Cancel"),
        esc_attr(&tr("Add Link"))
    )
}
