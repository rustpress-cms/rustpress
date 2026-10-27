#!/usr/bin/env python3
"""send-attachment-to-editor on both servers (post_id 0: nothing is attached) with the forms the media modal sends."""
import http.cookiejar, re, urllib.error, urllib.parse, urllib.request


def session(port):
    jar = http.cookiejar.CookieJar()
    o = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    base = f'http://127.0.0.1:{port}'
    o.open(base + '/wp-login.php').read()
    o.open(base + '/wp-login.php', urllib.parse.urlencode({'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()
    page = o.open(base + '/wp-admin/post-new.php').read().decode()
    nonce = re.search(r'"sendToEditor":"([0-9a-f]+)"', page)
    if not nonce:
        nonce = re.search(r'media-send-to-editor[^0-9a-f]*([0-9a-f]{10})', page)
    return o, base, nonce.group(1)


def cases(port):
    site = f'http://localhost:{port}'
    up = f'{site}/wp-content/uploads/2026/10'
    return [
        ('image medium file link', {'id': 26, 'url': f'{up}/building-exterior.webp', 'align': 'none', 'image-size': 'medium', 'image_alt': 'Fassade'}),
        ('image full no link', {'id': 26, 'url': '', 'align': 'left', 'image-size': 'full', 'image_alt': ''}),
        ('image thumbnail attachment page', {'id': 26, 'url': f'{site}/hello-world/gebaude-ausen/', 'align': 'center', 'image-size': 'thumbnail', 'image_alt': 'a "b" <c>'}),
        ('image caption', {'id': 26, 'url': f'{up}/building-exterior.webp', 'align': 'right', 'image-size': 'large', 'post_excerpt': 'Zeile eins\r\nZeile <em>zwei</em>\n  drei', 'image_alt': 'x'}),
        ('image caption blank', {'id': 25, 'url': '', 'align': 'none', 'image-size': 'medium', 'post_excerpt': '   ', 'image_alt': 'Logo'}),
        ('image size missing', {'id': 25, 'url': '', 'image-size': 'large'}),
        ('image defaults', {'id': 299}),
        ('image attachment_id url', {'id': 299, 'url': f'{site}/?attachment_id=299', 'image-size': 'medium'}),
        ('video html', {'id': 281, 'url': f'{up}/film.mp4', 'post_title': 'Film'}, {'html': '[video src="x.mp4"][/video]'}),
        ('missing', {'id': 999999}),
    ]


results = {}
for port in (8080, 3001):
    o, base, nonce = session(port)
    rows = []
    for case in cases(port):
        name, attachment = case[0], case[1]
        extra = case[2] if len(case) > 2 else {}
        data = {'action': 'send-attachment-to-editor', 'nonce': nonce, 'post_id': 0}
        data.update({f'attachment[{k}]': v for k, v in attachment.items()})
        data.update(extra)
        try:
            r = o.open(base + '/wp-admin/admin-ajax.php', urllib.parse.urlencode(data).encode())
            body = r.read().decode()
            status = r.status
        except urllib.error.HTTPError as e:
            status, body = e.code, e.read().decode()
        rows.append((name, status, body.replace(f'localhost:{port}', 'localhost:PORT').replace(f'localhost:\\/\\/{port}', '')))
    results[port] = rows
same = 0
for (name, s1, b1), (_, s2, b2) in zip(results[8080], results[3001]):
    if (s1, b1) == (s2, b2):
        same += 1
        print('ok  ', name)
    else:
        print('DIFF', name, s1, s2)
        print('   ref', b1[:500])
        print('   rp ', b2[:500])
print(f'{same}/{len(results[8080])} identical')
