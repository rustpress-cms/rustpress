#!/usr/bin/env python3
"""Compares Twenty Seventeen on the copy of a real site (dev/up-soli.sh, :8081) with RustPress (dev/serve-soli.ps1,
:3002): post formats, paged posts, the front page panels, the custom colors, the header text color and the social menu.
The posts, options and theme mods are made through PHP in the copy and put back afterwards.

    python dev/seventeen-compare.py [--diff N] [substring-of-scenario]"""
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
REFERENCE, RUSTPRESS = 8081, 3002

SEED = r'''<?php
$made = array( 'posts' => array(), 'terms' => array() );
$tag  = wp_insert_term( 'rpts', 'post_tag' );
$made['terms'][] = array( $tag['term_id'], 'post_tag' );
function rpts_post( $title, $content, $format = '', $args = array() ) {
	global $made;
	static $minute = 0;
	++$minute;
	$id = wp_insert_post( array_merge( array( 'post_title' => $title, 'post_name' => $title, 'post_author' => 1, 'post_status' => 'publish', 'post_date' => gmdate( 'Y-m-d H:i:s', 1577934245 + 60 * $minute ), 'post_content' => wp_slash( $content ), 'tags_input' => array( 'rpts' ) ), $args ) );
	if ( $format ) {
		set_post_format( $id, $format );
	}
	$made['posts'][] = $id;
	return $id;
}
$image   = get_posts( array( 'post_type' => 'attachment', 'post_mime_type' => 'image', 'numberposts' => 1, 'post_status' => 'inherit', 'orderby' => 'ID', 'order' => 'ASC' ) )[0];
$url     = wp_get_attachment_url( $image->ID );
$paged   = "Erste Seite\n<!--nextpage-->\nZweite Seite\n<!--nextpage-->\nDritte Seite";
$ids     = array();
$ids['plain']        = rpts_post( 'rpts-plain', 'Einfach' );
$ids['paged']        = rpts_post( 'rpts-paged', $paged );
$ids['paged_block']  = rpts_post( 'rpts-paged-block', "<!-- wp:paragraph --><p>Eins</p><!-- /wp:paragraph -->\n<!-- wp:nextpage -->\n<!--nextpage-->\n<!-- /wp:nextpage -->\n<!-- wp:paragraph --><p>Zwei</p><!-- /wp:paragraph -->" );
$ids['page_paged']   = rpts_post( 'rpts-page-paged', $paged, '', array( 'post_type' => 'page', 'tags_input' => array() ) );
$ids['audio']        = rpts_post( 'rpts-audio', "Davor\n\n[audio src=\"http://example.com/a.mp3\"]\n\nDanach", 'audio' );
$ids['audio_none']   = rpts_post( 'rpts-audio-none', 'Nur Text', 'audio' );
$ids['audio_thumb']  = rpts_post( 'rpts-audio-thumb', '[audio src="http://example.com/a.mp3"]', 'audio' );
$ids['audio_more']   = rpts_post( 'rpts-audio-paged', "[audio src=\"http://example.com/a.mp3\"]\n<!--nextpage-->\nMehr", 'audio' );
$ids['video']        = rpts_post( 'rpts-video', "[video src=\"http://example.com/a.mp4\"]\n\nText", 'video' );
$ids['video_thumb']  = rpts_post( 'rpts-video-thumb', '[video src="http://example.com/a.mp4"]', 'video' );
$ids['video_none']   = rpts_post( 'rpts-video-none', 'Nur Text', 'video' );
$ids['video_html']   = rpts_post( 'rpts-video-html', '<iframe src="http://example.com/f"></iframe><object data="x"></object><embed src="y" />', 'video' );
$ids['image']        = rpts_post( 'rpts-image', "<img src=\"$url\" alt=\"\" />", 'image' );
$ids['image_thumb']  = rpts_post( 'rpts-image-thumb', 'Text', 'image' );
$ids['gallery']      = rpts_post( 'rpts-gallery', "Text\n\n[gallery ids=\"{$image->ID}\" columns=\"1\"]\n\nMehr", 'gallery' );
$ids['gallery_thumb'] = rpts_post( 'rpts-gallery-thumb', "[gallery ids=\"{$image->ID}\"]", 'gallery' );
$ids['gallery_none'] = rpts_post( 'rpts-gallery-none', 'Keine', 'gallery' );
$ids['gallery_none_thumb'] = rpts_post( 'rpts-gallery-none-thumb', 'Keine', 'gallery' );
$ids['gallery_block'] = rpts_post( 'rpts-gallery-block', '<!-- wp:gallery {"linkTo":"none"} --><figure class="wp-block-gallery has-nested-images columns-default is-cropped"><!-- wp:image {"id":' . $image->ID . '} --><figure class="wp-block-image"><img src="' . $url . '" alt="" class="wp-image-' . $image->ID . '"/></figure><!-- /wp:image --></figure><!-- /wp:gallery -->', 'gallery' );
$ids['aside']        = rpts_post( 'rpts-aside', 'Am Rande', 'aside' );
$track = wp_insert_attachment( array( 'post_title' => 'Lied', 'post_mime_type' => 'audio/mpeg', 'post_status' => 'inherit' ), false, 0 );
update_post_meta( $track, '_wp_attached_file', '2020/01/lied.mp3' );
$made['posts'][]     = $track;
$ids['playlist']     = rpts_post( 'rpts-playlist', "[playlist ids=\"$track\"]\n\nText", 'audio' );
foreach ( array( 'audio_thumb', 'video_thumb', 'image_thumb', 'gallery_thumb', 'gallery_none_thumb' ) as $key ) {
	set_post_thumbnail( $ids[ $key ], $image->ID );
}
$made['ids']   = $ids;
$made['image'] = $image->ID;
$made['tag']   = $tag['term_id'];

// The pages of the front page.
$front  = rpts_post( 'rpts-front', "Vorn\n\n<!--nextpage-->\n\nWeiter", '', array( 'post_type' => 'page', 'tags_input' => array() ) );
$blog   = rpts_post( 'rpts-blog', 'Blogseite', '', array( 'post_type' => 'page', 'tags_input' => array() ) );
$panel1 = rpts_post( 'rpts-panel-eins', 'Inhalt Eins', '', array( 'post_type' => 'page', 'tags_input' => array() ) );
$panel2 = rpts_post( 'rpts-panel-zwei', "Inhalt Zwei\n<!--nextpage-->\nZwei B", '', array( 'post_type' => 'page', 'tags_input' => array() ) );
set_post_thumbnail( $panel1, $image->ID );
set_post_thumbnail( $front, $image->ID );
$made['front'] = array( $front, $blog, $panel1, $panel2 );

// A menu for the social links.
$menu = wp_create_nav_menu( 'rpts-social' );
foreach ( array( 'https://facebook.com/x' => 'Facebook', 'mailto:a@example.com' => 'Mail', 'https://example.com/' => 'Sonst', 'https://github.com/y' => 'GitHub' ) as $link => $title ) {
	wp_update_nav_menu_item( $menu, 0, array( 'menu-item-title' => $title, 'menu-item-url' => $link, 'menu-item-status' => 'publish', 'menu-item-type' => 'custom' ) );
}
$made['menu']    = $menu;
$made['options'] = array();
foreach ( array( 'show_on_front', 'page_on_front', 'page_for_posts', 'theme_mods_twentyseventeen', 'WPLANG', 'permalink_structure' ) as $name ) {
	$made['options'][ $name ] = get_option( $name );
}
// An administrator, because the edit links are for those who may edit.
$made['user'] = wp_insert_user( array( 'user_login' => 'rptsadmin', 'user_pass' => 'rustpress-dev', 'user_email' => 'rptsadmin@example.com', 'role' => 'administrator' ) );
require_once ABSPATH . 'wp-admin/includes/file.php';
require_once ABSPATH . 'wp-admin/includes/translation-install.php';
if ( ! in_array( 'ar', get_available_languages(), true ) ) {
	// `wp_download_language_pack()` refuses here: the files of the copy do not belong to the web server.
	$translations = wp_get_available_translations();
	$package      = download_url( $translations['ar']['package'] );
	$zip          = new ZipArchive();
	if ( ! is_wp_error( $package ) && true === $zip->open( $package ) ) {
		$zip->extractTo( WP_LANG_DIR );
		$zip->close();
	}
}
echo base64_encode( json_encode( $made ) );
'''

