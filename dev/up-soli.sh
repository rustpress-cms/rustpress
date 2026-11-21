#!/usr/bin/env bash
# Second reference WordPress for a real site: imports a mysqldump into the database `soli_ref`
# of the reference MariaDB (dev/up.sh) and serves it with real WordPress on http://localhost:8081,
# so RustPress can be compared on that site's own content, menus, widgets and theme.
#
#   dev/up-soli.sh <dump.sql> <themes-dir> [theme-slug]
#
# The dump is only read. The site and home URLs of the copy are set to http://localhost:8081, the
# `wordpress` user gets full rights on the copy (WordPress writes options and transients).
set -euo pipefail
dump=$1; themes=$2; theme=${3:-}
db=soli_ref
sql() { podman exec -i rustpress-db mariadb -uroot -prustpress "$@"; }
sql -e "DROP DATABASE IF EXISTS \`$db\`; CREATE DATABASE \`$db\` CHARACTER SET utf8mb4; GRANT ALL ON \`$db\`.* TO 'wordpress'@'%';"
sql "$db" < "$dump"
sql "$db" -e "UPDATE wp_options SET option_value='http://localhost:8081' WHERE option_name IN ('siteurl','home');"
if [ -n "$theme" ]; then
  sql "$db" -e "UPDATE wp_options SET option_value='$theme' WHERE option_name IN ('template','stylesheet');"
fi

podman volume exists rustpress-wp-soli || podman volume create rustpress-wp-soli >/dev/null
if podman container exists rustpress-wp-soli; then
  podman start rustpress-wp-soli >/dev/null
else
  podman run -d --name rustpress-wp-soli --network host \
    -e WORDPRESS_DB_HOST=127.0.0.1 -e WORDPRESS_DB_USER=wordpress \
    -e WORDPRESS_DB_PASSWORD=wordpress -e WORDPRESS_DB_NAME=$db \
    -v rustpress-wp-soli:/var/www/html --entrypoint bash \
    docker.io/library/wordpress:latest \
    -c 'sed -i "s/Listen 80/Listen 127.0.0.1:8081/; s/:80>/:8081>/" /etc/apache2/ports.conf /etc/apache2/sites-available/*.conf && exec docker-entrypoint.sh apache2-foreground' >/dev/null
  for _ in $(seq 1 60); do [ -f "$(podman volume inspect rustpress-wp-soli --format '{{.Mountpoint}}')/wp-config.php" ] && break; sleep 1; done
fi
for t in "$themes"/*/; do
  name=$(basename "$t")
  podman exec rustpress-wp-soli rm -rf "/var/www/html/wp-content/themes/$name"
  podman cp "${t%/}" "rustpress-wp-soli:/var/www/html/wp-content/themes/$name"
done
podman exec rustpress-wp-soli chown -R www-data:www-data /var/www/html/wp-content/themes
echo "reference of the site: http://localhost:8081 (database $db)"
