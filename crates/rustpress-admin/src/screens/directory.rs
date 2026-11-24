//! The plugin directory of RustPress on `plugin-install.php`: in place of WordPress.org's directory (`plugins_api()`),
//! whose plugins need PHP, the browse tabs list the plugins RustPress has a native module for
//! ([`rustpress_core::native::catalog`]), and "Install Now" installs the release the module follows.
//!
//! This is a deliberate difference from WordPress. The cards are those of `WP_Plugin_Install_List_Table::display_rows()`
//! and the buttons those of `wp_get_plugin_action_button()`, so that `updates.js` installs and activates them like on
//! WordPress (`wp_ajax_install_plugin()`, then `wp_ajax_activate_plugin()`); without JavaScript the button leads to
//! `update.php?action=install-plugin`. Ratings, installation counts and the "More Details" modal, which come from
//! WordPress.org, are left out: "More Details" opens the plugin's page there.
//!
//! The package is downloaded from WordPress.org and installed only if its SHA-256 is the one of the listing.

use std::path::{Path, PathBuf};

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::native::Listing;

use super::Ctx;
use crate::plugins::{self, PluginInfo};

/// Where users ask for a module for another plugin (the "Port request" issue template).
pub const PORT_REQUEST_URL: &str =
    "https://github.com/rustpress-cms/rustpress/issues/new?template=port-request.yml";

/// A text of RustPress' own (not one of WordPress), in the language of `locale`. Only German has translations.
pub fn own(locale: &str, text: &str) -> String {
    if !locale.starts_with("de") {
        return text.to_owned();
    }
    let german = match text {
        "RustPress Plugins" => "RustPress-Plugins",
        "These plugins have a native module in RustPress. Installing one downloads the release the module follows from WordPress.org and checks its checksum. Other plugins do not run with RustPress, since it does not run PHP; you can still upload a plugin in .zip format by clicking the button above." => {
            "Für diese Plugins hat RustPress ein eigenes Modul. Beim Installieren lädt RustPress die Version, der das Modul folgt, von WordPress.org und prüft ihre Prüfsumme. Andere Plugins laufen mit RustPress nicht, weil es kein PHP ausführt; ein Plugin im .zip-Format kannst du trotzdem mit dem Button oben hochladen."
        }
        "Missing a plugin? <a href=\"%s\">Ask for a port</a>." => {
            "Dir fehlt ein Plugin? <a href=\"%s\">Wünsch dir eine Portierung</a>."
        }
        "<strong>Native module</strong> in RustPress" => {
            "<strong>Eigenes Modul</strong> in RustPress"
        }
        "RustPress follows version %1$s, version %2$s is installed." => {
            "RustPress folgt Version %1$s, installiert ist Version %2$s."
        }
        "RustPress has no module for this plugin." => "Für dieses Plugin hat RustPress kein Modul.",
        "The package could not be downloaded. If this server cannot reach WordPress.org, download it yourself from <a href=\"%1$s\">%1$s</a> and upload it here." => {
            "Das Paket konnte nicht heruntergeladen werden. Wenn dieser Server WordPress.org nicht erreicht, lade es selbst von <a href=\"%1$s\">%1$s</a> herunter und lade es hier hoch."
        }
        "The package does not match its checksum." => {
            "Das Paket stimmt nicht mit seiner Prüfsumme überein."
        }
        _ => text,
    };
    german.to_owned()
}

/// The listing of `slug`.
pub fn listing(slug: &str) -> Option<&'static Listing> {
    rustpress_core::native::catalog()
        .iter()
        .find(|l| l.slug == slug)
}

fn locale(ctx: &Ctx<'_>) -> String {
    ctx.site().i18n().locale.clone()
}

fn wp_root(ctx: &Ctx<'_>) -> Option<PathBuf> {
    ctx.env.loader.includes.parent().map(Path::to_path_buf)
}

/// `install_plugin_install_status()` against the installed plugins: the installed plugin of the listing, if any.
fn installed<'a>(listing: &Listing, all: &'a [PluginInfo]) -> Option<&'a PluginInfo> {
    all.iter().find(|p| p.file == listing.file).or_else(|| {
        all.iter()
            .find(|p| p.file.starts_with(&format!("{}/", listing.slug)))
    })
}

