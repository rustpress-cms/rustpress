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

/// `wpcf7_kses_allowed_html( 'form' )`: the rules the screens of the plugin print their markup through `wp_kses()`
/// with.
pub fn form_context() -> Allowed {
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
}

/// The rules of `contact-forms.php`: `form` keeps only `method`.
fn allowed_html() -> &'static Allowed {
    static ALLOWED: OnceLock<Allowed> = OnceLock::new();
    ALLOWED.get_or_init(|| form_context().merged(&[("form", &["method"])]))
}

/// `wpcf7_link( $url, $anchor_text, $atts )`.
fn link(url: &str, text: &str, class: Option<&str>) -> String {
    let mut atts = Atts::default();
    atts.set("id", Att::opt(None));
    atts.set("class", Att::opt(class.map(str::to_owned)));
    atts.set("href", Att::str(esc_url(url)));
    format!("<a {}>{}</a>", format_atts(&atts), esc_html(text))
}

/// `get_user_option( $name )`: the option of the site's table prefix, else the plain one.
async fn user_option(site: &Site, user: u64, name: &str) -> Result<Option<String>> {
    if let Some(value) = site.db.user_meta(user, &format!("{}{name}", site.db.table_prefix())).await? {
        return Ok(Some(value));
    }
    Ok(site.db.user_meta(user, name).await?)
}