RESTORE = r'''<?php
$made = json_decode( base64_decode( '%s' ), true );
foreach ( $made['options'] as $name => $value ) { update_option( $name, $value ); }
foreach ( $made['posts'] as $id ) { wp_delete_post( $id, true ); }
foreach ( $made['terms'] as $term ) { wp_delete_term( $term[0], $term[1] ); }
wp_delete_nav_menu( $made['menu'] );
require_once ABSPATH . 'wp-admin/includes/user.php';
wp_delete_user( $made['user'], 1 );
global $wpdb;
echo $wpdb->get_var( "SELECT COUNT(*) FROM {$wpdb->posts} WHERE post_name LIKE 'rpts-%%'" ), "\n";
'''

SET_OPTIONS = r'''<?php
$changes = json_decode( base64_decode( '%s' ), true );
$mods    = get_option( 'theme_mods_twentyseventeen' );
foreach ( $changes['mods'] as $name => $value ) {
	if ( null === $value ) {
		unset( $mods[ $name ] );
	} else {
		$mods[ $name ] = $value;
	}
}
update_option( 'theme_mods_twentyseventeen', $mods );
foreach ( $changes['options'] as $name => $value ) { update_option( $name, $value ); }
'''


def php(code):
    with tempfile.NamedTemporaryFile('w', suffix='.php', delete=False, encoding='utf-8', newline='') as f:
        f.write(code)
    path = subprocess.run(['wsl', 'wslpath', '-a', f.name.replace('\\', '/')], capture_output=True, text=True).stdout.strip()
    script = f"podman cp '{path}' rustpress-wp-soli:/tmp/rp-snippet.php >/dev/null && podman exec rustpress-wp-soli php -d display_errors=0 -r 'define(\"WP_USE_THEMES\", false); require \"/var/www/html/wp-load.php\"; require \"/tmp/rp-snippet.php\";'"
    out = subprocess.run(['wsl', 'bash', '-c', script], capture_output=True, text=True, encoding='utf-8')
    os.unlink(f.name)
    return out.stdout.strip()


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def get(port, path):
    request = urllib.request.Request(f'http://127.0.0.1:{port}{path}', headers={'Host': f'localhost:{port}'})
    try:
        response = urllib.request.build_opener(NoRedirect).open(request, timeout=120)
    except urllib.error.HTTPError as error:
        response = error
    return response.code, response.read().decode('utf-8', 'replace')


