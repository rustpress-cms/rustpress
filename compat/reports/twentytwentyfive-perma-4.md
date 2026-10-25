# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 04:19 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**802 URLs verglichen: 782 identisch, 19 mit bekannten Abweichungen, 1 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 329 | 329 | 0 | 0 | 710 ms | 227 ms |
| Feeds | 275 | 275 | 0 | 0 | 298 ms | 23 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 12 ms | 18 ms |
| REST-API | 160 | 158 | 1 | 1 | 13 ms | 34 ms |
| Dateien | 19 | 5 | 14 | 0 | 63 ms | 9 ms |
| Sonstiges | 7 | 3 | 4 | 0 | 555 ms | 204 ms |

## Unerwartete Abweichungen

### `/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fhello-world%2F&format=xml` (REST-API)

- **header:content-type**: RustPress `application/json; charset=UTF-8` statt `text/xml; charset=UTF-8`
- **bytes**

```text
73 statt 132 Bytes, erster Unterschied bei 0:
RustPress: …{"code":"oembed_invalid_url","message":"Not Found","data":{"status":404}}…
WordPress: …<?xml version="1.0"?>⏎<oembed><code>oembed_invalid_url</code><message>Not Found</message><data><stat…
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
