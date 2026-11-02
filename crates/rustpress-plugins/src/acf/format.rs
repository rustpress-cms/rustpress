//! `acf_format_value()`: the `format_value()` methods of the field types, as the template functions, the `standard`
//! format of the REST API and the shortcode see a value. Objects come out as PHP arrays in the order of their
//! properties, so `json_encode()` of a `WP_Post` or `WP_Term` and of the array are the same.

use futures::future::BoxFuture;
use rustpress_core::native::PageServices;
use rustpress_core::{Result, Site};
use rustpress_db::{AcfWpPost, Post};
use rustpress_phpser::{Key, Value};

use super::fields::Field;
use super::php::{intval, is_numeric, key, set, string, strval, truthy};

/// What formatting a value needs from the request.
pub struct Formatter<'a> {
    pub site: &'a Site,
    pub services: &'a dyn PageServices,
    /// `global $post`, for the shortcodes of a WYSIWYG field.
    pub global_post: Option<&'a Post>,
}

/// `acf_is_empty()`: false but not a number.
fn is_empty(value: &Value) -> bool {
    !truthy(Some(value)) && !is_numeric(value)
}

/// A list as PHP builds it with `$array[] = $item`.
fn list(items: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(items.into_iter().enumerate().map(|(i, v)| (Key::Int(i as i64), v)).collect())
}

/// `acf_array()`/`acf_get_array()`: an array stays, a scalar becomes a one-item array (an empty string none).
fn acf_array(value: Value) -> Vec<(Key, Value)> {
    match value {
        Value::Array(items) => items,
        Value::Null => Vec::new(),
        Value::Str(s) if s.is_empty() => Vec::new(),
        other => vec![(Key::Int(0), other)],
    }
}

/// `array_shift()` of an array (`null` when it is empty).
fn first(items: Vec<(Key, Value)>) -> Value {
    items.into_iter().next().map_or(Value::Null, |(_, v)| v)
}

/// `acf_get_numeric()`: the numeric values as integers; one for a scalar; false when there are none.
fn get_numeric(value: &Value) -> Value {
    let (items, is_array) = match value {
        Value::Array(items) => (items.iter().map(|(_, v)| v).collect::<Vec<_>>(), true),
        Value::Null => (Vec::new(), false),
        other => (vec![other], false),
    };
    let numbers: Vec<Value> = items.into_iter().filter(|v| is_numeric(v)).map(|v| Value::Int(intval(Some(v)))).collect();
    if numbers.is_empty() {
        return Value::Bool(false);
    }
    if is_array { list(numbers) } else { numbers.into_iter().next().unwrap_or(Value::Bool(false)) }
}

/// `esc_html()`.
fn esc_html(text: &str) -> String {
    rustpress_core::formatting::esc_html(text)
}

/// `nl2br()`: `<br />` before every line break (`\r\n`, `\n\r`, `\n`, `\r`).
pub(super) fn nl2br(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + 16);
    let mut i = 0;
    let mut start = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' || b == b'\r' {
            out.push_str(&text[start..i]);
            out.push_str("<br />");
            let pair = bytes.get(i + 1).is_some_and(|&n| (n == b'\n' || n == b'\r') && n != b);
            let end = if pair { i + 2 } else { i + 1 };
            out.push_str(&text[i..end]);
            i = end;
            start = end;
        } else {
            i += 1;
        }
    }
    out.push_str(&text[start..]);
    out
}

/// `strtotime()` for the values ACF stores (`Ymd`, `Y-m-d H:i:s`, `Y-m-d`, `H:i:s`, `H:i`), read as UTC like
/// WordPress's default time zone does; a time alone is on today's date.
fn strtotime(value: &str) -> Option<i64> {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    let v = value.trim();
    for format in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(v, format) {
            return Some(dt.and_utc().timestamp());
        }
    }
    for format in ["%Y%m%d", "%Y-%m-%d", "%Y/%m/%d"] {
        if let Ok(d) = NaiveDate::parse_from_str(v, format) {
            return d.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc().timestamp());
        }
    }
    for format in ["%H:%M:%S", "%H:%M"] {
        if let Ok(t) = NaiveTime::parse_from_str(v, format) {
            return Some(chrono::Utc::now().date_naive().and_time(t).and_utc().timestamp());
        }
    }
    None
}

