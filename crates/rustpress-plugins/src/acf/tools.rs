//! ACF's own pages below its menu besides the lists: the options pages preview of the free version
//! (`ACF_Admin_Options_Preview`, `views/options-page-preview.php`) and the tools (`acf_admin_tools`).

use rustpress_core::Result;
use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::native::{AdminContext, Assets, FormPart, PluginMetaBox};
use rustpress_phpser::Value;

use super::php::{key, string};

const OPTIONS_PREVIEW: &str = "acf_options_preview";
const TOOLS: &str = "acf-tools";

fn tr(site: &Site, text: &str) -> String {
    site.trd("acf", text)
}

/// The page of this module the request is for.
fn page<'a>(ctx: &'a AdminContext<'a>) -> Option<&'a str> {
    super::admin_lists::plugin_page(ctx).filter(|p| matches!(*p, OPTIONS_PREVIEW | TOOLS))
}

/// `load()` of both pages: `acf_enqueue_scripts()`.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    if page(ctx).is_some() {
        super::form::enqueue_with_texts(ctx.site, ctx.wp_version, assets, true, &[]);
    }
}

/// `ACF_Admin_Options_Preview::admin_body_class()` and `acf_admin_tools::admin_body_class()`.
pub fn admin_body_class(ctx: &AdminContext<'_>, classes: String) -> String {
    match page(ctx) {
        Some(OPTIONS_PREVIEW) => format!("{classes} acf-admin-page acf-internal-post-type acf-options-preview acf-no-options-pages"),
        Some(TOOLS) => format!("{classes} acf-admin-page"),
        _ => classes,
    }
}

/// `acf.data` of both pages (`acf_enqueue_scripts()` without form data).
pub fn footer_scripts(ctx: &AdminContext<'_>) -> Option<String> {
    page(ctx)?;
    let form = super::form::FormData::default();
    Some(super::form::footer_with(ctx.site, &form, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, super::form::Uploader::No, ""))
}

/// `acf_admin_tools::html()` with `metabox_html()`: the boxes of the tools (all, or the one of `tool`) in
/// `views/tools/tools.php`.
pub async fn admin_page_parts(slug: &str, ctx: &AdminContext<'_>) -> Result<Option<(Vec<FormPart>, Vec<PluginMetaBox>)>> {
    if slug != TOOLS {
        return Ok(None);
    }
    let site = ctx.site;
    let active = ctx.query.iter().rev().find(|(k, _)| k == "tool").map(|(_, v)| v.as_str()).filter(|v| !v.is_empty());
    let services = super::edit_comment::AdminServices(ctx.wp_root.to_owned());
    let render = super::edit_post::render_ctx(ctx, &services);
    let mut boxes = Vec::new();
    // `ACF_Admin_Tool_Export::initialize()`: the active export is "Generate PHP".
    let export_title = if active == Some("export") { format!("{} - {}", tr(site, "Export Field Groups"), tr(site, "Generate PHP")) } else { tr(site, "Export Field Groups") };
    for (name, title) in [("export", export_title), ("import", tr(site, "Import Field Groups"))] {
        if active.is_some_and(|a| a != name) {
            continue;
        }
        let html = match name {
            "export" if active == Some("export") => export_single(ctx, &render).await?,
            "export" => export_archive(ctx, &render).await?,
            _ => import_html(&render).await?,
        };
        let form = if name == "import" { "<form method=\"post\" onsubmit=\"acf.disableForm(event)\">" } else { "<form method=\"post\">" };
        let html = format!("{form}{html}<input type=\"hidden\" name=\"_acf_nonce\" value=\"{}\" /></form>", esc_attr(&(ctx.nonce)(name)));
        boxes.push(PluginMetaBox { id: format!("acf-admin-tool-{name}"), title: super::render::esc_html_acf(&string(&title)), context: "normal".into(), priority: "default".into(), html });
    }
    let (class, tool) = match active {
        Some(active) => ("single", format!(" tool-{active}")),
        None => ("grid", String::new()),
    };
    let back = match active {
        Some(_) => format!("\t\t\t<a class=\"page-title-action\" href=\"{}\">{}</a>\n\t\t", esc_url(&site.site_url("wp-admin/edit.php?post_type=acf-field-group&page=acf-tools")), esc_html(&tr(site, "Back to all tools"))),
        None => String::new(),
    };
    let head = format!("<div id=\"acf-admin-tools\" class=\"wrap{}\">\n\n\t<h1>\n\t\t{}\t\t{back}\t</h1>\n\n\t<div class=\"acf-meta-box-wrap -{}\">\n\t\t", esc_attr(&tool), esc_html(&tr(site, "Tools")), esc_attr(class));
    let tail = format!("\t</div>\n\n\t{}</div>\n", super::admin_lists::pro_features(site));
    Ok(Some((vec![FormPart::Html(head), FormPart::MetaBoxes("normal".into()), FormPart::Html(tail)], boxes)))
}

