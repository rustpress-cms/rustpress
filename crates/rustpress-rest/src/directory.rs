//! The pattern directory of wordpress.org (`wp/v2/pattern-directory/patterns`). RustPress asks no other server; it
//! answers from the site transients WordPress keeps (`wp_remote_block_patterns_<hash>`), the same way, and the
//! patterns of `_load_remote_block_patterns()` and `_load_remote_featured_patterns()` come from there.

use rustpress_theme::registered_patterns::PatternEntry;
use serde_json::{Value, json};

use crate::Rest;
use crate::params::Params;

/// PHP's `serialize()` of the scalar and list values the query arguments have.
enum Arg {
    Int(i64),
    Str(String),
    List(Vec<String>),
}

fn php_serialize(args: &[(String, Arg)]) -> String {
    let string = |s: &str| format!("s:{}:\"{s}\";", s.len());
    let mut out = format!("a:{}:{{", args.len());
    for (key, value) in args {
        out.push_str(&string(key));
        match value {
            Arg::Int(n) => out.push_str(&format!("i:{n};")),
            Arg::Str(s) => out.push_str(&string(s)),
            Arg::List(items) => {
                out.push_str(&format!("a:{}:{{", items.len()));
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&format!("i:{i};{}", string(item)));
                }
                out.push('}');
            }
        }
    }
    out.push('}');
    out
}

/// `wp_kses_post()` of block content (`filter_block_content()`): the strings of the attributes and the HTML are
/// cleaned, and the blocks are written again, which escapes the attributes the way `serialize_block_attributes()` does.
fn filter_block_content(content: &str) -> String {
    fn clean_value(value: &mut Value) {
        match value {
            Value::String(s) => *s = rustpress_core::kses::normalize_entities(s),
            Value::Array(items) => items.iter_mut().for_each(clean_value),
            Value::Object(map) => map.values_mut().for_each(clean_value),
            _ => {}
        }
    }
    fn clean(block: &mut rustpress_blocks::parser::Block) {
        block.attrs.values_mut().for_each(clean_value);
        block.inner_html = rustpress_core::kses::normalize_entities(&block.inner_html);
        for piece in &mut block.inner_content {
            if let rustpress_blocks::parser::Content::Html(html) = piece {
                *html = rustpress_core::kses::normalize_entities(html);
            }
        }
        block.inner_blocks.iter_mut().for_each(clean);
    }
    let mut blocks = rustpress_blocks::parser::parse_blocks(content);
    blocks.iter_mut().for_each(clean);
    rustpress_blocks::serializer::serialize_blocks(&blocks)
}

