#!/usr/bin/env python3
"""`wp-admin/media-upload.php` on the reference (:8080) and RustPress (:3001), the whole page as rpadmin (or rpeditor,
rpauthor): the four tabs and the forms they post. Nonces, ports and times are neutralized.

    python dev/media-upload-compare.py [--show N] [--only <part of a scenario name>]

Every scenario counts what it is meant to reach in both pages, so that an empty page does not pass as equal."""
import difflib
import importlib.util
import os
import re
import struct
import sys
import urllib.error
import urllib.request
import uuid
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

PORTS = (8080, 3001)


def option(args, name, default):
    if name in args:
        i = args.index(name)
        value = args[i + 1]
        del args[i:i + 2]
        return value
    return default


def norm(text, port):
    text = text.replace(f'localhost:{port}', 'localhost:PORT').replace(f'127.0.0.1:{port}', 'localhost:PORT')
    text = re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', text)
    text = re.sub(r'"time":"\d+"', '"time":"T"', text)
    # The sizes the servers accept are their configuration.
    text = re.sub(r'"max_file_size":"\d+b"', '"max_file_size":"Nb"', text)
    text = re.sub(r'Maximum upload file size: [\d.]+ [A-Z]+\.', 'Maximum upload file size: N.', text)
    # What an upload creates: the row and, as both sides fill the same folder and table, the name.
    text = re.sub(r'rp-upload((?:-[Aa]b)?)(-\d+)?(?![\dx])', r'rp-upload\1', text)
    if NEW_IDS:
        text = re.sub(r'(?<![0-9])(' + '|'.join(str(i) for i in NEW_IDS) + r')(?![0-9])', 'NEW', text)
    return text


# `--only`.
ONLY = ''
# The rows of what an upload created, for `norm()`.
NEW_IDS = set()
# Headers that say what a page is; the others depend on the server software.
HEADERS = ('content-type', 'cache-control', 'expires', 'x-frame-options', 'referrer-policy', 'x-robots-tag')


def png(width, height):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    rows = b''.join(b'\0' + b'\x20\x60\xa0' * width for _ in range(height))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b'')


def multipart(fields, filename, data):
    boundary = uuid.uuid4().hex
    body = b''
    for key, value in fields.items():
        body += f'--{boundary}\r\nContent-Disposition: form-data; name="{key}"\r\n\r\n{value}\r\n'.encode()
    body += f'--{boundary}\r\nContent-Disposition: form-data; name="async-upload"; filename="{filename}"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode() + data + b'\r\n'
    body += f'--{boundary}--\r\n'.encode()
    return body, f'multipart/form-data; boundary={boundary}'


class Side:
    def __init__(self, port, user):
        self.port = port
        self.client = lc.Client(port)
        self.client.request('/wp-login.php')
        self.client.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})

    def get(self, page):
        return self.client.request('/wp-admin/' + page)

    def post(self, page, form, nonce_from=None):
        """Posts `form`, with the `media-form` nonce of the page `nonce_from`."""
        if nonce_from is not None:
            _, _, body = self.get(nonce_from)
            found = re.search(r'name="_wpnonce" value="([0-9a-f]{10})"', body)
            form = dict(form, _wpnonce=found.group(1) if found else '')
        return self.client.request('/wp-admin/' + page, form)

    def upload(self, page, form, filename, data, nonce_from):
        _, _, body = self.get(nonce_from)
        form = dict(form, _wpnonce=re.search(r'name="_wpnonce" value="([0-9a-f]{10})"', body).group(1))
        payload, content_type = multipart(form, filename, data)
        request = urllib.request.Request(f'http://localhost:{self.port}/wp-admin/{page}', data=payload, headers={'Content-Type': content_type})
        try:
            response = self.client.op.open(request)
            return response.status, dict(response.headers.items()), response.read().decode()
        except urllib.error.HTTPError as e:
            return e.code, dict(e.headers.items()), e.read().decode()


