//! `core/navigation`, `core/page-list` and `core/navigation-link`.

use rustpress_core::formatting::{esc_attr, esc_url};
use rustpress_core::kses::kses_post;
use rustpress_db::{OrderBy, Order, Post, PostQuery};
use serde_json::{Map, Value, json};

use crate::Result;
use crate::html::{TagEditor, tags};
use crate::parser::{Block, parse_blocks};
use crate::php_json;
use crate::render::BlockCtx;
use crate::style_engine::to_kebab_case;
use crate::supports::safecss_filter_attr;

const SUBMENU_ICON: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"12\" height=\"12\" viewBox=\"0 0 12 12\" fill=\"none\" aria-hidden=\"true\" focusable=\"false\"><path d=\"M1.50002 4L6.00002 8L10.5 4\" stroke-width=\"1.5\"></path></svg>";

fn str_attr<'a>(attrs: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    attrs.get(key).and_then(Value::as_str)
}

fn is_responsive(attrs: &Map<String, Value>) -> bool {
    let old = attrs.get("isResponsive").and_then(Value::as_bool).unwrap_or(false);
    attrs.get("overlayMenu").and_then(Value::as_str).is_some_and(|m| m != "never") || old
}

/// `block_core_navigation_get_submenu_visibility`.
fn submenu_visibility(source: &Map<String, Value>) -> String {
    if let Some(v) = source.get("openSubmenusOnClick").filter(|v| !v.is_null()) {
        return if crate::style_engine::is_empty(v) { "hover".into() } else { "click".into() };
    }
    str_attr(source, "submenuVisibility").unwrap_or("hover").to_owned()
}

struct Colors {
    css_classes: Vec<String>,
    inline_styles: String,
    overlay_css_classes: Vec<String>,
    overlay_inline_styles: String,
}

/// `block_core_navigation_build_css_colors`.
fn build_colors(attrs: &Map<String, Value>) -> Colors {
    let mut c = Colors { css_classes: vec![], inline_styles: String::new(), overlay_css_classes: vec![], overlay_inline_styles: String::new() };
    let named = |k: &str| attrs.contains_key(k);
    let text = |k: &str| attrs.get(k).map(|v| v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())).unwrap_or_default();

    if named("customTextColor") || named("textColor") {
        c.css_classes.push("has-text-color".into());
    }
    if named("textColor") {
        c.css_classes.push(format!("has-{}-color", text("textColor")));
    } else if named("customTextColor") {
        c.inline_styles.push_str(&format!("color: {};", text("customTextColor")));
    }
    if named("customBackgroundColor") || named("backgroundColor") {
        c.css_classes.push("has-background".into());
    }
    if named("backgroundColor") {
        c.css_classes.push(format!("has-{}-background-color", text("backgroundColor")));
    } else if named("customBackgroundColor") {
        c.inline_styles.push_str(&format!("background-color: {};", text("customBackgroundColor")));
    }
    if named("customOverlayTextColor") || named("overlayTextColor") {
        c.overlay_css_classes.push("has-text-color".into());
    }
    if named("overlayTextColor") {
        c.overlay_css_classes.push(format!("has-{}-color", text("overlayTextColor")));
    } else if named("customOverlayTextColor") {
        c.overlay_inline_styles.push_str(&format!("color: {};", text("customOverlayTextColor")));
    }
    if named("customOverlayBackgroundColor") || named("overlayBackgroundColor") {
        c.overlay_css_classes.push("has-background".into());
    }
    if named("overlayBackgroundColor") {
        c.overlay_css_classes.push(format!("has-{}-background-color", text("overlayBackgroundColor")));
    } else if named("customOverlayBackgroundColor") {
        c.overlay_inline_styles.push_str(&format!("background-color: {};", text("customOverlayBackgroundColor")));
    }
    c
}

/// `block_core_navigation_build_css_font_sizes`.
fn build_font_sizes(attrs: &Map<String, Value>) -> (Vec<String>, String) {
    if let Some(size) = attrs.get("fontSize") {
        return (vec![format!("has-{}-font-size", size.as_str().unwrap_or_default())], String::new());
    }
    if let Some(custom) = attrs.get("customFontSize") {
        let n = custom.as_str().map(str::to_owned).unwrap_or_else(|| custom.to_string());
        return (vec![], format!("font-size: {n}px;"));
    }
    (vec![], String::new())
}

