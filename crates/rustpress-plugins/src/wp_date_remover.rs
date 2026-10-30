//! WP Date Remover 1.1.3: hides the date of posts in chosen categories.
//!
//! The categories carry the term meta `wp-date-remover` = 1 (set on the plugin's settings page). On `init` the plugin
//! collects them from `get_categories()`; on `the_post` it makes `get_the_date()` & co return `false` for posts in one
//! of them and remembers the post, and in `wp_footer` it prints jQuery lines that hide what a theme printed anyway.
//! Its stylesheet and script (both empty, `ver` 1.0.0, the script with `jquery`) are on every page.
//!
//! In the administration it adds "Settings > WP Date Remover" (a checkbox per category), its own (empty) assets on
//! every screen, a "Settings" link in the list of plugins and a notice on every screen until it is closed (Ajax
//! `wp_date_remover_hide_notice`). The settings form has no nonce; RustPress saves it only when it was sent from a
//! page of the site itself.

use futures::future::BoxFuture;
use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_core::native::{ActivePlugin, AdminContext, AdminPage, AjaxAnswer, Assets, NativePlugin};
use rustpress_core::{Result, Site};
use rustpress_db::{Post, Term};

pub struct WpDateRemover;

/// The version the plugin hands to its assets (`Wp_Date_Remover::$version`).
const ASSET_VERSION: &str = "1.0.0";

impl NativePlugin for WpDateRemover {
    fn file(&self) -> &'static str {
        "wp-date-remover/wp-date-remover.php"
    }

    fn version(&self) -> &'static str {
        "1.1.3"
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move {
            let categories = site.db.terms("category").await?;
            let flagged: Vec<u64> = site.db.term_meta_by_key("wp-date-remover").await?.into_iter().filter(|(_, value)| loosely_one(value)).map(|(id, _)| id).collect();
            let categories = shown_categories(&categories).into_iter().filter(|id| flagged.contains(id)).collect();
            Ok(Box::new(Active { categories }) as Box<dyn ActivePlugin>)
        })
    }
}

/// `$value == 1` for the string `get_term_meta()` returns.
fn loosely_one(value: &str) -> bool {
    value.trim_start().trim_end_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']).parse::<f64>().is_ok_and(|n| n == 1.0)
}

/// The IDs `get_categories()` returns: `hide_empty` keeps a category with posts or with a descendant that has some.
fn shown_categories(categories: &[Term]) -> Vec<u64> {
    let has_posts = |term: &Term| {
        let mut stack = vec![term.term_id];
        let mut seen = Vec::new();
        while let Some(id) = stack.pop() {
            if seen.contains(&id) {
                continue;
            }
            seen.push(id);
            if categories.iter().any(|t| t.term_id == id && t.count > 0) {
                return true;
            }
            stack.extend(categories.iter().filter(|t| t.parent == id).map(|t| t.term_id));
        }
        false
    };
    categories.iter().filter(|t| has_posts(t)).map(|t| t.term_id).collect()
}

struct Active {
    /// `$wdr_cat_arry`: the chosen categories.
    categories: Vec<u64>,
}

impl Active {
    /// `in_category( $term_id )` for one of the chosen categories.
    async fn in_chosen_category(&self, site: &Site, post_id: u64) -> Result<bool> {
        if self.categories.is_empty() {
            return Ok(false);
        }
        Ok(site.db.terms_for_post(post_id, Some("category")).await?.iter().any(|t| self.categories.contains(&t.term_id)))
    }
}

impl ActivePlugin for Active {
    fn enqueue(&self, site: &Site, assets: &mut Assets) {
        let base = site.site_url("wp-content/plugins/wp-date-remover/public");
        assets.style("wp-date-remover", format!("{base}/css/wp-date-remover-public.css"), &[], ASSET_VERSION, "all");
        assets.script("wp-date-remover", format!("{base}/js/wp-date-remover-public.js"), &["jquery"], ASSET_VERSION, false);
    }

