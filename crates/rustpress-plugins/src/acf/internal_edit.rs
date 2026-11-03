//! The editors of ACF's post types and taxonomies (`post.php`/`post-new.php` of `acf-post-type` and `acf-taxonomy`):
//! `ACF_Admin_Post_Type` and `ACF_Admin_Taxonomy` – the scripts and texts, the boxes "Basic Settings" and "Advanced
//! Settings" (`views/acf-post-type/*.php`, `views/acf-taxonomy/*.php`), the form data and saving
//! (`save_post()` with `acf_update_internal_post_type()`).

use rustpress_core::Result;
use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_core::native::{AdminContext, Assets, FormPart, PluginMetaBox, PostMetaBoxes, PostSaved};
use rustpress_db::Post;
use rustpress_phpser::{Key, Value};

use super::php::{key, string, strval, truthy};
use super::render::{Ctx, RenderField, render_field_wrap};

const PLUGIN_DIR: &str = "wp-content/plugins/advanced-custom-fields";

/// The two kinds of items these editors edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    PostType,
    Taxonomy,
}

impl Kind {
    pub fn of(post_type: &str) -> Option<Self> {
        match post_type {
            "acf-post-type" => Some(Self::PostType),
            "acf-taxonomy" => Some(Self::Taxonomy),
            _ => None,
        }
    }

    pub fn post_type(self) -> &'static str {
        match self {
            Self::PostType => "acf-post-type",
            Self::Taxonomy => "acf-taxonomy",
        }
    }

    /// `post_type`/`taxonomy`: the screen of `acf_form_data()`, the nonce and the name of the key setting.
    fn screen(self) -> &'static str {
        match self {
            Self::PostType => "post_type",
            Self::Taxonomy => "taxonomy",
        }
    }

    /// The `$_POST` array of the form.
    fn prefix(self) -> &'static str {
        match self {
            Self::PostType => "acf_post_type",
            Self::Taxonomy => "acf_taxonomy",
        }
    }

    fn defaults(self) -> Value {
        match self {
            Self::PostType => super::internal::post_type_defaults(),
            Self::Taxonomy => super::internal::taxonomy_defaults(),
        }
    }
}

fn tr(site: &rustpress_core::Site, text: &str) -> String {
    site.trd("acf", text)
}

fn esc_tr(site: &rustpress_core::Site, text: &str) -> String {
    esc_html(&tr(site, text))
}

/// `__( $text, 'acf' )` as a value.
fn t(site: &rustpress_core::Site, text: &str) -> Value {
    string(&tr(site, text))
}

fn arr(items: Vec<(&str, Value)>) -> Value {
    Value::Array(items.into_iter().map(|(k, v)| (key(k), v)).collect())
}

fn list(items: Vec<Value>) -> Value {
    Value::Array(items.into_iter().enumerate().map(|(i, v)| (Key::Int(i as i64), v)).collect())
}

/// `.` of strings.
fn cat(parts: Vec<Value>) -> Value {
    string(&parts.iter().map(|p| strval(Some(p))).collect::<String>())
}

/// `$acf_post_type['a']['b']` (`null` when it is not set).
fn g(pt: &Value, path: &[&str]) -> Value {
    let mut at = pt;
    for name in path {
        match at.get(name) {
            Some(next) => at = next,
            None => return Value::Null,
        }
    }
    at.clone()
}

/// `isset( $acf_post_type['a']['b'] ) ? $acf_post_type['a']['b'] : $default`.
fn g_or(pt: &Value, path: &[&str], default: Value) -> Value {
    match g(pt, path) {
        Value::Null => default,
        value => value,
    }
}

/// `acf_render_field_wrap( $field, $element, $instruction )`.
async fn wrap(r: &Ctx<'_>, items: Vec<(&str, Value)>, element: &str, instruction: &str) -> Result<String> {
    render_field_wrap(r, RenderField::raw(items.into_iter().map(|(k, v)| (key(k), v)).collect(), Vec::new()), element, instruction).await
}

/// `acf_get_combined_post_type_settings_tabs()` and `acf_get_combined_taxonomy_settings_tabs()` (the same tabs).
fn settings_tabs(site: &rustpress_core::Site) -> Vec<(&'static str, String)> {
    [("general", "General"), ("labels", "Labels"), ("visibility", "Visibility"), ("urls", "URLs"), ("permissions", "Permissions"), ("rest_api", "REST API")].iter().map(|(k, label)| (*k, tr(site, label))).collect()
}

/// The labels bar of the "Labels" tab.
fn regenerate_labels_bar(site: &rustpress_core::Site) -> String {
    format!(
        "<div class=\"acf-field acf-regenerate-labels-bar\"><span class=\"acf-btn acf-btn-sm acf-btn-clear acf-regenerate-labels\"><i class=\"acf-icon acf-icon-regenerate\"></i>{}</span><span class=\"acf-btn acf-btn-sm acf-btn-clear acf-clear-labels\"><i class=\"acf-icon acf-icon-trash\"></i>{}</span><span class=\"acf-tip acf-labels-tip\"><i class=\"acf-icon acf-icon-help acf-js-tooltip\" title=\"{}\"></i></span></div>",
        esc_tr(site, "Regenerate"),
        esc_tr(site, "Clear"),
        esc_attr(&tr(site, "Regenerate all labels using the Singular and Plural labels"))
    )
}

/// The hidden inputs at the end of the basic settings.
fn hidden_keys(kind: Kind, pt: &Value) -> String {
    let prefix = kind.prefix();
    let value = |name: &str| esc_attr(&strval(Some(&g(pt, &[name]))));
    // The taxonomy view is indented one level deeper.
    let i = if kind == Kind::Taxonomy { "\t" } else { "" };
    format!(
        "{i}<div class=\"acf-hidden\">\n{i}\t<input type=\"hidden\" name=\"{prefix}[key]\" value=\"{}\" />\n{i}\t<input type=\"hidden\" name=\"{prefix}[import_source]\" value=\"{}\" />\n{i}\t<input type=\"hidden\" name=\"{prefix}[import_date]\" value=\"{}\" />\n{i}</div>\n",
        value("key"),
        value("import_source"),
        value("import_date")
    )
}

/// `array_unique( array_filter( $values ) )` with the keys kept.
fn unique_filtered(values: &Value) -> Value {
    let Value::Array(items) = values else { return Value::Array(Vec::new()) };
    let mut out: Vec<(Key, Value)> = Vec::new();
    for (k, v) in items {
        if !truthy(Some(v)) || out.iter().any(|(_, seen)| strval(Some(seen)) == strval(Some(v))) {
            continue;
        }
        out.push((k.clone(), v.clone()));
    }
    Value::Array(out)
}

/// Every capability of every role, once (`array_unique( array_keys( array_merge( … ) ) )`), as choices.
fn all_capabilities(site: &rustpress_core::Site) -> Value {
    let mut caps: Vec<String> = Vec::new();
    if let Some(Value::Array(roles)) = site.options.value(&format!("{}user_roles", site.db.table_prefix())) {
        for (_, role) in roles {
            if let Some(Value::Array(role_caps)) = role.get("capabilities") {
                for (cap, _) in role_caps {
                    let cap = super::render::key_text(cap);
                    if !caps.contains(&cap) {
                        caps.push(cap);
                    }
                }
            }
        }
    }
    Value::Array(caps.iter().map(|c| (key(c), string(c))).collect())
}

/// The ACF post type or taxonomy of `post.php`/`post-new.php`.
pub fn edited<'a>(ctx: &AdminContext<'a>) -> Option<(&'a Post, Kind)> {
    if !matches!(ctx.hook_suffix, "post.php" | "post-new.php") {
        return None;
    }
    let post = ctx.edited_post?;
    Some((post, Kind::of(&post.post_type)?))
}

/// `ACF_Admin_Internal_Post_Type::admin_body_class()`.
pub fn admin_body_class(ctx: &AdminContext<'_>, classes: String) -> String {
    match edited(ctx) {
        Some((_, Kind::PostType)) => format!("{classes} acf-admin-page acf-internal-post-type acf-admin-single-post-type"),
        Some((_, Kind::Taxonomy)) => format!("{classes} acf-admin-page acf-internal-post-type acf-admin-single-taxonomy"),
        None => classes,
    }
}

/// `ACF_Admin_Internal_Post_Type::current_screen()` (`acf_enqueue_scripts()`) and the `admin_enqueue_scripts()` of
/// the post type and taxonomy editors: `acf-field-group`, the texts, `acf-internal-post-type`, no `autosave`.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    let Some((_, kind)) = edited(ctx) else { return };
    let site = ctx.site;
    let texts: &[&str] = match kind {
        Kind::PostType => &["Post", "Posts", "Page", "Pages", "Default"],
        Kind::Taxonomy => &["Tag", "Tags", "Category", "Categories", "Default"],
    };
    let texts: Vec<(&'static str, String)> = texts.iter().map(|text| (*text, esc_tr(site, text))).collect();
    super::form::enqueue_with_texts(site, ctx.wp_version, assets, true, &texts);
    let url = |path: &str| site.site_url(&format!("{PLUGIN_DIR}/{path}"));
    let mut own = Assets::default();
    own.style("acf-field-group", url("assets/build/css/acf-field-group.min.css"), &["acf-input"], super::VERSION, "all");
    own.script("acf-internal-post-type", url("assets/build/js/acf-internal-post-type.min.js"), &["acf-input"], super::VERSION, false);
    for mut asset in own.styles {
        asset.priority = 20;
        assets.styles.push(asset);
    }
    for mut asset in own.scripts {
        asset.priority = 20;
        assets.scripts.push(asset);
    }
    assets.dequeue.push("autosave".into());
}

/// `get_user_option_screen_layout_acf-post-type` and `_acf-taxonomy`: one column.
pub fn screen_layout(screen_id: &str) -> Option<u32> {
    matches!(screen_id, "acf-post-type" | "acf-taxonomy").then_some(1)
}

/// `acf_request_arg( $name )`: `$_REQUEST`.
fn request<'a>(ctx: &'a AdminContext<'_>, name: &str) -> Option<&'a str> {
    ctx.post_field(name).or_else(|| ctx.query.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str()))
}

/// `acf_get_post_type_from_request_args( $action )` and `acf_get_taxonomy_from_request_args( $action )`: the item of
/// `use_post_type`/`use_taxonomy` when the nonce `{$action}-{$id}` is valid.
pub(super) async fn from_request_args(ctx: &AdminContext<'_>, kind: Kind, action: &str) -> Result<Option<Value>> {
    let name = match kind {
        Kind::PostType => "use_post_type",
        Kind::Taxonomy => "use_taxonomy",
    };
    let id = request(ctx, name).map_or(0, |v| super::php::intval(Some(&string(v))));
    if id <= 0 || !request(ctx, "_wpnonce").is_some_and(|nonce| (ctx.verify_nonce)(nonce, &format!("{action}-{id}"))) {
        return Ok(None);
    }
    Ok(ctx.site.db.post(id as u64).await?.filter(|p| p.post_type == kind.post_type()).map(|p| loaded(&p, kind)))
}

/// `acf_get_internal_post_type( $id, $post_type )` of a post.
fn loaded(post: &Post, kind: Kind) -> Value {
    let acf_post = rustpress_db::AcfPost { id: post.id, parent: post.post_parent, title: post.post_title.clone(), name: post.post_name.clone(), excerpt: post.post_excerpt.clone(), menu_order: i64::from(post.menu_order), status: post.post_status.clone(), content: post.post_content.clone() };
    super::internal::validated(&acf_post, kind.defaults())
}

