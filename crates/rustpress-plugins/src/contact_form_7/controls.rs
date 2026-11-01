//! The form-tag handlers of the modules (`modules/*.php`): the HTML of each form control.

use std::sync::LazyLock;

use regex::Regex;
use rustpress_core::formatting::{esc_attr, esc_html};

use super::form_tag::{FormTag, truthy};
use super::formatter::{Att, Atts, format_atts};

/// The user whose `user_*` defaults the fields take (`wp_get_current_user()`).
#[derive(Debug, Clone, Default)]
pub struct UserProps {
    pub login: String,
    pub email: String,
    pub url: String,
    pub display_name: String,
    pub nicename: String,
    /// User meta (`first_name`, `last_name`, `nickname`, `description`, ...).
    pub meta: Vec<(String, String)>,
}

impl UserProps {
    /// `WP_User::get( $key )`.
    fn get(&self, key: &str) -> Option<String> {
        let value = match key {
            "user_login" => self.login.clone(),
            "user_email" => self.email.clone(),
            "user_url" => self.url.clone(),
            "display_name" => self.display_name.clone(),
            "user_nicename" => self.nicename.clone(),
            other => self.meta.iter().find(|(k, _)| k == other).map(|(_, v)| v.clone()).unwrap_or_default(),
        };
        (!value.is_empty() && value != "0").then_some(value)
    }
}

/// What the handlers see of the request and of the form being rendered.
pub struct Render<'a> {
    /// `__( $text, 'contact-form-7' )` in the language of the form.
    pub tr: &'a (dyn Fn(&str) -> String + Send + Sync),
    /// `wp_hash( $answer, 'wpcf7_quiz' )`.
    pub quiz_hash: &'a (dyn Fn(&str) -> String + Send + Sync),
    /// `$_GET`.
    pub query: &'a [(String, String)],
    pub user: Option<&'a UserProps>,
    /// The meta of the post of the loop (`get_post_meta( get_the_ID() )`), when the form is in the loop.
    pub post_meta: Option<&'a [(String, String)]>,
    /// The attributes of the shortcode (`shortcode_attr()`).
    pub shortcode_atts: &'a [(String, String)],
    /// The form template, for `wpcf7_scan_form_tags()`.
    pub template: &'a str,
    /// `Y-m-d` of today in the timezone of the site, for relative dates.
    pub today: chrono::NaiveDate,
    /// `static $used` of `get_id_option()`: the ids given out during the request.
    pub used_ids: &'a mut Vec<String>,
    /// `random_pipe()`: an index below the given count.
    pub random: &'a mut (dyn FnMut(usize) -> usize + Send),
    /// `$this->responses_count`.
    pub responses: u32,
}

impl Render<'_> {
    /// `get_id_option()`.
    fn id_option(&mut self, tag: &FormTag) -> Att {
        match tag.get_option("id", "id") {
            Some(id) if truthy(&id) && !id.starts_with("wpcf7") && !self.used_ids.contains(&id) => {
                self.used_ids.push(id.clone());
                Att::Str(id)
            }
            _ => Att::Bool(false),
        }
    }

    /// `get_default_option( $default, array( 'multiple' => false ) )`.
    fn default_option(&self, tag: &FormTag, default: Option<String>) -> Option<String> {
        self.default_values(tag, false).into_iter().next().or(default)
    }

    /// The values `get_default_option()` collects; with `multiple` false only the first counts.
    fn default_values(&self, tag: &FormTag, multiple: bool) -> Vec<String> {
        static NUMS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9_]+$").unwrap());
        let mut values: Vec<String> = Vec::new();
        for opt in tag.get_options("default", "") {
            let opt = sanitize_key(&opt);
            let mut found: Vec<String> = Vec::new();
            if opt.starts_with("user_") && self.user.is_some() {
                let key = if ["user_login", "user_email", "user_url"].contains(&opt.as_str()) { opt.as_str() } else { &opt[5..] };
                found.extend(self.user.and_then(|u| u.get(key)));
            } else if opt == "post_meta" && self.post_meta.is_some() {
                let rows = self.post_meta.unwrap_or_default();
                let mut matching = rows.iter().filter(|(k, _)| *k == tag.name).map(|(_, v)| v.clone());
                if multiple {
                    found.extend(matching);
                } else if let Some(v) = matching.next().filter(|v| !v.is_empty()) {
                    found.push(v);
                }
            } else if opt == "get" {
                let vals: Vec<String> = query_values(self.query, &tag.name).iter().map(|v| sanitize_query_var(v)).collect();
                if multiple {
                    found.extend(vals);
                } else if let Some(v) = vals.into_iter().next().filter(|v| !v.is_empty()) {
                    found.push(v);
                }
            } else if opt == "shortcode_attr" {
                found.extend(self.shortcode_atts.iter().find(|(k, _)| *k == tag.name).map(|(_, v)| v.clone()).filter(|v| !v.is_empty()));
            } else if NUMS.is_match(&opt) {
                for num in opt.split('_') {
                    let num: i64 = num.parse().unwrap_or(0);
                    if let Some(v) = usize::try_from(num - 1).ok().and_then(|i| tag.values.get(i)) {
                        found.push(v.clone());
                    }
                }
            }
            if !multiple {
                if let Some(first) = found.into_iter().next() {
                    return vec![first];
                }
                continue;
            }
            for v in found {
                if !values.contains(&v) {
                    values.push(v);
                }
            }
        }
        values
    }

    /// `get_date_option( $name )`.
    fn date_option(&self, tag: &FormTag, name: &str) -> Att {
        match tag.get_option(name, "").filter(|v| truthy(v)) {
            Some(value) => Att::opt(parse_date(&value.replace('_', " "), self.today)),
            None => Att::Bool(false),
        }
    }
}

