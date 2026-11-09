#!/usr/bin/env python3
"""The website in the preview of the Customizer: pages as the frame of `customize.php` loads them
(`?customize_changeset_uuid=...&customize_theme=...&customize_messenger_channel=preview-0`), on the copy of a real site
with Twenty Seventeen (dev/up-soli.sh, :8081, against RustPress of dev/serve-soli.ps1, :3002) and on the reference with
its block theme (dev/up.sh, :8080, against dev/serve-admin.ps1, :3001). Compared whole and raw with the headers of the
preview, as an administrator, without the frame, as an editor and as a visitor (who may not look at a changeset that
does not exist), with a UUID that is none, and with a static front page and its panels. The nonces are masked.

    python dev/customize-preview-compare.py [--show N] [--only <part of a case>] [--save DIR]"""
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
OTHER = f'customize_changeset_uuid={UUID}&customize_theme=twentytwentyfour&customize_messenger_channel=preview-0'
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
    # The preview of a theme that is not active: Twenty Twenty-Four on the reference with Twenty Twenty-Five (a block
    # theme in the preview of a classic one is open in PLAN.md).
    ('theme preview', 'rpadmin', None, f'/?{OTHER}', BLOCK),
    ('theme preview post', 'rpadmin', None, f'/?p=1&{OTHER}', BLOCK),
    ('theme preview frameless', 'rpadmin', None, f'/?customize_changeset_uuid={UUID}&customize_theme=twentytwentyfour', BLOCK),
    ('theme preview missing', 'rpadmin', None, f'/?customize_changeset_uuid={UUID}&customize_theme=nope&customize_messenger_channel=preview-0', BLOCK),
    ('theme preview bad name', 'rpadmin', None, f'/?customize_changeset_uuid={UUID}&customize_theme=../../x&customize_messenger_channel=preview-0', BLOCK),
    ('theme preview no uuid', 'rpadmin', None, '/?wp_customize=on&customize_theme=twentytwentyfour', BLOCK),
]

# Menu items of the copy as the Customizer sends them: an item changed in each way it can be, one that moves up, a new
# custom link.
ITEM_26 = {'object_id': 24, 'object': 'page', 'menu_item_parent': 0, 'position': 1, 'type': 'post_type', 'title': '  Solidarität "neu"  ', 'url': 'http://example.org/ignored',
           'target': '_blank', 'attr_title': 'Ein <b>"Titel"</b> &#150;', 'description': 'x', 'classes': 'eins zwei%20drei', 'xfn': 'nofollow me', 'status': 'draft', 'original_title': '', 'nav_menu_term_id': 2, '_invalid': False}
ITEM_100 = {'object_id': 97, 'object': 'post', 'menu_item_parent': 64, 'position': 2, 'type': 'post_type', 'title': '', 'url': '', 'target': '', 'attr_title': '', 'description': '', 'classes': '', 'xfn': '', 'status': 'publish', 'original_title': '', 'nav_menu_term_id': 2, '_invalid': False}
ITEM_64 = dict(ITEM_100, object_id=41, object='page', menu_item_parent=0, position=1)
NEW_ITEM = {'object_id': 0, 'object': 'custom', 'menu_item_parent': 26, 'position': 3, 'type': 'custom', 'title': 'Neu & <i>frisch</i>', 'url': 'https://example.org/a b', 'target': '', 'attr_title': '', 'description': '', 'classes': '', 'xfn': '', 'status': 'publish', 'original_title': '', 'nav_menu_term_id': 2, '_invalid': False}

