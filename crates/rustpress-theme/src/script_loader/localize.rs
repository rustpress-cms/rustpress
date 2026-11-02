//! Data WordPress attaches to core scripts when they are enqueued (`wp_enqueue_scripts`, `admin_enqueue_scripts`).

use rustpress_core::Site;

/// `wp_localize_jquery_ui_datepicker()`: the inline script after `jquery-ui-datepicker` with the defaults of the
/// date picker in the site's language and date format.
pub fn jquery_ui_datepicker(site: &Site) -> String {
    // `str_replace()` with arrays replaces one pair after the other, each over the result of the one before.
    let pairs = [("d", "dd"), ("j", "d"), ("l", "DD"), ("z", "o"), ("F", "MM"), ("M", "M"), ("n", "m"), ("m", "mm"), ("Y", "yy"), ("y", "y")];
    let mut format = site.options.get("date_format").unwrap_or_default().to_owned();
    for (search, replace) in pairs {
        format = format.replace(search, replace);
    }
    let locale = site.locale_data();
    let first_day = absint(site.options.get("start_of_week").unwrap_or_default());
    let defaults = serde_json::json!({
        "closeText": site.tr("Close"),
        "currentText": site.tr("Today"),
        "monthNames": locale.month,
        "monthNamesShort": locale.month_abbrev,
        "nextText": site.trx("Next", "datepicker: navigate to next month"),
        "prevText": site.trx("Previous", "datepicker: navigate to previous month"),
        "dayNames": locale.weekday,
        "dayNamesShort": locale.weekday_abbrev,
        "dayNamesMin": locale.weekday_initial,
        "dateFormat": format,
        "firstDay": first_day,
        "isRTL": locale.rtl,
    });
    let json = rustpress_blocks::php_json::encode_with(&defaults, rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false });
    // `JSON_HEX_TAG`.
    let json = json.replace('<', "\\u003C").replace('>', "\\u003E");
    format!("jQuery(function(jQuery){{jQuery.datepicker.setDefaults({json});}});")
}

/// `absint()` of an option: the leading digits, without sign.
fn absint(text: &str) -> u64 {
    let t = text.trim_start();
    let t = t.strip_prefix(['-', '+']).unwrap_or(t);
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}