/// The listings `search` finds (all of them without a term): the term in the name, slug or description.
pub fn search(locale: &str, term: &str) -> Vec<&'static Listing> {
    let term = term.trim().to_lowercase();
    rustpress_core::native::catalog()
        .iter()
        .filter(|l| {
            term.is_empty()
                || [l.name, l.slug, l.description, l.description(locale)]
                    .iter()
                    .any(|text| text.to_lowercase().contains(&term))
        })
        .collect()
}

/// `wp_get_plugin_action_button()`.
fn action_button(
    ctx: &Ctx<'_>,
    listing: &Listing,
    name: &str,
    installed: Option<&PluginInfo>,
    active: &[String],
) -> String {
    match installed {
        None => format!(
            "<a class=\"install-now button button-compact\" data-slug=\"{}\" href=\"{}\" aria-label=\"{}\" data-name=\"{}\" role=\"button\">{}</a>",
            esc_attr(listing.slug),
            esc_url(&format!(
                "{}?action=install-plugin&plugin={}&_wpnonce={}",
                ctx.admin_url("update.php"),
                listing.slug,
                ctx.create_nonce(&format!("install-plugin_{}", listing.slug))
            )),
            esc_attr(&rustpress_core::i18n::sprintf(
                &ctx.site().trx("Install %s now", "plugin"),
                &[name]
            )),
            esc_attr(name),
            ctx.site().trx("Install Now", "plugin")
        ),
        Some(plugin)
            if ctx.can("update_plugins")
                && crate::updates::version_compare(&plugin.version, listing.version)
                    == std::cmp::Ordering::Less =>
        {
            format!(
                "<a class=\"update-now button button-compact aria-button-if-js\" data-plugin=\"{}\" data-slug=\"{}\" href=\"{}\" aria-label=\"{}\" data-name=\"{}\" role=\"button\">{}</a>",
                esc_attr(&plugin.file),
                esc_attr(listing.slug),
                esc_url(&update_url(ctx, &plugin.file)),
                esc_attr(&rustpress_core::i18n::sprintf(
                    &ctx.site().trx("Update %s now", "plugin"),
                    &[name]
                )),
                esc_attr(name),
                ctx.site().trx("Update Now", "plugin")
            )
        }
        Some(plugin) if active.contains(&plugin.file) => format!(
            "<button type=\"button\" class=\"button button-compact button-disabled\" disabled=\"disabled\">{}</button>",
            ctx.site().trx("Active", "plugin")
        ),
        Some(plugin) if ctx.can("activate_plugins") => format!(
            "<a href=\"{}\" data-name=\"{}\" data-slug=\"{}\" data-plugin=\"{}\" class=\"button button-compact button-primary activate-now\" aria-label=\"{}\" role=\"button\">{}</a>",
            esc_url(&activate_url(ctx, &plugin.file)),
            esc_attr(name),
            esc_attr(listing.slug),
            esc_attr(&plugin.file),
            esc_attr(&rustpress_core::i18n::sprintf(
                &ctx.site().trx("Activate %s", "plugin"),
                &[name]
            )),
            ctx.site().trx("Activate", "plugin")
        ),
        Some(_) => format!(
            "<button type=\"button\" class=\"button button-compact button-disabled\" disabled=\"disabled\">{}</button>",
            ctx.site().trx("Installed", "plugin")
        ),
    }
}

/// `add_query_arg( array( '_wpnonce', 'action' => 'activate', 'plugin' ), admin_url( 'plugins.php' ) )`.
fn activate_url(ctx: &Ctx<'_>, file: &str) -> String {
    format!(
        "{}?_wpnonce={}&action=activate&plugin={}",
        ctx.admin_url("plugins.php"),
        ctx.create_nonce(&format!("activate-plugin_{file}")),
        super::install::urlencode(file)
    )
}

/// `wp_nonce_url( self_admin_url( 'update.php?action=upgrade-plugin&plugin=' . $file ), 'upgrade-plugin_' . $file )`.
pub fn update_url(ctx: &Ctx<'_>, file: &str) -> String {
    format!(
        "{}?action=upgrade-plugin&plugin={}&_wpnonce={}",
        ctx.admin_url("update.php"),
        file,
        ctx.create_nonce(&format!("upgrade-plugin_{file}"))
    )
}

