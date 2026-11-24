# Plan: RustPress, a drop-in replacement for WordPress without PHP

This file holds the plan, the state of every area and what is still open (as of 2026-11-23).
[HISTORY.md](HISTORY.md) keeps the entry of every step as it was written, oldest first: what was ported, how it was
compared and what deliberately differs.

## Goal and decisions

- **Goal:** an existing WordPress site runs without PHP and without converting anything. The Rust binary reads the
  original MySQL/MariaDB schema (`wp_*`) and the existing `wp-content/` and answers byte for byte like the WordPress
  release in `WORDPRESS_VERSION`: HTML, headers, feeds, REST JSON, redirects and the rows it stores.
- **WordPress is the oracle.** Everything is ported from the PHP source and compared with a real WordPress (golden
  tests, compare scripts in `dev/`, `rustpress-compat`). A difference that cannot be avoided is recorded with its reason
  (`compat/known.toml` for the compat runs, this file for the rest).
- **Themes:** block themes (`theme.json` + HTML templates). Twenty Seventeen, Twenty Twenty and Twenty Twenty-One are
  the classic themes, rebuilt natively (decisions of 2026-10-25, 2026-11-08 and 2026-11-13). Other classic PHP themes do not run.
- **Plugins:** PHP never runs. Important plugins are rebuilt as native modules (`crates/rustpress-plugins`) that take
  over while the plugin is in `active_plugins`; the plugin's files stay for assets, headers and language packs.
- **Writing is an explicit switch** (`RUSTPRESS_ALLOW_WRITES=1` / `allow_writes`): without it RustPress is a read-only
  replacement; with it the administration, sign-in, comments and the REST write API work.
- **Branding (2026-11-23):** the administration, the sign-in and the toolbar name RustPress and show its logo, so
  that nobody takes a RustPress site for WordPress and no WordPress trademark is used for RustPress
  (`rustpress_core::brand`, the switch `branding`). Everything else stays byte-identical; the comparisons of the
  administration run with `branding = false` (`RUSTPRESS_BRANDING=0`).
- **Plugin directory:** Add Plugins lists the plugins with a native module (built into the binary) instead of
  WordPress.org's directory, and installs the archive of the release a module follows, checked against its SHA-256
  (decision of 2026-11-23).
- **Not planned:** a PHP interpreter, Multisite, outgoing HTTP to WordPress.org or other services (updates, the theme
  directory and the other plugins of the plugin directory, remote oEmbed), caching and security plugins (architecture), WooCommerce, Elementor.

## Architecture

| Crate | Purpose |
|---|---|
| `rustpress-phpser` | PHP `serialize`/`unserialize`, byte-exact |
| `rustpress-db` | access to the WordPress schema (sqlx), the read pool and the `Writer` |
| `rustpress-core` | options, rewrite rules and their generation, main query, template hierarchy, canonical redirects, sitemaps, kses, the native plugin framework (`native.rs`) |
| `rustpress-blocks` | block parser and renderer, block supports, style engine, dynamic blocks, Interactivity |
| `rustpress-theme` | `theme.json`, templates, global styles, documents and feeds, the script loader, Twenty Seventeen, Twenty Twenty and Twenty Twenty-One |
| `rustpress-content` | writing posts, terms, users, comments and uploads |
| `rustpress-auth` | passwords, cookies, sessions, nonces, capabilities (`map_meta_cap`), application passwords |
| `rustpress-admin` | `wp-admin`, sign-in and the installer |
| `rustpress-rest` | the REST API, read and write |
| `rustpress-mail` | e-mail (`wp_mail()` over SMTP) |
| `rustpress-plugins` | the native plugin modules |
| `rustpress-server` | the axum server and the CGI mode |
| `rustpress-compat` | the comparison run against a real WordPress |

The reference stack, the tools and the working rules are in [AGENTS.md](AGENTS.md).

## State by area

