#!/usr/bin/env bash
# Makes the golden files of the forms of Contact Form 7: every crates/rustpress-plugins/tests/data/cf7-forms/NAME.form
# is printed by the plugin (WPCF7_ContactForm::form_html() of a new form in en_US, outside the loop, for the request
# /contact/?ref=golden) into NAME.html. NAME.settings holds additional settings, NAME.atts the options of form_html()
# as a query string (html_id=...&html_class=...). The quiz answer hash depends on the salts and becomes HASH.
#
#   dev/cf7-form-golden.sh
#
# Needs the reference container (dev/up.sh) with contact-form-7 in wp-content/plugins and active. Run inside WSL.
set -euo pipefail
cd "$(dirname "$0")/.."
cases=crates/rustpress-plugins/tests/data/cf7-forms
podman exec rustpress-wp rm -rf /tmp/cf7cases
podman exec rustpress-wp mkdir -p /tmp/cf7cases
for f in "$cases"/*.form "$cases"/*.settings "$cases"/*.atts; do
  [ -e "$f" ] && podman cp "$f" "rustpress-wp:/tmp/cf7cases/$(basename "$f")"
done
podman exec -i rustpress-wp tee /tmp/cf7cases/render.php >/dev/null <<'PHP'
<?php
// One form per run: the unit-tag counter and the used ids are static variables.
$_SERVER['HTTP_HOST'] = 'localhost:8080';
$_SERVER['REQUEST_URI'] = '/contact/?ref=golden';
require '/var/www/html/wp-load.php';
$name = $argv[1];
$dir = '/tmp/cf7cases';
$settings = is_file( "$dir/$name.settings" ) ? file_get_contents( "$dir/$name.settings" ) : '';
$atts = array();
if ( is_file( "$dir/$name.atts" ) ) {
	parse_str( trim( file_get_contents( "$dir/$name.atts" ) ), $atts );
}
$form = WPCF7_ContactForm::get_template( array( 'locale' => 'en_US', 'title' => 'Golden' ) );
$form->set_properties( array(
	'form' => file_get_contents( "$dir/$name.form" ),
	'additional_settings' => $settings,
) );
$html = $form->form_html( $atts );
$html = preg_replace( '/(name="_wpcf7_quiz_answer_[^"]*" value=")[0-9a-f]+"/', '$1HASH"', $html );
file_put_contents( "$dir/$name.html", $html );
PHP
for f in "$cases"/*.form; do
  name=$(basename "$f" .form)
  podman exec rustpress-wp php /tmp/cf7cases/render.php "$name"
  podman cp "rustpress-wp:/tmp/cf7cases/$name.html" "$cases/$name.html"
done
echo "$(ls "$cases"/*.form | wc -l) cases"
