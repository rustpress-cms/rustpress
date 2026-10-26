//! `WP_List_Table`: the markup around the list tables of the administration area (column headers, bulk actions,
//! pagination, the table itself).

use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_url};
use rustpress_core::urls::{add_query_arg, remove_query_arg};

/// How a column sorts: `[ orderby, desc_first, abbr, orderby_text, initial_order ]`.
#[derive(Debug, Clone)]
pub struct Sortable {
    pub orderby: String,
    pub desc_first: bool,
    pub abbr: Option<String>,
    pub initial_order: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub key: String,
    /// The header text, HTML.
    pub label: String,
    pub sortable: Option<Sortable>,
    pub hidden: bool,
}

impl Column {
    pub fn new(key: &str, label: &str) -> Self {
        Self { key: key.to_owned(), label: label.to_owned(), sortable: None, hidden: false }
    }

    pub fn sortable(mut self, orderby: &str, desc_first: bool, abbr: Option<&str>, initial_order: Option<&str>) -> Self {
        self.sortable = Some(Sortable { orderby: orderby.to_owned(), desc_first, abbr: abbr.map(str::to_owned), initial_order: initial_order.map(str::to_owned) });
        self
    }
}

/// The state shared by header, footer and navigation.
pub struct Table<'a> {
    pub site: &'a Site,
    pub plural: String,
    pub singular: String,
    pub columns: Vec<Column>,
    pub primary: String,
    /// `http://host/wp-admin/edit.php?...`: the address the links are made from.
    pub current_url: String,
    pub orderby: String,
    /// `asc` or `desc`.
    pub order: String,
    pub total_items: u64,
    pub per_page: u64,
    pub current_page: u64,
    /// `(name, label)`; an empty list hides the bulk action selector.
    pub bulk_actions: Vec<(String, String)>,
    pub has_items: bool,
    /// `table-view-{mode}` of the table (`get_user_setting( 'posts_list_mode', 'list' )`).
    pub view_mode: String,
}

/// `submit_button( $text, $type, $name, false, array( 'id' => $id ) )`.
pub fn submit_button(text: &str, kind: &str, name: &str, id: &str) -> String {
    let mut classes: Vec<String> = Vec::new();
    for word in kind.split_whitespace() {
        classes.push(if word == "primary" { "button-primary".to_owned() } else if word == "small" || word == "large" || word == "compact" || word == "hero" { format!("button-{word}") } else { word.to_owned() });
    }
    let mut parts: Vec<String> = Vec::new();
    let mut class_names = vec!["button".to_owned()];
    if kind == "primary" || kind.is_empty() {
        class_names = vec!["button".to_owned(), "button-primary".to_owned()];
    } else {
        class_names.extend(classes.into_iter().filter(|c| c != "button-primary"));
    }
    parts.push(format!("class=\"{}\"", class_names.join(" ")));
    let id_attr = if id.is_empty() { name } else { id };
    let name_html = if name.is_empty() { String::new() } else { format!(" name=\"{}\"", esc_attr(name)) };
    format!("<input type=\"submit\"{name_html} id=\"{}\" {} value=\"{}\"  />", esc_attr(id_attr), parts.join(" "), esc_attr(text))
}

