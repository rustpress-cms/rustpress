//! `core/tabs`, `core/tab-list` and `core/tab-panel`.

use rustpress_core::formatting::{esc_attr, strip_tags};
use serde_json::{Value, json};

use crate::Result;
use crate::env::Context;
use crate::html::{Tag, TagEditor, tags};
use crate::parser::Block;
use crate::render::BlockCtx;

fn has_class(tag: &Tag, class: &str) -> bool {
    tag.attr("class").is_some_and(|c| c.split_ascii_whitespace().any(|x| x == class))
}

/// `block_core_tabs_provide_context()`: the `render_block_context` filter of `core/tabs`.
pub(crate) fn provide_context(env: &crate::env::Env<'_>, block: &Block, available: &Context) -> Context {
    let tabs_id = match block.attrs.get("anchor").filter(|a| !crate::style_engine::is_empty(a)) {
        Some(Value::String(anchor)) => anchor.clone(),
        Some(other) => other.to_string(),
        None => env.with_state(|s| s.unique_id("tabs_")),
    };
    let mut list: Vec<Value> = Vec::new();
    if let Some(panels) = block.inner_blocks.iter().find(|b| b.name.as_deref() == Some("core/tab-panels")) {
        for (index, panel) in panels.inner_blocks.iter().filter(|b| b.name.as_deref() == Some("core/tab-panel")).enumerate() {
            let id = match panel.attrs.get("anchor").and_then(Value::as_str).filter(|a| !a.is_empty()) {
                Some(anchor) => anchor.to_owned(),
                None if !tabs_id.is_empty() => format!("{tabs_id}-tab-{index}"),
                None => format!("tab-{index}"),
            };
            list.push(Value::String(esc_attr(&id)));
        }
    }
    available.with("core/tabs-list", Value::Array(list)).with("core/tabs-id", Value::String(tabs_id))
}

/// `block_core_tabs_render_block_callback()`.
pub(super) fn tabs(bc: &BlockCtx<'_>) -> Result<String> {
    let tabs_id = bc.context.get("core/tabs-id").and_then(Value::as_str).unwrap_or_default().to_owned();
    if tabs_id.is_empty() {
        return Ok(String::new());
    }
    let list = bc.context.get("core/tabs-list").cloned().unwrap_or(Value::Array(Vec::new()));
    let active = bc.attrs.get("activeTabIndex").cloned().unwrap_or(Value::from(0));
    let content = &bc.content;
    let context = crate::php_json::encode(&json!({ "tabsId": tabs_id, "activeTabIndex": active }));
    let output = match tags(content, 0).into_iter().find(|t| !t.is_closer && has_class(t, "wp-block-tabs")) {
        Some(tag) => {
            let mut editor = TagEditor::from_tag(content, tag);
            editor.set_attribute("data-wp-interactive", "core/tabs");
            editor.set_attribute("data-wp-context", &context);
            editor.set_attribute("data-wp-init", "callbacks.onTabsInit");
            editor.set_attribute("data-wp-on--keydown", "actions.handleTabKeyDown");
            editor.finish()
        }
        None => content.clone(),
    };
    bc.env.with_state(|s| {
        s.script_modules.insert("@wordpress/block-library/tabs/view".to_owned());
        let mut state = serde_json::Map::new();
        state.insert(tabs_id, list);
        s.interactivity.state("core/tabs", Value::Object(state));
    });
    Ok(output)
}

/// `block_core_tab_list_render_callback()`.
pub(super) fn tab_list(bc: &BlockCtx<'_>) -> Result<String> {
    let list: Vec<String> = bc.context.get("core/tabs-list").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default();
    let aria = match bc.attr_str("ariaLabel").filter(|l| !l.is_empty()) {
        Some(label) => strip_tags(label),
        None => bc.env.site.tr("Tabbed content"),
    };
    let mut content = bc.content.clone();
    let mut from = 0;
    if let Some(tag) = tags(&content, 0).into_iter().find(|t| !t.is_closer && has_class(t, "wp-block-tab-list")) {
        let mut editor = TagEditor::from_tag(&content, tag);
        editor.set_attribute("aria-label", &aria);
        let (start, _, text) = editor.tag_replacement();
        from = start + text.len();
        content = editor.finish();
    }
    if list.is_empty() {
        return Ok(content);
    }
    for (index, id) in list.iter().enumerate() {
        let Some(button) = tags(&content, from).into_iter().find(|t| !t.is_closer && t.name == "button") else { break };
        let mut editor = TagEditor::from_tag(&content, button);
        editor.set_attribute("id", &format!("tab__{id}"));
        editor.set_attribute("aria-controls", id);
        editor.set_attribute("data-wp-on--click", "actions.handleTabClick");
        editor.set_attribute("data-wp-on--keydown", "actions.handleTabKeyDown");
        editor.set_attribute("data-wp-bind--aria-selected", "state.isActiveTab");
        editor.set_attribute("data-wp-bind--tabindex", "state.tabIndexAttribute");
        editor.set_attribute("data-wp-context", &crate::php_json::encode(&json!({ "tabIndex": index })));
        let (start, _, text) = editor.tag_replacement();
        from = start + text.len();
        content = editor.finish();
    }
    Ok(content)
}

/// `block_core_tab_panel_render()`.
pub(super) fn tab_panel(bc: &BlockCtx<'_>) -> Result<String> {
    let tabs_id = bc.context.get("core/tabs-id").and_then(Value::as_str).unwrap_or_default().to_owned();
    let index = bc.env.with_state(|s| {
        let counter = s.tab_counters.entry(tabs_id.clone()).or_insert(0);
        let index = *counter;
        *counter += 1;
        index
    });
    let content = &bc.content;
    let Some(tag) = tags(content, 0).into_iter().find(|t| !t.is_closer && has_class(t, "wp-block-tab-panel")) else { return Ok(content.clone()) };
    let mut editor = TagEditor::from_tag(content, tag);
    let mut tab_id = editor.get_attribute("id").unwrap_or_default().to_owned();
    if tab_id.is_empty() || tab_id == "0" {
        tab_id = if tabs_id.is_empty() { format!("tab-{index}") } else { format!("{tabs_id}-tab-{index}") };
        editor.set_attribute("id", &tab_id);
    }
    editor.set_attribute("aria-labelledby", &format!("tab__{tab_id}"));
    editor.set_attribute("data-wp-bind--hidden", "!state.isActiveTab");
    Ok(editor.finish())
}
