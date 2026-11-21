#!/usr/bin/env bash
# Fills the reference WordPress with the test content the golden tests expect (run inside WSL, after
# dev/up.sh): installs WordPress if needed and runs every seed in the order the fixtures were recorded in
# (the fixtures name post IDs). Each seed is idempotent through its own option.
#
# Dates: the fixtures were recorded with content created on 2026-10-24 (archives /2026/10/24/, uploads in
# /2026/10/). A fresh seed is therefore moved to that day: uploads go to 2026/10, and at the end every date
# written during this run is moved so that the installation happened at 2026-10-24 14:00:00, keeping the order of
# everything (one second apart, see the end). Dates a seed sets on purpose (2026-01-01, 2030-01-01) stay.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }
sql() { podman exec -i rustpress-db mariadb -uroot -prustpress wordpress "$@"; }
# Imports a file into uploads/2026/10 whatever the current month is.
media_import() {
  wp --exec='WP_CLI::add_wp_hook("upload_dir", function ($d) { $d["subdir"] = "/2026/10"; $d["path"] = $d["basedir"] . "/2026/10"; $d["url"] = $d["baseurl"] . "/2026/10"; return $d; });' \
    media import "$@"
}

fresh=0
if ! wp core is-installed 2>/dev/null; then
  wp core install --url=http://localhost:8080 --title="RustPress Test" --admin_user=admin --admin_password=admin \
    --admin_email=admin@example.com --skip-email >/dev/null
  fresh=1
fi
# The image brings the newest WordPress: seed with the release in WORDPRESS_VERSION (dev/up.sh could not pin it before
# the installation).
./wp-reference.sh

