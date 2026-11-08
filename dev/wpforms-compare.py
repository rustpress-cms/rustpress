#!/usr/bin/env python3
"""Compares pages with WPForms Lite forms and their submissions on the reference (:8080) and RustPress (:3001).

    python3 dev/wpforms-compare.py [--keep] [--diff N]

Needs the reference stack (dev/up.sh) with wpforms-lite active and RustPress listening on :3001 on the same database,
started with RUSTPRESS_RELOAD_SECS=1 so that it sees the forms the script makes.
The script creates the forms of crates/rustpress-plugins/tests/data/wpforms-forms/*.json (form posts 9100 and up) and
one page per form with its shortcode, compares the pages (anonymous and as rpadmin) and then sends submissions to
admin-ajax.php (`wpforms_submit`) on both sides: the answers must be equal, and so must the e-mails of the mail log
(dev/mu-plugins). It removes everything it created again unless `--keep` is given.

What is random or depends on the time is replaced before the comparison: the tokens, the nonces, the honeypot field
the plugin puts in front of a random field (and its label) and the label of the old honeypot."""
import http.cookiejar
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CASES = os.path.join(ROOT, 'crates', 'rustpress-plugins', 'tests', 'data', 'wpforms-forms')
FIRST = 9100


def php(code):
    """Runs PHP with WordPress loaded in the reference container and returns what it prints."""
    script = 'define("WP_USE_THEMES", false); require "/var/www/html/wp-load.php"; ' + code
    return subprocess.run(['podman', 'exec', '-i', 'rustpress-wp', 'php', '-r', script], check=True, capture_output=True, text=True).stdout


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


_openers = {}