/// The contact forms as they are now in the database (the snapshot may be older than a change just made).
pub async fn current_forms(site: &Site) -> Result<Vec<(ContactForm, rustpress_db::Post)>> {
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
    let per_page = user_option(site, ctx.user, "wpcf7_contact_forms_per_page").await?.and_then(|v| v.trim().parse::<u64>().ok()).filter(|n| *n > 0).unwrap_or(20);
    // `get_hidden_columns()`: what the user hid in the screen options.
    let hidden: Vec<String> = match user_option(site, ctx.user, &format!("manage{}columnshidden", ctx.hook_suffix)).await?.map(|raw| rustpress_phpser::unserialize(raw.as_bytes())) {
        Some(Ok(rustpress_phpser::Value::Array(items))) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    };
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
    let columns: Vec<Column> = columns.into_iter().map(|mut c| {
        c.hidden = hidden.contains(&c.key);
        c
    }).collect();
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

/// `wpcf7_admin_management_page()`: the editor of the form `post`, else the list.
pub async fn management_page(ctx: &AdminContext<'_>) -> Result<String> {
    let id = super::php_int(ctx.get("post").unwrap_or_default());
    if id != 0
        && let Some((form, _)) = current_forms(ctx.site).await?.into_iter().find(|(f, _)| f.id == id)
    {
        return Ok(super::editor::page(ctx, &super::editor::Edited { form: &form, initial: false }, &updated_message(ctx)));
    }
    list_page(ctx).await
}

/// `wpcf7_admin_add_new_page()`: the editor of a new form from the template.
pub fn add_new_page(ctx: &AdminContext<'_>) -> String {
    let form = super::install::template(ctx.site);
    super::editor::page(ctx, &super::editor::Edited { form: &form, initial: true }, &updated_message(ctx))
}

/// `wpcf7_dashboard_widgets()`: id, title and column.
fn dashboard_widgets(site: &Site) -> Vec<(&'static str, String, &'static str)> {
    let t = |text: &str| tr(site, text);
    vec![
        ("wpcf7_dashboard_right_now", t("At a Glance"), "normal"),
        ("wpcf7_dashboard_news", t("News"), "normal"),
        ("wpcf7_get_support", t("Get Support"), "side"),
        ("wpcf7_spam_protection", t("Spam Protection"), "side"),
        ("wpcf7_save_messages", t("Save Messages"), "side"),
    ]
}

/// The boxes of the dashboard for the screen options (`meta_box_prefs()`).
pub fn dashboard_boxes(site: &Site) -> Vec<(String, String)> {
    dashboard_widgets(site).into_iter().map(|(id, title, _)| (id.to_owned(), title)).collect()
}

/// `wpcf7_dashboard_right_now()`.
async fn right_now(site: &Site) -> Result<String> {
    let t = |text: &str| tr(site, text);
    let count = current_forms(site).await?.len() as i64;
    let mut f = HtmlFormatter::new(false);
    f.open("ul", &[]);
    f.open("li", &[("class", Att::str("page-count"))]);
    f.open("a", &[("href", Att::str(site.site_url("wp-admin/admin.php?page=wpcf7")))]);
    let text = site.i18n().ngettext(DOMAIN, "%s Contact form", "%s Contact forms", count);
    f.append_preformatted(&esc_html(&text.replacen("%s", &site.number_i18n(count as u64), 1)));
    f.end_tag("ul");
    f.open("p", &[]);
    f.append_preformatted(&esc_html(&t("Contact Form 7 version %s is running.").replacen("%s", super::VERSION, 1)));
    // `get_plugin_updates()`: the update WordPress found for the plugin.
    let updates = site.options.fetch(&site.db, "_site_transient_update_plugins").await?.unwrap_or_default();
    if let Ok(transient) = rustpress_phpser::unserialize(updates.as_bytes())
        && let Some(rustpress_phpser::Value::Object { props, .. }) = Some(transient)
        && props.iter().any(|(k, v)| matches!(k, rustpress_phpser::Key::Str(name) if name == b"response") && v.get("contact-form-7/wp-contact-form-7.php").is_some())
    {
        f.append_whitespace();
        f.open("a", &[("href", Att::str(site.site_url("wp-admin/update-core.php")))]);
        f.append_preformatted(&esc_html(&t("&raquo; New version available")));
    }
    Ok(kses(&f.output()))
}

/// `wpcf7_dashboard_news()`: the cached feed (`wp_dashboard_cached_rss_widget()`), else the placeholder its script
/// replaces.
async fn news(site: &Site) -> Result<String> {
    let key = format!("dash_v2_{:x}", md5::compute(format!("wpcf7_dashboard_cf7com_news_{}", site.locale())));
    let cached = site.options.fetch(&site.db, &format!("_transient_{key}")).await?;
    let fresh = match site.options.fetch(&site.db, &format!("_transient_timeout_{key}")).await?.and_then(|t| t.trim().parse::<i64>().ok()) {
        Some(timeout) => timeout >= std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64),
        None => true,
    };
    let inner = match cached.filter(|_| fresh) {
        Some(feed) => feed,
        None => format!(
            "<p class=\"widget-loading hide-if-no-js\">{}</p><div class=\"notice notice-error inline hide-if-js\"><p>{}</p></div>",
            site.tr("Loading&hellip;"),
            site.tr("This widget requires JavaScript.")
        ),
    };
    Ok(format!("<div class=\"cf7com-news hide-if-no-js\">{inner}</div>"))
}

