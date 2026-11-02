//! ACF's fields in the comment form of the website (`acf_form_comment::comment_form_field_comment()`) with what
//! `acf_enqueue_scripts()` brings along on the website: the scripts and styles of the inputs (`ACF_Assets` and the
//! `input_admin_enqueue_scripts()` of the field types, on `wp_enqueue_scripts` at priority 20) and the data of
//! `ACF_Assets::print_footer_scripts()` (`acf.data`).

use rustpress_core::native::{Assets, CommentFormContext};
use rustpress_core::{Result, Site};
use rustpress_db::MetaKind;
use rustpress_phpser::Value;
use serde_json::{Map, Value as Json, json};

use super::fields::{FieldGroup, MatchContext, Screen};
use super::render::{Attrs, Ctx, RenderField, hidden_input, render_field_wrap};
use super::value::{Object, get_value};

const PLUGIN_DIR: &str = "wp-content/plugins/advanced-custom-fields";

/// `acf_get_url( $path )`.
fn acf_url(site: &Site, path: &str) -> String {
    site.site_url(&format!("{PLUGIN_DIR}/{path}"))
}

fn tr(site: &Site, text: &str) -> String {
    site.trd("acf", text)
}

fn trx(site: &Site, text: &str, context: &str) -> String {
    site.i18n().pgettext("acf", context, text)
}

/// `esc_html__( $text, 'acf' )`.
fn esc_tr(site: &Site, text: &str) -> String {
    rustpress_core::formatting::esc_html(&tr(site, text))
}

/// The texts ACF hands its scripts (`acf_localize_text()`), in the order they are added; only those the language
/// translates go to `acfL10n`.
fn texts(site: &Site) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    let mut add = |key: &'static str, value: String| match out.iter_mut().find(|(k, _)| *k == key) {
        Some(slot) => slot.1 = value,
        None => out.push((key, value)),
    };
    for text in ["Are you sure?", "Yes", "No", "Remove", "Cancel", "Close modal", "The changes you made will be lost if you navigate away from this page"] {
        add(text, esc_tr(site, text));
    }
    add("Toggle panel", tr(site, "Toggle panel"));
    for text in ["Validation successful", "Validation failed", "1 field requires attention", "%d fields require attention", "An ACF Block on this page requires attention before you can save.", "Edit field group"] {
        add(text, esc_tr(site, text));
    }
    // The field types (`acf/input/admin_enqueue_scripts`): image, file, relationship, Google Map.
    for text in ["Select Image", "Edit Image", "Update Image", "All images", "Select File", "Edit File", "Update File", "Maximum values reached ( {max} values )", "Loading", "No matches found", "Sorry, this browser does not support geolocation"] {
        add(text, tr(site, text));
    }
    // `ACF_Media::enqueue_scripts()` on `acf/enqueue_scripts`.
    add("Select.verb", trx(site, "Select", "verb"));
    add("Edit.verb", trx(site, "Edit", "verb"));
    add("Update.verb", trx(site, "Update", "verb"));
    for text in ["Uploaded to this post", "Expand Details", "Collapse Details", "Restricted", "All images"] {
        add(text, tr(site, text));
    }
    out
}