seed_base() {
wp rewrite structure '/%postname%/' >/dev/null
wp option update posts_per_page 5 >/dev/null
# Public, so that sitemaps and the indexing robots tag are exercised.
wp option update blog_public 1 >/dev/null

# Taxonomies; `term` creates a term or returns the ID of the existing one.
term() { # taxonomy name [extra args]
  local tax=$1 name=$2; shift 2
  wp term create "$tax" "$name" "$@" --porcelain 2>/dev/null \
    || wp term list "$tax" --name="$name" --field=term_id
}
news=$(term category News)
tech=$(term category Tech)
rust=$(term category Rust --parent="$tech")
for t in alpha beta gamma; do term post_tag "$t" >/dev/null; done

# 12 posts so that archives paginate (5 per page). Mix of block markup.
for i in $(seq 1 12); do
  cat <<EOF | wp post create - --post_title="Testbeitrag $i" --post_name="testbeitrag-$i" \
      --post_status=publish --post_category="$news,$rust" --tags_input="alpha,beta" \
      --post_excerpt="Auszug zu Beitrag $i" >/dev/null
<!-- wp:paragraph -->
<p>Erster Absatz von Beitrag $i mit <strong>fett</strong>, <em>kursiv</em> und einem <a href="https://example.com/">Link</a>. Umlaute: äöü ß “Anführungszeichen” – Gedankenstrich…</p>
<!-- /wp:paragraph -->

<!-- wp:heading {"level":3} -->
<h3 class="wp-block-heading">Überschrift $i</h3>
<!-- /wp:heading -->

<!-- wp:list -->
<ul class="wp-block-list"><!-- wp:list-item -->
<li>Punkt eins</li>
<!-- /wp:list-item -->

<!-- wp:list-item -->
<li>Punkt zwei</li>
<!-- /wp:list-item --></ul>
<!-- /wp:list -->

<!-- wp:quote -->
<blockquote class="wp-block-quote"><!-- wp:paragraph -->
<p>Ein Zitat.</p>
<!-- /wp:paragraph --><cite>Jemand</cite></blockquote>
<!-- /wp:quote -->

<!-- wp:columns -->
<div class="wp-block-columns"><!-- wp:column -->
<div class="wp-block-column"><!-- wp:paragraph -->
<p>Links</p>
<!-- /wp:paragraph --></div>
<!-- /wp:column -->

<!-- wp:column -->
<div class="wp-block-column"><!-- wp:paragraph -->
<p>Rechts</p>
<!-- /wp:paragraph --></div>
<!-- /wp:column --></div>
<!-- /wp:columns -->

<!-- wp:separator -->
<hr class="wp-block-separator has-alpha-channel-opacity"/>
<!-- /wp:separator -->

<!-- wp:code -->
<pre class="wp-block-code"><code>fn main() { println!("Beitrag $i"); }</code></pre>
<!-- /wp:code -->
EOF
done

# A classic-editor style post (no blocks) and a draft that must stay hidden.
wp post create --post_title="Klassischer Beitrag" --post_name="klassisch" --post_status=publish \
  --post_content=$'Zeile eins\n\nZeile zwei mit "Anführungszeichen" -- und ...\nUmbruch' >/dev/null
wp post create --post_title="Entwurf" --post_name="entwurf" --post_status=draft --post_content="geheim" >/dev/null
wp post create --post_title="Passwortgeschützt" --post_name="geschuetzt" --post_status=publish \
  --post_password=secret --post_content="versteckt" >/dev/null

# Pages, including a child page.
about=$(wp post create --post_type=page --post_title="Über uns" --post_name="ueber-uns" \
  --post_status=publish --post_content="<!-- wp:paragraph --><p>Über uns.</p><!-- /wp:paragraph -->" --porcelain)
wp post create --post_type=page --post_title="Team" --post_name="team" --post_parent="$about" \
  --post_status=publish --post_content="<!-- wp:paragraph --><p>Das Team.</p><!-- /wp:paragraph -->" >/dev/null

# Meta with PHP-serialized values (the reason rustpress-phpser exists).
first=$(wp post list --name=testbeitrag-1 --field=ID)
wp post meta add "$first" demo_array '{"a":1,"b":[true,null,1.5],"c":"ä"}' --format=json >/dev/null
wp option update demo_option '{"list":[1,2,3],"nested":{"x":"y"}}' --format=json >/dev/null

# Comments (one approved, one pending).
wp comment create --comment_post_ID="$first" --comment_content="Guter Beitrag" \
  --comment_author="Gast" --comment_author_email="gast@example.com" --comment_approved=1 >/dev/null
wp comment create --comment_post_ID="$first" --comment_content="Wartet auf Freigabe" \
  --comment_author="Spam" --comment_author_email="x@example.com" --comment_approved=0 >/dev/null

# A sticky post (moves to the top of the first blog page) and a multi-page post.
wp option update sticky_posts "[$(wp post list --name=testbeitrag-3 --field=ID)]" --format=json >/dev/null
# Two posts with identical dates: shows how WordPress orders ties (newest ID first).
for n in a b; do
  wp post create --post_title="Gleich ${n^^}" --post_name="gleich-$n" --post_status=publish \
    --post_date="2026-01-01 12:00:00" --post_content=x >/dev/null
done
printf 'Seite eins\n<!--nextpage-->\nSeite zwei\n<!--nextpage-->\nSeite drei' \
  | wp post create - --post_title="Mehrseitig" --post_name="mehrseitig" --post_status=publish >/dev/null

# wp-cli creates posts without an author; give them to the admin like the UI would.
sql -e "UPDATE wp_posts SET post_author = 1 WHERE post_author = 0 AND post_type IN ('post', 'page')"

wp option update rustpress_seeded 1 >/dev/null
echo "seeded"
}

if wp option get rustpress_seeded >/dev/null 2>&1; then
  echo "base content already seeded"
else
  seed_base
fi