/// `acf_render_field_wrap( $field )`.
async fn wrap(render: &super::render::Ctx<'_>, items: Vec<(&str, Value)>) -> Result<String> {
    super::render::render_field_wrap(render, super::render::RenderField::raw(items.into_iter().map(|(k, v)| (key(k), v)).collect(), Vec::new()), "div", "label").await
}

/// `ACF_Admin_Tool_Export::get_selected_keys()`: `keys`, `taxonomy_keys`, `post_type_keys` and
/// `ui_options_page_keys` of the POST body, or `+`-separated in the query string.
fn selected_keys(ctx: &AdminContext<'_>) -> Vec<String> {
    let mut all = Vec::new();
    for name in ["keys", "taxonomy_keys", "post_type_keys", "ui_options_page_keys"] {
        let posted: Vec<String> = match rustpress_core::native::form_value(ctx.form, name) {
            Some(Value::Array(items)) => items.into_iter().map(|(_, v)| super::php::strval(Some(&v))).collect(),
            Some(value) if super::php::truthy(Some(&value)) => vec![super::php::strval(Some(&value))],
            _ => Vec::new(),
        };
        if !posted.is_empty() {
            all.extend(posted);
        } else if let Some(keys) = ctx.query.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str()).filter(|v| !v.is_empty() && *v != "0") {
            all.extend(keys.replace(' ', "+").split('+').map(str::to_owned));
        }
    }
    all
}

/// `ACF_Admin_Tool_Export::html_field_selection()`: a checkbox list per kind of item (field groups always, the others
/// when there are some).
async fn field_selection(ctx: &AdminContext<'_>, render: &super::render::Ctx<'_>) -> Result<String> {
    let site = ctx.site;
    let selected = Value::Array(selected_keys(ctx).into_iter().enumerate().map(|(i, k)| (rustpress_phpser::Key::Int(i as i64), string(&k))).collect());
    let mut out = String::new();
    for (post_type, label, name, prefix) in [
        ("acf-field-group", "Select Field Groups", "keys", "group_"),
        ("acf-post-type", "Select Post Types", "post_type_keys", "post_type_"),
        ("acf-taxonomy", "Select Taxonomies", "taxonomy_keys", "taxonomy_"),
        ("acf-ui-options-page", "Select Options Pages", "ui_options_page_keys", "ui_options_page_"),
    ] {
        // `acf_get_internal_post_type_posts()` with a valid key.
        let items: Vec<_> = site.db.acf_posts(post_type).await?.into_iter().filter(|p| p.name.starts_with(prefix)).collect();
        if items.is_empty() && post_type != "acf-field-group" {
            continue;
        }
        let choices = Value::Array(items.iter().map(|p| (key(&p.name), string(&esc_html(&p.title)))).collect());
        out.push_str(&wrap(render, vec![("label", string(&tr(site, label))), ("type", string("checkbox")), ("name", string(name)), ("prefix", Value::Bool(false)), ("value", selected.clone()), ("toggle", Value::Bool(true)), ("choices", choices)]).await?);
    }
    Ok(out)
}

/// `ACF_Admin_Tool_Export::html_archive()`.
async fn export_archive(ctx: &AdminContext<'_>, render: &super::render::Ctx<'_>) -> Result<String> {
    let site = ctx.site;
    Ok(format!(
        "\t\t<div class=\"acf-postbox-header\">\n\t\t\t<h2 class=\"acf-postbox-title\">{}</h2>\n\t\t\t<div class=\"acf-tip\"><i tabindex=\"0\" class=\"acf-icon acf-icon-help acf-js-tooltip\" title=\"{}\">?</i></div>\n\t\t</div>\n\t\t<div class=\"acf-postbox-inner\">\n\t\t\t<div class=\"acf-fields\">\n\t\t\t\t{}\t\t\t</div>\n\t\t\t<p class=\"acf-submit acf-actions-strip\">\n\t\t\t\t<button type=\"submit\" name=\"action\" class=\"acf-btn acf-button-primary\" value=\"download\">{}</button>\n\t\t\t\t<button type=\"submit\" name=\"action\" class=\"acf-btn acf-btn-secondary\" value=\"generate\">{}</button>\n\t\t\t</p>\n\t\t</div>\n\t\t\t",
        esc_html(&tr(site, "Export")),
        esc_attr(&tr(site, "Select the items you would like to export and then select your export method. Export As JSON to export to a .json file which you can then import to another ACF installation. Generate PHP to export to PHP code which you can place in your theme.")),
        field_selection(ctx, render).await?,
        esc_html(&tr(site, "Export As JSON")),
        esc_html(&tr(site, "Generate PHP")),
    ))
}

