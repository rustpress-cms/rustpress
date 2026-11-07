#!/usr/bin/env python3
"""`themes.php?page=custom-header` of a classic theme on the copy of a real site (dev/up-soli.sh, :8081) and on RustPress
(dev/serve-soli.ps1, :3002): the same forms are sent to each side from the same theme mods, and the answers and the stored
theme mods are compared. Uploading and cropping are compared by what they store: the attachments, their meta and the
files (RustPress needs to write into the folder of the month of the copy's uploads). Everything is put back after each step.

    python dev/custom-header-compare.py [--show N] [substring-of-step]"""
import base64
import difflib
import html
import importlib.util
import json
import os
import re
import sys
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('wc', os.path.join(HERE, 'widgets-compare.py'))
wc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wc)
sys.stdout.reconfigure(encoding='utf-8')

PAGE = '/wp-admin/themes.php?page=custom-header'

READ = r'''<?php
global $wpdb;
echo base64_encode( (string) $wpdb->get_var( "SELECT option_value FROM $wpdb->options WHERE option_name = 'theme_mods_" . get_option( 'stylesheet' ) . "'" ) );
'''

WRITE = r'''<?php
global $wpdb;
$wpdb->update( $wpdb->options, array( 'option_value' => base64_decode( '%s' ) ), array( 'option_name' => 'theme_mods_' . get_option( 'stylesheet' ) ) );
wp_cache_flush();
'''

# `(name, fields of the form, whether the page shows a random header)`.
STEPS = [
    ('page', None, False),
    ('text color', {'display-header-text': 'on', 'text-color': '#123abc'}, False),
    ('short text color', {'display-header-text': 'on', 'text-color': 'f0a'}, False),
    ('invalid text color', {'display-header-text': 'on', 'text-color': 'zz'}, False),
    ('four digits', {'display-header-text': 'on', 'text-color': '#abcd'}, False),
    ('empty text color', {'display-header-text': 'on', 'text-color': ''}, False),
    ('hide the text', {'text-color': '#123456'}, False),
    ('default image', {'display-header-text': 'on', 'text-color': '', 'default-header': 'default-image'}, False),
    ('uploaded image', {'display-header-text': 'on', 'text-color': '#eeeeee', 'default-header': '70'}, False),
    ('random default', {'default-header': 'random-default-image'}, False),
    ('random uploaded', {'default-header': 'random-uploaded-image'}, True),
    ('unknown choice', {'default-header': 'nope'}, False),
    ('remove', {'removeheader': 'Remove Header Image', 'text-color': '#000000', 'display-header-text': 'on'}, False),
    ('reset', {'resetheader': 'Restore'}, False),
    ('empty form', {'x': '1'}, False),
    ('bad nonce', {'text-color': '#111111', 'bad': True}, False),
]


def norm(text, port, random):
    text = wc.norm(text, port)
    # A nonce in an address that `customize.php?return=` encodes.
    text = re.sub(r'(_wpnonce[-a-z]*%3D)[0-9a-f]{10}', r'\1NONCE', text)
    if random:
        # `array_rand()` picks the image of the preview.
        text = re.sub(r'background-image:url\([^)]*\);max-width:\d+px;height:\d+px;', 'background-image:url(RANDOM);', text)
    return text


def run(side, fields, random):
    if fields is None:
        status, _, body = side.client.request(PAGE)
        return [(status, norm(body, side.port, random))]
    _, _, page = side.client.request(PAGE)
    nonce = re.search(r'name="_wpnonce-custom-header-options" value="([0-9a-f]+)"', page)
    form = {k: v for k, v in fields.items() if k != 'bad'}
    form['_wpnonce-custom-header-options'] = 'bad' if fields.get('bad') else (nonce.group(1) if nonce else '')
    form['_wp_http_referer'] = '/wp-admin/themes.php?page=custom-header'
    status, _, body = side.client.request(PAGE + '&step=1', form)
    return [(status, norm(body, side.port, random))]


