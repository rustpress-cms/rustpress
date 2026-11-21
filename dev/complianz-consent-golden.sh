#!/usr/bin/env bash
# Makes the golden file of the consent area of Complianz: the real plugin renders `[cmplz-consent-area]`
# (`show_consent_area()`) for many attributes, cookies and consent types, the block `complianz/consent-area`
# (`cmplz_render_consent_area_block()`) for many attributes, and finds the consented content of posts the way
# `cmplz_rest_consented_content()` does. crates/rustpress-plugins/tests/data/complianz/consent.json keeps the
# inputs and the outputs.
#
#   dev/complianz-consent-golden.sh
#
# Needs the reference (dev/up.sh) with complianz-gdpr 7.5.6 active. It changes the plugin's settings; deactivate and
# remove the plugin afterwards (see PLAN.md, Complianz).
set -euo pipefail
cd "$(dirname "$0")/.."
out=crates/rustpress-plugins/tests/data/complianz/consent.json
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cat > "$work/golden.php" <<'PHP'
<?php
$cases = array();
$attributes = array(
	array(),
	array( 'service' => 'youtube' ),
	array( 'service' => 'Google Maps' ),
	array( 'service' => 'vimeo.com', 'text' => 'Accept <b>Vimeo</b> please' ),
	array( 'category' => 'statistics' ),
	array( 'category' => 'preferences', 'text' => '' ),
	array( 'category' => 'bogus' ),
	array( 'category' => '0', 'service' => '0' ),
	array( 'id' => 'My Area', 'service' => 'youtube', 'cache_redirect' => 'true' ),
	array( 'category' => 'marketing', 'cache_redirect' => '1', 'scroll_into_view' => 'false' ),
	array( 'service' => 'youtube', 'cache_redirect' => '1', 'scroll_into_view' => '1' ),
	array( 'service' => 'youtube', 'scroll_into_view' => 'true', 'cache_redirect' => 'yes' ),
	array( 'clientside' => '1' ),
	array( 'clientside' => 'true', 'service' => 'Google Maps', 'category' => 'statistics', 'id' => 'x_Y' ),
	array( 'clientside' => '1', 'category' => 'bogus', 'text' => "Two\nlines <i>x</i>" ),
	array( 'clientside' => '0', 'service' => 'youtube' ),
	array( 'TEXT' => 'Upper case key', 'Service' => 'facebook' ),
);
$cookie_sets = array(
	array(),
	array( 'cmplz_marketing' => 'allow' ),
	array( 'cmplz_statistics' => 'allow', 'cmplz_preferences' => 'deny' ),
	array( 'cmplz_consented_services' => '{\"youtube\":1}' ),
	array( 'cmplz_consented_services' => '{"youtube":"1","facebook":true,"Google-Maps":0}' ),
	array( 'cmplz_no-category' => 'allow' ),
	array( 'cmplz_marketing' => 'deny', 'cmplz_statistics' => 'deny', 'cmplz_preferences' => 'deny' ),
);
$regions = array( 'eu', 'us' );
foreach ( $regions as $region ) {
	$options = get_option( 'cmplz_options', array() );
	$options['regions'] = $region;
	update_option( 'cmplz_options', $options );
	$consent_type = COMPLIANZ::$company->get_default_consenttype();
	foreach ( $attributes as $atts ) {
		foreach ( $cookie_sets as $cookies ) {
			foreach ( array( 0, 42 ) as $post_id ) {
				$_COOKIE = $cookies;
				$GLOBALS['post'] = $post_id ? new WP_Post( (object) array( 'ID' => $post_id ) ) : null;
				$content = '<p>Consented <strong>content</strong></p>';
				$level = ob_get_level();
				$html = COMPLIANZ::$document->show_consent_area( $atts, $content, 'cmplz-consent-area' );
				$cases[] = array(
					'kind'         => 'shortcode',
					'consent_type' => $consent_type,
					'atts'         => (object) $atts,
					'cookies'      => (object) $cookies,
					'post_id'      => $post_id,
					'content'      => $content,
					'default_text' => __( 'Click to accept the cookies for this service', 'complianz-gdpr' ),
					'expected'     => $html,
				);
			}
		}
	}
}
$_COOKIE = array();
$cases[] = array( 'kind' => 'shortcode', 'consent_type' => 'optin', 'atts' => (object) array(), 'cookies' => (object) array(), 'post_id' => 0, 'content' => '', 'default_text' => '', 'expected' => COMPLIANZ::$document->show_consent_area( array(), '', 'cmplz-consent-area' ) );
$blocks = array(
	array(),
	array( 'postId' => 12, 'blockId' => 'abc-1', 'category' => 'statistics', 'service' => 'youtube', 'placeholderContent' => '<p>Please accept</p><script>alert(1)</script>' ),
	array( 'postId' => '7x', 'blockId' => 'Mixed Case_ID', 'category' => 'bogus', 'service' => 'Google Maps' ),
	array( 'postId' => 3.7, 'blockId' => 'b', 'category' => null, 'service' => '' ),
	array( 'postId' => true, 'category' => 5, 'service' => 'vimeo.com', 'placeholderContent' => 'Plain & <em>simple</em>' ),
	array( 'postId' => '12e1', 'blockId' => 'ä ö', 'category' => 'preferences' ),
);
foreach ( $blocks as $attrs ) {
	$cases[] = array( 'kind' => 'block', 'attrs' => (object) $attrs, 'expected' => @cmplz_render_consent_area_block( $attrs, '' ) );
}
// cmplz_rest_consented_content() up to do_shortcode(), on a fake post.
$contents = array(
	'<!-- wp:paragraph --><p>x</p><!-- /wp:paragraph --><!-- wp:complianz/consent-area {"blockId":"one","consentedContent":"<p>First</p>"} /--><!-- wp:group --><div><!-- wp:complianz/consent-area {"blockId":"two","consentedContent":"<p>Nested</p>"} /--></div><!-- /wp:group --><!-- wp:complianz/consent-area {"blockId":"two","consentedContent":"<p>Top level second</p>"} /-->',
	'<!-- wp:complianz/consent-area {"blockId":5,"consentedContent":"num"} /--><!-- wp:complianz/consent-area {"blockId":"5"} /-->',
	'Before [cmplz-consent-area service="youtube"]<p>Default area</p>[/cmplz-consent-area] [cmplz-consent-area id="second" service="x"]Second
area[/CMPLZ-CONSENT-AREA] after',
	"[cmplz-consent-area id='quoted']Single quotes[/cmplz-consent-area]",
	'[cmplz-consent-area id="open"]never closed',
	'No area at all',
);
foreach ( $contents as $content ) {
	foreach ( array( 'one', 'two', '5', 'default', 'second', 'quoted', 'open', 'missing' ) as $block_id ) {
		$output = '';
		if ( has_block( 'complianz/consent-area', $content ) ) {
			$queue = parse_blocks( $content );
			while ( ! empty( $queue ) ) {
				$block = array_shift( $queue );
				if ( 'complianz/consent-area' === $block['blockName'] && ( $block['attrs']['blockId'] ?? '' ) === $block_id ) {
					$output = $block['attrs']['consentedContent'] ?? '';
					break;
				}
				if ( ! empty( $block['innerBlocks'] ) ) {
					array_push( $queue, ...$block['innerBlocks'] );
				}
			}
		} elseif ( strpos( $content, '[cmplz-consent-area' ) !== false ) {
			$pattern = '/\[cmplz-consent-area[^\]]*?id=[\"\']' . $block_id . '[\"\'][^\]]*?\](.*?)\[\/cmplz-consent-area\]/is';
			if ( 'default' === $block_id ) {
				$pattern = '/\[cmplz-consent-area[^\]]*?\](.*?)\[\/cmplz-consent-area\]/is';
			}
			if ( preg_match( $pattern, $content, $matches ) ) {
				$output = $matches[1] ?? '';
			}
		}
		$cases[] = array( 'kind' => 'rest', 'content' => $content, 'block_id' => $block_id, 'expected' => $output );
	}
}
echo wp_json_encode( $cases, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE );
PHP
dev/php.sh "$work/golden.php" > "$out"
python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))), "cases")' "$out"
