<div align="center">

# RustPress

**WordPress, without PHP.** A drop-in replacement for WordPress written in Rust: point it at an existing WordPress database and `wp-content/`, and it serves the same pages.

**English** · [Deutsch](README.de.md)

[![CI](https://github.com/rustpress-cms/rustpress/actions/workflows/ci.yml/badge.svg)](https://github.com/rustpress-cms/rustpress/actions/workflows/ci.yml)
[![License: GPL v2+](https://img.shields.io/badge/license-GPL--2.0--or--later-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Edition 2024](https://img.shields.io/badge/edition-2024-orange.svg?logo=rust)](https://doc.rust-lang.org/edition-guide/rust-2024/)
[![WordPress](https://img.shields.io/badge/WordPress-7.1.4_compatible-21759b.svg?logo=wordpress)](compat/reports)
[![PHP](https://img.shields.io/badge/PHP-not_required-777bb4.svg?logo=php&logoColor=white)](#how-it-works)
[![Database](https://img.shields.io/badge/MySQL%20%7C%20MariaDB-supported-4479a1.svg?logo=mariadb&logoColor=white)](#getting-started)
[![Docker](https://img.shields.io/badge/docker-ready-2496ed.svg?logo=docker&logoColor=white)](#docker)
[![Shared hosting](https://img.shields.io/badge/shared_hosting-CGI-success.svg)](#shared-hosting-cgi)
[![Status](https://img.shields.io/badge/status-experimental-yellow.svg)](#status)

[![Last commit](https://img.shields.io/github/last-commit/rustpress-cms/rustpress.svg)](https://github.com/rustpress-cms/rustpress/commits/main)
[![Commit activity](https://img.shields.io/github/commit-activity/m/rustpress-cms/rustpress.svg)](https://github.com/rustpress-cms/rustpress/graphs/commit-activity)
[![Code size](https://img.shields.io/github/languages/code-size/rustpress-cms/rustpress.svg)](https://github.com/rustpress-cms/rustpress)
[![Top language](https://img.shields.io/github/languages/top/rustpress-cms/rustpress.svg)](https://github.com/rustpress-cms/rustpress)
[![Issues](https://img.shields.io/github/issues/rustpress-cms/rustpress.svg)](https://github.com/rustpress-cms/rustpress/issues)
[![Pull requests](https://img.shields.io/github/issues-pr/rustpress-cms/rustpress.svg)](https://github.com/rustpress-cms/rustpress/pulls)
[![Stars](https://img.shields.io/github/stars/rustpress-cms/rustpress.svg?style=social)](https://github.com/rustpress-cms/rustpress/stargazers)
[![Dependabot](https://img.shields.io/badge/dependabot-enabled-025e8c.svg?logo=dependabot)](.github/dependabot.yml)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](#contributing)

</div>

---

## How it works

RustPress reads the existing MySQL/MariaDB database (the usual `wp_*` schema, no conversion) and the existing `wp-content/` folder, and answers requests the way WordPress would: same HTML, same feeds, same REST responses, same redirects. There is no PHP interpreter. Themes and core behaviour are ported to Rust and checked against a real WordPress, page by page and byte by byte.

- **Block themes** (`theme.json` + HTML templates) run as they are. The classic theme **Twenty Seventeen** is rebuilt natively in Rust.
- Other classic PHP themes and PHP plugins do **not** run. Important plugins are meant to be rebuilt as native modules later.
- Theme assets, uploads, translations and `block.json` files are read from the WordPress files on disk, so a WordPress installation (or the files the installer downloads) is still needed.

## Features

- Pages, posts, archives, search and 404 with the site's block theme, including images (`srcset`, lazy loading, galleries)
- Canonical redirects, permalinks of every kind, feeds (RSS2, Atom, comment feeds), sitemaps, `robots.txt`
- REST API `wp/v2` (posts, pages, media, categories, tags, users, comments), read and write
- Administration (`wp-admin`): sign-in, posts and pages with the block editor, media, comments, users, settings, privacy tools, export
- A WordPress-style installer (`setup-config.php`, `install.php`) that can download and verify the WordPress files itself
- Shared hosting as a CGI program: one statically linked binary, configured from `wp-config.php`
- E-mail over SMTP (password reset, new users, comment notifications, privacy requests)
- Every language that WordPress has a language pack for, including right-to-left languages

## Compatibility

`rustpress-compat` crawls a reference WordPress 7.1.4 and RustPress side by side and compares status, headers and bodies. Latest runs ([reports](compat/reports)):

| Theme | URLs compared | identical | known, documented differences | unexpected |
|---|---:|---:|---:|---:|
| Twenty Twenty-Five | 921 | 900 | 21 | **0** |
| Twenty Twenty-Four | 916 | 897 | 19 | **0** |
| Twenty Twenty-Three | 913 | 897 | 16 | **0** |
| Twenty Twenty-Two | 916 | 897 | 19 | **0** |

Accepted differences are listed with their reason in [`compat/known.toml`](compat/known.toml).

## Getting started

Every [release](https://github.com/rustpress-cms/rustpress/releases) carries ready-made builds: static Linux binaries (x86_64, aarch64, armv7, armv6, i686) also as `.deb`, `.rpm` and `.apk` packages, plus macOS (Intel and Apple Silicon), Windows (x64, x86, ARM64) and FreeBSD. The container image is `ghcr.io/rustpress-cms/rustpress` for amd64, arm64, arm/v7, arm/v6 and 386. From source:

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
```

| Variable | Meaning |
|---|---|
| `RUSTPRESS_DATABASE_URL` | MySQL/MariaDB connection |
| `RUSTPRESS_TABLE_PREFIX` | table prefix (default `wp_`) |
| `RUSTPRESS_WP_ROOT` | WordPress folder |
| `RUSTPRESS_WP_CONFIG` | `wp-config.php` with the keys and salts (default: the one in `wp_root`) |
| `RUSTPRESS_LISTEN` | address to listen on |
| `RUSTPRESS_ALLOW_WRITES` | `1` enables writing (sign-in, admin, comments) |
| `RUSTPRESS_TRUSTED_PROXIES` | reverse proxies whose `X-Forwarded-*` headers are trusted |
| `RUSTPRESS_SMTP_URL`, `RUSTPRESS_MAIL_FROM` | outgoing mail |

Without `allow_writes` the database session is `READ ONLY` and the administration area stays closed. TLS is left to a reverse proxy.

## Docker

The `Dockerfile` builds a small Alpine image with a statically linked binary; `compose.yaml` runs it together with MariaDB. The released image can be used instead:

```sh
docker run -p 3000:3000 -v /path/to/wordpress:/var/www/html   -e RUSTPRESS_DATABASE_URL=mysql://user:pass@db:3306/wordpress ghcr.io/rustpress-cms/rustpress
```


```sh
cp .env.example .env     # set the password and address
docker compose up -d     # creates ./wordpress with the WordPress files
```

Then open the site in the browser and follow the installer, or point `WORDPRESS_DIR` in `.env` at an existing WordPress folder and import its database dump.

## Shared hosting (CGI)

On hosts that only offer Apache with CGI, RustPress runs as a single static binary in `cgi-bin/`. It detects CGI mode by itself (`GATEWAY_INTERFACE`), reads the database settings from `wp-config.php` and takes over the site through `.htaccess`. On an empty webspace the installer downloads WordPress 7.1.4 from wordpress.org (checksum-verified) for the theme files and translations. The binary (`rustpress-<target>`, e.g. `rustpress-x86_64-unknown-linux-musl`) is built by CI and attached to every [release](https://github.com/rustpress-cms/rustpress/releases).

## Development

The tests compare RustPress with a real WordPress. A reference stack (MariaDB on `:3306`, WordPress 7 on `:8080`) is started with:

```sh
dev/up.sh       # start containers
dev/seed.sh     # create test content
cargo test --workspace
```

Without the stack the golden tests skip themselves, which is what CI does.

```sh
cargo run -p rustpress-compat -- --report compat/reports/twentytwentyfive.md
```

### Following WordPress releases

`WORDPRESS_VERSION` names the release RustPress follows; the installer downloads it, and `dev/up.sh` keeps the reference WordPress at it with automatic updates off. A daily workflow opens an issue for each newer release, with the changed PHP code, the RustPress files that follow it and the steps to move over:

```sh
dev/wp-diff.sh 7.1.3 7.1.4    # the same report locally, plus the full PHP diff
dev/wp-bump.sh 7.1.4          # set the new version and bring the reference WordPress to it
```

### Crates

| Crate | Purpose |
|---|---|
| `rustpress-phpser` | PHP `serialize`/`unserialize`, byte-exact |
| `rustpress-db` | access to the WordPress schema (sqlx) |
| `rustpress-core` | options, rewrite rules, main query, template hierarchy, canonical redirects, sitemaps |
| `rustpress-blocks` | block parser and renderer, block supports, style engine |
| `rustpress-theme` | `theme.json`, templates, global styles, documents and feeds, Twenty Seventeen |
| `rustpress-content` | writing posts, terms, users, comments and uploads |
| `rustpress-auth` | passwords, cookies, sessions, nonces and capabilities |
| `rustpress-admin` | `wp-admin`, sign-in and the installer |
| `rustpress-rest` | REST API |
| `rustpress-mail` | e-mail |
| `rustpress-server` | axum server and CGI mode |
| `rustpress-compat` | comparison run against a real WordPress |

## Status

RustPress is experimental. It is tested against WordPress 7.1.4 and runs a real site in production, but expect rough edges, and keep a backup of your database and the WordPress files before pointing it at a live site.

## Contributing

Issues and pull requests are welcome. Please run `cargo test --workspace` and `cargo clippy --workspace --all-targets` before opening a pull request. Security issues: please report them privately through [GitHub security advisories](https://github.com/rustpress-cms/rustpress/security/advisories/new) instead of a public issue.

## License

RustPress is free software under the GNU General Public License, version 2 or (at your option) any later version (GPL-2.0-or-later); see [LICENSE](LICENSE).

RustPress is a port of WordPress to Rust. Texts, templates, data and the Twenty Seventeen port come from WordPress, © the WordPress contributors, also GPLv2 or later.

RustPress is an independent project and is not affiliated with or endorsed by the WordPress Foundation or Automattic. WordPress is a registered trademark of the WordPress Foundation.
