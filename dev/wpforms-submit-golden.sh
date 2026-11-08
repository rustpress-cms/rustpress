#!/usr/bin/env bash
# Makes the golden files of the submissions of WPForms Lite: every
# crates/rustpress-plugins/tests/data/wpforms-submit/NAME.case.json is submitted by the plugin as admin-ajax.php's
# `wpforms_submit` would (WPForms_Process::ajax_submit() with DOING_AJAX) and NAME.json gets the case with what the
# request and the site look like (the time, the secret key, the full settings, the user, the permalinks and authors the
# tags read), the JSON answer and the arguments of every wp_mail() call (caught by pre_wp_mail, so nothing is sent; the
# From header as the filters wp_mail_from and wp_mail_from_name leave it).
#
# A case holds `form_from` (a form of tests/data/wpforms-forms) or `form`, `form_settings` (merged into the form's
# settings), `fields` (merged into the form's fields by ID), `wpforms_settings` (merged into the option
# wpforms_settings), `post` (the request body as a query string; `{token}` and `{time_token}` are replaced by valid
# tokens) and `user` (sign in as this user ID; `{nonce}` in `post` is replaced by the nonce of the form).
#
#   dev/wpforms-submit-golden.sh [NAME]
#
# Needs the reference container (dev/up.sh) with wpforms-lite in wp-content/plugins and active.
set -euo pipefail
cd "$(dirname "$0")/.."
cases=crates/rustpress-plugins/tests/data/wpforms-submit
forms=crates/rustpress-plugins/tests/data/wpforms-forms
podman exec rustpress-wp rm -rf /tmp/wpfsubmit
podman exec rustpress-wp mkdir -p /tmp/wpfsubmit/forms
for f in "$cases"/*.case.json; do
  podman cp "$f" "rustpress-wp:/tmp/wpfsubmit/$(basename "$f")"
done
for f in "$forms"/*.json; do
  podman cp "$f" "rustpress-wp:/tmp/wpfsubmit/forms/$(basename "$f")"
done
podman exec -i rustpress-wp tee /tmp/wpfsubmit/submit.php >/dev/null <<'PHP'
<?php
// One submission per run: the plugin keeps static state (the process, the notifications' filters).
define( 'DOING_AJAX', true );
$_SERVER['HTTP_HOST'] = 'localhost:8080';
$_SERVER['REQUEST_URI'] = '/wp-admin/admin-ajax.php';
$_SERVER['REQUEST_METHOD'] = 'POST';
$_SERVER['REMOTE_ADDR'] = '192.0.2.1';
$_SERVER['HTTP_USER_AGENT'] = 'Golden/1.0';
require '/var/www/html/wp-load.php';
$name = $argv[1];
$dir = '/tmp/wpfsubmit';
$case = json_decode( file_get_contents( "$dir/$name.case.json" ), true );
$id = 9300;
// The plugin reads its settings while it loads, so they are changed in a run of their own before this one.
if ( 'settings' === ( $argv[2] ?? '' ) ) {
	$saved = get_option( 'wpforms_settings' );
	file_put_contents( "$dir/saved.ser", serialize( $saved ) );
	update_option( 'wpforms_settings', array_merge( (array) $saved, $case['wpforms_settings'] ?? array() ) );
	exit;
}
if ( 'restore' === ( $argv[2] ?? '' ) ) {
	update_option( 'wpforms_settings', unserialize( file_get_contents( "$dir/saved.ser" ) ) );
	wp_delete_post( $id, true );
	exit;
}
$form = $case['form'] ?? json_decode( file_get_contents( "$dir/forms/{$case['form_from']}.json" ), true )['form'];
$form['settings'] = array_merge( $form['settings'] ?? array(), $case['form_settings'] ?? array() );
foreach ( $case['fields'] ?? array() as $field_id => $field ) {
	$form['fields'][ $field_id ] = null === $field ? null : array_merge( $form['fields'][ $field_id ] ?? array(), $field );
	if ( null === $field ) {
		unset( $form['fields'][ $field_id ] );
	}
}
$form['id'] = (string) $id;
wp_delete_post( $id, true );
kses_remove_filters();
wp_insert_post( array(
	'import_id'    => $id,
	'post_type'    => 'wpforms',
	'post_status'  => $case['status'] ?? 'publish',
	'post_title'   => 'Golden Form',
	'post_content' => wpforms_encode( $form ),
) );
kses_init_filters();
$user = null;
if ( ! empty( $case['user'] ) ) {
	wp_set_current_user( (int) $case['user'] );
	$u = wp_get_current_user();
	$user = array(
		'id'           => $u->ID,
		'display_name' => $u->display_name,
		'email'        => $u->user_email,
		'first_name'   => $u->first_name,
		'last_name'    => $u->last_name,
	);
}
$now = time();
$form_data = wpforms_decode( get_post( $id )->post_content );
$nonce = wp_create_nonce( "wpforms::form_$id" );
$query = strtr( $case['post'] ?? '', array(
	'{token}'      => rawurlencode( wpforms()->obj( 'token' )->get( true, $form_data ) ),
	'{time_token}' => rawurlencode( wpforms()->obj( 'token' )->get_time_token( $form_data ) ),
	'{nonce}'      => $nonce,
) );
parse_str( $query, $post );
$_POST = wp_slash( $post );
$_REQUEST = $_POST;
$mails = array();
add_filter( 'pre_wp_mail', static function ( $null, $atts ) use ( &$mails ) {
	$headers = is_array( $atts['headers'] ) ? $atts['headers'] : explode( "\n", str_replace( "\r\n", "\n", (string) $atts['headers'] ) );
	$from_name = '';
	$from_email = '';
	foreach ( $headers as $header ) {
		if ( 0 === stripos( $header, 'From:' ) ) {
			$content = trim( substr( $header, 5 ) );
			$bracket = strpos( $content, '<' );
			if ( false !== $bracket ) {
				$from_name = str_replace( '"', '', trim( substr( $content, 0, $bracket ) ) );
				$from_email = trim( str_replace( '>', '', substr( $content, $bracket + 1 ) ) );
			}
		}
	}
	$from_email = apply_filters( 'wp_mail_from', $from_email );
	$from_name = apply_filters( 'wp_mail_from_name', $from_name );
	$lines = array();
	foreach ( $headers as $header ) {
		if ( '' === trim( $header ) ) {
			continue;
		}
		$lines[] = 0 === stripos( $header, 'From:' ) ? "From: $from_name <$from_email>" : $header;
	}
	$mails[] = array(
		'to'      => implode( ',', (array) $atts['to'] ),
		'subject' => $atts['subject'],
		'body'    => $atts['message'],
		'headers' => $lines,
	);
	return true;
}, 10, 2 );
add_filter( 'wp_die_ajax_handler', static function () {
	return static function () {
		throw new RuntimeException( 'die' );
	};
} );
$permalinks = array();
foreach ( (array) ( $form_data['settings']['confirmations'] ?? array() ) as $confirmation ) {
	$page = absint( $confirmation['page'] ?? 0 );
	if ( $page ) {
		$permalinks[ $page ] = get_permalink( $page );
	}
}
$page_author = null;
if ( ! empty( $post['page_id'] ) ) {
	$author = (int) get_post_field( 'post_author', absint( $post['page_id'] ) );
	$page_author = array( $author, (string) get_the_author_meta( 'display_name', $author ) );
}
$managers = array();
foreach ( get_users( array( 'capability' => 'manage_options' ) ) as $m ) {
	$managers[] = strtolower( $m->user_email );
}
ob_start();
try {
	wpforms()->obj( 'process' )->ajax_submit();
} catch ( RuntimeException $e ) {
	// wp_send_json() ends with wp_die().
}
$response = ob_get_clean();
$out = $case + array(
	'form_data'      => $form_data,
	'now'            => $now,
	'secret'         => get_option( 'wpforms_crypto_secret_key' ),
	'settings'       => get_option( 'wpforms_settings' ),
	'query'          => $query,
	'nonce'          => $nonce,
	'user_data'      => $user,
	'can_manage'     => current_user_can( 'manage_options' ),
	'super_admin'    => is_super_admin(),
	'site'           => array(
		'admin_email' => get_option( 'admin_email' ),
		'blogname'    => get_option( 'blogname' ),
		'home'        => home_url(),
		'login'       => wp_login_url(),
		'date_format' => get_option( 'date_format' ),
		'gmt_offset'  => (float) get_option( 'gmt_offset' ),
		'language'    => get_language_attributes(),
	),
	'permalinks'     => (object) $permalinks,
	'page_author'    => $page_author,
	'manager_emails' => $managers,
	'response'       => $response,
	'mails'          => $mails,
);
file_put_contents( "$dir/$name.json", wp_json_encode( $out, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE ) . "\n" );
PHP
names=()
if [ $# -gt 0 ]; then names=("$@"); else for f in "$cases"/*.case.json; do names+=("$(basename "$f" .case.json)"); done; fi
for name in "${names[@]}"; do
  podman exec rustpress-wp php /tmp/wpfsubmit/submit.php "$name" settings
  podman exec rustpress-wp php /tmp/wpfsubmit/submit.php "$name" || { podman exec rustpress-wp php /tmp/wpfsubmit/submit.php "$name" restore; exit 1; }
  podman exec rustpress-wp php /tmp/wpfsubmit/submit.php "$name" restore
  podman cp "rustpress-wp:/tmp/wpfsubmit/$name.json" "$cases/$name.json"
done
echo "${#names[@]} cases"
