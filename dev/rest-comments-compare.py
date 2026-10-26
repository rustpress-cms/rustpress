#!/usr/bin/env python3
"""Compares `wp/v2/comments` of the reference (:8080) and RustPress (:3001) for visitors and every role: status,
X-WP-Total/TotalPages and the whole body. It first adds comments of every status and type (waiting, spam, trash,
a note, comments of a private and a draft post, comments of the users themselves) and the role users through the
reference's own PHP, and removes them afterwards.

    python dev/rest-comments-compare.py [--keep] [--diff N] [substring]"""
import difflib
import importlib.util
import json
import os
import re
import socket
import subprocess
import sys

socket.setdefaulttimeout(60)
SEED_FILE = os.path.join(os.environ.get('TEMP', '/tmp'), 'rp-rest-comments-ids.json')

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

ROLES = ['author', 'contributor', 'subscriber']

SEED = r'''<?php
$ids = array( 'users' => array(), 'comments' => array() );
foreach ( array( 'author', 'contributor', 'subscriber' ) as $role ) {
	$id = wp_insert_user( array( 'user_login' => 'rp' . $role, 'user_pass' => 'rustpress-dev', 'user_email' => 'rp' . $role . '@example.com', 'role' => $role, 'display_name' => 'RP ' . $role ) );
	$ids['users'][ $role ] = $id;
}
$editor = get_user_by( 'login', 'rpeditor' );
$add = function ( $post, $approved, $extra = array() ) use ( &$ids ) {
	$id = wp_insert_comment( array_merge( array( 'comment_post_ID' => $post, 'comment_author' => 'cmp ' . $approved, 'comment_author_email' => 'cmp@example.com', 'comment_author_url' => 'https://example.org/cmp', 'comment_author_IP' => '192.0.2.7', 'comment_agent' => 'cmp-agent/1.0', 'comment_content' => 'cmp comment ' . $approved . ' <b>bold</b>', 'comment_approved' => $approved, 'comment_date' => '2026-10-20 10:00:00', 'comment_date_gmt' => '2026-10-20 10:00:00' ), $extra ) );
	$ids['comments'][] = $id;
	return $id;
};
$add( 1, '0' );
$add( 1, 'spam' );
$add( 1, 'trash' );
$add( 269, '1' );
$add( 18, '1' );
$add( 268, '0' );
$own = $add( 1, '1', array( 'user_id' => $editor->ID, 'comment_author' => $editor->display_name ) );
$add( 1, '0', array( 'user_id' => $editor->ID, 'comment_author' => $editor->display_name, 'comment_parent' => $own ) );
$add( 1, '0', array( 'user_id' => $ids['users']['contributor'], 'comment_author' => 'RP contributor' ) );
$note = $add( 1, '1', array( 'comment_type' => 'note', 'user_id' => 1, 'comment_author' => 'admin' ) );
update_comment_meta( $note, '_wp_note_status', 'resolved' );
$add( 1, '0', array( 'comment_type' => 'note', 'user_id' => $editor->ID, 'comment_parent' => $note ) );
$add( 18, '1', array( 'comment_type' => 'note', 'user_id' => $ids['users']['author'] ) );
echo json_encode( $ids );
'''

CLEAN = r'''<?php
$ids = json_decode( '%s', true );
foreach ( $ids['comments'] as $id ) { wp_delete_comment( $id, true ); }
require_once ABSPATH . 'wp-admin/includes/user.php';
foreach ( $ids['users'] as $id ) { wp_delete_user( $id, 1 ); }
global $wpdb;
echo $wpdb->get_var( "SELECT COUNT(*) FROM {$wpdb->comments} WHERE comment_author LIKE 'cmp %%' OR comment_content LIKE 'cmp %%'" ), "\n";
'''


def php(code):
    path = os.path.join(os.environ.get('TEMP', '/tmp'), 'rp-rest-comments.php')
    with open(path, 'w', encoding='utf-8') as f:
        f.write(code)
    wsl_path = subprocess.run(['wsl', '-d', 'Debian', 'wslpath', '-a', path.replace('\\', '/')], capture_output=True, text=True).stdout.strip()
    repo = subprocess.run(['wsl', '-d', 'Debian', 'wslpath', '-a', os.path.dirname(HERE).replace('\\', '/')], capture_output=True, text=True).stdout.strip()
    out = subprocess.run(['wsl', '-d', 'Debian', '--', 'bash', '-c', f"cd '{repo}' && dev/php.sh '{wsl_path}'"], capture_output=True, text=True, timeout=120)
    return out.stdout.strip()


