#!/usr/bin/env python3
"""The website in the preview of the Customizer: pages as the frame of `customize.php` loads them
(`?customize_changeset_uuid=...&customize_theme=...&customize_messenger_channel=preview-0`), on the copy of a real site
with Twenty Seventeen (dev/up-soli.sh, :8081, against RustPress of dev/serve-soli.ps1, :3002) and on the reference with
its block theme (dev/up.sh, :8080, against dev/serve-admin.ps1, :3001). Compared whole and raw with the headers of the
preview, as an administrator, without the frame, as an editor and as a visitor (who may not look at a changeset that
does not exist), with a UUID that is none, and with a static front page and its panels. The nonces are masked.

    python dev/customize-preview-compare.py [--show N] [--only <part of a case>]"""
import difflib
import importlib.util
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('cc', os.path.join(HERE, 'customize-compare.py'))
cc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cc)
wc = cc.wc

CLASSIC, BLOCK = (8081, 3002), (8080, 3001)
UUID = '0f8c4a1e-2b3d-4c5e-9f60-718293a4b5c6'
FRAME = f'customize_changeset_uuid={UUID}&customize_theme=THEME&customize_messenger_channel=preview-0'
CASES = [
    # (name, user, language, address, ports)
    ('home', 'rpwidget', None, f'/?{FRAME}', CLASSIC),
    ('home en', 'rpwidget', '', f'/?{FRAME}', CLASSIC),
    ('page 2', 'rpwidget', None, f'/?paged=2&{FRAME}', CLASSIC),
    ('post', 'rpwidget', None, f'/?p=452&{FRAME}', CLASSIC),
    ('page', 'rpwidget', None, f'/?page_id=24&{FRAME}', CLASSIC),
    ('category', 'rpwidget', None, f'/?cat=1&{FRAME}', CLASSIC),
    ('search', 'rpwidget', None, f'/?s=cuba&{FRAME}', CLASSIC),
    ('not found', 'rpwidget', None, f'/?p=999999&{FRAME}', CLASSIC),
    ('static front page', 'rpwidget', None, f'/?{FRAME}', CLASSIC),
    ('frameless', 'rpwidget', None, f'/?customize_changeset_uuid={UUID}', CLASSIC),
    ('frameless post', 'rpwidget', None, f'/?p=452&customize_changeset_uuid={UUID}', CLASSIC),
    ('autosaved', 'rpwidget', None, f'/?{FRAME}&customize_autosaved=on', CLASSIC),
    ('editor', 'rpwidgeted', None, f'/?{FRAME}', CLASSIC),
    ('visitor', None, None, f'/?customize_changeset_uuid={UUID}', CLASSIC),
    ('bad uuid', 'rpwidget', None, '/?customize_changeset_uuid=nope&customize_messenger_channel=preview-0', CLASSIC),
    ('block home', 'rpadmin', None, f'/?{FRAME}', BLOCK),
    ('block post', 'rpadmin', None, f'/?p=1&{FRAME}', BLOCK),
    ('block page', 'rpadmin', None, f'/?page_id=2&{FRAME}', BLOCK),
    ('block frameless', 'rpadmin', None, f'/?customize_changeset_uuid={UUID}', BLOCK),
    ('block editor', 'rpeditor', None, f'/?{FRAME}', BLOCK),
    ('block visitor', None, None, f'/?customize_changeset_uuid={UUID}', BLOCK),
]

# The changes the Customizer has not written to the changeset yet: it posts them to the frame (`customized`).
POSTS = [
    # (name, user, values, address, ports, with a valid nonce)
    ('posted title', 'rpwidget', {'blogname': 'Neuer <b>Name</b> & Co', 'blogdescription': 'Unterzeile'}, f'/?{FRAME}', CLASSIC, True),
    ('posted colors', 'rpwidget', {'colorscheme': 'custom', 'colorscheme_hue': '120', 'page_layout': 'one-column', 'header_textcolor': 'blank'}, f'/?page_id=24&{FRAME}', CLASSIC, True),
    ('posted bad colors', 'rpwidget', {'colorscheme': 'pink', 'colorscheme_hue': '-40', 'page_layout': 'wide', 'header_textcolor': 'zz'}, f'/?{FRAME}', CLASSIC, True),
    ('posted front page', 'rpwidget', {'show_on_front': 'page', 'page_on_front': '24', 'panel_1': '16', 'panel_2': 51}, f'/?{FRAME}', CLASSIC, True),
    ('posted css', 'rpwidget', {'custom_css[twentyseventeen]': 'body { color: red; }'}, f'/?{FRAME}', CLASSIC, True),
    ('posted widgets', 'rpwidget', {'sidebars_widgets[sidebar-1]': ['search-2', 'meta-2'], 'widget_search[2]': {'encoded_serialized_instance': '', 'title': 'Finden', 'is_widget_customizer_js_value': True, 'instance_hash_key': '', 'raw_instance': {'title': 'Finden'}}}, f'/?{FRAME}', CLASSIC, True),
    ('posted menu location', 'rpwidget', {'nav_menu_locations[top]': 0}, f'/?{FRAME}', CLASSIC, True),
    ('posted without nonce', 'rpwidget', {'blogname': 'Nicht übernommen'}, f'/?{FRAME}', CLASSIC, False),
    ('block posted', 'rpadmin', {'blogname': 'Block <i>Titel</i>', 'custom_css[twentytwentyfive]': 'p { margin: 0; }'}, f'/?{FRAME}', BLOCK, True),
]

