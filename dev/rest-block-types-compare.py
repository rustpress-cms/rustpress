#!/usr/bin/env python3
"""Compares `wp/v2/block-types` of the reference (:8080) and RustPress (:3001): the list block by block and field by
field, then single blocks, namespaces and errors (status, `Allow` header, body).

    python dev/rest-block-types-compare.py [--diff]"""
import difflib
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('rc', os.path.join(HERE, 'rest_compare.py'))
rc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rc)

PATHS = [
    '/wp/v2/block-types/core/paragraph', '/wp/v2/block-types/core/archives', '/wp/v2/block-types/core/post-comments',
    '/wp/v2/block-types/core/nope', '/wp/v2/block-types/nope', '/wp/v2/block-types?namespace=nope',
    '/wp/v2/block-types/core/navigation-link?context=edit', '/wp/v2/block-types/core/a/b',
    '/wp/v2/block-types?_fields=name,title', '/wp/v2/block-types/core/list?context=embed',
]


def fetch(port, path):
    status, headers, body = rc.call(port, 'GET', path, None)
    try:
        body = rc.normalize(json.loads(body), port)
    except ValueError:
        pass
    return status, headers.get('Allow'), body


def main():
    show = '--diff' in sys.argv
    same = total = 0
    a, b = fetch(8080, '/wp/v2/block-types')[2], fetch(3001, '/wp/v2/block-types')[2]
    names_a, names_b = [x['name'] for x in a], [x['name'] for x in b]
    if names_a != names_b:
        print('ORDER', [n for n in names_a if n not in names_b], [n for n in names_b if n not in names_a])
    by_b = {x['name']: x for x in b}
    for x in a:
        y = by_b.get(x['name'], {})
        for key in x:
            total += 1
            if x.get(key) == y.get(key):
                same += 1
                continue
            sa, sb = json.dumps(x.get(key), ensure_ascii=False), json.dumps(y.get(key), ensure_ascii=False)
            at = next((i for i, (p, q) in enumerate(zip(sa, sb)) if p != q), min(len(sa), len(sb)))
            print('FIELD', x['name'], key, '|', sa[max(0, at - 60):at + 80], '|', sb[max(0, at - 60):at + 80])
        if list(x) != list(y):
            print('KEYS', x['name'], list(x), list(y))
    for path in PATHS:
        total += 1
        out = [fetch(port, path) for port in (8080, 3001)]
        if out[0] == out[1]:
            same += 1
            continue
        print('DIFF', path, out[0][:2], out[1][:2])
        if show:
            pa, pb = json.dumps(out[0][2], indent=1).splitlines(), json.dumps(out[1][2], indent=1).splitlines()
            for line in list(difflib.unified_diff(pa, pb, lineterm='', n=1))[2:20]:
                print('   ', line[:200])
    print(f'{same}/{total} identical')


if __name__ == '__main__':
    main()
