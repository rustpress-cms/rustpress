#!/usr/bin/env bash
# Posts with the newer core blocks that change their markup on the server (video with metadata,
# accordion), for the reference WordPress. The video attachment is made up: no file, only the
# metadata WordPress would have read from an upload.
# Idempotent through the `rustpress_seeded_blocks3` option.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

seed_main=1
if wp option get rustpress_seeded_blocks3 >/dev/null 2>&1; then
  seed_main=0
fi

if [ "$seed_main" = 1 ]; then
video=$(wp post create --post_type=attachment --post_mime_type=video/mp4 --post_title=film --post_status=inherit --porcelain)
wp post meta update "$video" _wp_attached_file 2026/10/film.mp4 >/dev/null
wp post meta update "$video" _wp_attachment_metadata '{"filesize":1234,"mime_type":"video/mp4","length":3,"width":640,"height":360}' --format=json >/dev/null
video_url=$(wp eval "echo wp_get_attachment_url($video);")

wp post create - --post_title="Video und Co" --post_name="video-und-co" --post_status=publish --post_author=1 --porcelain >/dev/null <<MARKUP
<!-- wp:video {"id":$video} -->
<figure class="wp-block-video"><video controls src="$video_url"></video><figcaption class="wp-element-caption">Ein Film</figcaption></figure>
<!-- /wp:video -->

<!-- wp:video -->
<figure class="wp-block-video"><video controls src="https://example.com/ohne-id.mp4"></video></figure>
<!-- /wp:video -->
MARKUP

wp post create - --post_title="Akkordeon" --post_name="akkordeon" --post_status=publish --post_author=1 --porcelain >/dev/null <<'MARKUP'
<!-- wp:accordion -->
<div role="group" class="wp-block-accordion"><!-- wp:accordion-item -->
<div class="wp-block-accordion-item"><!-- wp:accordion-heading -->
<h3 class="wp-block-accordion-heading"><button type="button" class="wp-block-accordion-heading__toggle"><span class="wp-block-accordion-heading__toggle-title">Frage eins</span><span class="wp-block-accordion-heading__toggle-icon" aria-hidden="true">+</span></button></h3>
<!-- /wp:accordion-heading --><!-- wp:accordion-panel -->
<div role="region" class="wp-block-accordion-panel"><!-- wp:paragraph -->
<p>Antwort eins</p>
<!-- /wp:paragraph --></div>
<!-- /wp:accordion-panel --></div>
<!-- /wp:accordion-item --><!-- wp:accordion-item {"openByDefault":true} -->
<div class="wp-block-accordion-item is-open"><!-- wp:accordion-heading -->
<h3 class="wp-block-accordion-heading"><button type="button" class="wp-block-accordion-heading__toggle"><span class="wp-block-accordion-heading__toggle-title">Frage zwei</span><span class="wp-block-accordion-heading__toggle-icon" aria-hidden="true">+</span></button></h3>
<!-- /wp:accordion-heading --><!-- wp:accordion-panel -->
<div role="region" class="wp-block-accordion-panel"><!-- wp:paragraph -->
<p>Antwort zwei</p>
<!-- /wp:paragraph --></div>
<!-- /wp:accordion-panel --></div>
<!-- /wp:accordion-item --></div>
<!-- /wp:accordion -->
MARKUP

wp option update rustpress_seeded_blocks3 1 >/dev/null
echo "blocks3 seeded"
fi

if ! wp option get rustpress_seeded_tabs >/dev/null 2>&1; then
wp post create - --post_title="Reiter" --post_name="reiter" --post_status=publish --post_author=1 --porcelain >/dev/null <<'MARKUP'
<!-- wp:tabs -->
<div class="wp-block-tabs"><!-- wp:tab-list -->
<div role="tablist" class="wp-block-tab-list"><button type="button" role="tab">Eins</button><button type="button" role="tab">Zwei</button></div>
<!-- /wp:tab-list -->

<!-- wp:tab-panels -->
<div class="wp-block-tab-panels"><!-- wp:tab-panel -->
<section role="tabpanel" tabindex="0" class="wp-block-tab-panel"><!-- wp:paragraph -->
<p>Inhalt eins</p>
<!-- /wp:paragraph --></section>
<!-- /wp:tab-panel -->

<!-- wp:tab-panel {"anchor":"zwei"} -->
<section role="tabpanel" tabindex="0" class="wp-block-tab-panel" id="zwei"><!-- wp:paragraph -->
<p>Inhalt zwei</p>
<!-- /wp:paragraph --></section>
<!-- /wp:tab-panel --></div>
<!-- /wp:tab-panels --></div>
<!-- /wp:tabs -->
MARKUP
wp option update rustpress_seeded_tabs 1 >/dev/null
echo "tabs seeded"
fi

if ! wp option get rustpress_seeded_icon >/dev/null 2>&1; then
wp post create - --post_title="Symbole" --post_name="symbole" --post_status=publish --post_author=1 --porcelain >/dev/null <<'MARKUP'
<!-- wp:icon {"icon":"core/arrow-down"} /-->

<!-- wp:icon {"icon":"core/arrow-right","ariaLabel":"Weiter","flipHorizontal":true,"rotation":90,"textColor":"accent-1","backgroundColor":"accent-5","style":{"border":{"radius":"8px","width":"2px","style":"solid"},"spacing":{"padding":{"top":"4px","bottom":"4px","left":"4px","right":"4px"}},"dimensions":{"width":"48px"}}} /-->

<!-- wp:icon {"icon":"core/gibt-es-nicht"} /-->

<!-- wp:icon /-->
MARKUP
wp option update rustpress_seeded_icon 1 >/dev/null
echo "icon seeded"
fi
