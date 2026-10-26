#!/usr/bin/env python3
"""Compares administration pages of the reference (:8080) and RustPress (:3001) as rpadmin (or `--user u`), with
the normalization of dev/admin_norm.py; works for addresses with `&` (dev/admin-compare.sh does not).

    python dev/page-compare.py [--user rpeditor] [--diff N] "edit-comments.php?comment_status=all&paged=2" ..."""
import difflib
import importlib.util
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)


def normalize(html):
    out = subprocess.run([sys.executable, os.path.join(HERE, 'admin_norm.py')], input=html.encode('utf-8'), capture_output=True)
    return out.stdout.decode('utf-8', 'replace')


def main():
    args = sys.argv[1:]
    user, lines, pages = 'rpadmin', 12, []
    while args:
        a = args.pop(0)
        if a == '--user':
            user = args.pop(0)
        elif a == '--diff':
            lines = int(args.pop(0))
        else:
            pages.append(a)
    clients = {}
    for port in (8080, 3001):
        c = lc.Client(port)
        c.request('/wp-login.php')
        c.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
        clients[port] = c
    for page in pages:
        got = {}
        for port, c in clients.items():
            status, headers, body = c.request('/wp-admin/' + page)
            got[port] = (status, headers.get('Location'), normalize(body).splitlines())
        (sa, la, ba), (sb, lb, bb) = got[8080], got[3001]
        diff = [d for d in difflib.unified_diff(ba, bb, lineterm='', n=0) if not d.startswith(('---', '+++'))]
        note = '' if sa == sb else f' [status {sa} vs {sb}]'
        print(('identical ' if not diff and not note else 'DIFFERENT ') + page + note + (f' ({len(diff)} diff lines)' if diff else ''))
        for d in diff[:lines]:
            print('   ', d[:240])


if __name__ == '__main__':
    main()
