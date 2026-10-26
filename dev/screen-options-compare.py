#!/usr/bin/env python3
"""Compares saving the Screen Options (`set_screen_options()`) of the reference (:8080) and RustPress (:3001) as
rpadmin: status and redirect of the POST, what is stored, and the list page that follows. The two share the
database, so each side runs alone and the stored options are removed (and `user-settings` restored) afterwards.

    python dev/screen-options-compare.py"""
import difflib
import importlib.util
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

USER = "(SELECT ID FROM wp_users WHERE user_login='rpadmin')"
KEYS = "meta_key LIKE '%per\\_page' OR meta_key LIKE 'manage%columnshidden' OR meta_key = 'layout_columns'"

# (page, option, value, extra form fields, referer path or None, nonce ok)
CASES = [
    ('edit.php', 'edit_post_per_page', '5', {'mode': 'excerpt'}, '/wp-admin/edit.php?paged=2&post_status=all', True),
    ('edit.php?post_type=page', 'edit_page_per_page', '3', {'mode': 'list'}, None, True),
    ('users.php', 'users_per_page', '1000', {}, None, True),
    ('users.php', 'users_per_page', '2', {}, '/wp-admin/users.php?paged=3&role=administrator', True),
    ('edit-tags.php?taxonomy=post_tag', 'edit_post_tag_per_page', '7', {}, None, True),
    ('edit-tags.php?taxonomy=category', 'edit_category_per_page', '0', {}, None, True),
    ('upload.php?mode=list', 'upload_per_page', '12abc', {}, None, True),
    ('plugins.php', 'plugins_per_page', '50', {}, None, True),
    ('edit-comments.php', 'edit_comments_per_page', '4', {'mode': 'list'}, '/wp-admin/edit-comments.php?apage=2&pagenum=3', True),
    ('index.php', 'layout_columns', '2', {}, None, True),
    ('edit.php', 'Edit_post_per_page', '5', {}, None, True),
    ('edit.php', 'edit_post_per_page', '5', {}, None, False),
]


def normalize(html):
    out = subprocess.run([sys.executable, os.path.join(HERE, 'admin_norm.py')], input=html.encode('utf-8'), capture_output=True)
    return out.stdout.decode('utf-8', 'replace')


def stored():
    return lc.sql(f"SELECT meta_key, meta_value FROM wp_usermeta WHERE user_id = {USER} AND ({KEYS}) ORDER BY meta_key")


def run(port, settings):
    c = lc.Client(port)
    c.request('/wp-login.php')
    c.request('/wp-login.php', {'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'})
    results = []
    for page, option, value, extra, referer, nonce_ok in CASES:
        status, headers, body = c.request('/wp-admin/' + page)
        nonce = re.search(r'name="screenoptionnonce" value="([0-9a-f]{10})"', body)
        form = {'wp_screen_options[option]': option, 'wp_screen_options[value]': value, 'screenoptionnonce': nonce.group(1) if nonce and nonce_ok else 'bad', 'screen-options-apply': 'Apply', **extra}
        hdrs = {'Referer': f'http://localhost:{port}{referer}'} if referer else {}
        status, headers, body = c.request('/wp-admin/' + page, form, hdrs)
        location = (headers.get('Location') or '').replace(f'localhost:{port}', 'HOST')
        after = ''
        if status in (301, 302) and headers.get('Location'):
            target = headers['Location'].replace(f'http://localhost:{port}', '')
            _, _, follow = c.request(target)
            after = normalize(follow)
        elif status == 200:
            after = normalize(body)
        else:
            after = re.sub(r'\s+', ' ', re.sub(r'<[^>]+>', ' ', body[body.find('<body'):]))[:300]
        results.append((page, option, value, status, location, stored(), after))
        lc.sql(f"DELETE FROM wp_usermeta WHERE user_id = {USER} AND ({KEYS})")
        lc.sql(f"UPDATE wp_usermeta SET meta_value = '{settings}' WHERE user_id = {USER} AND meta_key = 'wp_user-settings'")
    return results


def main():
    settings = lc.sql(f"SELECT meta_value FROM wp_usermeta WHERE user_id = {USER} AND meta_key = 'wp_user-settings'") or 'posts_list_mode=list'
    lc.sql(f"DELETE FROM wp_usermeta WHERE user_id = {USER} AND ({KEYS})")
    ref = run(8080, settings)
    rp = run(3001, settings)
    bad = 0
    for a, b in zip(ref, rp):
        label = f'{a[0]} {a[1]}={a[2]}'
        same = a[3:6] == b[3:6]
        diff = [d for d in difflib.unified_diff(a[6].splitlines(), b[6].splitlines(), lineterm='', n=0) if not d.startswith(('---', '+++'))]
        if same and not diff:
            print(f'ok        {label} [{a[3]} {a[4]}] {a[5]!r}')
            continue
        bad += 1
        print(f'DIFFERENT {label}')
        if not same:
            print(f'    ref: {a[3]} {a[4]!r} {a[5]!r}')
            print(f'    rp:  {b[3]} {b[4]!r} {b[5]!r}')
        for d in diff[:12]:
            print('   ', d[:240])
    print(f'{len(ref) - bad}/{len(ref)} ok')


if __name__ == '__main__':
    main()
