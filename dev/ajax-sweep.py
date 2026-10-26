#!/usr/bin/env python3
"""Calls every core action of admin-ajax.php without parameters on the reference (:8080) and RustPress (:3001) as
rpadmin and lists the actions whose status or answer differ (an action RustPress does not know answers 400 "0").

    python dev/ajax-sweep.py [--user rpeditor] [substring]"""
import importlib.util
import os
import re
import sys

spec = importlib.util.spec_from_file_location('lc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

SRC = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'wp-local', 'wp-admin', 'admin-ajax.php')


def actions():
    text = open(SRC, encoding='utf-8').read()
    out = []
    for name in ('core_actions_get', 'core_actions_post', 'core_actions_post_deprecated'):
        m = re.search(r'\$' + name + r'\s*=\s*array\((.*?)\);', text, re.S)
        for a in re.findall(r"'([^']+)'", m.group(1)):
            if a not in out:
                out.append((a))
    return out


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
    for action in actions():
        if only and only not in action:
            continue
        rows = []
        for port in (8080, 3001):
            status, headers, body = clients[port].request('/wp-admin/admin-ajax.php', {'action': action})
            rows.append((status, lc.norm(body, port)[:300]))
        same = rows[0] == rows[1]
        bad += not same
        print(('ok   ' if same else 'DIFF ') + action + ('' if same else f'  ref={rows[0]} rp={rows[1]}'), flush=True)
    print(bad, 'differ')


if __name__ == '__main__':
    main()
