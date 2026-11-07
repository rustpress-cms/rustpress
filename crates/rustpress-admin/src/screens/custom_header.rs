//! `themes.php?page=custom-header`: `Custom_Image_Header` of `wp-admin/includes/class-custom-image-header.php`, the
//! header screen of a classic theme that supports `custom-header`.

use rustpress_blocks::env::ThemeAccess;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::add_query_arg;
use rustpress_phpser::{Key, Value};
use rustpress_theme::classic::custom_header::{self as header, DefaultHeader, RandomHeader, Support, Uploaded};

use rustpress_core::OrLog;

use super::meta_boxes::submit_button;
use crate::content::{Content, PostData};
use crate::media;
use super::notices::notice;
use super::{AdminResponse, Ctx, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};

/// What the screen reads: the support of the theme, its headers and the theme mods.
struct State {
    support: Support,
    defaults: Vec<DefaultHeader>,
    uploaded: Vec<Uploaded>,
    mods_name: String,
    mods: Value,
    /// What `_get_random_header_data()` picked for the request.
    random: Option<RandomHeader>,
}

impl State {
    fn header_image(&self, ctx: &Ctx<'_>) -> Option<String> {
        header::header_image(ctx.site(), &self.mods, &self.support, self.random.as_ref())
    }

    /// `set_theme_mod()`.
    fn set_mod(&mut self, name: &str, value: Value) {
        let key = Key::Str(name.as_bytes().to_vec());
        let Value::Array(items) = &mut self.mods else {
            self.mods = Value::Array(vec![(key, value)]);
            return;
        };
        match items.iter_mut().find(|(k, _)| *k == key) {
            Some((_, slot)) => *slot = value,
            None => items.push((key, value)),
        }
    }

    /// `remove_theme_mod()`.
    fn remove_mod(&mut self, name: &str) {
        let key = Key::Str(name.as_bytes().to_vec());
        if let Value::Array(items) = &mut self.mods {
            items.retain(|(k, _)| *k != key);
        }
    }

    /// `Custom_Image_Header::set_header_image()` for a choice of the form.
    fn set_header_image(&mut self, choice: &str) {
        let text = |s: &str| Value::Str(s.as_bytes().to_vec());
        if matches!(choice, "remove-header" | "random-default-image" | "random-uploaded-image") {
            self.set_mod("header_image", text(choice));
            self.remove_mod("header_image_data");
            return;
        }
        let data = match self.uploaded.iter().find(|u| u.attachment_id.to_string() == choice) {
            Some(uploaded) => (rustpress_core::formatting::esc_url_raw(&uploaded.url), uploaded.to_php()),
            None => match self.defaults.iter().find(|d| d.key == choice) {
                Some(default) => (rustpress_core::formatting::esc_url_raw(&default.url), default.to_php()),
                None => return,
            },
        };
        self.set_mod("header_image", text(&data.0));
        self.set_mod("header_image_data", data.1);
    }

    /// `Custom_Image_Header::reset_header_image()`.
    fn reset_header_image(&mut self) {
        let default = self.support.default_image.clone();
        if default.is_empty() {
            self.set_header_image("remove-header");
            return;
        }
        let props = match self.defaults.iter().find(|d| d.url == default) {
            Some(details) => vec![
                (Key::Str(b"url".to_vec()), Value::Str(details.url.as_bytes().to_vec())),
                (Key::Str(b"thumbnail_url".to_vec()), Value::Str(details.thumbnail_url.as_bytes().to_vec())),
                (Key::Str(b"description".to_vec()), Value::Str(details.description.as_bytes().to_vec())),
            ],
            None => Vec::new(),
        };
        self.set_mod("header_image", Value::Str(default.into_bytes()));
        self.set_mod("header_image_data", Value::Object { class: b"stdClass".to_vec(), props });
    }

    /// `Custom_Image_Header::set_header_image()` for an attachment the screen made, saved at once.
    async fn set_header_image_data(&mut self, ctx: &Ctx<'_>, url: &str, attachment_id: u64, width: u64, height: u64) {
        let url = rustpress_core::formatting::esc_url_raw(url);
        let text = |s: &str| Value::Str(s.as_bytes().to_vec());
        let key = |s: &str| Key::Str(s.as_bytes().to_vec());
        let props = vec![
            (key("attachment_id"), Value::Int(attachment_id as i64)),
            (key("url"), text(&url)),
            (key("thumbnail_url"), text(&url)),
            (key("height"), Value::Int(height as i64)),
            (key("width"), Value::Int(width as i64)),
        ];
        if let Some(writer) = ctx.env.writer {
            writer.update_post_meta(attachment_id, "_wp_attachment_is_custom_header", &ctx.site().settings.stylesheet).await.or_log();
        }
        self.set_mod("header_image", text(&url));
        self.set_mod("header_image_data", Value::Object { class: b"stdClass".to_vec(), props });
        self.save(ctx).await;
    }

    /// Writes the theme mods back (`update_option( "theme_mods_$theme", $mods )`).
    async fn save(&self, ctx: &Ctx<'_>) {
        let Some(writer) = ctx.env.writer else { return };
        let empty = matches!(&self.mods, Value::Array(items) if items.is_empty());
        let result = if empty {
            // `remove_theme_mods()`.
            writer.delete_option(&self.mods_name).await
        } else {
            writer.update_option(&self.mods_name, &String::from_utf8_lossy(&rustpress_phpser::serialize(&self.mods))).await
        };
        if let Err(error) = result {
            tracing::error!(%error, "saving the theme mods failed");
        }
    }
}

/// Whether the screen exists: a ported classic theme that supports `custom-header`.
pub fn available(ctx: &Ctx<'_>) -> bool {
    let theme = ctx.env.theme;
    !ctx.env.block_theme && header::support(&theme.template, &theme.template_directory_uri()).is_some()
}

