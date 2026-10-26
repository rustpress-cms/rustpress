//! The settings screens besides General: Writing, Reading, Discussion and Media. `options.php` saves them.

use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_core::i18n::sprintf;

use super::common::{checked, dropdown_pages, dropdown_terms, form_option, forbidden, selected, settings_errors, submit};
use super::{AdminResponse, Ctx, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};

fn help(ctx: &Ctx<'_>, overview: &str, docs: &str) -> Help {
    Help {
        tabs: vec![HelpTab { id: "overview".into(), title: ctx.tr("Overview"), content: overview.to_owned() }],
        sidebar: format!("<p><strong>{}</strong></p><p>{docs}</p><p>{}</p>", ctx.tr("For more information:"), ctx.tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    }
}

/// `add_help_tab()` after the overview.
fn with_tab(mut help: Help, id: &str, title: String, content: String) -> Help {
    help.tabs.push(HelpTab { id: id.into(), title, content });
    help
}

async fn finish(ctx: &Ctx<'_>, file: &str, title: String, body: String, help: Help, assets: Assets) -> AdminResponse {
    let screen = Screen { file: file.into(), parent_file: "options-general.php".into(), submenu_file: Some(file.into()), title, help: Some(help), ..Default::default() };
    render(ctx, &screen, &body, &assets).await
}

fn option_is(ctx: &Ctx<'_>, name: &str, value: &str) -> bool {
    ctx.site().options.get(name) == Some(value)
}

/// `options-writing.php`.
pub async fn writing(ctx: &Ctx<'_>) -> AdminResponse {
    if !ctx.can("manage_options") {
        return forbidden(ctx, "Sorry, you are not allowed to manage options for this site.");
    }
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let title = tr("Writing Settings");
    let mut h = format!("<div class=\"wrap\">\n<h1>{}</h1>\n\n", esc_html(&title));
    h.push_str(&settings_errors(ctx).await);
    h.push_str(&format!("<form method=\"post\" action=\"options.php\">\n{}\n<table class=\"form-table\" role=\"presentation\">\n", ctx.settings_fields("writing")));
    let default_category = site.options.u64("default_category", 1);
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"default_category\">{}</label></th>\n<td>\n{}</td>\n</tr>\n",
        tr("Default Post Category"),
        dropdown_terms(ctx, "category", "default_category", "default_category", default_category, "postform", None, &[]).await
    ));
    let current_format = site.options.string("default_post_format", "0");
    let formats = [("0", "Standard"), ("aside", "Aside"), ("chat", "Chat"), ("gallery", "Gallery"), ("link", "Link"), ("image", "Image"), ("quote", "Quote"), ("status", "Status"), ("video", "Video"), ("audio", "Audio")];
    let mut options = String::new();
    for (value, label) in formats {
        let label = site.trx(label, "Post format");
        options.push_str(&format!("\t\t<option value=\"{value}\"{}>{label}</option>\n", if current_format == value && value != "0" { " selected=\"selected\"" } else { "" }));
    }
    h.push_str(&format!("<tr>\n<th scope=\"row\"><label for=\"default_post_format\">{}</label></th>\n<td>\n\t<select name=\"default_post_format\" id=\"default_post_format\">\n{options}\t</select>\n</td>\n</tr>\n\n</table>\n\n", tr("Default Post Format")));

    let strings: Vec<String> = ["a", "b", "c"].iter().map(|_| random_string(8)).collect();
    let email_help = sprintf(
        &tr("To post to WordPress by email, you must set up a secret email account with POP3 access. Any mail received at this address will be posted, so it&#8217;s a good idea to keep this address very secret. Here are three random strings you could use: %1$s, %2$s, %3$s."),
        &strings.iter().map(|s| format!("<kbd>{s}</kbd>")).collect::<Vec<_>>().iter().map(String::as_str).collect::<Vec<_>>(),
    );
    h.push_str(&format!("<h2 id=\"wp-settings-section-post-via-email\" class=\"title\">{}</h2>\n<p>\n\t{email_help}</p>\n\n<table class=\"form-table\" role=\"presentation\">\n", tr("Post via email")));
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"mailserver_url\">{}</label></th>\n<td><input name=\"mailserver_url\" type=\"text\" id=\"mailserver_url\" value=\"{}\" class=\"regular-text code\" />\n<label for=\"mailserver_port\">{}</label>\n<input name=\"mailserver_port\" type=\"text\" id=\"mailserver_port\" value=\"{}\" class=\"small-text\" />\n</td>\n</tr>\n",
        tr("Mail Server"),
        form_option(ctx, "mailserver_url"),
        tr("Port"),
        form_option(ctx, "mailserver_port")
    ));
    h.push_str(&format!("<tr>\n<th scope=\"row\"><label for=\"mailserver_login\">{}</label></th>\n<td><input name=\"mailserver_login\" type=\"text\" id=\"mailserver_login\" value=\"{}\" class=\"regular-text ltr\" /></td>\n</tr>\n", tr("Login Name"), form_option(ctx, "mailserver_login")));
    h.push_str(&format!(
        "<tr class=\"mailserver-pass-wrap\">\n\t<th scope=\"row\">\n\t\t<label for=\"mailserver_pass\">\n\t\t\t{}\t\t</label>\n\t</th>\n\t<td>\n\t\t<input type=\"hidden\" value=\" \" /><!-- #24364 workaround -->\n\t\t<span class=\"wp-pwd\">\n\t\t\t<input type=\"text\" name=\"mailserver_pass\" id=\"mailserver_pass\" class=\"regular-text ltr\" autocomplete=\"off\" data-reveal=\"1\" data-pw=\"{}\" />\n\t\t\t<button type=\"button\" class=\"button wp-hide-pw hide-if-no-js\" data-toggle=\"0\" data-start-masked=\"1\" aria-label=\"{}\">\n\t\t\t\t<span class=\"dashicons dashicons-visibility\" aria-hidden=\"true\"></span>\n\t\t\t</button>\n\t\t</span>\n\t</td>\n</tr>\n",
        tr("Password"),
        form_option(ctx, "mailserver_pass"),
        tr("Hide password")
    ));
    let mail_category = site.options.u64("default_email_category", 1);
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"default_email_category\">{}</label></th>\n<td>\n\t{}</td>\n</tr>\n\t</table>\n\n",
        tr("Default Mail Category"),
        dropdown_terms(ctx, "category", "default_email_category", "default_email_category", mail_category, "postform", None, &[]).await
    ));

    h.push_str(&format!(
        "<h2 id=\"wp-settings-section-update-services\" class=\"title\">{}</h2>\n\n\t\n\t<p><label for=\"ping_sites\">\n\t\t{}\t</label></p>\n\n\t<textarea name=\"ping_sites\" id=\"ping_sites\" class=\"large-text code\" rows=\"3\">{}</textarea>\n\n\t\n\n",
        tr("Update Services"),
        sprintf(&tr("When you publish a new post, WordPress automatically notifies the following site update services. For more about this, see the <a href=\"%s\">Update Services</a> documentation article. Separate multiple service URLs with line breaks."), &["https://developer.wordpress.org/advanced-administration/wordpress/update-services/"]),
        esc_html(&site.options.string("ping_sites", ""))
    ));
    h.push_str(&submit(ctx));
    h.push_str("</form>\n</div>\n");
    let overview = format!("<p>{}</p><p>{}</p>", tr("You can submit content in several different ways; this screen holds the settings for all of them. The top section controls the editor within the dashboard, while the rest control external publishing methods. For more information on any of these methods, use the documentation links."), tr("You must click the Save Changes button at the bottom of the screen for new settings to take effect."));
    finish(ctx, "options-writing.php", title, h, with_tab(
            with_tab(
                help(ctx, &overview, &tr("<a href=\"https://wordpress.org/documentation/article/settings-writing-screen/\">Documentation on Writing Settings</a>")),
                "options-postemail",
                tr("Post Via Email"),
                format!("<p>{}</p>", tr("Post via email settings allow you to send your WordPress installation an email with the content of your post. You must set up a secret email account with POP3 access to use this, and any mail received at this address will be posted, so it&#8217;s a good idea to keep this address very secret.")),
            ),
            "options-services",
            tr("Update Services"),
            format!("<p>{}</p>", tr("If desired, WordPress will automatically alert various services of your new posts.")),
        ), Assets { scripts: vec!["user-profile".into()], ..Default::default() }).await
}