/// `ACF_Admin_Tool_Import::html()` (without the import from Custom Post Type UI, which needs that plugin).
async fn import_html(render: &super::render::Ctx<'_>) -> Result<String> {
    let site = render.site;
    let file = wrap(
        render,
        vec![
            ("label", string(&tr(site, "Select JSON File"))),
            ("type", string("file")),
            ("name", string("acf_import_file")),
            ("value", Value::Bool(false)),
            ("uploader", string("basic")),
            ("mime_types", string("application/json,application/x-json,application/x-javascript,text/javascript,text/x-javascript,text/json")),
            ("instructions", string(&tr(site, "Import JSON containing field groups, post types, or taxonomies (trusted sources only)"))),
        ],
    )
    .await?;
    Ok(format!(
        "\t\t<div class=\"acf-postbox-header\">\n\t\t\t<h2 class=\"acf-postbox-title\">{}</h2>\n\t\t\t<div class=\"acf-tip\"><i tabindex=\"0\" class=\"acf-icon acf-icon-help acf-js-tooltip\" title=\"{}\">?</i></div>\n\t\t</div>\n\t\t<div class=\"acf-postbox-inner\">\n\t\t\t<div class=\"acf-fields\">\n\t\t\t\t{file}\t\t\t</div>\n\t\t\t<p class=\"acf-submit\">\n\t\t\t\t<button type=\"submit\" class=\"acf-btn\" name=\"import_type\" value=\"json\">\n\t\t\t\t\t{}\t\t\t\t</button>\n\t\t\t</p>\n\n\t\t\t\t\t</div>\n\t\t\t",
        esc_html(&tr(site, "Import")),
        esc_attr(&tr(site, "Choose an ACF JSON file to import. Use only files from trusted sources, then click Import.")),
        esc_html(&tr(site, "Import JSON")),
    ))
}

/// `acf_add_admin_notice( $text, $type )` as `ACF_Admin_Notice::render()` prints it (dismissible, not persisted).
pub(super) fn admin_notice(text: &str, kind: &str) -> String {
    let trimmed = text.trim();
    let shown = if rustpress_core::formatting::wp_strip_all_tags(trimmed).trim().ends_with('.') { trimmed.to_owned() } else { format!("{trimmed}.") };
    format!(
        "<div class=\"acf-admin-notice notice notice-{} is-dismissible\" data-persisted=\"false\" data-persist-id=\"{:x}\">{}</div>",
        esc_attr(kind),
        md5::compute(text.as_bytes()),
        super::render::esc_html_acf(&string(&rustpress_core::formatting::wpautop(&shown)))
    )
}

/// The internal post type of a key (`acf_determine_internal_post_type()`).
fn key_type(key: &str) -> Option<&'static str> {
    [("group_", "acf-field-group"), ("field_", "acf-field"), ("post_type_", "acf-post-type"), ("taxonomy_", "acf-taxonomy"), ("ui_options_page_", "acf-ui-options-page")].into_iter().find(|(prefix, _)| key.starts_with(prefix)).map(|(_, t)| t)
}

/// `acf_prepare_field_for_export()` of a loaded field, with the sub fields of a group
/// (`acf_field_group::prepare_field_for_export()`).
fn field_for_export(site: &Site, field: &super::fields::Field) -> Value {
    let loaded = super::save_group::loaded(site, field);
    let mut items = loaded.items;
    if !field.sub_fields.is_empty() {
        let subs: Vec<(rustpress_phpser::Key, Value)> = field.sub_fields.iter().enumerate().map(|(i, f)| (rustpress_phpser::Key::Int(i as i64), field_for_export(site, f))).collect();
        super::php::set(&mut items, "sub_fields", Value::Array(subs));
    }
    items.retain(|(k, _)| !["ID", "prefix", "value", "menu_order", "id", "class", "parent", "_name", "_prepare", "_valid"].iter().any(|name| super::php::key_eq(k, &key(name))));
    Value::Array(items)
}

