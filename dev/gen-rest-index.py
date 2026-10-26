#!/usr/bin/env python3
"""Writes crates/rustpress-rest/src/index_routes.json: the `routes` of GET /wp-json/ of the reference (:8080), with the
address of the REST API in the links replaced by "@REST@" (the server puts its own back).

    python dev/gen-rest-index.py"""
import json
import os
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
BASE = 'http://localhost:8080/wp-json'

data = json.load(urllib.request.urlopen(BASE + '/'))
routes = data['routes']
for route in routes.values():
    for link in route.get('_links', {}).get('self', []):
        href = link['href']
        assert href.startswith(BASE), href
        link['href'] = '@REST@' + href[len(BASE):]
out = os.path.join(HERE, '..', 'crates', 'rustpress-rest', 'src', 'index_routes.json')
with open(out, 'w', encoding='utf-8', newline='\n') as f:
    json.dump(routes, f, ensure_ascii=False, separators=(',', ':'))
print(len(routes), 'routes,', os.path.getsize(out), 'bytes')
