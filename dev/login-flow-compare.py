#!/usr/bin/env python3
"""Runs the flows of wp-login.php that change data on the reference (:8080) and on RustPress (:3001) and compares
the answers: reset of a password with a key, registration, the check of the administration e-mail address, the link
of a request for personal data. The data is put back afterwards.

    python dev/login-flow-compare.py [--diff N] [flow]"""
import difflib
import importlib.util
import os
import re
import sys
import time

spec = importlib.util.spec_from_file_location('lc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

KEY = 'abcdefghijklmnopqrst'
results = []
DIFF = 0


def compare(name, a, b):
    """a and b are (status, location, cookies, body) of the reference and of RustPress."""
    problems = []
    if a[0] != b[0]:
        problems.append(f'status {a[0]} vs {b[0]}')
    if a[1] != b[1]:
        problems.append(f'location {a[1]!r} vs {b[1]!r}')
    if a[2] != b[2]:
        problems.append(f'cookies {a[2]} vs {b[2]}')
    if a[3] != b[3]:
        problems.append('body')
    print(('ok   ' if not problems else 'DIFF ') + name + ('  [' + '; '.join(problems) + ']' if problems else ''), flush=True)
    if problems and DIFF and 'body' in problems:
        for line in list(difflib.unified_diff(a[3].splitlines(), b[3].splitlines(), lineterm='', n=0))[:DIFF]:
            print('     ', line[:240])


def shape(client, response):
    status, headers, body = response
    location = re.sub(r'^http://localhost:\d+', '', headers.get('Location', '')) if headers.get('Location') else ''
    return status, location, client.cookies(), lc.norm(body, client.port)


def both(name, steps):
    """Runs `steps(client)` -> response for both servers; prepares nothing."""
    out = []
    for port in (8080, 3001):
        client = lc.Client(port)
        out.append(shape(client, steps(client)))
    compare(name, *out)


def set_key(login, age=0, key=KEY):
    lc.sql(f"update wp_users set user_activation_key='{int(time.time()) - age}:{lc.fast_hash(key)}' where user_login='{login}'")


def reset_flow():
    def with_key(age, key=KEY, login='rpeditor'):
        def run(client):
            set_key('rpeditor', age)
            return client.request(f'/wp-login.php?action=rp&key={key}&login={login}')
        return run

    both('rp: link with a valid key', with_key(0))
    both('rp: link with a wrong key', with_key(0, 'zzzzzzzzzzzzzzzzzzzz'))

    def form(client):
        set_key('rpeditor')
        client.request(f'/wp-login.php?action=rp&key={KEY}&login=rpeditor')
        return client.request('/wp-login.php?action=rp')
    both('rp: the form', form)

    def stale(client):
        set_key('rpeditor', 90_000)
        client.request(f'/wp-login.php?action=rp&key={KEY}&login=rpeditor')
        return client.request('/wp-login.php?action=rp')
    both('rp: the key has expired', stale)

    def post(fields):
        def run(client):
            set_key('rpeditor')
            client.request(f'/wp-login.php?action=rp&key={KEY}&login=rpeditor')
            return client.request('/wp-login.php?action=resetpass', fields)
        return run

    both('resetpass: passwords differ', post({'pass1': 'one-password-1', 'pass2': 'other', 'rp_key': KEY}))
    both('resetpass: only spaces', post({'pass1': '   ', 'pass2': '   ', 'rp_key': KEY}))
    both('resetpass: wrong key in the form', post({'pass1': 'one-password-1', 'pass2': 'one-password-1', 'rp_key': 'nope'}))
    both('resetpass: success', post({'pass1': 'rustpress-dev', 'pass2': 'rustpress-dev', 'rp_key': KEY}))
    lc.sql("update wp_users set user_activation_key='' where user_login='rpeditor'")


def register_flow():
    lc.sql("update wp_options set option_value='1' where option_name='users_can_register'")
    try:
        both('register: form', lambda c: c.request('/wp-login.php?action=register'))
        both('register: nothing sent', lambda c: c.request('/wp-login.php?action=register', {'user_login': '', 'user_email': ''}))
        both('register: bad characters', lambda c: c.request('/wp-login.php?action=register', {'user_login': 'bad name!', 'user_email': 'x@example.com'}))
        both('register: bad e-mail', lambda c: c.request('/wp-login.php?action=register', {'user_login': 'someone', 'user_email': 'not-an-email'}))
        both('register: user exists', lambda c: c.request('/wp-login.php?action=register', {'user_login': 'rpeditor', 'user_email': 'unique@example.com'}))
        both('register: e-mail exists', lambda c: c.request('/wp-login.php?action=register', {'user_login': 'someone', 'user_email': lc.sql("select user_email from wp_users where user_login='rpeditor'")}))
        both('login form with the register link', lambda c: c.request('/wp-login.php'))
        both('lostpassword with the register link', lambda c: c.request('/wp-login.php?action=lostpassword'))
        rows = {}
        for port, name in ((8080, 'regtest-ref'), (3001, 'regtest-rp')):
            client = lc.Client(port)
            response = shape(client, client.request('/wp-login.php?action=register', {'user_login': name, 'user_email': f'{name}@example.com'}))
            rows[port] = (response, lc.sql(f"select user_login,user_nicename,user_url,user_status,display_name,(select group_concat(meta_key,'=',meta_value order by meta_key separator ';') from wp_usermeta where user_id=ID and meta_key not in ('session_tokens')) from wp_users where user_login='{name}'").replace(name, 'NAME'))
            lc.sql(f"delete from wp_usermeta where user_id in (select ID from wp_users where user_login='{name}'); delete from wp_users where user_login='{name}'")
        compare('register: success (answer)', rows[8080][0], rows[3001][0])
        same = rows[8080][1] == rows[3001][1]
        print(('ok   ' if same else 'DIFF ') + 'register: the user that was made')
        if not same:
            print('   ref:', rows[8080][1][:600])
            print('   rp: ', rows[3001][1][:600])
    finally:
        lc.sql("update wp_options set option_value='0' where option_name='users_can_register'")
        lc.sql("delete from wp_usermeta where user_id in (select ID from wp_users where user_login like 'regtest-%'); delete from wp_users where user_login like 'regtest-%'")


def login_as(client, user='rpadmin'):
    client.request('/wp-login.php')
    client.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})


