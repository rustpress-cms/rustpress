<div align="center">

# RustPress

**WordPress, without PHP.** A replacement for WordPress written in Rust: point it at the database and `wp-content/` of an existing site with a block theme, and it serves the same pages, byte for byte.

[Website](https://rustpress-cms.github.io/rustpress/) · **English** · [Deutsch](README.de.md)

[![CI](https://github.com/rustpress-cms/rustpress/actions/workflows/ci.yml/badge.svg)](https://github.com/rustpress-cms/rustpress/actions/workflows/ci.yml)
[![License: GPL v2+](https://img.shields.io/badge/license-GPL--2.0--or--later-blue.svg)](LICENSE)
[![WordPress](https://img.shields.io/badge/WordPress-7.1.5_compatible-21759b.svg?logo=wordpress)](compat/reports)
[![Status](https://img.shields.io/badge/status-experimental-yellow.svg)](#status)

</div>

---

## How it works

RustPress reads the existing MySQL/MariaDB database (the usual `wp_*` schema, no conversion) and the existing `wp-content/` folder, and answers requests the way WordPress would: same HTML, same feeds, same REST responses, same redirects. There is no PHP interpreter. Themes and core behaviour are ported to Rust and checked against a real WordPress, page by page and byte by byte.

- **Block themes** (`theme.json` + HTML templates) run as they are; checked with the bundled themes, Frost, Ollie, Extendable and Bluehost Blueprint (the last three with their `functions.php` ported). The classic themes **Twenty Seventeen**, **Twenty Twenty** and **Twenty Twenty-One** are rebuilt natively in Rust.
- Other classic PHP themes and PHP plugins do **not** run. Important plugins are rebuilt as native modules that take over while the plugin is active (its files stay in `wp-content/plugins`): so far **WP Date Remover** 1.1.3 and **SVG Support** 2.6.1 (website, settings and sanitized SVG uploads); **Contact Form 7** 6.2.4.1 (forms, validation, spam checks and mail on the website; the list, the editor, the dashboard and the integration page in the administration); **Yoast SEO** 28.8 (the head of the website: title, meta description, robots, canonical, Open Graph, Twitter and the schema graph; XML sitemaps, breadcrumbs, `robots.txt`, `yoast_head` in the REST API and the feed changes); **Redirection** 5.10.1 (redirects with all conditions and actions, custom headers, the canonical domain, hit counters and the logs of redirects and 404 pages); **Advanced Custom Fields** 6.8.10 (the post types and taxonomies defined in ACF, the `acf` field of the REST API with the values of the field groups, raw or formatted with `?acf_format=standard`, the `[acf]` shortcode, the fields of the comment form, which are saved with the comment, the fields on the edit screens of comments, posts (in the block editor and the classic one), terms, users and media, and ACF's lists of field groups, post types and taxonomies with their actions, the editors of field groups, post types and taxonomies, which save, and the tools to export and import); **Classic Editor** 1.7.0 (the classic edit screen instead of the block editor, with its settings and the choice per user and post); **WPForms Lite** 2.0.4 (forms on the website, validation, spam checks, confirmations and the notification e-mails in the plugin's templates); **Site Kit by Google** 1.191.0 (only the tags on the website: Analytics with consent mode and the opt-out for signed-in users, site verification; not the dashboard); **Complianz – GDPR/CCPA Cookie Consent** 7.5.6 (the cookie banner with its settings, the statistics scripts, the cookie blocker with placeholders, the public REST routes, the consent shortcodes, the consent area and the generated cookie policy of every region with the cookie list); **WPCode Lite** 2.4.1 (the global header, body and footer scripts and the site-wide snippets with their conditions and the `[wpcode]` shortcode; PHP snippets do not run); **Disable Comments** 2.9.1 (comments and pings off everywhere, per post type or by rule, on the website and in the REST API); **WP Mail SMTP** 4.10.4 (the site's e-mail through the SMTP server of its settings, with their sender; not the web API mailers). In the administration, **Plugins › Add Plugin** lists exactly these plugins and installs the release a module follows with one click.
- Theme assets, uploads, translations and `block.json` files are read from the WordPress files on disk, so a WordPress installation (or the files the installer downloads) is still needed.

## Will it run my site?

Run `rustpress check` with the site's configuration (the same `RUSTPRESS_*` variables or TOML file as the server). It reads the active theme and plugins and says which of them run, without changing anything:

```text
$ RUSTPRESS_DATABASE_URL=mysql://user:pass@localhost/wordpress RUSTPRESS_WP_ROOT=/var/www/html rustpress check
[ok  ] Theme twentytwentyfive: block theme, runs as it is
[ok  ] Plugin wordpress-seo/wp-seo.php 28.8: native module
[no  ] Plugin elementor/elementor.php 3.30.0: PHP plugin without a native module; RustPress ignores it

Not yet: 1 part(s) of this site do not run in RustPress. Tell us which ones you need:
https://github.com/rustpress-cms/rustpress/issues/new?template=port-request.yml
```

| Runs | Does not run (yet) |
|---|---|
| Block themes, including Twenty Twenty-Two to Twenty Twenty-Five, Frost, Ollie, Extendable and Bluehost Blueprint | Classic PHP themes other than Twenty Seventeen, Twenty Twenty and Twenty Twenty-One (Astra, GeneratePress, Divi, Hello Elementor, ...) |
| Twenty Seventeen, Twenty Twenty and Twenty Twenty-One (ported natively) | Page builders (Elementor, Divi, WPBakery) and WooCommerce |
| Yoast SEO, Contact Form 7, WPForms Lite, Redirection, Advanced Custom Fields, Classic Editor, Complianz (banner, cookie blocker, cookie policy), Site Kit by Google (website tags), WPCode (scripts, no PHP), Disable Comments (website and REST API), WP Mail SMTP (SMTP), SVG Support, WP Date Remover | Any other PHP plugin: it is ignored, so whatever it adds to the site is missing |
| Plugins that RustPress makes unnecessary: page caches, WP Mail SMTP | Multisite, saving in the Customizer (its page and the preview work), PHP in a block theme's `functions.php` beyond simple registrations |
| Every site language WordPress has a language pack for | Services that need outgoing HTTP (Akismet, reCAPTCHA, update checks) |

The plugins that people need most are ported next: ask for one with a [port request](https://github.com/rustpress-cms/rustpress/issues/new?template=port-request.yml), or port it yourself with [docs/porting-a-plugin.md](docs/porting-a-plugin.md).

## Features

- Pages, posts, archives, search and 404 with the site's block theme, including images (`srcset`, lazy loading, galleries)
- Canonical redirects, permalinks of every kind, feeds (RSS2, Atom, comment feeds), sitemaps, `robots.txt`
- REST API `wp/v2` (posts, pages, media, categories, tags, users, comments), read and write
- Administration (`wp-admin`): sign-in, posts and pages with the block editor, media, comments, users, settings, privacy tools, export
- A WordPress-style installer (`setup-config.php`, `install.php`) that can download and verify the WordPress files itself
- `rustpress check`, which tells whether a site runs before RustPress takes it over
- Shared hosting as a CGI program: one statically linked binary, configured from `wp-config.php`
- E-mail over SMTP (password reset, new users, comment notifications, privacy requests, contact forms)
- Every language that WordPress has a language pack for, including right-to-left languages

## Compatibility

`rustpress-compat` crawls a reference WordPress 7.1.5 and RustPress side by side and compares status, headers and bodies. Latest runs ([reports](compat/reports)):

| Theme | URLs compared | identical | known, documented differences | unexpected |
|---|---:|---:|---:|---:|
| Twenty Twenty-Five | 921 | 900 | 21 | **0** |
| Twenty Twenty-Four | 916 | 897 | 19 | **0** |
| Twenty Twenty-Three | 913 | 897 | 16 | **0** |
| Twenty Twenty-Two | 916 | 897 | 19 | **0** |

The runs cover what visitors and API clients see: pages, feeds, sitemaps, the REST API, redirects and files. They run RustPress without `allow_writes`; sign-in, `wp-admin`, the installer and the native plugin modules are compared with their own golden tests and scripts in [`dev/`](dev). Accepted differences are listed with their reason in [`compat/known.toml`](compat/known.toml); [compat/reports/README.md](compat/reports/README.md) says which run each report comes from. The times in the reports come from one local run and are no benchmark.

## How it compares

Other projects also try to take WordPress sites somewhere new. They answer different questions:

| Project | Approach | Uses the existing database and `wp-content/` | How compatibility is checked |
|---|---|---|---|
| **RustPress** | WordPress core, block themes, three classic themes and selected plugins ported to Rust; no PHP | yes, unchanged | byte for byte against a real WordPress: golden tests and `rustpress-compat` runs; every accepted difference is listed with its reason |
| [WpDotNet](https://github.com/iolevel/wpdotnet-sdk) (PeachPie) | compiles WordPress's own PHP to .NET | yes; PHP plugins and themes generally run | it is WordPress's code |
| [LegacyToRustProject/RustPress](https://github.com/LegacyToRustProject/RustPress) (same name, unrelated) | a Rust CMS on the `wp_*` tables; themes and plugins converted with AI | yes | "visual parity" with WordPress, according to its README |
| [OxidePress](https://github.com/Vabian124/OxidePress) | a fork of the project above; runs PHP plugins in a `php` CLI worker | yes | |
| [Daypress](https://github.com/Trooper-lab/Daypress) | a Rust CMS with its own SQLite database and part of the `wp/v2` REST API | imports a WXR export | |
| [EmDash](https://github.com/emdash-cms/emdash) (Cloudflare) | a TypeScript CMS on Astro | imports a WXR export; blocks become Portable Text | |

In short: if you need every PHP plugin, WpDotNet runs WordPress itself. If you want a new CMS, EmDash and Daypress import your content. RustPress is for keeping a WordPress site exactly as it is, database and files included, and serving it without PHP; what it supports answers like WordPress down to the byte, and what it does not support, `rustpress check` tells you before you switch. The descriptions of the other projects are taken from their own pages (November 2026).

## Getting started

Every [release](https://github.com/rustpress-cms/rustpress/releases) carries ready-made builds: static Linux binaries (x86_64, aarch64, armv7, armv6, i686) also as `.deb`, `.rpm` and `.apk` packages, plus macOS (Intel and Apple Silicon), Windows (x64, x86, ARM64) and FreeBSD. The container image is `ghcr.io/rustpress-cms/rustpress` for amd64, arm64, arm/v7, arm/v6 and 386. CI tests on x86_64 Linux and builds for Windows; the other targets are cross-compiled and not tested, so please report what does not work there. From source:

```sh
cargo run --release -p rustpress-server
```

Configuration comes from a TOML file (`RUSTPRESS_CONFIG=path.toml`) and/or environment variables; the variables win:

```toml
database_url = "mysql://wordpress:secret@127.0.0.1:3306/wordpress"
table_prefix = "wp_"
wp_root      = "/var/www/html"   # contains wp-content/ and wp-includes/
listen       = "127.0.0.1:3000"
reload_secs  = 60                # options and theme are re-read this often
allow_writes = false             # true: sign-in, comments and wp-admin
branding     = true              # false: WordPress' texts and logo in wp-admin
```

| Variable | Meaning |
|---|---|
| `RUSTPRESS_DATABASE_URL` | MySQL/MariaDB connection |
| `RUSTPRESS_TABLE_PREFIX` | table prefix (default `wp_`) |
| `RUSTPRESS_WP_ROOT` | WordPress folder |
| `RUSTPRESS_WP_CONFIG` | `wp-config.php` with the keys and salts (default: the one in `wp_root`) |
| `RUSTPRESS_LISTEN` | address to listen on |
| `RUSTPRESS_ALLOW_WRITES` | `1` enables writing (sign-in, admin, comments) |
| `RUSTPRESS_BRANDING` | `0` shows WordPress' texts and logo in the administration instead of RustPress' (for comparisons) |
| `RUSTPRESS_TRUSTED_PROXIES` | reverse proxies whose `X-Forwarded-*` headers are trusted |
| `RUSTPRESS_SMTP_URL`, `RUSTPRESS_MAIL_FROM` | outgoing mail; without an SMTP server it goes to the host's `sendmail` like PHP's `mail()` (`RUSTPRESS_SENDMAIL` names another program, `off` sends nothing) |

Without `allow_writes` the database session is `READ ONLY` and the administration area stays closed; contact forms (Contact Form 7, WPForms Lite) can still be sent, since they only send e-mail. TLS is left to a reverse proxy.

## Docker

The `Dockerfile` builds a small Alpine image with a statically linked binary; `compose.yaml` runs it together with MariaDB. The released image can be used instead:

```sh
docker run -p 3000:3000 -v /path/to/wordpress:/var/www/html   -e RUSTPRESS_DATABASE_URL=mysql://user:pass@db:3306/wordpress ghcr.io/rustpress-cms/rustpress
```

With Compose, from an empty folder:

```sh
cp .env.example .env     # set the password and address
docker compose up -d     # creates ./wordpress with the WordPress files
```

Then open the site in the browser and follow the installer. For an existing site, point `WORDPRESS_DIR` in `.env` at its WordPress folder and import its database dump before the first start:

```sh
docker compose up -d db
docker compose exec -T db sh -c 'mariadb -uwordpress -p"$MARIADB_PASSWORD" wordpress' < dump.sql
docker compose up -d
```

The site keeps the address stored in the dump (`siteurl` and `home`).

## Shared hosting (CGI)

On hosts that only offer Apache with CGI, RustPress runs as a single static binary in `cgi-bin/`. It detects CGI mode by itself (`GATEWAY_INTERFACE`), reads the database settings from `wp-config.php` and takes over the site through `.htaccess`. On an empty webspace the installer downloads WordPress 7.1.5 from wordpress.org (checksum-verified) for the theme files and translations. The binary (`rustpress-<target>`, e.g. `rustpress-x86_64-unknown-linux-musl`) is built by CI and attached to every [release](https://github.com/rustpress-cms/rustpress/releases).

Each request starts a new process, so RustPress keeps the theme and WordPress files it reads in `cgi-bin/rustpress-cache.bin` for the next one (a file that changed is read again). Delete it at any time; the next request writes it again.

Scheduled posts are published by the first request after their time, as WordPress's `wp-cron` does (unless `wp-config.php` defines `DISABLE_WP_CRON`); a read-only site (`allow_writes = false`) cannot publish them.

## Development

The tests compare RustPress with a real WordPress. A reference stack (MariaDB on `:3306`, WordPress 7 on `:8080`) is started with:

```sh
dev/up.sh       # start containers
dev/seed.sh     # create test content
cargo test --workspace
```

Without the stack the golden tests skip themselves. CI starts and seeds the stack in its job `golden` and runs them all, followed by a compat run.

```sh
cargo run -p rustpress-compat -- --report compat/reports/twentytwentyfive.md
```

### Following WordPress releases

`WORDPRESS_VERSION` names the release RustPress follows; the installer downloads it, and `dev/up.sh` keeps the reference WordPress at it with automatic updates off. A daily workflow opens an issue for each newer release, with the changed PHP code, the RustPress files that follow it and the steps to move over:

```sh
dev/wp-diff.sh 7.1.4 7.1.5    # the same report locally, plus the full PHP diff
dev/wp-bump.sh 7.1.5          # set the new version and bring the reference WordPress to it
```

### Crates

| Crate | Purpose |
|---|---|
| `rustpress-phpser` | PHP `serialize`/`unserialize`, byte-exact |
| `rustpress-db` | access to the WordPress schema (sqlx) |
| `rustpress-core` | options, rewrite rules, main query, template hierarchy, canonical redirects, sitemaps |
| `rustpress-blocks` | block parser and renderer, block supports, style engine |
| `rustpress-theme` | `theme.json`, templates, global styles, documents and feeds, Twenty Seventeen, Twenty Twenty and Twenty Twenty-One |
| `rustpress-content` | writing posts, terms, users, comments and uploads |
| `rustpress-auth` | passwords, cookies, sessions, nonces and capabilities |
| `rustpress-admin` | `wp-admin`, sign-in and the installer |
| `rustpress-rest` | REST API |
| `rustpress-mail` | e-mail |
| `rustpress-plugins` | native modules that stand in for WordPress plugins |
| `rustpress-server` | axum server and CGI mode |
| `rustpress-compat` | comparison run against a real WordPress |

## Status

RustPress is experimental. It is tested against WordPress 7.1.5 and runs a real site in production, but expect rough edges, and keep a backup of your database and the WordPress files before pointing it at a live site.

## Contributing

Issues and pull requests are welcome; [CONTRIBUTING.md](CONTRIBUTING.md) has the short version. [AGENTS.md](AGENTS.md) explains how the project works (WordPress as the oracle, the reference stack, golden tests, native plugin modules) for contributors and coding agents alike, and [docs/porting-a-plugin.md](docs/porting-a-plugin.md) walks through porting a plugin step by step. Please run `cargo test --workspace` and `cargo clippy --workspace --all-targets` before opening a pull request. Security issues: please report them privately as described in [SECURITY.md](SECURITY.md). Changes are listed in [CHANGELOG.md](CHANGELOG.md).

## License

RustPress is free software under the GNU General Public License, version 2 or (at your option) any later version (GPL-2.0-or-later); see [LICENSE](LICENSE).

RustPress is a port of WordPress to Rust. Texts, templates, data and the ports of Twenty Seventeen, Twenty Twenty and Twenty Twenty-One come from WordPress, © the WordPress contributors, also GPLv2 or later.

RustPress is an independent project and is not affiliated with or endorsed by the WordPress Foundation or Automattic. WordPress is a registered trademark of the WordPress Foundation.
