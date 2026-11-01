//! The administration of Contact Form 7 (`admin/admin.php`, `admin/contact-forms.php`, `admin/includes/list-table.php`,
//! `admin/includes/help-tabs.php`): the menu "Contact" and the list of the contact forms.

use std::sync::OnceLock;

use rustpress_admin::list_table::{Column, Table};
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::kses::Allowed;
use rustpress_core::native::{AdminContext, AdminHelp, AdminPage, HelpTab, PageLoad};
use rustpress_core::{Result, Site};
use rustpress_db::{Order, OrderBy, PostQuery};

use super::DOMAIN;
use super::form::ContactForm;
use super::formatter::{Att, Atts, HtmlFormatter, format_atts};

/// `WPCF7_ADMIN_READ_CAPABILITY` and `WPCF7_ADMIN_READ_WRITE_CAPABILITY`, as `wpcf7_map_meta_cap()` maps the plugin's
/// capabilities.
pub const READ: &str = "edit_posts";
pub const READ_WRITE: &str = "publish_pages";

fn tr(site: &Site, text: &str) -> String {
    site.trd(DOMAIN, text)
}

/// `wpcf7_admin_menu()`.
pub fn pages(site: &Site) -> Vec<AdminPage> {
    let dashboard = tr(site, "Contact Form 7 Dashboard");
    vec![
        AdminPage { parent: "", slug: "wpcf7-dashboard", page_title: dashboard.clone(), menu_title: tr(site, "Contact"), capability: READ, icon: "dashicons-email", position: Some(30) },
        AdminPage { parent: "wpcf7-dashboard", slug: "wpcf7-dashboard", page_title: dashboard, menu_title: tr(site, "Dashboard"), capability: READ, ..AdminPage::default() },
        AdminPage { parent: "wpcf7-dashboard", slug: "wpcf7", page_title: tr(site, "Contact Forms"), menu_title: tr(site, "Contact Forms"), capability: READ, ..AdminPage::default() },
        AdminPage { parent: "wpcf7-dashboard", slug: "wpcf7-new", page_title: tr(site, "Add Contact Form"), menu_title: tr(site, "Add Contact Form"), capability: READ_WRITE, ..AdminPage::default() },
        // The services (reCAPTCHA, Turnstile, Stripe, ...) are always registered, so the page is always there.
        AdminPage {
            parent: "wpcf7-dashboard",
            slug: "wpcf7-integration",
            page_title: tr(site, "Integration with External API"),
            menu_title: tr(site, "Integration"),
            capability: "manage_options",
            ..AdminPage::default()
        },
    ]
}

/// `wpcf7_kses_allowed_html( 'form' )`, with `form` limited to `method` as `contact-forms.php` has it: what the
/// screens of the plugin print goes through `wp_kses()` with it.
fn allowed_html() -> &'static Allowed {
    static ALLOWED: OnceLock<Allowed> = OnceLock::new();
    ALLOWED.get_or_init(|| {
        Allowed::post()
            .merged(&[
                ("datalist", &[]),
                ("fieldset", &["disabled", "name"]),
                ("input", &["accept", "alt", "autocomplete", "capture", "checked", "disabled", "list", "max", "maxlength", "min", "minlength", "multiple", "name", "pattern", "placeholder", "readonly", "required", "size", "step", "type", "value"]),
                ("optgroup", &["disabled", "label"]),
                ("option", &["disabled", "label", "selected", "value"]),
                ("output", &["for", "name"]),
                ("select", &["autocomplete", "disabled", "multiple", "name", "required", "size"]),
                ("textarea", &["autocomplete", "cols", "disabled", "maxlength", "minlength", "name", "placeholder", "readonly", "required", "rows", "wrap"]),
            ])
            .with_global(&[
                "aria-atomic", "aria-checked", "aria-controls", "aria-current", "aria-describedby", "aria-details", "aria-disabled", "aria-expanded", "aria-hidden", "aria-invalid", "aria-label", "aria-labelledby", "aria-live", "aria-relevant", "aria-required",
                "aria-selected", "class", "data-*", "dir", "hidden", "id", "inputmode", "lang", "popover", "role", "spellcheck", "style", "tabindex", "title", "xml:lang",
            ])
            .merged(&[("form", &["method"])])
    })
}

