#!/usr/bin/env python3
"""Compares pages with WPForms Lite forms and their submissions on the reference (:8080) and RustPress (:3001).

    python3 dev/wpforms-compare.py [--keep] [--read-only] [--diff N]

Needs the reference stack (dev/up.sh) with wpforms-lite active and RustPress listening on :3001 on the same database,
started with RUSTPRESS_RELOAD_SECS=1 so that it sees the forms the script makes.
The script creates the forms of crates/rustpress-plugins/tests/data/wpforms-forms/*.json (form posts 9100 and up) and
one page per form with its shortcode, compares the pages (anonymous and as rpadmin) and then sends the submissions of
SUBMISSIONS to admin-ajax.php (`wpforms_submit`, as the plugin's script does) on both sides: the answers must be
equal, and so must the e-mails (the reference's from the mail log of dev/mu-plugins, RustPress's from the mail sink
dev/smtp-sink.py, RustPress started with RUSTPRESS_SMTP_URL=smtp://127.0.0.1:2525) and what the entries count in the
database. `wpforms_get_token` and `wpforms_restricted_email` are compared as well. It removes everything it created
again unless `--keep` is given. With `--read-only` RustPress runs without RUSTPRESS_ALLOW_WRITES: the pages are compared
only anonymously, and RustPress must answer and send the same but count nothing (it cannot write).

What is random or depends on the time is replaced before the comparison: the tokens, the nonces, the honeypot field
the plugin puts in front of a random field (and its label) and the label of the old honeypot."""
import email
import email.policy
import http.cookiejar
import json
import os
import re
import subprocess
import time
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


# The submissions: the form case and the fields of the request (the hidden inputs are taken from the page).
SUBMISSIONS = [
    ('contact', [('wpforms[fields][1][first]', 'Ada'), ('wpforms[fields][1][last]', "O'Neil & <Co>"), ('wpforms[fields][2]', 'ada@example.com'), ('wpforms[fields][3]', 'Hello\r\nhttps://example.org/x & more')]),
    ('contact', [('wpforms[fields][1][first]', ''), ('wpforms[fields][2]', 'nope')]),
    ('checkbox', [('wpforms[fields][1][]', 'First Choice'), ('wpforms[fields][1][]', 'Third Choice'), ('wpforms[fields][3][]', 'I agree'), ('wpforms[fields][5][]', 'A'), ('wpforms[fields][5][other]', 'Mine'), ('wpforms[fields][6][]', 'Pic')]),
    ('email', [('wpforms[fields][1]', 'ada@bücher.de'), ('wpforms[fields][2][primary]', 'x@example.com'), ('wpforms[fields][2][secondary]', 'x@example.com')]),
    ('email', [('wpforms[fields][1]', 'ada@'), ('wpforms[fields][2][primary]', 'x@example.com'), ('wpforms[fields][2][secondary]', 'y@example.com')]),
    ('number', [('wpforms[fields][1]', '-12.5'), ('wpforms[fields][4]', '7'), ('wpforms[fields][5]', '1.25')]),
    ('radio', [('wpforms[fields][1]', 'First Choice'), ('wpforms[fields][2]', 'va'), ('wpforms[fields][4][other]', 'My own')]),
    ('select', [('wpforms[fields][1]', 'Third Choice'), ('wpforms[fields][3][]', 'A')]),
    ('text', [('wpforms[fields][2]', 'Hi'), ('wpforms[fields][8]', '123456789012345678901')]),
    ('textarea', [('wpforms[fields][1]', 'One\r\nTwo'), ('wpforms[fields][2]', 'short')]),
    ('name', [('wpforms[fields][1]', 'Jane'), ('wpforms[fields][3][first]', 'F'), ('wpforms[fields][3][last]', 'L')]),
]

SINK = os.path.join(os.environ.get('TEMP', '.'), 'smtp-sink.log')


def multipart(fields):
    boundary = '----rustpresswpforms'
    out = []
    for k, v in fields:
        out.append(f'--{boundary}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n')
    out.append(f'--{boundary}--\r\n')
    return ''.join(out).encode('utf-8'), f'multipart/form-data; boundary={boundary}'