/// `acf_format_date()`: `date_i18n()` of the stored date.
pub(super) fn format_date(site: &Site, value: Value, format: &str) -> Value {
    let text = match &value {
        Value::Str(s) if !s.is_empty() && s.as_slice() != b"0" => String::from_utf8_lossy(s).into_owned(),
        Value::Int(i) if *i != 0 => i.to_string(),
        _ => return value,
    };
    let timestamp = if is_numeric(&value) && text.len() != 8 { intval(Some(&value)) } else { strtotime(&text).unwrap_or_else(|| chrono::Utc::now().timestamp()) };
    string(&site.date_utc(format, timestamp))
}

/// `(float) $text`: the number at the start of the text, 0 without one.
fn leading_float(text: &str) -> f64 {
    let t = text.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let mut end = 0;
    let mut seen_dot = false;
    for (i, c) in t.char_indices() {
        let ok = c.is_ascii_digit() || (c == '.' && !seen_dot) || (i == 0 && (c == '-' || c == '+'));
        if !ok {
            break;
        }
        seen_dot |= c == '.';
        end = i + c.len_utf8();
    }
    t[..end].parse().unwrap_or(0.0)
}

/// `acf_field_color_picker::string_to_array()`.
fn color_to_array(value: &Value) -> Value {
    let text = match value {
        Value::Str(s) => String::from_utf8_lossy(s).trim().to_owned(),
        _ => String::new(),
    };
    let rgba = |r: i64, g: i64, b: i64, a: f64| Value::Array(vec![(key("red"), Value::Int(r)), (key("green"), Value::Int(g)), (key("blue"), Value::Int(b)), (key("alpha"), Value::Float(a))]);
    let hex = |s: &str| i64::from_str_radix(s, 16).unwrap_or(0);
    let digits = text.strip_prefix('#').filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit()));
    match digits {
        Some(d) if d.len() == 6 => return rgba(hex(&d[0..2]), hex(&d[2..4]), hex(&d[4..6]), 1.0),
        Some(d) if d.len() == 3 => {
            let twice = |i: usize| hex(&d[i..i + 1].repeat(2));
            return rgba(twice(0), twice(1), twice(2), 1.0);
        }
        _ => {}
    }
    let lower = text.to_ascii_lowercase();
    let inner = lower.strip_prefix("rgba(").or_else(|| lower.strip_prefix("rgb(")).and_then(|rest| {
        let end = rest.find(|c: char| !(c.is_ascii_digit() || c == ',' || c == '.'))?;
        (rest.as_bytes().get(end) == Some(&b')') && end > 0).then(|| rest[..end].to_owned())
    });
    if let Some(inner) = inner {
        let parts: Vec<&str> = inner.split(',').collect();
        let int = |s: &str| intval(Some(&string(s)));
        let float = |s: &str| leading_float(s);
        match parts.len() {
            4 => return rgba(int(parts[0]), int(parts[1]), int(parts[2]), float(parts[3])),
            3 => return rgba(int(parts[0]), int(parts[1]), int(parts[2]), 1.0),
            _ => {}
        }
    }
    rgba(0, 0, 0, 0.0)
}

