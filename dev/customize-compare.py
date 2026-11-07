#!/usr/bin/env python3
"""`wp-admin/customize.php` on the copy of a real site with Twenty Seventeen (dev/up-soli.sh, :8081, against RustPress of
dev/serve-soli.ps1, :3002) and on the reference with its block theme (dev/up.sh, :8080, against dev/serve-admin.ps1,
:3001): the page of the Customizer, compared whole and raw, as an administrator and an editor, in the language of the
site and in English. What is random or depends on the time is masked: the UUID of the changeset, the nonces and the
dates of `_wpCustomizeSettings`.

    python dev/customize-compare.py [--show N] [--only <part of a case>] [--save DIR]"""
import difflib
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('wc', os.path.join(HERE, 'widgets-compare.py'))
wc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wc)

REFERENCE, RUSTPRESS = wc.REFERENCE, wc.RUSTPRESS

CLASSIC, BLOCK = (REFERENCE, RUSTPRESS), (8080, 3001)
CASES = [
    # (name, user, language, address, ports)
    ('admin', 'rpwidget', None, 'customize.php', CLASSIC),
    ('admin en', 'rpwidget', '', 'customize.php', CLASSIC),
    ('admin return', 'rpwidget', None, 'customize.php?return=%2Fwp-admin%2Fthemes.php&url=http%3A%2F%2Flocalhost%3APORT%2F%3Fpage_id%3D24', CLASSIC),
    ('admin autofocus', 'rpwidget', None, 'customize.php?autofocus%5Bsection%5D=title_tagline', CLASSIC),
    ('editor', 'rpwidgeted', None, 'customize.php', CLASSIC),
    ('block theme', 'rpadmin', None, 'customize.php', BLOCK),
    ('block theme editor', 'rpeditor', None, 'customize.php', BLOCK),
]


def norm(text, port):
    for colon in (':', '%3A', '%253A'):
        text = text.replace(f'localhost{colon}{port}', f'localhost{colon}PORT')
    text = re.sub(r'[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}', 'UUID', text)
    text = re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', text)
    text = re.sub(r'"(initialServerDate|publishDate)":"[0-9: -]+"', r'"\1":"DATE"', text)
    text = re.sub(r'"initialServerTimestamp":\d+', '"initialServerTimestamp":0', text)
    return text


def main():
    args = sys.argv[1:]
    show = int(wc.option(args, '--show', '6'))
    only = wc.option(args, '--only', '')
    save = wc.option(args, '--save', '')
    wc.php(wc.PREPARE)
    failures = 0
    try:
        sides = {}
        for name, user, language, address, ports in CASES:
            if only and only not in name:
                continue
            if language is not None:
                wc.set_language(language)
            pages = []
            for port in ports:
                key = (port, user)
                if key not in sides:
                    sides[key] = wc.Side(port, user)
                status, _, body = sides[key].client.request('/wp-admin/' + address.replace('PORT', str(port)))
                pages.append((status, norm(body, port)))
            if language is not None:
                wc.php(wc.RESET)
                wc.set_language(wc.php("<?php echo get_option( 'rp_widgets_lang' );").strip())
            (sa, a), (sb, b) = pages
            if save:
                slug = re.sub(r'\W+', '-', name)
                for side, text in (('ref', a), ('rp', b)):
                    with open(os.path.join(save, f'cz-{slug}-{side}.html'), 'w', encoding='utf-8', newline='') as f:
                        f.write(text)
            if (sa, a) == (sb, b):
                print(f'= {name}')
                continue
            failures += 1
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
    finally:
        wc.php(wc.RESET)
        wc.php(wc.CLEANUP)
    print(f'{len([c for c in CASES if not only or only in c[0]]) - failures} of {len([c for c in CASES if not only or only in c[0]])} equal')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