/// The paragraphs of the boxes Get Support, Spam Protection and Save Messages.
fn text_box(site: &Site, id: &str) -> String {
    let t = |text: &str| tr(site, text);
    let mut f = HtmlFormatter::new(false);
    match id {
        "wpcf7_get_support" => {
            f.open("p", &[]);
            f.append_preformatted(&t("Do you need help? These support channels are available:"));
            f.open("ul", &[]);
            f.open("li", &[("class", Att::str("public"))]);
            f.open("a", &[("href", Att::str("https://wordpress.org/support/plugin/contact-form-7/"))]);
            f.append_preformatted(&t("Public support forum on WordPress.org"));
            f.open("li", &[("class", Att::str("private"))]);
            f.open("a", &[("href", Att::str("https://contactform7.com/custom-development/"))]);
            f.append_preformatted(&t("Private support by Codeable experts"));
            f.end_tag("ul");
            f.open("p", &[]);
            f.append_preformatted(&t("Or, consult the <a href=\"%1$s\">docs</a> and <a href=\"%2$s\">FAQ</a>.").replacen("%1$s", "https://contactform7.com/docs/", 1).replacen("%2$s", "https://contactform7.com/faq/", 1));
        }
        "wpcf7_spam_protection" => {
            f.open("p", &[]);
            f.append_preformatted(&t("Spammers target everything. Your contact forms are not an exception."));
            f.open("p", &[]);
            f.append_preformatted(
                &t("To provide effective protection, Contact Form 7 recommends using <a href=\"%1$s\"><strong>Turnstile</strong></a> (good at blocking bots) and <a href=\"%2$s\"><strong>Akismet</strong></a> (good at blocking human spammers) together.")
                    .replacen("%1$s", "https://contactform7.com/turnstile-integration/", 1)
                    .replacen("%2$s", "https://contactform7.com/spam-filtering-with-akismet/", 1),
            );
            f.open("p", &[]);
            f.append_preformatted(&t("You can also block submissions that contain specific keywords or come from specific IP addresses (<a href=\"%s\"><strong>Disallowed list</strong></a>).").replacen("%s", "https://contactform7.com/comment-blacklist/", 1));
        }
        _ => {
            f.open("p", &[]);
            f.append_preformatted(
                &t("Contact Form 7 itself doesn&#8217;t have a feature to store submitted messages; therefore, there is always a risk of losing important messages. Contact Form 7 recommends installing a separate plugin called <a href=\"%s\"><strong>Flamingo</strong></a>, which is dedicated to storing messages in the database.")
                    .replacen("%s", "https://contactform7.com/save-submitted-messages-with-flamingo/", 1),
            );
            f.open("p", &[]);
            f.append_preformatted(
                &t("If you use contact forms for customer relationship management, consider activating the <a href=\"%s\"><strong>Brevo</strong></a> integration. With the integration, Contact Form 7 can safely connect to the leading CRM platform in Europe, while protecting user privacy.")
                    .replacen("%s", "https://contactform7.com/sendinblue-integration/", 1),
            );
        }
    }
    kses(&f.output())
}

/// `print()` of a formatter made with the default options.
fn kses(html: &str) -> String {
    static ALLOWED: OnceLock<Allowed> = OnceLock::new();
    rustpress_core::kses::kses(html, ALLOWED.get_or_init(form_context))
}

/// `wpcf7_admin_dashboard_page()` (`admin/dashboard.php`).
pub async fn dashboard_page(ctx: &AdminContext<'_>) -> Result<String> {
    let site = ctx.site;
    let mut widgets = Vec::new();
    for (id, title, column) in dashboard_widgets(site) {
        let body = match id {
            "wpcf7_dashboard_right_now" => right_now(site).await?,
            "wpcf7_dashboard_news" => news(site).await?,
            other => text_box(site, other),
        };
        widgets.push(rustpress_admin::screens::dashboard::PluginWidget { id: id.to_owned(), title, column, body });
    }
    let dashboard = rustpress_admin::screens::dashboard::plugin_dashboard(site, ctx.user, ctx.hook_suffix, &widgets, ctx.nonce).await;
    let mut f = HtmlFormatter::new(false);
    f.open("div", &[("class", Att::str("wrap"))]);
    f.open("h1", &[]);
    f.append_preformatted(&esc_html(&tr(site, "Contact Form 7 Dashboard")));
    f.end_tag("h1");
    f.append_printed("");
    f.open("div", &[("id", Att::str("dashboard-widgets-wrap"))]);
    f.append_printed(&dashboard);
    Ok(kses(&f.output()))
}

/// A service of the integration page: name, title, categories, the link of its infobox, its description, the title
/// of its documentation link and the address of that page.
struct Service {
    name: &'static str,
    title: &'static str,
    category: &'static str,
    link: Option<(&'static str, &'static str)>,
    description: &'static str,
    doc: (&'static str, &'static str),
    setup: bool,
}