/// `$_GET[ $name ]` as PHP builds it: one value, or the values of `name[]`.
fn query_values(query: &[(String, String)], name: &str) -> Vec<String> {
    if let Some((_, v)) = query.iter().rev().find(|(k, _)| k == name) {
        return vec![v.clone()];
    }
    let list: Vec<String> = query.iter().filter(|(k, _)| k.strip_prefix(name).is_some_and(|rest| rest.starts_with('['))).map(|(_, v)| v.clone()).collect();
    list
}

/// `sanitize_key()`.
fn sanitize_key(key: &str) -> String {
    key.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '-').collect()
}

/// `wpcf7_sanitize_query_var()`.
pub fn sanitize_query_var(text: &str) -> String {
    static PERCENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)%[a-f0-9]{2}").unwrap());
    static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" +").unwrap());
    let mut text = text.to_owned();
    if text.contains('<') {
        text = rustpress_core::formatting::wp_strip_all_tags(&rustpress_core::kses::pre_kses_less_than(&text));
    }
    let text = PERCENT.replace_all(&text, "");
    let text = SPACES.replace_all(&text, " ");
    text.trim_matches(' ').to_owned()
}

/// `date_create_immutable( $text, wp_timezone() )->format( 'Y-m-d' )` for the dates and the relative words forms use
/// (`2026-01-01`, `today`, `tomorrow`, `yesterday`, `now`, `+3 days`, `-1 week`, `next month`).
pub fn parse_date(text: &str, today: chrono::NaiveDate) -> Option<String> {
    use chrono::{Datelike, Duration, Months, NaiveDate};
    static YMD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]{4})-([0-9]{1,2})-([0-9]{1,2})$").unwrap());
    static RELATIVE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(?:([+-]?[0-9]+)|(next|last))\s*(day|week|fortnight|month|year)s?$").unwrap());
    let text = text.trim();
    if let Some(c) = YMD.captures(text) {
        return NaiveDate::from_ymd_opt(c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?).map(|d| d.format("%Y-%m-%d").to_string());
    }
    let mut date = today;
    let lower = text.to_lowercase();
    match lower.as_str() {
        "today" | "now" | "midnight" => return Some(date.format("%Y-%m-%d").to_string()),
        "tomorrow" => return Some((date + Duration::days(1)).format("%Y-%m-%d").to_string()),
        "yesterday" => return Some((date - Duration::days(1)).format("%Y-%m-%d").to_string()),
        _ => {}
    }
    let c = RELATIVE.captures(&lower)?;
    let n: i64 = match (c.get(1), c.get(2).map(|m| m.as_str())) {
        (Some(n), _) => n.as_str().parse().ok()?,
        (None, Some("next")) => 1,
        _ => -1,
    };
    date = match &c[3] {
        "day" => date + Duration::days(n),
        "week" => date + Duration::days(7 * n),
        "fortnight" => date + Duration::days(14 * n),
        unit => {
            // PHP adds months by overflowing the day (Jan 31 + 1 month = Mar 3).
            let months = if unit == "year" { 12 * n } else { n };
            let first = date.with_day(1)?;
            let moved = if months >= 0 { first.checked_add_months(Months::new(months as u32))? } else { first.checked_sub_months(Months::new((-months) as u32))? };
            moved + Duration::days(i64::from(date.day()) - 1)
        }
    };
    Some(date.format("%Y-%m-%d").to_string())
}

/// `wpcf7_form_controls_class( $type, $default_classes )`.
pub fn form_controls_class(tag_type: &str, default_classes: &str) -> String {
    let tag_type = tag_type.trim();
    let mut classes: Vec<String> = vec!["wpcf7-form-control".to_owned(), format!("wpcf7-{}", tag_type.trim_end_matches('*'))];
    if tag_type.ends_with('*') {
        classes.push("wpcf7-validates-as-required".to_owned());
    }
    for class in default_classes.split(' ') {
        classes.push(class.to_owned());
    }
    let mut unique: Vec<String> = Vec::new();
    for class in classes {
        if !class.is_empty() && class != "0" && !unique.contains(&class) {
            unique.push(class);
        }
    }
    unique.join(" ")
}