fn random_string(len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64) | 1;
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..len)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            alphabet[(seed % alphabet.len() as u64) as usize] as char
        })
        .collect()
}

/// `options-reading.php`.
pub async fn reading(ctx: &Ctx<'_>) -> AdminResponse {
    if !ctx.can("manage_options") {
        return forbidden(ctx, "Sorry, you are not allowed to manage options for this site.");
    }
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let title = tr("Reading Settings");
    let mut h = format!("<div class=\"wrap\">\n<h1>{}</h1>\n\n", esc_html(&title));
    h.push_str(&settings_errors(ctx).await);
    h.push_str(&format!("<form method=\"post\" action=\"options.php\">\n{}\n<table class=\"form-table\" role=\"presentation\">\n", ctx.settings_fields("reading")));
    let on_front = site.options.string("show_on_front", "posts");
    let none = format!("&mdash; {} &mdash;", tr("Select"));
    let front = dropdown_pages(ctx, "page_on_front", "page_on_front", site.options.u64("page_on_front", 0), Some(&none)).await;
    let posts = dropdown_pages(ctx, "page_for_posts", "page_for_posts", site.options.u64("page_for_posts", 0), Some(&none)).await;
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{homepage}</th>\n<td id=\"front-static-pages\"><fieldset>\n\t<legend class=\"screen-reader-text\"><span>{homepage}</span></legend>\n\t<p><label>\n\t\t<input name=\"show_on_front\" type=\"radio\" value=\"posts\" {c1} />\n\t\t{latest}\t</label>\n\t</p>\n\t<p><label>\n\t\t<input name=\"show_on_front\" type=\"radio\" value=\"page\" {c2} />\n\t\t{page}\t</label>\n\t</p>\n<ul>\n\t<li><label for=\"page_on_front\">\n\t{front_label} {front}</label></li>\n\t<li><label for=\"page_for_posts\">\n\t{posts_label} {posts}</label></li>\n</ul>\n",
        homepage = tr("Your homepage displays"),
        c1 = checked(on_front == "posts"),
        c2 = checked(on_front == "page"),
        latest = tr("Your latest posts"),
        page = sprintf(&tr("A <a href=\"%s\">static page</a> (select below)"), &["edit.php?post_type=page"]),
        front_label = sprintf(&tr("Homepage: %s"), &[""]).trim_end(),
        posts_label = sprintf(&tr("Posts page: %s"), &[""]).trim_end(),
        front = front,
        posts = posts
    ));
    let front_id = site.options.u64("page_on_front", 0);
    let posts_id = site.options.u64("page_for_posts", 0);
    if on_front == "page" && front_id != 0 && front_id == posts_id {
        h.push_str(&format!("<div id=\"front-page-warning\" class=\"notice notice-warning inline\"><p>{}</p></div>\n", tr("<strong>Warning:</strong> these pages should not be the same!")));
    }
    h.push_str("\t</fieldset></td>\n</tr>\n");
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"posts_per_page\">{}</label></th>\n<td>\n<input name=\"posts_per_page\" type=\"number\" step=\"1\" min=\"1\" id=\"posts_per_page\" value=\"{}\" class=\"small-text\" />\n</td>\n</tr>\n",
        tr("Number of recent posts shown on archive pages"),
        form_option(ctx, "posts_per_page")
    ));
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"posts_per_rss\">{}</label></th>\n<td><input name=\"posts_per_rss\" type=\"number\" step=\"1\" min=\"1\" id=\"posts_per_rss\" value=\"{}\" class=\"small-text\" /></td>\n</tr>\n\n",
        tr("Number of recent items shown in syndication feeds"),
        form_option(ctx, "posts_per_rss")
    ));
    let excerpt = site.options.u64("rss_use_excerpt", 0);
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{} </th>\n<td><fieldset>\n\t<legend class=\"screen-reader-text\"><span>{}</span></legend>\n\t<p>\n\t\t<label><input name=\"rss_use_excerpt\" type=\"radio\" value=\"0\" {}\t/> {}</label><br />\n\t\t<label><input name=\"rss_use_excerpt\" type=\"radio\" value=\"1\" {} /> {}</label>\n\t</p>\n\t<p class=\"description\">\n\t\t{}\t</p>\n</fieldset></td>\n</tr>\n\n",
        tr("For each post in a feed, include"),
        tr("For each post in a feed, include"),
        checked(excerpt == 0),
        tr("Full text"),
        checked(excerpt != 0),
        tr("Excerpt"),
        sprintf(&tr("Your theme determines how content is displayed in browsers. <a href=\"%s\">Learn more about feeds</a>."), &["https://developer.wordpress.org/advanced-administration/wordpress/feeds/"])
    ));
    let discouraged = option_is(ctx, "blog_public", "0");
    h.push_str(&format!(
        "<tr class=\"option-site-visibility\">\n<th scope=\"row\">{title} </th>\n<td><fieldset>\n\t<legend class=\"screen-reader-text\"><span>{title}</span></legend>\n\t<label for=\"blog_public\"><input name=\"blog_public\" type=\"checkbox\" id=\"blog_public\" value=\"0\" {c} />\n\t{label}</label>\n\t<p class=\"description\">{desc}</p>\n</fieldset></td>\n</tr>\n\n</table>\n\n\n",
        title = tr("Search engine visibility"),
        c = checked(discouraged),
        label = tr("Discourage search engines from indexing this site"),
        desc = tr("It is up to search engines to honor this request.")
    ));
    h.push_str(&submit(ctx));
    h.push_str("</form>\n</div>\n");
    let overview = format!(
        "<p>{}</p><p>{}</p><p>{}</p><p>{}</p>",
        tr("This screen contains the settings that affect the display of your content."),
        sprintf(&tr("You can choose what&#8217;s displayed on the homepage of your site. It can be posts in reverse chronological order (classic blog), or a fixed/static page. To set a static homepage, you first need to create two <a href=\"%s\">Pages</a>. One will become the homepage, and the other will be where your posts are displayed."), &["post-new.php?post_type=page"]),
        sprintf(&tr("You can also control the display of your content in RSS feeds, including the maximum number of posts to display and whether to show full text or an excerpt. <a href=\"%s\">Learn more about feeds</a>."), &[&tr("https://developer.wordpress.org/advanced-administration/wordpress/feeds/")]),
        tr("You must click the Save Changes button at the bottom of the screen for new settings to take effect.")
    );
    let assets = Assets {
        head: "<script>\n\tjQuery( function($) {\n\t\tvar section = $('#front-static-pages'),\n\t\t\tstaticPage = section.find('input:radio[value=\"page\"]'),\n\t\t\tselects = section.find('select'),\n\t\t\tcheck_disabled = function(){\n\t\t\t\tselects.prop( 'disabled', ! staticPage.prop('checked') );\n\t\t\t};\n\t\tcheck_disabled();\n\t\tsection.find( 'input:radio' ).on( 'change', check_disabled );\n\t} );\n</script>\n".into(),
        footer: "<script>\n\tjQuery( function($) {\n\t\tvar $showOnFront = $( 'input[name=\"show_on_front\"]' ),\n\t\t\t$pageSelects = $( '#page_on_front, #page_for_posts' );\n\t\t$showOnFront.on( 'change', function() {\n\t\t\t$pageSelects.prop( 'disabled', 'page' !== $showOnFront.filter( ':checked' ).val() );\n\t\t}).trigger( 'change' );\n\t});\n</script>\n".into(),
        ..Default::default()
    };
    finish(ctx, "options-reading.php", title, h, with_tab(
            help(ctx, &overview, &tr("<a href=\"https://wordpress.org/documentation/article/settings-reading-screen/\">Documentation on Reading Settings</a>")),
            "site-visibility",
            tr("Search engine visibility"),
            format!(
                "<p>{}</p><p>{}</p><p>{}</p>",
                tr("You can choose whether or not your site will be crawled by robots, ping services, and spiders. If you want those services to ignore your site, click the checkbox next to &#8220;Discourage search engines from indexing this site&#8221; and click the Save Changes button at the bottom of the screen."),
                tr("Note that even when set to discourage search engines, your site is still visible on the web and not all search engines adhere to this directive."),
                tr("When this setting is in effect, a reminder is shown in the At a Glance box of the Dashboard that says, &#8220;Search engines discouraged&#8221;, to remind you that you have directed search engines to not crawl your site.")
            )
        ), assets).await
}