/// `Custom_Image_Header::help()`.
fn help(ctx: &Ctx<'_>) -> Help {
    let tr = |text: &str| ctx.tr(text);
    let tabs = vec![
        HelpTab {
            id: "overview".into(),
            title: tr("Overview"),
            content: format!(
                "<p>{}</p><p>{}<p>",
                tr("This screen is used to customize the header section of your theme."),
                tr("You can choose from the theme&#8217;s default header images, or use one of your own. You can also customize how your Site Title and Tagline are displayed.")
            ),
        },
        HelpTab {
            id: "set-header-image".into(),
            title: tr("Header Image"),
            content: [
                "You can set a custom image header for your site. Simply upload the image and crop it, and the new header will go live immediately. Alternatively, you can use an image that has already been uploaded to your Media Library by clicking the &#8220;Choose Image&#8221; button.",
                "Some themes come with additional header images bundled. If you see multiple images displayed, select the one you would like and click the &#8220;Save Changes&#8221; button.",
                "If your theme has more than one default header image, or you have uploaded more than one custom header image, you have the option of having WordPress display a randomly different image on each page of your site. Click the &#8220;Random&#8221; radio button next to the Uploaded Images or Default Images section to enable this feature.",
                "If you do not want a header image to be displayed on your site at all, click the &#8220;Remove Header Image&#8221; button at the bottom of the Header Image section of this page. If you want to re-enable the header image later, you just have to select one of the other image options and click &#8220;Save Changes&#8221;.",
            ]
            .iter()
            .map(|text| format!("<p>{}</p>", tr(text)))
            .collect(),
        },
        HelpTab {
            id: "set-header-text".into(),
            title: tr("Header Text"),
            content: format!(
                "<p>{}</p><p>{}</p><p>{}</p>",
                sprintf(&tr("For most themes, the header text is your Site Title and Tagline, as defined in the <a href=\"%s\">General Settings</a> section."), &[&ctx.admin_url("options-general.php")]),
                tr("In the Header Text section of this page, you can choose whether to display this text or hide it. You can also choose a color for the text by clicking the Select Color button and either typing in a legitimate HTML hex value, e.g. &#8220;#ff0000&#8221; for red, or by choosing a color using the color picker."),
                tr("Do not forget to click &#8220;Save Changes&#8221; when you are done!")
            ),
        },
    ];
    let sidebar = format!(
        "<p><strong>{}</strong></p><p>{}</p><p>{}</p>",
        tr("For more information:"),
        tr("<a href=\"https://codex.wordpress.org/Appearance_Header_Screen\">Documentation on Custom Header</a>"),
        tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
    );
    Help { tabs, sidebar }
}

/// `Custom_Image_Header::step()`.
fn step(ctx: &Ctx<'_>) -> u8 {
    let Some(raw) = ctx.request.get("step") else { return 1 };
    let step = super::media_upload::php_int(raw);
    match step {
        2 if ctx.verify_nonce_field("custom-header-upload", "_wpnonce-custom-header-upload") => 2,
        3 if ctx.verify_nonce("custom-header-crop-image") => 3,
        _ => 1,
    }
}

/// `Custom_Image_Header::take_action()`. `Err` is the page of a failed `check_admin_referer()`.
fn take_action(ctx: &Ctx<'_>, state: &mut State) -> Result<bool, AdminResponse> {
    let request = ctx.request;
    if !ctx.can("edit_theme_options") || !request.post || request.form.is_empty() {
        return Ok(false);
    }
    let check = || if ctx.verify_nonce_field("custom-header-options", "_wpnonce-custom-header-options") { Ok(()) } else { Err(super::options::nonce_ays(ctx)) };
    if request.post_field("resetheader").is_some() {
        check()?;
        state.reset_header_image();
        return Ok(true);
    }
    if request.post_field("removeheader").is_some() {
        check()?;
        state.set_header_image("remove-header");
        return Ok(true);
    }
    if let Some(color) = request.post_field("text-color") {
        check()?;
        if request.post_field("display-header-text").is_none() {
            state.set_mod("header_textcolor", Value::Str(b"blank".to_vec()));
        } else {
            let color: String = color.replace('#', "").chars().filter(char::is_ascii_hexdigit).collect();
            if color.len() == 6 || color.len() == 3 {
                state.set_mod("header_textcolor", Value::Str(color.into_bytes()));
            } else if color.is_empty() {
                state.set_mod("header_textcolor", Value::Str(b"blank".to_vec()));
            }
        }
    }
    if let Some(choice) = request.post_field("default-header") {
        check()?;
        state.set_header_image(choice);
    }
    Ok(true)
}

/// `checked( $a, $b )` of two values as PHP compares them as strings.
fn checked(on: bool) -> &'static str {
    if on { " checked='checked'" } else { "" }
}

