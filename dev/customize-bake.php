<?php
/**
 * Prints the Underscore templates of `wp-admin/customize.php` as JSON, with every translatable text marked, so that
 * RustPress can print them in the language of a site. Run by dev/customize-bake.py.
 *
 * A text is `\u{E001}` kind `\u{E003}` context `\u{E003}` text `\u{E002}`. The kind says how WordPress escaped it. The
 * conversions of a format stay visible after `printf()`: `\u{E004}` conversion `\u{E006}` value `\u{E005}`.
 */

define( 'WP_ADMIN', true );
define( 'WP_USE_THEMES', false );
$_SERVER['HTTP_HOST']   = 'localhost:8080';
$_SERVER['REQUEST_URI'] = '/wp-admin/customize.php';
require '/var/www/html/wp-load.php';
require_once ABSPATH . 'wp-admin/includes/admin.php';
wp_set_current_user( 1 );

$format = static fn( $text ) => preg_replace_callback(
	'/%%|%(\d+\$)?[sd]/',
	static fn( $match ) => "\u{E004}" . ( '%%' === $match[0] ? '%%%%' : "%{$match[0]}" ) . "\u{E006}{$match[0]}\u{E005}",
	$text
);
$mark   = static function ( $text, $context = '' ) use ( $format ) {
	// `esc_url()` would drop the marks of a translated address.
	if ( str_starts_with( $text, 'http' ) ) {
		return "$text#rustpress-text-" . strtr( base64_encode( "$context\u{E003}$text" ), '+/=', '-_.' );
	}
	return "\u{E001}r\u{E003}$context\u{E003}{$format( $text )}\u{E002}";
};
add_filter( 'gettext', static fn( $translation, $text, $domain ) => 'default' === $domain ? $mark( $text ) : $translation, 10, 3 );
add_filter( 'gettext_with_context', static fn( $translation, $text, $context, $domain ) => 'default' === $domain ? $mark( $text, $context ) : $translation, 10, 4 );
add_filter( 'esc_html', static fn( $text ) => str_replace( "\u{E001}r", "\u{E001}h", $text ) );
add_filter( 'attribute_escape', static fn( $text ) => str_replace( "\u{E001}r", "\u{E001}a", $text ) );

// Changesets are not branched (the lock notices speak of the site).
add_filter( 'customize_changeset_branching', '__return_false' );
require_once ABSPATH . WPINC . '/class-wp-customize-manager.php';
$GLOBALS['wp_customize'] = new WP_Customize_Manager();
$wp_customize            = $GLOBALS['wp_customize'];
$wp_customize->wp_loaded();

$capture = static function ( $callback ) {
	ob_start();
	$callback();
	return ob_get_clean();
};

// The form of each core widget as a template (`form_callback( -1 )`), but the menu widget's, which lists the menus.
$forms = array();
foreach ( $GLOBALS['wp_widget_factory']->widgets as $widget ) {
	if ( 'nav_menu' !== $widget->id_base ) {
		$forms[ $widget->id_base ] = $capture( static fn() => $widget->form_callback( -1 ) );
	}
}

echo json_encode(
	array(
		'templates' => base64_encode(
			$capture(
				static function () use ( $wp_customize ) {
					$wp_customize->render_panel_templates();
					$wp_customize->render_section_templates();
					$wp_customize->render_control_templates();
				}
			)
		),
		'menus'     => base64_encode( $capture( array( $wp_customize->nav_menus, 'print_templates' ) ) ),
		'notices'   => base64_encode( $capture( 'wp_print_admin_notice_templates' ) ),
		'themes'    => base64_encode( $capture( 'customize_themes_print_templates' ) ),
		'header'    => base64_encode( $capture( array( new WP_Customize_Header_Image_Control( $wp_customize ), 'print_header_image_template' ) ) ),
		'forms'     => base64_encode( wp_json_encode( $forms ) ),
	)
);
