# Contributing

Thank you for helping. RustPress has one rule that shapes everything else: **WordPress is the oracle.** A change
is done when RustPress answers exactly like the WordPress release in `WORDPRESS_VERSION`, checked against a real
WordPress, not when the output merely looks right.

- **Read [AGENTS.md](AGENTS.md) first.** It explains the layout, the reference stack, the golden tests and how
  native plugin modules are built. It is written for coding agents and for people alike.
- **Found a difference?** Open a [“Differs from WordPress”](https://github.com/rustpress-cms/rustpress/issues/new?template=difference.yml)
  issue with the URL and both responses.
- **Need a theme or plugin?** Open a [port request](https://github.com/rustpress-cms/rustpress/issues/new?template=port-request.yml),
  or give a 👍 to an existing one. The requests decide what is ported next.
- **Pull requests:** keep them small and say how you compared the result with WordPress (a golden test, a
  script in `dev/`, a `rustpress-compat` run). Before opening one, run:

  ```sh
  cargo test --workspace
  cargo clippy --workspace --all-targets
  ```

  Without the reference stack (`dev/up.sh`), the tests that need it skip themselves; run them locally when you
  change behaviour.
- **Security issues** go through [GitHub security advisories](https://github.com/rustpress-cms/rustpress/security/advisories/new);
  see [SECURITY.md](SECURITY.md).

By contributing you agree that your work is licensed under the GPL-2.0-or-later, like the rest of RustPress.