/// A `WP_Post` as `json_encode()` writes its properties.
pub fn post_object(row: &AcfWpPost) -> Value {
    let p = &row.post;
    Value::Array(vec![
        (key("ID"), Value::Int(p.id as i64)),
        (key("post_author"), string(&p.post_author.to_string())),
        (key("post_date"), string(&p.post_date)),
        (key("post_date_gmt"), string(&p.post_date_gmt)),
        (key("post_content"), string(&p.post_content)),
        (key("post_title"), string(&p.post_title)),
        (key("post_excerpt"), string(&p.post_excerpt)),
        (key("post_status"), string(&p.post_status)),
        (key("comment_status"), string(&p.comment_status)),
        (key("ping_status"), string(&p.ping_status)),
        (key("post_password"), string(&p.post_password)),
        (key("post_name"), string(&p.post_name)),
        (key("to_ping"), string(&row.to_ping)),
        (key("pinged"), string(&row.pinged)),
        (key("post_modified"), string(&p.post_modified)),
        (key("post_modified_gmt"), string(&p.post_modified_gmt)),
        (key("post_content_filtered"), string(&row.post_content_filtered)),
        (key("post_parent"), Value::Int(p.post_parent as i64)),
        (key("guid"), string(&p.guid)),
        (key("menu_order"), Value::Int(i64::from(p.menu_order))),
        (key("post_type"), string(&p.post_type)),
        (key("post_mime_type"), string(&p.post_mime_type)),
        (key("comment_count"), string(&p.comment_count.to_string())),
        (key("filter"), string("raw")),
    ])
}

/// A `WP_Term` as `json_encode()` writes its properties.
fn term_object(term: &rustpress_db::Term, group: i64) -> Value {
    Value::Array(vec![
        (key("term_id"), Value::Int(term.term_id as i64)),
        (key("name"), string(&term.name)),
        (key("slug"), string(&term.slug)),
        (key("term_group"), Value::Int(group)),
        (key("term_taxonomy_id"), Value::Int(term.term_taxonomy_id as i64)),
        (key("taxonomy"), string(&term.taxonomy)),
        (key("description"), string(&term.description)),
        (key("parent"), Value::Int(term.parent as i64)),
        (key("count"), Value::Int(term.count)),
        (key("filter"), string("raw")),
    ])
}

/// `acf_get_post_types()`: every registered post type except ACF's own and the built-in ones that are not public.
pub fn all_post_types(site: &Site) -> Vec<String> {
    let mut out: Vec<String> = ["post", "page", "attachment"].iter().map(|s| (*s).to_owned()).collect();
    for info in site.types.rewrite_post_types().iter().chain(site.types.post_types.iter()) {
        if !out.contains(&info.name) && !info.name.starts_with("acf-") {
            out.push(info.name.clone());
        }
    }
    for name in &site.types.hidden_post_types {
        if !out.contains(name) && !name.starts_with("acf-") && !rustpress_core::types::is_internal_post_type(name) {
            out.push(name.clone());
        }
    }
    out
}

/// The `post_type` setting of a field, or all types (`acf_get_posts()`).
fn field_post_types(site: &Site, field: &Field) -> Vec<String> {
    match field.get("post_type") {
        Some(Value::Array(items)) if !items.is_empty() => items.iter().map(|(_, v)| strval(Some(v))).collect(),
        Some(v) if truthy(Some(v)) && !matches!(v, Value::Array(_)) => vec![strval(Some(v))],
        _ => all_post_types(site),
    }
}

/// `acf_get_posts( [ 'post__in' => $ids, 'post_type' => $field['post_type'] ] )`.
async fn get_posts(site: &Site, field: &Field, ids: &[u64]) -> Result<Vec<AcfWpPost>> {
    Ok(site.db.acf_get_posts(ids, &field_post_types(site, field)).await?)
}

/// The ids of a list of values that are numbers.
fn numeric_ids(items: &[(Key, Value)]) -> Vec<u64> {
    items.iter().filter(|(_, v)| is_numeric(v)).map(|(_, v)| intval(Some(v)).max(0) as u64).collect()
}

/// The largest size `image_constrain_size_for_editor()` allows for a named size on the website (`$content_width`
/// of a classic theme is not known here).
fn size_limit(site: &Site, size: &str, width: u64, height: u64) -> (u64, u64) {
    let option = |name: &str, default: u64| site.options.u64(name, default);
    match size {
        "thumb" | "thumbnail" => {
            let (w, h) = (option("thumbnail_size_w", 150), option("thumbnail_size_h", 150));
            if w == 0 && h == 0 { (128, 96) } else { (w, h) }
        }
        "medium" => (option("medium_size_w", 300), option("medium_size_h", 300)),
        "medium_large" => (option("medium_large_size_w", 768), option("medium_large_size_h", 0)),
        "large" => (option("large_size_w", 1024), option("large_size_h", 1024)),
        "1536x1536" => (1536, 1536),
        "2048x2048" => (2048, 2048),
        _ => (width, height),
    }
}

