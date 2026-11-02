# Compatibility reports

Each report is the output of one `rustpress-compat` run: a reference WordPress and an in-process RustPress
answer the same URLs from the same database, and the tool compares status, headers and bodies. Differences
that are understood and accepted are listed with their reason in [`../known.toml`](../known.toml); anything
else counts as unexpected.

| Reports | Reference | Run |
|---|---|---|
| `twentytwentyfive.md`, `twentytwentyfour.md`, `twentytwentythree.md`, `twentytwentytwo.md` | WordPress 7.1.4 | 2026-10-30, the current state |
| `*-ar.md`, `*-de_DE.md`, `*-ja.md`, `*-ru_RU.md` (site languages) | WordPress 7.1.3 | 2026-10-25 |
| `twentytwentyfive-perma-*.md` (permalink structures), `twentytwentyfive-settings-*.md` (reading and discussion settings) | WordPress 7.1.3 | 2026-10-25 |
| `release-twentytwentyfive.md` (the release build) | WordPress 7.1.3 | 2026-10-24 |

The older runs are kept until they are repeated against the current release; they predate several fixes,
so a difference listed there may be gone. The median times are those of one local run of the reference
stack (`dev/up.sh`) and are no benchmark.

What the runs cover: everything the reference lists in its sitemaps and REST API, plus fixed probes such as
feeds, archives, search, 404 pages, redirects and the files of the front page. They run RustPress without
`allow_writes`, so sign-in and `wp-admin` are not part of them; those, the installer and the native plugin
modules are compared with their own scripts and golden tests in [`dev/`](../../dev).

To repeat a run:

```sh
dev/up.sh && dev/seed.sh
cargo run --release -p rustpress-compat -- --report compat/reports/twentytwentyfive.md
```
