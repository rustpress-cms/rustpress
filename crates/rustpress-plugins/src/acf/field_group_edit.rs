//! The field group editor (`post.php`/`post-new.php` of `acf-field-group`): `ACF_Admin_Internal_Post_Type` and
//! `ACF_Admin_Field_Group` – the scripts and styles, the header of `views/global/form-top.php`, the boxes "Fields"
//! (`views/acf-field-group/fields.php` and `field.php` with every field's settings) and "Settings"
//! (`options.php` with the location rules), the form data, the one-column layout, the screen settings and
//! `acf.data` with the field types.

use futures::future::BoxFuture;
use rustpress_core::Result;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::native::{AdminContext, AjaxAnswer, Assets, FormPart, PluginMetaBox, PostMetaBoxes};
use rustpress_db::Post;
use rustpress_phpser::{Key, Value};

use super::field_settings::{Env, Part, Tab};
use super::fields::{Field, FieldGroup};
use super::php::{key, string, strval, truthy};
use super::render::{Attrs, Ctx, RenderField, esc_attrs, hidden_input, render_field_wrap};

const PLUGIN_DIR: &str = "wp-content/plugins/advanced-custom-fields";

fn tr(site: &rustpress_core::Site, text: &str) -> String {
    site.trd("acf", text)
}

fn esc_tr(site: &rustpress_core::Site, text: &str) -> String {
    esc_html(&tr(site, text))
}

/// The field group of `post.php`/`post-new.php`.
pub fn edited_group<'a>(ctx: &AdminContext<'a>) -> Option<&'a Post> {
    if !matches!(ctx.hook_suffix, "post.php" | "post-new.php") {
        return None;
    }
    ctx.edited_post.filter(|p| p.post_type == "acf-field-group")
}

/// `ACF_Admin_Internal_Post_Type::admin_body_class()`.
pub fn admin_body_class(ctx: &AdminContext<'_>, classes: String) -> String {
    if edited_group(ctx).is_none() {
        return classes;
    }
    format!("{classes} acf-admin-page acf-internal-post-type acf-admin-single-field-group")
}

/// The texts `ACF_Admin_Field_Group::admin_enqueue_scripts()` localizes, as `acf_localize_text()` gets them.
fn texts(site: &rustpress_core::Site) -> Vec<(&'static str, String)> {
    [
        "The string \"field_\" may not be used at the start of a field name",
        "This field cannot be moved until its changes have been saved",
        "Field group title is required",
        "Move field group to trash?",
        "No toggle fields available",
        "Move Custom Field",
        "Field moved to other group",
        "Field groups linked successfully.",
        "Checked",
        "(no label)",
        "(this field)",
        "copy",
        "or",
        "Show this field group if",
        "Null",
        "PRO Only",
        "Has any value",
        "Has no value",
        "Value is equal to",
        "Value is not equal to",
        "Value matches pattern",
        "Value contains",
        "Value is greater than",
        "Value is less than",
        "Selection is greater than",
        "Selection is less than",
        "Relationship is equal to",
        "Relationship is not equal to",
        "Relationships contain",
        "Relationships do not contain",
        "Post is equal to",
        "Post is not equal to",
        "Posts contain",
        "Posts do not contain",
        "Has any post selected",
        "Has no post selected",
        "Has any relationship selected",
        "Has no relationship selected",
        "Page is equal to",
        "Page is not equal to",
        "Pages contain",
        "Pages do not contain",
        "Has any page selected",
        "Has no page selected",
        "User is equal to",
        "User is not equal to",
        "Users contain",
        "Users do not contain",
        "Has any user selected",
        "Has no user selected",
        "Term is equal to",
        "Term is not equal to",
        "Terms contain",
        "Terms do not contain",
        "Has any term selected",
        "Has no term selected",
        "Type to search...",
        "This Field",
    ]
    .iter()
    .map(|text| (*text, esc_tr(site, text)))
    .collect()
}

/// `ACF_Admin_Internal_Post_Type::current_screen()` (`acf_enqueue_scripts()`) and `admin_enqueue_scripts()`.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    if edited_group(ctx).is_none() {
        return;
    }
    let site = ctx.site;
    super::form::enqueue_with_texts(site, ctx.wp_version, assets, true, &texts(site));
    let url = |path: &str| site.site_url(&format!("{PLUGIN_DIR}/{path}"));
    let mut own = Assets::default();
    own.script("acf-internal-post-type", url("assets/build/js/acf-internal-post-type.min.js"), &["acf-input"], super::VERSION, false);
    own.style("acf-field-group", url("assets/build/css/acf-field-group.min.css"), &["acf-input"], super::VERSION, "all");
    own.script("acf-field-group", url("assets/build/js/acf-field-group.min.js"), &["acf-input"], super::VERSION, false);
    for mut asset in own.styles {
        asset.priority = 20;
        assets.styles.push(asset);
    }
    for mut asset in own.scripts {
        asset.priority = 20;
        assets.scripts.push(asset);
    }
    // `wp_dequeue_script( 'autosave' )`.
    assets.dequeue.push("autosave".into());
}

/// `get_user_option_screen_layout_acf-field-group`: one column.
pub fn screen_layout(screen_id: &str) -> Option<u32> {
    (screen_id == "acf-field-group").then_some(1)
}

/// `ACF_Admin_Field_Group::screen_settings()`: the field keys and the settings tabs.
pub async fn screen_settings(ctx: &AdminContext<'_>, html: String) -> Result<String> {
    if edited_group(ctx).is_none() {
        return Ok(html);
    }
    let site = ctx.site;
    let settings = super::admin_lists::user_settings(site, ctx.user).await?;
    let show_keys = settings.get("show_field_keys").is_some_and(|v| truthy(Some(v)));
    let show_tabs = settings.get("show_field_settings_tabs").is_none_or(|v| truthy(Some(v)));
    Ok(format!(
        "{html}<div id=\"acf-append-show-on-screen\" class=\"acf-hidden\"><label for=\"acf-field-key-hide\"><input id=\"acf-field-key-hide\" type=\"checkbox\" value=\"1\" name=\"show_field_keys\" {} /> {}</label><label for=\"acf-field-settings-tabs\"><input id=\"acf-field-settings-tabs\" type=\"checkbox\" value=\"1\" name=\"show_field_settings_tabs\" {} />{}</label></div>",
        if show_keys { "checked=\"checked\"" } else { "" },
        tr(site, "Field Keys"),
        if show_tabs { "checked=\"checked\"" } else { "" },
        tr(site, "Field Settings Tabs"),
    ))
}

/// `views/global/form-top.php` for a field group.
pub fn form_top(ctx: &AdminContext<'_>, post: &Post) -> String {
    let site = ctx.site;
    let title = if post.post_status == "auto-draft" { "add_new_item" } else { "edit_item" };
    let title = site.types.admin_post_types().into_iter().find(|t| t.name == "acf-field-group").and_then(|t| t.label(title)).unwrap_or_default();
    format!(
        "<div class=\"acf-headerbar acf-headerbar-field-editor\">\n\t<div class=\"acf-headerbar-inner\">\n\n\t\t<div class=\"acf-headerbar-content\">\n\t\t\t<h1 class=\"acf-page-title\">\n\t\t\t{}\t\t\t</h1>\n\t\t\t\t\t\t<div class=\"acf-title-wrap\">\n\t\t\t\t<label class=\"screen-reader-text\" id=\"title-prompt-text\" for=\"title\">{}</label>\n\t\t\t\t<input form=\"post\" type=\"text\" name=\"post_title\" size=\"30\" value=\"{}\" id=\"title\" class=\"acf-headerbar-title-field\" spellcheck=\"true\" autocomplete=\"off\" placeholder=\"{}\" />\n\t\t\t</div>\n\t\t\t\t\t</div>\n\n\t\t<div class=\"acf-headerbar-actions\" id=\"submitpost\">\n\t\t\t\t\t\t\t<a href=\"#\" class=\"acf-btn acf-btn-secondary add-field\">\n\t\t\t\t\t<i class=\"acf-icon acf-icon-plus\"></i>\n\t\t\t\t\t{}\t\t\t\t</a>\n\t\t\t\t\t\t<button form=\"post\" class=\"acf-btn acf-publish\" name=\"{}\" type=\"submit\">\n\t\t\t\t{}\t\t\t</button>\n\t\t</div>\n\n\t</div>\n</div>\n",
        esc_html(&title),
        esc_html(&site.tr("Add title")),
        esc_attr(&post.post_title),
        esc_attr(&tr(site, "Field Group Title")),
        esc_tr(site, "Add Field"),
        if post.post_status == "publish" { "save" } else { "publish" },
        esc_tr(site, "Save Changes"),
    )
}

