//! `wp-admin/site-editor.php`: the site editor of block themes (templates, template parts, patterns, styles,
//! navigation). The editor itself is WordPress' JavaScript; this module starts it with the same data.

use rustpress_core::formatting::esc_html;
use rustpress_core::urls::{add_query_arg, remove_query_arg};
use serde_json::{Value, json};

use super::block_editor::{inline_json, preload_paths, shared_inline};
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Screen};

/// `_wp_get_site_editor_redirection_url()`: old query arguments are turned into the `p` of the router.
fn redirection(ctx: &Ctx<'_>) -> Option<String> {
    let request = ctx.request;
    let query_string = request.request_uri.split_once('?').map_or("", |(_, q)| q);
    if request.get("p").is_some() || query_string.is_empty() {
        return None;
    }
    let url = ctx.admin_url(&format!("site-editor.php?{query_string}"));
    let get = |name: &str| request.get(name).filter(|v| !v.is_empty());
    let post_type = get("postType");
    let post_id = get("postId");
    let canvas = get("canvas");
    let path = get("path");
    let with_p = |p: &str, drop: &[&str]| {
        let mut out = url.clone();
        for name in drop {
            out = remove_query_arg(&out, name);
        }
        add_query_arg(&out, "p", p)
    };
    let result = match (post_type, post_id, path) {
        (Some("wp_navigation"), Some(id), _) => with_p(&format!("/wp_navigation/{id}"), &["postType", "postId"]),
        (Some("wp_navigation"), None, _) => with_p("/navigation", &["postType"]),
        (_, _, Some("/wp_global_styles")) => with_p("/styles", &["path"]),
        (Some("page"), id, _) if canvas.is_none() || id.is_none() => with_p("/page", &["postType"]),
        (Some("page"), Some(id), _) => with_p(&format!("/page/{id}"), &["postType", "postId"]),
        (Some("wp_template"), id, _) if canvas.is_none() || id.is_none() => with_p("/template", &["postType"]),
        (Some("wp_template"), Some(id), _) => with_p(&format!("/wp_template/{id}"), &["postType", "postId"]),
        (Some("wp_block"), id, _) if canvas.is_none() || id.is_none() => with_p("/pattern", &["postType"]),
        (Some("wp_block"), Some(id), _) => with_p(&format!("/wp_block/{id}"), &["postType", "postId"]),
        (Some("wp_template_part"), id, _) if canvas.is_none() || id.is_none() => add_query_arg(&url, "p", "/pattern"),
        (Some("wp_template_part"), Some(id), _) => with_p(&format!("/wp_template_part/{id}"), &["postType", "postId"]),
        (_, _, Some("/wp_template_part/all")) => add_query_arg(&add_query_arg(&remove_query_arg(&url, "path"), "p", "/pattern"), "postType", "wp_template_part"),
        (_, _, Some("/page")) => with_p("/page", &["path"]),
        (_, _, Some("/wp_template")) => with_p("/template", &["path"]),
        (_, _, Some("/patterns")) => with_p("/pattern", &["path"]),
        (_, _, Some("/navigation")) => with_p("/navigation", &["path"]),
        _ => add_query_arg(&url, "p", "/"),
    };
    Some(result)
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let env = ctx.env;
    if !ctx.can("edit_theme_options") {
        return die(env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("You need a higher level of permission."), tr("Sorry, you are not allowed to edit theme options on this site.")));
    }
    if let Some(url) = redirection(ctx) {
        return redirect(&url);
    }
    let stylesheet = site.settings.stylesheet.clone();
    let settings = super::block_editor::editor_settings(ctx, &rustpress_db::Post::default(), &super::posts::TypeInfo { name: String::new(), hierarchical: false, label: String::new(), singular: String::new() }).await;
    let gs_id = super::block_editor::global_styles_id(ctx).await;

    let mut paths: Vec<(String, String)> = vec![
        ("OPTIONS".into(), "/wp/v2/media".into()),
        ("OPTIONS".into(), "/wp/v2/pages".into()),
        ("GET".into(), "/wp/v2/types?context=view".into()),
        ("GET".into(), "/wp/v2/types/wp_template?context=edit".into()),
        ("GET".into(), "/wp/v2/types/wp_template_part?context=edit".into()),
        ("GET".into(), "/wp/v2/templates?context=edit&per_page=-1".into()),
        ("GET".into(), "/wp/v2/template-parts?context=edit&per_page=-1".into()),
        ("GET".into(), "/wp/v2/themes?context=edit&status=active".into()),
    ];
    if let Some(id) = gs_id {
        paths.push(("GET".into(), format!("/wp/v2/global-styles/{id}?context=edit")));
        paths.push(("OPTIONS".into(), format!("/wp/v2/global-styles/{id}")));
    }
    paths.push(("GET".into(), format!("/wp/v2/global-styles/themes/{stylesheet}?context=view")));
    paths.push(("GET".into(), format!("/wp/v2/global-styles/themes/{stylesheet}/variations?context=view")));
    paths.push(("OPTIONS".into(), "/wp/v2/navigation".into()));
    paths.push(("GET".into(), "/wp/v2/navigation?context=edit&per_page=100&order=desc&orderby=date&status[0]=publish&status[1]=draft".into()));
    paths.push(("GET".into(), "/wp/v2/settings".into()));
    paths.push(("OPTIONS".into(), "/wp/v2/settings".into()));
    paths.push(("GET".into(), "/wp/v2/block-patterns/categories".into()));
    paths.push(("GET".into(), "/?_fields=description,gmt_offset,home,image_max_bit_depth,image_sizes,image_size_threshold,image_strip_meta,name,site_icon,site_icon_url,site_logo,timezone_string,url,page_for_posts,page_on_front,show_on_front".into()));
    // A page being edited: its route and its template; otherwise the templates of the front page.
    let page_id: Option<u64> = ctx.request.get("p").and_then(|p| p.strip_prefix("/page/")).and_then(|id| id.parse().ok()).or_else(|| ctx.request.get("postId").and_then(|v| v.parse().ok()));
    let page = match page_id {
        Some(id) => env.db.post(id).await.ok().flatten(),
        None => None,
    };
    match &page {
        Some(page) => {
            paths.push(("GET".into(), format!("/wp/v2/{}/{}?context=edit", if page.post_type == "page" { "pages".to_owned() } else { format!("{}s", page.post_type) }, page.id)));
            if page.post_type == "page" {
                paths.push(("GET".into(), format!("/wp/v2/templates/lookup?slug={}", if page.post_name.is_empty() { "page".to_owned() } else { format!("page-{}", page.post_name) })));
            }
        }
        None => {
            paths.push(("GET".into(), "/wp/v2/templates/lookup?slug=front-page".into()));
            paths.push(("GET".into(), "/wp/v2/templates/lookup?slug=home".into()));
        }
    }
    let preloaded = preload_paths(ctx, paths).await;

    let mut assets = Assets {
        editor_modules: true,
        scripts: ["wp-edit-site", "wp-format-library"].iter().map(|s| (*s).to_owned()).collect(),
        styles: ["wp-edit-site", "wp-format-library", "wp-edit-blocks", "wp-commands"].iter().map(|s| (*s).to_owned()).collect(),
        ..Default::default()
    };
    super::media_ui::enqueue(ctx, &mut assets, 0).await;
    assets.styles.push("wp-block-directory".into());
    super::block_editor::editor_head_styles(ctx, &mut assets);
    assets.early_scripts = std::mem::take(&mut assets.scripts);
    assets.early_scripts.push("wp-block-styles".into());
    assets.early_scripts.push("wp-block-directory".into());
    assets.inline.push(("wp-block-styles".into(), super::block_editor::block_styles_script(ctx), "after".into()));
    shared_inline(ctx, &mut assets, &preloaded).await;
    // The classic editor's TinyMCE settings are not needed by the site editor.
    assets.inline.retain(|(handle, _, position)| !(handle == "wp-block-library" && position == "before"));
    assets.inline.push(("wp-edit-site".into(), format!("wp.domReady( function() {{\n\t\t\twp.editSite.initializeEditor( \"site-editor\", {} );\n\t\t}} );", inline_json(&settings)), "after".into()));

    let content = format!(
        "\n<div class=\"edit-site\" id=\"site-editor\">\n\t\t<div class=\"wrap hide-if-js site-editor-no-js\">\n\t\t<h1 class=\"wp-heading-inline\">{}</h1>\n\t\t<div class=\"notice notice-error hide-if-js\"><p>{}</p></div>\t</div>\n</div>\n",
        esc_html(&tr("Edit Site")),
        tr("The site editor requires JavaScript. Please enable JavaScript in your browser settings.")
    );
    let screen = Screen {
        file: "site-editor.php".into(),
        parent_file: "themes.php".into(),
        submenu_file: Some("site-editor.php".into()),
        title: ctx.site().trx("Editor", "site editor title tag"),
        help: Some(crate::chrome::Help::default()),
        fullscreen: true,
        body_classes: vec!["block-editor-page".into(), "wp-embed-responsive".into()],
        ..Default::default()
    };
    let _ = json!(Value::Null);
    render(ctx, &screen, &content, &assets).await
}
