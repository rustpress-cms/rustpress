//! The toolbar (`WP_Admin_Bar`): the black bar at the top of the administration area and of the
//! frontend for signed-in users.

use rustpress_auth::CurrentUser;
use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

/// What the toolbar shows besides the user.
#[derive(Debug, Clone, Default)]
pub struct BarContext {
    /// Updates available (`wp_get_update_data()['counts']['total']`).
    pub updates: u64,
    /// Comments in moderation.
    pub pending_comments: u64,
    /// `wp_create_nonce( 'log-out' )`.
    pub logout_nonce: String,
    /// The "View Post" entry of the screen that edits a post: `(label, address)`.
    pub view_item: Option<(String, String)>,
    /// The entry is the archive of a post type (`View Posts`), not the view of one item.
    pub view_archive: bool,
    /// The screen loads the command palette (`wp-core-commands` is enqueued).
    pub command_palette: bool,
}

fn item(id: &str, class: &str, inner: &str) -> String {
    let class = if class.is_empty() { String::new() } else { format!(" class=\"{class}\"") };
    format!("<li role='group' id='wp-admin-bar-{id}'{class}>{inner}</li>")
}

fn link(href: &str, inner: &str, popup: bool) -> String {
    let expanded = if popup { " aria-expanded=\"false\"" } else { "" };
    format!("<a class='ab-item' role=\"menuitem\"{expanded} href='{href}'>{inner}</a>")
}

fn submenu(label: &str, id: &str, class: &str, items: &[String]) -> String {
    format!("<ul role='menu' aria-label='{}' id='wp-admin-bar-{id}' class=\"{class}\">{}</ul>", esc_attr(label), items.concat())
}

/// `get_avatar()` the way the toolbar prints it.
fn avatar(site: &Site, email: &str, size: u64) -> String {
    let url = |s: u64| esc_url(&rustpress_blocks::content::avatar_url(&site.options, email, s));
    format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-{size} photo' height='{size}' width='{size}' loading='lazy' decoding='async'/>", url(size), url(size * 2))
}