/// `wpcf7_is_number()`.
fn is_number(text: &str) -> bool {
    static INT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[-]?[0-9]+(?:[eE][+-]?[0-9]+)?$").unwrap());
    static DEC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[-]?(?:[0-9]+)?[.][0-9]+(?:[eE][+-]?[0-9]+)?$").unwrap());
    INT.is_match(text) || DEC.is_match(text)
}

/// A numeric string as PHP's arithmetic reads it.
enum Num {
    Int(i64),
    Float(f64),
}

fn num(text: &str) -> Num {
    match text.parse::<i64>() {
        Ok(n) if !text.contains(['.', 'e', 'E']) => Num::Int(n),
        _ => Num::Float(text.parse().unwrap_or(0.0)),
    }
}

fn as_f64(n: &Num) -> f64 {
    match n {
        Num::Int(i) => *i as f64,
        Num::Float(f) => *f,
    }
}

fn wrap(name: &str, inner: &str) -> String {
    format!("<span class=\"wpcf7-form-control-wrap\" data-name=\"{}\">{inner}</span>", esc_attr(name))
}

/// The attributes common to most controls: class, id, tabindex.
fn common(r: &mut Render<'_>, tag: &FormTag, class: &str) -> Atts {
    let mut atts = Atts::default();
    atts.set("class", Att::opt(tag.get_class_option(class)));
    atts.set("id", r.id_option(tag));
    atts.set("tabindex", Att::opt(tag.get_option("tabindex", "signed_int")));
    atts
}

/// `$maxlength < $minlength` for two numeric strings.
fn length_conflict(atts: &Atts) -> bool {
    match (atts.get("maxlength"), atts.get("minlength")) {
        (Some(Att::Str(max)), Some(Att::Str(min))) if truthy(max) && truthy(min) => max.parse::<i64>().unwrap_or(0) < min.parse::<i64>().unwrap_or(0),
        _ => false,
    }
}

fn required_and_valid(atts: &mut Atts, tag: &FormTag) {
    if tag.is_required() {
        atts.set("aria-required", Att::str("true"));
    }
    atts.set("aria-invalid", Att::str("false"));
}

/// The value of a text-like field and its placeholder.
fn value_and_placeholder(r: &Render<'_>, tag: &FormTag, atts: &mut Atts, start: String) -> String {
    let mut value = start;
    if tag.has_option("placeholder") || tag.has_option("watermark") {
        atts.set("placeholder", Att::Str(value));
        value = String::new();
    }
    r.default_option(tag, Some(value)).unwrap_or_default()
}

/// `wpcf7_text_form_tag_handler()`.
fn text(r: &mut Render<'_>, tag: &FormTag) -> String {
    let mut class = form_controls_class(&tag.tag_type, "wpcf7-text");
    if ["email", "url", "tel"].contains(&tag.basetype.as_str()) {
        class.push_str(&format!(" wpcf7-validates-as-{}", tag.basetype));
    }
    let mut atts = Atts::default();
    atts.set("size", Att::Str(tag.get_size_option().unwrap_or_else(|| "40".to_owned())));
    atts.set("maxlength", Att::Str(tag.get_maxlength_option().unwrap_or_else(|| "400".to_owned())));
    atts.set("minlength", Att::opt(tag.get_minlength_option()));
    if length_conflict(&atts) {
        atts.unset("maxlength");
        atts.unset("minlength");
    }
    atts.set("class", Att::opt(tag.get_class_option(&class)));
    atts.set("id", r.id_option(tag));
    atts.set("list", Att::opt(tag.get_option("list", "id")));
    atts.set("tabindex", Att::opt(tag.get_option("tabindex", "signed_int")));
    atts.set("readonly", Att::Bool(tag.has_option("readonly")));
    atts.set("autocomplete", Att::opt(tag.get_autocomplete_option()));
    required_and_valid(&mut atts, tag);
    let value = value_and_placeholder(r, tag, &mut atts, tag.values.first().cloned().unwrap_or_default());
    atts.set("value", Att::Str(value));
    atts.set("type", Att::str(&tag.basetype));
    atts.set("name", Att::str(&tag.name));
    wrap(&tag.name, &format!("<input {} />", format_atts(&atts)))
}

/// `esc_textarea()`: `htmlspecialchars()` with double encoding.
fn esc_textarea(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#039;")
}

