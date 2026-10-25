#!/usr/bin/env bash
# One page per block pattern of the bundled block themes (Twenty Twenty-Two to Twenty Twenty-Five), each
# containing just that pattern. The patterns exercise most core blocks in their saved form; the compat run
# compares the pages of the active theme (the other themes' patterns are not registered and render empty).
# Idempotent through the `rustpress_seeded_patterns` option.
set -euo pipefail
cd "$(dirname "$0")"
themes=/mnt/c/work/claude/rustpress/themes
wp() { ./wp.sh "$@"; }

if wp option get rustpress_seeded_patterns >/dev/null 2>&1; then
  echo "patterns already seeded"; exit 0
fi

slugs=""
for theme in twentytwentyfive twentytwentyfour twentytwentythree; do
  for file in "$themes/$theme"/patterns/*.php; do
    slug=$(grep -m1 -i '^\s*\*\?\s*Slug:' "$file" | sed 's/.*Slug:\s*//' | tr -d '\r ')
    [ -n "$slug" ] && slugs="$slugs $slug"
  done
done
# Twenty Twenty-Two lists its patterns in inc/block-patterns.php.
for name in $(sed -n "/\$block_patterns = array(/,/);/p" "$themes/twentytwentytwo/inc/block-patterns.php" | grep -o "'[a-z0-9-]*'" | tr -d "'"); do
  slugs="$slugs twentytwentytwo/$name"
done

{
  echo '$slugs = explode(" ", trim("'"$slugs"'"));'
  cat <<'PHP'
foreach ( array_unique( $slugs ) as $slug ) {
	$title = str_replace( '/', ' ', $slug );
	$name  = 'pat-' . str_replace( '/', '-', $slug );
	if ( get_page_by_path( $name ) ) { continue; }
	wp_insert_post( array(
		'post_type'    => 'page',
		'post_status'  => 'publish',
		'post_title'   => 'Pattern ' . $title,
		'post_name'    => $name,
		'post_author'  => 1,
		'post_content' => '<!-- wp:pattern {"slug":"' . $slug . '"} /-->',
	) );
}
update_option( 'rustpress_seeded_patterns', 1 );
echo count( $slugs ), " patterns\n";
PHP
} | ./wp-eval.sh
