//! The image editor of the media screens: `image-editor` and `imgedit-preview` of `admin-ajax.php`
//! (`wp-admin/includes/image-edit.php`). Rotating, flipping, cropping and scaling work on the current file
//! of the attachment; the result is a new file next to it, the original stays and can be restored.

use std::path::PathBuf;

use image::DynamicImage;
use rustpress_auth::CurrentUser;
use rustpress_phpser::{Key, Value};
use serde_json::{Value as Json, json};

use super::ajax::{json, text};
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};

const EDITOR: &str = include_str!("../data/image_editor.tpl");
const RESTORE: &str = include_str!("../data/image_editor_restore.tpl");

fn key(name: &str) -> Key {
    Key::Str(name.as_bytes().to_vec())
}

fn string(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn number(value: Option<&Value>) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(0)
}

/// Sets an entry of a PHP array, keeping its place when it exists.
fn set(map: &mut Value, name: &str, value: Value) {
    if let Value::Array(items) = map {
        match items.iter_mut().find(|(k, _)| *k == key(name)) {
            Some(entry) => entry.1 = value,
            None => items.push((key(name), value)),
        }
    }
}

fn remove(map: &mut Value, name: &str) {
    if let Value::Array(items) = map {
        items.retain(|(k, _)| *k != key(name));
    }
}

fn parse(raw: Option<String>) -> Option<Value> {
    raw.and_then(|r| rustpress_phpser::unserialize(r.as_bytes()).ok()).filter(|v| matches!(v, Value::Array(_)))
}

fn serialize(value: &Value) -> String {
    String::from_utf8_lossy(&rustpress_phpser::serialize(value)).into_owned()
}

fn text_entry(name: &str, value: &str) -> (Key, Value) {
    (key(name), Value::Str(value.as_bytes().to_vec()))
}

/// What the editor knows about an attachment.
struct State {
    id: u64,
    mime: String,
    uploads: PathBuf,
    /// `_wp_attached_file`: the path below the uploads directory.
    relative: String,
    meta: Option<Value>,
    backup: Option<Value>,
}

impl State {
    fn path(&self) -> PathBuf {
        self.uploads.join(&self.relative)
    }
}

async fn load(env: &AdminEnv<'_>, id: u64) -> Option<State> {
    let post = env.db.post(id).await.ok().flatten().filter(|p| p.post_type == "attachment")?;
    let relative = env.db.post_meta_value(id, "_wp_attached_file").await.ok().flatten().unwrap_or_default();
    let meta = parse(env.db.post_meta_value(id, "_wp_attachment_metadata").await.ok().flatten());
    let backup = parse(env.db.post_meta_value(id, "_wp_attachment_backup_sizes").await.ok().flatten());
    Some(State { id, mime: post.post_mime_type, uploads: super::ajax_media::uploads_dir(env), relative, meta, backup })
}

/// `_image_get_preview_ratio()`.
fn preview_ratio(width: u32, height: u32) -> f64 {
    let max = width.max(height);
    if max > 600 { 600.0 / f64::from(max) } else { 1.0 }
}

#[derive(Debug, Clone)]
enum Op {
    Rotate(i64),
    Flip(i64),
    Crop { x: f64, y: f64, w: f64, h: f64, scale: Option<f64> },
}

fn operations(history: &str) -> Vec<Op> {
    let Ok(Json::Array(items)) = serde_json::from_str::<Json>(history) else { return Vec::new() };
    let mut out: Vec<Op> = Vec::new();
    for item in items {
        let op = if let Some(angle) = item.get("r").and_then(Json::as_i64) {
            Op::Rotate(angle)
        } else if let Some(axis) = item.get("f").and_then(Json::as_i64) {
            Op::Flip(axis)
        } else if let Some(sel) = item.get("c") {
            let f = |name: &str| sel.get(name).and_then(Json::as_f64).unwrap_or(0.0);
            Op::Crop { x: f("x"), y: f("y"), w: f("w"), h: f("h"), scale: sel.get("r").and_then(Json::as_f64) }
        } else {
            continue;
        };
        // Neighbours of the same kind are combined, as `image_edit_apply_changes()` does.
        match (out.last_mut(), &op) {
            (Some(Op::Rotate(a)), Op::Rotate(b)) => *a += b,
            (Some(Op::Flip(a)), Op::Flip(b)) => *a ^= b,
            _ => out.push(op),
        }
    }
    out
}

