#!/usr/bin/env python3
"""Signs in with stored password hashes of every format on the reference (:8080) and RustPress (:3001) and compares
what happens to the row: `wp_password_needs_rehash()` and `wp_set_password()` in `wp_authenticate_username_password()`
and `wp_authenticate_email_password()`, and that the cookie of that sign-in works with the new hash. The hash of the
user is put back afterwards.

    python dev/login-rehash-compare.py"""
import base64
import hashlib
import hmac
import importlib.util
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

USER = 'rpeditor'
EMAIL_KEY = 'user_email'
PASSWORD = 'test12345'


def php_hash(password, cost):
    code = f"echo password_hash('{password}', PASSWORD_BCRYPT, ['cost' => {cost}]);"
    out = subprocess.run(['wsl', 'podman', 'exec', 'rustpress-wp', 'php', '-r', code], capture_output=True, text=True, env={**os.environ, 'MSYS_NO_PATHCONV': '1'})
    return out.stdout.strip()


def prehash(password):
    return base64.b64encode(hmac.new(b'wp-sha384', password.encode(), hashlib.sha384).digest()).decode()


def row():
    out = lc.sql(f"SELECT user_pass, user_activation_key FROM wp_users WHERE user_login = '{USER}'")
    return (out.split('\n')[-1].split('\t') + [''])[:2]


def store(hash_value, key):
    # `$` survives the quoting of the SQL string; the shell of sql.ps1 gets it through a variable.
    lc.sql(f"UPDATE wp_users SET user_pass = '{hash_value}', user_activation_key = '{key}' WHERE user_login = '{USER}'")


def shape(stored):
    if stored.startswith('$wp$2y$10$') and len(stored) == 63:
        return 'current'
    return stored[:12]


def attempt(port, login, password):
    client = lc.Client(port)
    client.request('/wp-login.php')
    status, headers, _ = client.request('/wp-login.php', {'log': login, 'pwd': password, 'testcookie': '1', 'redirect_to': '/wp-admin/profile.php'})
    profile, _, _ = client.request('/wp-admin/profile.php')
    return status, headers.get('Location', '').replace(f'localhost:{port}', 'HOST'), profile


def main():
    original, original_key = row()
    email = lc.sql(f"SELECT {EMAIL_KEY} FROM wp_users WHERE user_login = '{USER}'").split('\n')[-1]
    formats = {
        'md5': (hashlib.md5(PASSWORD.encode()).hexdigest(), True),
        'phpass': ('$P$9IQRaTwmfeRo7ud9Fh4E2PdI0S3r.L0', True),
        'bcrypt without prefix': (php_hash(PASSWORD, 10), True),
        'prefixed, cost 8': ('$wp' + php_hash(prehash(PASSWORD), 8), True),
        'prefixed, cost 10': ('$wp' + php_hash(prehash(PASSWORD), 10), False),
    }
    bad = 0
    try:
        for name, (stored, _) in formats.items():
            for login, label in ((USER, 'login'), (email, 'email')):
                for password, wrong in ((PASSWORD, False), (PASSWORD + 'x', True)):
                    results = []
                    for port in (8080, 3001):
                        store(stored, 'stale-key')
                        status, location, profile = attempt(port, login, password)
                        now, key = row()
                        results.append((status, location, profile, 'unchanged' if now == stored else shape(now), key))
                    same = results[0] == results[1]
                    bad += not same
                    print(('ok   ' if same else 'DIFF ') + f'{name} / {label} / {"wrong" if wrong else "right"} password: {results[0]}' + ('' if same else f' vs {results[1]}'), flush=True)
    finally:
        store(original, original_key)
    print('differences:', bad)


if __name__ == '__main__':
    main()