/// `wpcf7_textarea_form_tag_handler()`.
fn textarea(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "");
    let mut atts = Atts::default();
    atts.set("cols", Att::Str(tag.get_cols_option().unwrap_or_else(|| "40".to_owned())));
    atts.set("rows", Att::Str(tag.get_rows_option().unwrap_or_else(|| "10".to_owned())));
    atts.set("maxlength", Att::Str(tag.get_maxlength_option().unwrap_or_else(|| "2000".to_owned())));
    atts.set("minlength", Att::opt(tag.get_minlength_option()));
    if length_conflict(&atts) {
        atts.unset("maxlength");
        atts.unset("minlength");
    }
    atts.set("class", Att::opt(tag.get_class_option(&class)));
    atts.set("id", r.id_option(tag));
    atts.set("tabindex", Att::opt(tag.get_option("tabindex", "signed_int")));
    atts.set("readonly", Att::Bool(tag.has_option("readonly")));
    atts.set("autocomplete", Att::opt(tag.get_autocomplete_option()));
    required_and_valid(&mut atts, tag);
    let start = if tag.content.is_empty() || tag.content == "0" { tag.values.first().cloned().unwrap_or_default() } else { tag.content.clone() };
    let value = value_and_placeholder(r, tag, &mut atts, start);
    atts.set("name", Att::str(&tag.name));
    wrap(&tag.name, &format!("<textarea {}>{}</textarea>", format_atts(&atts), esc_textarea(&value)))
}

/// `wpcf7_number_form_tag_handler()`.
fn number(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "") + " wpcf7-validates-as-number";
    let mut atts = common(r, tag, &class);
    atts.set("min", Att::opt(tag.get_option("min", "signed_num")));
    atts.set("max", Att::opt(tag.get_option("max", "signed_num")));
    atts.set("step", Att::opt(tag.get_option("step", "num")));
    atts.set("readonly", Att::Bool(tag.has_option("readonly")));
    atts.set("autocomplete", Att::opt(tag.get_autocomplete_option()));
    required_and_valid(&mut atts, tag);
    let value = value_and_placeholder(r, tag, &mut atts, tag.values.first().cloned().unwrap_or_default());
    atts.set("value", Att::Str(value.clone()));
    if tag.basetype == "range" {
        let text_of = |a: Option<&Att>| match a {
            Some(Att::Str(s)) => s.clone(),
            _ => String::new(),
        };
        let mut min = text_of(atts.get("min"));
        let mut max = text_of(atts.get("max"));
        if !is_number(&min) {
            min = "0".to_owned();
            atts.set("min", Att::str("0"));
        }
        if !is_number(&max) {
            max = "100".to_owned();
            atts.set("max", Att::str("100"));
        }
        if value.is_empty() {
            let (lo, hi) = (num(&min), num(&max));
            if as_f64(&lo) < as_f64(&hi) {
                let mid = match (lo, hi) {
                    (Num::Int(a), Num::Int(b)) if (a + b) % 2 == 0 => Att::Int((a + b) / 2),
                    (a, b) => Att::Float((as_f64(&a) + as_f64(&b)) / 2.0),
                };
                atts.set("value", mid);
            } else {
                atts.set("value", Att::Str(min));
            }
        }
    }
    atts.set("type", Att::str(&tag.basetype));
    atts.set("name", Att::str(&tag.name));
    wrap(&tag.name, &format!("<input {} />", format_atts(&atts)))
}

/// `wpcf7_date_form_tag_handler()`.
fn date(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "") + " wpcf7-validates-as-date";
    let mut atts = common(r, tag, &class);
    atts.set("min", r.date_option(tag, "min"));
    atts.set("max", r.date_option(tag, "max"));
    atts.set("step", Att::opt(tag.get_option("step", "int")));
    atts.set("readonly", Att::Bool(tag.has_option("readonly")));
    atts.set("autocomplete", Att::opt(tag.get_autocomplete_option()));
    required_and_valid(&mut atts, tag);
    let mut value = value_and_placeholder(r, tag, &mut atts, tag.values.first().cloned().unwrap_or_default());
    if truthy(&value)
        && let Some(date) = parse_date(&value.replace('_', " "), r.today)
    {
        value = date;
    }
    atts.set("value", Att::Str(value));
    atts.set("type", Att::str(&tag.basetype));
    atts.set("name", Att::str(&tag.name));
    wrap(&tag.name, &format!("<input {} />", format_atts(&atts)))
}