/// The toolbar for the administration area.
pub fn render_admin(site: &Site, user: &CurrentUser, ctx: &BarContext) -> String {
    let tr = |s: &str| site.tr(s);
    let admin = |path: &str| esc_url(&site.site_url(&format!("wp-admin/{path}")));
    let number = |n: u64| site.number_i18n(n);
    let blog_name = site.options.string("blogname", "");

    let mut top: Vec<String> = Vec::new();
    top.push(item("menu-toggle", "", &link("#", &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"screen-reader-text\">{}</span>", tr("Menu")), false)));

    // WordPress logo
    let about = tr("About WordPress");
    let logo_primary = vec![
        item("about", "", &link(&admin("about.php"), &about, false)),
        item("contribute", "", &link(&admin("contribute.php"), &tr("Get Involved"), false)),
    ];
    let logo_secondary = vec![
        item("wporg", "", &link("https://wordpress.org/", &tr("WordPress.org"), false)),
        item("documentation", "", &link(&tr("https://wordpress.org/documentation/"), &tr("Documentation"), false)),
        item("learn", "", &link("https://learn.wordpress.org/", &tr("Learn WordPress"), false)),
        item("support-forums", "", &link(&tr("https://wordpress.org/support/forums/"), &tr("Support"), false)),
        item("feedback", "", &link(&tr("https://wordpress.org/support/forum/requests-and-feedback"), &tr("Feedback"), false)),
    ];
    top.push(item(
        "wp-logo",
        "menupop",
        &format!(
            "{}<div class=\"ab-sub-wrapper\">{}{}</div>",
            link(&admin("about.php"), &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"screen-reader-text\">{about}</span>"), true),
            submenu(&about, "wp-logo-default", "ab-submenu", &logo_primary),
            submenu(&about, "wp-logo-external", "ab-sub-secondary ab-submenu", &logo_secondary)
        ),
    ));

    // Site name
    let site_title = if blog_name.is_empty() { tr("Dashboard") } else { blog_name.clone() };
    let visit = vec![item("view-site", "", &link(&esc_url(&site.home_url("/")), &tr("Visit Site"), false))];
    top.push(item(
        "site-name",
        "menupop",
        &format!("{}<div class=\"ab-sub-wrapper\">{}</div>", link(&esc_url(&site.home_url("/")), &esc_html(&site_title), true), submenu(&site_title, "site-name-default", "ab-submenu", &visit)),
    ));

    // Updates
    if ctx.updates > 0 && ["update_core", "update_plugins", "update_themes"].iter().any(|c| user.can(c)) {
        let text = rustpress_core::i18n::sprintf(&site.trn("%s update available", "%s updates available", ctx.updates as i64), &[&number(ctx.updates)]);
        top.push(item(
            "updates",
            "",
            &link(&admin("update-core.php"), &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"ab-label\" aria-hidden=\"true\">{}</span><span class=\"screen-reader-text updates-available-text\">{text}</span>", number(ctx.updates)), false),
        ));
    }

    // Command palette
    if ctx.command_palette {
        let shortcut = site.trx("Ctrl+K", "keyboard shortcut to open the command palette");
        let flags = rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false };
        let apple = rustpress_blocks::php_json::encode_with(&serde_json::Value::from(site.trx("\u{2318}K", "keyboard shortcut to open the command palette")), flags).replace('<', "\\u003C").replace('>', "\\u003E");
        // A nowdoc whose closing marker is indented by one tab: every line loses one.
        let script = format!(
            "( \t( applePattern, appleOSLabel ) => {{\n\t\tif ( ! ( new RegExp( applePattern, 'i' ) ).test( navigator.userAgent ) ) {{\n\t\t\treturn;\n\t\t}}\n\t\tconst kbd = document.querySelector( '#wp-admin-bar-command-palette .ab-label kbd' );\n\t\tif ( kbd ) {{\n\t\t\tkbd.textContent = appleOSLabel;\n\t\t}}\n\t}} )( \"Macintosh|Mac OS X|Mac_PowerPC\", {apple} );\n//# sourceURL=wp_admin_bar_command_palette_menu"
        );
        top.push(format!(
            "<li role='group' id='wp-admin-bar-command-palette' class=\"hide-if-no-js\"><a class='ab-item' role=\"menuitem\" href='#' onclick='wp.data.dispatch( &quot;core/commands&quot; ).open(); return false;'><span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"ab-label\"><kbd>{shortcut}</kbd><span class=\"screen-reader-text\"> {}</span></span></a><script>\n{script}\n</script>\n</li>",
            tr("Open command palette")
        ));
    }

    // Comments
    if user.can("edit_posts") {
        let text = rustpress_core::i18n::sprintf(&site.trn("%s Comment in moderation", "%s Comments in moderation", ctx.pending_comments as i64), &[&number(ctx.pending_comments)]);
        top.push(item(
            "comments",
            "",
            &link(
                &admin("edit-comments.php"),
                &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"ab-label awaiting-mod pending-count count-{}\" aria-hidden=\"true\">{}</span><span class=\"screen-reader-text comments-in-moderation-text\">{text}</span>", ctx.pending_comments, number(ctx.pending_comments)),
                false,
            ),
        ));
    }

    // New
    let mut new_items: Vec<String> = Vec::new();
    let mut add_new = |id: &str, href: String, label: &str| new_items.push(item(&format!("new-{id}"), "", &link(&href, &esc_html(label), false)));
    if user.can("edit_posts") {
        add_new("post", admin("post-new.php"), &site.trx("Post", "post type singular name"));
    }
    if user.can("upload_files") {
        add_new("media", admin("media-new.php"), &site.trx("Media", "add new from admin bar"));
    }
    if user.can("edit_pages") {
        add_new("page", admin("post-new.php?post_type=page"), &site.trx("Page", "post type singular name"));
    }
    for custom in &site.types.post_types {
        if (custom.public || custom.queryable) && user.can("edit_posts") {
            let label = custom.singular_label();
            add_new(&custom.name, admin(&format!("post-new.php?post_type={}", custom.name)), &label);
        }
    }
    if user.can("create_users") || user.can("promote_users") {
        add_new("user", admin("user-new.php"), &site.trx("User", "add new from admin bar"));
    }
    if !new_items.is_empty() {
        let first = admin(if user.can("edit_posts") { "post-new.php" } else if user.can("upload_files") { "media-new.php" } else { "user-new.php" });
        top.push(item(
            "new-content",
            "menupop",
            &format!(
                "{}<div class=\"ab-sub-wrapper\">{}</div>",
                link(&first, &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"ab-label\">{}</span>", site.trx("New", "admin bar menu group label")), true),
                submenu(&site.trx("New", "admin bar menu group label"), "new-content-default", "ab-submenu", &new_items)
            ),
        ));
    }

    if let Some((label, url)) = &ctx.view_item {
        top.push(item(if ctx.view_archive { "archive" } else { "view" }, "", &link(&esc_url(url), &esc_html(label), false)));
    }

    // My account
    let howdy = rustpress_core::i18n::sprintf(&tr("Howdy, %s"), &[&format!("<span class=\"display-name\">{}</span>", esc_html(&user.display_name))]);
    let profile = admin("profile.php");
    let info = item(
        "user-info",
        "",
        &link(&profile, &format!("{}<span class='display-name'>{}</span><span class='display-name edit-profile'>{}</span>", avatar(site, &user.email, 64), esc_html(&user.display_name), tr("Edit Profile")), false),
    );
    let logout_url = esc_url(&format!("{}?action=logout&_wpnonce={}", site.site_url("wp-login.php"), ctx.logout_nonce));
    let logout = item("logout", "", &link(&logout_url, &tr("Log Out"), false));
    let account = item(
        "my-account",
        "menupop with-avatar",
        &format!("{}<div class=\"ab-sub-wrapper\">{}</div>", link(&profile, &format!("{howdy}{}", avatar(site, &user.email, 28)), true), submenu(&rustpress_core::i18n::sprintf(&tr("Howdy, %s"), &[&user.display_name]), "user-actions", "ab-submenu", &[info, logout])),
    );

    format!(
        "\n\t\t<div id=\"wpadminbar\" class=\"nojq nojs\">\n\t\t\t\t\t\t<div class=\"quicklinks\" id=\"wp-toolbar\" role=\"navigation\" aria-label=\"{}\">\n\t\t\t\t<ul role='menu' id='wp-admin-bar-root-default' class=\"ab-top-menu\">{}</ul><ul role='menu' id='wp-admin-bar-top-secondary' class=\"ab-top-secondary ab-top-menu\">{}</ul>\t\t\t</div>\n\t\t</div>\n\n\t\t",
        esc_attr(&tr("Toolbar")),
        top.concat(),
        account
    )
}

