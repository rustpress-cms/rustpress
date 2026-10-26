#!/usr/bin/env python3
"""Shows the differing fields of two list answers item by item, with a character diff of the first change.

    python dev/rest-detail.py /wp/v2/block-patterns/patterns [key-field]"""
import difflib
import importlib.util
import json
import os
import sys

spec = importlib.util.spec_from_file_location('rc', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'rest_compare.py'))
rc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rc)

path = sys.argv[1]
key = sys.argv[2] if len(sys.argv) > 2 else 'name'
a = json.loads(rc.call(8080, 'GET', path, None)[2])
b = json.loads(rc.call(3001, 'GET', path, None)[2])
print(len(a), len(b))
for i, (x, y) in enumerate(zip(a, b)):
    if x == y:
        continue
    for k in x:
        if x.get(k) != y.get(k):
            sa, sb = json.dumps(x.get(k), ensure_ascii=False), json.dumps(y.get(k), ensure_ascii=False)
            for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, sa, sb, autojunk=False).get_opcodes():
                if tag != 'equal':
                    print(i, x.get(key), k, tag, repr(sa[max(0, i1 - 50):i2 + 50]), '=>', repr(sb[max(0, j1 - 50):j2 + 50]))
                    break
