//! Dates: the site's timezone and PHP's `date()` format characters
//! (`wp_date()`, `get_post_time()`).

use chrono::{DateTime, Datelike, FixedOffset, NaiveDateTime, Offset, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;

use crate::options::Options;

/// The timezone from `timezone_string` or `gmt_offset` (`wp_timezone()`).
#[derive(Debug, Clone)]
pub enum SiteTimezone {
    Named(Tz),
    Offset(FixedOffset),
}

impl SiteTimezone {
    pub fn from_options(options: &Options) -> Self {
        if let Some(name) = options.get("timezone_string").filter(|s| !s.is_empty())
            && let Ok(tz) = name.parse::<Tz>() {
                return SiteTimezone::Named(tz);
            }
        let hours: f64 = options.get("gmt_offset").and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
        let seconds = (hours * 3600.0).round() as i32;
        SiteTimezone::Offset(FixedOffset::east_opt(seconds).unwrap_or_else(|| FixedOffset::east_opt(0).expect("utc")))
    }

    pub fn utc() -> Self {
        SiteTimezone::Offset(FixedOffset::east_opt(0).expect("utc"))
    }

    /// Interprets a `2026-10-24 14:10:42` string as local time in this zone.
    pub fn local_timestamp(&self, mysql_datetime: &str) -> Option<i64> {
        let naive = NaiveDateTime::parse_from_str(mysql_datetime, "%Y-%m-%d %H:%M:%S").ok()?;
        match self {
            SiteTimezone::Named(tz) => tz.from_local_datetime(&naive).earliest().map(|d| d.timestamp()),
            SiteTimezone::Offset(offset) => offset.from_local_datetime(&naive).single().map(|d| d.timestamp()),
        }
    }

    /// Interprets a MySQL datetime as UTC (`post_date_gmt`).
    pub fn gmt_timestamp(mysql_datetime: &str) -> Option<i64> {
        NaiveDateTime::parse_from_str(mysql_datetime, "%Y-%m-%d %H:%M:%S").ok().map(|n| Utc.from_utc_datetime(&n).timestamp())
    }

    /// `wp_date( $format, $timestamp )`.
    pub fn format(&self, format: &str, timestamp: i64) -> String {
        let Some(utc) = DateTime::<Utc>::from_timestamp(timestamp, 0) else { return String::new() };
        match self {
            SiteTimezone::Named(tz) => {
                let local = utc.with_timezone(tz);
                let abbreviation = tz_abbreviation(tz, &local);
                php_date(format, &local.fixed_offset(), tz.name(), &abbreviation, matches!(local.offset().fix().local_minus_utc(), o if is_dst(tz, &local, o)))
            }
            SiteTimezone::Offset(offset) => {
                let local = utc.with_timezone(offset);
                let name = offset_string(offset.local_minus_utc(), true);
                php_date(format, &local, &name, &name, false)
            }
        }
    }
}

fn tz_abbreviation(_tz: &Tz, local: &DateTime<Tz>) -> String {
    local.format("%Z").to_string()
}

/// Whether the offset differs from the zone's standard offset in January.
fn is_dst(tz: &Tz, local: &DateTime<Tz>, offset: i32) -> bool {
    let year = local.year();
    let jan = tz.with_ymd_and_hms(year, 1, 1, 12, 0, 0).earliest().map(|d| d.offset().fix().local_minus_utc());
    let jul = tz.with_ymd_and_hms(year, 7, 1, 12, 0, 0).earliest().map(|d| d.offset().fix().local_minus_utc());
    match (jan, jul) {
        (Some(j), Some(u)) => offset == j.max(u) && j != u,
        _ => false,
    }
}

/// `+02:00` (`with_colon`) or `+0200`.
pub fn offset_string(seconds: i32, with_colon: bool) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let abs = seconds.abs();
    let (h, m) = (abs / 3600, (abs % 3600) / 60);
    if with_colon { format!("{sign}{h:02}:{m:02}") } else { format!("{sign}{h:02}{m:02}") }
}