/// `wp_localize_script( 'acf', 'acfL10n', $text )` with the texts that differ from English.
fn localized_texts(site: &Site) -> Option<String> {
    let mut map = Map::new();
    for (key, value) in texts(site) {
        if key.replace(".verb", "") != value {
            map.insert(key.to_owned(), Json::String(rustpress_core::formatting::html_entity_decode(&value)));
        }
    }
    if map.is_empty() {
        return None;
    }
    let json = rustpress_blocks::php_json::encode_with(&Json::Object(map), rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E");
    Some(format!("var acfL10n = {json};"))
}

/// The scripts and styles of `acf_enqueue_scripts()` on the website: `ACF_Assets::register_scripts()`, then what
/// `enqueue_scripts()` and the field types enqueue, in that order.
pub(super) fn enqueue(site: &Site, wp_version: &str, assets: &mut Assets, admin: bool) {
    let ver = super::VERSION;
    let mut scripts = Assets::default();
    let acf = scripts.script("acf", acf_url(site, "assets/build/js/acf.min.js"), &["jquery"], ver, false);
    acf.registered_only = true;
    acf.extra = localized_texts(site);
    scripts.script("acf-input", acf_url(site, "assets/build/js/acf-input.min.js"), &["jquery", "jquery-ui-sortable", "jquery-ui-resizable", "acf", "wp-a11y"], ver, false);
    scripts.style("acf-global", acf_url(site, "assets/build/css/acf-global.min.css"), &["dashicons"], ver, "all").registered_only = true;
    scripts.style("acf-input", acf_url(site, "assets/build/css/acf-input.min.css"), &["acf-global"], ver, "all");
    // `acf_field_select`.
    scripts.script("select2", acf_url(site, "assets/inc/select2/4/select2.full.min.js"), &["jquery"], "4.0.13", false);
    scripts.style("select2", acf_url(site, "assets/inc/select2/4/select2.min.css"), &[], "4.0.13", "all");
    // `acf_field_date_picker`: the date picker of WordPress; the time picker's script needs it in the head.
    scripts.core_script("jquery-ui-datepicker", false);
    scripts.style("acf-datepicker", acf_url(site, "assets/inc/datepicker/jquery-ui.min.css"), &[], "1.11.4", "all");
    // `acf_field_date_and_time_picker`.
    scripts.script("acf-timepicker", acf_url(site, "assets/inc/timepicker/jquery-ui-timepicker-addon.min.js"), &["jquery-ui-datepicker"], "1.6.1", false);
    scripts.style("acf-timepicker", acf_url(site, "assets/inc/timepicker/jquery-ui-timepicker-addon.min.css"), &[], "1.6.1", "all");
    // `acf_field_color_picker`: outside the administration it registers Iris and the color picker itself.
    if !admin {
        scripts.script("iris", site.site_url("wp-admin/js/iris.min.js"), &["jquery-ui-draggable", "jquery-ui-slider", "jquery-touch-punch"], "1.0.7", false).registered_only = true;
    }
    scripts.script("acf-color-picker-alpha", acf_url(site, "assets/inc/color-picker-alpha/wp-color-picker-alpha.js"), &["jquery", "wp-color-picker"], "3.0.0", false);
    scripts.core_style("wp-color-picker");
    if admin {
        scripts.core_script("wp-color-picker", false);
    } else {
        scripts.script("wp-color-picker", site.site_url("wp-admin/js/color-picker.min.js"), &["iris"], wp_version, false);
    }
    for mut asset in scripts.styles {
        asset.priority = 20;
        assets.styles.push(asset);
    }
    for mut asset in scripts.scripts {
        asset.priority = 20;
        assets.scripts.push(asset);
    }
}

/// `acf_get_browser()`.
pub(super) fn browser(user_agent: Option<&str>) -> &'static str {
    let Some(agent) = user_agent else { return "" };
    [("Firefox", "firefox"), ("Trident", "msie"), ("MSIE", "msie"), ("Edge", "edge"), ("Chrome", "chrome"), ("Safari", "safari")].iter().find(|(k, _)| agent.contains(k)).map_or("", |(_, v)| v)
}

/// `acf_get_locale()`.
fn acf_locale(site: &Site) -> String {
    let locale = site.locale();
    match locale.as_str() {
        "az_TR" => "az".to_owned(),
        "zh_HK" => "zh_TW".to_owned(),
        "fr_BE" => "fr_FR".to_owned(),
        "nn_NO" => "nb_NO".to_owned(),
        "fa_AF" => "fa_IR".to_owned(),
        "ru_UA" => "ru_RU".to_owned(),
        _ => locale,
    }
}

/// The form data of `acf_form_data()` that the footer reads back.
pub struct FormData {
    /// `null` without form data.
    pub screen: Json,
    pub post_id: Json,
    pub validation: Json,
}

