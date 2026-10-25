#!/usr/bin/env python3
"""Bakes the media library's static client data from the reference WordPress (:8080), as the administrator:
the Underscore templates (`wp_print_media_templates()`), `_wpMediaViewsL10n`, `pluploadL10n` and the upload filters.
Run after `dev/up.sh`:  python3 dev/bake-media.py"""
import http.cookiejar, json, os, re, sys, urllib.parse, urllib.request

base = "http://localhost:8080"
jar = http.cookiejar.CookieJar()
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
opener.open(base + "/wp-login.php").read()
data = urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()
opener.open(base + "/wp-login.php", data).read()
page = opener.open(base + "/wp-admin/upload.php").read().decode("utf-8")

templates = re.findall(r'\t\t<script type="text/html" id="tmpl-.*?</script>\n', page, re.S)
assert len(templates) >= 25, len(templates)
out = os.path.join(os.path.dirname(__file__), "..", "crates", "rustpress-admin", "src", "data")
with open(os.path.join(out, "media_templates.html"), "w", encoding="utf-8", newline="\n") as f:
    f.write("".join(templates))

def js_var(name):
    m = re.search(r"var " + re.escape(name) + r" = (\{.*?\});\n", page, re.S)
    return json.loads(m.group(1))

views = js_var("_wpMediaViewsL10n")
settings = views.pop("settings")
plupload = js_var("pluploadL10n")
pl = js_var("_wpPluploadSettings")
bake = {
    "views": views,
    "plupload_l10n": plupload,
    "mime_extensions": pl["defaults"]["filters"]["mime_types"][0]["extensions"],
    "mime_types": settings["mimeTypes"],
    "embed_exts": settings["embedExts"],
    "embed_mimes": settings["embedMimes"],
}
with open(os.path.join(out, "media_bake.json"), "w", encoding="utf-8", newline="\n") as f:
    json.dump(bake, f, ensure_ascii=False, indent=1)
print("templates:", len(templates), "views strings:", len(views))