# A header image as the header control sends it.
HEADER = {'attachment_id': 5, 'url': 'http://example.org/kopf.jpg', 'thumbnail_url': 'http://example.org/kopf-150x150.jpg', 'height': 768, 'width': 768, 'timestamp': 1700000000}

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
    ('posted menu item', 'rpwidget', {'nav_menu_item[26]': ITEM_26, 'nav_menu_item[100]': dict(ITEM_100, menu_item_parent=0, position=20)}, f'/?{FRAME}', CLASSIC, True),
    ('posted new menu item', 'rpwidget', {'nav_menu_item[-5]': NEW_ITEM, 'nav_menu_item[82]': False, 'nav_menu_item[-6]': dict(NEW_ITEM, type='taxonomy', object='category', object_id=1, title='', position=1)}, f'/?p=452&{FRAME}', CLASSIC, True),
    ('posted menu', 'rpwidget', {'nav_menu[2]': {'name': 'Neues <Menü> & Co', 'description': '', 'parent': 0, 'auto_add': False}, 'nav_menu_item[26]': ITEM_26}, f'/?{FRAME}', CLASSIC, True),
    ('posted new menu', 'rpwidget', {'nav_menu[-10]': {'name': '', 'auto_add': True}, 'nav_menu_locations[top]': -10, 'nav_menu_item[-11]': dict(NEW_ITEM, nav_menu_term_id=-10, type='post_type', object='page', object_id=24, title='')}, f'/?{FRAME}', CLASSIC, True),
    # The Customizer takes a deleted menu out of its locations (`wp_page_menu()` of a location without its menu is open
    # in PLAN.md).
    ('posted deleted menu', 'rpwidget', {'nav_menu[2]': False, 'nav_menu_locations[top]': 0}, f'/?{FRAME}', CLASSIC, True),
    ('posted invalid menu items', 'rpwidget', {'nav_menu_item[26]': dict(ITEM_26, type='custom', object='custom', url='javascript:alert(1)'), 'nav_menu_item[65]': 'nope', 'nav_menu_item[64]': dict(ITEM_64, _invalid=True)}, f'/?{FRAME}', CLASSIC, True),
    ('posted header image', 'rpwidget', {'header_image': 'http://example.org/kopf.jpg', 'header_image_data': HEADER}, f'/?{FRAME}', CLASSIC, True),
    ('posted removed header', 'rpwidget', {'header_image': 'remove-header', 'header_image_data': 'remove-header'}, f'/?page_id=24&{FRAME}', CLASSIC, True),
    ('posted without nonce', 'rpwidget', {'blogname': 'Nicht übernommen'}, f'/?{FRAME}', CLASSIC, False),
    ('theme posted', 'rpadmin', {'blogname': 'Vorschau <b>4</b>', 'custom_css[twentytwentyfour]': 'p { color: green; }', 'custom_css[twentytwentyfive]': 'p { color: red; }'}, f'/?{OTHER}', BLOCK, True),
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
    ('partials header', 'rpwidget', {'custom_header': [{}]}, {'header_image': 'http://example.org/kopf.jpg', 'header_image_data': HEADER}, '/', CLASSIC),
    ('partials menus posted', 'rpwidget', 'MENUS', {'nav_menu_item[26]': ITEM_26, 'nav_menu_item[-5]': NEW_ITEM}, '/', CLASSIC),
    ('partials missing', 'rpwidget', None, {}, '/', CLASSIC),
    ('theme partials', 'rpadmin', {'blogname': [{}]}, {'blogname': 'X'}, '/?customize_theme=twentytwentyfour', BLOCK),
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

