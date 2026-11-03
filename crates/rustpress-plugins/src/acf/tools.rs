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
    for (name, title) in [("export", tr(site, "Export Field Groups")), ("import", tr(site, "Import Field Groups"))] {
        if active.is_some_and(|a| a != name) {
            continue;
        }
        let html = match name {
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