/// The item the boxes show: the post, or – with `acfduplicate` – the one it duplicates with the names emptied
/// (`basic-settings.php`), and the linked types of `create-post-type`/`create-taxonomy`; a key that is not one of the
/// type becomes a new `uniqid()` (`mb_basic_settings()`).
async fn edited_item(ctx: &AdminContext<'_>, post: &Post, kind: Kind) -> Result<Value> {
    let mut pt = loaded(post, kind);
    let key_prefix = match kind {
        Kind::PostType => "post_type_",
        Kind::Taxonomy => "taxonomy_",
    };
    if !strval(pt.get("key")).starts_with(key_prefix) {
        set(&mut pt, &["key"], string(&super::render::uniqid(key_prefix)));
    }
    if let Some(mut duplicate) = from_request_args(ctx, kind, "acfduplicate").await? {
        set(&mut duplicate, &["key"], string(&super::render::uniqid(key_prefix)));
        set(&mut duplicate, &["title"], string(""));
        if let Some(Value::Array(labels)) = duplicate.get("labels").cloned() {
            set(&mut duplicate, &["labels"], Value::Array(labels.into_iter().map(|(k, _)| (k, string(""))).collect()));
        }
        let names: &[&str] = match kind {
            Kind::PostType => &["post_type", "rest_base", "query_var_name"],
            Kind::Taxonomy => &["taxonomy", "query_var_name", "rest_base"],
        };
        for name in names {
            set(&mut duplicate, &[name], string(""));
        }
        set(&mut duplicate, &["rewrite", "slug"], string(""));
        pt = duplicate;
    }
    let (other, action, setting, target) = match kind {
        Kind::PostType => (Kind::Taxonomy, "create-post-type", "taxonomy", "taxonomies"),
        Kind::Taxonomy => (Kind::PostType, "create-taxonomy", "post_type", "object_type"),
    };
    if let Some(linked) = from_request_args(ctx, other, action).await?
        && truthy(linked.get(setting))
    {
        set(&mut pt, &[target], list(vec![linked.get(setting).cloned().unwrap_or(Value::Null)]));
    }
    Ok(pt)
}

/// `$array['a']['b'] = $value`.
fn set(target: &mut Value, path: &[&str], value: Value) {
    let Value::Array(items) = target else { return };
    let Some((first, rest)) = path.split_first() else { return };
    if rest.is_empty() {
        super::php::set(items, first, value);
        return;
    }
    let mut inner = items.iter().find(|(k, _)| super::php::key_eq(k, &key(first))).map(|(_, v)| v.clone()).filter(|v| matches!(v, Value::Array(_))).unwrap_or(Value::Array(Vec::new()));
    set(&mut inner, rest, value);
    super::php::set(items, first, inner);
}

/// `ACF_Admin_Post_Type::admin_head()` and `ACF_Admin_Taxonomy::admin_head()`: the boxes "Basic Settings" and
/// "Advanced Settings", and the form data after the title.
pub async fn meta_boxes(ctx: &AdminContext<'_>, post: &Post) -> Result<PostMetaBoxes> {
    let Some((_, kind)) = edited(ctx) else { return Ok(PostMetaBoxes::default()) };
    let site = ctx.site;
    let services = super::edit_comment::AdminServices(ctx.wp_root.to_owned());
    let render = super::edit_post::render_ctx(ctx, &services);
    let pt = edited_item(ctx, post, kind).await?;
    // `acf_get_setting( 'enable_meta_box_cb_edit' )`: `is_super_admin()`.
    let cb_edit = (ctx.can)("delete_users");
    let (basic, advanced) = match kind {
        Kind::PostType => (post_type_basic(&render, &pt).await?, post_type_advanced(&render, &pt, cb_edit).await?),
        Kind::Taxonomy => (taxonomy_basic(&render, &pt).await?, taxonomy_advanced(&render, &pt, cb_edit).await?),
    };
    let boxes = vec![
        PluginMetaBox { id: "acf-basic-settings".into(), title: tr(site, "Basic Settings"), context: "normal".into(), priority: "high".into(), html: basic },
        PluginMetaBox { id: "acf-advanced-settings".into(), title: tr(site, "Advanced Settings"), context: "normal".into(), priority: "high".into(), html: advanced },
    ];
    let form_data = super::form::form_data_with(kind.screen(), Value::Int(post.id as i64), &[("validation", Value::Int(1)), ("delete_fields", Value::Int(0))], &(ctx.nonce)(kind.screen()));
    // The icon picker calls `acf_enqueue_uploader()`: the media library and `ACF_Assets::print_uploader_scripts()`.
    let uploader = render.wants_uploader.load(std::sync::atomic::Ordering::Relaxed);
    let hidden_editor = uploader.then(|| rustpress_core::native::HiddenEditor { id: "acf_content".into(), before: "\t\t<div id=\"acf-hidden-wp-editor\" style=\"display: none;\">\n\t\t\t".into(), after: "\t\t</div>\n\t\t\t".into() });
    Ok(PostMetaBoxes { boxes, removed: vec!["submitdiv".into()], after_title: vec![FormPart::Html(form_data)], hidden_editor, media: uploader, ..Default::default() })
}

/// `views/global/form-top.php` of the edit screens of field groups, post types and taxonomies: a field group made
/// for a post type or taxonomy (`add-fields`) gets "{singular} fields" as its title, a duplicate says what it
/// duplicates.
pub async fn form_top(ctx: &AdminContext<'_>) -> Result<String> {
    if !matches!(ctx.hook_suffix, "post.php" | "post-new.php") {
        return Ok(String::new());
    }
    let Some(post) = ctx.edited_post else { return Ok(String::new()) };
    let site = ctx.site;
    let singular = |item: Option<Value>| item.map(|i| strval(Some(&g(&i, &["labels", "singular_name"])))).filter(|s| !s.is_empty());
    if post.post_type == "acf-field-group" {
        let mut prefilled = None;
        for kind in [Kind::PostType, Kind::Taxonomy] {
            if let Some(name) = singular(from_request_args(ctx, kind, "add-fields").await?) {
                prefilled = Some(rustpress_core::i18n::sprintf(&tr(site, "%s fields"), &[&name]));
                break;
            }
        }
        return Ok(super::field_group_edit::form_top(ctx, post, prefilled));
    }
    let Some(kind) = Kind::of(&post.post_type) else { return Ok(String::new()) };
    let mut duplicated = String::new();
    for from in [Kind::PostType, Kind::Taxonomy] {
        if let Some(name) = singular(from_request_args(ctx, from, "acfduplicate").await?) {
            duplicated = format!("<span class=\"acf-duplicated-from\">{}</span>", esc_html(&format!(" {}", rustpress_core::i18n::sprintf(&tr(site, "(Duplicated from %s)"), &[&name]))));
            break;
        }
    }
    let label = if post.post_status == "auto-draft" { "add_new_item" } else { "edit_item" };
    let title = site.types.admin_post_types().into_iter().find(|t| t.name == kind.post_type()).and_then(|t| t.label(label)).unwrap_or_default();
    Ok(format!(
        "<div class=\"acf-headerbar acf-headerbar-field-editor\">\n\t<div class=\"acf-headerbar-inner\">\n\n\t\t<div class=\"acf-headerbar-content\">\n\t\t\t<h1 class=\"acf-page-title\">\n\t\t\t{}{duplicated}\t\t\t</h1>\n\t\t\t\t\t</div>\n\n\t\t<div class=\"acf-headerbar-actions\" id=\"submitpost\">\n\t\t\t\t\t\t<button form=\"post\" class=\"acf-btn acf-publish\" name=\"{}\" type=\"submit\">\n\t\t\t\t{}\t\t\t</button>\n\t\t</div>\n\n\t</div>\n</div>\n",
        esc_html(&title),
        if post.post_status == "publish" { "save" } else { "publish" },
        esc_tr(site, "Save Changes"),
    ))
}

/// `ACF_Admin_Post_Type::post_updated_messages()` and `ACF_Admin_Taxonomy::post_updated_messages()`, with the links
/// of `post_type_created_message()`/`taxonomy_saved_message()`.
pub fn post_updated_message(site: &rustpress_core::Site, post: &Post, number: u32, nonce: &dyn Fn(&str) -> String) -> Option<Option<String>> {
    let kind = Kind::of(&post.post_type)?;
    let simple = |text: &str| Some(Some(tr(site, text)));
    let saved = |created: bool| {
        let title = rustpress_blocks::content::the_title_in_admin(site, post);
        let id = post.id;
        let admin = |path: &str| site.site_url(&format!("wp-admin/{path}"));
        // `wp_nonce_url()`, printed with `esc_url()`.
        let nonce_url = |path: &str, action: &str| rustpress_core::formatting::esc_url(&esc_html(&format!("{}&_wpnonce={}", admin(path), nonce(&format!("{action}-{id}")))));
        let link = |href: &str, text: &str| format!("\t\t\t\t<a href=\"{href}\">{}</a>\n", esc_tr(site, text));
        let (text, links) = match kind {
            Kind::PostType => (
                rustpress_core::i18n::sprintf(&tr(site, if created { "%s post type created" } else { "%s post type updated" }), &[&title]),
                [
                    link(&nonce_url(&format!("post-new.php?post_type=acf-field-group&use_post_type={id}"), "add-fields"), "Add fields"),
                    format!("\t\t\t\t<a class=\"acf-link-field-groups\" href=\"#\">{}</a>\n", esc_tr(site, "Link field groups")),
                    link(&rustpress_core::formatting::esc_url(&admin("post-new.php?post_type=acf-post-type")), "Create post type"),
                    link(&nonce_url(&format!("post-new.php?post_type=acf-post-type&use_post_type={id}"), "acfduplicate"), "Duplicate post type"),
                    link(&nonce_url(&format!("post-new.php?post_type=acf-taxonomy&use_post_type={id}"), "create-taxonomy"), "Create taxonomy"),
                ],
            ),
            Kind::Taxonomy => (
                rustpress_core::i18n::sprintf(&tr(site, if created { "%s taxonomy created" } else { "%s taxonomy updated" }), &[&title]),
                [
                    link(&nonce_url(&format!("post-new.php?post_type=acf-field-group&use_taxonomy={id}"), "add-fields"), "Add fields"),
                    format!("\t\t\t\t<a class=\"acf-link-field-groups\" href=\"#\">{}</a>\n", esc_tr(site, "Link field groups")),
                    link(&rustpress_core::formatting::esc_url(&admin("post-new.php?post_type=acf-taxonomy")), "Create taxonomy"),
                    link(&nonce_url(&format!("post-new.php?post_type=acf-taxonomy&use_taxonomy={id}"), "acfduplicate"), "Duplicate taxonomy"),
                    link(&nonce_url(&format!("post-new.php?post_type=acf-post-type&use_taxonomy={id}"), "create-post-type"), "Create post type"),
                ],
            ),
        };
        Some(Some(format!("\t\t\t<p class=\"acf-item-saved-text\">{}</p>\n\t\t\t<div class=\"acf-item-saved-links\">\n{}\t\t\t</div>\n\t\t\t", esc_html(&text), links.concat())))
    };
    match (kind, number) {
        (_, 5) => Some(None),
        (Kind::PostType, 1 | 2) | (Kind::Taxonomy, 1 | 4) => saved(false),
        (_, 6) => saved(true),
        (Kind::PostType, 3) => simple("Post type deleted."),
        (Kind::PostType, 4) => simple("Post type updated."),
        (Kind::PostType, 7) => simple("Post type saved."),
        (Kind::PostType, 8) => simple("Post type submitted."),
        (Kind::PostType, 9) => simple("Post type scheduled for."),
        (Kind::PostType, 10) => simple("Post type draft updated."),
        (Kind::Taxonomy, 2) => simple("Taxonomy updated."),
        (Kind::Taxonomy, 3) => simple("Taxonomy deleted."),
        (Kind::Taxonomy, 7) => simple("Taxonomy saved."),
        (Kind::Taxonomy, 8) => simple("Taxonomy submitted."),
        (Kind::Taxonomy, 9) => simple("Taxonomy scheduled for."),
        (Kind::Taxonomy, 10) => simple("Taxonomy draft updated."),
        _ => Some(None),
    }
}