/// The fields of a group as `acf_get_fields()` gives them: from the groups that are loaded, else from the database
/// (an auto-draft, a trashed group).
async fn group_fields(groups: &[FieldGroup], site: &rustpress_core::Site, post: &Post) -> Result<Vec<Field>> {
    if let Some(group) = groups.iter().find(|g| g.id == post.id) {
        return Ok(group.fields.clone());
    }
    let all = site.db.acf_fields().await?;
    Ok(super::fields::children_of(post.id, &all))
}

/// The label of a field by its key, for the targets of bidirectional fields.
fn field_labels(groups: &[FieldGroup]) -> Vec<(String, String)> {
    fn walk(fields: &[Field], out: &mut Vec<(String, String)>) {
        for field in fields {
            out.push((field.key.clone(), strval(field.get("label"))));
            walk(&field.sub_fields, out);
        }
    }
    let mut out = Vec::new();
    for group in groups {
        walk(&group.fields, &mut out);
    }
    out
}

/// `acf_render_field_setting( $field, $setting, $global )`.
async fn render_setting(render: &Ctx<'_>, field: &RenderField, items: Vec<(Key, Value)>, global: bool) -> Result<String> {
    let mut setting = RenderField::raw(items, Vec::new());
    let name = strval(setting.get("name"));
    let mut wrapper = match setting.get("wrapper") {
        Some(Value::Array(items)) => items.clone(),
        _ => Vec::new(),
    };
    super::php::set(&mut wrapper, "data-key", string(&name));
    let class = format!("{} acf-field-setting-{name}", strval(wrapper.iter().find(|(k, _)| super::php::key_eq(k, &key("class"))).map(|(_, v)| v)));
    super::php::set(&mut wrapper, "class", string(&class));
    if !global {
        super::php::set(&mut wrapper, "data-setting", field.get("type").cloned().unwrap_or(Value::Null));
    }
    let mut class = class;
    if truthy(setting.get("append")) {
        class.push_str(" acf-field-appended");
    }
    if truthy(setting.get("prepend")) {
        class.push_str(" acf-field-prepended");
    }
    super::php::set(&mut wrapper, "class", string(&class));
    setting.set("prefix", field.get("prefix").cloned().unwrap_or(Value::Null));
    if matches!(setting.get("value"), None | Some(Value::Null)) {
        if let Some(value) = field.get(&name).filter(|v| !matches!(v, Value::Null)) {
            setting.set("value", value.clone());
        } else if let Some(default) = setting.get("default_value").filter(|v| !matches!(v, Value::Null)).cloned() {
            setting.set("value", default);
        }
    }
    if let Some(append) = setting.get("_append").cloned() {
        super::php::set(&mut wrapper, "data-append", append);
    }
    setting.set("wrapper", Value::Array(wrapper));
    let instruction = if truthy(setting.get("instructions")) && strval(setting.get("type")) != "true_false" { "field" } else { "label" };
    render_field_wrap(render, setting, "div", instruction).await
}

/// A field array as the editor builds it: the field, its prefix `acf_fields[{$ID}]`.
fn editor_field(render: &Ctx<'_>, field: &Field) -> RenderField {
    let mut f = RenderField::load(field, render);
    let prefix = format!("acf_fields[{}]", strval(f.get("ID")));
    f.set("prefix", string(&prefix));
    f
}

/// What the views need besides the field.
struct View<'a> {
    render: &'a Ctx<'a>,
    env: &'a Env<'a>,
    /// `count( acf_get_field_groups() )`.
    num_groups: usize,
    /// `$field_group['ID']`.
    group_id: u64,
}

/// `views/acf-field-group/conditional-logic.php`.
async fn conditional_logic(view: &View<'_>, field: &RenderField) -> Result<String> {
    let site = view.render.site;
    let prefix = strval(field.get("prefix"));
    let (disabled, groups) = match field.get("conditional_logic") {
        Some(Value::Array(groups)) if !groups.is_empty() => (false, groups.clone()),
        _ => (true, vec![(Key::Int(0), Value::Array(vec![(Key::Int(0), Value::Array(Vec::new()))]))]),
    };
    let toggle = RenderField::raw(vec![(key("type"), string("true_false")), (key("name"), string("conditional_logic")), (key("prefix"), string(&prefix)), (key("value"), Value::Int(if disabled { 0 } else { 1 })), (key("ui"), Value::Int(1)), (key("class"), string("conditions-toggle"))], Vec::new());
    let mut out = format!(
        "<div class=\"acf-field acf-field-true-false acf-field-setting-conditional_logic\" data-type=\"true_false\" data-name=\"conditional_logic\">\n\t<div class=\"acf-conditional-toggle\">\n\t\t<div class=\"acf-label\">\n\t\t\t\t\t\t<label for=\"{}\">{}</label>\n\t\t</div>\n\t\t<div class=\"acf-input\">\n\t\t\t{}\n\t\t</div>\n\t</div>\n\t<div class=\"rule-groups\" \n\t{}\t>\n\t\t",
        esc_attr(&super::render::idify(&format!("{prefix}[conditional_logic]"))),
        esc_tr(site, "Conditional Logic"),
        super::render::render_field(view.render, toggle).await?,
        if disabled { " style=\"display:none\"" } else { "" },
    );
    for (group_index, group) in &groups {
        let Value::Array(rules) = group else { continue };
        if rules.is_empty() {
            continue;
        }
        let group_id = format!("group_{}", super::render::key_text(group_index));
        let h4 = if group_id == "group_0" { tr(site, "Show this field if") } else { tr(site, "or") };
        out.push_str(&format!("\t\t\t<div class=\"rule-group\" data-id=\"{}\">\n\n\t\t\t\t<h4>{}</h4>\n\n\t\t\t\t<table class=\"acf-table -clear\">\n\t\t\t\t\t<tbody>\n\t\t\t\t\t", esc_attr(&group_id), esc_html(&h4)));
        for (rule_index, rule) in rules {
            let get = |name: &str| rule.get(name).cloned().unwrap_or_else(|| string(""));
            let (rule_field, operator, value) = (get("field"), get("operator"), get("value"));
            let rule_id = format!("rule_{}", super::render::key_text(rule_index));
            let rule_prefix = format!("{prefix}[conditional_logic][{group_id}][{rule_id}]");
            let attrs = Attrs::new(&[("data-id", string(&rule_id)), ("data-field", rule_field.clone()), ("data-operator", operator.clone()), ("data-value", value.clone())]);
            let select = |name: &str, class: &str, value: &Value| {
                RenderField::raw(
                    vec![
                        (key("type"), string("select")),
                        (key("prefix"), string(&rule_prefix)),
                        (key("name"), string(name)),
                        (key("class"), string(class)),
                        (key("disabled"), Value::Bool(disabled)),
                        (key("value"), value.clone()),
                        (key("choices"), Value::Array(vec![(key(&strval(Some(value))), value.clone())])),
                    ],
                    Vec::new(),
                )
            };
            out.push_str(&format!(
                "\t\t\t\t\t\t<tr class=\"rule\" {}>\n\t\t\t\t\t\t\t<td class=\"param\">\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t<td class=\"operator\">\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t<td class=\"value\">\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t<td class=\"add\">\n\t\t\t\t\t\t\t\t<a href=\"#\" class=\"button add-conditional-rule\">{}</a>\n\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t<td class=\"remove\">\n\t\t\t\t\t\t\t\t<a href=\"#\" class=\"acf-icon -minus remove-conditional-rule\"></a>\n\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t</tr>\n\t\t\t\t\t\t",
                esc_attrs(&attrs),
                super::render::render_field(view.render, select("field", "condition-rule-field", &rule_field)).await?,
                super::render::render_field(view.render, select("operator", "condition-rule-operator", &operator)).await?,
                super::render::render_field(view.render, select("value", "condition-rule-value", &value)).await?,
                esc_tr(site, "and"),
            ));
        }
        out.push_str("\t\t\t\t\t</tbody>\n\t\t\t\t</table>\n\n\t\t\t</div>\n\t\t");
    }
    out.push_str(&format!("\n\t\t<h4>{}</h4>\n\n\t\t<a href=\"#\" class=\"button add-conditional-group\">{}</a>\n\t</div>\t\t\t\t\t\t\t\n</div>\n", esc_tr(site, "or"), esc_tr(site, "Add rule group")));
    Ok(out)
}