/// `wpcf7_link( $url, $anchor_text, $atts )`.
fn link(url: &str, text: &str, class: Option<&str>) -> String {
    let mut atts = Atts::default();
    atts.set("id", Att::opt(None));
    atts.set("class", Att::opt(class.map(str::to_owned)));
    atts.set("href", Att::str(esc_url(url)));
    format!("<a {}>{}</a>", format_atts(&atts), esc_html(text))
}

/// The contact forms as they are now in the database (the snapshot may be older than a change just made).
async fn current_forms(site: &Site) -> Result<Vec<(ContactForm, rustpress_db::Post)>> {
    let query = PostQuery {
        post_types: vec![super::form::POST_TYPE.into()],
        statuses: ["publish", "future", "draft", "pending", "private"].map(String::from).to_vec(),
        order_by: OrderBy::Id,
        order: Order::Asc,
        limit: None,
        ..PostQuery::default()
    };
    let mut out = Vec::new();
    for post in site.db.posts(&query).await? {
        let meta: Vec<(String, String)> = site.db.post_meta(post.id).await?.into_iter().filter_map(|m| Some((m.meta_key?, m.meta_value.unwrap_or_default()))).collect();
        let mut form = ContactForm::from_post(&post, &meta);
        form.config_errors = meta.iter().find(|(k, _)| k == "_config_validation").map(|(_, v)| count_config_errors(v)).unwrap_or(0);
        out.push((form, post));
    }
    Ok(out)
}

/// `WPCF7_ConfigValidator::restore()` and `count_errors()`: the errors stored by the last validation.
fn count_config_errors(stored: &str) -> usize {
    let Ok(rustpress_phpser::Value::Array(sections)) = rustpress_phpser::unserialize(stored.as_bytes()) else { return 0 };
    sections
        .iter()
        .filter_map(|(_, errors)| errors.as_array())
        .map(|errors| errors.iter().filter(|(_, e)| e.get("code").is_some_and(super::mail::php_truthy)).count())
        .sum()
}

/// `WP_Query` with the search of `s`: every word in the title, the excerpt or the content.
fn matches_search(post: &rustpress_db::Post, search: &str) -> bool {
    search.split_whitespace().all(|word| {
        let word = word.to_lowercase();
        [&post.post_title, &post.post_excerpt, &post.post_content].iter().any(|field| field.to_lowercase().contains(&word))
    })
}