/// The services in the order the modules register them (`WPCF7_Integration::add_service()`).
const SERVICES: &[Service] = &[
    Service {
        name: "sendinblue",
        title: "Brevo",
        category: "Email marketing",
        link: Some(("https://get.brevo.com/wpcf7-integration", "brevo.com")),
        description: "Store and organize your contacts while protecting user privacy on Brevo, the leading CRM & email marketing platform in Europe. Brevo offers unlimited contacts and advanced marketing features.",
        doc: ("https://contactform7.com/sendinblue-integration/", "Brevo integration"),
        setup: true,
    },
    Service {
        name: "akismet",
        title: "Akismet",
        category: "Spam protection",
        link: Some(("https://akismet.com/", "akismet.com")),
        description: "CAPTCHAs are designed to distinguish spambots from humans, and are therefore helpless against human spammers. In contrast to CAPTCHAs, Akismet checks form submissions against the global database of spam; this means Akismet is a comprehensive solution against spam. This is why we consider Akismet to be the centerpiece of the spam prevention strategy.",
        doc: ("https://contactform7.com/spam-filtering-with-akismet/", "Spam filtering with Akismet"),
        setup: false,
    },
    Service {
        name: "turnstile",
        title: "Turnstile",
        category: "Spam protection",
        link: Some(("https://www.cloudflare.com/application-services/products/turnstile/", "cloudflare.com")),
        description: "Turnstile is Cloudflare&#8217;s smart CAPTCHA alternative, which confirms web visitors are real and blocks unwanted bots without slowing down web experiences for real users.",
        doc: ("https://contactform7.com/turnstile-integration/", "Cloudflare Turnstile integration"),
        setup: true,
    },
    Service {
        name: "recaptcha",
        title: "reCAPTCHA",
        category: "Spam protection",
        link: Some(("https://www.google.com/recaptcha/intro/index.html", "google.com/recaptcha")),
        description: "reCAPTCHA protects you against spam and other types of automated abuse. With Contact Form 7&#8217;s reCAPTCHA integration module, you can block abusive form submissions by spam bots.",
        doc: ("https://contactform7.com/recaptcha/", "reCAPTCHA (v3)"),
        setup: true,
    },
    Service {
        name: "stripe",
        title: "Stripe",
        category: "Payments",
        link: Some(("https://stripe.com/", "stripe.com")),
        description: "Stripe is a simple and powerful way to accept payments online. Stripe has no setup fees, no monthly fees, and no hidden costs. Millions of businesses rely on Stripe&#8217;s software tools to accept payments securely and expand globally.",
        doc: ("https://contactform7.com/stripe-integration/", "Stripe integration"),
        setup: true,
    },
];

/// `is_active()` of a service from what the option `wpcf7` stores for it.
fn service_active(option: &rustpress_phpser::Value, name: &str) -> bool {
    let stored = option.get(name);
    match name {
        "sendinblue" => stored.and_then(|v| v.get("api_key")).is_some_and(super::mail::php_truthy),
        "stripe" => stored.and_then(|v| v.get("api_keys")).is_some_and(super::mail::php_truthy),
        // `sitekey => secret`: the first pair.
        "turnstile" | "recaptcha" => stored.and_then(|v| v.as_array()).and_then(|pairs| pairs.first()).is_some_and(|(k, secret)| !matches!(k, rustpress_phpser::Key::Str(s) if s.is_empty()) && super::mail::php_truthy(secret)),
        _ => false,
    }
}