/// The type-specific settings of a tab: `render_field_{$tab}_settings()` with what ACF adds before it.
async fn type_settings(view: &View<'_>, field: &mut RenderField, tab: Tab) -> Result<String> {
    let site = view.render.site;
    let kind = strval(field.get("type"));
    let mut parts: Vec<Part> = Vec::new();
    match tab {
        Tab::Validation => parts.extend(super::field_settings::required_setting(site, &kind)),
        Tab::Presentation => parts.extend(super::field_settings::bindings_setting(site, field)),
        _ => {}
    }
    // `render_required_setting()` renders the field as it was; the type's settings may change it.
    let mut out = String::new();
    for part in parts {
        if let Part::Setting(items) = part {
            out.push_str(&render_setting(view.render, field, items, true).await?);
        }
    }
    for part in super::field_settings::settings(view.env, field, tab).await? {
        match part {
            Part::Setting(items) => out.push_str(&render_setting(view.render, field, items, false).await?),
            Part::Html(html) => out.push_str(&html),
            Part::SubFields => {
                let sub_fields: Vec<RenderField> = field.sub_fields.iter().map(|f| editor_field(view.render, f)).collect();
                let parent = field.get("ID").cloned().unwrap_or(Value::Null);
                out.push_str(&format!(
                    "\t\t\t<div class=\"acf-field acf-field-setting-sub_fields\" data-setting=\"group\" data-name=\"sub_fields\">\n\t\t\t\t<div class=\"acf-label\">\n\t\t\t\t\t<label>{}</label>\t\n\t\t\t\t</div>\n\t\t\t\t<div class=\"acf-input acf-input-sub\">\n\t\t\t\t\t{}\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t",
                    esc_tr(site, "Sub Fields"),
                    fields_list(view, sub_fields, parent, true).await?
                ));
            }
        }
    }
    Ok(out)
}