/// `image_edit_apply_changes()`.
fn apply_changes(mut image: DynamicImage, history: &str) -> DynamicImage {
    for op in operations(history) {
        match op {
            Op::Rotate(angle) => {
                // `imagerotate()` turns counter-clockwise.
                image = match angle.rem_euclid(360) {
                    90 => image.rotate270(),
                    180 => image.rotate180(),
                    270 => image.rotate90(),
                    _ => image,
                };
            }
            Op::Flip(axis) => {
                if axis & 1 != 0 {
                    image = image.flipv();
                }
                if axis & 2 != 0 {
                    image = image.fliph();
                }
            }
            Op::Crop { x, y, w, h, scale } => {
                let scale = scale.unwrap_or_else(|| 1.0 / preview_ratio(image.width(), image.height()));
                let (x, y, w, h) = ((x * scale) as i64, (y * scale) as i64, (w * scale) as i64, (h * scale) as i64);
                if w > 0 && h > 0 && x >= 0 && y >= 0 {
                    let (x, y) = (x as u32, y as u32);
                    let w = (w as u32).min(image.width().saturating_sub(x));
                    let h = (h as u32).min(image.height().saturating_sub(y));
                    if w > 0 && h > 0 {
                        image = image.crop_imm(x, y, w, h);
                    }
                }
            }
        }
    }
    image
}

fn encode(image: &DynamicImage, mime: &str) -> Option<(Vec<u8>, &'static str)> {
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    let content_type = match mime {
        "image/jpeg" => {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 82);
            image.to_rgb8().write_with_encoder(encoder).ok()?;
            "image/jpeg"
        }
        "image/png" => {
            image.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
            "image/png"
        }
        "image/gif" => {
            image.write_to(&mut cursor, image::ImageFormat::Gif).ok()?;
            "image/gif"
        }
        "image/webp" => {
            image.write_to(&mut cursor, image::ImageFormat::WebP).ok()?;
            "image/webp"
        }
        _ => return None,
    };
    Some((out, content_type))
}

/// `wp_image_editor()`.
async fn editor_html(ctx: &Ctx<'_>, state: &State, note: Option<(bool, String)>) -> String {
    let Some(meta) = &state.meta else { return ctx.tr("Image data does not exist. Please re-upload the image.") };
    let (width, height) = (number(meta.get("width")), number(meta.get("height")));
    if width == 0 && height == 0 {
        return ctx.tr("Image data does not exist. Please re-upload the image.");
    }
    let nonce = ctx.create_nonce(&format!("image_editor-{}", state.id));
    let big = width.max(height) as f64;
    let sizer = if big > 600.0 { 600.0 / big } else { 1.0 };
    let can_restore = state.backup.as_ref().and_then(|b| b.get("full-orig")).is_some_and(|orig| {
        let current = string(meta.get("file"));
        current.rsplit('/').next().unwrap_or_default() != string(orig.get("file"))
    });
    let note_html = match note {
        Some((true, message)) => format!("<div class='notice notice-error' role='alert'><p>{message}</p></div>"),
        Some((false, message)) => format!("<div class='notice notice-success' role='alert'><p>{message}</p></div>"),
        None => String::new(),
    };
    let restore = if can_restore { super::pages::translate_html(ctx, RESTORE) } else { String::new() };
    let rand = rand::random_range(1..99999u32);
    super::pages::translate_html(ctx, EDITOR)
        .replace("{{RESTORE}}", &restore)
        .replace("{{NOTE}}", &note_html)
        .replace("{{NONCE}}", &nonce)
        .replace("{{SIZER}}", &rustpress_blocks::general::php_float(sizer))
        .replace("{{RAND}}", &rand.to_string())
        .replace("{{WIDTH}}", &width.to_string())
        .replace("{{HEIGHT}}", &height.to_string())
        .replace("{{ID}}", &state.id.to_string())
}

fn fail(message: &str) -> AdminResponse {
    json(200, &json!({"success": false, "data": {"error": message}}))
}

/// The unix time and three random digits: the suffix of the files an edit writes.
fn suffix() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    format!("{now}{}", rand::random_range(100..1000u32))
}

