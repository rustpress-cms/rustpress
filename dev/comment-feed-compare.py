#!/usr/bin/env python3
"""Compares the comment feeds of archives and searches (`withcomments`, `feed=comments-*`; `WP_Query::get_posts()`) of
the reference (:8080) and RustPress (:3001): status, headers and body. The stored GUIDs carry the address of the
reference.

    python dev/comment-feed-compare.py [lines of diff]"""
import re
import sys
import urllib.request
import urllib.error
import difflib

PATHS = [
    '/?feed=comments-rss2',
    '/?s=hello&feed=comments-rss2',
    '/?s=hello&feed=comments-atom',
    '/?s=hello&feed=rss2&withcomments=1',
    '/?s=no-such-thing-xyz&feed=comments-rss2',
    '/?s=no-such-thing-xyz&feed=comments-atom',
    '/?s=hello&feed=comments-rss',
    '/?s=hello&feed=comments-rdf',
    '/?cat=1&feed=comments-rss2',
    '/?cat=1&feed=comments-atom',
    '/?cat=1&feed=comments-rss',
    '/?cat=1&feed=comments-rdf',
    '/?cat=1&feed=rss2&withcomments=1',
    '/?cat=1&feed=rss2&withcomments=0',
    '/?author=1&feed=comments-rss2',
    '/?m=2026&feed=comments-rss2',
    '/?year=2026&monthnum=10&feed=comments-atom',
    '/?tag=rp-tag&feed=comments-rss2',
    '/?post_type=post&feed=comments-rss2',
    '/?cat=1&paged=2&feed=comments-rss2',
    '/?cat=999&feed=comments-rss2',
    '/category/uncategorized/feed/?withcomments=1',
    '/category/uncategorized/feed/atom/?withcomments=1',
    '/search/hello/feed/?withcomments=1',
    '/search/hello/feed/atom/?withcomments=1',
    '/search/hello+world/feed/?withcomments=1',
    '/search/a%26b/feed/atom/?withcomments=1',
    '/author/admin/feed/?withcomments=1',
    '/2026/feed/?withcomments=1',
    '/2026/10/feed/atom/?withcomments=1',
]


def get(port, path):
    req = urllib.request.Request(f'http://127.0.0.1:{port}{path}', headers={'Host': f'localhost:{port}'})

    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *a, **k):
            return None

    opener = urllib.request.build_opener(NoRedirect)
    try:
        r = opener.open(req, timeout=30)
        return r.status, dict(r.headers), r.read().decode('utf-8', 'replace')
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read().decode('utf-8', 'replace')


def norm(s, port):
    return s.replace(f'localhost:{port}', 'HOST').replace('localhost:8080', 'HOST')


show = int(sys.argv[1]) if len(sys.argv) > 1 else 0
bad = 0
for path in PATHS:
    a = get(8080, path)
    b = get(3001, path)
    ha = {k.lower(): norm(v, 8080) for k, v in a[1].items() if k.lower() in ('content-type', 'location', 'last-modified', 'etag')}
    hb = {k.lower(): norm(v, 3001) for k, v in b[1].items() if k.lower() in ('content-type', 'location', 'last-modified', 'etag')}
    ba, bb = norm(a[2], 8080), norm(b[2], 3001)
    problems = []
    if a[0] != b[0]:
        problems.append(f'status {a[0]} {b[0]}')
    if ha != hb:
        problems.append(f'headers {ha} {hb}')
    if ba != bb:
        problems.append('body')
    items = len(re.findall(r'<(?:item|entry)\b', ba))
    print(('ok   ' if not problems else 'DIFF ') + f'{path} [{a[0]}, {items} items] ' + '; '.join(problems), flush=True)
    if problems:
        bad += 1
        if show and 'body' in problems:
            for line in list(difflib.unified_diff(ba.splitlines(), bb.splitlines(), lineterm='', n=0))[:show]:
                print('     ', line[:220])
print('differences:', bad, 'of', len(PATHS))