# Selective refresh: the partials the preview asks for, posted to `url.self`. `MENUS` stands for the menus of the page
# with the arguments it exported, and one with a hash that does not fit.
WIDGET = {'sidebar_id': 'sidebar-1', 'sidebar_instance_number': 1}
PARTIALS = [
    # (name, user, partials, values, address, ports)
    ('partials title', 'rpwidget', {'blogname': [{}], 'blogdescription': [], 'custom_logo': [{}], 'custom_header': [{}], 'nope': [{}]}, {'blogname': 'A & <b>B</b>'}, '/', CLASSIC),
    ('partials panels', 'rpwidget', {'panel_1': [{}], 'panel_2': [{}], 'panel_3': [{}]}, {'show_on_front': 'page', 'page_on_front': '24', 'panel_1': '16', 'panel_2': '999999'}, '/', CLASSIC),
    ('partials widgets', 'rpwidget', {'widget[search-2]': [WIDGET], 'widget[pages-2]': [{'sidebar_id': 'sidebar-2', 'sidebar_instance_number': 1}], 'widget[meta-2]': [{'sidebar_id': 'sidebar-9'}], 'widget[nope-9]': [WIDGET]},
     {'widget_search[2]': {'encoded_serialized_instance': '', 'title': 'Finden', 'is_widget_customizer_js_value': True, 'instance_hash_key': '', 'raw_instance': {'title': 'Finden'}}}, '/?p=452', CLASSIC),
    ('partials menus', 'rpwidget', 'MENUS', {}, '/', CLASSIC),
    ('partials missing', 'rpwidget', None, {}, '/', CLASSIC),
    ('block partials', 'rpadmin', {'blogname': [{}], 'custom_logo': [{}]}, {'blogname': 'X'}, '/', BLOCK),
]

# The front page with the page of the menu, two panels with a page, one with a page that is gone, one empty.
FRONT = r'''<?php
update_option( 'rp_preview_saved', wp_json_encode( array( get_option( 'show_on_front' ), get_option( 'page_on_front' ), get_theme_mods() ) ), false );
update_option( 'show_on_front', 'page' );
update_option( 'page_on_front', 24 );
set_theme_mod( 'panel_1', 16 );
set_theme_mod( 'panel_2', 999999 );
set_theme_mod( 'panel_3', 51 );
remove_theme_mod( 'panel_4' );
'''
FRONT_RESET = r'''<?php
$saved = json_decode( get_option( 'rp_preview_saved' ), true );
if ( $saved ) {
	update_option( 'show_on_front', $saved[0] );
	update_option( 'page_on_front', $saved[1] );
	update_option( 'theme_mods_' . get_option( 'stylesheet' ), $saved[2] );
}
delete_option( 'rp_preview_saved' );
'''

HEADERS = ('Cache-Control', 'Expires', 'X-Robots', 'X-Robots-Tag', 'X-Frame-Options', 'Content-Security-Policy', 'Content-Type')


def norm(text, ports):
    for port in ports:
        text = text.replace(f'localhost:{port}', 'localhost:PORT')
    return cc.norm(text, ports[0])


def fetch(side, address, port, theme):
    status, headers, body = side.client.request(address.replace('THEME', theme))
    lower = {k.lower(): v for k, v in headers.items()}
    head = ''.join(f'{name}: {lower.get(name.lower(), "")}\n' for name in HEADERS)
    return status, head + '\n' + body


def preview_nonce(side):
    """`_wpCustomizeSettings.nonce.preview` of the user's `customize.php`."""
    _, _, body = side.client.request('/wp-admin/customize.php')
    found = re.search(r'"preview":"([0-9a-f]{10})"', body)
    return found.group(1) if found else ''


def post(side, address, theme, values, valid):
    """The form `wp.customize.PreviewFrame` submits to the frame."""
    form = {
        '_method': 'GET',
        'wp_customize': 'on',
        'customize_theme': theme,
        'nonce': preview_nonce(side) if valid else '0123456789',
        'customize_changeset_uuid': UUID,
        'customized': json.dumps(values),
        'customize_messenger_channel': 'preview-0',
    }
    status, headers, body = side.client.request(address.replace('THEME', theme), form)
    lower = {k.lower(): v for k, v in headers.items()}
    return status, ''.join(f'{name}: {lower.get(name.lower(), "")}\n' for name in HEADERS) + '\n' + body


