#!/usr/bin/env python3
"""Raw (byte-level) comparison of every top-level file of wp-admin (and a few pages with arguments) between the
reference (:8080) and RustPress (:3001): prints how many lines differ per page. Nonces, ports, lock and request
times are neutralized like in raw-page-diff.py; the upload size limit is an environment difference and ignored.

    python dev/raw-sweep.py [--user rpeditor] [substring]"""
import difflib
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

ROOT = os.path.join(HERE, '..', '..', 'wp-local', 'wp-admin')
SKIP = {'admin-ajax.php', 'admin-post.php', 'async-upload.php', 'upgrade.php', 'install.php', 'setup-config.php', 'load-scripts.php', 'load-styles.php', 'admin-header.php', 'admin-footer.php', 'admin.php', 'menu.php', 'menu-header.php', 'admin-functions.php', 'user-new.php.bak',
        # They change state or depend on the network.
        'post-new.php', 'media-upload.php', 'customize.php', 'update.php', 'update-core.php', 'plugin-install.php', 'theme-install.php'}
EXTRA = ['edit.php?post_type=page', 'edit-tags.php?taxonomy=category', 'term.php?taxonomy=category&tag_ID=1', 'post.php?post=1&action=edit&rp-classic=1',
         'post.php?post=2&action=edit&rp-classic=1', 'comment.php?action=editcomment&c=1', 'user-edit.php?user_id=1', 'options-general.php']


def normalize(body, port):
    body = body.replace(f'localhost:{port}', 'localhost:PORT')
    body = re.sub(r'(value|_wpnonce=|nonce["\']?\s*[:=]\s*["\']?)([0-9a-f]{10})\b', r'\1NONCE', body)
    body = re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', body)
    body = re.sub(r'\b1[0-9]{9}:([0-9]+)\b', r'TIME:\1', body)
    body = re.sub(r'"max_file_size":"[0-9]+b"', '"max_file_size":"SIZE"', body)
    return body.split('\n')


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
    pages = [n for n in sorted(os.listdir(ROOT)) if n.endswith('.php') and n not in SKIP] + EXTRA
    total = 0
    for page in pages:
        if only and only not in page:
            continue
        results = []
        for port in (8080, 3001):
            status, headers, body = clients[port].request('/wp-admin/' + page)
            results.append((status, normalize(body, port)))
        (sa, a), (sb, b) = results
        changed = sum(1 for d in difflib.unified_diff(a, b, lineterm='', n=0) if d[:1] in '+-' and not d.startswith(('+++', '---')))
        if sa != sb or changed:
            total += 1
            print(f'{page:55} status {sa} {sb}  {changed} lines')
    print(f'{total} pages differ')


if __name__ == '__main__':
    main()