def post(port, fields, kind='multipart'):
    if kind == 'multipart':
        body, ctype = multipart(fields)
    else:
        body, ctype = urllib.parse.urlencode(fields).encode(), 'application/x-www-form-urlencoded'
    req = urllib.request.Request(f'http://localhost:{port}/wp-admin/admin-ajax.php', body, {'Content-Type': ctype, 'Referer': f'http://localhost:{port}/'})
    try:
        r = opener(port, None).open(req)
        return r.status, r.read().decode('utf-8', 'replace')
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode('utf-8', 'replace')


def hidden_fields(page, port, path):
    """The inputs the plugin's script sends along with the fields."""
    get = lambda pattern: (re.search(pattern, page) or [None, ''])[1]
    out = [('wpforms[id]', get(r'name="wpforms\[id\]" value="(\d+)"')), ('wpforms[time_token]', get(r'name="wpforms\[time_token\]" value="([^"]*)"')),
           ('wpforms[post_id]', get(r'name="wpforms\[post_id\]" value="(\d+)"')), ('wpforms[token]', get(r'data-token="([0-9a-f]+)"')),
           ('page_title', 'WPForms page'), ('page_url', f'http://localhost:{port}{path}'), ('url_referer', ''), ('page_id', get(r'name="page_id" value="(\d+)"')),
           ('wpforms[submit]', 'wpforms-submit'), ('action', 'wpforms_submit')]
    # The honeypot of AntiSpam v3 is sent empty.
    for hp in re.findall(r'<div id="wpforms-\d+-field_(\d+)-container"\n\t\t\tclass="wpforms-field wpforms-field-text"', page):
        if f'wpforms[fields][{hp}]' not in page or not re.search(rf'wpforms-\d+-field_{hp}-container"\n\t\t\tclass="wpforms-field wpforms-field-text"\n\t\t\tdata-field-type="text"\n', page):
            continue
        out.append((f'wpforms[fields][{hp}]', ''))
    return out


def reference_mails(start):
    log = subprocess.run(['podman', 'exec', 'rustpress-wp', 'sh', '-c', 'cat /tmp/rustpress-wp-mail.log 2>/dev/null'], capture_output=True, text=True).stdout.splitlines()
    out = []
    for line in log[start:]:
        atts = json.loads(line)
        headers = atts['headers'] if isinstance(atts['headers'], list) else atts['headers'].replace('\r\n', '\n').split('\n')
        head = {h.split(':', 1)[0].lower(): h.split(':', 1)[1].strip() for h in headers if ':' in h}
        to = atts['to'] if isinstance(atts['to'], list) else [atts['to']]
        out.append({'to': ','.join(to), 'subject': atts['subject'], 'body': atts['message'], 'reply-to': head.get('reply-to', ''), 'cc': head.get('cc', ''), 'content-type': head.get('content-type', '').split(';')[0]})
    return out, len(log)


def sink_mails(start):
    if not os.path.exists(SINK):
        return [], 0
    blocks = open(SINK, encoding='utf-8', newline='').read().split('=== FROM ')[1:]
    out = []
    for block in blocks[start:]:
        _, _, raw = block.partition('\n')
        msg = email.message_from_string(raw, policy=email.policy.default)
        part = msg.get_body(('html', 'plain')) or msg
        body = part.get_content() if part.get_content_maintype() == 'text' else ''
        out.append({'to': str(msg['To'] or ''), 'subject': str(msg['Subject'] or ''), 'body': body.replace('\r\n', '\n'), 'reply-to': str(msg['Reply-To'] or ''), 'cc': str(msg['Cc'] or ''), 'content-type': msg.get_content_type()})
    return out, len(blocks)


