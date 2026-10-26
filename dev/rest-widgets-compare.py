#!/usr/bin/env python3
"""Compares the widget routes (`wp/v2/sidebars`, `wp/v2/widgets`, `wp/v2/widget-types`) of the reference (:8080) and
RustPress (:3001) as rpadmin: status, `Allow` header and body, in every context.

    python dev/rest-widgets-compare.py [--diff]"""
import difflib
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('rc', os.path.join(HERE, 'rest_compare.py'))
rc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rc)

PATHS = [
    '/wp/v2/sidebars', '/wp/v2/sidebars/wp_inactive_widgets', '/wp/v2/sidebars/sidebar-1', '/wp/v2/sidebars/nope',
    '/wp/v2/widgets', '/wp/v2/widgets?sidebar=sidebar-2', '/wp/v2/widgets?sidebar=nope', '/wp/v2/widgets/block-2',
    '/wp/v2/widgets/block-6', '/wp/v2/widgets/pages-1', '/wp/v2/widgets/nope',
    '/wp/v2/widget-types', '/wp/v2/widget-types/block', '/wp/v2/widget-types/recent-posts', '/wp/v2/widget-types/nope',
    '/wp/v2/sidebars?_embed=1', '/wp/v2/widgets?_embed=1', '/wp/v2/widgets?_fields=id,sidebar',
]


def main():
    show = '--diff' in sys.argv
    same = total = 0
    for base in PATHS:
        for context in ('', 'view', 'edit', 'embed'):
            path = base if not context else base + ('&' if '?' in base else '?') + 'context=' + context
            out = []
            for port in (8080, 3001):
                status, headers, body = rc.call(port, 'GET', path, None)
                try:
                    body = json.dumps(rc.normalize(json.loads(body), port), indent=1, ensure_ascii=False)
                except ValueError:
                    pass
                out.append((status, headers.get('Allow'), body))
            total += 1
            if out[0] == out[1]:
                same += 1
                continue
            print('DIFF', path, out[0][:2], out[1][:2])
            if show:
                for line in list(difflib.unified_diff(out[0][2].splitlines(), out[1][2].splitlines(), lineterm='', n=1))[2:30]:
                    print('   ', line[:200])
    print(f'{same}/{total} identical')


if __name__ == '__main__':
    main()
