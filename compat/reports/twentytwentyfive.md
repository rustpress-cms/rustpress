# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 05:17 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**229 URLs verglichen: 222 identisch, 7 mit bekannten Abweichungen, 0 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 88 | 85 | 3 | 0 | 956 ms | 78 ms |
| Feeds | 21 | 21 | 0 | 0 | 3691 ms | 108 ms |
| REST-API | 115 | 114 | 1 | 0 | 142 ms | 58 ms |
| Dateien | 2 | 0 | 2 | 0 | 1246 ms | 98 ms |
| Sonstiges | 3 | 2 | 1 | 0 | 49 ms | 16 ms |

## Unerwartete Abweichungen

Keine.

## Bekannte Abweichungen

- **Der Apache der Referenz sendet für statische Dateien keinen Cache-Control-Header; RustPress sendet bewusst public, max-age=86400.**: `/readme.html` (header:cache-control), `/license.txt` (header:cache-control)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-content/` (header:cache-control), `/wp-includes/` (header:cache-control)
- **Seiten mit gleichem menu_order haben in MariaDB keine definierte Reihenfolge.**: `/wp-json/wp/v2/pages?parent=0&orderby=menu_order` (bytes)
- **Verhalten des Apache und von PHP der Referenz (leere index.php, 403 für Verzeichnisse und Dotfiles, ausgeführte wp-config.php); RustPress liefert nur Assets aus.**: `/wp-content/` (status), `/wp-content/` (header:expires), `/wp-content/` (header:link), `/wp-includes/` (status), `/wp-includes/` (header:content-type), `/wp-includes/` (header:expires), `/wp-includes/` (header:link), `/.htaccess` (status), `/.htaccess` (header:content-type), `/.htaccess` (header:cache-control), `/.htaccess` (header:expires), `/.htaccess` (header:link) … und 4 weitere

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/(wp-login[.]php|wp-admin/|xmlrpc[.]php|wp-cron[.]php)` (*): Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.
- `^/wp-json/$` (bytes): Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
