#!/usr/bin/env bash
# Posts with dynamic core blocks and shortcodes for the reference WordPress, to port the
# renderers that were still missing. Idempotent through the `rustpress_seeded_blocks` option.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

if wp option get rustpress_seeded_blocks >/dev/null 2>&1; then
  echo "blocks already seeded"; exit 0
fi

first=$(wp post list --post_type=post --name=testbeitrag-1 --field=ID)
photo=$(wp post list --post_type=attachment --name=gebaude-ausen --field=ID)
photo_url=$(wp post get "$photo" --field=guid)
wp comment create --comment_post_ID="$first" --comment_content="Ein Kommentar fuer die Bloecke." --comment_author=Anna --comment_author_email=anna@example.com --comment_approved=1 >/dev/null || true
wp user meta update 1 description "Autorin mit einer kurzen Biografie." >/dev/null

cat <<'MARKUP' | wp post create - --post_title="Dynamische Bloecke" --post_name="dynamische-bloecke" --post_status=publish --post_author=1 >/dev/null
<!-- wp:archives {"showPostCounts":true} /-->

<!-- wp:archives {"displayAsDropdown":true,"showPostCounts":true,"type":"yearly"} /-->

<!-- wp:categories {"showPostCounts":true,"showHierarchy":true} /-->

<!-- wp:categories {"displayAsDropdown":true,"showOnlyTopLevel":true} /-->

<!-- wp:tag-cloud /-->

<!-- wp:tag-cloud {"showTagCounts":true,"smallestFontSize":"10px","largestFontSize":"30px"} /-->

<!-- wp:latest-comments {"commentsToShow":3,"displayAvatar":false} /-->

<!-- wp:calendar /-->

<!-- wp:home-link {"label":"Start"} /-->

<!-- wp:loginout /-->

<!-- wp:post-comments-count /-->

<!-- wp:post-comments-link /-->

<!-- wp:post-author-biography /-->

<!-- wp:post-time-to-read /-->

<!-- wp:read-more /-->

<!-- wp:shortcode -->
[unknown_shortcode foo="bar"]
<!-- /wp:shortcode -->
MARKUP

cat <<'MARKUP' | wp post create - --post_title="Abfrage-Bloecke" --post_name="abfrage-bloecke" --post_status=publish --post_author=1 >/dev/null
<!-- wp:query {"queryId":1,"query":{"perPage":2,"pages":0,"offset":0,"postType":"post","order":"desc","orderBy":"date","inherit":false}} -->
<div class="wp-block-query"><!-- wp:query-total /-->

<!-- wp:post-template -->
<!-- wp:post-title {"isLink":true} /-->

<!-- wp:post-comments-count /-->

<!-- wp:read-more /-->
<!-- /wp:post-template -->

<!-- wp:query-pagination -->
<!-- wp:query-pagination-previous /-->
<!-- wp:query-pagination-next /-->
<!-- /wp:query-pagination --></div>
<!-- /wp:query -->

<!-- wp:terms-query {"termQuery":{"taxonomy":"category","order":"asc","orderBy":"name","hideEmpty":false,"showNested":false,"perPage":10}} -->
<div class="wp-block-terms-query"><!-- wp:term-template -->
<!-- wp:term-name {"isLink":true} /-->

<!-- wp:term-count /-->

<!-- wp:term-description /-->
<!-- /wp:term-template --></div>
<!-- /wp:terms-query -->
MARKUP

cat <<MARKUP | wp post create - --post_title="Klassischer Inhalt" --post_name="klassischer-inhalt" --post_status=publish --post_author=1 >/dev/null
Ein Absatz mit klassischem Inhalt.

[caption id="attachment_$photo" align="alignright" width="300"]<img src="$photo_url" alt="Fassade" width="300" height="200" class="size-medium wp-image-$photo" /> Eine Bildunterschrift[/caption]

[caption width="200" caption="Alt-Schreibweise"]<img src="$photo_url" alt="" width="200" height="100" />[/caption]

[gallery ids="$photo,$photo" columns="2" size="thumbnail"]

[gallery ids="$photo" link="file" columns="1"]

[unbekannt]Inhalt[/unbekannt] und [kein] Shortcode [] hier.

[[caption]] bleibt wörtlich.
MARKUP
wp option update rustpress_seeded_blocks 1 >/dev/null
echo "blocks seeded"
