//! `WP_Locale`: everything that depends on the language but is not a sentence, read from the
//! core catalog. No language is known to the code; a new language only needs its `.mo` file.
//!
//! The English texts below are the `msgid`s WordPress uses, so any WordPress language pack
//! (about 200 on translate.wordpress.org) works unchanged.

use regex::Regex;

use crate::datetime::SiteTimezone;
use crate::i18n::I18n;

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const MONTH_ABBREVIATIONS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const WEEKDAY_ABBREVIATIONS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// How words are counted when text is shortened (`WP_Locale::get_word_count_type()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WordCountType {
    /// Split at whitespace.
    #[default]
    Words,
    /// Count characters, without whitespace (e.g. Chinese, Japanese, Korean).
    CharactersExcludingSpaces,
    /// Count characters, with whitespace.
    CharactersIncludingSpaces,
}

/// The translated names and conventions of one locale.
#[derive(Debug, Clone)]
pub struct LocaleData {
    pub weekday: [String; 7],
    pub weekday_abbrev: [String; 7],
    /// One-letter weekday names (`M` for Monday).
    pub weekday_initial: [String; 7],
    pub month: [String; 12],
    pub month_genitive: [String; 12],
    pub month_abbrev: [String; 12],
    /// `am`, `pm`, `AM`, `PM`.
    pub meridiem: [String; 4],
    pub thousands_sep: String,
    pub decimal_point: String,
    pub list_item_separator: String,
    pub rtl: bool,
    /// Months take the genitive case after a day number (Russian, Polish, Finnish, ...).
    pub decline_months: bool,
    pub word_count_type: WordCountType,
    /// `get_bloginfo( 'language' )`.
    pub language_tag: String,
}

impl LocaleData {
    /// Reads the locale from the `default` catalog of `i18n`; texts without a translation stay English.
    pub fn from_i18n(i18n: &I18n) -> Self {
        let tr = |text: &str| i18n.gettext("default", text);
        let trx = |text: &str, context: &str| i18n.pgettext("default", context, text);
        let thousands_sep = tr("number_format_thousands_sep");
        let thousands_sep = if thousands_sep == "number_format_thousands_sep" { ",".to_owned() } else { thousands_sep.replace(' ', "&nbsp;") };
        let decimal_point = tr("number_format_decimal_point");
        let decimal_point = if decimal_point == "number_format_decimal_point" { ".".to_owned() } else { decimal_point };
        let word_count_type = match trx("words", "Word count type. Do not translate!").as_str() {
            "characters_excluding_spaces" => WordCountType::CharactersExcludingSpaces,
            "characters_including_spaces" => WordCountType::CharactersIncludingSpaces,
            _ => WordCountType::Words,
        };
        let language_tag = {
            let tag = tr("html_lang_attribute");
            if tag == "html_lang_attribute" || tag.chars().any(|c| !(c.is_ascii_alphanumeric() || c == '-')) {
                let locale = if i18n.locale.is_empty() { "en_US" } else { i18n.locale.as_str() };
                locale.replace('_', "-")
            } else {
                tag
            }
        };
        Self {
            weekday: WEEKDAYS.map(&tr),
            weekday_initial: std::array::from_fn(|i| trx(&WEEKDAYS[i][..1], &format!("{} initial", WEEKDAYS[i]))),
            weekday_abbrev: std::array::from_fn(|i| trx(WEEKDAY_ABBREVIATIONS[i], &format!("{} abbreviation", WEEKDAYS[i]))),
            month: MONTHS.map(&tr),
            month_genitive: MONTHS.map(|m| trx(m, "genitive")),
            month_abbrev: std::array::from_fn(|i| trx(MONTH_ABBREVIATIONS[i], &format!("{} abbreviation", MONTHS[i]))),
            meridiem: ["am", "pm", "AM", "PM"].map(&tr),
            thousands_sep,
            decimal_point,
            list_item_separator: tr(", "),
            rtl: trx("ltr", "text direction") == "rtl",
            decline_months: trx("off", "decline months names: on or off") == "on",
            word_count_type,
            language_tag,
        }
    }

