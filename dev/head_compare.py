#!/usr/bin/env python3
"""Compares the <head> of wp-admin screens of the reference (:8080) and RustPress (:3001), signed in as rpadmin.

    python dev/head_compare.py index.php options-general.php [--full]

Prints the elements of the head (styles, links, scripts with their text) that differ. Hosts, ports, nonces and
times are replaced; `--full` also prints the elements that are the same."""
import difflib
import http.cookiejar
import re
import sys
import urllib.parse
import urllib.request


_openers = {}


def fetch(port, page):
    """The page as rpadmin; the sign-in happens once per port (it is slow in a debug build)."""
    if port not in _openers:
        jar = http.cookiejar.CookieJar()
        opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
        opener.open(f'http://localhost:{port}/wp-login.php').read()
        opener.open(f'http://localhost:{port}/wp-login.php', urllib.parse.urlencode({'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()
        _openers[port] = opener
    return _openers[port].open(f'http://localhost:{port}/wp-admin/{page}').read().decode('utf-8', 'replace')


def normalize(text, port):
    text = text.replace(f'localhost:{port}', 'HOST').replace(f'127.0.0.1:{port}', 'HOST')
    text = re.sub(r'"time":"\d+"', '"time":"T"', text)
    text = re.sub(r'(_wpnonce|nonce)(["\':= ]+)[0-9a-f]{10}', r'\1\2N', text)
    text = re.sub(r'\s+', ' ', text)
    return text


def elements(html, port):
    head = html.split('</head>')[0]
    head = head.split('<head>', 1)[-1]
    head = normalize(head, port)
    parts = re.findall(r'<script[^>]*>.*?</script>|<style[^>]*>.*?</style>|<link[^>]*>|<meta[^>]*>|<title>.*?</title>', head, re.S)
    body = re.search(r'<body[^>]*>', html)
    if body:
        parts.append(normalize(body.group(0), port))
    return [p.strip() for p in parts]


def main():
    pages = [a for a in sys.argv[1:] if not a.startswith('--')]
    full = '--full' in sys.argv
    failed = False
    for page in pages:
        a = elements(fetch(8080, page), 8080)
        b = elements(fetch(3001, page), 3001)
        ops = [op for op in difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes() if op[0] != 'equal']
        if not ops:
            print(f'same head  {page} ({len(a)} elements)')
            continue
        failed = True
        print(f'DIFFERENT  {page}: {len(ops)} places')
        for tag, i1, i2, j1, j2 in ops:
            print(f'  {tag}:')
            for item in a[i1:i2]:
                print('    ref: ' + item[:260])
            for item in b[j1:j2]:
                print('    rp : ' + item[:260])
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