DOCUMENTS = 'application/msword,application/vnd.openxmlformats-officedocument.wordprocessingml.document,application/vnd.ms-word.document.macroEnabled.12,application/vnd.ms-word.template.macroEnabled.12,application/vnd.oasis.opendocument.text,application/vnd.apple.pages,application/pdf,application/vnd.ms-xpsdocument,application/oxps,application/rtf,application/wordperfect,application/octet-stream'
ARCHIVES = 'application/x-gzip,application/rar,application/x-tar,application/zip,application/x-7z-compressed'
GETS = [
    # name, address, what the page must contain
    ('computer', 'media-upload.php', 'plupload-upload-ui'),
    ('computer image', 'media-upload.php?post_id=1&type=image', 'plupload-upload-ui'),
    ('computer chromeless', 'media-upload.php?chromeless=1&type=image', 'plupload-upload-ui'),
    ('computer inline', 'media-upload.php?inline=1', 'plupload-upload-ui'),
    ('computer missing post', 'media-upload.php?post_id=999999', 'allowed to edit this item'),
    ('url image', 'media-upload.php?type=image&tab=type_url&post_id=1', 'addExtImage'),
    ('url video', 'media-upload.php?type=video&tab=type_url', 'not-image'),
    ('url audio', 'media-upload.php?type=audio&tab=type_url&post_id=1', 'Insert media'),
    ('url file', 'media-upload.php?type=file&tab=type_url', 'Insert media'),
    ('url unknown', 'media-upload.php?type=x&tab=type_url', ''),
    ('edit without item', 'media-upload.php?action=edit', 'upload process'),
    ('post id text', 'media-upload.php?post_id=abc', 'allowed to edit this item'),
    ('post id zero', 'media-upload.php?post_id=0', 'plupload-upload-ui'),
    ('post id trailing', 'media-upload.php?post_id=1abc&tab=gallery', 'allowed to edit this item'),
    ('post id spaces', 'media-upload.php?post_id=%201%20&tab=gallery', 'media-item'),
    ('page', 'media-upload.php?post_id=2&tab=library&type=image', 'media-item'),
    ('unknown type', 'media-upload.php?type=x', ''),
    ('unknown tab', 'media-upload.php?tab=x', ''),
    ('gallery', 'media-upload.php?post_id=1&tab=gallery', 'media-item'),
    ('gallery type', 'media-upload.php?post_id=1&tab=gallery&type=image', 'media-item'),
    ('gallery no post', 'media-upload.php?tab=gallery', ''),
    ('library', 'media-upload.php?tab=library', 'media-item'),
    ('library post', 'media-upload.php?post_id=1&tab=library&type=image', 'media-item'),
    ('library video', 'media-upload.php?post_id=1&tab=library&type=video', 'media-item'),
    ('library all', 'media-upload.php?tab=library&post_mime_type=all', 'media-item'),
    ('library images', 'media-upload.php?tab=library&post_mime_type=image', 'media-item'),
    ('library search', 'media-upload.php?tab=library&s=e', 'media-item'),
    ('library none', 'media-upload.php?tab=library&s=zzzzzz', ''),
    ('library quote', "media-upload.php?tab=library&s=a%27b%22c", ''),
    ('tab and type slashes', 'media-upload.php?tab=library&post_mime_type=a%22b&context=%27', 'media-item'),
    ('library args', 'media-upload.php?tab=library&type=audio&context=x%22y&m=0&paged=1', 'media-item'),
    ('library unknown type', 'media-upload.php?tab=library&post_mime_type=nothing', 'media-item'),
    ('library mine', 'media-upload.php?tab=library&mine=1', ''),
    ('library detached', 'media-upload.php?tab=library&detached=1', ''),
    ('library page 2', 'media-upload.php?tab=library&paged=2', 'media-item'),
    ('library month', 'media-upload.php?tab=library&m=202610', 'media-item'),
]