/// The screen `admin.php?page=wpcf7`: the list of the contact forms (`WPCF7_List_Table`).
pub async fn list_page(ctx: &AdminContext<'_>) -> Result<String> {
    let site = ctx.site;
    let t = |text: &str| tr(site, text);
    let get = |name: &str| ctx.get(name).unwrap_or_default().to_owned();
    // `prepare_items()`: without `orderby` the title column sorts, ascending.
    let (mut orderby, mut order) = (get("orderby"), get("order"));
    if orderby.is_empty() {
        orderby = "title".into();
        order = "asc".into();
    }
    let per_page = site.db.user_meta(ctx.user, "wpcf7_contact_forms_per_page").await?.and_then(|v| v.trim().parse::<u64>().ok()).filter(|n| *n > 0).unwrap_or(20);
    let paged = get("paged").trim().parse::<u64>().unwrap_or(1).max(1);
    let search = get("s");
    let mut forms: Vec<(ContactForm, rustpress_db::Post)> = current_forms(site).await?.into_iter().filter(|(_, post)| search.is_empty() || matches_search(post, &search)).collect();
    let descending = if order.is_empty() { true } else { order.eq_ignore_ascii_case("desc") };
    match orderby.as_str() {
        "title" => forms.sort_by(|a, b| a.1.post_title.cmp(&b.1.post_title)),
        "author" => forms.sort_by_key(|f| f.1.post_author),
        _ => forms.sort_by(|a, b| a.1.post_date.cmp(&b.1.post_date)),
    }
    if descending {
        forms.reverse();
    }
    let total = forms.len() as u64;
    let shown: Vec<&(ContactForm, rustpress_db::Post)> = forms.iter().skip(((paged - 1) * per_page) as usize).take(per_page as usize).collect();

    let columns = vec![
        Column::new("cb", "<input type=\"checkbox\" />"),
        Column::new("title", &t("Title")).sortable("title", false, Some(&t("Title")), Some("asc")),
        Column::new("author", &t("Author")).sortable("author", false, Some(&t("Author")), None),
        Column::new("date", &t("Date")).sortable("date", true, Some(&t("Date")), None),
    ];
    let page_url = site.site_url("wp-admin/admin.php?page=wpcf7");
    let table = Table {
        site,
        plural: "posts".into(),
        singular: "post".into(),
        columns,
        primary: "title".into(),
        current_url: format!("{}{}", site.site_url("").split('/').take(3).collect::<Vec<_>>().join("/"), ctx.request_uri),
        orderby: get("orderby"),
        order: get("order").to_ascii_lowercase(),
        total_items: total,
        per_page,
        current_page: paged,
        bulk_actions: if (ctx.can)(READ_WRITE) { vec![("delete".into(), t("Delete"))] } else { Vec::new() },
        has_items: !shown.is_empty(),
        view_mode: "list".into(),
        // `ceil( $total_items / $per_page )`.
        float_pages: true,
    };
    let mut rows = String::new();
    for (form, post) in &shown {
        rows.push_str(&row(ctx, form, post, &page_url).await?);
    }
    if shown.is_empty() {
        rows.push_str(&format!("<tr class=\"no-items\"><td class=\"colspanchange\" colspan=\"4\">{}</td></tr>", site.tr("No items found.")));
    }
    // `print_table_description()`: the order of the table for screen readers.
    let caption = match orderby.as_str() {
        "title" => Some(t("Table ordered by Title.")),
        "author" => Some(t("Table ordered by Author.")),
        "date" => Some(t("Table ordered by Date.")),
        _ => None,
    }
    .map(|text| format!("{text} {}", if descending { site.tr("Descending.") } else { site.tr("Ascending.") }))
    .unwrap_or_default();
    let nonce_field = format!(
        "<input type=\"hidden\" id=\"_wpnonce\" name=\"_wpnonce\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />",
        (ctx.nonce)("bulk-posts"),
        esc_attr(ctx.request_uri)
    );
    let search_box = if search.is_empty() && shown.is_empty() {
        String::new()
    } else {
        // `search_box()` keeps the order of the list.
        let mut hidden = String::new();
        for name in ["orderby", "order"] {
            let value = get(name);
            if !value.is_empty() && value != "0" {
                hidden.push_str(&format!("<input type=\"hidden\" name=\"{name}\" value=\"{}\" />\n", esc_attr(&value)));
            }
        }
        hidden + &format!(
            "<p class=\"search-box\">\n\t<label class=\"screen-reader-text\" for=\"wpcf7-contact-search-input\">{}:</label>\n\t<input type=\"search\" id=\"wpcf7-contact-search-input\" name=\"s\" value=\"{}\" />\n\t\t<input type=\"submit\" id=\"search-submit\" class=\"button button-compact\" value=\"{}\"  /></p>\n\t\t",
            t("Search Contact Forms"),
            esc_attr(&search),
            esc_attr(&t("Search Contact Forms"))
        )
    };
    let list = table.display(&nonce_field, "", "", &caption, &rows, "");

    let mut f = HtmlFormatter::new(false);
    let mut wrap = Atts::default();
    wrap.set("class", Att::str("wrap"));
    wrap.set("id", Att::str("wpcf7-contact-form-list-table"));
    f.append_start_tag("div", Some(&wrap), "");
    let mut heading = Atts::default();
    heading.set("class", Att::str("wp-heading-inline"));
    f.append_start_tag("h1", Some(&heading), "");
    f.append_preformatted(&esc_html(&t("Contact Forms")));
    f.end_tag("h1");
    if (ctx.can)(READ_WRITE) {
        f.append_preformatted(&link(&site.site_url("wp-admin/admin.php?page=wpcf7-new"), &t("Add Contact Form"), Some("page-title-action")));
    }
    if !search.is_empty() {
        let mut subtitle = Atts::default();
        subtitle.set("class", Att::str("subtitle"));
        f.append_start_tag("span", Some(&subtitle), "");
        f.append_preformatted(&t("Search results for: <strong>%s</strong>").replacen("%s", &esc_html(&search), 1));
        f.end_tag("span");
    }
    let mut hr = Atts::default();
    hr.set("class", Att::str("wp-header-end"));
    f.append_start_tag("hr", Some(&hr), "");
    // `wpcf7_admin_warnings` and `wpcf7_admin_notices`.
    f.append_printed(&updated_message(ctx));
    let mut form_atts = Atts::default();
    form_atts.set("method", Att::str("get"));
    f.append_start_tag("form", Some(&form_atts), "");
    let mut page_input = Atts::default();
    page_input.set("type", Att::str("hidden"));
    page_input.set("name", Att::str("page"));
    page_input.set("value", Att::str(get("page")));
    f.append_start_tag("input", Some(&page_input), "");
    f.append_printed(&format!("{search_box}{list}"));
    Ok(rustpress_core::kses::kses(&f.output(), allowed_html()))
}

