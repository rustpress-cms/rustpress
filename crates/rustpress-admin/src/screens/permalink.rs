//! `wp-admin/options-permalink.php`: the permalink structure and the category and tag bases.
//!
//! Saving writes the three options and regenerates the `rewrite_rules` option with
//! [`rustpress_core::rewrite_gen`]. There is no `.htaccess` or `web.config` here: RustPress answers the
//! requests itself.

use rustpress_core::OrLog;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_phpser::{Key, Value};
use rustpress_core::rewrite_gen::{RewriteInput, rewrite_rules, serialize_rules, taxonomy_base};

use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Screen};

/// `sanitize_option( 'permalink_structure' )` and the bases: `esc_url_raw()`, no `http://`.
fn sanitize(value: &str) -> String {
    rustpress_core::formatting::esc_url(value).replace("&#038;", "&").replace("&#039;", "'").replace("http://", "")
}

/// `preg_replace( '#/+#', '/', '/' . str_replace( '#', '', $value ) )`.
fn rooted(value: &str) -> String {
    let mut out = String::from("/");
    out.push_str(&value.replace('#', ""));
    let mut collapsed = String::with_capacity(out.len());
    for c in out.chars() {
        if c == '/' && collapsed.ends_with('/') {
            continue;
        }
        collapsed.push(c);
    }
    collapsed
}

struct Structure {
    id: &'static str,
    label: String,
    value: String,
    example: String,
}

fn checked(on: bool) -> &'static str {
    if on { " checked='checked'" } else { "" }
}

/// Saves the form; `None` when the request has nothing to save.
async fn save(ctx: &Ctx<'_>) -> Option<AdminResponse> {
    let request = ctx.request;
    if request.post_field("permalink_structure").is_none() && request.post_field("category_base").is_none() {
        return None;
    }
    if !ctx.verify_nonce("update-permalink") {
        return Some(die(ctx.env, 403, &ctx.tr("An error occurred."), &ctx.tr("The link you followed has expired.")));
    }
    let Some(writer) = ctx.env.writer else { return Some(super::common::read_only(ctx)) };
    let site = ctx.site();
    let old_structure = site.options.string("permalink_structure", "");
    let old_front = old_structure.find('%').map_or("", |at| &old_structure[..at]).to_owned();
    let mut structure = old_structure.clone();
    let old_category_base = taxonomy_base(&site.options.string("category_base", ""));
    let old_tag_base = taxonomy_base(&site.options.string("tag_base", ""));
    let mut category_base = site.options.string("category_base", "");
    let mut tag_base = site.options.string("tag_base", "");
    let mut errors: Vec<(&str, String)> = Vec::new();

    if let Some(custom) = request.post_field("permalink_structure") {
        let chosen = match request.post_field("selection") {
            Some(selection) if selection != "custom" => selection,
            _ => custom,
        };
        let chosen = if chosen.is_empty() { String::new() } else { rooted(chosen) };
        let chosen = if chosen.is_empty() { chosen } else { rooted(&sanitize(&chosen)) };
        // `sanitize_option()`: custom permalinks need a structure tag, otherwise the old value stays.
        if !chosen.is_empty() && !has_structure_tag(&chosen) {
            errors.push(("invalid_permalink_structure", sprintf(&ctx.tr("A structure tag is required when using custom permalinks. <a href=\"%s\">Learn more</a>"), &[&ctx.tr("https://wordpress.org/documentation/article/customize-permalinks/#choosing-your-permalink-structure")])));
        } else {
            structure = chosen;
        }
    }
    if let Some(base) = request.post_field("category_base") {
        category_base = if base.is_empty() { String::new() } else { sanitize(&rooted(base)) };
    }
    if let Some(base) = request.post_field("tag_base") {
        tag_base = if base.is_empty() { String::new() } else { sanitize(&rooted(base)) };
    }

    let home = site.settings.home.trim_end_matches('/');
    let input = RewriteInput {
        permalink_structure: &structure,
        old_front: &old_front,
        category_base: &category_base,
        tag_base: &tag_base,
        page_on_front: site.options.u64("page_on_front", 0),
        home_at_root: home.split_once("://").is_none_or(|(_, rest)| !rest.contains('/')),
        blog_public: site.options.string("blog_public", "1") != "0",
        types: &site.types,
    };
    let rules = rewrite_rules(&input);
    let serialized = if rules.is_empty() { String::new() } else { serialize_rules(&rules) };
    if structure != old_structure {
        writer.update_option("permalink_structure", &structure).await.or_log();
    }
    if old_category_base != category_base {
        writer.update_option("category_base", &category_base).await.or_log();
    }
    if old_tag_base != tag_base {
        writer.update_option("tag_base", &tag_base).await.or_log();
    }
    writer.update_option("rewrite_rules", &serialized).await.or_log();

    // `add_settings_error()` and `set_transient( 'settings_errors', ..., 30 )`.
    let message = if errors.is_empty() { vec![("settings_updated", ctx.tr("Permalink structure updated."), "success", "general")] } else { errors.iter().map(|(code, text)| (*code, text.clone(), "error", "permalink_structure")).collect() };
    let entries = message
        .into_iter()
        .enumerate()
        .map(|(i, (code, text, kind, setting))| {
            let field = |k: &str, v: &str| (Key::Str(k.as_bytes().to_vec()), Value::Str(v.as_bytes().to_vec()));
            (Key::Int(i as i64), Value::Array(vec![field("setting", setting), field("code", code), field("message", &text), field("type", kind)]))
        })
        .collect();
    let value = String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(entries))).into_owned();
    writer.update_option_with_autoload("_transient_settings_errors", &value, "off").await.or_log();
    writer.update_option_with_autoload("_transient_timeout_settings_errors", &(ctx.env.now + 30).to_string(), "off").await.or_log();
    Some(redirect(&ctx.admin_url("options-permalink.php?settings-updated=true")))
}