/// `wpcf7_select_form_tag_handler()`.
fn select(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "");
    let mut atts = Atts::default();
    atts.set("class", Att::opt(tag.get_class_option(&class)));
    atts.set("id", r.id_option(tag));
    atts.set("tabindex", Att::opt(tag.get_option("tabindex", "signed_int")));
    atts.set("autocomplete", Att::opt(tag.get_autocomplete_option()));
    required_and_valid(&mut atts, tag);
    let multiple = tag.has_option("multiple");
    let include_blank = tag.has_option("include_blank");
    let first_as_label = tag.has_option("first_as_label");
    if tag.has_option("size") {
        match tag.get_option("size", "int").filter(|s| truthy(s)) {
            Some(size) => atts.set("size", Att::Str(size)),
            None => atts.set("size", Att::Int(if multiple { 4 } else { 1 })),
        }
    }
    let mut values = tag.values.clone();
    let mut labels = tag.labels.clone();
    let default_choice = r.default_values(tag, multiple);
    let default_choice = if multiple { default_choice } else { default_choice.into_iter().take(1).collect() };
    if include_blank || values.is_empty() {
        labels.insert(0, (r.tr)("&#8212;Please choose an option&#8212;"));
        values.insert(0, String::new());
    } else if first_as_label {
        values[0] = String::new();
    }
    let mut options = String::new();
    for (i, value) in values.iter().enumerate() {
        let mut item = Atts::default();
        item.set("value", Att::Str(value.clone()));
        item.set("selected", Att::Bool(default_choice.contains(value)));
        let label = labels.get(i).unwrap_or(value);
        options.push_str(&format!("<option {}>{}</option>", format_atts(&item), esc_html(label)));
    }
    atts.set("multiple", Att::Bool(multiple));
    atts.set("name", Att::Str(format!("{}{}", tag.name, if multiple { "[]" } else { "" })));
    wrap(&tag.name, &format!("<select {}>{options}</select>", format_atts(&atts)))
}

/// `wpcf7_checkbox_form_tag_handler()` (checkboxes and radio buttons).
fn checkbox(r: &mut Render<'_>, tag: &FormTag) -> String {
    let mut class = form_controls_class(&tag.tag_type, "");
    let label_first = tag.has_option("label_first");
    let use_label_element = tag.has_option("use_label_element");
    let mut exclusive = tag.has_option("exclusive");
    let free_text = tag.has_option("free_text");
    let multiple = if tag.basetype == "checkbox" {
        !exclusive
    } else {
        exclusive = false;
        false
    };
    if exclusive {
        class.push_str(" wpcf7-exclusive-checkbox");
    }
    let mut atts = Atts::default();
    atts.set("class", Att::opt(tag.get_class_option(&class)));
    atts.set("id", r.id_option(tag));
    let mut tabindex: Option<i64> = tag.get_option("tabindex", "signed_int").map(|t| t.parse().unwrap_or(0));
    let values = &tag.values;
    let labels = &tag.labels;
    let default_choice = r.default_values(tag, multiple);
    let default_choice: Vec<String> = if multiple { default_choice } else { default_choice.into_iter().take(1).collect() };
    let mut html = String::new();
    for (i, value) in values.iter().enumerate() {
        let label = labels.get(i).unwrap_or(value);
        let mut item = Atts::default();
        item.set("type", Att::str(&tag.basetype));
        item.set("name", Att::Str(format!("{}{}", tag.name, if multiple { "[]" } else { "" })));
        item.set("value", Att::Str(value.clone()));
        item.set("checked", Att::Bool(default_choice.contains(value)));
        item.set("tabindex", tabindex.map_or(Att::Bool(false), Att::Int));
        let item_atts = format_atts(&item);
        let mut item_html = if label_first {
            format!("<span class=\"wpcf7-list-item-label\">{}</span><input {item_atts} />", esc_html(label))
        } else {
            format!("<input {item_atts} /><span class=\"wpcf7-list-item-label\">{}</span>", esc_html(label))
        };
        if use_label_element {
            item_html = format!("<label>{item_html}</label>");
        }
        if let Some(t) = tabindex.as_mut()
            && *t > 0
        {
            *t += 1;
        }
        let mut item_class = "wpcf7-list-item".to_owned();
        if i == 0 {
            item_class.push_str(" first");
        }
        if i + 1 == values.len() {
            item_class.push_str(" last");
            if free_text {
                let mut free = Atts::default();
                free.set("type", Att::str("text"));
                free.set("name", Att::Str(format!("_wpcf7_free_text_{}", tag.name)));
                free.set("class", Att::str("wpcf7-free-text"));
                free.set("tabindex", tabindex.map_or(Att::Bool(false), Att::Int));
                item_html.push_str(&format!(" <input {} />", format_atts(&free)));
                item_class.push_str(" has-free-text");
            }
        }
        html.push_str(&format!("<span class=\"{}\">{item_html}</span>", esc_attr(&item_class)));
    }
    wrap(&tag.name, &format!("<span {}>{html}</span>", format_atts(&atts)))
}

