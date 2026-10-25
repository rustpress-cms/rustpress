#!/usr/bin/env bash
# Saves the same General Settings form on the reference WordPress (:8080) and on RustPress (:3001) and compares
# the options both leave behind (and the redirect). Both use the same scratch database; the options are put
# back afterwards. Needs dev/auth_oracle.sh's users.
set -u
T="${TEMP:-/tmp}"
wp() { powershell.exe -NoProfile -Command "wsl.exe -d Debian -- bash -lc 'cd /mnt/c/work/experimental/rustpress && bash dev/wp.sh $*'" 2>/dev/null | tr -d '\r'; }
login() { rm -f "$T/as_$1.txt"; curl -s -c "$T/as_$1.txt" -b "$T/as_$1.txt" "http://localhost:$2/wp-login.php" -o /dev/null; curl -s -c "$T/as_$1.txt" -b "$T/as_$1.txt" -X POST "http://localhost:$2/wp-login.php" --data-urlencode log=rpadmin --data-urlencode pwd=rustpress-dev --data-urlencode testcookie=1 -o /dev/null; }
OPTS="blogname blogdescription date_format time_format start_of_week timezone_string gmt_offset users_can_register default_role admin_email new_admin_email"
snapshot() { for o in $OPTS; do echo "$o=$(wp option get $o)"; done; }
save() {
  name=$1; port=$2; shift 2
  login $name $port
  nonce=$(curl -s -b "$T/as_$name.txt" "http://localhost:$port/wp-admin/options-general.php" | grep -o 'name="_wpnonce" value="[0-9a-f]*"' | head -1 | sed 's/.*value="//; s/"//')
  curl -s -b "$T/as_$name.txt" -D "$T/as_${name}_h.txt" -o /dev/null -X POST "http://localhost:$port/wp-admin/options.php" \
    --data-urlencode "option_page=general" --data-urlencode "action=update" --data-urlencode "_wpnonce=$nonce" --data-urlencode "_wp_http_referer=/wp-admin/options-general.php" "$@"
  grep -i '^location' "$T/as_${name}_h.txt" | tr -d '\r'
}
ORIG=$(snapshot)
FORM=(--data-urlencode "blogname=RustPress Test" --data-urlencode 'blogdescription=Hello & <b>x</b> "q"' --data-urlencode "date_format=\c\u\s\t\o\m" --data-urlencode "date_format_custom=d.m.Y" --data-urlencode "time_format=H:i" --data-urlencode "start_of_week=3" --data-urlencode "timezone_string=Europe/Berlin" --data-urlencode "default_role=author" --data-urlencode "new_admin_email=admin@example.com" --data-urlencode "siteurl=http://localhost:8080" --data-urlencode "home=http://localhost:8080")
echo "== reference"; save ref 8080 "${FORM[@]}"; REF=$(snapshot)
# put back
wp option update blogdescription "$(echo "$ORIG" | sed -n 's/^blogdescription=//p')" >/dev/null
echo "== rustpress"; save rp 3001 "${FORM[@]}"; RP=$(snapshot)
if [ "$REF" = "$RP" ]; then echo "options identical"; else echo "OPTIONS DIFFER"; diff <(echo "$REF") <(echo "$RP"); fi
echo "$RP"
# restore
for o in $OPTS; do v=$(echo "$ORIG" | sed -n "s/^$o=//p"); wp option update $o "$v" >/dev/null; done