/// `ACF_Admin_Tool_Export::get_selected()`: each selected item as `acf_prepare_internal_post_type_for_export()`
/// gives it – without `ID`, `local` and `_valid`, a field group with its fields.
async fn selected_items(ctx: &AdminContext<'_>) -> Result<Vec<Value>> {
    let site = ctx.site;
    let mut out = Vec::new();
    for selected in selected_keys(ctx) {
        let Some(post_type) = key_type(&selected) else { continue };
        let Some(post) = site.db.acf_posts(post_type).await?.into_iter().find(|p| p.name == selected) else { continue };
        let defaults = match post_type {
            "acf-field-group" => Value::Array(super::save_group::group_defaults()),
            "acf-post-type" => super::internal::post_type_defaults(),
            "acf-taxonomy" => super::internal::taxonomy_defaults(),
            _ => continue,
        };
        let Value::Array(mut items) = super::internal::validated(&post, defaults) else { continue };
        if post_type == "acf-field-group" {
            let all = site.db.acf_fields().await?;
            let fields: Vec<(rustpress_phpser::Key, Value)> = super::fields::children_of(post.id, &all).iter().enumerate().map(|(i, f)| (rustpress_phpser::Key::Int(i as i64), field_for_export(site, f))).collect();
            super::php::set(&mut items, "fields", Value::Array(fields));
        }
        items.retain(|(k, _)| !["ID", "local", "_valid"].iter().any(|name| super::php::key_eq(k, &key(name))));
        out.push(Value::Array(items));
    }
    Ok(out)
}

/// `load-{$page}` of the tools (`acf_admin_tools::load()` with `check_submit()`): the export's notice, a download
/// (`submit_download()`, `acf_json_encode()` and the date of `date( 'Y-m-d' )`) or the redirect to the PHP of the
/// selection (`submit_generate()`).
pub async fn load(slug: &str, ctx: &AdminContext<'_>) -> Result<Option<rustpress_core::native::PageLoad>> {
    use rustpress_core::native::PageLoad;
    if slug != TOOLS {
        return Ok(None);
    }
    let site = ctx.site;
    let mut notices = String::new();
    let active = ctx.query.iter().rev().find(|(k, _)| k == "tool").map(|(_, v)| v.as_str());
    // `ACF_Admin_Tool_Export::load()`.
    if active == Some("export") {
        let count = selected_keys(ctx).len();
        if count > 0 {
            notices.push_str(&admin_notice(&rustpress_core::i18n::sprintf(&site.i18n().ngettext("acf", "Exported 1 item.", "Exported %s items.", count as i64), &[&count.to_string()]), "success"));
        }
    }
    let nonce = ctx.post_field("_acf_nonce").unwrap_or_default();
    if !nonce.is_empty() && (ctx.verify_nonce)(nonce, "export") {
        match ctx.post_field("action") {
            Some("download") => {
                if selected_keys(ctx).is_empty() {
                    notices.push_str(&admin_notice(&tr(site, "No field groups selected"), "warning"));
                } else {
                    let items = selected_items(ctx).await?;
                    let json = rustpress_core::options::php_to_json(&Value::Array(items.into_iter().enumerate().map(|(i, v)| (rustpress_phpser::Key::Int(i as i64), v)).collect()));
                    let body = format!("{}\r\n", rustpress_blocks::php_json::encode_pretty(&json, rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: true }));
                    // `date()` runs in UTC in WordPress.
                    let date = rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d", ctx.now);
                    let headers = vec![
                        ("Content-Description".to_owned(), "File Transfer".to_owned()),
                        ("Content-Disposition".to_owned(), format!("attachment; filename=acf-export-{date}.json")),
                        ("Content-Type".to_owned(), "application/json; charset=utf-8".to_owned()),
                    ];
                    return Ok(Some(PageLoad::Output { headers, body }));
                }
            }
            Some("generate") => {
                let keys = selected_keys(ctx);
                if keys.is_empty() {
                    notices.push_str(&admin_notice(&tr(site, "No field groups selected"), "warning"));
                } else {
                    return Ok(Some(PageLoad::Redirect(format!("{}&tool=export&keys={}", site.site_url("wp-admin/edit.php?post_type=acf-field-group&page=acf-tools"), keys.join("+")))));
                }
            }
            _ => {}
        }
    }
    if !nonce.is_empty() && (ctx.verify_nonce)(nonce, "import") {
        notices.push_str(&import_submit(ctx).await?);
    }
    Ok((!notices.is_empty()).then_some(PageLoad::Notices(notices)))
}

/// `json_decode( $json, true )`.
fn json_value(json: &serde_json::Value) -> Value {
    use serde_json::Value as J;
    match json {
        J::Null => Value::Null,
        J::Bool(b) => Value::Bool(*b),
        J::Number(n) => match n.as_i64() {
            Some(i) => Value::Int(i),
            None => Value::Float(n.as_f64().unwrap_or_default()),
        },
        J::String(s) => string(s),
        J::Array(items) => Value::Array(items.iter().enumerate().map(|(i, v)| (rustpress_phpser::Key::Int(i as i64), json_value(v))).collect()),
        // Numeric keys become integers, as in PHP arrays.
        J::Object(map) => Value::Array(map.iter().map(|(k, v)| (k.parse::<i64>().ok().filter(|i| i.to_string() == *k).map_or_else(|| key(k), rustpress_phpser::Key::Int), json_value(v))).collect()),
    }
}