fn ordinal_suffix(day: u32) -> &'static str {
    match (day % 100, day % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    }
}

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// PHP's `date()` format characters for a moment in a fixed-offset zone.
pub fn php_date(format: &str, dt: &DateTime<FixedOffset>, tz_name: &str, tz_abbr: &str, is_dst: bool) -> String {
    let mut out = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        let month_idx = dt.month0() as usize;
        let weekday = dt.weekday();
        let day_idx = weekday.num_days_from_monday() as usize;
        let hour12 = match dt.hour() % 12 {
            0 => 12,
            h => h,
        };
        let days_in_month = days_in_month(dt.year(), dt.month());
        match c {
            'd' => out.push_str(&format!("{:02}", dt.day())),
            'D' => out.push_str(&DAYS[day_idx][..3]),
            'j' => out.push_str(&dt.day().to_string()),
            'l' => out.push_str(DAYS[day_idx]),
            'N' => out.push_str(&(day_idx + 1).to_string()),
            'S' => out.push_str(ordinal_suffix(dt.day())),
            'w' => out.push_str(&weekday.num_days_from_sunday().to_string()),
            'z' => out.push_str(&dt.ordinal0().to_string()),
            'W' => out.push_str(&format!("{:02}", dt.iso_week().week())),
            'F' => out.push_str(MONTHS[month_idx]),
            'm' => out.push_str(&format!("{:02}", dt.month())),
            'M' => out.push_str(&MONTHS[month_idx][..3]),
            'n' => out.push_str(&dt.month().to_string()),
            't' => out.push_str(&days_in_month.to_string()),
            'L' => out.push(if is_leap(dt.year()) { '1' } else { '0' }),
            'o' => out.push_str(&dt.iso_week().year().to_string()),
            'Y' => out.push_str(&dt.year().to_string()),
            'y' => out.push_str(&format!("{:02}", dt.year().rem_euclid(100))),
            'a' => out.push_str(if dt.hour() < 12 { "am" } else { "pm" }),
            'A' => out.push_str(if dt.hour() < 12 { "AM" } else { "PM" }),
            'g' => out.push_str(&hour12.to_string()),
            'G' => out.push_str(&dt.hour().to_string()),
            'h' => out.push_str(&format!("{hour12:02}")),
            'H' => out.push_str(&format!("{:02}", dt.hour())),
            'i' => out.push_str(&format!("{:02}", dt.minute())),
            's' => out.push_str(&format!("{:02}", dt.second())),
            'u' => out.push_str("000000"),
            'v' => out.push_str("000"),
            'e' => out.push_str(tz_name),
            'I' => out.push(if is_dst { '1' } else { '0' }),
            'O' => out.push_str(&offset_string(dt.offset().local_minus_utc(), false)),
            'P' => out.push_str(&offset_string(dt.offset().local_minus_utc(), true)),
            'p' => {
                let secs = dt.offset().local_minus_utc();
                out.push_str(&if secs == 0 { "Z".to_owned() } else { offset_string(secs, true) });
            }
            'T' => out.push_str(tz_abbr),
            'Z' => out.push_str(&dt.offset().local_minus_utc().to_string()),
            'c' => out.push_str(&php_date("Y-m-d\\TH:i:sP", dt, tz_name, tz_abbr, is_dst)),
            'r' => out.push_str(&php_date("D, d M Y H:i:s O", dt, tz_name, tz_abbr, is_dst)),
            'U' => out.push_str(&dt.timestamp().to_string()),
            '\\' => {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            other => out.push(other),
        }
        let _ = Weekday::Mon;
    }
    out
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => 28 + is_leap(year) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tz(options: &[(&str, &str)]) -> SiteTimezone {
        SiteTimezone::from_options(&Options::from_pairs(options.iter().copied()))
    }

    #[test]
    fn utc_formatting_with_common_formats() {
        let tz = SiteTimezone::utc();
        let ts = tz.local_timestamp("2026-10-24 14:10:42").unwrap();
        assert_eq!(tz.format("F j, Y", ts), "October 24, 2026");
        assert_eq!(tz.format("c", ts), "2026-10-24T14:10:42+00:00");
        assert_eq!(tz.format("D, d M Y H:i:s O", ts), "Sat, 24 Oct 2026 14:10:42 +0000");
        assert_eq!(tz.format("l jS \\o\\f F Y h:i:s A", ts), "Saturday 24th of October 2026 02:10:42 PM");
        assert_eq!(tz.format("g:i a", ts), "2:10 pm");
        assert_eq!(tz.format("N w z t L W", ts), "6 6 296 31 0 43");
    }

    #[test]
    fn named_timezone_follows_dst() {
        let tz = tz(&[("timezone_string", "Europe/Berlin")]);
        let summer = tz.local_timestamp("2026-07-01 12:00:00").unwrap();
        let winter = tz.local_timestamp("2026-01-01 12:00:00").unwrap();
        assert_eq!(tz.format("c", summer), "2026-07-01T12:00:00+02:00");
        assert_eq!(tz.format("c", winter), "2026-01-01T12:00:00+01:00");
        assert_eq!(tz.format("T I e", summer), "CEST 1 Europe/Berlin");
        assert_eq!(tz.format("T I", winter), "CET 0");
    }

    #[test]
    fn numeric_offset() {
        let tz = tz(&[("gmt_offset", "5.5")]);
        let ts = tz.local_timestamp("2026-01-01 00:00:00").unwrap();
        assert_eq!(tz.format("c", ts), "2026-01-01T00:00:00+05:30");
        assert_eq!(tz.format("e", ts), "+05:30");
    }

    #[test]
    fn invalid_dates_yield_none() {
        assert!(SiteTimezone::utc().local_timestamp("0000-00-00 00:00:00").is_none());
        assert!(SiteTimezone::utc().local_timestamp("junk").is_none());
    }

    #[test]
    fn ordinal_suffixes() {
        assert_eq!((1..=4).map(ordinal_suffix).collect::<Vec<_>>(), ["st", "nd", "rd", "th"]);
        assert_eq!([11, 12, 13, 21, 22, 23].map(ordinal_suffix), ["th", "th", "th", "st", "nd", "rd"]);
    }
}
