#!/usr/bin/env bash
# Runs WP-CLI against the reference WordPress (see dev/up.sh).
# Usage (from WSL): dev/wp.sh option get siteurl
exec podman run --rm -i --network host --user 33:33 \
  -e WORDPRESS_DB_HOST=127.0.0.1 -e WORDPRESS_DB_USER=wordpress \
  -e WORDPRESS_DB_PASSWORD=wordpress -e WORDPRESS_DB_NAME=wordpress \
  -v rustpress-wp:/var/www/html \
  docker.io/library/wordpress:cli wp --path=/var/www/html "$@"
