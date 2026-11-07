//! The parts of the control templates that depend on the site: `WP_Customize_Date_Time_Control::content_template()`,
//! `WP_Customize_Nav_Menu_Locations_Control::content_template()` and the Screen Options of the menus panel.

use rustpress_core::formatting::{esc_attr, wp_strip_all_tags};
use rustpress_core::i18n::sprintf;
use serde_json::{Map, Value, json};

use crate::screens::Ctx;

/// `WP_Customize_Date_Time_Control::format_gmt_offset()`.
fn format_gmt_offset(offset: f64) -> String {
    let text = if offset.fract() == 0.0 { format!("{}", offset as i64) } else { format!("{offset}") };
    let text = if offset >= 0.0 { format!("+{text}") } else { text };
    text.replace(".25", ":15").replace(".5", ":30").replace(".75", ":45")
}

/// `WP_Customize_Date_Time_Control::get_timezone_info()['description']`.
fn timezone_description(ctx: &Ctx<'_>) -> String {
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    match site.options.get("timezone_string").filter(|name| !name.is_empty()) {
        Some(name) => {
            let Ok(tz) = name.parse::<chrono_tz::Tz>() else { return String::new() };
            let now = chrono::DateTime::from_timestamp(ctx.env.now, 0).unwrap_or_default().with_timezone(&tz);
            let offset = chrono::Offset::fix(now.offset()).local_minus_utc();
            let formatted = format_gmt_offset(f64::from(offset) / 3600.0);
            sprintf(
                &tr("Your timezone is set to %1$s (%2$s), currently %3$s (Coordinated Universal Time %4$s)."),
                &[&name.replace('_', " "), &format!("<abbr>{}</abbr>", now.format("%Z")), &format!("<abbr>UTC</abbr>{formatted}"), &formatted],
            )
        }
        None => {
            // `(int) get_option( 'gmt_offset', 0 )`.
            let offset = site.options.get("gmt_offset").and_then(|value| value.trim().parse::<f64>().ok()).unwrap_or(0.0).trunc();
            let formatted = format_gmt_offset(offset);
            sprintf(&tr("Your timezone is set to %1$s (Coordinated Universal Time %2$s)."), &[&format!("<abbr>UTC</abbr>{formatted}"), &formatted])
        }
    }
}

/// The date format of the fields: the year, month and day of `date_format` as `%1$s`, `%2$s` and `%3$s`.
fn date_fields_format(date_format: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for c in date_format.chars() {
        let field = match c {
            'Y' | 'y' | 'o' if !escaped => Some("%1$s"),
            'F' | 'm' | 'M' | 'n' if !escaped => Some("%2$s"),
            'j' | 'd' if !escaped => Some("%3$s"),
            _ => None,
        };
        match field {
            Some(field) => out.push_str(field),
            None => out.push(c),
        }
        escaped = c == '\\';
    }
    if ["%1$s", "%2$s", "%3$s"].iter().any(|field| out.matches(field).count() != 1) {
        return "%1$s-%2$s-%3$s".to_owned();
    }
    out
}

