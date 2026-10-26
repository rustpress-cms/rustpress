#!/usr/bin/env python3
"""Compares the writes of `wp/v2/comments` (create, update, delete) of the reference (:8080) and RustPress (:3001)
for every role. Each side runs the same steps on its own; the comments a side creates are compared row by row and
deleted before the other side runs, so duplicate and flood checks see the same database. The role users are made
and removed through the reference's PHP.

    python dev/rest-comments-write-compare.py [--diff N] [user]"""
import difflib
import importlib.util
import json
import os
import re
import socket
import subprocess
import sys

socket.setdefaulttimeout(60)
HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)
spec2 = importlib.util.spec_from_file_location('rcc', os.path.join(HERE, 'rest-comments-compare.py'))
rcc = importlib.util.module_from_spec(spec2)
spec2.loader.exec_module(rcc)

USERS_SEED = r'''<?php
$ids = array();
foreach ( array( 'author', 'contributor', 'subscriber' ) as $role ) {
	$ids[ $role ] = wp_insert_user( array( 'user_login' => 'rp' . $role, 'user_pass' => 'rustpress-dev', 'user_email' => 'rp' . $role . '@example.com', 'role' => $role, 'display_name' => 'RP ' . $role ) );
}
echo json_encode( $ids );
'''
USERS_CLEAN = r'''<?php
require_once ABSPATH . 'wp-admin/includes/user.php';
foreach ( json_decode( '%s', true ) as $id ) { wp_delete_user( $id, 1 ); }
echo "ok\n";
'''
MODERATORS = {'rpadmin', 'rpeditor'}


class Site(rcc.Site):
    def send(self, method, path, body=None):
        url = '/wp-json' + path + ('&' if '?' in path else '?') + '_wpnonce=' + self.nonce
        import urllib.request
        data = json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request(f'http://localhost:{self.port}{url}', data=data, method=method, headers={'Content-Type': 'application/json', 'User-Agent': 'rcw-agent/1.0'})
        try:
            r = self.client.op.open(req)
            status, text = r.status, r.read().decode('utf-8', 'replace')
        except urllib.error.HTTPError as e:
            status, text = e.code, e.read().decode('utf-8', 'replace')
        return status, rcc.normalize(self.port, text)


def steps(user):
    tag = f'rcw {user}'
    out = [
        ('create', 'POST', '/wp/v2/comments', {'post': 1, 'content': f'{tag} first <u>u</u> <script>x</script> http://example.com/x'}),
        ('empty', 'POST', '/wp/v2/comments', {'post': 1, 'content': ''}),
        ('draft', 'POST', '/wp/v2/comments', {'post': 18, 'content': f'{tag} draft'}),
        ('closed', 'POST', '/wp/v2/comments', {'post': 2, 'content': f'{tag} closed'}),
        ('nopost', 'POST', '/wp/v2/comments', {'content': f'{tag} nopost'}),
        ('status', 'POST', '/wp/v2/comments', {'post': 1, 'content': f'{tag} status', 'status': 'hold'}),
        ('author', 'POST', '/wp/v2/comments', {'post': 1, 'content': f'{tag} author', 'author': 1}),
        ('pingback', 'POST', '/wp/v2/comments', {'post': 1, 'content': f'{tag} ping', 'type': 'pingback'}),
        ('bademail', 'POST', '/wp/v2/comments', {'post': 1, 'content': f'{tag} email', 'author_email': 'nope'}),
        ('note', 'POST', '/wp/v2/comments', {'post': 1, 'content': f'{tag} note', 'type': 'note'}),
        ('note-draft', 'POST', '/wp/v2/comments', {'post': 18, 'content': f'{tag} note draft', 'type': 'note', 'meta': {'_wp_note_status': 'resolved'}}),
        ('update', 'PUT', '/wp/v2/comments/{create}', {'content': f'{tag} changed <b>b</b>'}),
        ('update-status', 'PUT', '/wp/v2/comments/{create}', {'status': 'hold'}),
        ('update-type', 'PUT', '/wp/v2/comments/{create}', {'type': 'note'}),
        ('trash', 'DELETE', '/wp/v2/comments/{create}', None),
        ('trash-again', 'DELETE', '/wp/v2/comments/{create}', None),
        ('force', 'DELETE', '/wp/v2/comments/{create}?force=true', None),
        ('note-force', 'DELETE', '/wp/v2/comments/{note}?force=true', None),
    ]
    if user not in MODERATORS:
        out += [('foreign-update', 'PUT', '/wp/v2/comments/1', {'content': 'nope'}), ('foreign-delete', 'DELETE', '/wp/v2/comments/1', None)]
    return out


