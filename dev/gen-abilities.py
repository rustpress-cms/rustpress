#!/usr/bin/env python3
"""Writes crates/rustpress-rest/src/abilities.json: the abilities and their categories of the reference (:8080) without
the links (the server makes its own).

    python dev/gen-abilities.py"""
import importlib.util
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('rc', os.path.join(HERE, 'rest_compare.py'))
rc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rc)


def get(path):
    status, _, body = rc.call(8080, 'GET', path, None)
    assert status == 200, (path, status)
    return json.loads(body)


abilities = get('/wp-abilities/v1/abilities')
categories = get('/wp-abilities/v1/categories')
for item in abilities + categories:
    item.pop('_links', None)
out = os.path.join(HERE, '..', 'crates', 'rustpress-rest', 'src', 'abilities.json')
with open(out, 'w', encoding='utf-8', newline='\n') as f:
    json.dump({'abilities': abilities, 'categories': categories}, f, ensure_ascii=False, separators=(',', ':'))
print(len(abilities), 'abilities,', len(categories), 'categories')