fn layout_class(attrs: &Map<String, Value>) -> String {
    let layout = attrs.get("layout").and_then(Value::as_object);
    let mut class = String::new();
    if let Some(j) = layout.and_then(|l| l.get("justifyContent")).and_then(Value::as_str) {
        let mapped = match j {
            "left" => Some("items-justified-left"),
            "right" => Some("items-justified-right"),
            "center" => Some("items-justified-center"),
            "space-between" => Some("items-justified-space-between"),
            _ => None,
        };
        if let Some(m) = mapped {
            class.push_str(m);
        }
    }
    if layout.and_then(|l| l.get("orientation")).and_then(Value::as_str) == Some("vertical") {
        class.push_str(" is-vertical");
    }
    if layout.and_then(|l| l.get("flexWrap")).and_then(Value::as_str) == Some("nowrap") {
        class.push_str(" no-wrap");
    }
    class
}

/// `get_classes`.
fn nav_classes(attrs: &Map<String, Value>) -> String {
    let colors = build_colors(attrs);
    let (font_classes, _) = build_font_sizes(attrs);
    let mut classes = colors.css_classes;
    classes.extend(font_classes);
    if is_responsive(attrs) {
        classes.push("is-responsive".into());
    }
    let layout = layout_class(attrs);
    if !layout.is_empty() {
        classes.push(layout);
    }
    if let Some(decoration) = attrs.get("style").and_then(|s| s.get("typography")).and_then(|t| t.get("textDecoration")).and_then(Value::as_str).filter(|d| !d.is_empty()) {
        classes.push(format!("has-text-decoration-{decoration}"));
    }
    classes.join(" ")
}

/// `get_styles`.
fn nav_styles(attrs: &Map<String, Value>) -> String {
    let colors = build_colors(attrs);
    let (_, font_style) = build_font_sizes(attrs);
    format!("{}{}{}", str_attr(attrs, "styles").unwrap_or(""), colors.inline_styles, font_style)
}

fn tree_has_block(blocks: &[Block], name: &str) -> bool {
    blocks.iter().any(|b| b.name.as_deref() == Some(name) || tree_has_block(&b.inner_blocks, name))
}

fn only_named(blocks: Vec<Block>) -> Vec<Block> {
    blocks.into_iter().filter(|b| b.name.is_some()).collect()
}

/// The blocks to render: the referenced `wp_navigation` post, the block's own
/// children, or the fallback (newest navigation post, else the page list).
async fn inner_blocks(bc: &BlockCtx<'_>) -> Result<Vec<Block>> {
    let attrs = &bc.attrs;
    let reference = attrs.get("navigationMenuId").or_else(|| attrs.get("ref")).and_then(Value::as_u64);
    if let Some(id) = reference {
        if let Some(post) = bc.env.site.db.post(id).await?
            && post.post_status == "publish" {
                return Ok(only_named(parse_blocks(&post.post_content)));
            }
        return fallback_blocks(bc).await;
    }
    if !bc.block.inner_blocks.is_empty() {
        return Ok(bc.block.inner_blocks.clone());
    }
    fallback_blocks(bc).await
}

async fn fallback_blocks(bc: &BlockCtx<'_>) -> Result<Vec<Block>> {
    let newest = bc
        .env
        .site
        .db
        .posts(&PostQuery {
            post_types: vec!["wp_navigation".into()],
            statuses: vec!["publish".into()],
            order_by: OrderBy::Date,
            order: Order::Desc,
            limit: Some(1),
            ..PostQuery::default()
        })
        .await?
        .pop();
    if let Some(post) = newest {
        let blocks = only_named(parse_blocks(&post.post_content));
        if !blocks.is_empty() {
            return Ok(blocks);
        }
    }
    Ok(parse_blocks("<!-- wp:page-list /-->"))
}

