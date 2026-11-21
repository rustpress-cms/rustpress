#!/usr/bin/env bash
# Makes the golden files of the field inputs of Advanced Custom Fields: every case of
# crates/rustpress-plugins/tests/data/acf-render/cases.json (a field, its value, the wrapping element and where the
# instructions go) is printed by the plugin with acf_render_field_wrap() into NAME.html. The field goes through what
# acf_get_field() does to a stored field (acf_validate_field(), the prefix `acf`, acf/load_field); sub fields of a
# group as well. Nonces become NONCE. The CLI user cannot upload files, so ACF uses its basic uploader.
#
#   dev/acf-render-golden.sh
#
# Needs the reference container (dev/up.sh) with advanced-custom-fields in wp-content/plugins and active. Run inside
# WSL.
set -euo pipefail
cd "$(dirname "$0")/.."
cases=crates/rustpress-plugins/tests/data/acf-render
podman exec rustpress-wp rm -rf /tmp/acfcases
podman exec rustpress-wp mkdir -p /tmp/acfcases
podman cp "$cases/cases.json" rustpress-wp:/tmp/acfcases/cases.json
podman exec -i rustpress-wp tee /tmp/acfcases/render.php >/dev/null <<'PHP'
<?php
$_SERVER['HTTP_HOST'] = 'localhost:8080';
$_SERVER['REQUEST_URI'] = '/?ref=golden';
require '/var/www/html/wp-load.php';
function rp_load_field( $raw ) {
	$field = acf_validate_field( $raw );
	$field['prefix'] = 'acf';
	$subs = isset( $raw['sub_fields'] ) ? $raw['sub_fields'] : null;
	$field = apply_filters( 'acf/load_field', $field );
	if ( $subs ) {
		$field['sub_fields'] = array_map( 'rp_load_field', $subs );
	}
	return $field;
}
$cases = json_decode( file_get_contents( '/tmp/acfcases/cases.json' ), true );
foreach ( $cases as $case ) {
	$field = rp_load_field( $case['field'] );
	if ( array_key_exists( 'value', $case ) && null !== $case['value'] ) {
		$field['value'] = $case['value'];
	}
	ob_start();
	acf_render_field_wrap( $field, $case['element'] ?? 'div', $case['instruction'] ?? 'label' );
	// Nonces depend on the salts and the time.
	$html = preg_replace( '/data-nonce="[0-9a-f]{10}"/', 'data-nonce="NONCE"', ob_get_clean() );
	$html = preg_replace( '/(_file_nonce\]" value=")[0-9a-f]{10}"/', '$1NONCE"', $html );
	file_put_contents( '/tmp/acfcases/' . $case['name'] . '.html', $html );
}
PHP
podman exec rustpress-wp php /tmp/acfcases/render.php
for name in $(python3 -c "import json,sys; print(' '.join(c['name'] for c in json.load(open(sys.argv[1]))))" "$cases/cases.json"); do
  podman cp "rustpress-wp:/tmp/acfcases/$name.html" "$cases/$name.html"
done
echo "$(ls "$cases"/*.html | wc -l) cases"
