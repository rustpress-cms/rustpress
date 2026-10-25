# Kompatibilitätsbericht: twentytwentytwo

Stand: 2026-10-25 07:25 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentytwo`

**913 URLs verglichen: 894 identisch, 19 mit bekannten Abweichungen, 0 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 397 | 394 | 3 | 0 | 938 ms | 68 ms |
| Feeds | 298 | 298 | 0 | 0 | 320 ms | 27 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 9 ms | 52 ms |
| REST-API | 188 | 186 | 2 | 0 | 81 ms | 40 ms |
| Dateien | 9 | 0 | 9 | 0 | 91 ms | 3 ms |
| Sonstiges | 9 | 4 | 5 | 0 | 335 ms | 42 ms |

## Unerwartete Abweichungen

Keine.

## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Apache der Referenz sendet für statische Dateien keinen Cache-Control-Header; RustPress sendet bewusst public, max-age=86400.**: `/readme.html` (header:cache-control), `/license.txt` (header:cache-control)
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-content/` (header:cache-control), `/wp-includes/` (header:cache-control), `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-content/themes/twentytwentytwo/assets/images/flight-path-on-transparent-d.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/blocks/navigation/style.min.css?ver=7.1.3` (header:cache-control), `/wp-content/themes/twentytwentytwo/style.css?ver=2.0` (header:cache-control)
- **Seiten mit gleichem menu_order haben in MariaDB keine definierte Reihenfolge.**: `/wp-json/wp/v2/pages?parent=0&orderby=menu_order` (bytes)
- **Verhalten des Apache und von PHP der Referenz (leere index.php, 403 für Verzeichnisse und Dotfiles, ausgeführte wp-config.php); RustPress liefert nur Assets aus.**: `/wp-content/` (status), `/wp-content/` (header:expires), `/wp-content/` (header:link), `/wp-includes/` (status), `/wp-includes/` (header:content-type), `/wp-includes/` (header:expires), `/wp-includes/` (header:link), `/.htaccess` (status), `/.htaccess` (header:content-type), `/.htaccess` (header:cache-control), `/.htaccess` (header:expires), `/.htaccess` (header:link) … und 4 weitere

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
