#!/usr/bin/env python3
"""Shows the help tabs and the sidebar of administration pages where the reference (:8080) and RustPress (:3001)
differ, panel by panel.

    python dev/help-compare.py [--user rpeditor] options-reading.php profile.php ..."""
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)


def panels(html):
    meta = html[html.find('<div id="screen-meta"'):html.find('<div id="screen-options-wrap"') if '<div id="screen-options-wrap"' in html else html.find('<div id="screen-meta-links"')]
    out = {}
    titles = re.findall(r'aria-controls="tab-panel-([\w-]+)">\s*(.*?)\s*</a>', meta, re.S)
    out['TABS'] = ' | '.join(f'{i}={t}' for i, t in titles)
    for m in re.finditer(r'<div id="tab-panel-([\w-]+)" class="help-tab-content[^"]*">\s*(.*?)\s*</div>\n', meta, re.S):
        out[m.group(1)] = m.group(2)
    side = re.search(r'contextual-help-sidebar">\s*(.*?)\s*</div>', meta, re.S)
    out['SIDEBAR'] = side.group(1) if side else ''
    return out


def main():
    args = sys.argv[1:]
    user = 'rpadmin'
    if args[:1] == ['--user']:
        user, args = args[1], args[2:]
    clients = {}
    for port in (8080, 3001):
        c = lc.Client(port)
        c.request('/wp-login.php')
        c.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
        clients[port] = c
    for page in args:
        a = panels(clients[8080].request('/wp-admin/' + page)[2])
        b = panels(clients[3001].request('/wp-admin/' + page)[2])
        keys = list(dict.fromkeys(list(a) + list(b)))
        bad = [k for k in keys if a.get(k) != b.get(k)]
        print(('identical ' if not bad else 'DIFFERENT ') + page)
        for k in bad:
            print(f'  [{k}]')
            print(f'    ref: {a.get(k)}')
            print(f'    rp:  {b.get(k)}')


if __name__ == '__main__':
    main()
