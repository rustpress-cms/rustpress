#!/usr/bin/env bash
# Installs the test plugin with custom post types into the reference WordPress and creates content for it.
# Idempotent through the `rustpress_seeded_types` option.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

if wp option get rustpress_seeded_types >/dev/null 2>&1; then
  echo "types already seeded"; exit 0
fi
podman exec -u 0 rustpress-wp mkdir -p /var/www/html/wp-content/mu-plugins
podman cp mu-plugins/rustpress-test-types.php rustpress-wp:/var/www/html/wp-content/mu-plugins/
podman exec -u 0 rustpress-wp chown -R www-data:www-data /var/www/html/wp-content/mu-plugins

fiction=$(wp term create genre Roman --porcelain)
crime=$(wp term create genre Krimi --parent="$fiction" --porcelain)
wp term create genre Sachbuch >/dev/null
for i in 1 2 3; do
  wp post create --post_type=book --post_title="Buch $i" --post_name="buch-$i" --post_status=publish --post_author=1 \
    --post_excerpt="Auszug zu Buch $i" --post_content="Inhalt von Buch $i mit :) Smiley." >/dev/null
  id=$(wp post list --post_type=book --name="buch-$i" --field=ID)
  wp post term set "$id" genre krimi >/dev/null
  wp post meta add "$id" isbn "978-3-16-14841$i-0" >/dev/null
done
parent=$(wp post create --post_type=event --post_title="Sommerfest" --post_name="sommerfest" --post_status=publish --post_author=1 --post_content="Fest im Sommer." --porcelain)
wp post create --post_type=event --post_title="Aufbau" --post_name="aufbau" --post_parent="$parent" --post_status=publish --post_author=1 --post_content="Aufbau am Vortag." >/dev/null
wp post create --post_type=secret --post_title="Geheimnis" --post_name="geheimnis" --post_status=publish --post_author=1 --post_content="nicht oeffentlich" >/dev/null
wp rewrite flush --hard >/dev/null 2>&1 || wp rewrite flush >/dev/null
wp option update rustpress_seeded_types 1 >/dev/null
echo "types seeded"