/// `admin/integration.php` with `list_services()`: the cards of the services (or of the one asked for). Setting up a
/// service is not offered: RustPress does not talk to these services.
pub async fn integration_page(ctx: &AdminContext<'_>) -> Result<String> {
    let site = ctx.site;
    let t = |text: &str| tr(site, text);
    let option = site.options.fetch(&site.db, "wpcf7").await?.and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok()).unwrap_or(rustpress_phpser::Value::Array(Vec::new()));
    let requested = ctx.get("service").unwrap_or_default();
    let mut cards = String::new();
    let constant_contact = ("constant_contact", "Constant Contact");
    let wanted = |name: &str| requested.is_empty() || !SERVICES.iter().any(|s| s.name == requested) && requested != constant_contact.0 || requested == name;
    for service in SERVICES.iter().filter(|s| wanted(s.name)) {
        let active = service_active(&option, service.name);
        let mut f = HtmlFormatter::new(false);
        f.open("p", &[]);
        f.append_preformatted(&esc_html(&t(service.description)));
        f.end_tag("p");
        f.open("p", &[]);
        f.open("strong", &[]);
        f.append_preformatted(&link(service.doc.0, &t(service.doc.1), None));
        f.end_tag("p");
        if active {
            f.open("p", &[("class", Att::str("dashicons-before dashicons-yes"))]);
            f.append_preformatted(&esc_html(&t("%s is active on this site.").replacen("%s", service.title, 1)));
            f.end_tag("p");
        }
        let https = site.settings.siteurl.starts_with("https://");
        if service.name == "stripe" && !https {
            f.open("p", &[("class", Att::str("dashicons-before dashicons-warning"))]);
            f.append_preformatted(&esc_html(&t("Stripe is not available on this site. It requires an HTTPS-enabled site.")));
            f.end_tag("p");
        } else if service.setup {
            f.open("p", &[]);
            f.open("a", &[("href", Att::str(esc_url(&site.site_url(&format!("wp-admin/admin.php?page=wpcf7-integration&service={}&action=setup", service.name))))), ("class", Att::str("button"))]);
            f.append_preformatted(&esc_html(&t("Setup integration")));
            f.end_tag("p");
        }
        let link_html = service.link.map(|(url, text)| link(url, text, None)).unwrap_or_default();
        cards.push_str(&format!(
            "<div class=\"card{}\" id=\"{}\">\n<h2 class=\"title\">{}</h2>\n<div class=\"infobox\">\n{}<br />\n{link_html}</div>\n<br class=\"clear\" />\n\n<div class=\"inside\">\n{}</div>\n</div>\n",
            if active { " active" } else { "" },
            esc_attr(service.name),
            esc_html(&t(service.title)),
            esc_html(&t(service.category)),
            kses(&f.output())
        ));
    }
    if wanted(constant_contact.0) {
        let mut f = HtmlFormatter::new(false);
        f.open("p", &[]);
        f.append_preformatted(&link("https://contactform7.com/2024/02/02/we-end-the-constant-contact-integration/", &t("The Constant Contact integration has been removed."), None));
        f.end_tag("p");
        cards.push_str(&format!(
            "<div class=\"card\" id=\"{}\">\n<h2 class=\"title\">{}</h2>\n<div class=\"infobox\">\n{}<br />\n</div>\n<br class=\"clear\" />\n\n<div class=\"inside\">\n{}</div>\n</div>\n",
            constant_contact.0,
            esc_html(&t(constant_contact.1)),
            esc_html(&t("Email marketing")),
            kses(&f.output())
        ));
    }
    let mut f = HtmlFormatter::new(false);
    f.open("div", &[("class", Att::str("wrap")), ("id", Att::str("wpcf7-integration"))]);
    f.open("h1", &[]);
    f.append_preformatted(&esc_html(&t("Integration with External API")));
    f.end_tag("h1");
    f.open("p", &[]);
    f.append_preformatted(&t("You can expand the possibilities of your contact forms by integrating them with external services. For details, see <a href=\"%s\">Integration with external APIs</a>.").replacen("%s", "https://contactform7.com/integration-with-external-apis/", 1));
    f.end_tag("p");
    f.append_printed(&cards);
    static ALLOWED: OnceLock<Allowed> = OnceLock::new();
    Ok(rustpress_core::kses::kses(&f.output(), ALLOWED.get_or_init(|| form_context().merged(&[("form", &["action", "method"])]))))
}

/// `wpcf7_admin_updated_message()`: the notice after an action of the list or the editor.
pub fn updated_message(ctx: &AdminContext<'_>) -> String {
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
    PageLoad::Die { status: 403, title: site.tr("An error occurred."), message: site.tr("The link you followed has expired.") }
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

/// `wpcf7_is_valid_locale()`.
fn is_valid_locale(locale: &str) -> bool {
    static LOCALE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"^[a-z]{2,3}(?:_[a-zA-Z_]{2,})?$").unwrap());
    LOCALE.is_match(locale)
}