- **Website** – equal to WordPress for the bundled block themes Twenty Twenty-Five, -Four, -Three and -Two (about 780
  URLs each: pages, patterns, feeds, sitemaps, REST, files; 0 unexpected differences), across eight permalink
  structures and a matrix of settings, and in de_DE, ru_RU, ar and ja. Twenty Seventeen on a copy of the maintainer's real site,
  0 unexpected differences (2026-11-06): 322 URLs with its plain permalinks, all 105 HTML pages identical; 539 URLs with
  `/%postname%/`. Twenty Twenty-One on the reference, 0 unexpected differences in 778 URLs (2026-11-08); Twenty Twenty likewise (2026-11-13). Covered: the main query and template hierarchy, canonical redirects, custom
  post types and taxonomies (from the stored rules and the literal `register_*()` calls of plugins and themes),
  shortcodes, embeds/oEmbed, the dynamic blocks including Interactivity, the comment form and submitting comments,
  previews and private content for signed-in users, the admin bar.
- **REST API** – read and write: posts, pages, media, terms, users, comments and notes, settings, templates and the site
  editor's routes, widgets and `batch/v1`, block types, fonts, application passwords, Site Health, abilities, the full index;
  `OPTIONS` with the schemas. Compared with `dev/rest-sweep.py` and the write compare scripts.
- **Sign-in and users** – `wp-login.php` complete (sign-in/out, lost password, reset, registration, admin e-mail check,
  `postpass`, interim login), sessions, nonces and capabilities as in WordPress, application passwords, e-mail.
- **Administration** – every screen of a block-theme site byte-identical except environment and randomness
  (`dev/raw-sweep.py`, 12 known pages): dashboard, lists with bulk and quick edit, the classic and the block editor,
  the site editor, media with the image editor, comments, users, settings including permalinks, themes and plugins
  (also installing from a ZIP), tools, privacy, Site Health, the file editors, revisions, Screen Options and help.
- **Languages** – the language packs of WordPress, themes and plugins are the only source; plural forms, dates,
  texturize characters and right-to-left work for every language (checked in four). `admin-<locale>.mo` in the
  administration, JS translations for plugin scripts.
- **Native plugins** – the framework in `rustpress-core/src/native.rs` (hooks for the website, the administration, REST,
  saving and rewriting) and these modules, each following one plugin release:
  - WP Date Remover 1.1.3 – complete.
  - SVG Support 2.6.1 – website, settings, sanitized SVG uploads.
  - Contact Form 7 6.2.4.1 – forms, submitting with mail, the administration.
  - Yoast SEO 28.8 – the head, schema, sitemaps, breadcrumbs, REST, feeds.
  - Redirection 5.10.1 – the website (redirects, 404s, logs with write access).
  - Advanced Custom Fields 6.8.10 – post types and taxonomies, values in REST and the shortcode, fields on every edit
    screen and in the comment form with saving, the lists, the editors of field groups, post types and taxonomies with
    their Ajax, the tools (export as JSON and PHP, import), the options pages preview.
  - Classic Editor 1.7.0 – complete except the frontend toolbar link (see HISTORY.md).
  - Complianz – GDPR/CCPA Cookie Consent 7.5.6 – stage 1, the website: banner, statistics, cookie blocker, REST,
    the consent shortcodes; stage 2, the legal documents: the cookie policies of every region, the cookie list, the
    do-not-sell form; stage 3: the consent area and the REST route `documents`. Sending data requests is open (see
    HISTORY.md).
  - Site Kit by Google 1.191.0 – the tags on the website (Analytics with consent mode, site verification), not the dashboard.
  - WPCode Lite 2.4.1 – the global header, body and footer scripts and the site-wide snippets with their conditions, not PHP.
  - WPForms Lite 2.0.4 – forms on the website (shortcode, block), submitting with validation, confirmations and the
    notification e-mails, also on read-only sites; not the administration (forms are made in WordPress).
  - Disable Comments 2.9.1 – comments and pings off everywhere, per post type or by rule, on the website and in the
    REST API; not the administration.
  - WP Mail SMTP 4.10.4 – the SMTP server and the sender of its settings for every `wp_mail()`; not the web API
    mailers, the log or the administration.