/// What `content_template()` of the date control prints; `instance` is the number of the temporary control.
pub(super) fn date_time(ctx: &Ctx<'_>, instance: u32) -> String {
    let site = ctx.site();
    let tr = |text: &str| ctx.tr(text);
    let html = |text: &str| rustpress_core::formatting::esc_html(&tr(text));
    let time_format = site.options.string("time_format", "");
    let mut months = Map::new();
    let locale = site.locale_data();
    for i in 1..=12usize {
        let abbrev = locale.month_abbrev[i - 1].clone();
        months.insert(i.to_string(), json!({"text": sprintf(&tr("%1$s-%2$s"), &[&i.to_string(), &abbrev]), "value": i}));
    }
    let data = json!({
        "settings": [],
        "type": "date_time",
        "priority": 10,
        "active": true,
        "section": "",
        "content": "<li id=\"customize-control-temp\" class=\"customize-control customize-control-date_time\"></li>",
        "label": "",
        "description": "",
        "instanceNumber": instance,
        "maxYear": 9999,
        "minYear": 1000,
        "allowPastDate": true,
        "twelveHourFormat": time_format.to_lowercase().contains('a'),
        "includeTime": true,
        "month_choices": Value::Object(months),
    });
    let encoded = rustpress_blocks::php_json::encode_with(&data, rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: false });
    let month = format!(
        "<label for=\"{{{{ idPrefix }}}}date-time-month\" class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</label>\n\t\t\t\t\t<select id=\"{{{{ idPrefix }}}}date-time-month\" class=\"date-input month\" data-component=\"month\">\n\t\t\t\t\t\t<# _.each( data.month_choices, function( choice ) {{\n\t\t\t\t\t\t\tif ( _.isObject( choice ) && ! _.isUndefined( choice.text ) && ! _.isUndefined( choice.value ) ) {{\n\t\t\t\t\t\t\t\ttext = choice.text;\n\t\t\t\t\t\t\t\tvalue = choice.value;\n\t\t\t\t\t\t\t}}\n\t\t\t\t\t\t\t#>\n\t\t\t\t\t\t\t<option value=\"{{{{ value }}}}\" >\n\t\t\t\t\t\t\t\t{{{{ text }}}}\n\t\t\t\t\t\t\t</option>\n\t\t\t\t\t\t<# }} ); #>\n\t\t\t\t\t</select>",
        html("Month")
    );
    let day = format!(
        "<label for=\"{{{{ idPrefix }}}}date-time-day\" class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</label>\n\t\t\t\t\t<input id=\"{{{{ idPrefix }}}}date-time-day\" type=\"number\" size=\"2\" autocomplete=\"off\" class=\"date-input day tiny-text\" data-component=\"day\" min=\"1\" max=\"31\" />",
        html("Day")
    );
    let year = format!(
        "<label for=\"{{{{ idPrefix }}}}date-time-year\" class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</label>\n\t\t\t\t\t<input id=\"{{{{ idPrefix }}}}date-time-year\" type=\"number\" size=\"4\" autocomplete=\"off\" class=\"date-input year tiny-text\" data-component=\"year\" min=\"{{{{ data.minYear }}}}\" max=\"{{{{ data.maxYear }}}}\">",
        html("Year")
    );
    let fields = sprintf(&date_fields_format(&site.options.string("date_format", "")), &[&year, &month, &day]);
    format!(
        "\n\t\t\t\n\t\t<# _.defaults( data, {encoded} ); #>\n\t\t<# var idPrefix = _.uniqueId( 'el' ) + '-'; #>\n\n\t\t<# if ( data.label ) {{ #>\n\t\t\t<span class=\"customize-control-title\">\n\t\t\t\t{{{{ data.label }}}}\n\t\t\t</span>\n\t\t<# }} #>\n\t\t<div class=\"customize-control-notifications-container\"></div>\n\t\t<# if ( data.description ) {{ #>\n\t\t\t<span class=\"description customize-control-description\">{{{{ data.description }}}}</span>\n\t\t<# }} #>\n\t\t<div class=\"date-time-fields {{{{ data.includeTime ? 'includes-time' : '' }}}}\">\n\t\t\t<fieldset class=\"day-row\">\n\t\t\t\t<legend class=\"title-day {{{{ ! data.includeTime ? 'screen-reader-text' : '' }}}}\">{date}</legend>\n\t\t\t\t<div class=\"day-fields clear\">\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t\t{fields}\t\t\t\t</div>\n\t\t\t</fieldset>\n\t\t\t<# if ( data.includeTime ) {{ #>\n\t\t\t\t<fieldset class=\"time-row clear\">\n\t\t\t\t\t<legend class=\"title-time\">{time}</legend>\n\t\t\t\t\t<div class=\"time-fields clear\">\n\t\t\t\t\t\t<label for=\"{{{{ idPrefix }}}}date-time-hour\" class=\"screen-reader-text\">\n\t\t\t\t\t\t\t{hour}\t\t\t\t\t\t</label>\n\t\t\t\t\t\t<# var maxHour = data.twelveHourFormat ? 12 : 23; #>\n\t\t\t\t\t\t<# var minHour = data.twelveHourFormat ? 1 : 0; #>\n\t\t\t\t\t\t<input id=\"{{{{ idPrefix }}}}date-time-hour\" type=\"number\" size=\"2\" autocomplete=\"off\" class=\"date-input hour tiny-text\" data-component=\"hour\" min=\"{{{{ minHour }}}}\" max=\"{{{{ maxHour }}}}\">\n\t\t\t\t\t\t:\n\t\t\t\t\t\t<label for=\"{{{{ idPrefix }}}}date-time-minute\" class=\"screen-reader-text\">\n\t\t\t\t\t\t\t{minute}\t\t\t\t\t\t</label>\n\t\t\t\t\t\t<input id=\"{{{{ idPrefix }}}}date-time-minute\" type=\"number\" size=\"2\" autocomplete=\"off\" class=\"date-input minute tiny-text\" data-component=\"minute\" min=\"0\" max=\"59\">\n\t\t\t\t\t\t<# if ( data.twelveHourFormat ) {{ #>\n\t\t\t\t\t\t\t<label for=\"{{{{ idPrefix }}}}date-time-meridian\" class=\"screen-reader-text\">\n\t\t\t\t\t\t\t\t{meridian}\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t\t<select id=\"{{{{ idPrefix }}}}date-time-meridian\" class=\"date-input meridian\" data-component=\"meridian\">\n\t\t\t\t\t\t\t\t<option value=\"am\">{am}</option>\n\t\t\t\t\t\t\t\t<option value=\"pm\">{pm}</option>\n\t\t\t\t\t\t\t</select>\n\t\t\t\t\t\t<# }} #>\n\t\t\t\t\t\t<p>{timezone}</p>\n\t\t\t\t\t</div>\n\t\t\t\t</fieldset>\n\t\t\t<# }} #>\n\t\t</div>\n\t\t\t\t",
        date = html("Date"),
        time = html("Time"),
        hour = html("Hour"),
        minute = html("Minute"),
        meridian = html("Meridian"),
        am = html("AM"),
        pm = html("PM"),
        timezone = timezone_description(ctx),
    )
}

