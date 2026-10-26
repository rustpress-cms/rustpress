#!/usr/bin/env python3
"""Requests every top-level file of wp-admin as rpadmin on the reference (:8080) and RustPress (:3001) and lists those
whose status, redirect or <title> differ.

    python dev/admin-files-sweep.py [--user rpeditor] [substring]"""
import importlib.util
import os
import re
import sys

spec = importlib.util.spec_from_file_location('lc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'wp-local', 'wp-admin')
SKIP = {'admin-ajax.php', 'admin-post.php', 'async-upload.php', 'upgrade.php', 'install.php', 'setup-config.php', 'load-scripts.php', 'load-styles.php', 'admin-header.php', 'admin-footer.php', 'admin.php', 'menu.php', 'menu-header.php', 'admin-functions.php', 'user-new.php.bak'}


def main():
    args = sys.argv[1:]
    user, only = 'rpadmin', None
    i = 0
    while i < len(args):
        if args[i] == '--user':
            user = args[i + 1]
            i += 2
        else:
            only = args[i]
            i += 1
    clients = {}
    for port in (8080, 3001):
        c = lc.Client(port)
        c.request('/wp-login.php')
        c.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
        clients[port] = c
    bad = 0
    for name in sorted(os.listdir(ROOT)):
        if not name.endswith('.php') or name in SKIP or (only and only not in name):
            continue
        rows = []
        for port in (8080, 3001):
            status, headers, body = clients[port].request('/wp-admin/' + name)
            title = re.search(r'<title>(.*?)</title>', body, re.S)
            location = re.sub(r'localhost:\d+', 'HOST', headers.get('Location', ''))
            small = lc.norm(body, port) if len(body) < 10000 else ''
            small = re.sub(r'(_wpnonce=|nonce=)[0-9a-f]{10}', r'N', small)
            rows.append((status, location, re.sub(r'\s+', ' ', title.group(1)).strip() if title else '', small))
        same = rows[0] == rows[1]
        if not same:
            bad += 1
        print(('ok   ' if same else 'DIFF ') + name + ('' if same else f'  ref={rows[0][:3]} rp={rows[1][:3]}'), flush=True)
    print(bad, 'differ')


if __name__ == '__main__':
    main()