    /// `number_format_i18n()` for a whole number.
    pub fn number(&self, n: u64) -> String {
        let digits = n.to_string();
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                out.push_str(&self.thousands_sep);
            }
            out.push(c);
        }
        out
    }

    /// `wp_date( $format, $timestamp )` in the site's time zone: names, abbreviations and
    /// meridiems come from this locale, months are declined where the language needs it.
    pub fn date(&self, timezone: &SiteTimezone, format: &str, timestamp: i64) -> String {
        // Letters of a translation must not be read as format characters (`addcslashes`).
        let literal = |text: &str| text.chars().map(|c| if c.is_ascii_alphabetic() || c == '\\' { format!("\\{c}") } else { c.to_string() }).collect::<String>();
        let index = |code: &str, max: usize| timezone.format(code, timestamp).parse::<usize>().unwrap_or(0).min(max);
        let month = index("n", 12).clamp(1, 12) - 1;
        let weekday = index("w", 6);
        let mut localized = String::new();
        // `r` is unpacked first: its parts can be translated.
        let mut stack: Vec<char> = format.chars().rev().collect();
        while let Some(c) = stack.pop() {
            match c {
                'r' => stack.extend("D, d M Y H:i:s O".chars().rev()),
                'D' => localized.push_str(&literal(&self.weekday_abbrev[weekday])),
                'F' => localized.push_str(&literal(&self.month[month])),
                'l' => localized.push_str(&literal(&self.weekday[weekday])),
                'M' => localized.push_str(&literal(&self.month_abbrev[month])),
                'a' | 'A' => {
                    let slot = match timezone.format(&c.to_string(), timestamp).as_str() {
                        "am" => 0,
                        "pm" => 1,
                        "AM" => 2,
                        _ => 3,
                    };
                    localized.push_str(&literal(&self.meridiem[slot]));
                }
                '\\' => {
                    localized.push(c);
                    if let Some(next) = stack.pop() {
                        localized.push(next);
                    }
                }
                other => localized.push(other),
            }
        }
        let date = timezone.format(&localized, timestamp);
        self.maybe_decline(&date, &unpack_r(format))
    }

    /// `wp_maybe_decline_date()` with a known format.
    fn maybe_decline(&self, date: &str, format: &str) -> String {
        if !self.decline_months {
            return date.to_owned();
        }
        let mut date = date.to_owned();
        if Regex::new(r"[dj]\.? F").is_ok_and(|re| re.is_match(format)) {
            // "j F Y": the month after a day number.
            for (nominative, genitive) in self.month.iter().zip(&self.month_genitive) {
                if let Ok(re) = Regex::new(&format!(r" {}\b", regex::escape(nominative))) {
                    date = re.replace_all(&date, regex::NoExpand(&format!(" {genitive}"))).into_owned();
                }
            }
        }
        if Regex::new(r"F [dj]").is_ok_and(|re| re.is_match(format)) {
            // "F j": becomes "j F" with the declined month.
            for (nominative, genitive) in self.month.iter().zip(&self.month_genitive) {
                if let Ok(re) = Regex::new(&format!(r"\b{} (\d{{1,2}})(st|nd|rd|th)?([-–]\d{{1,2}})?(st|nd|rd|th)?\b", regex::escape(nominative))) {
                    date = re
                        .replace_all(&date, |caps: &regex::Captures<'_>| {
                            format!("{}{} {genitive}", caps.get(1).map_or("", |m| m.as_str()), caps.get(3).map_or("", |m| m.as_str()))
                        })
                        .into_owned();
                }
            }
        }
        date
    }
}

/// `r` stands for "D, d M Y H:i:s O"; `wp_date` unpacks it before it checks the format.
fn unpack_r(format: &str) -> String {
    let mut out = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                out.push(c);
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            'r' => out.push_str("D, d M Y H:i:s O"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn russian() -> LocaleData {
        let mut data = LocaleData::from_i18n(&I18n::default());
        data.month[9] = "октябрь".into();
        data.month_genitive[9] = "октября".into();
        data.decline_months = true;
        data
    }

    #[test]
    fn english_is_the_fallback() {
        let data = LocaleData::from_i18n(&I18n::default());
        assert_eq!(data.month[2], "March");
        assert_eq!(data.weekday_abbrev[1], "Mon");
        assert_eq!(data.month_abbrev[11], "Dec");
        assert_eq!(data.thousands_sep, ",");
        assert_eq!(data.decimal_point, ".");
        assert_eq!(data.language_tag, "en-US");
        assert!(!data.rtl && !data.decline_months);
        assert_eq!(data.word_count_type, WordCountType::Words);
        assert_eq!(data.number(1234567), "1,234,567");
    }

    #[test]
    fn months_are_declined_after_a_day_number() {
        let data = russian();
        assert_eq!(data.maybe_decline("24 октябрь 2026", "j F Y"), "24 октября 2026");
        assert_eq!(data.maybe_decline("24. октябрь 2026", "j. F Y"), "24. октября 2026");
        // Without a day in front of the month the nominative stays.
        assert_eq!(data.maybe_decline("октябрь 2026", "F Y"), "октябрь 2026");
        // "F j" becomes "j F" and loses the ordinal suffix.
        assert_eq!(data.maybe_decline("октябрь 24", "F j"), "24 октября");
    }

    #[test]
    fn declension_is_off_unless_the_language_asks_for_it() {
        let mut data = russian();
        data.decline_months = false;
        assert_eq!(data.maybe_decline("24 октябрь 2026", "j F Y"), "24 октябрь 2026");
    }

    #[test]
    fn the_shorthand_r_is_unpacked_before_the_format_is_checked() {
        assert_eq!(unpack_r("r"), "D, d M Y H:i:s O");
        assert_eq!(unpack_r("\\r"), "\\r");
    }
}
