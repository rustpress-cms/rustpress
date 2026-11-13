#!/usr/bin/env python3
"""Compares scheduled posts on the reference WordPress (:8080) and RustPress (:3001, writing allowed, see
dev/serve-admin.ps1): what both store in the option `cron` while a post is scheduled, rescheduled, made a draft,
scheduled again, trashed and deleted (REST, as rpadmin with an application password), and what publishing a due
post leaves behind: WordPress through `wp-cron.php`, RustPress through one CGI request of the given binary.

    dev/future-post-compare.py --cgi target/debug/rustpress-server --wp-root <reference files>

The user and the application password are made for the run and removed afterwards, like the posts. The sides run
one after the other, since every request to WordPress may run its cron and publish the other side's post.
"""

import argparse
import base64
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.request

HOOK = "publish_future_post"


def sql(statement):
    out = subprocess.run(["podman", "exec", "-i", "rustpress-db", "mariadb", "-uwordpress", "-pwordpress", "wordpress", "-N", "-B", "-e", statement], capture_output=True, text=True, check=True)
    return out.stdout.rstrip("\n")


def php(code, stdin=""):
    out = subprocess.run(["podman", "exec", "-i", "rustpress-wp", "php", "-r", code], input=stdin, capture_output=True, text=True, check=True)
    return out.stdout


def wp(*args):
    out = subprocess.run(["dev/wp.sh", *args], capture_output=True, text=True)
    return out.stdout.strip()


def cron_events(post_id, base=0):
    """The `publish_future_post` events of the post (timestamps relative to `base`)."""
    raw = sql("SELECT option_value FROM wp_options WHERE option_name = 'cron'")
    cron = json.loads(php("echo json_encode(unserialize(stream_get_contents(STDIN)));", raw) or "{}") or {}
    events = []
    for timestamp, hooks in cron.items():
        if timestamp == "version" or not isinstance(hooks, dict):
            continue
        for key, event in hooks.get(HOOK, {}).items():
            if event.get("args") == [post_id]:
                events.append((int(timestamp), key, event))
    version = cron.get("version")
    key = hashlib.md5(f"a:1:{{i:0;i:{post_id};}}".encode()).hexdigest()
    return {"events": [(t - base, "KEY" if k == key else k, e) for t, k, e in events], "version": version}