/// `wp_save_image()`.
async fn save(ctx: &Ctx<'_>, state: &mut State) -> Result<Json, String> {
    let env = ctx.env;
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    let Some(writer) = env.writer else { return Err(tr("Unable to save the image.")) };
    let mut image = image::open(state.path()).map_err(|_| tr("Unable to create new image."))?;
    let target = request.request("target").unwrap_or_default().chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect::<String>();
    let scale = request.request("do") == Some("scale");
    let full_width = request.request("fwidth").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let full_height = request.request("fheight").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let mut scaled = false;
    if scale {
        let (original_width, original_height) = (image.width(), image.height());
        if full_width > original_width || full_height > original_height {
            return Err(tr("Images cannot be scaled to a size larger than the original."));
        }
        if full_width > 0 && full_height > 0 {
            let diff = (f64::from(original_width) / f64::from(original_height) * 100.0).round() / 100.0 - (f64::from(full_width) / f64::from(full_height) * 100.0).round() / 100.0;
            if -0.1 < diff && diff < 0.1 {
                image = image.resize_exact(full_width, full_height, image::imageops::FilterType::Lanczos3);
                scaled = true;
            }
        }
        if !scaled {
            return Err(tr("Error while saving the scaled image. Please reload the page and try again."));
        }
    } else if let Some(history) = request.request("history").filter(|h| !h.is_empty()) {
        image = apply_changes(image, history);
    } else {
        return Err(tr("Nothing to save, the image has not changed."));
    }
    let Some(mut meta) = state.meta.clone() else { return Err(tr("Image data does not exist. Please re-upload the image.")) };
    if meta.get("sizes").is_none() {
        set(&mut meta, "sizes", Value::Array(Vec::new()));
    }
    let mut backup = state.backup.clone().unwrap_or(Value::Array(Vec::new()));

    let path = state.path();
    let dir = path.parent().map(PathBuf::from).unwrap_or_default();
    let basename = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (stem, ext) = basename.rsplit_once('.').map(|(a, b)| (a.to_owned(), b.to_owned())).unwrap_or((basename.clone(), String::new()));
    let mut suffix_value = suffix();
    let mut filename = stem;
    let new_path = loop {
        // `preg_replace( '/-e([0-9]+)$/', '', $filename )` and the new marker.
        if let Some(at) = filename.rfind("-e") {
            if filename[at + 2..].chars().all(|c| c.is_ascii_digit()) && at + 2 < filename.len() {
                filename.truncate(at);
            }
        }
        let candidate = format!("{filename}-e{suffix_value}");
        let candidate_path = dir.join(format!("{candidate}.{ext}"));
        if candidate_path.exists() {
            suffix_value = (suffix_value.parse::<u64>().unwrap_or(0) + 1).to_string();
            continue;
        }
        break candidate_path;
    };
    rustpress_content::media::save_image(&image, &new_path, &state.mime).map_err(|_| tr("Unable to save the image."))?;
    let new_size = std::fs::metadata(&new_path).map(|m| m.len() as i64).unwrap_or(0);
    let new_basename = new_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let new_relative = match state.relative.rsplit_once('/') {
        Some((folder, _)) => format!("{folder}/{new_basename}"),
        None => new_basename.clone(),
    };

    // The original file is kept as `full-orig`; later edits keep the latest replaced file as well.
    let mut fw = 0;
    let mut fh = 0;
    let replaces_full = matches!(target.as_str(), "nothumb" | "all" | "full") || scaled;
    if replaces_full {
        let tag = match backup.get("full-orig") {
            Some(orig) if string(orig.get("file")) != basename => Some(format!("full-{suffix_value}")),
            Some(_) => None,
            None => Some("full-orig".to_owned()),
        };
        if let Some(tag) = tag {
            let entry = Value::Array(vec![
                (key("width"), Value::Int(number(meta.get("width")))),
                (key("height"), Value::Int(number(meta.get("height")))),
                (key("filesize"), Value::Int(number(meta.get("filesize")))),
                text_entry("file", &basename),
            ]);
            set(&mut backup, &tag, entry);
        }
        set(&mut meta, "file", Value::Str(new_relative.clone().into_bytes()));
        set(&mut meta, "width", Value::Int(i64::from(image.width())));
        set(&mut meta, "height", Value::Int(i64::from(image.height())));
        set(&mut meta, "filesize", Value::Int(new_size));
        fw = image.width();
        fh = image.height();
        if matches!(target.as_str(), "nothumb" | "all") {
            if let Some((_, _, sizes)) = rustpress_content::media::generate_sizes(&env.site.options, &new_path, &state.mime) {
                let size_names: Vec<String> = rustpress_content::media::registered_sizes(&env.site.options).into_iter().map(|(n, ..)| n).collect();
                for name in &size_names {
                    let current = meta.get("sizes").and_then(|s| s.get(name)).cloned();
                    if let Some(current) = current {
                        let tag = match backup.get(&format!("{name}-orig")) {
                            Some(orig) if string(orig.get("file")) != string(current.get("file")) => Some(format!("{name}-{suffix_value}")),
                            Some(_) => None,
                            None => Some(format!("{name}-orig")),
                        };
                        if let Some(tag) = tag {
                            set(&mut backup, &tag, current);
                        }
                    }
                }
                let mut merged = meta.get("sizes").cloned().unwrap_or(Value::Array(Vec::new()));
                for (name, entry) in sizes {
                    set(&mut merged, &name, entry);
                }
                set(&mut meta, "sizes", merged);
            }
        }
    }
    let _ = writer.update_post_meta(state.id, "_wp_attached_file", &new_relative).await;
    let _ = writer.update_post_meta(state.id, "_wp_attachment_metadata", &serialize(&meta)).await;
    let _ = writer.update_post_meta(state.id, "_wp_attachment_backup_sizes", &serialize(&backup)).await;
    state.relative = new_relative.clone();
    state.meta = Some(meta);
    state.backup = Some(backup);

    let site = env.site;
    let url = rustpress_blocks::images::attachment_url(site, &new_relative);
    let mut done = serde_json::Map::new();
    if replaces_full {
        done.insert("fw".into(), json!(fw));
        done.insert("fh".into(), json!(fh));
    }
    if matches!(target.as_str(), "thumbnail" | "all" | "full") {
        let thumbnail = state.meta.as_ref().and_then(|m| rustpress_blocks::images::Metadata::parse(&serialize(m))).and_then(|m| rustpress_blocks::images::intermediate_size(&url, &m, 900, 600)).map_or(url, |(u, _, _)| u);
        done.insert("thumbnail".into(), json!(thumbnail));
    }
    done.insert("msg".into(), json!(tr("Image saved")));
    Ok(Json::Object(done))
}

