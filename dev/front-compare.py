#!/usr/bin/env python3
"""Compares pages of the site (not wp-admin) as seen by a signed-in user on the reference (:8080) and RustPress (:3001).

    python dev/front-compare.py [--user rpeditor] [--diff N] / /?p=18&preview=true /privat/

Prints one line per address (identical or the number of differing lines); `--diff N` prints the first N differences.
The user is rpadmin unless `--user` says otherwise; `--anon` compares without signing in."""
import difflib
import http.cookiejar
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

_openers = {}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


def opener(port, user):
    key = (port, user)
    if key not in _openers:
        jar = http.cookiejar.CookieJar()
        handlers = [urllib.request.HTTPCookieProcessor(jar), NoRedirect()]
        o = urllib.request.build_opener(*handlers)
        if user:
            o.open(f'http://localhost:{port}/wp-login.php').read()
            try:
                o.open(f'http://localhost:{port}/wp-login.php', urllib.parse.urlencode({'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()).read()
            except urllib.error.HTTPError:
                pass
        _openers[key] = o
    return _openers[key]


def fetch(port, user, path):
    try:
        r = opener(port, user).open(f'http://localhost:{port}{path}')
        return r.status, {k.title(): v for k, v in r.headers.items()}, r.read().decode('utf-8', 'replace')
    except urllib.error.HTTPError as e:
        return e.code, {k.title(): v for k, v in e.headers.items()}, e.read().decode('utf-8', 'replace')


def normalize(text, port):
    for p in ('8080', '3001'):
        text = text.replace(f'localhost:{p}', 'HOST').replace(f'localhost%3A{p}', 'HOST').replace(f'127.0.0.1:{p}', 'HOST')
    text = re.sub(r'(_wpnonce|nonce)(["\':= ]+)[0-9a-f]{10}', r'\1\2N', text)
    text = re.sub(r'(_wpnonce=)[0-9a-f]{10}', r'\1N', text)
    text = re.sub(r'"time":"\d+"', '"time":"T"', text)
    text = re.sub(r'(_wp_unfiltered_html_comment_disabled" value=")[0-9a-f]{10}', r'N', text)
    return text


def main():
    args = sys.argv[1:]
    user = 'rpadmin'
    diff_lines = 0
    pages = []
    i = 0
    while i < len(args):
        if args[i] == '--user':
            user = args[i + 1]
            i += 2
        elif args[i] == '--diff':
            diff_lines = int(args[i + 1])
            i += 2
        elif args[i] == '--anon':
            user = ''
            i += 1
        else:
            pages.append(args[i])
            i += 1
    for page in pages:
        a = fetch(8080, user, page)
        b = fetch(3001, user, page)
        la = normalize(a[2], 8080).splitlines()
        lb = normalize(b[2], 3001).splitlines()
        diff = list(difflib.unified_diff(la, lb, lineterm='', n=0))
        changed = [d for d in diff if d[:1] in '+-' and not d.startswith(('---', '+++'))]
        note = '' if a[0] == b[0] else f' [status {a[0]} vs {b[0]}]'
        loc = tuple(re.sub(r'^http://localhost:(8080|3001)', '', x) if x else x for x in (a[1].get('Location'), b[1].get('Location')))
        if loc[0] != loc[1] and (loc[0] or loc[1]):
            note += f' [location {loc[0]} vs {loc[1]}]'
        print(('identical ' if not changed and not note else 'DIFFERENT ') + page + (f' ({len(changed)} lines)' if changed else '') + note)
        if diff_lines and (changed or note):
            for line in diff[:diff_lines]:
                print('   ', line[:300])


if __name__ == '__main__':
    main()
