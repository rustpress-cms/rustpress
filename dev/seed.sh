#!/usr/bin/env bash
# Fills the reference WordPress with test content (run inside WSL, after dev/up.sh
# and the web installer). Idempotent: does nothing if the marker option exists.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

if wp option get rustpress_seeded >/dev/null 2>&1; then
  echo "already seeded"; exit 0
fi

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
podman exec rustpress-db mariadb -uroot -prustpress wordpress \
  -e "UPDATE wp_posts SET post_author = 1 WHERE post_author = 0 AND post_type IN ('post', 'page')"

wp option update rustpress_seeded 1 >/dev/null
echo "seeded"

# Two attachments for the media endpoint: an unattached PNG (a single size) and a WebP
# attached to the first post (several sizes, so that srcset has something to choose from).
if [ -z "$(wp post list --post_type=attachment --name=test-logo --field=ID)" ]; then
  wp media import /var/www/html/wp-admin/images/wordpress-logo.png --title="Test Logo" --caption="Eine Bildunterschrift" --alt="Logo Alt" >/dev/null
  wp media import /var/www/html/wp-content/themes/twentytwentyfour/assets/images/building-exterior.webp --post_id=1 --title="Gebäude außen" --caption="Ein Gebäude & mehr" --alt="Fassade" --desc="Beschreibung des Bildes" >/dev/null
fi
