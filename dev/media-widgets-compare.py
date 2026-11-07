#!/usr/bin/env python3
"""What the media widgets print on the copy of a real site (dev/up-soli.sh, :8081) and on RustPress (dev/serve-soli.ps1,
:3002): whole pages with Twenty Seventeen and the `rendered` of `wp/v2/widgets`. The attachments and widgets are made through
PHP in the copy and put back afterwards.

    python dev/media-widgets-compare.py [--diff N] [substring-of-kind]"""
import base64
import importlib.util
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


sc = load('sc', 'seventeen-compare.py')
wc = load('wc', 'widgets-compare.py')
sys.stdout.reconfigure(encoding='utf-8')

SEED = r'''<?php
require_once ABSPATH . 'wp-admin/includes/image.php';
$made = array( 'posts' => array(), 'images' => array(), 'options' => array() );
foreach ( array( 'widget_media_image', 'widget_media_audio', 'widget_media_video', 'widget_media_gallery', 'sidebars_widgets' ) as $name ) {
	$made['options'][ $name ] = get_option( $name );
}
foreach ( array( array( 2400, 1600 ), array( 1200, 800 ), array( 640, 900 ) ) as $i => $size ) {
	$image = imagecreatetruecolor( $size[0], $size[1] );
	ob_start();
	imagejpeg( $image );
	$file = wp_upload_bits( "rpmw-$i.jpg", null, ob_get_clean() );
	$id   = wp_insert_attachment( array( 'post_title' => "rpmw image $i", 'post_excerpt' => "The excerpt $i", 'post_mime_type' => 'image/jpeg', 'post_status' => 'inherit' ), $file['file'] );
	wp_update_attachment_metadata( $id, wp_generate_attachment_metadata( $id, $file['file'] ) );
	update_post_meta( $id, '_wp_attachment_image_alt', "Stored alt $i" );
	$made['images'][] = $id;
	$made['posts'][]  = $id;
}
foreach ( array( 'audio' => 'audio/mpeg', 'video' => 'video/mp4' ) as $kind => $mime ) {
	$ext = 'audio' === $kind ? 'mp3' : 'mp4';
	$id  = wp_insert_attachment( array( 'post_title' => "rpmw $kind", 'post_mime_type' => $mime, 'post_status' => 'inherit' ), "2026/01/rpmw.$ext" );
	$made[ $kind ]   = $id;
	$made['posts'][] = $id;
}
$made['post']    = wp_insert_post( array( 'post_title' => 'rpmw post', 'post_name' => 'rpmw-post', 'post_status' => 'publish', 'post_author' => 1, 'post_content' => 'Text.' ) );
$made['posts'][] = $made['post'];
echo base64_encode( json_encode( $made ) );
'''

RESTORE = r'''<?php
$made = json_decode( base64_decode( '%s' ), true );
foreach ( $made['options'] as $name => $value ) { update_option( $name, $value ); }
foreach ( $made['posts'] as $id ) { wp_delete_post( $id, true ); }
global $wpdb;
echo $wpdb->get_var( "SELECT COUNT(*) FROM {$wpdb->posts} WHERE post_title LIKE 'rpmw %%'" ), "\n";
'''

SET_WIDGETS = r'''<?php
$sets = json_decode( base64_decode( '%s' ), true );
foreach ( $sets as $name => $value ) { update_option( $name, $value ); }
'''