/// `views/acf-field-group/field.php`.
fn field_object<'a>(view: &'a View<'a>, mut field: RenderField, i: usize) -> BoxFuture<'a, Result<String>> {
    Box::pin(async move {
        let site = view.render.site;
        let id = strval(field.get("ID"));
        let prefix = format!("acf_fields[{id}]");
        let input_id = super::render::idify(&prefix);
        field.set("prefix", string(&prefix));
        let kind = strval(field.get("type"));
        let slug_kind = super::admin_lists::slugify_ascii(&kind);
        let mut class = format!("acf-field-object acf-field-object-{slug_kind}");
        if truthy(field.get("endpoint")) {
            class.push_str(" acf-field-is-endpoint");
        }
        let div = Attrs::new(&[("class", string(&class)), ("data-id", field.get("ID").cloned().unwrap_or(Value::Null)), ("data-key", field.get("key").cloned().unwrap_or(Value::Null)), ("data-type", string(&kind))]);
        // `acf_get_field_label( $field, 'admin' )`.
        let mut label = esc_html(&strval(field.get("label")));
        if label.is_empty() {
            label = tr(site, "(no label)");
        }
        if truthy(field.get("required")) {
            label.push_str(" <span class=\"acf-required\">*</span>");
        }
        let type_label = super::field_types::type_label(site, &kind);
        let conditional = matches!(field.get("conditional_logic"), Some(Value::Array(groups)) if !groups.is_empty());
        let mut meta = String::new();
        for (name, value) in [("ID", field.get("ID").cloned().unwrap_or(Value::Null)), ("key", field.get("key").cloned().unwrap_or(Value::Null)), ("parent", field.get("parent").cloned().unwrap_or(Value::Null)), ("menu_order", Value::Int(i as i64)), ("save", string(""))] {
            meta.push_str(&hidden_input(&Attrs::new(&[("name", string(&format!("{prefix}[{name}]"))), ("value", value), ("id", string(&format!("{input_id}-{name}")))])));
        }
        let move_link = if view.num_groups > 1 { format!("\t\t\t\t\t<a class=\"move-field\" title=\"{}\" href=\"#\">{}</a>\n\t\t\t\t\t", esc_attr(&tr(site, "Move field to another group")), esc_tr(site, "Move")) } else { String::new() };
        let mut out = format!(
            "<div {}>\n\n\t<div class=\"meta\">\n\t\t{meta}\t</div>\n\n\t<div class=\"handle\">\n\t\t<ul class=\"acf-hl acf-tbody\">\n\t\t\t<li class=\"li-field-order\">\n\t\t\t\t<span class=\"acf-icon acf-sortable-handle\" title=\"{}\">{}</span>\n\t\t\t</li>\n\t\t\t<li class=\"li-field-label\">\n\t\t\t\t<strong class=\"\" title=\"\">\n\t\t\t\t\t<a class=\"edit-field\" title=\"{}\" href=\"#\">{}</a>\n\t\t\t\t</strong>\n\t\t\t\t<div class=\"row-options\">\n\t\t\t\t\t<a class=\"edit-field\" title=\"{}\" href=\"#\">{}</a>\n\t\t\t\t\t<a class=\"duplicate-field\" title=\"{}\" href=\"#\">{}</a>\n\t\t\t\t\t{move_link}\t\t\t\t\t<a class=\"delete-field\" title=\"{}\" href=\"#\">{}</a>\n\t\t\t\t</div>\n\t\t\t</li>\n\t\t\t<li class=\"li-field-name\"><span class=\"copyable\">{}</span></li>\n\t\t\t<li class=\"li-field-key\"><span class=\"copyable\">{}</span></li>\n\t\t\t<li class=\"li-field-type\">\n\t\t\t\t<i class=\"field-type-icon field-type-icon-{}\"></i>\n\t\t\t\t<span class=\"field-type-label\">\n\t\t\t\t\t{}\t\t\t\t</span>\n\t\t\t</li>\n\t\t</ul>\n\t</div>\n\n\t<div class=\"settings\">\n\t\t<div class=\"acf-field-editor\">\n\t\t\t<div class=\"acf-field-settings acf-fields\">\n\n\t\t\t\t",
            esc_attrs(&div),
            esc_attr(&tr(site, "Drag to reorder")),
            i + 1,
            esc_attr(&tr(site, "Edit field")),
            super::render::esc_html_acf(&string(&label)),
            esc_attr(&tr(site, "Edit field")),
            esc_tr(site, "Edit"),
            esc_attr(&tr(site, "Duplicate field")),
            esc_tr(site, "Duplicate"),
            esc_attr(&tr(site, "Delete field")),
            esc_tr(site, "Delete"),
            esc_html(&strval(field.get("name"))),
            esc_html(&strval(field.get("key"))),
            esc_attr(&slug_kind),
            super::render::esc_html_acf(&string(&type_label)),
        );
        for tab in Tab::ALL {
            let mut label = tab.label(site);
            if tab == Tab::ConditionalLogic {
                label = format!("{}<i class=\"conditional-logic-badge{}\">{}</i>", tr(site, "Conditional Logic"), if conditional { " is-enabled" } else { "" }, if conditional { tr(site, "Active") } else { String::new() });
            }
            let tab_field = RenderField::raw(vec![(key("type"), string("tab")), (key("label"), string(&label)), (key("key"), string("acf_field_settings_tabs")), (key("settings-type"), string(tab.key()))], Vec::new());
            out.push_str(&render_field_wrap(view.render, tab_field, "div", "label").await?);
            out.push_str(&format!("\t\t\t\t\t\t\t\t\t\t<div class=\"acf-field-settings-main acf-field-settings-main-{}\">\n\t\t\t\t\t\t", esc_attr(&tab.key().replace('_', "-"))));
            let type_box = |content: String| format!("\t\t\t\t\t\t\t\t<div class=\"acf-field-type-settings\" data-parent-tab=\"{}\">\n\t\t\t\t\t\t\t\t\t{content}\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t", esc_attr(tab.key()));
            match tab {
                Tab::General => {
                    let choices = super::field_types::grouped(site);
                    out.push_str(&render_setting(view.render, &field, vec![(key("label"), string(&tr(site, "Field Type"))), (key("instructions"), string("")), (key("type"), string("select")), (key("name"), string("type")), (key("choices"), choices), (key("class"), string("field-type"))], true).await?);
                    out.push_str(&format!("\t\t\t\t\t\t\t\t\t<div class=\"acf-field acf-field-setting-browse-fields\" data-append=\"type\">\n\t\t\t\t\t\t\t\t\t\t<div class=\"acf-input\">\n\t\t\t\t\t\t\t\t\t\t\t<button class=\"acf-btn browse-fields\">\n\t\t\t\t\t\t\t\t\t\t\t\t<i class=\"acf-icon acf-icon-dots-grid\"></i>\n\t\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t\t</button>\n\t\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t", esc_tr(site, "Browse Fields")));
                    out.push_str(&render_setting(view.render, &field, vec![(key("label"), string(&tr(site, "Field Label"))), (key("instructions"), string(&tr(site, "This is the name which will appear on the EDIT page"))), (key("name"), string("label")), (key("type"), string("text")), (key("class"), string("field-label"))], true).await?);
                    out.push_str(&render_setting(view.render, &field, vec![(key("label"), string(&tr(site, "Field Name"))), (key("instructions"), string(&tr(site, "Single word, no spaces. Underscores and dashes allowed"))), (key("name"), string("name")), (key("type"), string("text")), (key("class"), string("field-name"))], true).await?);
                    let settings = type_settings(view, &mut field, tab).await?;
                    out.push_str(&type_box(settings));
                }
                Tab::Validation | Tab::Advanced => {
                    let settings = type_settings(view, &mut field, tab).await?;
                    out.push_str(&type_box(settings));
                }
                Tab::Presentation => {
                    out.push_str(&render_setting(view.render, &field, vec![(key("label"), string(&tr(site, "Instructions"))), (key("instructions"), string(&tr(site, "Instructions for content editors. Shown when submitting data."))), (key("type"), string("textarea")), (key("name"), string("instructions")), (key("rows"), Value::Int(5))], true).await?);
                    let wrapper = |name: &str| field.get("wrapper").and_then(|w| w.get(name)).cloned().unwrap_or_else(|| string(""));
                    for (name, prepend) in [("class", "class"), ("id", "id")] {
                        let wrap = RenderField::raw(
                            vec![
                                (key("label"), string("")),
                                (key("instructions"), string("")),
                                (key("type"), string("text")),
                                (key("name"), string(name)),
                                (key("prefix"), string(&format!("{prefix}[wrapper]"))),
                                (key("value"), wrapper(name)),
                                (key("prepend"), string(&tr(site, prepend))),
                                (key("wrapper"), Value::Array(vec![(key("data-append"), string("wrapper"))])),
                            ],
                            Vec::new(),
                        );
                        out.push_str(&render_field_wrap(view.render, wrap, "div", "label").await?);
                    }
                    let width = wrapper("width");
                    let settings = type_settings(view, &mut field, tab).await?;
                    out.push_str(&type_box(settings));
                    let wrap = RenderField::raw(
                        vec![
                            (key("label"), string(&tr(site, "Wrapper Attributes"))),
                            (key("instructions"), string("")),
                            (key("type"), string("number")),
                            (key("name"), string("width")),
                            (key("prefix"), string(&format!("{prefix}[wrapper]"))),
                            (key("value"), width),
                            (key("prepend"), string(&tr(site, "width"))),
                            (key("append"), string("%")),
                            (key("wrapper"), Value::Array(vec![(key("data-name"), string("wrapper")), (key("class"), string("acf-field-setting-wrapper"))])),
                        ],
                        Vec::new(),
                    );
                    out.push_str(&render_field_wrap(view.render, wrap, "div", "label").await?);
                }
                Tab::ConditionalLogic => {
                    out.push_str(&conditional_logic(view, &field).await?);
                    let settings = type_settings(view, &mut field, tab).await?;
                    out.push_str(&type_box(settings));
                }
            }
            out.push_str("\t\t\t\t\t</div>\n\t\t\t\t\t");
        }
        out.push_str(&format!(
            "\t\t\t\t<div class=\"acf-field-settings-footer\">\n\t\t\t\t\t<a class=\"button close-field edit-field\" title=\"{}\" href=\"#\">{}</a>\n\t\t\t\t\t<a class=\"acf-btn acf-btn-secondary close-add-field\" title=\"{}\" href=\"#\">{}</a>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t</div>\n\t</div>\n\n</div>\n",
            esc_attr(&tr(site, "Close Field")),
            esc_tr(site, "Close Field"),
            esc_attr(&tr(site, "Close and Add Field")),
            esc_tr(site, "Close and Add Field"),
        ));
        Ok(out)
    })
}

