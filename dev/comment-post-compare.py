#!/usr/bin/env python3
"""Sends the same comments to wp-comments-post.php of the reference (:8080) and of RustPress (:3001) and compares the
answer (status, redirect, cookies, error page) and the row that is saved. The comments are deleted afterwards.

    python dev/comment-post-compare.py [--diff N] [substring]"""
import difflib
import importlib.util
import os
import re
import sys

spec = importlib.util.spec_from_file_location('lc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

DIFF = 0
POST = 5  # Testbeitrag 1
COLUMNS = 'comment_post_ID,comment_author,comment_author_email,comment_author_url,comment_content,comment_karma,comment_approved,comment_agent,comment_type,comment_parent,user_id'


def clean():
    lc.sql("delete from wp_commentmeta where comment_id in (select comment_ID from wp_comments where comment_content like 'cpc-%' or comment_author like 'cpc%'); delete from wp_comments where comment_content like 'cpc-%' or comment_author like 'cpc%'")
    lc.sql("update wp_posts p set comment_count = (select count(*) from wp_comments c where c.comment_post_ID = p.ID and c.comment_approved = '1')")


def last_row():
    return lc.sql(f"select {COLUMNS} from wp_comments where comment_content like 'cpc-%' order by comment_ID desc limit 1")


def count():
    return lc.sql("select count(*) from wp_comments where comment_content like 'cpc-%'")


def normalize_location(location, port):
    location = re.sub(r'^http://localhost:\d+', '', location or '')
    return re.sub(r'moderation-hash=[0-9a-f]{32}', 'moderation-hash=H', re.sub(r'unapproved=\d+', 'unapproved=ID', re.sub(r'#comment-\d+', '#comment-ID', location)))


def scenario(name, fields, login=None, prepare=None, cookies=False):
    results = []
    for port in (8080, 3001):
        clean()
        client = lc.Client(port)
        if login:
            client.request('/wp-login.php')
            client.request('/wp-login.php', {'log': login, 'pwd': 'rustpress-dev', 'testcookie': '1'})
        if prepare:
            prepare(client, port)
        data = {k: (v(port) if callable(v) else v) for k, v in fields.items()}
        status, headers, body = client.request('/wp-comments-post.php', data)
        cookie_names = sorted(re.sub(r'[0-9a-f]{32}', 'H', c.name) + '=' + c.value for c in client.jar if c.name.startswith('comment_author'))
        row = last_row()
        results.append((status, normalize_location(headers.get('Location'), port), cookie_names, lc.norm(body, port), row, count()))
        clean()
    a, b = results
    problems = []
    for index, label in enumerate(['status', 'location', 'cookies', 'body', 'row', 'count']):
        if a[index] != b[index]:
            problems.append(f'{label}: {a[index]!r:.160} vs {b[index]!r:.160}' if label != 'body' else 'body')
    print(('ok   ' if not problems else 'DIFF ') + name + ('  [' + '; '.join(problems) + ']' if problems else ''), flush=True)
    if problems and DIFF and 'body' in ''.join(problems):
        for line in list(difflib.unified_diff(a[3].splitlines(), b[3].splitlines(), lineterm='', n=0))[:DIFF]:
            print('     ', line[:240])


def good(**extra):
    base = {'comment_post_ID': str(POST), 'comment_parent': '0', 'author': 'cpc Tester', 'email': 'cpc@example.com', 'url': 'http://example.org/', 'comment': 'cpc-hello world'}
    base.update(extra)
    return base


def main():
    global DIFF
    args = sys.argv[1:]
    only = None
    i = 0
    while i < len(args):
        if args[i] == '--diff':
            DIFF = int(args[i + 1])
            i += 2
        else:
            only = args[i]
            i += 1
    cases = [
        ('plain comment', good()),
        ('with cookie consent', good(**{'wp-comment-cookies-consent': 'yes'})),
        ('with redirect_to', good(redirect_to=lambda port: f'http://localhost:{port}/testbeitrag-1/')),
        ('no email, no name', good(author='', email='')),
        ('bad email', good(email='nope')),
        ('empty comment', good(comment='')),
        ('only spaces', good(comment='   ')),
        ('long name', good(author='cpc' + 'x' * 300)),
        ('long email', good(email='a' * 95 + '@example.com')),
        ('long url', good(url='http://example.com/' + 'x' * 200)),
        ('html in comment', good(comment='cpc-<b>bold</b> <script>x</script> <a href="http://example.org/" onclick=x>link</a> & <i>i</i>')),
        ('links', good(comment='cpc-see http://a.example/ and http://b.example/ and <a href="http://c.example/">c</a>')),
        ('url javascript', good(url='javascript:alert(1)')),
        ('url without scheme', good(url='example.org/x')),
        ('html in name', good(author='cpc <b>x</b> & y')),
        ('unicode', good(comment='cpc-ünïcödé 日本語 😀')),
        ('missing post id', {'comment': 'cpc-x'}),
        ('unknown post', good(comment_post_ID='99999')),
        ('closed comments (page 2)', good(comment_post_ID='2')),
        ('draft post', good(comment_post_ID='18')),
        ('draft page', good(comment_post_ID='3')),
        ('password protected', good(comment_post_ID='19')),
        ('reply to unknown comment', good(comment_parent='99999')),
        ('post id as text', good(comment_post_ID='abc')),
    ]
    for name, fields in cases:
        if only and only not in name:
            continue
        scenario(name, fields)
    if not only or 'logged' in only:
        scenario('signed in as administrator', good(), login='rpadmin')
        scenario('signed in as editor', good(), login='rpeditor')

    def duplicate(client, port):
        client.request('/wp-comments-post.php', good())
    if not only or 'duplicate' in only:
        scenario('duplicate comment', good(), prepare=duplicate)
    if not only or 'flood' in only:
        scenario('flood', good(comment='cpc-another one'), prepare=duplicate)
    # GET.
    if not only or 'GET' in only:
        for port in (8080, 3001):
            status, headers, body = lc.Client(port).request('/wp-comments-post.php')
            print('GET', port, status, headers.get('Allow'), headers.get('Content-Type'), repr(body[:40]))
    clean()


if __name__ == '__main__':
    main()