/// `wp_get_attachment_image_src( $id, $size )` of an image: the generated file of the size, or the full image, both
/// constrained to the size.
pub(super) fn image_src(site: &Site, meta: Option<&rustpress_blocks::images::Metadata>, full_url: &str, size: &str) -> (String, u64, u64) {
    let (url, width, height) = match meta {
        Some(meta) => {
            let entry = meta.sizes().and_then(|s| s.get(size)).filter(|e| e.get("file").and_then(serde_json::Value::as_str).is_some_and(|f| !f.is_empty()));
            match entry {
                Some(_) if size != "full" => {
                    let d = rustpress_blocks::images::image_downsize(full_url, meta, size);
                    (d.url, d.width, d.height)
                }
                _ => (full_url.to_owned(), meta.width(), meta.height()),
            }
        }
        None => (full_url.to_owned(), 0, 0),
    };
    let (max_w, max_h) = size_limit(site, size, width, height);
    let (w, h) = rustpress_blocks::images::constrain_dimensions(width, height, max_w, max_h);
    (url, w, h)
}

/// `wp_get_attachment_url()`: the uploads URL of the attached file, the GUID without one.
pub(super) async fn attachment_url(site: &Site, post: &Post) -> Result<(Option<rustpress_blocks::images::Metadata>, Option<String>, String)> {
    let (meta, attached) = rustpress_blocks::images::load_metadata(site, post).await?;
    let url = match attached.as_deref().filter(|f| !f.is_empty()) {
        Some(file) => rustpress_blocks::images::attachment_url(site, file),
        None => post.guid.clone(),
    };
    Ok((meta, attached, url))
}