/// `preg_match( '/%[^\/%]+%/', $value )`.
fn has_structure_tag(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'/' && bytes[j] != b'%' {
                j += 1;
            }
            if j > i + 1 && j < bytes.len() && bytes[j] == b'%' {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// `settings_errors()`: the notices the last save left in the transient.
async fn settings_errors(ctx: &Ctx<'_>) -> String {
    if ctx.request.get("settings-updated").is_none() {
        return String::new();
    }
    let Ok(Some(raw)) = ctx.env.db.option("_transient_settings_errors").await else { return String::new() };
    let Ok(value) = rustpress_phpser::unserialize(raw.as_bytes()) else { return String::new() };
    let Some(items) = value.as_array() else { return String::new() };
    // `delete_transient( 'settings_errors' )`: shown once.
    if let Some(writer) = ctx.env.writer {
        writer.delete_option("_transient_settings_errors").await.or_log();
        writer.delete_option("_transient_timeout_settings_errors").await.or_log();
    }
    let mut out = String::new();
    for (_, entry) in items {
        let field = |n: &str| entry.get(n).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
        let kind = match field("type").as_str() {
            "updated" => "success".to_owned(),
            other => other.to_owned(),
        };
        out.push_str(&format!("<div id='setting-error-{}' class='notice notice-{kind} settings-error is-dismissible'> \n<p><strong>{}</strong></p></div> \n", field("code"), field("message")));
    }
    out
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("manage_options") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to manage options for this site.")));
    }
    if ctx.request.post {
        if let Some(response) = save(ctx).await {
            return response;
        }
    }
    let site = ctx.site();
    let permalink_structure = site.options.string("permalink_structure", "");
    let category_base = taxonomy_base(&site.options.string("category_base", ""));
    let tag_base = taxonomy_base(&site.options.string("tag_base", ""));
    let home = site.settings.home.trim_end_matches('/').to_owned();
    let url_base = home.clone();
    let today = rustpress_core::datetime::SiteTimezone::utc();
    let sample = site.trx("sample-post", "sample permalink structure");
    let archives = site.trx("archives", "sample permalink base");
    let structures = [
        Structure { id: "plain", label: tr("Plain"), value: String::new(), example: format!("{home}/?p=123") },
        Structure { id: "day-name", label: tr("Day and name"), value: "/%year%/%monthnum%/%day%/%postname%/".into(), example: format!("{url_base}/{}/{sample}/", today.format("Y/m/d", ctx.env.now)) },
        Structure { id: "month-name", label: tr("Month and name"), value: "/%year%/%monthnum%/%postname%/".into(), example: format!("{url_base}/{}/{sample}/", today.format("Y/m", ctx.env.now)) },
        Structure { id: "numeric", label: tr("Numeric"), value: format!("/{archives}/%post_id%"), example: format!("{url_base}/{archives}/123") },
        Structure { id: "post-name", label: tr("Post name"), value: "/%postname%/".into(), example: format!("{url_base}/{sample}/") },
    ];
    let is_default = structures.iter().any(|s| s.value == permalink_structure);

    let tags: [(&str, &str); 10] = [
        ("year", "%s (The year of the post, four digits, for example 2004.)"),
        ("monthnum", "%s (Month of the year, for example 05.)"),
        ("day", "%s (Day of the month, for example 28.)"),
        ("hour", "%s (Hour of the day, for example 15.)"),
        ("minute", "%s (Minute of the hour, for example 43.)"),
        ("second", "%s (Second of the minute, for example 33.)"),
        ("post_id", "%s (The unique ID of the post, for example 423.)"),
        ("postname", "%s (The sanitized post title (slug).)"),
        ("category", "%s (Category slug. Nested sub-categories appear as nested directories in the URL.)"),
        ("author", "%s (A sanitized version of the author name.)"),
    ];
    let tag_added = tr("%s added to permalink structure");
    let tag_removed = tr("%s removed from permalink structure");
    let tag_used = tr("%s (already used in permalink structure)");

    let mut h = String::new();
    h.push_str(&settings_errors(ctx).await);
    h.push_str(&format!("<div class=\"wrap\">\n<h1>{}</h1>\n\n<form name=\"form\" action=\"options-permalink.php\" method=\"post\">\n{}\n<p>\n", esc_html(&tr("Permalink Settings")), ctx.nonce_field("update-permalink")));
    h.push_str(&sprintf(&tr("WordPress offers you the ability to create a custom URL structure for your permalinks and archives. Custom URL structures can improve the aesthetics, usability, and forward-compatibility of your links. A <a href=\"%s\">number of tags are available</a>, and here are some examples to get you started."), &[&tr("https://wordpress.org/documentation/article/customize-permalinks/")]));
    h.push_str("</p>\n\n");
    h.push_str(&format!("<h2 id=\"wp-settings-section-common-settings\" class=\"title\">{}</h2>\n<p>\n", tr("Common Settings")));
    h.push_str(&sprintf(&tr("Select the permalink structure for your website. Including the %s tag makes links easy to understand, and can help your posts rank higher in search engines."), &["<code>%postname%</code>"]));
    h.push_str("</p>\n<table class=\"form-table permalink-structure\" role=\"presentation\">\n<tbody>\n");
    let title = tr("Permalink structure");
    h.push_str(&format!("<tr>\n\t<th scope=\"row\">{title}</th>\n\t<td>\n\t\t<fieldset class=\"structure-selection\">\n\t\t\t<legend class=\"screen-reader-text\">{title}</legend>\n\t\t\t"));
    for s in &structures {
        h.push_str(&format!(
            "\t\t\t<div class=\"row\">\n\t\t\t\t<input id=\"permalink-input-{id}\"\n\t\t\t\t\tname=\"selection\" aria-describedby=\"permalink-{id}\"\n\t\t\t\t\ttype=\"radio\" value=\"{value}\"\n\t\t\t\t\t{checked}\t\t\t\t/>\n\t\t\t\t<div>\n\t\t\t\t\t<label for=\"permalink-input-{id}\">\n\t\t\t\t\t\t{label}\t\t\t\t\t</label>\n\t\t\t\t\t<p>\n\t\t\t\t\t\t<code id=\"permalink-{id}\">\n\t\t\t\t\t\t\t{example}\t\t\t\t\t\t</code>\n\t\t\t\t\t</p>\n\t\t\t\t</div>\n\t\t\t</div><!-- .row -->\n\t\t\t",
            id = esc_attr(s.id),
            value = esc_attr(&s.value),
            checked = checked(s.value == permalink_structure),
            label = esc_html(&s.label),
            example = esc_html(&s.example)
        ));
    }
    h.push_str(&format!(
        "\n\t\t\t<div class=\"row\">\n\t\t\t\t<input id=\"custom_selection\"\n\t\t\t\t\tname=\"selection\" type=\"radio\" value=\"custom\"\n\t\t\t\t\t{checked}\t\t\t\t/>\n\t\t\t\t<div>\n\t\t\t\t\t<label for=\"custom_selection\">{custom}</label>\n\t\t\t\t\t<p>\n\t\t\t\t\t\t<label for=\"permalink_structure\" class=\"screen-reader-text\">\n\t\t\t\t\t\t\t{hidden}\t\t\t\t\t\t</label>\n\t\t\t\t\t\t<span class=\"code\">\n\t\t\t\t\t\t\t<code id=\"permalink-custom\">{base}</code>\n\t\t\t\t\t\t\t<input name=\"permalink_structure\" id=\"permalink_structure\"\n\t\t\t\t\t\t\t\ttype=\"text\" value=\"{value}\"\n\t\t\t\t\t\t\t\taria-describedby=\"permalink-custom\" class=\"regular-text code\"\n\t\t\t\t\t\t\t/>\n\t\t\t\t\t\t</span>\n\t\t\t\t\t</p>\n\n\t\t\t\t\t<div class=\"available-structure-tags hide-if-no-js\">\n\t\t\t\t\t\t<div id=\"custom_selection_updated\" aria-live=\"assertive\" class=\"screen-reader-text\"></div>\n\t\t\t\t\t\t\t\t\t\t\t\t<fieldset>\n\t\t\t\t\t\t\t<legend>{available}</legend>\n\t\t\t\t\t\t\t<ul role=\"list\">\n\t\t\t\t\t\t\t",
        checked = checked(!is_default),
        custom = tr("Custom Structure"),
        hidden = tr("Customize permalink structure by selecting available tags"),
        base = esc_url(&url_base),
        value = esc_attr(&permalink_structure),
        available = tr("Available tags:")
    ));
    for (tag, explanation) in tags {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t<li>\n\t\t\t\t\t\t\t\t\t<button type=\"button\"\n\t\t\t\t\t\t\t\t\t\tclass=\"button button-secondary\"\n\t\t\t\t\t\t\t\t\t\taria-label=\"{label}\"\n\t\t\t\t\t\t\t\t\t\tdata-added=\"{added}\"\n\t\t\t\t\t\t\t\t\t\tdata-removed=\"{removed}\"\n\t\t\t\t\t\t\t\t\t\tdata-used=\"{used}\">\n\t\t\t\t\t\t\t\t\t\t%{tag}%\t\t\t\t\t\t\t\t\t</button>\n\t\t\t\t\t\t\t\t</li>\n\t\t\t\t\t\t\t",
            label = esc_attr(&sprintf(&tr(explanation), &[tag])),
            added = esc_attr(&sprintf(&tag_added, &[tag])),
            removed = esc_attr(&sprintf(&tag_removed, &[tag])),
            used = esc_attr(&sprintf(&tag_used, &[tag])),
        ));
    }
    h.push_str("\t\t\t\t\t\t\t</ul>\n\t\t\t\t\t\t</fieldset>\n\t\t\t\t\t\t\t\t\t\t\t</div><!-- .available-structure-tags -->\n\t\t\t\t</div>\n\t\t\t</div><!-- .row -->\n\t\t</fieldset><!-- .structure-selection -->\n\t</td>\n</tr>\n</tbody>\n</table>\n\n");
    h.push_str(&format!("<h2 id=\"wp-settings-section-optional\" class=\"title\">{}</h2>\n<p class=\"permalink-structure-optional-description\">\n", tr("Optional")));
    h.push_str(&sprintf(&tr("If you like, you may enter custom structures for your category and tag URLs here. For example, using <code>topics</code> as your category base would make your category links like <code>%s/topics/uncategorized/</code>. If you leave these blank the defaults will be used."), &[&url_base]));
    h.push_str("</p>\n\n<table class=\"form-table\" role=\"presentation\">\n");
    h.push_str(&format!(
        "\t<tr>\n\t\t<th>\n\t\t\t<label for=\"category_base\">\n\t\t\t\t{category}\t\t\t</label>\n\t\t</th>\n\t\t<td>\n\t\t\t\t\t<input name=\"category_base\" id=\"category_base\" type=\"text\"\n\t\t\t\tvalue=\"{category_base}\" class=\"regular-text code\"\n\t\t\t/>\n\t\t\t\t</td>\n\t</tr>\n\t<tr>\n\t\t<th>\n\t\t\t<label for=\"tag_base\">{tag}</label>\n\t\t</th>\n\t\t<td>\n\t\t\t\t\t<input name=\"tag_base\" id=\"tag_base\" type=\"text\"\n\t\t\t\tvalue=\"{tag_base}\" class=\"regular-text code\"\n\t\t\t/>\n\t\t\t\t</td>\n\t</tr>\n\t</table>\n\n\n{submit}</form>\n\n\t\t\t\t\n</div><!-- .wrap -->\n\n",
        category = tr("Category base"),
        category_base = esc_attr(&category_base),
        tag = tr("Tag base"),
        tag_base = esc_attr(&tag_base),
        submit = super::common::submit(ctx)
    ));

    let baked = super::pages::load(ctx, "options-permalink");
    let screen = Screen { file: "options-permalink.php".into(), parent_file: "options-general.php".into(), submenu_file: Some("options-permalink.php".into()), title: tr("Permalink Settings"), help: Some(baked.help), ..Default::default() };
    render(ctx, &screen, &h, &Assets::default()).await
}
