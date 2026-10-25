//! Screens whose markup is baked from the reference WordPress (`dev/bake-pages.py`): the content
//! and the help tabs come from `data/pages/<screen>.json`, the text is translated node by node.

use serde_json::Value;

use super::{AdminResponse, Ctx, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};

/// The baked pieces of a screen.
pub struct Baked {
    pub help: Help,
    pub content: String,
}

fn source(name: &str) -> &'static str {
    match name {
        "tools" => include_str!("../data/pages/tools.json"),
        "import" => include_str!("../data/pages/import.json"),
        "export" => include_str!("../data/pages/export.json"),
        "options-privacy" => include_str!("../data/pages/options-privacy.json"),
        "privacy-policy-guide" => include_str!("../data/pages/privacy-policy-guide.json"),
        "options-permalink" => include_str!("../data/pages/options-permalink.json"),
        "plugins" => include_str!("../data/pages/plugins.json"),
        "themes" => include_str!("../data/pages/themes.json"),
        "site-health" => include_str!("../data/pages/site-health.json"),
        "attachment-edit" => include_str!("../data/pages/attachment-edit.json"),
        "plugin-install-upload" => include_str!("../data/pages/plugin-install-upload.json"),
        "theme-install-upload" => include_str!("../data/pages/theme-install-upload.json"),
        "about" => include_str!("../data/pages/about.json"),
        "credits" => include_str!("../data/pages/credits.json"),
        "freedoms" => include_str!("../data/pages/freedoms.json"),
        "contribute" => include_str!("../data/pages/contribute.json"),
        "update-core" => include_str!("../data/pages/update-core.json"),
        _ => "{}",
    }
}

/// Translates the text nodes of `html` (the message IDs are the English texts WordPress uses) and the
/// parts between `@@` markers, which may contain markup. Scripts and styles stay as they are.
pub fn translate_html(ctx: &Ctx<'_>, html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('<') {
            // A tag; raw text elements run up to their end tag.
            let end = after.find('>').map_or(rest.len(), |i| i + 2);
            let tag = &rest[..end];
            out.push_str(tag);
            rest = &rest[end..];
            let lower = tag.to_ascii_lowercase();
            // Underscore templates are translated, code is not.
            let template = lower.contains("text/html") || lower.contains("text/template");
            for raw in ["script", "style", "textarea"] {
                if !template && lower.starts_with(&format!("<{raw}")) && !lower.ends_with("/>") {
                    let close = format!("</{raw}");
                    let at = rest.to_ascii_lowercase().find(&close).unwrap_or(rest.len());
                    out.push_str(&rest[..at]);
                    rest = &rest[at..];
                    break;
                }
            }
            continue;
        }
        let end = rest.find('<').unwrap_or(rest.len());
        let text = &rest[..end];
        rest = &rest[end..];
        out.push_str(&translate_text(ctx, text));
    }
    translate_marked(ctx, &out)
}

/// One text node: the whitespace around stays, the words between are looked up.
fn translate_text(ctx: &Ctx<'_>, text: &str) -> String {
    let core = text.trim();
    if core.is_empty() || !core.chars().any(char::is_alphabetic) || core.starts_with("{{") {
        return text.to_owned();
    }
    let start = text.len() - text.trim_start().len();
    let end = text.trim_end().len();
    format!("{}{}{}", &text[..start], ctx.tr(core), &text[end..])
}

/// `@@Text with <b>markup</b>@@`.
fn translate_marked(ctx: &Ctx<'_>, text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("@@") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("@@") else { break };
        out.push_str(&ctx.tr(&after[..end]));
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// A paragraph of the help: `<p>...</p>` pieces are translated as a whole.
fn translate_help(ctx: &Ctx<'_>, html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find("<p>") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 3..];
        let Some(end) = after.find("</p>") else { break };
        out.push_str("<p>");
        out.push_str(&ctx.tr(&after[..end]));
        out.push_str("</p>");
        rest = &after[end + 4..];
    }
    out.push_str(rest);
    out
}

