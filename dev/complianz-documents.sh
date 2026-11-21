#!/usr/bin/env bash
# Writes crates/rustpress-plugins/src/complianz/documents.json: the document elements of the cookie policies of
# Complianz 7.5.6 (config/documents/cookie-policy-*.php) with their translatable strings kept as calls, so that
# RustPress translates them with the plugin's language pack like the plugin does, and the fields of the plugin.
#
#   dev/complianz-documents.sh
#
# Needs the reference (dev/up.sh) with complianz-gdpr 7.5.6 in wp-content/plugins. The configuration files are
# included without WordPress, with __(), _x(), _n() and cmplz_sprintf() replaced by functions that record the call;
# the fields (`cmplz_config::$fields`, with the options and labels of those the documents print) come from the plugin
# loaded in WordPress, in English.
set -euo pipefail
cd "$(dirname "$0")/.."
snippet=$(mktemp)
trap 'rm -f "$snippet" "$snippet.json" "$snippet.lists"' EXIT
cat > "$snippet" <<'PHP'
<?php
define( 'ABSPATH', '/var/www/html/' );
// \x01 opens a recorded call, \x1f separates its arguments, \x02 closes it.
function __( $text, $domain = 'default' ) { return "\x01__\x1f$text\x02"; }
function _x( $text, $context, $domain = 'default' ) { return "\x01_x\x1f$text\x1f$context\x02"; }
function _n( $single, $plural, $number, $domain = 'default' ) { return "\x01_n\x1f$single\x1f$plural\x1f$number\x02"; }
function cmplz_sprintf( ...$args ) { return "\x01sprintf\x1f" . implode( "\x1f", $args ) . "\x02"; }
function cmplz_count_socialmedia() { return 'socialmedia'; }
class Complianz_Documents {
	public $pages = array();
	public function load( $dir ) {
		foreach ( array( 'eu', 'us', 'uk', 'ca', 'au', 'za', 'br' ) as $region ) {
			require "$dir/config/documents/cookie-policy-$region.php";
		}
		$out = array();
		foreach ( $this->pages as $region => $types ) {
			foreach ( $types as $type => $page ) {
				$elements = array();
				foreach ( $page['document_elements'] as $key => $element ) {
					$elements[] = array( (string) $key, $element );
				}
				$out[] = array( 'region' => $region, 'type' => $type, 'elements' => $elements );
			}
		}
		echo json_encode( $out );
	}
}
( new Complianz_Documents() )->load( '/var/www/html/wp-content/plugins/complianz-gdpr' );
PHP
podman cp "$snippet" rustpress-wp:/tmp/complianz-documents.php
podman exec rustpress-wp php /tmp/complianz-documents.php > "$snippet.json"
cat > "$snippet" <<'PHP'
<?php
$fields = array();
foreach ( COMPLIANZ::$config->fields as $field ) {
	$entry = array( 'id' => $field['id'], 'type' => $field['type'] ?? '' );
	if ( isset( $field['options'] ) ) {
		$entry['options'] = $field['options'];
	}
	if ( isset( $field['document_label'] ) ) {
		$entry['document_label'] = $field['document_label'];
	}
	$fields[] = $entry;
}
echo json_encode( array( 'fields' => $fields ) );
PHP
dev/php.sh "$snippet" > "$snippet.lists"
python3 - "$snippet.json" "$snippet.lists" crates/rustpress-plugins/src/complianz/documents.json <<'PY'
import json, sys

def parse(text, i=0, stop=None):
    """The recorded calls in a string as a list of literal strings and calls."""
    parts, buf = [], ''
    while i < len(text):
        c = text[i]
        if c == '\x01':
            if buf:
                parts.append(buf); buf = ''
            name_end = text.index('\x1f', i)
            name = text[i + 1:name_end]
            args, i = [], name_end + 1
            while True:
                arg, i, end = parse(text, i, ('\x1f', '\x02'))
                args.append(arg)
                if end == '\x02':
                    break
            parts.append({name: args})
            continue
        if stop and c in stop:
            if buf:
                parts.append(buf)
            return parts, i + 1, c
        buf += c
        i += 1
    if buf:
        parts.append(buf)
    return parts, i, None

def convert(value):
    if isinstance(value, str):
        return parse(value)[0]
    if isinstance(value, dict):
        return {k: convert(v) if k in ('title', 'subtitle', 'content') else v for k, v in value.items()}
    return value

docs = json.load(open(sys.argv[1]))
for doc in docs:
    doc['elements'] = [[key, convert(element)] for key, element in doc['elements']]
# The fields in their order, with the options and labels of those the documents print ([id], [comma_id]).
text = json.dumps(docs)
fields = json.load(open(sys.argv[2]))['fields']
for field in fields:
    if f'[{field["id"]}]' not in text and f'[comma_{field["id"]}]' not in text:
        field.pop('options', None)
        field.pop('document_label', None)
with open(sys.argv[3], 'w', newline='') as f:
    json.dump({'documents': docs, 'fields': fields}, f, ensure_ascii=False, indent=1)
    f.write('\n')
PY