/// `ACF_Assets::print_footer_scripts()`: `acf.data` with what the field types localized, and `acf.doAction(
/// 'prepare' )`.
pub(super) fn footer(site: &Site, form: &FormData, wp_version: &str, nonce: &str, user_agent: Option<&str>, can: &(dyn Fn(&str) -> bool + Send + Sync)) -> String {
    let locale = site.locale_data();
    let mut data = Map::new();
    let select2 = [
        ("matches_1", "One result is available, press enter to select it."),
        ("matches_n", "%d results are available, use up and down arrow keys to navigate."),
        ("matches_0", "No matches found"),
        ("input_too_short_1", "Please enter 1 or more characters"),
        ("input_too_short_n", "Please enter %d or more characters"),
        ("input_too_long_1", "Please delete 1 character"),
        ("input_too_long_n", "Please delete %d characters"),
        ("selection_too_long_1", "You can only select 1 item"),
        ("selection_too_long_n", "You can only select %d items"),
        ("load_more", "Loading more results&hellip;"),
        ("searching", "Searching&hellip;"),
        ("load_fail", "Loading failed"),
    ];
    data.insert("select2L10n".into(), Json::Object(select2.iter().map(|(k, text)| ((*k).to_owned(), Json::String(trx(site, text, &format!("Select2 JS {k}"))))).collect()));
    // `add_query_arg()` of the API's arguments (no key, no client).
    let maps = format!("https://maps.googleapis.com/maps/api/js?libraries=places&ver=3&callback=Function.prototype&language={}", acf_locale(site));
    data.insert("google_map_api".into(), Json::String(maps));
    let date = |key: &str, text: &str| Json::String(trx(site, text, &format!("Date Picker JS {key}")));
    data.insert(
        "datePickerL10n".into(),
        json!({
            "closeText": date("closeText", "Done"),
            "currentText": date("currentText", "Today"),
            "nextText": date("nextText", "Next"),
            "prevText": date("prevText", "Prev"),
            "weekHeader": date("weekHeader", "Wk"),
            "monthNames": locale.month,
            "monthNamesShort": locale.month_abbrev,
            "dayNames": locale.weekday,
            "dayNamesMin": locale.weekday_initial,
            "dayNamesShort": locale.weekday_abbrev,
        }),
    );
    let dt = |key: &str, text: &str| Json::String(trx(site, text, &format!("Date Time Picker JS {key}")));
    data.insert(
        "dateTimePickerL10n".into(),
        json!({
            "timeOnlyTitle": dt("timeOnlyTitle", "Choose Time"),
            "timeText": dt("timeText", "Time"),
            "hourText": dt("hourText", "Hour"),
            "minuteText": dt("minuteText", "Minute"),
            "secondText": dt("secondText", "Second"),
            "millisecText": dt("millisecText", "Millisecond"),
            "microsecText": dt("microsecText", "Microsecond"),
            "timezoneText": dt("timezoneText", "Time Zone"),
            "currentText": dt("currentText", "Now"),
            "closeText": dt("closeText", "Done"),
            "selectText": dt("selectText", "Select"),
            "amNames": [dt("amText", "AM"), dt("amTextShort", "A")],
            "pmNames": [dt("pmText", "PM"), dt("pmTextShort", "P")],
        }),
    );
    data.insert("colorPickerL10n".into(), json!({"hex_string": tr(site, "Hex String"), "rgba_string": tr(site, "RGBA String")}));
    data.insert(
        "iconPickerA11yStrings".into(),
        json!({
            "noResultsForSearchTerm": esc_tr(site, "No results found for that search term"),
            "newResultsFoundForSearchTerm": esc_tr(site, "The available icons matching your search query have been updated in the icon picker below."),
        }),
    );
    data.insert("iconPickeri10n".into(), Json::Object(super::dashicons::DASHICONS.iter().map(|(name, label)| ((*name).to_owned(), Json::String(esc_tr(site, label)))).collect()));
    data.insert("mimeTypeIcon".into(), Json::String(site.site_url("wp-includes/images/media/default.png")));
    let mimes = rustpress_content::media::allowed_mime_types_for(site, &[], can, false, false);
    data.insert("mimeTypes".into(), Json::Object(mimes.into_iter().map(|(k, v)| (k, Json::String(v))).collect()));
    data.insert("admin_url".into(), Json::String(format!("{}/", site.site_url("wp-admin"))));
    data.insert("ajaxurl".into(), Json::String(site.site_url("wp-admin/admin-ajax.php")));
    data.insert("nonce".into(), Json::String(nonce.to_owned()));
    data.insert("acf_version".into(), Json::String(super::VERSION.to_owned()));
    data.insert("wp_version".into(), Json::String(wp_version.to_owned()));
    data.insert("browser".into(), Json::String(browser(user_agent).to_owned()));
    data.insert("locale".into(), Json::String(acf_locale(site)));
    data.insert("rtl".into(), Json::Bool(site.is_rtl()));
    data.insert("screen".into(), form.screen.clone());
    data.insert("post_id".into(), form.post_id.clone());
    data.insert("validation".into(), form.validation.clone());
    data.insert("editor".into(), Json::String("classic".into()));
    data.insert("is_pro".into(), Json::Bool(false));
    data.insert("debug".into(), Json::Bool(false));
    data.insert("StrictMode".into(), Json::Bool(false));
    let json = rustpress_blocks::php_json::encode_with(&Json::Object(data), rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: false });
    format!("<script>\nacf.data = {json};\n</script>\n<script>\nacf.doAction( 'prepare' );\n</script>\n")
}

