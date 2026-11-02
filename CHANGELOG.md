# Changelog

All notable changes to RustPress. Each release names the WordPress release it follows.

## Unreleased

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
    shortcode and the fields of the comment form ([#21](https://github.com/rustpress-cms/rustpress/pull/21) to
    [#26](https://github.com/rustpress-cms/rustpress/pull/26)).
- The project website in English and German ([#13](https://github.com/rustpress-cms/rustpress/pull/13)) and
  `AGENTS.md` for contributors and coding agents ([#15](https://github.com/rustpress-cms/rustpress/pull/15)).
- The compatibility reports, `compat/known.toml` and the project plan are in English.

## 0.1.1 (2026-10-30)

- Follows WordPress 7.1.4 ([#10](https://github.com/rustpress-cms/rustpress/pull/10)).

## 0.1.0 (2026-10-30)

First release, following WordPress 7.1.3: block themes and a native Twenty Seventeen, the main query, permalinks
and canonical redirects, feeds, sitemaps, the REST API `wp/v2`, the administration with the block editor, the
WordPress installer, e-mail over SMTP, CGI mode for shared hosting, and builds for Linux, macOS, Windows and
FreeBSD plus a container image.
