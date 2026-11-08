//! `WP_Customize_Manager` with the base classes of its settings, panels, sections and controls: what they are, how
//! `prepare_controls()` sorts them and what `customize_pane_settings()` exports of them.

use serde_json::{Map, Value, json};

use super::super::block_editor::inline_json;

/// A `WP_Customize_Setting`, with what `json()` exports.
#[derive(Debug, Clone)]
pub(super) struct Setting {
    pub id: String,
    /// `$type`: `theme_mod`, `option` or a type of a subclass.
    pub kind: String,
    pub transport: &'static str,
    /// `check_capabilities()`: the capability and the theme supports.
    pub allowed: bool,
    /// `js_value()`.
    pub value: Value,
    /// `value()`, what the markup of a control prints.
    pub raw: Value,
    pub dirty: bool,
}

impl Setting {
    pub fn new(id: &str, kind: &str, transport: &'static str, value: Value) -> Self {
        Self { id: id.to_owned(), kind: kind.to_owned(), transport, allowed: true, raw: value.clone(), value, dirty: false }
    }

    fn json(&self) -> Value {
        json!({"value": self.value, "transport": self.transport, "dirty": self.dirty, "type": self.kind})
    }
}

/// A `WP_Customize_Panel`.
#[derive(Debug, Clone)]
pub(super) struct Panel {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: i64,
    pub kind: &'static str,
    pub allowed: bool,
    pub active: bool,
    pub instance: u32,
    pub auto_expand_sole_section: bool,
}

