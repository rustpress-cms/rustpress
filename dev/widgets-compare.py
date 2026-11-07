#!/usr/bin/env python3
"""`wp-admin/widgets.php` of Twenty Seventeen and the routes behind it on the copy of a real site (dev/up-soli.sh, :8081)
and RustPress (dev/serve-soli.ps1, :3002): the same requests run on each side from the same state, and every answer and
the stored options are compared. The page is compared whole, in the language of the site and in English.

    python dev/widgets-compare.py [--show N] [--only <part of a step>]"""
import base64
import difflib
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

REFERENCE, RUSTPRESS = 8081, 3002
sys.stdout.reconfigure(encoding='utf-8')


def option(args, name, default):
    if name in args:
        i = args.index(name)
        value = args[i + 1]
        del args[i:i + 2]
        return value
    return default


def php(code):
    """Runs PHP in the copy, with WordPress loaded, and returns what it prints."""
    with tempfile.NamedTemporaryFile('w', suffix='.php', delete=False, encoding='utf-8', newline='') as f:
        f.write(code)
    path = subprocess.run(['wsl', 'wslpath', '-a', f.name.replace('\\', '/')], capture_output=True, text=True).stdout.strip()
    script = f"podman cp '{path}' rustpress-wp-soli:/tmp/rp-snippet.php >/dev/null && podman exec rustpress-wp-soli php -d display_errors=0 -r 'define(\"WP_USE_THEMES\", false); require \"/var/www/html/wp-load.php\"; require \"/tmp/rp-snippet.php\";'"
    output = subprocess.run(['wsl', 'bash', '-c', script], capture_output=True, encoding='utf-8', errors='replace')
    os.unlink(f.name)
    return output.stdout


PREPARE = r'''<?php
foreach ( array( 'rpwidget' => 'administrator', 'rpwidgeted' => 'editor' ) as $login => $role ) {
	if ( ! get_user_by( 'login', $login ) ) {
		wp_insert_user( array( 'user_login' => $login, 'user_pass' => 'rustpress-dev', 'user_email' => "$login@example.com", 'role' => $role ) );
	}
}
global $wpdb;
$rows = $wpdb->get_results( "SELECT option_name, option_value, autoload FROM $wpdb->options WHERE option_name LIKE 'widget\_%' OR option_name = 'sidebars_widgets'", ARRAY_A );
update_option( 'rp_widgets_saved', wp_json_encode( $rows ), false );
update_option( 'rp_widgets_lang', get_option( 'WPLANG' ), false );
'''

RESET = r'''<?php
global $wpdb;
$rows  = json_decode( get_option( 'rp_widgets_saved' ), true );
$names = wp_list_pluck( $rows, 'option_name' );
foreach ( $wpdb->get_col( "SELECT option_name FROM $wpdb->options WHERE option_name LIKE 'widget\_%'" ) as $name ) {
	if ( ! in_array( $name, $names, true ) ) {
		$wpdb->delete( $wpdb->options, array( 'option_name' => $name ) );
	}
}
foreach ( $rows as $row ) {
	$wpdb->update( $wpdb->options, array( 'option_value' => $row['option_value'], 'autoload' => $row['autoload'] ), array( 'option_name' => $row['option_name'] ) );
}
wp_cache_flush();
'''

CLEANUP = r'''<?php
require_once ABSPATH . 'wp-admin/includes/user.php';
foreach ( array( 'rpwidget', 'rpwidgeted' ) as $login ) {
	$user = get_user_by( 'login', $login );
	if ( $user ) {
		wp_delete_user( $user->ID );
	}
}
update_option( 'WPLANG', get_option( 'rp_widgets_lang' ) );
delete_option( 'rp_widgets_saved' );
delete_option( 'rp_widgets_lang' );
delete_option( 'can_compress_scripts' );
'''

SNAPSHOT = r'''<?php
global $wpdb;
$rows = $wpdb->get_results( "SELECT option_name, option_value, autoload FROM $wpdb->options WHERE option_name LIKE 'widget\_%' OR option_name = 'sidebars_widgets' ORDER BY option_name", ARRAY_A );
echo json_encode( $rows, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE );
'''


