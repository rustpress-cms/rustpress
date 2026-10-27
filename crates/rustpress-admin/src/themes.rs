//! The themes installed under `wp-content/themes` (`WP_Theme`): the headers of `style.css`.

use std::path::{Path, PathBuf};

/// One installed theme.
#[derive(Debug, Clone, Default)]
pub struct ThemeInfo {
    /// The directory name.
    pub stylesheet: String,
    pub name: String,
    pub theme_uri: String,
    pub description: String,
    pub author: String,
    pub author_uri: String,
    pub version: String,
    /// The directory of the parent theme, or the theme's own.
    pub template: String,
    pub tags: Vec<String>,
    pub text_domain: String,
    pub requires_wp: String,
    pub requires_php: String,
    /// The file name of the screenshot.
    pub screenshot: Option<String>,
    /// Has `templates/index.html`.
    pub block_theme: bool,
    pub dir: PathBuf,
}

impl ThemeInfo {
    pub fn has_parent(&self) -> bool {
        self.template != self.stylesheet
    }
}

pub use rustpress_core::plugins::file_header;

/// Strips the tags `wp_kses( $value, array() )` would remove.
fn plain(text: &str) -> String {
    rustpress_core::formatting::strip_tags(text)
}

/// The theme in `dir`, if its `style.css` names one.
fn read_theme(dir: &Path) -> Option<ThemeInfo> {
    let stylesheet = dir.file_name()?.to_string_lossy().into_owned();
    let bytes = std::fs::read(dir.join("style.css")).ok()?;
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]).into_owned();
    let name = file_header(&head, "Theme Name");
    if name.is_empty() {
        return None;
    }
    let template = file_header(&head, "Template");
    let tags: Vec<String> = plain(&file_header(&head, "Tags")).split(',').map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()).collect();
    let screenshot = ["png", "gif", "jpg", "jpeg", "webp", "avif"].iter().map(|ext| format!("screenshot.{ext}")).find(|f| dir.join(f).is_file());
    Some(ThemeInfo {
        name,
        theme_uri: file_header(&head, "Theme URI"),
        description: file_header(&head, "Description"),
        author: plain(&file_header(&head, "Author")),
        author_uri: file_header(&head, "Author URI"),
        version: plain(&file_header(&head, "Version")),
        template: if template.is_empty() { stylesheet.clone() } else { template },
        tags,
        text_domain: file_header(&head, "Text Domain"),
        requires_wp: file_header(&head, "Requires at least"),
        requires_php: file_header(&head, "Requires PHP"),
        screenshot,
        block_theme: dir.join("templates").join("index.html").is_file(),
        stylesheet,
        dir: dir.to_path_buf(),
    })
}

/// The installed themes by name (`wp_get_themes()`).
pub fn installed(wp_root: &Path) -> Vec<ThemeInfo> {
    let Ok(entries) = std::fs::read_dir(wp_root.join("wp-content").join("themes")) else { return Vec::new() };
    let mut themes: Vec<ThemeInfo> = entries.flatten().filter(|e| e.path().is_dir()).filter_map(|e| read_theme(&e.path())).collect();
    themes.sort_by_key(|t| t.name.to_lowercase());
    themes
}

/// One theme by its directory name.
pub fn find(wp_root: &Path, stylesheet: &str) -> Option<ThemeInfo> {
    if stylesheet.is_empty() || stylesheet.contains(['/', '\\']) || stylesheet.starts_with('.') {
        return None;
    }
    read_theme(&wp_root.join("wp-content").join("themes").join(stylesheet))
}

pub use rustpress_theme::theme_headers::feature_label;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_are_found_in_a_comment() {
        let css = "/*\nTheme Name: Twenty Test\nAuthor: the team\nTags: one-column, custom-menu\nVersion: 1.2\n*/\nbody{}";
        assert_eq!(file_header(css, "Theme Name"), "Twenty Test");
        assert_eq!(file_header(css, "Version"), "1.2");
        assert_eq!(file_header(css, "Theme URI"), "");
    }

    #[test]
    fn tags_have_labels() {
        assert_eq!(feature_label("one-column"), Some("One Column"));
        assert_eq!(feature_label("custom-menu"), None);
    }
}