def requests(ids):
    c = ids['comments']
    hold, spam, trash, private, draft, pending, own, own_reply, contrib, note, note_reply, draft_note = c
    paths = [
        '/wp/v2/comments', '/wp/v2/comments?context=edit', '/wp/v2/comments?context=embed', '/wp/v2/comments?per_page=5&page=2',
        '/wp/v2/comments?status=hold', '/wp/v2/comments?status=hold&context=edit', '/wp/v2/comments?status=all&context=edit', '/wp/v2/comments?status=spam&context=edit',
        '/wp/v2/comments?status=trash&context=edit', '/wp/v2/comments?status=any&context=edit', '/wp/v2/comments?status=all', '/wp/v2/comments?type=all',
        '/wp/v2/comments?type=pings', '/wp/v2/comments?type=note', '/wp/v2/comments?type=note&post=1&status=all&context=edit', '/wp/v2/comments?type=note&post=1&status=all',
        '/wp/v2/comments?type=note&post=18&status=all&context=edit', '/wp/v2/comments?type=note&post=2', '/wp/v2/comments?post=1', '/wp/v2/comments?post=1&context=edit',
        '/wp/v2/comments?post=269', '/wp/v2/comments?post=18', '/wp/v2/comments?post=0', '/wp/v2/comments?post=1,5', '/wp/v2/comments?author=0',
        '/wp/v2/comments?author_email=cmp@example.com', '/wp/v2/comments?author_exclude=1', '/wp/v2/comments?search=cmp&status=all&context=edit',
        f'/wp/v2/comments?parent={own}&status=all', f'/wp/v2/comments?parent={note}&type=note&status=all', '/wp/v2/comments?orderby=id&order=asc&status=all',
        '/wp/v2/comments?_embed=1&post=1', '/wp/v2/comments?_fields=id,status,author_email',
    ]
    for cid in [1, hold, spam, trash, private, draft, pending, own, own_reply, contrib, note, note_reply, draft_note, 999999, 0]:
        paths.append(f'/wp/v2/comments/{cid}')
        paths.append(f'/wp/v2/comments/{cid}?context=edit')
    return paths


def normalize(port, text):
    # Both ports: addresses stored in the shared database name the reference.
    for p in (8080, 3001):
        text = text.replace(f'localhost:{p}', 'HOST').replace(f'localhost%3A{p}', 'HOST')
    text = re.sub(r'_wpnonce=[0-9a-f]{10}', '_wpnonce=N', text)
    return text


class Site:
    def __init__(self, port, user):
        self.port = port
        self.client = lc.Client(port)
        self.nonce = None
        if user:
            self.client.request('/wp-login.php')
            self.client.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
            self.nonce = self.client.request('/wp-admin/admin-ajax.php?action=rest-nonce')[2].strip()

    def get(self, path):
        url = '/wp-json' + path
        if self.nonce:
            url += ('&' if '?' in url else '?') + '_wpnonce=' + self.nonce
        status, headers, body = self.client.request(url)
        return status, headers.get('X-Wp-Total'), headers.get('X-Wp-Totalpages'), normalize(self.port, body)


def pretty(body):
    try:
        return json.dumps(json.loads(body), indent=1, ensure_ascii=False).splitlines()
    except Exception:
        return body.splitlines()


def main():
    args = sys.argv[1:]
    keep = '--keep' in args
    args = [a for a in args if a != '--keep']
    lines = 8
    if '--diff' in args:
        i = args.index('--diff')
        lines = int(args[i + 1])
        del args[i:i + 2]
    only = args[0] if args else None
    if os.path.exists(SEED_FILE):
        with open(SEED_FILE, encoding='utf-8') as f:
            print('cleaning a leftover seed, left:', php(CLEAN % f.read().strip()))
        os.remove(SEED_FILE)
    seeded = php(SEED)
    ids = json.loads(seeded)
    with open(SEED_FILE, 'w', encoding='utf-8') as f:
        f.write(seeded)
    print('seeded', ids, flush=True)
    try:
        bad = total = 0
        for user in [None, 'rpadmin', 'rpeditor', 'rpauthor', 'rpcontributor', 'rpsubscriber']:
            print('user', user or 'anon', flush=True)
            ref, rp = Site(8080, user), Site(3001, user)
            for path in requests(ids):
                if only and only not in path:
                    continue
                total += 1
                a, b = ref.get(path), rp.get(path)
                if a == b:
                    continue
                bad += 1
                print(f'DIFFERENT {user or "anon"} {path}: {a[:3]} vs {b[:3]}', flush=True)
                diff = [d for d in difflib.unified_diff(pretty(a[3]), pretty(b[3]), lineterm='', n=0) if not d.startswith(('---', '+++'))]
                for d in diff[:lines]:
                    print('   ', d[:200])
        print(f'{total - bad}/{total} identical')
    finally:
        if not keep:
            print('cleaned, left:', php(CLEAN % json.dumps(ids)))
            os.remove(SEED_FILE)


if __name__ == '__main__':
    main()
