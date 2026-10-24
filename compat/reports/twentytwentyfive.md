# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-24 21:33 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**171 URLs verglichen: 156 identisch, 15 mit bekannten Abweichungen, 0 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 59 | 59 | 0 | 0 | 951 ms | 85 ms |
| Feeds | 31 | 30 | 1 | 0 | 33 ms | 18 ms |
| Sitemaps | 9 | 9 | 0 | 0 | 12 ms | 34 ms |
| REST-API | 57 | 54 | 3 | 0 | 109 ms | 33 ms |
| Dateien | 8 | 1 | 7 | 0 | 84 ms | 59 ms |
| Sonstiges | 7 | 3 | 4 | 0 | 270 ms | 14 ms |

## Unerwartete Abweichungen

Keine.

## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **Die Referenz auf Port 8080 verliert beim Kanonisieren den Port (WordPress-Eigenheit) und leitet ?feed=… deshalb nicht um; RustPress leitet wie WordPress auf /feed/ um.**: `/?feed=rss2` (status), `/?feed=rss2` (header:content-type), `/?feed=rss2` (header:location), `/?feed=rss2` (header:etag), `/?feed=rss2` (header:last-modified), `/?feed=rss2` (header:link)
- **Diese REST-Routen sind noch nicht umgesetzt (siehe PLAN.md).**: `/wp-json/wp/v2/types` (status), `/wp-json/wp/v2/types` (header:allow), `/wp-json/wp/v2/search?search=test` (status), `/wp-json/wp/v2/search?search=test` (header:link), `/wp-json/wp/v2/search?search=test` (header:x-wp-total), `/wp-json/wp/v2/search?search=test` (header:x-wp-totalpages), `/wp-json/wp/v2/search?search=test` (header:allow)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/blocks/navigation/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/cover/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/gallery/style.min.css?ver=7.1.3` (header:cache-control)

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