def set_language(lang):
    php(f"<?php update_option( 'WPLANG', '{lang}' ); wp_cache_flush();")


def norm(text, port):
    text = text.replace(f'localhost:{port}', 'localhost:PORT')
    # The head of the preview of a widget lacks the styles of all blocks (PLAN.md).
    text = re.sub(r'"preview":"\\t\\t<!doctype.*?<body', '"preview":"<body', text, flags=re.S)
    # What the media and RSS widgets print is open in PLAN.md.
    text = re.sub(r'"id":"(media_\w+|rss)-(\d+)","id_base":"(\w+)","sidebar":"([\w-]+)","rendered":"(?:[^"\\]|\\.)*"', r'"id":"\1-\2","id_base":"\3","sidebar":"\4","rendered":""', text)
    text = re.sub(r'"preview":"(?:[^"\\]|\\.)*"', '"preview":""', text) if 'media_' in text or 'widget-rss' in text else text
    text = re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', text)
    text = re.sub(r'_wpnonce%3D[0-9a-f]{10}', '_wpnonce%3DNONCE', text)
    return text


class Side:
    def __init__(self, port, user='rpwidget'):
        self.port = port
        self.client = lc.Client(port)
        self.nonce = ''
        if user:
            self.client.request('/wp-login.php')
            self.client.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
            status, _, body = self.client.request('/wp-admin/admin-ajax.php?action=rest-nonce')
            self.nonce = body.strip() if status == 200 else ''

    def rest(self, method, route, query='', body=None):
        url = f'http://localhost:{self.port}/index.php?rest_route={route}' + (f'&{query}' if query else '')
        headers = {'X-WP-Nonce': self.nonce} if self.nonce else {}
        data = None
        if body is not None:
            data = json.dumps(body).encode()
            headers['Content-Type'] = 'application/json'
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            r = self.client.op.open(req)
            return r.status, {k.title(): v for k, v in r.headers.items()}, r.read().decode('utf-8', 'replace')
        except urllib.error.HTTPError as e:
            return e.code, {k.title(): v for k, v in e.headers.items()}, e.read().decode('utf-8', 'replace')


RAW = {
    'block': {'content': '<!-- wp:paragraph -->\n<p>Hi "you" &amp; me</p>\n<!-- /wp:paragraph -->'},
    'text': {'title': 'A <b>title</b> & more', 'text': '<p>Hello <em>world</em></p>', 'filter': True, 'visual': True},
    'custom_html': {'title': 'Code', 'content': '<script>1</script><p>x</p>'},
    'recent-posts': {'title': 'Latest', 'number': '3', 'show_date': 'on'},
    'recent-comments': {'title': '', 'number': -4},
    'archives': {'title': 'By month', 'count': 'on', 'dropdown': ''},
    'categories': {'title': '', 'count': 1, 'hierarchical': 0, 'dropdown': 'on'},
    'pages': {'title': 'Pages', 'sortby': 'ID', 'exclude': '1, 2'},
    'meta': {'title': 'Meta'},
    'search': {'title': 'Find'},
    'tag_cloud': {'title': 'Tags', 'count': 'on', 'taxonomy': 'category'},
    'calendar': {'title': 'When'},
    'nav_menu': {'title': 'Menu', 'nav_menu': '2'},
    'rss': {'title': ' <b>Feed</b> ', 'items': 25, 'show_date': 1},
    'media_image': {'url': 'http://example.com/a.png', 'title': 'Pic', 'size': 'medium', 'width': 10, 'link_type': 'none', 'link_target_blank': 'true', 'image_classes': 'a b!', 'alt': ' x '},
    'media_audio': {'url': 'http://example.com/a.mp3', 'preload': 'bad', 'loop': ''},
    'media_video': {'url': 'http://example.com/a.mp4', 'content': '<script>x</script><b>y</b>', 'loop': 'false'},
    'media_gallery': {'ids': [3, '5', 3], 'columns': 12, 'size': 'thumbnail', 'orderby_random': 1},
}

