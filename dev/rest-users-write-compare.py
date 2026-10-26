#!/usr/bin/env python3
"""Compares the writes of `wp/v2/users/<id>` and `wp/v2/users/me` (update, delete) of the reference (:8080) and
RustPress (:3001). Each side gets fresh users made through the reference's PHP (an administrator, an editor, an
author, a contributor, a subscriber and a target), runs the same steps as each of them and removes the users
again; the IDs are replaced by names before the answers are compared.

    python dev/rest-users-write-compare.py [--diff N]"""
import difflib
import importlib.util
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('rcc', os.path.join(HERE, 'rest-comments-compare.py'))
rcc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rcc)
spec2 = importlib.util.spec_from_file_location('rcw', os.path.join(HERE, 'rest-comments-write-compare.py'))
rcw = importlib.util.module_from_spec(spec2)
spec2.loader.exec_module(rcw)

ROLES = ['administrator', 'editor', 'author', 'contributor', 'subscriber']

SEED = r'''<?php
$ids = array();
foreach ( array( 'administrator' => 'rpuwadmin', 'editor' => 'rpuweditor', 'author' => 'rpuwauthor', 'contributor' => 'rpuwcontributor', 'subscriber' => 'rpuwsubscriber', 'subscriber ' => 'rpuwtarget' ) as $role => $login ) {
	$ids[ $login ] = wp_insert_user( array( 'user_login' => $login, 'user_pass' => 'rustpress-dev', 'user_email' => $login . '@example.com', 'role' => trim( $role ), 'display_name' => 'UW ' . $login, 'user_registered' => '2026-10-01 10:00:00' ) );
}
$post = wp_insert_post( array( 'post_title' => 'uw post', 'post_status' => 'publish', 'post_author' => $ids['rpuwtarget'] ) );
$ids['post'] = $post;
echo json_encode( $ids );
'''
CLEAN = r'''<?php
require_once ABSPATH . 'wp-admin/includes/user.php';
$ids = json_decode( '%s', true );
foreach ( $ids as $key => $id ) { if ( 'post' !== $key && get_userdata( $id ) ) { wp_delete_user( $id, 1 ); } }
wp_delete_post( $ids['post'], true );
global $wpdb;
echo $wpdb->get_var( "SELECT COUNT(*) FROM {$wpdb->users} WHERE user_login LIKE 'rpuw%%'" ), "\n";
'''


def steps(login):
    out = [
        ('me-name', 'PUT', '/wp/v2/users/me', {'first_name': 'First ' + login, 'nickname': 'nick-' + login}),
        ('me-post', 'POST', '/wp/v2/users/me', {'description': 'about <b>me</b>'}),
        ('me-email-taken', 'PUT', '/wp/v2/users/me', {'email': 'rpuwtarget@example.com'}),
        ('me-email-bad', 'PUT', '/wp/v2/users/me', {'email': 'not-an-email'}),
        ('me-username', 'PUT', '/wp/v2/users/me', {'username': 'someoneelse'}),
        ('me-slug-taken', 'PUT', '/wp/v2/users/me', {'slug': 'rpuwtarget'}),
        ('me-slug', 'PUT', '/wp/v2/users/me', {'slug': 'Neuer Slug ' + login}),
        ('me-password-empty', 'PUT', '/wp/v2/users/me', {'password': ''}),
        ('me-roles', 'PUT', '/wp/v2/users/me', {'roles': ['subscriber']}),
        ('target-name', 'PUT', '/wp/v2/users/{rpuwtarget}', {'first_name': 'Changed'}),
        ('target-roles-only', 'PUT', '/wp/v2/users/{rpuwtarget}', {'roles': ['author']}),
        ('target-role-unknown', 'PUT', '/wp/v2/users/{rpuwtarget}', {'roles': ['nope']}),
        ('missing', 'PUT', '/wp/v2/users/999999', {'first_name': 'x'}),
        ('delete-no-reassign', 'DELETE', '/wp/v2/users/{rpuwtarget}', None),
        ('delete-no-force', 'DELETE', '/wp/v2/users/{rpuwtarget}?reassign=1', None),
        ('delete-bad-reassign', 'DELETE', '/wp/v2/users/{rpuwtarget}?reassign={rpuwtarget}&force=true', None),
        ('delete-reassign-text', 'DELETE', '/wp/v2/users/{rpuwtarget}?reassign=abc&force=true', None),
        ('delete', 'DELETE', '/wp/v2/users/{rpuwtarget}?reassign=false&force=true', None),
    ]
    return out


def run(port, ids, login):
    site = rcw.Site(port, login)
    results = []
    for name, method, path, body in steps(login):
        for key, value in ids.items():
            path = path.replace('{' + key + '}', str(value))
        status, text = site.send(method, path, body)
        results.append((name, status, text))
    return results


def normalize(text, ids):
    for key, value in sorted(ids.items(), key=lambda kv: -len(str(kv[1]))):
        # Only where an ID stands: in a path, as a JSON value, or in a query.
        text = re.sub(rf'(?<=/){value}(?=[/"?&\\]|$)', f'<{key}>', text)
        text = re.sub(rf'(?<=":){value}(?=[,}}\]])', f'<{key}>', text)
        text = re.sub(rf'(?<==){value}(?=[&"]|$)', f'<{key}>', text)
    text = re.sub(r'"registered_date": ?"[^"]*"', '"registered_date":"DATE"', text)
    return text


def main():
    args = sys.argv[1:]
    lines = 10
    if '--diff' in args:
        i = args.index('--diff')
        lines = int(args[i + 1])
    logins = ['rpuwadmin', 'rpuweditor', 'rpuwauthor', 'rpuwcontributor', 'rpuwsubscriber']
    sides = {}
    for port in (8080, 3001):
        sides[port] = {}
        for login in logins:
            ids = json.loads(rcc.php(SEED))
            try:
                sides[port][login] = [tuple(normalize(str(x), ids) for x in r) for r in run(port, ids, login)]
            finally:
                left = rcc.php(CLEAN % json.dumps(ids))
                if left.strip() != '0':
                    print('users left:', left)
    bad = total = 0
    for login in logins:
        print('user', login)
        for a, b in zip(sides[8080][login], sides[3001][login]):
            total += 1
            if a == b:
                continue
            bad += 1
            print(f'DIFFERENT {login} {a[0]}: {a[1]} vs {b[1]}')
            pa, pb = rcc.pretty(a[-1]), rcc.pretty(b[-1])
            if len(pa) == len(pb) == 1:
                at = next((i for i, (x, y) in enumerate(zip(pa[0], pb[0])) if x != y), 0)
                pa, pb = [pa[0][max(0, at - 80):at + 120]], [pb[0][max(0, at - 80):at + 120]]
            for d in [d for d in difflib.unified_diff(pa, pb, lineterm='', n=0) if not d.startswith(('---', '+++'))][:lines]:
                print('   ', d[:200])
    print(f'{total - bad}/{total} identical')


if __name__ == '__main__':
    main()