def image(width, height):
    """A JPEG with some noise, so that it has a size."""
    import io
    from PIL import Image
    buffer = io.BytesIO()
    Image.effect_noise((width, height), 30).convert('RGB').save(buffer, 'JPEG', quality=70)
    return buffer.getvalue()


def multipart(side, path, fields, files):
    boundary = 'rpheaderboundary'
    body = b''
    for name, value in fields.items():
        body += f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"\r\n\r\n{value}\r\n'.encode()
    for name, (filename, data) in files.items():
        body += f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"; filename="{filename}"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode() + data + b'\r\n'
    body += f'--{boundary}--\r\n'.encode()
    request = urllib.request.Request(f'http://localhost:{side.port}{path}', data=body, headers={'Content-Type': f'multipart/form-data; boundary={boundary}'})
    try:
        response = side.client.op.open(request)
        return response.status, response.read().decode('utf-8', 'replace')
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode('utf-8', 'replace')


def hidden(page, name):
    found = re.search(rf'name="{re.escape(name)}"(?: id="[^"]*")? value="([^"]*)"', page)
    return html.unescape(found.group(1)) if found else ''


def upload_flow(filename, data, finish):
    """Uploads a file in step 2 and sends the crop form of the answer to step 3: `finish` is `crop` or `skip`."""
    def flow(side, made):
        _, _, page = side.client.request(PAGE)
        nonce = hidden(page, '_wpnonce-custom-header-upload')
        fields = {'action': 'save', '_wpnonce-custom-header-upload': nonce, '_wp_http_referer': '/wp-admin/themes.php?page=custom-header', 'submit': 'Upload'}
        status, crop = multipart(side, PAGE + '&step=2', fields, {'import': (filename, data)})
        out = [(status, crop)]
        if 'name="oitar"' in crop and finish:
            action = html.unescape(re.search(r'<form method="post" action="([^"]+)"', crop).group(1))
            form = {name: hidden(crop, name) for name in ('x1', 'y1', 'width', 'height', 'attachment_id', 'oitar', '_wpnonce', '_wp_http_referer')}
            if finish == 'skip':
                form['skip-cropping'] = 'Skip'
            else:
                form['submit'] = 'Crop'
                # A part of the image, as imgAreaSelect sends it.
                form['x1'], form['y1'] = '10', '20'
                form['height'] = str(int(float(form['height']) * 0.5))
            status, body = side.client.request(action, form)[0:3:2]
            out.append((status, body))
        return out
    return flow


def library_flow(finish):
    """Chooses the seeded image from the media library and sends the crop form to step 3."""
    def flow(side, made):
        _, _, page = side.client.request(PAGE)
        link = html.unescape(re.search(r'data-update-link="([^"]+)"', page).group(1)).replace(f'http://localhost:{side.port}', '')
        status, _, crop = side.client.request(link + f'&file={made["library"]}')
        out = [(status, crop)]
        if 'name="oitar"' in crop and finish:
            action = html.unescape(re.search(r'<form method="post" action="([^"]+)"', crop).group(1))
            form = {name: hidden(crop, name) for name in ('x1', 'y1', 'width', 'height', 'attachment_id', 'oitar', '_wpnonce', '_wp_http_referer', 'create-new-attachment')}
            form['skip-cropping' if finish == 'skip' else 'submit'] = 'Go'
            status, body = side.client.request(action, form)[0:3:2]
            out.append((status, body))
        return out
    return flow


FLOWS = [
    ('upload a wide image, crop it', upload_flow('rp-wide.jpg', image(2400, 1500), 'crop')),
    ('upload a small image, skip', upload_flow('rp-small.jpg', image(1600, 900), 'skip')),
    ('upload a small image, crop it', upload_flow('rp-small.jpg', image(1600, 900), 'crop')),
    ('upload a text file', upload_flow('rp-notes.txt', b'just text', None)),
    ('library image, crop it', library_flow('crop')),
    ('library image, skip', library_flow('skip')),
]

