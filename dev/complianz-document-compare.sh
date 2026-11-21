#!/usr/bin/env bash
# Compares pages with the legal documents of Complianz on the reference (:8080) and RustPress (:3001): a page with the
# `complianz/document` block, one with `[cmplz-document]` and one with `[cmplz-cookies]`, for each scenario of
# crates/rustpress-plugins/tests/data/complianz/documents.json given by name (default: eu us uk). The pages are
# deleted afterwards; the plugin's settings and tables keep the last scenario.
#
#   dev/complianz-document-compare.sh [scenario...]      DIFF=20 prints the first differences
#
# Needs the reference (dev/up.sh) with complianz-gdpr 7.5.6 active, and RustPress on :3001 on the same database.
set -euo pipefail
cd "$(dirname "$0")/.."
golden=crates/rustpress-plugins/tests/data/complianz/documents.json
work=$(mktemp -d)
ids=()
cleanup() {
  for id in "${ids[@]}"; do dev/wp.sh post delete "$id" --force > /dev/null 2>&1 || true; done
  rm -rf "$work"
}
trap cleanup EXIT
[ $# -eq 0 ] && set -- eu us uk
for name in "$@"; do
  python3 -c 'import json,sys; s=[x for x in json.load(open(sys.argv[1])) if x["name"]==sys.argv[2]][0]; s.pop("golden"); json.dump(s, open(sys.argv[3], "w"))' "$golden" "$name" "$work/scenario.json"
  podman cp "$work/scenario.json" rustpress-wp:/tmp/cmplz-scenario.json
  cat > "$work/setup.php" <<'PHP'
<?php
$s = json_decode( file_get_contents( '/tmp/cmplz-scenario.json' ), true );
update_option( 'cmplz_options', $s['options'] );
foreach ( array( 'publish_date', 'documents_update_date', 'last_cookie_sync' ) as $name ) {
	if ( null === $s[ $name ] ) {
		delete_option( "cmplz_$name" );
	} else {
		update_option( "cmplz_$name", $s[ $name ], false );
	}
}
delete_option( 'cmplz_transients' );
delete_transient( 'cmplz_cookies' );
global $wpdb;
$wpdb->query( "DELETE FROM {$wpdb->prefix}cmplz_cookies" );
$wpdb->query( "DELETE FROM {$wpdb->prefix}cmplz_services" );
foreach ( $s['services'] as $row ) {
	$wpdb->insert( "{$wpdb->prefix}cmplz_services", $row + array( 'category' => '', 'sharesData' => 0, 'secondParty' => 0, 'sync' => 1, 'lastUpdatedDate' => 0 ) );
}
foreach ( $s['cookies'] as $row ) {
	$wpdb->insert( "{$wpdb->prefix}cmplz_cookies", $row + array( 'sync' => 1, 'type' => 'cookie', 'collectedPersonalData' => '', 'isOwnDomainCookie' => 0, 'domain' => 'self', 'lastUpdatedDate' => 0, 'lastAddDate' => 0, 'firstAddDate' => 0 ) );
}
PHP
  dev/php.sh "$work/setup.php" > /dev/null
  region=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["options"]["regions"])' "$work/scenario.json")
  suffix=$([ "$region" = eu ] && echo "" || echo "-$region")
  block=$(dev/wp.sh post create --post_type=page --post_status=publish --post_title="Cookie Policy ($name)" --post_name="cmplz-block-$name" \
    --post_content="<!-- wp:complianz/document {\"title\":\"Cookie Policy\",\"selectedDocument\":\"cookie-statement$suffix\"} /-->" --porcelain)
  shortcode=$(dev/wp.sh post create --post_type=page --post_status=publish --post_title="Cookie Policy shortcode ($name)" --post_name="cmplz-shortcode-$name" \
    --post_content="[cmplz-document type=\"cookie-statement\" region=\"$region\"]" --porcelain)
  cookies=$(dev/wp.sh post create --post_type=page --post_status=publish --post_title="Cookies ($name)" --post_name="cmplz-cookies-$name" \
    --post_content="<!-- wp:paragraph --><p>Before</p><!-- /wp:paragraph -->[cmplz-cookies]" --porcelain)
  ids+=("$block" "$shortcode" "$cookies")
  # The pages cache what the plugin finds; RustPress reloads its snapshot every second.
  python3 -c 'import time; time.sleep(2)'
  echo "== $name"
  python3 dev/front-compare.py --anon ${DIFF:+--diff "$DIFF"} "/cmplz-block-$name/" "/cmplz-shortcode-$name/" "/cmplz-cookies-$name/"
  for id in "$block" "$shortcode" "$cookies"; do dev/wp.sh post delete "$id" --force > /dev/null; done
  ids=()
done
