//! `wp_timezone_choice()`: the drop-down of timezones.

use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html};

/// `(continent, identifier, label)` in the order of the PHP list.
const LIST: &str = include_str!("../timezones.tsv");

/// The identifiers of `timezone_identifiers_list()` that are in a continent.
pub fn identifiers() -> impl Iterator<Item = &'static str> {
    LIST.lines().filter_map(|l| l.split('\t').nth(1))
}

/// Whether `zone` is accepted by `timezone_identifiers_list( DateTimeZone::ALL_WITH_BC )`.
pub fn is_valid(zone: &str) -> bool {
    zone == "UTC" || identifiers().any(|z| z == zone) || zone.parse::<chrono_tz::Tz>().is_ok()
}

fn offset_label(offset: f64) -> String {
    // `UTC+5:30`, `UTC-12`, `UTC+0`
    let sign = if offset < 0.0 { "-" } else { "+" };
    let abs = offset.abs();
    let hours = abs.trunc() as i64;
    let minutes = ((abs - abs.trunc()) * 60.0).round() as i64;
    if minutes > 0 { format!("UTC{sign}{hours}:{minutes:02}") } else { format!("UTC{sign}{hours}") }
}

fn offset_value(offset: f64) -> String {
    // `UTC+5.75`, `UTC-11.5`, `UTC+0`
    let sign = if offset < 0.0 { "-" } else { "+" };
    let abs = offset.abs();
    if abs.fract() == 0.0 { format!("UTC{sign}{}", abs as i64) } else { format!("UTC{sign}{abs}") }
}

/// The options of the `<select>`, with `selected` marked.
pub fn choice(site: &Site, selected: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    if selected.is_empty() {
        out.push(format!("<option selected=\"selected\" value=\"\">{}</option>", site.tr("Select a city")));
    }
    let mut previous = "";
    let mut lines = LIST.lines().peekable();
    while let Some(line) = lines.next() {
        let mut fields = line.split('\t');
        let (continent, value, label) = (fields.next().unwrap_or_default(), fields.next().unwrap_or_default(), fields.next().unwrap_or_default());
        if continent == "UTC" {
            continue;
        }
        if continent != previous {
            out.push(format!("<optgroup label=\"{}\" dir=\"auto\">", esc_attr(continent)));
            previous = continent;
        }
        let chosen = if value == selected { "selected=\"selected\" " } else { "" };
        out.push(format!("<option {chosen}value=\"{}\" dir=\"auto\">{}</option>", esc_attr(value), esc_html(label)));
        let next_continent = lines.peek().and_then(|l| l.split('\t').next()).unwrap_or_default();
        if next_continent != continent {
            out.push("</optgroup>".to_owned());
        }
    }
    out.push(format!("<optgroup label=\"{}\" dir=\"auto\">", esc_attr(&site.tr("UTC"))));
    out.push(format!("<option {}value=\"UTC\" dir=\"auto\">{}</option>", if selected == "UTC" { "selected=\"selected\" " } else { "" }, site.tr("UTC")));
    out.push("</optgroup>".to_owned());
    out.push(format!("<optgroup label=\"{}\" dir=\"auto\">", esc_attr(&site.tr("Manual Offsets"))));
    for offset in [
        -12.0, -11.5, -11.0, -10.5, -10.0, -9.5, -9.0, -8.5, -8.0, -7.5, -7.0, -6.5, -6.0, -5.5, -5.0, -4.5, -4.0, -3.5, -3.0, -2.5, -2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 5.75, 6.0, 6.5, 7.0, 7.5, 8.0, 8.5, 8.75, 9.0, 9.5,
        10.0, 10.5, 11.0, 11.5, 12.0, 12.75, 13.0, 13.75, 14.0,
    ] {
        let value = offset_value(offset);
        let label = offset_label(offset);
        let chosen = if value == selected { "selected=\"selected\" " } else { "" };
        out.push(format!("<option {chosen}value=\"{value}\" dir=\"auto\">{}</option>", esc_html(&label)));
    }
    out.push("</optgroup>".to_owned());
    out.join("\n")
}
