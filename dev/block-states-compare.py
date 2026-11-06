#!/usr/bin/env python3
"""Compares block visibility, block states and the viewport values of the layout support on real pages of the
reference (:8080) and RustPress (:3001): `block-visibility.php`, `states.php` and `layout.php`. The posts are made
through the reference's PHP and removed afterwards.

    python dev/block-states-compare.py [--diff N] [substring-of-post]"""
import difflib
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('mbc', os.path.join(HERE, 'media-blocks-compare.py'))
mbc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mbc)

GROUP = '<!-- wp:group %s --><div class="wp-block-group"><!-- wp:paragraph --><p>Eins</p><!-- /wp:paragraph --><!-- wp:paragraph --><p>Zwei</p><!-- /wp:paragraph --></div><!-- /wp:group -->'
BUTTON = '<!-- wp:buttons --><div class="wp-block-buttons"><!-- wp:button %s --><div class="wp-block-button"><a class="wp-block-button__link wp-element-button">Los</a></div><!-- /wp:button --></div><!-- /wp:buttons -->'
PARAGRAPH = '<!-- wp:paragraph %s --><p>Text mit <a href="http://example.com/">Link</a></p><!-- /wp:paragraph -->'
IMAGE = '<!-- wp:image %s --><figure class="wp-block-image"><img src="http://example.com/a.jpg" alt=""/></figure><!-- /wp:image -->'


def attrs(value):
    return json.dumps(value, separators=(',', ':'))


def group(value):
    return GROUP % attrs(value)


def viewport(**sizes):
    return {'metadata': {'blockVisibility': {'viewport': sizes}}}