/// What `content_template()` of the menu locations control prints for the registered locations.
pub(super) fn menu_locations(ctx: &Ctx<'_>, locations: &[(String, String)]) -> String {
    // `current_theme_supports( 'menus' )`: `register_nav_menus()` adds it.
    if locations.is_empty() {
        return "\n\t\t\t\t\t".to_owned();
    }
    let site = ctx.site();
    let trx = |text: &str, context: &str| site.trx(text, context);
    let mut out = format!(
        "\n\t\t\t\t\t\t<# var elementId; #>\n\t\t\t<ul class=\"menu-location-settings\">\n\t\t\t\t<li class=\"customize-control assigned-menu-locations-title\">\n\t\t\t\t\t<span class=\"customize-control-title\">{{{{ wp.customize.Menus.data.l10n.locationsTitle }}}}</span>\n\t\t\t\t\t<# if ( data.isCreating ) {{ #>\n\t\t\t\t\t\t<p>\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t{}\t\t\t\t\t\t</p>\n\t\t\t\t\t<# }} else {{ #>\n\t\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t<# }} #>\n\t\t\t\t</li>\n\n\t\t\t\t",
        trx("Where do you want this menu to appear?", "menu locations"),
        sprintf(
            &trx("(If you plan to use a menu <a href=\"%1$s\" %2$s>widget%3$s</a>, skip this step.)", "menu locations"),
            &[&ctx.tr("https://wordpress.org/documentation/article/manage-wordpress-widgets/"), " class=\"external-link\" target=\"_blank\"", &format!("<span class=\"screen-reader-text\"> {}</span>", ctx.tr("(opens in a new tab)"))]
        ),
        trx("Here&#8217;s where this menu appears. If you would like to change that, pick another location.", "menu locations")
    );
    for (location, description) in locations {
        let location = esc_attr(location);
        out.push_str(&format!(
            "\t\t\t\t\t<# elementId = _.uniqueId( 'customize-nav-menu-control-location-' ); #>\n\t\t\t\t\t<li class=\"customize-control customize-control-checkbox assigned-menu-location\">\n\t\t\t\t\t\t<span class=\"customize-inside-control-row\">\n\t\t\t\t\t\t\t<input id=\"{{{{ elementId }}}}\" type=\"checkbox\" data-menu-id=\"{{{{ data.menu_id }}}}\" data-location-id=\"{location}\" class=\"menu-location\" />\n\t\t\t\t\t\t\t<label for=\"{{{{ elementId }}}}\">\n\t\t\t\t\t\t\t\t{description}\t\t\t\t\t\t\t\t<span class=\"theme-location-set\">\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t</span>\n\t\t\t\t\t</li>\n\t\t\t\t",
            sprintf(&trx("(Current: %s)", "menu location"), &[&format!("<span class=\"current-menu-location-name-{location}\"></span>")])
        ));
    }
    out.push_str("\t\t\t</ul>\n\t\t\t\t\t");
    out
}

/// `WP_Screen::render_screen_options( array( 'wrap' => false ) )` of `nav-menus.php`: the columns the menu items can show.
pub(super) fn screen_options(ctx: &Ctx<'_>, hidden: &[String]) -> String {
    let tr = |text: &str| ctx.tr(text);
    let mut out = format!("<form id='adv-settings' method='post'>\n\t\t<fieldset class=\"metabox-prefs\">\n\t\t<legend>{}</legend>\n\t\t", tr("Show advanced menu properties"));
    for (column, label) in [("link-target", "Link Target"), ("title-attribute", "Title Attribute"), ("css-classes", "CSS Classes"), ("xfn", "Link Relationship (XFN)"), ("description", "Description")] {
        let id = format!("{column}-hide");
        out.push_str(&format!(
            "<label><input class=\"hide-column-tog\" name=\"{id}\" type=\"checkbox\" id=\"{id}\" value=\"{column}\"{} />{}</label>\n",
            if hidden.iter().any(|h| h == column) { "" } else { " checked='checked'" },
            wp_strip_all_tags(&tr(label))
        ));
    }
    out.push_str(&format!("\t\t</fieldset>\n\t\t\n<input type=\"hidden\" id=\"screenoptionnonce\" name=\"screenoptionnonce\" value=\"{}\" />\n</form>\n", ctx.create_nonce("screen-options-nonce")));
    out
}
