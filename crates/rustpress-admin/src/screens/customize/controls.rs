//! `render_content()` of the controls whose markup the server prints: `WP_Customize_Control` and the subclasses that
//! override it. PHP drops the line break after each `?>`, which is where the tabs come from.

use rustpress_core::formatting::{esc_attr, esc_html};

/// What `render_content()` of `WP_Customize_Control` reads.
pub(super) struct Field<'a> {
    pub id: &'a str,
    pub kind: &'a str,
    pub label: &'a str,
    pub description: &'a str,
    /// `get_link()`.
    pub link: String,
    /// `$this->value()` as PHP prints it.
    pub value: String,
    /// `$this->value()` is `true` (only a checkbox asks).
    pub value_true: bool,
    pub choices: &'a [(String, String)],
    pub input_attrs: &'a [(String, String)],
}

/// `get_link()` of a setting.
pub(super) fn link(setting_id: &str) -> String {
    format!("data-customize-setting-link=\"{}\"", esc_attr(setting_id))
}

/// `checked()` / `selected()`: the attribute when both are equal as strings.
fn mark(name: &str, helper: &str, current: &str) -> String {
    if helper == current { format!(" {name}='{name}'") } else { String::new() }
}

impl Field<'_> {
    fn input_id(&self) -> String {
        format!("_customize-input-{}", self.id)
    }

    fn description_id(&self) -> String {
        format!("_customize-description-{}", self.id)
    }

    fn describedby(&self) -> String {
        if self.description.is_empty() { String::new() } else { format!(" aria-describedby=\"{}\" ", esc_attr(&self.description_id())) }
    }

    /// The label as a title and the description, each in its `<?php if ... : ?>`.
    fn title(&self, tag: &str) -> String {
        let mut out = String::from("\t\t\t\t");
        if !self.label.is_empty() {
            match tag {
                "span" => out.push_str(&format!("\t\t\t\t\t<span class=\"customize-control-title\">{}</span>\n\t\t\t\t", esc_html(self.label))),
                _ => out.push_str(&format!("\t\t\t\t\t<label for=\"{}\" class=\"customize-control-title\">{}</label>\n\t\t\t\t", esc_attr(&self.input_id()), esc_html(self.label))),
            }
        }
        out.push_str(&self.description_block("\t\t\t\t"));
        out
    }

    fn description_block(&self, indent: &str) -> String {
        let mut out = indent.to_owned();
        if !self.description.is_empty() {
            out.push_str(&format!("{indent}\t<span id=\"{}\" class=\"description customize-control-description\">{}</span>\n{indent}", esc_attr(&self.description_id()), self.description));
        }
        out
    }

    /// `WP_Customize_Control::render_content()` for every type but `dropdown-pages`, which [`dropdown_pages`] renders.
    pub fn render(&self) -> String {
        let input_id = esc_attr(&self.input_id());
        match self.kind {
            "checkbox" => {
                let checked = if self.value_true { mark("checked", "1", "1") } else { mark("checked", &self.value, "1") };
                let mut out = format!(
                    "\t\t\t\t<span class=\"customize-inside-control-row\">\n\t\t\t\t\t<input\n\t\t\t\t\t\tid=\"{input_id}\"\n\t\t\t\t\t\t{}\t\t\t\t\t\ttype=\"checkbox\"\n\t\t\t\t\t\tvalue=\"{}\"\n\t\t\t\t\t\t{}\t\t\t\t\t\t{checked}\t\t\t\t\t/>\n\t\t\t\t\t<label for=\"{input_id}\">{}</label>\n",
                    self.describedby(),
                    esc_attr(&self.value),
                    self.link,
                    esc_html(self.label)
                );
                out.push_str(&self.description_block("\t\t\t\t\t"));
                out.push_str("\t\t\t\t</span>\n\t\t\t\t");
                out
            }
            "radio" => {
                if self.choices.is_empty() {
                    return String::new();
                }
                let mut out = self.title("span");
                out.push_str("\n\t\t\t\t");
                let name = esc_attr(&format!("_customize-radio-{}", self.id));
                for (value, label) in self.choices {
                    let id = esc_attr(&format!("{}-radio-{value}", self.input_id()));
                    out.push_str(&format!(
                        "\t\t\t\t\t<span class=\"customize-inside-control-row\">\n\t\t\t\t\t\t<input\n\t\t\t\t\t\t\tid=\"{id}\"\n\t\t\t\t\t\t\ttype=\"radio\"\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t\tvalue=\"{}\"\n\t\t\t\t\t\t\tname=\"{name}\"\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t/>\n\t\t\t\t\t\t<label for=\"{id}\">{}</label>\n\t\t\t\t\t</span>\n\t\t\t\t",
                        self.describedby(),
                        esc_attr(value),
                        self.link,
                        mark("checked", &self.value, value),
                        esc_html(label)
                    ));
                }
                out.push_str("\t\t\t\t");
                out
            }
            "select" => {
                if self.choices.is_empty() {
                    return String::new();
                }
                let mut out = self.title("label");
                out.push_str(&format!("\n\t\t\t\t<select id=\"{input_id}\" {} {}>\n\t\t\t\t\t", self.describedby(), self.link));
                for (value, label) in self.choices {
                    out.push_str(&format!("<option value=\"{}\"{}>{}</option>", esc_attr(value), mark("selected", &self.value, value), esc_html(label)));
                }
                out.push_str("\t\t\t\t</select>\n\t\t\t\t");
                out
            }
            "textarea" => {
                let mut attrs = self.input_attrs.to_vec();
                if !attrs.iter().any(|(name, _)| name == "rows") {
                    attrs.push(("rows".to_owned(), "5".to_owned()));
                }
                let mut out = self.title("label");
                out.push_str(&format!(
                    "\t\t\t\t<textarea\n\t\t\t\t\tid=\"{input_id}\"\n\t\t\t\t\t{}\t\t\t\t\t{}\t\t\t\t\t{}\t\t\t\t>{}</textarea>\n\t\t\t\t",
                    self.describedby(),
                    attributes(&attrs),
                    self.link,
                    crate::sanitize::esc_textarea(&self.value)
                ));
                out
            }
            _ => {
                let mut out = self.title("label");
                let value = if self.input_attrs.iter().any(|(name, _)| name == "value") { String::new() } else { format!("\t\t\t\t\t\tvalue=\"{}\"\n\t\t\t\t\t", esc_attr(&self.value)) };
                out.push_str(&format!(
                    "\t\t\t\t<input\n\t\t\t\t\tid=\"{input_id}\"\n\t\t\t\t\ttype=\"{}\"\n\t\t\t\t\t{}\t\t\t\t\t{}\t\t\t\t\t{value}\t\t\t\t\t{}\t\t\t\t\t/>\n\t\t\t\t",
                    esc_attr(self.kind),
                    self.describedby(),
                    attributes(self.input_attrs),
                    self.link
                ));
                out
            }
        }
    }

    /// `render_content()` of a `dropdown-pages` control: `dropdown` is what `wp_dropdown_pages()` printed, `add_new_item`
    /// the label of the pages when the control offers to add one.
    pub fn dropdown_pages(&self, dropdown: &str, add_new_item: Option<(&str, &dyn Fn(&str) -> String)>) -> String {
        let mut out = self.title("label");
        out.push_str("\n\t\t\t\t");
        // "Hackily add in the data link parameter."
        out.push_str(&dropdown.replace("<select", &format!("<select {} id=\"{}\" {}", self.link, esc_attr(&self.input_id()), self.describedby())));
        out.push_str("\t\t\t\t");
        if let Some((label, tr)) = add_new_item {
            let id = esc_attr(self.id);
            out.push_str(&format!(
                "\t\t\t\t\t<button type=\"button\" class=\"button-link add-new-toggle\">\n\t\t\t\t\t\t{}\t\t\t\t\t</button>\n\t\t\t\t\t<div class=\"new-content-item-wrapper\">\n\t\t\t\t\t\t<label for=\"create-input-{id}\">{}</label>\n\t\t\t\t\t\t<div class=\"new-content-item\">\n\t\t\t\t\t\t\t<input type=\"text\" id=\"create-input-{id}\" class=\"create-item-input form-required\">\n\t\t\t\t\t\t\t<button type=\"button\" class=\"button add-content\">{}</button>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t\t<span id=\"create-input-{id}-error\" class=\"create-item-error error-message\" style=\"display: none;\">{}</span>\n\n\t\t\t\t\t</div>\n\t\t\t\t",
                rustpress_core::i18n::sprintf(&tr("+ %s"), &[label]),
                tr("New page title"),
                tr("Add"),
                tr("Please enter a page title")
            ));
        }
        out.push_str("\t\t\t\t");
        out
    }
}