/// `wpcf7_acceptance_form_tag_handler()`.
fn acceptance(r: &mut Render<'_>, tag: &FormTag) -> String {
    let mut class = form_controls_class(&tag.tag_type, "");
    if tag.has_option("invert") {
        class.push_str(" invert");
    }
    if tag.has_option("optional") {
        class.push_str(" optional");
    }
    let mut atts = Atts::default();
    atts.set("class", Att::Str(class.trim().to_owned()));
    let mut item = Atts::default();
    item.set("type", Att::str("checkbox"));
    item.set("name", Att::str(&tag.name));
    item.set("value", Att::str("1"));
    item.set("tabindex", Att::opt(tag.get_option("tabindex", "signed_int")));
    item.set("checked", Att::Bool(tag.has_option("default:on")));
    item.set("class", tag.get_class_option("").map_or(Att::Null, Att::Str));
    item.set("id", r.id_option(tag));
    item.set("aria-invalid", Att::str("false"));
    let item_atts = format_atts(&item);
    let content = if tag.content.is_empty() || tag.content == "0" { tag.values.first().cloned().unwrap_or_default() } else { tag.content.clone() };
    let content = content.trim();
    let html = if truthy(content) {
        let inner = if tag.has_option("label_first") {
            format!("<span class=\"wpcf7-list-item-label\">{content}</span><input {item_atts} />")
        } else {
            format!("<input {item_atts} /><span class=\"wpcf7-list-item-label\">{content}</span>")
        };
        format!("<span class=\"wpcf7-list-item\"><label>{inner}</label></span>")
    } else {
        format!("<span class=\"wpcf7-list-item\"><input {item_atts} /></span>")
    };
    wrap(&tag.name, &format!("<span {}>{html}</span>", format_atts(&atts)))
}

/// `wpcf7_canonicalize( $text, array( 'strip_separators' => true ) )` (without `mb_convert_kana()`'s width folding).
pub fn canonicalize(text: &str, strip_separators: bool) -> String {
    static SEPARATORS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\r\n\t ]+").unwrap());
    let text = rustpress_core::formatting::html_entity_decode(text);
    let text = SEPARATORS.replace_all(&text, if strip_separators { "" } else { " " });
    super::formatter::strip_whitespaces(&text.to_lowercase(), super::formatter::Side::Both).to_owned()
}

/// `wpcf7_quiz_form_tag_handler()`.
fn quiz(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "");
    let mut atts = Atts::default();
    atts.set("size", Att::Str(tag.get_size_option().unwrap_or_else(|| "40".to_owned())));
    atts.set("maxlength", Att::opt(tag.get_maxlength_option()));
    atts.set("minlength", Att::opt(tag.get_minlength_option()));
    if length_conflict(&atts) {
        atts.unset("maxlength");
        atts.unset("minlength");
    }
    atts.set("class", Att::opt(tag.get_class_option(&class)));
    atts.set("id", r.id_option(tag));
    atts.set("tabindex", Att::opt(tag.get_option("tabindex", "signed_int")));
    atts.set("autocomplete", Att::str("off"));
    atts.set("aria-required", Att::str("true"));
    atts.set("aria-invalid", Att::str("false"));
    let (question, answer) = match tag.pipes.len() {
        0 => ("1+1=?".to_owned(), "2".to_owned()),
        n => {
            let pipe = &tag.pipes[(r.random)(n)];
            (pipe.before.clone(), pipe.after.clone())
        }
    };
    let answer = canonicalize(&answer, true);
    atts.set("type", Att::str("text"));
    atts.set("name", Att::str(&tag.name));
    format!(
        "<span class=\"wpcf7-form-control-wrap\" data-name=\"{}\"><label><span class=\"wpcf7-quiz-label\">{}</span> <input {} /></label><input type=\"hidden\" name=\"_wpcf7_quiz_answer_{}\" value=\"{}\" /></span>",
        esc_attr(&tag.name),
        esc_html(&question),
        format_atts(&atts),
        tag.name,
        (r.quiz_hash)(&answer)
    )
}

/// `wpcf7_acceptable_filetypes( $types, 'attr' )`.
fn acceptable_filetypes_attr(types: &[String]) -> String {
    static WILDCARD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^([a-z]+)/[*]$").unwrap());
    static SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\t\n\x0B\x0C\r |,]+").unwrap());
    let mut list: Vec<String> = Vec::new();
    if types.is_empty() {
        list = vec!["audio/*".into(), "video/*".into(), "image/*".into()];
    } else {
        for t in types {
            for part in SPLIT.split(&t.to_lowercase()) {
                if !part.is_empty() && part != "0" && !list.iter().any(|l| l == part) {
                    list.push(part.to_owned());
                }
            }
        }
    }
    let out: Vec<String> = list
        .iter()
        .filter_map(|t| {
            if !t.contains('/') {
                Some(format!(".{}", t.trim_matches('.')))
            } else if let Some(c) = WILDCARD.captures(t) {
                ["audio", "video", "image"].contains(&&c[1]).then(|| t.clone())
            } else {
                rustpress_content::media::MIME_TYPES.iter().any(|(_, m)| m == t).then(|| t.clone())
            }
        })
        .filter(|t| !t.is_empty())
        .collect();
    out.join(",")
}