def normalize(text, port):
    text = text.replace(f'localhost:{port}', 'HOST').replace(f'localhost%3A{port}', 'HOST').replace('localhost:8081', 'HOST')
    # The credit deliberately names RustPress.
    text = re.sub(r'<a href="[^"]*" class="imprint">\s*[^<]*</a>', '<IMPRINT>', text)
    return text.replace('\r\n', '\n')


def compare(name, path, lines, marker=None):
    a, b = get(REFERENCE, path), get(RUSTPRESS, path)
    ta, tb = normalize(a[1], REFERENCE), normalize(b[1], RUSTPRESS)
    same = a[0] == b[0] and ta == tb
    seen = f' {marker}: {ta.count(marker)}' if marker else ''
    print(('ok   ' if same else 'DIFF ') + f'{name} {path} [{a[0]} {b[0]}] {len(ta)} {len(tb)}{seen}', flush=True)
    if not same:
        la, lb = ta.split('\n'), tb.split('\n')
        shown = 0
        for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, la, lb, autojunk=False).get_opcodes():
            if tag == 'equal' or shown >= lines:
                continue
            shown += 1
            x, y = '\n'.join(la[i1:i2]), '\n'.join(lb[j1:j2])
            if not x or not y:
                print('   after', repr(la[i1 - 1][:150]))
            at = next((k for k, (p, q) in enumerate(zip(x, y)) if p != q), min(len(x), len(y)))
            if not x.strip() or not y.strip():
                print('   around A', [line[:70] for line in la[i1 - 1:i1 + 3]])
                print('   around B', [line[:70] for line in lb[j1 - 1:j1 + 3]])
            print('   A', repr(x[max(0, at - 90):at + 190]))
            print('   B', repr(y[max(0, at - 90):at + 190]))
    return same


def apply(mods=None, options=None):
    import base64
    php(SET_OPTIONS % base64.b64encode(json.dumps({'mods': mods or {}, 'options': options or {}}).encode()).decode())
    # The servers reload their options once a second.
    import time
    time.sleep(2.5)