/// `acf_form_data()`: the hidden inputs ACF reads when the form is sent.
pub fn form_data(screen: &str, post_id: Value, nonce: &str) -> String {
    let inputs = [("screen", super::php::string(screen)), ("post_id", post_id), ("validation", Value::Bool(true)), ("nonce", super::php::string(nonce)), ("changed", Value::Int(0))];
    let mut out = "\t<div id=\"acf-form-data\" class=\"acf-hidden\">\n\t\t".to_owned();
    for (name, value) in inputs {
        out.push_str(&hidden_input(&Attrs::new(&[("id", super::php::string(&format!("_acf_{name}"))), ("name", super::php::string(&format!("_acf_{name}"))), ("value", value)])));
    }
    out.push_str("\t</div>\n\t");
    out
}

/// `acf_form_comment::comment_form_field_comment()`.
pub async fn comment_form_field_comment(groups: &[FieldGroup], ctx: &CommentFormContext<'_>, html: String) -> Result<String> {
    let site = ctx.site;
    let matcher = MatchContext { site, viewer: ctx.query.viewer.as_ref() };
    let screen = Screen::Comment { post_type: ctx.post.post_type.clone() };
    let mut visible = Vec::new();
    for group in groups {
        if group.visible(&screen, &matcher).await? {
            visible.push(group);
        }
    }
    if visible.is_empty() {
        return Ok(html);
    }
    let wp_version = wp_version(ctx.services.wp_root());
    let form = FormData { screen: Json::String("comment".into()), post_id: Json::Bool(false), validation: Json::Bool(true) };
    let first = {
        let mut state = ctx.state.lock().expect("plugin state");
        let first = state.flags.insert("acf_enqueue_scripts");
        if first {
            enqueue(site, &wp_version, &mut state.assets, false);
        }
        first
    };
    if first {
        let can = |capability: &str| ctx.query.viewer.as_ref().is_some_and(|v| v.can(capability));
        let footer = footer(site, &form, &wp_version, &ctx.query.create_nonce("acf_nonce"), ctx.query.user_agent.as_deref(), &can);
        ctx.state.lock().expect("plugin state").footer_scripts.push(footer);
    }
    let nonce = |action: &str| ctx.query.create_nonce(action);
    let can = |capability: &str| ctx.query.viewer.as_ref().is_some_and(|v| v.can(capability));
    // `user_can_richedit()` for a visitor: the browser decides.
    let rich_edit = rich_edit_browser(ctx.query.user_agent.as_deref().unwrap_or_default());
    // `_acf_settings_uploader()`: the media library only for who may upload files.
    let uploader = if ctx.query.viewer.as_ref().is_some_and(|v| v.can("upload_files")) { "wp" } else { "basic" };
    let render = Ctx { site, nonce: &nonce, services: ctx.services, admin: false, uploader, wants_uploader: Default::default(), can: &can, rich_edit, default_editor: if rich_edit { "tinymce" } else { "html" }, wysiwyg_height: 300 };
    let mut out = html;
    out.push_str(&form_data("comment", Value::Bool(false), &nonce("comment")));
    out.push_str("<div class=\"acf-comment-fields acf-fields -clear\">");
    // `acf_get_value( false, $field )`: `acf_decode_post_id( false )` names no object, so the fields show their
    // defaults (after the types' `load_value()`).
    let object = Object { kind: MetaKind::Post, id: 0 };
    for group in visible {
        let instruction = group.setting("instruction_placement", "label");
        for field in &group.fields {
            let mut f = RenderField::load(field, &render);
            if matches!(f.get("value"), None | Some(Value::Null)) {
                f.set("value", get_value(&site.db, object, field, &field.name).await?);
            }
            out.push_str(&render_field_wrap(&render, f, "p", &instruction).await?);
        }
    }
    out.push_str("</div>");
    Ok(out)
}

/// The browser part of `user_can_richedit()`.
pub(super) fn rich_edit_browser(agent: &str) -> bool {
    let mobile = ["Mobile", "Android", "Silk/", "Kindle", "BlackBerry", "Opera Mini", "Opera Mobi"].iter().any(|m| agent.contains(m));
    let ie = agent.contains("MSIE") || agent.contains("Trident/");
    let safari = agent.contains("Safari") && !agent.contains("Chrome") && !agent.contains("Chromium");
    if safari {
        return !mobile || agent.split("AppleWebKit/").nth(1).and_then(|r| r.split('.').next()).and_then(|v| v.parse::<u32>().ok()).is_some_and(|v| v >= 534);
    }
    !ie && (agent.contains("Gecko") || agent.contains("Chrome") || agent.contains("Edg/") || (agent.contains("Opera") && !mobile))
}

/// `$wp_version` of the WordPress files.
fn wp_version(wp_root: &std::path::Path) -> String {
    std::fs::read_to_string(wp_root.join("wp-includes").join("version.php"))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("$wp_version = '").and_then(|r| r.split('\'').next().map(str::to_owned))))
        .unwrap_or_default()
}