impl Formatter<'_> {
    /// `acf_get_attachment()`: the array of an attachment, false for anything else.
    pub async fn attachment(&self, id: i64) -> Result<Value> {
        let site = self.site;
        let Some(post) = (if id > 0 { site.db.post(id as u64).await? } else { None }) else { return Ok(Value::Bool(false)) };
        if post.post_type != "attachment" {
            return Ok(Value::Bool(false));
        }
        let (meta, attached, url) = attachment_url(site, &post).await?;
        let (kind, subtype) = post.post_mime_type.split_once('/').map_or((post.post_mime_type.clone(), String::new()), |(a, b)| (a.to_owned(), b.to_owned()));
        let filename = attached.as_deref().map(|f| f.rsplit('/').next().unwrap_or(f).to_owned()).unwrap_or_default();
        let alt = site.db.acf_meta(rustpress_db::MetaKind::Post, post.id, "_wp_attachment_image_alt").await?.unwrap_or_default();
        let link = site.permalink(&post).await?;
        let icon = rustpress_admin::screens::ajax_media::mime_icon(&post.post_mime_type).replace(".svg", ".png");
        let filesize = match meta.as_ref().and_then(|m| m.0.get("filesize")) {
            Some(size) => Value::Int(size.as_i64().or_else(|| size.as_str().map(|s| intval(Some(&string(s))))).unwrap_or(0)),
            None => {
                let path = attached.as_deref().map(|f| self.services.wp_root().join("wp-content/uploads").join(f));
                Value::Int(path.and_then(|p| std::fs::metadata(p).ok()).map_or(0, |m| m.len() as i64))
            }
        };
        let mut out = vec![
            (key("ID"), Value::Int(post.id as i64)),
            (key("id"), Value::Int(post.id as i64)),
            (key("title"), string(&post.post_title)),
            (key("filename"), string(&filename)),
            (key("filesize"), filesize),
            (key("url"), string(&url)),
            (key("link"), string(&link)),
            (key("alt"), string(&alt)),
            (key("author"), string(&post.post_author.to_string())),
            (key("description"), string(&post.post_content)),
            (key("caption"), string(&post.post_excerpt)),
            (key("name"), string(&post.post_name)),
            (key("status"), string(&post.post_status)),
            (key("uploaded_to"), Value::Int(post.post_parent as i64)),
            (key("date"), string(&post.post_date_gmt)),
            (key("modified"), string(&post.post_modified_gmt)),
            (key("menu_order"), Value::Int(i64::from(post.menu_order))),
            (key("mime_type"), string(&post.post_mime_type)),
            (key("type"), string(&kind)),
            (key("subtype"), string(&subtype)),
            (key("icon"), string(&site.site_url(&format!("wp-includes/images/media/{icon}")))),
        ];
        let mut sizes_of: Option<(Option<rustpress_blocks::images::Metadata>, String)> = None;
        match kind.as_str() {
            "image" => {
                let (src, w, h) = image_src(site, meta.as_ref(), &url, "full");
                set(&mut out, "url", string(&src));
                set(&mut out, "width", Value::Int(w as i64));
                set(&mut out, "height", Value::Int(h as i64));
                sizes_of = Some((meta.clone(), url.clone()));
            }
            "video" | "audio" => {
                if kind == "video" {
                    let dimension = |name: &str| meta.as_ref().and_then(|m| m.0.get(name)).map_or(Value::Int(0), |v| v.as_i64().map_or_else(|| string(v.as_str().unwrap_or_default()), Value::Int));
                    set(&mut out, "width", dimension("width"));
                    set(&mut out, "height", dimension("height"));
                }
                let thumbnail = site.db.acf_meta(rustpress_db::MetaKind::Post, post.id, "_thumbnail_id").await?.map_or(0, |v| intval(Some(&string(&v))));
                if thumbnail > 0
                    && let Some(image) = site.db.post(thumbnail as u64).await?
                {
                    let (image_meta, _, image_url) = attachment_url(site, &image).await?;
                    sizes_of = Some((image_meta, image_url));
                }
            }
            _ => {}
        }
        if let Some((sizes_meta, full_url)) = sizes_of {
            let mut sizes = Vec::new();
            for (name, ..) in rustpress_content::media::registered_sizes(&site.options) {
                let (src, w, h) = image_src(site, sizes_meta.as_ref(), &full_url, &name);
                sizes.push((key(&name), string(&src)));
                sizes.push((key(&format!("{name}-width")), Value::Int(w as i64)));
                sizes.push((key(&format!("{name}-height")), Value::Int(h as i64)));
            }
            out.push((key("sizes"), Value::Array(sizes)));
        }
        Ok(Value::Array(out))
    }

    /// `acf_field_select::format_value_single()`.
    fn select_single(value: Value, field: &Field) -> Value {
        if is_empty(&value) {
            return value;
        }
        let label = match field.get("choices") {
            Some(choices @ Value::Array(_)) => choices.get(&strval(Some(&value))).cloned().unwrap_or_else(|| value.clone()),
            _ => value.clone(),
        };
        match strval(field.get("return_format")).as_str() {
            "label" => label,
            "array" => Value::Array(vec![(key("value"), value), (key("label"), label)]),
            _ => value,
        }
    }

    /// `acf_field_select::format_value()` (also of the checkbox, radio and button group fields).
    fn select(value: Value, field: &Field) -> Value {
        match value {
            Value::Array(items) => Value::Array(items.into_iter().map(|(k, v)| (k, Self::select_single(v, field))).collect()),
            other => Self::select_single(other, field),
        }
    }

    /// `acf_format_value( $value, $post_id, $field, $escape_html )`.
    pub fn format<'b>(&'b self, field: &'b Field, value: Value, escape_html: bool) -> BoxFuture<'b, Result<Value>> {
        Box::pin(async move {
            let site = self.site;
            let return_format = strval(field.get("return_format"));
            let multiple = truthy(field.get("multiple"));
            Ok(match field.kind.as_str() {
                "select" | "radio" | "button_group" => Self::select(value, field),
                "checkbox" => {
                    if is_empty(&value) {
                        Value::Array(Vec::new())
                    } else {
                        Self::select(Value::Array(acf_array(value)), field)
                    }
                }
                "color_picker" if return_format == "array" => color_to_array(&value),
                "date_picker" => {
                    if truthy(field.get("save_format")) {
                        value
                    } else {
                        format_date(site, value, &return_format)
                    }
                }
                "date_time_picker" | "time_picker" => format_date(site, value, &return_format),
                "file" | "image" => {
                    if !truthy(Some(&value)) || !is_numeric(&value) {
                        return Ok(Value::Bool(false));
                    }
                    let id = intval(Some(&value));
                    match return_format.as_str() {
                        "url" => match site.db.post(id.max(0) as u64).await? {
                            Some(post) if post.post_type == "attachment" => string(&attachment_url(site, &post).await?.2),
                            _ => Value::Bool(false),
                        },
                        "array" => self.attachment(id).await?,
                        _ => Value::Int(id),
                    }
                }
                "link" => {
                    if !truthy(Some(&value)) {
                        return Ok(value);
                    }
                    let mut link = vec![(key("title"), string("")), (key("url"), string("")), (key("target"), string(""))];
                    match &value {
                        Value::Array(items) => {
                            for (k, v) in items {
                                match link.iter_mut().find(|(existing, _)| super::php::key_eq(existing, k)) {
                                    Some(slot) => slot.1 = v.clone(),
                                    None => link.push((k.clone(), v.clone())),
                                }
                            }
                        }
                        v if is_numeric(v) => {
                            if let Some(post) = site.db.post(intval(Some(v)).max(0) as u64).await? {
                                set(&mut link, "title", string(&post.post_title));
                                set(&mut link, "url", string(&site.permalink(&post).await?));
                            } else {
                                set(&mut link, "url", Value::Bool(false));
                            }
                        }
                        Value::Str(_) => set(&mut link, "url", value.clone()),
                        _ => {}
                    }
                    if return_format == "url" {
                        return Ok(Value::Array(link).get("url").cloned().unwrap_or(Value::Null));
                    }
                    Value::Array(link)
                }
                // `WP_Embed::shortcode()` without a network: the address as a link.
                "oembed" => {
                    if !truthy(Some(&value)) {
                        return Ok(value);
                    }
                    let url = strval(Some(&value));
                    string(&format!("<a href=\"{}\">{}</a>", rustpress_core::formatting::esc_url(&url), esc_html(&url)))
                }
                "page_link" => {
                    if value == string("null") {
                        return Ok(Value::Bool(false));
                    }
                    if !truthy(Some(&value)) {
                        return Ok(value);
                    }
                    let items = acf_array(value);
                    let ids = numeric_ids(&items);
                    let mut values: Vec<Value> = if ids.is_empty() {
                        items.into_iter().map(|(_, v)| v).collect()
                    } else {
                        let mut posts = get_posts(site, field, &ids).await?.into_iter();
                        let mut out = Vec::new();
                        for (_, v) in items {
                            if is_numeric(&v) {
                                if let Some(post) = posts.next() {
                                    out.push(string(&site.permalink(&post.post).await?));
                                }
                            } else {
                                out.push(v);
                            }
                        }
                        out
                    };
                    if multiple {
                        list(values)
                    } else if values.is_empty() {
                        Value::Null
                    } else {
                        values.remove(0)
                    }
                }
                "post_object" => {
                    let numeric = get_numeric(&value);
                    if !truthy(Some(&numeric)) {
                        return Ok(Value::Bool(false));
                    }
                    let value = if return_format == "object" {
                        let ids: Vec<u64> = match &numeric {
                            Value::Array(items) => items.iter().map(|(_, v)| intval(Some(v)).max(0) as u64).collect(),
                            v => vec![intval(Some(v)).max(0) as u64],
                        };
                        list(get_posts(site, field, &ids).await?.iter().map(post_object))
                    } else {
                        numeric
                    };
                    match value {
                        // `current()`: false for an empty array.
                        Value::Array(items) if !multiple => items.into_iter().next().map_or(Value::Bool(false), |(_, v)| v),
                        other => other,
                    }
                }
                "relationship" => {
                    if !truthy(Some(&value)) {
                        return Ok(value);
                    }
                    let ids: Vec<i64> = acf_array(value).iter().map(|(_, v)| intval(Some(v))).collect();
                    if return_format == "object" {
                        let wanted: Vec<u64> = ids.iter().map(|i| (*i).max(0) as u64).collect();
                        list(get_posts(site, field, &wanted).await?.iter().map(post_object))
                    } else {
                        list(ids.into_iter().map(Value::Int))
                    }
                }
                "taxonomy" => {
                    if !truthy(Some(&value)) {
                        return Ok(Value::Bool(false));
                    }
                    let mut items = acf_array(value);
                    if return_format == "object" {
                        let taxonomy = strval(field.get("taxonomy"));
                        let mut terms = Vec::new();
                        for (k, v) in items {
                            let id = intval(Some(&v));
                            // `get_term()`; `array_filter()` keeps the keys of the ones found.
                            if id > 0
                                && let Some(term) = site.db.term_by_id(Some(&taxonomy), id as u64).await?
                            {
                                let group = site.db.acf_term_group(term.term_id).await?;
                                terms.push((k, term_object(&term, group)));
                            }
                        }
                        items = terms;
                    }
                    if matches!(strval(field.get("field_type")).as_str(), "select" | "radio") { first(items) } else { Value::Array(items) }
                }
                "textarea" => match &value {
                    Value::Str(s) if truthy(Some(&value)) => {
                        let text = String::from_utf8_lossy(s);
                        match strval(field.get("new_lines")).as_str() {
                            "wpautop" => string(&rustpress_core::formatting::wpautop(&text)),
                            "br" => string(&nl2br(&text)),
                            _ => value,
                        }
                    }
                    _ => value,
                },
                "true_false" => Value::Bool(truthy(Some(&value))),
                "url" if escape_html => string(&rustpress_core::formatting::esc_url(&strval(Some(&value)))),
                "wysiwyg" => match &value {
                    Value::Str(s) if truthy(Some(&value)) => {
                        let mut text = String::from_utf8_lossy(s).into_owned();
                        if escape_html {
                            text = rustpress_core::kses::kses_post(&text);
                        }
                        let text = self.services.content_filters(self.global_post, &text, "acf_the_content").await?;
                        string(&text.replace("]]>", "]]&gt;"))
                    }
                    _ => value,
                },
                "group" => {
                    if !truthy(Some(&value)) {
                        return Ok(Value::Bool(false));
                    }
                    let mut items = match value {
                        Value::Array(items) => items,
                        other => return Ok(other),
                    };
                    for sub in &field.sub_fields {
                        let k = key(&sub.key);
                        let sub_value = match items.iter().position(|(existing, _)| super::php::key_eq(existing, &k)) {
                            Some(i) => items.remove(i).1,
                            None => Value::Null,
                        };
                        let formatted = self.format(sub, sub_value, escape_html).await?;
                        set(&mut items, &sub.name, formatted);
                    }
                    Value::Array(items)
                }
                "icon_picker" => {
                    if !truthy(Some(&value)) {
                        return Ok(field.get("default_value").filter(|v| !matches!(v, Value::Null)).cloned().unwrap_or(Value::Bool(false)));
                    }
                    let Value::Array(mut items) = value else { return Ok(value) };
                    let get = |items: &Vec<(Key, Value)>, name: &str| items.iter().find(|(k, _)| super::php::key_eq(k, &key(name))).map(|(_, v)| v.clone());
                    if get(&items, "type") == Some(string("media_library")) {
                        let id = intval(get(&items, "value").as_ref());
                        set(&mut items, "value", Value::Int(id));
                        match return_format.as_str() {
                            "string" => {
                                return Ok(match site.db.post(id.max(0) as u64).await? {
                                    Some(post) if post.post_type == "attachment" => string(&attachment_url(site, &post).await?.2),
                                    _ => Value::Bool(false),
                                });
                            }
                            "array" => {
                                let attachment = self.attachment(id).await?;
                                set(&mut items, "value", attachment);
                                return Ok(Value::Array(items));
                            }
                            _ => {}
                        }
                    }
                    if return_format == "string" {
                        return Ok(get(&items, "value").unwrap_or(Value::Null));
                    }
                    Value::Array(items)
                }
                "user" => {
                    if !truthy(Some(&value)) {
                        return Ok(Value::Bool(false));
                    }
                    let ids: Vec<u64> = acf_array(value).iter().map(|(_, v)| intval(Some(v)).max(0) as u64).collect();
                    let mut users = site.db.users_by_ids(&ids).await?;
                    if users.is_empty() {
                        return Ok(Value::Bool(false));
                    }
                    users.sort_by_key(|u| ids.iter().position(|id| *id == u.id));
                    let mut out = Vec::new();
                    for user in users {
                        out.push(match return_format.as_str() {
                            // A `WP_User` object is not rebuilt; it comes out like the array.
                            "array" | "object" => {
                                let meta = |name: &'static str| async move { site.db.user_meta(user.id, name).await.map(|v| string(&v.unwrap_or_default())) };
                                Value::Array(vec![
                                    (key("ID"), Value::Int(user.id as i64)),
                                    (key("user_firstname"), meta("first_name").await?),
                                    (key("user_lastname"), meta("last_name").await?),
                                    (key("nickname"), meta("nickname").await?),
                                    (key("user_nicename"), string(&user.user_nicename)),
                                    (key("display_name"), string(&user.display_name)),
                                    (key("user_email"), string(&user.user_email)),
                                    (key("user_url"), string(&user.user_url)),
                                    (key("user_registered"), string(&user.user_registered)),
                                    (key("user_description"), meta("description").await?),
                                    (key("user_avatar"), string(&self.services.avatar(&user.user_email, 96))),
                                ])
                            }
                            _ => Value::Int(user.id as i64),
                        });
                    }
                    if multiple { list(out) } else { out.into_iter().next().unwrap_or(Value::Null) }
                }
                _ => value,
            })
        })
    }
}

