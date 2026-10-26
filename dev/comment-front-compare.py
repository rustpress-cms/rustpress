#!/usr/bin/env python3
"""After a comment is sent: the page the visitor lands on (own comment awaiting moderation, form filled from the
cookies) on the reference (:8080) and RustPress (:3001).

    python dev/comment-front-compare.py [--diff N] [substring]"""
import difflib
import importlib.util
import os
import re
import sys

spec = importlib.util.spec_from_file_location('cpc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'comment-post-compare.py'))
cpc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cpc)
lc = cpc.lc


def page(client, port, location):
    path = re.sub(r'^http://localhost:\d+', '', location)
    return client.request(path)


def clean_page(text, port):
    text = lc.norm(text, port)
    text = re.sub(r'comment-\d+', 'comment-ID', text)
    text = re.sub(r'(comment_post_ID|unapproved|replytocom|c)=\d+', r'\1=ID', text)
    text = re.sub(r'moderation-hash=[0-9a-f]{32}', 'moderation-hash=H', text)
    text = re.sub(r'datetime="[^"]*"', 'datetime="T"', text)
    text = re.sub(r'data-commentid="\d+"', 'data-commentid="ID"', text)
    text = re.sub(r'"id":"comment-[^"]*"', '"id":"c"', text)
    return text


def scenario(name, fields, diff, login=None, second=None):
    out = []
    for port in (8080, 3001):
        cpc.clean()
        client = lc.Client(port)
        if login:
            client.request('/wp-login.php')
            client.request('/wp-login.php', {'log': login, 'pwd': 'rustpress-dev', 'testcookie': '1'})
        status, headers, body = client.request('/wp-comments-post.php', {k: (v(port) if callable(v) else v) for k, v in fields.items()})
        location = headers.get('Location', '')
        status2, _, html = page(client, port, location) if location else (0, {}, '')
        # A second visit without the hash: only the cookies tell who the visitor is.
        again = ''
        if second:
            path = re.sub(r'^http://localhost:\d+', '', location).split('?')[0].split('#')[0]
            _, _, again = client.request(path)
        out.append((status2, clean_page(html, port), clean_page(again, port)))
        cpc.clean()
    a, b = out
    problems = [label for label, x, y in (('status', a[0], b[0]), ('landing', a[1], b[1]), ('revisit', a[2], b[2])) if x != y]
    print(('ok   ' if not problems else 'DIFF ') + name + ('  [' + ', '.join(problems) + ']' if problems else ''), flush=True)
    if problems and diff:
        for label, i in (('landing', 1), ('revisit', 2)):
            if label in problems:
                for line in list(difflib.unified_diff(a[i].splitlines(), b[i].splitlines(), lineterm='', n=0))[:diff]:
                    print('     ', line[:260])


def main():
    args = sys.argv[1:]
    diff, only = 0, None
    i = 0
    while i < len(args):
        if args[i] == '--diff':
            diff = int(args[i + 1])
            i += 2
        else:
            only = args[i]
            i += 1
    cases = [
        ('visitor, no consent', cpc.good(), None, True),
        ('visitor, consent', cpc.good(**{'wp-comment-cookies-consent': 'yes'}), None, True),
        ('visitor with html', cpc.good(comment='cpc-<b>bold</b> line\n\nsecond <a href="http://example.org/">link</a>'), None, False),
        ('reply', cpc.good(comment_parent='1'), None, False),
        ('administrator', cpc.good(), 'rpadmin', True),
        ('editor', cpc.good(), 'rpeditor', True),
    ]
    for name, fields, login, second in cases:
        if only and only not in name:
            continue
        scenario(name, fields, diff, login, second)


if __name__ == '__main__':
    main()
