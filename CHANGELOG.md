# Changelog

All notable changes to RustPress. Each release names the WordPress release it follows.

## Unreleased

- Native plugin module **Complianz** 7.5.6, stage 2: the legal documents. The cookie policy of every region (EU, US,
  UK, Canada, Australia, South Africa, Brazil) through `[cmplz-document]` and the block `complianz/document`, with
  the plugin's numbering, conditions, company details and translations, the list of cookies per service and purpose
  (`[cmplz-cookies]`) from the plugin's tables, and the do-not-sell form of the US policy (sending a request follows).
  Plugins can now print on `wp_print_footer_scripts` before the emoji script.
- **Twenty Twenty** 3.3 rebuilt natively, the third classic theme: all templates including the cover and full width
  page templates, the search and menu modals, the primary, expanded, mobile, footer and social menus with sub-menu
  toggles and social icons, the page list fallback, the footer widgets, the retina logo, the accent colors of the
  Customizer and the post meta. On the way: the password form of classic themes has no block button, and a page
  template in a folder gets a body class for each part of its path.
- Native plugin module **WP Mail SMTP** 4.10.4: the site's e-mail goes through the SMTP server of the plugin's
  settings or of the `WPMS_*` constants (host, port, SSL/TLS/auto TLS, authentication with the encrypted password),
  with the sender of the settings ("force from email/name", the original sender as `Reply-To`), the `X-Mailer` of
  the plugin and "Do not send". A new hook lets plugins decide how mail is sent (`phpmailer_init`, `wp_mail_from`).
- E-mail without `RUSTPRESS_SMTP_URL` goes to the host's `sendmail`, as PHP's `mail()` sends it, so password
  resets, comment notifications and form mails work on shared hosting such as Strato without any setting.
  `RUSTPRESS_SENDMAIL` names another program, `off` sends nothing.
- Scheduled posts are published in CGI mode too, by the first request after their time (`DISABLE_WP_CRON` turns
  that off). Saving, trashing and deleting a post now keeps its `publish_future_post` event in the option `cron` as
  WordPress does, so a site that goes back to PHP publishes the posts scheduled in RustPress.

## 0.3.0 (2026-11-09)

Follows WordPress 7.1.4.

- Native plugin module **WPForms Lite** 2.0.4: forms of the shortcode and the block on the website, submitting with
  validation, the honeypot and the AntiSpam token, confirmations (message, page, redirect), the smart tags, and the
  notification e-mails in the plugin's templates Classic, Compact and Plain text. Forms can also be sent on read-only
  sites: `admin-ajax.php` answers the plugins' visitor actions without write access.
- **Twenty Twenty-One** 2.9 rebuilt natively, the second classic theme: all templates, menus with sub-menu toggles,
  descriptions and social icons, the footer widgets, logo, colors and the dark mode switch; on the way the shared parts
  of classic themes got `wp_restore_group_inner_container()`/`wp_restore_image_outer_container()`, the core block styles
  in registration order, the archive titles of post types and taxonomies and the signed-in comment form.
- Native plugin module **Complianz – GDPR/CCPA Cookie Consent** 7.5.6, stage 1 (the website): the cookie banner and
  the manage consent button for every region and consent type, the settings of `complianz.min.js`, the statistics
  scripts (Google Analytics, Tag Manager with the head option, Matomo, Clicky, Clarity and the others the plugin
  configures), the scripts and styles of the services, the cookie blocker with its placeholders, the REST routes
  `complianz/v1` the website calls, and the shortcodes `[cmplz-manage-consent]`, `[cmplz-revoke-link]` and
  `[cmplz-accept-link]`. The legal documents are stage 2. New hooks for plugins: `wp_footer` and
  `wp_print_footer_scripts` with the query, `body_class`, the whole page before it is sent (`template_output`),
  `wp_head` at priority 0; plugin assets can be enqueued asynchronously.
