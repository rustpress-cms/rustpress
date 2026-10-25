# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 06:10 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**904 URLs verglichen: 878 identisch, 26 mit bekannten Abweichungen, 0 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 387 | 384 | 3 | 0 | 725 ms | 233 ms |
| Feeds | 293 | 293 | 0 | 0 | 309 ms | 26 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 10 ms | 57 ms |
| REST-API | 182 | 180 | 2 | 0 | 34 ms | 48 ms |
| Dateien | 21 | 5 | 16 | 0 | 120 ms | 4 ms |
| Sonstiges | 9 | 4 | 5 | 0 | 541 ms | 49 ms |

## Unerwartete Abweichungen

Keine.

## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Apache der Referenz sendet für statische Dateien keinen Cache-Control-Header; RustPress sendet bewusst public, max-age=86400.**: `/readme.html` (header:cache-control), `/license.txt` (header:cache-control)
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-content/` (header:cache-control), `/wp-includes/` (header:cache-control), `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-includes/js/jquery/jquery.min.js?ver=3.7.1` (header:cache-control), `/wp-includes/js/jquery/jquery-migrate.min.js?ver=3.4.1` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/images/smilies/mrgreen.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior-150x150.webp` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-migrate.min.js?ver=7.1.3` (header:cache-control) … und 4 weitere
- **Seiten mit gleichem menu_order haben in MariaDB keine definierte Reihenfolge.**: `/wp-json/wp/v2/pages?parent=0&orderby=menu_order` (bytes)
- **Verhalten des Apache und von PHP der Referenz (leere index.php, 403 für Verzeichnisse und Dotfiles, ausgeführte wp-config.php); RustPress liefert nur Assets aus.**: `/wp-content/` (status), `/wp-content/` (header:expires), `/wp-content/` (header:link), `/wp-includes/` (status), `/wp-includes/` (header:content-type), `/wp-includes/` (header:expires), `/wp-includes/` (header:link), `/.htaccess` (status), `/.htaccess` (header:content-type), `/.htaccess` (header:cache-control), `/.htaccess` (header:expires), `/.htaccess` (header:link) … und 4 weitere

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
