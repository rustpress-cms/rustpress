<?php
/**
 * Prints the Underscore templates of `wp_print_media_templates()` and of the widgets as JSON, with every translatable
 * text marked, so that RustPress can print them in the language of a site. Run by dev/templates-bake.py.
 *
 * A text is `\u{E001}` kind `\u{E003}` context `\u{E003}` text `\u{E002}`. The kind says how WordPress escaped it. The conversions of
 * a format stay visible after `printf()`: `\u{E004}` conversion `\u{E006}` value `\u{E005}`.
 */

define( 'WP_ADMIN', true );
define( 'WP_USE_THEMES', false );
$_SERVER['HTTP_HOST']   = 'localhost:8080';
$_SERVER['REQUEST_URI'] = '/wp-admin/widgets.php';
require '/var/www/html/wp-load.php';
require_once ABSPATH . 'wp-admin/includes/admin.php';
require_once ABSPATH . WPINC . '/media-template.php';
wp_set_current_user( 1 );

$mark = static function ( $text, $context = '' ) {
	// `esc_url()` would drop the marks of a translated address.
	if ( str_starts_with( $text, 'http' ) ) {
		return $text;
	}
	$format = preg_replace( '/%(\d+\$)?[sd]/', "\u{E004}%$0\u{E006}$0\u{E005}", $text );
	return "\u{E001}r\u{E003}$context\u{E003}$format\u{E002}";
};
add_filter( 'gettext', static fn( $translation, $text, $domain ) => 'default' === $domain ? $mark( $text ) : $translation, 10, 3 );
add_filter( 'gettext_with_context', static fn( $translation, $text, $context, $domain ) => 'default' === $domain ? $mark( $text, $context ) : $translation, 10, 4 );
add_filter( 'esc_html', static fn( $text ) => str_replace( "\u{E001}r", "\u{E001}h", $text ) );
add_filter( 'attribute_escape', static fn( $text ) => str_replace( "\u{E001}r", "\u{E001}a", $text ) );

$capture = static function ( $callback ) {
	ob_start();
	$callback();
	return ob_get_clean();
};

$media   = $capture( 'wp_print_media_templates' );
$widgets = $capture(
	static function () {
		// The widgets took their texts when they were made: new ones get them marked.
		foreach ( array( 'WP_Widget_Media_Audio', 'WP_Widget_Media_Image', 'WP_Widget_Media_Gallery', 'WP_Widget_Media_Video' ) as $class ) {
			( new $class() )->render_control_template_scripts();
		}
		WP_Widget_Text::render_control_template_scripts();
		WP_Widget_Custom_HTML::render_control_template_scripts();
	}
);

echo json_encode(
	array(
		'media'   => base64_encode( $media ),
		'widgets' => base64_encode( $widgets ),
	)
);
