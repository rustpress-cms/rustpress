#!/usr/bin/env python3
"""Downloads wp-block-editor/v1/export from the reference (:8080) and RustPress (:3001) and compares the ZIP files:
the names of the entries and the content of each (theme.json parsed, the rest byte for byte).

    python dev/export-compare.py"""
import importlib.util
import io
import json
import os
import urllib.request
import zipfile

spec = importlib.util.spec_from_file_location('rc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'rest_compare.py'))
rc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rc)


def fetch(port):
    opener, nonce = rc.session(port)
    request = urllib.request.Request(f'http://127.0.0.1:{port}/wp-json/wp-block-editor/v1/export', headers={'X-WP-Nonce': nonce})
    response = opener.open(request)
    return response.status, {k.title(): v for k, v in response.headers.items()}, response.read()


def walk(a, b, path, out):
    if len(out) >= 12:
        return
    if type(a) != type(b):
        out.append(f'{path}: ref {json.dumps(a)[:100]} / rp {json.dumps(b)[:100]}')
    elif isinstance(a, dict):
        for k in a:
            if k not in b:
                out.append(f'{path}/{k}: missing in rp')
            else:
                walk(a[k], b[k], f'{path}/{k}', out)
        for k in b:
            if k not in a:
                out.append(f'{path}/{k}: extra in rp')
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(f'{path}: length {len(a)} / {len(b)}')
        for i, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f'{path}[{i}]', out)
    elif a != b:
        out.append(f'{path}: ref {json.dumps(a)[:100]} / rp {json.dumps(b)[:100]}')


(sa, ha, da), (sb, hb, db) = fetch(8080), fetch(3001)
print('status', sa, sb)
for name in ('Content-Type', 'Content-Disposition'):
    print(name, ha.get(name), '|', hb.get(name))
za, zb = zipfile.ZipFile(io.BytesIO(da)), zipfile.ZipFile(io.BytesIO(db))
na, nb = sorted(za.namelist()), sorted(zb.namelist())
print('entries', len(na), len(nb))
print('only in ref:', [n for n in na if n not in nb][:10])
print('only in rp:', [n for n in nb if n not in na][:10])
different = [name for name in na if name in nb and za.read(name) != zb.read(name)]
print('different:', different[:20])
if 'theme.json' in different:
    a, b = json.loads(za.read('theme.json')), json.loads(zb.read('theme.json'))
    out = []
    walk(a, b, '', out)
    print('theme.json same data:', a == b)
    for line in out:
        print('  ', line)
    print('same text:', za.read('theme.json') == zb.read('theme.json'), len(za.read('theme.json')), len(zb.read('theme.json')))
