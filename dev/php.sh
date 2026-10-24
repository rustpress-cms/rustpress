#!/usr/bin/env bash
# Runs a PHP file inside the reference WordPress container with wp-load.php
# loaded, to ask the real implementation what a function returns.
# Usage (inside WSL): dev/php.sh /path/to/script.php
set -euo pipefail
podman cp "$1" rustpress-wp:/tmp/rustpress-snippet.php
podman exec rustpress-wp php -d display_errors=1 -r 'define("WP_USE_THEMES", false); require "/var/www/html/wp-load.php"; require "/tmp/rustpress-snippet.php";'