/// `wp_ajax_acf/link_field_groups` (`ACF_Admin_Internal_Post_Type::ajax_link_field_groups()`): the dialog with the
/// field groups, or – with `field_groups[]` – a location rule for the post type or taxonomy added to each of them.
pub async fn ajax_link_field_groups(ctx: &AdminContext<'_>) -> Result<rustpress_core::native::AjaxAnswer> {
    use rustpress_core::native::AjaxAnswer;
    let site = ctx.site;
    let empty = || AjaxAnswer::text(200, "");
    // `acf_parse_args( $_POST, … )`.
    let nonce = ctx.post_field("nonce").unwrap_or_default();
    let post_id = ctx.post_field("post_id").map_or(0, |v| super::php::intval(Some(&string(v))));
    if nonce.is_empty() || !(ctx.verify_nonce)(nonce, "acf_nonce") || !(ctx.can)("manage_options") || post_id <= 0 {
        return Ok(empty());
    }
    let encode = |data: serde_json::Value| rustpress_blocks::php_json::encode_with(&serde_json::json!({"success": true, "data": data}), rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: false });
    let saved = site.db.post(post_id as u64).await?;
    let post_type = saved.as_ref().map(|p| p.post_type.clone()).unwrap_or_default();
    let chosen: Vec<String> = match rustpress_core::native::form_value(ctx.form, "field_groups") {
        Some(Value::Array(items)) => items.into_iter().map(|(_, v)| strval(Some(&v))).collect(),
        _ => Vec::new(),
    };
    // `acf_get_internal_post_type( $post_id, $post_type )`: an ACF post type, taxonomy or options page.
    let saved_item = saved.as_ref().and_then(|p| Kind::of(&p.post_type).map(|kind| loaded(p, kind)));
    if let (false, Some(item), Some(writer)) = (chosen.is_empty(), &saved_item, ctx.writer) {
        let (param, setting) = if post_type == "acf-post-type" { ("post_type", "post_type") } else { ("taxonomy", "taxonomy") };
        let value = item.get(setting).cloned().unwrap_or(Value::Null);
        let system = rustpress_auth::CurrentUser::system();
        let content = rustpress_content::content::Content { site, db: &site.db, writer, user: &system, now: ctx.now };
        for id in &chosen {
            let id = super::php::intval(Some(&string(id)));
            let Some(group) = (if id > 0 { site.db.post(id as u64).await? } else { None }).filter(|p| p.post_type == "acf-field-group") else { continue };
            let acf_post = rustpress_db::AcfPost { id: group.id, parent: group.post_parent, title: group.post_title.clone(), name: group.post_name.clone(), excerpt: group.post_excerpt.clone(), menu_order: i64::from(group.menu_order), status: group.post_status.clone(), content: group.post_content.clone() };
            let mut items = super::internal::validated(&acf_post, Value::Array(super::save_group::group_defaults()));
            // `$field_group['location'][] = array( array( … ) )`.
            let mut location = match items.get("location").cloned() {
                Some(Value::Array(groups)) => groups,
                _ => Vec::new(),
            };
            let next = location.iter().filter_map(|(k, _)| if let Key::Int(i) = k { Some(*i + 1) } else { None }).max().unwrap_or(0);
            location.push((Key::Int(next), list(vec![arr(vec![("param", string(param)), ("operator", string("==")), ("value", value.clone())])])));
            set(&mut items, &["location"], Value::Array(location));
            let Value::Array(items) = super::php::wp_unslash(items) else { continue };
            super::save_group::update_internal_post(&content, None, "acf-field-group", items).await?;
        }
        let text = site.i18n().ngettext("acf", "Field group linked successfully.", "Field groups linked successfully.", chosen.len() as i64);
        let html = format!(
            "\t\t\t\t<p class=\"acf-link-successful\">\n\t\t\t\t\t{}\t\t\t\t</p>\n\t\t\t\t<div class=\"acf-actions\">\n\t\t\t\t\t<button type=\"button\" class=\"acf-btn acf-btn-secondary acf-close-popup\">{}</button>\n\t\t\t\t</div>\n\t\t\t\t",
            esc_html(&text),
            esc_tr(site, "Close Modal")
        );
        return Ok(AjaxAnswer::json(200, encode(serde_json::json!({"content": html}))));
    }
    let choices: Vec<(Key, Value)> = site.db.acf_posts("acf-field-group").await?.into_iter().filter(|g| g.id != 0).map(|g| (Key::Int(g.id as i64), string(&g.title))).collect();
    let what = if post_type == "acf-post-type" { tr(site, "post type") } else { tr(site, "taxonomy") };
    let instructions = rustpress_core::i18n::sprintf(&tr(site, "Add this %s to the location rules of the selected field groups."), &[&what]);
    let services = super::edit_comment::AdminServices(ctx.wp_root.to_owned());
    let render = super::edit_post::render_ctx(ctx, &services);
    let field = wrap(
        &render,
        vec![
            ("type", string("select")),
            ("name", string("acf_field_groups")),
            ("choices", Value::Array(choices)),
            ("aria-label", t(site, "Please select the field groups to link.")),
            ("placeholder", t(site, "Select one or many field groups...")),
            ("label", t(site, "Field Group(s)")),
            ("instructions", string(&instructions)),
            ("ui", Value::Bool(true)),
            ("multiple", Value::Bool(true)),
            ("allow_null", Value::Bool(true)),
        ],
        "div",
        "field",
    )
    .await?;
    let html = format!(
        "\t\t\t<form id=\"acf-link-field-groups-form\">\n\t\t\t\t{field}\t\t\t\t<div class=\"acf-actions\">\n\t\t\t\t\t<button type=\"button\" class=\"acf-btn acf-btn-secondary acf-close-popup\">{}</button>\n\t\t\t\t\t<button type=\"submit\" class=\"acf-btn acf-btn-primary\">{}</button>\n\t\t\t\t</div>\n\t\t\t</form>\n\t\t\t",
        esc_tr(site, "Cancel"),
        esc_tr(site, "Done")
    );
    Ok(AjaxAnswer::json(200, encode(serde_json::json!({"content": html, "title": esc_tr(site, "Link Existing Field Groups")}))))
}

/// `acf_get_wp_reserved_terms()`.
const RESERVED_TERMS: &[&str] = &[
    "action", "attachment", "attachment_id", "author", "author_name", "calendar", "cat", "category", "category__and", "category__in", "category__not_in", "category_name", "comments_per_page", "comments_popup", "custom", "customize_messenger_channel", "customized", "cpage", "day", "debug", "embed", "error", "exact", "feed", "fields", "hour", "link", "link_category", "m", "minute", "monthnum", "more", "name", "nav_menu", "nonce", "nopaging", "offset", "order", "orderby", "p", "page", "page_id", "paged", "pagename", "pb", "perm", "post", "post__in", "post__not_in", "post_format", "post_mime_type", "post_status", "post_tag", "post_type", "posts", "posts_per_archive_page", "posts_per_page", "preview", "robots", "s", "search", "second", "sentence", "showposts", "static", "status", "subpost", "subpost_id", "tag", "tag__and", "tag__in", "tag__not_in", "tag_id", "tag_slug__and", "tag_slug__in", "taxonomy", "tb", "term", "terms", "theme", "themes", "title", "type", "types", "w", "withcomments", "withoutcomments", "year",
];

/// The post types and taxonomies WordPress and ACF register before ACF's own (`post_type_exists()`,
/// `taxonomy_exists()` for keys no registration of the site names).
const CORE_POST_TYPES: [&str; 21] = [
    "post", "page", "attachment", "revision", "nav_menu_item", "custom_css", "customize_changeset", "oembed_cache", "user_request", "wp_block", "wp_template", "wp_template_part", "wp_global_styles", "wp_navigation", "wp_font_family", "wp_font_face", "acf-field-group", "acf-field", "acf-post-type", "acf-taxonomy", "acf-ui-options-page",
];
const CORE_TAXONOMIES: [&str; 8] = ["category", "post_tag", "nav_menu", "link_category", "post_format", "wp_theme", "wp_template_part_area", "wp_pattern_category"];

/// `acf_validate_internal_post_type_values()` of `acf/validate_save_post`: `ACF_Post_Type::ajax_validate_values()` and
/// `ACF_Taxonomy::ajax_validate_values()` – the key's length, its characters, the reserved terms and the keys other
/// ACF items or other registrations use – as `(input, message)`.
pub async fn validate_values(ctx: &AdminContext<'_>) -> Result<Vec<(String, String)>> {
    let screen = request(ctx, "_acf_screen").unwrap_or_default();
    let Some(kind) = request(ctx, "post_type").and_then(Kind::of) else { return Ok(Vec::new()) };
    if !matches!(screen, "post_type" | "taxonomy" | "ui_options_page") {
        return Ok(Vec::new());
    }
    let site = ctx.site;
    let (setting, max, store_type) = match kind {
        Kind::PostType => ("post_type", 20, "acf-post-type"),
        Kind::Taxonomy => ("taxonomy", 32, "acf-taxonomy"),
    };
    let posted = rustpress_core::native::form_value(ctx.form, kind.prefix()).unwrap_or(Value::Null);
    let key = match g(&posted, &[setting]) {
        Value::Str(s) => String::from_utf8_lossy(&s).into_owned(),
        _ => String::new(),
    };
    if key.is_empty() {
        return Ok(Vec::new());
    }
    // `acf_add_internal_post_type_validation_error()`: the input of the screen's array.
    let prefix = match screen {
        "taxonomy" => "acf_taxonomy",
        "ui_options_page" => "acf_ui_options_page",
        _ => "acf_post_type",
    };
    let input = format!("{prefix}[{setting}]");
    let mut errors = Vec::new();
    let what = match kind {
        Kind::PostType => "post type",
        Kind::Taxonomy => "taxonomy",
    };
    if key.len() > max {
        errors.push((input.clone(), tr(site, &format!("The {what} key must be under {max} characters."))));
    }
    if !key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-') {
        errors.push((input.clone(), tr(site, &format!("The {what} key must only contain lower case alphanumeric characters, underscores or dashes."))));
    }
    if RESERVED_TERMS.contains(&key.as_str()) {
        errors.push((input, rustpress_core::i18n::sprintf(&tr(site, "This field must not be a WordPress <a href=\"%s\" target=\"_blank\">reserved term</a>."), &["https://codex.wordpress.org/Reserved_Terms"])));
        return Ok(errors);
    }
    // The ACF items of the store with this key that could be registered (`not_registered`: a key of WordPress).
    let core: &[&str] = if kind == Kind::Taxonomy { &CORE_TAXONOMIES } else { &CORE_POST_TYPES };
    let post_id = ctx.post_field("post_id").map_or(0, |v| super::php::intval(Some(&string(v))));
    let items = site.db.acf_posts(store_type).await?;
    let matches: Vec<i64> = items
        .iter()
        .map(|p| (p.id as i64, super::internal::validated(p, kind.defaults())))
        .filter(|(_, item)| strval(item.get(setting)) == key && !(truthy(item.get("active")) && core.contains(&key.as_str())))
        .map(|(id, _)| id)
        .collect();
    let registered = core.contains(&key.as_str()) || site.types.registrations.iter().any(|r| r.taxonomy == (kind == Kind::Taxonomy) && r.name == key);
    if matches.iter().any(|id| *id != post_id) {
        errors.push((input, tr(site, &format!("This {what} key is already in use by another {what} in ACF and cannot be used."))));
    } else if matches.is_empty() && registered {
        errors.push((input, tr(site, &format!("This {what} key is already in use by another {what} registered outside of ACF and cannot be used."))));
    }
    Ok(errors)
}

/// `acf.data` of the editors (`acf_localize_data()` with the form data, `validation` on).
pub fn footer_scripts(ctx: &AdminContext<'_>) -> Option<String> {
    let (post, kind) = edited(ctx)?;
    let site = ctx.site;
    let form = super::form::FormData { screen: serde_json::Value::String(kind.screen().into()), post_id: serde_json::json!(post.id), validation: serde_json::json!(1), ..Default::default() };
    // The icon picker of a post type enqueues the uploader while it is rendered.
    let uploader = if kind == Kind::PostType { super::form::Uploader::Render } else { super::form::Uploader::No };
    Some(super::form::footer_with(site, &form, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, uploader, ""))
}

