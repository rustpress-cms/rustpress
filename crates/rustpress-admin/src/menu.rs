//! The main menu of the administration area: the core entries of `wp-admin/menu.php`, the entries of
//! custom post types, and the output of `_wp_menu_output()`.

use rustpress_auth::CurrentUser;
use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, wptexturize};

/// A top-level entry: `$menu[ $position ]`.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub position: u32,
    /// HTML title.
    pub title: String,
    pub capability: String,
    pub slug: String,
    /// `$item[4]`: CSS classes.
    pub classes: String,
    /// `$item[5]`: the `id` of the entry.
    pub id: String,
    /// `$item[6]`: `dashicons-…`, `none`, or an image address.
    pub icon: String,
    /// The address of a plugin's page (`admin.php?page=<slug>`); other entries link their slug.
    pub href: Option<String>,
}

/// An entry of a submenu: `$submenu[ $parent ][ $position ]`.
#[derive(Debug, Clone)]
pub struct SubItem {
    pub position: u32,
    pub title: String,
    pub capability: String,
    pub slug: String,
    pub classes: String,
    /// The address of a plugin's page (`options-general.php?page=<slug>`); other entries link their slug.
    pub href: Option<String>,
    /// `menu_page_url()` of a plugin's page: `admin.php?page=<slug>` below a plugin's own menu, the parent with the
    /// page otherwise (unescaped).
    pub page_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AdminMenu {
    pub items: Vec<MenuItem>,
    pub submenu: Vec<(String, Vec<SubItem>)>,
}

/// What the menu depends on besides the user.
#[derive(Debug, Clone, Default)]
pub struct MenuContext {
    pub block_theme: bool,
    /// Comments waiting for moderation.
    pub pending_comments: u64,
    /// Updates available, by kind: total, plugins, themes.
    pub updates: (u64, u64, u64),
    /// `$_wp_last_object_menu`-based ordering of custom types needs their definitions.
    pub site_title: String,
    /// `$_SERVER['REQUEST_URI']`: the Customizer returns to it.
    pub request_uri: String,
}

fn item(position: u32, title: &str, capability: &str, slug: &str, classes: &str, id: &str, icon: &str) -> MenuItem {
    MenuItem { position, title: title.to_owned(), capability: capability.to_owned(), slug: slug.to_owned(), classes: classes.to_owned(), id: id.to_owned(), icon: icon.to_owned(), href: None }
}

fn sub(position: u32, title: &str, capability: &str, slug: &str) -> SubItem {
    SubItem { position, title: title.to_owned(), capability: capability.to_owned(), slug: slug.to_owned(), classes: String::new(), href: None, page_url: None }
}

impl AdminMenu {
    fn add_sub(&mut self, parent: &str, entry: SubItem) {
        match self.submenu.iter_mut().find(|(p, _)| p == parent) {
            Some((_, list)) => list.push(entry),
            None => self.submenu.push((parent.to_owned(), vec![entry])),
        }
    }

