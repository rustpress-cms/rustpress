#!/usr/bin/env bash
# Content with images for the reference WordPress: featured images, a site logo, image
# blocks, a gallery and classic <img> tags. Called by dev/seed.sh after the media import;
# idempotent through the `rustpress_seeded_images` option.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

if wp option get rustpress_seeded_images >/dev/null 2>&1; then
  echo "images already seeded"; exit 0
fi

logo=$(wp post list --post_type=attachment --name=test-logo --field=ID)
photo=$(wp post list --post_type=attachment --name=gebaude-ausen --field=ID)
photo_url=$(wp post get "$photo" --field=guid)   # .../building-exterior.webp
medium_url=${photo_url%.webp}-300x200.webp
logo_url=$(wp post get "$logo" --field=guid)

# Featured images: a WebP with several sizes and a PNG with only the original.
wp post meta update "$(wp post list --name=testbeitrag-1 --field=ID)" _thumbnail_id "$photo" >/dev/null
wp post meta update "$(wp post list --name=testbeitrag-2 --field=ID)" _thumbnail_id "$logo" >/dev/null
wp post meta update "$(wp post list --post_type=page --name=ueber-uns --field=ID)" _thumbnail_id "$photo" >/dev/null

# Image blocks, a gallery and an external image.
cat <<MARKUP | wp post create - --post_title="Bilder" --post_name="bilder" --post_status=publish --post_author=1 >/dev/null
<!-- wp:paragraph -->
<p>Bilder im Inhalt.</p>
<!-- /wp:paragraph -->

<!-- wp:image {"id":$photo,"sizeSlug":"large","linkDestination":"none"} -->
<figure class="wp-block-image size-large"><img src="$photo_url" alt="Fassade" class="wp-image-$photo"/><figcaption class="wp-element-caption">Ein Gebäude</figcaption></figure>
<!-- /wp:image -->

<!-- wp:image {"id":$photo,"width":"300px","sizeSlug":"medium","linkDestination":"none","align":"right"} -->
<figure class="wp-block-image alignright size-medium is-resized"><img src="$medium_url" alt="" class="wp-image-$photo" style="width:300px"/></figure>
<!-- /wp:image -->

<!-- wp:image {"id":$logo,"sizeSlug":"full","linkDestination":"media"} -->
<figure class="wp-block-image size-full"><a href="$logo_url"><img src="$logo_url" alt="Logo" class="wp-image-$logo"/></a></figure>
<!-- /wp:image -->

<!-- wp:gallery {"linkTo":"none"} -->
<figure class="wp-block-gallery has-nested-images columns-default is-cropped"><!-- wp:image {"id":$photo,"sizeSlug":"large","linkDestination":"none"} -->
<figure class="wp-block-image size-large"><img src="$photo_url" alt="" class="wp-image-$photo"/></figure>
<!-- /wp:image -->

<!-- wp:image {"id":$logo,"sizeSlug":"large","linkDestination":"none"} -->
<figure class="wp-block-image size-large"><img src="$logo_url" alt="" class="wp-image-$logo"/></figure>
<!-- /wp:image --></figure>
<!-- /wp:gallery -->

<!-- wp:image -->
<figure class="wp-block-image"><img src="https://example.com/extern.png" alt="Extern"/></figure>
<!-- /wp:image -->
MARKUP

# Classic-editor style content with plain <img> tags (no blocks, so wpautop runs).
printf '<img class="alignnone wp-image-%s size-medium" src="%s" alt="Klassisch" />\n\nText zwischen den Bildern.\n\n<img src="%s" class="wp-image-%s" alt="" width="40" height="9" />\n\n<img src="https://example.com/b.png" alt="">' \
  "$photo" "$medium_url" "$logo_url" "$logo" \
  | wp post create - --post_title="Klassische Bilder" --post_name="klassische-bilder" --post_status=publish --post_author=1 >/dev/null

# Site logo blocks; the logo comes from the custom_logo theme mod.
wp theme mod set custom_logo "$logo" >/dev/null
cat <<MARKUP | wp post create - --post_title="Logo" --post_name="logo" --post_status=publish --post_author=1 >/dev/null
<!-- wp:site-logo /-->

<!-- wp:site-logo {"width":60,"isLink":false,"shouldSyncIcon":false} /-->

<!-- wp:site-logo {"width":120,"linkTarget":"_blank"} /-->
MARKUP

wp option update rustpress_seeded_images 1 >/dev/null
echo "images seeded"
