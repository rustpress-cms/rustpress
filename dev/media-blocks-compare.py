#!/usr/bin/env python3
"""Compares the media blocks on real pages of the reference (:8080) and RustPress (:3001): `core/image` with the
lightbox, `core/gallery` (lightbox navigation, `dynamicContent`, gaps per viewport), `core/post-featured-image` with
`useFirstImageFromPost`, `core/playlist` and `core/playlist-track`, `[playlist]`. The posts and attachments are made through the
reference's PHP and removed afterwards. `uniqid()` differs by nature and is neutralized.

    python dev/media-blocks-compare.py [--diff N] [substring-of-post]"""
import difflib
import importlib.util
import json
import os
import re
import sys
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('rcc', os.path.join(HERE, 'rest-comments-compare.py'))
rcc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rcc)

SEED = r'''<?php
function rpmb_attachment( $title, $excerpt, $date, $parent ) {
	$file = get_post_meta( 26, '_wp_attached_file', true );
	$meta = wp_get_attachment_metadata( 26 );
	$id   = wp_insert_attachment( array( 'post_title' => $title, 'post_excerpt' => $excerpt, 'post_mime_type' => 'image/webp', 'post_status' => 'inherit', 'post_date' => $date ), false, $parent );
	update_post_meta( $id, '_wp_attached_file', $file );
	wp_update_attachment_metadata( $id, $meta );
	return $id;
}
function rpmb_post( $slug, $content ) {
	return wp_insert_post( array( 'post_title' => $slug, 'post_name' => $slug, 'post_status' => 'publish', 'post_date' => '2026-01-02 03:04:05', 'post_content' => wp_slash( $content ) ) );
}
$src  = wp_get_attachment_url( 26 );
$made = array();

$image = function ( $attrs, $alt = 'Ein Bild', $caption = '' ) use ( $src ) {
	$figcaption = $caption ? '<figcaption class="wp-element-caption">' . $caption . '</figcaption>' : '';
	return '<!-- wp:image ' . wp_json_encode( $attrs ) . ' --><figure class="wp-block-image size-large"><img src="' . $src . '" alt="' . $alt . '" class="wp-image-26"/>' . $figcaption . '</figure><!-- /wp:image -->';
};

$made[] = rpmb_post( 'rpmb-first', '<p>Text</p><figure><img src="http://example.com/a.jpg?x=1&amp;y=2" alt=\'single\' class="a b" data-x=unq data-bare DATA-Up="1" src="dup" style="color:red;" title="&hellip; &quot;q&quot;"/></figure><img src="http://example.com/b.jpg"/>
<!-- wp:post-featured-image {"useFirstImageFromPost":true} /-->
<!-- wp:post-featured-image {"useFirstImageFromPost":true,"isLink":true,"aspectRatio":"4/3","width":"100px","height":"50px","scale":"cover"} /-->
<!-- wp:post-featured-image {"useFirstImageFromPost":false} /-->' );
$made[] = rpmb_post( 'rpmb-first-none', '<p>Keine Bilder</p>
<!-- wp:post-featured-image {"useFirstImageFromPost":true} /-->' );

$made[] = rpmb_post( 'rpmb-lightbox', implode( "\n", array(
	$image( array( 'id' => 26, 'sizeSlug' => 'large', 'lightbox' => array( 'enabled' => true ) ) ),
	$image( array( 'id' => 26, 'lightbox' => array( 'enabled' => true ), 'scale' => 'cover' ), '', 'Unter dem Bild' ),
	$image( array( 'lightbox' => array( 'enabled' => true ) ), 'Ohne ID "q"' ),
	$image( array( 'id' => 26, 'linkDestination' => 'media', 'lightbox' => array( 'enabled' => true ) ) ),
	$image( array( 'id' => 26 ) ),
	$image( array( 'id' => 999999, 'lightbox' => array( 'enabled' => true ) ) ),
	$image( array( 'id' => 26, 'lightbox' => array( 'enabled' => false ) ) ),
) ) );
$made[] = rpmb_post( 'rpmb-no-lightbox', $image( array( 'id' => 26 ) ) );

$made[] = rpmb_post( 'rpmb-lightbox-gallery', '<!-- wp:gallery {"linkTo":"none","navigationButtonType":"text"} --><figure class="wp-block-gallery has-nested-images columns-default is-cropped">'
	. $image( array( 'id' => 26, 'lightbox' => array( 'enabled' => true ) ), 'Erstes' )
	. $image( array( 'id' => 26, 'lightbox' => array( 'enabled' => true ) ), '' )
	. $image( array( 'id' => 26 ), 'Ohne' )
	. '</figure><!-- /wp:gallery -->' );

$made[] = rpmb_post( 'rpmb-gap', '<!-- wp:gallery {"linkTo":"none","style":{"spacing":{"blockGap":"3px"},"@mobile":{"spacing":{"blockGap":"1px"}},"@tablet":{"spacing":{"blockGap":{"left":"2px","top":"9px"}}}}} --><figure class="wp-block-gallery has-nested-images columns-default is-cropped">' . $image( array( 'id' => 26 ) ) . '</figure><!-- /wp:gallery -->
<!-- wp:gallery {"linkTo":"none","style":{"@mobile":{"spacing":{"blockGap":"var:preset|spacing|40"}}}} --><figure class="wp-block-gallery has-nested-images columns-default is-cropped">' . $image( array( 'id' => 26 ) ) . '</figure><!-- /wp:gallery -->' );

$dynamic = rpmb_post( 'rpmb-dynamic', 'x' );
$made[]  = $dynamic;
$a       = rpmb_attachment( 'Bravo', 'Mit <em>Text</em> & mehr', '2026-01-03 10:00:00', $dynamic );
$b       = rpmb_attachment( 'Alpha', '', '2026-01-04 10:00:00', $dynamic );
$c       = rpmb_attachment( 'Charlie', '', '2026-01-02 10:00:00', $dynamic );
$source  = '"dynamicContent":{"source":"core/attached-media"}';
wp_update_post( array( 'ID' => $dynamic, 'post_content' => wp_slash( implode( "\n", array(
	'<!-- wp:gallery {' . $source . '} /-->',
	'<!-- wp:gallery {"dynamicContent":{"source":"core/attached-media","args":{"orderBy":"title","order":"asc"}},"columns":2} --><figcaption class="blocks-gallery-caption wp-element-caption">Galerie</figcaption><!-- /wp:gallery -->',
	'<!-- wp:gallery {' . $source . ',"linkTo":"media","linkTarget":"_blank","sizeSlug":"thumbnail"} /-->',
	'<!-- wp:gallery {' . $source . ',"linkTo":"file"} /-->',
	'<!-- wp:gallery {' . $source . ',"linkTo":"attachment","imageCrop":false,"align":"wide"} /-->',
	'<!-- wp:gallery {' . $source . ',"linkTo":"post","linkTarget":"_blank"} /-->',
	'<!-- wp:gallery {' . $source . ',"linkTo":"lightbox","navigationButtonType":"both"} /-->',
	'<!-- wp:gallery {' . $source . ',"aspectRatio":"16/9"} /-->',
	'<!-- wp:gallery {"dynamicContent":{"source":"core/unknown"}} --><figcaption class="wp-element-caption">Weg</figcaption><!-- /wp:gallery -->',
	'<!-- wp:gallery {"dynamicContent":{"source":"core/attached-media","args":{"orderBy":"menu_order","order":"ASC"}}} /-->',
) ) ) ) );
$made[] = $a;
$made[] = $b;
$made[] = $c;
$made[] = rpmb_post( 'rpmb-dynamic-none', '<!-- wp:gallery {' . $source . '} /-->' );

$track = function ( $attrs ) {
	return '<!-- wp:playlist-track ' . wp_json_encode( $attrs ) . ' /-->';
};
$made[] = rpmb_post( 'rpmb-playlist', implode( "\n", array(
	'<!-- wp:playlist {"waveformStyle":"mirror","showPlayButtonArtwork":true,"waveformColor":"#111","waveformGradient":"a\"b","waveformBackgroundColor":"#222","waveformBackgroundGradient":"x"} --><figure class="wp-block-playlist"><ol class="wp-block-playlist__tracklist">'
	. $track( array( 'id' => 1, 'src' => 'http://example.com/a.mp3?x=1&y=2', 'title' => 'Eins <b>fett</b>', 'artist' => 'Künstler', 'album' => 'Album', 'image' => 'http://example.com/a.jpg', 'imageAlt' => 'Alt "q"', 'length' => '3:21' ) )
	. $track( array( 'title' => 'Ohne ID' ) )
	. $track( array( 'id' => 2, 'src' => 'http://example.com/b.mp3' ) )
	. $track( array( 'id' => 3, 'src' => 'http://example.com/c.mp3', 'title' => '0', 'artist' => 'Nur Name', 'length' => '0' ) )
	. '</ol><figcaption class="wp-element-caption">Liste</figcaption></figure><!-- /wp:playlist -->',
	'<!-- wp:playlist {"showImages":false,"showArtists":false} --><figure class="wp-block-playlist"><ol class="wp-block-playlist__tracklist">'
	. $track( array( 'id' => 4, 'src' => 'http://example.com/d.mp3', 'title' => 'Vier', 'image' => 'http://example.com/d.jpg' ) )
	. '</ol></figure><!-- /wp:playlist -->',
	'<!-- wp:playlist --><figure class="wp-block-playlist"><ol class="wp-block-playlist__tracklist">' . $track( array( 'title' => 'Keine ID' ) ) . '</ol></figure><!-- /wp:playlist -->',
) ) );
$audio_parent = rpmb_post( 'rpmb-shortcode-playlist', 'x' );
$made[]       = $audio_parent;
$audio        = array();
foreach ( array( array( 'Lied Eins', 'artist', 'Band', 'album', 'Platte' ), array( 'Lied <b>Zwei</b>', 'genre', 'Rock', 'year', '1999' ), array( 'Lied Drei', 'length_formatted', '3:21', 'x', 'y' ) ) as $n => $info ) {
	$track = wp_insert_attachment( array( 'post_title' => $info[0], 'post_excerpt' => $n ? '' : 'Ein Text', 'post_mime_type' => 'audio/mpeg', 'post_status' => 'inherit', 'post_date' => '2026-01-0' . ( $n + 1 ) . ' 10:00:00', 'menu_order' => $n ), false, $audio_parent );
	update_post_meta( $track, '_wp_attached_file', '2026/01/lied-' . $n . '.mp3' );
	wp_update_attachment_metadata( $track, array( $info[1] => $info[2], $info[3] => $info[4] ) );
	$audio[] = $track;
	$made[]  = $track;
}
set_post_thumbnail( $audio[1], 26 );
wp_update_post( array( 'ID' => $audio_parent, 'post_content' => implode( "\n\n", array(
	'[playlist]',
	'[playlist tracklist="false" images="false" tracknumbers="no" artists="0" style="dark"]',
	'[playlist ids="' . $audio[2] . ',' . $audio[0] . '"]',
	'[playlist orderby="title" order="DESC" exclude="' . $audio[1] . '"]',
) ) ) );
$made[] = rpmb_post( 'rpmb-shortcode-playlist-video', '[playlist type="video" ids="281"]' );
$made[] = rpmb_post( 'rpmb-shortcode-playlist-none', '[playlist]' );
echo json_encode( $made );
'''
CLEAN = r'''<?php
foreach ( json_decode( '%s', true ) as $id ) { wp_delete_post( $id, true ); }
global $wpdb;
echo $wpdb->get_var( "SELECT COUNT(*) FROM {$wpdb->posts} WHERE post_name LIKE 'rpmb-%%'" ), "\n";
'''

