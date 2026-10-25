#!/usr/bin/env python3
"""Diffs the block editor bootstrap of the reference (:8080) and RustPress (:3001).

    python3 dev/editor_diff.py [post.php?post=1&action=edit]

Compares the settings object of `initializeEditor( 'editor', type, id, {settings}, initialEdits )` and the keys of the
preloaded REST data. The pages are saved to $TEMP/ed_ref.html and $TEMP/ed_rp.html."""
import http.cookiejar, json, os, re, sys, urllib.parse, urllib.request

PAGE = sys.argv[1] if len(sys.argv) > 1 else "post.php?post=1&action=edit"
TEMP = os.environ.get("TEMP", "/tmp")


def fetch(port):
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    opener.open(f"http://127.0.0.1:{port}/wp-login.php").read()
    data = urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()
    opener.open(f"http://127.0.0.1:{port}/wp-login.php", data).read()
    return opener.open(f"http://127.0.0.1:{port}/wp-admin/{PAGE}").read().decode("utf-8")


def settings(html):
    start = html.index("initializeEditor(")
    start = html.index("{", start)
    obj, end = json.JSONDecoder().raw_decode(html[start:])
    return obj


def norm(v, port):
    if isinstance(v, dict):
        return {k: norm(x, port) for k, x in v.items()}
    if isinstance(v, list):
        return [norm(x, port) for x in v]
    if isinstance(v, str):
        v = re.sub(r"localhost:(8080|3001)", "localhost:PORT", v)
        v = re.sub(r"127\.0\.0\.1:(8080|3001)", "localhost:PORT", v)
        v = re.sub(r"\b[0-9a-f]{10}\b", "NONCE", v)
        return v
    return v


def diff(a, b, path=""):
    out = []
    if isinstance(a, dict) and isinstance(b, dict):
        for k in a:
            if k not in b:
                out.append(f"- {path}/{k}")
            else:
                out += diff(a[k], b[k], f"{path}/{k}")
        for k in b:
            if k not in a:
                out.append(f"+ {path}/{k}")
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            out.append(f"~ {path}: list length {len(a)} vs {len(b)}")
        for i, (x, y) in enumerate(zip(a, b)):
            out += diff(x, y, f"{path}[{i}]")
    elif a != b:
        out.append(f"~ {path}: {json.dumps(a)[:160]} vs {json.dumps(b)[:160]}")
    return out


ref_html, rp_html = fetch(8080), fetch(3001)
open(f"{TEMP}/ed_ref.html", "w", encoding="utf-8").write(ref_html)
open(f"{TEMP}/ed_rp.html", "w", encoding="utf-8").write(rp_html)
ref, rp = norm(settings(ref_html), 8080), norm(settings(rp_html), 3001)
lines = diff(ref, rp)
print("\n".join(lines) if lines else "settings identical")