def compare(name, page, wanted, responses, show):
    """Returns True if both pages are equal and carry `wanted`."""
    (status_a, _, body_a), (status_b, _, body_b) = responses
    a, b = norm(body_a, PORTS[0]), norm(body_b, PORTS[1])
    counts = (a.count(wanted), b.count(wanted)) if wanted else ('-', '-')
    def headers(response):
        return {k.lower(): v for k, v in response[1].items() if k.lower() in HEADERS}
    head_a, head_b = headers(responses[0]), headers(responses[1])
    same = status_a == status_b and a == b and head_a == head_b
    vacuous = bool(wanted) and counts[0] == 0
    mark = 'equal' if same else 'DIFF '
    print(f'{mark} {name:28} {status_a} {status_b} {len(a):7} {len(b):7} {wanted!r}: {counts[0]}/{counts[1]}' + (' EMPTY' if vacuous else ''))
    if head_a != head_b:
        print('    headers', head_a, head_b)
    if not same and show:
        lines = list(difflib.unified_diff(a.split('\n'), b.split('\n'), lineterm='', n=0))[2:]
        for index, line in enumerate(lines[:show]):
            # Long lines from where they part.
            other = lines[index + 1] if line.startswith('-') and index + 1 < len(lines) else lines[index - 1] if line.startswith('+') and index else ''
            at = next((i for i, (x, y) in enumerate(zip(line[1:], other[1:])) if x != y), 0)
            print('   ', line[0], line[1:][max(0, at - 80):at + 220])
        if len(lines) > show:
            print(f'    ... {len(lines) - show} more')
    return same and not vacuous


SEEDED = [
    # name, address, what the page must contain
    ('seeded library', 'media-upload.php?tab=library', 'tablenav-pages'),
    ('seeded page 2', 'media-upload.php?tab=library&paged=2', 'prev page-numbers'),
    ('seeded page 3', 'media-upload.php?tab=library&paged=3', 'tablenav-pages'),
    ('seeded page 4', 'media-upload.php?tab=library&paged=4', 'media-item'),
    ('seeded page 9', 'media-upload.php?tab=library&paged=9', ''),
    ('seeded image', 'media-upload.php?tab=library&type=image', 'media-item'),
    ('seeded image page 2', 'media-upload.php?tab=library&type=image&paged=2', 'media-item'),
    ('seeded documents', 'media-upload.php?tab=library&post_id=2&post_mime_type=' + DOCUMENTS.replace(',', '%2C'), 'Documents'),
    ('seeded archives', 'media-upload.php?tab=library&post_mime_type=' + ARCHIVES, 'application/zip'),
    ('seeded gallery', 'media-upload.php?post_id=2&tab=gallery', 'menu_order_input'),
    ('seeded search', 'media-upload.php?tab=library&s=rp-mu', 'tablenav-pages'),
    ('seeded two terms', 'media-upload.php?tab=library&s=lang+rp-mu', 'media-item'),
    ('seeded file name', 'media-upload.php?tab=library&s=mp4', 'media-item'),
    ('seeded relevance list', 'upload.php?mode=list&s=rp-mu', 'rp-mu'),
    ('seeded month', 'media-upload.php?tab=library&m=201503', 'media-item'),
    ('seeded attachment', 'media-upload.php?post_id={attachment}', 'plupload-upload-ui'),
]