/// One row of the list: `column_cb()`, `column_title()` with its row actions, `column_author()`, `column_date()`.
async fn row(ctx: &AdminContext<'_>, form: &ContactForm, post: &rustpress_db::Post, page_url: &str) -> Result<String> {
    let site = ctx.site;
    let t = |text: &str| tr(site, text);
    let id = form.id;
    let edit_link = format!("{page_url}&post={id}&action=edit");
    let mut out = String::from("<tr>");
    if (ctx.can)(READ_WRITE) {
        out.push_str(&format!(
            "<td class=\"check-column\"><input id=\"cb-select-{id}\" type=\"checkbox\" name=\"post[]\" value=\"{id}\" /><label for=\"cb-select-{id}\"><span class=\"screen-reader-text\">{}</span></label></td>",
            esc_html(&t("Select %s").replacen("%s", &form.title, 1))
        ));
    } else {
        out.push_str("<td class=\"check-column\"></td>");
    }
    let mut title = format!(
        "<strong><a class=\"row-title\" href=\"{}\" aria-label=\"{}\">{}</a></strong>",
        esc_url(&edit_link),
        esc_attr(&t("Edit &#8220;%s&#8221;").replacen("%s", &form.title, 1)),
        esc_html(&form.title)
    );
    for (setting, label) in [("subscribers_only", "subscribers only"), ("demo_mode", "demo mode"), ("skip_mail", "skip mail"), ("do_not_store", "do not store")] {
        if form.is_true(setting) {
            title.push_str(&format!("<span class=\"tag info\">{}</span>", esc_html(&t(label))));
        }
    }
    if form.config_errors > 0 && (ctx.can)(READ_WRITE) {
        title.push_str(&format!("<span class=\"tag warning\">{}</span>", esc_html(&t("has config errors"))));
    }
    let mut actions = vec![("edit", link(&edit_link, &t("Edit"), None))];
    if (ctx.can)(READ_WRITE) {
        let copy_link = format!("{page_url}&post={id}&action=copy&_wpnonce={}", (ctx.nonce)(&format!("wpcf7-copy-contact-form_{id}")));
        actions.push(("copy", link(&copy_link, &t("Duplicate"), None)));
    }
    let actions: Vec<String> = actions.iter().enumerate().map(|(i, (class, html))| format!("<span class=\"{class}\">{html}{}</span>", if i + 1 < actions.len() { " | " } else { "" })).collect();
    out.push_str(&format!(
        "<th class=\"title column-title has-row-actions column-primary\" data-colname=\"{}\" scope=\"row\">{title}<div class=\"row-actions\">{}</div><button type=\"button\" class=\"toggle-row\"><span class=\"screen-reader-text\">{}</span></button></th>",
        esc_attr(&t("Title")),
        actions.join(""),
        site.tr("Show more details")
    ));
    let author = if post.post_author == 0 { None } else { site.db.user(post.post_author).await? };
    out.push_str(&format!("<td class=\"author column-author\" data-colname=\"{}\">{}</td>", esc_attr(&t("Author")), author.map(|u| esc_html(&u.display_name)).unwrap_or_default()));
    // `get_post_datetime()` formatted by `DateTimeImmutable::format()` (no translated month names).
    let date = chrono::NaiveDateTime::parse_from_str(&post.post_date, "%Y-%m-%d %H:%M:%S").map(|d| d.and_utc().timestamp()).ok();
    let date = date.map_or_else(String::new, |ts| {
        let utc = rustpress_core::datetime::SiteTimezone::utc();
        t("%1$s at %2$s").replacen("%1$s", &utc.format(&t("Y/m/d"), ts), 1).replacen("%2$s", &utc.format(&t("g:i a"), ts), 1)
    });
    out.push_str(&format!("<td class=\"date column-date\" data-colname=\"{}\">{date}</td></tr>", esc_attr(&t("Date"))));
    Ok(out)
}

