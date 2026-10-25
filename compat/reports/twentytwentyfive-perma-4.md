# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 03:55 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**781 URLs verglichen: 751 identisch, 20 mit bekannten Abweichungen, 10 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 318 | 309 | 0 | 9 | 719 ms | 243 ms |
| Feeds | 275 | 274 | 1 | 0 | 303 ms | 25 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 10 ms | 19 ms |
| REST-API | 150 | 148 | 1 | 1 | 78 ms | 38 ms |
| Dateien | 19 | 5 | 14 | 0 | 71 ms | 5 ms |
| Sonstiges | 7 | 3 | 4 | 0 | 351 ms | 66 ms |

## Unerwartete Abweichungen

### `/` (HTML-Seiten)

- **body**

```text
87153 statt 87172 Bytes, erster Unterschied bei 77541:
RustPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/?p=266#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser">(more&hellip;)</sp…
WordPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/uncategorized/mit-teaser/#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser"…
```


### `/secret/geheimnis/` (HTML-Seiten)

- **body**

```text
87221 statt 87240 Bytes, erster Unterschied bei 77541:
RustPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/?p=266#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser">(more&hellip;)</sp…
WordPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/uncategorized/mit-teaser/#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser"…
```


### `/?secret=geheimnis` (HTML-Seiten)

- **body**

```text
87221 statt 87240 Bytes, erster Unterschied bei 77541:
RustPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/?p=266#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser">(more&hellip;)</sp…
WordPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/uncategorized/mit-teaser/#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser"…
```


### `/?post_type=secret` (HTML-Seiten)

- **body**

```text
87221 statt 87240 Bytes, erster Unterschied bei 77541:
RustPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/?p=266#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser">(more&hellip;)</sp…
WordPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/uncategorized/mit-teaser/#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser"…
```


### `/wp-json/wp/v2/navigation` (REST-API)

- **bytes**

```text
51949 statt 51964 Bytes, erster Unterschied bei 310:
RustPress: …sh","type":"wp_navigation","link":"http:\/\/localhost:8080\/navigation\/","title":{"rendered":"Navigation"},"content":{"rendered":"<ul class=\"wp-block-page-lis…
WordPress: …sh","type":"wp_navigation","link":"http:\/\/localhost:8080\/uncategorized\/navigation\/","title":{"rendered":"Navigation"},"content":{"rendered":"<ul class=\"wp…
```


### `/2026/` (HTML-Seiten)

- **body**

```text
84343 statt 84362 Bytes, erster Unterschied bei 74773:
RustPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/?p=266#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser">(more&hellip;)</sp…
WordPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/uncategorized/mit-teaser/#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser"…
```


### `/2026/10/` (HTML-Seiten)

- **body**

```text
84364 statt 84383 Bytes, erster Unterschied bei 74782:
RustPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/?p=266#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser">(more&hellip;)</sp…
WordPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/uncategorized/mit-teaser/#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser"…
```


### `/2026/10/24/page/2/` (HTML-Seiten)

- **body**

```text
84833 statt 84853 Bytes, erster Unterschied bei 79083:
RustPress: …pe="hidden" name="redirect_to" value="http://localhost:8080/?p=19" /></p><p>This content is password-protected. To view it, please enter the password below.</p>…
WordPress: …pe="hidden" name="redirect_to" value="http://localhost:8080/uncategorized/geschuetzt/" /></p><p>This content is password-protected. To view it, please enter the…
```


### `/uncategorized/geschuetzt/` (HTML-Seiten)

- **body**

```text
82503 statt 82523 Bytes, erster Unterschied bei 71970:
RustPress: …pe="hidden" name="redirect_to" value="http://localhost:8080/?p=19" /></p><p>This content is password-protected. To view it, please enter the password below.</p>…
WordPress: …pe="hidden" name="redirect_to" value="http://localhost:8080/uncategorized/geschuetzt/" /></p><p>This content is password-protected. To view it, please enter the…
```


### `/author/redakteur/` (HTML-Seiten)

- **body**

```text
76422 statt 76441 Bytes, erster Unterschied bei 71759:
RustPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/?p=266#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser">(more&hellip;)</sp…
WordPress: …ned"><p>Vor dem Teaser.<br /><a href="http://localhost:8080/uncategorized/mit-teaser/#more-266" class="more-link"><span aria-label="Continue reading Mit Teaser"…
```


## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **Die Referenz auf Port 8080 verliert beim Kanonisieren den Port (WordPress-Eigenheit) und leitet ?feed=… deshalb nicht um; RustPress leitet wie WordPress auf /feed/ um.**: `/?feed=rss2` (status), `/?feed=rss2` (header:content-type), `/?feed=rss2` (header:location), `/?feed=rss2` (header:etag), `/?feed=rss2` (header:last-modified), `/?feed=rss2` (header:link)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-includes/js/jquery/jquery.min.js?ver=3.7.1` (header:cache-control), `/wp-includes/js/jquery/jquery-migrate.min.js?ver=3.4.1` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/images/smilies/mrgreen.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior-150x150.webp` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-migrate.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/wp-mediaelement.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/renderers/vimeo.min.js?ver=4.2.17` (header:cache-control) … und 2 weitere

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
