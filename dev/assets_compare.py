#!/usr/bin/env python3
"""Compares the scripts and stylesheets a wp-admin screen loads on the reference (:8080) and RustPress (:3001).

    python3 dev/assets_compare.py index.php edit.php "edit.php?post_type=page"

The normalizer of dev/admin-compare.sh drops scripts; this one looks at nothing else: the handles in the order
of the page, the files of the chunks `load-scripts.php` / `load-styles.php`, and the inline scripts by id."""
import http.cookiejar
import re
import sys
import urllib.parse
import urllib.request


def fetch(port, page):
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    opener.open(f"http://127.0.0.1:{port}/wp-login.php").read()
    data = urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()
    opener.open(f"http://127.0.0.1:{port}/wp-login.php", data).read()
    return opener.open(f"http://127.0.0.1:{port}/wp-admin/{page}").read().decode("utf-8", "replace")


def items(html):
    out = []
    for m in re.finditer(r"<(script|link|style)\b([^>]*)>", html):
        tag, attrs = m.group(1), m.group(2)
        if tag == "link" and "stylesheet" not in attrs:
            continue
        ident = re.search(r"""\bid=["']([^"']*)["']""", attrs)
        src = re.search(r"""\b(?:src|href)=["']([^"']*)["']""", attrs)
        name = ident.group(1) if ident else ""
        if src:
            url = re.sub(r"https?://[^/]+", "", src.group(1)).replace("&#038;", "&").replace("&amp;", "&")
            url = re.sub(r"[?&]ver=[^&]*", "", url)
            if "load-scripts.php" in url or "load-styles.php" in url:
                chunk = re.search(r"load(?:%5B|\[)chunk_\d+(?:%5D|\])=([^&]*)", url)
                url = url.split("?")[0] + " " + (chunk.group(1) if chunk else "")
            out.append(f"{tag} {name} {url}".strip())
        elif tag == "script" and name:
            out.append(f"{tag} {name}")
    return out


def inline_bodies(html):
    """The inline scripts by id, with nonces, hosts and times neutralized."""
    out = {}
    for m in re.finditer(r"<script([^>]*)>(.*?)</script>", html, re.S):
        ident = re.search(r"""id=["']([^"']*)["']""", m.group(1))
        if not ident or "src=" in m.group(1):
            continue
        body = m.group(2)
        body = re.sub(r"(127\.0\.0\.1|localhost):(8080|3001)", "HOST", body)
        body = re.sub(r'[0-9a-f]{10}', "NONCE", body)
        body = re.sub(r"\d{4}-\d{2}-\d{2}T[\d:.]+Z", "TIME", body)
        body = re.sub(r"\s+", " ", body).strip()
        out[ident.group(1)] = body
    return out


def main():
    inline = "--inline" in sys.argv
    screens = [a for a in sys.argv[1:] if not a.startswith("--")] or ["index.php"]
    status = 0
    for screen in screens:
        ref_html, rp_html = fetch(8080, screen), fetch(3001, screen)
        ref, rp = items(ref_html), items(rp_html)
        if inline:
            a, b = inline_bodies(ref_html), inline_bodies(rp_html)
            for key in a:
                if key in b and a[key] != b[key]:
                    status = 1
                    print(f"  ~ inline {key} of {screen} differs")
                    for i, (x, y) in enumerate(zip(a[key], b[key])):
                        if x != y:
                            print(f"      reference: ...{a[key][max(0, i - 40):i + 80]}")
                            print(f"      RustPress: ...{b[key][max(0, i - 40):i + 80]}")
                            break
                    else:
                        print(f"      lengths {len(a[key])} vs {len(b[key])}")
        if ref == rp:
            print(f"identical  {screen} ({len(ref)} assets)")
            continue
        status = 1
        print(f"DIFFERENT  {screen}")
        only_ref = [x for x in ref if x not in rp]
        only_rp = [x for x in rp if x not in ref]
        for x in only_ref:
            print("  - only in the reference:", x)
        for x in only_rp:
            print("  + only in RustPress:   ", x)
        common_ref = [x for x in ref if x in rp]
        common_rp = [x for x in rp if x in ref]
        if common_ref != common_rp:
            for i, (a, b) in enumerate(zip(common_ref, common_rp)):
                if a != b:
                    print(f"  ~ order differs at {i}: reference {a!r}, RustPress {b!r}")
                    break
    sys.exit(status)


main()