impl Table<'_> {
    fn total_pages(&self) -> u64 {
        if self.per_page == 0 { 0 } else { self.total_items.div_ceil(self.per_page) }
    }

    fn column_info_hidden(&self, key: &str) -> bool {
        self.columns.iter().any(|c| c.key == key && c.hidden)
    }

    /// `print_column_headers()`.
    pub fn column_headers(&self, with_id: bool, cb_counter: u64) -> String {
        let site = self.site;
        let current_url = remove_query_arg(&self.current_url, "paged");
        let mut current_orderby = self.orderby.clone();
        let mut current_order = self.order.clone();
        let mut out = String::new();
        for column in &self.columns {
            let mut class: Vec<String> = vec!["manage-column".to_owned(), format!("column-{}", column.key)];
            let mut aria_sort = String::new();
            let mut abbr_attr = String::new();
            let mut order_text = String::new();
            let mut display = column.label.clone();
            if column.hidden {
                class.push("hidden".to_owned());
            }
            if column.key == "cb" {
                class.push("check-column".to_owned());
                display = format!("<input id=\"cb-select-all-{cb_counter}\" type=\"checkbox\" />\n\t\t\t<label for=\"cb-select-all-{cb_counter}\"><span class=\"screen-reader-text\">{}</span></label>", site.tr("Select All"));
            } else if matches!(column.key.as_str(), "posts" | "comments" | "links") {
                class.push("num".to_owned());
            }
            if column.key == self.primary {
                class.push("column-primary".to_owned());
            }
            if let Some(sortable) = &column.sortable {
                if current_orderby.is_empty()
                    && let Some(initial) = &sortable.initial_order
                {
                    current_orderby = sortable.orderby.clone();
                    current_order = initial.clone();
                }
                let order;
                if current_orderby == sortable.orderby {
                    if current_order == "asc" {
                        order = "desc";
                        aria_sort = " aria-sort=\"ascending\"".to_owned();
                    } else {
                        order = "asc";
                        aria_sort = " aria-sort=\"descending\"".to_owned();
                    }
                    class.push("sorted".to_owned());
                    class.push(current_order.clone());
                } else {
                    order = if sortable.desc_first { "desc" } else { "asc" };
                    class.push("sortable".to_owned());
                    class.push(if order == "desc" { "asc".to_owned() } else { "desc".to_owned() });
                    let text = if order == "asc" { site.tr("Sort ascending.") } else { site.tr("Sort descending.") };
                    order_text = format!(" <span class=\"screen-reader-text\">{text}</span>");
                }
                if let Some(abbr) = &sortable.abbr {
                    abbr_attr = format!(" abbr=\"{}\"", esc_attr(abbr));
                }
                let href = add_query_arg(&add_query_arg(&current_url, "orderby", &sortable.orderby), "order", order);
                display = format!(
                    "<a href=\"{}\"><span>{display}</span><span class=\"sorting-indicators\"><span class=\"sorting-indicator asc\" aria-hidden=\"true\"></span><span class=\"sorting-indicator desc\" aria-hidden=\"true\"></span></span>{order_text}</a>",
                    esc_url(&href)
                );
            }
            let tag = if column.key == "cb" { "td" } else { "th" };
            let scope = if tag == "th" { "scope=\"col\"" } else { "" };
            let id = if with_id { format!("id='{}'", column.key) } else { String::new() };
            out.push_str(&format!("<{tag} {scope} {id} class='{}' {aria_sort} {abbr_attr}>{display}</{tag}>", class.join(" ")));
        }
        out
    }

    /// `bulk_actions( $which )`.
    pub fn bulk_actions(&self, which: &str, second: bool) -> String {
        if self.bulk_actions.is_empty() {
            return String::new();
        }
        let two = if second { "2" } else { "" };
        let mut out = format!(
            "<label for=\"bulk-action-selector-{which}\" class=\"screen-reader-text\">{}</label>",
            self.site.tr("Select bulk action")
        );
        out.push_str(&format!("<select name=\"action{two}\" id=\"bulk-action-selector-{which}\">\n"));
        out.push_str(&format!("<option value=\"-1\">{}</option>\n", self.site.tr("Bulk actions")));
        for (name, title) in &self.bulk_actions {
            let class = if name == "edit" { " class=\"hide-if-no-js\"" } else { "" };
            out.push_str(&format!("\t<option value=\"{}\"{class}>{title}</option>\n", esc_attr(name)));
        }
        out.push_str("</select>\n");
        out.push_str(&submit_button(&self.site.tr("Apply"), "action compact", "bulk_action", &format!("doaction{two}")));
        out.push('\n');
        out
    }

    /// `pagination( $which )`.
    pub fn pagination(&self, which: &str) -> String {
        let site = self.site;
        if self.total_items == 0 {
            return format!("<div class=\"tablenav-pages no-pages\"><span class=\"displaying-num\">{}</span></div>", site.tr("0 items"));
        }
        let total_pages = self.total_pages();
        let current = self.current_page;
        let mut output = String::new();
        if which == "top" && total_pages > 1 {
            // heading_pagination is printed by the caller (a screen reader heading).
        }
        output.push_str(&format!(
            "<span class=\"displaying-num\">{}</span>",
            rustpress_core::i18n::sprintf(&site.trn("%s item", "%s items", self.total_items as i64), &[&site.number_i18n(self.total_items)])
        ));
        let current_url = &self.current_url;
        let mut links: Vec<String> = Vec::new();
        let (first_disabled, last_disabled) = (current == 1, current == total_pages);
        let link = |class: &str, href: String, text: String, glyph: &str| {
            format!("<a class='{class} button' href='{}'><span class='screen-reader-text'>{text}</span><span aria-hidden='true'>{glyph}</span></a>", esc_url(&href))
        };
        links.push(if first_disabled {
            "<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&laquo;</span>".to_owned()
        } else {
            link("first-page", remove_query_arg(current_url, "paged"), site.tr("First page"), "&laquo;")
        });
        links.push(if first_disabled {
            "<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&lsaquo;</span>".to_owned()
        } else {
            link("prev-page", add_query_arg(current_url, "paged", &current.saturating_sub(1).max(1).to_string()), site.tr("Previous page"), "&lsaquo;")
        });
        let (before, current_html, after);
        if which == "bottom" {
            before = format!("<span class=\"screen-reader-text\">{}</span><span id=\"table-paging\" class=\"paging-input\"><span class=\"tablenav-paging-text\">", site.tr("Current Page"));
            current_html = current.to_string();
            after = "</span></span>".to_owned();
        } else {
            before = "<span class=\"paging-input\">".to_owned();
            current_html = format!(
                "<label for=\"current-page-selector\" class=\"screen-reader-text\">{}</label><input class='current-page' id='current-page-selector' type='text'\n\t\t\t\t\tname='paged' value='{current}' size='{}' aria-describedby='table-paging' /><span class='tablenav-paging-text'>",
                site.tr("Current Page"),
                total_pages.to_string().len()
            );
            after = "</span></span>".to_owned();
        }
        let total_html = format!("<span class='total-pages'>{}</span>", site.number_i18n(total_pages));
        links.push(format!("{before}{}{after}", rustpress_core::i18n::sprintf(&site.trx("%1$s of %2$s", "paging"), &[&current_html, &total_html])));
        links.push(if last_disabled {
            "<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&rsaquo;</span>".to_owned()
        } else {
            link("next-page", add_query_arg(current_url, "paged", &(current + 1).min(total_pages).to_string()), site.tr("Next page"), "&rsaquo;")
        });
        links.push(if last_disabled {
            "<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&raquo;</span>".to_owned()
        } else {
            link("last-page", add_query_arg(current_url, "paged", &total_pages.to_string()), site.tr("Last page"), "&raquo;")
        });
        output.push_str(&format!("\n<span class='pagination-links'>{}</span>", links.join("\n")));
        let page_class = if total_pages > 0 && total_pages < 2 { " one-page" } else if total_pages == 0 { " no-pages" } else { "" };
        format!("<div class='tablenav-pages{page_class}'>{output}</div>")
    }

    /// `display_tablenav( $which )`; `nonce_field` is printed at the top only, `extra` is `extra_tablenav()`.
    pub fn tablenav(&self, which: &str, nonce_field: &str, extra: &str) -> String {
        if which == "bottom" && !self.has_items {
            return String::new();
        }
        let mut out = String::new();
        if which == "top" {
            out.push_str(nonce_field);
        }
        let visibility = if self.has_items { "" } else { " hidden" };
        out.push_str(&format!("	<div class=\"tablenav {which}\">

				<div class=\"alignleft actions bulkactions{visibility}\">
			"));
        out.push_str(&self.bulk_actions(which, which == "bottom"));
        out.push_str("		</div>
		");
        out.push_str(extra);
        if which == "top" && self.total_pages() > 1 {
            out.push_str(&format!("<h2 class='screen-reader-text'>{}</h2>", self.heading_pagination()));
        }
        out.push_str(&self.pagination(which));
        out.push_str("
		<br class=\"clear\" />
	</div>
");
        out
    }

    /// The hidden heading above the pagination (`Posts list navigation`).
    pub fn heading_pagination(&self) -> String {
        self.site.trx(&format!("{} list navigation", self.plural_title()), "list table navigation heading")
    }

    fn plural_title(&self) -> String {
        let mut chars = self.plural.chars();
        chars.next().map_or_else(String::new, |c| c.to_uppercase().collect::<String>() + chars.as_str())
    }

    /// `display()`: top navigation, the table, bottom navigation. `rows` is the HTML of the `<tr>` elements.
    pub fn display(&self, nonce_field: &str, extra_top: &str, extra_bottom: &str, caption: &str, rows: &str, list_heading: &str) -> String {
        self.display_opts(true, nonce_field, extra_top, extra_bottom, caption, rows, list_heading)
    }

    /// [`Table::display`], optionally without the top navigation (the comments screen leaves it out of an empty list).
    #[allow(clippy::too_many_arguments)]
    pub fn display_opts(&self, show_top: bool, nonce_field: &str, extra_top: &str, extra_bottom: &str, caption: &str, rows: &str, list_heading: &str) -> String {
        let mut out = String::new();
        let indent = if show_top {
            out.push_str(&self.tablenav("top", nonce_field, extra_top));
            "		"
        } else {
            out.push_str(nonce_field);
            out.push('\n');
            ""
        };
        out.push_str(&format!("{indent}<h2 class='screen-reader-text'>{list_heading}</h2><table class=\"wp-list-table widefat fixed striped table-view-{} {}\">
", self.view_mode, self.plural));
        let caption_html = if caption.is_empty() { String::new() } else { format!("<caption class=\"screen-reader-text\">{caption}</caption>") };
        out.push_str(&format!("		{caption_html}	<thead>
	<tr>
		"));
        out.push_str(&self.column_headers(true, 1));
        out.push_str("	</tr>
	</thead>

");
        let data_lists = if self.singular.is_empty() { String::new() } else { format!(" data-wp-lists='list:{}'", self.singular) };
        out.push_str(&format!("	<tbody id=\"the-list\"
		{data_lists}		>
		"));
        // `row_actions()`: in the excerpt view every row shows its actions.
        if self.view_mode == "excerpt" {
            out.push_str(&rows.replace("<div class=\"row-actions\">", "<div class=\"row-actions visible\">"));
        } else {
            out.push_str(rows);
        }
        out.push_str("	</tbody>

	<tfoot>
	<tr>
		");
        out.push_str(&self.column_headers(false, 2));
        out.push_str("	</tr>
	</tfoot>

</table>
		");
        out.push_str(&self.tablenav("bottom", nonce_field, extra_bottom));
        out.push_str("		
");
        let _ = self.column_info_hidden("");
        out
    }
}

/// `row_actions( $actions )`: `(class, html)` pairs.
pub fn row_actions(site: &Site, actions: &[(String, String)]) -> String {
    if actions.is_empty() {
        return String::new();
    }
    let mut out = String::from("<div class=\"row-actions\">");
    for (index, (class, html)) in actions.iter().enumerate() {
        let separator = if index + 1 < actions.len() { " | " } else { "" };
        out.push_str(&format!("<span class='{class}'>{html}{separator}</span>"));
    }
    out.push_str("</div>");
    out.push_str(&format!("<button type=\"button\" class=\"toggle-row\"><span class=\"screen-reader-text\">{}</span></button>", site.tr("Show more details")));
    out
}

/// `views()`: the status links above the table, `(class, html)`.
pub fn views(heading: &str, links: &[(String, String)]) -> String {
    if links.is_empty() {
        return String::new();
    }
    let mut out = format!("<h2 class='screen-reader-text'>{heading}</h2><ul class='subsubsub'>\n");
    let items: Vec<String> = links.iter().map(|(class, html)| format!("\t<li class='{class}'>{html}")).collect();
    out.push_str(&items.join(" |</li>\n"));
    out.push_str("</li>\n</ul>");
    out
}

/// `get_views_links()` entry: `<a href="..." class="current" aria-current="page">label</a>`.
pub fn view_link(url: &str, label: &str, current: bool) -> String {
    format!("<a href=\"{}\"{}>{label}</a>", esc_url(url), if current { " class=\"current\" aria-current=\"page\"" } else { "" })
}
