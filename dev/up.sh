#!/usr/bin/env bash
# Starts the reference stack in rootless Podman (run inside WSL):
#   rustpress-db  MariaDB 11     127.0.0.1:3306  (wordpress/wordpress, root/rustpress)
#   rustpress-wp  WordPress+PHP  127.0.0.1:8080
# Host networking is used because the WSL kernel has no tun device, which
# rootless port publishing (pasta/slirp4netns) needs. Apache listens on 8080
# because rootless processes cannot bind ports below 1024.
set -euo pipefail

start() { # name, run-args...
  local name=$1; shift
  if podman container exists "$name"; then
    podman start "$name" >/dev/null
  else
    podman run -d --name "$name" --network host "$@" >/dev/null
  fi
}

podman volume exists rustpress-db || podman volume create rustpress-db >/dev/null
podman volume exists rustpress-wp || podman volume create rustpress-wp >/dev/null

start rustpress-db \
  -e MARIADB_ROOT_PASSWORD=rustpress -e MARIADB_DATABASE=wordpress \
  -e MARIADB_USER=wordpress -e MARIADB_PASSWORD=wordpress \
  -v rustpress-db:/var/lib/mysql \
  docker.io/library/mariadb:11 --bind-address=127.0.0.1

start rustpress-wp \
  -e WORDPRESS_DB_HOST=127.0.0.1 -e WORDPRESS_DB_USER=wordpress \
  -e WORDPRESS_DB_PASSWORD=wordpress -e WORDPRESS_DB_NAME=wordpress \
  -e WORDPRESS_CONFIG_EXTRA="define( 'AUTOMATIC_UPDATER_DISABLED', true );" \
  -v rustpress-wp:/var/www/html --entrypoint bash \
  docker.io/library/wordpress:latest \
  -c 'sed -i "s/Listen 80/Listen 127.0.0.1:8080/; s/:80>/:8080>/" /etc/apache2/ports.conf /etc/apache2/sites-available/*.conf && exec docker-entrypoint.sh apache2-foreground'

echo "db: 127.0.0.1:3306  wp: http://localhost:8080"
# The image brings the newest WordPress; the files in the volume stay at WORDPRESS_VERSION.
"$(dirname "$0")/wp-reference.sh" || true
