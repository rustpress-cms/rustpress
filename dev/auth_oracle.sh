#!/usr/bin/env bash
# Makes the users of the authentication tests in the reference WordPress (dev-only passwords) and prints
# a JSON document with what WordPress computes for them: password hashes in the old and the new
# format, session cookies of every scheme and nonces. crates/rustpress-auth/tests/oracle.rs compares
# RustPress with it. Usage: dev/auth_oracle.sh > crates/rustpress-auth/tests/fixtures/oracle.json
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

for spec in "rpadmin:administrator" "rpeditor:editor"; do
  login="${spec%%:*}"; role="${spec##*:}"
  if ! wp user get "$login" --field=ID >/dev/null 2>&1; then
    wp user create "$login" "$login@example.org" --role="$role" --user_pass=rustpress-dev --porcelain >/dev/null
  fi
done

cat <<'PHP' | ./wp-eval.sh
require_once ABSPATH . WPINC . '/class-phpass.php';
$out = array();
$out['siteurl'] = get_option( 'siteurl' );
$out['cookiehash'] = COOKIEHASH;
$out['now'] = time();
$out['password'] = 'rustpress-dev';
$out['hash_wp'] = wp_hash_password( 'rustpress-dev' );
$out['hash_phpass'] = ( new PasswordHash( 8, true ) )->HashPassword( 'rustpress-dev' );
$out['hash_md5'] = md5( 'rustpress-dev' );
$out['hash_bcrypt'] = password_hash( 'rustpress-dev', PASSWORD_BCRYPT );
$out['users'] = array();
foreach ( array( 'rpadmin', 'rpeditor' ) as $login ) {
	$user = get_user_by( 'login', $login );
	wp_set_current_user( $user->ID );
	$expiration = time() + 3 * DAY_IN_SECONDS;
	$token = WP_Session_Tokens::get_instance( $user->ID )->create( $expiration );
	$entry = array(
		'id' => $user->ID,
		'login' => $login,
		'stored_hash_prefix' => substr( $user->user_pass, 0, 4 ),
		// The part of the hash the cookies carry (`wp_generate_auth_cookie()`): another password, other cookies.
		'pass_frag' => substr( $user->user_pass, 8, 4 ),
		'expiration' => $expiration,
		'token' => $token,
		'cookies' => array(),
	);
	foreach ( array( 'auth', 'secure_auth', 'logged_in' ) as $scheme ) {
		$entry['cookies'][ $scheme ] = wp_generate_auth_cookie( $user->ID, $expiration, $scheme, $token );
	}
	// Nonces use the token of the current session.
	$_COOKIE[ LOGGED_IN_COOKIE ] = $entry['cookies']['logged_in'];
	$entry['nonces'] = array( 'wp_rest' => wp_create_nonce( 'wp_rest' ), 'other' => wp_create_nonce( 'log-out' ) );
	$out['users'][] = $entry;
}
echo wp_json_encode( $out, JSON_PRETTY_PRINT ), "\n";
PHP