/// `wp_restore_image()`: the message to show.
async fn restore(ctx: &Ctx<'_>, state: &mut State) -> Result<String, String> {
    let env = ctx.env;
    let tr = |s: &str| ctx.tr(s);
    let (Some(mut meta), Some(old_backup)) = (state.meta.clone(), state.backup.clone()) else { return Err(tr("Cannot load image metadata.")) };
    let mut backup = old_backup.clone();
    if meta.get("sizes").is_none() {
        set(&mut meta, "sizes", Value::Array(Vec::new()));
    }
    let path = state.path();
    let basename = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let suffix_value = suffix();
    let mut restored = false;
    let mut new_relative = state.relative.clone();
    if let Some(data) = backup.get("full-orig").cloned() {
        let file = string(data.get("file"));
        if basename != file {
            let entry = Value::Array(vec![(key("width"), Value::Int(number(meta.get("width")))), (key("height"), Value::Int(number(meta.get("height")))), (key("filesize"), Value::Int(number(meta.get("filesize")))), text_entry("file", &basename)]);
            set(&mut backup, &format!("full-{suffix_value}"), entry);
        }
        new_relative = match state.relative.rsplit_once('/') {
            Some((folder, _)) => format!("{folder}/{file}"),
            None => file.clone(),
        };
        restored = true;
        set(&mut meta, "file", Value::Str(new_relative.clone().into_bytes()));
        set(&mut meta, "width", data.get("width").cloned().unwrap_or(Value::Int(0)));
        set(&mut meta, "height", data.get("height").cloned().unwrap_or(Value::Int(0)));
        if let Some(filesize) = data.get("filesize") {
            set(&mut meta, "filesize", filesize.clone());
        }
    }
    for (name, ..) in rustpress_content::media::registered_sizes(&env.site.options) {
        match backup.get(&format!("{name}-orig")).cloned() {
            Some(data) => {
                let replaced = meta.get("sizes").and_then(|s| s.get(&name)).cloned();
                if let Some(current) = replaced.filter(|c| string(c.get("file")) != string(data.get("file"))) {
                    set(&mut backup, &format!("{name}-{suffix_value}"), current);
                }
                let mut sizes = meta.get("sizes").cloned().unwrap_or(Value::Array(Vec::new()));
                set(&mut sizes, &name, data);
                set(&mut meta, "sizes", sizes);
            }
            None => {
                let mut sizes = meta.get("sizes").cloned().unwrap_or(Value::Array(Vec::new()));
                remove(&mut sizes, &name);
                set(&mut meta, "sizes", sizes);
            }
        }
    }
    let Some(writer) = env.writer else { return Err(tr("Cannot save image metadata.")) };
    let _ = writer.update_post_meta(state.id, "_wp_attached_file", &new_relative).await;
    let _ = writer.update_post_meta(state.id, "_wp_attachment_metadata", &serialize(&meta)).await;
    if backup != old_backup {
        let _ = writer.update_post_meta(state.id, "_wp_attachment_backup_sizes", &serialize(&backup)).await;
    }
    state.relative = new_relative;
    state.meta = Some(meta);
    state.backup = Some(backup);
    if restored { Ok(tr("Image restored successfully.")) } else { Err(tr("Image metadata is inconsistent.")) }
}