/// `ACF_Admin_Post_Type::save_post()` and `ACF_Admin_Taxonomy::save_post()` (after `verify_save_post()`): the posted
/// settings with the title from the plural label, the callbacks kept when the user may not edit them, through
/// `acf_update_internal_post_type()`.
pub async fn save_post(ctx: &PostSaved<'_>) -> Result<()> {
    let post: &Post = ctx.post;
    let actx = ctx.ctx;
    let Some(kind) = Kind::of(&post.post_type) else { return Ok(()) };
    if !actx.post_field("_acf_nonce").is_some_and(|nonce| !nonce.is_empty() && (actx.verify_nonce)(nonce, kind.screen())) {
        return Ok(());
    }
    if !(actx.can)("manage_options") {
        return Ok(());
    }
    let mut item = match rustpress_core::native::form_value(actx.form, kind.prefix()) {
        Some(value @ Value::Array(_)) => value,
        _ => Value::Array(Vec::new()),
    };
    set(&mut item, &["ID"], Value::Int(post.id as i64));
    let title = g(&item, &["labels", "name"]);
    set(&mut item, &["title"], if matches!(title, Value::Null) { string("") } else { title });
    if !(actx.can)("delete_users") {
        let existing = match rustpress_core::options::maybe_unserialize(&post.post_content) {
            value @ Value::Array(_) => value,
            _ => Value::Array(Vec::new()),
        };
        let callbacks: &[&str] = match kind {
            Kind::PostType => &["register_meta_box_cb"],
            Kind::Taxonomy => &["meta_box_cb", "meta_box_sanitize_cb"],
        };
        for name in callbacks {
            set(&mut item, &[name], string(""));
        }
        if kind == Kind::Taxonomy && strval(Some(&g(&item, &["meta_box"]))) == "custom" {
            set(&mut item, &["meta_box"], string("default"));
        }
        let kept: &[&str] = match kind {
            Kind::PostType => &["register_meta_box_cb"],
            Kind::Taxonomy => &["meta_box", "meta_box_cb", "meta_box_sanitize_cb"],
        };
        for name in kept {
            let value = g(&existing, &[name]);
            if truthy(Some(&value)) {
                set(&mut item, &[name], value);
            }
        }
    }
    let Value::Array(items) = item else { return Ok(()) };
    let system = rustpress_auth::CurrentUser::system();
    let content = rustpress_content::content::Content { site: actx.site, db: &actx.site.db, writer: ctx.writer, user: &system, now: actx.now };
    super::save_group::update_internal_post(&content, Some(actx.wp_root), kind.post_type(), items).await
}

/// `views/acf-post-type/basic-settings.php`.
async fn post_type_basic(r: &Ctx<'_>, pt: &Value) -> Result<String> {
    let site = r.site;
    let mut out = String::new();
    out.push_str(&wrap(r, vec![("label", t(site, "Plural Label")), ("placeholder", t(site, "Movies")), ("type", string("text")), ("name", string("name")), ("key", string("name")), ("class", string("acf_plural_label")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "name"])), ("required", Value::Bool(true))], "div", "field").await?);
    out.push_str(&wrap(r, vec![("label", t(site, "Singular Label")), ("placeholder", t(site, "Movie")), ("type", string("text")), ("name", string("singular_name")), ("key", string("singular_name")), ("class", string("acf_slugify_to_key acf_singular_label")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "singular_name"])), ("required", Value::Bool(true))], "div", "field").await?);
    out.push_str(&wrap(r, vec![("label", t(site, "Post Type Key")), ("instructions", t(site, "Lower case letters, underscores and dashes only, Max 20 characters.")), ("placeholder", t(site, "movie")), ("type", string("text")), ("name", string("post_type")), ("key", string("post_type")), ("maxlength", Value::Int(20)), ("class", string("acf_slugified_key")), ("prefix", string("acf_post_type")), ("value", g(pt, &["post_type"])), ("required", Value::Bool(true))], "div", "field").await?);
    out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("taxonomies")), ("key", string("taxonomies")), ("prefix", string("acf_post_type")), ("value", g(pt, &["taxonomies"])), ("label", t(site, "Taxonomies")), ("instructions", t(site, "Select existing taxonomies to classify items of the post type.")), ("choices", super::field_settings::taxonomy_labels(site)), ("ui", Value::Bool(true)), ("allow_null", Value::Bool(true)), ("multiple", Value::Bool(true))], "div", "field").await?);
    out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
    out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("public")), ("key", string("public")), ("prefix", string("acf_post_type")), ("value", g(pt, &["public"])), ("label", t(site, "Public")), ("instructions", t(site, "Visible on the frontend and in the admin dashboard.")), ("ui", Value::Bool(true)), ("default", Value::Int(1))], "div", "label").await?);
    out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("hierarchical")), ("key", string("hierarchical")), ("class", string("acf_hierarchical_switch")), ("prefix", string("acf_post_type")), ("value", g(pt, &["hierarchical"])), ("label", t(site, "Hierarchical")), ("instructions", t(site, "Hierarchical post types can have descendants (like pages).")), ("ui", Value::Bool(true))], "div", "label").await?);
    out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
    out.push_str(&wrap(r, vec![("label", t(site, "Advanced Configuration")), ("instructions", t(site, "I know what I'm doing, show me all the options.")), ("type", string("true_false")), ("name", string("advanced_configuration")), ("key", string("advanced_configuration")), ("prefix", string("acf_post_type")), ("value", g(pt, &["advanced_configuration"])), ("ui", Value::Int(1)), ("class", string("acf-advanced-settings-toggle"))], "div", "label").await?);
    out.push_str(&hidden_keys(Kind::PostType, pt));
    Ok(out)
}