/// `acf_esc_html()` over a formatted value (`map_deep()`): every scalar through `wp_kses()` with the tags of posts;
/// numbers and booleans become strings.
pub fn esc_html_deep(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(|(k, v)| (k, esc_html_deep(v))).collect()),
        Value::Null => Value::Bool(false),
        other => string(&rustpress_core::kses::kses_post(&strval(Some(&other)))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_breaks_like_nl2br() {
        assert_eq!(nl2br("a\nb\r\nc\n\rd\re"), "a<br />\nb<br />\r\nc<br />\n\rd<br />\re");
        assert_eq!(nl2br("a\n\nb"), "a<br />\n<br />\nb");
    }

    #[test]
    fn colors_as_arrays() {
        let rgba = |r, g, b, a| Value::Array(vec![(key("red"), Value::Int(r)), (key("green"), Value::Int(g)), (key("blue"), Value::Int(b)), (key("alpha"), Value::Float(a))]);
        assert_eq!(color_to_array(&string("#ff0080")), rgba(255, 0, 128, 1.0));
        assert_eq!(color_to_array(&string("#F08")), rgba(255, 0, 136, 1.0));
        assert_eq!(color_to_array(&string("rgba(1,2,3,0.5)")), rgba(1, 2, 3, 0.5));
        assert_eq!(color_to_array(&string("RGB(1,2,3)")), rgba(1, 2, 3, 1.0));
        assert_eq!(color_to_array(&string("red")), rgba(0, 0, 0, 0.0));
    }

    #[test]
    fn numeric_values() {
        assert_eq!(get_numeric(&string("5")), Value::Int(5));
        assert_eq!(get_numeric(&string("x")), Value::Bool(false));
        assert_eq!(get_numeric(&Value::Array(vec![(Key::Int(0), string("1")), (Key::Int(1), string("a")), (Key::Int(2), Value::Int(2))])), list([Value::Int(1), Value::Int(2)]));
    }

    #[test]
    fn stored_dates() {
        assert_eq!(strtotime("20240131"), Some(1_706_659_200));
        assert_eq!(strtotime("2024-01-31 10:00:00"), Some(1_706_695_200));
    }
}