POSTS = ['rpmb-first', 'rpmb-first-none', 'rpmb-lightbox', 'rpmb-no-lightbox', 'rpmb-lightbox-gallery', 'rpmb-gap', 'rpmb-dynamic', 'rpmb-dynamic-none', 'rpmb-playlist', 'rpmb-shortcode-playlist', 'rpmb-shortcode-playlist-video', 'rpmb-shortcode-playlist-none']


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def get(port, path):
    request = urllib.request.Request(f'http://127.0.0.1:{port}{path}', headers={'Host': f'localhost:{port}'})
    try:
        response = urllib.request.build_opener(NoRedirect).open(request, timeout=60)
    except urllib.error.HTTPError as error:
        response = error
    return response.code, response.read().decode('utf-8', 'replace')


def normalize(text, port):
    text = text.replace(f'localhost:{port}', 'HOST').replace('localhost:8080', 'HOST').replace(f'localhost%3A{port}', 'HOST')
    return re.sub(r'\b[0-9a-f]{13}\b', 'UNIQID', text)


def signed_in_compare(lines):
    spec2 = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
    lc = importlib.util.module_from_spec(spec2)
    spec2.loader.exec_module(lc)
    texts = []
    for port in (8080, 3001):
        client = lc.Client(port)
        client.request('/wp-login.php')
        client.request('/wp-login.php', {'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'})
        texts.append(re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', normalize(client.request('/rpmb-shortcode-playlist/')[2], port)).split('\n'))
    blocks = [op for op in difflib.SequenceMatcher(None, texts[0], texts[1], autojunk=False).get_opcodes() if op[0] != 'equal']
    tracks = sum(line.count('"src":') for line in texts[0] if 'wp-playlist-script' in line)
    print(('ok   ' if not blocks else 'DIFF ') + f'rpmb-shortcode-playlist signed in, {tracks} sources', flush=True)
    for _, i1, i2, j1, j2 in blocks[:lines]:
        x, y = '\n'.join(texts[0][i1:i2]), '\n'.join(texts[1][j1:j2])
        at = next((k for k, (p, q) in enumerate(zip(x, y)) if p != q), min(len(x), len(y)))
        print('   A', repr(x[max(0, at - 100):at + 200]))
        print('   B', repr(y[max(0, at - 100):at + 200]))
    return bool(blocks)


def main():
    args = sys.argv[1:]
    lines = 12
    if '--diff' in args:
        i = args.index('--diff')
        lines = int(args[i + 1])
        del args[i:i + 2]
    only = args[0] if args else ''
    ids = rcc.php(SEED)
    json.loads(ids)
    bad = 0
    try:
        for post in POSTS:
            if only not in post:
                continue
            a, b = get(8080, f'/{post}/'), get(3001, f'/{post}/')
            ta, tb = normalize(a[1], 8080), normalize(b[1], 3001)
            same = a[0] == b[0] and ta == tb
            bad += not same
            print(('ok   ' if same else 'DIFF ') + f'{post} [{a[0]} {b[0]}] {len(ta)} {len(tb)}', flush=True)
            if not same:
                la, lb = ta.split('\n'), tb.split('\n')
                shown = 0
                for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, la, lb, autojunk=False).get_opcodes():
                    if tag == 'equal' or shown >= lines:
                        continue
                    shown += 1
                    x, y = '\n'.join(la[i1:i2]), '\n'.join(lb[j1:j2])
                    at = next((k for k, (p, q) in enumerate(zip(x, y)) if p != q), min(len(x), len(y)))
                    print('   A', repr(x[max(0, at - 100):at + 200]))
                    print('   B', repr(y[max(0, at - 100):at + 200]))
        # Without `ids` a playlist needs the `read` capability, so visitors get nothing.
        if only in 'rpmb-shortcode-playlist':
            bad += signed_in_compare(lines)
    finally:
        left = rcc.php(CLEAN % ids)
        if left.strip() != '0':
            print('posts left:', left)
    print('differences:', bad)


if __name__ == '__main__':
    main()