fn avatar_choice(ctx: &Ctx<'_>, key: &str, label: &str, current: &str) -> String {
    use sha2::{Digest, Sha256};
    let email = ctx.user.email.trim().to_lowercase();
    let hash: String = Sha256::digest(email.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let default = match key {
        "mystery" => "&d=mm".to_owned(),
        "gravatar_default" => String::new(),
        other => format!("&d={other}"),
    };
    let rating = ctx.site().options.string("avatar_rating", "G").to_lowercase();
    let initials = if key == "initials" {
        let name = ctx.user.display_name.to_lowercase();
        let words: Vec<&str> = name.split_whitespace().collect();
        let value = if words.len() >= 2 { format!("{}{}", words[0].chars().next().unwrap_or(' '), words[words.len() - 1].chars().next().unwrap_or(' ')) } else { name.chars().take(2).collect() };
        format!("&initials={value}")
    } else {
        String::new()
    };
    let url = |size: u32| format!("https://secure.gravatar.com/avatar/{hash}?s={size}{default}&f=y&r={rating}{initials}").replace('&', "&#038;");
    format!(
        "\t<label><input type='radio' name='avatar_default' id='avatar_{key}' value='{key}'{} /> <img alt='' src='{}' srcset='{} 2x' class='avatar avatar-32 photo avatar-default' height='32' width='32' loading='lazy' decoding='async'/> {label}</label><br />\n",
        if current == key { " checked=\"checked\"" } else { "" },
        url(32),
        url(64)
    )
}

/// `options-discussion.php`.
pub async fn discussion(ctx: &Ctx<'_>) -> AdminResponse {
    if !ctx.can("manage_options") {
        return forbidden(ctx, "Sorry, you are not allowed to manage options for this site.");
    }
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let o = |name: &str| site.options.string(name, "");
    let on = |name: &str| o(name) == "1";
    let title = tr("Discussion Settings");
    let mut h = format!("<div class=\"wrap\">\n<h1>{}</h1>\n\n", esc_html(&title));
    h.push_str(&settings_errors(ctx).await);
    h.push_str(&format!("<form method=\"post\" action=\"options.php\">\n{}\n<table class=\"form-table indent-children\" role=\"presentation\">\n", ctx.settings_fields("discussion")));
    let legend = |t: &str| format!("<fieldset><legend class=\"screen-reader-text\"><span>{t}</span></legend>\n");
    // Default post settings.
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{t}</th>\n<td>{legend}<label for=\"default_pingback_flag\">\n<input name=\"default_pingback_flag\" type=\"checkbox\" id=\"default_pingback_flag\" value=\"1\" {c1} />\n{l1}</label>\n<br />\n<label for=\"default_ping_status\">\n<input name=\"default_ping_status\" type=\"checkbox\" id=\"default_ping_status\" value=\"open\" {c2} />\n{l2}</label>\n<br />\n<label for=\"default_comment_status\">\n<input name=\"default_comment_status\" type=\"checkbox\" id=\"default_comment_status\" value=\"open\" {c3} />\n{l3}</label>\n<br />\n<p class=\"description\">{d}</p>\n</fieldset></td>\n</tr>\n",
        t = tr("Default post settings"),
        legend = legend(&tr("Default post settings")),
        c1 = checked(on("default_pingback_flag")),
        c2 = checked(o("default_ping_status") == "open"),
        c3 = checked(o("default_comment_status") == "open"),
        l1 = tr("Attempt to notify any blogs linked to from the post"),
        l2 = tr("Allow link notifications from other blogs (pingbacks and trackbacks) on new posts"),
        l3 = tr("Allow people to submit comments on new posts"),
        d = tr("Individual posts may override these settings. Changes here will only be applied to new posts.")
    ));
    // Other comment settings.
    let depth = site.options.u64("thread_comments_depth", 5);
    let depth_options: String = (2..=10).map(|d| format!("<option value='{d}'{}>{d}</option>", selected(depth == d))).collect();
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{t}</th>\n<td>{legend}<label for=\"require_name_email\"><input type=\"checkbox\" name=\"require_name_email\" id=\"require_name_email\" value=\"1\" {c1} /> {l1}</label>\n<br />\n<label for=\"comment_registration\">\n<input name=\"comment_registration\" type=\"checkbox\" id=\"comment_registration\" value=\"1\" {c2} />\n{l2}</label>\n<br />\n<input name=\"close_comments_for_old_posts\" type=\"checkbox\" id=\"close_comments_for_old_posts\" value=\"1\" {c3} /> <label for=\"close_comments_for_old_posts\">{l3}</label>\n<ul>\n\t<li>\n\t\t<label for=\"close_comments_days_old\">{l4}</label>\n\t\t<input name=\"close_comments_days_old\" type=\"number\" step=\"1\" min=\"0\" id=\"close_comments_days_old\" value=\"{days}\" class=\"small-text\" />\n\t</li>\n</ul>\n\n<input name=\"show_comments_cookies_opt_in\" type=\"checkbox\" id=\"show_comments_cookies_opt_in\" value=\"1\" {c5} />\n<label for=\"show_comments_cookies_opt_in\">{l5}</label>\n<br />\n<input name=\"thread_comments\" type=\"checkbox\" id=\"thread_comments\" value=\"1\" {c6} />\n<label for=\"thread_comments\">{l6}</label>\n\n<ul>\n\t<li>\n\t\t<label for=\"thread_comments_depth\">{l7}</label>\n\t\t<select name=\"thread_comments_depth\" id=\"thread_comments_depth\">{depth_options}</select>\t</li>\n</ul>\n</fieldset></td>\n</tr>\n\n",
        t = tr("Other comment settings"),
        legend = legend(&tr("Other comment settings")),
        c1 = checked(on("require_name_email")),
        c2 = checked(on("comment_registration")),
        c3 = checked(on("close_comments_for_old_posts")),
        c5 = checked(on("show_comments_cookies_opt_in")),
        c6 = checked(on("thread_comments")),
        l1 = tr("Comment author must fill out name and email"),
        l2 = tr("Users must be registered and logged in to comment"),
        l3 = tr("Automatically close comments on old posts"),
        l4 = tr("Close comments when post is how many days old"),
        l5 = tr("Show comments cookies opt-in checkbox, allowing comment author cookies to be set"),
        l6 = tr("Enable threaded (nested) comments"),
        l7 = tr("Number of levels for threaded (nested) comments"),
        days = form_option(ctx, "close_comments_days_old")
    ));
    // Pagination.
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{t}</th>\n<td>{legend}<input name=\"page_comments\" type=\"checkbox\" id=\"page_comments\" value=\"1\" {c1} />\n<label for=\"page_comments\">{l1}</label>\n<ul>\n\t<li>\n\t\t<label for=\"comments_per_page\">{l2}</label>\n\t\t<input name=\"comments_per_page\" type=\"number\" step=\"1\" min=\"0\" id=\"comments_per_page\" value=\"{per}\" class=\"small-text\" />\n\t</li>\n\t<li>\n\t\t<label for=\"default_comments_page\">{l3}</label>\n\t\t<select name=\"default_comments_page\" id=\"default_comments_page\">\n\t\t\t<option value=\"newest\" {s1}>{newest}</option>\n\t\t\t<option value=\"oldest\" {s2}>{oldest}</option>\n\t\t</select>\n\t</li>\n\t<li>\n\t\t<label for=\"comment_order\">{l4}</label>\n\t\t<select name=\"comment_order\" id=\"comment_order\">\n\t\t\t<option value=\"asc\" {s3}>{older}</option>\n\t\t\t<option value=\"desc\" {s4}>{newer}</option>\n\t\t</select>\n\t</li>\n</ul>\n</fieldset></td>\n</tr>\n",
        t = tr("Comment Pagination"),
        legend = legend(&tr("Comment Pagination")),
        c1 = checked(on("page_comments")),
        l1 = tr("Break comments into pages"),
        l2 = tr("Top level comments per page"),
        l3 = tr("Comments page to display by default"),
        l4 = tr("Comments to display at the top of each page"),
        per = form_option(ctx, "comments_per_page"),
        s1 = selected(o("default_comments_page") == "newest"),
        s2 = selected(o("default_comments_page") == "oldest"),
        s3 = selected(o("comment_order") == "asc"),
        s4 = selected(o("comment_order") == "desc"),
        newest = tr("last page"),
        oldest = tr("first page"),
        older = tr("older"),
        newer = tr("newer")
    ));
    // Notifications and moderation.
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{t}</th>\n<td>{legend}<label for=\"comments_notify\">\n<input name=\"comments_notify\" type=\"checkbox\" id=\"comments_notify\" value=\"1\" {c1} />\n{l1} </label>\n<br />\n<label for=\"moderation_notify\">\n<input name=\"moderation_notify\" type=\"checkbox\" id=\"moderation_notify\" value=\"1\" {c2} />\n{l2} </label>\n<br />\n<label for=\"wp_notes_notify\">\n<input name=\"wp_notes_notify\" type=\"checkbox\" id=\"wp_notes_notify\" value=\"1\" {c3} />\n{l3} </label>\n\n</fieldset></td>\n</tr>\n",
        t = tr("Email me whenever"),
        legend = legend(&tr("Email me whenever")),
        c1 = checked(on("comments_notify")),
        c2 = checked(on("moderation_notify")),
        c3 = checked(site.options.get("wp_notes_notify").is_none_or(|v| v == "1")),
        l1 = tr("Anyone posts a comment"),
        l2 = tr("A comment is held for moderation"),
        l3 = tr("Anyone posts a note")
    ));
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{t}</th>\n<td>{legend}<label for=\"comment_moderation\">\n<input name=\"comment_moderation\" type=\"checkbox\" id=\"comment_moderation\" value=\"1\" {c1} />\n{l1} </label>\n<br />\n<label for=\"comment_previously_approved\"><input type=\"checkbox\" name=\"comment_previously_approved\" id=\"comment_previously_approved\" value=\"1\" {c2} /> {l2}</label>\n</fieldset></td>\n</tr>\n",
        t = tr("Before a comment appears"),
        legend = legend(&tr("Before a comment appears")),
        c1 = checked(on("comment_moderation")),
        c2 = checked(on("comment_previously_approved")),
        l1 = tr("Comment must be manually approved"),
        l2 = tr("Comment author must have a previously approved comment")
    ));
    let keys_text = tr("When a comment contains any of these words in its content, author name, URL, email, IP address, or browser&#8217;s user agent string, it will be %s. One word or IP address per line. It will match inside words, so &#8220;press&#8221; will match &#8220;WordPress&#8221;.");
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{t}</th>\n<td>{legend}<p><label for=\"comment_max_links\">{l1}</label>\n<input name=\"comment_max_links\" type=\"number\" step=\"1\" min=\"0\" id=\"comment_max_links\" value=\"{links}\" class=\"small-text\" aria-describedby=\"comment_links_moderation_desc\" />\n<span id=\"comment_links_moderation_desc\">{l2}</span>\n</p>\n\n<p><label for=\"moderation_keys\">{k1}</label></p>\n<p>\n<textarea name=\"moderation_keys\" rows=\"10\" cols=\"50\" id=\"moderation_keys\" class=\"large-text code\">{mod_keys}</textarea>\n</p>\n</fieldset></td>\n</tr>\n",
        t = tr("Comment Moderation"),
        legend = legend(&tr("Comment Moderation")),
        l1 = tr("Number of links to allow in a comment before holding for moderation:"),
        l2 = tr("(A common characteristic of comment spam is a large number of hyperlinks.)"),
        links = form_option(ctx, "comment_max_links"),
        k1 = sprintf(&keys_text, &[&sprintf(&tr("held in the <a href=\"%s\">moderation queue</a>"), &["edit-comments.php?comment_status=moderated"]).to_string()]),
        mod_keys = esc_html(&o("moderation_keys"))
    ));
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{t}</th>\n<td>{legend}<p><label for=\"disallowed_keys\">{k}</label></p>\n<p>\n<textarea name=\"disallowed_keys\" rows=\"10\" cols=\"50\" id=\"disallowed_keys\" class=\"large-text code\">{keys}</textarea>\n</p>\n</fieldset></td>\n</tr>\n</table>\n\n",
        t = tr("Disallowed Comment Keys"),
        legend = legend(&tr("Disallowed Comment Keys")),
        k = sprintf(&keys_text, &[&tr("put in the Trash")]),
        keys = esc_html(&o("disallowed_keys"))
    ));
    // Avatars.
    h.push_str(&format!("<h2 id=\"wp-settings-section-avatars\" class=\"title\">{}</h2>\n\n<p>{}</p>\n\n\n<table class=\"form-table\" role=\"presentation\">\n", tr("Avatars"), tr("An avatar is an image that can be associated with a user across multiple websites. In this area, you can choose to display avatars of users who interact with the site.")));
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{}</th>\n<td>\n\t<label for=\"show_avatars\">\n\t\t<input type=\"checkbox\" id=\"show_avatars\" name=\"show_avatars\" value=\"1\" {} />\n\t\t{}\t</label>\n</td>\n</tr>\n",
        tr("Avatar Display"),
        checked(on("show_avatars")),
        tr("Show Avatars")
    ));
    let rating = site.options.string("avatar_rating", "G");
    let ratings = [("G", tr("Suitable for all audiences")), ("PG", tr("Possibly offensive, usually for audiences 13 and above")), ("R", tr("Intended for adult audiences above 17")), ("X", tr("Even more mature than above"))];
    let mut rating_html = String::new();
    for (key, label) in &ratings {
        rating_html.push_str(&format!("\n\t<label><input type='radio' name='avatar_rating' value='{key}'{}/> {key} &#8212; {label}</label><br />", if rating == *key { " checked=\"checked\"" } else { " " }));
    }
    h.push_str(&format!("<tr class=\"avatar-settings\">\n<th scope=\"row\">{t}</th>\n<td>{}{rating_html}\n</fieldset></td>\n</tr>\n", legend(&tr("Maximum Rating")), t = tr("Maximum Rating")));
    let current = match o("avatar_default").as_str() {
        "" => "mystery".to_owned(),
        other => other.to_owned(),
    };
    let choices = [
        ("mystery", tr("Mystery Person")),
        ("blank", tr("Blank")),
        ("gravatar_default", tr("Gravatar Logo")),
        ("identicon", tr("Identicon (Generated)")),
        ("wavatar", tr("Wavatar (Generated)")),
        ("monsterid", tr("MonsterID (Generated)")),
        ("retro", tr("Retro (Generated)")),
        ("robohash", tr("RoboHash (Generated)")),
        ("initials", tr("Initials (Generated)")),
        ("color", tr("Color (Generated)")),
    ];
    let mut list = String::new();
    for (key, label) in &choices {
        list.push_str(&avatar_choice(ctx, key, label, &current));
    }
    h.push_str(&format!(
        "<tr class=\"avatar-settings\">\n<th scope=\"row\">{t}</th>\n<td class=\"defaultavatarpicker\">{}\n<p>\n{}<br />\n</p>\n\n\n{list}</fieldset></td>\n</tr>\n</table>\n\n\n",
        legend(&tr("Default Avatar")),
        tr("For users without a custom avatar of their own, you can either display a generic logo or a generated one based on their email address."),
        t = tr("Default Avatar")
    ));
    h.push_str(&submit(ctx));
    h.push_str("</form>\n</div>\n");
    let overview = format!("<p>{}</p><p>{}</p>", tr("This screen provides many options for controlling the management and display of comments and links to your posts/pages. So many, in fact, they will not all fit here! :) Use the documentation links to get information on what each discussion setting does."), tr("You must click the Save Changes button at the bottom of the screen for new settings to take effect."));
    finish(ctx, "options-discussion.php", title, h, help(ctx, &overview, &tr("<a href=\"https://wordpress.org/documentation/article/settings-discussion-screen/\">Documentation on Discussion Settings</a>")), Assets::default()).await
}