/// `views/acf-field-group/fields.php`.
fn fields_list<'a>(view: &'a View<'a>, fields: Vec<RenderField>, parent: Value, is_subfield: bool) -> BoxFuture<'a, Result<String>> {
    Box::pin(async move {
        let site = view.render.site;
        let has_parent = truthy(Some(&parent));
        let wrapper_class = if is_subfield {
            if fields.is_empty() { " acf-is-subfields -empty" } else { " acf-is-subfields" }
        } else if fields.is_empty() && !has_parent {
            " acf-auto-add-field"
        } else {
            ""
        };
        let mut out = String::new();
        if has_parent || is_subfield {
            out.push_str(&format!("<div class=\"acf-sub-field-list-header\">\n\t<h3 class=\"acf-sub-field-list-title\">{}</h3>\n\t<a href=\"#\" class=\"acf-btn acf-btn-secondary add-field\"><i class=\"acf-icon acf-icon-plus\"></i>{}</a>\n</div>\n", esc_tr(site, "Fields"), esc_tr(site, "Add Field")));
        }
        let learn_more = rustpress_core::kses::kses_post(&rustpress_core::i18n::sprintf(&tr(site, "Choose from over 30 field types. <a href=\"%s\" target=\"_blank\">Learn more</a>."), &[&super::admin_lists::utm("https://www.advancedcustomfields.com/resources/", "docs", "empty-field-group", "field-types", "", "")]));
        out.push_str(&format!(
            "<div class=\"acf-field-list-wrap{wrapper_class}\">\n\n\t<ul class=\"acf-hl acf-thead\">\n\t\t<li class=\"li-field-order\">\n\t\t\t{}\t\t\t<span class=\"acf-hidden\">\n\t\t\t\t{}\t\t\t</span>\n\t\t</li>\n\t\t<li class=\"li-field-label\">{}</li>\n\t\t<li class=\"li-field-name\">{}</li>\n\t\t<li class=\"li-field-key\">{}</li>\n\t\t<li class=\"li-field-type\">{}</li>\n\t</ul>\n\n\t\t<div class=\"acf-field-list{wrapper_class}\">\n\n\t\t<div class=\"no-fields-message\">\n\t\t\t<div class=\"no-fields-message-inner\">\n\t\t\t\t<img src=\"{}\" />\n\t\t\t\t<h2>{}</h2>\n\t\t\t\t<p>{}</p>\n\t\t\t\t<a href=\"#\" class=\"acf-btn acf-btn-primary add-field add-first-field\n\"><i class=\"acf-icon acf-icon-plus\"></i> {}</a>\n\t\t\t\t<p class=\"acf-small\">\n\t\t\t\t{learn_more}\t\t\t\t</p>\n\t\t\t</div>\n\t\t</div>\n\n\t\t",
            esc_tr(site, "#"),
            esc_tr(site, "Order"),
            esc_tr(site, "Label"),
            esc_tr(site, "Name"),
            esc_tr(site, "Key"),
            esc_tr(site, "Type"),
            esc_url(&site.site_url(&format!("{PLUGIN_DIR}/assets/images/empty-group.svg"))),
            esc_tr(site, "Add Your First Field"),
            esc_tr(site, "Get started creating new custom fields for your posts, pages, custom post types and other WordPress content."),
            esc_tr(site, "Add Field"),
        ));
        for (i, field) in fields.into_iter().enumerate() {
            out.push_str(&field_object(view, field, i).await?);
        }
        out.push_str(&format!("\n\t</div>\n\n\t<ul class=\"acf-hl acf-tfoot\">\n\t\t<li class=\"acf-fr\">\n\t\t\t<a href=\"#\" class=\"acf-btn acf-btn-sm add-field\"><i class=\"acf-icon acf-icon-plus\"></i>{}</a>\n\t\t</li>\n\t</ul>\n\n", esc_tr(site, "Add Field")));
        if !has_parent {
            // The template of a new field (`acf_get_valid_field()`).
            let clone = RenderField::raw(
                vec![(key("ID"), string("acfcloneindex")), (key("key"), string("acfcloneindex")), (key("label"), string(&tr(site, "New Field"))), (key("name"), string("new_field")), (key("type"), string("text")), (key("parent"), Value::Int(view.group_id as i64))],
                Vec::new(),
            );
            out.push_str(&format!("\t<script type=\"text/html\" id=\"tmpl-acf-field\">\n\t{}\t</script>\n\t<script type=\"text/html\" id=\"tmpl-acf-browse-fields-modal\">\n\t\t{}\t</script>\n", field_object(view, clone, 0).await?, browse_fields_modal(site)));
        }
        out.push_str("\n</div>\n");
        Ok(out)
    })
}

/// `views/browse-fields-modal.php`.
fn browse_fields_modal(site: &rustpress_core::Site) -> String {
    let upgrade = super::admin_lists::utm("https://www.advancedcustomfields.com/pro/", "field-type-modal", "", "", "", "");
    let mut tabs = String::new();
    let mut categories = vec![("popular", tr(site, "Popular"))];
    categories.extend(super::field_types::categories(site));
    for (name, label) in categories {
        tabs.push_str(&format!(
            "<div class=\"acf-field acf-field-tab acf-field-acf-browse-fields-tabs\" data-type=\"tab\" data-key=\"acf_browse_fields_tabs\">\n<div class=\"acf-label\">\n<label for=\"acf_browse_fields_tabs\">{}</label></div>\n<div class=\"acf-input\">\n\t\t\t<a href=\"\" class=\"acf-tab-button\" data-placement=\"top\" data-endpoint=\"0\" data-key=\"acf_browse_fields_tabs\" data-selected=\"0\">{}</a>\n\t\t\t</div>\n</div>\n<div class=\"acf-field-types-tab\" data-category=\"{}\"></div>",
            esc_html(&label),
            rustpress_core::kses::kses_post(&label),
            esc_attr(name)
        ));
    }
    let no_results = rustpress_core::i18n::sprintf(&rustpress_core::kses::kses_post(&tr(site, "No search results for '%s'")), &["<span class=\"acf-invalid-search-term\"></span>"]);
    let popular = format!("<a href=\"#\" class=\"acf-browse-popular-fields\">{}</a>", esc_tr(site, "Popular fields"));
    let try_other = rustpress_core::i18n::sprintf(&rustpress_core::kses::kses_post(&tr(site, "Try a different search term or browse %s")), &[&popular]);
    format!(
        "<div class=\"acf-browse-fields-modal-wrap\">\n\t<div class=\"acf-modal acf-browse-fields-modal\">\n\t\t<div class=\"acf-field-picker\">\n\t\t\t<div class=\"acf-modal-title\">\n\t\t\t\t<h1>{}</h1>\n\t\t\t\t<span class=\"acf-search-field-types-wrap\">\n\t\t\t\t\t<input class=\"acf-search-field-types\" type=\"search\" placeholder=\"{}\" />\n\t\t\t\t</span>\n\t\t\t</div>\n\t\t\t<div class=\"acf-modal-content\">\n\t\t\t\t{tabs}\t\t\t\t<div class=\"acf-field-type-search-results\"></div>\n\t\t\t\t<div class=\"acf-field-type-search-no-results\">\n\t\t\t\t\t<img src=\"{}\" />\n\t\t\t\t\t<p class=\"acf-no-results-text\">\n\t\t\t\t\t\t{no_results}\t\t\t\t\t</p>\n\t\t\t\t\t<p>\n\t\t\t\t\t\t{try_other}\t\t\t\t\t</p>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t<div class=\"acf-modal-toolbar acf-field-picker-toolbar\">\n\t\t\t\t<div class=\"acf-field-picker-label\">\n\t\t\t\t\t<input class=\"acf-insert-field-label\" type=\"text\" placeholder=\"{}\" />\n\t\t\t\t</div>\n\t\t\t\t<div class=\"acf-field-picker-actions\">\n\t\t\t\t\t<button class=\"button acf-cancel acf-modal-close\">{}</button>\n\t\t\t\t\t<button class=\"acf-btn acf-select-field\">{}</button>\n\t\t\t\t\t<a target=\"_blank\" data-url-base=\"{}\" class=\"acf-btn acf-btn-pro\">\n\t\t\t\t\t\t{}\t\t\t\t\t</a>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t</div>\n\t\t<div class=\"acf-field-type-preview\">\n\t\t\t<div class=\"field-type-info\">\n\t\t\t\t<h2 class=\"field-type-name\"></h2>\n\t\t\t\t<a target=\"_blank\" data-url-base=\"{}\" class=\"field-type-upgrade-to-unlock\">\n\t\t\t\t\t<i class=\"acf-icon acf-icon-lock\"></i>\n\t\t\t\t\t{}\t\t\t\t</a>\n\t\t\t\t<p class=\"field-type-desc\"></p>\n\t\t\t\t<div class=\"field-type-preview-container\">\n\t\t\t\t\t<img class=\"field-type-image\" />\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t<ul class=\"acf-hl field-type-links\">\n\t\t\t\t<li>\n\t\t\t\t\t<a class=\"field-type-tutorial\" href=\"#\" target=\"_blank\">\n\t\t\t\t\t\t<i class=\"acf-icon acf-icon-play\"></i>\n\t\t\t\t\t\t{}\t\t\t\t\t</a>\n\t\t\t\t</li>\n\t\t\t\t<li>\n\t\t\t\t\t<a class=\"field-type-doc\" href=\"#\" target=\"_blank\">\n\t\t\t\t\t\t<i class=\"acf-icon acf-icon-document\"></i>\n\t\t\t\t\t\t{}\t\t\t\t\t</a>\n\t\t\t\t</li>\n\t\t\t</ul>\n\t\t</div>\n\t</div>\n\t<div class=\"acf-modal-backdrop acf-modal-close\"></div>\n</div>\n",
        esc_tr(site, "Select Field Type"),
        esc_attr(&tr(site, "Search fields...")),
        esc_url(&site.site_url(&format!("{PLUGIN_DIR}/assets/images/face-sad.svg"))),
        esc_attr(&tr(site, "Field Label")),
        esc_tr(site, "Cancel"),
        esc_tr(site, "Select Field"),
        esc_attr(&upgrade),
        esc_tr(site, "Upgrade to PRO"),
        esc_attr(&upgrade),
        esc_tr(site, "ACF PRO Feature"),
        esc_tr(site, "Tutorial"),
        esc_tr(site, "Documentation"),
    )
}