SEED = r"""<?php
require_once ABSPATH . 'wp-admin/includes/image.php';
global $wpdb;
$made = array( 'before' => (int) $wpdb->get_var( "SELECT MAX(ID) FROM $wpdb->posts" ) );
$image = imagecreatetruecolor( 2200, 1400 );
ob_start();
imagejpeg( $image );
$file = wp_upload_bits( 'rp-library.jpg', null, ob_get_clean() );
$id   = wp_insert_attachment( array( 'post_title' => 'A library image', 'post_excerpt' => 'Its caption', 'post_mime_type' => 'image/jpeg', 'post_status' => 'inherit' ), $file['file'] );
wp_update_attachment_metadata( $id, wp_generate_attachment_metadata( $id, $file['file'] ) );
update_post_meta( $id, '_wp_attachment_image_alt', 'Library alt' );
$made['library'] = $id;
echo json_encode( $made );
"""

# The attachments made since the seed, their meta and the files of the folder.
SNAPSHOT = r"""<?php
global $wpdb;
$after = %d;
$rows  = array();
foreach ( $wpdb->get_results( $wpdb->prepare( "SELECT ID, post_title, post_content, post_excerpt, post_mime_type, post_parent, guid, post_status, comment_status, ping_status FROM $wpdb->posts WHERE ID > %%d ORDER BY ID", $after ), ARRAY_A ) as $row ) {
	$row['meta'] = $wpdb->get_results( $wpdb->prepare( "SELECT meta_key, meta_value FROM $wpdb->postmeta WHERE post_id = %%d AND meta_key NOT IN ( '_edit_lock', '_wp_old_slug' ) ORDER BY meta_key", $row['ID'] ), ARRAY_A );
	$rows[] = $row;
}
$dir   = wp_upload_dir();
$files = glob( $dir['path'] . '/rp-*' ) ?: array();
$files = array_merge( $files, glob( $dir['path'] . '/*-rp-*' ) ?: array() );
sort( $files );
echo json_encode( array( 'rows' => $rows, 'files' => array_map( 'basename', $files ) ) );
"""

# Removes what a flow made after the seed (`$keep` is the seeded image).
UNDO = r"""<?php
global $wpdb;
foreach ( $wpdb->get_col( $wpdb->prepare( "SELECT ID FROM $wpdb->posts WHERE ID > %%d AND ID <> %%d", %d, %d ) ) as $id ) {
	wp_delete_attachment( $id, true );
}
$dir = wp_upload_dir();
foreach ( array_merge( glob( $dir['path'] . '/rp-*' ) ?: array(), glob( $dir['path'] . '/*-rp-*' ) ?: array() ) as $file ) {
	if ( ! str_starts_with( basename( $file ), 'rp-library' ) ) {
		wp_delete_file( $file );
	}
}
"""

# Removes the seeded image as well.
UNSEED = r"""<?php
wp_delete_attachment( %d, true );
"""


def snapshot(made, port):
    data = json.loads(wc.php(SNAPSHOT % made['before']))
    ids = {}
    for row in data['rows']:
        ids[row['ID']] = f'ID{len(ids)}'
    text = json.dumps(data, sort_keys=True)
    # IDs differ between the sides; file sizes and the pixels of what was cropped as well.
    for real, label in sorted(ids.items(), key=lambda item: -len(item[0])):
        text = re.sub(rf'(?<![0-9]){real}(?![0-9])', label, text)
    text = re.sub(r's:8:\\"filesize\\";i:\d+;', 's:8:\\"filesize\\";i:N;', text)
    return text.replace('localhost:8081', 'localhost:PORT').replace('localhost:3002', 'localhost:PORT'), ids


def normalize_ids(text, ids):
    for real, label in sorted(ids.items(), key=lambda item: -len(item[0])):
        text = re.sub(rf'(?<![0-9]){real}(?![0-9])', label, text)
    return text


