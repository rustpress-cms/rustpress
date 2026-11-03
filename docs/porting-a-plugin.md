# Porting a WordPress plugin

A step-by-step guide for coding agents and people who want to add a plugin to RustPress. Read
[AGENTS.md](../AGENTS.md) first; this guide assumes its rules, above all: **WordPress is the oracle**, and a port is
done when its output equals the real plugin's, byte for byte.

PHP never runs in RustPress. A plugin is supported when a **native module** in `crates/rustpress-plugins` rebuilds
what it does. The module takes over while the plugin's file is listed in `active_plugins`, so activating and
deactivating in `wp-admin` work as before. The plugin's files stay in `wp-content/plugins`: RustPress serves their
assets, reads their headers and loads their language packs from there.

The existing modules are the best examples, from small to large:

| Module | Plugin | Shows |
|---|---|---|
| `wp_date_remover.rs` | WP Date Remover 1.1.3 | the smallest complete module: assets, a filter, a settings page, Ajax |
| `svg_support/` | SVG Support 2.6.1 | uploads and a sanitizer rebuilt from a PHP library, golden data |
| `contact_form_7/` | Contact Form 7 6.2.4.1 | shortcodes, REST routes, mail, a whole administration |
| `wordpress_seo/` | Yoast SEO 28.8 | `wp_head`, sitemaps, `robots.txt`, REST fields, feeds |
| `redirection/` | Redirection 5.10.1 | requests before the main query, headers, logs |
| `acf/` | Advanced Custom Fields 6.8.10 | registrations, REST fields, form rendering and saving |

## 0. Pick the scope