FORM = {
    'text': 'widget-text[-1][title]=T%26x&widget-text[-1][text]=%3Cb%3Ex%3C%2Fb%3E&widget-text[-1][filter]=on&widget-text[-1][visual]=on',
    'recent-posts': 'widget-recent-posts[-1][title]=R&widget-recent-posts[-1][number]=2&widget-recent-posts[-1][show_date]=on',
    'archives': 'widget-archives[-1][title]=&widget-archives[-1][count]=on',
    'nav_menu': 'widget-nav_menu[-1][title]=M&widget-nav_menu[-1][nav_menu]=2',
    'tag_cloud': 'widget-tag_cloud[-1][title]=&widget-tag_cloud[-1][taxonomy]=post_tag',
    'pages': 'widget-pages[-1][sortby]=nope&widget-pages[-1][exclude]=7',
    'custom_html': 'widget-custom_html[-1][title]=H&widget-custom_html[-1][content]=%3Ci%3Ex%3C%2Fi%3E',
    'media_image': 'widget-media_image[-1][url]=http%3A%2F%2Fexample.com%2Fb.png&widget-media_image[-1][size]=full',
}


# An instance of the text widget, as `encode` gives it to the editor.
INSTANCE = 'a:4:{s:5:"title";s:7:"Preview";s:4:"text";s:9:"<b>hi</b>";s:6:"filter";b:1;s:6:"visual";b:1;}'
ENCODED = base64.b64encode(INSTANCE.encode()).decode()
SIGNED = {}