POSTS = {
    'rpbs-hidden': [
        PARAGRAPH % attrs({'metadata': {'blockVisibility': False}}),
        PARAGRAPH % attrs({}),
        group({'metadata': {'blockVisibility': False}}),
    ],
    'rpbs-visibility': [
        PARAGRAPH % attrs(viewport(mobile=False)),
        PARAGRAPH % attrs(viewport(tablet=False, mobile=False)),
        PARAGRAPH % attrs(viewport(desktop=False)),
        PARAGRAPH % attrs(viewport(mobile=True)),
        PARAGRAPH % attrs(viewport(huge=False)),
        PARAGRAPH % attrs({'metadata': {'blockVisibility': {'viewport': {}}}}),
        IMAGE % attrs(viewport(mobile=False)),
        IMAGE % attrs({}),
        group(viewport(desktop=False)),
    ],
    'rpbs-states': [
        BUTTON % attrs({'style': {':hover': {'color': {'text': '#ff0000', 'background': 'var:preset|color|accent-1'}}}}),
        BUTTON % attrs({'style': {':hover': {'border': {'color': '#00ff00', 'width': '2px'}}, ':focus': {'color': {'background': '#111111'}}}}),
        BUTTON % attrs({'style': {':active': {'border': {'top': {'color': 'red'}, 'left': {'width': '1px', 'style': 'dashed'}}}, ':focus-visible': {'typography': {'textAlign': 'center'}}}}),
        BUTTON % attrs({'style': {':link': {'color': {'text': 'red'}}}}),
        PARAGRAPH % attrs({'style': {':hover': {'color': {'text': 'red'}}}}),
        BUTTON % attrs({'style': {'@mobile': {':hover': {'color': {'text': 'blue'}}, 'spacing': {'padding': {'top': '1px'}}}}}),
    ],
    'rpbs-viewport': [
        PARAGRAPH % attrs({'style': {'@mobile': {'color': {'text': '#123456'}, 'spacing': {'margin': {'top': '10px'}}}}}),
        PARAGRAPH % attrs({'style': {'@tablet': {'typography': {'fontSize': '20px', 'textAlign': 'right'}}}}),
        PARAGRAPH % attrs({'style': {'@mobile': {'elements': {'link': {'color': {'text': 'red'}, ':hover': {'color': {'text': 'green'}}}, 'h1': {'color': {'text': 'blue'}}, 'nope': {'color': {'text': 'blue'}}}}}}),
        group({'style': {'@mobile': {'color': {'background': '#eeeeee'}, 'layout': {'type': 'flex'}}, '@tablet': {'border': {'radius': '4px', 'color': 'red'}}}}),
        IMAGE % attrs({'style': {'@mobile': {'border': {'radius': '9px'}, 'color': {'duotone': 'var:preset|duotone|x'}}}}),
        group({'style': {'@desktop': {'color': {'text': 'red'}}}}),
    ],
    'rpbs-custom-css': [
        PARAGRAPH % attrs({'style': {'css': 'color:red;& a{color:blue}&::before{content:"x"}'}}),
        PARAGRAPH % attrs({'className': 'mine', 'style': {'css': 'font-size:2em;', 'color': {'text': '#333333'}}}),
        PARAGRAPH % attrs({'style': {'css': 'color:red;'}, 'data': 2}),
        group({'style': {'css': '& p{margin:0}'}, 'layout': {'type': 'constrained'}}),
        '<!-- wp:group {"layout":{"type":"flex"}} --><div class="wp-block-group">' + (PARAGRAPH % attrs({'style': {'css': 'color:green;'}})) + '</div><!-- /wp:group -->',
        (PARAGRAPH % attrs({'style': {'css': '</style><b>x'}})).replace('</style><b>', '\\u003c/style\\u003e\\u003cb\\u003e'),
        PARAGRAPH % attrs({'style': {'css': '  '}}),
    ],
    'rpbs-layout-flex': [
        group({'layout': {'type': 'flex'}, 'style': {'@mobile': {'layout': {'orientation': 'vertical'}}}}),
        group({'layout': {'type': 'flex', 'justifyContent': 'center'}, 'style': {'@mobile': {'layout': {'justifyContent': 'right'}}, '@tablet': {'layout': {'flexWrap': 'nowrap'}}}}),
        group({'layout': {'type': 'flex', 'orientation': 'vertical'}, 'style': {'@mobile': {'layout': {'verticalAlignment': 'bottom'}}, 'spacing': {'blockGap': '3px'}}}),
        group({'layout': {'type': 'flex', 'flexWrap': 'nowrap'}, 'style': {'@mobile': {'spacing': {'blockGap': {'top': '1px', 'left': '2px'}}}, '@tablet': {'spacing': {'blockGap': 'var:preset|spacing|30'}}}}),
    ],
    'rpbs-layout-constrained': [
        group({'layout': {'type': 'constrained', 'contentSize': '500px'}, 'style': {'@mobile': {'layout': {'contentSize': '300px'}}}}),
        group({'layout': {'type': 'constrained', 'contentSize': '500px', 'wideSize': '700px'}, 'style': {'@mobile': {'layout': {'contentSize': None}}, '@tablet': {'layout': {'wideSize': None}}}}),
        group({'layout': {'type': 'constrained', 'justifyContent': 'left'}, 'style': {'@mobile': {'layout': {'justifyContent': 'right'}}}}),
        group({'layout': {'type': 'constrained', 'contentSize': '500px'}, 'style': {'@tablet': {'layout': {'justifyContent': 'left', 'wideSize': '900px'}}, '@mobile': {'spacing': {'blockGap': '7px'}}}}),
        group({'layout': {'type': 'default'}, 'style': {'@mobile': {'spacing': {'blockGap': '7px'}}}}),
    ],
    'rpbs-layout-grid': [
        group({'layout': {'type': 'grid', 'columnCount': 3}, 'style': {'@mobile': {'layout': {'columnCount': 1}}}}),
        group({'layout': {'type': 'grid', 'minimumColumnWidth': '10rem'}, 'style': {'@mobile': {'layout': {'minimumColumnWidth': '5rem'}}, '@tablet': {'layout': {'autoFit': True}}}}),
        group({'layout': {'type': 'grid', 'columnCount': 3, 'minimumColumnWidth': '10rem'}, 'style': {'@mobile': {'spacing': {'blockGap': '1px'}}, 'spacing': {'blockGap': '2px'}}}),
        group({'layout': {'type': 'grid', 'columnCount': 2, 'rowCount': 2}, 'style': {'@tablet': {'layout': {'rowCount': 4}}}}),
        group({'layout': {'type': 'grid', 'columnCount': 2}, 'style': {'@mobile': {'layout': {'minimumColumnWidth': '4rem', 'columnCount': None}}}}),
    ],
    'rpbs-layout-child': [
        '<!-- wp:group {"layout":{"type":"flex"}} --><div class="wp-block-group">' + ''.join((PARAGRAPH % attrs(a)) for a in [
            {'style': {'layout': {'selfStretch': 'fixed', 'flexSize': '100px'}, '@mobile': {'layout': {'selfStretch': 'fit'}}}},
            {'style': {'layout': {'selfStretch': 'fixedNoShrink', 'flexSize': '100px'}, '@mobile': {'layout': {'selfStretch': 'fill'}}, '@tablet': {'layout': {'selfStretch': 'fixed'}}}},
            {'style': {'@mobile': {'layout': {'selfStretch': 'fixed', 'flexSize': '50px'}}}},
            {'style': {'layout': {'selfStretch': 'fill'}, '@tablet': {'layout': {'selfStretch': 'fit'}}}},
        ]) + '</div><!-- /wp:group -->',
        '<!-- wp:group {"layout":{"type":"grid","columnCount":3}} --><div class="wp-block-group">' + ''.join((PARAGRAPH % attrs(a)) for a in [
            {'style': {'layout': {'columnSpan': 2, 'rowSpan': 1}, '@mobile': {'layout': {'columnSpan': 1}}}},
            {'style': {'layout': {'columnStart': 2}, '@tablet': {'layout': {'columnStart': 1, 'rowStart': 2}}}},
            {'style': {'@mobile': {'layout': {'rowSpan': 2}}}},
        ]) + '</div><!-- /wp:group -->',
    ],
}


