//! `wp-admin/widgets.php` with `widgets-form-blocks.php`: the block editor of the widget areas of a classic theme. The
//! editor itself is WordPress' JavaScript; this module starts it with the same data.

use rustpress_core::formatting::{esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use serde_json::{Map, Value, json};

use super::block_editor::{EditorPage, SettingsFor, editor_head_styles, editor_settings, inline_json, preload_paths, shared_inline};
use super::{AdminResponse, Ctx, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};

/// The instance schemas of the media widgets, baked from the classes (`get_instance_schema()`), and the templates of
/// `render_control_template_scripts()`.
const SCHEMAS: &str = include_str!("../data/widgets_schemas.json");
const TEMPLATES: &str = include_str!("../data/widgets_templates.html");

/// `wp_json_encode( $value, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )` of a plain text.
fn json_text(text: &str) -> String {
    inline_json(&json!(text))
}

/// `_n_noop()`: the same text for one and for many.
fn noop(text: &str) -> Value {
    json!({"0": text, "1": text, "singular": text, "plural": text, "context": null, "domain": null})
}

/// `WP_Widget_Media::$l10n` of a media widget.
fn media_l10n(ctx: &Ctx<'_>, base: &str) -> Value {
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    let trx = |text: &str, context: &str| site.trx(text, context);
    let library = esc_url(&ctx.admin_url("upload.php"));
    let missing = |text: &str| sprintf(&tr(text), &[&library]);
    let short = "; should preferably not be longer than ~13 characters long";
    // The labels of a kind of media.
    let labels = |kind: &str, name: &str, missing_text: &str, state: &str, unsupported: String| {
        let context = format!("label for button in the {kind} widget");
        let long_context = format!("{context}{short}");
        let mut map = Map::new();
        map.insert("no_media_selected".into(), json!(tr(&format!("No {kind} selected"))));
        map.insert("add_media".into(), json!(trx(&format!("Add {name}"), &context)));
        map.insert("replace_media".into(), json!(trx(&format!("Replace {name}"), &long_context)));
        map.insert("edit_media".into(), json!(trx(&format!("Edit {name}"), &long_context)));
        map.insert("add_to_widget".into(), json!(tr("Add to Widget")));
        map.insert("missing_attachment".into(), json!(missing(missing_text)));
        map.insert("media_library_state_multi".into(), noop(&format!("{state} (%d)")));
        map.insert("media_library_state_single".into(), json!(tr(state)));
        map.insert("unsupported_file_type".into(), json!(unsupported));
        Value::Object(map)
    };
    match base {
        "media_audio" => labels("audio", "Audio", "That audio file cannot be found. Check your <a href=\"%s\">media library</a> and make sure it was not deleted.", "Audio Widget", tr("Looks like this is not the correct kind of file. Please link to an audio file instead.")),
        "media_image" => labels("image", "Image", "That image cannot be found. Check your <a href=\"%s\">media library</a> and make sure it was not deleted.", "Image Widget", tr("Looks like this is not the correct kind of file. Please link to an appropriate file instead.")),
        "media_video" => {
            let extensions = "<code>.mp4</code>, <code>.m4v</code>, <code>.webm</code>, <code>.ogv</code>, <code>.flv</code>";
            labels("video", "Video", "That video cannot be found. Check your <a href=\"%s\">media library</a> and make sure it was not deleted.", "Video Widget", sprintf(&tr("Sorry, the video at the supplied URL cannot be loaded. Please check that the URL is for a supported video file (%s) or stream (e.g. YouTube and Vimeo)."), &[extensions]))
        }
        _ => {
            let mut map = Map::new();
            map.insert("no_media_selected".into(), json!(tr("No images selected")));
            map.insert("add_media".into(), json!(trx("Add Images", "label for button in the gallery widget; should not be longer than ~13 characters long")));
            map.insert("replace_media".into(), json!(""));
            map.insert("edit_media".into(), json!(trx("Edit Gallery", "label for button in the gallery widget; should not be longer than ~13 characters long")));
            map.insert("add_to_widget".into(), json!(tr("Add to Widget")));
            map.insert("missing_attachment".into(), json!(missing("That file cannot be found. Check your <a href=\"%s\">media library</a> and make sure it was not deleted.")));
            map.insert("media_library_state_multi".into(), noop("Media Widget (%d)"));
            map.insert("media_library_state_single".into(), json!(tr("Media Widget")));
            map.insert("unsupported_file_type".into(), json!(tr("Looks like this is not the correct kind of file. Please link to an appropriate file instead.")));
            Value::Object(map)
        }
    }
}

/// `enqueue_admin_scripts()` of a media widget.
fn enqueue_media_widget(ctx: &Ctx<'_>, assets: &mut Assets, base: &str, handle: &str, mime: &str) {
    let mut schema: Value = serde_json::from_str(SCHEMAS).ok().and_then(|mut all: Value| all.get_mut(base).map(Value::take)).unwrap_or(Value::Null);
    if let Some(sizes) = schema.get_mut("size").and_then(|size| size.get_mut("enum")) {
        // `get_intermediate_image_sizes()` and the two of the widget.
        let mut names: Vec<String> = rustpress_content::media::registered_sizes(&ctx.site().options).into_iter().map(|(name, ..)| name).collect();
        names.extend(["full".to_owned(), "custom".to_owned()]);
        *sizes = json!(names);
    }
    assets.late_scripts.push(handle.to_owned());
    assets.inline.push((handle.to_owned(), format!("wp.mediaWidgets.modelConstructors[ {} ].prototype.schema = {};", json_text(base), inline_json(&schema)), "after".into()));
    let name = json_text(base);
    let l10n = inline_json(&media_l10n(ctx, base));
    let extend = if base == "media_gallery" {
        format!("_.extend( wp.mediaWidgets.controlConstructors[ {name} ].prototype.l10n, {l10n} );")
    } else {
        format!("wp.mediaWidgets.controlConstructors[ {name} ].prototype.l10n = _.extend( {{}}, wp.mediaWidgets.controlConstructors[ {name} ].prototype.l10n, {l10n} );")
    };
    assets.inline.push((handle.to_owned(), format!("\n\t\t\t\t\twp.mediaWidgets.controlConstructors[ {name} ].prototype.mime_type = {};\n\t\t\t\t\t{extend}\n\t\t\t\t", json_text(mime)), "after".into()));
}

/// `WP_Widget_Custom_HTML::add_help_text()`.
fn help(ctx: &Ctx<'_>, syntax_highlighting: bool) -> Help {
    let tr = |text: &str| ctx.tr(text);
    let mut content = format!("<p>{}</p>", tr("Use the Custom HTML widget to add arbitrary HTML code to your widget areas."));
    if syntax_highlighting {
        content.push_str(&format!(
            "<p>{}</p>",
            sprintf(
                &tr("The edit field automatically highlights code syntax. You can disable this in your <a href=\"%1$s\" %2$s>user profile%3$s</a> to work in plain text mode."),
                &[&esc_url(&ctx.admin_url("profile.php")), "class=\"external-link\" target=\"_blank\"", &format!("<span class=\"screen-reader-text\"> {}</span>", tr("(opens in a new tab)"))]
            )
        ));
        content.push_str(&format!("<p id=\"editor-keyboard-trap-help-1\">{}</p>", tr("When using a keyboard to navigate:")));
        content.push_str(&format!(
            "<ul><li id=\"editor-keyboard-trap-help-2\">{}</li><li id=\"editor-keyboard-trap-help-3\">{}</li><li id=\"editor-keyboard-trap-help-4\">{}</li></ul>",
            tr("In the editing area, the Tab key enters a tab character."),
            tr("To move away from this area, press the Esc key followed by the Tab key."),
            tr("Screen reader users: when in forms mode, you may need to press the Esc key twice.")
        ));
    }
    Help { tabs: vec![HelpTab { id: "custom_html_widget".into(), title: tr("Custom HTML Widget"), content }], sidebar: String::new() }
}

/// `render_control_template_scripts()` of the media, text and custom HTML widgets, as a user who dismissed some pointers
/// sees them.
fn control_templates(ctx: &Ctx<'_>, dismissed: &str) -> String {
    let mut out = TEMPLATES.replace("http://localhost:8080/wp-admin/", &ctx.admin_url("")).replace("http://localhost:8080", ctx.site().settings.siteurl.trim_end_matches('/'));
    // The block of a pointer ends where the block after it starts.
    let dismissed: Vec<&str> = dismissed.split(',').collect();
    for (pointer, class) in [("text_widget_custom_html", "custom-html-widget-pointer"), ("text_widget_paste_html", "paste-html-pointer")] {
        if !dismissed.contains(&pointer) {
            continue;
        }
        let Some(at) = out.find(&format!("<div hidden class=\"wp-pointer {class} wp-pointer-top\">")) else { continue };
        let start = out[..at].rfind('\n').map_or(at, |i| i + 1);
        let Some(end) = out[at..].find("<div class=\"wp-pointer-arrow-inner\"></div>\n\t\t\t\t\t</div>\n\t\t\t\t</div>\n").map(|i| at + i + "<div class=\"wp-pointer-arrow-inner\"></div>\n\t\t\t\t\t</div>\n\t\t\t\t</div>\n".len()) else { continue };
        out.replace_range(start..end, "");
    }
    super::template_text::translate(ctx.site(), &out)
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let env = ctx.env;
    let paths = vec![
        ("OPTIONS".to_owned(), "/wp/v2/media".to_owned()),
        ("GET".to_owned(), "/wp/v2/widget-types?context=edit&per_page=-1".to_owned()),
        ("GET".to_owned(), "/wp/v2/sidebars?context=edit&per_page=-1".to_owned()),
        ("GET".to_owned(), "/wp/v2/widgets?context=edit&per_page=-1&_embed=about".to_owned()),
    ];
    let preloaded = preload_paths(ctx, paths).await;
    let info = super::posts::TypeInfo { name: String::new(), hierarchical: false, label: String::new(), singular: String::new(), labels: super::posts::ListLabels::defaults(ctx, false) };
    let settings = editor_settings(ctx, &rustpress_db::Post::default(), &info, SettingsFor::Widgets).await;

    let mut assets = Assets {
        editor_modules: true,
        scripts: ["wp-edit-widgets", "admin-widgets", "wp-block-styles", "wp-format-library"].iter().map(|s| (*s).to_owned()).collect(),
        styles: ["wp-edit-widgets", "wp-format-library"].iter().map(|s| (*s).to_owned()).collect(),
        ..Default::default()
    };
    super::media_ui::enqueue(ctx, &mut assets, 0).await;
    // The widgets enqueue on `admin_print_scripts-widgets.php`: their scripts still reach the head, their styles the footer.
    let media_styles: Vec<String> = assets.styles.iter().filter(|style| !matches!(style.as_str(), "wp-edit-widgets" | "wp-format-library")).cloned().collect();
    assets.styles.retain(|style| !media_styles.contains(style));
    assets.post_header_styles = media_styles;
    assets.post_header_styles.insert(1, "editor-buttons".to_owned());
    assets.post_header_styles.push("code-editor".to_owned());
    // `media-editor` is what is left of the media scripts by then: the plugins' scripts come before it.
    let media: Vec<String> = assets.scripts.iter().filter(|script| matches!(script.as_str(), "media-editor" | "media-audiovideo" | "mce-view" | "image-edit" | "imgareaselect")).cloned().collect();
    assets.scripts.retain(|script| !media.contains(script));
    assets.scripts.push("media-views".into());
    assets.late_scripts.extend(media);
    // `wp_enqueue_editor()`.
    assets.late_scripts.extend(["quicktags", "wplink", "jquery-ui-autocomplete", "thickbox", "media-upload", "media-widgets"].map(str::to_owned));
    enqueue_media_widget(ctx, &mut assets, "media_audio", "media-audio-widget", "audio");
    enqueue_media_widget(ctx, &mut assets, "media_image", "media-image-widget", "image");
    enqueue_media_widget(ctx, &mut assets, "media_gallery", "media-gallery-widget", "image");
    enqueue_media_widget(ctx, &mut assets, "media_video", "media-video-widget", "video");
    assets.late_scripts.push("text-widgets".into());
    assets.inline.push(("text-widgets".into(), "wp.textWidgets.idBases.push( \"text\" );".into(), "after".into()));
    assets.inline.push(("text-widgets".into(), "wp.textWidgets.init();".into(), "after".into()));
    // `WP_Widget_Custom_HTML::enqueue_admin_scripts()`.
    let syntax_highlighting = env.db.user_meta(ctx.user.id, "syntax_highlighting").await.ok().flatten().as_deref() != Some("false");
    let code = if syntax_highlighting {
        let mut code = super::file_editor::code_editor_settings(ctx, "widget.html");
        code["codemirror"]["indentUnit"] = json!(2);
        code["codemirror"]["tabSize"] = json!(2);
        assets.late_scripts.extend(["code-editor".to_owned(), "htmlhint".to_owned(), "csslint".to_owned(), "custom-html-widgets".to_owned()]);
        assets.inline.push(("code-editor".into(), format!("jQuery.extend( wp.codeEditor.defaultSettings, {} );", inline_json(&code)), "after".into()));
        code
    } else {
        assets.late_scripts.push("custom-html-widgets".into());
        json!({"disabled": true})
    };
    assets.inline.push(("custom-html-widgets".into(), "wp.customHtmlWidgets.idBases.push( \"custom_html\" );".into(), "after".into()));
    assets.inline.push(("custom-html-widgets".into(), format!("wp.customHtmlWidgets.init( {} );", inline_json(&code)), "after".into()));
    let error_notice = |n: i64| ctx.site().trn("There is %d error which must be fixed before you can save.", "There are %d errors which must be fixed before you can save.", n);
    assets.inline.push(("custom-html-widgets".into(), format!("jQuery.extend( wp.customHtmlWidgets.l10n, {} );", inline_json(&json!({"errorNotice": {"singular": error_notice(1), "plural": error_notice(2)}}))), "after".into()));

    // `enqueue_block_editor_assets` of the plugins.
    {
        let parts = ctx.plugin_parts();
        let mut plugin_ctx = ctx.plugin_context(&parts, "widgets.php");
        plugin_ctx.block_editor = true;
        for script in ctx.site().plugins.block_editor_assets(&plugin_ctx).scripts {
            if script.deps.iter().any(|dep| dep == "wp-edit-post") {
                assets.route_first = true;
            }
            assets.scripts.push(script.handle.clone());
            assets.plugin_page_scripts.push(script);
        }
    }
    editor_head_styles(ctx, &mut assets);
    assets.inline.push(("wp-block-styles".into(), super::block_editor::block_styles_script(ctx), "after".into()));
    shared_inline(ctx, &mut assets, &preloaded, EditorPage::Site).await;
    // The TinyMCE settings of the classic block are for the post editor.
    assets.inline.retain(|(handle, _, position)| !(handle == "wp-block-library" && position == "before"));
    assets.inline.push(("wp-edit-widgets".into(), format!("wp.domReady( function() {{\n\t\t\twp.editWidgets.initialize( \"widgets-editor\", {} );\n\t\t}} );", inline_json(&settings)), "after".into()));

    let rich_editing = env.db.user_meta(ctx.user.id, "rich_editing").await.ok().flatten();
    assets.footer = super::editor::default_editor_scripts(ctx, super::editor::can_richedit(ctx, rich_editing.as_deref()));
    let dismissed = env.db.user_meta(ctx.user.id, "dismissed_wp_pointers").await.ok().flatten().unwrap_or_default();
    // The output before ends with the indentation of its closing `<?php`, and so do the templates.
    assets.footer.push_str(&control_templates(ctx, &dismissed));

    // An installed Classic Widgets gets a link to activate it, otherwise one to install it.
    let wp_root = env.loader.includes.parent().unwrap_or(&env.loader.includes);
    let message = if wp_root.join("wp-content/plugins/classic-widgets/classic-widgets.php").is_file() {
        let url = format!("plugins.php?action=activate&amp;plugin=classic-widgets/classic-widgets.php&amp;_wpnonce={}", ctx.create_nonce("activate-plugin_classic-widgets/classic-widgets.php"));
        sprintf(&tr("The block widgets require JavaScript. Please enable JavaScript in your browser settings, or activate the <a href=\"%s\">Classic Widgets plugin</a>."), &[&esc_url(&url).replace("&#038;", "&amp;")])
    } else {
        let url = format!("{}&_wpnonce={}", ctx.admin_url("update.php?action=install-plugin&plugin=classic-widgets"), ctx.create_nonce("install-plugin_classic-widgets"));
        sprintf(&tr("The block widgets require JavaScript. Please enable JavaScript in your browser settings, or install the <a href=\"%s\">Classic Widgets plugin</a>."), &[&esc_url(&url).replace("&#038;", "&amp;")])
    };
    let content = format!(
        "<div id=\"widgets-editor\" class=\"blocks-widgets-container\">\n\t\t<div class=\"wrap hide-if-js widgets-editor-no-js\">\n\t\t<h1 class=\"wp-heading-inline\">{}</h1>\n\t\t<div class=\"notice notice-error hide-if-js\"><p>{message}</p></div>\t</div>\n</div>\n\n",
        esc_html(&tr("Widgets"))
    );
    let screen = Screen {
        file: "widgets.php".into(),
        parent_file: "themes.php".into(),
        title: tr("Widgets"),
        help: Some(help(ctx, syntax_highlighting)),
        body_classes: vec!["block-editor-page".into(), "wp-embed-responsive".into()],
        block_editor: true,
        ..Default::default()
    };
    render(ctx, &screen, &content, &assets).await
}