def steps():
    """`(name, method, route, query, body)`."""
    for query in ('', 'context=edit'):
        yield (f'sidebars {query}', 'GET', '/wp/v2/sidebars', query, None)
        yield (f'widgets {query}', 'GET', '/wp/v2/widgets', query, None)
    yield ('sidebar', 'GET', '/wp/v2/sidebars/sidebar-1', 'context=edit', None)
    yield ('inactive sidebar', 'GET', '/wp/v2/sidebars/wp_inactive_widgets', '', None)
    yield ('no sidebar', 'GET', '/wp/v2/sidebars/nope', '', None)
    yield ('widgets of a sidebar', 'GET', '/wp/v2/widgets', 'sidebar=sidebar-2&context=edit', None)
    yield ('widget', 'GET', '/wp/v2/widgets/text-3', 'context=edit', None)
    yield ('no widget', 'GET', '/wp/v2/widgets/nope-9', '', None)
    yield ('widget types', 'GET', '/wp/v2/widget-types', '', None)
    yield ('widget type', 'GET', '/wp/v2/widget-types/search', 'context=edit', None)
    yield ('no widget type', 'GET', '/wp/v2/widget-types/nope', '', None)
    for base, raw in RAW.items():
        yield (f'create {base}', 'POST', '/wp/v2/widgets', '', {'id_base': base, 'sidebar': 'sidebar-2', 'instance': {'raw': raw}})
    for base, raw in RAW.items():
        yield (f'encode {base}', 'POST', f'/wp/v2/widget-types/{base}/encode', '', {'instance': {}, 'number': 5})
    for base, data in FORM.items():
        yield (f'encode form {base}', 'POST', f'/wp/v2/widget-types/{base}/encode', '', {'form_data': data})
    for base in ('search', 'meta', 'nav_menu', 'recent-posts'):
        yield (f'preview {base}', 'POST', f'/wp/v2/widget-types/{base}/render', '', {})
    yield ('preview of text', 'POST', '/wp/v2/widget-types/text/render', '', {'instance': {'encoded': ENCODED, 'hash': SIGNED['hash']}})
    yield ('preview bad hash', 'POST', '/wp/v2/widget-types/text/render', '', {'instance': {'encoded': ENCODED, 'hash': 'nope'}})
    yield ('preview unknown', 'POST', '/wp/v2/widget-types/nope/render', '', {})
    yield ('encode unknown', 'POST', '/wp/v2/widget-types/nope/encode', '', {})
    yield ('encode bad hash', 'POST', '/wp/v2/widget-types/text/encode', '', {'instance': {'encoded': 'YTowOnt9', 'hash': 'nope'}})
    yield ('create without type', 'POST', '/wp/v2/widgets', '', {'sidebar': 'sidebar-1'})
    yield ('create unknown type', 'POST', '/wp/v2/widgets', '', {'id_base': 'nope', 'sidebar': 'sidebar-1'})
    yield ('create without instance', 'POST', '/wp/v2/widgets', '', {'id_base': 'meta', 'sidebar': 'sidebar-1'})
    yield ('create bad instance', 'POST', '/wp/v2/widgets', '', {'id_base': 'meta', 'sidebar': 'sidebar-1', 'instance': {'nope': 1}})
    yield ('create default sidebar', 'POST', '/wp/v2/widgets', '', {'id_base': 'meta', 'instance': {'raw': {'title': 'Lost'}}})
    yield ('update form', 'PUT', '/wp/v2/widgets/text-3', '', {'form_data': 'widget-text[3][title]=Neu&widget-text[3][text]=%3Cb%3Ex%3C%2Fb%3E&widget-text[3][filter]=on&widget-text[3][visual]=on'})
    yield ('update raw', 'POST', '/wp/v2/widgets/search-2', '', {'instance': {'raw': {'title': 'Gesucht'}}})
    yield ('move', 'PUT', '/wp/v2/widgets/meta-2', '', {'sidebar': 'sidebar-3'})
    yield ('move to inactive', 'PATCH', '/wp/v2/widgets/pages-2', '', {'sidebar': 'wp_inactive_widgets'})
    yield ('update nothing', 'PUT', '/wp/v2/widgets/tag_cloud-2', '', {})
    yield ('update unknown', 'PUT', '/wp/v2/widgets/text-99', '', {'instance': {'raw': {'title': 'x'}}})
    yield ('delete to inactive', 'DELETE', '/wp/v2/widgets/archives-2', '', None)
    yield ('delete for good', 'DELETE', '/wp/v2/widgets/categories-2', 'force=true', None)
    yield ('delete unknown', 'DELETE', '/wp/v2/widgets/text-99', 'force=true', None)
    yield ('order a sidebar', 'POST', '/wp/v2/sidebars/sidebar-1', '', {'widgets': ['recent-comments-2', 'search-2', 'nav_menu-7']})
    yield ('sidebar without widgets', 'PUT', '/wp/v2/sidebars/sidebar-3', '', {})
    yield ('delete a sidebar', 'DELETE', '/wp/v2/sidebars/sidebar-3', '', None)
    yield ('batch', 'POST', '/batch/v1', '', {'validation': 'require-all-validate', 'requests': [
        {'method': 'POST', 'path': '/wp/v2/widgets', 'body': {'id_base': 'block', 'sidebar': 'sidebar-3', 'instance': {'raw': {'content': '<p>one</p>'}}}},
        {'method': 'POST', 'path': '/wp/v2/widgets', 'body': {'id_base': 'block', 'sidebar': 'sidebar-3', 'instance': {'raw': {'content': '<p>two</p>'}}}},
        {'method': 'PUT', 'path': '/wp/v2/widgets/search-2', 'body': {'instance': {'raw': {'title': 'Batch'}}}},
        {'method': 'DELETE', 'path': '/wp/v2/widgets/block-2?force=true'},
    ]})
    yield ('batch refused', 'POST', '/batch/v1', '', {'validation': 'require-all-validate', 'requests': [
        {'method': 'POST', 'path': '/wp/v2/widgets', 'body': {'id_base': 'block', 'sidebar': 'sidebar-3', 'instance': {'raw': {'content': '<p>no</p>'}}}},
        {'method': 'POST', 'path': '/wp/v2/sidebars/sidebar-3', 'body': {}},
        {'method': 'POST', 'path': '/wp/v2/nope', 'body': {}},
    ]})
    yield ('batch without validation', 'POST', '/batch/v1', '', {'requests': [
        {'method': 'POST', 'path': '/wp/v2/sidebars/sidebar-3', 'body': {}},
        {'method': 'POST', 'path': '/wp/v2/widgets', 'body': {'id_base': 'block', 'sidebar': 'sidebar-3', 'instance': {'raw': {'content': '<p>on</p>'}}}},
    ]})
    yield ('batch without requests', 'POST', '/batch/v1', '', {})
    yield ('batch too long', 'POST', '/batch/v1', '', {'requests': [{'path': '/wp/v2/widgets'}] * 26})
    yield ('widgets at the end', 'GET', '/wp/v2/widgets', 'context=edit&_embed=about', None)
    yield ('sidebars at the end', 'GET', '/wp/v2/sidebars', 'context=edit', None)