    /// `wp-admin/menu.php` for a site. The titles are translated with the site's catalog.
    pub fn core(site: &Site, user: &CurrentUser, ctx: &MenuContext) -> Self {
        let tr = |s: &str| site.tr(s);
        let number = |n: u64| site.number_i18n(n);
        let mut menu = AdminMenu::default();
        menu.items.push(item(2, &tr("Dashboard"), "read", "index.php", "menu-top menu-top-first menu-icon-dashboard", "menu-dashboard", "dashicons-dashboard"));
        menu.add_sub("index.php", sub(0, &tr("Home"), "read", "index.php"));
        let update_capability = ["update_core", "update_plugins", "update_themes"].iter().find(|c| user.can(c)).copied().unwrap_or("update_languages");
        menu.add_sub(
            "index.php",
            sub(
                10,
                &rustpress_core::i18n::sprintf(&tr("Updates %s"), &[&format!("<span class=\"update-plugins count-{}\"><span class=\"update-count\">{}</span></span>", ctx.updates.0, number(ctx.updates.0))]),
                update_capability,
                "update-core.php",
            ),
        );
        menu.items.push(item(4, "", "read", "separator1", "wp-menu-separator", "", ""));
        menu.items.push(item(10, &tr("Media"), "upload_files", "upload.php", "menu-top menu-icon-media", "menu-media", "dashicons-admin-media"));
        menu.add_sub("upload.php", sub(5, &site.trx("Library", "media library menu item"), "upload_files", "upload.php"));
        menu.add_sub("upload.php", sub(10, &tr("Add Media File"), "upload_files", "media-new.php"));
        if user.can("edit_posts") {
            let count = ctx.pending_comments;
            let text = rustpress_core::i18n::sprintf(&site.trn("%s Comment in moderation", "%s Comments in moderation", count as i64), &[&number(count)]);
            let badge = format!("<span class=\"awaiting-mod count-{count}\"><span class=\"pending-count\" aria-hidden=\"true\">{}</span><span class=\"comments-in-moderation-text screen-reader-text\">{text}</span></span>", number(count));
            menu.items.push(item(25, &rustpress_core::i18n::sprintf(&tr("Comments %s"), &[&badge]), "edit_posts", "edit-comments.php", "menu-top menu-icon-comments", "menu-comments", "dashicons-admin-comments"));
        }
        menu.add_sub("edit-comments.php", sub(0, &tr("All Comments"), "edit_posts", "edit-comments.php"));

        // Posts, pages and the custom post types, in that order.
        let mut last_object_menu = 25u32;
        // (name, menu name, all items, add new, icon, taxonomies, hierarchical, menu position)
        type TypeEntry = (String, String, String, String, String, Vec<(String, String)>, bool, Option<i64>);
        let registered = site.types.admin_post_types();
        let registered_taxonomies = site.types.admin_taxonomies();
        // `get_taxonomies()`: the taxonomies of a type that have a screen and a menu entry.
        let taxonomies_of = |post_type: &str| -> Vec<(String, String)> {
            let mut out: Vec<(String, String)> = Vec::new();
            if post_type == "post" {
                out.push(("category".into(), site.taxonomy_label("category")));
                out.push(("post_tag".into(), site.taxonomy_label("post_tag")));
            }
            for t in &registered_taxonomies {
                if t.show_ui && t.show_in_menu && t.object_types.iter().any(|o| o == post_type) && !out.iter().any(|(n, _)| *n == t.name) {
                    out.push((t.name.clone(), t.menu_name().unwrap_or_else(|| site.taxonomy_label(&t.name))));
                }
            }
            // Taxonomies only the rewrite rules know.
            for t in &site.types.taxonomies {
                if !registered_taxonomies.iter().any(|r| r.name == t.name) && t.object_types.iter().any(|o| o == post_type) && !out.iter().any(|(n, _)| *n == t.name) {
                    out.push((t.name.clone(), site.taxonomy_label(&t.name)));
                }
            }
            out
        };
        let pick = |hierarchical: bool, post: &str, page: &str| tr(if hierarchical { page } else { post });
        let mut types: Vec<TypeEntry> = Vec::new();
        types.push(("post".into(), site.post_type_label("post"), tr("All Posts"), tr("Add Post"), "dashicons-admin-post".into(), taxonomies_of("post"), false, Some(5)));
        types.push(("page".into(), site.post_type_label("page"), tr("All Pages"), tr("Add Page"), "dashicons-admin-page".into(), taxonomies_of("page"), true, Some(20)));
        for custom in registered.iter().filter(|t| t.own_menu) {
            // `menu_icon`: an empty or Dashicons icon or an SVG as it is, a URL escaped.
            let icon = match custom.menu_icon.as_deref() {
                Some(icon) if icon == "none" || icon == "div" || icon.starts_with("data:image/svg+xml;base64,") || icon.starts_with("dashicons-") => icon.to_owned(),
                Some(icon) => rustpress_core::formatting::esc_url(icon),
                None => "dashicons-admin-post".to_owned(),
            };
            let menu_name = custom.label("menu_name").unwrap_or_else(|| site.post_type_label(&custom.name));
            let all_items = custom.label("all_items").unwrap_or_else(|| pick(custom.hierarchical, "All Posts", "All Pages"));
            let add_new = custom.label("add_new_item").unwrap_or_else(|| pick(custom.hierarchical, "Add Post", "Add Page"));
            types.push((custom.name.clone(), menu_name, all_items, add_new, icon, taxonomies_of(&custom.name), custom.hierarchical, custom.menu_position));
        }
        // Types only the rewrite rules know of.
        for custom in &site.types.post_types {
            if registered.iter().any(|r| r.name == custom.name) || (!custom.public && !custom.queryable) {
                continue;
            }
            let icon = custom.menu_icon.clone().unwrap_or_else(|| "dashicons-admin-post".to_owned());
            let label = site.post_type_label(&custom.name);
            let add_new = pick(custom.hierarchical, "Add Post", "Add Page");
            types.push((custom.name.clone(), label.clone(), label, add_new, icon, taxonomies_of(&custom.name), custom.hierarchical, None));
        }
        for (name, label, all_items, add_new, icon, taxonomies, _hierarchical, menu_position) in types {
            let mut position = match menu_position {
                Some(p) if p >= 0 => p as u32,
                _ => {
                    last_object_menu += 1;
                    last_object_menu
                }
            };
            while menu.items.iter().any(|m| m.position == position) || [59, 60, 65, 70, 75, 80, 85, 99].contains(&position) {
                position += 1;
            }
            let for_id = rustpress_core::formatting::sanitize_html_class(&name);
            let (file, new_file, tags_file) = if name == "post" {
                ("edit.php".to_owned(), "post-new.php".to_owned(), "edit-tags.php?taxonomy=%s".to_owned())
            } else {
                (format!("edit.php?post_type={name}"), format!("post-new.php?post_type={name}"), format!("edit-tags.php?taxonomy=%s&amp;post_type={name}"))
            };
            let mut classes = format!("menu-top menu-icon-{for_id}");
            if name == "post" {
                classes.push_str(" open-if-no-js");
            }
            let id = if name == "post" || name == "page" { format!("menu-{for_id}s") } else { format!("menu-posts-{for_id}") };
            menu.items.push(item(position, &esc_attr(&label), "edit_posts", &file, &classes, &id, &icon));
            menu.add_sub(&file, sub(5, &all_items, "edit_posts", &file));
            menu.add_sub(&file, sub(10, &add_new, "edit_posts", &new_file));
            for (index, (taxonomy, label)) in taxonomies.iter().enumerate() {
                let capability = "manage_categories";
                menu.add_sub(&file, sub(15 + index as u32, &esc_attr(label), capability, &tags_file.replace("%s", taxonomy)));
            }
        }
        menu.items.push(item(59, "", "read", "separator2", "wp-menu-separator", "", ""));
        let appearance = if user.can("switch_themes") { "switch_themes" } else { "edit_theme_options" };
        menu.items.push(item(60, &tr("Appearance"), appearance, "themes.php", "menu-top menu-icon-appearance", "menu-appearance", "dashicons-admin-appearance"));
        let theme_count = if user.can("update_themes") { format!("<span class=\"update-plugins count-{}\"><span class=\"theme-count\">{}</span></span>", ctx.updates.2, number(ctx.updates.2)) } else { String::new() };
        menu.add_sub("themes.php", sub(5, &rustpress_core::i18n::sprintf(&tr("Themes %s"), &[&theme_count]), appearance, "themes.php"));
        if ctx.block_theme {
            menu.add_sub("themes.php", sub(6, &site.trx("Editor", "site editor menu item"), "edit_theme_options", "site-editor.php"));
        } else {
            menu.add_sub("themes.php", sub(6, &site.trx("Design", "design menu item"), "edit_theme_options", "site-editor.php"));
        }
        if !ctx.block_theme {
            // `$customize_url`, with `wp_removable_query_args()` taken out of the way back.
            let mut back = ctx.request_uri.clone();
            for arg in crate::chrome::REMOVABLE_QUERY_ARGS {
                back = rustpress_core::urls::remove_query_arg(&back, arg);
            }
            let customize = format!("customize.php?return={}", rustpress_core::canonical::php_urlencode(&back));
            let hidden = |mut entry: SubItem| {
                entry.classes = "hide-if-no-customize".to_owned();
                entry
            };
            menu.add_sub("themes.php", hidden(sub(7, &tr("Customize"), "edit_theme_options", &rustpress_core::formatting::esc_url(&customize))));
            menu.add_sub("themes.php", sub(8, &tr("Widgets"), "edit_theme_options", "widgets.php"));
            menu.add_sub("themes.php", sub(9, &tr("Fonts"), "edit_theme_options", "font-library.php"));
            menu.add_sub("themes.php", sub(10, &tr("Menus"), "edit_theme_options", "nav-menus.php"));
            let header = format!("{customize}&autofocus%5Bcontrol%5D=header_image");
            menu.add_sub("themes.php", hidden(sub(15, &site.trx("Header", "custom image header"), appearance, &rustpress_core::formatting::esc_url(&header))));
            menu.add_sub("themes.php", sub(16, &site.trx("Header", "custom image header"), "edit_theme_options", "themes.php?page=custom-header"));
            menu.add_sub("themes.php", sub(17, &tr("Theme File Editor"), "edit_themes", "theme-editor.php"));
        } else {
            menu.add_sub("themes.php", sub(9, &tr("Fonts"), "edit_theme_options", "font-library.php"));
        }
        let plugin_count = if user.can("update_plugins") { format!("<span class=\"update-plugins count-{}\"><span class=\"plugin-count\">{}</span></span>", ctx.updates.1, number(ctx.updates.1)) } else { String::new() };
        menu.items.push(item(65, &rustpress_core::i18n::sprintf(&tr("Plugins %s"), &[&plugin_count]), "activate_plugins", "plugins.php", "menu-top menu-icon-plugins", "menu-plugins", "dashicons-admin-plugins"));
        menu.add_sub("plugins.php", sub(5, &tr("Installed Plugins"), "activate_plugins", "plugins.php"));
        menu.add_sub("plugins.php", sub(10, &tr("Add Plugin"), "install_plugins", "plugin-install.php"));
        if !ctx.block_theme {
            menu.add_sub("plugins.php", sub(15, &tr("Plugin File Editor"), "edit_plugins", "plugin-editor.php"));
        }
        if user.can("list_users") {
            menu.items.push(item(70, &tr("Users"), "list_users", "users.php", "menu-top menu-icon-users", "menu-users", "dashicons-admin-users"));
            menu.add_sub("users.php", sub(5, &tr("All Users"), "list_users", "users.php"));
            if user.can("create_users") {
                menu.add_sub("users.php", sub(10, &tr("Add User"), "create_users", "user-new.php"));
            }
            menu.add_sub("users.php", sub(15, &tr("Profile"), "read", "profile.php"));
        } else {
            menu.items.push(item(70, &tr("Profile"), "read", "profile.php", "menu-top menu-icon-users", "menu-users", "dashicons-admin-users"));
            menu.add_sub("profile.php", sub(5, &tr("Profile"), "read", "profile.php"));
            if user.can("create_users") {
                menu.add_sub("profile.php", sub(10, &tr("Add User"), "create_users", "user-new.php"));
            }
        }
        menu.items.push(item(75, &tr("Tools"), "edit_posts", "tools.php", "menu-top menu-icon-tools", "menu-tools", "dashicons-admin-tools"));
        menu.add_sub("tools.php", sub(5, &tr("Available Tools"), "edit_posts", "tools.php"));
        menu.add_sub("tools.php", sub(10, &tr("Import"), "import", "import.php"));
        menu.add_sub("tools.php", sub(15, &tr("Export"), "export", "export.php"));
        menu.add_sub("tools.php", sub(20, &rustpress_core::i18n::sprintf(&tr("Site Health %s"), &["<span class=\"menu-counter site-health-counter count-0\"><span class=\"count\">0</span></span>"]), "view_site_health_checks", "site-health.php"));
        menu.add_sub("tools.php", sub(25, &tr("Export Personal Data"), "export_others_personal_data", "export-personal-data.php"));
        menu.add_sub("tools.php", sub(30, &tr("Erase Personal Data"), "erase_others_personal_data", "erase-personal-data.php"));
        if ctx.block_theme {
            menu.add_sub("tools.php", sub(35, &tr("Theme File Editor"), "edit_themes", "theme-editor.php"));
            menu.add_sub("tools.php", sub(36, &tr("Plugin File Editor"), "edit_plugins", "plugin-editor.php"));
        }
        menu.items.push(item(80, &tr("Settings"), "manage_options", "options-general.php", "menu-top menu-icon-settings", "menu-settings", "dashicons-admin-settings"));
        for (position, title, capability, slug) in [
            (10, site.trx("General", "settings screen"), "manage_options", "options-general.php"),
            (12, tr("Connectors"), "manage_options", "options-connectors.php"),
            (15, tr("Writing"), "manage_options", "options-writing.php"),
            (20, tr("Reading"), "manage_options", "options-reading.php"),
            (25, tr("Discussion"), "manage_options", "options-discussion.php"),
            (30, tr("Media"), "manage_options", "options-media.php"),
            (40, tr("Permalinks"), "manage_options", "options-permalink.php"),
            (45, tr("Privacy"), "manage_privacy_options", "options-privacy.php"),
        ] {
            menu.add_sub("options-general.php", sub(position, &title, capability, slug));
        }
        menu.items.push(item(99, "", "read", "separator-last", "wp-menu-separator", "", ""));
        menu
    }

