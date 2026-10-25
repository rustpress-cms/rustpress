#!/usr/bin/env bash
# Creates a post with the classic edit form on the reference WordPress (:8080) and on RustPress (:3001) with the
# same input, saves it as a draft, publishes it, trashes and restores it, and compares the rows both leave in the
# scratch database (ID and dates neutralized). The posts it makes are removed afterwards.
set -u
export MSYS_NO_PATHCONV=1
T="${TEMP:-/tmp}"
cd "$(dirname "$0")/.."
sql() { powershell.exe -NoProfile -File dev/sql.ps1 wordpress "$1" | tr -d '\r'; }
login() { rm -f "$T/ps_$1.txt"; curl -s -c "$T/ps_$1.txt" -b "$T/ps_$1.txt" "http://localhost:$2/wp-login.php" -o /dev/null; curl -s -c "$T/ps_$1.txt" -b "$T/ps_$1.txt" -X POST "http://localhost:$2/wp-login.php" --data-urlencode log=rpadmin --data-urlencode pwd=rustpress-dev --data-urlencode testcookie=1 -o /dev/null; }
field() { grep -o "$2" | head -1 | sed -E "s/.*$3//; s/['\"].*//"; }
run() {
  name=$1; port=$2; title=$3
  login $name $port
  curl -s -b "$T/ps_$name.txt" "http://localhost:$port/wp-admin/post-new.php" -o "$T/ps_${name}_new.html"
  id=$(grep -o "name='post_ID' value='[0-9]*'" "$T/ps_${name}_new.html" | head -1 | grep -o "[0-9]*")
  nonce=$(grep -o 'id="_wpnonce" name="_wpnonce" value="[0-9a-f]*"' "$T/ps_${name}_new.html" | head -1 | sed 's/.*value="//; s/"//')
  now_y=$(date +%Y);
  common=(--data-urlencode "_wpnonce=$nonce" --data-urlencode "action=editpost" --data-urlencode "post_ID=$id" --data-urlencode "post_type=post" --data-urlencode "post_title=$title" --data-urlencode $'content=<!-- wp:paragraph -->\n<p>Hallo "Welt" & mehr</p>\n<!-- /wp:paragraph -->' --data-urlencode "post_category[]=0" --data-urlencode "post_category[]=2" --data-urlencode "tax_input[post_tag]=alpha, beta" --data-urlencode "comment_status=open" --data-urlencode "ping_status=open" --data-urlencode "visibility=public" --data-urlencode "excerpt=Ein Auszug" --data-urlencode "original_post_status=auto-draft" --data-urlencode "hidden_post_status=draft" --data-urlencode "post_author=3")
  echo "-- draft"
  curl -s -b "$T/ps_$name.txt" -D - -o /dev/null -X POST "http://localhost:$port/wp-admin/post.php" "${common[@]}" --data-urlencode "post_status=draft" --data-urlencode "save=Save Draft" | grep -i '^location' | sed -E "s/$id/ID/; s#http://localhost:[0-9]+##" | tr -d '\r'
  nonce2=$(curl -s -b "$T/ps_$name.txt" "http://localhost:$port/wp-admin/post.php?post=$id&action=edit" | grep -o 'id="_wpnonce" name="_wpnonce" value="[0-9a-f]*"' | head -1 | sed 's/.*value="//; s/"//')
  echo "-- publish"
  curl -s -b "$T/ps_$name.txt" -D - -o /dev/null -X POST "http://localhost:$port/wp-admin/post.php" "${common[@]}" --data-urlencode "_wpnonce=$nonce2" --data-urlencode "post_status=draft" --data-urlencode "publish=Publish" --data-urlencode "original_post_status=draft" | grep -i '^location' | sed -E "s/$id/ID/; s#http://localhost:[0-9]+##" | tr -d '\r'
  snapshot $id
  nonce3=$(curl -s -b "$T/ps_$name.txt" "http://localhost:$port/wp-admin/post.php?post=$id&action=edit" | grep -o 'action=trash&amp;_wpnonce=[0-9a-f]*' | head -1 | sed 's/.*_wpnonce=//')
  echo "-- trash"
  curl -s -b "$T/ps_$name.txt" -D - -o /dev/null "http://localhost:$port/wp-admin/post.php?post=$id&action=trash&_wpnonce=$nonce3" | grep -i '^location' | sed -E "s/ids=$id/ids=ID/; s#http://localhost:[0-9]+##" | tr -d '\r'
  snapshot $id
  echo "$id" > "$T/ps_${name}_id.txt"
}
snapshot() {
  sql "select post_author, post_content, post_title, post_excerpt, post_status, comment_status, ping_status, post_password, post_name, to_ping, pinged, post_parent, menu_order, post_type, post_mime_type, comment_count, (post_date_gmt = '0000-00-00 00:00:00') as floating, (post_modified >= post_date) as modok, (guid like '%?p=$1' or guid like '%/%') as guid from wp_posts where ID = $1; select concat(taxonomy, ':', t.name, ':', tt.count) from wp_term_relationships tr join wp_term_taxonomy tt on tt.term_taxonomy_id = tr.term_taxonomy_id join wp_terms t on t.term_id = tt.term_id where tr.object_id = $1 order by taxonomy, t.name; select concat(meta_key, '=', left(meta_value, 20)) from wp_postmeta where post_id = $1 and meta_key not in ('_edit_lock') order by meta_key; select count(*), group_concat(post_name) from wp_posts where post_parent = $1 and post_type = 'revision'" | sed -E 's/_wp_trash_meta_time=[0-9]+/_wp_trash_meta_time=T/; s/_edit_last=[0-9]+/_edit_last=U/'
}
cleanup() { for n in ref rp; do [ -f "$T/ps_${n}_id.txt" ] && sql "delete from wp_postmeta where post_id in (select ID from wp_posts where ID=$(cat $T/ps_${n}_id.txt) or post_parent=$(cat $T/ps_${n}_id.txt)); delete from wp_term_relationships where object_id=$(cat $T/ps_${n}_id.txt); delete from wp_posts where ID=$(cat $T/ps_${n}_id.txt) or post_parent=$(cat $T/ps_${n}_id.txt); update wp_term_taxonomy tt set count=(select count(*) from wp_term_relationships tr join wp_posts p on p.ID=tr.object_id where tr.term_taxonomy_id=tt.term_taxonomy_id and p.post_status='publish'); delete from wp_terms where name in ('alpha','beta') and term_id not in (select term_id from wp_term_taxonomy where count > 0 and taxonomy != 'post_tag')" >/dev/null; done; sql "delete from wp_term_taxonomy where taxonomy='post_tag' and term_id in (select term_id from wp_terms where name in ('alpha','beta')); delete from wp_terms where name in ('alpha','beta'); delete from wp_posts where post_status='auto-draft'" > /dev/null; }
rm -f "$T/ps_ref_id.txt" "$T/ps_rp_id.txt"
TITLE="Testbeitrag $(printf 'Ã') & Co"
run ref 8080 "$TITLE" > "$T/ps_ref_out.txt" 2>&1
cleanup
run rp 3001 "$TITLE" > "$T/ps_rp_out.txt" 2>&1
cleanup
for f in "$T/ps_ref_out.txt" "$T/ps_rp_out.txt"; do sed -E -i "s/^[Ll]ocation:/location:/; s/[0-9]+-revision-v1/N-revision-v1/" "$f"; done
if diff "$T/ps_ref_out.txt" "$T/ps_rp_out.txt" > "$T/ps_diff.txt"; then echo "identical"; cat "$T/ps_rp_out.txt"; else echo "DIFFERENT"; cat "$T/ps_diff.txt"; fi
