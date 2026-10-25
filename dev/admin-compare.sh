#!/usr/bin/env bash
# Logs in as rpadmin on the reference WordPress (:8080) and on RustPress (:3001, dev/restart-admin.ps1) and
# diffs one wp-admin screen after normalizing nonces, times and ports.
# Usage: dev/admin-compare.sh options-general.php [more screens]
set -u
export MSYS_NO_PATHCONV=1
T="${TEMP:-/tmp}"
login() { rm -f "$T/ac_$1.txt"; curl -s -c "$T/ac_$1.txt" -b "$T/ac_$1.txt" "http://localhost:$2/wp-login.php" -o /dev/null; curl -s -c "$T/ac_$1.txt" -b "$T/ac_$1.txt" -X POST "http://localhost:$2/wp-login.php" --data-urlencode log=rpadmin --data-urlencode pwd=rustpress-dev --data-urlencode testcookie=1 -o /dev/null; }
# Only the frame and the content: from the wrapper to the footer (the head carries fonts and global styles RustPress leaves out).
norm() { python3 "$(dirname "$0")/admin_norm.py"; }
login ref 8080; login rp 3001
rc=0
for screen in "$@"; do
  safe=$(printf %s "$screen" | tr '?&=/' '____')
  curl -s -b "$T/ac_ref.txt" "http://localhost:8080/wp-admin/$screen" | norm > "$T/ac_ref_$safe.html"
  curl -s -b "$T/ac_rp.txt" "http://localhost:3001/wp-admin/$screen" | norm > "$T/ac_rp_$safe.html"
  if diff -q "$T/ac_ref_$safe.html" "$T/ac_rp_$safe.html" > /dev/null; then echo "identical  $screen"; else echo "DIFFERENT  $screen ($(diff "$T/ac_ref_$safe.html" "$T/ac_rp_$safe.html" | grep -c '^[<>]') lines)"; rc=1; fi
done
exit $rc