/// The field group as `acf_get_field_group()` validates it: the stored settings over the defaults, `menu_order` and
/// `active` from the post.
fn group_settings(post: &Post) -> Value {
    let stored = rustpress_core::options::maybe_unserialize(&post.post_content);
    let defaults: Vec<(&str, Value)> = vec![
        ("location", Value::Array(Vec::new())),
        ("position", string("normal")),
        ("style", string("default")),
        ("label_placement", string("top")),
        ("instruction_placement", string("label")),
        ("hide_on_screen", Value::Array(Vec::new())),
        ("description", string("")),
        ("show_in_rest", Value::Bool(false)),
        ("display_title", string("")),
    ];
    let mut items: Vec<(Key, Value)> = defaults.into_iter().map(|(k, v)| (key(k), v)).collect();
    if let Value::Array(stored) = stored {
        for (k, v) in stored {
            match items.iter_mut().find(|(existing, _)| super::php::key_eq(existing, &k)) {
                Some(slot) => slot.1 = v,
                None => items.push((k, v)),
            }
        }
    }
    // `mb_options()`: a group without a key gets one (`acf_is_field_group_key()`).
    let key_value = if post.post_name.starts_with("group_") { post.post_name.clone() } else { super::render::uniqid("group_") };
    super::php::set(&mut items, "key", string(&key_value));
    super::php::set(&mut items, "menu_order", Value::Int(i64::from(post.menu_order)));
    super::php::set(&mut items, "active", Value::Bool(matches!(post.post_status.as_str(), "publish" | "auto-draft")));
    Value::Array(items)
}

/// A setting of the field group (`acf_render_field_wrap()` with the prefix `acf_field_group`).
async fn group_field(render: &Ctx<'_>, items: Vec<(&str, Value)>, instruction: &str, tooltip: bool) -> Result<String> {
    let mut items: Vec<(Key, Value)> = items.into_iter().map(|(k, v)| (key(k), v)).collect();
    items.push((key("prefix"), string("acf_field_group")));
    super::render::render_field_wrap_with(render, RenderField::raw(items, Vec::new()), "div", instruction, tooltip).await
}

/// `views/acf-field-group/options.php`.
async fn options_box(ctx: &AdminContext<'_>, render: &Ctx<'_>, post: &Post) -> Result<String> {
    let site = ctx.site;
    let group = group_settings(post);
    let get = |name: &str| group.get(name).cloned().unwrap_or(Value::Null);
    let mut location = super::fields::parse_location(&group);
    if location.is_empty() {
        location = vec![vec![super::fields::Rule { param: "post_type".into(), operator: "==".into(), value: string("post") }]];
    }
    let t = |text: &str| string(&tr(site, text));
    let choices = |items: &[(&str, &str)]| Value::Array(items.iter().map(|(k, v)| (key(k), string(&tr(site, v)))).collect());
    let mut out = String::new();
    for (tab, label) in [("location_rules", "Location Rules"), ("presentation", "Presentation"), ("group_settings", "Group Settings")] {
        let tab_field = RenderField::raw(vec![(key("type"), string("tab")), (key("label"), t(label)), (key("key"), string("acf_field_group_settings_tabs")), (key("settings-type"), string(tab))], Vec::new());
        out.push_str(&render_field_wrap(render, tab_field, "div", "label").await?);
        match tab {
            "location_rules" => {
                out.push_str("<div class=\"field-group-locations field-group-settings-tab\">");
                out.push_str(&super::locations::locations(render, ctx.wp_root, &location).await?);
                out.push_str("</div>");
            }
            "presentation" => {
                out.push_str("<div class=\"field-group-setting-split-container field-group-settings-tab\"><div class=\"field-group-setting-split\">");
                out.push_str(&group_field(render, vec![("label", t("Style")), ("instructions", string("")), ("type", string("button_group")), ("name", string("style")), ("value", get("style")), ("choices", choices(&[("default", "Standard (WP metabox)"), ("seamless", "Seamless (no metabox)")]))], "label", false).await?);
                out.push_str(&group_field(render, vec![("label", t("Position")), ("instructions", t("'High' position not supported in the Block Editor")), ("type", string("button_group")), ("name", string("position")), ("value", get("position")), ("choices", choices(&[("acf_after_title", "High (after title)"), ("normal", "Normal (after content)"), ("side", "Side")])), ("default_value", string("normal"))], "field", false).await?);
                out.push_str(&group_field(render, vec![("label", t("Label Placement")), ("instructions", string("")), ("type", string("button_group")), ("name", string("label_placement")), ("value", get("label_placement")), ("choices", choices(&[("top", "Top aligned"), ("left", "Left aligned")]))], "label", false).await?);
                out.push_str(&group_field(render, vec![("label", t("Instruction Placement")), ("instructions", string("")), ("type", string("button_group")), ("name", string("instruction_placement")), ("value", get("instruction_placement")), ("choices", choices(&[("label", "Below labels"), ("field", "Below fields")]))], "label", false).await?);
                out.push_str(&group_field(render, vec![("label", t("Order No.")), ("instructions", t("Field groups with a lower order will appear first")), ("type", string("number")), ("name", string("menu_order")), ("value", get("menu_order"))], "field", false).await?);
                out.push_str("</div><div class=\"field-group-setting-split\">");
                // `remove_wp_meta_box` is on: no "Custom Fields".
                let hide = choices(&[
                    ("permalink", "Permalink"),
                    ("the_content", "Content Editor"),
                    ("excerpt", "Excerpt"),
                    ("discussion", "Discussion"),
                    ("comments", "Comments"),
                    ("revisions", "Revisions"),
                    ("slug", "Slug"),
                    ("author", "Author"),
                    ("format", "Format"),
                    ("page_attributes", "Page Attributes"),
                    ("featured_image", "Featured Image"),
                    ("categories", "Categories"),
                    ("tags", "Tags"),
                    ("send-trackbacks", "Send Trackbacks"),
                ]);
                let instructions = format!("{}<br /><br />{}", tr(site, "<b>Select</b> items to <b>hide</b> them from the edit screen."), tr(site, "If multiple field groups appear on an edit screen, the first field group's options will be used (the one with the lowest order number)"));
                out.push_str(&group_field(render, vec![("label", t("Hide on screen")), ("instructions", string(&instructions)), ("type", string("checkbox")), ("name", string("hide_on_screen")), ("value", get("hide_on_screen")), ("toggle", Value::Bool(true)), ("choices", hide)], "label", true).await?);
                out.push_str("</div></div>");
            }
            _ => {
                out.push_str("<div class=\"field-group-settings field-group-settings-tab\">");
                out.push_str(&group_field(render, vec![("label", t("Active")), ("instructions", string("")), ("type", string("true_false")), ("name", string("active")), ("value", get("active")), ("ui", Value::Int(1))], "label", false).await?);
                out.push_str(&group_field(render, vec![("label", t("Show in REST API")), ("instructions", string("")), ("type", string("true_false")), ("name", string("show_in_rest")), ("value", get("show_in_rest")), ("ui", Value::Int(1))], "label", false).await?);
                out.push_str(&group_field(render, vec![("label", t("Description")), ("instructions", t("Shown in field group list")), ("type", string("text")), ("name", string("description")), ("value", get("description"))], "field", false).await?);
                out.push_str(&group_field(render, vec![("label", t("Display Title")), ("instructions", t("Title shown on the edit screen for the field group meta box to use instead of the field group title")), ("type", string("text")), ("name", string("display_title")), ("value", get("display_title"))], "field", false).await?);
                // `get_the_date()`, `get_the_time()` and `get_delete_post_link()`.
                let ts = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
                let created = rustpress_core::i18n::sprintf(&tr(site, "Created on %1$s at %2$s"), &[&site.date(&site.options.string("date_format", "F j, Y"), ts), &site.date(&site.options.string("time_format", "g:i a"), ts)]);
                let delete = format!("{}&amp;_wpnonce={}", site.site_url(&format!("wp-admin/post.php?post={}&amp;action=trash", post.id)), (ctx.nonce)(&format!("trash-post_{}", post.id)));
                out.push_str(&format!(
                    "\t\t\t<div class=\"acf-field-group-settings-footer\">\n\t\t\t\t<span class=\"acf-created-on\">{}</span>\n\t\t\t\t<a href=\"{delete}\" class=\"acf-btn acf-btn-tertiary  acf-delete-field-group\">\n\t\t\t\t\t<i class=\"acf-icon acf-icon-trash\"></i>\n\t\t\t\t\t{}\t\t\t\t</a>\n\t\t\t</div>\n\t\t\t</div>",
                    esc_html(&created),
                    esc_tr(site, "Delete Field Group")
                ));
            }
        }
    }
    out.push_str(&format!(
        "\n<div class=\"acf-hidden\">\n\t<input type=\"hidden\" name=\"acf_field_group[key]\" value=\"{}\" />\n</div>\n<script type=\"text/javascript\">\nif( typeof acf !== 'undefined' ) {{\n\n\tacf.newPostbox({{\n\t\t'id': 'acf-field-group-options',\n\t\t'label': 'top'\n\t}});\n\n}}\n</script>\n",
        esc_attr(&strval(Some(&get("key"))))
    ));
    Ok(out)
}