/// A letter on a colored square, in place of the plugin's icon from WordPress.org.
fn icon(listing: &Listing) -> String {
    let letter = listing
        .name
        .chars()
        .next()
        .unwrap_or('P')
        .to_ascii_uppercase();
    let hue = listing
        .slug
        .bytes()
        .fold(0u32, |h, b| (h * 31 + u32::from(b)) % 360);
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 128 128'><rect width='128' height='128' fill='hsl({hue},45%,42%)'/><text x='64' y='86' font-family='sans-serif' font-size='64' font-weight='bold' fill='white' text-anchor='middle'>{letter}</text></svg>"
    );
    format!(
        "data:image/svg+xml,{}",
        svg.replace('<', "%3C")
            .replace('>', "%3E")
            .replace('#', "%23")
            .replace(' ', "%20")
    )
}

/// The cards of `WP_Plugin_Install_List_Table::display_rows()` for `listings`.
pub fn cards(ctx: &Ctx<'_>, listings: &[&Listing]) -> String {
    let locale = locale(ctx);
    let all = wp_root(ctx)
        .map(|root| plugins::installed(&root))
        .unwrap_or_default();
    let active = super::plugins::active_plugins(ctx);
    let mut out = String::new();
    for listing in listings {
        let name = format!("{} {}", listing.name, listing.version);
        let name = rustpress_core::formatting::strip_tags(&name);
        let installed = installed(listing, &all);
        let author = if listing.author_profile.is_empty() {
            esc_html(listing.author)
        } else {
            format!(
                "<a href=\"{}\">{}</a>",
                esc_url(listing.author_profile),
                esc_html(listing.author)
            )
        };
        let author = format!(
            " <cite>{}</cite>",
            rustpress_core::i18n::sprintf(&ctx.tr("By %s"), &[&author])
        );
        let details = listing.directory_url();
        let compatibility = match installed {
            Some(plugin) if plugin.version != listing.version => format!(
                "<span class=\"compatibility-untested\">{}</span>",
                rustpress_core::i18n::sprintf(
                    &own(
                        &locale,
                        "RustPress follows version %1$s, version %2$s is installed."
                    ),
                    &[listing.version, &esc_html(&plugin.version)]
                )
            ),
            _ => format!(
                "<span class=\"compatibility-compatible\">{}</span>",
                own(&locale, "<strong>Native module</strong> in RustPress")
            ),
        };
        out.push_str(&format!(
            "\t\t<div class=\"plugin-card plugin-card-{slug}\">\n\t\t\t\t\t<div class=\"plugin-card-top\">\n\t\t\t\t<div class=\"name column-name\">\n\t\t\t\t\t<h3>\n\t\t\t\t\t\t<a href=\"{details}\" target=\"_blank\" rel=\"noopener\">\n\t\t\t\t\t\t{title}\t\t\t\t\t\t<img src=\"{icon}\" class=\"plugin-icon\" alt=\"\" />\n\t\t\t\t\t\t</a>\n\t\t\t\t\t</h3>\n\t\t\t\t</div>\n\t\t\t\t<div class=\"action-links\">\n\t\t\t\t\t<ul class=\"plugin-action-buttons\"><li>{button}</li><li><a href=\"{details}\" target=\"_blank\" rel=\"noopener\" aria-label=\"{more_label}\">{more}</a></li></ul>\t\t\t\t</div>\n\t\t\t\t<div class=\"desc column-description\">\n\t\t\t\t\t<p>{description}</p>\n\t\t\t\t\t<p class=\"authors\">{author}</p>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t\t\t\t<div class=\"plugin-card-bottom\">\n\t\t\t\t<div class=\"vers column-rating\">\n\t\t\t\t\t{version}\n\t\t\t\t</div>\n\t\t\t\t<div class=\"column-compatibility\">\n\t\t\t\t\t{compatibility}\t\t\t\t</div>\n\t\t\t</div>\n\t\t</div>\n",
            slug = esc_attr(listing.slug),
            details = esc_url(&details),
            title = listing.name,
            icon = esc_attr(&icon(listing)),
            button = action_button(ctx, listing, &name, installed, &active),
            more_label = esc_attr(&rustpress_core::i18n::sprintf(
                &ctx.tr("More information about %s"),
                &[&name]
            )),
            more = ctx.tr("More Details"),
            description = rustpress_core::formatting::strip_tags(listing.description(&locale)),
            version = rustpress_core::i18n::sprintf(&ctx.tr("Version %s"), &[listing.version]),
        ));
    }
    out
}

