#!/usr/bin/env python3
"""Compares the `get-attachment` answer (`wp_prepare_attachment_for_js()`) of the reference and RustPress for attachments:
python dev/attachment-js-compare.py [id ...]  (default: every attachment the reference lists)"""
import json
import re
import sys
import urllib.parse

sys.path.insert(0, __file__.rsplit('/', 1)[0] if '/' in __file__ else '.')
import head_compare as h


def post(port, fields):
    h.fetch(port, 'index.php')
    body = urllib.parse.urlencode(fields).encode()
    return json.loads(h._openers[port].open(f'http://localhost:{port}/wp-admin/admin-ajax.php', body).read().decode())


def norm(data):
    s = json.dumps(data, sort_keys=True)
    s = re.sub(r'localhost:\d+', 'H', s)
    s = re.sub(r'"(update|delete|edit)": "[0-9a-f]+"', r'"\1": "N"', s)
    return json.loads(s)


def walk(x, y, path=''):
    out = []
    if type(x) is not type(y):
        return [f'{path} TYPE {x!r:.80} | {y!r:.80}']
    if isinstance(x, dict):
        for k in sorted(set(x) | set(y)):
            if k not in x:
                out.append(f'{path}/{k} only RP {y[k]!r:.100}')
            elif k not in y:
                out.append(f'{path}/{k} only REF {x[k]!r:.100}')
            else:
                out += walk(x[k], y[k], f'{path}/{k}')
    elif isinstance(x, list):
        if len(x) != len(y):
            out.append(f'{path} LEN {len(x)} {len(y)}')
        for i, (p, q) in enumerate(zip(x, y)):
            out += walk(p, q, f'{path}[{i}]')
    elif x != y:
        out.append(f'{path} VAL {x!r:.100} | {y!r:.100}')
    return out


ids = sys.argv[1:]
if not ids:
    listing = post(8080, [('action', 'query-attachments'), ('query[posts_per_page]', '100')])
    ids = [str(a['id']) for a in listing['data']]
bad = 0
for aid in ids:
    a = norm(post(8080, [('action', 'get-attachment'), ('id', aid)]))
    b = norm(post(3001, [('action', 'get-attachment'), ('id', aid)]))
    diffs = walk(a, b)
    if diffs:
        bad += 1
        print('DIFF', aid)
        for d in diffs:
            print('   ', d)
    else:
        print('same', aid)
sys.exit(1 if bad else 0)
