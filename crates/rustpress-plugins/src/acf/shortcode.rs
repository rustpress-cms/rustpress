//! The `[acf field="…" post_id="…" format_value="…"]` shortcode (`acf_shortcode()`): the escaped, formatted value
//! of a field, lists joined with commas.

use rustpress_core::Result;
use rustpress_core::native::ShortcodeContext;
use rustpress_core::query::Queried;
use rustpress_core::shortcodes::Atts;
use rustpress_db::MetaKind;
use rustpress_phpser::Value;

use super::fields::{Field, FieldGroup};
use super::format::{Formatter, esc_html_deep};
use super::php::{intval, is_numeric, string, strval, truthy};
use super::value::Object;

/// PHP's `version_compare( $a, $b ) < 0` for plain dotted versions: a shorter version with the same parts is
/// smaller (`6.3` < `6.3.0`).
fn version_less(a: &str, b: &str) -> bool {
    let parts = |v: &str| v.split(['.', '-', '_', '+']).map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parts(a) < parts(b)
}

/// `acf_get_setting( 'enable_shortcode' )`: on, unless ACF was first activated in version 6.3 or later.
pub fn enabled(options: &rustpress_core::options::Options) -> bool {
    match options.get("acf_first_activated_version").filter(|v| !v.is_empty() && *v != "0") {
        Some(first) => version_less(first, "6.3"),
        None => true,
    }
}

/// What `acf_get_valid_post_id()` and `acf_decode_post_id()` make of a `post_id`.
#[derive(Debug, Clone, Copy)]
enum Target {
    Object(Object),
    /// Options, blocks, widgets and the like: no meta table RustPress reads.
    Other,
    None,
}

impl Target {
    /// `(int) $post_id`.
    fn as_int(self) -> i64 {
        match self {
            Target::Object(Object { kind: MetaKind::Post, id }) => id as i64,
            _ => 0,
        }
    }
}

/// `acf_decode_post_id()` of a `post_id` attribute.
fn decode(site: &rustpress_core::Site, text: &str) -> Target {
    if is_numeric(&string(text)) {
        return Target::Object(Object { kind: MetaKind::Post, id: intval(Some(&string(text))).unsigned_abs() });
    }
    let (kind, id) = match text.rfind('_') {
        Some(i) if i > 0 => (&text[..i], &text[i + 1..]),
        _ => (text, ""),
    };
    let numeric = is_numeric(&string(id));
    let id_value = intval(Some(&string(id))).unsigned_abs();
    let object = |kind| Target::Object(Object { kind, id: id_value });
    match (kind, numeric) {
        ("post" | "attachment" | "menu_item", true) => object(MetaKind::Post),
        ("term" | "menu", true) => object(MetaKind::Term),
        ("comment", true) => object(MetaKind::Comment),
        ("user", true) => object(MetaKind::User),
        (taxonomy, true) if site.types.is_taxonomy(taxonomy) => object(MetaKind::Term),
        _ => Target::Other,
    }
}

/// `acf_get_valid_post_id()` without an argument: `get_the_ID()`, else the queried object.
fn current(ctx: &ShortcodeContext<'_>) -> Target {
    if let Some(id) = ctx.loop_post.or(ctx.global_post).filter(|id| *id != 0) {
        return Target::Object(Object { kind: MetaKind::Post, id });
    }
    match &ctx.query.queried {
        Queried::Post(post) => Target::Object(Object { kind: MetaKind::Post, id: post.id }),
        Queried::Term(term) => Target::Object(Object { kind: MetaKind::Term, id: term.term_id }),
        Queried::User(user) => Target::Object(Object { kind: MetaKind::User, id: user.id }),
        _ => Target::None,
    }
}

/// `acf_get_field( $key )`: a field of any group by its key, sub fields too.
fn field_by_key<'a>(fields: &'a [Field], field_key: &str) -> Option<&'a Field> {
    for field in fields {
        if field.key == field_key {
            return Some(field);
        }
        if let Some(found) = field_by_key(&field.sub_fields, field_key) {
            return Some(found);
        }
    }
    None
}

