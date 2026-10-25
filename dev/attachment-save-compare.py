#!/usr/bin/env python3
"""Saves the same Edit Media form on the reference WordPress (:8080) and on RustPress (:3001) and compares the
attachment row, its alternative text and the redirect. Both use the same scratch database; the row is put back.

    python dev/attachment-save-compare.py [attachment id]
"""
import http.cookiejar, os, re, subprocess, sys, urllib.error, urllib.parse, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ID = sys.argv[1] if len(sys.argv) > 1 else "26"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


def sql(query):
    out = subprocess.run(["powershell.exe", "-NoProfile", "-File", os.path.join(HERE, "sql.ps1"), "wordpress", query], capture_output=True, text=True, encoding="utf-8").stdout
    return out.replace("\r", "")


def snapshot():
    row = sql(f"select post_author, post_title, post_excerpt, post_content, post_name, post_status, comment_status, ping_status, post_parent, post_mime_type from wp_posts where ID={ID}")
    meta = sql(f"select meta_key, meta_value from wp_postmeta where post_id={ID} and meta_key in ('_wp_attachment_image_alt','_edit_last') order by meta_key")
    return row + meta


def save(port, form):
    jar = http.cookiejar.CookieJar()
    op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    op2 = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar), NoRedirect)
    op.open(f"http://127.0.0.1:{port}/wp-login.php").read()
    op.open(f"http://127.0.0.1:{port}/wp-login.php", urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()).read()
    page = op.open(f"http://127.0.0.1:{port}/wp-admin/post.php?post={ID}&action=edit").read().decode()
    nonce = re.search(r'name="_wpnonce" value="([0-9a-f]+)"', page).group(1)
    data = {"_wpnonce": nonce, "_wp_http_referer": f"/wp-admin/post.php?post={ID}&action=edit", "post_ID": ID, "action": "editpost", "post_type": "attachment", "advanced_view": "1", "save": "Update", **form}
    try:
        r = op2.open(f"http://127.0.0.1:{port}/wp-admin/post.php", urllib.parse.urlencode(data).encode())
        return str(r.status)
    except urllib.error.HTTPError as e:
        return f"{e.code} {(e.headers.get('Location') or '').replace('localhost:' + str(port), 'HOST')}"


original = snapshot()
FORM = {"post_title": "Neuer Titel <b>x</b> & \"q\"", "excerpt": "Bildunterschrift & <i>mehr</i>", "content": "Beschreibung\nzweite Zeile <a href=\"x\">l</a>", "_wp_attachment_image_alt": "Alt <b>fett</b> & \"x\"", "post_name": "neuer-slug", "post_author_override": "2", "comment_status": "open"}
# What the media item 26 of the scratch database looks like (put back at the end).
BACK = {"post_title": "Gebäude außen", "excerpt": "Ein Gebäude & mehr", "content": "Beschreibung des Bildes", "_wp_attachment_image_alt": "Fassade", "post_name": "gebaude-ausen", "post_author_override": "1", "comment_status": "open"}
try:
    results = {}
    for name, port in (("ref", 8080), ("rp", 3001)):
        results[name] = (save(port, FORM), snapshot())
        save(8080, BACK)
        sql(f"update wp_posts set post_author=0 where ID={ID}")
    ref, rp = results["ref"], results["rp"]
    print("redirect:", ref[0], "|", rp[0])
    if ref == rp:
        print("identical")
        print(ref[1])
    else:
        for a, b in zip(ref[1].splitlines(), rp[1].splitlines()):
            if a != b:
                print("  ref:", a)
                print("  rp :", b)
        sys.exit(1)
finally:
    print("before:", original)
    print("after: ", snapshot())
