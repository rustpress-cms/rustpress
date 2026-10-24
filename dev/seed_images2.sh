#!/usr/bin/env bash
# More image scenarios for the reference WordPress: many images on one page (the lazy-loading
# threshold), cover and media-text blocks, query loops with featured images and a database
# template with an inheriting query. Called by dev/seed.sh after dev/seed_images.sh; idempotent
# through the `rustpress_seeded_images2` option.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

if wp option get rustpress_seeded_images2 >/dev/null 2>&1; then
  echo "images2 already seeded"; exit 0
fi

logo=$(wp post list --post_type=attachment --name=test-logo --field=ID)
photo=$(wp post list --post_type=attachment --name=gebaude-ausen --field=ID)
photo_url=$(wp post get "$photo" --field=guid)
logo_url=$(wp post get "$logo" --field=guid)

# Featured images on more posts, so that lists have something to show.
for n in 3 4 5 6; do
  id=$(wp post list --name="testbeitrag-$n" --field=ID)
  if [ $((n % 2)) -eq 1 ]; then thumb=$photo; else thumb=$logo; fi
  wp post meta update "$id" _thumbnail_id "$thumb" >/dev/null
done

# Five image blocks in a row: only the first three are exempt from lazy loading.
{
  for i in 1 2 3 4 5; do
    printf '<!-- wp:image {"id":%s,"sizeSlug":"full","linkDestination":"none"} -->\n<figure class="wp-block-image size-full"><img src="%s" alt="Bild %s" class="wp-image-%s"/></figure>\n<!-- /wp:image -->\n\n' "$photo" "$photo_url" "$i" "$photo"
  done
} | wp post create - --post_title="Viele Bilder" --post_name="viele-bilder" --post_status=publish --post_author=1 >/dev/null

# Cover and media-text blocks with images.
cat <<MARKUP | wp post create - --post_title="Cover Bilder" --post_name="cover-bilder" --post_status=publish --post_author=1 >/dev/null
<!-- wp:cover {"url":"$photo_url","id":$photo,"dimRatio":50,"isDark":false} -->
<div class="wp-block-cover is-light"><span aria-hidden="true" class="wp-block-cover__background has-background-dim"></span><img class="wp-block-cover__image-background wp-image-$photo" alt="" src="$photo_url" data-object-fit="cover"/><div class="wp-block-cover__inner-container"><!-- wp:paragraph {"align":"center","fontSize":"large"} -->
<p class="has-text-align-center has-large-font-size">Im Cover</p>
<!-- /wp:paragraph --></div></div>
<!-- /wp:cover -->

<!-- wp:media-text {"mediaId":$photo,"mediaLink":"$photo_url","mediaType":"image"} -->
<div class="wp-block-media-text is-stacked-on-mobile"><figure class="wp-block-media-text__media"><img src="$photo_url" alt="Medientext" class="wp-image-$photo size-full"/></figure><div class="wp-block-media-text__content"><!-- wp:paragraph {"placeholder":"Inhalt"} -->
<p>Text neben dem Bild</p>
<!-- /wp:paragraph --></div></div>
<!-- /wp:media-text -->
MARKUP

# Query loops with featured images: one that builds its own query, one that inherits the
# main query (used by the category template below).
loop_markup() { # inherit
  cat <<MARKUP
<!-- wp:query {"queryId":0,"query":{"perPage":3,"pages":0,"offset":0,"postType":"post","order":"desc","orderBy":"date","author":"","search":"","exclude":[],"sticky":"","inherit":$1}} -->
<div class="wp-block-query"><!-- wp:post-template -->
<!-- wp:post-featured-image {"isLink":true,"aspectRatio":"16/9"} /-->

<!-- wp:post-title {"isLink":true} /-->
<!-- /wp:post-template --></div>
<!-- /wp:query -->
MARKUP
}
loop_markup false | wp post create - --post_type=page --post_title="Bildliste" --post_name="bildliste" --post_status=publish --post_author=1 >/dev/null

# A `category` template stored in the database, with an inheriting query loop.
template_content=$(cat <<MARKUP
<!-- wp:template-part {"slug":"header","tagName":"header"} /-->

<!-- wp:group {"tagName":"main","layout":{"type":"constrained"}} -->
<main class="wp-block-group">$(loop_markup true)</main>
<!-- /wp:group -->

<!-- wp:template-part {"slug":"footer","tagName":"footer"} /-->
MARKUP
)
tid=$(wp post create --post_type=wp_template --post_name=category --post_title="Category" --post_status=publish --post_content="$template_content" --porcelain)
wp term create wp_theme twentytwentyfive >/dev/null 2>&1 || true
wp post term set "$tid" wp_theme twentytwentyfive >/dev/null

wp option update rustpress_seeded_images2 1 >/dev/null
echo "images2 seeded"