/// `wp_ajax_image_editor()`.
pub async fn editor(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str) -> AdminResponse {
    let id: u64 = request.post_field("postid").and_then(|p| p.parse().ok()).unwrap_or(0);
    let Some(post) = (if id == 0 { None } else { env.db.post(id).await.ok().flatten() }) else { return text(200, "-1") };
    let may_edit = user.can("edit_posts") && (post.post_author == user.id || user.can("edit_others_posts"));
    if !may_edit {
        return text(200, "-1");
    }
    if env.auth.verify_nonce(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), &format!("image_editor-{id}"), user.id, token, env.now).is_none() {
        return text(403, "-1");
    }
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let Some(mut state) = load(env, id).await else { return text(403, "-1") };
    let mut note: Option<(bool, String)> = None;
    let mut message_object: Option<Json> = None;
    match request.request("do").unwrap_or_default() {
        "save" => {
            return match save(&ctx, &mut state).await {
                Ok(done) => json(200, &json!({"success": true, "data": done})),
                Err(message) => fail(&message),
            };
        }
        "scale" => {
            note = Some(match save(&ctx, &mut state).await {
                Ok(done) => {
                    let message = done["msg"].as_str().unwrap_or_default().to_owned();
                    message_object = Some(done);
                    (false, message)
                }
                Err(message) => (true, message),
            });
        }
        "restore" => {
            note = Some(match restore(&ctx, &mut state).await {
                Ok(message) => (false, message),
                Err(message) => (true, message),
            });
        }
        _ => {}
    }
    let html = editor_html(&ctx, &state, note.clone()).await;
    let message = match &note {
        Some((true, m)) => json!({"error": m}),
        Some((false, m)) => message_object.clone().unwrap_or_else(|| json!({"msg": m})),
        None => json!(false),
    };
    let ok = !matches!(note, Some((true, _)));
    json(200, &json!({"success": ok, "data": {"message": message, "html": html}}))
}

/// `wp_ajax_imgedit_preview()`.
pub async fn preview(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str) -> AdminResponse {
    let id: u64 = request.request("postid").and_then(|p| p.parse().ok()).unwrap_or(0);
    let Some(post) = (if id == 0 { None } else { env.db.post(id).await.ok().flatten() }) else { return text(403, "-1") };
    let may_edit = user.can("edit_posts") && (post.post_author == user.id || user.can("edit_others_posts"));
    if !may_edit {
        return text(403, "-1");
    }
    if env.auth.verify_nonce(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), &format!("image_editor-{id}"), user.id, token, env.now).is_none() {
        return text(403, "-1");
    }
    let Some(state) = load(env, id).await else { return text(403, "-1") };
    let Ok(mut image) = image::open(state.path()) else { return text(403, "-1") };
    if let Some(history) = request.request("history").filter(|h| !h.is_empty()) {
        image = apply_changes(image, history);
    }
    let ratio = preview_ratio(image.width(), image.height());
    let (w, h) = (((f64::from(image.width()) * ratio) as u32).max(1), ((f64::from(image.height()) * ratio) as u32).max(1));
    if (w, h) != (image.width(), image.height()) {
        image = image.resize_exact(w, h, image::imageops::FilterType::Lanczos3);
    }
    let Some((bytes, content_type)) = encode(&image, &state.mime) else { return text(403, "-1") };
    let mut response = text(200, "");
    response.headers.retain(|(k, _)| k != "Content-Type");
    response.headers.push(("Content-Type".into(), content_type.into()));
    response.bytes = bytes;
    response
}