def run(port, user):
    site = Site(port, user)
    created, results = {}, []
    for name, method, path, body in steps(user):
        if '{' in path:
            key = path.split('{')[1].split('}')[0]
            if key not in created:
                results.append((name, 'skipped (nothing created)'))
                continue
            path = path.replace('{' + key + '}', str(created[key]))
        status, text = site.send(method, path, body)
        try:
            data = json.loads(text)
        except Exception:
            data = None
        if method == 'POST' and status == 201 and isinstance(data, dict):
            created[name] = data['id']
        results.append((name, status, text))
    return created, results


def normalize_ids(text, created):
    for name, cid in created.items():
        text = re.sub(rf'\b{cid}\b', f'<{name}>', text)
    text = re.sub(r'"(date|date_gmt)": ?"[0-9T:-]+"', r'"\1":"DATE"', text)
    text = re.sub(r'"author_ip": ?"[^"]*"', '"author_ip":"IP"', text)
    return text


def rows(created):
    if not created:
        return ''
    ids = ','.join(str(i) for i in created.values())
    out = lc.sql(f"SELECT comment_ID, comment_post_ID, comment_author, comment_author_email, comment_author_url, comment_content, comment_approved, comment_agent, comment_type, comment_parent, user_id FROM wp_comments WHERE comment_ID IN ({ids}) ORDER BY comment_ID")
    meta = lc.sql(f"SELECT comment_id, meta_key, meta_value FROM wp_commentmeta WHERE comment_id IN ({ids}) ORDER BY comment_id, meta_key")
    return normalize_ids(out + '\n' + meta, created)


def clean(created):
    if created:
        ids = ','.join(str(i) for i in created.values())
        lc.sql(f"DELETE FROM wp_commentmeta WHERE comment_id IN ({ids}); DELETE FROM wp_comments WHERE comment_ID IN ({ids})")
    lc.sql("UPDATE wp_posts p SET comment_count = (SELECT COUNT(*) FROM wp_comments c WHERE c.comment_post_ID = p.ID AND c.comment_approved = '1') WHERE p.ID IN (1, 18)")


def main():
    args = sys.argv[1:]
    lines = 10
    if '--diff' in args:
        i = args.index('--diff')
        lines = int(args[i + 1])
        del args[i:i + 2]
    users = args or ['rpadmin', 'rpeditor', 'rpauthor', 'rpcontributor', 'rpsubscriber']
    role_ids = json.loads(rcc.php(USERS_SEED))
    bad = total = 0
    try:
        for user in users:
            print('user', user, flush=True)
            sides = {}
            for port in (8080, 3001):
                created, results = run(port, user)
                sides[port] = (created, [tuple(normalize_ids(str(x), created) for x in r) for r in results], rows(created))
                clean(created)
            (_, ra, da), (_, rb, db) = sides[8080], sides[3001]
            for a, b in zip(ra, rb):
                total += 1
                if a == b:
                    continue
                bad += 1
                print(f'DIFFERENT {user} {a[0]}: {a[1]} vs {b[1]}', flush=True)
                pa, pb = rcc.pretty(a[-1]), rcc.pretty(b[-1])
                if len(pa) == len(pb) == 1:
                    # One line of JSON with ids replaced: show where it starts to differ.
                    at = next((i for i, (x, y) in enumerate(zip(pa[0], pb[0])) if x != y), 0)
                    pa, pb = [pa[0][max(0, at - 80):at + 120]], [pb[0][max(0, at - 80):at + 120]]
                for d in [d for d in difflib.unified_diff(pa, pb, lineterm='', n=0) if not d.startswith(('---', '+++'))][:lines]:
                    print('   ', d[:200])
            total += 1
            if da != db:
                bad += 1
                print(f'DIFFERENT {user} rows')
                for d in [d for d in difflib.unified_diff(da.splitlines(), db.splitlines(), lineterm='', n=0) if not d.startswith(('---', '+++'))][:lines * 2]:
                    print('   ', d[:200])
        print(f'{total - bad}/{total} identical')
    finally:
        print('users removed:', rcc.php(USERS_CLEAN % json.dumps(role_ids)))


if __name__ == '__main__':
    main()