# A changeset that exists (a draft of rpwidget, with an autosave of the same user), and what is looked at with it: by
# a visitor, without a UUID (`wp_customize=on` loads the latest one that is not published), and the answers that are
# not pages, compared with all their headers. The UUID is not of version 4, so it is not masked.
SAVED = '1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d'
CHANGESET = r'''<?php
$user = get_user_by( 'login', '%(user)s' );
$data = array(
	'blogname' => array( 'value' => 'Aus dem Changeset', 'type' => 'option', 'user_id' => $user->ID ),
	'%(theme)s::colorscheme' => array( 'value' => 'dark', 'type' => 'theme_mod', 'user_id' => $user->ID ),
	'other::colorscheme' => array( 'value' => 'custom', 'type' => 'theme_mod', 'user_id' => $user->ID ),
	'custom_css[%(theme)s]' => array( 'value' => 'h1 { color: blue; }', 'type' => 'custom_css', 'user_id' => $user->ID ),
	'no_value' => array( 'type' => 'option' ),
);
$id = wp_insert_post( array( 'post_type' => 'customize_changeset', 'post_status' => 'draft', 'post_name' => '%(uuid)s', 'post_author' => $user->ID, 'post_content' => wp_json_encode( $data ) ) );
$data['blogname']['value'] = 'Aus der Autosicherung';
$wpdb->insert( $wpdb->posts, array( 'post_type' => 'revision', 'post_status' => 'inherit', 'post_parent' => $id, 'post_name' => $id . '-autosave-v1', 'post_author' => $user->ID, 'post_content' => wp_json_encode( $data ), 'post_date' => current_time( 'mysql' ), 'post_date_gmt' => current_time( 'mysql', 1 ), 'post_modified' => current_time( 'mysql' ), 'post_modified_gmt' => current_time( 'mysql', 1 ), 'post_title' => '', 'post_excerpt' => '', 'to_ping' => '', 'pinged' => '', 'post_content_filtered' => '' ) );
clean_post_cache( $id );
'''
CHANGESET_CLEANUP = r'''<?php
global $wpdb;
foreach ( $wpdb->get_col( $wpdb->prepare( "SELECT ID FROM $wpdb->posts WHERE post_type = 'customize_changeset' AND post_name = %%s", '%(uuid)s' ) ) as $id ) {
	$wpdb->delete( $wpdb->posts, array( 'post_parent' => $id, 'post_type' => 'revision' ) );
	$wpdb->delete( $wpdb->posts, array( 'ID' => $id ) );
	clean_post_cache( $id );
}
'''
SAVED_FRAME = f'customize_changeset_uuid={SAVED}&customize_theme=THEME&customize_messenger_channel=preview-0'
SAVED_CASES = [
    # (name, user, address, ports, all headers)
    ('changeset', 'rpwidget', f'/?{SAVED_FRAME}', CLASSIC, False),
    ('changeset frameless', 'rpwidget', f'/?customize_changeset_uuid={SAVED}', CLASSIC, False),
    ('changeset autosaved', 'rpwidget', f'/?{SAVED_FRAME}&customize_autosaved=on', CLASSIC, False),
    ('changeset visitor', None, f'/?customize_changeset_uuid={SAVED}', CLASSIC, False),
    ('changeset visitor post', None, f'/?p=452&customize_changeset_uuid={SAVED}', CLASSIC, False),
    ('changeset visitor autosaved', None, f'/?customize_changeset_uuid={SAVED}&customize_autosaved=on', CLASSIC, False),
    ('changeset visitor frame', None, f'/?{SAVED_FRAME}', CLASSIC, False),
    ('no uuid', 'rpwidget', '/?wp_customize=on', CLASSIC, False),
    ('no uuid frame', 'rpwidget', '/?wp_customize=on&customize_messenger_channel=preview-0', CLASSIC, False),
    ('no uuid visitor', None, '/?wp_customize=on', CLASSIC, False),
    ('feed', 'rpwidget', f'/?feed=rss2&customize_changeset_uuid={SAVED}', CLASSIC, True),
    ('feed visitor', None, f'/?feed=rss2&customize_changeset_uuid={SAVED}', CLASSIC, True),
    ('comments feed', None, f'/?feed=comments-rss2&{SAVED_FRAME}', CLASSIC, True),
    ('robots', None, f'/robots.txt?customize_changeset_uuid={SAVED}', CLASSIC, True),
    ('favicon', 'rpwidget', f'/favicon.ico?{SAVED_FRAME}', CLASSIC, True),
    ('sitemap', None, f'/wp-sitemap.xml?customize_changeset_uuid={SAVED}', CLASSIC, True),
    ('block changeset', 'rpadmin', f'/?{SAVED_FRAME}', BLOCK, False),
    ('block changeset frameless', 'rpadmin', f'/?p=1&customize_changeset_uuid={SAVED}', BLOCK, False),
    ('block changeset visitor', None, f'/?customize_changeset_uuid={SAVED}', BLOCK, False),
    ('block no uuid', 'rpadmin', '/?wp_customize=on', BLOCK, False),
    ('changeset visitor theme', None, f'/?customize_changeset_uuid={SAVED}&customize_theme=twentytwentyfour', BLOCK, False),
    ('block feed', None, f'/?feed=rss2&customize_changeset_uuid={SAVED}', BLOCK, True),
]
# Headers of the server or the moment, not of WordPress.
OWN_HEADERS = {'date', 'server', 'x-powered-by', 'connection', 'keep-alive', 'transfer-encoding', 'content-length', 'vary'}


def php_in(container, code):
    """PHP in a reference with WordPress loaded (`wc.php` runs in the copy of the real site only)."""
    import subprocess
    import tempfile
    with tempfile.NamedTemporaryFile('w', suffix='.php', delete=False, encoding='utf-8', newline='') as f:
        f.write(code)
    path = subprocess.run(['wsl', 'wslpath', '-a', f.name.replace('\\', '/')], capture_output=True, text=True).stdout.strip()
    script = f"podman cp '{path}' {container}:/tmp/rp-snippet.php >/dev/null && podman exec {container} php -d display_errors=0 -r 'define(\"WP_USE_THEMES\", false); require \"/var/www/html/wp-load.php\"; require \"/tmp/rp-snippet.php\";'"
    output = subprocess.run(['wsl', 'bash', '-c', script], capture_output=True, encoding='utf-8', errors='replace')
    os.unlink(f.name)
    return output.stdout


