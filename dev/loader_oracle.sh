#!/usr/bin/env bash
# Dumps the registered scripts and styles of the reference WordPress (handle, source, dependencies,
# version, arguments and extra data) as JSON for crates/rustpress-admin/tests/loader.rs.
# Usage: dev/loader_oracle.sh > crates/rustpress-admin/tests/fixtures/loader.json
set -euo pipefail
cd "$(dirname "$0")"
cat <<'PHP' | ./wp-eval.sh
$out = array( 'version' => get_bloginfo( 'version' ), 'scripts' => array(), 'styles' => array() );
foreach ( array( 'scripts' => wp_scripts(), 'styles' => wp_styles() ) as $kind => $deps ) {
	foreach ( $deps->registered as $handle => $dep ) {
		$out[ $kind ][] = array(
			'handle' => $handle,
			'src' => $dep->src,
			'deps' => $dep->deps,
			'ver' => $dep->ver,
			'args' => $dep->args,
			'extra' => $dep->extra,
			'textdomain' => isset( $dep->textdomain ) ? $dep->textdomain : null,
			'translations_path' => isset( $dep->translations_path ) ? $dep->translations_path : null,
		);
	}
}
echo wp_json_encode( $out, JSON_UNESCAPED_SLASHES ), "\n";
PHP