/// A `WP_Customize_Section`; `extra` is what a subclass adds to `json()`.
#[derive(Debug, Clone)]
pub(super) struct Section {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: i64,
    pub panel: String,
    pub kind: &'static str,
    pub description_hidden: bool,
    pub allowed: bool,
    pub active: bool,
    pub instance: u32,
    pub extra: Vec<(&'static str, Value)>,
}

/// A `WP_Customize_Control`: `extra` is what a subclass adds in `to_json()` (after the base keys), `json_extra` what it
/// adds in `json()` (after `to_json()`).
#[derive(Debug, Clone)]
pub(super) struct Control {
    pub id: String,
    pub kind: String,
    pub priority: i64,
    pub instance: u32,
    pub section: String,
    /// The key of each setting and its id.
    pub settings: Vec<(String, String)>,
    pub capability: Option<&'static str>,
    pub active: bool,
    /// What `render_content()` printed.
    pub content: String,
    pub label: String,
    pub description: String,
    pub allow_addition: bool,
    pub extra: Vec<(&'static str, Value)>,
    pub json_extra: Vec<(&'static str, Value)>,
}

impl Control {
    pub fn new(id: &str, kind: &str, section: &str, instance: u32) -> Self {
        Self {
            id: id.to_owned(),
            kind: kind.to_owned(),
            priority: 10,
            instance,
            section: section.to_owned(),
            settings: vec![("default".to_owned(), id.to_owned())],
            capability: None,
            active: true,
            content: String::new(),
            label: String::new(),
            description: String::new(),
            allow_addition: false,
            extra: Vec::new(),
            json_extra: Vec::new(),
        }
    }

    /// `render()`: the item around `render_content()`.
    pub fn wrapped_content(&self) -> String {
        let id = format!("customize-control-{}", self.id.replace('[', "-").replace(']', ""));
        let class = format!("customize-control customize-control-{}", self.kind);
        // `get_content()` trims what was printed.
        format!("<li id=\"{}\" class=\"{}\">{}</li>", rustpress_core::formatting::esc_attr(&id), rustpress_core::formatting::esc_attr(&class), self.content).trim().to_owned()
    }

    fn json(&self) -> Value {
        let mut settings = Map::new();
        for (key, id) in &self.settings {
            settings.insert(key.clone(), json!(id));
        }
        let mut out = Map::new();
        out.insert("settings".into(), Value::Object(settings));
        out.insert("type".into(), json!(self.kind));
        out.insert("priority".into(), json!(self.priority));
        out.insert("active".into(), json!(self.active));
        out.insert("section".into(), json!(self.section));
        out.insert("content".into(), json!(self.wrapped_content()));
        out.insert("label".into(), json!(self.label));
        out.insert("description".into(), json!(self.description));
        out.insert("instanceNumber".into(), json!(self.instance));
        if self.kind == "dropdown-pages" {
            out.insert("allow_addition".into(), json!(self.allow_addition));
        }
        for (key, value) in self.extra.iter().chain(&self.json_extra) {
            out.insert((*key).to_owned(), value.clone());
        }
        Value::Object(out)
    }
}

/// What the Customizer registered, with the counters of `$instance_count`.
#[derive(Debug, Default)]
pub(super) struct Manager {
    pub settings: Vec<Setting>,
    pub panels: Vec<Panel>,
    pub sections: Vec<Section>,
    pub controls: Vec<Control>,
    panel_count: u32,
    section_count: u32,
    control_count: u32,
}

impl Manager {
    pub fn next_panel(&mut self) -> u32 {
        self.panel_count += 1;
        self.panel_count
    }

    pub fn next_section(&mut self) -> u32 {
        self.section_count += 1;
        self.section_count
    }

    pub fn next_control(&mut self) -> u32 {
        self.control_count += 1;
        self.control_count
    }

    /// How many controls were made.
    pub fn control_count(&self) -> u32 {
        self.control_count
    }

    /// `add_setting()`: a setting that exists keeps its place and is replaced.
    pub fn add_setting(&mut self, setting: Setting) {
        match self.settings.iter_mut().find(|s| s.id == setting.id) {
            Some(slot) => *slot = setting,
            None => self.settings.push(setting),
        }
    }

    pub fn setting_mut(&mut self, id: &str) -> Option<&mut Setting> {
        self.settings.iter_mut().find(|s| s.id == id)
    }

    pub fn add_panel(&mut self, panel: Panel) {
        self.panels.retain(|p| p.id != panel.id);
        self.panels.push(panel);
    }

    pub fn add_section(&mut self, section: Section) {
        self.sections.retain(|s| s.id != section.id);
        self.sections.push(section);
    }

    pub fn add_control(&mut self, control: Control) {
        self.controls.retain(|c| c.id != control.id);
        self.controls.push(control);
    }

    pub fn panel(&self, id: &str) -> Option<&Panel> {
        self.panels.iter().find(|p| p.id == id)
    }

    fn section_allowed(&self, id: &str) -> Option<bool> {
        self.sections.iter().find(|s| s.id == id).map(|s| s.allowed)
    }

    /// `WP_Customize_Control::check_capabilities()`.
    pub fn control_allowed(&self, control: &Control, can: &dyn Fn(&str) -> bool) -> bool {
        if control.capability.is_some_and(|cap| !can(cap)) {
            return false;
        }
        if !control.settings.iter().all(|(_, id)| self.settings.iter().any(|s| s.id == *id && s.allowed)) {
            return false;
        }
        self.section_allowed(&control.section) != Some(false)
    }

    /// `prepare_controls()`: the controls of a section that exists and that may be used, the sections and panels that
    /// may be used, each sorted by priority and instance.
    pub fn prepare(&mut self, can: &dyn Fn(&str) -> bool) {
        let mut controls = std::mem::take(&mut self.controls);
        controls.sort_by_key(|c| (c.priority, c.instance));
        controls.retain(|c| self.sections.iter().any(|s| s.id == c.section) && self.control_allowed(c, can));
        self.controls = controls;
        self.sections.sort_by_key(|s| (s.priority, s.instance));
        self.sections.retain(|s| s.allowed);
        self.panels.sort_by_key(|p| (p.priority, p.instance));
        self.panels.retain(|p| p.allowed);
    }

    /// The JSON of a section (`WP_Customize_Section::json()`).
    fn section_json(&self, section: &Section, tr: &dyn Fn(&str) -> String) -> Value {
        let mut out = Map::new();
        out.insert("id".into(), json!(section.id));
        out.insert("description".into(), json!(section.description));
        out.insert("priority".into(), json!(section.priority));
        out.insert("panel".into(), json!(section.panel));
        out.insert("type".into(), json!(section.kind));
        out.insert("description_hidden".into(), json!(section.description_hidden));
        out.insert("title".into(), json!(rustpress_core::formatting::html_entity_decode(&section.title)));
        out.insert("content".into(), json!(""));
        out.insert("active".into(), json!(section.active));
        out.insert("instanceNumber".into(), json!(section.instance));
        let action = match self.panel(&section.panel).filter(|_| !section.panel.is_empty()) {
            Some(panel) => rustpress_core::i18n::sprintf(&tr("Customizing &#9656; %s"), &[&rustpress_core::formatting::esc_html(&panel.title)]),
            None => tr("Customizing"),
        };
        out.insert("customizeAction".into(), json!(action));
        for (key, value) in &section.extra {
            out.insert((*key).to_owned(), value.clone());
        }
        Value::Object(out)
    }

    /// `WP_Customize_Panel::json()`.
    fn panel_json(panel: &Panel) -> Value {
        json!({
            "id": panel.id,
            "description": panel.description,
            "priority": panel.priority,
            "type": panel.kind,
            "title": rustpress_core::formatting::html_entity_decode(&panel.title),
            "content": "",
            "active": panel.active,
            "instanceNumber": panel.instance,
            "autoExpandSoleSection": panel.auto_expand_sole_section,
        })
    }

    /// The sections and panels of `_wpCustomizeSettings`: the top-level sections, then each panel with its sections.
    pub fn containers(&self, tr: &dyn Fn(&str) -> String) -> (Value, Value) {
        let mut sections = Map::new();
        for section in self.sections.iter().filter(|s| s.panel.is_empty()) {
            sections.insert(section.id.clone(), self.section_json(section, tr));
        }
        let mut panels = Map::new();
        for panel in &self.panels {
            panels.insert(panel.id.clone(), Self::panel_json(panel));
            for section in self.sections.iter().filter(|s| s.panel == panel.id) {
                sections.insert(section.id.clone(), self.section_json(section, tr));
            }
        }
        (Value::Object(panels), Value::Object(sections))
    }

    /// The settings and controls of `customize_pane_settings()`, one statement each.
    pub fn statements(&self) -> String {
        let mut out = String::from("(function ( s ){\n");
        for setting in self.settings.iter().filter(|s| s.allowed) {
            out.push_str(&format!("s[{}] = {};\n", inline_json(&json!(setting.id)), inline_json(&setting.json())));
        }
        out.push_str("})( _wpCustomizeSettings.settings );\n");
        out.push_str("(function ( c ){\n");
        for control in &self.controls {
            out.push_str(&format!("c[{}] = {};\n", inline_json(&json!(control.id)), inline_json(&control.json())));
        }
        out.push_str("})( _wpCustomizeSettings.controls );\n");
        out
    }
}