/// `ACF_Admin_Tool_Import::submit()` for a JSON file (the import from Custom Post Type UI needs that plugin): the
/// checks of the upload, then each item over the one with its key, if any; the notice.
async fn import_submit(ctx: &AdminContext<'_>) -> Result<String> {
    let site = ctx.site;
    let warning = |text: &str| admin_notice(&tr(site, text), "warning");
    let Some(file) = ctx.files.iter().find(|f| f.field == "acf_import_file").filter(|f| !f.bytes.is_empty()) else { return Ok(warning("No file selected")) };
    // `pathinfo( $name, PATHINFO_EXTENSION )`.
    let base = file.filename.rsplit(['/', '\\']).next().unwrap_or_default();
    if base.rsplit_once('.').map(|(_, ext)| ext) != Some("json") {
        return Ok(warning("Incorrect file type"));
    }
    let json = serde_json::from_slice::<serde_json::Value>(&file.bytes).map(|j| json_value(&j)).unwrap_or(Value::Null);
    let mut list = match json {
        Value::Array(items) if !items.is_empty() => items,
        _ => return Ok(warning("Import file empty")),
    };
    // A single item.
    if list.iter().any(|(k, v)| super::php::key_eq(k, &key("key")) && !matches!(v, Value::Null)) {
        list = vec![(rustpress_phpser::Key::Int(0), Value::Array(list))];
    }
    let mut ids = Vec::new();
    let mut reregistered = Vec::new();
    for (_, to_import) in list {
        let Value::Array(items) = to_import else { continue };
        let item_key = super::php::strval(items.iter().find(|(k, _)| super::php::key_eq(k, &key("key"))).map(|(_, v)| v));
        let Some(post_type) = key_type(&item_key).filter(|t| matches!(*t, "acf-field-group" | "acf-post-type" | "acf-taxonomy")) else { continue };
        let existing = site.db.acf_post_by_key(post_type, &item_key).await?.map(|p| p.id);
        ids.push(super::save_group::import_item(ctx, post_type, items, existing, &mut reregistered).await?);
    }
    // Each saved type flushed the rewrite rules; the last flush, after all of them, is what stays.
    if let (false, Some(writer)) = (reregistered.is_empty(), ctx.writer) {
        let changes: Vec<rustpress_core::rewrite_gen::Reregistered<'_>> = reregistered.iter().map(|(taxonomy, name, active)| rustpress_core::rewrite_gen::Reregistered { taxonomy: *taxonomy, name, active: *active }).collect();
        rustpress_core::rewrite_gen::flush_rewrite_rules_after(site, writer, ctx.wp_root, &changes).await?;
    }
    let count = ids.len();
    let mut text = rustpress_core::i18n::sprintf(&site.i18n().ngettext("acf", "Imported 1 item", "Imported %s items", count as i64), &[&count.to_string()]);
    let mut links = Vec::new();
    for id in ids {
        let post = site.db.post(id).await?;
        let title = post.as_ref().map(|p| rustpress_blocks::content::the_title_in_admin(site, p)).unwrap_or_default();
        links.push(format!("<a href=\"{}\">{}</a>", esc_url(&site.site_url(&format!("wp-admin/post.php?post={id}&amp;action=edit"))), esc_html(&title)));
    }
    text.push(' ');
    text.push_str(&links.join(", "));
    Ok(admin_notice(&text, "success"))
}

/// A literal argument list (`get_post_type_args()`) as the array it stands for.
fn lit_value(lit: &rustpress_core::php_lit::Lit) -> Value {
    use rustpress_core::php_lit::Lit;
    match lit {
        Lit::Null | Lit::Unknown => Value::Null,
        Lit::Bool(b) => Value::Bool(*b),
        Lit::Int(i) => Value::Int(*i),
        Lit::Float(f) => Value::Float(*f),
        Lit::Str(s) => string(s),
        Lit::Array(items) => {
            let mut next = 0;
            let mut out = Vec::new();
            for (k, v) in items {
                let key = match k {
                    Some(Lit::Int(i)) => {
                        next = next.max(*i + 1);
                        rustpress_phpser::Key::Int(*i)
                    }
                    Some(Lit::Str(s)) => key(s),
                    _ => {
                        next += 1;
                        rustpress_phpser::Key::Int(next - 1)
                    }
                };
                out.push((key, lit_value(v)));
            }
            Value::Array(out)
        }
    }
}

/// A PHP string literal in single quotes (`var_export()`).
fn php_quote(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'").replace('\0', "' . \"\\0\" . '"))
}