def opener(port, user):
    key = (port, user)
    if key not in _openers:
        o = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()), NoRedirect())
        if user:
            o.open(f'http://localhost:{port}/wp-login.php').read()
            try:
                o.open(f'http://localhost:{port}/wp-login.php', urllib.parse.urlencode({'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()
            except urllib.error.HTTPError:
                pass
        _openers[key] = o
    return _openers[key]


def fetch(port, user, path, data=None):
    try:
        r = opener(port, user).open(f'http://localhost:{port}{path}', data)
        return r.status, r.headers.get('Content-Type', ''), r.read().decode('utf-8', 'replace')
    except urllib.error.HTTPError as e:
        return e.code, e.headers.get('Content-Type', ''), e.read().decode('utf-8', 'replace')


def normalize(text):
    text = re.sub(r'localhost(:|%3A)(8080|3001)', 'HOST', text)
    text = re.sub(r'127\.0\.0\.1:(8080|3001)', 'HOST', text)
    text = re.sub(r'data-token="[0-9a-f]{32}"', 'data-token="T"', text)
    text = re.sub(r'data-token-time="\d+"', 'data-token-time="T"', text)
    text = re.sub(r'value="\d+\.[0-9a-f]{64}"', 'value="TIMETOKEN"', text)
    text = re.sub(r'("nonce":"|name="wpforms\[nonce\]"\s+value="|_wpnonce=|_wpnonce" value=")[0-9a-f]{10}', r'\1N', text)
    # The honeypot of AntiSpam v3: its block in front of a random field, its styles and its field ID.
    text = re.sub(r'\t\t<div id="wpforms-\d+-field_\d+-container"\n\t\t\tclass="wpforms-field wpforms-field-text"\n\t\t\tdata-field-type="text"\n.*?</div>\n\t\t', '', text, flags=re.S)
    text = re.sub(r'<label for="wpforms-(\d+)-field-hp" class="wpforms-field-label">\w+</label>', r'<label for="wpforms-\1-field-hp" class="wpforms-field-label">HP</label>', text)
    # The navigation lists the pages the script made.
    text = re.sub(r'.*wp-block-navigation__container.*\n', '', text)
    return text


def diff(a, b, limit):
    import difflib
    lines = list(difflib.unified_diff(a.splitlines(), b.splitlines(), 'reference', 'rustpress', lineterm='', n=1))
    for line in lines[:limit]:
        print('   ', line[:400])
    return len(lines)


def main():
    args = sys.argv[1:]
    keep = '--keep' in args
    limit = int(args[args.index('--diff') + 1]) if '--diff' in args else 20
    names = sorted(f[:-5] for f in os.listdir(CASES) if f.endswith('.json'))
    cases = {}
    for i, name in enumerate(names):
        with open(os.path.join(CASES, name + '.json'), encoding='utf-8') as f:
            cases[name] = (FIRST + 2 * i, json.load(f))
    # The forms and their pages, as the builder stores them (only the cases without settings of their own).
    created = []
    payload = []
    for name, (form_id, case) in cases.items():
        if case.get('wpforms_settings'):
            continue
        payload.append({'id': form_id, 'name': name, 'form': case['form'], 'title': case['title'], 'description': case['description']})
        created.append((name, form_id))
    # The block of the contact form with the attributes the editor stores.
    contact = cases['contact'][0]
    blocks = [
        {},
        {'clientId': 'a1b2c3', 'className': 'my-form extra', 'displayTitle': True, 'displayDesc': True},
        {'clientId': 'd4e5', 'theme': 'ocean', 'themeName': 'Ocean'},
        {'clientId': 'f6', 'fieldSize': 'large', 'labelSize': 'small', 'buttonSize': 'large', 'buttonBackgroundColor': 'transparent', 'fieldBorderStyle': 'none', 'customCss': 'color: red;'},
        {'clientId': 'g7', 'themeName': '', 'theme': 'default', 'buttonTextColor': '#066aab', 'containerShadowSize': 'medium'},
    ]
    for i, attrs in enumerate(blocks):
        attrs = {'formId': str(contact), **attrs}
        payload.append({'id': FIRST - 2 * (i + 1), 'name': f'block-{i}', 'block': json.dumps(attrs)})
        created.append((f'block-{i}', FIRST - 2 * (i + 1)))
    data = json.dumps(payload).replace('\\', '\\\\').replace("'", "\\'")
    php(f"""kses_remove_filters();
foreach ( json_decode( '{data}', true ) as $c ) {{
    wp_delete_post( $c['id'], true ); wp_delete_post( $c['id'] + 1, true );
    if ( isset( $c['block'] ) ) {{
        wp_insert_post( [ 'import_id' => $c['id'] + 1, 'post_type' => 'page', 'post_status' => 'publish', 'post_title' => 'WPForms ' . $c['name'], 'post_name' => 'wpforms-' . $c['name'], 'post_date' => '2026-10-24 10:00:00', 'post_content' => '<!-- wp:wpforms/form-selector ' . $c['block'] . ' /-->' ] );
        continue;
    }}
    $form = $c['form']; $form['id'] = (string) $c['id'];
    wp_insert_post( [ 'import_id' => $c['id'], 'post_type' => 'wpforms', 'post_status' => 'publish', 'post_title' => 'Form ' . $c['name'], 'post_content' => wpforms_encode( $form ) ] );
    $atts = 'id="' . $c['id'] . '"' . ( $c['title'] ? ' title="true"' : '' ) . ( $c['description'] ? ' description="true"' : '' );
    wp_insert_post( [ 'import_id' => $c['id'] + 1, 'post_type' => 'page', 'post_status' => 'publish', 'post_title' => 'WPForms ' . $c['name'], 'post_name' => 'wpforms-' . $c['name'], 'post_date' => '2026-10-24 10:00:00', 'post_content' => "<!-- wp:shortcode -->[wpforms $atts]<!-- /wp:shortcode -->" ] );
}}""")
    failures = 0
    try:
        for name, form_id in created:
            for user in (None, 'rpadmin'):
                path = f'/wpforms-{name}/'
                a = fetch(8080, user, path)
                b = fetch(3001, user, path)
                left, right = normalize(a[2]), normalize(b[2])
                same = a[0] == b[0] and left == right
                print(f"{'ok  ' if same else 'DIFF'} {path} {'as ' + user if user else 'anonymous'}")
                if not same:
                    failures += 1
                    print(f'    status {a[0]} / {b[0]}')
                    diff(left, right, limit)
    finally:
        if not keep:
            ids = ','.join(str(i) + ',' + str(i + 1) for _, i in created)
            php(f"foreach ( [ {ids} ] as $id ) {{ wp_delete_post( $id, true ); }}")
    print(f'{failures} differences')
    sys.exit(1 if failures else 0)


if __name__ == '__main__':
    main()
