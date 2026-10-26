<?php
/**
 * Plugin Name: RustPress mail log
 * Description: Appends every message WordPress hands to wp_mail() to /tmp/rustpress-wp-mail.log (one JSON object a
 *              line), so that the e-mail RustPress sends can be compared with it. Sending itself is unchanged.
 */
add_filter(
	'wp_mail',
	function ( $atts ) {
		file_put_contents( '/tmp/rustpress-wp-mail.log', wp_json_encode( $atts ) . "\n", FILE_APPEND );
		return $atts;
	},
	9999
);
