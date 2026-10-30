#!/usr/bin/env bash
# Makes the golden files of the SVG sanitizer of SVG Support: every crates/rustpress-plugins/tests/data/svg-sanitize/*.in.svg
# is cleaned by the PHP library the plugin bundles (enshrined/svg-sanitize, remote references removed), formatted
# (NAME.out) and minified (NAME.min); FALSE marks a file the library refuses or fails on.
#
#   dev/svg-sanitize-golden.sh
#
# Needs the reference container (dev/up.sh) with svg-support in wp-content/plugins. Run inside WSL.
set -euo pipefail
cd "$(dirname "$0")/.."
cases=crates/rustpress-plugins/tests/data/svg-sanitize
podman exec rustpress-wp rm -rf /tmp/svgcases
podman exec rustpress-wp mkdir -p /tmp/svgcases
for f in "$cases"/*.in.svg; do
  podman cp "$f" "rustpress-wp:/tmp/svgcases/$(basename "$f")"
done
podman exec -i rustpress-wp php <<'PHP'
<?php
require '/var/www/html/wp-content/plugins/svg-support/vendor/autoload.php';
use enshrined\svgSanitize\Sanitizer;
foreach ( glob( '/tmp/svgcases/*.in.svg' ) as $file ) {
	$name  = basename( $file, '.in.svg' );
	$dirty = file_get_contents( $file );
	foreach ( array( 'out' => false, 'min' => true ) as $suffix => $minify ) {
		$sanitizer = new Sanitizer();
		$sanitizer->removeRemoteReferences( true );
		$sanitizer->minify( $minify );
		try {
			$clean = $sanitizer->sanitize( $dirty );
		} catch ( \Throwable $e ) {
			$clean = false;
		}
		file_put_contents( "/tmp/svgcases/$name.$suffix", false === $clean ? 'FALSE' : $clean );
	}
}
PHP
for f in "$cases"/*.in.svg; do
  name=$(basename "$f" .in.svg)
  podman cp "rustpress-wp:/tmp/svgcases/$name.out" "$cases/$name.out"
  podman cp "rustpress-wp:/tmp/svgcases/$name.min" "$cases/$name.min"
done
echo "$(ls "$cases"/*.in.svg | wc -l) cases"
