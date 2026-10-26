//! `GET wp-block-editor/v1/export`: the theme as a ZIP file with the templates, the template parts and the
//! `theme.json` as the site editor has them (`wp_generate_block_templates_export_file()`).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use rustpress_blocks::parser::{Block, parse_blocks};
use rustpress_blocks::serializer::serialize_with;
use serde_json::{Map, Value};

use super::templates::TKind;
use crate::error::RestError;
use crate::{Outcome, Resp, Rest, Result};

/// `wp_is_theme_directory_ignored()`.
fn ignored(relative: &str) -> bool {
    [".DS_Store", ".svn", ".git", ".hg", ".bzr", "node_modules", "vendor"].iter().any(|d| relative.starts_with(d))
}

fn collect(dir: &Path, base: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, base, out);
        } else if let (Ok(relative), Ok(bytes)) = (path.strip_prefix(base), std::fs::read(&path)) {
            let relative = relative.to_string_lossy().replace('\\', "/");
            if !ignored(&relative) {
                out.insert(relative, bytes);
            }
        }
    }
}

/// `WP_Theme_JSON::merge()` as far as the exported file goes: what the user chose replaces what the theme says.
fn merge(base: &mut Value, user: &Value) {
    match (base, user) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in b {
                match a.get_mut(key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        a.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (slot, value) => *slot = value.clone(),
    }
}

/// The keys of every object in alphabetical order, the order of the lists stays.
fn sort_keys(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = std::mem::take(map).into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (_, v) in &mut entries {
                sort_keys(v);
            }
            *map = entries.into_iter().collect();
        }
        Value::Array(items) => items.iter_mut().for_each(sort_keys),
        _ => {}
    }
}

/// `wp_json_encode( $data, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES )` with the four spaces turned into tabs.
fn pretty(value: &Value) -> String {
    use serde::Serialize;
    let mut buffer = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
    if value.serialize(&mut serializer).is_err() {
        return String::new();
    }
    let text = String::from_utf8(buffer).unwrap_or_default();
    // PHP writes everything outside ASCII as `\uXXXX`.
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

impl Rest<'_> {
    /// The ZIP of the theme for the site editor's "Export" menu.
    pub(crate) async fn export_theme(&self) -> Result<Outcome> {
        if !self.can("export") {
            return Ok(Err(self.forbidden("rest_cannot_export_templates", "Sorry, you are not allowed to export templates and template parts.")));
        }
        let stylesheet = self.theme.stylesheet.clone();
        let dir = self.theme.stylesheet_dir().to_path_buf();
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        collect(&dir, &dir, &mut files);
        // The templates and template parts as the editor has them (customized ones win over the files).
        let theme_attribute = |markup: &str| -> String {
            let mut blocks = parse_blocks(markup);
            serialize_with(&mut blocks, &|block: &mut Block| {
                if block.name.as_deref() == Some("core/template-part") && block.attrs.get("theme").and_then(Value::as_str) == Some(stylesheet.as_str()) {
                    block.attrs.remove("theme");
                }
            })
        };
        for tpl in self.block_templates(TKind::Template, None, None, None, &[]).await? {
            files.insert(format!("templates/{}.html", tpl.slug), theme_attribute(&tpl.content).into_bytes());
        }
        for tpl in self.block_templates(TKind::Part, None, None, None, &[]).await? {
            files.insert(format!("parts/{}.html", tpl.slug), tpl.content.into_bytes());
        }
        // theme.json with the styles of the user.
        // The theme(s) with the block style variations of `styles/blocks`, as `WP_Theme_JSON_Resolver::get_theme_data()` has it.
        let mut data: Value = self.theme.json.raw_theme.clone();
        if !data.is_object() {
            data = Value::Object(Map::new());
        }
        if let Some(styles) = data.get_mut("styles") {
            rustpress_theme::theme_json::resolve_custom_vars(styles);
        }
        let query = rustpress_db::PostQuery { post_types: vec!["wp_global_styles".into()], statuses: vec!["publish".into(), "draft".into()], names: vec![format!("wp-global-styles-{}", rustpress_core::slug::sanitize_title(&stylesheet))], limit: Some(1), ..Default::default() };
        if let Some(post) = self.site.db.posts(&query).await.ok().and_then(|posts| posts.into_iter().next())
            && let Ok(user) = serde_json::from_str::<Value>(&post.post_content)
            && let Value::Object(user) = user
        {
            for key in ["settings", "styles"] {
                if let Some(value) = user.get(key) {
                    let target = data.as_object_mut().map(|o| o.entry(key.to_owned()).or_insert_with(|| Value::Object(Map::new())));
                    if let Some(target) = target {
                        merge(target, value);
                    }
                }
            }
        }
        if let Value::Object(map) = &mut data
            && let Some(version) = map.get("version").and_then(Value::as_u64) {
                let schema = if version >= 3 { format!("https://schemas.wp.org/wp/{}/theme.json", self.wp_major_minor()) } else { "https://schemas.wp.org/trunk/theme.json".to_owned() };
                map.insert("$schema".into(), Value::String(schema));
            }
        sort_keys(&mut data);
        files.insert("theme.json".into(), pretty(&data).into_bytes());

        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            let failed = |_: zip::result::ZipError| RestError::new("rest_export_failed", "Zip Export not supported.", 500);
            for directory in ["templates/", "parts/"] {
                if let Err(e) = zip.add_directory(directory, options) {
                    return Ok(Err(failed(e)));
                }
            }
            for (name, bytes) in &files {
                if let Err(e) = zip.start_file(name, options) {
                    return Ok(Err(failed(e)));
                }
                if zip.write_all(bytes).is_err() {
                    return Ok(Err(RestError::new("rest_export_failed", "Zip Export not supported.", 500)));
                }
            }
            if let Err(e) = zip.finish() {
                return Ok(Err(failed(e)));
            }
        }
        let mut resp = Resp::item(Value::Null);
        resp.download = Some((buffer.into_inner(), format!("{stylesheet}.zip")));
        Ok(Ok(resp))
    }

    /// `7.1` of `7.1.3`.
    fn wp_major_minor(&self) -> String {
        let version = self.wp_version();
        version.split('.').take(2).collect::<Vec<_>>().join(".")
    }
}
