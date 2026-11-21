#!/usr/bin/env bash
# Makes the golden files of the forms of WPForms Lite: every crates/rustpress-plugins/tests/data/wpforms-forms/NAME.json
# holds a form (`form`: the form data the builder stores, without its ID), the shortcode's `title` and `description`
# and changes to the option `wpforms_settings`. The plugin prints it with do_shortcode( '[wpforms id="..."]' ) as the
# form 9000 (outside the loop, for the request /golden/?ref=golden) into NAME.html.
#
# What is random or depends on the time and the secret key is replaced: the antispam token by TOKEN, its time by TIME,
# the time token by TIMETOKEN. The field the honeypot is put in front of and its label (chosen by array_rand()) are
# written to NAME.hp as `field id|label`, the label of the old honeypot to NAME.hp-legacy.
#
#   dev/wpforms-form-golden.sh
#
# Needs the reference container (dev/up.sh) with wpforms-lite in wp-content/plugins and active.
set -euo pipefail
cd "$(dirname "$0")/.."
cases=crates/rustpress-plugins/tests/data/wpforms-forms
podman exec rustpress-wp rm -rf /tmp/wpfcases
podman exec rustpress-wp mkdir -p /tmp/wpfcases
for f in "$cases"/*.json; do
  podman cp "$f" "rustpress-wp:/tmp/wpfcases/$(basename "$f")"
done
podman exec -i rustpress-wp tee /tmp/wpfcases/render.php >/dev/null <<'PHP'
<?php
// One form per run: the plugin keeps static state (rendered forms, honeypot settings).
$_SERVER['HTTP_HOST'] = 'localhost:8080';
$_SERVER['REQUEST_URI'] = '/golden/?ref=golden';
require '/var/www/html/wp-load.php';
$name = $argv[1];
$dir = '/tmp/wpfcases';
$case = json_decode( file_get_contents( "$dir/$name.json" ), true );
$id = 9000;
// The plugin reads its settings while it loads, so they are changed in a run of their own before this one.
if ( 'settings' === ( $argv[2] ?? '' ) ) {
	$saved = get_option( 'wpforms_settings' );
	file_put_contents( "$dir/saved.ser", serialize( $saved ) );
	update_option( 'wpforms_settings', array_merge( (array) $saved, $case['wpforms_settings'] ?? array() ) );
	exit;
}
if ( 'restore' === ( $argv[2] ?? '' ) ) {
	update_option( 'wpforms_settings', unserialize( file_get_contents( "$dir/saved.ser" ) ) );
	exit;
}
wp_delete_post( $id, true );
// Saved as the builder saves it, by a user who may post unfiltered HTML.
kses_remove_filters();
$form = $case['form'];
$form['id'] = (string) $id;
wp_insert_post( array(
	'import_id'    => $id,
	'post_type'    => 'wpforms',
	'post_status'  => 'publish',
	'post_title'   => 'Golden Form',
	'post_content' => wpforms_encode( $form ),
) );
$atts = sprintf( 'id="%d"', $id );
if ( $case['title'] ) {
	$atts .= ' title="true"';
}
if ( $case['description'] ) {
	$atts .= ' description="true"';
}
try {
	$html = do_shortcode( "[wpforms $atts]" );
} finally {
	wp_delete_post( $id, true );
}
$html = preg_replace( '/data-token="[0-9a-f]{32}"/', 'data-token="TOKEN"', $html );
$html = preg_replace( '/data-token-time="\d+"/', 'data-token-time="TIME"', $html );
$html = preg_replace( '/(name="wpforms\[time_token\]" value=")\d+\.[0-9a-f]{64}"/', '$1TIMETOKEN"', $html );
@unlink( "$dir/$name.hp" );
@unlink( "$dir/$name.hp-legacy" );
if ( preg_match( '/<div id="wpforms-\d+-field_(\d+)-container"\s+class="wpforms-field wpforms-field-text"\s+data-field-type="text".*?<label[^>]*>([^<]*)<\/label>.*?<\/div>\s*<div[^>]* id="wpforms-\d+-field_(\d+)-container"/s', $html, $m ) ) {
	file_put_contents( "$dir/$name.hp", $m[3] . '|' . html_entity_decode( $m[2], ENT_QUOTES ) );
}
if ( preg_match( '/<label for="wpforms-\d+-field-hp" class="wpforms-field-label">([^<]*)</', $html, $m ) ) {
	file_put_contents( "$dir/$name.hp-legacy", $m[1] );
}
file_put_contents( "$dir/$name.html", $html );
PHP
for f in "$cases"/*.json; do
  name=$(basename "$f" .json)
  podman exec rustpress-wp php /tmp/wpfcases/render.php "$name" settings
  podman exec rustpress-wp php /tmp/wpfcases/render.php "$name" || { podman exec rustpress-wp php /tmp/wpfcases/render.php "$name" restore; exit 1; }
  podman exec rustpress-wp php /tmp/wpfcases/render.php "$name" restore
  podman cp "rustpress-wp:/tmp/wpfcases/$name.html" "$cases/$name.html"
  rm -f "$cases/$name.hp" "$cases/$name.hp-legacy"
  for ext in hp hp-legacy; do
    if podman exec rustpress-wp test -e "/tmp/wpfcases/$name.$ext"; then
      podman cp "rustpress-wp:/tmp/wpfcases/$name.$ext" "$cases/$name.$ext"
    fi
  done
done
echo "$(ls "$cases"/*.json | wc -l) cases"