    fn hides_date<'a>(&'a self, site: &'a Site, post: &'a Post) -> BoxFuture<'a, Result<bool>> {
        Box::pin(self.in_chosen_category(site, post.id))
    }

    fn wp_footer<'a>(&'a self, site: &'a Site, looped: &'a [u64]) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            // `$wdr_post_arry`: each post once, in the order the loop reached it.
            let mut hidden: Vec<u64> = Vec::new();
            for &id in looped {
                if !hidden.contains(&id) && self.in_chosen_category(site, id).await? {
                    hidden.push(id);
                }
            }
            Ok(footer_script(&hidden))
        })
    }

    fn admin_pages(&self, _site: &Site) -> Vec<AdminPage> {
        vec![AdminPage { parent: "options-general.php", slug: "wp-date-remover", page_title: TITLE.into(), menu_title: TITLE.into(), capability: "manage_options" }]
    }

    fn admin_enqueue(&self, ctx: &AdminContext<'_>, assets: &mut Assets) {
        let base = ctx.site.site_url("wp-content/plugins/wp-date-remover/admin");
        assets.style("wp-date-remover", format!("{base}/css/wp-date-remover-admin.css"), &[], ASSET_VERSION, "all");
        assets.script("wp-date-remover", format!("{base}/js/wp-date-remover-admin.js"), &["jquery"], ASSET_VERSION, false);
    }

    /// `wp-date-remover-admin-display-notice.php`: its comment always, the notice until it was closed.
    fn admin_notices<'a>(&'a self, ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let show = ctx.site.options.get("wdr_admin_notices_show").is_none_or(|v| !v.is_empty() && v != "0");
            let mut out = String::from("\r\n<!-- This file should primarily consist of HTML with a little bit of PHP. -->\r\n");
            if show {
                out.push_str(NOTICE);
            }
            Ok(out)
        })
    }

    fn admin_page<'a>(&'a self, _slug: &'a str, ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(settings_page(ctx))
    }

    fn plugin_action_links(&self, site: &Site, links: Vec<String>) -> Vec<String> {
        let mut all = vec![format!("<a href=\"{}\">Settings</a>", site.site_url("wp-admin/options-general.php?page=wp-date-remover"))];
        all.extend(links);
        all
    }

    /// `wp_date_remover_hide_notice()` for any signed-in user; the answer is `admin-ajax.php`'s `0`.
    fn admin_ajax<'a>(&'a self, action: &'a str, ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<Option<AjaxAnswer>>> {
        Box::pin(async move {
            if action != "wp_date_remover_hide_notice" {
                return Ok(None);
            }
            if let Some(writer) = ctx.writer {
                writer.update_option("wdr_admin_notices_show", "0").await?;
            }
            Ok(Some(AjaxAnswer::text(200, "0")))
        })
    }
}

/// The title of the settings page and of its menu entry.
const TITLE: &str = "WP Date Remover";

/// The notice of `wp-date-remover-admin-display-notice.php` (the file has Windows line ends).
const NOTICE: &str = "<div id=\"wp-date-remover-notice\" class=\"update-nag\" style=\"border-left: 4px solid lightgreen;\">\r\n\t <b>Thank you for using WP Date Remover</b>\r\n\r\n<p style='font-size: 1.15em;'>Want to get WordPress Plugins, Videos, and Other Useful Resources - FREE!</em></p>\r\n\r\n<p>\r\n\r\n<div>\r\n<div class=\"_form_1\"></div><script src=\"https://shm.activehosted.com/f/embed.php?id=1\" type=\"text/javascript\" charset=\"utf-8\"></script><br><br>\r\n<input id=\"wp_date_remover_no_btn\" type=\"button\" class=\"button\" value=\"Close\" style='margin-left: 0.5em;'>\r\n</div>\r\n</p>\r\n</div>\r\n";

/// `wp-date-remover-admin-display.php`: moves the settings of old versions to the term meta, saves the form and
/// prints a checkbox per category, three to a row.
async fn settings_page(ctx: &AdminContext<'_>) -> Result<String> {
    let site = ctx.site;
    // Old versions kept `removedt_<term_id>` => 1 in the option `wp-date-remover`.
    if let (Some(writer), Some(old)) = (ctx.writer, site.options.value("wp-date-remover"))
        && let Some(items) = old.as_array()
    {
        for (key, value) in items {
            let key = match key {
                rustpress_phpser::Key::Str(k) => String::from_utf8_lossy(k).into_owned(),
                rustpress_phpser::Key::Int(n) => n.to_string(),
            };
            if value.as_i64() == Some(1)
                && let Some(term_id) = key.split('_').nth(1).and_then(|id| id.parse::<u64>().ok())
            {
                writer.update_term_meta(term_id, "wp-date-remover", "1").await?;
                writer.delete_option("wp-date-remover").await?;
            }
        }
    }
    let categories = site.db.terms("category").await?;
    let shown = shown_categories(&categories);
    let categories: Vec<&Term> = categories.iter().filter(|t| shown.contains(&t.term_id)).collect();
    let mut saved = false;
    if ctx.post
        && ctx.post_field("wp_date_btn_submit").is_some()
        && ctx.same_origin
        && let Some(writer) = ctx.writer
    {
        for term in &categories {
            let value = ctx.post_field(&format!("wp-date-remover-removedt[{}]", term.term_id)).is_some_and(loosely_one);
            writer.update_term_meta(term.term_id, "wp-date-remover", if value { "1" } else { "0" }).await?;
        }
        saved = true;
    }
    let meta = site.db.term_meta_by_key("wp-date-remover").await?;
    let rows: Vec<(u64, String, bool)> = categories.iter().map(|t| (t.term_id, t.name.clone(), meta.iter().any(|(id, value)| *id == t.term_id && loosely_one(value)))).collect();
    Ok(settings_html(TITLE, saved, &rows))
}