# Two attachments for the media endpoint: an unattached PNG (a single size) and a WebP
# attached to the first post (several sizes, so that srcset has something to choose from).
if [ -z "$(wp post list --post_type=attachment --name=test-logo --field=ID)" ]; then
  media_import /var/www/html/wp-admin/images/wordpress-logo.png --title="Test Logo" --caption="Eine Bildunterschrift" --alt="Logo Alt" >/dev/null
  media_import /var/www/html/wp-content/themes/twentytwentyfour/assets/images/building-exterior.webp --post_id=1 --title="Gebäude außen" --caption="Ein Gebäude & mehr" --alt="Fassade" --desc="Beschreibung des Bildes" >/dev/null
fi

./seed_images.sh
./seed_images2.sh
./seed_blocks.sh
./seed_patterns.sh
./seed_content2.sh
./seed_types.sh
./seed_blocks2.sh
./seed_blocks3.sh

# Moves a fresh installation to 2026-10-24 (see the top). Only the dates written since the installation
# are moved; a database that was seeded before keeps its dates. They are not shifted but laid out again one second
# apart in the order they were written (post dates, comment dates and changed modification dates, ties by ID):
# WP-CLI writes several rows in the same second, and rows with the same date have no defined order in MariaDB, so
# RustPress and WordPress could list them differently (feeds, archives). Ties a seed makes on purpose (2026-01-01)
# lie outside the installation's dates and stay.
if [ "$fresh" = 1 ]; then
  sql <<'SQL'
SET @from = (SELECT post_date FROM wp_posts WHERE ID = 1);
SET @offset = TIMESTAMPDIFF(SECOND, (SELECT post_date_gmt FROM wp_posts WHERE ID = 1), @from);
SET @to = DATE_ADD(UTC_TIMESTAMP(), INTERVAL 1 DAY);
SET @base = '2026-10-24 14:00:00';
SET @shift = TIMESTAMPDIFF(SECOND, @from, @base);
DROP TABLE IF EXISTS rustpress_seed_dates;
CREATE TABLE rustpress_seed_dates AS
  SELECT kind, id, same, DATE_ADD(@base, INTERVAL ROW_NUMBER() OVER (ORDER BY d, kind, id) - 1 SECOND) AS at FROM (
    SELECT 0 AS kind, ID AS id, post_modified = post_date AS same, post_date AS d FROM wp_posts WHERE post_date BETWEEN @from AND @to
    UNION ALL SELECT 1, comment_ID, 0, comment_date FROM wp_comments WHERE comment_date BETWEEN @from AND @to
    UNION ALL SELECT 2, ID, 0, post_modified FROM wp_posts
      WHERE post_modified BETWEEN @from AND @to AND NOT (post_modified = post_date AND post_date BETWEEN @from AND @to)
  ) AS written;
UPDATE wp_posts p JOIN rustpress_seed_dates e ON e.kind = 0 AND e.id = p.ID
  SET p.post_modified = IF(e.same, e.at, p.post_modified), p.post_modified_gmt = IF(e.same, DATE_SUB(e.at, INTERVAL @offset SECOND), p.post_modified_gmt),
      p.post_date = e.at, p.post_date_gmt = DATE_SUB(e.at, INTERVAL @offset SECOND);
UPDATE wp_posts p JOIN rustpress_seed_dates e ON e.kind = 2 AND e.id = p.ID
  SET p.post_modified = e.at, p.post_modified_gmt = DATE_SUB(e.at, INTERVAL @offset SECOND);
UPDATE wp_comments c JOIN rustpress_seed_dates e ON e.kind = 1 AND e.id = c.comment_ID
  SET c.comment_date = e.at, c.comment_date_gmt = DATE_SUB(e.at, INTERVAL @offset SECOND);
DROP TABLE rustpress_seed_dates;
UPDATE wp_users SET user_registered = DATE_ADD(user_registered, INTERVAL @shift SECOND)
  WHERE user_registered BETWEEN DATE_SUB(@from, INTERVAL @offset SECOND) AND @to;
SQL
  echo "dates moved to 2026-10-24"
fi
