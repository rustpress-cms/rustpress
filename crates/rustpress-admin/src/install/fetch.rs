//! The files of WordPress that RustPress serves and reads (styles, scripts, block definitions, themes,
//! translations): downloaded from WordPress.org like `download_url()` and `update_core()` do, checked against the
//! checksums of `get_core_checksums()` and unpacked into the site.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// The release whose PHP code RustPress follows: the files must be of the same version.
pub const VERSION: &str = rustpress_core::WP_VERSION;

/// Downloads larger than this are refused (a release is about 30 MB).
const MAX_PACKAGE: u64 = 200 * 1024 * 1024;

/// How the package goes into the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// An empty webspace: every file is written, files that are already there are kept.
    Fresh,
    /// `update_core()` of an existing site: the core files are replaced; `wp-content` keeps its themes and plugins
    /// (new bundled ones are added) and gets the translations of the package.
    Replace,
}

/// A language the release is translated into (`wp_get_available_translations()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translation {
    pub language: String,
    pub native_name: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(240))).user_agent(format!("WordPress/{VERSION}; RustPress")).build().into()
}

fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut response = agent().get(url).call().map_err(|e| format!("Download failed. {e}"))?;
    response.body_mut().with_config().limit(limit).read_to_vec().map_err(|e| format!("Download failed. {e}"))
}

fn json(url: &str) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&get(url, 16 * 1024 * 1024)?).map_err(|_| "Invalid data provided.".to_owned())
}

/// `sanitize_locale_name()` of a locale that goes into a URL.
fn clean_locale(locale: &str) -> String {
    locale.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect()
}

/// The `package` of a core offer: the localized build, or the English one for `en_US`.
pub fn package_url(locale: &str) -> String {
    let locale = clean_locale(locale);
    if locale.is_empty() || locale == "en_US" {
        format!("https://downloads.wordpress.org/release/wordpress-{VERSION}.zip")
    } else {
        format!("https://downloads.wordpress.org/release/{locale}/wordpress-{VERSION}.zip")
    }
}

/// `wp_get_available_translations()` for the release (the translations API of WordPress.org).
pub fn translations() -> Vec<Translation> {
    let Ok(body) = json(&format!("https://api.wordpress.org/translations/core/1.0/?version={VERSION}")) else { return Vec::new() };
    let mut list: Vec<Translation> = body["translations"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|t| Some(Translation { language: t["language"].as_str()?.to_owned(), native_name: t["native_name"].as_str().unwrap_or_default().to_owned() }))
                .collect()
        })
        .unwrap_or_default();
    list.sort_by(|a, b| a.language.cmp(&b.language));
    list
}

/// `get_core_checksums()`: the MD5 of every file of the release, by its path below the site root.
pub fn checksums(locale: &str) -> Result<HashMap<String, String>, String> {
    let locale = match clean_locale(locale) {
        l if l.is_empty() => "en_US".to_owned(),
        l => l,
    };
    let body = json(&format!("https://api.wordpress.org/core/checksums/1.0/?version={VERSION}&locale={locale}"))?;
    let mut sums = &body["checksums"];
    // Asked for one version, the API answers with the files; asked for several, it groups them by version.
    if let Some(by_version) = sums.get(VERSION).filter(|v| v.is_object()) {
        sums = by_version;
    }
    let map: HashMap<String, String> = sums.as_object().map(|o| o.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned()))).collect()).unwrap_or_default();
    if map.is_empty() {
        return Err("The checksums could not be retrieved from WordPress.org.".to_owned());
    }
    Ok(map)
}

/// What [install] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Unpacked {
    pub written: usize,
    pub kept: usize,
}

/// Downloads the release in `locale` and puts it into `wp_root`.
pub fn install(wp_root: &Path, locale: &str, mode: Mode) -> Result<Unpacked, String> {
    let sums = checksums(locale)?;
    let package = get(&package_url(locale), MAX_PACKAGE)?;
    unpack(&package, &sums, wp_root, mode)
}

/// Whether `path` (below the site root) is written in `mode` when it exists already.
fn overwrite(path: &str, mode: Mode) -> bool {
    match mode {
        Mode::Fresh => false,
        Mode::Replace => !path.starts_with("wp-content/") || path.starts_with("wp-content/languages/"),
    }
}