/// `wpcf7_file_form_tag_handler()`.
fn file(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "");
    let mut atts = Atts::default();
    atts.set("size", Att::Str(tag.get_size_option().unwrap_or_else(|| "40".to_owned())));
    atts.set("class", Att::opt(tag.get_class_option(&class)));
    atts.set("id", r.id_option(tag));
    atts.set("capture", Att::opt(tag.get_option("capture", "(user|environment)")));
    atts.set("tabindex", Att::opt(tag.get_option("tabindex", "signed_int")));
    atts.set("accept", Att::Str(acceptable_filetypes_attr(&tag.get_options("filetypes", ""))));
    required_and_valid(&mut atts, tag);
    atts.set("type", Att::str("file"));
    atts.set("name", Att::str(&tag.name));
    wrap(&tag.name, &format!("<input {} />", format_atts(&atts)))
}

/// `wpcf7_hidden_form_tag_handler()`.
fn hidden(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "");
    let mut atts = Atts::default();
    atts.set("class", Att::opt(tag.get_class_option(&class)));
    atts.set("id", r.id_option(tag));
    let value = r.default_option(tag, tag.values.first().cloned()).unwrap_or_default();
    atts.set("value", Att::Str(value));
    atts.set("type", Att::str("hidden"));
    atts.set("name", Att::str(&tag.name));
    format!("<input {} />", format_atts(&atts))
}

/// `wpcf7_count_form_tag_handler()`.
fn count(r: &mut Render<'_>, tag: &FormTag) -> String {
    let target = super::form_tag::scan(r.template).into_iter().find(|t| t.name == tag.name && t.tag_type != "count");
    let (mut maxlength, mut minlength) = target.map_or((None, None), |t| (t.get_maxlength_option(), t.get_minlength_option()));
    if let (Some(max), Some(min)) = (&maxlength, &minlength)
        && max.parse::<i64>().unwrap_or(0) < min.parse::<i64>().unwrap_or(0)
    {
        maxlength = None;
        minlength = None;
    }
    let (value, class) = if tag.has_option("down") {
        (maxlength.as_deref().and_then(|m| m.parse::<i64>().ok()).unwrap_or(0).to_string(), "wpcf7-character-count down")
    } else {
        ("0".to_owned(), "wpcf7-character-count up")
    };
    let mut atts = Atts::default();
    atts.set("id", r.id_option(tag));
    atts.set("class", Att::opt(tag.get_class_option(class)));
    atts.set("data-target-name", Att::str(&tag.name));
    atts.set("data-starting-value", Att::Str(value.clone()));
    atts.set("data-current-value", Att::Str(value.clone()));
    atts.set("data-maximum-value", maxlength.map_or(Att::Null, Att::Str));
    atts.set("data-minimum-value", minlength.map_or(Att::Null, Att::Str));
    format!("<span {}>{value}</span>", format_atts(&atts))
}

/// `wpcf7_submit_form_tag_handler()`.
fn submit(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "has-spinner");
    let mut atts = common(r, tag, &class);
    let value = tag.values.first().cloned().filter(|v| truthy(v)).unwrap_or_else(|| (r.tr)("Send"));
    atts.set("type", Att::str("submit"));
    atts.set("value", Att::Str(value));
    format!("<input {} />", format_atts(&atts))
}

/// `wpcf7_reflection_form_tag_handler()`.
fn reflection(r: &mut Render<'_>, tag: &FormTag) -> String {
    let values = if tag.values.is_empty() { vec![String::new()] } else { tag.values.clone() };
    let mut content = String::new();
    for item in &values {
        let mut atts = Atts::default();
        atts.set("name", Att::str(&tag.name));
        atts.set("data-default", Att::Str(item.clone()));
        content.push_str(&format!("<output {}>{}</output>", format_atts(&atts), if item.is_empty() { "&nbsp;".to_owned() } else { esc_html(item) }));
    }
    let mut atts = Atts::default();
    atts.set("data-reflection-of", Att::str(&tag.name));
    atts.set("class", Att::opt(tag.get_class_option(&form_controls_class(&tag.tag_type, ""))));
    atts.set("id", r.id_option(tag));
    format!("<fieldset {}>{content}</fieldset>", format_atts(&atts))
}

/// `wpcf7_output_form_tag_handler()`.
fn output(r: &mut Render<'_>, tag: &FormTag) -> String {
    let value = tag.values.first().cloned().unwrap_or_default();
    let mut atts = Atts::default();
    atts.set("data-reflection-of", Att::str(&tag.name));
    atts.set("data-default", Att::Str(value.clone()));
    atts.set("name", Att::str(&tag.name));
    atts.set("class", Att::opt(tag.get_class_option(&form_controls_class(&tag.tag_type, ""))));
    atts.set("id", r.id_option(tag));
    format!("<output {}>{}</output>", format_atts(&atts), esc_html(&value))
}

