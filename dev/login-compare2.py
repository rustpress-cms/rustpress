#!/usr/bin/env python3
"""Compares the actions of wp-login.php (lost password, reset, register, check e-mail, post password, the dialog of an
expired session, unknown actions) of the reference (:8080) and RustPress (:3001): status, redirect, cookies and body.

    python dev/login-compare2.py [--diff N] [substring-of-scenario]

Scenarios that change data (keys of password reset, the option users_can_register) put it back afterwards."""
import base64
import difflib
import hashlib
import http.cookiejar
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

# `localhost` resolves to ::1 first, and Windows gives up on it only after about two seconds; connecting to
# 127.0.0.1 directly keeps the `Host: localhost:port` header the sites expect.
import socket

_getaddrinfo = socket.getaddrinfo


def _ipv4_localhost(host, *args, **kwargs):
    return _getaddrinfo('127.0.0.1' if host == 'localhost' else host, *args, **kwargs)


socket.getaddrinfo = _ipv4_localhost

HERE = __file__.rsplit('\\', 1)[0].rsplit('/', 1)[0]


def sql(statement):
    # The call through PowerShell and WSL now and then never returns: give it a limit and try again.
    for _ in range(3):
        try:
            out = subprocess.run(['powershell', '-File', f'{HERE}/sql.ps1', 'wordpress', statement], capture_output=True, text=True, encoding='utf-8', timeout=60)
            return out.stdout.strip()
        except subprocess.TimeoutExpired:
            continue
    raise RuntimeError(f'sql did not answer: {statement[:80]}')


def fast_hash(key):
    digest = hashlib.blake2b(key.encode(), key=b'wp_fast_hash_6.8+', digest_size=30).digest()
    return '$generic$' + base64.urlsafe_b64encode(digest).decode().rstrip('=')


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


class Client:
    def __init__(self, port):
        self.port = port
        self.jar = http.cookiejar.CookieJar()
        self.op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.jar), NoRedirect())

    def request(self, path, data=None, headers=None):
        url = f'http://localhost:{self.port}{path}'
        body = urllib.parse.urlencode(data).encode() if data is not None else None
        req = urllib.request.Request(url, data=body, headers=headers or {})
        try:
            r = self.op.open(req)
            return r.status, {k.title(): v for k, v in r.headers.items()}, r.read().decode('utf-8', 'replace')
        except urllib.error.HTTPError as e:
            return e.code, {k.title(): v for k, v in e.headers.items()}, e.read().decode('utf-8', 'replace')

    def cookies(self):
        return sorted(re.sub(r'[0-9a-f]{32}', 'H', c.name) + '@' + c.path for c in self.jar)


def norm(text, port):
    text = text.replace(f'localhost:{port}', 'HOST').replace(f'localhost%3A{port}', 'HOST')
    text = re.sub(r'(_wpnonce|nonce|remind_me_later|confirm_admin_email_nonce)(=|[=":\' ]+value=")[0-9a-f]{10}', r'\1N', text)
    text = re.sub(r'value="[0-9a-f]{10}"', 'value="N"', text)
    text = re.sub(r'data-pw="[^"]*"', 'data-pw="PW"', text)
    text = re.sub(r'"time":"\d+"', '"time":"T"', text)
    text = re.sub(r'"nonce":"[0-9a-f]{10}"', '"nonce":"N"', text)
    return text.replace('\r\n', '\n')


