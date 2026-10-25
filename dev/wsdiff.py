#!/usr/bin/env python3
"""Compares the outputs `dev/admin-compare.sh` left in $TEMP for a screen, ignoring differences in whitespace.

    python dev/wsdiff.py profile.php [more screens]

Prints the first differences of the markup (tags and text) that are not just a matter of indentation."""
import difflib
import os
import re
import sys

TEMP = os.environ.get('TEMP', '/tmp')


def tokens(path):
    text = open(path, encoding='utf-8', errors='replace').read()
    text = re.sub(r'\s+', ' ', text)
    return [t.strip() for t in re.split(r'(<[^>]*>)', text) if t.strip()]


def main():
    failed = False
    for screen in sys.argv[1:]:
        a = tokens(os.path.join(TEMP, f'ac_ref_{screen}.html'))
        b = tokens(os.path.join(TEMP, f'ac_rp_{screen}.html'))
        ops = [op for op in difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes() if op[0] != 'equal']
        if not ops:
            print(f'same markup  {screen}')
            continue
        failed = True
        print(f'DIFFERENT    {screen}: {len(ops)} places')
        for tag, i1, i2, j1, j2 in ops[:12]:
            print(f'  {tag}: {a[i1:i2][:3]!r} => {b[j1:j2][:3]!r}')
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