async fn has_submenus(bc: &BlockCtx<'_>, blocks: &[Block]) -> Result<bool> {
    if bc.env.with_state(|s| s.nav_has_submenus) {
        return Ok(true);
    }
    let mut found = false;
    for block in blocks {
        match block.name.as_deref() {
            Some("core/page-list") => {
                let pages = published_pages(bc).await?;
                if pages.iter().any(|p| p.post_parent != 0) {
                    found = true;
                }
            }
            Some("core/navigation-submenu") => found = true,
            _ => {}
        }
        if found {
            break;
        }
    }
    if found {
        bc.env.with_state(|s| s.nav_has_submenus = true);
    }
    Ok(found)
}

/// `get_pages()`: published pages by `menu_order, post_title`.
async fn published_pages(bc: &BlockCtx<'_>) -> Result<Vec<Post>> {
    let mut pages = bc.env.site.db.posts(&PostQuery { post_types: vec!["page".into()], ..PostQuery::default() }).await?;
    pages.sort_by_key(|a| (a.menu_order, a.post_title.to_lowercase(), a.id));
    Ok(pages)
}

pub(super) async fn navigation(bc: &BlockCtx<'_>) -> Result<String> {
    let blocks = inner_blocks(bc).await?;
    if tree_has_block(&blocks, "core/navigation") {
        return Ok(String::new());
    }
    let submenus = has_submenus(bc, &blocks).await?;
    let attrs = &bc.attrs;
    let responsive = is_responsive(attrs);
    let visibility = submenu_visibility(attrs);
    let show_icon = attrs.get("showSubmenuIcon").is_some_and(|v| !crate::style_engine::is_empty(v));
    let interactive = (submenus && (visibility == "click" || show_icon)) || responsive;
    if interactive {
        bc.env.with_state(|s| s.script_modules.insert("@wordpress/block-library/navigation/view".to_owned()));
    }

    // Inner blocks as <ul> items.
    let class = nav_classes(attrs);
    let style = nav_styles(attrs);
    let container = bc.wrapper_attributes(&[("class", format!("wp-block-navigation__container {class}")), ("style", style.clone())]);
    let mut html = String::new();
    let mut list_open = false;
    for block in &blocks {
        let mut markup = bc.env.render_block(block, bc.available, Some(bc.block)).await?;
        if !markup.is_empty() && matches!(block.name.as_deref(), Some("core/site-title" | "core/site-logo" | "core/social-links")) {
            markup = format!("<li class=\"wp-block-navigation-item\">{markup}</li>");
        }
        let is_list_item = tags(&markup, 0).iter().any(|t| !t.is_closer && t.name == "li");
        if is_list_item && !list_open {
            list_open = true;
            html.push_str(&format!("<ul {container}>"));
        }
        if !is_list_item && list_open {
            list_open = false;
            html.push_str("</ul>");
        }
        html.push_str(&markup);
    }
    if list_open {
        html.push_str("</ul>");
    }
    if submenus && interactive {
        html = add_directives_to_submenu(&html, &visibility);
    }

    let body = if responsive { responsive_container(bc, &html, interactive) } else { html };

    // The <nav> element.
    let name = unique_navigation_name(bc).await?;
    let mut extra: Vec<(&str, String)> = vec![("class", class), ("style", style)];
    if !name.is_empty() {
        extra.push(("aria-label", name));
    }
    let mut wrapper = bc.wrapper_attributes(&extra);
    if responsive && interactive {
        let context = json!({"overlayOpenedBy": {"click": false, "hover": false, "focus": false}, "type": "overlay", "roleAttribute": "", "ariaLabel": bc.env.site.tr("Menu")});
        wrapper.push_str(&format!(" \n\t\t data-wp-interactive=\"core/navigation\" data-wp-context='{}'", php_json::encode(&context)));
    }
    Ok(format!("<nav {wrapper}>{body}</nav>"))
}