/// `views/acf-post-type/advanced-settings.php`.
async fn post_type_advanced(r: &Ctx<'_>, pt: &Value, cb_edit: bool) -> Result<String> {
    let site = r.site;
    let mut out = String::new();
    for (tab, label) in settings_tabs(site) {
        out.push_str(&wrap(r, vec![("type", string("tab")), ("label", string(&label)), ("key", string("acf_post_type_tabs"))], "div", "label").await?);
        out.push_str(&format!("<div class=\"acf-post-type-advanced-settings acf-post-type-{}-settings\">", esc_attr(&tab.replace('_', "-"))));
        match tab {
            "general" => {
                let available = arr(vec![
                    ("title", t(site, "Title")),
                    ("editor", t(site, "Editor")),
                    ("notes", t(site, "Notes")),
                    ("thumbnail", t(site, "Featured Image")),
                    ("author", t(site, "Author")),
                    ("trackbacks", t(site, "Trackbacks")),
                    ("revisions", t(site, "Revisions")),
                    ("custom-fields", t(site, "Custom Fields")),
                    ("comments", t(site, "Comments")),
                    ("excerpt", t(site, "Excerpt")),
                    ("page-attributes", t(site, "Page Attributes")),
                    ("post-formats", t(site, "Post Formats")),
                ]);
                let selected = match g(pt, &["supports"]) {
                    value @ Value::Array(_) => value,
                    _ => Value::Array(Vec::new()),
                };
                // Notes need the editor.
                let with_editor = matches!(&selected, Value::Array(items) if items.iter().any(|(_, v)| matches!(v, Value::Str(s) if s == b"editor")));
                let disabled = if with_editor { list(Vec::new()) } else { list(vec![string("notes")]) };
                out.push_str(&wrap(r, vec![("type", string("checkbox")), ("name", string("supports")), ("key", string("supports")), ("label", t(site, "Supports")), ("instructions", t(site, "Enable various features in the content editor.")), ("prefix", string("acf_post_type")), ("value", unique_filtered(&selected)), ("choices", available), ("disabled", disabled), ("allow_custom", Value::Bool(true)), ("class", string("acf_post_type_supports")), ("custom_choice_button_text", t(site, "Add Custom"))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("description")), ("key", string("description")), ("prefix", string("acf_post_type")), ("value", g(pt, &["description"])), ("label", t(site, "Description")), ("instructions", t(site, "A descriptive summary of the post type."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("active")), ("key", string("active")), ("prefix", string("acf_post_type")), ("value", g(pt, &["active"])), ("label", t(site, "Active")), ("instructions", t(site, "Active post types are enabled and registered with WordPress.")), ("ui", Value::Bool(true)), ("default", Value::Int(1))], "div", "label").await?);
            }
            "labels" => {
                out.push_str(&regenerate_labels_bar(site));
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("menu_name")), ("key", string("menu_name")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "menu_name"])), ("data", arr(vec![("label", string("%s")), ("replace", string("plural"))])), ("label", t(site, "Menu Name")), ("instructions", t(site, "Admin menu name for the post type.")), ("placeholder", t(site, "Posts"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("all_items")), ("key", string("all_items")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "all_items"])), ("data", arr(vec![("label", t(site, "All %s")), ("replace", string("plural"))])), ("label", t(site, "All Items")), ("instructions", t(site, "In the post type submenu in the admin dashboard.")), ("placeholder", t(site, "All Posts"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("edit_item")), ("key", string("edit_item")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "edit_item"])), ("data", arr(vec![("label", t(site, "Edit %s")), ("replace", string("singular"))])), ("label", t(site, "Edit Item")), ("instructions", t(site, "At the top of the editor screen when editing an item.")), ("placeholder", t(site, "Edit Post"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("view_item")), ("key", string("view_item")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "view_item"])), ("data", arr(vec![("label", t(site, "View %s")), ("replace", string("singular"))])), ("label", t(site, "View Item")), ("instructions", t(site, "In the admin bar to view item when editing it.")), ("placeholder", t(site, "View Post"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("view_items")), ("key", string("view_items")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "view_items"])), ("data", arr(vec![("label", t(site, "View %s")), ("replace", string("plural"))])), ("label", t(site, "View Items")), ("instructions", t(site, "Appears in the admin bar in the 'All Posts' view, provided the post type supports archives and the home page is not an archive of that post type.")), ("placeholder", t(site, "View Posts"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("add_new_item")), ("key", string("add_new_item")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "add_new_item"])), ("data", arr(vec![("label", t(site, "Add New %s")), ("replace", string("singular"))])), ("label", t(site, "Add New Item")), ("instructions", t(site, "At the top of the editor screen when adding a new item.")), ("placeholder", t(site, "Add New Post"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("add_new")), ("key", string("add_new")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "add_new"])), ("data", arr(vec![("label", t(site, "Add New %s")), ("replace", string("singular"))])), ("label", t(site, "Add New")), ("instructions", t(site, "In the post type submenu in the admin dashboard.")), ("placeholder", t(site, "Add New Post"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("new_item")), ("key", string("new_item")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "new_item"])), ("data", arr(vec![("label", t(site, "New %s")), ("replace", string("singular"))])), ("label", t(site, "New Item")), ("instructions", t(site, "In the post type submenu in the admin dashboard.")), ("placeholder", t(site, "New Post"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("parent_item_colon")), ("key", string("parent_item_colon")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "parent_item_colon"])), ("data", arr(vec![("label", t(site, "Parent %s:")), ("replace", string("singular"))])), ("label", t(site, "Parent Item Prefix")), ("instructions", t(site, "For hierarchical types in the post type list screen.")), ("placeholder", t(site, "Parent Page:"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("search_items")), ("key", string("search_items")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "search_items"])), ("data", arr(vec![("label", t(site, "Search %s")), ("replace", string("plural"))])), ("label", t(site, "Search Items")), ("instructions", t(site, "At the top of the items screen when searching for an item.")), ("placeholder", t(site, "Search Posts"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("not_found")), ("key", string("not_found")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "not_found"])), ("data", arr(vec![("label", t(site, "No %s found")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "No Items Found")), ("instructions", t(site, "At the top of the post type list screen when there are no posts to display.")), ("placeholder", t(site, "No posts found"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("not_found_in_trash")), ("key", string("not_found_in_trash")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "not_found_in_trash"])), ("data", arr(vec![("label", t(site, "No %s found in Trash")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "No Items Found in Trash")), ("instructions", t(site, "At the top of the post type list screen when there are no posts in the trash.")), ("placeholder", t(site, "No posts found in Trash"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("archives")), ("key", string("archives")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "archives"])), ("data", arr(vec![("label", t(site, "%s Archives")), ("replace", string("singular"))])), ("label", t(site, "Archives Nav Menu")), ("instructions", t(site, "Adds 'Post Type Archive' items with this label to the list of posts shown when adding items to an existing menu in a CPT with archives enabled. Only appears when editing menus in 'Live Preview' mode and a custom archive slug has been provided.")), ("placeholder", t(site, "Post Archives"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("attributes")), ("key", string("attributes")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "attributes"])), ("data", arr(vec![("label", t(site, "%s Attributes")), ("replace", string("singular"))])), ("label", t(site, "Attributes Meta Box")), ("instructions", t(site, "In the editor used for the title of the post attributes meta box.")), ("placeholder", t(site, "Post Attributes"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("featured_image")), ("key", string("featured_image")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "featured_image"])), ("label", t(site, "Featured Image Meta Box")), ("instructions", t(site, "In the editor used for the title of the featured image meta box.")), ("placeholder", t(site, "Featured image"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("set_featured_image")), ("key", string("set_featured_image")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "set_featured_image"])), ("label", t(site, "Set Featured Image")), ("instructions", t(site, "As the button label when setting the featured image.")), ("placeholder", t(site, "Set featured image"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("remove_featured_image")), ("key", string("remove_featured_image")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "remove_featured_image"])), ("label", t(site, "Remove Featured Image")), ("instructions", t(site, "As the button label when removing the featured image.")), ("placeholder", t(site, "Remove featured image"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("use_featured_image")), ("key", string("use_featured_image")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "use_featured_image"])), ("label", t(site, "Use Featured Image")), ("instructions", t(site, "As the button label for selecting to use an image as the featured image.")), ("placeholder", t(site, "Use as featured image"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("insert_into_item")), ("key", string("insert_into_item")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "insert_into_item"])), ("data", arr(vec![("label", t(site, "Insert into %s")), ("replace", string("singular")), ("transform", string("lower"))])), ("label", t(site, "Insert Into Media Button")), ("instructions", t(site, "As the button label when adding media to content.")), ("placeholder", t(site, "Insert into post"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("uploaded_to_this_item")), ("key", string("uploaded_to_this_item")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "uploaded_to_this_item"])), ("data", arr(vec![("label", t(site, "Uploaded to this %s")), ("replace", string("singular")), ("transform", string("lower"))])), ("label", t(site, "Uploaded To This Item")), ("instructions", t(site, "In the media modal showing all media uploaded to this item.")), ("placeholder", t(site, "Uploaded to this post"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("filter_items_list")), ("key", string("filter_items_list")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "filter_items_list"])), ("data", arr(vec![("label", t(site, "Filter %s list")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "Filter Items List")), ("instructions", t(site, "Used by screen readers for the filter links heading on the post type list screen.")), ("placeholder", t(site, "Filter posts list"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("filter_by_date")), ("key", string("filter_by_date")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "filter_by_date"])), ("data", arr(vec![("label", t(site, "Filter %s by date")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "Filter Items By Date")), ("instructions", t(site, "Used by screen readers for the filter by date heading on the post type list screen.")), ("placeholder", t(site, "Filter posts by date"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("items_list_navigation")), ("key", string("items_list_navigation")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "items_list_navigation"])), ("data", arr(vec![("label", t(site, "%s list navigation")), ("replace", string("plural"))])), ("label", t(site, "Items List Navigation")), ("instructions", t(site, "Used by screen readers for the filter list pagination on the post type list screen.")), ("placeholder", t(site, "Posts list navigation"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("items_list")), ("key", string("items_list")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "items_list"])), ("data", arr(vec![("label", t(site, "%s list")), ("replace", string("plural"))])), ("label", t(site, "Items List")), ("instructions", t(site, "Used by screen readers for the items list on the post type list screen.")), ("placeholder", t(site, "Posts list"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("item_published")), ("key", string("item_published")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "item_published"])), ("data", arr(vec![("label", t(site, "%s published.")), ("replace", string("singular"))])), ("label", t(site, "Item Published")), ("instructions", t(site, "In the editor notice after publishing an item.")), ("placeholder", t(site, "Post published."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("item_published_privately")), ("key", string("item_published_privately")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "item_published_privately"])), ("data", arr(vec![("label", t(site, "%s published privately.")), ("replace", string("singular"))])), ("label", t(site, "Item Published Privately")), ("instructions", t(site, "In the editor notice after publishing a private item.")), ("placeholder", t(site, "Post published privately."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("item_reverted_to_draft")), ("key", string("item_reverted_to_draft")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "item_reverted_to_draft"])), ("data", arr(vec![("label", t(site, "%s reverted to draft.")), ("replace", string("singular"))])), ("label", t(site, "Item Reverted To Draft")), ("instructions", t(site, "In the editor notice after reverting an item to draft.")), ("placeholder", t(site, "Post reverted to draft."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("item_scheduled")), ("key", string("item_scheduled")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "item_scheduled"])), ("data", arr(vec![("label", t(site, "%s scheduled.")), ("replace", string("singular"))])), ("label", t(site, "Item Scheduled")), ("instructions", t(site, "In the editor notice after scheduling an item.")), ("placeholder", t(site, "Post scheduled."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("item_updated")), ("key", string("item_updated")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "item_updated"])), ("data", arr(vec![("label", t(site, "%s updated.")), ("replace", string("singular"))])), ("label", t(site, "Item Updated")), ("instructions", t(site, "In the editor notice after an item is updated.")), ("placeholder", t(site, "Post updated."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("item_link")), ("key", string("item_link")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "item_link"])), ("data", arr(vec![("label", t(site, "%s Link")), ("replace", string("singular"))])), ("label", t(site, "Item Link")), ("instructions", t(site, "Title for a navigation link block variation.")), ("placeholder", t(site, "Post Link"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("item_link_description")), ("key", string("item_link_description")), ("prefix", string("acf_post_type[labels]")), ("value", g(pt, &["labels", "item_link_description"])), ("data", arr(vec![("label", t(site, "A link to a %s.")), ("replace", string("singular")), ("transform", string("lower"))])), ("label", t(site, "Item Link Description")), ("instructions", t(site, "Description for a navigation link block variation.")), ("placeholder", t(site, "A link to a post."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("enter_title_here")), ("key", string("enter_title_here")), ("prefix", string("acf_post_type")), ("value", g(pt, &["enter_title_here"])), ("label", t(site, "Title Placeholder")), ("instructions", t(site, "In the editor used as the placeholder of the title.")), ("placeholder", t(site, "Add title"))], "div", "field").await?);
            }
            "visibility" => {
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("show_ui")), ("key", string("show_ui")), ("prefix", string("acf_post_type")), ("value", g(pt, &["show_ui"])), ("label", t(site, "Show In UI")), ("instructions", t(site, "Items can be edited and managed in the admin dashboard.")), ("ui", Value::Bool(true)), ("default", Value::Int(1))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("show_in_menu")), ("key", string("show_in_menu")), ("prefix", string("acf_post_type")), ("value", g(pt, &["show_in_menu"])), ("label", t(site, "Show In Admin Menu")), ("instructions", t(site, "Admin editor navigation in the sidebar menu.")), ("ui", Value::Bool(true)), ("default", Value::Int(1)), ("conditions", arr(vec![("field", string("show_ui")), ("operator", string("==")), ("value", Value::Int(1))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("admin_menu_parent")), ("key", string("admin_menu_parent")), ("prefix", string("acf_post_type")), ("value", g(pt, &["admin_menu_parent"])), ("placeholder", string("edit.php?post_type={parent_page}")), ("label", t(site, "Admin Menu Parent")), ("instructions", t(site, "By default the post type will get a new top level item in the admin menu. If an existing top level item is supplied here, the post type will be added as a submenu item under it.")), ("conditions", arr(vec![("field", string("show_in_menu")), ("operator", string("==")), ("value", Value::Int(1))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("number")), ("name", string("menu_position")), ("key", string("menu_position")), ("prefix", string("acf_post_type")), ("value", g(pt, &["menu_position"])), ("label", t(site, "Menu Position")), ("instructions", t(site, "The position in the sidebar menu in the admin dashboard.")), ("conditions", arr(vec![("field", string("show_in_menu")), ("operator", string("==")), ("value", Value::Int(1))]))], "div", "field").await?);
                // The default icon, and icons from before the icon picker.
                let icon = match g(pt, &["menu_icon"]) {
                    value if !truthy(Some(&value)) => arr(vec![("type", string("dashicons")), ("value", string("dashicons-admin-post"))]),
                    Value::Str(icon) => arr(vec![("type", string(if String::from_utf8_lossy(&icon).contains("dashicons-") { "dashicons" } else { "url" })), ("value", Value::Str(icon))]),
                    value => value,
                };
                out.push_str(&wrap(r, vec![("type", string("icon_picker")), ("name", string("menu_icon")), ("key", string("menu_icon")), ("prefix", string("acf_post_type")), ("value", icon), ("label", t(site, "Menu Icon")), ("placeholder", string("dashicons-admin-post")), ("conditions", list(vec![arr(vec![("field", string("show_in_menu")), ("operator", string("==")), ("value", string("1"))]), arr(vec![("field", string("admin_menu_parent")), ("operator", string("==")), ("value", string(""))])]))], "div", "field").await?);
                // Only for a user who may change the callback, or when there is one.
                if cb_edit || truthy(Some(&g(pt, &["register_meta_box_cb"]))) {
                    let mut instructions = tr(site, "A PHP function name to be called when setting up the meta boxes for the edit screen. For security, this callback will be executed in a special context without access to any superglobals like $_POST or $_GET.");
                    if !cb_edit {
                        instructions = format!("{instructions} {}", tr(site, "By default only admin users can edit this setting."));
                    }
                    out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("register_meta_box_cb")), ("key", string("register_meta_box_cb")), ("prefix", string("acf_post_type")), ("value", g(pt, &["register_meta_box_cb"])), ("label", t(site, "Custom Meta Box Callback")), ("instructions", string(&instructions)), ("readonly", Value::Bool(!cb_edit)), ("conditions", arr(vec![("field", string("show_ui")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                }
                out.push_str(&wrap(r, vec![("type", string("seperator")), ("conditions", arr(vec![("field", string("show_ui")), ("operator", string("==")), ("value", Value::Int(1))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("show_in_admin_bar")), ("key", string("show_in_admin_bar")), ("prefix", string("acf_post_type")), ("value", g(pt, &["show_in_admin_bar"])), ("label", t(site, "Show In Admin Bar")), ("instructions", t(site, "Appears as an item in the 'New' menu in the admin bar.")), ("ui", Value::Bool(true)), ("default", Value::Int(1)), ("conditions", arr(vec![("field", string("show_ui")), ("operator", string("==")), ("value", Value::Int(1))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("show_in_nav_menus")), ("key", string("show_in_nav_menus")), ("prefix", string("acf_post_type")), ("value", g(pt, &["show_in_nav_menus"])), ("label", t(site, "Appearance Menus Support")), ("instructions", t(site, "Allow items to be added to menus in the 'Appearance' > 'Menus' screen. Must be turned on in 'Screen options'.")), ("ui", Value::Bool(true)), ("default", Value::Int(1))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("exclude_from_search")), ("key", string("exclude_from_search")), ("prefix", string("acf_post_type")), ("value", g(pt, &["exclude_from_search"])), ("label", t(site, "Exclude From Search")), ("instructions", t(site, "Sets whether posts should be excluded from search results and taxonomy archive pages.")), ("ui", Value::Bool(true))], "div", "label").await?);
            }
            "permissions" => {
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("rename_capabilities")), ("key", string("rename_capabilities")), ("prefix", string("acf_post_type")), ("value", g(pt, &["rename_capabilities"])), ("label", t(site, "Rename Capabilities")), ("instructions", t(site, "By default the capabilities of the post type will inherit the 'Post' capability names, eg. edit_post, delete_posts. Enable to use post type specific capabilities, eg. edit_{singular}, delete_{plural}.")), ("default", Value::Bool(false)), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("singular_capability_name")), ("key", string("singular_capability_name")), ("prefix", string("acf_post_type")), ("value", g(pt, &["singular_capability_name"])), ("label", t(site, "Singular Capability Name")), ("instructions", t(site, "Choose another post type to base the capabilities for this post type.")), ("conditions", arr(vec![("field", string("rename_capabilities")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("plural_capability_name")), ("key", string("plural_capability_name")), ("prefix", string("acf_post_type")), ("value", g(pt, &["plural_capability_name"])), ("label", t(site, "Plural Capability Name")), ("instructions", t(site, "Optionally provide a plural to be used in capabilities.")), ("conditions", arr(vec![("field", string("rename_capabilities")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator")), ("conditions", arr(vec![("field", string("rename_capabilities")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("can_export")), ("key", string("can_export")), ("prefix", string("acf_post_type")), ("value", g(pt, &["can_export"])), ("label", t(site, "Can Export")), ("instructions", t(site, "Allow the post type to be exported from 'Tools' > 'Export'.")), ("default", Value::Int(1)), ("ui", Value::Int(1))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("delete_with_user")), ("key", string("delete_with_user")), ("prefix", string("acf_post_type")), ("value", g(pt, &["delete_with_user"])), ("label", t(site, "Delete With User")), ("instructions", t(site, "Delete items by a user when that user is deleted.")), ("ui", Value::Int(1))], "div", "label").await?);
            }
            "urls" => {
                out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("permalink_rewrite")), ("key", string("permalink_rewrite")), ("prefix", string("acf_post_type[rewrite]")), ("value", g_or(pt, &["rewrite", "permalink_rewrite"], string("post_type_key"))), ("label", t(site, "Permalink Rewrite")), ("instructions", cat(vec![t(site, "Rewrite the URL using the post type key as the slug. Your permalink structure will be"), string(" {slug}.")])), ("choices", arr(vec![("post_type_key", t(site, "Post Type Key")), ("custom_permalink", t(site, "Custom Permalink")), ("no_permalink", t(site, "No Permalink (prevent URL rewriting)"))])), ("default", string("post_type_key")), ("hide_search", Value::Bool(true)), ("data", arr(vec![("post_type_key_instructions", cat(vec![t(site, "Rewrite the URL using the post type key as the slug. Your permalink structure will be"), string(" {slug}.")])), ("custom_permalink_instructions", cat(vec![t(site, "Rewrite the URL using a custom slug defined in the input below. Your permalink structure will be"), string(" {slug}.")])), ("no_permalink_instructions", t(site, "Permalinks for this post type are disabled.")), ("site_url", string(&site.settings.siteurl))])), ("class", string("permalink_rewrite"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("slug")), ("key", string("slug")), ("prefix", string("acf_post_type[rewrite]")), ("value", g_or(pt, &["rewrite", "slug"], string(""))), ("label", t(site, "URL Slug")), ("instructions", t(site, "Customize the slug used in the URL.")), ("conditions", arr(vec![("field", string("permalink_rewrite")), ("operator", string("==")), ("value", string("custom_permalink"))])), ("class", string("rewrite_slug_field"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("with_front")), ("key", string("with_front")), ("prefix", string("acf_post_type[rewrite]")), ("value", g_or(pt, &["rewrite", "with_front"], Value::Bool(true))), ("label", t(site, "Front URL Prefix")), ("instructions", t(site, "Alters the permalink structure to add the `WP_Rewrite::$front` prefix to URLs.")), ("ui", Value::Bool(true)), ("default", Value::Int(1)), ("conditions", arr(vec![("field", string("permalink_rewrite")), ("operator", string("!=")), ("value", string("no_permalink"))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("feeds")), ("key", string("feeds")), ("prefix", string("acf_post_type[rewrite]")), ("value", g_or(pt, &["rewrite", "feeds"], g(pt, &["has_archive"]))), ("label", t(site, "Feed URL")), ("instructions", t(site, "RSS feed URL for the post type items.")), ("ui", Value::Bool(true)), ("conditions", arr(vec![("field", string("permalink_rewrite")), ("operator", string("!=")), ("value", string("no_permalink"))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("pages")), ("key", string("pages")), ("prefix", string("acf_post_type[rewrite]")), ("value", g_or(pt, &["rewrite", "pages"], Value::Bool(true))), ("label", t(site, "Pagination")), ("instructions", t(site, "Pagination support for the items URLs such as the archives.")), ("ui", Value::Bool(true)), ("default", Value::Int(1)), ("conditions", arr(vec![("field", string("permalink_rewrite")), ("operator", string("!=")), ("value", string("no_permalink"))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("has_archive")), ("key", string("has_archive")), ("prefix", string("acf_post_type")), ("value", g(pt, &["has_archive"])), ("label", t(site, "Archive")), ("instructions", t(site, "Has an item archive that can be customized with an archive template file in your theme.")), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("has_archive_slug")), ("key", string("has_archive_slug")), ("prefix", string("acf_post_type")), ("value", g(pt, &["has_archive_slug"])), ("label", t(site, "Archive Slug")), ("instructions", t(site, "Custom slug for the Archive URL.")), ("ui", Value::Bool(true)), ("conditions", arr(vec![("field", string("has_archive")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("publicly_queryable")), ("key", string("publicly_queryable")), ("prefix", string("acf_post_type")), ("value", g(pt, &["publicly_queryable"])), ("label", t(site, "Publicly Queryable")), ("instructions", t(site, "URLs for an item and items can be accessed with a query string.")), ("default", Value::Int(1)), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator")), ("conditions", arr(vec![("field", string("publicly_queryable")), ("operator", string("==")), ("value", Value::Int(1))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("query_var")), ("key", string("query_var")), ("prefix", string("acf_post_type")), ("value", g(pt, &["query_var"])), ("label", t(site, "Query Variable Support")), ("instructions", t(site, "Items can be accessed using the non-pretty permalink, eg. {post_type}={post_slug}.")), ("choices", arr(vec![("post_type_key", t(site, "Post Type Key")), ("custom_query_var", t(site, "Custom Query Variable")), ("none", t(site, "No Query Variable Support"))])), ("default", Value::Int(1)), ("hide_search", Value::Bool(true)), ("class", string("query_var")), ("conditions", arr(vec![("field", string("publicly_queryable")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("query_var_name")), ("key", string("query_var_name")), ("prefix", string("acf_post_type")), ("value", g(pt, &["query_var_name"])), ("label", t(site, "Query Variable")), ("instructions", t(site, "Customize the query variable name.")), ("ui", Value::Bool(true)), ("conditions", arr(vec![("field", string("query_var")), ("operator", string("==")), ("value", string("custom_query_var"))]))], "div", "field").await?);
            }
            "rest_api" => {
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("show_in_rest")), ("key", string("show_in_rest")), ("prefix", string("acf_post_type")), ("value", g(pt, &["show_in_rest"])), ("label", t(site, "Show In REST API")), ("instructions", t(site, "Exposes this post type in the REST API. Required to use the block editor.")), ("default", Value::Int(1)), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("rest_base")), ("key", string("rest_base")), ("prefix", string("acf_post_type")), ("value", g(pt, &["rest_base"])), ("label", t(site, "Base URL")), ("instructions", t(site, "The base URL for the post type REST API URLs.")), ("conditions", arr(vec![("field", string("show_in_rest")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("rest_namespace")), ("key", string("rest_namespace")), ("prefix", string("acf_post_type")), ("value", g(pt, &["rest_namespace"])), ("label", t(site, "Namespace Route")), ("instructions", t(site, "The namespace part of the REST API URL.")), ("conditions", arr(vec![("field", string("show_in_rest")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("rest_controller_class")), ("key", string("rest_controller_class")), ("prefix", string("acf_post_type")), ("value", g(pt, &["rest_controller_class"])), ("label", t(site, "Controller Class")), ("instructions", t(site, "Optional custom controller to use instead of `WP_REST_Posts_Controller`.")), ("default", string("WP_REST_Posts_Controller")), ("conditions", arr(vec![("field", string("show_in_rest")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
            }
            _ => {}
        }
        out.push_str("</div>");
    }
    Ok(out)
}

/// `views/acf-taxonomy/basic-settings.php`.
async fn taxonomy_basic(r: &Ctx<'_>, pt: &Value) -> Result<String> {
    let site = r.site;
    let mut out = String::new();
    out.push_str(&wrap(r, vec![("label", t(site, "Plural Label")), ("placeholder", t(site, "Genres")), ("type", string("text")), ("key", string("name")), ("name", string("name")), ("class", string("acf_plural_label")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "name"])), ("required", Value::Int(1))], "div", "field").await?);
    out.push_str(&wrap(r, vec![("label", t(site, "Singular Label")), ("placeholder", t(site, "Genre")), ("type", string("text")), ("key", string("singular_name")), ("name", string("singular_name")), ("class", string("acf_slugify_to_key acf_singular_label")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "singular_name"])), ("required", Value::Int(1))], "div", "field").await?);
    out.push_str(&wrap(r, vec![("label", t(site, "Taxonomy Key")), ("instructions", t(site, "Lower case letters, underscores and dashes only, Max 32 characters.")), ("placeholder", t(site, "genre")), ("type", string("text")), ("key", string("taxonomy")), ("name", string("taxonomy")), ("maxlength", Value::Int(32)), ("class", string("acf_slugified_key")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["taxonomy"])), ("required", Value::Int(1))], "div", "field").await?);
    out.push_str(&wrap(r, vec![("label", t(site, "Post Types")), ("type", string("select")), ("name", string("object_type")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["object_type"])), ("choices", super::field_settings::pretty_post_types(site)), ("multiple", Value::Int(1)), ("ui", Value::Int(1)), ("allow_null", Value::Int(1)), ("instructions", t(site, "One or many post types that can be classified with this taxonomy."))], "div", "field").await?);
    out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
    out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("public")), ("name", string("public")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["public"])), ("label", t(site, "Public")), ("instructions", t(site, "Makes a taxonomy visible on the frontend and in the admin dashboard.")), ("ui", Value::Bool(true)), ("default", Value::Int(1))], "div", "label").await?);
    out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("hierarchical")), ("name", string("hierarchical")), ("class", string("acf_hierarchical_switch")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["hierarchical"])), ("label", t(site, "Hierarchical")), ("instructions", t(site, "Hierarchical taxonomies can have descendants (like categories).")), ("ui", Value::Bool(true))], "div", "label").await?);
    out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
    out.push_str(&wrap(r, vec![("label", t(site, "Advanced Configuration")), ("instructions", t(site, "I know what I'm doing, show me all the options.")), ("type", string("true_false")), ("key", string("advanced_configuration")), ("name", string("advanced_configuration")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["advanced_configuration"])), ("ui", Value::Int(1)), ("class", string("acf-advanced-settings-toggle"))], "div", "label").await?);
    out.push_str(&hidden_keys(Kind::Taxonomy, pt));
    Ok(out)
}

/// `views/acf-taxonomy/advanced-settings.php`.
async fn taxonomy_advanced(r: &Ctx<'_>, pt: &Value, cb_edit: bool) -> Result<String> {
    let site = r.site;
    let mut out = String::new();
    for (tab, label) in settings_tabs(site) {
        out.push_str(&wrap(r, vec![("type", string("tab")), ("label", string(&label)), ("key", string("acf_taxonomy_tabs"))], "div", "label").await?);
        out.push_str(&format!("<div class=\"acf-taxonomy-advanced-settings acf-taxonomy-{}-settings\">", esc_attr(&tab.replace('_', "-"))));
        match tab {
            "general" => {
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("sort")), ("name", string("sort")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["sort"])), ("label", t(site, "Sort Terms")), ("instructions", t(site, "Whether terms in this taxonomy should be sorted in the order they are provided to `wp_set_object_terms()`.")), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("default_term_enabled")), ("name", string("default_term_enabled")), ("prefix", string("acf_taxonomy[default_term]")), ("value", g(pt, &["default_term", "default_term_enabled"])), ("label", t(site, "Default Term")), ("instructions", t(site, "Create a term for the taxonomy that cannot be deleted. It will not be selected for posts by default.")), ("ui", Value::Bool(true))], "div", "label").await?);
                // Between `?>` and `<?php` of the view.
                out.push_str("\t\t\t\n\t\t\t");
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("default_term_name")), ("key", string("default_term_name")), ("prefix", string("acf_taxonomy[default_term]")), ("value", g_or(pt, &["default_term", "default_term_name"], string(""))), ("label", t(site, "Term Name")), ("instructions", t(site, "The name of the default term.")), ("required", Value::Int(1)), ("conditions", arr(vec![("field", string("default_term_enabled")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("default_term_slug")), ("key", string("default_term_slug")), ("prefix", string("acf_taxonomy[default_term]")), ("value", g_or(pt, &["default_term", "default_term_slug"], string(""))), ("label", t(site, "Term Slug")), ("instructions", t(site, "Single word, no spaces. Underscores and dashes allowed.")), ("conditions", arr(vec![("field", string("default_term_enabled")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("description")), ("key", string("default_term_description")), ("prefix", string("acf_taxonomy[default_term]")), ("value", g_or(pt, &["default_term", "default_term_description"], string(""))), ("label", t(site, "Term Description")), ("instructions", t(site, "A descriptive summary of the term.")), ("conditions", arr(vec![("field", string("default_term_enabled")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("description")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["description"])), ("label", t(site, "Description")), ("instructions", t(site, "A descriptive summary of the taxonomy."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("active")), ("key", string("active")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["active"])), ("label", t(site, "Active")), ("instructions", t(site, "Active taxonomies are enabled and registered with WordPress.")), ("ui", Value::Bool(true)), ("default", Value::Int(1))], "div", "label").await?);
            }
            "labels" => {
                out.push_str(&regenerate_labels_bar(site));
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("menu_name")), ("name", string("menu_name")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "menu_name"])), ("data", arr(vec![("label", string("%s")), ("replace", string("plural"))])), ("label", t(site, "Menu Label")), ("instructions", t(site, "Assigns the menu name text.")), ("placeholder", t(site, "Tags"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("all_items")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "all_items"])), ("data", arr(vec![("label", t(site, "All %s")), ("replace", string("plural"))])), ("label", t(site, "All Items")), ("instructions", t(site, "Assigns the all items text.")), ("placeholder", t(site, "All Tags"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("edit_item")), ("name", string("edit_item")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "edit_item"])), ("data", arr(vec![("label", t(site, "Edit %s")), ("replace", string("singular"))])), ("label", t(site, "Edit Item")), ("instructions", t(site, "At the top of the editor screen when editing a term.")), ("placeholder", t(site, "Edit Tag"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("view_item")), ("name", string("view_item")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "view_item"])), ("data", arr(vec![("label", t(site, "View %s")), ("replace", string("singular"))])), ("label", t(site, "View Item")), ("instructions", t(site, "In the admin bar to view term during editing.")), ("placeholder", t(site, "View Tag"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("update_item")), ("name", string("update_item")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "update_item"])), ("data", arr(vec![("label", t(site, "Update %s")), ("replace", string("singular"))])), ("label", t(site, "Update Item")), ("instructions", t(site, "Assigns the update item text.")), ("placeholder", t(site, "Update Tag"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("add_new_item")), ("name", string("add_new_item")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "add_new_item"])), ("data", arr(vec![("label", t(site, "Add New %s")), ("replace", string("singular"))])), ("label", t(site, "Add New Item")), ("instructions", t(site, "Assigns the add new item text.")), ("placeholder", t(site, "Add New Tag"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("new_item_name")), ("name", string("new_item_name")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "new_item_name"])), ("data", arr(vec![("label", t(site, "New %s Name")), ("replace", string("singular"))])), ("label", t(site, "New Item Name")), ("instructions", t(site, "Assigns the new item name text.")), ("placeholder", t(site, "New Tag Name"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("parent_item")), ("name", string("parent_item")), ("prefix", string("acf_taxonomy[labels]")), ("value", g_or(pt, &["labels", "parent_item"], string(""))), ("data", arr(vec![("label", t(site, "Parent %s")), ("replace", string("singular"))])), ("label", t(site, "Parent Item")), ("instructions", t(site, "Assigns parent item text. Only used on hierarchical taxonomies.")), ("placeholder", t(site, "Parent Category")), ("conditions", arr(vec![("field", string("hierarchical")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("parent_item_colon")), ("name", string("parent_item_colon")), ("prefix", string("acf_taxonomy[labels]")), ("value", g_or(pt, &["labels", "parent_item_colon"], string(""))), ("data", arr(vec![("label", t(site, "Parent %s:")), ("replace", string("singular"))])), ("label", t(site, "Parent Item With Colon")), ("instructions", t(site, "Assigns parent item text, but with a colon (:) added to the end.")), ("placeholder", t(site, "Parent Category:")), ("conditions", arr(vec![("field", string("hierarchical")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("search_items")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "search_items"])), ("data", arr(vec![("label", t(site, "Search %s")), ("replace", string("plural"))])), ("label", t(site, "Search Items")), ("instructions", t(site, "Assigns search items text.")), ("placeholder", t(site, "Search Tags"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("popular_items")), ("prefix", string("acf_taxonomy[labels]")), ("value", g_or(pt, &["labels", "popular_items"], string(""))), ("data", arr(vec![("label", t(site, "Popular %s")), ("replace", string("plural"))])), ("label", t(site, "Popular Items")), ("instructions", t(site, "Assigns popular items text. Only used for non-hierarchical taxonomies.")), ("placeholder", t(site, "Popular Tags")), ("conditions", arr(vec![("field", string("hierarchical")), ("operator", string("==")), ("value", string("0"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("separate_items_with_commas")), ("name", string("separate_items_with_commas")), ("prefix", string("acf_taxonomy[labels]")), ("value", g_or(pt, &["labels", "separate_items_with_commas"], string(""))), ("data", arr(vec![("label", t(site, "Separate %s with commas")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "Separate Items With Commas")), ("instructions", t(site, "Assigns the separate item with commas text used in the taxonomy meta box. Only used on non-hierarchical taxonomies.")), ("placeholder", t(site, "Separate tags with commas")), ("conditions", arr(vec![("field", string("hierarchical")), ("operator", string("==")), ("value", string("0"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("add_or_remove_items")), ("name", string("add_or_remove_items")), ("prefix", string("acf_taxonomy[labels]")), ("value", g_or(pt, &["labels", "add_or_remove_items"], string(""))), ("data", arr(vec![("label", t(site, "Add or remove %s")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "Add Or Remove Items")), ("instructions", t(site, "Assigns the add or remove items text used in the meta box when JavaScript is disabled. Only used on non-hierarchical taxonomies")), ("placeholder", t(site, "Add or remove tags")), ("conditions", arr(vec![("field", string("hierarchical")), ("operator", string("==")), ("value", string("0"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("choose_from_most_used")), ("name", string("choose_from_most_used")), ("prefix", string("acf_taxonomy[labels]")), ("value", g_or(pt, &["labels", "choose_from_most_used"], string(""))), ("data", arr(vec![("label", t(site, "Choose from the most used %s")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "Choose From Most Used")), ("instructions", t(site, "Assigns the 'choose from most used' text used in the meta box when JavaScript is disabled. Only used on non-hierarchical taxonomies.")), ("placeholder", t(site, "Choose from the most used tags")), ("conditions", arr(vec![("field", string("hierarchical")), ("operator", string("==")), ("value", string("0"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("most_used")), ("name", string("most_used")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "most_used"])), ("label", t(site, "Most Used")), ("instructions", t(site, "Assigns text to the Title field of the Most Used tab.")), ("default", t(site, "Most Used")), ("placeholder", t(site, "Most Used"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("not_found")), ("name", string("not_found")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "not_found"])), ("data", arr(vec![("label", t(site, "No %s found")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "Not Found")), ("instructions", t(site, "Assigns the text displayed when clicking the 'choose from most used' text in the taxonomy meta box when no tags are available, and assigns the text used in the terms list table when there are no items for a taxonomy.")), ("placeholder", t(site, "No tags found"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("no_terms")), ("name", string("no_terms")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "no_terms"])), ("data", arr(vec![("label", t(site, "No %s")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "No Terms")), ("instructions", t(site, "Assigns the text displayed in the posts and media list tables when no tags or categories are available.")), ("placeholder", t(site, "No tags"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("name_field_description")), ("name", string("name_field_description")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "name_field_description"])), ("label", t(site, "Name Field Description")), ("instructions", t(site, "Describes the Name field on the Edit Tags screen.")), ("placeholder", t(site, "The name is how it appears on your site")), ("default", t(site, "The name is how it appears on your site"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("textarea")), ("key", string("slug_field_description")), ("name", string("slug_field_description")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "slug_field_description"])), ("label", t(site, "Slug Field Description")), ("instructions", t(site, "Describes the Slug field on the Edit Tags screen.")), ("placeholder", t(site, "The \"slug\" is the URL-friendly version of the name. It is usually all lower case and contains only letters, numbers, and hyphens.")), ("default", t(site, "The \"slug\" is the URL-friendly version of the name. It is usually all lower case and contains only letters, numbers, and hyphens."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("parent_field_description")), ("name", string("parent_field_description")), ("prefix", string("acf_taxonomy[labels]")), ("value", g_or(pt, &["labels", "parent_field_description"], string(""))), ("label", t(site, "Parent Field Description")), ("instructions", t(site, "Describes the Parent field on the Edit Tags screen.")), ("placeholder", t(site, "Assign a parent term to create a hierarchy. The term Jazz, for example, would be the parent of Bebop and Big Band")), ("default", t(site, "Assign a parent term to create a hierarchy. The term Jazz, for example, would be the parent of Bebop and Big Band")), ("conditions", arr(vec![("field", string("hierarchical")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("desc_field_description")), ("name", string("desc_field_description")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "desc_field_description"])), ("label", t(site, "Description Field Description")), ("instructions", t(site, "Describes the Description field on the Edit Tags screen.")), ("placeholder", t(site, "The description is not prominent by default; however, some themes may show it.")), ("default", t(site, "The description is not prominent by default; however, some themes may show it."))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("filter_by_item")), ("name", string("filter_by_item")), ("prefix", string("acf_taxonomy[labels]")), ("value", g_or(pt, &["labels", "filter_by_item"], string(""))), ("data", arr(vec![("label", t(site, "Filter by %s")), ("replace", string("singular")), ("transform", string("lower"))])), ("label", t(site, "Filter By Item")), ("instructions", t(site, "Assigns text to the filter button in the posts lists table.")), ("placeholder", t(site, "Filter by category")), ("conditions", arr(vec![("field", string("hierarchical")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("items_list_navigation")), ("name", string("items_list_navigation")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "items_list_navigation"])), ("data", arr(vec![("label", t(site, "%s list navigation")), ("replace", string("plural"))])), ("label", t(site, "Items List Navigation")), ("instructions", t(site, "Assigns text to the table pagination hidden heading.")), ("placeholder", t(site, "Tags list navigation"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("items_list")), ("name", string("items_list")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "items_list"])), ("data", arr(vec![("label", t(site, "%s list")), ("replace", string("plural"))])), ("label", t(site, "Items List")), ("instructions", t(site, "Assigns text to the table hidden heading.")), ("placeholder", t(site, "Tags list"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("back_to_items")), ("name", string("back_to_items")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "back_to_items"])), ("data", arr(vec![("label", t(site, "← Go to %s")), ("replace", string("plural")), ("transform", string("lower"))])), ("label", t(site, "Back To Items")), ("instructions", t(site, "Assigns the text used to link back to the main index after updating a term.")), ("placeholder", t(site, "← Go to tags"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("item_link")), ("name", string("item_link")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "item_link"])), ("data", arr(vec![("label", t(site, "%s Link")), ("replace", string("singular"))])), ("label", t(site, "Item Link")), ("instructions", t(site, "Assigns a title for navigation link block variation used in the block editor.")), ("placeholder", t(site, "Tag Link"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("item_link_description")), ("name", string("item_link_description")), ("prefix", string("acf_taxonomy[labels]")), ("value", g(pt, &["labels", "item_link_description"])), ("data", arr(vec![("label", t(site, "A link to a %s")), ("replace", string("singular")), ("transform", string("lower"))])), ("label", t(site, "Item Link Description")), ("instructions", t(site, "Describes a navigation link block variation used in the block editor.")), ("placeholder", t(site, "A link to a tag"))], "div", "field").await?);
            }
            "visibility" => {
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("show_ui")), ("name", string("show_ui")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["show_ui"])), ("label", t(site, "Show In UI")), ("instructions", t(site, "Items can be edited and managed in the admin dashboard.")), ("default", Value::Int(1)), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("show_in_menu")), ("name", string("show_in_menu")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["show_in_menu"])), ("label", t(site, "Show In Admin Menu")), ("instructions", t(site, "Admin editor navigation in the sidebar menu.")), ("default", Value::Int(1)), ("ui", Value::Bool(true)), ("conditions", arr(vec![("field", string("show_ui")), ("operator", string("==")), ("value", string("1"))]))], "div", "label").await?);
                let default_text = if truthy(Some(&g(pt, &["hierarchical"]))) { tr(site, "Categories Meta Box") } else { tr(site, "Tags Meta Box") };
                let custom = strval(Some(&g(pt, &["meta_box"]))) == "custom";
                let mut meta_box_choices = vec![(key("default"), string(&default_text)), (key("custom"), t(site, "Custom Meta Box")), (key("disabled"), t(site, "No Meta Box"))];
                if !cb_edit && !custom {
                    meta_box_choices.remove(1);
                }
                out.push_str(&wrap(r, vec![("type", string("select")), ("key", string("meta_box")), ("name", string("meta_box")), ("class", string("meta_box")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["meta_box"])), ("label", t(site, "Meta Box")), ("instructions", t(site, "Controls the meta box on the content editor screen. By default, the Categories meta box is shown for hierarchical taxonomies, and the Tags meta box is shown for non-hierarchical taxonomies.")), ("hide_search", Value::Bool(true)), ("choices", Value::Array(meta_box_choices)), ("data", arr(vec![("tags_meta_box", t(site, "Tags Meta Box")), ("categories_meta_box", t(site, "Categories Meta Box"))])), ("conditions", arr(vec![("field", string("show_ui")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator")), ("conditions", list(vec![arr(vec![("field", string("meta_box")), ("operator", string("!=")), ("value", string("custom"))]), arr(vec![("field", string("show_ui")), ("operator", string("==")), ("value", string("1"))])]))], "div", "label").await?);
                if cb_edit || custom {
                    let mut instructions = tr(site, "A PHP function name to be called to handle the content of a meta box on your taxonomy. For security, this callback will be executed in a special context without access to any superglobals like $_POST or $_GET.");
                    if !cb_edit {
                        instructions = format!("{instructions} {}", tr(site, "By default only admin users can edit this setting."));
                    }
                    out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("meta_box_cb")), ("key", string("meta_box_cb")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["meta_box_cb"])), ("label", t(site, "Register Meta Box Callback")), ("instructions", string(&instructions)), ("readonly", Value::Bool(!cb_edit)), ("conditions", arr(vec![("field", string("meta_box")), ("operator", string("==")), ("value", string("custom"))]))], "div", "field").await?);
                    out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("meta_box_sanitize_cb")), ("key", string("meta_box_sanitize_cb")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["meta_box_sanitize_cb"])), ("label", t(site, "Meta Box Sanitization Callback")), ("instructions", t(site, "A PHP function name to be called for sanitizing taxonomy data saved from a meta box.")), ("readonly", Value::Bool(!cb_edit)), ("conditions", arr(vec![("field", string("meta_box")), ("operator", string("==")), ("value", string("custom"))]))], "div", "field").await?);
                    out.push_str(&wrap(r, vec![("type", string("seperator")), ("conditions", arr(vec![("field", string("meta_box")), ("operator", string("==")), ("value", string("custom"))]))], "div", "label").await?);
                }
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("show_in_nav_menus")), ("name", string("show_in_nav_menus")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["show_in_nav_menus"])), ("label", t(site, "Appearance Menus Support")), ("instructions", t(site, "Allow items to be added to menus in the 'Appearance' > 'Menus' screen. Must be turned on in 'Screen options'.")), ("default", Value::Int(1)), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("show_tagcloud")), ("name", string("show_tagcloud")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["show_tagcloud"])), ("label", t(site, "Tag Cloud")), ("instructions", t(site, "List the taxonomy in the Tag Cloud Widget controls.")), ("default", Value::Int(1)), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("show_in_quick_edit")), ("name", string("show_in_quick_edit")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["show_in_quick_edit"])), ("label", t(site, "Quick Edit")), ("instructions", t(site, "Show the taxonomy in the quick/bulk edit panel.")), ("default", Value::Int(1)), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("show_admin_column")), ("name", string("show_admin_column")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["show_admin_column"])), ("label", t(site, "Show Admin Column")), ("instructions", t(site, "Display a column for the taxonomy on post type listing screens.")), ("ui", Value::Bool(true))], "div", "label").await?);
            }
            "urls" => {
                out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("permalink_rewrite")), ("key", string("permalink_rewrite")), ("prefix", string("acf_taxonomy[rewrite]")), ("value", g_or(pt, &["rewrite", "permalink_rewrite"], string("taxonomy_key"))), ("label", t(site, "Permalink Rewrite")), ("instructions", cat(vec![t(site, "Select the type of permalink to use for this taxonomy."), string(" {slug}.")])), ("choices", arr(vec![("taxonomy_key", t(site, "Taxonomy Key")), ("custom_permalink", t(site, "Custom Permalink")), ("no_permalink", t(site, "No Permalink (prevent URL rewriting)"))])), ("default", string("taxonomy_key")), ("hide_search", Value::Bool(true)), ("data", arr(vec![("taxonomy_key_instructions", cat(vec![t(site, "Rewrite the URL using the taxonomy key as the slug. Your permalink structure will be"), string(" {slug}.")])), ("custom_permalink_instructions", cat(vec![t(site, "Rewrite the URL using a custom slug defined in the input below. Your permalink structure will be"), string(" {slug}.")])), ("no_permalink_instructions", t(site, "Permalinks for this taxonomy are disabled.")), ("site_url", string(&site.settings.siteurl))])), ("class", string("permalink_rewrite"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("slug")), ("key", string("slug")), ("prefix", string("acf_taxonomy[rewrite]")), ("value", g_or(pt, &["rewrite", "slug"], string(""))), ("label", t(site, "URL Slug")), ("instructions", t(site, "Customize the slug used in the URL")), ("conditions", arr(vec![("field", string("permalink_rewrite")), ("operator", string("==")), ("value", string("custom_permalink"))])), ("class", string("rewrite_slug_field"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("with_front")), ("key", string("with_front")), ("prefix", string("acf_taxonomy[rewrite]")), ("value", g_or(pt, &["rewrite", "with_front"], Value::Bool(true))), ("label", t(site, "Front URL Prefix")), ("instructions", t(site, "Alters the permalink structure to add the `WP_Rewrite::$front` prefix to URLs.")), ("ui", Value::Bool(true)), ("default", Value::Int(1)), ("conditions", arr(vec![("field", string("permalink_rewrite")), ("operator", string("!=")), ("value", string("no_permalink"))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("rewrite_hierarchical")), ("key", string("rewrite_hierarchical")), ("prefix", string("acf_taxonomy[rewrite]")), ("value", g_or(pt, &["rewrite", "rewrite_hierarchical"], Value::Bool(false))), ("label", t(site, "Hierarchical")), ("instructions", t(site, "Parent-child terms in URLs for hierarchical taxonomies.")), ("ui", Value::Bool(true)), ("default", Value::Int(0)), ("conditions", arr(vec![("field", string("permalink_rewrite")), ("operator", string("!=")), ("value", string("no_permalink"))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator"))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("name", string("publicly_queryable")), ("key", string("publicly_queryable")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["publicly_queryable"])), ("label", t(site, "Publicly Queryable")), ("instructions", t(site, "URLs for an item and items can be accessed with a query string.")), ("default", Value::Int(1)), ("ui", Value::Bool(true))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("seperator")), ("conditions", arr(vec![("field", string("publicly_queryable")), ("operator", string("==")), ("value", Value::Int(1))]))], "div", "label").await?);
                out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("query_var")), ("key", string("query_var")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["query_var"])), ("label", t(site, "Query Variable Support")), ("instructions", t(site, "Terms can be accessed using the non-pretty permalink, e.g., {query_var}={term_slug}.")), ("choices", arr(vec![("post_type_key", t(site, "Taxonomy Key")), ("custom_query_var", t(site, "Custom Query Variable")), ("none", t(site, "No Query Variable Support"))])), ("default", Value::Int(1)), ("hide_search", Value::Bool(true)), ("class", string("query_var")), ("conditions", arr(vec![("field", string("publicly_queryable")), ("operator", string("==")), ("value", Value::Int(1))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("query_var_name")), ("key", string("query_var_name")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["query_var_name"])), ("label", t(site, "Query Variable")), ("instructions", t(site, "Customize the query variable name")), ("ui", Value::Bool(true)), ("conditions", arr(vec![("field", string("query_var")), ("operator", string("==")), ("value", string("custom_query_var"))]))], "div", "field").await?);
            }
            "permissions" => {
                let all_caps = all_capabilities(site);
                out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("manage_terms")), ("key", string("manage_terms")), ("prefix", string("acf_taxonomy[capabilities]")), ("value", g(pt, &["capabilities", "manage_terms"])), ("label", t(site, "Manage Terms Capability")), ("instructions", t(site, "The capability name for managing terms of this taxonomy.")), ("choices", all_caps.clone()), ("default", string("manage_categories")), ("class", string("acf-taxonomy-manage_terms"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("edit_terms")), ("key", string("edit_terms")), ("prefix", string("acf_taxonomy[capabilities]")), ("value", g(pt, &["capabilities", "edit_terms"])), ("label", t(site, "Edit Terms Capability")), ("instructions", t(site, "The capability name for editing terms of this taxonomy.")), ("choices", all_caps.clone()), ("default", string("manage_categories")), ("class", string("acf-taxonomy-edit_terms"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("delete_terms")), ("key", string("delete_terms")), ("prefix", string("acf_taxonomy[capabilities]")), ("value", g(pt, &["capabilities", "delete_terms"])), ("label", t(site, "Delete Terms Capability")), ("instructions", t(site, "The capability name for deleting terms of this taxonomy.")), ("choices", all_caps.clone()), ("default", string("manage_categories")), ("class", string("acf-taxonomy-delete_terms"))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("select")), ("name", string("assign_terms")), ("key", string("assign_terms")), ("prefix", string("acf_taxonomy[capabilities]")), ("value", g(pt, &["capabilities", "assign_terms"])), ("label", t(site, "Assign Terms Capability")), ("instructions", t(site, "The capability name for assigning terms of this taxonomy.")), ("choices", all_caps.clone()), ("default", string("edit_posts")), ("class", string("acf-taxonomy-assign_terms"))], "div", "field").await?);
            }
            "rest_api" => {
                out.push_str(&wrap(r, vec![("type", string("true_false")), ("key", string("show_in_rest")), ("name", string("show_in_rest")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["show_in_rest"])), ("label", t(site, "Show In REST API")), ("instructions", t(site, "Expose this post type in the REST API.")), ("default", Value::Int(1)), ("ui", Value::Bool(true))], "div", "label").await?);
                // Between `?>` and `<?php` of the view.
                out.push_str("\n\t\t\t");
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("rest_base")), ("name", string("rest_base")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["rest_base"])), ("label", t(site, "Base URL")), ("instructions", t(site, "The base URL for the post type REST API URLs.")), ("conditions", arr(vec![("field", string("show_in_rest")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("name", string("rest_namespace")), ("key", string("rest_namespace")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["rest_namespace"])), ("label", t(site, "Namespace Route")), ("instructions", t(site, "The namespace part of the REST API URL.")), ("placeholder", string("wp/v2")), ("conditions", arr(vec![("field", string("show_in_rest")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
                out.push_str(&wrap(r, vec![("type", string("text")), ("key", string("rest_controller_class")), ("name", string("rest_controller_class")), ("prefix", string("acf_taxonomy")), ("value", g(pt, &["rest_controller_class"])), ("label", t(site, "Controller Class")), ("instructions", t(site, "Optional custom controller to use instead of `WP_REST_Terms_Controller `.")), ("placeholder", string("WP_REST_Terms_Controller")), ("conditions", arr(vec![("field", string("show_in_rest")), ("operator", string("==")), ("value", string("1"))]))], "div", "field").await?);
            }
            _ => {}
        }
        out.push_str("</div>");
    }
    Ok(out)
}