/// `$_POST[ $name ]` of the editor, cleaned like `wpcf7_superglobal_post()`.
fn posted(ctx: &AdminContext<'_>, name: &str) -> Option<super::posted::Posted> {
    let post = super::posted::PostData::from_fields(ctx.form);
    post.get(name).map(super::posted::clean)
}

fn posted_string(value: Option<&super::posted::Posted>, key: &str) -> Option<String> {
    match value? {
        super::posted::Posted::Map(items) => items.iter().find(|(k, _)| k == key).and_then(|(_, v)| match v {
            super::posted::Posted::Str(s) => Some(s.clone()),
            super::posted::Posted::Map(_) => None,
        }),
        super::posted::Posted::Str(_) => None,
    }
}

/// `wpcf7_sanitize_mail()`.
fn sanitize_mail(input: Option<&super::posted::Posted>) -> Vec<(&'static str, rustpress_phpser::Value)> {
    use rustpress_phpser::Value;
    let text = |key: &str| posted_string(input, key).unwrap_or_default();
    let flag = |key: &str| posted_string(input, key).is_some_and(|v| !v.is_empty() && v != "0");
    let trim = |s: String| super::formatter::php_trim(&s).to_owned();
    let s = |v: String| Value::Str(v.into_bytes());
    let headers: Vec<String> = text("additional_headers").replace("\r\n", "\n").split('\n').map(|h| super::formatter::php_trim(h).to_owned()).filter(|h| !h.is_empty()).collect();
    vec![
        ("active", Value::Bool(flag("active"))),
        ("subject", s(trim(text("subject")))),
        ("sender", s(trim(text("sender")))),
        ("recipient", s(trim(text("recipient")))),
        ("body", s(trim(text("body")))),
        ("additional_headers", s(headers.join("\n"))),
        ("attachments", s(trim(text("attachments")))),
        ("use_html", Value::Bool(flag("use_html"))),
        ("exclude_blank", Value::Bool(flag("exclude_blank"))),
    ]
}