/// Downloads the package of `listing`, checks it and unpacks it into `wp-content/plugins`: the work of
/// `Plugin_Upgrader::install()`. Returns the feedback lines of the skin so far and, on failure, the error.
pub async fn install(ctx: &Ctx<'_>, listing: &'static Listing) -> Result<Vec<String>, String> {
    put(ctx, listing, false).await
}

/// [install] for a plugin that is installed in an older version: `Plugin_Upgrader::upgrade()`. The new release is
/// unpacked beside the old one and swapped in, so that a failure leaves the old one in place. The plugin stays in
/// `active_plugins`: WordPress deactivates it while it updates and activates it again, a module needs neither.
pub async fn update(ctx: &Ctx<'_>, listing: &'static Listing) -> Result<Vec<String>, String> {
    put(ctx, listing, true).await
}

async fn put(
    ctx: &Ctx<'_>,
    listing: &'static Listing,
    update: bool,
) -> Result<Vec<String>, String> {
    let tr = |s: &str| ctx.tr(s);
    let locale = locale(ctx);
    let Some(root) = wp_root(ctx) else {
        return Err("The WordPress files are not available.".to_owned());
    };
    let plugins_dir = root.join("wp-content").join("plugins");
    let destination = plugins_dir.join(listing.slug);
    if update {
        if !plugins_dir.join(listing.file).is_file() {
            return Err(tr("The plugin is not installed."));
        }
    } else if destination.exists() {
        return Err(format!(
            "{} {}/",
            tr("Destination folder already exists."),
            destination.display().to_string().replace('\\', "/")
        ));
    }
    let mut lines = vec![format!(
        "<p>{}</p>",
        rustpress_core::i18n::sprintf(
            &rustpress_core::i18n::sprintf(
                &tr(if update {
                    "Downloading update from %s&#8230;"
                } else {
                    "Downloading installation package from %s&#8230;"
                }),
                &["<span class=\"code pre\">%s</span>"]
            ),
            &[&esc_html(listing.package)]
        )
    )];
    let package = listing.package;
    let size = listing.size;
    let downloaded = rustpress_content::media::blocking(move || download(package, size + 1)).await;
    let Ok(bytes) = downloaded else {
        return Err(rustpress_core::i18n::sprintf(
            &own(
                &locale,
                "The package could not be downloaded. If this server cannot reach WordPress.org, download it yourself from <a href=\"%1$s\">%1$s</a> and upload it here.",
            ),
            &[&esc_url(listing.package)],
        ));
    };
    if sha256_hex(&bytes) != listing.sha256 {
        return Err(own(&locale, "The package does not match its checksum."));
    }
    lines.push(format!(
        "<p>{}</p>",
        tr(if update {
            "Unpacking the update&#8230;"
        } else {
            "Unpacking the package&#8230;"
        })
    ));
    let entries = rustpress_content::media::blocking(move || super::install::read_zip(&bytes))
        .await
        .map_err(|reason| tr(&reason))?;
    lines.push(format!(
        "<p>{}</p>",
        tr(if update {
            "Installing the latest version&#8230;"
        } else {
            "Installing the plugin&#8230;"
        })
    ));
    let (folder, files) = super::install::layout(&entries, &format!("{}.zip", listing.slug));
    if folder != listing.slug {
        return Err(tr("No valid plugins were found."));
    }
    if !update {
        let written = {
            let destination = destination.clone();
            rustpress_content::media::blocking(move || {
                super::install::write_tree(&destination, &files)
            })
            .await
        };
        if written.is_err() {
            let _ = std::fs::remove_dir_all(&destination);
            return Err(tr("Could not create directory."));
        }
        return Ok(lines);
    }
    let staged = plugins_dir.join(format!(".rustpress-update-{}", listing.slug));
    let old = plugins_dir.join(format!(".rustpress-old-{}", listing.slug));
    let swapped = {
        let (staged, old, destination) = (staged.clone(), old.clone(), destination.clone());
        rustpress_content::media::blocking(move || swap(&staged, &old, &destination, &files)).await
    };
    match swapped {
        Ok(()) => {
            lines.push(format!(
                "<p>{}</p>",
                tr("Removing the old version of the plugin&#8230;")
            ));
            let _ = std::fs::remove_dir_all(&old);
            Ok(lines)
        }
        Err(()) => Err(tr("Could not create directory.")),
    }
}

