#!/usr/bin/env python3
"""Compares the e-mail the REST API sends about comments (approving one, a new note that mentions a user, a changed
note) on the reference (:8080) and RustPress (:3001). Needs what dev/comment-notify-compare.py needs (the mail log
mu-plugin on the reference, RUSTPRESS_SMTP_URL and dev/smtp-sink.py for RustPress).

    python dev/rest-notify-compare.py"""
import importlib.util
import json
import os
import time

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


cnc = load('cnc', 'comment-notify-compare.py')
rc = load('rc', 'rest_compare.py')

SEED = r'''<?php
$editor = get_user_by( 'login', 'rpeditor' );
$mentioned = wp_insert_user( array( 'user_login' => 'rpnotemention', 'user_pass' => 'rustpress-dev', 'user_email' => 'rpnotemention@example.com', 'role' => 'editor', 'display_name' => 'Mentioned One' ) );
$post = wp_insert_post( array( 'post_title' => 'Notify REST post', 'post_status' => 'publish', 'post_author' => $editor->ID, 'post_content' => 'x' ) );
$pending = wp_insert_comment( array( 'comment_post_ID' => $post, 'comment_author' => 'Visitor R', 'comment_author_email' => 'visitor-r@example.com', 'comment_author_url' => '', 'comment_content' => 'cpc-notify rest comment', 'comment_approved' => 0, 'comment_author_IP' => '127.0.0.1', 'comment_date' => '2026-10-20 11:00:00', 'comment_date_gmt' => '2026-10-20 11:00:00' ) );
echo json_encode( array( 'post' => $post, 'pending' => $pending, 'mentioned' => $mentioned, 'admin' => get_user_by( 'login', 'rpadmin' )->ID ) );
'''
CLEAN = r'''<?php
require_once ABSPATH . 'wp-admin/includes/user.php';
$ids = json_decode( '%s', true );
foreach ( get_comments( array( 'post_id' => $ids['post'], 'status' => 'all', 'type' => 'all' ) ) as $c ) { wp_delete_comment( $c->comment_ID, true ); }
wp_delete_post( $ids['post'], true );
wp_delete_user( $ids['mentioned'] );
echo 'ok';
'''


def run(port, ids):
    steps = []
    status, _, _ = rc.call(port, 'PUT', f'/wp/v2/comments/{ids["pending"]}', json.dumps({'status': 'approved'}))
    steps.append(status)
    content = f'cpc-notify note <span class="wp-note-mention user-{ids["mentioned"]}">@Mentioned One</span> and <span class="wp-note-mention user-{ids["admin"]}">@rpadmin</span>'
    status, _, body = rc.call(port, 'POST', '/wp/v2/comments', json.dumps({'post': ids['post'], 'type': 'note', 'content': content}))
    steps.append(status)
    note = json.loads(body).get('id') if status == 201 else None
    if note:
        ids['note'] = note
        status, _, _ = rc.call(port, 'PUT', f'/wp/v2/comments/{note}', json.dumps({'content': '', 'meta': {'_wp_note_status': 'resolved'}}))
        steps.append(status)
        status, _, _ = rc.call(port, 'PUT', f'/wp/v2/comments/{note}', json.dumps({'meta': {'_wp_note_status': 'resolved'}}))
        steps.append(status)
    return steps


def main():
    results = {}
    for port in (8080, 3001):
        ids = json.loads(cnc.rcc.php(SEED))
        cnc.wp_log(': > /tmp/rustpress-wp-mail.log; chmod 666 /tmp/rustpress-wp-mail.log')
        before = len(cnc.mft.mails())
        try:
            steps = run(port, ids)
            time.sleep(1.5)
            mails = cnc.wp_mails() if port == 8080 else cnc.rp_mails(before)
            results[port] = (steps, [cnc.normalize(m, ids, port) for m in mails])
        finally:
            ids.pop('note', None)
            cnc.rcc.php(CLEAN % json.dumps(ids))
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