/// `input_attrs()`.
fn attributes(attrs: &[(String, String)]) -> String {
    attrs.iter().map(|(name, value)| format!("{name}=\"{}\" ", esc_attr(value))).collect()
}

/// `WP_Customize_Header_Image_Control::render_content()`.
pub(super) fn header_image(tr: &dyn Fn(&str) -> String, trx: &dyn Fn(&str, &str) -> String, label: &str, video: bool, width: u64, height: u64, has_image: bool, can_upload: bool) -> String {
    let visibility = if has_image { "" } else { " style=\"display:none\" " };
    let title = if video { format!("<span class=\"customize-control-title\">{label}</span>") } else { String::new() };
    let intro = if video {
        tr("Click &#8220;Add Image&#8221; to upload an image file from your computer. Your theme works best with an image that matches the size of your video &#8212; you&#8217;ll be able to crop your image once you upload it for a perfect fit.")
    } else if width > 0 && height > 0 {
        rustpress_core::i18n::sprintf(&tr("Click &#8220;Add Image&#8221; to upload an image file from your computer. Your theme works best with an image with a header size of %s pixels &#8212; you&#8217;ll be able to crop your image once you upload it for a perfect fit."), &[&format!("<strong>{width} &times; {height}</strong>")])
    } else if width > 0 {
        rustpress_core::i18n::sprintf(&tr("Click &#8220;Add Image&#8221; to upload an image file from your computer. Your theme works best with an image with a header width of %s pixels &#8212; you&#8217;ll be able to crop your image once you upload it for a perfect fit."), &[&format!("<strong>{width}</strong>")])
    } else {
        rustpress_core::i18n::sprintf(&tr("Click &#8220;Add Image&#8221; to upload an image file from your computer. Your theme works best with an image with a header height of %s pixels &#8212; you&#8217;ll be able to crop your image once you upload it for a perfect fit."), &[&format!("<strong>{height}</strong>")])
    };
    let actions = if can_upload {
        format!(
            "\t\t\t\t<button type=\"button\"{visibility} class=\"button remove\" aria-label=\"{}\">{}</button>\n\t\t\t\t<button type=\"button\" class=\"button new {}\" id=\"header_image-button\" aria-label=\"{}\">{}</button>\n\t\t\t\t",
            esc_attr(&tr("Hide header image")),
            tr("Hide image"),
            if has_image { "customize-header-image-not-selected" } else { "" },
            esc_attr(&tr("Add Header Image")),
            tr("Add Image")
        )
    } else {
        String::new()
    };
    format!(
        "\t\t<div class=\"customize-control-content\">\n\t\t\t{title}\t\t\t<div class=\"customize-control-notifications-container\"></div>\n\t\t\t<p class=\"customizer-section-intro customize-control-description\">\n\t\t\t\t{intro}\t\t\t</p>\n\t\t\t<div class=\"current\">\n\t\t\t\t<label for=\"header_image-button\">\n\t\t\t\t\t<span class=\"customize-control-title\">\n\t\t\t\t\t\t{}\t\t\t\t\t</span>\n\t\t\t\t</label>\n\t\t\t\t<div class=\"container\">\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t<div class=\"actions\">\n\t\t\t\t{actions}\t\t\t</div>\n\t\t\t<div class=\"choices\">\n\t\t\t\t<span class=\"customize-control-title header-previously-uploaded\">\n\t\t\t\t\t{}\t\t\t\t</span>\n\t\t\t\t<div class=\"uploaded\">\n\t\t\t\t\t<div class=\"list\">\n\t\t\t\t\t</div>\n\t\t\t\t</div>\n\t\t\t\t<span class=\"customize-control-title header-default\">\n\t\t\t\t\t{}\t\t\t\t</span>\n\t\t\t\t<div class=\"default\">\n\t\t\t\t\t<div class=\"list\">\n\t\t\t\t\t</div>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t</div>\n\t\t",
        tr("Current header"),
        trx("Previously uploaded", "custom headers"),
        trx("Suggested", "custom headers")
    )
}