/// Writes `files` to `staged`, then moves `destination` to `old` and `staged` to `destination`; on a failure the old
/// version stays (or is moved back).
fn swap(
    staged: &Path,
    old: &Path,
    destination: &Path,
    files: &[(String, Option<Vec<u8>>)],
) -> Result<(), ()> {
    let _ = std::fs::remove_dir_all(staged);
    let _ = std::fs::remove_dir_all(old);
    if super::install::write_tree(staged, files).is_err() {
        let _ = std::fs::remove_dir_all(staged);
        return Err(());
    }
    if std::fs::rename(destination, old).is_err() {
        let _ = std::fs::remove_dir_all(staged);
        return Err(());
    }
    if std::fs::rename(staged, destination).is_err() {
        let _ = std::fs::rename(old, destination);
        let _ = std::fs::remove_dir_all(staged);
        return Err(());
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn download(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .user_agent(format!(
            "WordPress/{}; RustPress",
            rustpress_core::WP_VERSION
        ))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| e.to_string())?;
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

/// `update.php?action=install-plugin`: the page of `Plugin_Installer_Skin`.
pub async fn install_page(ctx: &Ctx<'_>) -> super::AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("install_plugins") {
        return super::common::die_plain(
            ctx,
            "Sorry, you are not allowed to install plugins on this site.",
        );
    }
    let slug = ctx.request.get("plugin").unwrap_or_default().to_owned();
    if !ctx.verify_nonce(&format!("install-plugin_{slug}")) {
        return super::die(
            ctx.env,
            403,
            &tr("An error occurred."),
            &tr("The link you followed has expired."),
        );
    }
    let Some(listing) = listing(&slug) else {
        return super::die(
            ctx.env,
            404,
            "WordPress &rsaquo; Error",
            &format!(
                "<p>{}</p>",
                own(&locale(ctx), "RustPress has no module for this plugin.")
            ),
        );
    };
    if ctx.env.writer.is_none() {
        return super::common::read_only(ctx);
    }
    let heading = rustpress_core::i18n::sprintf(
        &tr("Installing Plugin: %s"),
        &[&format!("{} {}", listing.name, listing.version)],
    );
    let lines = match install(ctx, listing).await {
        Ok(mut lines) => {
            lines.push(format!("<p>{}</p>", tr("Plugin installed successfully.")));
            lines.push(format!(
                "<p><a class=\"button button-primary\" href=\"{}\" target=\"_parent\">{}</a> <a href=\"{}\" target=\"_parent\">{}</a></p>",
                esc_url(&activate_url(ctx, listing.file)),
                tr("Activate Plugin"),
                ctx.admin_url("plugin-install.php"),
                tr("Go to Plugin Installer")
            ));
            lines
        }
        Err(reason) => vec![
            format!("<p>{reason}</p>"),
            format!("<p>{}</p>", tr("Plugin installation failed.")),
            format!(
                "<p><a href=\"{}\" target=\"_parent\">{}</a></p>",
                ctx.admin_url("plugin-install.php"),
                tr("Go to Plugin Installer")
            ),
        ],
    };
    super::install::feedback(
        ctx,
        "plugins.php",
        "plugin-install.php",
        &tr("Plugin Installation"),
        &heading,
        &lines,
    )
    .await
}

/// `wp_ajax_install_plugin()` for a plugin of the directory; `None` for any other slug.
pub async fn ajax_install(ctx: &Ctx<'_>, slug: &str) -> Option<serde_json::Value> {
    let listing = listing(slug)?;
    let mut status = serde_json::json!({"install": "plugin", "slug": slug});
    if !ctx.can("install_plugins") {
        status["errorMessage"] = ctx
            .tr("Sorry, you are not allowed to install plugins on this site.")
            .into();
        return Some(serde_json::json!({"success": false, "data": status}));
    }
    status["pluginName"] = listing.name.into();
    if ctx.env.writer.is_none() {
        status["errorCode"] = "fs_unavailable".into();
        status["errorMessage"] = ctx.tr("Could not access filesystem.").into();
        return Some(serde_json::json!({"success": false, "data": status}));
    }
    match install(ctx, listing).await {
        Ok(_) => {
            if ctx.can("activate_plugins") {
                status["activateUrl"] = activate_url(ctx, listing.file).into();
            }
            Some(serde_json::json!({"success": true, "data": status}))
        }
        Err(reason) => {
            status["errorCode"] = "install_failed".into();
            status["errorMessage"] = reason.into();
            Some(serde_json::json!({"success": false, "data": status}))
        }
    }
}

/// `WP_Plugin_Install_List_Table::display()` for `listings`.
pub fn list_table(ctx: &Ctx<'_>, listings: &[&'static Listing]) -> String {
    let rows = if listings.is_empty() {
        format!(
            "<div class=\"no-plugin-results\">{}</div>",
            ctx.tr("No plugins found. Try a different search.")
        )
    } else {
        cards(ctx, listings)
    };
    format!(
        "<div class=\"wp-list-table widefat plugin-install\">\n\t\t<h2 class='screen-reader-text'>{}</h2>\t<div id=\"the-list\">\n\t{rows}\t</div>\n\t</div>\n",
        ctx.tr("Plugins list")
    )
}