Check the [port requests](https://github.com/rustpress-cms/rustpress/issues?q=is%3Aissue+label%3A%22port+request%22)
first; one with many 👍 is worth more. Then decide on **one release** of the plugin (the newest stable one, as a
rule) and on a **stage**: what visitors see on the website comes first, the administration later. Large plugins are
ported in stages, each one merged on its own (see the ACF and Yoast sections of [PLAN.md](../PLAN.md)). Write the
plan as a new section in PLAN.md before you start.

Plugins that need outgoing HTTP to a service (Akismet, reCAPTCHA, payment providers) or that execute user-supplied
PHP cannot be ported faithfully. Say so in the port request instead of writing a partial module.

## 1. Set up the reference

```sh
dev/up.sh && dev/seed.sh
```

Install the chosen release of the plugin in the reference (`podman exec rustpress-wp wp plugin install
<slug> --version=<version>` through `dev/wp.sh`, or unpack the ZIP into its `wp-content/plugins`). The reference
and RustPress share one database: activating the plugin activates it for both. Keep it **deactivated** while you
are not comparing, because an active plugin changes what the core goldens expect.

## 2. Read the plugin's PHP

Find every place the plugin hooks into WordPress: `add_action`, `add_filter`, `add_shortcode`,
`register_rest_route`, `register_post_type`, `register_block_type`, `add_menu_page`, `wp_enqueue_*`. List them in
the PLAN.md section with the WordPress hook each one uses. Port from this code, not from the plugin's documentation:
the behaviour the code has, quirks included, is what has to come out.

## 3. Write a golden generator

Let the real plugin produce the expected output for many inputs, and commit that output as test data. Examples:
`dev/cf7-form-golden.sh` (forms printed by Contact Form 7), `dev/cf7-mail-golden.sh` (the mails it sends, caught at
`wp_mail()`), `dev/svg-sanitize-golden.sh` (SVGs cleaned by the PHP library), `dev/acf-render-golden.sh` (fields
rendered by ACF). The pattern:

1. Put the cases in `crates/rustpress-plugins/tests/data/<plugin>/`.
2. A shell script copies them into the reference container, runs a small PHP file there with
   `require '/var/www/html/wp-load.php'` that calls the plugin's own functions, and copies the results back.
3. Neutralize what is random or depends on the salts (replace it with a fixed marker such as `HASH`).
4. Mark golden files whose bytes matter as `-text` in `.gitattributes`.

A unit test in the module reads the cases and compares (see `golden_forms` in `contact_form_7/mod.rs`). These
tests run without the reference stack, so CI checks them too.

## 4. Write the module

1. Create `crates/rustpress-plugins/src/<plugin>/mod.rs` (or one file for a small plugin) with a doc comment that
   names the plugin, its release, what it does and what is not ported.
2. Implement `NativePlugin` (`rustpress-core/src/native.rs`):
   - `file()`: the plugin file exactly as `active_plugins` lists it (`contact-form-7/wp-contact-form-7.php`).
   - `version()`: the release the module follows.
   - `load()`: read what the plugin reads at `plugins_loaded`/`init` (options, term meta, its own tables) once per
     snapshot of the site, and return the `ActivePlugin`.
3. Implement the hooks of `ActivePlugin` that the plugin needs. Every hook has a default that does nothing. There
   are hooks for assets (`enqueue`, `admin_enqueue`), the head and footer (`wp_head`, `wp_footer`), requests
   (`init`, `serve`, `template_redirect`, `send_headers`, `not_found`), content (`shortcode_tags`, `shortcode`,
   `block_names`, `render_block`), REST (`rest_index`, `rest`, `rest_fields`, `rest_links`), feeds, sitemaps and
   `robots.txt`, comments, uploads, and the administration (`admin_pages`, `admin_page`, `admin_notices`,
   `registered_settings`, `admin_ajax`, ...). Read the doc comment of each hook in `native.rs`.
4. **A hook is missing?** Add it to `ActivePlugin` with a default that does nothing and a doc comment that names
   the WordPress action or filter, and call it from the place where WordPress fires that hook. Keep it as narrow as
   the plugin needs.
5. Register the module in `rustpress_plugins::register()` (`crates/rustpress-plugins/src/lib.rs`).
6. Every function that ports a PHP function names it in its doc comment (`` `wpcf7_autop()` ``), so that
   `dev/wp-diff.sh` can find it when the plugin or WordPress changes.
7. Port PHP semantics exactly: loose comparisons (`==`), `(int)` casts of strings, array order, `trim()` character
   sets, PCRE (`fancy-regex` for back references and look-arounds), `esc_html()` without double encoding,
   `json_encode()` flags. `rustpress-core` already has many of these helpers (`formatting`, `kses`, `php_json`,
   `rustpress-phpser` for `serialize()`); reuse them.
8. Translations come from the plugin's language packs (`wp-content/languages/plugins/<domain>-<locale>.mo`) through
   `Site::trd`; the code holds only the English message IDs.

## 5. Compare until equal

- The golden unit tests: `cargo test -p rustpress-plugins`.
- End to end against the reference with the plugin active: fetch the same URLs from both and diff the responses
  (status, headers, body); for writes, also compare the database rows. Write the comparison as a script in `dev/`
  (`dev/svg-upload-compare.py`, `dev/rest-comments-write-compare.py` are examples) so the next person can repeat it.
- The whole site: `cargo run --release -p rustpress-compat -- --report /tmp/with-plugin.md` with the plugin active.
  No unexpected difference may come from the module.
- A difference that cannot be avoided (randomness, outgoing HTTP, a deliberately unsupported part) is written down
  with its reason: in `compat/known.toml` for compat runs, in the PLAN.md section otherwise.

## 6. Clean up and finish

- Deactivate the plugin in the reference again and remove every test post, option, user and upload you created;
  then `cargo test --workspace` must be green with the reference stack running.
- Run `cargo clippy --workspace --all-targets`.
- Update PLAN.md (the state of the stage, what is open), the plugin list in README.md and README.de.md, the website
  (`site/index.html`, `site/de/index.html`) and CHANGELOG.md.
- In `crates/rustpress-server/src/check.rs` nothing changes: `rustpress check` lists every registered module by
  itself.
- Open a pull request that says which release you followed, which goldens and comparisons you ran and what is not
  ported.
