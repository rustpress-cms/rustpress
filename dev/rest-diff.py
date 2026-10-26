#!/usr/bin/env python3
"""Prints the differences between the answers of the reference and RustPress for GET routes (as rpadmin).

    python dev/rest-diff.py /wp/v2/tags [/wp/v2/categories ...] [--max N]"""
import importlib.util
import json
import os
import sys

spec = importlib.util.spec_from_file_location('rc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'rest_compare.py'))
rc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rc)
MAX = 8


def walk(a, b, path, out):
    if len(out) >= MAX:
        return
    if type(a) != type(b):
        out.append(f'{path}: ref {json.dumps(a)[:110]} / rp {json.dumps(b)[:110]}')
    elif isinstance(a, dict):
        for k in a:
            if k not in b:
                out.append(f'{path}/{k}: missing in rp ({json.dumps(a[k])[:90]})')
            else:
                walk(a[k], b[k], f'{path}/{k}', out)
        for k in b:
            if k not in a:
                out.append(f'{path}/{k}: extra in rp ({json.dumps(b[k])[:90]})')
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(f'{path}: length ref {len(a)} / rp {len(b)}')
        for i, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f'{path}[{i}]', out)
    elif a != b:
        out.append(f'{path}: ref {json.dumps(a)[:110]} / rp {json.dumps(b)[:110]}')


args = sys.argv[1:]
if '--max' in args:
    MAX = int(args[args.index('--max') + 1])
    del args[args.index('--max'):args.index('--max') + 2]
for path in args:
    a = rc.call(8080, 'GET', path, None)
    b = rc.call(3001, 'GET', path, None)
    print('==', path, a[0], b[0])
    if a[0] != b[0]:
        print('  ref:', a[2][:200].replace('\n', ' '))
        print('  rp: ', b[2][:200].replace('\n', ' '))
        continue
    try:
        out = []
        walk(rc.normalize(json.loads(a[2]), 8080), rc.normalize(json.loads(b[2]), 3001), '', out)
        for line in out:
            print('  ', line)
    except ValueError:
        print('  not JSON', a[2][:100], b[2][:100])
