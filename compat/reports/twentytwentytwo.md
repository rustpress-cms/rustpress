# Kompatibilitätsbericht: twentytwentytwo

Stand: 2026-10-25 03:28 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentytwo`

**22 URLs verglichen: 22 identisch, 0 mit bekannten Abweichungen, 0 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| REST-API | 22 | 22 | 0 | 0 | 510 ms | 45 ms |

## Unerwartete Abweichungen

Keine.

## Bekannte Abweichungen

Keine aufgetreten.


Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/(wp-login[.]php|wp-admin/|xmlrpc[.]php|wp-cron[.]php)` (*): Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.
- `^/wp-json/$` (bytes): Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `[?&]feed=` (*): Die Referenz auf Port 8080 verliert beim Kanonisieren den Port (WordPress-Eigenheit) und leitet ?feed=… deshalb nicht um; RustPress leitet wie WordPress auf /feed/ um.
- `^/wp-(content|includes)/` (header:cache-control): RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.
- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