/// The markup of the settings page for `(term_id, name, checked)` of each category.
fn settings_html(title: &str, saved: bool, categories: &[(u64, String, bool)]) -> String {
    let mut out = String::from("\r\n<!-- This file should primarily consist of HTML with a little bit of PHP. -->\r\n<div class=\"wrap\">\r\n\r\n\t");
    if saved {
        out.push_str("<div class=\"updated\"><p>Settings saved.</p></div>");
    }
    out.push_str(&format!("    \r\n    <h2>{}</h2>\r\n    ", esc_html(title)));
    if !categories.is_empty() {
        out.push_str("    <form method=\"post\" name=\"wdr_settings\">\r\n    <table class=\"form-table\">\r\n    <tbody>\r\n        <!-- remove date and time from specific categories -->\r\n        ");
        for (index, (id, name, checked)) in categories.iter().enumerate() {
            let i = index + 1;
            if i == 1 || i % 3 == 1 {
                out.push_str("\t\t\r\n\t\t\t\t\t<tr>\r\n        ");
            }
            out.push_str(&format!(
                "\t\t\t\t\t\t<td>\r\n\t\t\t\t\t\t\t<fieldset>\r\n\t\t\t\t\t\t\t\t<legend class=\"screen-reader-text\"><span>Remove Date/Time</span></legend>\r\n\t\t\t\t\t\t\t\t<label for=\"wp-date-remover-removedt_{id}\">\r\n\t\t\t\t\t\t\t\t\t<input type=\"checkbox\" value=\"1\" {} id=\"wp-date-remover-removedt_{id}\" name=\"wp-date-remover-removedt[{id}]\">\r\n\t\t\t\t\t\t\t\t\t <span>{}</span>\r\n\t\t\t\t\t\t\t\t</label>\r\n\t\t\t\t\t\t\t</fieldset>\r\n\t\t\t\t\t   </td>\r\n\r\n       \t",
                if *checked { "checked" } else { "" },
                esc_attr(name)
            ));
            if i % 3 == 0 || i == categories.len() {
                out.push_str("        \t\t\t</tr>\r\n        ");
            }
        }
        out.push_str("  \t</tbody>\r\n\t</table>\r\n             \t<input type=\"submit\" name=\"wp_date_btn_submit\" id=\"submit\" class=\"button button-primary\" value=\"Save all changes\"  />\r\n    </form>\r\n\t");
    }
    out.push_str("</div>");
    out
}

/// `wp_date_remover_custom_script()`, with the whitespace its template leaves.
fn footer_script(posts: &[u64]) -> String {
    let mut out = String::from("\t\t<script type=\"text/javascript\">\n\t\t");
    for id in posts {
        out.push_str(&format!(
            "\t\t\t\t\tjQuery(\"#post-{id} .entry-meta .date\").css(\"display\",\"none\");\n\t\t\t\t\tjQuery(\"#post-{id} .entry-date\").css(\"display\",\"none\");\n\t\t\t\t\tjQuery(\"#post-{id} .posted-on\").css(\"display\",\"none\");\n\t\t"
        ));
    }
    out.push_str("\t\t</script>\n\t");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(term_id: u64, parent: u64, count: i64) -> Term {
        Term { term_id, term_taxonomy_id: term_id, name: format!("c{term_id}"), slug: format!("c{term_id}"), taxonomy: "category".into(), description: String::new(), parent, count }
    }

    #[test]
    fn empty_categories_count_through_their_children() {
        let terms = [category(1, 0, 0), category(2, 1, 3), category(3, 0, 0), category(4, 3, 0)];
        assert_eq!(shown_categories(&terms), vec![1, 2]);
    }

    #[test]
    fn meta_is_compared_like_php() {
        assert!(loosely_one("1"));
        assert!(loosely_one(" 1.0 "));
        assert!(!loosely_one("1abc"));
        assert!(!loosely_one(""));
        assert!(!loosely_one("0"));
    }

    #[test]
    fn footer_keeps_the_template_whitespace() {
        assert_eq!(footer_script(&[]), "\t\t<script type=\"text/javascript\">\n\t\t\t\t</script>\n\t");
        assert_eq!(
            footer_script(&[7]),
            "\t\t<script type=\"text/javascript\">\n\t\t\t\t\t\t\tjQuery(\"#post-7 .entry-meta .date\").css(\"display\",\"none\");\n\t\t\t\t\tjQuery(\"#post-7 .entry-date\").css(\"display\",\"none\");\n\t\t\t\t\tjQuery(\"#post-7 .posted-on\").css(\"display\",\"none\");\n\t\t\t\t</script>\n\t"
        );
    }
}