def seed():
    """Attachments of every kind on page 2, beside the content of the reference."""
    rows = []
    mimes = [('image/png', 'png'), ('application/pdf', 'pdf'), ('application/zip', 'zip'), ('text/plain', 'txt'), ('application/vnd.ms-excel', 'xls'), ('audio/mpeg', 'mp3'), ('video/mp4', 'mp4')]
    long_title = 'rp-mu ' + 'lang &amp; ' * 8
    for i in range(30):
        mime, extension = mimes[i % len(mimes)]
        title = ['rp-mu <b>fett</b> "q"', '', long_title, f'rp-mu {i}'][i % 4]
        date = f'2015-{i % 3 + 1:02}-{i % 28 + 1:02} 10:{i:02}:00'
        title = title.replace("'", "''")
        rows.append(f"(3, '{date}', '{date}', '', '{title}', '', 'inherit', 'open', 'closed', '', 'rp-mu-{i}-{extension}', '', '', '{date}', '{date}', '', 2, 'http://localhost/rp-mu-{i}', {i % 4}, 'attachment', '{mime}', 0)")
    lc.sql('INSERT INTO wp_posts (post_author, post_date, post_date_gmt, post_content, post_title, post_excerpt, post_status, comment_status, ping_status, post_password, post_name, to_ping, pinged, post_modified, post_modified_gmt, post_content_filtered, post_parent, guid, menu_order, post_type, post_mime_type, comment_count) VALUES ' + ', '.join(rows))
    lc.sql("INSERT INTO wp_postmeta (post_id, meta_key, meta_value) SELECT ID, '_wp_attached_file', CONCAT('2015/03/', REPLACE(post_name, CONCAT('-', SUBSTRING_INDEX(post_name, '-', -1)), ''), '.', SUBSTRING_INDEX(post_name, '-', -1)) FROM wp_posts WHERE post_name LIKE 'rp-mu-%' AND post_type = 'attachment' AND ID % 3 > 0")


def unseed():
    lc.sql("DELETE m FROM wp_postmeta m JOIN wp_posts p ON p.ID = m.post_id WHERE p.post_name LIKE 'rp-mu-%' AND p.post_type = 'attachment'")
    lc.sql("DELETE FROM wp_posts WHERE post_name LIKE 'rp-mu-%' AND post_type = 'attachment'")


def run_seeded(sides, show):
    failed = 0
    if not any(ONLY in name for name, _, _ in SEEDED):
        return 0, 0
    seed()
    try:
        attachment_id = lc.sql("SELECT MIN(ID) FROM wp_posts WHERE post_name LIKE 'rp-mu-%' AND post_type = 'attachment'").split()[-1]
        for name, page, wanted in SEEDED:
            if ONLY not in name:
                continue
            page = page.replace('{attachment}', attachment_id)
            if not compare(name, page, wanted, [s.get(page) for s in sides], show):
                failed += 1
    finally:
        unseed()
    return sum(ONLY in name for name, _, _ in SEEDED), failed


# The rows the forms change, and what `snapshot()` compares.
EDITED = (26, 281, 299)


def backup():
    ids = ','.join(str(i) for i in EDITED)
    lc.sql('DROP TABLE IF EXISTS rp_mu_posts, rp_mu_meta')
    lc.sql(f'CREATE TABLE rp_mu_posts AS SELECT * FROM wp_posts WHERE ID IN ({ids})')
    lc.sql(f'CREATE TABLE rp_mu_meta AS SELECT * FROM wp_postmeta WHERE post_id IN ({ids})')


def restore():
    ids = ','.join(str(i) for i in EDITED)
    lc.sql('REPLACE INTO wp_posts SELECT * FROM rp_mu_posts')
    lc.sql(f'DELETE FROM wp_postmeta WHERE post_id IN ({ids})')
    lc.sql('INSERT INTO wp_postmeta SELECT * FROM rp_mu_meta')
    lc.sql(f'DELETE FROM wp_posts WHERE post_type="revision" AND post_parent IN ({ids})')


def drop_backup():
    lc.sql('DROP TABLE IF EXISTS rp_mu_posts, rp_mu_meta')


def snapshot():
    ids = ','.join(str(i) for i in EDITED)
    row = lc.sql(f'SELECT ID, post_author, post_title, post_excerpt, post_content, post_name, post_status, post_parent, menu_order, post_mime_type FROM wp_posts WHERE ID IN ({ids}) ORDER BY ID')
    alt = lc.sql(f"SELECT post_id, meta_key, meta_value FROM wp_postmeta WHERE post_id IN ({ids}) AND meta_key IN ('_wp_attachment_image_alt', '_edit_last') ORDER BY post_id, meta_key")
    return f'{row}\n{alt}'