/// Checks the package against `sums` and writes it into `wp_root`. Nothing is written unless every core file is
/// there with its checksum; `wp-includes/version.php` comes last, so an interrupted run leaves the old version
/// (or none) behind and is simply repeated.
pub fn unpack(package: &[u8], sums: &HashMap<String, String>, wp_root: &Path, mode: Mode) -> Result<Unpacked, String> {
    let incompatible = || "Incompatible Archive.".to_owned();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(package)).map_err(|_| incompatible())?;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| incompatible())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.enclosed_name().ok_or_else(incompatible)?.to_string_lossy().replace('\\', "/");
        let Some(path) = name.strip_prefix("wordpress/") else { return Err(incompatible()) };
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data).map_err(|_| incompatible())?;
        files.push((path.to_owned(), data));
    }
    // `verify_file_md5()` of every file the release lists; the core files must all be there.
    let present: HashMap<&str, &[u8]> = files.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
    for (path, sum) in sums {
        let Some(data) = present.get(path.as_str()) else {
            if path.starts_with("wp-content/") {
                continue;
            }
            return Err(format!("The package could not be installed. The file {path} is missing."));
        };
        if format!("{:x}", md5::compute(data)) != sum.to_ascii_lowercase() {
            return Err(format!("The checksum of the file {path} does not match."));
        }
    }
    files.sort_by_key(|(p, _)| p == "wp-includes/version.php");
    let mut done = Unpacked::default();
    for (path, data) in &files {
        // `update_core()` never copies a `wp-config.php` of the package (7.1.4).
        if path == "wp-config.php" {
            continue;
        }
        let target = wp_root.join(path);
        if target.exists() && !overwrite(path, mode) {
            done.kept += 1;
            continue;
        }
        write(&target, data).map_err(|e| format!("Could not copy file. {path}: {e}"))?;
        done.written += 1;
    }
    Ok(done)
}

/// Writes next to `target` and renames, so a file is never left half written.
fn write(target: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut temporary = target.as_os_str().to_owned();
    temporary.push(".rustpress-new");
    std::fs::write(&temporary, data)?;
    std::fs::rename(&temporary, target)
}