/// `var_export( $value, true )` (`php_var_export_ex()` from `level` 1).
fn var_export(value: &Value, level: usize, out: &mut String) {
    match value {
        Value::Null => out.push_str("NULL"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::Float(f) => {
            let text = super::php::float_string(*f);
            out.push_str(&text);
            if f.is_finite() && !text.contains(['.', 'E', 'e']) {
                out.push_str(".0");
            }
        }
        Value::Str(s) => out.push_str(&php_quote(s)),
        Value::Array(items) => {
            if level > 1 {
                out.push('\n');
                out.push_str(&" ".repeat(level - 1));
            }
            out.push_str("array (\n");
            for (k, v) in items {
                out.push_str(&" ".repeat(level + 1));
                match k {
                    rustpress_phpser::Key::Int(i) => out.push_str(&i.to_string()),
                    rustpress_phpser::Key::Str(s) => out.push_str(&php_quote(s)),
                }
                out.push_str(" => ");
                var_export(v, level + 2, out);
                out.push_str(",\n");
            }
            if level > 1 {
                out.push_str(&" ".repeat(level - 1));
            }
            out.push(')');
        }
        _ => out.push_str("NULL"),
    }
}

/// `ACF_Internal_Post_Type::format_code_for_export()`: tabs, `array(` without the space, nested arrays after their
/// key, no numeric keys before arrays.
fn format_code(code: &str) -> String {
    let code = code.replace("  ", "\t").replace("'!!__(!!\\'", "__( '").replace("!!\\', !!\\'", "', '").replace("!!\\')!!'", "' )").replace("array (", "array(");
    // `/([\t\r\n]+?)array/` and `/[0-9]+ => array/`.
    let mut out = String::with_capacity(code.len());
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if matches!(bytes[i], b'\t' | b'\r' | b'\n') {
            let mut j = i;
            while j < bytes.len() && matches!(bytes[j], b'\t' | b'\r' | b'\n') {
                j += 1;
            }
            if code[j..].starts_with("array") {
                i = j;
                continue;
            }
            out.push_str(&code[i..j]);
            i = j;
            continue;
        }
        let ch = code[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    let digits = regex::Regex::new(r"[0-9]+ => array").expect("valid pattern");
    digits.replace_all(&out, "array").into_owned()
}

/// `esc_textarea()`.
fn esc_textarea(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#039;")
}

/// `export_post_as_php()` of a field group, post type or taxonomy (`ACF_Field_Group`, `ACF_Post_Type`, `ACF_Taxonomy`).
fn export_as_php(post_type: &str, item: &Value) -> String {
    let export = |value: &Value| {
        let mut code = String::new();
        var_export(value, 1, &mut code);
        format_code(&code)
    };
    let code = match post_type {
        "acf-field-group" => format!("acf_add_local_field_group( {} );\r\n", export(item)),
        "acf-post-type" => {
            let post = super::internal::validated_value(item, super::internal::post_type_defaults());
            let mut args = lit_value(&super::internal::post_type_args_with(&post, false));
            // The callback as it was saved.
            if let Value::Array(items) = &mut args
                && super::php::truthy(items.iter().find(|(k, _)| super::php::key_eq(k, &key("register_meta_box_cb"))).map(|(_, v)| v))
                && super::php::truthy(post.get("register_meta_box_cb"))
            {
                super::php::set(items, "register_meta_box_cb", string(&super::php::strval(post.get("register_meta_box_cb"))));
            }
            format!("register_post_type( '{}', {} );\r\n", super::php::strval(post.get("post_type")), export(&args))
        }
        "acf-taxonomy" => {
            let post = super::internal::validated_value(item, super::internal::taxonomy_defaults());
            let objects = match post.get("object_type") {
                Some(Value::Array(items)) => Value::Array(items.clone()),
                None | Some(Value::Null) => Value::Array(Vec::new()),
                Some(other) => Value::Array(vec![(rustpress_phpser::Key::Int(0), other.clone())]),
            };
            let mut args = lit_value(&super::internal::taxonomy_args_with(&post, false));
            if let Value::Array(items) = &mut args
                && super::php::truthy(items.iter().find(|(k, _)| super::php::key_eq(k, &key("meta_box_cb"))).map(|(_, v)| v))
                && super::php::truthy(post.get("meta_box_cb"))
            {
                super::php::set(items, "meta_box_cb", post.get("meta_box_cb").cloned().unwrap_or(Value::Null));
            }
            format!("register_taxonomy( '{}', {}, {} );\r\n", super::php::strval(post.get("taxonomy")), export(&objects), export(&args))
        }
        _ => String::new(),
    };
    esc_textarea(&code)
}

/// `acf_export_enter_title_here()`.
fn export_enter_title_here(items: &[Value]) -> String {
    let titles: Vec<(String, String)> = items.iter().filter(|i| super::php::truthy(i.get("enter_title_here"))).map(|i| (super::php::strval(i.get("post_type")), super::php::strval(i.get("enter_title_here")))).collect();
    if titles.is_empty() {
        return String::new();
    }
    let mut out = String::from("\r\nadd_filter( 'enter_title_here', function( $default, $post ) {\r\n\tswitch ( $post->post_type ) {\r\n");
    for (post_type, title) in titles {
        out.push_str(&format!("\t\tcase '{post_type}':\r\n\t\t\treturn '{title}';\r\n"));
    }
    out.push_str("\t}\r\n\r\n\treturn $default;\r\n}, 10, 2 );\r\n\r\n");
    esc_textarea(&out)
}

/// `ACF_Admin_Tool_Export::html_generate()`: the PHP of the selection, the copy button and its script.
async fn generate_html(ctx: &AdminContext<'_>) -> Result<String> {
    let site = ctx.site;
    let items = selected_items(ctx).await?;
    // Grouped by internal post type, in the order they come first.
    let mut groups: Vec<(&'static str, Vec<&Value>)> = Vec::new();
    for item in &items {
        let Some(post_type) = key_type(&super::php::strval(item.get("key"))) else { continue };
        match groups.iter_mut().find(|(t, _)| *t == post_type) {
            Some((_, list)) => list.push(item),
            None => groups.push((post_type, vec![item])),
        }
    }
    let mut out = String::from("<textarea id=\"acf-export-textarea\" readonly=\"readonly\">");
    for (post_type, posts) in &groups {
        match *post_type {
            "acf-field-group" => out.push_str("add_action( 'acf/include_fields', function() {\r\n\tif ( ! function_exists( 'acf_add_local_field_group' ) ) {\r\n\t\treturn;\r\n\t}\r\n\r\n"),
            "acf-post-type" | "acf-taxonomy" => out.push_str("add_action( 'init', function() {\r\n"),
            "acf-ui-options-page" => out.push_str("add_action( 'acf/init', function() {\r\n"),
            _ => {}
        }
        for (i, post) in posts.iter().enumerate() {
            if i != 0 {
                out.push_str("\r\n");
            }
            out.push('\t');
            out.push_str(&export_as_php(post_type, post));
        }
        out.push_str("} );\r\n\r\n");
        if *post_type == "acf-post-type" {
            let owned: Vec<Value> = posts.iter().map(|p| (*p).clone()).collect();
            out.push_str(&export_enter_title_here(&owned));
        }
    }
    out.push_str("</textarea>");
    out.push_str(&format!(
        "\t\t\t<p class=\"acf-submit\">\n\t\t\t\t<a class=\"button\" id=\"acf-export-copy\">{}</a>\n\t\t\t</p>\n\t\t\t<script type=\"text/javascript\">\n\t\t\t(function($){{\n\t\t\t\tconst $a = $('#acf-export-copy');\n\t\t\t\tconst $textarea = $('#acf-export-textarea');\n\n\t\t\t\t// Remove $a if 'copy' is not supported.\n\t\t\t\tif( !document.queryCommandSupported('copy') ) {{\n\t\t\t\t\treturn $a.remove();\n\t\t\t\t}}\n\n\t\t\t\t$a.on('click', function( e ){{\n\t\t\t\t\te.preventDefault();\n\n\t\t\t\t\t$textarea.get(0).select();\n\n\t\t\t\t\ttry {{\n\t\t\t\t\t\tvar copy = document.execCommand('copy');\n\t\t\t\t\t\tif ( ! copy ) {{\n\t\t\t\t\t\t\treturn;\n\t\t\t\t\t\t}}\n\n\t\t\t\t\t\tacf.newTooltip({{\n\t\t\t\t\t\t\ttext: \t\t\"{}\",\n\t\t\t\t\t\t\ttimeout:\t250,\n\t\t\t\t\t\t\ttarget: \t$(this),\n\t\t\t\t\t\t}});\n\t\t\t\t\t}} catch (err) {{\n\t\t\t\t\t\t// Do nothing.\n\t\t\t\t\t}}\n\t\t\t\t}});\n\t\t\t}})(jQuery);\n\t\t\t</script>\n\t\t\t",
        esc_html(&tr(site, "Copy to clipboard")),
        esc_html(&tr(site, "Copied")),
    ));
    Ok(out)
}

/// `ACF_Admin_Tool_Export::html_single()`: the PHP and, beside it, the selection.
async fn export_single(ctx: &AdminContext<'_>, render: &super::render::Ctx<'_>) -> Result<String> {
    let site = ctx.site;
    Ok(format!(
        "\t\t\t<div class=\"acf-postbox-header\">\n\t\t\t\t<h2 class=\"acf-postbox-title\">{}</h2>\n\t\t\t\t<i tabindex=\"0\" class=\"acf-icon acf-icon-help acf-js-tooltip\" title=\"{}\">?</i>\n\t\t\t</div>\n\t\t\t<div class=\"acf-postbox-columns\">\n\t\t\t\t<div class=\"acf-postbox-main\">\n\t\t\t\t\t{}\t\t\t\t</div>\n\t\t\t\t<div class=\"acf-postbox-side\">\n\t\t\t\t\t\t\t\t<div class=\"acf-panel acf-panel-selection\">\n\t\t\t\t{}\t\t\t</div>\n\t\t\t\t\t\t\t\t<p class=\"acf-submit\">\n\t\t\t\t\t\t<button type=\"submit\" name=\"action\" class=\"acf-btn\" value=\"generate\">{}</button>\n\t\t\t\t\t</p>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t",
        esc_html(&tr(site, "Export - Generate PHP")),
        esc_attr(&tr(site, "The following code can be used to register a local version of the selected items. Storing field groups, post types, or taxonomies locally can provide many benefits such as faster load times, version control & dynamic fields/settings. Simply copy and paste the following code to your theme's functions.php file or include it within an external file, then deactivate or delete the items from the ACF admin.")),
        generate_html(ctx).await?,
        field_selection(ctx, render).await?,
        esc_html(&tr(site, "Generate PHP")),
    ))
}

/// The callback of a page.
pub async fn admin_page(slug: &str, ctx: &AdminContext<'_>) -> Result<String> {
    Ok(match slug {
        OPTIONS_PREVIEW => options_preview(ctx.site),
        _ => String::new(),
    })
}

/// `views/options-page-preview.php`.
fn options_preview(site: &Site) -> String {
    let utm = super::admin_lists::utm;
    let learn_more = utm("https://www.advancedcustomfields.com/pro/", "ACF upgrade", "no-options-pages", "", "", "");
    let upgrade = utm("https://www.advancedcustomfields.com/pro/", "ACF upgrade", "no-options-pages-pricing", "pricing-table", "", "");
    let description = rustpress_core::i18n::sprintf(&tr(site, "ACF <a href=\"%s\" target=\"_blank\">options pages</a> are custom admin pages for managing global settings via fields. You can create multiple pages and sub-pages."), &[&utm("https://www.advancedcustomfields.com/resources/options-page/", "docs", "no-options-pages", "", "", "")]);
    let getting_started = rustpress_core::i18n::sprintf(&tr(site, "New to ACF? Take a look at our <a href=\"%s\" target=\"_blank\">getting started guide</a>."), &[&utm("https://www.advancedcustomfields.com/resources/getting-started-with-acf/", "docs", "no-options-pages", "", "", "")]);
    let acf_esc = |text: &str| super::render::esc_html_acf(&string(text));
    format!(
        "<div class=\"wrap acf_options_preview_wrap\">\n\t<table class=\"wp-list-table widefat fixed striped\">\n\t\t<tbody id=\"the-list\">\n\t\t\t<tr class=\"no-items\">\n\t\t\t\t<td class=\"colspanchange\" colspan=\"6\">\n\t\t\t\t\t<div class=\"acf-no-field-groups-wrapper\">\n\t\t\t\t\t\t<div class=\"acf-no-field-groups-inner acf-field-group-pro-features-content\">\n\t\t\t\t\t\t\t<img src=\"{}\" />\n\t\t\t\t\t\t\t\t<h2>{}</h2>\n\t\t\t\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t\t\t\t<div class=\"acf-ui-options-page-pro-features-actions\">\n\t\t\t\t\t\t\t\t\t<a target=\"_blank\" href=\"{learn_more}\" class=\"acf-btn acf-btn-muted\">{} <i class=\"acf-icon acf-icon-arrow-up-right\"></i></a>\t\t\t\t\t\t\t\t\t<a target=\"_blank\" href=\"{upgrade}\" class=\"acf-btn acf-options-pages-preview-upgrade-button\"> {} <i class=\"acf-icon acf-icon-arrow-up-right\"></i></a>\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t<p class=\"acf-small\">{}</p>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t</div>\n\t\t\t\t</td>\n\t\t\t</tr>\n\t\t</tbody>\n\t</table>\n</div>\n",
        esc_url(&site.site_url("wp-content/plugins/advanced-custom-fields/assets/images/empty-post-types.svg")),
        acf_esc(&tr(site, "Upgrade to ACF PRO to create options pages in just a few clicks")),
        acf_esc(&description),
        esc_html(&tr(site, "Learn More")),
        esc_html(&tr(site, "Upgrade to ACF PRO")),
        acf_esc(&getting_started),
    )
}