/// `<p><strong>For more information:</strong></p>` and the links: each paragraph is one message.
fn translate_sidebar(ctx: &Ctx<'_>, html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find("<p>") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 3..];
        let Some(end) = after.find("</p>") else { break };
        let inner = &after[..end];
        out.push_str("<p>");
        match inner.strip_prefix("<strong>").and_then(|i| i.strip_suffix("</strong>")) {
            Some(label) => out.push_str(&format!("<strong>{}</strong>", ctx.tr(label))),
            None => out.push_str(&ctx.tr(inner)),
        }
        out.push_str("</p>");
        rest = &after[end + 4..];
    }
    out.push_str(rest);
    out
}

/// The help and the (translated) content of a baked screen; `{{site}}` is the address of the site.
pub fn load(ctx: &Ctx<'_>, name: &str) -> Baked {
    let data: Value = serde_json::from_str(source(name)).unwrap_or(Value::Null);
    let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    let tabs = data["help"]["tabs"]
        .as_array()
        .map(|tabs| tabs.iter().map(|t| HelpTab { id: text(&t["id"]), title: ctx.tr(&text(&t["title"])), content: translate_help(ctx, &text(&t["content"])) }).collect())
        .unwrap_or_default();
    let help = Help { tabs, sidebar: translate_sidebar(ctx, &text(&data["help"]["sidebar"])) };
    let site = ctx.site().settings.siteurl.trim_end_matches('/').to_owned();
    let mut content = translate_html(ctx, &text(&data["content"])).replace("{{site}}", &site);
    // `{{nonce:action}}`
    while let Some(start) = content.find("{{nonce:") {
        let Some(end) = content[start..].find("}}") else { break };
        let action = content[start + 8..start + end].to_owned();
        content.replace_range(start..start + end + 2, &ctx.create_nonce(&action));
    }
    Baked { help, content }
}

/// A screen that is only its baked content.
pub async fn simple(ctx: &Ctx<'_>, file: &str, name: &str, title: &str, parent: &str, assets: Assets) -> AdminResponse {
    let baked = load(ctx, name);
    let screen = Screen { file: file.to_owned(), parent_file: parent.to_owned(), submenu_file: Some(file.to_owned()), title: ctx.tr(title), help: Some(baked.help), ..Default::default() };
    render(ctx, &screen, &format!("{}\n", baked.content.trim_end_matches('\n')), &assets).await
}

/// `tools.php`.
pub async fn tools(ctx: &Ctx<'_>) -> AdminResponse {
    if ctx.can("import") {
        return simple(ctx, "tools.php", "tools", "Tools", "tools.php", Assets::default()).await;
    }
    // The converter of categories and tags belongs to those who may import.
    let baked = load(ctx, "tools");
    let content = match baked.content.find("\t\t<div class=\"card\">") {
        Some(start) => format!("{}</div>\n", &baked.content[..start]),
        None => baked.content.clone(),
    };
    let screen = Screen { file: "tools.php".to_owned(), parent_file: "tools.php".to_owned(), submenu_file: Some("tools.php".to_owned()), title: ctx.tr("Tools"), help: Some(baked.help), ..Default::default() };
    render(ctx, &screen, &format!("{}\n", content.trim_end_matches('\n')), &Assets::default()).await
}

/// `import.php`.
pub async fn import(ctx: &Ctx<'_>) -> AdminResponse {
    simple(ctx, "import.php", "import", "Import", "tools.php", Assets { scripts: vec!["thickbox".into(), "plugin-install".into(), "updates".into()], styles: vec!["thickbox".into()], ..Default::default() }).await
}

/// `about.php`, `credits.php`, `freedoms.php` and `contribute.php`: the pages about the release.
pub async fn about(ctx: &Ctx<'_>, file: &str) -> AdminResponse {
    let (name, title) = match file {
        "credits.php" => ("credits", "Credits"),
        "freedoms.php" => ("freedoms", "Freedoms"),
        "contribute.php" => ("contribute", "Get Involved"),
        _ => ("about", "About"),
    };
    simple(ctx, file, name, title, file, Assets::default()).await
}