def scenarios(made):
    ids = made['ids']
    front, blog, panel1, panel2 = made['front']
    formats = [(f'single-{key}', f'/?p={post}', None) for key, post in ids.items()]
    formats.append(('archive', f'/?tag=rpts', 'entry-content'))
    formats.append(('archive-2', f'/?tag=rpts&paged=2', 'entry-content'))
    yield 'formats', {}, {}, formats
    yield 'pages', {}, {}, [
        ('page-2', f'/?p={ids["paged"]}&page=2', 'page-links'),
        ('page-3', f'/?p={ids["paged"]}&page=3', 'page-links'),
        ('page-9', f'/?p={ids["paged"]}&page=9', 'page-links'),
        ('block-2', f'/?p={ids["paged_block"]}&page=2', 'page-links'),
        ('page-page', f'/?page_id={ids["page_paged"]}', 'page-links'),
        ('page-page-2', f'/?page_id={ids["page_paged"]}&page=2', 'page-links'),
    ]
    yield 'pretty', {}, {'permalink_structure': '/%postname%/'}, [
        ('pretty-paged', f'/rpts-paged/', 'page-links'),
        ('pretty-paged-2', f'/rpts-paged/2/', 'page-links'),
    ]
    yield 'panels', {'panel_1': panel1, 'panel_2': panel2, 'panel_3': blog, 'panel_4': 0}, {'show_on_front': 'page', 'page_on_front': front, 'page_for_posts': blog, 'permalink_structure': ''}, [
        ('front', '/', 'twentyseventeen-panel'),
        ('front-page-2', '/?page=2', 'twentyseventeen-panel'),
        ('blog', '/?page_id=' + str(blog), 'entry-title'),
    ]
    yield 'front-no-panels', {'panel_1': 0, 'panel_2': 0, 'panel_3': 0}, {'show_on_front': 'page', 'page_on_front': front, 'page_for_posts': 0}, [('front-plain', '/', 'twentyseventeen-panel')]
    yield 'colors', {'panel_1': None, 'panel_2': None, 'panel_3': None, 'panel_4': None, 'colorscheme': 'custom', 'colorscheme_hue': 120}, {'show_on_front': 'posts', 'page_on_front': 0, 'page_for_posts': 0}, [('custom', '/', 'colors-custom')]
    yield 'colors-default-hue', {'colorscheme_hue': None}, {}, [('custom-250', '/', 'colors-custom')]
    yield 'dark', {'colorscheme': 'dark'}, {}, [('dark', '/', 'colors-dark')]
    yield 'header-color', {'colorscheme': 'light', 'header_textcolor': 'ff0000'}, {}, [('color', '/', 'twentyseventeen-custom-header-styles')]
    yield 'header-blank', {'header_textcolor': 'blank'}, {}, [('blank', '/', 'twentyseventeen-custom-header-styles')]
    yield 'social', {'header_textcolor': None, 'nav_menu_locations': {'top': 2, 'social': made['menu']}}, {}, [('social', '/', 'social-navigation')]
    yield 'rtl', {'nav_menu_locations': {'top': 2}}, {'WPLANG': 'ar'}, [('rtl', '/', 'dir="rtl"'), ('rtl-single', f'/?p={ids["paged"]}', 'rtl.css')]
    yield 'language-back', {}, {'WPLANG': made['options']['WPLANG']}, []


def signed_in(made, lines):
    """The edit links and the entry footer are only for who may edit."""
    spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
    lc = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(lc)
    ids = made['ids']
    bad = 0
    clients = {}
    for port in (REFERENCE, RUSTPRESS):
        clients[port] = lc.Client(port)
        clients[port].request('/wp-login.php')
        status = clients[port].request('/wp-login.php', {'log': 'rptsadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'})[0]
        print('signed in on', port, status, flush=True)
    for label, path in (('plain', f'/?p={ids["plain"]}'), ('audio', f'/?p={ids["audio"]}'), ('archive', '/?tag=rpts'), ('page', f'/?page_id={ids["page_paged"]}')):
        a = clients[REFERENCE].request(path)
        b = clients[RUSTPRESS].request(path)
        clean = lambda text, port: re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', normalize(text, port))
        ta, tb = clean(a[2], REFERENCE), clean(b[2], RUSTPRESS)
        same = a[0] == b[0] and ta == tb
        bad += not same
        print(('ok   ' if same else 'DIFF ') + f'signed in {label} {path} [{a[0]} {b[0]}] {len(ta)} {len(tb)} edit links: {ta.count("post-edit-link")}', flush=True)
        if not same:
            la, lb = ta.split('\n'), tb.split('\n')
            shown = 0
            for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, la, lb, autojunk=False).get_opcodes():
                if tag == 'equal' or shown >= lines:
                    continue
                shown += 1
                x, y = '\n'.join(la[i1:i2]), '\n'.join(lb[j1:j2])
                at = next((k for k, (p, q) in enumerate(zip(x, y)) if p != q), min(len(x), len(y)))
                print('   A', repr(x[max(0, at - 90):at + 200]))
                print('   B', repr(y[max(0, at - 90):at + 200]))
    return bad


def main():
    args = sys.argv[1:]
    lines = 6
    if '--diff' in args:
        i = args.index('--diff')
        lines = int(args[i + 1])
        del args[i:i + 2]
    only = args[0] if args else ''
    raw = php(SEED)
    made = json.loads(__import__('base64').b64decode(raw))
    bad = total = 0
    try:
        for name, mods, options, pages in scenarios(made):
            print('==', name, flush=True)
            apply(mods, options)
            if only and only not in name:
                continue
            for label, path, marker in pages:
                total += 1
                bad += not compare(label, path, lines, marker)
        if not only or only == 'signed':
            total += 4
            bad += signed_in(made, lines)
    finally:
        left = php(RESTORE % raw)
        if left.strip() != '0':
            print('posts left:', left)
    print(f'differences: {bad} of {total}')


if __name__ == '__main__':
    main()
