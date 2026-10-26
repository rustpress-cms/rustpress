#!/usr/bin/env python3
"""Runs the flows that send e-mail against RustPress (:3001) and checks what arrives at the mail sink
(dev/smtp-sink.py; RustPress started with RUSTPRESS_SMTP_URL=smtp://127.0.0.1:2525).

    python dev/mail-flow-test.py

The test data it makes is removed again."""
import importlib.util
import os
import re
import sys
import time

spec = importlib.util.spec_from_file_location('lc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

LOG = os.path.join(os.environ.get('TEMP', '.'), 'smtp-sink.log')
failures = 0


def mails():
    if not os.path.exists(LOG):
        return []
    text = open(LOG, encoding='utf-8', newline='').read().replace('\r\n', '\n')
    out = []
    for block in text.split('=== FROM ')[1:]:
        head, _, body = block.partition('\n')
        out.append((head, body))
    return out


def wait_for(count_before, timeout=10):
    end = time.time() + timeout
    while time.time() < end:
        if len(mails()) > count_before:
            return mails()[count_before:]
        time.sleep(0.3)
    return []


def check(name, condition, detail=''):
    global failures
    print(('ok   ' if condition else 'FAIL ') + name + (f'  [{detail}]' if detail and not condition else ''), flush=True)
    if not condition:
        failures += 1


def body_of(mail):
    # The message is text/plain; quoted-printable or 8bit.
    text = mail[1]
    return text.split('\n\n', 1)[1] if '\n\n' in text else text


def decode(mail):
    import quopri
    raw = mail[1].replace('\r\n', '\n')
    headers, _, body = raw.partition('\n\n')
    if 'quoted-printable' in headers.lower():
        body = quopri.decodestring(body.encode()).decode('utf-8', 'replace')
    return headers, body


def signed_in(port=3001, user='rpadmin'):
    c = lc.Client(port)
    c.request('/wp-login.php')
    c.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
    return c


def lost_password():
    before = len(mails())
    c = lc.Client(3001)
    status, headers, body = c.request('/wp-login.php?action=lostpassword', {'user_login': 'rpeditor'})
    check('lostpassword: redirect to checkemail', status == 302 and 'checkemail=confirm' in headers.get('Location', ''), f'{status} {headers.get("Location")}')
    new = wait_for(before)
    check('lostpassword: a message arrives', len(new) == 1)
    if not new:
        return
    h, b = decode(new[0])
    check('lostpassword: subject', 'Password Reset' in h, h[:300])
    link = re.search(r'(http://\S+wp-login\.php\?login=\S+&key=\S+&action=rp\S*)', b)
    check('lostpassword: the link', bool(link), b[:400])
    if not link:
        return
    url = re.sub(r'^http://[^/]+', '', link.group(1))
    c2 = lc.Client(3001)
    s, h2, _ = c2.request(url)
    check('reset link: redirect', s == 302, str(s))
    s, h2, page = c2.request('/wp-login.php?action=rp')
    check('reset link: the form', 'resetpassform' in page)
    before = len(mails())
    s, h2, page = c2.request('/wp-login.php?action=resetpass', {'pass1': 'rustpress-dev', 'pass2': 'rustpress-dev', 'rp_key': re.search(r'key=([A-Za-z0-9]+)', url).group(1)})
    check('reset: success page', 'Your password has been reset' in page)
    new = wait_for(before)
    check('reset: the administrator is told', len(new) == 1 and 'Password Changed' in decode(new[0])[0], str(new)[:200])


def register():
    lc.sql("update wp_options set option_value='1' where option_name='users_can_register'")
    try:
        before = len(mails())
        c = lc.Client(3001)
        status, headers, body = c.request('/wp-login.php?action=register', {'user_login': 'regmail', 'user_email': 'regmail@example.com'})
        check('register: redirect', status == 302 and 'registered' in headers.get('Location', ''), f'{status} {headers.get("Location")}')
        new = wait_for(before + 1) if False else None
        time.sleep(1.5)
        new = mails()[before:]
        check('register: two messages', len(new) == 2, str(len(new)))
        subjects = [decode(m)[0] for m in new]
        check('register: admin and user messages', any('New User Registration' in s for s in subjects) and any('Login Details' in s for s in subjects))
    finally:
        lc.sql("update wp_options set option_value='0' where option_name='users_can_register'")
        lc.sql("delete from wp_usermeta where user_id in (select ID from wp_users where user_login='regmail'); delete from wp_users where user_login='regmail'")


def privacy():
    c = signed_in()
    page = c.request('/wp-admin/export-personal-data.php')[2]
    nonce = re.search(r'name="_wpnonce" value="([0-9a-f]+)"', page).group(1)
    before = len(mails())
    c.request('/wp-admin/export-personal-data.php', {'_wpnonce': nonce, 'action': 'add_export_personal_data_request', 'type_of_action': 'export_personal_data', 'username_or_email_for_privacy_request': 'mailtest@example.com', 'send_confirmation_email': '1'})
    new = wait_for(before)
    check('privacy: the confirmation e-mail', len(new) == 1 and 'Confirm Action' in decode(new[0])[0], str(new)[:200])
    if not new:
        return
    link = re.search(r'(http://\S+wp-login\.php\?action=confirmaction&request_id=\d+&confirm_key=\w+)', decode(new[0])[1])
    check('privacy: the link', bool(link))
    if link:
        before = len(mails())
        s, h, page = lc.Client(3001).request(re.sub(r'^http://[^/]+', '', link.group(1)))
        check('privacy: confirmed', 'Thanks for confirming your export request' in page, page[-300:])
        new = wait_for(before)
        check('privacy: the administrator is told', len(new) == 1 and 'Action Confirmed' in decode(new[0])[0], str(new)[:200])
        # Send the export link.
        page = c.request('/wp-admin/export-personal-data.php')[2]
        rid = re.search(r'data-send-as-email="1"[^>]*data-request-id="(\d+)" data-nonce="([0-9a-f]+)"', page)
        if rid:
            for exporter in (1, 2, 3):
                before = len(mails())
                s, text = c.request('/wp-admin/admin-ajax.php', {'action': 'wp-privacy-export-personal-data', 'id': rid.group(1), 'security': rid.group(2), 'exporter': exporter, 'page': 1, 'sendAsEmail': 'true'})[0::2]
            new = wait_for(before)
            check('privacy: the export e-mail', len(new) == 1 and 'Personal Data Export' in decode(new[0])[0], str(new)[:200])
            check('privacy: the request is completed', lc.sql(f"select post_status from wp_posts where ID={rid.group(1)}") == 'request-completed')
        else:
            check('privacy: the send export link button', False)
    lc.sql("delete from wp_postmeta where post_id in (select ID from wp_posts where post_type='user_request'); delete from wp_posts where post_type='user_request'")


def profile_email():
    c = signed_in(user='rpeditor')
    page = c.request('/wp-admin/profile.php')[2]
    nonce = re.search(r'name="_wpnonce" value="([0-9a-f]+)"', page).group(1)
    original = lc.sql("select user_email from wp_users where user_login='rpeditor'")
    before = len(mails())
    fields = {'_wpnonce': nonce, 'action': 'update', 'user_id': lc.sql("select ID from wp_users where user_login='rpeditor'"), 'email': 'newaddress@example.com', 'nickname': 'rpeditor', 'display_name': 'rpeditor'}
    s, h, b = c.request('/wp-admin/profile.php', fields)
    new = wait_for(before)
    check('profile: confirmation to the new address', len(new) == 1 and 'Email Change Request' in decode(new[0])[0], str(new)[:300])
    check('profile: the address stays', lc.sql("select user_email from wp_users where user_login='rpeditor'") == original)
    if new:
        link = re.search(r'(http://\S+profile\.php\?newuseremail=\w+)', decode(new[0])[1])
        check('profile: link', bool(link))
        if link:
            s, h, b = c.request(re.sub(r'^http://[^/]+', '', link.group(1)))
            check('profile: link works', s == 302 and lc.sql("select user_email from wp_users where user_login='rpeditor'") == 'newaddress@example.com', f'{s}')
    lc.sql(f"update wp_users set user_email='{original}' where user_login='rpeditor'; delete from wp_usermeta where meta_key='_new_email'")


def users_reset():
    c = signed_in()
    page = c.request('/wp-admin/users.php')[2]
    nonce = re.search(r'name="_wpnonce" value="([0-9a-f]+)"', page).group(1)
    rid = lc.sql("select ID from wp_users where user_login='rpeditor'")
    before = len(mails())
    s, h, b = c.request('/wp-admin/users.php', {'_wpnonce': nonce, 'action': 'resetpassword', 'users[]': rid})
    new = wait_for(before)
    check('users.php: reset link mailed', len(new) == 1 and 'Password Reset' in decode(new[0])[0], f'{s} {h.get("Location")} {str(new)[:200]}')
    check('users.php: redirect', 'reset_count=1' in h.get('Location', ''), h.get('Location', ''))
    lc.sql("update wp_users set user_activation_key='' where user_login='rpeditor'")


def admin_email():
    c = signed_in()
    original = lc.sql("select option_value from wp_options where option_name='admin_email'")
    page = c.request('/wp-admin/options-general.php')[2]
    fields = {}
    for name in re.findall(r'<input[^>]*name="([^"]+)"[^>]*>', page):
        pass
    nonce = re.search(r'name="_wpnonce" value="([0-9a-f]+)"', page).group(1)
    before = len(mails())
    s, h, b = c.request('/wp-admin/options.php', {'_wpnonce': nonce, 'option_page': 'general', 'action': 'update', 'new_admin_email': 'newadmin@example.com', 'blogname': 'RustPress Test', 'admin_email': original, 'timezone_string': 'UTC', 'date_format': 'F j, Y', 'time_format': 'g:i a', 'start_of_week': '1', 'WPLANG': ''})
    new = wait_for(before)
    check('options: the new administration address gets a link', len(new) == 1 and 'New Admin Email Address' in decode(new[0])[0], f'{s} {str(new)[:200]}')
    lc.sql("delete from wp_options where option_name in ('adminhash','new_admin_email')")


def main():
    flows = {'lost': lost_password, 'register': register, 'privacy': privacy, 'profile': profile_email, 'users': users_reset, 'admin_email': admin_email}
    only = sys.argv[1] if len(sys.argv) > 1 else None
    for name, flow in flows.items():
        if only and only != name:
            continue
        print('==', name)
        flow()
    print('failures:', failures)


if __name__ == '__main__':
    main()