/// `wp_ajax_search_install_plugins()`: the list for the live search of `updates.js`, which sends the term
/// `encodeURIComponent()`ed.
pub fn ajax_search(ctx: &Ctx<'_>, term: &str) -> serde_json::Value {
    if !ctx.can("install_plugins") {
        return serde_json::json!({"success": false, "data": {"errorMessage": ctx.tr("Sorry, you are not allowed to manage plugins for this site.")}});
    }
    let term = percent_decode(term);
    let listings = search(&locale(ctx), &term);
    serde_json::json!({"success": true, "data": {"count": listings.len(), "items": list_table(ctx, &listings)}})
}

/// `urldecode()` without `+`.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = text.get(i + 1..i + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `WP_Plugin_Dependencies::check_plugin_dependencies_during_ajax()`, which `updates.js` asks after an installation
/// before it turns the button into "Activate". The plugins of the directory require no other plugins; a plugin
/// with a `Requires Plugins` header is answered the same way, its dependencies are not checked.
pub fn check_dependencies(ctx: &Ctx<'_>, slug: &str) -> serde_json::Value {
    if slug.is_empty() {
        return serde_json::json!({"success": false, "data": {"slug": "", "pluginName": "", "errorCode": "no_plugin_specified", "errorMessage": ctx.tr("No plugin specified.")}});
    }
    let mut status = serde_json::json!({"slug": slug});
    let all = wp_root(ctx)
        .map(|root| plugins::installed(&root))
        .unwrap_or_default();
    // `get_plugin_dirnames()`: the first plugin file of each folder.
    let Some(plugin) = all
        .iter()
        .find(|p| p.file.split_once('/').is_some_and(|(dir, _)| dir == slug))
    else {
        status["errorCode"] = "plugin_not_installed".into();
        status["errorMessage"] = ctx.tr("The plugin is not installed.").into();
        return serde_json::json!({"success": false, "data": status});
    };
    status["pluginName"] = plugin.name.clone().into();
    status["plugin"] = plugin.file.clone().into();
    if ctx.can("activate_plugins") && !super::plugins::active_plugins(ctx).contains(&plugin.file) {
        status["activateUrl"] = activate_url(ctx, &plugin.file).into();
    }
    status["message"] = ctx.tr("The plugin has no required plugins.").into();
    serde_json::json!({"success": true, "data": status})
}

/// The listing of an installed plugin file, if the plugin directory has one.
fn listing_of_file(file: &str) -> Option<&'static Listing> {
    rustpress_core::native::catalog()
        .iter()
        .find(|l| l.file == file)
}

