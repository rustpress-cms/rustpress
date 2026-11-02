# Security policy

## Reporting a vulnerability

Please report security issues privately through
[GitHub security advisories](https://github.com/rustpress-cms/rustpress/security/advisories/new), not as a public
issue. Include the version, how RustPress runs (server, Docker, CGI) and the steps to reproduce. You will get an
answer as soon as possible; a fix is released as a new patch version and credited in the advisory unless you prefer
otherwise.

## Supported versions

Only the latest release gets security fixes. RustPress is experimental and has had no external audit.

## Threat model

RustPress replaces the PHP code of WordPress, not its data: it reads and writes the same database and
`wp-content/` folder. What that means for security:

- **No PHP runs.** PHP files in `wp-content/` (themes, plugins, uploads) are never executed by RustPress, so a
  vulnerable plugin or an uploaded `.php` file is not an entry point while RustPress serves the site. If PHP is
  still enabled on the same web space (shared hosting), those files can still be reached through PHP directly;
  block them in the web server.
- **Read-only mode.** With `allow_writes = false` (the default) the database session is `READ ONLY`, sign-in and
  `wp-admin` are closed, and comments cannot be posted. Contact forms can still send e-mail.
- **Write mode.** With `allow_writes = true` RustPress implements WordPress's sign-in, cookies, nonces,
  capabilities, password hashing (including the hashes of older WordPress releases), uploads and the
  administration. These are ports of the WordPress code and are compared with it, but they are new code and the
  most likely place for bugs that matter. Use strong passwords and keep a backup.
- **Keys and salts** come from `wp-config.php`, as in WordPress, so cookies stay valid when switching between
  WordPress and RustPress.
- **TLS** is left to a reverse proxy or the web server. Set `trusted_proxies` only to proxies you control, since
  their `X-Forwarded-*` headers are believed.
- **Bugs in WordPress itself** that RustPress ports exactly are reported to WordPress as well; RustPress follows
  each WordPress release and its security fixes (`dev/wp-diff.sh`, the daily release check).