    /// `add_submenu_page()` of the plugins (`admin_menu`): each page goes after the entries its parent has.
    pub fn with_plugin_pages(mut self, pages: &[rustpress_core::native::AdminPage]) -> Self {
        // `add_menu_page()`: the entries of the main menu come first, at their position or last.
        for page in pages.iter().filter(|p| p.is_top_level()) {
            let position = page.position.unwrap_or_else(|| self.items.iter().map(|m| m.position + 1).max().unwrap_or(0));
            // A menu without a callback whose slug is a screen (`edit.php?post_type=...`) links to that screen.
            let file = page.slug.contains(".php");
            let hook = format!("toplevel_page_{}", page.slug.replace(".php", ""));
            let (icon, icon_class) = if page.icon.is_empty() { ("dashicons-admin-generic", "menu-icon-generic ") } else { (page.icon, "") };
            self.items.push(MenuItem {
                position,
                title: page.menu_title.clone(),
                capability: page.capability.to_owned(),
                slug: page.slug.to_owned(),
                classes: format!("menu-top {icon_class}{hook}"),
                id: hook,
                icon: icon.to_owned(),
                href: (!file).then(|| format!("admin.php?page={}", page.slug)),
            });
        }
        let plugin_menus: Vec<&str> = pages.iter().filter(|p| p.is_top_level() && !p.slug.contains(".php")).map(|p| p.slug).collect();
        // `$_parent_pages`: every menu of `add_menu_page()`.
        let parent_pages: Vec<&str> = pages.iter().filter(|p| p.is_top_level()).map(|p| p.slug).collect();
        for page in pages.iter().filter(|p| !p.is_top_level()) {
            let position = self.submenu.iter().find(|(p, _)| p == page.parent).and_then(|(_, list)| list.iter().map(|s| s.position + 1).max()).unwrap_or(0);
            let href = if page.slug.contains(".php") {
                // A submenu entry that is a screen of its own.
                None
            } else if plugin_menus.contains(&page.parent) {
                Some(format!("admin.php?page={}", page.slug))
            } else if page.parent.contains('?') {
                Some(format!("{}&#038;page={}", page.parent, page.slug))
            } else {
                Some(format!("{}?page={}", page.parent, page.slug))
            };
            let page_url = if parent_pages.contains(&page.parent) {
                format!("admin.php?page={}", page.slug)
            } else {
                rustpress_core::urls::add_query_arg(page.parent, "page", page.slug)
            };
            self.add_sub(page.parent, SubItem { position, title: page.menu_title.clone(), capability: page.capability.to_owned(), slug: page.slug.to_owned(), classes: String::new(), href, page_url: Some(page_url) });
        }
        self
    }