def scenario_pages(c):
    for path in ['/wp-login.php', '/wp-login.php?action=lostpassword', '/wp-login.php?action=retrievepassword', '/wp-login.php?action=register', '/wp-login.php?action=rp',
                 '/wp-login.php?action=resetpass', '/wp-login.php?key=x&login=y', '/wp-login.php?loggedout=true', '/wp-login.php?checkemail=confirm',
                 '/wp-login.php?checkemail=registered', '/wp-login.php?checkemail=nope', '/wp-login.php?action=nope', '/wp-login.php?action=',
                 '/wp-login.php?interim-login=1', '/wp-login.php?reauth=1', '/wp-login.php?registration=disabled', '/wp-login.php?action=confirmaction',
                 '/wp-login.php?action=confirmaction&request_id=1', '/wp-login.php?action=confirmaction&request_id=1&confirm_key=x',
                 '/wp-login.php?action=confirm_admin_email', '/wp-login.php?action=lostpassword&error=invalidkey', '/wp-login.php?action=lostpassword&error=expiredkey',
                 '/wp-login.php?redirect_to=%2Fwp-admin%2Fplugins.php&action=lostpassword',
                 '/wp-login.php?redirect_to=http%3A%2F%2Flocalhost%3A8080%2Fwp-admin%2Fauthorize-application.php%3Fapp_name%3DTest']:
        yield 'GET ' + path, lambda cl, p=path: cl.request(p)


def scenario_posts(c):
    form = {'user_login': 'nobody-here'}
    yield 'POST lostpassword empty', lambda cl: cl.request('/wp-login.php?action=lostpassword', {'user_login': ''})
    yield 'POST lostpassword unknown', lambda cl: cl.request('/wp-login.php?action=lostpassword', form)
    yield 'POST lostpassword unknown email', lambda cl: cl.request('/wp-login.php?action=lostpassword', {'user_login': 'nobody@example.com'})
    yield 'POST lostpassword known (mail fails)', lambda cl: cl.request('/wp-login.php?action=lostpassword', {'user_login': 'rpeditor'})
    yield 'POST postpass', lambda cl: cl.request('/wp-login.php?action=postpass', {'post_password': 'secret', 'redirect_to': f'http://localhost:{cl.port}/geschuetzt/'})
    yield 'POST postpass no redirect', lambda cl: cl.request('/wp-login.php?action=postpass', {'post_password': 'secret'})
    yield 'POST postpass no password', lambda cl: cl.request('/wp-login.php?action=postpass', {'redirect_to': f'http://localhost:{cl.port}/'})
    yield 'GET postpass', lambda cl: cl.request('/wp-login.php?action=postpass')
    for log, pwd in (('rpadmin', 'falsch'), ('nobody', 'x'), ('', ''), ('rpadmin', ''), ('x@y.de', 'z')):
        yield f'POST login {log}|{pwd}', lambda cl, log=log, pwd=pwd: (cl.request('/wp-login.php'), cl.request('/wp-login.php', {'log': log, 'pwd': pwd, 'testcookie': '1', 'redirect_to': '/wp-admin/'}))[1]
    yield 'POST login ok', lambda cl: (cl.request('/wp-login.php'), cl.request('/wp-login.php', {'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1', 'redirect_to': '/wp-admin/'}))[1]
    yield 'POST register disabled', lambda cl: cl.request('/wp-login.php?action=register', {'user_login': 'x', 'user_email': 'x@example.com'})


def main():
    args = sys.argv[1:]
    diff_lines = 0
    only = None
    i = 0
    while i < len(args):
        if args[i] == '--diff':
            diff_lines = int(args[i + 1])
            i += 2
        else:
            only = args[i]
            i += 1
    scenarios = list(scenario_pages(None)) + list(scenario_posts(None))
    for name, run in scenarios:
        if only and only not in name:
            continue
        results = []
        for port in (8080, 3001):
            client = Client(port)
            status, headers, body = run(client)
            location = re.sub(r'^http://localhost:\d+', '', headers.get('Location', '')) if headers.get('Location') else ''
            results.append((status, location, client.cookies(), norm(body, port)))
        a, b = results
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
        if problems and diff_lines and 'body' in problems:
            for line in list(difflib.unified_diff(a[3].splitlines(), b[3].splitlines(), lineterm='', n=0))[:diff_lines]:
                print('     ', line[:240])


if __name__ == '__main__':
    main()