/// `Custom_Image_Header::show_header_selector()`.
fn header_selector(ctx: &Ctx<'_>, state: &State, kind: &str) -> String {
    // `(key, url, thumbnail, alt, is_attachment)`.
    let headers: Vec<(String, String, String, String, bool)> = if kind == "default" {
        state.defaults.iter().map(|d| (d.key.clone(), d.url.clone(), d.thumbnail_url.clone(), String::new(), false)).collect()
    } else {
        state.uploaded.iter().map(|u| (u.attachment_id.to_string(), u.url.clone(), u.url.clone(), u.alt_text.clone(), u.attachment_id != 0)).collect()
    };
    let mut out = String::new();
    if headers.len() > 1 {
        let random = header::is_random_header_image(kind, &state.mods, &state.support, state.random.as_ref());
        out.push_str(&format!(
            "<div class=\"random-header\"><label><input name=\"default-header\" type=\"radio\" value=\"random-{kind}-image\"{} />{}</label></div>",
            checked(random),
            ctx.tr("<strong>Random:</strong> Show a different image on each page.")
        ));
    }
    out.push_str("<div class=\"available-headers\">");
    // `get_theme_mod( 'header_image' )`: `false` when it is not set.
    let current = header::mod_value(&state.mods, "header_image").and_then(|v| match v {
        Value::Str(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Value::Int(n) => Some(n.to_string()),
        _ => None,
    });
    for (key, url, thumbnail, alt, attachment) in headers {
        let width = if attachment { " width=\"230\"" } else { "" };
        out.push_str(&format!(
            "<div class=\"default-header\"><label><input name=\"default-header\" type=\"radio\" value=\"{}\" {} /><img src=\"{}\" alt=\"{}\"{width} /></label></div>",
            esc_attr(&key),
            checked(current.as_deref().unwrap_or_default() == url),
            esc_url(&set_url_scheme(ctx, &thumbnail)),
            esc_attr(&alt)
        ));
    }
    out.push_str("<div class=\"clear\"></div></div>");
    out
}

fn set_url_scheme(ctx: &Ctx<'_>, url: &str) -> String {
    let scheme = if ctx.site().settings.siteurl.starts_with("https://") { "https" } else { "http" };
    let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")).or_else(|| url.strip_prefix("//"));
    rest.map_or(url.to_owned(), |rest| format!("{scheme}://{rest}"))
}

/// The theme's default text color with its `#`.
fn default_color(support: &Support) -> String {
    let color = &support.default_text_color;
    if !color.is_empty() && !color.contains('#') { format!("#{color}") } else { color.clone() }
}

/// `bloginfo()`: the `display` filters.
fn bloginfo(ctx: &Ctx<'_>, text: &str) -> String {
    esc_html(&rustpress_core::formatting::convert_chars(&ctx.site().texturize(text)))
}

/// `Custom_Image_Header::js_1()`.
fn js_1(state: &State) -> String {
    let toggle = if header::display_header_text(&state.mods, &state.support) { "" } else { "\t\ttoggle_text();\n\t\t" };
    format!(
        "<script>\n(function($){{\n\tvar default_color = '{}',\n\t\theader_text_fields;\n\n\tfunction pickColor(color) {{\n\t\t$('#name').css('color', color);\n\t\t$('#desc').css('color', color);\n\t\t$('#text-color').val(color);\n\t}}\n\n\tfunction toggle_text() {{\n\t\tvar checked = $('#display-header-text').prop('checked'),\n\t\t\ttext_color;\n\t\theader_text_fields.toggle( checked );\n\t\tif ( ! checked )\n\t\t\treturn;\n\t\ttext_color = $('#text-color');\n\t\tif ( '' === text_color.val().replace('#', '') ) {{\n\t\t\ttext_color.val( default_color );\n\t\t\tpickColor( default_color );\n\t\t}} else {{\n\t\t\tpickColor( text_color.val() );\n\t\t}}\n\t}}\n\n\t$( function() {{\n\t\tvar text_color = $('#text-color');\n\t\theader_text_fields = $('.displaying-header-text');\n\t\ttext_color.wpColorPicker({{\n\t\t\tchange: function( event, ui ) {{\n\t\t\t\tpickColor( text_color.wpColorPicker('color') );\n\t\t\t}},\n\t\t\tclear: function() {{\n\t\t\t\tpickColor( '' );\n\t\t\t}}\n\t\t}});\n\t\t$('#display-header-text').click( toggle_text );\n\t\t{toggle}\t}} );\n}})(jQuery);\n</script>\n\t\t",
        // `esc_js()` of a hex color is the color.
        esc_attr(&default_color(&state.support))
    )
}

/// `Custom_Image_Header::step_1()`.
fn step_1(ctx: &Ctx<'_>, state: &State, updated: bool) -> String {
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    let support = &state.support;
    let mut out = format!("\n<div class=\"wrap\">\n<h1>{}</h1>\n\n\t\t", tr("Custom Header"));
    if ctx.can("customize") {
        let message = sprintf(&tr("You can now manage and live-preview Custom Header in the <a href=\"%s\">Customizer</a>."), &[&ctx.admin_url("customize.php?autofocus[control]=header_image")]);
        out.push_str(notice(&message, "info", false, "", &["hide-if-no-customize"], true).trim_end());
    }
    if updated {
        let message = sprintf(&tr("Header updated. <a href=\"%s\">Visit your site</a> to see how it looks."), &[&esc_url(&site.home_url("/"))]);
        out.push_str(notice(&message, "", false, "message", &["updated"], true).trim_end());
    }
    out.push_str(&format!("\n<h2>{}</h2>\n\n<table class=\"form-table\" role=\"presentation\">\n<tbody>\n\n\t\t", tr("Header Image")));

    // `get_custom_header()` is an object, so the preview always shows.
    let header_image = state.header_image(ctx);
    let (width, height) = header::custom_header_size(&state.mods, support, state.random.as_ref());
    let mut image_style = match header_image.as_deref().filter(|url| !url.is_empty()) {
        Some(url) => format!("background-image:url({});", esc_url(url)),
        None => String::new(),
    };
    if width != 0 {
        image_style.push_str(&format!("max-width:{width}px;"));
    }
    if height != 0 {
        image_style.push_str(&format!("height:{height}px;"));
    }
    let text_style = if header::display_header_text(&state.mods, support) {
        format!(" style=\"color:#{};\"", header::header_textcolor(&state.mods, support))
    } else {
        " style=\"display:none;\"".to_owned()
    };
    out.push_str(&format!(
        "<tr>\n<th scope=\"row\">{}</th>\n<td>\n\t\t\t\t<div id=\"headimg\" style=\"{}\">\n\t\t\t\t\t\t<h1><a id=\"name\" class=\"displaying-header-text\" {text_style} onclick=\"return false;\" href=\"{}\" tabindex=\"-1\">{}</a></h1>\n\t\t<div id=\"desc\" class=\"displaying-header-text\" {text_style}>{}</div>\n\t</div>\n\t\t\t</td>\n</tr>\n\t\t\n\t\t",
        esc_html(&site.trx("Preview", "noun")),
        esc_attr(&image_style),
        site.home_url(""),
        bloginfo(ctx, &site.settings.blogname),
        bloginfo(ctx, site.options.get("blogdescription").unwrap_or_default())
    ));

    if ctx.can("upload_files") && support.uploads {
        let pixels = |n: u64| format!("<strong>{}</strong>", sprintf(&tr("%d pixels"), &[&n.to_string()]));
        let mut sizes = String::new();
        if !support.flex_height && !support.flex_width {
            sizes.push_str(&format!("{}<br />", sprintf(&tr("Images of exactly <strong>%1$d &times; %2$d pixels</strong> will be used as-is."), &[&support.width.to_string(), &support.height.to_string()])));
        } else if support.flex_height {
            if !support.flex_width {
                sizes.push_str(&format!("{} ", sprintf(&tr("Images should be at least %s wide."), &[&pixels(support.width)])));
            }
        } else if support.flex_width && !support.flex_height {
            sizes.push_str(&format!("{} ", sprintf(&tr("Images should be at least %s tall."), &[&pixels(support.height)])));
        }
        if support.flex_height || support.flex_width {
            if support.width != 0 {
                sizes.push_str(&format!("{} ", sprintf(&tr("Suggested width is %s."), &[&pixels(support.width)])));
            }
            if support.height != 0 {
                sizes.push_str(&format!("{} ", sprintf(&tr("Suggested height is %s."), &[&pixels(support.height)])));
            }
        }
        let modal = format!("{}?page=custom-header&step=2&_wpnonce-custom-header-upload={}", ctx.admin_url("themes.php"), ctx.create_nonce("custom-header-upload"));
        out.push_str(&format!(
            "<tr>\n<th scope=\"row\">{}</th>\n<td>\n\t<p>{}<br />\n\t\t\t{sizes}\t</p>\n\t<form enctype=\"multipart/form-data\" id=\"upload-form\" class=\"wp-upload-form\" method=\"post\" action=\"{}\">\n\t<p>\n\t\t<label for=\"upload\">{}</label><br />\n\t\t<input type=\"file\" id=\"upload\" name=\"import\" />\n\t\t<input type=\"hidden\" name=\"action\" value=\"save\" />\n\t\t\t{}\t\t\t{}\t</p>\n\t\t\t\t<p>\n\t\t<label for=\"choose-from-library-link\">{}</label><br />\n\t\t<button id=\"choose-from-library-link\" class=\"button\"\n\t\t\tdata-update-link=\"{}\"\n\t\t\tdata-choose=\"{}\"\n\t\t\tdata-update=\"{}\">{}</button>\n\t</p>\n\t</form>\n</td>\n</tr>\n\t\t",
            tr("Select Image"),
            tr("You can select an image to be shown at the top of your site by uploading from your computer or choosing from your media library. After selecting an image you will be able to crop it."),
            esc_url(&add_query_arg(&ctx.request.request_uri, "step", "2")),
            tr("Choose an image from your computer:"),
            ctx.nonce_field_named("custom-header-upload", "_wpnonce-custom-header-upload"),
            submit_button(&site.trx("Upload", "verb"), "", "submit", false, None, &[]),
            tr("Or choose an image from your media library:"),
            esc_url(&modal),
            esc_attr(&tr("Choose a Custom Header")),
            esc_attr(&tr("Set as header")),
            tr("Choose Image")
        ));
    }
    out.push_str(&format!(
        "</tbody>\n</table>\n\n<form method=\"post\" action=\"{}\">\n\t\t{}<table class=\"form-table\" role=\"presentation\">\n<tbody>\n\t\t",
        esc_url(&add_query_arg(&ctx.request.request_uri, "step", "1")),
        submit_button(&tr("Save Changes"), "screen-reader-text", "save-header-options", false, None, &[])
    ));
    if !state.uploaded.is_empty() {
        out.push_str(&format!(
            "<tr>\n<th scope=\"row\">{}</th>\n<td>\n\t<p>{}</p>\n\t\t\t{}</td>\n</tr>\n\t\t\t",
            tr("Uploaded Images"),
            tr("You can choose one of your previously uploaded headers, or show a random one."),
            header_selector(ctx, state, "uploaded")
        ));
    }
    if !state.defaults.is_empty() {
        let text = if support.uploads {
            tr("If you do not want to upload your own image, you can use one of these cool headers, or show a random one.")
        } else {
            tr("You can use one of these cool headers or show a random one on each page.")
        };
        out.push_str(&format!("<tr>\n<th scope=\"row\">{}</th>\n<td>\n\t\t\t\t<p>{text}</p>\n\t\t\t\t{}</td>\n</tr>\n\t\t\t", tr("Default Images"), header_selector(ctx, state, "default")));
    }
    let has_image = header_image.as_deref().is_some_and(|url| !url.is_empty());
    if has_image {
        out.push_str(&format!(
            "<tr>\n<th scope=\"row\">{}</th>\n<td>\n\t<p>{}</p>\n\t\t\t{}</td>\n</tr>\n\t\t\t",
            tr("Remove Image"),
            tr("This will remove the header image. You will not be able to restore any customizations."),
            submit_button(&tr("Remove Header Image"), "", "removeheader", false, None, &[])
        ));
    }
    let default_image = &support.default_image;
    if !default_image.is_empty() && header_image.as_deref() != Some(default_image.as_str()) {
        out.push_str(&format!(
            "<tr>\n<th scope=\"row\">{}</th>\n<td>\n\t<p>{}</p>\n\t\t\t{}</td>\n</tr>\n\t",
            tr("Reset Image"),
            tr("This will restore the original header image. You will not be able to restore any customizations."),
            submit_button(&tr("Restore Original Header Image"), "", "resetheader", false, None, &[])
        ));
    }
    out.push_str("</tbody>\n</table>\n\n\t\t");
    if support.header_text {
        let default = default_color(support);
        let default_attr = if default.is_empty() { String::new() } else { format!(" data-default-color=\"{}\"", esc_attr(&default)) };
        let mut color = if header::display_header_text(&state.mods, support) { header::header_textcolor(&state.mods, support) } else { support.default_text_color.clone() };
        if !color.is_empty() && !color.contains('#') {
            color = format!("#{color}");
        }
        let hint = if default.is_empty() {
            String::new()
        } else {
            format!(" <span class=\"description hide-if-js\">{}</span>", sprintf(&site.trx("Default: %s", "color"), &[&esc_html(&default)]))
        };
        out.push_str(&format!(
            "\n<h2>{}</h2>\n\n<table class=\"form-table\" role=\"presentation\">\n<tbody>\n<tr>\n<th scope=\"row\">{}</th>\n<td>\n\t<p>\n\t<label><input type=\"checkbox\" name=\"display-header-text\" id=\"display-header-text\"{} /> {}</label>\n\t</p>\n</td>\n</tr>\n\n<tr class=\"displaying-header-text\">\n<th scope=\"row\">{}</th>\n<td>\n\t<p>\n\t\t\t<input type=\"text\" name=\"text-color\" id=\"text-color\" value=\"{}\"{default_attr} />{hint}\t</p>\n</td>\n</tr>\n</tbody>\n</table>\n\t\t\t",
            tr("Header Text"),
            tr("Header Text"),
            checked(header::display_header_text(&state.mods, support)),
            tr("Show header text with your image."),
            tr("Text Color"),
            esc_attr(&color)
        ));
    }
    out.push_str(&ctx.nonce_field_named("custom-header-options", "_wpnonce-custom-header-options"));
    out.push_str(&format!("\n\t\t{}</form>\n</div>\n\n\t\t", submit_button(&tr("Save Changes"), "primary", "save-header-options", true, None, &[])));
    out
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let site = ctx.site();
    let theme = ctx.env.theme;
    let template_uri = theme.template_directory_uri();
    let Some(support) = header::support(&theme.template, &template_uri) else { return super::common::forbidden(ctx, &ctx.tr("Sorry, you are not allowed to access this page.")) };
    let mods_name = format!("theme_mods_{}", site.settings.stylesheet);
    let stored = ctx.env.db.option(&mods_name).await.ok().flatten().unwrap_or_default();
    let mods = rustpress_core::options::maybe_unserialize(&stored);
    let uploaded = header::uploaded_header_images(site, &site.settings.stylesheet).await.unwrap_or_default();
    let defaults = header::default_headers(site, &theme.template, &template_uri);
    let mut state = State { support, defaults, uploaded, mods_name, mods, random: None };

    let before = state.mods.clone();
    let (updated, failed) = match take_action(ctx, &mut state) {
        Ok(updated) => (updated, None),
        Err(response) => (true, Some(response)),
    };
    if state.mods != before {
        state.save(ctx).await;
    }
    state.random = header::random_header(&state.mods, &state.support, &state.defaults, &state.uploaded);
    let step = step(ctx);

    let mut assets = Assets::default();
    if step == 2 {
        // `css_includes()`, `js_includes()` and `js_2()`.
        // It is not concatenated: it follows the emoji styles.
        assets.plugin_late_styles.push_str(&format!("<link rel='stylesheet' id='imgareaselect-css' href='{}' media='all' />\n", esc_url(&ctx.site().site_url("wp-includes/js/imgareaselect/imgareaselect.css?ver=0.9.8"))));
        assets.late_scripts.push("imgareaselect".into());
        assets.plugin_admin_head.push_str(&js_2(&state.support));
    } else {
        // `css_includes()` runs on `admin_print_styles-{$page}`, after the styles of every screen.
        if state.support.header_text {
            assets.plugin_core_styles.push("wp-color-picker".into());
        }
        // `js_includes()` runs on `admin_print_scripts-{$page}`: the styles are printed already, so those of
        // `wp_enqueue_media()` come in the footer, and the scripts after those of every screen.
        super::media_ui::enqueue(ctx, &mut assets, 0).await;
        assets.post_header_styles = std::mem::take(&mut assets.styles);
        assets.late_scripts = std::mem::take(&mut assets.scripts);
        assets.late_scripts.push("custom-header".into());
        if state.support.header_text {
            assets.late_scripts.push("wp-color-picker".into());
            assets.plugin_admin_head.push_str(&js_1(&state));
        }
    }
    // `take_action()` runs on `admin_head` at priority 50: a failed `check_admin_referer()` dies after the head printed its
    // scripts, and the status is sent already.
    if failed.is_some() {
        assets.plugin_admin_head.insert_str(0, CUT);
    }
    let screen = Screen {
        file: "themes.php".into(),
        parent_file: "themes.php".into(),
        submenu_file: Some("custom-header".into()),
        title: site.trx("Header", "custom image header"),
        help: Some(help(ctx)),
        hook_suffix: Some("appearance_page_custom-header".into()),
        content_inline: true,
        ..Default::default()
    };
    // `admin_page()` runs after the header printed.
    let outcome = if failed.is_some() {
        Outcome::Page(String::new())
    } else {
        match step {
            2 => step_2(ctx, &mut state).await,
            3 => step_3(ctx, &mut state).await,
            _ => Outcome::Page(step_1(ctx, &state, updated)),
        }
    };
    let (content, died) = match outcome {
        Outcome::Page(content) => (content, None),
        Outcome::Die(message) => (CUT.to_owned(), Some(message)),
    };
    let mut response = render(ctx, &screen, &content, &assets).await;
    if let Some(die) = failed {
        let head = response.body.split(CUT).next().unwrap_or_default().to_owned();
        response.body = format!("{head}{}", die.body);
    } else if let Some(message) = died {
        // `_default_wp_die_handler()` after `admin_head`: no document of its own, and the status is sent already.
        let head = response.body.split(CUT).next().unwrap_or_default().to_owned();
        response.body = format!("{head}\t<div class=\"wp-die-message\">{message}</div></body>\n</html>\n\t");
    }
    response
}

/// What `admin_page()` printed: the screen, or the message of a `wp_die()`.
enum Outcome {
    Page(String),
    Die(String),
}

/// PHP's `echo` of a float (`precision` 14): `2000`, `1333.3333333333`.
fn php_float(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let digits = 14 - (value.abs().log10().floor() as i32 + 1);
    let text = format!("{:.*}", digits.max(0) as usize, value);
    if text.contains('.') { text.trim_end_matches('0').trim_end_matches('.').to_owned() } else { text }
}

/// A number as PHP prints it: an `int` or a float.
#[derive(Clone, Copy)]
enum Number {
    Int(i64),
    Float(f64),
}

impl Number {
    fn value(self) -> f64 {
        match self {
            Number::Int(n) => n as f64,
            Number::Float(f) => f,
        }
    }

    fn text(self) -> String {
        match self {
            Number::Int(n) => n.to_string(),
            Number::Float(f) => php_float(f),
        }
    }
}

/// `Custom_Image_Header::js_2()`.
fn js_2(support: &Support) -> String {
    let mut options = String::new();
    if !support.flex_height && !support.flex_width {
        options.push_str("\n\t\t\taspectRatio: xinit + ':' + yinit,\n\t\t\t\t");
    }
    if !support.flex_height {
        options.push_str(&format!("\t\t\tmaxHeight: {},\n\t\t\t\t", support.height));
    }
    if !support.flex_width {
        options.push_str(&format!("\t\t\tmaxWidth: {},\n\t\t\t\t", support.width));
    }
    format!(
        "<script>\n\tfunction onEndCrop( coords ) {{\n\t\tjQuery( '#x1' ).val(coords.x);\n\t\tjQuery( '#y1' ).val(coords.y);\n\t\tjQuery( '#width' ).val(coords.w);\n\t\tjQuery( '#height' ).val(coords.h);\n\t}}\n\n\tjQuery( function() {{\n\t\tvar xinit = {};\n\t\tvar yinit = {};\n\t\tvar ratio = xinit / yinit;\n\t\tvar ximg = jQuery('img#upload').width();\n\t\tvar yimg = jQuery('img#upload').height();\n\n\t\tif ( yimg < yinit || ximg < xinit ) {{\n\t\t\tif ( ximg / yimg > ratio ) {{\n\t\t\t\tyinit = yimg;\n\t\t\t\txinit = yinit * ratio;\n\t\t\t}} else {{\n\t\t\t\txinit = ximg;\n\t\t\t\tyinit = xinit / ratio;\n\t\t\t}}\n\t\t}}\n\n\t\tjQuery('img#upload').imgAreaSelect({{\n\t\t\thandles: true,\n\t\t\tkeys: true,\n\t\t\tshow: true,\n\t\t\tx1: 0,\n\t\t\ty1: 0,\n\t\t\tx2: xinit,\n\t\t\ty2: yinit,\n\t\t\t{options}\t\t\tonInit: function () {{\n\t\t\t\tjQuery('#width').val(xinit);\n\t\t\t\tjQuery('#height').val(yinit);\n\t\t\t}},\n\t\t\tonSelectChange: function(img, c) {{\n\t\t\t\tjQuery('#x1').val(c.x1);\n\t\t\t\tjQuery('#y1').val(c.y1);\n\t\t\t\tjQuery('#width').val(c.width);\n\t\t\t\tjQuery('#height').val(c.height);\n\t\t\t}}\n\t\t}});\n\t}} );\n</script>\n\t\t",
        support.width, support.height
    )
}

/// The message of the `wp_die()` for a theme without uploads.
fn no_uploads(ctx: &Ctx<'_>) -> String {
    format!(
        "<h1>{}</h1><p>{}</p>",
        ctx.tr("An error occurred while processing your header image."),
        ctx.tr("The active theme does not support uploading a custom header image. Please ensure your theme supports custom headers and try again.")
    )
}

/// `max( 1500 if flex-width, max-width, width )` of `step_2()` and `get_header_dimensions()`.
fn max_width(support: &Support) -> u64 {
    let flex = if support.flex_width { 1500 } else { 0 };
    flex.max(support.max_width).max(support.width)
}

/// `wp_insert_attachment()` of what the screen makes, with `context` and the file below the uploads.
async fn insert_attachment(ctx: &Ctx<'_>, writer: &rustpress_db::Writer, data: PostData, relative: &str, context: &str, alt: Option<&str>) -> Option<u64> {
    let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let id = content.insert_post(&data).await.ok()?;
    writer.update_post_meta(id, "_wp_attached_file", relative).await.or_log();
    if let Some(alt) = alt {
        writer.update_post_meta(id, "_wp_attachment_image_alt", alt).await.or_log();
    }
    if !context.is_empty() {
        writer.update_post_meta(id, "_wp_attachment_context", context).await.or_log();
    }
    Some(id)
}

/// The fields every attachment of the screen has.
fn attachment_data(ctx: &Ctx<'_>, title: String, content: String, mime: String, guid: String) -> PostData {
    PostData {
        post_title: Some(title),
        post_content: Some(content),
        post_status: Some("inherit".into()),
        post_type: Some("attachment".into()),
        post_mime_type: Some(mime),
        post_parent: Some(0),
        guid: Some(guid),
        post_author: Some(ctx.user.id),
        comment_status: Some(ctx.site().options.string("default_comment_status", "closed")),
        ping_status: Some("closed".into()),
        ..Default::default()
    }
}

/// `Custom_Image_Header::step_2_manage_upload()`: `Err` is the message of a `wp_die()`.
async fn manage_upload(ctx: &Ctx<'_>) -> Result<(u64, std::path::PathBuf, String), String> {
    let site = ctx.site();
    let invalid = || ctx.tr("The uploaded file is not a valid image. Please try again.");
    let Some(file) = ctx.request.files.iter().find(|f| f.field == "import") else { return Err(invalid()) };
    // `wp_check_filetype_and_ext()` before the file is stored.
    let mime = media::check_filetype(&file.filename).and_then(|(_, mime)| media::check_image_upload(&file.filename, &mime, &file.bytes)).map(|(_, mime)| mime);
    if !mime.as_deref().is_some_and(|mime| mime.starts_with("image/")) {
        return Err(invalid());
    }
    let Some(writer) = ctx.env.writer else { return Err(invalid()) };
    let content = Content { site, db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let uploads = super::ajax_media::uploads_dir(ctx.env);
    let mut upload = media::UploadedFile { name: file.filename.clone(), bytes: file.bytes.clone(), rest: false, media_form_nonce: false };
    let stored = media::store_upload(&content, &uploads, &mut upload, &media::upload_subdir(&content)).map_err(|media::UploadError(message)| message)?;
    let filename = stored.relative.rsplit('/').next().unwrap_or_default().to_owned();
    let data = attachment_data(ctx, filename, stored.url.clone(), stored.mime.clone(), stored.url.clone());
    let id = insert_attachment(ctx, writer, data, &stored.relative, "custom-header", None).await.ok_or_else(invalid)?;
    Ok((id, stored.path, stored.url))
}

/// `Custom_Image_Header::step_2()`.
async fn step_2(ctx: &Ctx<'_>, state: &mut State) -> Outcome {
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    if !state.support.uploads {
        return Outcome::Die(no_uploads(ctx));
    }
    let uploads = super::ajax_media::uploads_dir(ctx.env);
    let from_library = ctx.request.form.is_empty() && ctx.request.files.is_empty() && ctx.request.get("file").is_some();
    let (attachment_id, file, mut url) = if from_library {
        let id = super::media_upload::php_int(ctx.request.get("file").unwrap_or_default()).unsigned_abs();
        let attached = ctx.env.db.post_meta_value(id, "_wp_attached_file").await.ok().flatten().unwrap_or_default();
        let url = if attached.is_empty() { String::new() } else { rustpress_blocks::images::attachment_url(site, &attached) };
        (id, uploads.join(&attached), url)
    } else {
        match manage_upload(ctx).await {
            Ok(uploaded) => uploaded,
            Err(message) => return Outcome::Die(message),
        }
    };
    let (mut width, mut height) = match image::image_dimensions(&file) {
        Ok((w, h)) if file.is_file() => (Number::Int(i64::from(w)), Number::Int(i64::from(h))),
        _ => {
            let meta = ctx.env.db.post_meta_value(attachment_id, "_wp_attachment_metadata").await.ok().flatten().map(|raw| rustpress_core::options::maybe_unserialize(&raw)).unwrap_or(Value::Null);
            let number = |name: &str| meta.get(name).and_then(Value::as_i64).unwrap_or(0);
            (Number::Int(number("width")), Number::Int(number("height")))
        }
    };
    let support = state.support.clone();
    let max = max_width(&support);
    let oitar;
    if !support.flex_height && !support.flex_width && support.width as f64 == width.value() && support.height as f64 == height.value() {
        // The image has the size of the theme: it becomes the header as it is.
        if file.is_file()
            && let Some(writer) = ctx.env.writer
        {
            let relative = file.strip_prefix(&uploads).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
            let mime = media::check_filetype(&relative).map(|(_, mime)| mime).unwrap_or_default();
            let (options, uploads, rel) = (site.options.clone(), uploads.clone(), relative.clone());
            if let Some((metadata, ..)) = media::blocking(move || media::generate_image_metadata(&options, &uploads, &rel, &mime)).await {
                writer.update_post_meta(attachment_id, "_wp_attachment_metadata", &String::from_utf8_lossy(&rustpress_phpser::serialize(&metadata))).await.or_log();
            }
        }
        state.set_header_image_data(ctx, &url, attachment_id, width.value() as u64, height.value() as u64).await;
        return Outcome::Page(step_1(ctx, &*state, true));
    } else if width.value() > max as f64 {
        let ratio = width.value() / max as f64;
        let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mime = media::check_filetype(&name).map(|(_, mime)| mime).unwrap_or_default();
        let (source, w, h, dst_h) = (file.clone(), width.value() as u32, height.value() as u32, (height.value() / ratio) as u32);
        let midsize = media::blocking(move || media::crop_image_as(&source, 0, 0, w, h, max as u32, dst_h, &mime, &format!("midsize-{name}"))).await;
        let Ok(midsize) = midsize else { return Outcome::Die(tr("Image could not be processed. Please go back and try again.")) };
        let (old, new) = (url.rsplit('/').next().unwrap_or_default().to_owned(), midsize.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        url = url.replace(&old, &new);
        width = Number::Float(width.value() / ratio);
        height = Number::Float(height.value() / ratio);
        oitar = Number::Float(ratio);
    } else {
        oitar = Number::Int(1);
    }
    let create_new = if from_library { "\t<input type=\"hidden\" name=\"create-new-attachment\" value=\"true\" />\n\t" } else { "" };
    let skip = if matches!(oitar, Number::Int(1)) && (support.flex_height || support.flex_width) {
        submit_button(&tr("Skip Cropping, Publish Image as Is"), "", "skip-cropping", false, None, &[])
    } else {
        String::new()
    };
    Outcome::Page(format!(
        "\n<div class=\"wrap\">\n<h1>{}</h1>\n\n<form method=\"post\" action=\"{}\">\n\t<p class=\"hide-if-no-js\">{}</p>\n\t<p class=\"hide-if-js\"><strong>{}</strong></p>\n\n\t<div id=\"crop_image\" style=\"position: relative\">\n\t\t<img src=\"{}\" id=\"upload\" width=\"{w}\" height=\"{h}\" alt=\"\" />\n\t</div>\n\n\t<input type=\"hidden\" name=\"x1\" id=\"x1\" value=\"0\" />\n\t<input type=\"hidden\" name=\"y1\" id=\"y1\" value=\"0\" />\n\t<input type=\"hidden\" name=\"width\" id=\"width\" value=\"{w}\" />\n\t<input type=\"hidden\" name=\"height\" id=\"height\" value=\"{h}\" />\n\t<input type=\"hidden\" name=\"attachment_id\" id=\"attachment_id\" value=\"{attachment_id}\" />\n\t<input type=\"hidden\" name=\"oitar\" id=\"oitar\" value=\"{}\" />\n\t\t{create_new}\t\t{}\n\t<p class=\"submit\">\n\t\t{}\t\t{skip}\t</p>\n</form>\n</div>\n\t\t",
        tr("Crop Header Image"),
        esc_url(&add_query_arg(&ctx.request.request_uri, "step", "3")),
        tr("Choose the part of the image you want to use as your header."),
        tr("You need JavaScript to choose a part of the image."),
        esc_url(&url),
        oitar.text(),
        ctx.nonce_field("custom-header-crop-image"),
        submit_button(&tr("Crop and Publish"), "primary", "submit", false, None, &[]),
        w = esc_attr(&width.text()),
        h = esc_attr(&height.text()),
    ))
}

/// A number of the form as PHP's arithmetic reads it.
fn form_number(ctx: &Ctx<'_>, name: &str) -> f64 {
    let text = ctx.request.post_field(name).unwrap_or_default().trim();
    let end = text.char_indices().find(|(i, c)| !(c.is_ascii_digit() || *c == '.' || (*i == 0 && (*c == '-' || *c == '+')))).map_or(text.len(), |(i, _)| i);
    text[..end].parse::<f64>().unwrap_or(0.0)
}

/// `Custom_Image_Header::step_3()`.
async fn step_3(ctx: &Ctx<'_>, state: &mut State) -> Outcome {
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    let support = state.support.clone();
    if !support.uploads {
        return Outcome::Die(no_uploads(ctx));
    }
    let skip = ctx.request.post_field("skip-cropping").is_some_and(|v| !v.is_empty() && v != "0");
    let create_new = ctx.request.post_field("create-new-attachment").is_some_and(|v| !v.is_empty() && v != "0");
    if skip && !support.flex_height && !support.flex_width {
        return Outcome::Die(format!("<h1>{}</h1><p>{}</p>", tr("An error occurred while processing your header image."), tr("The active theme does not support a flexible sized header image.")));
    }
    let oitar = form_number(ctx, "oitar");
    let scale = |value: f64| if oitar > 1.0 { value * oitar } else { value };
    let (x1, y1, crop_w, crop_h) = (scale(form_number(ctx, "x1")), scale(form_number(ctx, "y1")), scale(form_number(ctx, "width")), scale(form_number(ctx, "height")));
    let attachment_id = super::media_upload::php_int(ctx.request.post_field("attachment_id").unwrap_or_default()).unsigned_abs();
    let uploads = super::ajax_media::uploads_dir(ctx.env);
    let attached = ctx.env.db.post_meta_value(attachment_id, "_wp_attached_file").await.ok().flatten().unwrap_or_default();
    let original = uploads.join(&attached);

    // `get_header_dimensions()`.
    let (width, height) = (crop_w.trunc().abs() as u64, crop_h.trunc().abs() as u64);
    let max = max_width(&support);
    let ratio = |n: u64| if width == 0 { 0 } else { (n as f64 * (max as f64 / width as f64)).trunc().abs() as u64 };
    let dst_height = if support.flex_height && (!support.flex_width || width > max) {
        ratio(height)
    } else if support.flex_height && support.flex_width {
        height
    } else {
        support.height
    };
    let dst_width = if support.flex_width && (!support.flex_height || width > max) {
        ratio(width)
    } else if support.flex_width && support.flex_height {
        width
    } else {
        support.width
    };

    let failed = || Outcome::Die(tr("Image could not be processed. Please go back and try again."));
    let Some(writer) = ctx.env.writer else { return failed() };
    let Some(parent) = ctx.env.db.post(attachment_id).await.ok().flatten() else { return failed() };
    let name = original.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mime = parent.post_mime_type.clone();
    let cropped = if !skip {
        let (source, mime) = (original.clone(), mime.clone());
        let (x, y, w, h, dw, dh) = (x1 as i64 as u32, y1 as i64 as u32, crop_w as i64 as u32, crop_h as i64 as u32, dst_width as u32, dst_height as u32);
        media::blocking(move || media::crop_image(&source, x, y, w, h, dw, dh, &mime)).await.ok()
    } else if create_new {
        // `_copy_image_file()`.
        let dir = original.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
        let target = dir.join(media::unique_filename(&dir, &format!("copy-{name}")));
        std::fs::copy(&original, &target).ok().map(|_| target)
    } else {
        original.is_file().then(|| original.clone())
    };
    let Some(cropped) = cropped else { return failed() };

    // `wp_copy_parent_attachment_properties()`.
    let parent_url = rustpress_blocks::images::attachment_url(site, &attached);
    let parent_basename = parent_url.rsplit('/').next().unwrap_or_default().to_owned();
    let cropped_name = cropped.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let url = parent_url.replace(&parent_basename, &cropped_name);
    let image_type = image::ImageFormat::from_path(&cropped).ok().map(|format| format.to_mime_type().to_owned()).unwrap_or_else(|| "image/jpeg".to_owned());
    let sanitized_title = crate::sanitize::file_name(&parent.post_title, &site.locale());
    let parent_stem = parent_basename.rsplit_once('.').map_or(parent_basename.as_str(), |(stem, _)| stem);
    let use_original_title = !parent.post_title.trim().is_empty() && parent_basename != sanitized_title && parent_stem != sanitized_title;
    let title = if use_original_title { parent.post_title.clone() } else { cropped_name.clone() };
    let description = if parent.post_content.trim().is_empty() { url.clone() } else { parent.post_content.clone() };
    let mut data = attachment_data(ctx, title, description, image_type.clone(), url.clone());
    if !parent.post_excerpt.trim().is_empty() {
        data.post_excerpt = Some(parent.post_excerpt.clone());
    }
    let alt = ctx.env.db.post_meta_value(attachment_id, "_wp_attachment_image_alt").await.ok().flatten().filter(|alt| !alt.trim().is_empty());
    let relative = cropped.strip_prefix(&uploads).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_else(|_| cropped_name.clone());

    // `insert_attachment()`: the metadata of the new file, with the original as `attachment_parent`.
    let Some(new_id) = insert_attachment(ctx, writer, data, &relative, "custom-header", alt.as_deref()).await else { return failed() };
    let (options, uploads_dir, rel, generated_mime) = (site.options.clone(), uploads.clone(), relative.clone(), image_type.clone());
    if let Some((metadata, final_relative, _)) = media::blocking(move || media::generate_image_metadata(&options, &uploads_dir, &rel, &generated_mime)).await {
        let mut metadata = metadata;
        if let Value::Array(items) = &mut metadata {
            items.push((Key::Str(b"attachment_parent".to_vec()), Value::Int(attachment_id as i64)));
        }
        if final_relative != relative {
            writer.update_post_meta(new_id, "_wp_attached_file", &final_relative).await.or_log();
        }
        writer.update_post_meta(new_id, "_wp_attachment_metadata", &String::from_utf8_lossy(&rustpress_phpser::serialize(&metadata))).await.or_log();
    }
    let attached_url = ctx.env.db.post_meta_value(new_id, "_wp_attached_file").await.ok().flatten().map(|file| rustpress_blocks::images::attachment_url(site, &file)).unwrap_or(url);
    state.set_header_image_data(ctx, &attached_url, new_id, dst_width, dst_height).await;

    // Cleanup.
    let midsize = original.with_file_name(format!("midsize-{name}"));
    if midsize.is_file() {
        let _ = std::fs::remove_file(&midsize);
    }
    if !create_new && !skip {
        let _ = std::fs::remove_file(&original);
    }
    state.uploaded = header::uploaded_header_images(site, &site.settings.stylesheet).await.unwrap_or_default();
    Outcome::Page(step_1(ctx, &*state, true))
}

/// Where the head stops when `take_action()` dies.
const CUT: &str = "\u{0}rustpress-cut\u{0}";