def fetch_all(side, address, theme):
    """The answer with all headers of WordPress, in the order of their names."""
    status, headers, body = side.client.request(address.replace('THEME', theme))
    head = ''.join(f'{k.lower()}: {v}\n' for k, v in sorted(headers.items(), key=lambda kv: kv[0].lower()) if k.lower() not in OWN_HEADERS)
    return status, head + '\n' + body


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


def preview_nonce(side, theme=None):
    """`_wpCustomizeSettings.nonce.preview` of the user's `customize.php`; for a theme that is not active, of its preview."""
    address = f'/?customize_changeset_uuid={UUID}&customize_theme={theme}&customize_messenger_channel=preview-0' if theme and theme_of(OTHER, '') == theme else '/wp-admin/customize.php'
    _, _, body = side.client.request(address)
    found = re.search(r'"preview":"([0-9a-f]{10})"', body)
    return found.group(1) if found else ''


def theme_of(address, theme):
    """The theme the address previews, if it names one."""
    found = re.search(r'customize_theme=([a-z0-9-]+)', address)
    return found.group(1) if found else theme


def post(side, address, theme, values, valid):
    """The form `wp.customize.PreviewFrame` submits to the frame."""
    theme = theme_of(address, theme)
    form = {
        '_method': 'GET',
        'wp_customize': 'on',
        'customize_theme': theme,
        'nonce': preview_nonce(side, theme) if valid else '0123456789',
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
    theme = theme_of(address, theme)
    form = {'wp_customize': 'on', 'nonce': preview_nonce(side, theme), 'customize_theme': theme, 'customized': json.dumps(values), 'customize_changeset_uuid': UUID, 'wp_customize_render_partials': '1'}
    if partials is not None:
        form['partials'] = json.dumps(menu_partials(side, theme) if partials == 'MENUS' else partials)
    status, headers, body = side.client.request(address, form)
    lower = {k.lower(): v for k, v in headers.items()}
    return status, ''.join(f'{name}: {lower.get(name.lower(), "")}\n' for name in HEADERS) + '\n' + body


SAVE = ''


def report(name, pages, show):
    (sa, a), (sb, b) = pages
    # The credits of Twenty Seventeen name RustPress and point at the site (PLAN.md).
    a = re.sub(r'<a href="https://[a-z.]*wordpress\.org/" class="imprint">\n\t\t[^\t]*WordPress\t</a>', 'IMPRINT', a)
    b = re.sub(r'<a href="http://localhost:PORT/" class="imprint">\n\t\t[^\t]*RustPress\t</a>', 'IMPRINT', b)
    if SAVE:
        os.makedirs(SAVE, exist_ok=True)
        for side, text in (('reference', a), ('rustpress', b)):
            with open(os.path.join(SAVE, f"{re.sub(r'[^a-z0-9]+', '-', name)}.{side}.html"), 'w', encoding='utf-8', newline='') as f:
                f.write(text)
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
    global SAVE
    SAVE = wc.option(args, '--save', '')
    show = int(wc.option(args, '--show', '6'))
    only = wc.option(args, '--only', '')
    wc.php(wc.PREPARE)
    failures = 0
    cases = [c for c in CASES if not only or only in c[0]]
    posts = [c for c in POSTS if not only or only in c[0]]
    partials = [c for c in PARTIALS if not only or only in c[0]]
    saved = [c for c in SAVED_CASES if not only or only in c[0]]
    containers = {CLASSIC: ('rustpress-wp-soli', 'rpwidget', 'twentyseventeen'), BLOCK: ('rustpress-wp', 'rpadmin', 'twentytwentyfive')}
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
        for ports in {c[3] for c in saved}:
            container, user, theme = containers[ports]
            php_in(container, CHANGESET_CLEANUP % {'uuid': SAVED})
            php_in(container, CHANGESET % {'user': user, 'theme': theme, 'uuid': SAVED})
        for name, user, address, ports, all_headers in saved:
            theme = containers[ports][2]
            pages = []
            for port in ports:
                key = (port, user)
                if key not in sides:
                    sides[key] = wc.Side(port, user)
                status, body = fetch_all(sides[key], address, theme) if all_headers else fetch(sides[key], address, port, theme)
                pages.append((status, norm(body, (port, 8080) if ports == BLOCK else (port,))))
            if not report(name, pages, show):
                failures += 1
    finally:
        for container, _, _ in containers.values():
            php_in(container, CHANGESET_CLEANUP % {'uuid': SAVED})
        wc.php(FRONT_RESET)
        wc.php(wc.RESET)
        wc.php(wc.CLEANUP)
    total = len(cases) + len(posts) + len(partials) + len(saved)
    print(f'{total - failures} of {total} equal')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