    /// `wp-admin/includes/menu.php`: hide what the user may not use, order the entries, number the
    /// first and last ones.
    pub fn prepare(mut self, user: &CurrentUser) -> Self {
        // Entries without permission go; submenus are ordered by position.
        for (_, list) in &mut self.submenu {
            list.retain(|s| user.can(&s.capability));
            list.sort_by_key(|s| s.position);
        }
        self.submenu.retain(|(_, list)| !list.is_empty());
        // A parent points at its first allowed child.
        let moves: Vec<(String, String)> = self
            .items
            .iter()
            .filter_map(|m| {
                let list = &self.submenu.iter().find(|(p, _)| *p == m.slug)?.1;
                let first = list.first()?;
                (first.slug != m.slug).then(|| (m.slug.clone(), first.slug.clone()))
            })
            .collect();
        for (old, new) in moves {
            for m in &mut self.items {
                if m.slug == old {
                    m.slug = new.clone();
                }
            }
            for (parent, _) in &mut self.submenu {
                if *parent == old {
                    *parent = new.clone();
                }
            }
        }
        self.items.sort_by_key(|m| m.position);
        // Hidden: no permission and no submenu; a lone submenu entry that repeats its parent is dropped.
        let allowed: Vec<bool> = self.items.iter().map(|m| user.can(&m.capability)).collect();
        let mut keep = Vec::new();
        for (m, ok) in self.items.drain(..).zip(allowed) {
            if let Some((_, list)) = self.submenu.iter().find(|(p, _)| *p == m.slug)
                && list.len() == 1
                && list[0].slug == m.slug
            {
                self.submenu.retain(|(p, _)| *p != m.slug);
            }
            let has_sub = self.submenu.iter().any(|(p, _)| *p == m.slug);
            if !ok && !has_sub {
                continue;
            }
            keep.push(m);
        }
        self.items = keep;
        // Two separators in a row, or a separator at the end, go.
        let mut cleaned: Vec<MenuItem> = Vec::new();
        let mut previous_separator = false;
        for m in std::mem::take(&mut self.items) {
            let separator = m.classes.contains("wp-menu-separator");
            if separator && previous_separator {
                continue;
            }
            previous_separator = separator;
            cleaned.push(m);
        }
        if cleaned.last().is_some_and(|m| m.classes == "wp-menu-separator") {
            cleaned.pop();
        }
        self.items = cleaned;
        // `add_menu_classes()`
        let count = self.items.len();
        let mut first_item = false;
        let mut last_order: Option<usize> = None;
        for index in 0..count {
            if self.items[index].position == 0 {
                self.items[index].classes = add_class("menu-top-first", &self.items[index].classes);
                last_order = Some(index);
                continue;
            }
            if self.items[index].slug.starts_with("separator")
                && let Some(last) = last_order
            {
                first_item = true;
                self.items[last].classes = add_class("menu-top-last", &self.items[last].classes);
                continue;
            }
            if first_item {
                first_item = false;
                self.items[index].classes = add_class("menu-top-first", &self.items[index].classes);
            }
            if index + 1 == count {
                self.items[index].classes = add_class("menu-top-last", &self.items[index].classes);
            }
            last_order = Some(index);
        }
        self
    }

