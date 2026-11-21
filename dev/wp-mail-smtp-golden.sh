#!/usr/bin/env bash
# Makes the golden file of WP Mail SMTP: for each case (the option `wp_mail_smtp`, the key `wp_mail_smtp_mail_key`,
# WPMS_* constants and the headers of a wp_mail() call) the plugin configures PHPMailer, and the state it leaves
# right before sending (`wp_mail_smtp_mailcatcher_smtp_pre_send_before`) is written to
# crates/rustpress-plugins/tests/data/wp-mail-smtp/cases.json. Nothing is sent: the hook throws.
#
#   dev/wp-mail-smtp-golden.sh
#
# Needs the reference (dev/up.sh) with wp-mail-smtp 4.10.4 in wp-content/plugins; the script activates it and
# deactivates it again, and restores the two options. Run inside WSL.
set -euo pipefail
cd "$(dirname "$0")/.."
out=crates/rustpress-plugins/tests/data/wp-mail-smtp/cases.json
dev/wp.sh plugin activate wp-mail-smtp >/dev/null
trap 'dev/wp.sh plugin deactivate wp-mail-smtp >/dev/null; podman exec rustpress-wp rm -f /var/www/html/wpms-golden.php' EXIT
podman exec -i -u 33 rustpress-wp bash -c 'cat > /var/www/html/wpms-golden.php' <<'PHP'
<?php
$key   = str_repeat( "\x07", SODIUM_CRYPTO_SECRETBOX_KEYBYTES );
$nonce = str_repeat( "\x01", SODIUM_CRYPTO_SECRETBOX_NONCEBYTES );
$enc   = function ( $text ) use ( $key, $nonce ) {
	return base64_encode( $nonce . sodium_crypto_secretbox( $text, $nonce, $key ) );
};
$smtp = function ( $over = array() ) use ( $enc ) {
	return array_replace_recursive(
		array(
			'mail' => array( 'from_email' => 'site@example.org', 'from_name' => 'Die Seite', 'mailer' => 'smtp', 'return_path' => false, 'from_email_force' => false, 'from_name_force' => false ),
			'smtp' => array( 'host' => 'smtp.example.org', 'port' => 587, 'encryption' => 'tls', 'auth' => true, 'autotls' => true, 'user' => 'postmaster@example.org', 'pass' => $enc( 'geheim "1"' ) ),
		),
		$over
	);
};
$headers = array(
	'none'     => array(),
	'from'     => array( 'From: Jemand <jemand@example.net>' ),
	'fromname' => array( 'From: Nur Name <wordpress@localhost>' ),
	'reply'    => array( 'From: Jemand <jemand@example.net>', 'Reply-To: antwort@example.com' ),
);
$cases = array(
	'smtp-tls'           => array( $smtp(), true, array() ),
	'smtp-ssl-forced'    => array( $smtp( array( 'mail' => array( 'from_email_force' => true, 'from_name_force' => true, 'return_path' => true ), 'smtp' => array( 'encryption' => 'ssl', 'port' => '465' ) ) ), true, array() ),
	'smtp-none-noauto'   => array( $smtp( array( 'smtp' => array( 'encryption' => 'none', 'autotls' => false, 'auth' => false ) ) ), true, array() ),
	'smtp-none-auto'     => array( $smtp( array( 'smtp' => array( 'encryption' => 'none', 'autotls' => true ) ) ), true, array() ),
	'smtp-no-host'       => array( $smtp( array( 'smtp' => array( 'host' => '' ) ) ), true, array() ),
	'smtp-plain-pass'    => array( $smtp( array( 'smtp' => array( 'pass' => 'klartext\\\'s' ) ) ), true, array() ),
	'smtp-wrong-key'     => array( $smtp(), false, array() ),
	'smtp-empty-from'    => array( $smtp( array( 'mail' => array( 'from_email' => '', 'from_name' => '' ) ) ), true, array() ),
	'mail'               => array( array( 'mail' => array( 'mailer' => 'mail', 'from_email' => 'post@example.org', 'from_name' => 'Post' ) ), true, array() ),
	'mail-defaults'      => array( array(), true, array() ),
	'blocked'            => array( $smtp( array( 'general' => array( 'do_not_send' => true ) ) ), true, array() ),
	'sendgrid'           => array( array( 'mail' => array( 'mailer' => 'sendgrid' ), 'sendgrid' => array( 'api_key' => 'x' ) ), true, array() ),
	'constants'          => array( $smtp(), true, array( 'WPMS_ON' => true, 'WPMS_SMTP_HOST' => 'mx.example.com', 'WPMS_SMTP_PORT' => 2525, 'WPMS_SSL' => '', 'WPMS_SMTP_AUTH' => true, 'WPMS_SMTP_AUTOTLS' => false, 'WPMS_SMTP_USER' => 'konst', 'WPMS_SMTP_PASS' => 'konst-pass', 'WPMS_MAIL_FROM' => 'konst@example.com', 'WPMS_MAIL_FROM_FORCE' => true, 'WPMS_MAIL_FROM_NAME' => 'Konstante' ) ),
);
$results = array();
$state   = null;
$error = null;
add_action(
	'wp_mail_failed',
	function ( $e ) use ( &$error ) {
		$error = $e->get_error_message();
	}
);
add_action(
	'wp_mail_smtp_mailcatcher_smtp_pre_send_before',
	function ( $mailer ) use ( &$state ) {
		$state = array(
			'Mailer'      => $mailer->Mailer,
			'Host'        => $mailer->Host,
			'Port'        => $mailer->Port,
			'SMTPSecure'  => $mailer->SMTPSecure,
			'SMTPAutoTLS' => $mailer->SMTPAutoTLS,
			'SMTPAuth'    => $mailer->SMTPAuth,
			'Username'    => $mailer->Username,
			'Password'    => $mailer->Password,
			'From'        => $mailer->From,
			'FromName'    => $mailer->FromName,
			'Sender'      => $mailer->Sender,
			'ReplyTo'     => array_values( array_map( function ( $r ) { return $r[0]; }, $mailer->getReplyToAddresses() ) ),
			'XMailer'     => $mailer->XMailer,
		);
		throw new PHPMailer\PHPMailer\Exception( 'golden' );
	}
);
foreach ( $cases as $name => list( $option, $right_key, $constants ) ) {
	if ( $constants && ! defined( 'WPMS_ON' ) ) {
		foreach ( $constants as $constant => $value ) {
			define( $constant, $value );
		}
	}
	update_option( 'wp_mail_smtp', $option );
	update_option( 'wp_mail_smtp_mail_key', base64_encode( $right_key ? $key : str_repeat( "\x09", SODIUM_CRYPTO_SECRETBOX_KEYBYTES ) ) );
	$populate = new ReflectionMethod( WPMailSMTP\Options::class, 'populate_options' );
	$populate->setAccessible( true );
	$populate->invoke( WPMailSMTP\Options::init() );
	foreach ( $headers as $kind => $lines ) {
		$state = null;
		$error = null;
		// A request's first message: a new MailCatcher and nothing remembered from earlier ones.
		wp_mail_smtp()->replace_phpmailer();
		( function () {
			$this->wp_mail_from = null;
		} )->call( wp_mail_smtp()->get_processor() );
		$sent  = wp_mail( 'empfang@example.com', 'Betreff', 'Text', $lines );
		$results[] = array(
			'name'      => "$name/$kind",
			'option'    => maybe_serialize( get_option( 'wp_mail_smtp' ) ),
			'mail_key'  => get_option( 'wp_mail_smtp_mail_key' ),
			'constants' => $constants ? $constants : new stdClass(),
			'headers'   => $lines,
			'sent'      => $sent,
			'error'     => $error,
			'expected'  => $state,
		);
	}
}
echo wp_json_encode( $results, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE ), "\n";
PHP
before=$(dev/wp.sh option get wp_mail_smtp --format=json 2>/dev/null || echo '')
key_before=$(dev/wp.sh option get wp_mail_smtp_mail_key 2>/dev/null || echo '')
dev/wp.sh eval-file /var/www/html/wpms-golden.php > "$out.tmp" || { cat "$out.tmp"; exit 1; }
mv "$out.tmp" "$out"
dev/wp.sh option delete wp_mail_smtp wp_mail_smtp_mail_key >/dev/null 2>&1 || true
[[ -n $before ]] && dev/wp.sh option update wp_mail_smtp "$before" --format=json >/dev/null
[[ -n $key_before ]] && dev/wp.sh option update wp_mail_smtp_mail_key "$key_before" >/dev/null
echo "wrote $out ($(grep -c '"name"' "$out") cases)"