/// `acf_maybe_get_field()`: a field key, or the reference (`_{name}`) the object keeps for a field name; with the
/// name the value is read under.
async fn find_field<'a>(site: &rustpress_core::Site, groups: &'a [FieldGroup], selector: &str, target: Target) -> Result<Option<(&'a Field, String)>> {
    let by_key = |k: &str| groups.iter().find_map(|g| field_by_key(&g.fields, k));
    if selector.starts_with("field_") {
        return Ok(by_key(selector).map(|f| (f, f.name.clone())));
    }
    let Target::Object(object) = target else { return Ok(None) };
    if object.id == 0 {
        return Ok(None);
    }
    let reference = site.db.acf_meta(object.kind, object.id, &format!("_{selector}")).await?;
    Ok(reference.and_then(|k| by_key(&k)).map(|f| (f, selector.to_owned())))
}

/// A value as the text a shortcode returns (`(string)`; lists joined with `, `).
fn output(value: Value) -> String {
    match value {
        Value::Array(items) => items.iter().map(|(_, v)| strval(Some(v))).collect::<Vec<_>>().join(", "),
        other => strval(Some(&other)),
    }
}

/// `acf_shortcode()`.
pub async fn render(groups: &[FieldGroup], enabled: bool, atts: &Atts, ctx: &ShortcodeContext<'_>) -> Result<String> {
    let site = ctx.site;
    let preview = ctx.query.flags.is_preview;
    let message = |text: &str, brackets: bool| {
        if !preview {
            return String::new();
        }
        let text = rustpress_core::formatting::esc_html(&site.trd("acf", text));
        if brackets { format!("[{text}]") } else { text }
    };
    if !enabled {
        return Ok(message("[The ACF shortcode is disabled on this site]", false));
    }
    // Not in the template parts of a block theme, only in `the_content`.
    if ctx.block_theme && !ctx.in_the_content {
        return Ok(String::new());
    }
    let viewer = ctx.query.viewer.as_ref();
    if preview && !viewer.is_some_and(|v| v.can("publish_posts")) {
        return Ok(message("[ACF shortcode value disabled for preview]", false));
    }
    let selector = atts.get("field").unwrap_or("").to_owned();
    let format_value = atts.get("format_value").is_none_or(|v| truthy(Some(&string(v))));
    let here = current(ctx);
    let given = atts.get("post_id");
    let target = match given {
        Some(text) if truthy(Some(&string(text))) => decode(site, text),
        _ => here,
    };
    // Fields of another post only when that post is public.
    if let (Some(text), Target::Object(Object { kind: MetaKind::Post, id })) = (given, target)
        && intval(Some(&string(text))) != here.as_int()
    {
        let public = match site.db.post(id).await? {
            Some(post) => site.publicly_viewable(&post).await?,
            None => false,
        };
        if !public {
            return Ok(message("[The ACF shortcode cannot display fields from non-public posts]", false));
        }
    }
    let found = find_field(site, groups, &selector, target).await?;
    let Some((field, name)) = found else {
        // `get_field()` with a dummy field: unknown fields are not read.
        return Ok(String::new());
    };
    if !super::fields::supports_bindings(&field.kind) {
        return Ok(message("The requested ACF field type does not support output in bindings or the ACF Shortcode.", true));
    }
    if field.get("allow_in_bindings").is_some_and(|v| !matches!(v, Value::Null) && !truthy(Some(v))) {
        return Ok(message("The requested ACF field is not allowed to be output in bindings or the ACF Shortcode.", true));
    }
    let Target::Object(object) = target else { return Ok(String::new()) };
    if !format_value {
        return Ok(String::new());
    }
    let raw = super::value::get_value(&site.db, object, field, &name).await?;
    let global = match ctx.global_post {
        Some(id) => site.db.post(id).await?,
        None => None,
    };
    let formatter = Formatter { site, services: ctx.services, global_post: global.as_ref() };
    let value = if matches!(field.kind.as_str(), "oembed" | "url" | "wysiwyg") {
        formatter.format(field, raw, true).await?
    } else {
        esc_html_deep(formatter.format(field, raw, false).await?)
    };
    Ok(output(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(version_less("5.12.3", "6.3"));
        assert!(version_less("6.2.9", "6.3"));
        assert!(!version_less("6.3", "6.3"));
        assert!(!version_less("6.8.10", "6.3"));
    }

    #[test]
    fn lists_are_joined() {
        let list = Value::Array(vec![(rustpress_phpser::Key::Int(0), string("a")), (rustpress_phpser::Key::Int(1), Value::Int(2))]);
        assert_eq!(output(list), "a, 2");
        assert_eq!(output(Value::Bool(false)), "");
    }
}
