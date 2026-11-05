#!/usr/bin/env bash
# Measures RustPress against the reference WordPress (dev/up.sh, seeded): the server with ApacheBench (one client
# and eight in parallel) and the CGI mode as a shared host runs it (one process per request, median of the
# Server-Timing values `load` and `app`).
#   dev/bench.sh [BINARY]   default: target/release/rustpress-server
# Needs `ab` (apache2-utils), the database on 127.0.0.1:3306 and the WordPress files in WP_ROOT.
set -euo pipefail
cd "$(dirname "$0")/.."
bin=$(realpath "${1:-target/release/rustpress-server}")
wp_root=${WP_ROOT:-$(podman volume inspect rustpress-wp -f '{{.Mountpoint}}' 2>/dev/null || echo /var/www/html)}
db=${RUSTPRESS_TEST_DB:-mysql://wordpress:wordpress@127.0.0.1:3306/wordpress}
paths=(/ /testbeitrag-1/ /category/news/ /feed/ /wp-json/wp/v2/posts)
host=localhost:8080

ab_line() { # base url, path
  local rps ms
  rps=$(ab -q -n 400 -c 8 -H "Host: $host" "$1$2" 2>&1 | awk '/Requests per second/ {print $4}')
  ms=$(ab -q -n 100 -c 1 -H "Host: $host" "$1$2" 2>&1 | awk '/Time per request.*\(mean\)$/ {print $4}')
  printf "  %-24s %8s req/s (8 clients)  %7s ms (1 client)\n" "$2" "$rps" "$ms"
}

echo "WordPress (reference, http://127.0.0.1:8080)"
for p in "${paths[@]}"; do ab_line http://127.0.0.1:8080 "$p"; done

echo "RustPress server ($bin)"
RUSTPRESS_DATABASE_URL=$db RUSTPRESS_WP_ROOT=$wp_root RUSTPRESS_LISTEN=127.0.0.1:3001 RUST_LOG=warn "$bin" &
server=$!
trap 'kill $server 2>/dev/null || true' EXIT
sleep 2
for p in "${paths[@]}"; do curl -s -o /dev/null -H "Host: $host" "http://127.0.0.1:3001$p"; done
for p in "${paths[@]}"; do ab_line http://127.0.0.1:3001 "$p"; done
kill $server; wait $server 2>/dev/null || true

echo "RustPress CGI (median of 25 processes; load = start-up until the router is ready, db = time in statements)"
cgi_dir=$(mktemp -d)
cgi() { # path
  (cd "$cgi_dir" && env -i PATH=/usr/bin:/bin GATEWAY_INTERFACE=CGI/1.1 REQUEST_METHOD=GET REQUEST_URI="$1" \
    SERVER_NAME=localhost SERVER_PORT=8080 HTTP_HOST=$host REMOTE_ADDR=127.0.0.1 SERVER_PROTOCOL=HTTP/1.1 \
    DOCUMENT_ROOT="$wp_root" SCRIPT_FILENAME="$wp_root/cgi-bin/rustpress.cgi" \
    RUSTPRESS_DATABASE_URL="$db?ssl-mode=disabled" "$bin")
}
cgi / >/dev/null 2>&1 # writes the file snapshot
for p in "${paths[@]}"; do
  for _ in $(seq 25); do cgi "$p" 2>/dev/null | grep -a -m1 -i '^server-timing' || true; done | python3 -c "
import re, statistics, sys
rows = [[float(v) for v in re.findall(r'dur=([\d.]+)', line)] for line in sys.stdin]
print('  %-24s load %5.1f ms  db %5.1f ms  total %6.1f ms' % ('$p', statistics.median(r[0] for r in rows), statistics.median(r[1] for r in rows), statistics.median(r[-1] for r in rows)))"
done
rm -rf "$cgi_dir"
