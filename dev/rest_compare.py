#!/usr/bin/env python3
"""Calls an authenticated REST route on the reference WordPress (:8080) and on RustPress (:3001) and diffs the JSON.

    python3 dev/rest_compare.py GET "/wp/v2/users/me?context=edit"
    python3 dev/rest_compare.py POST "/wp/v2/posts" '{"title":"x","status":"draft"}' [--keep]
    python3 dev/rest_compare.py GET "/wp/v2/types/post?context=edit" --show      # print the RustPress answer only

The sessions are those of rpadmin (password rustpress-dev). Values that differ by nature (nonces, ports) are neutralized;
`--shape` compares only the keys and the types of the values."""
import http.cookiejar, json, re, sys, urllib.error, urllib.parse, urllib.request

PORTS = {"ref": 8080, "rp": 3001}


_sessions = {}


def session(port):
    if port not in _sessions:
        _sessions[port] = _new_session(port)
    return _sessions[port]


def _new_session(port):
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    opener.open(f"http://127.0.0.1:{port}/wp-login.php").read()
    data = urllib.parse.urlencode({"log": "rpadmin", "pwd": "rustpress-dev", "testcookie": "1"}).encode()
    opener.open(f"http://127.0.0.1:{port}/wp-login.php", data).read()
    nonce = opener.open(f"http://127.0.0.1:{port}/wp-admin/admin-ajax.php?action=rest-nonce").read().decode().strip()
    return opener, nonce


def call(port, method, path, body):
    opener, nonce = session(port)
    url = f"http://127.0.0.1:{port}/wp-json{path}"
    data = body.encode() if body else None
    request = urllib.request.Request(url, data=data, method=method, headers={"X-WP-Nonce": nonce, "Content-Type": "application/json"})
    try:
        with opener.open(request) as response:
            return response.status, {k.title(): v for k, v in response.headers.items()}, response.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        return e.code, {k.title(): v for k, v in e.headers.items()}, e.read().decode("utf-8", "replace")


def normalize(value, port):
    if isinstance(value, dict):
        return {k: normalize(v, port) for k, v in value.items()}
    if isinstance(value, list):
        return [normalize(v, port) for v in value]
    if isinstance(value, str):
        value = value.replace(f"localhost:{port}", "localhost:PORT")
        value = re.sub(r"\b[0-9a-f]{10}\b", "NONCE", value)
        return value
    return value


def shape(value):
    if isinstance(value, dict):
        return {k: shape(v) for k, v in value.items()}
    if isinstance(value, list):
        return [shape(value[0])] if value else []
    return type(value).__name__


def diff(a, b, path=""):
    out = []
    if isinstance(a, dict) and isinstance(b, dict):
        for k in a:
            if k not in b:
                out.append(f"- {path}/{k}  (only in the reference)")
            else:
                out.extend(diff(a[k], b[k], f"{path}/{k}"))
        for k in b:
            if k not in a:
                out.append(f"+ {path}/{k}  (only in RustPress)")
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            out.append(f"~ {path}: {len(a)} items in the reference, {len(b)} in RustPress")
        for i, (x, y) in enumerate(zip(a, b)):
            out.extend(diff(x, y, f"{path}[{i}]"))
    elif a != b:
        out.append(f"~ {path}: {json.dumps(a, ensure_ascii=False)[:160]}  !=  {json.dumps(b, ensure_ascii=False)[:160]}")
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    flags = [a for a in sys.argv[1:] if a.startswith("--")]
    method, path = args[0], args[1]
    body = args[2] if len(args) > 2 else None
    if "--show" in flags or "--ref" in flags:
        status, headers, text = call(PORTS["ref" if "--ref" in flags else "rp"], method, path, body)
        print(status)
        try:
            print(json.dumps(json.loads(text), indent=1, ensure_ascii=False))
        except ValueError:
            print(text[:3000])
        return
    results = {}
    for name, port in PORTS.items():
        status, headers, text = call(port, method, path, body)
        try:
            data = json.loads(text)
        except ValueError:
            data = text
        results[name] = (status, normalize(data, port), headers)
    (sa, da, ha), (sb, db, hb) = results["ref"], results["rp"]
    print(f"status: ref {sa}, rp {sb}")
    for header in ("X-Wp-Total", "X-Wp-Totalpages"):
        if ha.get(header) != hb.get(header):
            print(f"header {header}: ref {ha.get(header)}, rp {hb.get(header)}")
    if "--shape" in flags:
        da, db = shape(da), shape(db)
    lines = diff(da, db)
    print("\n".join(lines) if lines else "identical")


if __name__ == "__main__":
    main()