/// `wpcf7_admin_updated_message()`: the notice after an action of the list or the editor.
fn updated_message(ctx: &AdminContext<'_>) -> String {
    let t = |text: &str| tr(ctx.site, text);
    let (kind, message) = match ctx.get("message").unwrap_or_default() {
        "created" => ("success", t("Contact form created.")),
        "saved" => ("success", t("Contact form saved.")),
        "deleted" => ("success", t("Contact form deleted.")),
        "failed" => ("error", t("There was an error saving the contact form.")),
        _ => return String::new(),
    };
    format!("<div class=\"notice notice-{kind}\"><p>{message}</p></div>\n")
}

/// `check_admin_referer()` that failed: `wp_nonce_ays()`.
fn expired(site: &Site) -> PageLoad {
    PageLoad::Die { status: 403, title: site.tr("Something went wrong."), message: site.tr("The link you followed has expired.") }
}

/// `wp_die( $message )` of the plugin.
fn die(site: &Site, message: &str) -> PageLoad {
    PageLoad::Die { status: 500, title: site.tr("WordPress &rsaquo; Error"), message: esc_html(&tr(site, message)) }
}

/// The properties of a form as `get_properties()` has them after `upgrade()`: the messages it never stored are the
/// defaults, a mail without a recipient goes to the administrator.
fn upgraded_properties(site: &Site, form: &ContactForm) -> Vec<(&'static str, rustpress_phpser::Value)> {
    use rustpress_phpser::{Key, Value};
    let s = |text: &str| Value::Str(text.as_bytes().to_vec());
    let empty = || Value::Array(Vec::new());
    let mut mail = form.mail.clone().unwrap_or_else(empty);
    if let Value::Array(items) = &mut mail
        && mail_lacks_recipient(items)
    {
        items.push((Key::Str(b"recipient".to_vec()), s(site.options.get("admin_email").unwrap_or_default())));
    }
    let mut messages = form.messages.clone().unwrap_or_else(empty);
    if let Value::Array(items) = &mut messages {
        for (key, text) in super::submission::DEFAULT_MESSAGES {
            if !items.iter().any(|(k, _)| matches!(k, Key::Str(name) if name == key.as_bytes())) {
                items.push((Key::Str(key.as_bytes().to_vec()), s(&tr(site, text))));
            }
        }
    }
    vec![
        ("form", s(&form.form)),
        ("mail", mail),
        ("mail_2", form.mail_2.clone().unwrap_or_else(empty)),
        ("messages", messages),
        ("additional_settings", s(&form.additional_settings)),
    ]
}

fn mail_lacks_recipient(items: &[(rustpress_phpser::Key, rustpress_phpser::Value)]) -> bool {
    !items.iter().any(|(k, _)| matches!(k, rustpress_phpser::Key::Str(name) if name == b"recipient"))
}