/// `ACF_Admin_Field_Group::admin_head()`: the boxes "Fields" and "Settings", the form data after the title; the
/// box "Publish" is gone (`ACF_Form_Post::initialize()`).
pub async fn meta_boxes(groups: &[FieldGroup], ctx: &AdminContext<'_>, post: &Post) -> Result<PostMetaBoxes> {
    if edited_group(ctx).is_none() {
        return Ok(PostMetaBoxes::default());
    }
    let site = ctx.site;
    let services = super::edit_comment::AdminServices(ctx.wp_root.to_owned());
    let render = super::edit_post::render_ctx(ctx, &services);
    let labels = field_labels(groups);
    let field_label = move |k: &str| labels.iter().find(|(key, _)| key == k).map(|(_, l)| l.clone());
    let theme_json = ctx.wp_root.join("wp-content/themes").join(&site.settings.stylesheet).join("theme.json").exists() || ctx.wp_root.join("wp-content/themes").join(&site.settings.template).join("theme.json").exists();
    let env = Env { site, render: &render, field_label: &field_label, now: ctx.now, can_upload: (ctx.can)("upload_files"), theme_json };
    let view = View { render: &render, env: &env, num_groups: groups.len(), group_id: post.id };
    let fields: Vec<RenderField> = group_fields(groups, site, post).await?.iter().map(|f| editor_field(&render, f)).collect();
    let fields_html = fields_list(&view, fields, Value::Int(0), false).await?;
    let boxes = vec![
        PluginMetaBox { id: "acf-field-group-fields".into(), title: tr(site, "Fields"), context: "normal".into(), priority: "high".into(), html: fields_html },
        PluginMetaBox { id: "acf-field-group-options".into(), title: tr(site, "Settings"), context: "normal".into(), priority: "high".into(), html: options_box(ctx, &render, post).await? },
    ];
    let form_data = super::form::form_data_with("field_group", Value::Int(post.id as i64), &[("validation", Value::Int(0)), ("delete_fields", Value::Int(0))], &(ctx.nonce)("field_group"));
    Ok(PostMetaBoxes { boxes, removed: vec!["submitdiv".into()], after_title: vec![FormPart::Html(form_data)], ..Default::default() })
}

/// `acf.data` of the field group screen: the field types (`acf_localize_data()`), with `acf/input/admin_footer`
/// (the PRO features) before `acf.doAction( 'prepare' )`.
pub fn footer_scripts(ctx: &AdminContext<'_>) -> Option<String> {
    let post = edited_group(ctx)?;
    let site = ctx.site;
    let data = vec![
        ("fieldTypes".to_owned(), super::field_types::types_info(site)),
        ("fieldCategoriesL10n".to_owned(), serde_json::Value::Object(super::field_types::categories(site).into_iter().map(|(k, v)| (k.to_owned(), serde_json::Value::String(v))).collect())),
        ("PROUpgradeURL".to_owned(), serde_json::Value::String(super::admin_lists::utm("https://www.advancedcustomfields.com/pro/", "ACF upgrade", "field-type-selection", "", "", ""))),
        ("PROFieldTypes".to_owned(), super::field_types::pro_types(site)),
        ("PROLocationTypes".to_owned(), serde_json::json!({"block": esc_tr(site, "Block"), "options_page": esc_tr(site, "Options Page")})),
    ];
    let form = super::form::FormData { screen: serde_json::Value::String("field_group".into()), post_id: serde_json::json!(post.id), validation: serde_json::Value::Bool(false), field_group_data: data, ..Default::default() };
    let input_footer = super::admin_lists::pro_features(site);
    Some(super::form::footer_with(site, &form, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, super::form::Uploader::No, &input_footer))
}

/// A value of `$_REQUEST` (the POST body before the query string).
fn request<'a>(ctx: &'a AdminContext<'_>, name: &str) -> Option<&'a str> {
    ctx.post_field(name).or_else(|| ctx.query.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str()))
}

/// `acf_verify_ajax()` and `acf_current_user_can_admin()`: the nonce `acf_nonce` of the request and `manage_options`.
fn ajax_allowed(ctx: &AdminContext<'_>) -> bool {
    let nonce = request(ctx, "nonce").unwrap_or_default();
    !nonce.is_empty() && (ctx.verify_nonce)(nonce, "acf_nonce") && (ctx.can)("manage_options")
}

