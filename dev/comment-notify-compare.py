#!/usr/bin/env python3
"""Compares the e-mail the comment screens send (approving a comment, replying to one) on the reference (:8080) and
RustPress (:3001). The reference logs what it hands to wp_mail() (dev/mu-plugins/rustpress-mail-log.php); RustPress
must run with RUSTPRESS_SMTP_URL=smtp://127.0.0.1:2525 and dev/smtp-sink.py listening.

    python dev/comment-notify-compare.py

Each side gets a post by rpeditor with comments that wait for approval; rpadmin approves them in the four ways the
screens offer and replies to one. The test data is removed again."""
import email.header
import importlib.util
import json
import os
import re
import subprocess
import time
import urllib.parse

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


rcc = load('rcc', 'rest-comments-compare.py')
mft = load('mft', 'mail-flow-test.py')

SEED = r'''<?php
$editor = get_user_by( 'login', 'rpeditor' );
$post = wp_insert_post( array( 'post_title' => 'Notify post', 'post_status' => 'publish', 'post_author' => $editor->ID, 'post_content' => 'x' ) );
$ids = array( 'post' => $post );
foreach ( array( 'a', 'b', 'c', 'd' ) as $i => $key ) {
	$ids[ $key ] = wp_insert_comment( array( 'comment_post_ID' => $post, 'comment_author' => 'Visitor ' . strtoupper( $key ), 'comment_author_email' => 'visitor-' . $key . '@example.com', 'comment_author_url' => '', 'comment_content' => 'cpc-notify comment ' . $key, 'comment_approved' => 0, 'comment_author_IP' => '127.0.0.1', 'comment_date' => '2026-10-20 10:0' . $i . ':00', 'comment_date_gmt' => '2026-10-20 10:0' . $i . ':00' ) );
}
echo json_encode( $ids );
'''
CLEAN = r'''<?php
$ids = json_decode( '%s', true );
foreach ( get_comments( array( 'post_id' => $ids['post'], 'status' => 'all' ) ) as $c ) { wp_delete_comment( $c->comment_ID, true ); }
wp_delete_post( $ids['post'], true );
echo 'ok';
'''


def wp_log(command):
    return subprocess.run(['wsl', '-d', 'Debian', '--', 'podman', 'exec', 'rustpress-wp', 'sh', '-c', command], capture_output=True, text=True).stdout


def header_text(value):
    return str(email.header.make_header(email.header.decode_header(value)))


def rp_mails(before):
    out = []
    for mail in mft.mails()[before:]:
        headers, body = mft.decode(mail)
        fields = {}
        for line in headers.split('\n'):
            if ':' in line and not line.startswith(' '):
                k, v = line.split(':', 1)
                fields[k.strip().lower()] = v.strip()
        out.append({'to': fields.get('to', ''), 'subject': header_text(fields.get('subject', '')), 'message': body})
    return out


def wp_mails():
    out = []
    for line in wp_log('cat /tmp/rustpress-wp-mail.log 2>/dev/null').splitlines():
        atts = json.loads(line)
        to = atts['to'] if isinstance(atts['to'], str) else ', '.join(atts['to'])
        out.append({'to': to, 'subject': atts['subject'], 'message': atts['message']})
    return out


def normalize(mail, ids, port):
    # The mail sink adds line breaks at the end of what it keeps.
    mail = dict(mail, message=mail['message'].replace('\r\n', '\n').rstrip('\n'))
    text = json.dumps(mail, ensure_ascii=False)
    for key, value in sorted(ids.items(), key=lambda kv: -len(str(kv[1]))):
        text = re.sub(rf'(?<![0-9]){value}(?![0-9])', f'<{key}>', text)
    text = text.replace(f'localhost:{port}', 'localhost:PORT')
    # The reply gets an ID of its own.
    text = re.sub(r'(?<=comment-)[0-9]+|(?<=c=)[0-9]+', '<new>', text)
    return json.loads(text)


def nonce_of(page, pattern):
    m = re.search(pattern, page)
    return m.group(1) if m else ''


def run(port, ids):
    c = mft.signed_in(port)
    post = ids['post']
    page = c.request(f'/wp-admin/edit-comments.php?comment_status=moderated&p={post}')[2].replace('&#038;', '&').replace('&amp;', '&')
    approve = {key: nonce_of(page, rf'action=approvecomment&c={ids[key]}&_wpnonce=([0-9a-f]+)') for key in ('a', 'b', 'c', 'd')}
    bulk = nonce_of(page, r'name="_wpnonce" value="([0-9a-f]+)"')
    reply = nonce_of(page, r'name="_ajax_nonce-replyto-comment" value="([0-9a-f]+)"')
    steps = []
    # The row action with Ajax (`dim-comment`).
    steps.append(c.request('/wp-admin/admin-ajax.php', {'action': 'dim-comment', 'id': ids['a'], 'new': 'approved', '_ajax_nonce': approve['a']})[0])
    # The row action without JavaScript.
    steps.append(c.request(f'/wp-admin/comment.php?c={ids["b"]}&action=approvecomment&_wpnonce={approve["b"]}')[0])
    # The bulk action.
    steps.append(c.request(f'/wp-admin/edit-comments.php?{urllib.parse.urlencode({"action": "approve", "delete_comments[]": ids["c"], "_wpnonce": bulk, "comment_status": "moderated"})}')[0])
    # A reply that approves the comment it answers.
    steps.append(c.request('/wp-admin/admin-ajax.php', {'action': 'replyto-comment', 'comment_ID': ids['d'], 'comment_post_ID': post, 'approve_parent': '1', 'content': 'cpc-notify reply', '_ajax_nonce-replyto-comment': reply})[0])
    return steps


def main():
    results = {}
    for port in (8080, 3001):
        ids = json.loads(rcc.php(SEED))
        wp_log(": > /tmp/rustpress-wp-mail.log; chmod 666 /tmp/rustpress-wp-mail.log")
        before = len(mft.mails())
        try:
            steps = run(port, ids)
            time.sleep(1.5)
            mails = wp_mails() if port == 8080 else rp_mails(before)
            results[port] = (steps, [normalize(m, ids, port) for m in mails])
        finally:
            rcc.php(CLEAN % json.dumps(ids))
    (steps_a, mails_a), (steps_b, mails_b) = results[8080], results[3001]
    print('steps', steps_a, steps_b, 'same' if steps_a == steps_b else 'DIFFERENT')
    print('mails', len(mails_a), len(mails_b))
    same = 0
    for i, (a, b) in enumerate(zip(mails_a, mails_b)):
        if a == b:
            same += 1
            continue
        for key in a:
            if a[key] != b.get(key):
                print(f'DIFFERENT mail {i} {key}:\n  ref: {a[key]!r}\n  rp:  {b.get(key)!r}')
    for extra in mails_a[len(mails_b):]:
        print('only ref:', extra['subject'], extra['to'])
    for extra in mails_b[len(mails_a):]:
        print('only rp:', extra['subject'], extra['to'])
    print(f'{same}/{max(len(mails_a), len(mails_b))} identical')


if __name__ == '__main__':
    main()
