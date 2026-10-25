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
}

/// An entry of a submenu: `$submenu[ $parent ][ $position ]`.
#[derive(Debug, Clone)]
pub struct SubItem {
    pub position: u32,
    pub title: String,
    pub capability: String,
    pub slug: String,
    pub classes: String,
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
}

fn item(position: u32, title: &str, capability: &str, slug: &str, classes: &str, id: &str, icon: &str) -> MenuItem {
    MenuItem { position, title: title.to_owned(), capability: capability.to_owned(), slug: slug.to_owned(), classes: classes.to_owned(), id: id.to_owned(), icon: icon.to_owned() }
}

fn sub(position: u32, title: &str, capability: &str, slug: &str) -> SubItem {
    SubItem { position, title: title.to_owned(), capability: capability.to_owned(), slug: slug.to_owned(), classes: String::new() }
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
        type TypeEntry = (String, String, String, Vec<(String, String)>, bool);
        let mut types: Vec<TypeEntry> = Vec::new();
        types.push(("post".into(), site.post_type_label("post"), "dashicons-admin-post".into(), vec![("category".into(), site.taxonomy_label("category")), ("post_tag".into(), site.taxonomy_label("post_tag"))], false));
        types.push(("page".into(), site.post_type_label("page"), "dashicons-admin-page".into(), Vec::new(), true));
        for custom in &site.types.post_types {
            if !custom.public && !custom.queryable {
                continue;
            }
            let icon = custom.menu_icon.clone().unwrap_or_else(|| "dashicons-admin-post".to_owned());
            let taxonomies = site.types.taxonomies.iter().filter(|t| t.object_types.contains(&custom.name)).map(|t| (t.name.clone(), site.taxonomy_label(&t.name))).collect();
            types.push((custom.name.clone(), site.post_type_label(&custom.name), icon, taxonomies, custom.hierarchical));
        }
        for (name, label, icon, taxonomies, hierarchical) in types {
            let position = loop {
                last_object_menu += 1;
                if !menu.items.iter().any(|m| m.position == last_object_menu) && ![59, 60, 65, 70, 75, 80, 85, 99].contains(&last_object_menu) {
                    break last_object_menu;
                }
            };
            let (position, for_id) = if name == "post" {
                (5, "post".to_owned())
            } else if name == "page" {
                (20, "page".to_owned())
            } else {
                (position, rustpress_core::formatting::sanitize_html_class(&name))
            };
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
            let (all_items, add_new) = match name.as_str() {
                "post" => (tr("All Posts"), tr("Add Post")),
                "page" => (tr("All Pages"), tr("Add Page")),
                _ => (label.clone(), if hierarchical { tr("Add Page") } else { tr("Add Post") }),
            };
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
        menu.add_sub("themes.php", sub(9, &tr("Fonts"), "edit_theme_options", "font-library.php"));
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
        menu.add_sub("tools.php", sub(35, &tr("Theme File Editor"), "edit_themes", "theme-editor.php"));
        if ctx.block_theme {
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
                out.push_str(&format!("\n\t<a href='{}'{class} {aria}><div class='wp-menu-image{img_class}'{img_style} aria-hidden='true'>{img}</div><div class='wp-menu-name'>{title}</div></a>", subs[0].slug));
            } else if !item.slug.is_empty() {
                out.push_str(&format!("\n\t<a href='{}'{class} {aria}><div class='wp-menu-image{img_class}'{img_style} aria-hidden='true'>{img}</div><div class='wp-menu-name'>{}</div></a>", item.slug, item.title));
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
                    out.push_str(&format!("<li{class}><a href='{}'{class}{aria}>{}</a></li>", entry.slug, wptexturize(&entry.title)));
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
