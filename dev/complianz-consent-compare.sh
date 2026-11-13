#!/usr/bin/env bash
# Compares the consent area of Complianz on the reference (:8080) and RustPress (:3001): pages with
# `[cmplz-consent-area]` and the block `complianz/consent-area`, without and with consent cookies, and the REST route
# `consent-area/<post>/<block>`. The pages are deleted afterwards.
#
#   dev/complianz-consent-compare.sh      DIFF=20 prints the first differences
#
# Needs the reference (dev/up.sh) with complianz-gdpr 7.5.6 active and the wizard completed, and RustPress on :3001 on
# the same database.
set -euo pipefail
cd "$(dirname "$0")/.."
ids=()
cleanup() { for id in "${ids[@]}"; do dev/wp.sh post delete "$id" --force > /dev/null 2>&1 || true; done; }
trap cleanup EXIT
# Sets `id` (a command substitution would keep the list of pages to delete in a subshell).
page() {
  id=$(dev/wp.sh post create --post_type=page --post_status="${3:-publish}" --post_title="$1" --post_name="$1" --post_content="$2" ${4:+--post_password="$4"} --porcelain)
  ids+=("$id")
}
shortcodes='[cmplz-consent-area service="youtube"]<p>YouTube [cmplz-accept-link]</p>[/cmplz-consent-area]
[cmplz-consent-area id="stats" category="statistics" cache_redirect="1" text="Accept <b>statistics</b>"]Stats area[/cmplz-consent-area]
[cmplz-consent-area id="client" clientside="1" service="vimeo"]Client side[/cmplz-consent-area]
[cmplz-consent-area id="empty"][/cmplz-consent-area]'
blocks='<!-- wp:paragraph --><p>Intro</p><!-- /wp:paragraph -->
<!-- wp:complianz/consent-area {"blockId":"area-1","postId":PID,"category":"marketing","service":"youtube","placeholderContent":"<p>Please accept YouTube</p>","consentedContent":"<p>Video [cmplz-revoke-link]</p>"} /-->
<!-- wp:group --><div class="wp-block-group"><!-- wp:complianz/consent-area {"blockId":"area-2","postId":PID,"category":"statistics","consentedContent":"<p>Nested</p>"} /--></div><!-- /wp:group -->'
page cmplz-consent-shortcodes "$shortcodes"; sc=$id
page cmplz-consent-blocks "placeholder"; bl=$id
dev/wp.sh post update "$bl" --post_content="${blocks//PID/$bl}" > /dev/null
page cmplz-consent-draft "${blocks//PID/0}" draft; draft=$id
page cmplz-consent-locked "${blocks//PID/0}" publish secret; locked=$id
python3 -c 'import time; time.sleep(2)'
paths=("/cmplz-consent-shortcodes/" "/cmplz-consent-blocks/")
rest=("/wp-json/complianz/v1/consent-area/$bl/area-1" "/wp-json/complianz/v1/consent-area/$bl/area-2" "/wp-json/complianz/v1/consent-area/$bl/AREA-1"
  "/wp-json/complianz/v1/consent-area/$bl/missing" "/wp-json/complianz/v1/consent-area/$sc/default" "/wp-json/complianz/v1/consent-area/$sc/stats"
  "/wp-json/complianz/v1/consent-area/$draft/area-1" "/wp-json/complianz/v1/consent-area/$locked/area-1" "/wp-json/complianz/v1/consent-area/999999/x"
  "/wp-json/complianz/v1/consent-area/$bl/a%3Ab" "/wp-json/complianz/v1/consent-area/$bl/area-1/" "/wp-json/complianz/v1/consent-area/abc/x")
for cookies in "" "cmplz_marketing=allow" "cmplz_statistics=allow" 'cmplz_consented_services={"youtube":1}'; do
  echo "== cookies: ${cookies:-none}"
  python3 dev/front-compare.py --anon ${cookies:+--cookie "$cookies"} ${DIFF:+--diff "$DIFF"} "${paths[@]}"
done
echo "== REST"
python3 dev/front-compare.py --anon ${DIFF:+--diff "$DIFF"} "${rest[@]}"
