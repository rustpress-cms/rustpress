#!/usr/bin/env python3
"""Runs the same Ajax sequence of the classic edit form (featured image, custom fields) on the reference and on
RustPress, each on a draft of its own, and compares the answers (IDs and nonces normalized). The drafts are deleted."""
import http.cookiejar, json, re, sys, urllib.error, urllib.parse, urllib.request


def session(port):
    jar = http.cookiejar.CookieJar()
    o = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    base = f'http://127.0.0.1:{port}'
    o.open(base + '/wp-login.php').read()
    o.open(base + '/wp-login.php', urllib.parse.urlencode({'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()
    page = o.open(base + '/wp-admin/').read().decode()
    rest = re.search(r'wpApiSettings = \{.*?"nonce":"([0-9a-f]+)"', page).group(1)
    return o, base, rest


def req(o, url, data=None, headers=None, method=None):
    body = urllib.parse.urlencode(data).encode() if isinstance(data, (dict, list)) else data
    r = urllib.request.Request(url, data=body, headers=headers or {}, method=method)
    try:
        resp = o.open(r)
        return resp.status, resp.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()


def run(port):
    o, base, rest = session(port)
    status, body = req(o, base + '/wp-json/wp/v2/posts', json.dumps({'title': 'cpc-ajax', 'status': 'draft'}).encode(), {'X-WP-Nonce': rest, 'Content-Type': 'application/json'})
    pid = json.loads(body)['id']
    page = req(o, f'{base}/wp-admin/post.php?post={pid}&action=edit&rp-classic=1')[1]
    update = re.search(r"id='_wpnonce' name='_wpnonce' value='([0-9a-f]+)'|id=\"_wpnonce\" name=\"_wpnonce\" value=\"([0-9a-f]+)\"", page)
    update = update.group(1) or update.group(2)
    addmeta = re.search(r'id="_ajax_nonce-add-meta" name="_ajax_nonce-add-meta" value="([0-9a-f]+)"', page).group(1)
    ajax = base + '/wp-admin/admin-ajax.php'
    out = []
    steps = [
        ('thumb set 25', {'action': 'set-post-thumbnail', 'post_id': pid, 'thumbnail_id': 25, 'json': 1, '_ajax_nonce': update}),
        ('thumb set 25 again', {'action': 'set-post-thumbnail', 'post_id': pid, 'thumbnail_id': 25, 'json': 1, '_ajax_nonce': update}),
        ('thumb html 26', {'action': 'get-post-thumbnail-html', 'post_id': pid, 'thumbnail_id': 26, '_ajax_nonce': update}),
        ('thumb html -1', {'action': 'get-post-thumbnail-html', 'post_id': pid, 'thumbnail_id': -1, '_ajax_nonce': update}),
        ('thumb set 26', {'action': 'set-post-thumbnail', 'post_id': pid, 'thumbnail_id': 26, 'json': 1, '_ajax_nonce': update}),
        ('thumb remove', {'action': 'set-post-thumbnail', 'post_id': pid, 'thumbnail_id': -1, 'json': 1, '_ajax_nonce': update}),
        ('thumb remove again', {'action': 'set-post-thumbnail', 'post_id': pid, 'thumbnail_id': -1, 'json': 1, '_ajax_nonce': update}),
        ('thumb post 1 (not an image)', {'action': 'set-post-thumbnail', 'post_id': pid, 'thumbnail_id': 1, 'json': 1, '_ajax_nonce': update}),
        ('meta none', {'action': 'add-meta', 'post_id': pid, 'metakeyselect': '#NONE#', 'metakeyinput': '', 'metavalue': 'x', '_ajax_nonce-add-meta': addmeta}),
        ('meta add', {'action': 'add-meta', 'post_id': pid, 'metakeyinput': 'cpc_key', 'metavalue': ' Wert <b>&</b> ', '_ajax_nonce-add-meta': addmeta}),
        ('meta protected', {'action': 'add-meta', 'post_id': pid, 'metakeyinput': '_secret', 'metavalue': 'v', '_ajax_nonce-add-meta': addmeta}),
    ]
    mid = None
    for name, data in steps:
        status, body = req(o, ajax, data)
        out.append((name, status, body))
        m = re.search(r"<meta id='(\d+)'", body)
        if name == 'meta add' and m:
            mid = m.group(1)
    if mid:
        dnonce = re.search(r'_ajax_nonce=([0-9a-f]{10})', out[-2][2]).group(1)
        for name, data in [
            ('meta update', {'action': 'add-meta', 'post_id': pid, f'meta[{mid}][key]': 'cpc_key2', f'meta[{mid}][value]': 'neu', '_ajax_nonce-add-meta': addmeta}),
            ('meta update same', {'action': 'add-meta', 'post_id': pid, f'meta[{mid}][key]': 'cpc_key2', f'meta[{mid}][value]': 'neu', '_ajax_nonce-add-meta': addmeta}),
            ('meta update empty key', {'action': 'add-meta', 'post_id': pid, f'meta[{mid}][key]': ' ', f'meta[{mid}][value]': 'neu', '_ajax_nonce-add-meta': addmeta}),
            ('meta delete', {'action': 'delete-meta', 'id': mid, '_ajax_nonce': dnonce}),
            ('meta delete again', {'action': 'delete-meta', 'id': mid, '_ajax_nonce': dnonce}),
        ]:
            status, body = req(o, ajax, data)
            out.append((name, status, body))
    req(o, f'{base}/wp-json/wp/v2/posts/{pid}?force=true', None, {'X-WP-Nonce': rest}, 'DELETE')

    def norm(text):
        text = text.replace(f'localhost:{port}', 'localhost:PORT').replace(str(pid), 'PID')
        if mid:
            text = text.replace(mid, 'MID')
        return re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', text)
    return [(n, s, norm(b)) for n, s, b in out]


a, b = run(8080), run(3001)
same = 0
for (name, s1, b1), (_, s2, b2) in zip(a, b):
    if (s1, b1) == (s2, b2):
        same += 1
        print('ok  ', name)
    else:
        print('DIFF', name, s1, s2)
        print('   ref', b1[:700])
        print('   rp ', b2[:700])
print(f'{same}/{len(a)} identical ({len(b)} steps on RustPress)')