def admin_email_flow():
    original = lc.sql("select option_value from wp_options where option_name='admin_email_lifespan'")

    def run(path, data=None, name=None):
        def go(client):
            login_as(client)
            return client.request(path, data)
        return go
    try:
        both('confirm_admin_email: page', run('/wp-login.php?action=confirm_admin_email'))
        both('confirm_admin_email: redirect_to', run('/wp-login.php?action=confirm_admin_email&redirect_to=%2Fwp-admin%2Fplugins.php'))
        both('confirm_admin_email: bad remind nonce', run('/wp-login.php?action=confirm_admin_email&remind_me_later=0000000000'))
        both('confirm_admin_email: not signed in', lambda c: c.request('/wp-login.php?action=confirm_admin_email'))

        def remind(client):
            login_as(client)
            page = client.request('/wp-login.php?action=confirm_admin_email')[2]
            link = re.search(r'href="([^"]*remind_me_later=[0-9a-f]+)"', page).group(1).replace('&#038;', '&')
            return client.request(re.sub(r'^http://localhost:\d+', '', link))
        both('confirm_admin_email: remind me later', remind)
        lc.sql(f"update wp_options set option_value='{original}' where option_name='admin_email_lifespan'")

        def correct(client):
            login_as(client)
            page = client.request('/wp-login.php?action=confirm_admin_email')[2]
            nonce = re.search(r'name="confirm_admin_email_nonce" value="([0-9a-f]+)"', page).group(1)
            return client.request('/wp-login.php?action=confirm_admin_email', {'confirm_admin_email_nonce': nonce, 'redirect_to': f'http://localhost:{client.port}/wp-admin/', 'correct-admin-email': 'The email is correct'})
        both('confirm_admin_email: the e-mail is correct', correct)
        both('confirm_admin_email: bad nonce', lambda c: (login_as(c), c.request('/wp-login.php?action=confirm_admin_email', {'confirm_admin_email_nonce': 'bad', 'correct-admin-email': 'x'}))[1])
        # After the half year the sign-in leads to the confirmation.
        lc.sql("update wp_options set option_value='1' where option_name='admin_email_lifespan'")

        def signin(client):
            client.request('/wp-login.php')
            return client.request('/wp-login.php', {'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1', 'redirect_to': f'http://localhost:{client.port}/wp-admin/'})
        both('sign-in after the lifespan has ended', signin)
    finally:
        lc.sql(f"update wp_options set option_value='{original}' where option_name='admin_email_lifespan'")


