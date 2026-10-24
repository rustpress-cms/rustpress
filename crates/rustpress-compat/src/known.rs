//! Documented deviations from WordPress: differences that are understood and accepted
//! (or planned) and therefore do not fail a run.

use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Deviation {
    /// Regular expression for the request path with query string.
    pub path: String,
    /// `*`, an aspect (`status`, `body`, ...), or a prefix ending in `*` (`header:*`).
    #[serde(default = "any")]
    pub aspect: String,
    pub reason: String,
    /// Only for this theme (the `wp-theme-<slug>` of the reference); all themes if absent.
    #[serde(default)]
    pub theme: Option<String>,
}

fn any() -> String {
    "*".to_owned()
}

#[derive(Debug, Deserialize, Default)]
struct File {
    #[serde(default)]
    deviation: Vec<Deviation>,
}

#[derive(Debug, Default)]
pub struct Known {
    entries: Vec<(Regex, Deviation)>,
}

impl Known {
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut entries = Vec::new();
        for deviation in file.deviation {
            let regex = Regex::new(&deviation.path).map_err(|e| format!("`{}`: {e}", deviation.path))?;
            entries.push((regex, deviation));
        }
        Ok(Self { entries })
    }

    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    /// The deviation that covers `aspect` of `path` for `theme`, if any.
    pub fn matching(&self, path: &str, aspect: &str, theme: &str) -> Option<&Deviation> {
        self.entries.iter().find(|(regex, d)| {
            regex.is_match(path)
                && d.theme.as_deref().is_none_or(|t| t == theme)
                && (d.aspect == "*" || d.aspect == aspect || d.aspect.strip_suffix('*').is_some_and(|prefix| aspect.starts_with(prefix)))
        }).map(|(_, d)| d)
    }

    pub fn entries(&self) -> impl Iterator<Item = &Deviation> {
        self.entries.iter().map(|(_, d)| d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[deviation]]
path = "^/wp-login[.]php"
reason = "no login"

[[deviation]]
path = "^/wp-json/$"
aspect = "body"
reason = "route list is a subset"

[[deviation]]
path = "^/"
aspect = "header:*"
theme = "twentytwentyfour"
reason = "only for the second theme"
"#;

    #[test]
    fn deviations_match_by_path_aspect_and_theme() {
        let known = Known::parse(SAMPLE).unwrap();
        assert!(known.matching("/wp-login.php", "status", "twentytwentyfive").is_some());
        assert!(known.matching("/wp-json/", "body", "twentytwentyfive").is_some());
        assert!(known.matching("/wp-json/", "status", "twentytwentyfive").is_none());
        assert!(known.matching("/x/", "header:link", "twentytwentyfive").is_none());
        assert!(known.matching("/x/", "header:link", "twentytwentyfour").is_some());
        assert_eq!(known.entries().count(), 3);
    }

    #[test]
    fn broken_files_are_reported() {
        assert!(Known::parse("[[deviation]]\npath = \"(\"\nreason = \"x\"").is_err());
        assert!(Known::parse("not toml [").is_err());
        assert!(Known::load(std::path::Path::new("does/not/exist.toml")).unwrap().entries().next().is_none());
    }
}
