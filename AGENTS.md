# Working on RustPress

Notes for coding agents and new contributors. [README.md](README.md) explains what RustPress is and how to run it.
[PLAN.md](PLAN.md) (in German) holds the plan, the status of every area and what is still open; read the part about
the area you touch before you start.

## The one rule: WordPress is the oracle

RustPress replaces WordPress for an existing database and `wp-content/`, and must answer **byte for byte** like it:
same HTML, headers, feeds, REST JSON, redirects and stored rows. So:

- Port behaviour from the PHP source of the WordPress release in [`WORDPRESS_VERSION`](WORDPRESS_VERSION), or of the
  plugin release a native module follows. Don't write from memory or from documentation.
- Check every change against a real WordPress: a golden test, a compare script in `dev/`, or a `rustpress-compat`
  run. A change counts as done when the output is equal, not when it merely looks right.
- When a difference is unavoidable (randomness, environment, deliberately unsupported features), document it with its
  reason: in [`compat/known.toml`](compat/known.toml) for the compat runs, in PLAN.md for everything else.
- PHP quirks are part of the behaviour: loose comparisons, `(int)` casts of strings, array key order, `trim()`
  character sets, PCRE semantics (`\s` without `/u`, `$` before a trailing newline), `esc_html()` not double
  encoding. The goldens usually find them; keep them in the port.

## Layout

| Path | What |
|---|---|
| `crates/rustpress-*` | the workspace; the table in README.md says what each crate does |
| `crates/rustpress-plugins` | native modules that stand in for WordPress plugins, one module per plugin |
| `compat/` | `known.toml` (accepted differences) and `reports/` (latest compat runs) |
| `dev/` | the reference stack, seeding, golden generators and compare scripts |
| `dev/mu-plugins/` | must-use plugins for the reference (no auto-updates, mail log, test post types, ...) |
| `packaging/`, `Dockerfile*` | release packages and container images |
| `site/` | the project website (GitHub Pages, English and German) |

## Building and testing

```sh
cargo build -p rustpress-server
cargo test --workspace
cargo clippy --workspace --all-targets   # CI fails on new warnings
```

CI runs the same commands with `--locked`. Without the reference stack, the tests that need it print a note and skip
themselves. That is how CI runs them, so a green CI does not mean the goldens were compared. Run them locally when you
change behaviour.

Some tests recurse deeply; if a test overflows its stack, set `RUST_MIN_STACK=16777216`.

## The reference stack

`dev/up.sh` starts MariaDB on `127.0.0.1:3306` (`wordpress`/`wordpress`) and WordPress on `127.0.0.1:8080` in
rootless Podman. It keeps the WordPress files at `WORDPRESS_VERSION` with automatic updates off. `dev/seed.sh` fills in the
test content. The golden tests read `RUSTPRESS_TEST_DB` (default: the database above).

- RustPress and the reference share **one database**. Anything you create for a comparison (posts, options, active
  plugins, uploads) changes what the goldens expect. Remove it afterwards and deactivate plugins again. The core golden
  test of the main query is the first one to fail while test posts exist.
- `dev/php.sh` and `podman exec rustpress-wp php ...` run PHP inside the reference; that is how golden generators ask
  WordPress or a plugin for the expected output (see `dev/cf7-form-golden.sh`, `dev/svg-sanitize-golden.sh`).
- `rustpress-compat` crawls the reference and an in-process RustPress side by side and writes a report:
  `cargo run --release -p rustpress-compat -- --report compat/reports/twentytwentyfive.md`. With `--reference`,
  `--host`, `--db` and `--wp-root` it compares against another WordPress.

## Native plugin modules

PHP plugins never run. An important plugin is rebuilt as a module in `crates/rustpress-plugins`. The module takes over
while the plugin file is listed in `active_plugins`, and the plugin's files stay in `wp-content/plugins` for assets,
headers and language packs.

- **Framework:** `rustpress-core/src/native.rs`.
  - `NativePlugin` holds the plugin file, its version and `load()`, which reads the plugin's data once per snapshot.
  - `ActivePlugin` has the hooks, each with a default that does nothing.
  - Register a new module in `rustpress_plugins::register()`.
- **New hooks:** add one when a plugin needs it, at the place where WordPress fires the action or filter. Give it a
  default and a doc comment naming the WordPress hook.
- **Version:** each module follows exactly one plugin release (`version()`). Compare against that release, installed
  and active in the reference.
- **Workflow:**
  1. Read the plugin's PHP.
  2. Write a golden generator that has the real plugin produce the expected output for many inputs.
  3. Port until all goldens are equal.
  4. Run `rustpress-compat` with the plugin active.
  5. Clean up the test data.

## Following WordPress releases

`dev/wp-diff.sh OLD NEW` shows the changed PHP code and the RustPress files that follow it. `dev/wp-bump.sh NEW`
moves `WORDPRESS_VERSION` and the reference. A daily workflow opens an issue for each new release. Every RustPress
file that ports a WordPress function names it in a doc comment (`` `wp_trim_excerpt()` ``); keep that up, because it
is how the diff report finds what to update.

## Conventions

- **Language:** code, comments, commit messages and PR descriptions are in English. PLAN.md and README.de.md are in
  German.
- **Code:** match the surrounding code in naming, comment density and idiom. Doc comments name the PHP function or
  hook a function stands for and say what differs, if anything. Prefer small, exact ports over general frameworks.
- **Regex:** Rust's `regex` has no back references or look-arounds. Use `fancy-regex` where the PHP pattern needs
  them, and mind PCRE's character classes.
- **Line ends:** text files are LF (`.gitattributes`). Golden data that must keep its bytes is marked `-text`. On
  Windows, write files without newline translation; in Python, use `open(..., newline='')`.
- **Docs:** after a change, update the status in PLAN.md, and the README or website when something user-visible changed.
- **Git:** `main` is protected; every change goes through a pull request with green CI. Releases are tags `v*` and
  build every package and image (`.github/workflows/release.yml`).

## Don'ts

- Don't run PHP in production code paths, and don't add a PHP interpreter.
- Don't "improve" WordPress output (escaping, ordering, whitespace) unless WordPress does the same.
- Don't commit test content, local paths, credentials or dumps of a real site.
- Don't accept a difference without recording it and its reason.