def entries(form_ids):
    ids = ','.join(str(i) for i in form_ids)
    return json.loads(php(f"echo json_encode( [ array_sum( array_map( function ( $id ) {{ return (int) get_post_meta( $id, 'wpforms_entries_count', true ); }}, [ {ids} ] ) ), get_option( 'wpforms_entries_count_daily' ), (int) get_option( 'wpforms_dashboard_cache_generation' ) ] );"))


def submissions(cases, limit, read_only):
    failures = 0
    ref_start = reference_mails(0)[1]
    sink_start = sink_mails(0)[1]
    for name, fields in SUBMISSIONS:
        form_id = cases[name][0]
        path = f'/wpforms-{name}/'
        answers = []
        counts = []
        for port in (8080, 3001):
            page = fetch(port, None, path)[2]
            start = entries([form_id])
            answers.append(post(port, hidden_fields(page, port, path) + fields))
            end = entries([form_id])
            counts.append((end[0] - start[0], end[2] - start[2], json.dumps(end[1]) != json.dumps(start[1])))
        left, right = (normalize(a[1]) for a in answers)
        same = answers[0][0] == answers[1][0] and left == right and counts[1] == ((0, 0, False) if read_only else counts[0])
        print(f"{'ok  ' if same else 'DIFF'} submit {name} {fields[0][1]!r} (entries, cache generation, daily: {counts[0]})")
        if not same:
            failures += 1
            print(f'    status {answers[0][0]} / {answers[1][0]}, counts {counts[0]} / {counts[1]}')
            diff(left, right, limit)
    time.sleep(1)
    ref, _ = reference_mails(ref_start)
    rp, _ = sink_mails(sink_start)
    if len(ref) != len(rp):
        failures += 1
        print(f'DIFF e-mails: {len(ref)} / {len(rp)}')
    for i, (a, b) in enumerate(zip(ref, rp)):
        for key in a:
            left, right = normalize(a[key]).replace('\r\n', '\n').strip(), normalize(b[key]).replace('\r\n', '\n').strip()
            if left != right:
                failures += 1
                print(f'DIFF e-mail {i} {key}')
                diff(left, right, limit)
    print(f'{len(ref)} e-mails compared')
    # get_token and restricted_email.
    contact = cases['contact'][0]
    email_form = cases['email'][0]
    for fields in ([('action', 'wpforms_get_token'), ('formId', str(contact))], [('action', 'wpforms_get_token'), ('formId', 'x')],
                   [('action', 'wpforms_restricted_email'), ('form_id', str(email_form)), ('field_id', '1'), ('email', 'a@b.de')],
                   [('action', 'wpforms_restricted_email'), ('form_id', str(email_form)), ('field_id', '9'), ('email', 'a@b.de')],
                   [('action', 'wpforms_restricted_email'), ('form_id', ''), ('field_id', '1'), ('email', 'a@b.de')]):
        a = post(8080, fields, 'urlencoded')
        b = post(3001, fields, 'urlencoded')
        norm = lambda t: re.sub(r'"time_token":"\d+\.[0-9a-f]{64}"', '"time_token":"T"', normalize(t))
        same = a[0] == b[0] and norm(a[1]) == norm(b[1])
        print(f"{'ok  ' if same else 'DIFF'} {fields[0][1]} {fields[1:]}")
        if not same:
            failures += 1
            print(f'    {a} / {b}')
    return failures


def diff(a, b, limit):
    import difflib
    lines = list(difflib.unified_diff(a.splitlines(), b.splitlines(), 'reference', 'rustpress', lineterm='', n=1))
    for line in lines[:limit]:
        print('   ', line[:400])
    return len(lines)


def main():
    args = sys.argv[1:]
    keep = '--keep' in args
    read_only = '--read-only' in args
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
            for user in (None,) if read_only else (None, 'rpadmin'):
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
        failures += submissions(cases, limit, read_only)
    finally:
        if not keep:
            ids = ','.join(str(i) + ',' + str(i + 1) for _, i in created)
            php(f"foreach ( [ {ids} ] as $id ) {{ wp_delete_post( $id, true ); }}")
    print(f'{failures} differences')
    sys.exit(1 if failures else 0)


if __name__ == '__main__':
    main()