- **Following WordPress** – `WORDPRESS_VERSION` is the target; a daily workflow opens an issue per release with the
  report of `dev/wp-diff.sh`; `dev/wp-bump.sh` moves the reference. 7.1.5 was followed (issue #90).
- **Review** – the security and robustness findings of the review of 2026-10-26 (A1, A2) are fixed except S12; the
  performance work (A3) is done except the rest of P14/P15. RustPress answers most pages faster than the reference.
- **Packaging** – release tags `v*` build every package and container image; the server also runs as CGI on shared
  hosting (see the README).

## Open

**Website**
- `core/rss` (network).
- Block hooks (`hooked_block_types`): no module registers hooked blocks yet, so `apply_block_hooks_to_content()` only
  normalizes.
- Twenty Seventeen: a page with `<!--more-->` in the audio and video formats is compared without it; `$content_width` for
  embeds (they need the network) and the `edit` context of the administration. More classic themes.
- The `functions.php` of themes: only literal calls are read.
- More third-party block themes. Frost 1.1 and Ollie 1.6.6 are equal (compat runs of 2026-11-05 against WordPress
  7.1.4) except the environment differences Twenty Twenty-Five shows as well (posts with the same date). A block
  theme whose `functions.php` does more than the literal calls gets a port in `rustpress-theme/src/block_themes/`
  (Ollie: its stylesheet handle, the `glob()` loop of `wp_enqueue_block_style()`, a `wp_head` style, a
  `render_block_core/navigation` filter, the template part area `sidebar`).
  Extendable 2.2.0 and Bluehost Blueprint 1.0.2 (the third-party block themes with the most active installs on
  wordpress.org after Frost/Ollie were chosen, 2026-11-09) are equal as well: Extendable 782 URLs, 760 identical,
  22 known; Bluehost Blueprint 781 URLs, 758 identical, 21 known, and 2 archive pages with posts of the same date
  (the environment difference above). Their ports: Extendable's footer scripts `header.js` and
  `navigation-customization.js` with `ExtendableNavData`; Bluehost Blueprint's stylesheet handle, the
  `assets/block-styles/` loop (with the `.min.css` files) and the YITH Wonder back-compat stylesheet. Not ported, as
  they need a plugin, a signed-in viewer or the editor: Extendable's WooCommerce/Contact Form 7/WPForms styles and
  filters, the duotone of Extendify demo logos, the animations of `inc/animations.php` (signed-in viewers or the
  option `extendify_animation_settings`), its pattern categories and REST filters; the YITH Wonder back-compat style
  reads the theme mods only when they are autoloaded. Found on the way (all block themes): `PROPERTIES_METADATA` of
  7.1, the elements `textInput`/`select`, block preset classes, the order of feature selector rules, `@font-face`
  properties, `_wp_normalize_relative_css_links()` for inlined stylesheets, the alternative `if` syntax in patterns,
  `home_url( '/' )`, `?paged=0` in custom query pagination.
- Caching of rendered pages.

**Languages**
- A language per request (the user's language, Polylang/WPML): an `I18nRegistry` from which each request gets its
  translator; all calls already go through `Site::tr`.
- JavaScript translations on the front end (Interactivity texts), hot reload of language packs, variants such as
  `de_DE_formal`, a command `rustpress i18n install <locale>`.
- Screens of a translated site: `dev/nav-menus-compare.py` still compares from the `body` on.
- `users.php?action=delete` still shows the confirmation form of older releases (one choice for all users, not one per
  user with `%1$s (ID #%2$s): ...`); its three texts are listed in `OWN_MESSAGES` of `i18n_coverage.rs` until it is ported.
- Not equal on the copy of the real site: `credits.php` (what `wp_credits()` gets from api.wordpress.org depends on the
  language, the bake is the English answer), `plugin-install.php` (the plugin directory of RustPress in place of api.wordpress.org), the tests of
  `site-health.php` that are about PHP (RustPress has none) and the `Info` tab, which reports its own environment.

**Sign-in, administration, REST**
- The Customizer beyond its page and preview: saving (`customize_save` with the `customize_changeset` posts, drafts,
  schedules, autosaves and locks), its other Ajax actions (`customize_load_themes`, `customize_refresh_nonces`,
  `customize_trash`, `customize_dismiss_autosave_or_lock`, `customize_override_changeset_lock`, the header control's
  `custom-header-crop`, `-add` and `-remove`, the menus' `load-available-menu-items-customizer`,
  `search-available-menu-items-customizer` and `customize-nav-menus-insert-auto-draft`, `update-widget`), and the
  arguments `changeset_uuid` and `theme` of `customize.php`.
- The preview of the Customizer (see "The preview of the Customizer"), what it does not do:
  - A block theme in the preview of a site whose active theme is classic. The widgets of the theme switch
    (`override_sidebars_widgets_for_theme_switch()`, `old_sidebars_widgets_data`) and the panels of the menus and widgets
    differ in the exports. A classic theme other than Twenty Seventeen is not rendered at all.
  - The pages the menus create in the Customizer (`nav_menus_created_posts`, `make_auto_draft_status_previewable()`).
  - The headers of the preview on what a native plugin answers by itself.
- Menus: a location whose menu is gone. WordPress prints the fallback `wp_page_menu()`; RustPress prints no navigation.
- Widgets: what the RSS widget prints (front and `rendered`; it needs `fetch_feed()`), the video widget with an address
  that `wp_oembed_get()` would ask a provider or the site itself about (RustPress prints nothing, as WordPress does for an
  address without a provider), the feed check of `wp_widget_rss_process()`, the old format of `wp_convert_widget_settings()`, `alt_option_name`,
  `wp-compression-test` (the page prints the script while `can_compress_scripts` is not stored, nothing answers it).
- The preview of a widget (`widget-types/<id>/render`): its head lacks the styles of all blocks, which `wp_head()` prints when
  no page was rendered before it. `dev/widgets-compare.py` compares it from the `body` on.
- `parse-media-shortcode` with a preview (the scripts of `wp-mediaelement`/`wp-playlist`).
- `wp/v2/block-patterns/patterns`: four remote patterns keep a second space before `/>` that the reference does not have
  (cause not looked at; seen in `dev/rest-sweep.py`).
- The dashboard and the block editor are still slower than the reference.

**Native plugins**
- Complianz, next: sending data requests (the REST route `datarequests` with its rate limits in transients, the
  table `cmplz_dnsmpd` and the mails by the cron event `cmplz_process_datarequest_mail`; needs writes from plugin REST
  routes and plugin hooks in the cron runner), the custom document CSS (`inline_styles()` on `wp_head` at priority
  100, off by default), documents unlinked in the classic editor (`revert_divs_to_summary()`). After that, maybe: A/B
  tests and records of consent (Premium only), the integrations with other plugins.
- Contact Form 7: the configuration check (`WPCF7_ConfigValidator`), setting up the services, `action=validate`, the
  REST routes to read, create, change and delete forms, the block `contact-form-7/contact-form-selector`.
- Yoast SEO: FAQ/HowTo blocks in the schema, the admin bar, signed-in viewers, the administration.
- Redirection: the administration (the React interface and REST `redirection/v1`), monitoring changed permalinks,
  import/export.
- ACF: block bindings `acf/field` (waiting for block bindings in the core), saving values through REST, the labels of
  chosen posts, terms and users in conditional logic (`acf/conditional_logic/choices`), widgets of plugins in the
  `widget` rule, `not_registered` (a key another plugin registered first), the import from Custom Post Type UI, field
  groups from local JSON or PHP.
- Site Kit: the dashboard, the setup with Google and its REST answers; the tags of Tag Manager, AdSense, Ads, Reader
  Revenue Manager and Sign in with Google; the Google tag gateway, conversion tracking and custom dimensions.
- WPCode: the locations in and around the content, excerpts and posts, the toolbar menu "WPCode" for administrators,
  the administration.
- WPForms Lite: the form builder and the rest of the administration; CAPTCHAs (reCAPTCHA, hCaptcha, Turnstile),
  Akismet, the legacy e-mail template, theme overrides of the e-mail templates, sending without JavaScript (see HISTORY.md).
- WP Mail SMTP: the mailers of web APIs (SendGrid, Mailgun, Brevo, Gmail, Outlook, ...), Pepipost, the email log
  and the reports, the administration (settings, setup wizard, test e-mail).
- Disable Comments: the settings and tools pages, the changes of the administration and the editor, the Abilities API,
  the counter of blocked attempts, the blocklist of comment types, WooCommerce reviews, network settings.
- Next: Polylang (needs the language per request). Following plugin releases (`PLUGIN_VERSIONS` and the diff report).

**Review and tests**
- S12: the CSS of global styles is stored unfiltered (only relevant with custom roles without `unfiltered_html`).
- The rest of P14/P15 (synchronous file access in the administration).
- Phase 4 of the review: merging duplicates, removing dead code, splitting long functions; a test suite for the database
  schema; the small deviations listed in A4.
- The role matrix (`crates/rustpress-server/tests/roles.rs`) does not send writes with a valid nonce to the screens of
  the administration and to `admin-ajax.php` yet, and leaves out plugins, media uploads and custom post types.
- `wpautop()` is slower than linear on very large input (5 MB of text: 14 s; 300,000 `<div>` in a row: 25 s in a
  release build). Posts of a realistic size are not affected (1 MB: 0.5 s); see "Hardening after the second review".
- `rustpress-auth/tests/oracle.rs` depends on session tokens in the shared database (create the fixture again first).
- Compat: more third-party block themes (see Website), a run with the release build.

## Recorded differences

Differences of the compat runs are in [`compat/known.toml`](compat/known.toml). Everything else:

- **Plugin directory:** the browse tabs of `plugin-install.php` list the plugins with a native module
  (`rustpress_plugins::catalog`) in the cards of WordPress, without ratings, installation counts and the details
  modal; "Install Now" (`install-plugin` in `admin-ajax.php` and `update.php`) downloads only the archive of a listing
  and only installs it if its SHA-256 matches. The only outgoing request of the administration.
- **Environment:** RustPress makes no other outgoing requests: the directories and updates of WordPress.org,
  `credits.php`, font collections, remote oEmbed, loopback and the Site Health tests that need them, Akismet and the
  services of Contact Form 7 (reCAPTCHA, Turnstile, Stripe), the e-mail opt-in of ACF. Without `RUSTPRESS_SMTP_URL`
  mail goes to the host's `sendmail` as PHP's `mail()` sends it, and fails as `wp_mail()` does on a host without
  `sendmail` (the reference container has none).
- **The container and the database:** the order of templates and of `readdir()` in the file editors, rows with the same
  sort value (`orderby=comment_count`, posts with the same date), random values (nonces, `antispambot()`, passwords,
  the quiz question of Contact Form 7).
- **Branding:** with `branding` on (the default) the texts of WordPress' own text domain in the administration, the
  sign-in and the toolbar say RustPress where they name the software (English and every translation), the links to
  wordpress.org's home, support forums, feedback forum and documentation lead to RustPress, the logo menu leads to
  RustPress' site and repository, and the logo is RustPress'. WordPress.org, its directories, the community, events
  and news, versions of WordPress (the release whose files RustPress uses), the documentation articles of the
  individual screens (decision of 2026-11-23), plugins, themes and the content stay as they are. E-mails sent from a
  request of the administration follow the same rule. Pages baked from WordPress (`about.php`, `credits.php`,
  `freedoms.php`, `contribute.php`) and the texts of scripts (the block editor) still say WordPress.
- **Per module:** the "Not rebuilt" and "Differs" notes in the entries of each plugin in [HISTORY.md](HISTORY.md) ("Plugins as
  native modules").