async fn unique_navigation_name(bc: &BlockCtx<'_>) -> Result<String> {
    let mut name = str_attr(&bc.attrs, "ariaLabel").unwrap_or("").to_owned();
    if name.is_empty()
        && let Some(id) = bc.attrs.get("ref").and_then(Value::as_u64)
            && let Some(post) = bc.env.site.db.post(id).await?
                && post.post_status == "publish" {
                    name = post.post_title;
                }
    let count = bc.env.with_state(|s| {
        let c = s.seen_menu_names.entry(name.clone()).or_insert(0);
        *c += 1;
        *c
    });
    Ok(if count > 1 { format!("{name} {count}") } else { name })
}

/// The overlay markup for responsive navigation.
fn responsive_container(bc: &BlockCtx<'_>, inner_html: &str, interactive: bool) -> String {
    let attrs = &bc.attrs;
    let colors = build_colors(attrs);
    let id = bc.env.with_state(|s| s.unique_id("modal-"));
    let hidden_by_default = str_attr(attrs, "overlayMenu") == Some("always");

    let mut container_classes = vec!["wp-block-navigation__responsive-container".to_owned()];
    if hidden_by_default {
        container_classes.push("hidden-by-default".into());
    }
    container_classes.push(colors.overlay_css_classes.join(" "));
    let open_classes = ["wp-block-navigation__responsive-container-open", if hidden_by_default { "always-shown" } else { "" }];

    let has_icon = attrs.get("hasIcon").and_then(Value::as_bool).unwrap_or(false);
    let open_icon = "<svg width=\"24\" height=\"24\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" aria-hidden=\"true\" focusable=\"false\"><path d=\"M4 7.5h16v1.5H4z\"></path><path d=\"M4 15h16v1.5H4z\"></path></svg>";
    let open_icon = if str_attr(attrs, "icon") == Some("menu") {
        "<svg width=\"24\" height=\"24\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M5 5v1.5h14V5H5z\"></path><path d=\"M5 12.8h14v-1.5H5v1.5z\"></path><path d=\"M5 19h14v-1.5H5V19z\"></path></svg>"
    } else {
        open_icon
    };
    let close_icon = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\" aria-hidden=\"true\" focusable=\"false\"><path d=\"m13.06 12 6.47-6.47-1.06-1.06L12 10.94 5.53 4.47 4.47 5.53 10.94 12l-6.47 6.47 1.06 1.06L12 13.06l6.47 6.47 1.06-1.06L13.06 12Z\"></path></svg>";
    let site = bc.env.site;
    let open_content = if has_icon { open_icon.to_owned() } else { site.tr("Menu") };
    let close_content = if has_icon { close_icon.to_owned() } else { site.tr("Close") };
    let aria_open = if has_icon { format!("aria-label=\"{}\"", site.tr("Open menu")) } else { String::new() };
    let aria_close = if has_icon { format!("aria-label=\"{}\"", site.tr("Close menu")) } else { String::new() };

    let (open_directives, container_directives, dialog_directives, close_directives, content_directives) = if interactive {
        (
            "\n\t\t\t\tdata-wp-on--click=\"actions.openMenuOnClick\"\n\t\t\t\tdata-wp-on--keydown=\"actions.handleMenuKeydown\"\n\t\t\t",
            "\n\t\t\t\tdata-wp-class--has-modal-open=\"state.isMenuOpen\"\n\t\t\t\tdata-wp-class--is-menu-open=\"state.isMenuOpen\"\n\t\t\t\tdata-wp-watch=\"callbacks.initMenu\"\n\t\t\t\tdata-wp-on--keydown=\"actions.handleMenuKeydown\"\n\t\t\t\tdata-wp-on--focusout=\"actions.handleMenuFocusout\"\n\t\t\t\ttabindex=\"-1\"\n\t\t\t",
            "\n\t\t\t\tdata-wp-bind--aria-modal=\"state.ariaModal\"\n\t\t\t\tdata-wp-bind--aria-label=\"state.ariaLabel\"\n\t\t\t\tdata-wp-bind--role=\"state.roleAttribute\"\n\t\t\t",
            "\n\t\t\t\tdata-wp-on--click=\"actions.closeMenuOnClick\"\n\t\t\t",
            "\n\t\t\t\tdata-wp-watch=\"callbacks.focusFirstElement\"\n\t\t\t",
        )
    } else {
        ("", "", "", "", "")
    };

    let overlay_style = safecss_filter_attr(&colors.overlay_inline_styles);
    let overlay_style = if overlay_style.is_empty() { String::new() } else { format!("style=\"{}\"", esc_attr(&overlay_style)) };
    let close_button = format!("<button {aria_close} class=\"wp-block-navigation__responsive-container-close\" {close_directives}>{close_content}</button>");

    format!(
        "<button aria-haspopup=\"dialog\" {aria_open} class=\"{open}\" {open_directives}>{open_content}</button>\n\t\t\t\t<div class=\"{container}\" {overlay_style} id=\"{id}\" {container_directives}>\n\t\t\t\t\t<div class=\"wp-block-navigation__responsive-close\" tabindex=\"-1\">\n\t\t\t\t\t\t<div class=\"wp-block-navigation__responsive-dialog\" {dialog_directives}>\n\t\t\t\t\t\t\t{close_button}\n\t\t\t\t\t\t\t<div class=\"wp-block-navigation__responsive-container-content\" {content_directives} id=\"{id}-content\">\n\t\t\t\t\t\t\t\t{inner_html}\n\t\t\t\t\t\t\t\t\n\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t</div>\n\t\t\t\t</div>",
        open = esc_attr(open_classes.iter().filter(|c| !c.is_empty()).copied().collect::<Vec<_>>().join(" ").trim()),
        container = esc_attr(container_classes.join(" ").trim()),
    )
}