/// The first page of an empty webspace: RustPress needs the files of WordPress before `setup-config.php` can
/// run. It has no styles of its own to load yet, so everything it shows is inline.
pub fn page(translations: &[Translation], error: Option<&str>) -> crate::login::LoginResponse {
    use rustpress_core::formatting::{esc_attr, esc_html};
    let mut options = String::from("<option value=\"en_US\" lang=\"en\">English (United States)</option>");
    for t in translations {
        let selected = if t.language == "de_DE" { " selected" } else { "" };
        let lang = t.language.split('_').next().unwrap_or_default();
        options.push_str(&format!("<option value=\"{}\" lang=\"{}\"{selected}>{}</option>", esc_attr(&t.language), esc_attr(lang), esc_html(&t.native_name)));
    }
    let logo = super::brand::logo_url();
    let error = error.map(|e| format!("<p class=\"error\">{}</p>", esc_html(e))).unwrap_or_default();
    let body = format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\" />\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n<meta name=\"robots\" content=\"noindex,nofollow\" />\n<title>RustPress &rsaquo; Setup</title>\n<style>\nbody{{background:#f0f0f1;color:#3c434a;font-family:-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,Oxygen-Sans,Ubuntu,Cantarell,\"Helvetica Neue\",sans-serif;font-size:14px;line-height:1.5;margin:0;padding:0 16px}}\nmain{{background:#fff;box-shadow:0 1px 3px rgba(0,0,0,.13);margin:0 auto 50px;max-width:700px;padding:1em 2em 2em}}\n#logo{{background:{logo} center top/84px no-repeat;height:84px;margin:50px auto 25px;overflow:hidden;text-indent:-9999px;width:84px}}\nh1{{font-size:20px;font-weight:600}}select,button{{font-size:14px;padding:6px 10px}}select{{min-width:16em}}\nbutton{{background:#2271b1;border:1px solid #2271b1;border-radius:3px;color:#fff;cursor:pointer}}\n.error{{border-left:4px solid #d63638;background:#fcf0f1;padding:8px 12px}}\n</style>\n</head>\n<body>\n<p id=\"logo\">RustPress</p>\n<main>\n<h1>RustPress</h1>\n{error}<p>Before RustPress can be set up, it needs the files of WordPress {VERSION}: the styles and scripts of the dashboard and the editor, the block definitions, the default theme and the translations. They are downloaded from WordPress.org and checked against their published checksums; files that are already on the webspace are kept.</p>\n<form method=\"post\" action=\"/wp-admin/setup-config.php\" onsubmit=\"this.querySelector('button').disabled=true;this.querySelector('button').textContent='Downloading&hellip;'\">\n<input type=\"hidden\" name=\"rustpress-fetch\" value=\"1\" />\n<p><label for=\"language\">Language</label><br /><select name=\"language\" id=\"language\">{options}</select></p>\n<p><button type=\"submit\">Download WordPress {VERSION}</button></p>\n</form>\n</main>\n</body>\n</html>\n"
    );
    let headers = vec![("Content-Type".to_owned(), "text/html; charset=utf-8".to_owned()), ("Cache-Control".to_owned(), "no-cache, must-revalidate, max-age=0, no-store, private".to_owned())];
    crate::login::LoginResponse { status: if error.is_empty() { 200 } else { 500 }, headers, cookies: Vec::new(), body, bytes: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn package(files: &[(&str, &str)]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
            for (name, text) in files {
                zip.start_file(format!("wordpress/{name}"), zip::write::SimpleFileOptions::default()).unwrap();
                zip.write_all(text.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        buffer
    }

    fn sums(files: &[(&str, &str)]) -> HashMap<String, String> {
        files.iter().map(|(n, t)| (n.to_string(), format!("{:x}", md5::compute(t)))).collect()
    }

    fn root(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustpress-fetch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const FILES: &[(&str, &str)] = &[
        ("index.php", "<?php // new"),
        ("wp-includes/version.php", "<?php $wp_version = '7.1.3';"),
        ("wp-content/themes/twentytwentyfive/style.css", "/* new theme */"),
        ("wp-content/languages/de_DE.mo", "new mo"),
    ];

    #[test]
    fn fresh_keeps_files_and_replace_keeps_wp_content() {
        let dir = root("modes");
        std::fs::create_dir_all(dir.join("wp-content/themes/twentytwentyfive")).unwrap();
        std::fs::write(dir.join("index.php"), "old").unwrap();
        std::fs::write(dir.join("wp-content/themes/twentytwentyfive/style.css"), "old").unwrap();
        let done = unpack(&package(FILES), &sums(FILES), &dir, Mode::Fresh).unwrap();
        assert_eq!(done, Unpacked { written: 2, kept: 2 });
        assert_eq!(std::fs::read_to_string(dir.join("index.php")).unwrap(), "old");
        std::fs::write(dir.join("wp-content/languages/de_DE.mo"), "old").unwrap();
        let done = unpack(&package(FILES), &sums(FILES), &dir, Mode::Replace).unwrap();
        assert_eq!(done, Unpacked { written: 3, kept: 1 });
        assert_eq!(std::fs::read_to_string(dir.join("index.php")).unwrap(), "<?php // new");
        assert_eq!(std::fs::read_to_string(dir.join("wp-content/languages/de_DE.mo")).unwrap(), "new mo");
        assert_eq!(std::fs::read_to_string(dir.join("wp-content/themes/twentytwentyfive/style.css")).unwrap(), "old");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_packages_write_nothing() {
        let dir = root("bad");
        let mut wrong = sums(FILES);
        wrong.insert("index.php".into(), "0".repeat(32));
        assert!(unpack(&package(FILES), &wrong, &dir, Mode::Fresh).unwrap_err().contains("checksum"));
        let mut missing = sums(FILES);
        missing.insert("wp-admin/admin.php".into(), "0".repeat(32));
        assert!(unpack(&package(FILES), &missing, &dir, Mode::Fresh).unwrap_err().contains("missing"));
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn urls() {
        assert_eq!(package_url("en_US"), format!("https://downloads.wordpress.org/release/wordpress-{VERSION}.zip"));
        assert_eq!(package_url("de_DE"), format!("https://downloads.wordpress.org/release/de_DE/wordpress-{VERSION}.zip"));
        assert_eq!(package_url("../x"), format!("https://downloads.wordpress.org/release/x/wordpress-{VERSION}.zip"));
    }
}