/// `options-media.php`.
pub async fn media(ctx: &Ctx<'_>) -> AdminResponse {
    if !ctx.can("manage_options") {
        return forbidden(ctx, "Sorry, you are not allowed to manage options for this site.");
    }
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let title = tr("Media Settings");
    let mut h = format!("<div class=\"wrap\">\n<h1>{}</h1>\n\n", esc_html(&title));
    h.push_str(&settings_errors(ctx).await);
    h.push_str(&format!("<form action=\"options.php\" method=\"post\">\n{}\n", ctx.settings_fields("media")));
    h.push_str(&format!("<h2 id=\"wp-settings-section-image-sizes\" class=\"title\">{}</h2>\n<p>{}</p>\n\n<table class=\"form-table\" role=\"presentation\">\n", tr("Image sizes"), tr("The sizes listed below determine the maximum dimensions in pixels to use when adding an image to the Media Library.")));
    let size = |name: &str, label_w: &str, label_h: &str, row_title: &str, extra: &str| {
        format!(
            "<tr>\n<th scope=\"row\">{row_title}</th>\n<td><fieldset><legend class=\"screen-reader-text\"><span>{row_title}</span></legend>\n<label for=\"{name}_size_w\">{label_w}</label>\n<input name=\"{name}_size_w\" type=\"number\" step=\"1\" min=\"0\" id=\"{name}_size_w\" value=\"{}\" class=\"small-text\" />\n<br />\n<label for=\"{name}_size_h\">{label_h}</label>\n<input name=\"{name}_size_h\" type=\"number\" step=\"1\" min=\"0\" id=\"{name}_size_h\" value=\"{}\" class=\"small-text\" />\n</fieldset>{extra}</td>\n</tr>\n\n",
            form_option(ctx, &format!("{name}_size_w")),
            form_option(ctx, &format!("{name}_size_h"))
        )
    };
    let crop = format!(
        "\n<input name=\"thumbnail_crop\" type=\"checkbox\" id=\"thumbnail_crop\" value=\"1\"{} />\n<label for=\"thumbnail_crop\">{}</label>\n",
        checked(site.options.get("thumbnail_crop").is_some_and(|v| !v.is_empty() && v != "0")),
        tr("Crop thumbnail to exact dimensions (normally thumbnails are proportional)")
    );
    h.push_str(&size("thumbnail", &tr("Width"), &tr("Height"), &tr("Thumbnail size"), &crop));
    h.push_str(&size("medium", &tr("Max Width"), &tr("Max Height"), &tr("Medium size"), ""));
    h.push_str(&size("large", &tr("Max Width"), &tr("Max Height"), &tr("Large size"), ""));
    h.push_str("</table>\n\n\n");
    h.push_str(&format!(
        "<h2 id=\"wp-settings-section-uploading-files\" class=\"title\">{}</h2>\n<table class=\"form-table\" role=\"presentation\">\n\t<tr>\n<td class=\"td-full\">\n<label for=\"uploads_use_yearmonth_folders\">\n<input name=\"uploads_use_yearmonth_folders\" type=\"checkbox\" id=\"uploads_use_yearmonth_folders\" value=\"1\"{} />\n\t{}</label>\n</td>\n</tr>\n\n\t</table>\n\n\n",
        tr("Uploading Files"),
        checked(site.options.get("uploads_use_yearmonth_folders").is_some_and(|v| v == "1")),
        tr("Organize my uploads into month- and year-based folders")
    ));
    h.push_str(&submit(ctx));
    h.push_str("\n</form>\n\n</div>\n");
    // The paths of the uploads only show where they were changed (`upload_url_path`, `upload_path`).
    let options = &ctx.site().options;
    let upload_path = options.string("upload_path", "");
    let custom_uploads = !options.string("upload_url_path", "").is_empty() || (!upload_path.is_empty() && upload_path != "wp-content/uploads");
    let mut overview = format!("<p>{}</p>", tr("You can set maximum sizes for images inserted into your written content; you can also insert an image as Full Size."));
    if custom_uploads {
        overview.push_str(&format!("<p>{}</p>", tr("Uploading Files allows you to choose the folder and path for storing your uploaded files.")));
    }
    overview.push_str(&format!("<p>{}</p>", tr("You must click the Save Changes button at the bottom of the screen for new settings to take effect.")));
    let _ = esc_attr("");
    finish(ctx, "options-media.php", title, h, help(ctx, &overview, &tr("<a href=\"https://wordpress.org/documentation/article/settings-media-screen/\">Documentation on Media Settings</a>")), Assets::default()).await
}