fn php_array(items: Vec<(&'static str, rustpress_phpser::Value)>) -> rustpress_phpser::Value {
    rustpress_phpser::Value::Array(items.into_iter().map(|(k, v)| (rustpress_phpser::Key::Str(k.as_bytes().to_vec()), v)).collect())
}

/// `wpcf7_save_contact_form()` of the editor's POST and `save()`: the ID of the form, `None` when there is none.
async fn save_form(ctx: &AdminContext<'_>, id: i64) -> Result<Option<(u64, bool)>> {
    use rustpress_phpser::Value;
    let site = ctx.site;
    let Some(writer) = ctx.writer else { return Ok(None) };
    let existing = if id == -1 { None } else { current_forms(site).await?.into_iter().find(|(f, _)| f.id as i64 == id).map(|(f, _)| f) };
    if id != -1 && existing.is_none() {
        return Ok(None);
    }
    let mut form = existing.clone().unwrap_or_else(|| super::install::template(site));
    let mut properties = upgraded_properties(site, &form);
    if id == -1 {
        properties = vec![
            ("form", Value::Str(form.form.clone().into_bytes())),
            ("mail", form.mail.clone().unwrap_or(Value::Array(Vec::new()))),
            ("mail_2", form.mail_2.clone().unwrap_or(Value::Array(Vec::new()))),
            ("messages", form.messages.clone().unwrap_or(Value::Array(Vec::new()))),
            ("additional_settings", Value::Str(Vec::new())),
        ];
    }
    let string = |name: &str| match posted(ctx, name) {
        Some(super::posted::Posted::Str(s)) => Some(s),
        _ => None,
    };
    // `set_title()`: no tags, no line breaks; an empty title is "Untitled".
    if let Some(title) = string("post_title") {
        static BREAKS: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"[\r\n\t ]+").unwrap());
        let title = BREAKS.replace_all(&rustpress_core::formatting::wp_strip_all_tags(&title), " ").into_owned();
        let title = super::formatter::strip_whitespaces(&title, super::formatter::Side::Both).to_owned();
        form.title = if title.is_empty() { tr(site, "Untitled") } else { title };
    }
    if let Some(locale) = string("wpcf7-locale") {
        let locale = super::formatter::php_trim(&locale).to_owned();
        form.locale = if is_valid_locale(&locale) { locale } else { "en_US".to_owned() };
    }
    let set = |properties: &mut Vec<(&'static str, Value)>, name: &'static str, value: Value| {
        if let Some(slot) = properties.iter_mut().find(|(k, _)| *k == name) {
            slot.1 = value;
        }
    };
    set(&mut properties, "form", Value::Str(super::formatter::php_trim(&string("wpcf7-form").unwrap_or_default()).as_bytes().to_vec()));
    let mut mail = sanitize_mail(posted(ctx, "wpcf7-mail").as_ref());
    mail[0].1 = Value::Bool(true);
    set(&mut properties, "mail", php_array(mail));
    set(&mut properties, "mail_2", php_array(sanitize_mail(posted(ctx, "wpcf7-mail-2").as_ref())));
    // `wpcf7_sanitize_messages()`: the messages of `wpcf7_messages()` that were sent.
    let sent = posted(ctx, "wpcf7-messages");
    let messages: Vec<(&'static str, Value)> = super::submission::DEFAULT_MESSAGES.iter().filter_map(|(key, _)| posted_string(sent.as_ref(), key).map(|v| (*key, Value::Str(super::formatter::php_trim(&v).as_bytes().to_vec())))).collect();
    set(&mut properties, "messages", php_array(messages));
    set(&mut properties, "additional_settings", Value::Str(super::formatter::php_trim(&string("wpcf7-additional-settings").unwrap_or_default()).as_bytes().to_vec()));
    let saved = match existing {
        None => super::install::store_new(site, writer, ctx.user, ctx.now, &form.title, &form.locale, &properties).await?,
        Some(old) => {
            super::install::update(site, writer, ctx.user, ctx.now, old.id, &form.title, &form.locale, &properties).await?;
            old.id
        }
    };
    Ok(Some((saved, id == -1)))
}

/// `wpcf7_load_contact_form_admin()`: the actions `save`, `copy` and `delete`.
pub async fn load(ctx: &AdminContext<'_>) -> Result<Option<PageLoad>> {
    let site = ctx.site;
    let action = ctx.get("action").filter(|a| !a.is_empty() && *a != "-1").or_else(|| ctx.post_field("action").filter(|a| !a.is_empty() && *a != "-1")).or_else(|| ctx.get("action2").filter(|a| *a != "-1")).unwrap_or_default();
    let list_url = site.site_url("wp-admin/admin.php?page=wpcf7");
    let field = |name: &str| ctx.post_field(name).or_else(|| ctx.get(name)).unwrap_or_default().to_owned();
    match action {
        "save" => {
            let id_text = ctx.post_field("post_ID").unwrap_or("-1").to_owned();
            if !(ctx.verify_nonce)(&field("_wpnonce"), &format!("wpcf7-save-contact-form_{id_text}")) {
                return Ok(Some(expired(site)));
            }
            if !(ctx.can)(READ_WRITE) {
                return Ok(Some(die(site, "You are not allowed to edit this item.")));
            }
            let id: i64 = id_text.trim().parse().unwrap_or(0);
            let saved = save_form(ctx, id).await?;
            // `wpcf7_canonicalize_name()` of the open tab.
            static NAME: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)[^0-9a-z]+").unwrap());
            let tab = NAME.replace_all(&super::posted::PostData::from_fields(ctx.form).string("active-tab"), "-").into_owned();
            let (post, message) = match saved {
                None => (0, "failed"),
                Some((id, true)) => (id, "created"),
                Some((id, false)) => (id, "saved"),
            };
            Ok(Some(PageLoad::Redirect(format!("{list_url}&post={post}&active-tab={}&message={message}", super::urlencode(&tab)))))
        }
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
pub fn help(site: &Site, screen_type: &str) -> AdminHelp {
    let t = |text: &str| tr(site, text);
    let p = |texts: &[&str]| texts.iter().map(|text| format!("<p>{}</p>", t(text))).collect::<String>();
    let tab = |id: &str, title: &str, content: String| HelpTab { id: id.into(), title: t(title), content };
    let tabs = match screen_type {
        "list" => vec![
            tab("list_overview", "Overview", p(&["On this screen, you can manage contact forms your team has created with Contact Form 7. You can embed contact forms in post content or a block theme template by inserting a Contact Form 7 block. For use in the classic editor environment, traditional shortcodes are also provided."])),
            tab(
                "list_available_actions",
                "Available Actions",
                p(&[
                    "Hovering over a row in the contact forms list will display action links that allow you to manage your contact form. You can perform the following actions:",
                    "<strong>Edit</strong> - Navigates to the editing screen for that contact form. You can also reach that screen by clicking on the contact form title.",
                    "<strong>Duplicate</strong> - Clones that contact form. A cloned contact form inherits all content from the original, but has a different ID.",
                ]),
            ),
        ],
        "edit" => vec![
            tab(
                "edit_overview",
                "Overview",
                p(&[
                    "On this screen, you can edit a contact form. A contact form is comprised of the following components:",
                    "<strong>Title</strong> is the title of a contact form. This title is only used for labeling a contact form, and can be edited.",
                    "<strong>Form</strong> is a content of HTML form. You can use arbitrary HTML, which is allowed inside a form element. You can also use Contact Form 7&#8217;s form-tags here.",
                    "<strong>Mail</strong> manages a mail template (headers and message body) that this contact form will send when users submit it. You can use Contact Form 7&#8217;s mail-tags here.",
                    "<strong>Mail (2)</strong> is an additional mail template that works similar to Mail. Mail (2) is different in that it is sent only when Mail has been sent successfully.",
                    "In <strong>Messages</strong>, you can edit various types of messages used for this contact form. These messages are relatively short messages, like a validation error message you see when you leave a required field blank.",
                    "<strong>Additional Settings</strong> provides a place where you can customize the behavior of this contact form by adding code snippets.",
                ]),
            ),
            tab(
                "edit_form_tags",
                "Form-tags",
                p(&[
                    "A form-tag is a short code enclosed in square brackets used in a form content. A form-tag generally represents an input field, and its components can be separated into four parts: type, name, options, and values. Contact Form 7 supports several types of form-tags including text fields, number fields, date fields, checkboxes, radio buttons, menus, file-uploading fields, CAPTCHAs, and quiz fields.",
                    "While form-tags have a comparatively complex syntax, you do not need to know the syntax to add form-tags because you can use the straightforward tag generator (<strong>Generate Tag</strong> button on this screen).",
                ]),
            ),
            tab(
                "edit_mail_tags",
                "Mail-tags",
                p(&[
                    "A mail-tag is also a short code enclosed in square brackets that you can use in every Mail and Mail (2) field. A mail-tag represents a user input value through an input field of a corresponding form-tag.",
                    "There are also special mail-tags that have specific names, but do not have corresponding form-tags. They are used to represent meta information of form submissions like the submitter&#8217;s IP address or the URL of the page.",
                ]),
            ),
        ],
        "integration" => vec![tab(
            "integration_overview",
            "Overview",
            p(&[
                "On this screen, you can manage services that are available through Contact Form 7. Using API will allow you to collaborate with any services that are available.",
                "You may need to first sign up for an account with the service that you plan to use. When you do so, you would need to authorize Contact Form 7 to access the service with your account.",
                "Any information you provide will not be shared with service providers without your authorization.",
            ]),
        )],
        _ => return AdminHelp::default(),
    };
    let sidebar = format!(
        "<p><strong>{}</strong></p><p>{}</p><p>{}</p><p>{}</p>",
        t("For more information:"),
        link("https://contactform7.com/docs/", &t("Docs"), None),
        link("https://contactform7.com/faq/", &t("FAQ"), None),
        link("https://contactform7.com/support/", &t("Support"), None)
    );
    AdminHelp { tabs, sidebar }
}
