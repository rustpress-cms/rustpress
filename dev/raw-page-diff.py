#!/usr/bin/env python3
"""Raw diff (whitespace counts) of an administration page of the reference (:8080) and RustPress (:3001), between two
markers, as rpadmin (or `--user u`). Nonces, ports, lock times and the request time are neutralized.

    python dev/raw-page-diff.py "post.php?post=1&action=edit&rp-classic=1" [--from '<div class="wrap">'] [--to '</form>'] [--context 2] [--user rpeditor]"""
import difflib
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)


def option(args, name, default):
    if name in args:
        i = args.index(name)
        value = args[i + 1]
        del args[i:i + 2]
        return value
    return default


def main():
    args = sys.argv[1:]
    start = option(args, '--from', '<div class="wrap">')
    end = option(args, '--to', '</html>')
    context = int(option(args, '--context', '2'))
    user = option(args, '--user', 'rpadmin')
    page = args[0]
    texts = []
    for port in (8080, 3001):
        c = lc.Client(port)
        c.request('/wp-login.php')
        c.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
        status, headers, body = c.request('/wp-admin/' + page)
        a = body.find(start)
        b = body.find(end, a + 1 if a >= 0 else 0)
        part = body[a if a >= 0 else 0:(b + len(end)) if b >= 0 else len(body)]
        part = part.replace(f'localhost:{port}', 'localhost:PORT')
        part = re.sub(r'(value|_wpnonce=|nonce["\']?\s*[:=]\s*["\']?)([0-9a-f]{10})\b', r'\1NONCE', part)
        part = re.sub(r'"[0-9a-f]{10}"', '"NONCE"', part)
        part = re.sub(r'\b1[0-9]{9}:([0-9]+)\b', r'TIME:\1', part)
        texts.append((status, part.split('\n')))
    print('status', texts[0][0], texts[1][0], 'lines', len(texts[0][1]), len(texts[1][1]))
    diff = list(difflib.unified_diff(texts[0][1], texts[1][1], lineterm='', n=context))
    for line in diff[2:]:
        print(repr(line)[1:-1][:300] if line.startswith(('-', '+')) else line[:300])
    print('identical' if not diff else f'{sum(1 for d in diff[2:] if d.startswith(("-", "+")))} changed lines')


if __name__ == '__main__':
    main()
