#!/usr/bin/env bash
# Prints the package lines of a listing in crates/rustpress-plugins/src/catalog.rs for a plugin release:
#   dev/plugin-catalog.sh SLUG VERSION
# It downloads the archive of the release from WordPress.org and prints its URL, SHA-256 and size.
set -euo pipefail
slug=${1:?slug}
version=${2:?version}
url="https://downloads.wordpress.org/plugin/$slug.$version.zip"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
curl -fsSL -o "$tmp" "$url"
echo "        package: \"$url\","
echo "        sha256: \"$(sha256sum "$tmp" | cut -c1-64)\","
echo "        size: $(stat -c %s "$tmp"),"
