#!/usr/bin/env python3
"""Calls every GET route of the reference WordPress (:8080) on both servers (as rpadmin) and lists the routes whose
status or top-level shape differ.

    python dev/rest-sweep.py [--only substring] [--shape]

Routes with parameters get sample values (see SAMPLES); routes with other patterns are skipped."""
import importlib.util
import json
import os
import re
import sys

spec = importlib.util.spec_from_file_location('rc', os.path.join(os.path.dirname(__file__), 'rest_compare.py'))
rc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rc)

SAMPLES = {
    'id': '1', 'parent': '1', 'user_id': 'me', 'taxonomy': 'category', 'type': 'post', 'status': 'publish',
    'namespace': 'core', 'name': 'core/paragraph', 'stylesheet': 'twentytwentyfive', 'slug': 'x', 'location': 'primary',
    'plugin': 'hello', 'uuid': 'x', 'font_family_id': '1', 'collection': 'core',
}


def expand(route):
    def repl(m):
        name = m.group(1)
        return SAMPLES.get(name, '1')
    out = re.sub(r'\(\?P<(\w+)>[^()]*(?:\([^()]*\)[^()]*)*\)', repl, route)
    return out if '(' not in out and '[' not in out else None


def main():
    only = None
    args = sys.argv[1:]
    if '--only' in args:
        only = args[args.index('--only') + 1]
    routes = json.load(__import__('urllib.request').request.urlopen('http://localhost:8080/wp-json/'))['routes']
    for route in sorted(routes):
        if not any('GET' in e['methods'] for e in routes[route]['endpoints']):
            continue
        path = expand(route)
        if path is None or (only and only not in path):
            continue
        a = rc.call(8080, 'GET', path, None)
        b = rc.call(3001, 'GET', path, None)
        note = ''
        if a[0] != b[0]:
            note = f'status {a[0]} vs {b[0]}'
        else:
            try:
                ja, jb = json.loads(a[2]), json.loads(b[2])
                if '--shape' in args:
                    if rc.shape(ja) != rc.shape(jb):
                        note = 'shape'
                elif rc.normalize(ja, 8080) != rc.normalize(jb, 3001):
                    note = 'body'
            except ValueError:
                if a[2] != b[2]:
                    note = 'text'
        print(('SAME ' if not note else 'DIFF ') + path + (' ' + note if note else ''), flush=True)


if __name__ == '__main__':
    main()