def attachment(id_, **fields):
    return {f'attachments[{id_}][{k}]': v for k, v in fields.items()}


TITLE = 'Neuer Titel <b>x</b> & "q"'
TEXT = {'post_title': TITLE, 'post_excerpt': 'Zeile eins\r\nZeile <em>zwei</em>', 'post_content': 'Beschreibung & <a href="x">l</a>', 'menu_order': '7'}
INSERT = 'Insert into Post'
BASE = 'media-upload.php?post_id=1&type=image'
POSTS = [
    # name, address to post to, form, what the answer must contain
    ('save library', 'media-upload.php?post_id=1&tab=library&type=image', {'save': 'x', **attachment(26, **TEXT, image_alt='Alt <b>t</b>  zwei')}, 'media-item'),
    ('save library no send', 'media-upload.php?tab=library', attachment(26, post_title=TITLE), 'media-item'),
    ('save gallery', 'media-upload.php?post_id=1&tab=gallery', {**attachment(26, **TEXT), 'post_id': '1', 'type': 'image', 'tab': 'gallery'}, 'media-item'),
    ('save button', BASE, {'save': 'Save all changes', 'post_id': '1', **attachment(26, post_title=TITLE)}, 'gallery-form'),
    ('insert gallery', 'media-upload.php?post_id=1&tab=gallery', {'insert-gallery': 'x', **attachment(26, post_title=TITLE)}, 'tb_remove'),
    ('update gallery', 'media-upload.php?post_id=1&tab=gallery', {'update-gallery': 'x', **attachment(26, menu_order='3')}, 'tb_remove'),
    ('send image', BASE, {'send[26]': INSERT, **attachment(26, **TEXT, image_alt='Alt', url='http://example.com/a?x=1&y=2', align='left', **{'image-size': 'medium'})}, 'send_to_editor'),
    ('send image no caption', BASE, {'send[26]': INSERT, **attachment(26, post_title='T', post_excerpt='', url='', align='none', **{'image-size': 'full'})}, 'wp-image-26'),
    ('send image defaults', BASE, {'send[26]': INSERT, **attachment(26, post_title='T', post_excerpt='', url='')}, 'wp-image-26'),
    ('send image attachment url', BASE, {'send[26]': INSERT, **attachment(26, post_title='T', post_excerpt='', url='http://localhost:PORT/?attachment_id=26')}, 'rel='),
    ('send file', 'media-upload.php?post_id=1&type=video', {'send[281]': INSERT, **attachment(281, post_title='Film', url='http://example.com/film.mp4')}, 'send_to_editor'),
    ('send file no url', 'media-upload.php?post_id=1&type=video', {'send[281]': INSERT, **attachment(281, post_title='Film <i>x</i>', url='')}, 'send_to_editor'),
    ('send sets parent', BASE, {'send[299]': INSERT, **attachment(299, post_title='T', post_excerpt='', url='', post_parent='1')}, 'wp-image-299'),
    ('url image', 'media-upload.php?type=image&tab=type&post_id=1', {'insertonlybutton': 'x', 'src': 'example.com/a b.png', 'alt': 'A "b"', 'align': 'left', 'media_type': 'image'}, 'send_to_editor'),
    ('url image no align', 'media-upload.php?type=image&tab=type&post_id=1', {'insertonlybutton': 'x', 'src': 'http://example.com/a.png', 'alt': ''}, 'send_to_editor'),
    ('url file', 'media-upload.php?type=file&tab=type&post_id=1', {'insertonlybutton': 'x', 'src': 'http://example.com/a.pdf', 'title': 'PDF <b>', 'media_type': 'generic'}, 'send_to_editor'),
    ('url file no title', 'media-upload.php?type=file&tab=type', {'insertonlybutton': 'x', 'src': 'http://example.com/dir/a%20b.mp3', 'title': '', 'media_type': 'generic'}, 'send_to_editor'),
]


