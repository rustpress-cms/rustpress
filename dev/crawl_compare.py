#!/usr/bin/env python3
"""Fetches the wp-admin pages linked from the menu and from the first pages (reference :8080 and RustPress :3001, signed in
as rpadmin) and reports the ones whose status or title differ.

    python dev/crawl_compare.py [--depth 2] [--limit 200]"""
import html
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

sys.path.insert(0, __file__.rsplit('/', 1)[0] if '/' in __file__ else '.')
import head_compare as h


def get(port, path):
    h.fetch(port, 'index.php')  # makes sure the sign-in happened
    opener = h._openers[port]
    try:
        r = opener.open(f'http://localhost:{port}/wp-admin/{path}', timeout=40)
        return r.status, r.read().decode('utf-8', 'replace'), r.geturl()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode('utf-8', 'replace'), path


SKIP = re.compile(r'(action=(trash|untrash|delete|spam|unspam|approve|unapprove|activate|deactivate|logout|dodelete)|_wpnonce|wp-login|logout|loadWizard|\.zip|download|export=|plugin-install\.php\?tab=(?!upload)|theme-install\.php\?(?!upload)|update-core\.php\?action|admin-post|admin-ajax)', re.I)


def links(page_html, port):
    found = set()
    for href in re.findall(r"""href=["']([^"'#]+)["']""", page_html):
        href = html.unescape(href)
        href = href.replace(f'http://localhost:{port}/wp-admin/', '')
        if href.startswith(('http', '//', 'mailto', 'javascript')) or href.startswith('/'):
            continue
        if not re.match(r'^[a-z0-9_-]+\.php(\?.*)?$', href) or SKIP.search(href):
            continue
        found.add(href)
    return found


def title(t):
    m = re.search(r'<title>(.*?)</title>', t, re.S)
    return re.sub(r'\s+', ' ', m.group(1)).strip() if m else ''


def main():
    depth = 2
    limit = 200
    args = sys.argv[1:]
    if '--depth' in args:
        depth = int(args[args.index('--depth') + 1])
    if '--limit' in args:
        limit = int(args[args.index('--limit') + 1])
    seen = set()
    queue = ['index.php']
    level = 0
    bad = 0
    while queue and level <= depth and len(seen) < limit:
        next_queue = set()
        for path in sorted(queue):
            if path in seen or len(seen) >= limit:
                continue
            seen.add(path)
            sa, ta, _ = get(8080, path)
            sb, tb, _ = get(3001, path)
            note = None
            if sa != sb:
                note = f'status {sa} vs {sb}'
            elif title(ta) != title(tb):
                note = f'title {title(ta)!r} vs {title(tb)!r}'
            if note:
                bad += 1
                print(f'DIFF {path}: {note}', flush=True)
            if sa == 200:
                next_queue |= links(ta, 8080)
        queue = list(next_queue - seen)
        level += 1
    print(f'{len(seen)} pages, {bad} differ')


main()
