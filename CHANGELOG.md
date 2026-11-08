# Changelog

All notable changes to RustPress. Each release names the WordPress release it follows.

## Unreleased

- **Twenty Twenty-One** 2.9 rebuilt natively, the second classic theme: all templates, menus with sub-menu toggles,
  descriptions and social icons, the footer widgets, logo, colors and the dark mode switch; on the way the shared parts
  of classic themes got `wp_restore_group_inner_container()`/`wp_restore_image_outer_container()`, the core block styles
  in registration order, the archive titles of post types and taxonomies and the signed-in comment form.
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
