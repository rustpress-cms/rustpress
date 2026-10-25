# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 07:48 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**703 URLs verglichen: 684 identisch, 19 mit bekannten Abweichungen, 0 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 299 | 299 | 0 | 0 | 799 ms | 223 ms |
| Feeds | 276 | 276 | 0 | 0 | 306 ms | 49 ms |
| Sitemaps | 11 | 11 | 0 | 0 | 15 ms | 87 ms |
| REST-API | 98 | 97 | 1 | 0 | 122 ms | 42 ms |
| Dateien | 12 | 0 | 12 | 0 | 92 ms | 19 ms |
| Sonstiges | 7 | 1 | 6 | 0 | 564 ms | 190 ms |

## Unerwartete Abweichungen

Keine.

## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/?rest_route=/` (bytes)
- **Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.**: `/wp-content/uploads/2026/10/datei.pdf` (status), `/wp-content/uploads/2026/10/datei.pdf` (header:location), `/wp-content/uploads/2026/10/datei.pdf` (header:expires), `/wp-content/uploads/2026/10/datei.pdf` (header:link), `/wp-content/uploads/2026/10/film.mp4` (status), `/wp-content/uploads/2026/10/film.mp4` (header:location), `/wp-content/uploads/2026/10/film.mp4` (header:expires), `/wp-content/uploads/2026/10/film.mp4` (header:link), `/wp-content/uploads/2026/10/ton.mp3` (status), `/wp-content/uploads/2026/10/ton.mp3` (header:location), `/wp-content/uploads/2026/10/ton.mp3` (header:expires), `/wp-content/uploads/2026/10/ton.mp3` (header:link)
- **Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.**: `/robots.txt` (status), `/robots.txt` (header:location), `/robots.txt` (header:cache-control), `/robots.txt` (header:expires), `/robots.txt` (header:link), `/favicon.ico` (status), `/favicon.ico` (header:location), `/favicon.ico` (header:cache-control), `/favicon.ico` (header:expires), `/favicon.ico` (header:link)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-content/uploads/2026/10/datei.pdf` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-content/uploads/2026/10/film.mp4` (header:cache-control), `/wp-content/uploads/2026/10/ton.mp3` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/tabs/view.min.js?ver=cc1a34b1bee3c2e17bc4` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/accordion/view.min.js?ver=2bfb90e7aee6b5300a7c` (header:cache-control), `/wp-includes/blocks/navigation/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/cover/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/image/style.min.css?ver=7.1.3` (header:cache-control)

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-json/$` (bytes): Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/(wp-config[.]php|[.]htaccess|wp-content/|wp-includes/)$` (*): Verhalten des Apache und von PHP der Referenz (leere index.php, 403 für Verzeichnisse und Dotfiles, ausgeführte wp-config.php); RustPress liefert nur Assets aus.
- `^/(license[.]txt|readme[.]html)$` (header:cache-control): Der Apache der Referenz sendet für statische Dateien keinen Cache-Control-Header; RustPress sendet bewusst public, max-age=86400.
- `^/wp-json/wp/v2/pages[?]parent=0&orderby=menu_order$` (*): Seiten mit gleichem menu_order haben in MariaDB keine definierte Reihenfolge.