- Third-party block themes **Extendable** 2.2.0 and **Bluehost Blueprint** 1.0.2 run, with their `functions.php`
  ported (scripts, the block styles of `assets/block-styles/`). On the way, for every block theme: the order of
  `PROPERTIES_METADATA` of WordPress 7.1 (`text-shadow`, the root padding variables), the theme.json elements
  `textInput` and `select` and plain `cite`, the preset classes of blocks (`:where(.wp-block-button …).has-…`), the
  rules of a block's feature selectors after its own, `@font-face` with `font-variation-settings` and the other
  properties in WordPress's order, relative `url()`s of inlined stylesheets (`_wp_normalize_relative_css_links()`),
  `if … : elseif … : else : endif;` in patterns, `home_url( '/' )` with its slash, and `?paged=0` in the page
  numbers of a custom query.
- Inline scripts before and after a script lose their leading line breaks, as in `WP_Scripts::get_inline_script_data()`.
- Third-party block themes: Frost and Ollie 1.6.6 run, with Ollie's `functions.php` ported; block presets,
  `gmdate()` in patterns and the query title ([#48](https://github.com/rustpress-cms/rustpress/pull/48), [#49](https://github.com/rustpress-cms/rustpress/pull/49)).
- **Classic Editor** 1.7.0 as a native module: the classic edit screen with its settings and the choice per user
  and post ([#57](https://github.com/rustpress-cms/rustpress/pull/57)).
- Classic themes in wp-admin: `nav-menus.php`, `widgets.php` with writing widgets and `batch/v1`, the header screen,
  the Customizer and its preview; media widgets on the website and in REST; the theme's image sizes in uploads
  ([#59](https://github.com/rustpress-cms/rustpress/pull/59), [#60](https://github.com/rustpress-cms/rustpress/pull/60), [#67](https://github.com/rustpress-cms/rustpress/pull/67) to [#71](https://github.com/rustpress-cms/rustpress/pull/71)).
- The administration in the language of the site: texts, templates, JS translations, the dashboard and the file
  editors ([#61](https://github.com/rustpress-cms/rustpress/pull/61) to [#63](https://github.com/rustpress-cms/rustpress/pull/63), [#65](https://github.com/rustpress-cms/rustpress/pull/65), [#66](https://github.com/rustpress-cms/rustpress/pull/66)); updates in the list of plugins ([#64](https://github.com/rustpress-cms/rustpress/pull/64)).
- Blocks and media of WordPress 7.1: block supports (visibility, states, viewports, custom CSS), the lightbox,
  dynamic galleries, playlists and `[playlist]`, `core/navigation-submenu`, embeds, calendar weeks
  ([#51](https://github.com/rustpress-cms/rustpress/pull/51) to [#53](https://github.com/rustpress-cms/rustpress/pull/53), [#55](https://github.com/rustpress-cms/rustpress/pull/55)); `media-upload.php` and `sanitize_file_name()` ([#58](https://github.com/rustpress-cms/rustpress/pull/58)).
- Twenty Seventeen: post formats, front page panels, colors, the social menu and signed-in viewers ([#54](https://github.com/rustpress-cms/rustpress/pull/54)).
- Comment feeds of archives and searches, password rehash on sign-in, REST `_method` ([#50](https://github.com/rustpress-cms/rustpress/pull/50)).
- **Site Kit by Google** 1.191.0 as a native module for the website: the Analytics tag with consent mode and the
  opt-out for signed-in users, the site verification meta tags and file, and the generator tag.
- **WPCode Lite** 2.4.1 as a native module for the website: the global header, body and footer scripts, the
  site-wide snippets (HTML, text, JavaScript, CSS) with their conditional logic and the `[wpcode]` shortcode.
  `wp_body_open` is new for plugins.
- **Disable Comments** 2.9.1 as a native module for the website and the REST API: comments and pings off
  everywhere, per post type or by conditional rule (terms, page template, roles, auto-close), the existing comments
  kept or hidden, allowed comment types, the comment feeds and REST routes refused. Twenty Seventeen no longer prints
  an extra line break before "Comments are closed.".
- `rustpress check` no longer counts backup, migration and translation plugins as missing, and reports security
  and admin-only plugins as a note that says what is lost instead of "does not run".

## 0.2.0 (2026-11-05)

Follows WordPress 7.1.4.

- Faster, above all in CGI mode: a single post as a CGI process (static musl binary) 140 → 58 ms, the server
  15–70 % more requests per second (mimalloc, regexes that compile in a fraction of the time, a file snapshot in
  `cgi-bin/rustpress-cache.bin`, fewer queries); `dev/bench.sh` measures it against the reference WordPress.
  In CGI mode the queries go out without a prepared statement (one round trip each instead of two), and
  `Server-Timing` shows the time spent in the database.
- `rustpress check` tells whether RustPress can serve a site: it lists the theme and the active plugins and says
  which of them run.
- Native plugin modules (the plugin's files stay in `wp-content/plugins`; the module takes over while the plugin
  is active):
  - **SVG Support** 2.6.1: sanitized SVG uploads ([#12](https://github.com/rustpress-cms/rustpress/pull/12)),
    after the module framework and WP Date Remover 1.1.3 ([#11](https://github.com/rustpress-cms/rustpress/pull/11)).
  - **Contact Form 7** 6.2.4.1: forms, validation, spam checks and mail; its REST API; the list, the editor,
    the dashboard and the integration page in the administration
    ([#14](https://github.com/rustpress-cms/rustpress/pull/14), [#16](https://github.com/rustpress-cms/rustpress/pull/16),
    [#17](https://github.com/rustpress-cms/rustpress/pull/17)).
  - **Yoast SEO** 28.8: the head of the website, the schema graph, XML sitemaps, breadcrumbs, `robots.txt`,
    `yoast_head` in the REST API and the feed changes ([#18](https://github.com/rustpress-cms/rustpress/pull/18),
    [#19](https://github.com/rustpress-cms/rustpress/pull/19)).
  - **Redirection** 5.10.1: redirects with all conditions and actions, custom headers, the canonical domain, hit
    counters and logs ([#20](https://github.com/rustpress-cms/rustpress/pull/20)).
  - **Advanced Custom Fields** 6.8.10: ACF post types and taxonomies, the `acf` field of the REST API, the `[acf]`
    shortcode, the fields of the comment form and on the edit screens of posts, terms, users, comments and media
    ([#21](https://github.com/rustpress-cms/rustpress/pull/21) to [#33](https://github.com/rustpress-cms/rustpress/pull/33)),
    and the ACF administration: the field group editor, the editors of post types and taxonomies, the options
    pages preview, the tools page and export and import
    ([#34](https://github.com/rustpress-cms/rustpress/pull/34) to [#40](https://github.com/rustpress-cms/rustpress/pull/40)).
- The project website in English and German ([#13](https://github.com/rustpress-cms/rustpress/pull/13)) and
  `AGENTS.md` for contributors and coding agents ([#15](https://github.com/rustpress-cms/rustpress/pull/15)).
- The compatibility reports, `compat/known.toml` and the project plan are in English.
- The golden tests are reproducible: `dev/seed.sh` installs WordPress and seeds every test post with fixed dates
  ([#45](https://github.com/rustpress-cms/rustpress/pull/45)). A "Port request" issue template asks for new plugin modules.

## 0.1.1 (2026-10-30)

- Follows WordPress 7.1.4 ([#10](https://github.com/rustpress-cms/rustpress/pull/10)).

## 0.1.0 (2026-10-30)

First release, following WordPress 7.1.3: block themes and a native Twenty Seventeen, the main query, permalinks
and canonical redirects, feeds, sitemaps, the REST API `wp/v2`, the administration with the block editor, the
WordPress installer, e-mail over SMTP, CGI mode for shared hosting, and builds for Linux, macOS, Windows and
FreeBSD plus a container image.
