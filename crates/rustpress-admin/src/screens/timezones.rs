//! `wp_timezone_choice()`: the drop-down of timezones.

use rustpress_core::Site;
use rustpress_blocks::general::strnatcasecmp;
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

/// A zone with its names in the language of the site.
struct Zone {
    continent: &'static str,
    value: &'static str,
    t_continent: String,
    t_city: String,
    t_subcity: String,
}

/// The zones in the order of `_wp_timezone_choice_usort_callback()`, with the names of `continents-cities-<locale>.mo`.
fn zones(site: &Site) -> Vec<Zone> {
    let i18n = site.i18n();
    let translate = |name: &str| i18n.header(".", "continents-cities", std::path::Path::new(""), &name.replace('_', " "));
    let mut zones: Vec<Zone> = LIST
        .lines()
        .filter(|line| !line.starts_with("UTC\t"))
        .filter_map(|line| {
            let value = line.split('\t').nth(1)?;
            let mut parts = value.split('/');
            let continent = parts.next()?;
            let (city, subcity) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
            Some(Zone { continent, value, t_continent: translate(continent), t_city: translate(city), t_subcity: translate(subcity) })
        })
        .collect();
    zones.sort_by(|a, b| {
        if a.t_continent != b.t_continent {
            return strnatcasecmp(&a.t_continent, &b.t_continent);
        }
        if a.t_city != b.t_city {
            return strnatcasecmp(&a.t_city, &b.t_city);
        }
        strnatcasecmp(&a.t_subcity, &b.t_subcity)
    });
    zones
}

/// The options of the `<select>`, with `selected` marked.
pub fn choice(site: &Site, selected: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    if selected.is_empty() {
        out.push(format!("<option selected=\"selected\" value=\"\">{}</option>", site.tr("Select a city")));
    }
    let mut previous = "";
    let zones = zones(site);
    let mut zones = zones.iter().peekable();
    while let Some(zone) = zones.next() {
        if zone.continent != previous {
            out.push(format!("<optgroup label=\"{}\" dir=\"auto\">", esc_attr(&zone.t_continent)));
            previous = zone.continent;
        }
        let chosen = if zone.value == selected { "selected=\"selected\" " } else { "" };
        let display = if zone.t_subcity.is_empty() { zone.t_city.clone() } else { format!("{} - {}", zone.t_city, zone.t_subcity) };
        out.push(format!("<option {chosen}value=\"{}\" dir=\"auto\">{}</option>", esc_attr(zone.value), esc_html(&display)));
        if zones.peek().map(|next| next.continent) != Some(zone.continent) {
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