/// The slashes `wp_magic_quotes()` adds to every string of `$_POST` (`addslashes()`).
fn slashed(value: Value) -> Value {
    match value {
        Value::Str(s) => {
            let mut out = Vec::with_capacity(s.len());
            for b in s {
                match b {
                    b'\'' | b'"' | b'\\' => out.extend([b'\\', b]),
                    0 => out.extend(*b"\\0"),
                    _ => out.push(b),
                }
            }
            Value::Str(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(|(k, v)| (k, slashed(v))).collect()),
        other => other,
    }
}

/// `acf_punctify()`.
fn punctify(text: &str) -> String {
    let trimmed = text.trim();
    if rustpress_core::formatting::wp_strip_all_tags(trimmed).trim().ends_with('.') { trimmed.to_owned() } else { format!("{trimmed}.") }
}

/// `wp_ajax_acf/field_group/render_field_settings` (`ACF_Admin_Field_Group::ajax_render_field_settings()`): the
/// settings of every tab after the type of a field changed, for the field as the editor posts it (with the slashes
/// of `wp_magic_quotes()`, which ACF keeps).
pub async fn ajax_render_field_settings(groups: &[FieldGroup], ctx: &AdminContext<'_>) -> Result<AjaxAnswer> {
    let failed = || AjaxAnswer::json(200, "{\"success\":false}".into());
    if !ajax_allowed(ctx) {
        return Ok(failed());
    }
    let mut items = match rustpress_core::native::form_value(ctx.form, "field").map(slashed) {
        Some(Value::Array(items)) if !items.is_empty() => items,
        _ => return Ok(failed()),
    };
    let prefix = ctx.post_field("prefix").map_or(Value::Null, |p| slashed(string(p)));
    super::php::set(&mut items, "prefix", prefix);
    // `acf_get_valid_field()`.
    let field = if truthy(items.iter().find(|(k, _)| super::php::key_eq(k, &key("_valid"))).map(|(_, v)| v)) { RenderField { items, sub_fields: Vec::new() } } else { RenderField::raw(items, Vec::new()) };
    let site = ctx.site;
    let services = super::edit_comment::AdminServices(ctx.wp_root.to_owned());
    let render = super::edit_post::render_ctx(ctx, &services);
    let labels = field_labels(groups);
    let field_label = move |k: &str| labels.iter().find(|(key, _)| key == k).map(|(_, l)| l.clone());
    let theme_json = ctx.wp_root.join("wp-content/themes").join(&site.settings.stylesheet).join("theme.json").exists() || ctx.wp_root.join("wp-content/themes").join(&site.settings.template).join("theme.json").exists();
    let env = Env { site, render: &render, field_label: &field_label, now: ctx.now, can_upload: (ctx.can)("upload_files"), theme_json };
    let group_id = request(ctx, "post_id").map_or(0, |id| super::php::intval(Some(&string(id))).max(0) as u64);
    let view = View { render: &render, env: &env, num_groups: groups.len(), group_id };
    let mut sections = serde_json::Map::new();
    for tab in Tab::ALL {
        // Every action gets the field as it was.
        let mut field = RenderField { items: field.items.clone(), sub_fields: Vec::new() };
        sections.insert(tab.key().to_owned(), serde_json::Value::String(type_settings(&view, &mut field, tab).await?));
    }
    let body = rustpress_blocks::php_json::encode_with(&serde_json::json!({"success": true, "data": sections}), rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: false });
    Ok(AjaxAnswer::json(200, body))
}

/// `wp_ajax_acf/field_group/render_location_rule` (`ACF_Admin_Field_Group::ajax_render_location_rule()`): the row of
/// a location rule whose type changed (`views/acf-field-group/location-rule.php`). Not rebuilt: the conversion of
/// numeric term IDs in `post_taxonomy`/`post_category` rules of ACF 4 (`validate_post_taxonomy_location_rule()`).
pub async fn ajax_render_location_rule(ctx: &AdminContext<'_>) -> Result<AjaxAnswer> {
    let empty = || AjaxAnswer::text(200, "");
    if !ajax_allowed(ctx) {
        return Ok(empty());
    }
    let items = match rustpress_core::native::form_value(ctx.form, "rule").map(slashed) {
        Some(Value::Array(items)) if !items.is_empty() => items,
        _ => return Ok(empty()),
    };
    // `acf_validate_location_rule()`.
    let get = |name: &str, default: &str| items.iter().find(|(k, _)| super::php::key_eq(k, &key(name))).map_or_else(|| string(default), |(_, v)| v.clone());
    let rule = super::fields::Rule { param: strval(Some(&get("param", ""))), operator: strval(Some(&get("operator", "=="))), value: get("value", "") };
    let services = super::edit_comment::AdminServices(ctx.wp_root.to_owned());
    let render = super::edit_post::render_ctx(ctx, &services);
    let html = super::locations::rule_row(&render, ctx.wp_root, &rule, &strval(Some(&get("group", ""))), &strval(Some(&get("id", "")))).await?;
    Ok(AjaxAnswer::text(200, &html))
}

/// `wp_ajax_acf/field_group/move_field` (`ACF_Admin_Field_Group::ajax_move_field()`): the dialog with the other field
/// groups, or the move to the chosen one and its confirmation.
pub async fn ajax_move_field(ctx: &AdminContext<'_>) -> Result<AjaxAnswer> {
    let empty = || AjaxAnswer::text(200, "");
    let site = ctx.site;
    // `acf_parse_args( $_POST, … )`: only the POST body counts.
    let nonce = ctx.post_field("nonce").unwrap_or_default();
    if nonce.is_empty() || !(ctx.verify_nonce)(nonce, "acf_nonce") || !(ctx.can)("manage_options") {
        return Ok(empty());
    }
    let arg = |name: &str| ctx.post_field(name).map(string).unwrap_or(Value::Int(0));
    let id = |name: &str| super::php::intval(Some(&arg(name))).max(0) as u64;
    let groups = site.db.acf_posts("acf-field-group").await?;
    if truthy(Some(&arg("field_id"))) && truthy(Some(&arg("field_group_id"))) {
        let target = groups.iter().find(|g| g.id == id("field_group_id"));
        let Some(label) = super::save_group::move_field(ctx, id("field_id"), target.map(|g| g.id)).await? else { return Ok(empty()) };
        let link = format!("<a href=\"{}\" target=\"_blank\">{}</a>", site.site_url(&format!("wp-admin/post.php?post={}&action=edit", target.map_or(0, |g| g.id))), esc_html(&target.map(|g| g.title.clone()).unwrap_or_default()));
        let text = rustpress_core::i18n::sprintf(&punctify(&tr(site, "The %1$s field can now be found in the %2$s field group")), &[&esc_html(&label), &link]);
        return Ok(AjaxAnswer::text(200, &format!("<p><strong>{}</strong></p><p>{text}</p><a href=\"#\" class=\"button button-primary acf-close-popup\">{}</a>", esc_tr(site, "Move Complete."), esc_tr(site, "Close Modal"))));
    }
    let current = id("post_id");
    let choices: Vec<(Key, Value)> = groups.iter().filter(|g| g.id != 0 && g.id != current).map(|g| (Key::Int(g.id as i64), string(&g.title))).collect();
    let services = super::edit_comment::AdminServices(ctx.wp_root.to_owned());
    let render = super::edit_post::render_ctx(ctx, &services);
    let field = RenderField::raw(vec![(key("type"), string("select")), (key("name"), string("acf_field_group")), (key("choices"), Value::Array(choices)), (key("aria-label"), string(&tr(site, "Please select the destination for this field")))], Vec::new());
    Ok(AjaxAnswer::text(
        200,
        &format!(
            "<p>{}</p><form id=\"acf-move-field-form\">{}<button type=\"submit\" class=\"acf-btn\">{}</button></form>",
            esc_tr(site, "Please select the destination for this field"),
            render_field_wrap(&render, field, "div", "label").await?,
            esc_tr(site, "Move Field")
        ),
    ))
}
