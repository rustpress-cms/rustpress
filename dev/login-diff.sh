#!/usr/bin/env bash
# Compares wp-login.php of RustPress (port 3001, dev/serve-admin.ps1) with the reference (port 8080).
#   dev/login-diff.sh [query]
q="${1:-}"
curl -s "http://localhost:8080/wp-login.php$q" -o /tmp/login_ref.html
curl -s "http://localhost:3001/wp-login.php$q" -o /tmp/login_rp.html
diff <(cat -A /tmp/login_ref.html) <(cat -A /tmp/login_rp.html) | cut -c1-300 | head -${LINES_SHOWN:-40} && echo "compared $(wc -c < /tmp/login_ref.html) / $(wc -c < /tmp/login_rp.html) bytes"
