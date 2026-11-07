<?php
/**
 * For dev/bake-marks.py: with the header `X-Rustpress-Marks`, a page of the reference comes with every translatable text
 * marked, so that the bake knows which message each text was. Without the header it does nothing.
 *
 * A text is `\u{E001}` kind `\u{E003}` context `\u{E003}` text `\u{E002}`. The kind says how WordPress escaped it: `r`aw, `h`tml,
 * `a`ttribute. A plural has the kind `n`, `N` or `A` and carries singular, plural and number in the context, split by
 * `\u{E007}`, with `\u{E008}` for a percent sign. The conversions of a format stay visible after `printf()`: `\u{E004}` conversion
 * `\u{E006}` value `\u{E005}`. A translated address has to pass `esc_url()`, so it is the address with `#rustpress-text-` and
 * context and message in base64.
 */

if ( empty( $_SERVER['HTTP_X_RUSTPRESS_MARKS'] ) ) {
	return;
}

// The marks must not reach the database: `WP_Privacy_Policy_Content` stores its text on every page, for one.
add_filter(
	'query',
	static fn( $query ) => ( str_contains( $query, "\u{E001}" ) || str_contains( $query, '#rustpress-text-' ) ) && ! preg_match( '/^\s*SELECT/i', $query ) ? '' : $query
);

add_action(
	'init',
	static function () {
		// `%%` is a percent sign, and printf() takes one pair of the marks away.
		$format = static fn( $text ) => preg_replace_callback(
			'/%%|%(\d+\$)?[sd]/',
			static fn( $match ) => "\u{E004}" . ( '%%' === $match[0] ? '%%%%' : "%{$match[0]}" ) . "\u{E006}{$match[0]}\u{E005}",
			$text
		);
		$mark   = static fn( $text, $context = '' ) => str_starts_with( $text, 'http' ) ? "$text#rustpress-text-" . strtr( base64_encode( "$context\u{E003}$text" ), '+/=', '-_.' ) : "\u{E001}r\u{E003}$context\u{E003}{$format( $text )}\u{E002}";
		$plural = static fn( $single, $plural, $number, $context = '' ) => "\u{E001}n\u{E003}$context\u{E007}" . str_replace( '%', "\u{E008}", "$single\u{E007}$plural" ) . "\u{E007}$number\u{E003}{$format( 1 === (int) $number ? $single : $plural )}\u{E002}";
		add_filter( 'gettext', static fn( $translation, $text, $domain ) => 'default' === $domain ? $mark( $text ) : $translation, 10, 3 );
		add_filter( 'gettext_with_context', static fn( $translation, $text, $context, $domain ) => 'default' === $domain ? $mark( $text, $context ) : $translation, 10, 4 );
		add_filter( 'ngettext', static fn( $translation, $single, $many, $number, $domain ) => 'default' === $domain ? $plural( $single, $many, $number ) : $translation, 10, 5 );
		add_filter( 'ngettext_with_context', static fn( $translation, $single, $many, $number, $context, $domain ) => 'default' === $domain ? $plural( $single, $many, $number, $context ) : $translation, 10, 6 );
		$escaped = static fn( $text, $plain, $many ) => str_replace( array( "\u{E001}r", "\u{E001}n" ), array( "\u{E001}$plain", "\u{E001}$many" ), $text );
		add_filter( 'esc_html', static fn( $text ) => $escaped( $text, 'h', 'N' ) );
		add_filter( 'attribute_escape', static fn( $text ) => $escaped( $text, 'a', 'A' ) );
	},
	-1
);
