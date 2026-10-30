#!/usr/bin/env bash
# Moves RustPress to a new WordPress release: sets WORDPRESS_VERSION (the installer, the update screen and
# the fallbacks read it), the version in the READMEs, and brings the reference WordPress to it.
# Porting the changes themselves is the work listed by dev/wp-diff.sh. Run inside WSL:
#
#   dev/wp-bump.sh 7.1.4
set -euo pipefail
new=${1:?new version}
[[ $new =~ ^[0-9]+(\.[0-9]+){1,2}$ ]] || { echo "not a version: $new" >&2; exit 2; }
cd "$(dirname "$0")/.."

old=$(tr -d '[:space:]' < WORDPRESS_VERSION)
printf '%s\n' "$new" > WORDPRESS_VERSION
sed -i "s/WordPress-${old}_compatible/WordPress-${new}_compatible/; s/WordPress ${old//./\\.}\b/WordPress $new/g" README.md README.de.md
echo "WORDPRESS_VERSION $old -> $new"
dev/wp-reference.sh
