#!/usr/bin/env python3
"""Calls get-comments like the comments box of the classic edit form (post.js) on both servers and compares."""
import html, http.cookiejar, re, sys, urllib.parse, urllib.request


def session(port, user='rpadmin'):
    jar = http.cookiejar.CookieJar()
    o = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    base = f'http://127.0.0.1:{port}'
    o.open(base + '/wp-login.php').read()
    o.open(base + '/wp-login.php', urllib.parse.urlencode({'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()
    return o, base


post = sys.argv[1] if len(sys.argv) > 1 else '1'
extra = sys.argv[2:]  # start=..., number=...
results = []
for port in (8080, 3001):
    o, base = session(port)
    page = o.open(f'{base}/wp-admin/post.php?post={post}&action=edit&rp-classic=1').read().decode()
    nonce = re.search(r'id="add_comment_nonce" name="add_comment_nonce" value="([0-9a-f]+)"', page).group(1)
    data = {'action': 'get-comments', 'mode': 'single', '_ajax_nonce': nonce, 'p': post, 'start': 0, 'number': 20}
    for e in extra:
        k, v = e.split('=', 1)
        data[k] = v
    r = o.open(base + '/wp-admin/admin-ajax.php', urllib.parse.urlencode(data).encode())
    body = r.read().decode()
    body = body.replace(f'localhost:{port}', 'localhost:PORT')
    body = re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', body)
    # antispambot() encodes addresses at random.
    body = re.sub(r'mailto:([^"]*)">([^<]*)</a>', lambda m: 'mailto:' + html.unescape(m.group(1)) + '">' + html.unescape(m.group(2)) + '</a>', body)
    results.append((r.status, r.headers.get('Content-Type'), body))
print(results[0][0], results[0][1], len(results[0][2]))
print(results[1][0], results[1][1], len(results[1][2]))
if results[0] == results[1]:
    print('identical')
else:
    import difflib
    a, b = results[0][2].replace('><', '>\n<').splitlines(), results[1][2].replace('><', '>\n<').splitlines()
    for line in list(difflib.unified_diff(a, b, lineterm='', n=1))[:60]:
        print(line[:300])
