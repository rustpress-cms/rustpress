#!/usr/bin/env python3
"""Sends the same admin-ajax.php requests to the reference WordPress (:8080) and to RustPress (:3001) and compares the
answers (after replacing the port, nonces and random passwords).

    python dev/ajax-compare.py
"""
import http.cookiejar, json, re, sys, urllib.error, urllib.parse, urllib.request


class Site:
    def __init__(self, port):
        self.port = port
        jar = http.cookiejar.CookieJar()
        self.op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
        self.op.open(f"http://127.0.0.1:{port}/wp-login.php").read()
        self.op.open(f"http://127.0.0.1:{port}/wp-login.php", urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()).read()

    def page(self, path):
        return self.op.open(f"http://127.0.0.1:{self.port}/wp-admin/{path}").read().decode("utf-8", "replace")

    def ajax(self, method, params):
        url = f"http://127.0.0.1:{self.port}/wp-admin/admin-ajax.php"
        data = urllib.parse.urlencode(params).encode()
        try:
            if method == "GET":
                r = self.op.open(url + "?" + data.decode())
            else:
                r = self.op.open(url, data)
            return r.status, r.read().decode("utf-8", "replace")
        except urllib.error.HTTPError as e:
            return e.code, e.read().decode("utf-8", "replace")


def nonce(site, path, pattern):
    return re.search(pattern, site.page(path)).group(1)


def norm(site, text):
    text = text.replace(f"localhost:{site.port}", "HOST")
    return re.sub(r'"data":"[A-Za-z0-9!@#$%^&*()]{24}"', '"data":"PASSWORD"', text)


CASES = [
    ("GET", lambda s: {"action": "ajax-tag-search", "tax": "post_tag", "q": "te"}),
    ("GET", lambda s: {"action": "ajax-tag-search", "tax": "category", "q": "a, un"}),
    ("GET", lambda s: {"action": "ajax-tag-search", "tax": "nonexistent", "q": "te"}),
    ("POST", lambda s: {"action": "get-tagcloud", "tax": "post_tag"}),
    ("POST", lambda s: {"action": "get-tagcloud", "tax": "category"}),
    ("POST", lambda s: {"action": "sample-permalink", "post_id": "1", "new_title": "Hallo Welt", "new_slug": "neuer-name", "samplepermalinknonce": nonce(s, "post.php?post=26&action=edit", r'name="samplepermalinknonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "sample-permalink", "post_id": "1", "new_title": "Hallo Welt", "new_slug": "", "samplepermalinknonce": nonce(s, "post.php?post=26&action=edit", r'name="samplepermalinknonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "find_posts", "ps": "hello", "_ajax_nonce": nonce(s, "upload.php?mode=list", r'id="_ajax_nonce" name="_ajax_nonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "find_posts", "ps": "", "_ajax_nonce": nonce(s, "upload.php?mode=list", r'id="_ajax_nonce" name="_ajax_nonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "find_posts", "ps": "zzzzzzzz", "_ajax_nonce": nonce(s, "upload.php?mode=list", r'id="_ajax_nonce" name="_ajax_nonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "wp-link-ajax", "page": "1", "_ajax_linking_nonce": nonce(s, "post.php?post=1&action=edit", r'name="_ajax_linking_nonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "wp-link-ajax", "page": "2", "_ajax_linking_nonce": nonce(s, "post.php?post=1&action=edit", r'name="_ajax_linking_nonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "wp-link-ajax", "page": "1", "search": "hello", "_ajax_linking_nonce": nonce(s, "post.php?post=1&action=edit", r'name="_ajax_linking_nonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "wp-link-ajax", "page": "999", "_ajax_linking_nonce": nonce(s, "post.php?post=1&action=edit", r'name="_ajax_linking_nonce" value="([0-9a-f]+)"')}),
    ("POST", lambda s: {"action": "generate-password"}),
    ("GET", lambda s: {"action": "autocomplete-user", "term": "rp", "autocomplete_type": "add", "autocomplete_field": "user_login"}),
    ("POST", lambda s: {"action": "dismiss-wp-pointer", "pointer": "bad pointer!"}),
]

ref, rp = Site(8080), Site(3001)
failed = False
for number, (method, make) in enumerate(CASES, 1):
    a = ref.ajax(method, make(ref))
    b = rp.ajax(method, make(rp))
    na, nb = (a[0], norm(ref, a[1])), (b[0], norm(rp, b[1]))
    if na == nb:
        print(f"{number:2} identical  {method} {make(ref)['action']} -> {na[0]} {na[1][:70]!r}")
    else:
        failed = True
        print(f"{number:2} DIFFERENT  {method} {make(ref)['action']}")
        print("   ref:", na[0], repr(na[1][:600]))
        print("   rp :", nb[0], repr(nb[1][:600]))
sys.exit(1 if failed else 0)