/// `block_core_navigation_add_directives_to_submenu`: interactivity attributes
/// on every `li.has-child`, the toggle button after it and its submenu container.
fn add_directives_to_submenu(html: &str, visibility: &str) -> String {
    fn has_class(tag: &crate::html::Tag, class: &str) -> bool {
        tag.attr("class").is_some_and(|c| c.split_whitespace().any(|x| x == class))
    }
    let mut out = html.to_owned();
    let mut index = 0;
    loop {
        let all = tags(&out, 0);
        let items: Vec<_> = all.iter().filter(|t| !t.is_closer && t.name == "li" && has_class(t, "has-child")).collect();
        let Some(li) = items.get(index).map(|t| (*t).clone()) else { break };
        index += 1;

        let mut editor = TagEditor::from_tag(&out, li.clone());
        editor.set_attribute("data-wp-interactive", "core/navigation");
        editor.set_attribute("data-wp-context", "{ \"submenuOpenedBy\": { \"click\": false, \"hover\": false, \"focus\": false }, \"type\": \"submenu\", \"modal\": null, \"previousFocus\": null }");
        editor.set_attribute("data-wp-watch", "callbacks.initMenu");
        editor.set_attribute("data-wp-on--focusout", "actions.handleMenuFocusout");
        editor.set_attribute("data-wp-on--keydown", "actions.handleMenuKeydown");
        editor.set_attribute("tabindex", "-1");
        if visibility == "hover" {
            editor.set_attribute("data-wp-on--pointerenter", "actions.openMenuOnHover");
            editor.set_attribute("data-wp-on--pointerleave", "actions.closeMenuOnHover");
        }
        out = editor.finish();

        // The toggle button and the submenu container that follow this item.
        let mut cursor = li.start;
        for (tag_name, class, attrs) in [
            ("button", "wp-block-navigation-submenu__toggle", &[("data-wp-on--click", "actions.toggleMenuOnClick"), ("data-wp-bind--aria-expanded", "state.isSubmenuOpen")][..]),
            ("ul", "wp-block-navigation__submenu-container", &[("data-wp-on--focus", "actions.openMenuOnFocus")][..]),
        ] {
            let found = tags(&out, cursor).into_iter().find(|t| !t.is_closer && t.name == tag_name && has_class(t, class));
            if let Some(tag) = found {
                cursor = tag.start;
                let mut editor = TagEditor::from_tag(&out, tag);
                for (name, value) in attrs {
                    editor.set_attribute(name, value);
                }
                out = editor.finish();
            }
        }
    }
    out
}
// ---------------------------------------------------------------- page list