def compare_texts(label, a, b, show):
    if a == b:
        return True
    print(f'   {label} differs')
    x, y = a.split('\n'), b.split('\n')
    for n, (tag, i1, i2, j1, j2) in enumerate(op for op in difflib.SequenceMatcher(None, x, y, autojunk=False).get_opcodes() if op[0] != 'equal'):
        if n >= show:
            break
        print('  ', tag, i1, j1)
        left, right = '\n'.join(x[i1:i2]), '\n'.join(y[j1:j2])
        at = next((k for k, (p, q) in enumerate(zip(left, right)) if p != q), min(len(left), len(right)))
        print('   R', repr(left[max(0, at - 120):at + 200]))
        print('   O', repr(right[max(0, at - 120):at + 200]))
    return False


def main():
    args = sys.argv[1:]
    show = int(wc.option(args, '--show', '4'))
    only = args[0] if args else ''
    wc.php(wc.PREPARE)
    original = wc.php(READ).strip()
    made = json.loads(wc.php(SEED))
    bad = total = 0
    try:
        sides = wc.Side(8081), wc.Side(3002)
        for name, fields, random in STEPS:
            if only and only not in name:
                continue
            answers, mods = [], []
            for side in sides:
                wc.php(WRITE % original)
                answers.append(run(side, fields, random)[0])
                mods.append(base64.b64decode(wc.php(READ).strip()).decode('utf-8', 'replace').replace(f'localhost:{side.port}', 'localhost:PORT'))
            total += 2
            same_page = answers[0] == answers[1]
            same_mods = mods[0] == mods[1]
            bad += (not same_page) + (not same_mods)
            print(f"{'equal' if same_page and same_mods else 'DIFF '} {name:30} {answers[0][0]} {answers[1][0]} {len(answers[0][1]):7} {len(answers[1][1]):7} mods {'equal' if same_mods else 'DIFF'}", flush=True)
            if not same_mods:
                print('   R', mods[0][:700])
                print('   O', mods[1][:700])
            if not same_page:
                compare_texts('page', answers[0][1], answers[1][1], show)
        for name, flow in FLOWS:
            if only and only not in name:
                continue
            results = []
            for side in sides:
                wc.php(WRITE % original)
                pages = flow(side, made)
                state, ids = snapshot(made, side.port)
                mods = normalize_ids(base64.b64decode(wc.php(READ).strip()).decode('utf-8', 'replace').replace(f'localhost:{side.port}', 'localhost:PORT'), ids)
                pages = [(status, normalize_ids(norm(body, side.port, False), ids)) for status, body in pages]
                results.append((pages, state, mods))
                wc.php(UNDO % (made['before'], made['library']))
            (pa, sa, ma), (pb, sb, mb) = results
            same = [len(pa) == len(pb)] + [x == y for x, y in zip(pa, pb)] + [sa == sb, ma == mb]
            total += len(same)
            bad += same.count(False)
            print(f"{'equal' if all(same) else 'DIFF '} {name:30} {[p[0] for p in pa]} {[p[0] for p in pb]} {[len(p[1]) for p in pa]} {[len(p[1]) for p in pb]}", flush=True)
            for n, (x, y) in enumerate(zip(pa, pb)):
                if x[0] != y[0]:
                    print(f'   status of answer {n + 1}: {x[0]} {y[0]}')
                compare_texts(f'answer {n + 1}', x[1], y[1], show)
            compare_texts('attachments', sa.replace('}, {', '},\n{'), sb.replace('}, {', '},\n{'), show)
            compare_texts('mods', ma.replace(';s:', ';\ns:'), mb.replace(';s:', ';\ns:'), show)
    finally:
        wc.php(UNDO % (made['before'], made['library']))
        wc.php(UNSEED % made['library'])
        wc.php(WRITE % original)
        wc.php(wc.RESET)
        wc.php(wc.CLEANUP)
    print(f'{total - bad} of {total} equal')


if __name__ == '__main__':
    main()