def menu_partials(side, theme):
    """The menus the preview of the front page exported, and one of them with its hash changed."""
    _, _, body = side.client.request(f'/?{FRAME}'.replace('THEME', theme))
    found = re.search(r'var _wpCustomizePreviewNavMenusExports = (.*);', body)
    instances = json.loads(found.group(1))['navMenuInstanceArgs'] if found else {}
    partials = {f'nav_menu_instance[{hmac}]': [args] for hmac, args in (instances.items() if isinstance(instances, dict) else [])}
    for hmac, args in list(instances.items() if isinstance(instances, dict) else [])[:1]:
        partials['nav_menu_instance[' + 'f' * 32 + ']'] = [dict(args, args_hmac='f' * 32)]
    return partials


def render_partials(side, address, theme, partials, values):
    form = {'wp_customize': 'on', 'nonce': preview_nonce(side), 'customize_theme': theme, 'customized': json.dumps(values), 'customize_changeset_uuid': UUID, 'wp_customize_render_partials': '1'}
    if partials is not None:
        form['partials'] = json.dumps(menu_partials(side, theme) if partials == 'MENUS' else partials)
    status, headers, body = side.client.request(address, form)
    lower = {k.lower(): v for k, v in headers.items()}
    return status, ''.join(f'{name}: {lower.get(name.lower(), "")}\n' for name in HEADERS) + '\n' + body


def report(name, pages, show):
    (sa, a), (sb, b) = pages
    # The credits of Twenty Seventeen name RustPress and point at the site (PLAN.md).
    a = re.sub(r'<a href="https://[a-z.]*wordpress\.org/" class="imprint">\n\t\t[^\t]*WordPress\t</a>', 'IMPRINT', a)
    b = re.sub(r'<a href="http://localhost:PORT/" class="imprint">\n\t\t[^\t]*RustPress\t</a>', 'IMPRINT', b)
    if (sa, a) == (sb, b):
        print(f'= {name}')
        return True
    print(f'x {name}: status {sa} / {sb}')
    x, y = a.split('\n'), b.split('\n')
    shown = 0
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, x, y, autojunk=False).get_opcodes():
        if tag == 'equal':
            continue
        if shown >= show:
            break
        shown += 1
        print(f'  {tag} reference {i1 + 1}-{i2}, RustPress {j1 + 1}-{j2}')
        for line in x[i1:i2][:4]:
            print('    R', line[:220])
        for line in y[j1:j2][:4]:
            print('    O', line[:220])
    return False


def main():
    args = sys.argv[1:]
    show = int(wc.option(args, '--show', '6'))
    only = wc.option(args, '--only', '')
    wc.php(wc.PREPARE)
    failures = 0
    cases = [c for c in CASES if not only or only in c[0]]
    posts = [c for c in POSTS if not only or only in c[0]]
    partials = [c for c in PARTIALS if not only or only in c[0]]
    try:
        sides = {}
        for name, user, language, address, ports in cases:
            if language is not None:
                wc.set_language(language)
            if name == 'static front page':
                wc.php(FRONT)
            theme = 'twentyseventeen' if ports == CLASSIC else 'twentytwentyfive'
            pages = []
            for port in ports:
                key = (port, user)
                if key not in sides:
                    sides[key] = wc.Side(port, user)
                status, body = fetch(sides[key], address, port, theme)
                # The block theme's reference stores its own address in the content.
                pages.append((status, norm(body, (port, 8080) if ports == BLOCK else (port,))))
            if name == 'static front page':
                wc.php(FRONT_RESET)
            if language is not None:
                wc.php(wc.RESET)
                wc.set_language(wc.php("<?php echo get_option( 'rp_widgets_lang' );").strip())
            if not report(name, pages, show):
                failures += 1
        for name, user, values, address, ports, valid in posts:
            theme = 'twentyseventeen' if ports == CLASSIC else 'twentytwentyfive'
            pages = []
            for port in ports:
                key = (port, user)
                if key not in sides:
                    sides[key] = wc.Side(port, user)
                status, body = post(sides[key], address, theme, values, valid)
                pages.append((status, norm(body, (port, 8080) if ports == BLOCK else (port,))))
            if not report(name, pages, show):
                failures += 1
        for name, user, wanted, values, address, ports in partials:
            theme = 'twentyseventeen' if ports == CLASSIC else 'twentytwentyfive'
            pages = []
            for port in ports:
                key = (port, user)
                if key not in sides:
                    sides[key] = wc.Side(port, user)
                status, body = render_partials(sides[key], address, theme, wanted, values)
                pages.append((status, norm(body, (port, 8080) if ports == BLOCK else (port,))))
            if not report(name, pages, show):
                failures += 1
    finally:
        wc.php(FRONT_RESET)
        wc.php(wc.RESET)
        wc.php(wc.CLEANUP)
    total = len(cases) + len(posts) + len(partials)
    print(f'{total - failures} of {total} equal')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
