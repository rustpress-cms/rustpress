#!/usr/bin/env python3
"""Saves the same permalink form on the reference WordPress (:8080) and on RustPress (:3001) and compares what
both leave in the options (structure, category and tag base, rewrite rules) and the redirect. Both use the same
scratch database; the reference gets the baseline form at the end.

    python dev/permalink-save-compare.py
"""
import http.cookiejar, os, re, subprocess, sys, time, urllib.error, urllib.parse, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
BASE = {"selection": "/%postname%/", "permalink_structure": "/%postname%/", "category_base": "", "tag_base": ""}
CASES = [
    {"selection": "/%year%/%monthnum%/%day%/%postname%/", "permalink_structure": "/%postname%/", "category_base": "", "tag_base": ""},
    {"selection": "custom", "permalink_structure": "blog//%category%/%postname%", "category_base": "themen/", "tag_base": "#schlagwort"},
    {"selection": "/archives/%post_id%", "permalink_structure": "x", "category_base": "", "tag_base": ""},
    {"selection": "custom", "permalink_structure": "/index.php/%postname%/", "category_base": "rubrik", "tag_base": ""},
    {"selection": "", "permalink_structure": "", "category_base": "", "tag_base": ""},
    {"selection": "custom", "permalink_structure": "/foo/", "category_base": "/index.php/x/", "tag_base": ""},
]


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


def save(port, form):
    jar = http.cookiejar.CookieJar()
    op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    op2 = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar), NoRedirect)
    op.open(f"http://127.0.0.1:{port}/wp-login.php").read()
    op.open(f"http://127.0.0.1:{port}/wp-login.php", urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()).read()
    page = op.open(f"http://127.0.0.1:{port}/wp-admin/options-permalink.php").read().decode()
    nonce = re.search(r'name="_wpnonce" value="([0-9a-f]+)"', page).group(1)
    data = {"_wpnonce": nonce, "_wp_http_referer": "/wp-admin/options-permalink.php", "submit": "Save Changes", **form}
    try:
        r = op2.open(f"http://127.0.0.1:{port}/wp-admin/options-permalink.php", urllib.parse.urlencode(data).encode())
        return f"{r.status}"
    except urllib.error.HTTPError as e:
        loc = (e.headers.get("Location") or "").replace(f"localhost:{port}", "HOST")
        # WordPress flushes the rules when the screen is shown again.
        op.open(f"http://127.0.0.1:{port}/wp-admin/options-permalink.php?settings-updated=true").read()
        return f"{e.code} {loc}"


def snapshot():
    sql = "select option_name, option_value, autoload from wp_options where option_name in ('permalink_structure','category_base','tag_base','rewrite_rules','_transient_settings_errors') order by option_name"
    out = subprocess.run(["powershell.exe", "-NoProfile", "-File", os.path.join(HERE, "sql.ps1"), "wordpress", sql], capture_output=True, text=True, encoding="utf-8").stdout
    return out.replace("\r", "")


failed = False
try:
    for n, form in enumerate(CASES, 1):
        print(f"== {n}: {form}")
        ref_redirect = save(8080, form)
        ref = snapshot()
        save(8080, BASE)
        time.sleep(3)  # RustPress reads the site again every second
        rp_redirect = save(3001, form)
        rp = snapshot()
        save(8080, BASE)
        if ref_redirect != rp_redirect:
            failed = True
            print("redirect differs:", ref_redirect, "|", rp_redirect)
        if ref != rp:
            failed = True
            print("options differ")
            for a, b in zip(ref.splitlines(), rp.splitlines()):
                if a != b:
                    if a.startswith("rewrite_rules"):
                        pairs = lambda t: re.findall(r's:\d+:"(.*?)";s:\d+:"(.*?)";(?=s:\d+:"|\})', t, flags=re.S)
                        ra, rb = pairs(a), pairs(b)
                        for i, (x, y) in enumerate(zip(ra, rb)):
                            if x != y:
                                print(f"  first difference at rule {i}:")
                                print(f"   ref: {x}")
                                print(f"   rp : {y}")
                                break
                    else:
                        print("  ref:", a[:300])
                        print("  rp :", b[:300])
        else:
            print("identical", ref_redirect)
finally:
    save(8080, BASE)
sys.exit(1 if failed else 0)
