#!/usr/bin/env bash
# More content for the reference WordPress: smilies, media shortcodes, a second author, hidden statuses, an
# attachment page and a threaded comment list that paginates. Idempotent through `rustpress_seeded_content2`.
set -euo pipefail
cd "$(dirname "$0")"
wp() { ./wp.sh "$@"; }

if wp option get rustpress_seeded_content2 >/dev/null 2>&1; then
  echo "content2 already seeded"; exit 0
fi

editor=$(wp user create redakteur redakteur@example.com --role=editor --display_name="Rita Redakteur" --porcelain 2>/dev/null || wp user get redakteur --field=ID)
wp user meta update "$editor" description "Redakteurin mit <em>Biografie</em>." >/dev/null

wp post create --post_title="Smilies" --post_name="smilies" --post_status=publish --post_author=1 \
  --post_content=$'Ein Lächeln :) und :-D oder 8) :P ;) :-( :arrow: :?: :!: 8-O :mrgreen: \n\nOhne Leerzeichen:)hier und http://example.com/:) im Link.' >/dev/null
printf 'Vor dem Teaser.\n<!--more-->\nNach dem Teaser mit mehr Text.' | wp post create - --post_title="Mit Teaser" --post_name="mit-teaser" --post_status=publish --post_author="$editor" >/dev/null
wp post create --post_title="Entwurf zwei" --post_name="entwurf-zwei" --post_status=draft --post_author=1 --post_content=geheim >/dev/null
wp post create --post_title="Ausstehend" --post_name="ausstehend" --post_status=pending --post_author=1 --post_content=geheim >/dev/null
wp post create --post_title="Privat" --post_name="privat" --post_status=private --post_author=1 --post_content=geheim >/dev/null
wp post create --post_title="Zukunft" --post_name="zukunft" --post_status=future --post_date="2030-01-01 10:00:00" --post_author=1 --post_content=geheim >/dev/null

cat <<MARKUP | wp post create - --post_title="Medien" --post_name="medien" --post_status=publish --post_author=1 >/dev/null
Audio:

[audio src="http://localhost:8080/wp-content/uploads/2026/10/test.mp3"]

Video:

[video src="http://localhost:8080/wp-content/uploads/2026/10/test.mp4" width="320" height="240"]

Einbettung:

[embed]https://example.invalid/video[/embed]

https://example.invalid/auf-eigener-zeile

Und [audio mp3="http://localhost:8080/wp-content/uploads/2026/10/test.mp3" loop="on"] im Text.
MARKUP

# A threaded list of comments that needs several pages.
post=$(wp post create --post_title="Diskussion" --post_name="diskussion" --post_status=publish --post_author=1 --post_content=$'Viele Kommentare.' --porcelain)
wp option update page_comments 1 >/dev/null
wp option update comments_per_page 5 >/dev/null
parent=0
for i in $(seq 1 14); do
  id=$(wp comment create --comment_post_ID="$post" --comment_content="Kommentar Nummer $i mit Link http://example.com/seite und :) Smiley." \
    --comment_author="Autor $i" --comment_author_email="autor$i@example.com" --comment_author_url="https://example.org/$i" \
    --comment_approved=1 --comment_parent=$(( i % 3 == 0 ? parent : 0 )) --porcelain)
  parent=$id
done
wp comment create --comment_post_ID="$post" --comment_content="Ein Pingback" --comment_type=pingback --comment_author="Anderes Blog" \
  --comment_author_url="https://blog.example/post" --comment_approved=1 >/dev/null
wp post update "$post" --comment_status=open >/dev/null

wp option update rustpress_seeded_content2 1 >/dev/null
echo "content2 seeded"