pub(super) async fn page_list(bc: &BlockCtx<'_>) -> Result<String> {
    let parent_page_id = bc.attr_u64("parentPageID").unwrap_or(0);
    let is_nested = bc.context.get("core/isInsideSubmenu").is_some_and(|v| !crate::style_engine::is_empty(v));
    let pages = published_pages(bc).await?;
    if pages.is_empty() {
        return Ok(String::new());
    }
    let queried_id = bc.env.query.queried_object_id();
    let mut active_ancestors: Vec<u64> = Vec::new();
    #[derive(Clone)]
    struct Item {
        id: u64,
        title: String,
        link: String,
        active: bool,
        parent: u64,
    }
    let mut items: Vec<Item> = Vec::new();
    for page in &pages {
        let active = page.id != 0 && queried_id == page.id;
        if active {
            let mut parent = page.post_parent;
            while parent != 0 {
                active_ancestors.push(parent);
                parent = pages.iter().find(|p| p.id == parent).map_or(0, |p| p.post_parent);
            }
        }
        let link = bc.env.site.permalink(page).await?;
        let title = if page.post_title.is_empty() { bc.env.site.tr("(no title)") } else { page.post_title.clone() };
        items.push(Item { id: page.id, title, link, active, parent: page.post_parent });
    }

    let colors = page_list_colors(&bc.attrs, &bc.context.values);
    let is_navigation_child = bc.context.values.contains_key("showSubmenuIcon");
    let visibility = if is_navigation_child { submenu_visibility(&bc.context.values) } else { "hover".to_owned() };
    let show_icons = bc.context.get("showSubmenuIcon").is_some_and(|v| !crate::style_engine::is_empty(v));

    let roots: Vec<&Item> = if parent_page_id != 0 {
        if !items.iter().any(|i| i.parent == parent_page_id) {
            return Ok(String::new());
        }
        items.iter().filter(|i| i.parent == parent_page_id).collect()
    } else {
        items.iter().filter(|i| i.parent == 0).collect()
    };

    let front_page_id = bc.env.site.settings.page_on_front;
    #[allow(clippy::too_many_arguments)]
    fn render_level(
        pages: &[&Item],
        all: &[Item],
        ctx: &Level<'_>,
        depth: u32,
        front_page_id: u64,
    ) -> String {
        let mut markup = String::new();
        for page in pages {
            let mut css = if page.active { " current-menu-item".to_owned() } else { String::new() };
            let aria_current = if page.active { " aria-current=\"page\"" } else { "" };
            let mut style = String::new();
            if ctx.ancestors.contains(&page.id) {
                css.push_str(" current-menu-ancestor");
            }
            let children: Vec<&Item> = all.iter().filter(|c| c.parent == page.id).collect();
            if !children.is_empty() {
                css.push_str(" has-child");
            }
            if ctx.is_navigation_child {
                css.push_str(" wp-block-navigation-item");
                if ctx.visibility == "click" {
                    css.push_str(" open-on-click");
                } else if ctx.visibility == "hover" && ctx.show_icons {
                    css.push_str(" open-on-hover-click");
                } else if ctx.visibility == "always" {
                    css.push_str(" open-always");
                }
            }
            let content_class = if ctx.is_navigation_child { " wp-block-navigation-item__content" } else { "" };
            if ((depth > 0 && !ctx.is_nested) || ctx.is_nested) && true {
                css.push(' ');
                css.push_str(ctx.overlay_classes.trim());
                if !ctx.overlay_style.is_empty() {
                    style = format!(" style=\"{}\"", esc_attr(&ctx.overlay_style));
                }
            }
            if page.id == front_page_id {
                css.push_str(" menu-item-home");
            }
            let title = page.title.clone();
            let aria_label = rustpress_core::i18n::sprintf(&ctx.submenu_label, &[&rustpress_core::formatting::esc_html(&strip_tags(&title)).replace("&amp;", "&")]);
            markup.push_str(&format!("<li class=\"wp-block-pages-list__item{}\"{style}>", esc_attr(&css)));
            if !children.is_empty() && ctx.is_navigation_child && ctx.visibility == "click" {
                markup.push_str(&format!(
                    "<button aria-label=\"{}\" class=\"{} wp-block-navigation-submenu__toggle\" aria-expanded=\"false\">{title}</button><span class=\"wp-block-page-list__submenu-icon wp-block-navigation__submenu-icon\">{SUBMENU_ICON}</span>",
                    esc_attr(&aria_label),
                    esc_attr(content_class)
                ));
            } else {
                markup.push_str(&format!(
                    "<a class=\"wp-block-pages-list__item__link{}\" href=\"{}\"{aria_current}>{title}</a>",
                    esc_attr(content_class),
                    esc_url(&page.link)
                ));
            }
            if !children.is_empty() {
                if ctx.is_navigation_child && ctx.show_icons && ctx.visibility != "click" {
                    markup.push_str(&format!("<button aria-label=\"{}\" class=\"wp-block-navigation__submenu-icon wp-block-navigation-submenu__toggle\" aria-expanded=\"false\">{SUBMENU_ICON}</button>", esc_attr(&aria_label)));
                }
                markup.push_str("<ul class=\"wp-block-navigation__submenu-container\">");
                markup.push_str(&render_level(&children, all, ctx, depth + 1, front_page_id));
                markup.push_str("</ul>");
            }
            markup.push_str("</li>");
        }
        markup
    }
    struct Level<'a> {
        is_navigation_child: bool,
        visibility: &'a str,
        show_icons: bool,
        is_nested: bool,
        ancestors: &'a [u64],
        overlay_classes: String,
        overlay_style: String,
        submenu_label: String,
    }
    let level = Level {
        submenu_label: bc.env.site.tr("%s submenu"),
        is_navigation_child,
        visibility: &visibility,
        show_icons,
        is_nested,
        ancestors: &active_ancestors,
        overlay_classes: colors.overlay_css_classes.join(" "),
        overlay_style: colors.overlay_inline_styles.clone(),
    };
    let items_markup = render_level(&roots, &items, &level, 0, front_page_id);
    if is_nested {
        return Ok(items_markup);
    }
    let class = colors.css_classes.join(" ");
    let wrapper = bc.wrapper_attributes(&[("class", class), ("style", colors.inline_styles.clone())]);
    Ok(format!("<ul {wrapper}>{items_markup}</ul>"))
}

