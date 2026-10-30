//! `wp_get_admin_notice()` and the settings errors of `settings_errors()`.

use rustpress_core::formatting::esc_attr;

/// `wp_get_admin_notice( $message, $args )`. `classes` may start with `notice-<type>`.
pub fn notice(message: &str, kind: &str, dismissible: bool, id: &str, additional: &[&str], paragraph: bool) -> String {
    let mut classes = String::from("notice");
    if !kind.is_empty() {
        classes.push_str(&format!(" notice-{kind}"));
    }
    if dismissible {
        classes.push_str(" is-dismissible");
    }
    if !additional.is_empty() {
        classes.push(' ');
        classes.push_str(&additional.join(" "));
    }
    let id = if id.trim().is_empty() { String::new() } else { format!("id=\"{}\" ", id.trim()) };
    let message = if paragraph { format!("<p>{message}</p>") } else { message.to_owned() };
    format!("<div {id}class=\"{classes}\">{message}</div>\n")
}

/// A notice with extra classes (`notice-warning`) and a paragraph.
pub fn admin_notice(message: &str, additional: &[&str]) -> String {
    notice(message, "", false, "", additional, true)
}

/// One entry of `get_settings_errors()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsError {
    pub setting: String,
    pub code: String,
    pub message: String,
    /// `error`, `success`, `warning` or `info`; `updated` is read as `success`.
    pub kind: String,
}

/// `settings_errors()`: the notices of a settings screen.
pub fn render_settings_errors(errors: &[SettingsError], tr_updated: bool) -> String {
    let _ = tr_updated;
    let mut out = String::new();
    for error in errors {
        let kind = if error.kind == "updated" { "success" } else { error.kind.as_str() };
        let kind = if matches!(kind, "error" | "success" | "warning" | "info") { format!("notice-{kind}") } else { kind.to_owned() };
        out.push_str(&format!(
            "<div id='setting-error-{}' class='notice {} settings-error is-dismissible'> \n<p><strong>{}</strong></p></div> \n",
            esc_attr(&error.code),
            esc_attr(&kind),
            error.message
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_errors_are_printed_like_wordpress() {
        let error = |kind: &str| SettingsError { setting: "general".into(), code: "settings_updated".into(), message: "Settings saved.".into(), kind: kind.into() };
        assert_eq!(
            render_settings_errors(&[error("updated")], false),
            "<div id='setting-error-settings_updated' class='notice notice-success settings-error is-dismissible'> \n<p><strong>Settings saved.</strong></p></div> \n"
        );
        // Types outside the four notice kinds stay as they are.
        assert!(render_settings_errors(&[error("custom")], false).contains("class='notice custom settings-error is-dismissible'"));
    }
}