def scenarios(made):
    image, wide, tall = made['images']
    url = 'http://example.com/a.png'
    return {
        'media_image': [
            {'attachment_id': image, 'title': 'Medium <b>"x"</b> & y', 'size': 'medium', 'link_type': 'none'},
            {'attachment_id': image, 'size': 'full', 'caption': 'A <em>caption</em>', 'link_type': 'file', 'link_classes': 'one two', 'link_rel': 'noopener', 'link_target_blank': True, 'image_title': 'Title', 'alt': 'Alt', 'image_classes': 'c d'},
            {'attachment_id': image, 'size': 'custom', 'width': 100, 'height': 67, 'link_type': 'post'},
            {'attachment_id': image, 'size': 'custom', 'width': 500, 'height': 200, 'caption': 'Custom'},
            {'attachment_id': image, 'size': 'custom', 'width': 0, 'height': 0},
            {'attachment_id': tall, 'size': 'custom', 'width': 5000, 'height': 5000},
            {'attachment_id': image, 'size': 'thumbnail', 'caption': 'Thumb', 'link_type': 'custom', 'link_url': 'http://example.com/x?a=1&b=2'},
            {'attachment_id': wide, 'size': 'bad', 'width': 300, 'height': 0},
            {'attachment_id': image, 'size': 'twentyseventeen-featured-image'},
            {'attachment_id': image, 'size': '1536x1536', 'caption': '   '},
            {'attachment_id': image, 'caption': None, 'size': 'large'},
            {'url': url, 'alt': 'Plain "alt"', 'width': 120, 'height': 80, 'caption': 'Below', 'link_type': 'custom', 'link_url': 'http://example.com/y', 'title': '0', 'image_classes': 'e'},
            {'url': url, 'link_type': 'file', 'caption': '0'},
            {'url': url, 'link_type': 'post', 'width': 50},
            {'attachment_id': made['post'], 'url': url},
            {'attachment_id': made['audio'], 'url': url, 'size': 'medium', 'link_type': 'file'},
            {'attachment_id': made['audio'], 'title': 'No image'},
            {'title': 'Nothing'},
        ],
        'media_audio': [
            {'attachment_id': made['audio'], 'title': 'Audio'},
            {'url': 'http://example.com/a.mp3', 'loop': True, 'preload': 'auto'},
            {'url': 'http://example.com/a.txt', 'title': 'Link'},
            {'attachment_id': made['audio'], 'ogg': 'http://example.com/a.ogg', 'flac': 'http://example.com/a.flac'},
            {'attachment_id': image, 'url': 'http://example.com/b.wav', 'preload': 'metadata'},
            {'attachment_id': image, 'title': 'No file'},
        ],
        'media_video': [
            {'attachment_id': made['video'], 'title': 'Video'},
            {'attachment_id': made['video'], 'loop': True, 'preload': 'none', 'content': '<track src="http://example.com/a.vtt" kind="subtitles">', 'webm': 'http://example.com/a.webm'},
            {'url': 'https://www.youtube.com/watch?v=abc'},
            {'url': 'https://vimeo.com/123456', 'title': 'Vimeo'},
            {'url': 'http://example.com/a.mp4', 'title': 'Printed by nothing'},
            {'attachment_id': image, 'url': 'http://example.com/a.ogv'},
            {'attachment_id': image, 'title': 'No file'},
        ],
        'media_gallery': [
            {'ids': [image, wide, tall], 'title': 'Gallery'},
            {'ids': [tall, image], 'columns': 2, 'size': 'medium', 'link_type': 'file'},
            {'ids': [wide], 'columns': 1, 'size': 'full', 'link_type': 'none'},
            {'ids': [image, made['post']], 'title': 'Not all attachments'},
            {'ids': [], 'title': 'Empty'},
        ],
    }


def main():
    args = sys.argv[1:]
    lines = wc.option(args, '--diff', '6')
    only = args[0] if args else ''
    wc.php(wc.PREPARE)
    raw = sc.php(SEED)
    made = json.loads(base64.b64decode(raw))
    bad = total = 0
    try:
        sides = wc.Side(sc.REFERENCE), wc.Side(sc.RUSTPRESS)
        for base, instances in scenarios(made).items():
            if only and only not in base:
                continue
            print('==', base, flush=True)
            option = {str(i + 2): instance for i, instance in enumerate(instances)}
            option['_multiwidget'] = 1
            ids = [f'{base}-{i + 2}' for i in range(len(instances))]
            sidebars = dict(made['options']['sidebars_widgets'])
            sidebars['sidebar-1'] = ids[:len(ids) // 2 + 1]
            sidebars['sidebar-2'] = ids[len(ids) // 2 + 1:]
            sidebars['sidebar-3'] = []
            sc.php(SET_WIDGETS % base64.b64encode(json.dumps({f'widget_{base}': option, 'sidebars_widgets': sidebars}).encode()).decode())
            import time
            time.sleep(2.5)
            for label, path in (('home', '/'), ('single', f'/?p={made["post"]}'), ('not found', '/?p=99999999')):
                total += 1
                bad += not sc.compare(f'{base} {label}', path, int(lines), f'class="widget {"widget_" + base}')
            answers = [side.rest('GET', '/wp/v2/widgets', 'per_page=100') for side in sides]
            for answer, side in zip(answers, sides):
                print('   rest', answer[0], len(answer[2]), end='')
            print()
            texts = [re.sub(r'localhost(:|%3A|\\u003A)\d+', 'HOST', answer[2]) for answer in answers]
            rows = [{row['id']: row['rendered'] for row in json.loads(text)} for text in texts]
            for key in rows[0]:
                total += 1
                same = rows[0][key] == rows[1].get(key)
                bad += not same
                if not same:
                    print('DIFF rest', key)
                    print('   A', repr(rows[0][key][:600]))
                    print('   B', repr((rows[1].get(key) or '')[:600]))
    finally:
        left = sc.php(RESTORE % raw)
        if left.strip() != '0':
            print('posts left:', left)
        wc.php(wc.RESET)
        wc.php(wc.CLEANUP)
    print(f'differences: {bad} of {total}')


if __name__ == '__main__':
    main()