fn strip_tags(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// `block_core_page_list_build_css_colors`.
fn page_list_colors(_attrs: &Map<String, Value>, context: &Map<String, Value>) -> Colors {
    let mut c = Colors { css_classes: vec![], inline_styles: String::new(), overlay_css_classes: vec![], overlay_inline_styles: String::new() };
    let has = |k: &str| context.contains_key(k);
    let val = |k: &str| context.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    let style_text = context.get("style").and_then(|s| s.get("color")).and_then(|c| c.get("text")).and_then(Value::as_str).map(str::to_owned);
    let style_bg = context.get("style").and_then(|s| s.get("color")).and_then(|c| c.get("background")).and_then(Value::as_str).map(str::to_owned);

    if style_text.is_some() || has("customTextColor") || has("textColor") {
        c.css_classes.push("has-text-color".into());
    }
    if has("textColor") {
        c.css_classes.push(format!("has-{}-color", to_kebab_case(&val("textColor"))));
    } else if has("customTextColor") {
        c.inline_styles.push_str(&format!("color: {};", val("customTextColor")));
    } else if let Some(t) = style_text {
        c.inline_styles.push_str(&format!("color: {t};"));
    }
    if style_bg.is_some() || has("customBackgroundColor") || has("backgroundColor") {
        c.css_classes.push("has-background".into());
    }
    if has("backgroundColor") {
        c.css_classes.push(format!("has-{}-background-color", to_kebab_case(&val("backgroundColor"))));
    } else if has("customBackgroundColor") {
        c.inline_styles.push_str(&format!("background-color: {};", val("customBackgroundColor")));
    } else if let Some(b) = style_bg {
        c.inline_styles.push_str(&format!("background-color: {b};"));
    }
    if has("overlayTextColor") || has("customOverlayTextColor") {
        c.overlay_css_classes.push("has-text-color".into());
    }
    if has("overlayTextColor") {
        c.overlay_css_classes.push(format!("has-{}-color", to_kebab_case(&val("overlayTextColor"))));
    } else if has("customOverlayTextColor") {
        c.overlay_inline_styles.push_str(&format!("color: {};", val("customOverlayTextColor")));
    }
    if has("overlayBackgroundColor") || has("customOverlayBackgroundColor") {
        c.overlay_css_classes.push("has-background".into());
    }
    if has("overlayBackgroundColor") {
        c.overlay_css_classes.push(format!("has-{}-background-color", to_kebab_case(&val("overlayBackgroundColor"))));
    } else if has("customOverlayBackgroundColor") {
        c.overlay_inline_styles.push_str(&format!("background-color: {};", val("customOverlayBackgroundColor")));
    }
    c
}

// ---------------------------------------------------------- navigation link

pub(super) async fn navigation_link(bc: &BlockCtx<'_>) -> Result<String> {
    let attrs = &bc.attrs;
    // `block_core_shared_navigation_item_should_render`
    let has_id = attrs.get("id").is_some_and(|v| v.is_number());
    let is_post_type = str_attr(attrs, "kind") == Some("post-type") || matches!(str_attr(attrs, "type"), Some("post" | "page"));
    if is_post_type && has_id {
        let id = attrs.get("id").and_then(Value::as_u64).unwrap_or(0);
        let published = bc.env.site.db.post(id).await?.is_some_and(|p| p.post_status == "publish");
        if !published {
            return Ok(String::new());
        }
    }
    let label = str_attr(attrs, "label").unwrap_or("");
    if label.is_empty() {
        return Ok(String::new());
    }

    let mut inner_html = String::new();
    for inner in &bc.block.inner_blocks {
        inner_html.push_str(&bc.env.render_block(inner, bc.available, Some(bc.block)).await?);
    }
    let has_submenu = !inner_html.trim().is_empty();
    let queried_id = bc.env.query.queried_object_id();
    let is_active = attrs.get("id").and_then(Value::as_u64).is_some_and(|id| id != 0 && id == queried_id);

    let class = format!(" wp-block-navigation-item{}{}", if has_submenu { " has-child" } else { "" }, if is_active { " current-menu-item" } else { "" });
    let wrapper = bc.wrapper_attributes(&[("class", class.trim().to_owned())]);
    let mut html = format!("<li {wrapper}><a class=\"wp-block-navigation-item__content\" ");
    if let Some(url) = str_attr(attrs, "url") {
        html.push_str(&format!(" href=\"{}\"", esc_url(url)));
    }
    if is_active {
        html.push_str(" aria-current=\"page\"");
    }
    if attrs.get("opensInNewTab").and_then(Value::as_bool) == Some(true) {
        html.push_str(" target=\"_blank\"  ");
    }
    if let Some(rel) = str_attr(attrs, "rel") {
        html.push_str(&format!(" rel=\"{}\"", esc_attr(rel)));
    } else if attrs.get("nofollow").is_some_and(|v| !crate::style_engine::is_empty(v)) {
        html.push_str(" rel=\"nofollow\"");
    }
    if let Some(title) = str_attr(attrs, "title") {
        html.push_str(&format!(" title=\"{}\"", esc_attr(title)));
    }
    html.push_str(&format!(">{}{}</a>", format_args!("<span class=\"wp-block-navigation-item__label\">{}</span>", kses_post(&label)), description(attrs)));
    if bc.context.get("showSubmenuIcon").is_some_and(|v| !crate::style_engine::is_empty(v)) && has_submenu {
        html.push_str(&format!("<span class=\"wp-block-navigation__submenu-icon\">{SUBMENU_ICON}</span>"));
    }
    if has_submenu {
        html.push_str(&format!("<ul class=\"wp-block-navigation__submenu-container\">{inner_html}</ul>"));
    }
    html.push_str("</li>");
    Ok(html)
}

fn description(attrs: &Map<String, Value>) -> String {
    match str_attr(attrs, "description").filter(|d| !d.is_empty()) {
        Some(d) => format!("<span class=\"wp-block-navigation-item__description\">{}</span>", kses_post(d)),
        None => String::new(),
    }
}