    /// `_wp_menu_output()` for the page `self_file` (`options-general.php`), whose parent is `parent_file`
    /// and whose submenu entry is `submenu_file`.
    pub fn render(&self, site: &Site, self_file: &str, parent_file: &str, submenu_file: Option<&str>, post_type: &str) -> String {
        let mut out = String::new();
        let mut first = true;
        for item in &self.items {
            let mut classes: Vec<String> = Vec::new();
            let mut aria = String::new();
            if first {
                classes.push("wp-first-item".to_owned());
                first = false;
            }
            let subs: &[SubItem] = self.submenu.iter().find(|(p, _)| *p == item.slug).map_or(&[], |(_, l)| l.as_slice());
            if !subs.is_empty() {
                classes.push("wp-has-submenu".to_owned());
            }
            if (!parent_file.is_empty() && item.slug == parent_file) || (post_type.is_empty() && self_file == item.slug) {
                if subs.is_empty() {
                    classes.push("current".to_owned());
                    aria.push_str("aria-current=\"page\"");
                } else {
                    classes.push("wp-has-current-submenu wp-menu-open".to_owned());
                }
            } else {
                classes.push("wp-not-current-submenu".to_owned());
                if !subs.is_empty() {
                    aria.push_str("data-ariahaspopup");
                }
            }
            if !item.classes.is_empty() {
                classes.push(esc_attr(&item.classes));
            }
            let class = if classes.is_empty() { String::new() } else { format!(" class=\"{}\"", classes.join(" ")) };
            let id = if item.id.is_empty() { String::new() } else { format!(" id=\"{}\"", item.id.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '.') { c } else { '-' }).collect::<String>()) };
            let mut img = String::new();
            let mut img_class = " dashicons-before".to_owned();
            let mut img_style = String::new();
            if !item.icon.is_empty() {
                img = format!("<img src=\"{}\" alt=\"\" />", rustpress_core::formatting::esc_url(&item.icon));
                if item.icon == "none" || item.icon == "div" {
                    img = "<br />".to_owned();
                } else if item.icon.starts_with("data:image/svg+xml;base64,") {
                    img = "<br />".to_owned();
                    img_style = format!(" style=\"background-image:url('{}')\"", esc_attr(&item.icon));
                    img_class = " svg".to_owned();
                } else if item.icon.starts_with("dashicons-") {
                    img = "<br />".to_owned();
                    img_class = format!(" dashicons-before {}", rustpress_core::formatting::sanitize_html_class(&item.icon));
                }
            }
            let separator = class.contains("wp-menu-separator");
            let title = wptexturize(&item.title);
            let hidden = if separator { " aria-hidden=\"true\"" } else { "" };
            out.push_str(&format!("\n\t<li{class}{id}{hidden}>"));
            if separator {
                out.push_str("<div class=\"separator\"></div>");
            } else if !subs.is_empty() {
                // The link to a plugin's page (`admin.php?page=`) comes without the line break of the others.
                let indent = if subs[0].href.is_some() { "" } else { "\n\t" };
                out.push_str(&format!("{indent}<a href='{}'{class} {aria}><div class='wp-menu-image{img_class}'{img_style} aria-hidden='true'>{img}</div><div class='wp-menu-name'>{title}</div></a>", subs[0].href.as_deref().unwrap_or(&subs[0].slug)));
            } else if !item.slug.is_empty() {
                out.push_str(&format!("\n\t<a href='{}'{class} {aria}><div class='wp-menu-image{img_class}'{img_style} aria-hidden='true'>{img}</div><div class='wp-menu-name'>{}</div></a>", item.href.as_deref().unwrap_or(&item.slug), item.title));
            }
            if !subs.is_empty() {
                out.push_str("\n\t<ul class='wp-submenu wp-submenu-wrap'>");
                out.push_str(&format!("<li class='wp-submenu-head' aria-hidden='true'>{}</li>", item.title));
                let mut first_sub = true;
                for entry in subs {
                    let mut class: Vec<String> = Vec::new();
                    let mut aria = String::new();
                    if first_sub {
                        class.push("wp-first-item".to_owned());
                        first_sub = false;
                    }
                    if submenu_file.is_some_and(|f| f == entry.slug) {
                        class.push("current".to_owned());
                        aria.push_str(" aria-current=\"page\"");
                    }
                    if !entry.classes.is_empty() {
                        class.push(esc_attr(&entry.classes));
                    }
                    let class = if class.is_empty() { String::new() } else { format!(" class=\"{}\"", class.join(" ")) };
                    let href = entry.href.as_deref().map_or_else(|| entry.slug.clone(), rustpress_core::formatting::esc_url);
                    out.push_str(&format!("<li{class}><a href='{href}'{class}{aria}>{}</a></li>", wptexturize(&entry.title)));
                }
                out.push_str("</ul>");
            }
            out.push_str("</li>");
        }
        out.push_str(&format!(
            "<li id=\"collapse-menu\" class=\"hide-if-no-js\"><button type=\"button\" id=\"collapse-button\" aria-label=\"{}\" aria-expanded=\"true\"><span class=\"collapse-button-icon\" aria-hidden=\"true\"></span><span class=\"collapse-button-label\">{}</span></button></li>",
            esc_attr(&site.tr("Collapse Main Menu")),
            site.tr("Collapse Menu")
        ));
        out
    }
}

fn add_class(class: &str, classes: &str) -> String {
    if classes.is_empty() { class.to_owned() } else { format!("{classes} {class}") }
}
