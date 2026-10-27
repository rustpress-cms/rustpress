#!/usr/bin/env python3
"""replyto-comment and edit-comment on both servers (list mode and the box of the edit form), answers compared with
IDs, nonces, contents and times normalized; the comments are deleted afterwards."""
import html, http.cookiejar, json, re, urllib.error, urllib.parse, urllib.request


def session(port):
    jar = http.cookiejar.CookieJar()
    o = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    base = f'http://127.0.0.1:{port}'
    o.open(base + '/wp-login.php').read()
    o.open(base + '/wp-login.php', urllib.parse.urlencode({'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()
    page = o.open(base + '/wp-admin/edit-comments.php').read().decode()
    reply = re.search(r'id="_ajax_nonce-replyto-comment" name="_ajax_nonce-replyto-comment" value="([0-9a-f]+)"', page).group(1)
    dashboard = o.open(base + '/wp-admin/').read().decode()
    rest = re.search(r'wpApiSettings = \{.*?"nonce":"([0-9a-f]+)"', dashboard).group(1)
    return o, base, reply, rest


def post(o, url, data):
    try:
        r = o.open(url, urllib.parse.urlencode(data).encode())
        return r.status, r.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()


results = {}
created = {}
for port, tag in ((8080, 'A'), (3001, 'B')):
    o, base, reply, rest = session(port)
    out = []
    ids = []
    for mode in ('', 'single'):
        data = {'action': 'replyto-comment', '_ajax_nonce-replyto-comment': reply, 'comment_ID': 1, 'comment_post_ID': 1, 'content': f'cpc-reply-{tag}-{mode or "list"}', 'approve_parent': ''}
        if mode:
            data['mode'] = mode
        status, body = post(o, base + '/wp-admin/admin-ajax.php', data)
        m = re.search(r"<comment id='(\d+)'", body)
        cid = m.group(1) if m else None
        if cid:
            ids.append(cid)
        out.append((f'replyto {mode or "list"}', status, body, cid))
        if cid:
            data = {'action': 'edit-comment', '_ajax_nonce-replyto-comment': reply, 'comment_ID': cid, 'comment_post_ID': 1, 'content': f'cpc-edited-{tag}-{mode or "list"}', 'newcomment_author': 'rpadmin', 'newcomment_author_email': 'rpadmin@example.org', 'newcomment_author_url': ''}
            if mode:
                data['mode'] = mode
            status, body = post(o, base + '/wp-admin/admin-ajax.php', data)
            out.append((f'edit {mode or "list"}', status, body, cid))
            # From the comments screen the row carries a checkbox.
            status, body = post(o, base + '/wp-admin/admin-ajax.php', dict(data, checkbox='1', content=data['content'] + '-cb'))
            out.append((f'edit {mode or "list"} checkbox', status, body, cid))
    # The next server must see the same counts: these comments go before it runs.
    for cid in ids:
        o.open(urllib.request.Request(f'{base}/wp-json/wp/v2/comments/{cid}?force=true', method='DELETE', headers={'X-WP-Nonce': rest})).read()
    created[port] = (o, base, rest, [])

    def norm(text, cid):
        text = text.replace(f'localhost:{port}', 'localhost:PORT').replace(f'-{tag}-', '-X-')
        if cid:
            text = text.replace(f"_{cid}'", "_CID'").replace(f"'{cid}'", "'CID'").replace(f'-{cid}', '-CID').replace(f'={cid}', '=CID').replace(f'"{cid}"', '"CID"').replace(f'c={cid}', 'c=CID')
        text = re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', text)
        text = re.sub(r'mailto:([^"]*)">([^<]*)</a>', lambda m: 'mailto:' + html.unescape(m.group(1)) + '">' + html.unescape(m.group(2)) + '</a>', text)
        return re.sub(r'\d{4}/\d{2}/\d{2} at \d{1,2}:\d{2} [ap]m', 'TIME', text)
    results[port] = [(n, s, norm(b, c)) for n, s, b, c in out]

for (name, s1, b1), (_, s2, b2) in zip(results[8080], results[3001]):
    if (s1, b1) == (s2, b2):
        print('ok  ', name)
    else:
        print('DIFF', name, s1, s2)
        import difflib
        for line in list(difflib.unified_diff(b1.replace('><', '>\n<').splitlines(), b2.replace('><', '>\n<').splitlines(), lineterm='', n=1))[:30]:
            print('   ', line[:250])
for port, (o, base, rest, ids) in created.items():
    for cid in ids:
        r = urllib.request.Request(f'{base}/wp-json/wp/v2/comments/{cid}?force=true', method='DELETE', headers={'X-WP-Nonce': rest})
        try:
            o.open(r).read()
        except urllib.error.HTTPError as e:
            print('delete failed', cid, e.code)
print('deleted', sum(len(v[3]) for v in created.values()))
