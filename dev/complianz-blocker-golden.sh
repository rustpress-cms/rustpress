#!/usr/bin/env bash
# Makes the golden files of the cookie blocker of Complianz: the real plugin runs `replace_tags()` of
# `cmplz_cookie_blocker` on every crates/rustpress-plugins/tests/data/complianz/cases/*.html, once for each
# scenario (a set of settings), and writes tests/data/complianz/blocker-SCENARIO.json with the settings and the
# outputs. The plugin's transient of blocked scripts is dropped first, so the list is computed from the settings.
#
#   dev/complianz-blocker-golden.sh
#
# Needs the reference (dev/up.sh) with complianz-gdpr 7.5.6 active and its wizard finished. It changes the
# plugin's settings; deactivate and remove the plugin afterwards (see PLAN.md, Complianz).
set -euo pipefail
cd "$(dirname "$0")/.."
data=crates/rustpress-plugins/tests/data/complianz
base='"regions": "eu", "enable_cookie_blocker": "yes", "uses_thirdparty_services": "yes", "thirdparty_services_on_site": ["youtube", "google-maps", "google-fonts", "vimeo", "dailymotion", "openstreetmaps", "google-recaptcha"], "uses_social_media": "yes", "socialmedia_on_site": ["facebook", "twitter", "instagram"], "compile_statistics": "google-analytics", "configuration_by_complianz": "yes", "ua_code": "G-TEST123", "safe_mode": false, "dont_use_placeholders": false'
declare -A scenarios=(
  [default]="{$base, \"consent_per_service\": \"no\", \"placeholder_style\": \"minimal\", \"google-maps-format\": \"1280x920\"}"
  [per-service]="{$base, \"consent_per_service\": \"yes\", \"placeholder_style\": \"light\", \"google-maps-format\": \"ratio\"}"
)
cases=$(python3 -c 'import json,sys,pathlib; print(json.dumps({p.stem: p.read_text() for p in sorted(pathlib.Path(sys.argv[1]).glob("*.html"))}))' "$data/cases")
for name in "${!scenarios[@]}"; do
  {
    echo "<?php \$opts = json_decode(<<<'JSON'"; echo "${scenarios[$name]}"; echo "JSON, true);"
    echo "\$cases = json_decode(<<<'JSON'"; echo "$cases"; echo "JSON, true);"
    cat <<'PHP'
wp_set_current_user( 1 );
foreach ( $opts as $k => $v ) {
	cmplz_update_option( $k, $v );
}
foreach ( array( 'cmplz_blocked_scripts' ) as $t ) {
	cmplz_delete_transient( $t );
}
$blocker = COMPLIANZ::$cookie_blocker;
$out     = array();
foreach ( $cases as $case => $html ) {
	$out[ $case ] = $blocker->replace_tags( $html );
}
$transients = get_option( 'cmplz_transients', array() );
$images     = array();
foreach ( $transients as $key => $t ) {
	if ( preg_match( '/^cmplz_(youtube|vimeo|dailymotion|googlemaps)_/', $key ) ) {
		$images[ $key ] = $t['value'];
	}
}
echo wp_json_encode(
	array(
		'options'              => serialize( get_option( 'cmplz_options' ) ),
		'custom_scripts'       => serialize( get_option( 'complianz_options_custom-scripts' ) ),
		'cf7'                  => is_plugin_active( 'contact-form-7/wp-contact-form-7.php' ),
		'plugin_url'           => CMPLZ_URL,
		'upload_url'           => cmplz_upload_url(),
		'placeholder_files'    => array_values( array_diff( scandir( CMPLZ_PATH . 'assets/images/placeholders' ), array( '.', '..' ) ) ),
		'image_transients'     => (object) $images,
		'blocked_content_text' => $blocker->blocked_content_text(),
		'cases'                => $out,
	),
	JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE
) . "\n";
PHP
  } | dev/wp.sh eval-file - > "$data/blocker-$name.json"
  echo "$name: $(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["cases"]))' "$data/blocker-$name.json") cases"
done