def run_posts(sides, show):
    failed = total = 0
    if ONLY and 'post' not in ONLY:
        return 0, 0
    for name, page, form, wanted in POSTS:
        if ONLY not in 'post ' + name:
            continue
        total += 1
        responses, states = [], []
        for side in sides:
            filled = {k: v.replace('localhost:PORT', f'localhost:{side.port}') for k, v in form.items()}
            backup()
            responses.append(side.post(page, filled, 'media-upload.php?post_id=1'))
            states.append(snapshot())
            restore()
        drop_backup()
        if not compare('post ' + name, page, wanted, responses, show):
            failed += 1
        if states[0] != states[1]:
            failed += 1
            print('    ROWS DIFFER')
            for line in list(difflib.unified_diff(states[0].split('\n'), states[1].split('\n'), lineterm='', n=0))[2:12]:
                print('   ', line[:300])
    # An expired nonce.
    total += 1
    responses = [side.client.request('/wp-admin/media-upload.php?post_id=1', {'_wpnonce': '0000000000', 'send[26]': INSERT}) for side in sides]
    if not compare('post old nonce', '', 'expired', responses, show):
        failed += 1
    # Uploads: the attachment, then a file nobody may upload.
    for name, filename, data, wanted in (('upload', 'rp-upload.png', png(400, 300), 'media-item'), ('upload named', 'rp upload Äb.png', png(20, 20), 'media-item'), ('upload refused', 'rp-upload.exe', b'MZ', 'media-upload-error')):
        total += 1
        before = lc.sql("SELECT COALESCE(MAX(ID), 0) FROM wp_posts").split()[-1]
        responses = [side.upload('media-upload.php?type=image&tab=type&post_id=1', {'html-upload': 'Upload', 'post_id': '1'}, filename, data, 'media-upload.php?post_id=1') for side in sides]
        created = [int(i) for i in lc.sql(f"SELECT ID FROM wp_posts WHERE ID > {before} AND post_type = 'attachment'").split() if i.isdigit()]
        NEW_IDS.update(created)
        if not compare('post ' + name, '', wanted, responses, show):
            failed += 1
        if name != 'upload refused' and len(created) != 2:
            failed += 1
            print('    CREATED', created)
        for created_id in created:
            side = sides[0]
            status, _, body = side.get(f'post.php?post={created_id}&action=edit')
            nonce = re.search(r'post\.php\?post=' + str(created_id) + r'&(?:amp;|#038;)action=delete&(?:amp;|#038;)_wpnonce=([0-9a-f]{10})', body)
            if nonce:
                side.get(f'post.php?post={created_id}&action=delete&_wpnonce={nonce.group(1)}')
            else:
                print('    NOT REMOVED', created_id)
        NEW_IDS.clear()
    # What the delete on the reference left in the folder of RustPress.
    for folder, _, files in os.walk(os.path.join(HERE, '..', '..', 'wp-local', 'wp-content', 'uploads')):
        for file in files:
            if file.startswith('rp-upload'):
                os.remove(os.path.join(folder, file))
    return total, failed


def main():
    args = sys.argv[1:]
    show = int(option(args, '--show', '8'))
    global ONLY
    ONLY = only = option(args, '--only', '')
    users = option(args, '--users', 'rpadmin,rpeditor,rpauthor,rpcontributor,rpsubscriber').split(',')
    failed = 0
    total = 0
    for user in users:
        print(f'--- {user}')
        sides = [Side(port, user) for port in PORTS]
        for name, page, wanted in GETS:
            if only not in name:
                continue
            total += 1
            # Lesser users do not see every page; the status and the page are still compared.
            if not compare(name, page, wanted if user == 'rpadmin' else '', [s.get(page) for s in sides], show):
                failed += 1
        if user == 'rpadmin' and 'nopost' not in args:
            for run in (run_seeded, run_posts):
                done, bad = run(sides, show)
                total += done
                failed += bad
    print(f'{total - failed} of {total} equal')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
