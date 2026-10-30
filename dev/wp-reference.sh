#!/usr/bin/env bash
# Brings the reference WordPress (dev/up.sh) to the release in WORDPRESS_VERSION and keeps it there: the
# must-use plugin rustpress-no-auto-updates.php stops the core and its translations from updating themselves, so the
# comparisons always run against the version RustPress follows (copy it to the WordPress RustPress reads as well).
# Run inside WSL; dev/up.sh calls it on every start.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

want=$(tr -d '[:space:]' < ../WORDPRESS_VERSION)
if ! wp core is-installed 2>/dev/null; then
  echo "reference WordPress is not installed yet: run the installer, then dev/wp-reference.sh" >&2
  exit 1
fi
# AUTOMATIC_UPDATER_DISABLED (used before) also hides the auto-update interface of the administration.
if wp config has AUTOMATIC_UPDATER_DISABLED --type=constant 2>/dev/null; then
  wp config delete AUTOMATIC_UPDATER_DISABLED --type=constant --quiet
fi
podman exec -u 0 rustpress-wp mkdir -p /var/www/html/wp-content/mu-plugins
podman cp mu-plugins/rustpress-no-auto-updates.php rustpress-wp:/var/www/html/wp-content/mu-plugins/
podman exec -u 0 rustpress-wp chown www-data:www-data /var/www/html/wp-content/mu-plugins/rustpress-no-auto-updates.php
have=$(wp core version)
if [[ $have != "$want" ]]; then
  echo "reference WordPress $have -> $want"
  # --force also goes back to an older release.
  wp core update --version="$want" --force
  wp core update-db
  wp language core update
  # Windows reads file times through \\wsl.localhost in steps of 100 ns, which would move the ETag Apache derives from
  # the microseconds by one. The files the update just wrote get whole microseconds.
  podman exec rustpress-wp bash -c 'find /var/www/html -type f -newermt "-15 minutes" -print0 |
    while IFS= read -r -d "" f; do touch -m -d "$(stat -c %y "$f" | cut -c1-26)" "$f"; done'
fi
echo "reference WordPress $(wp core version)"