/// `wpcf7_load_contact_form_admin()`: the actions `copy` and `delete` of the list.
pub async fn load(ctx: &AdminContext<'_>) -> Result<Option<PageLoad>> {
    let site = ctx.site;
    let action = ctx.get("action").filter(|a| !a.is_empty() && *a != "-1").or_else(|| ctx.post_field("action").filter(|a| !a.is_empty() && *a != "-1")).or_else(|| ctx.get("action2").filter(|a| *a != "-1")).unwrap_or_default();
    let list_url = site.site_url("wp-admin/admin.php?page=wpcf7");
    let field = |name: &str| ctx.post_field(name).or_else(|| ctx.get(name)).unwrap_or_default().to_owned();
    match action {
        "copy" => {
            let id = super::php_int(&ctx.post_field("post_ID").map_or_else(|| field("post"), str::to_owned));
            if !(ctx.verify_nonce)(&field("_wpnonce"), &format!("wpcf7-copy-contact-form_{id}")) {
                return Ok(Some(expired(site)));
            }
            if !(ctx.can)(READ_WRITE) {
                return Ok(Some(die(site, "You are not allowed to edit this item.")));
            }
            let Some(writer) = ctx.writer else { return Ok(Some(die(site, "There was an error saving the contact form."))) };
            let mut location = list_url;
            if let Some((form, _)) = current_forms(site).await?.into_iter().find(|(f, _)| f.id == id) {
                // `copy()`: the title gets `_copy`, the rest stays.
                let new = super::install::store_new(site, writer, ctx.user, ctx.now, &format!("{}_copy", form.title), &form.locale, &upgraded_properties(site, &form)).await?;
                location = format!("{location}&post={new}&message=created");
            }
            Ok(Some(PageLoad::Redirect(location)))
        }
        "delete" => {
            // One form from its editor (`post_ID`), or the checked ones of the list (`post[]`).
            let single = ctx.post_field("post_ID").map(str::to_owned).or_else(|| ctx.get("post").map(str::to_owned).filter(|_| !ctx.query.iter().any(|(k, _)| k == "post[]")));
            let ids: Vec<u64> = match &single {
                Some(id) => vec![super::php_int(id)],
                None => ctx.query.iter().chain(ctx.form.iter()).filter(|(k, _)| k == "post[]").map(|(_, v)| super::php_int(v)).collect(),
            };
            let nonce_action = match &single {
                Some(id) => format!("wpcf7-delete-contact-form_{id}"),
                None => "bulk-posts".to_owned(),
            };
            if !(ctx.verify_nonce)(&field("_wpnonce"), &nonce_action) {
                return Ok(Some(expired(site)));
            }
            let forms = current_forms(site).await?;
            let mut deleted = 0;
            for id in ids.into_iter().filter(|id| *id != 0) {
                if !forms.iter().any(|(f, _)| f.id == id) {
                    continue;
                }
                if !(ctx.can)(READ_WRITE) {
                    return Ok(Some(die(site, "You are not allowed to delete this item.")));
                }
                let Some(writer) = ctx.writer else { return Ok(Some(die(site, "Error in deleting."))) };
                // `wp_delete_post( $id, true )`.
                writer.delete_rows("postmeta", "post_id", id).await?;
                writer.delete_rows("posts", "ID", id).await?;
                deleted += 1;
            }
            Ok(Some(PageLoad::Redirect(if deleted > 0 { format!("{list_url}&message=deleted") } else { list_url })))
        }
        _ => Ok(None),
    }
}

/// `WPCF7_Help_Tabs`: the help of the list.
pub fn help(site: &Site, slug: &str) -> AdminHelp {
    let t = |text: &str| tr(site, text);
    let sidebar = format!(
        "<p><strong>{}</strong></p><p><a href=\"{}\">{}</a></p><p><a href=\"{}\">{}</a></p><p><a href=\"{}\">{}</a></p>",
        t("For more information:"),
        t("https://contactform7.com/docs/"),
        t("Docs"),
        t("https://contactform7.com/faq/"),
        t("FAQ"),
        t("https://contactform7.com/support/"),
        t("Support")
    );
    let tabs = match slug {
        "wpcf7" => vec![
            HelpTab { id: "list_overview".into(), title: t("Overview"), content: format!("<p>{}</p>", t("On this screen, you can manage contact forms your team has created with Contact Form 7. You can embed contact forms in post content or a block theme template by inserting a Contact Form 7 block. For use in the classic editor environment, traditional shortcodes are also provided.")) },
            HelpTab {
                id: "list_available_actions".into(),
                title: t("Available Actions"),
                content: format!(
                    "<p>{}</p><p>{}</p><p>{}</p>",
                    t("Hovering over a row in the contact forms list will display action links that allow you to manage your contact form. You can perform the following actions:"),
                    t("<strong>Edit</strong> - Navigates to the editing screen for that contact form. You can also reach that screen by clicking on the contact form title."),
                    t("<strong>Duplicate</strong> - Clones that contact form. A cloned contact form inherits all content from the original, but has a different ID.")
                ),
            },
        ],
        _ => return AdminHelp::default(),
    };
    AdminHelp { tabs, sidebar }
}