impl Rest<'_> {
    /// `wp_get_wp_version()`, from `wp-includes/version.php`.
    fn wp_version(&self) -> String {
        let text = std::fs::read_to_string(self.theme.wp_root.join("wp-includes").join("version.php")).unwrap_or_default();
        text.lines().find_map(|line| line.trim().strip_prefix("$wp_version").and_then(|rest| rest.split('\'').nth(1)).map(str::to_owned)).unwrap_or_else(|| "7.1.3".to_owned())
    }

    /// `WP_REST_Pattern_Directory_Controller::get_transient_key()` for a request.
    fn directory_key(&self, params: &Params) -> String {
        let mut args: Vec<(String, Arg)> = vec![("page".into(), Arg::Int(1)), ("per_page".into(), Arg::Int(100)), ("order".into(), Arg::Str("desc".into())), ("orderby".into(), Arg::Str("date".into()))];
        let int_of = |key: &str| params.get(key).and_then(|v| v.trim().parse::<i64>().ok());
        for (name, value) in params.raw().split('&').filter_map(|p| p.split_once('=')).map(|(k, v)| (rustpress_core::rewrite::urldecode(k.as_bytes()), v)) {
            let name = String::from_utf8_lossy(&name).into_owned();
            let name = name.trim_end_matches("[]").to_owned();
            let arg = match name.as_str() {
                "page" | "per_page" | "offset" => int_of(&name).map(Arg::Int),
                "order" | "orderby" | "search" => params.get(&name).map(|v| Arg::Str(v.to_owned())),
                "slug" => params.list("slug").map(|mut list| {
                    list.sort();
                    Arg::List(list)
                }),
                _ => continue,
            };
            let Some(arg) = arg else { continue };
            match args.iter_mut().find(|(k, _)| *k == name) {
                Some(slot) => slot.1 = arg,
                None => args.push((name, arg)),
            }
            let _ = value;
        }
        args.push(("locale".into(), Arg::Str(self.site.locale())));
        args.push(("wp-version".into(), Arg::Str(self.wp_version())));
        if let Some(category) = int_of("category") {
            args.push(("pattern-categories".into(), Arg::Int(category)));
        }
        if let Some(keyword) = int_of("keyword") {
            args.push(("pattern-keywords".into(), Arg::Int(keyword)));
        }
        // `array_filter()`: nothing empty, no zero.
        args.retain(|(_, v)| match v {
            Arg::Int(n) => *n != 0,
            Arg::Str(s) => !s.is_empty() && s != "0",
            Arg::List(l) => !l.is_empty(),
        });
        format!("wp_remote_block_patterns_{:x}", md5::compute(php_serialize(&args)))
    }

    /// The raw items of the pattern directory for a request, from the site transient.
    async fn directory_raw(&self, params: &Params) -> Vec<Value> {
        let name = format!("_site_transient_{}", self.directory_key(params));
        let Some(raw) = self.site.options.fetch(&self.site.db, &name).await.ok().flatten() else { return Vec::new() };
        let Ok(value) = rustpress_phpser::unserialize(raw.as_bytes()) else { return Vec::new() };
        match rustpress_core::options::php_to_json(&value) {
            Value::Array(items) => items,
            _ => Vec::new(),
        }
    }

    /// `WP_REST_Pattern_Directory_Controller::prepare_item_for_response()` for the patterns of a request.
    pub(crate) async fn directory_items(&self, params: &Params) -> Vec<Value> {
        let sanitize = |s: &str| rustpress_content::sanitize::text_field(s);
        let text = |v: &Value, path: &str| v.pointer(path).and_then(Value::as_str).unwrap_or_default().to_owned();
        let absint = |v: Option<&Value>| v.and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))).map_or(0, i64::unsigned_abs);
        let mut out = Vec::new();
        for raw in self.directory_raw(params).await {
            let categories: Vec<String> = raw.get("category_slugs").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(rustpress_core::slug::sanitize_title).collect()).unwrap_or_default();
            let keywords: Vec<String> = text(&raw, "/meta/wpop_keywords").split(',').map(sanitize).collect();
            let block_types: Vec<String> = raw.pointer("/meta/wpop_block_types").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(sanitize).collect()).unwrap_or_default();
            out.push(json!({
                "id": absint(raw.get("id")),
                "title": sanitize(&text(&raw, "/title/rendered")),
                "content": filter_block_content(&text(&raw, "/pattern_content")),
                "categories": categories,
                "keywords": keywords,
                "description": sanitize(&text(&raw, "/meta/wpop_description")),
                "viewport_width": absint(raw.pointer("/meta/wpop_viewport_width")),
                "block_types": block_types,
            }));
        }
        out
    }

    /// `_load_remote_block_patterns()` and `_load_remote_featured_patterns()`: the entries they register.
    pub(crate) async fn remote_pattern_entries(&self, local: &[PatternEntry]) -> Vec<PatternEntry> {
        let mut out: Vec<PatternEntry> = Vec::new();
        let entry = |item: &Value, name: String, source: &str| PatternEntry {
            name,
            title: item["title"].as_str().unwrap_or_default().to_owned(),
            content: item["content"].as_str().unwrap_or_default().to_owned(),
            description: item["description"].as_str().unwrap_or_default().to_owned(),
            viewport_width: item["viewport_width"].as_i64(),
            categories: item["categories"].as_array().map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default(),
            keywords: item["keywords"].as_array().map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default(),
            block_types: item["block_types"].as_array().map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default(),
            source: Some(source.to_owned()),
            all_fields: true,
            ..Default::default()
        };
        // The keyword "core" is 11.
        for item in self.directory_items(&Params::parse("keyword=11")).await {
            let name = format!("core/{}", rustpress_core::slug::sanitize_title(item["title"].as_str().unwrap_or_default()));
            // A pattern registered again under its name keeps its place and takes the new data.
            let new = entry(&item, name.clone(), "pattern-directory/core");
            match out.iter_mut().find(|p| p.name == name) {
                Some(existing) => *existing = new,
                None => out.push(new),
            }
        }
        // The category "Featured" is 26; what is registered already as a core pattern stays.
        for item in self.directory_items(&Params::parse("category=26")).await {
            let name = rustpress_core::slug::sanitize_title(item["title"].as_str().unwrap_or_default());
            let taken = |n: &str| out.iter().chain(local).any(|p| p.name == n);
            if !taken(&name) && !taken(&format!("core/{name}")) {
                out.push(entry(&item, name, "pattern-directory/featured"));
            }
        }
        out
    }
}
