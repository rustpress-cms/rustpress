//! `wp-admin/index.php`: the dashboard. Only the "At a Glance" box is built so far.

use rustpress_core::formatting::esc_html;
use rustpress_db::PostQuery;

use super::{AdminResponse, Ctx, render};
use crate::chrome::{Assets, Screen};

/// `Theme Name:` of the active theme's `style.css`.
fn theme_name(ctx: &Ctx<'_>) -> String {
    let slug = &ctx.site().settings.stylesheet;
    let root = ctx.env.loader.includes.parent();
    let css = root.map(|root| root.join("wp-content").join("themes").join(slug).join("style.css")).and_then(|path| std::fs::read(path).ok()).unwrap_or_default();
    let head = String::from_utf8_lossy(&css[..css.len().min(8192)]).into_owned();
    head.lines().find_map(|line| line.trim_start_matches([' ', '*', '\t']).strip_prefix("Theme Name:").map(|name| name.trim().to_owned())).unwrap_or_else(|| slug.clone())
}

async fn count(ctx: &Ctx<'_>, post_type: &str, status: &str) -> u64 {
    let query = PostQuery { post_types: vec![post_type.into()], statuses: vec![status.into()], ..Default::default() };
    ctx.env.db.count_posts(&query).await.unwrap_or(0)
}

/// `wp_dashboard_right_now()`.
async fn at_a_glance(ctx: &Ctx<'_>) -> String {
    let site = ctx.site();
    let mut items = String::new();
    for (post_type, class) in [("post", "post-count"), ("page", "page-count")] {
        let n = count(ctx, post_type, "publish").await;
        let text = match post_type {
            "post" => site.trn("%s Post", "%s Posts", n as i64),
            _ => site.trn("%s Page", "%s Pages", n as i64),
        };
        let text = rustpress_core::i18n::sprintf(&text, &[&site.number_i18n(n)]);
        let edit = if post_type == "post" { "edit.php".to_owned() } else { format!("edit.php?post_type={post_type}") };
        if ctx.can("edit_posts") {
            items.push_str(&format!("<li class=\"{class}\"><a href=\"{edit}\">{text}</a></li>"));
        } else {
            items.push_str(&format!("<li class=\"{class}\">{text}</li>"));
        }
    }
    let comments = ctx.env.db.count_comments_with_status("1").await.unwrap_or(0);
    let text = rustpress_core::i18n::sprintf(&site.trn("%s Comment", "%s Comments", comments as i64), &[&site.number_i18n(comments)]);
    items.push_str(&format!("<li class=\"comment-count\"><a href=\"edit-comments.php\">{text}</a></li>"));
    let theme = theme_name(ctx);
    let message = rustpress_core::i18n::sprintf(&ctx.tr("WordPress %1$s running %2$s theme."), &[&ctx.env.loader.version, &format!("<a href=\"themes.php\">{}</a>", esc_html(&theme))]);
    format!("<div class=\"main\">\n<ul>{items}</ul>\n<p id=\"wp-version-message\"><span id=\"wp-version\">{message}</span></p>\n</div>")
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let title = tr("Dashboard");
    let glance = at_a_glance(ctx).await;
    let content = format!(
        "<div class=\"wrap\">\n\t<h1>{title}</h1>\n\t<div id=\"dashboard-widgets-wrap\">\n\t<div id=\"dashboard-widgets\" class=\"metabox-holder\">\n\t<div id=\"postbox-container-1\" class=\"postbox-container\">\n\t<div id=\"normal-sortables\" class=\"meta-box-sortables\"><div id=\"dashboard_right_now\" class=\"postbox \">\n<div class=\"postbox-header\"><h2 class=\"hndle\" id=\"dashboard_right_now-title\">{}</h2>\n</div><div class=\"inside\">\n{glance}\n</div>\n</div></div>\n\t</div>\n\t<div id=\"postbox-container-2\" class=\"postbox-container\">\n\t<div id=\"side-sortables\" class=\"meta-box-sortables\"></div>\n\t</div>\n\t<div id=\"postbox-container-3\" class=\"postbox-container\">\n\t<div id=\"column3-sortables\" class=\"meta-box-sortables\"></div>\n\t</div>\n\t<div id=\"postbox-container-4\" class=\"postbox-container\">\n\t<div id=\"column4-sortables\" class=\"meta-box-sortables\"></div>\n\t</div>\n\t</div>\n\n\t</div>\n\n\t</div>\n\n",
        tr("At a Glance")
    );
    let screen = Screen { file: "index.php".into(), parent_file: "index.php".into(), submenu_file: Some("index.php".into()), title, ..Default::default() };
    render(ctx, &screen, &content, &Assets { scripts: vec!["dashboard".into()], ..Default::default() }).await
}