def request(base, auth, method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method, headers={"Authorization": auth, "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, json.loads(resp.read() or b"null")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"null")


def post_row(post_id):
    row = sql(f"SELECT post_status, post_date, post_date_gmt, post_name FROM wp_posts WHERE ID = {post_id}")
    terms = sql(f"SELECT concat(tt.taxonomy, ':', tt.term_id, ':', tt.count) FROM wp_term_relationships tr JOIN wp_term_taxonomy tt ON tt.term_taxonomy_id = tr.term_taxonomy_id WHERE tr.object_id = {post_id} ORDER BY tt.taxonomy, tt.term_id")
    return {"row": row, "terms": terms.split("\n") if terms else []}


def scheduling(base, auth):
    out = []
    status, post = request(base, auth, "POST", "/wp-json/wp/v2/posts", {"title": "Geplant", "status": "future", "date_gmt": "2031-02-03T04:05:06", "categories": [1]})
    post_id = post["id"]
    out.append(("create future", status, post["status"], cron_events(post_id)))
    status, post = request(base, auth, "POST", f"/wp-json/wp/v2/posts/{post_id}", {"date_gmt": "2031-02-03T05:05:06"})
    out.append(("reschedule", status, post["status"], cron_events(post_id)))
    status, post = request(base, auth, "POST", f"/wp-json/wp/v2/posts/{post_id}", {"status": "draft"})
    out.append(("draft", status, post["status"], cron_events(post_id)))
    status, post = request(base, auth, "POST", f"/wp-json/wp/v2/posts/{post_id}", {"status": "future", "date_gmt": "2031-02-03T06:05:06"})
    out.append(("future again", status, post["status"], cron_events(post_id)))
    status, post = request(base, auth, "DELETE", f"/wp-json/wp/v2/posts/{post_id}")
    out.append(("trash", status, post["status"], cron_events(post_id)))
    status, post = request(base, auth, "POST", f"/wp-json/wp/v2/posts/{post_id}", {"status": "future"})
    out.append(("untrash to future", status, post["status"], cron_events(post_id)))
    status, _ = request(base, auth, "DELETE", f"/wp-json/wp/v2/posts/{post_id}?force=true")
    out.append(("delete", status, None, cron_events(post_id)))
    return out, [post_id]


def publishing(base, auth, run_cron):
    # A post scheduled 70 seconds ahead, then the jobs after its time.
    at = int(time.time()) + 70
    gmt = time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(at))
    status, post = request(base, auth, "POST", "/wp-json/wp/v2/posts", {"title": "Bald", "status": "future", "date_gmt": gmt, "tags": []})
    post_id = post["id"]
    sql(f"DELETE FROM wp_term_relationships WHERE object_id = {post_id}")  # no category: wp_publish_post() adds the default one
    before = cron_events(post_id, at)
    time.sleep(75)
    run_cron()
    time.sleep(2)
    after = cron_events(post_id, at)
    row = post_row(post_id)
    row["row"] = row["row"].split("\t")[0::3]  # the status and the slug; the dates are the run's
    return [("future", status, post["status"], before), ("after cron", None, None, {"post": row, **after})], [post_id]


def neutral(results, ids):
    text = json.dumps(results, indent=1, sort_keys=True)
    for i in ids:
        text = re.sub(rf"\b{i}\b", "ID", text)
    return text


def cleanup(ids):
    for i in ids:
        sql(f"DELETE FROM wp_postmeta WHERE post_id IN (SELECT ID FROM wp_posts WHERE ID = {i} OR post_parent = {i}); DELETE FROM wp_term_relationships WHERE object_id = {i}; DELETE FROM wp_posts WHERE ID = {i} OR post_parent = {i}")
    sql("UPDATE wp_term_taxonomy tt SET count = (SELECT count(*) FROM wp_term_relationships tr JOIN wp_posts p ON p.ID = tr.object_id WHERE tr.term_taxonomy_id = tt.term_taxonomy_id AND p.post_status = 'publish')")
    cron = php(
        "$c = unserialize(stream_get_contents(STDIN)); foreach ($c as $t => $h) { if (is_array($h)) { unset($c[$t]['" + HOOK + "']); if (!$c[$t]) unset($c[$t]); } } echo serialize($c);",
        sql("SELECT option_value FROM wp_options WHERE option_name = 'cron'"),
    )
    if cron:
        escaped = cron.replace("\\", "\\\\").replace("'", "\\'")
        sql(f"UPDATE wp_options SET option_value = '{escaped}' WHERE option_name = 'cron'")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cgi", required=True, help="the RustPress binary, run once as a CGI program")
    parser.add_argument("--wp-root", required=True)
    args = parser.parse_args()
    os.chdir(os.path.join(os.path.dirname(__file__), ".."))
    if sql("SELECT count(*) FROM wp_users WHERE user_login = 'rpcron'") == "0":
        wp("user", "create", "rpcron", "rpcron@example.org", "--role=administrator", "--user_pass=rustpress-dev")
    password = wp("user", "application-password", "create", "rpcron", "future-compare", "--porcelain")
    auth = "Basic " + base64.b64encode(f"rpcron:{password}".encode()).decode()

    def wp_cron():
        urllib.request.urlopen("http://localhost:8080/wp-cron.php").read()

    def rp_cgi():
        with tempfile.TemporaryDirectory() as tmp:
            with open(os.path.join(tmp, "rustpress.toml"), "w") as f:
                f.write("allow_writes = true\n")
            env = {"PATH": os.environ["PATH"], "GATEWAY_INTERFACE": "CGI/1.1", "REQUEST_METHOD": "GET", "REQUEST_URI": "/", "SCRIPT_NAME": "/cgi-bin/rustpress.cgi", "SCRIPT_FILENAME": os.path.join(tmp, "rustpress.cgi"), "DOCUMENT_ROOT": args.wp_root, "SERVER_NAME": "localhost", "SERVER_PORT": "8080", "HTTP_HOST": "localhost:8080", "RUSTPRESS_CONFIG": os.path.join(tmp, "rustpress.toml"), "RUST_LOG": "info", "WORDPRESS_DB_HOST": "127.0.0.1", "WORDPRESS_DB_USER": "wordpress", "WORDPRESS_DB_PASSWORD": "wordpress", "WORDPRESS_DB_NAME": "wordpress"}
            out = subprocess.run([os.path.abspath(args.cgi)], cwd=tmp, env=env, capture_output=True)
            print("cgi:", out.stdout.split(b"\r\n", 1)[0].decode(), out.stderr.decode().strip()[-300:], file=sys.stderr)

    try:
        sides = {}
        for name, base, run_cron in [("ref", "http://localhost:8080", wp_cron), ("rp", "http://localhost:3001", rp_cgi)]:
            a, ids_a = scheduling(base, auth)
            cleanup(ids_a)
            b, ids_b = publishing(base, auth, run_cron)
            cleanup(ids_b)
            sides[name] = neutral(a, ids_a) + "\n" + neutral(b, ids_b)
    finally:
        wp("user", "delete", "rpcron", "--yes", "--reassign=1")
    if sides["ref"] == sides["rp"]:
        print("identical")
        print(sides["rp"])
    else:
        print("DIFFERENT")
        import difflib

        print("\n".join(difflib.unified_diff(sides["ref"].split("\n"), sides["rp"].split("\n"), "ref", "rp", lineterm="")))
        sys.exit(1)


if __name__ == "__main__":
    main()