def saved_css_compare():
    """`wp_strip_custom_css_from_blocks()`: what users without `edit_css` save keeps no `style.css`."""
    spec2 = importlib.util.spec_from_file_location('rcw', os.path.join(HERE, 'rest-comments-write-compare.py'))
    rcw = importlib.util.module_from_spec(spec2)
    spec2.loader.exec_module(rcw)
    seed = "<?php\necho wp_insert_user( array( 'user_login' => 'rpbsauthor', 'user_pass' => 'rustpress-dev', 'user_email' => 'rpbsauthor@example.com', 'role' => 'author' ) );\n"
    clean = "<?php\nrequire_once ABSPATH . 'wp-admin/includes/user.php';\nglobal $wpdb;\nforeach ( $wpdb->get_col( \"SELECT ID FROM {$wpdb->posts} WHERE post_title = 'rpbs-saved'\" ) as $id ) { wp_delete_post( $id, true ); }\nwp_delete_user( %s );\n"
    content = (PARAGRAPH % attrs({'style': {'css': 'color:red;'}})) + (PARAGRAPH % attrs({'style': {'css': 'x', 'color': {'text': '#333333'}}, 'className': 'k'})) + (PARAGRAPH % attrs({}))
    user = mbc.rcc.php(seed)
    bad = 0
    try:
        saved = {}
        for login in ('rpbsauthor', 'rpadmin'):
            for port in (8080, 3001):
                status, text = rcw.Site(port, login).send('POST', '/wp/v2/posts', {'title': 'rpbs-saved', 'status': 'draft', 'content': content})
                saved[login, port] = (status, json.loads(text).get('content', {}).get('raw') if text.startswith('{') else text)
            same = saved[login, 8080] == saved[login, 3001]
            bad += not same
            print(('ok   ' if same else 'DIFF ') + f'saved as {login}: {saved[login, 8080][0]}, css left: {str(saved[login, 8080][1]).count(chr(34) + "css" + chr(34))}', flush=True)
            if not same:
                x, y = str(saved[login, 8080][1]), str(saved[login, 3001][1])
                at = next((k for k, (p, q) in enumerate(zip(x, y)) if p != q), min(len(x), len(y)))
                print('   A', repr(x[max(0, at - 100):at + 120]))
                print('   B', repr(y[max(0, at - 100):at + 120]))
    finally:
        mbc.rcc.php(clean % user)
    return bad


def main():
    args = sys.argv[1:]
    lines = 12
    if '--diff' in args:
        i = args.index('--diff')
        lines = int(args[i + 1])
        del args[i:i + 2]
    only = args[0] if args else ''
    code = "<?php\nwp_custom_css_remove_filters();\n$made = array();\nforeach ( json_decode( base64_decode( '%s' ), true ) as $slug => $blocks ) {\n\t$made[] = wp_insert_post( array( 'post_title' => $slug, 'post_name' => $slug, 'post_status' => 'publish', 'post_date' => '2026-01-02 03:04:05', 'post_content' => wp_slash( implode( \"\\n\", $blocks ) ) ) );\n}\necho json_encode( $made );\n"
    import base64
    ids = mbc.rcc.php(code % base64.b64encode(json.dumps(POSTS).encode()).decode())
    json.loads(ids)
    bad = 0
    try:
        for post in POSTS:
            if only not in post:
                continue
            a, b = mbc.get(8080, f'/{post}/'), mbc.get(3001, f'/{post}/')
            ta, tb = mbc.normalize(a[1], 8080), mbc.normalize(b[1], 3001)
            same = a[0] == b[0] and ta == tb
            bad += not same
            seen = {name: ta.count(name) for name in ('wp-states-', 'wp-block-hidden-', '@media (width', 'wp-custom-css-')}
            print(('ok   ' if same else 'DIFF ') + f'{post} [{a[0]} {b[0]}] {len(ta)} {len(tb)} {seen}', flush=True)
            if not same:
                la, lb = ta.split('\n'), tb.split('\n')
                shown = 0
                for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, la, lb, autojunk=False).get_opcodes():
                    if tag == 'equal' or shown >= lines:
                        continue
                    shown += 1
                    x, y = '\n'.join(la[i1:i2]), '\n'.join(lb[j1:j2])
                    at = next((k for k, (p, q) in enumerate(zip(x, y)) if p != q), min(len(x), len(y)))
                    print('   A', repr(x[max(0, at - 110):at + 210]))
                    print('   B', repr(y[max(0, at - 110):at + 210]))
    finally:
        left = mbc.rcc.php(mbc.CLEAN.replace('rpmb-', 'rpbs-') % ids)
        if left.strip() != '0':
            print('posts left:', left)
    if not only:
        bad += saved_css_compare()
    print('differences:', bad)


if __name__ == '__main__':
    main()
