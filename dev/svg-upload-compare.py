#!/usr/bin/env python3
"""Uploads SVG files through async-upload.php to the reference (:8080) and RustPress (:3001) as the same user and
compares the answers, the stored metadata and the stored (sanitized) files. SVG Support must be active.

    python dev/svg-upload-compare.py [--user rpeditor] [--keep]

The attachments are deleted afterwards unless --keep is given."""
import importlib.util
import json
import os
import re
import subprocess
import sys
import urllib.request
import uuid

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

UPLOADS = {
    8080: r'\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data\wp-content\uploads',
    3001: os.path.join(HERE, '..', '..', 'wp-local', 'wp-content', 'uploads'),
}

FILES = {
    'rp-plain.svg': b'<?xml version="1.0" encoding="UTF-8"?>\n<svg xmlns="http://www.w3.org/2000/svg" width="120" height="60" viewBox="0 0 120 60">\n  <rect width="120" height="60" fill="#36c"/>\n</svg>\n',
    'rp-script.svg': b'<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><script>alert(2)</script><circle r="4"/></svg>',
    'rp-viewbox-only.svg': b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 20"><path d="M0 0"/></svg>',
    'rp-tall.svg': b'<svg xmlns="http://www.w3.org/2000/svg" width="40px" height="90px"><g/></svg>',
    'rp-not-svg.svg': b'<html><body>no</body></html>',
    'rp-broken.svg': b'<svg xmlns="http://www.w3.org/2000/svg"><g></svg>',
}


def sql(statement):
    return lc.sql(statement)


def multipart(fields, filename, data):
    boundary = uuid.uuid4().hex
    body = b''
    for key, value in fields.items():
        body += f'--{boundary}\r\nContent-Disposition: form-data; name="{key}"\r\n\r\n{value}\r\n'.encode()
    body += f'--{boundary}\r\nContent-Disposition: form-data; name="async-upload"; filename="{filename}"\r\nContent-Type: image/svg+xml\r\n\r\n'.encode() + data + b'\r\n'
    body += f'--{boundary}--\r\n'.encode()
    return body, f'multipart/form-data; boundary={boundary}'


def upload(client, port, filename, data):
    status, headers, page = client.request('/wp-admin/media-new.php')
    nonce = re.search(r'"_wpnonce":"([0-9a-f]{10})"', page).group(1)
    body, content_type = multipart({'name': filename, 'action': 'upload-attachment', '_wpnonce': nonce}, filename, data)
    request = urllib.request.Request(f'http://localhost:{port}/wp-admin/async-upload.php', data=body, headers={'Content-Type': content_type})
    try:
        response = client.op.open(request)
        return response.status, response.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()


def normalize(text, port):
    text = text.replace(f'localhost:{port}', 'HOST').replace(f'localhost\\/{port}', 'HOST')
    text = re.sub(r'"(id|uploadedTo|date|modified|author)":"?\d+"?', r'"\1":N', text)
    text = re.sub(r'[0-9a-f]{10}', 'NONCE', text)
    text = re.sub(r'post=\d+', 'post=N', text)
    text = re.sub(r'\?attachment_id=\d+', '?attachment_id=N', text)
    text = re.sub(r'"dateFormatted":"[^"]*"', '"dateFormatted":"D"', text)
    # Both sides write into the same database: the second upload gets the next free slug.
    text = re.sub(r'(rp-[a-z-]+?)-\d+(\\/|")', r'\1\2', text)
    return text


def main():
    user = sys.argv[sys.argv.index('--user') + 1] if '--user' in sys.argv else 'rpadmin'
    clients = {}
    for port in (8080, 3001):
        c = lc.Client(port)
        c.request('/wp-login.php')
        c.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
        clients[port] = c
    created = []
    differ = 0
    for name, data in FILES.items():
        results = {}
        for port in (8080, 3001):
            status, body = upload(clients[port], port, name, data)
            results[port] = (status, body)
            try:
                answer = json.loads(body)
            except ValueError:
                answer = {}
            if answer.get('success'):
                created.append(answer['data']['id'])
        a = (results[8080][0], normalize(results[8080][1], 8080))
        b = (results[3001][0], normalize(results[3001][1], 3001))
        if a == b:
            print(f'identical answer  {name}')
        else:
            differ += 1
            print(f'DIFFERENT answer  {name}\n  ref {a}\n  rp  {b}')
        ids = []
        for port in (8080, 3001):
            try:
                ids.append(json.loads(results[port][1])['data']['id'])
            except (ValueError, KeyError, TypeError):
                ids.append(None)
        if all(ids):
            meta = [sql(f"SELECT meta_value FROM wp_postmeta WHERE post_id={i} AND meta_key='_wp_attachment_metadata'") for i in ids]
            files = [sql(f"SELECT meta_value FROM wp_postmeta WHERE post_id={i} AND meta_key='_wp_attached_file'") for i in ids]
            if meta[0] != meta[1]:
                differ += 1
                print(f'  DIFFERENT metadata\n    ref {meta[0]}\n    rp  {meta[1]}')
            stored = []
            for port, relative in zip((8080, 3001), files):
                with open(os.path.join(UPLOADS[port], relative.replace('/', os.sep)), 'rb') as f:
                    stored.append(f.read())
            if stored[0] != stored[1]:
                differ += 1
                print(f'  DIFFERENT file\n    ref {stored[0]!r}\n    rp  {stored[1]!r}')
    if '--keep' not in sys.argv:
        for i in created:
            relative = sql(f"SELECT meta_value FROM wp_postmeta WHERE post_id={i} AND meta_key='_wp_attached_file'")
            for root in UPLOADS.values():
                path = os.path.join(root, relative.replace('/', os.sep))
                if relative and os.path.isfile(path):
                    try:
                        os.remove(path)
                    except OSError:
                        print(f'  could not delete {path}')
            sql(f'DELETE FROM wp_postmeta WHERE post_id={i}')
            sql(f'DELETE FROM wp_posts WHERE ID={i}')
    print(f'{differ} differences')


main()
