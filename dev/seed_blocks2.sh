#!/usr/bin/env bash
# A post with the core blocks that change their saved markup on the server (button, cover, details,
# media-text, footnotes, ...) and the plain ones, for the reference WordPress.
# Idempotent through the `rustpress_seeded_blocks2` option.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

if wp option get rustpress_seeded_blocks2 >/dev/null 2>&1; then
  echo "blocks2 already seeded"; exit 0
fi

photo=$(wp post list --post_type=attachment --name=gebaude-ausen --field=ID)
photo_url=$(wp post get "$photo" --field=guid)

post=$(cat <<MARKUP | wp post create - --post_title="Mehr Bloecke" --post_name="mehr-bloecke" --post_status=publish --post_author=1 --porcelain
<!-- wp:buttons -->
<div class="wp-block-buttons"><!-- wp:button -->
<div class="wp-block-button"><a class="wp-block-button__link wp-element-button" href="https://example.com/">Los</a></div>
<!-- /wp:button -->

<!-- wp:button {"width":50} -->
<div class="wp-block-button has-custom-width wp-block-button__width-50"><a class="wp-block-button__link wp-element-button" href="https://example.com/halb">Halb</a></div>
<!-- /wp:button -->

<!-- wp:button -->
<div class="wp-block-button"><a class="wp-block-button__link wp-element-button"></a></div>
<!-- /wp:button -->

<!-- wp:button {"style":{"dimensions":{"width":"40%"}}} -->
<div class="wp-block-button"><a class="wp-block-button__link wp-element-button" href="https://example.com/vierzig">Vierzig</a></div>
<!-- /wp:button -->

<!-- wp:button {"tagName":"button","type":"submit"} -->
<div class="wp-block-button"><button type="submit" class="wp-block-button__link wp-element-button">Senden</button></div>
<!-- /wp:button --></div>
<!-- /wp:buttons -->

<!-- wp:cover {"url":"$photo_url","id":$photo,"dimRatio":50} -->
<div class="wp-block-cover"><span aria-hidden="true" class="wp-block-cover__background has-background-dim"></span><img class="wp-block-cover__image-background wp-image-$photo" alt="" src="$photo_url" data-object-fit="cover"/><div class="wp-block-cover__inner-container"><!-- wp:paragraph {"align":"center","placeholder":"Titel"} -->
<p class="has-text-align-center">Text im Cover</p>
<!-- /wp:paragraph --></div></div>
<!-- /wp:cover -->

<!-- wp:cover {"useFeaturedImage":true,"dimRatio":30} -->
<div class="wp-block-cover"><span aria-hidden="true" class="wp-block-cover__background has-background-dim-30 has-background-dim"></span><div class="wp-block-cover__inner-container"><!-- wp:paragraph -->
<p>Beitragsbild als Hintergrund</p>
<!-- /wp:paragraph --></div></div>
<!-- /wp:cover -->

<!-- wp:details -->
<details class="wp-block-details"><summary>Geschlossen</summary><!-- wp:paragraph -->
<p>Versteckter Text</p>
<!-- /wp:paragraph --><!-- wp:image {"id":$photo,"sizeSlug":"large","linkDestination":"none"} -->
<figure class="wp-block-image size-large"><img src="$photo_url" alt="" class="wp-image-$photo"/></figure>
<!-- /wp:image --></details>
<!-- /wp:details -->

<!-- wp:details {"showContent":true} -->
<details class="wp-block-details" open><summary>Offen</summary><!-- wp:paragraph -->
<p>Sichtbarer Text</p>
<!-- /wp:paragraph --></details>
<!-- /wp:details -->

<!-- wp:media-text {"mediaId":$photo,"mediaLink":"$photo_url","mediaType":"image"} -->
<div class="wp-block-media-text is-stacked-on-mobile"><figure class="wp-block-media-text__media"><img src="$photo_url" alt="" class="wp-image-$photo size-full"/></figure><div class="wp-block-media-text__content"><!-- wp:paragraph {"placeholder":"Inhalt"} -->
<p>Text neben dem Bild</p>
<!-- /wp:paragraph --></div></div>
<!-- /wp:media-text -->

<!-- wp:media-text {"mediaPosition":"right","useFeaturedImage":true,"imageFill":true,"focalPoint":{"x":0.25,"y":0.75}} -->
<div class="wp-block-media-text has-media-on-the-right is-stacked-on-mobile is-image-fill"><figure class="wp-block-media-text__media"></figure><div class="wp-block-media-text__content"><!-- wp:paragraph -->
<p>Beitragsbild rechts</p>
<!-- /wp:paragraph --></div></div>
<!-- /wp:media-text -->

<!-- wp:paragraph -->
<p>Text mit Fussnote<sup data-fn="a1b2c3d4-0000-4000-8000-000000000001" class="fn"><a href="#a1b2c3d4-0000-4000-8000-000000000001" id="a1b2c3d4-0000-4000-8000-000000000001-link">1</a></sup>.</p>
<!-- /wp:paragraph -->

<!-- wp:footnotes /-->

<!-- wp:breadcrumbs /-->

<!-- wp:quote -->
<blockquote class="wp-block-quote"><!-- wp:paragraph -->
<p>Ein Zitat.</p>
<!-- /wp:paragraph --><cite>Jemand</cite></blockquote>
<!-- /wp:quote -->

<!-- wp:pullquote -->
<figure class="wp-block-pullquote"><blockquote><p>Ein Zitat im Vordergrund</p><cite>Jemand anderes</cite></blockquote></figure>
<!-- /wp:pullquote -->

<!-- wp:table -->
<figure class="wp-block-table"><table class="has-fixed-layout"><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody><tfoot><tr><td>x</td><td>y</td></tr></tfoot></table><figcaption class="wp-element-caption">Tabelle</figcaption></figure>
<!-- /wp:table -->

<!-- wp:verse -->
<pre class="wp-block-verse">Zeile eins
Zeile zwei</pre>
<!-- /wp:verse -->

<!-- wp:code -->
<pre class="wp-block-code"><code>fn main() {}</code></pre>
<!-- /wp:code -->

<!-- wp:preformatted -->
<pre class="wp-block-preformatted">Vorformatiert  mit   Leerzeichen</pre>
<!-- /wp:preformatted -->

<!-- wp:separator -->
<hr class="wp-block-separator has-alpha-channel-opacity"/>
<!-- /wp:separator -->

<!-- wp:spacer {"height":"40px"} -->
<div style="height:40px" aria-hidden="true" class="wp-block-spacer"></div>
<!-- /wp:spacer -->

<!-- wp:list {"ordered":true,"start":3} -->
<ol start="3" class="wp-block-list"><!-- wp:list-item -->
<li>Drei</li>
<!-- /wp:list-item --><!-- wp:list-item -->
<li>Vier</li>
<!-- /wp:list-item --></ol>
<!-- /wp:list -->

<!-- wp:audio -->
<figure class="wp-block-audio"><audio controls src="http://localhost:8080/wp-content/uploads/2026/10/ton.mp3"></audio><figcaption class="wp-element-caption">Ton</figcaption></figure>
<!-- /wp:audio -->

<!-- wp:file {"href":"http://localhost:8080/wp-content/uploads/2026/10/datei.pdf"} -->
<div class="wp-block-file"><a href="http://localhost:8080/wp-content/uploads/2026/10/datei.pdf">Datei</a><a href="http://localhost:8080/wp-content/uploads/2026/10/datei.pdf" class="wp-block-file__button wp-element-button" download>Herunterladen</a></div>
<!-- /wp:file -->

<!-- wp:html -->
<div class="eigenes">Eigenes <b>HTML</b></div>
<!-- /wp:html -->

<!-- wp:embed {"url":"https://example.com/video","type":"video","providerNameSlug":"example"} -->
<figure class="wp-block-embed is-type-video is-provider-example wp-block-embed-example"><div class="wp-block-embed__wrapper">
https://example.com/video
</div></figure>
<!-- /wp:embed -->

<!-- wp:more -->
<!--more-->
<!-- /wp:more -->

<!-- wp:paragraph -->
<p>Nach dem Mehr.</p>
<!-- /wp:paragraph -->
MARKUP
)
wp post meta update "$post" _thumbnail_id "$photo" >/dev/null
wp post meta update "$post" footnotes '[{"content":"Die Fussnote","id":"a1b2c3d4-0000-4000-8000-000000000001"}]' >/dev/null
wp option update rustpress_seeded_blocks2 1 >/dev/null
echo "blocks2 seeded"