/// `wpcf7_stripe_form_tag_handler()`.
fn stripe(r: &Render<'_>, tag: &FormTag) -> String {
    let first = tag.values.first().map_or_else(|| (r.tr)("Proceed to checkout"), |v| v.trim().to_owned());
    let second = tag.values.get(1).map_or_else(|| (r.tr)("Complete payment"), |v| v.trim().to_owned());
    format!(
        "<div class=\"wpcf7-stripe\"><div class=\"wpcf7-form-control-wrap hidden\"><div class=\"card-element wpcf7-form-control\" aria-invalid=\"false\"></div></div> <span class=\"buttons has-spinner\"><button type=\"submit\" class=\"first\">{}</button> <button type=\"button\" class=\"second hidden\">{}</button></span> <input type=\"hidden\" name=\"_wpcf7_stripe_payment_intent\" value=\"\" /></div>",
        esc_html(&first),
        esc_html(&second)
    )
}

/// `wpcf7_captchar_form_tag_handler()`: the answer field of Really Simple CAPTCHA.
fn captchar(r: &mut Render<'_>, tag: &FormTag) -> String {
    let class = form_controls_class(&tag.tag_type, "");
    let mut atts = Atts::default();
    atts.set("size", Att::Str(tag.get_size_option().unwrap_or_else(|| "40".to_owned())));
    atts.set("maxlength", Att::opt(tag.get_maxlength_option()));
    atts.set("minlength", Att::opt(tag.get_minlength_option()));
    if length_conflict(&atts) {
        atts.unset("maxlength");
        atts.unset("minlength");
    }
    atts.set("class", Att::opt(tag.get_class_option(&class)));
    atts.set("id", r.id_option(tag));
    atts.set("tabindex", Att::opt(tag.get_option("tabindex", "signed_int")));
    atts.set("autocomplete", Att::str("off"));
    atts.set("aria-invalid", Att::str("false"));
    let mut value = tag.values.first().cloned().unwrap_or_default();
    if tag.has_option("placeholder") || tag.has_option("watermark") {
        atts.set("placeholder", Att::Str(value));
        value = String::new();
    }
    atts.set("value", Att::Str(value));
    atts.set("type", Att::str("text"));
    atts.set("name", Att::str(&tag.name));
    wrap(&tag.name, &format!("<input {} />", format_atts(&atts)))
}

/// `wpcf7_captchac_form_tag_handler()` without the Really Simple CAPTCHA plugin.
fn captchac(r: &Render<'_>) -> String {
    let text = (r.tr)("<strong>Warning:</strong> The <a href=\"%s\">Really Simple CAPTCHA</a> plugin is not active.").replacen("%s", "https://wordpress.org/plugins/really-simple-captcha/", 1);
    rustpress_core::kses::kses_data(&text)
}

/// The HTML of a form-tag, `None` when it is not a type of a handler (the response tag is the form's).
pub fn render(r: &mut Render<'_>, tag: &FormTag) -> Option<String> {
    let named = !tag.name.is_empty();
    Some(match tag.basetype.as_str() {
        "text" | "email" | "url" | "tel" if named => text(r, tag),
        "textarea" if named => textarea(r, tag),
        "number" | "range" if named => number(r, tag),
        "date" if named => date(r, tag),
        "select" if named => select(r, tag),
        "checkbox" | "radio" if named => checkbox(r, tag),
        "acceptance" if named => acceptance(r, tag),
        "quiz" if named => quiz(r, tag),
        "file" if named => file(r, tag),
        "hidden" if named => hidden(r, tag),
        "count" if named => count(r, tag),
        "reflection" if named => reflection(r, tag),
        "output" if named => output(r, tag),
        "captchac" => captchac(r),
        "captchar" if named => captchar(r, tag),
        "stripe" => stripe(r, tag),
        "submit" => submit(r, tag),
        "response" => return None,
        _ => String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_dates() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(parse_date("2026-12-31", today).as_deref(), Some("2026-12-31"));
        assert_eq!(parse_date("tomorrow", today).as_deref(), Some("2026-02-01"));
        assert_eq!(parse_date("+1 month", today).as_deref(), Some("2026-03-03"));
        assert_eq!(parse_date("-2 weeks", today).as_deref(), Some("2026-01-17"));
        assert_eq!(parse_date("not a date", today), None);
    }

    #[test]
    fn controls_class() {
        assert_eq!(form_controls_class("text*", "wpcf7-text"), "wpcf7-form-control wpcf7-text wpcf7-validates-as-required");
        assert_eq!(form_controls_class("submit", "has-spinner"), "wpcf7-form-control wpcf7-submit has-spinner");
    }
}
