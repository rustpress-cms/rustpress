#!/usr/bin/env bash
# Logs in as rpadmin on the reference WordPress (:8080) and on RustPress (:3001, dev/restart-admin.ps1) and
# diffs one wp-admin screen after normalizing nonces, times and ports.
# Usage: dev/admin-compare.sh options-general.php [more screens]
set -u
T="${TEMP:-/tmp}"
login() { rm -f "$T/ac_$1.txt"; curl -s -c "$T/ac_$1.txt" -b "$T/ac_$1.txt" "http://localhost:$2/wp-login.php" -o /dev/null; curl -s -c "$T/ac_$1.txt" -b "$T/ac_$1.txt" -X POST "http://localhost:$2/wp-login.php" --data-urlencode log=rpadmin --data-urlencode pwd=rustpress-dev --data-urlencode testcookie=1 -o /dev/null; }
# Only the frame and the content: from the wrapper to the footer (the head carries fonts and global styles RustPress leaves out).
body() { sed -n '/<div id="wpwrap">/,/<div id="wpfooter"/p'; }
norm() { body | tr -d '\r' | sed 's/></>\n</g' | sed -E 's/(nonce|_wpnonce|welcomepanelnonce)(["'\'' =:]+)[0-9a-f]{10}/\1\2N/g; s/_wpnonce=[0-9a-f]{10}/_wpnonce=N/g; s/localhost:3001/localhost:8080/g; s/"time":"[0-9]+"/"time":"T"/; s/<code>[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9:]{8}<\/code>/<code>TIME<\/code>/g'; }
login ref 8080; login rp 3001
rc=0
for screen in "$@"; do
  curl -s -b "$T/ac_ref.txt" "http://localhost:8080/wp-admin/$screen" | norm > "$T/ac_ref_$screen.html"
  curl -s -b "$T/ac_rp.txt" "http://localhost:3001/wp-admin/$screen" | norm > "$T/ac_rp_$screen.html"
  if diff -q "$T/ac_ref_$screen.html" "$T/ac_rp_$screen.html" > /dev/null; then echo "identical  $screen"; else echo "DIFFERENT  $screen ($(diff "$T/ac_ref_$screen.html" "$T/ac_rp_$screen.html" | grep -c '^[<>]') lines)"; rc=1; fi
done
exit $rc
