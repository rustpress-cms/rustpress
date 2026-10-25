#!/usr/bin/env python3
"""Bakes the static parts of the block editor settings (block categories, default editor styles, template part areas,
binding attributes) and the server-side block definitions from the reference WordPress (:8080). Run after `dev/up.sh`."""
import http.cookiejar, json, os, re, urllib.parse, urllib.request

base = "http://localhost:8080"
jar = http.cookiejar.CookieJar()
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
opener.open(base + "/wp-login.php").read()
data = urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()
opener.open(base + "/wp-login.php", data).read()
page = opener.open(base + "/wp-admin/post.php?post=1&action=edit").read().decode("utf-8")

m = re.search(r'<script id="wp-edit-post-js-after">(.*?)</script>', page, re.S)
body = m.group(1)
marker = 'initializeEditor( \'editor\', "post", 1, '
settings, _ = json.JSONDecoder().raw_decode(body[body.index(marker) + len(marker):])
keep = ["defaultEditorStyles", "blockCategories", "defaultTemplatePartAreas", "__experimentalBlockBindingsSupportedAttributes", "imageSizes", "imageDimensions"]
static = {k: settings[k] for k in keep}

m = re.search(r'<script id="wp-blocks-js-after">(.*?)</script>', page, re.S)
blocks_js = m.group(1)
i = blocks_js.index("unstable__bootstrapServerSideBlockDefinitions(") + len("unstable__bootstrapServerSideBlockDefinitions(")
definitions, _ = json.JSONDecoder().raw_decode(blocks_js[i:])
static["blockDefinitions"] = definitions
out = os.path.join(os.path.dirname(__file__), "..", "crates", "rustpress-admin", "src", "data", "editor_static.json")
with open(out, "w", encoding="utf-8", newline="\n") as f:
    json.dump(static, f, ensure_ascii=False, separators=(",", ":"))
print("bytes:", os.path.getsize(out), "blocks:", len(definitions))
