#!/usr/bin/env python3
"""Short report of the head differences of many screens: python dev/head_summary.py [screens...] (default: the admin menu)."""
import difflib
import re
import sys
sys.path.insert(0, __file__.rsplit('/', 1)[0] if '/' in __file__ else '.')
import head_compare as h

DEFAULT = ['index.php', 'edit.php', 'post-new.php', 'edit.php?post_type=page', 'post-new.php?post_type=page', 'upload.php', 'media-new.php', 'edit-comments.php', 'edit-tags.php?taxonomy=category',
           'edit-tags.php?taxonomy=post_tag', 'themes.php', 'site-editor.php', 'font-library.php', 'plugins.php', 'plugin-install.php', 'users.php', 'user-new.php', 'profile.php', 'tools.php', 'import.php',
           'export.php', 'site-health.php', 'options-general.php', 'options-writing.php', 'options-reading.php', 'options-discussion.php', 'options-media.php', 'options-permalink.php', 'options-privacy.php',
           'update-core.php', 'about.php', 'edit.php?post_type=book', 'theme-install.php', 'authorize-application.php', 'privacy-policy-guide.php']


def short(item):
    m = re.search(r"pagenow = '([^']*)', typenow = '([^']*)', adminpage = '([^']*)'", item)
    if m:
        return 'pagenow=%s typenow=%s adminpage=%s' % m.groups()
    return item[:140]


def main():
    pages = [a for a in sys.argv[1:] if not a.startswith('--')] or DEFAULT
    for page in pages:
        try:
            a = h.elements(h.fetch(8080, page), 8080)
            b = h.elements(h.fetch(3001, page), 3001)
        except Exception as e:  # noqa
            print(f'ERROR      {page}: {e}')
            continue
        ops = [op for op in difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes() if op[0] != 'equal']
        if not ops:
            print(f'same       {page}')
            continue
        print(f'DIFFERENT  {page}')
        for tag, i1, i2, j1, j2 in ops:
            if a[i1:i2] and b[j1:j2] and a[i1].startswith('<body') and b[j1].startswith('<body'):
                ca, cb = a[i1].split(), b[j1].split()
                print('    body: ref only %s, rp only %s, order %s' % ([c for c in ca if c not in cb], [c for c in cb if c not in ca], 'same' if [c for c in ca if c in cb] == [c for c in cb if c in ca] else 'DIFFERENT'))
                continue
            for item in a[i1:i2]:
                print('    ref: ' + short(item))
            for item in b[j1:j2]:
                print('    rp : ' + short(item))


main()
