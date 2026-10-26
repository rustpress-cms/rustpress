#!/usr/bin/env python3
"""Runs the privacy export and erase of one request on the reference (:8080) and on RustPress (:3001) and compares
the answers of every step (and the files of the export).

    python dev/privacy-compare.py [email]

A request for the address is added through RustPress (confirmed, as RustPress cannot send e-mail), both servers
process it (they share the database; each writes its own export file) and the request is deleted at the end."""
import difflib
import http.cookiejar
import json
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
import zipfile
import io

EMAIL = sys.argv[1] if len(sys.argv) > 1 else 'admin@example.com'


class Site:
    def __init__(self, port):
        self.port = port
        self.op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
        self.op.open(f'http://127.0.0.1:{port}/wp-login.php').read()
        self.op.open(f'http://127.0.0.1:{port}/wp-login.php', urllib.parse.urlencode({'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()

    def get(self, path):
        return self.op.open(f'http://127.0.0.1:{self.port}{path}').read().decode('utf-8', 'replace')

    def post(self, path, fields):
        try:
            r = self.op.open(f'http://127.0.0.1:{self.port}{path}', urllib.parse.urlencode(fields).encode())
            return r.status, r.read().decode('utf-8', 'replace')
        except urllib.error.HTTPError as e:
            return e.code, e.read().decode('utf-8', 'replace')

    def raw(self, url):
        return self.op.open(url).read()


def norm(text, port):
    text = text.replace(f'localhost:{port}', 'HOST').replace(f'localhost%3A{port}', 'HOST')
    return re.sub(r'wp-personal-data-file-[A-Za-z0-9]{32}', 'wp-personal-data-file-X', text)


def main():
    rp, ref = Site(3001), Site(8080)
    page = rp.get('/wp-admin/export-personal-data.php')
    nonce = re.search(r'name="_wpnonce" value="([0-9a-f]+)"', page).group(1)
    status, body = rp.post('/wp-admin/export-personal-data.php', {'_wpnonce': nonce, 'action': 'add_export_personal_data_request', 'type_of_action': 'export_personal_data', 'username_or_email_for_privacy_request': EMAIL})
    page = rp.get('/wp-admin/export-personal-data.php')
    ids = re.findall(r'id="request-(\d+)"', page)
    request_id = max(int(i) for i in ids)
    raw_answers = {}
    print('request', request_id)
    try:
        for exporter in (1, 2, 3):
            answers = []
            for site in (ref, rp):
                list_page = site.get('/wp-admin/export-personal-data.php')
                token = re.search(rf'data-request-id="{request_id}" data-nonce="([0-9a-f]+)"', list_page).group(1)
                status, text = site.post('/wp-admin/admin-ajax.php', {'action': 'wp-privacy-export-personal-data', 'id': request_id, 'security': token, 'exporter': exporter, 'page': 1, 'sendAsEmail': 'false'})
                answers.append((status, norm(text, site.port)))
                raw_answers[site.port] = text
            same = answers[0] == answers[1]
            print('exporter', exporter, 'identical' if same else 'DIFFERENT', answers[0][0], answers[1][0])
            if not same:
                a, b = answers[0][1], answers[1][1]
                for line in difflib.unified_diff(re.sub(r'},{', '},\n{', a).splitlines(), re.sub(r'},{', '},\n{', b).splitlines(), lineterm='', n=0):
                    print('   ', line[:220])
        # The archives.
        for site in (ref, rp):
            url = re.search(r'"url":"([^"]+)"', raw_answers[site.port])
            if not url:
                print(site.port, 'no url')
                continue
            data = site.raw(re.sub(r'^https?://[^/]+', 'http://127.0.0.1:%d' % site.port, url.group(1).replace(chr(92) + '/', '/')))
            archive = zipfile.ZipFile(io.BytesIO(data))
            print(site.port, archive.namelist())
            for name in archive.namelist():
                text = archive.read(name).decode('utf-8')
                open(f'{__import__("os").environ.get("TEMP", ".")}/privacy-{site.port}-{name}', 'w', encoding='utf-8').write(norm(text, site.port))
    finally:
        for action in ('complete', 'delete'):
            page = rp.get('/wp-admin/export-personal-data.php')
            link = re.search(rf'action={action}&(?:amp;|#038;)?request_id%5B0%5D={request_id}&(?:amp;|#038;)?_wpnonce=([0-9a-f]+)', page)
            if link:
                rp.get(f'/wp-admin/export-personal-data.php?action={action}&request_id%5B0%5D={request_id}&_wpnonce={link.group(1)}')
        print('deleted' if f'request-{request_id}"' not in rp.get('/wp-admin/export-personal-data.php') else 'NOT deleted')


if __name__ == '__main__':
    main()
