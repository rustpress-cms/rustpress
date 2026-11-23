//! The plugin directory of RustPress (`plugin-install.php`): the plugins with a native module, with the archive of
//! the release each module follows. WordPress.org's directory lists plugins that cannot run without PHP, so RustPress
//! shows these instead; the listings are built in, so that the directory works without a connection to anything.
//!
//! Name, author and short description are those of the plugin's page on WordPress.org (`plugins_api()`), the German
//! descriptions those of de.wordpress.org where the plugin is translated. WP Date Remover was closed on WordPress.org
//! in 2022; its archive without a version is still the release 1.1.3. A new release of a module changes its listing
//! here: `dev/plugin-catalog.sh` prints the package lines.

use rustpress_core::native::Listing;

/// The listings, in the order of their slugs.
pub static CATALOG: &[Listing] = &[
    Listing {
        slug: "advanced-custom-fields",
        file: "advanced-custom-fields/acf.php",
        version: "6.8.10",
        name: "Advanced Custom Fields (ACF®)",
        author: "WP Engine",
        author_profile: "https://profiles.wordpress.org/wpengine/",
        description: "ACF helps customize WordPress with powerful, professional and intuitive fields. Proudly powering over 2 million sites, WordPress developers love ACF.",
        translations: &[(
            "de_DE",
            "ACF erweitert WordPress um leistungsstarke, professionelle und intuitive Felder. Über 2 Millionen Websites nutzen es, WordPress-Entwickler lieben ACF.",
        )],
        package: "https://downloads.wordpress.org/plugin/advanced-custom-fields.6.8.10.zip",
        sha256: "92d51a8f1d839f60aee4efb6a9cd269b37d1599a9fb44d03f792352dfe34c744",
        size: 9488938,
    },
    Listing {
        slug: "classic-editor",
        file: "classic-editor/classic-editor.php",
        version: "1.7.0",
        name: "Classic Editor",
        author: "WordPress.org",
        author_profile: "https://profiles.wordpress.org/wordpressdotorg/",
        description: "Enables the previous &quot;classic&quot; editor and the old-style Edit Post screen with TinyMCE, Meta Boxes, etc. Supports all plugins that extend this screen.",
        translations: &[(
            "de_DE",
            "Aktiviert den früheren &quot;klassischen&quot; WordPress-Editor sowie die Bearbeiten-Ansicht im alten Stil mit TinyMCE, Metaboxen usw. Unterstützt außerdem alle Plugins, die diese Ansicht erweitern.",
        )],
        package: "https://downloads.wordpress.org/plugin/classic-editor.1.7.0.zip",
        sha256: "7cc7799b0b7d820fbb7528dbff8027c736e52513992416cb37279f44d3d9dd77",
        size: 38882,
    },
    Listing {
        slug: "complianz-gdpr",
        file: "complianz-gdpr/complianz-gpdr.php",
        version: "7.5.6",
        name: "Complianz GDPR/CCPA Cookie Consent Banner",
        author: "Complianz",
        author_profile: "https://profiles.wordpress.org/complianz/",
        description: "Publish your cookie banner or notice in minutes. Automatically scan cookies, and create policies to align with GDPR, CCPA, and Google Consent Mode.",
        translations: &[(
            "de_DE",
            "Veröffentliche dein Cookie-Banner oder deinen Hinweis in wenigen Minuten. Überprüfe Cookies automatisch und erstelle Richtlinien, die den Anforderungen der DSGVO, des CCPA und des Google-Einwilligungsmodus entsprechen.",
        )],
        package: "https://downloads.wordpress.org/plugin/complianz-gdpr.7.5.6.zip",
        sha256: "f2124fe848cbd348cca56ad7a506872e4d0cb9855bb441c0eeab39441cb85072",
        size: 7941470,
    },
    Listing {
        slug: "contact-form-7",
        file: "contact-form-7/wp-contact-form-7.php",
        version: "6.2.4.1",
        name: "Contact Form 7",
        author: "Rock Lobster Inc.",
        author_profile: "https://profiles.wordpress.org/rocklobsterinc/",
        description: "Just another contact form plugin. Simple but flexible.",
        translations: &[(
            "de_DE",
            "Ein weiteres Kontaktformular-Plugin. Einfach, aber flexibel.",
        )],
        package: "https://downloads.wordpress.org/plugin/contact-form-7.6.2.4.1.zip",
        sha256: "9b28e8bccfea0b0dcc8fd27ba8745fc7133fdacf594bf2e1faee869a15f71f64",
        size: 414335,
    },
    Listing {
        slug: "disable-comments",
        file: "disable-comments/disable-comments.php",
        version: "2.9.1",
        name: "Disable Comments",
        author: "WPDeveloper",
        author_profile: "https://profiles.wordpress.org/wpdevteam/",
        description: "Disable comments site-wide or by post type, delete existing comments in bulk, and block comment spam via REST API and XML-RPC. Multisite ready.",
        translations: &[(
            "de_DE",
            "Deaktiviere Kommentare auf der ganzen Website oder je Inhaltstyp, lösche vorhandene Kommentare gesammelt und blockiere Kommentar-Spam über REST-API und XML-RPC. Multisite-fähig.",
        )],
        package: "https://downloads.wordpress.org/plugin/disable-comments.2.9.1.zip",
        sha256: "bc9e476f53921404b35207f1661269d57f16978854afb8b7eb403cdcf7193102",
        size: 455187,
    },
    Listing {
        slug: "google-site-kit",
        file: "google-site-kit/google-site-kit.php",
        version: "1.191.0",
        name: "Site Kit by Google &#8211; Analytics, Search Console, AdSense, Speed",
        author: "Google",
        author_profile: "https://profiles.wordpress.org/google/",
        description: "Site Kit is a one-stop solution for WordPress users to use everything Google has to offer to make them successful on the web.",
        translations: &[(
            "de_DE",
            "Site Kit ist eine Komplettlösung für WordPress-Anwender, mit der du alle Funktionen von Google nutzen kannst, um im Web erfolgreich zu sein.",
        )],
        package: "https://downloads.wordpress.org/plugin/google-site-kit.1.191.0.zip",
        sha256: "5b9f2c93bd66cf2dfbca5255ad0e8462226baa5326d42baa92847c64262f1992",
        size: 7505526,
    },
    Listing {
        slug: "insert-headers-and-footers",
        file: "insert-headers-and-footers/ihaf.php",
        version: "2.4.1",
        name: "WPCode &#8211; Insert Headers and Footers + Custom Code Snippets &#8211; WordPress Code Manager",
        author: "Syed Balkhi",
        author_profile: "https://profiles.wordpress.org/smub/",
        description: "Easily add code snippets in WordPress. Insert header &amp; footer scripts, add PHP code snippets with conditional logic, insert ads pixel code, and more.",
        translations: &[(
            "de_DE",
            "Bette ganz einfach Code-Snippets in WordPress ein. Füge Header- &amp; Footer-Skripte ein, füge PHP-Code-Snippets mit bedingter Logik hinzu, füge Code für Anzeigen-Pixel ein und vieles mehr.",
        )],
        package: "https://downloads.wordpress.org/plugin/insert-headers-and-footers.2.4.1.zip",
        sha256: "492c800246ec0d8d29a935e0af74e45f8f322777fd8e17c9fec1dd3cf806cf02",
        size: 1512651,
    },
    Listing {
        slug: "redirection",
        file: "redirection/redirection.php",
        version: "5.10.1",
        name: "Redirection",
        author: "John Godley",
        author_profile: "https://profiles.wordpress.org/johnny5/",
        description: "Manage 301 redirects, track 404 errors, and improve your site. No knowledge of Apache or Nginx required.",
        translations: &[(
            "de_DE",
            "Verwalte 301-Weiterleitungen, behalte den Überblick über 404-Fehler und verbessere deine Website, ohne dass du Kenntnisse über Apache oder NGINX benötigst.",
        )],
        package: "https://downloads.wordpress.org/plugin/redirection.5.10.1.zip",
        sha256: "f68a4be447ecba00c64e8a39d7517807ed49cc18e77b7a364c20cb31848235b7",
        size: 502191,
    },
    Listing {
        slug: "svg-support",
        file: "svg-support/svg-support.php",
        version: "2.6.1",
        name: "SVG Support",
        author: "Benbodhi",
        author_profile: "https://profiles.wordpress.org/benbodhi/",
        description: "Securely upload SVG files to your media library, with built-in sanitization and advanced features for styling and animation.",
        translations: &[(
            "de_DE",
            "Lade SVG-Dateien sicher in deine Mediathek hoch, mit eingebauter Bereinigung und erweiterten Funktionen für Gestaltung und Animation.",
        )],
        package: "https://downloads.wordpress.org/plugin/svg-support.2.6.1.zip",
        sha256: "76e670b7bca1d317ae5b11d7ef4c269dbb4b54b0e1c5ce248f392f17d5af7e5b",
        size: 255673,
    },
    Listing {
        slug: "wordpress-seo",
        file: "wordpress-seo/wp-seo.php",
        version: "28.8",
        name: "Yoast SEO &#8211; Advanced SEO with real-time guidance and built-in AI",
        author: "Yoast",
        author_profile: "https://profiles.wordpress.org/yoast/",
        description: "Real-time SEO guidance, schema, and AI built in. Help search engines and AI systems understand your content. All AI tools included, no hidden fees.",
        translations: &[(
            "de_DE",
            "Eingebaute Echtzeit-SEO-Anleitungen, Schema und KI. Hilf Suchmaschinen und KI-Systemen dabei, deine Inhalte zu verstehen. Alle KI-Tools sind enthalten, keine versteckten Gebühren.",
        )],
        package: "https://downloads.wordpress.org/plugin/wordpress-seo.28.8.zip",
        sha256: "27cc01cb149db7186ef750552a5cc2ae5028933473886e419918af273b075105",
        size: 5154170,
    },
    Listing {
        slug: "wp-date-remover",
        file: "wp-date-remover/wp-date-remover.php",
        version: "1.1.3",
        name: "WP Date Remover",
        author: "Selma Mariudottir",
        author_profile: "",
        description: "WP Date Remover allows you to quickly and easily remove the time and date from specific post categories.",
        translations: &[(
            "de_DE",
            "WP Date Remover entfernt schnell und einfach Datum und Uhrzeit aus den Beiträgen bestimmter Kategorien.",
        )],
        package: "https://downloads.wordpress.org/plugin/wp-date-remover.zip",
        sha256: "ee081290e990c072cecd0d5bc666e53c893bce57f074d2d47e9d766052370ba0",
        size: 340603,
    },
    Listing {
        slug: "wp-mail-smtp",
        file: "wp-mail-smtp/wp_mail_smtp.php",
        version: "4.10.4",
        name: "WP Mail SMTP by WPForms &#8211; SMTP for Reliable Email Delivery with Gmail, Outlook, Amazon SES, Brevo, Other SMTP &amp; More",
        author: "Syed Balkhi",
        author_profile: "https://profiles.wordpress.org/smub/",
        description: "Make email delivery easy for WordPress. Connect with SMTP, Gmail, Brevo, SendGrid, Amazon SES + more, with powerful WordPress SMTP Email plugin.",
        translations: &[(
            "de_DE",
            "Mach den E-Mail-Versand mit WordPress ganz einfach. Verbinde dich über SMTP, Gmail, Brevo, SendGrid, Amazon SES und mehr – mit einem leistungsstarken WordPress-SMTP-E-Mail-Plugin.",
        )],
        package: "https://downloads.wordpress.org/plugin/wp-mail-smtp.4.10.4.zip",
        sha256: "cd4f774e838280480d5bff6647294132f235f4b9d31f045f8a682edf7832c8de",
        size: 4413917,
    },
    Listing {
        slug: "wpforms-lite",
        file: "wpforms-lite/wpforms.php",
        version: "2.0.4",
        name: "WPForms &#8211; AI Form Builder for WordPress &#8211; Contact Forms, Payment Forms, Survey Form, Quiz &amp; More",
        author: "Syed Balkhi",
        author_profile: "https://profiles.wordpress.org/smub/",
        description: "The best WordPress contact form plugin. Drag &amp; Drop form builder to create beautiful contact forms, payment forms, &amp; other custom forms.",
        translations: &[(
            "de_DE",
            "Das beste WordPress Plugin für Kontaktformulare. Mit dem Drag-and-drop-Formular-Builder kannst du ansprechende Kontaktformulare, Zahlungsformulare, &amp; und andere individuelle Formulare erstellen.",
        )],
        package: "https://downloads.wordpress.org/plugin/wpforms-lite.2.0.4.zip",
        sha256: "0417412b098809eb8bf439b4c4d3419bc3e3343b199ba5d9a5dfd436a2415f7a",
        size: 13858658,
    },
];

#[cfg(test)]
mod tests {
    use super::CATALOG;

    /// Every module has a listing of exactly its release, and every listing a module.
    #[test]
    fn listings_follow_the_modules() {
        crate::register();
        let modules = rustpress_core::native::registered();
        assert_eq!(modules.len(), CATALOG.len());
        for module in modules {
            let listing = CATALOG
                .iter()
                .find(|l| l.file == module.file())
                .unwrap_or_else(|| panic!("no listing for {}", module.file()));
            assert_eq!(listing.version, module.version(), "{}", listing.slug);
            assert!(listing.file.starts_with(&format!("{}/", listing.slug)));
            assert!(
                listing
                    .package
                    .ends_with(&format!(".{}.zip", listing.version))
                    || listing.slug == "wp-date-remover",
                "{}",
                listing.slug
            );
            assert_eq!(listing.sha256.len(), 64);
        }
        let slugs: Vec<&str> = CATALOG.iter().map(|l| l.slug).collect();
        let mut sorted = slugs.clone();
        sorted.sort_unstable();
        assert_eq!(slugs, sorted);
    }
}