def interim_flow():
    def signin(client):
        client.request('/wp-login.php')
        return client.request('/wp-login.php', {'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1', 'interim-login': '1'})
    both('interim sign-in', signin)

    def wrong(client):
        client.request('/wp-login.php')
        return client.request('/wp-login.php', {'log': 'rpadmin', 'pwd': 'wrong', 'testcookie': '1', 'interim-login': '1'})
    both('interim sign-in, wrong password', wrong)


def confirmaction_flow():
    def make(action, key=KEY, age=0, status='request-pending'):
        lc.sql("delete from wp_posts where post_type='user_request'")
        now = time.strftime('%Y-%m-%d %H:%M:%S', time.gmtime(time.time() - age))
        lc.sql(f"insert into wp_posts (post_author,post_date,post_date_gmt,post_content,post_title,post_excerpt,post_status,comment_status,ping_status,post_password,post_name,to_ping,pinged,post_modified,post_modified_gmt,post_content_filtered,post_parent,guid,menu_order,post_type,post_mime_type,comment_count) values (0,'{now}','{now}','[]','confirm@example.com','','{status}','closed','closed','{lc.fast_hash(key)}','{action}','','','{now}','{now}','',0,'',0,'user_request','',0)")
        return lc.sql("select max(ID) from wp_posts where post_type='user_request'")
    try:
        for action in ('export_personal_data', 'remove_personal_data'):
            def run(client, action=action):
                rid = make(action)
                response = client.request(f'/wp-login.php?action=confirmaction&request_id={rid}&confirm_key={KEY}')
                status = lc.sql(f"select post_status from wp_posts where ID={rid}")
                meta = lc.sql(f"select count(*) from wp_postmeta where post_id={rid} and meta_key='_wp_user_request_confirmed_timestamp'")
                return response[0], response[1], response[2] + f'\n[status={status} meta={meta}]'
            both(f'confirmaction: {action}', run)

        def wrong(client):
            rid = make('export_personal_data')
            return client.request(f'/wp-login.php?action=confirmaction&request_id={rid}&confirm_key=wrongwrongwrongwrong')
        both('confirmaction: wrong key', wrong)

        def old(client):
            rid = make('export_personal_data', age=90_000)
            return client.request(f'/wp-login.php?action=confirmaction&request_id={rid}&confirm_key={KEY}')
        both('confirmaction: expired key', old)

        def done(client):
            rid = make('export_personal_data', status='request-confirmed')
            return client.request(f'/wp-login.php?action=confirmaction&request_id={rid}&confirm_key={KEY}')
        both('confirmaction: already confirmed', done)
    finally:
        lc.sql("delete from wp_postmeta where post_id in (select ID from wp_posts where post_type='user_request'); delete from wp_posts where post_type='user_request'")


FLOWS = {'reset': reset_flow, 'register': register_flow, 'admin_email': admin_email_flow, 'interim': interim_flow, 'confirmaction': confirmaction_flow}


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
    for name, flow in FLOWS.items():
        if only and only != name:
            continue
        print('==', name)
        flow()


if __name__ == '__main__':
    main()
