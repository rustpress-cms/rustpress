# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 04:22 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**802 URLs verglichen: 776 identisch, 19 mit bekannten Abweichungen, 7 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 329 | 324 | 0 | 5 | 697 ms | 236 ms |
| Feeds | 275 | 273 | 0 | 2 | 301 ms | 23 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 9 ms | 15 ms |
| REST-API | 160 | 159 | 1 | 0 | 13 ms | 26 ms |
| Dateien | 19 | 5 | 14 | 0 | 59 ms | 12 ms |
| Sonstiges | 7 | 3 | 4 | 0 | 427 ms | 19 ms |

## Unerwartete Abweichungen

### `/page/2/` (HTML-Seiten)

- **body**

```text
96818 statt 96817 Bytes, erster Unterschied bei 92079:
RustPress: …flex" aria-label="Pagination"><a href="http://localhost:8080/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …flex" aria-label="Pagination"><a href="http://localhost:8080" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


### `/buecher/feed/atom/` (Feeds)

- **bytes**

```text
3558 statt 3557 Bytes, erster Unterschied bei 399:
RustPress: …tp://localhost:8080" />⏎	<id>http://localhost:8080/feed/atom/</id>⏎	<link rel="self" type="application/atom+xml" href="http://localhost:8080/buecher/feed/atom/"…
WordPress: …tp://localhost:8080" />⏎	<id>http://localhost:8080/feed/atom</id>⏎	<link rel="self" type="application/atom+xml" href="http://localhost:8080/buecher/feed/atom/" …
```


### `/feed/atom/` (Feeds)

- **bytes**

```text
31028 statt 31027 Bytes, erster Unterschied bei 383:
RustPress: …tp://localhost:8080" />⏎	<id>http://localhost:8080/feed/atom/</id>⏎	<link rel="self" type="application/atom+xml" href="http://localhost:8080/feed/atom/" />⏎⏎	<g…
WordPress: …tp://localhost:8080" />⏎	<id>http://localhost:8080/feed/atom</id>⏎	<link rel="self" type="application/atom+xml" href="http://localhost:8080/feed/atom/" />⏎⏎	<ge…
```


### `/2026/10/24/page/2/` (HTML-Seiten)

- **body**

```text
84466 statt 84465 Bytes, erster Unterschied bei 79719:
RustPress: …label="Pagination"><a href="http://localhost:8080/2026/10/24/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …label="Pagination"><a href="http://localhost:8080/2026/10/24" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


### `/tag/alpha/page/2/` (HTML-Seiten)

- **body**

```text
87881 statt 87880 Bytes, erster Unterschied bei 83289:
RustPress: …-label="Pagination"><a href="http://localhost:8080/tag/alpha/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …-label="Pagination"><a href="http://localhost:8080/tag/alpha" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


### `/tag/beta/page/2/` (HTML-Seiten)

- **body**

```text
87876 statt 87875 Bytes, erster Unterschied bei 83287:
RustPress: …a-label="Pagination"><a href="http://localhost:8080/tag/beta/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …a-label="Pagination"><a href="http://localhost:8080/tag/beta" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


### `/author/admin/page/2/` (HTML-Seiten)

- **body**

```text
94792 statt 94791 Bytes, erster Unterschied bei 89988:
RustPress: …bel="Pagination"><a href="http://localhost:8080/author/admin/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …bel="Pagination"><a href="http://localhost:8080/author/admin" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


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
