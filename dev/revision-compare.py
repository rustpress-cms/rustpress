#!/usr/bin/env python3
"""Compares revision.php (the localized `_wpRevisionsSettings`) and the `get-revision-diffs` action of the reference
and RustPress for the revisions of post 1 (or the ids given): python dev/revision-compare.py [post_id revision_id]"""
import json
import re
import sys
import urllib.parse

sys.path.insert(0, __file__.rsplit('/', 1)[0] if '/' in __file__ else '.')
import head_compare as h

post_id = sys.argv[1] if len(sys.argv) > 1 else '1'
revision = sys.argv[2] if len(sys.argv) > 2 else '308'


def settings(port, query):
    t = h.fetch(port, query)
    m = re.search(r'var _wpRevisionsSettings = (\{.*?\});\n', t, re.S)
    return json.loads(m.group(1)) if m else None


def strip(d):
    if d:
        d = json.loads(json.dumps(d))
        d.pop('nonce', None)
        for r in d.get('revisionData', []):
            r.pop('restoreUrl', None)
    return d


bad = 0
for q in (f'revision.php?revision={revision}', f'revision.php?revision={revision}&from=0&to={revision}', 'revision.php?revision=999999'):
    a, b = strip(settings(8080, q)), strip(settings(3001, q))
    same = a == b
    bad += not same
    print('SAME' if same else 'DIFF', q)
ids = [r['id'] for r in (settings(8080, f'revision.php?revision={revision}') or {}).get('revisionData', [])]
pairs = [f'{ids[0]}:{ids[-1]}', f'0:{ids[0]}', f'{ids[-1]}:{ids[0]}', '999:1'] if len(ids) > 1 else [f'0:{revision}']
out = {}
for port in (8080, 3001):
    s = settings(port, f'revision.php?revision={revision}')
    body = urllib.parse.urlencode([('action', 'get-revision-diffs'), ('post_id', post_id), ('nonce', s['nonce'])] + [('compare[]', p) for p in pairs]).encode()
    out[port] = h._openers[port].open(f'http://localhost:{port}/wp-admin/admin-ajax.php', body).read().decode()
same = out[8080] == out[3001]
bad += not same
print('SAME' if same else 'DIFF', 'get-revision-diffs', pairs)
sys.exit(1 if bad else 0)