def compare(name, responses, states, show):
    (status_a, head_a, body_a), (status_b, head_b, body_b) = responses
    a, b = norm(body_a, REFERENCE), norm(body_b, RUSTPRESS)
    keep = ('content-type', 'allow', 'x-robots-tag', 'location')
    pick = lambda head, port: {k.lower(): norm(v, port) for k, v in head.items() if k.lower() in keep}
    head_a, head_b = pick(head_a, REFERENCE), pick(head_b, RUSTPRESS)
    same = status_a == status_b and a == b and head_a == head_b and states[0] == states[1]
    print(f"{'equal' if same else 'DIFF '} {name:34} {status_a} {status_b} {len(a):7} {len(b):7}")
    if head_a != head_b:
        print('    headers', head_a, head_b)
    if show and a != b:
        at = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
        print('    R', repr(a[max(0, at - 120):at + 240]))
        print('    O', repr(b[max(0, at - 120):at + 240]))
    if states[0] != states[1]:
        print('    ROWS DIFFER')
        for line in list(difflib.unified_diff(states[0].split('\n'), states[1].split('\n'), lineterm='', n=0))[2:12]:
            print('   ', line[:300])
    return same


def compare_page(name, show):
    # A change of the language empties the update transients, and the next page of the reference fills them after its menu.
    Side(REFERENCE).client.request('/wp-admin/widgets.php')
    pages = [Side(port).client.request('/wp-admin/widgets.php') for port in (REFERENCE, RUSTPRESS)]
    return compare(name, pages, ('', ''), show)


def main():
    args = sys.argv[1:]
    show = int(option(args, '--show', '1'))
    only = option(args, '--only', '')
    php(PREPARE)
    SIGNED['hash'] = php(f"<?php echo wp_hash( '{INSTANCE}' );").strip()
    failed = total = 0
    try:
        lang = php("<?php echo get_option( 'rp_widgets_lang' );").strip()
        results = []
        for port in (REFERENCE, RUSTPRESS):
            php(RESET)
            side = Side(port)
            run = []
            for name, method, route, query, body in steps():
                if only not in name:
                    continue
                response = side.rest(method, route, query, body)
                run.append((name, response, php(SNAPSHOT)))
            results.append(run)
        for left, right in zip(*results):
            total += 1
            states = [state.replace(f'localhost:{port}', 'localhost:PORT') for state, port in zip((left[2], right[2]), (REFERENCE, RUSTPRESS))]
            if not compare(left[0], (left[1], right[1]), states, show):
                failed += 1
        php(RESET)
        # Who may not edit theme options, and who is not signed in.
        for user in ('rpwidgeted', None):
            sides = [Side(port, user) for port in (REFERENCE, RUSTPRESS)]
            for name, method, route, query, body in (('read', 'GET', '/wp/v2/widgets', '', None), ('write', 'POST', '/wp/v2/widgets', '', {'id_base': 'meta'}), ('sidebar', 'POST', '/wp/v2/sidebars/sidebar-1', '', {'widgets': []}), ('encode', 'POST', '/wp/v2/widget-types/meta/encode', '', {})):
                total += 1
                if not compare(f'{user or "nobody"} {name}', [s.rest(method, route, query, body) for s in sides], ('', ''), show):
                    failed += 1
        total += 1
        pages = [Side(port, 'rpwidgeted').client.request('/wp-admin/widgets.php') for port in (REFERENCE, RUSTPRESS)]
        if not compare('editor, page', pages, ('', ''), show):
            failed += 1
        php(RESET)
        total += 1
        if not compare_page(f'the page in {lang}', show):
            failed += 1
        set_language('')
        total += 1
        if not compare_page('the page in English', show):
            failed += 1
        if lang:
            set_language(lang)
    finally:
        php(RESET)
        php(CLEANUP)
    print(f'{total - failed} of {total} equal')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