/// What the toolbar of the site shows besides the user.
#[derive(Debug, Clone, Default)]
pub struct FrontContext {
    pub updates: u64,
    pub pending_comments: u64,
    pub logout_nonce: String,
    /// "Edit Site": the address in the site editor, for a block theme.
    pub site_editor_url: Option<String>,
    /// "Edit Post", "Edit Page", "Edit Category": `(label, address)`.
    pub edit: Option<(String, String)>,
}

/// The toolbar of the site for a signed-in user (`wp_admin_bar_render()` outside of the administration area).
pub fn render_front(site: &Site, user: &CurrentUser, ctx: &FrontContext) -> String {
    let tr = |s: &str| site.tr(s);
    let admin = |path: &str| esc_url(&site.site_url(&format!("wp-admin/{path}")));
    let number = |n: u64| site.number_i18n(n);
    let blog_name = site.options.string("blogname", "");
    let mut top: Vec<String> = Vec::new();

    // WordPress logo
    let about = tr("About WordPress");
    let logo_primary = vec![
        item("about", "", &link(&admin("about.php"), &about, false)),
        item("contribute", "", &link(&admin("contribute.php"), &tr("Get Involved"), false)),
    ];
    let logo_secondary = vec![
        item("wporg", "", &link("https://wordpress.org/", &tr("WordPress.org"), false)),
        item("documentation", "", &link(&tr("https://wordpress.org/documentation/"), &tr("Documentation"), false)),
        item("learn", "", &link("https://learn.wordpress.org/", &tr("Learn WordPress"), false)),
        item("support-forums", "", &link(&tr("https://wordpress.org/support/forums/"), &tr("Support"), false)),
        item("feedback", "", &link(&tr("https://wordpress.org/support/forum/requests-and-feedback"), &tr("Feedback"), false)),
    ];
    top.push(item(
        "wp-logo",
        "menupop",
        &format!(
            "{}<div class=\"ab-sub-wrapper\">{}{}</div>",
            link(&admin("about.php"), &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"screen-reader-text\">{about}</span>"), true),
            submenu(&about, "wp-logo-default", "ab-submenu", &logo_primary),
            submenu(&about, "wp-logo-external", "ab-sub-secondary ab-submenu", &logo_secondary)
        ),
    ));

    // Site name: the dashboard, plugins and themes
    let site_title = if blog_name.is_empty() { tr("Dashboard") } else { blog_name.clone() };
    let mut default_items = vec![item("dashboard", "", &link(&admin(""), &tr("Dashboard"), false))];
    if user.can("activate_plugins") {
        default_items.push(item("plugins", "", &link(&admin("plugins.php"), &tr("Plugins"), false)));
    }
    let mut groups = submenu(&site_title, "site-name-default", "ab-submenu", &default_items);
    if user.can("switch_themes") || user.can("edit_theme_options") {
        let mut appearance = Vec::new();
        if user.can("switch_themes") {
            appearance.push(item("themes", "", &link(&admin("themes.php"), &tr("Themes"), false)));
        }
        groups.push_str(&submenu(&site_title, "appearance", "ab-submenu", &appearance));
    }
    top.push(item("site-name", "menupop", &format!("{}<div class=\"ab-sub-wrapper\">{groups}</div>", link(&admin(""), &esc_html(&site_title), true))));

    // Edit Site
    if let Some(url) = &ctx.site_editor_url {
        top.push(item("site-editor", "", &link(&esc_url(url), &tr("Edit Site"), false)));
    }

    // Updates
    if ctx.updates > 0 && ["update_core", "update_plugins", "update_themes"].iter().any(|c| user.can(c)) {
        let text = rustpress_core::i18n::sprintf(&site.trn("%s update available", "%s updates available", ctx.updates as i64), &[&number(ctx.updates)]);
        top.push(item(
            "updates",
            "",
            &link(&admin("update-core.php"), &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"ab-label\" aria-hidden=\"true\">{}</span><span class=\"screen-reader-text updates-available-text\">{text}</span>", number(ctx.updates)), false),
        ));
    }

    // Comments
    if user.can("edit_posts") {
        let text = rustpress_core::i18n::sprintf(&site.trn("%s Comment in moderation", "%s Comments in moderation", ctx.pending_comments as i64), &[&number(ctx.pending_comments)]);
        top.push(item(
            "comments",
            "",
            &link(
                &admin("edit-comments.php"),
                &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"ab-label awaiting-mod pending-count count-{}\" aria-hidden=\"true\">{}</span><span class=\"screen-reader-text comments-in-moderation-text\">{text}</span>", ctx.pending_comments, number(ctx.pending_comments)),
                false,
            ),
        ));
    }

    // New
    let mut new_items: Vec<String> = Vec::new();
    let mut add_new = |id: &str, href: String, label: &str| new_items.push(item(&format!("new-{id}"), "", &link(&href, &esc_html(label), false)));
    if user.can("edit_posts") {
        add_new("post", admin("post-new.php"), &site.trx("Post", "post type singular name"));
    }
    if user.can("upload_files") {
        add_new("media", admin("media-new.php"), &site.trx("Media", "add new from admin bar"));
    }
    if user.can("edit_pages") {
        add_new("page", admin("post-new.php?post_type=page"), &site.trx("Page", "post type singular name"));
    }
    for custom in &site.types.post_types {
        if (custom.public || custom.queryable) && user.can("edit_posts") {
            let label = custom.singular_label();
            add_new(&custom.name, admin(&format!("post-new.php?post_type={}", custom.name)), &label);
        }
    }
    if user.can("create_users") || user.can("promote_users") {
        add_new("user", admin("user-new.php"), &site.trx("User", "add new from admin bar"));
    }
    if !new_items.is_empty() {
        let first = admin(if user.can("edit_posts") { "post-new.php" } else if user.can("upload_files") { "media-new.php" } else { "user-new.php" });
        top.push(item(
            "new-content",
            "menupop",
            &format!(
                "{}<div class=\"ab-sub-wrapper\">{}</div>",
                link(&first, &format!("<span class=\"ab-icon\" aria-hidden=\"true\"></span><span class=\"ab-label\">{}</span>", site.trx("New", "admin bar menu group label")), true),
                submenu(&site.trx("New", "admin bar menu group label"), "new-content-default", "ab-submenu", &new_items)
            ),
        ));
    }

    // The item of the page
    if let Some((label, url)) = &ctx.edit {
        top.push(item("edit", "", &link(&esc_url(url), &esc_html(label), false)));
    }

    // My account and the search
    let howdy = rustpress_core::i18n::sprintf(&tr("Howdy, %s"), &[&format!("<span class=\"display-name\">{}</span>", esc_html(&user.display_name))]);
    let profile = admin("profile.php");
    let info = item(
        "user-info",
        "",
        &link(&profile, &format!("{}<span class='display-name'>{}</span><span class='display-name edit-profile'>{}</span>", avatar(site, &user.email, 64), esc_html(&user.display_name), tr("Edit Profile")), false),
    );
    let logout_url = esc_url(&format!("{}?action=logout&_wpnonce={}", site.site_url("wp-login.php"), ctx.logout_nonce));
    let logout = item("logout", "", &link(&logout_url, &tr("Log Out"), false));
    let account = item(
        "my-account",
        "menupop with-avatar",
        &format!("{}<div class=\"ab-sub-wrapper\">{}</div>", link(&profile, &format!("{howdy}{}", avatar(site, &user.email, 28)), true), submenu(&rustpress_core::i18n::sprintf(&tr("Howdy, %s"), &[&user.display_name]), "user-actions", "ab-submenu", &[info, logout])),
    );
    let search = format!(
        "<li role='group' id='wp-admin-bar-search' class=\"admin-bar-search\"><div class=\"ab-item ab-empty-item\" tabindex=\"-1\" role=\"menuitem\"><form action=\"{}\" method=\"get\" id=\"adminbarsearch\"><input class=\"adminbar-input\" name=\"s\" id=\"adminbar-search\" type=\"text\" value=\"\" maxlength=\"150\" /><label for=\"adminbar-search\" class=\"screen-reader-text\">{}</label><input type=\"submit\" class=\"adminbar-button\" value=\"{}\" /></form></div></li>",
        esc_url(&site.home_url("/")),
        tr("Search"),
        tr("Search")
    );

    format!(
        "\t\t<div id=\"wpadminbar\" class=\"nojq nojs\">\n\t\t\t\t\t\t<div class=\"quicklinks\" id=\"wp-toolbar\" role=\"navigation\" aria-label=\"{}\">\n\t\t\t\t<ul role='menu' id='wp-admin-bar-root-default' class=\"ab-top-menu\">{}</ul><ul role='menu' id='wp-admin-bar-top-secondary' class=\"ab-top-secondary ab-top-menu\">{account}{search}</ul>\t\t\t</div>\n\t\t</div>\n\n\t\t",
        esc_attr(&tr("Toolbar")),
        top.concat()
    )
}