/// `update.php?action=upgrade-plugin` for a plugin of the directory: the page of `Plugin_Upgrader_Skin`.
pub async fn update_page(ctx: &Ctx<'_>) -> super::AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("update_plugins") {
        return super::common::die_plain(
            ctx,
            "Sorry, you are not allowed to update plugins for this site.",
        );
    }
    let file = ctx.request.get("plugin").unwrap_or_default().to_owned();
    if !ctx.verify_nonce(&format!("upgrade-plugin_{file}")) {
        return super::die(
            ctx.env,
            403,
            &tr("An error occurred."),
            &tr("The link you followed has expired."),
        );
    }
    let Some(listing) = listing_of_file(&file) else {
        return super::die(
            ctx.env,
            501,
            "WordPress &rsaquo; Error",
            &format!(
                "<p>{}</p>",
                own(&locale(ctx), "RustPress has no module for this plugin.")
            ),
        );
    };
    if ctx.env.writer.is_none() {
        return super::common::read_only(ctx);
    }
    let all = wp_root(ctx)
        .map(|root| plugins::installed(&root))
        .unwrap_or_default();
    let current = all.iter().find(|p| p.file == file);
    let lines = match current {
        Some(plugin)
            if crate::updates::version_compare(&plugin.version, listing.version)
                != std::cmp::Ordering::Less =>
        {
            vec![format!(
                "<p>{}</p>",
                tr("The plugin is at the latest version.")
            )]
        }
        _ => match update(ctx, listing).await {
            Ok(mut lines) => {
                lines.push(format!("<p>{}</p>", tr("Plugin updated successfully.")));
                let mut actions = Vec::new();
                if !super::plugins::active_plugins(ctx).contains(&file)
                    && ctx.can("activate_plugins")
                {
                    actions.push(format!(
                        "<a href=\"{}\" target=\"_parent\">{}</a>",
                        esc_url(&activate_url(ctx, &file)),
                        tr("Activate Plugin")
                    ));
                }
                actions.push(format!(
                    "<a href=\"{}\" target=\"_parent\">{}</a>",
                    ctx.admin_url("plugins.php"),
                    tr("Go to Plugins page")
                ));
                lines.push(format!("<p>{}</p>", actions.join(" | ")));
                lines
            }
            Err(reason) => vec![
                format!("<p>{reason}</p>"),
                format!("<p>{}</p>", tr("Plugin update failed.")),
            ],
        },
    };
    super::install::feedback(
        ctx,
        "plugins.php",
        "plugins.php",
        &tr("Update Plugin"),
        &tr("Update Plugin"),
        &lines,
    )
    .await
}

/// `wp_ajax_update_plugin()` for a plugin of the directory; `None` for any other plugin.
pub async fn ajax_update(ctx: &Ctx<'_>, file: &str, slug: &str) -> Option<serde_json::Value> {
    let listing = listing_of_file(file)?;
    let failed =
        |status: serde_json::Value| Some(serde_json::json!({"success": false, "data": status}));
    let mut status =
        serde_json::json!({"update": "plugin", "slug": slug, "oldVersion": "", "newVersion": ""});
    if !ctx.can("update_plugins") {
        status["errorMessage"] = ctx
            .tr("Sorry, you are not allowed to update plugins for this site.")
            .into();
        return failed(status);
    }
    let all = wp_root(ctx)
        .map(|root| plugins::installed(&root))
        .unwrap_or_default();
    let plugin = all.iter().find(|p| p.file == file);
    status["plugin"] = file.into();
    status["pluginName"] = plugin.map(|p| p.name.clone()).unwrap_or_default().into();
    if let Some(plugin) = plugin.filter(|p| !p.version.is_empty()) {
        status["oldVersion"] =
            rustpress_core::i18n::sprintf(&ctx.tr("Version %s"), &[&plugin.version]).into();
    }
    if plugin.is_some_and(|p| {
        crate::updates::version_compare(&p.version, listing.version) != std::cmp::Ordering::Less
    }) {
        status["errorMessage"] = ctx.tr("The plugin is at the latest version.").into();
        return failed(status);
    }
    if ctx.env.writer.is_none() {
        status["errorCode"] = "fs_unavailable".into();
        status["errorMessage"] = ctx.tr("Could not access filesystem.").into();
        return failed(status);
    }
    match update(ctx, listing).await {
        Ok(_) => {
            status["newVersion"] =
                rustpress_core::i18n::sprintf(&ctx.tr("Version %s"), &[listing.version]).into();
            Some(serde_json::json!({"success": true, "data": status}))
        }
        Err(reason) => {
            status["errorCode"] = "update_failed".into();
            status["errorMessage"] = reason.into();
            failed(status)
        }
    }
}

