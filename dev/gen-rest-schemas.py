#!/usr/bin/env python3
"""Writes crates/rustpress-rest/src/route_schemas.json: the `schema` of `OPTIONS` for every route of the index of the
reference (:8080), as rpadmin. Routes with the same schema share one entry; the address of the REST API in the
schemas is replaced by "@REST@" (the server puts its own back).

    python dev/gen-rest-schemas.py"""
import importlib.util
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('rc', os.path.join(HERE, 'rest_compare.py'))
rc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rc)

BASE = 'http://localhost:8080/wp-json'
ROUTES = json.load(open(os.path.join(HERE, '..', 'crates', 'rustpress-rest', 'src', 'index_routes.json'), encoding='utf-8'))
SAMPLES = {'id': ['1', 'twentytwentyfive//index', 'core/paragraph', 'x'], 'parent': ['1', 'twentytwentyfive//index'], 'stylesheet': ['twentytwentyfive'], 'uuid': ['x'],
           'name': ['paragraph', 'core/paragraph', 'x'], 'namespace': ['core', 'x'], 'slug': ['x'], 'location': ['x'], 'taxonomy': ['category'], 'type': ['post']}


def parts_of(route):
    """The literal text and the named groups `(name, None)` of a route pattern (groups may nest)."""
    out, i, literal = [], 0, ''
    while i < len(route):
        m = re.match(r'\(\?P<(\w+)>', route[i:])
        if m:
            depth, j = 0, i
            while j < len(route):
                if route[j] == '\\':
                    j += 2
                    continue
                if route[j] == '(':
                    depth += 1
                elif route[j] == ')':
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            out.append(literal)
            literal = ''
            out.append((m.group(1), None))
            i = j + 1
        else:
            literal += route[i]
            i += 1
    out.append(literal)
    return out


def sample(route):
    """A concrete path the pattern matches, or None."""
    pattern = re.compile('^' + route + '$')
    paths = ['']
    for part in parts_of(route):
        if isinstance(part, tuple):
            paths = [p + s for p in paths for s in SAMPLES.get(part[0], ['x', '1', 'core/paragraph'])]
        else:
            paths = [p + part for p in paths]
    for path in paths:
        if pattern.match(path):
            return path
    return None


def replace_base(value):
    if isinstance(value, dict):
        return {k: replace_base(v) for k, v in value.items()}
    if isinstance(value, list):
        return [replace_base(v) for v in value]
    if isinstance(value, str) and value.startswith(BASE):
        return '@REST@' + value[len(BASE):]
    return value


def main():
    schemas, index, missing = [], {}, []
    for route in ROUTES:
        if route == '/':
            continue
        path = sample(route)
        if path is None:
            missing.append(route)
            continue
        status, headers, body = rc.call(8080, 'OPTIONS', path, None)
        try:
            data = json.loads(body)
        except ValueError:
            missing.append(route)
            continue
        if status != 200 or not isinstance(data, dict):
            missing.append(f'{route} ({status})')
            continue
        if 'schema' not in data:
            # The route has no schema: its OPTIONS answer has none either.
            index[route] = None
            continue
        schema = replace_base(data['schema'])
        if schema not in schemas:
            schemas.append(schema)
        index[route] = schemas.index(schema)
    out = os.path.join(HERE, '..', 'crates', 'rustpress-rest', 'src', 'route_schemas.json')
    with open(out, 'w', encoding='utf-8', newline='\n') as f:
        json.dump({'schemas': schemas, 'routes': index}, f, ensure_ascii=False, separators=(',', ':'))
    print(len(index), 'routes,', len(schemas), 'schemas,', os.path.getsize(out), 'bytes')
    for m in missing:
        print('missing', m)


if __name__ == '__main__':
    main()
