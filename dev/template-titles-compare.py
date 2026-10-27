#!/usr/bin/env python3
"""Makes custom templates (page-, single-, category-, tag-, taxonomy-, author-) on the reference, compares their titles
and descriptions in both answers and deletes them again."""
import http.cookiejar, json, re, urllib.error, urllib.parse, urllib.request


def session(port):
    jar = http.cookiejar.CookieJar()
    o = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    base = f'http://127.0.0.1:{port}'
    o.open(base + '/wp-login.php').read()
    o.open(base + '/wp-login.php', urllib.parse.urlencode({'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()
    page = o.open(base + '/wp-admin/').read().decode()
    return o, base, re.search(r'wpApiSettings = \{.*?"nonce":"([0-9a-f]+)"', page).group(1)


def call(o, base, nonce, method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(base + '/wp-json' + path, data=data, method=method, headers={'X-WP-Nonce': nonce, 'Content-Type': 'application/json'})
    try:
        resp = o.open(r)
        return resp.status, resp.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()


ref, rp = session(8080), session(3001)
slugs = ['page-ueber-uns', 'page-gibt-es-nicht', 'single-post-testbeitrag-1', 'single-post-fehlt', 'category-uncategorized', 'category-fehlt', 'tag-alpha',
         'taxonomy-post_tag-beta', 'author-rpadmin', 'author-niemand', 'my-page-ueber-uns']
made = []
for slug in slugs:
    status, body = call(*ref, 'POST', '/wp/v2/templates', {'slug': slug, 'content': '<!-- wp:paragraph --><p>x</p><!-- /wp:paragraph -->'})
    if status in (200, 201):
        made.append(json.loads(body)['id'])
    else:
        print('could not make', slug, status, body[:200])
try:
    for tid in made:
        rows = []
        for s in (ref, rp):
            status, body = call(*s, 'GET', f'/wp/v2/templates/{tid}?context=edit&_fields=title,description')
            rows.append((status, body))
        print('ok  ' if rows[0] == rows[1] else 'DIFF', tid, rows[0][1][:200])
        if rows[0] != rows[1]:
            print('   rp ', rows[1][1][:200])
finally:
    for tid in made:
        call(*ref, 'DELETE', f'/wp/v2/templates/{tid}?force=true')
    print('deleted', len(made))
