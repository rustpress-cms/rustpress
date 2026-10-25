#!/usr/bin/env bash
# Compares wp-login.php of RustPress (port 3001, dev/restart-admin.ps1) with the reference WordPress
# (port 8080): the form, the error pages, the headers and cookies of a sign-in and a sign-out, and that
# the reference accepts the session RustPress made. Needs dev/auth_oracle.sh to have made the users.
set -u
T="${TEMP:-/tmp}"
fail=0
norm() { tr -d '\r' < "$1" | grep -iv '^date:\|^server\|^x-powered\|^content-length\|^vary\|^HTTP' | sed -E 's/expires=[^;]*; //; s/(%7C)[A-Za-z0-9]{43}(%7C)[0-9a-f]{64}/\1T\2H/; s/%7C[0-9]{10}%7C/%7CEXP%7C/' | tr 'A-Z' 'a-z' | sort; }
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
curl -s http://localhost:8080/wp-login.php | sed -E 's/"nonce":"[0-9a-f]+"/"nonce":"N"/' > "$T/c_ref.html"
curl -s http://localhost:3001/wp-login.php | sed -E 's/"nonce":"[0-9a-f]+"/"nonce":"N"/' > "$T/c_rp.html"
check "form" "$(md5sum < "$T/c_ref.html")" "$(md5sum < "$T/c_rp.html")"
post() { rm -f "$T/cj_$1.txt"; curl -s -c "$T/cj_$1.txt" -b "$T/cj_$1.txt" "http://localhost:$2/wp-login.php" -o /dev/null; curl -s -b "$T/cj_$1.txt" -c "$T/cj_$1.txt" -D "$T/ch_$1.txt" -X POST "http://localhost:$2/wp-login.php" --data-urlencode "log=$3" --data-urlencode "pwd=$4" --data-urlencode "redirect_to=http://localhost:8080/wp-admin/" --data-urlencode "testcookie=1" -o "$T/cb_$1.html"; }
for c in "rpadmin|falsch" "nobody|x" "|" "rpadmin|" "x@y.de|z" "rpadmin|rustpress-dev"; do
  u="${c%%|*}"; p="${c##*|}"; post ref 8080 "$u" "$p"; post rp 3001 "$u" "$p"
  check "post [$u|${p:0:3}] body" "$(md5sum < "$T/cb_ref.html")" "$(md5sum < "$T/cb_rp.html")"
  check "post [$u|${p:0:3}] headers" "$(norm "$T/ch_ref.txt")" "$(norm "$T/ch_rp.txt")"
done
code=$(curl -s -b "$T/cj_rp.txt" -o /dev/null -w "%{http_code}" http://localhost:8080/wp-admin/)
check "reference accepts the RustPress session" 200 "$code"
for s in ref:8080 rp:3001; do n=${s%%:*}; p=${s##*:}; curl -s -b "$T/cj_$n.txt" "http://localhost:$p/wp-login.php?action=logout" -o "$T/cl_$n.html"; done
check "logout confirmation" "$(sed -E 's/_wpnonce=[0-9a-f]+/N/' "$T/cl_ref.html" | md5sum)" "$(sed -E 's/_wpnonce=[0-9a-f]+/N/' "$T/cl_rp.html" | md5sum)"
for s in ref:8080 rp:3001; do n=${s%%:*}; p=${s##*:}; nonce=$(grep -o '_wpnonce=[0-9a-f]*' "$T/cl_$n.html" | head -1 | cut -d= -f2); curl -s -b "$T/cj_$n.txt" -D "$T/cg_$n.txt" "http://localhost:$p/wp-login.php?action=logout&_wpnonce=$nonce" -o /dev/null; done
check "logout headers" "$(norm "$T/cg_ref.txt")" "$(norm "$T/cg_rp.txt")"
exit $fail
