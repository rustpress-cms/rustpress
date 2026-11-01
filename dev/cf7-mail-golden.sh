#!/usr/bin/env bash
# Makes the golden files of the mails of Contact Form 7: every crates/rustpress-plugins/tests/data/cf7-mail/NAME.post
# (the submitted fields as a query string) is submitted by the plugin to a new form in en_US (NAME.form, else the
# default form; NAME.mail.json and NAME.mail2.json change the mail templates, NAME.settings holds additional
# settings). NAME.json gets the form and templates used, the values of the special mail tags, the result of the
# submission and the arguments of every wp_mail() call (caught by pre_wp_mail, so nothing is sent).
#
#   dev/cf7-mail-golden.sh
#
# Needs the reference container (dev/up.sh) with contact-form-7 in wp-content/plugins and active. Run inside WSL.
set -euo pipefail
cd "$(dirname "$0")/.."
cases=crates/rustpress-plugins/tests/data/cf7-mail
podman exec rustpress-wp rm -rf /tmp/cf7mail
podman exec rustpress-wp mkdir -p /tmp/cf7mail
for f in "$cases"/*.post "$cases"/*.form "$cases"/*.settings "$cases"/*.mail.json "$cases"/*.mail2.json; do
  [ -e "$f" ] && podman cp "$f" "rustpress-wp:/tmp/cf7mail/$(basename "$f")"
done
podman exec -i rustpress-wp tee /tmp/cf7mail/submit.php >/dev/null <<'PHP'
<?php
// One submission per run: WPCF7_Submission is a singleton.
$_SERVER['HTTP_HOST'] = 'localhost:8080';
$_SERVER['REQUEST_URI'] = '/contact/?ref=golden';
$_SERVER['REMOTE_ADDR'] = '192.0.2.1';
$_SERVER['HTTP_USER_AGENT'] = 'Golden/1.0';
require '/var/www/html/wp-load.php';
$name = $argv[1];
$dir = '/tmp/cf7mail';
$read = static function ( $suffix ) use ( $dir, $name ) {
	return is_file( "$dir/$name.$suffix" ) ? file_get_contents( "$dir/$name.$suffix" ) : null;
};
parse_str( trim( $read( 'post' ) ), $_POST );
$_POST['_wpcf7_unit_tag'] = 'wpcf7-f0-o1';
$mails = array();
add_filter( 'pre_wp_mail', static function ( $null, $atts ) use ( &$mails ) {
	$mails[] = $atts;
	return true;
}, 10, 2 );
$form = WPCF7_ContactForm::get_template( array( 'locale' => 'en_US', 'title' => 'Golden' ) );
$properties = array( 'additional_settings' => (string) $read( 'settings' ) );
if ( null !== $read( 'form' ) ) {
	$properties['form'] = $read( 'form' );
}
foreach ( array( 'mail' => 'mail.json', 'mail_2' => 'mail2.json' ) as $prop => $file ) {
	if ( null !== $read( $file ) ) {
		$properties[ $prop ] = array_merge( $form->prop( $prop ), json_decode( $read( $file ), true ) );
	}
}
$form->set_properties( $properties );
$result = $form->submit();
unset( $result['contact_form_id'] );
if ( ! empty( $result['posted_data_hash'] ) ) {
	$result['posted_data_hash'] = 'HASH';
}
$special = array();
foreach ( array( '_url', '_date', '_time' ) as $tag ) {
	$special[ $tag ] = apply_filters( 'wpcf7_special_mail_tags', null, $tag, false, new WPCF7_MailTag( "[$tag]", $tag, '' ) );
}
$out = array(
	'form' => $form->prop( 'form' ),
	'mail' => $form->prop( 'mail' ),
	'mail_2' => $form->prop( 'mail_2' ),
	'additional_settings' => $form->prop( 'additional_settings' ),
	'site' => array(
		'blogname' => get_option( 'blogname' ),
		'blogdescription' => get_option( 'blogdescription' ),
		'home' => home_url(),
		'admin_email' => get_option( 'admin_email' ),
	),
	'special' => $special,
	'result' => $result,
	'mails' => $mails,
);
file_put_contents( "$dir/$name.json", wp_json_encode( $out, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE ) . "\n" );
PHP
for f in "$cases"/*.post; do
  name=$(basename "$f" .post)
  podman exec rustpress-wp php /tmp/cf7mail/submit.php "$name"
  podman cp "rustpress-wp:/tmp/cf7mail/$name.json" "$cases/$name.json"
done
echo "$(ls "$cases"/*.post | wc -l) cases"