/// `update-core.php?action=do-plugin-upgrade` (after the capability and nonce checks): the update of the checked
/// plugins. WordPress loads `update.php?action=update-selected` into an iframe that prints `Bulk_Plugin_Upgrader_Skin`;
/// RustPress prints the same steps in the page. Only plugins of the plugin directory can be updated; for the others
/// the package is not available.
pub async fn bulk_update_page(ctx: &Ctx<'_>) -> super::AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if ctx.env.writer.is_none() {
        return super::common::read_only(ctx);
    }
    let files: Vec<String> = match ctx.request.get("plugins") {
        Some(list) => list.split(',').map(str::to_owned).collect(),
        None => ctx
            .request
            .form
            .iter()
            .filter(|(k, _)| k == "checked" || k.starts_with("checked["))
            .map(|(_, v)| v.clone())
            .collect(),
    };
    let all = wp_root(ctx)
        .map(|root| plugins::installed(&root))
        .unwrap_or_default();
    let mut lines = vec![format!(
        "<p>{}</p>",
        tr(
            "The update process is starting. This process may take a while on some hosts, so please be patient."
        )
    )];
    for (index, file) in files.iter().enumerate() {
        let current = all.iter().find(|p| &p.file == file);
        let name = current.map_or_else(|| file.clone(), |p| p.name.clone());
        lines.push(format!(
            "<h2>{}</h2>",
            rustpress_core::i18n::sprintf(
                &tr("Updating Plugin %1$s (%2$d/%3$d)"),
                &[
                    &esc_html(&name),
                    &(index + 1).to_string(),
                    &files.len().to_string()
                ]
            )
        ));
        let outcome = match listing_of_file(file) {
            None => Err(tr("Update package not available.")),
            Some(listing)
                if current.is_some_and(|p| {
                    crate::updates::version_compare(&p.version, listing.version)
                        != std::cmp::Ordering::Less
                }) =>
            {
                Err(tr("The plugin is at the latest version."))
            }
            Some(listing) => update(ctx, listing).await,
        };
        match outcome {
            Ok(steps) => {
                lines.extend(steps);
                lines.push(format!(
                    "<div class=\"updated inline\"><p>{}</p></div>",
                    rustpress_core::i18n::sprintf(
                        &tr("%s updated successfully."),
                        &[&format!("<strong>{}</strong>", esc_html(&name))]
                    )
                ));
            }
            Err(reason) => lines.push(format!(
                "<div class=\"error inline\"><p>{}</p></div>",
                rustpress_core::i18n::sprintf(
                    &tr("An error occurred while updating %1$s: %2$s"),
                    &[&format!("<strong>{}</strong>", esc_html(&name)), &reason]
                )
            )),
        }
    }
    lines.push(format!("<p>{}</p>", tr("All updates have been completed.")));
    lines.push(format!(
        "<p><a href=\"{}\" target=\"_parent\">{}</a> | <a href=\"{}\" target=\"_parent\">{}</a></p>",
        ctx.admin_url("plugins.php"),
        tr("Go to Plugins page"),
        ctx.admin_url("update-core.php"),
        tr("Go to WordPress Updates page")
    ));
    super::install::feedback(
        ctx,
        "index.php",
        "update-core.php",
        &tr("Update Plugins"),
        &tr("Update Plugins"),
        &lines,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_texts_are_german_for_german_locales() {
        assert_eq!(own("de_AT", "RustPress Plugins"), "RustPress-Plugins");
        assert_eq!(own("en_US", "RustPress Plugins"), "RustPress Plugins");
        assert_eq!(own("de_DE", "anything else"), "anything else");
    }

    #[test]
    fn the_live_search_term_is_decoded() {
        assert_eq!(percent_decode("svg%20support"), "svg support");
        assert_eq!(percent_decode("%C3%BC%2"), "ü%2");
    }
}
