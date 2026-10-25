# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 04:49 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**803 URLs verglichen: 779 identisch, 19 mit bekannten Abweichungen, 5 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 329 | 328 | 0 | 1 | 714 ms | 240 ms |
| Feeds | 276 | 272 | 0 | 4 | 89 ms | 221 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 11 ms | 35 ms |
| REST-API | 160 | 159 | 1 | 0 | 15 ms | 26 ms |
| Dateien | 19 | 5 | 14 | 0 | 72 ms | 5 ms |
| Sonstiges | 7 | 3 | 4 | 0 | 471 ms | 45 ms |

## Unerwartete Abweichungen

### `/buecher/feed/` (Feeds)

- **bytes**

```text
712 statt 698 Bytes, erster Unterschied bei 293:
RustPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments for RustPress Test	</title>⏎	<atom:link href="http://localhost:8080/buecher/feed/" rel="self" type="…
WordPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments on: 	</title>⏎	<atom:link href="http://localhost:8080/buecher/feed/" rel="self" type="application/rs…
```


### `/buecher/feed/atom/` (Feeds)

- **bytes**

```text
598 statt 490 Bytes, erster Unterschied bei 186:
RustPress: …g/syndication/thread/1.0"⏎	>⏎	<title type="text">⏎	Comments for RustPress Test	</title>⏎	<subtitle type="text"></subtitle>⏎⏎	<updated>2026-10-25T03:10:51Z</upda…
WordPress: …g/syndication/thread/1.0"⏎	>⏎	<title type="text">⏎	Comments on 	</title>⏎	<subtitle type="text"></subtitle>⏎⏎	<updated>2026-10-25T03:10:51Z</updated>⏎⏎	<link re…
```


### `/kalender/feed/` (Feeds)

- **bytes**

```text
713 statt 699 Bytes, erster Unterschied bei 293:
RustPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments for RustPress Test	</title>⏎	<atom:link href="http://localhost:8080/kalender/feed/" rel="self" type=…
WordPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments on: 	</title>⏎	<atom:link href="http://localhost:8080/kalender/feed/" rel="self" type="application/r…
```


### `/genre/krimi/feed/` (Feeds)

- **bytes**

```text
716 statt 702 Bytes, erster Unterschied bei 293:
RustPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments for RustPress Test	</title>⏎	<atom:link href="http://localhost:8080/genre/krimi/feed/" rel="self" ty…
WordPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments on: 	</title>⏎	<atom:link href="http://localhost:8080/genre/krimi/feed/" rel="self" type="applicatio…
```


### `/secret/geheimnis/` (HTML-Seiten)

- **status**: RustPress 301 statt 404
- **header:location**: RustPress `/?post_type=secret&p=279` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-includes/js/jquery/jquery.min.js?ver=3.7.1` (header:cache-control), `/wp-includes/js/jquery/jquery-migrate.min.js?ver=3.4.1` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/images/smilies/mrgreen.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior-150x150.webp` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-migrate.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/wp-mediaelement.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/renderers/vimeo.min.js?ver=4.2.17` (header:cache-control) … und 2 weitere

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
