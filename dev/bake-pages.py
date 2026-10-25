#!/usr/bin/env python3
"""Bakes the static parts of wp-admin screens from the reference WordPress (:8080) into
crates/rustpress-admin/src/data/pages/<screen>.json: the help tabs and the content area.

    python3 dev/bake-pages.py tools.php import.php

The address of the reference is written as {{site}}; what depends on the request (nonces, counts, ...) has to be
replaced by hand in the baked file, then the screen's Rust code fills it in again."""
import html as htmllib, http.cookiejar, json, os, re, sys, urllib.parse, urllib.request

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "crates", "rustpress-admin", "src", "data", "pages")
PORT = 8080


def fetch(page):
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    opener.open(f"http://127.0.0.1:{PORT}/wp-login.php").read()
    data = urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()
    opener.open(f"http://127.0.0.1:{PORT}/wp-login.php", data).read()
    return opener.open(f"http://127.0.0.1:{PORT}/wp-admin/{page}").read().decode("utf-8")


def help_of(page_html):
    start = page_html.find('<div id="contextual-help-wrap"')
    if start < 0:
        return {"tabs": [], "sidebar": ""}
    end = page_html.find('<div id="screen-meta-links">', start)
    if end < 0:
        end = page_html.index('<!-- wpbody-content -->') if False else len(page_html)
    block = page_html[start:end]
    tabs = []
    for m in re.finditer(r'<li id="tab-link-([^"]+)"[^>]*>\s*<a href="[^"]*" aria-controls="[^"]*">\s*(.*?)\s*</a>', block, re.S):
        tab_id, title = m.group(1), m.group(2)
        panel = re.search(r'<div id="tab-panel-' + re.escape(tab_id) + r'" class="help-tab-content[^"]*">\s*(.*?)\s*</div>\s*(?=<div id="tab-panel-|</div>\s*</div>\s*</div>)', block, re.S)
        tabs.append({"id": tab_id, "title": title, "content": panel.group(1) if panel else ""})
    sidebar = ""
    m = re.search(r'<div class="contextual-help-sidebar">\s*(.*?)\s*</div>', block, re.S)
    if m:
        sidebar = m.group(1)
    return {"tabs": tabs, "sidebar": sidebar}


def content_of(page_html):
    # After the toolbar of the screen meta (the help button), where the page itself begins.
    meta = page_html.find('<div id="screen-meta-links">')
    if meta < 0:
        # No help button: the screen meta ends with its closing tags.
        meta = page_html.index('<div id="screen-meta"')
        m = re.compile(r"</div>\s*</div>\s*</div>\s*</div>\n\t\t").search(page_html, meta)
        start = m.end()
        end = page_html.index('<div class="clear"></div></div><!-- wpbody-content -->')
        return page_html[start:end].replace(f"http://localhost:{PORT}", "{{site}}")
    m = re.compile(r"</button>\s*</div>\s*</div>\n\t\t").search(page_html, meta)
    start = m.end() if m else page_html.index('<div class="wrap')
    end = page_html.index('<div class="clear"></div></div><!-- wpbody-content -->')
    body = page_html[start:end]
    return body.replace(f"http://localhost:{PORT}", "{{site}}")


def main():
    os.makedirs(ROOT, exist_ok=True)
    for arg in sys.argv[1:]:
        # `name=page.php?query` bakes a screen under another name.
        name, _, page = arg.partition("=") if "=" in arg and not arg.startswith("?") and arg.index("=") < arg.find(".php") else (None, "", arg)
        page = page or arg
        page_html = fetch(page)
        name = name or page.split("?")[0].replace(".php", "").replace("/", "-")
        data = {"help": help_of(page_html), "content": content_of(page_html)}
        path = os.path.join(ROOT, name + ".json")
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            json.dump(data, f, ensure_ascii=False, indent=1)
        print(page, "->", os.path.normpath(path), len(data["content"]), "bytes,", len(data["help"]["tabs"]), "help tabs")


if __name__ == "__main__":
    main()
