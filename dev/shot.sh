#!/usr/bin/env bash
# Takes a screenshot of a wp-admin page of the dev server (:3001, or the reference :8080 with "ref") using headless Edge.
# Usage: dev/shot.sh <page like options-general.php> <out.png> [ref]
set -u
export MSYS_NO_PATHCONV=1
page="$1"; out="$2"; which="${3:-rp}"
port=3001; [ "$which" = ref ] && port=8080
T="${TEMP:-/tmp}"
html="$T/shot_login_$which.html"
cat > "$html" <<EOF
<!doctype html><meta charset="utf-8"><body onload="document.f.submit()">
<form name="f" method="post" action="http://localhost:$port/wp-login.php">
<input name="log" value="rpadmin"><input name="pwd" value="rustpress-dev"><input name="redirect_to" value="http://localhost:$port/wp-admin/$page"><input name="testcookie" value="1">
</form>
EOF
winhtml=$(cygpath -w "$html" 2>/dev/null || echo "$html")
winout=$(cygpath -w "$out" 2>/dev/null || echo "$out")
profile=$(cygpath -w "$T/edge_profile_$which" 2>/dev/null || echo "$T/edge_profile_$which")
"/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe" --headless=new --disable-gpu --hide-scrollbars --window-size=1400,1000 --user-data-dir="$profile" --virtual-time-budget=20000 --screenshot="$winout" "file:///$winhtml" >/dev/null 2>&1
ls -la "$out" 2>/dev/null | awk '{print $5, $9}'
