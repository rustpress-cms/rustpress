# Compatibility report: twentytwentyfive

As of: 2026-10-25 07:48 UTC · Reference: WordPress 7.1.3 · Theme: `twentytwentyfive`

**703 URLs compared: 684 identical, 19 with known differences, 0 with unexpected differences.**

| Kind | URLs | identical | known | unexpected | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML pages | 299 | 299 | 0 | 0 | 799 ms | 223 ms |
| Feeds | 276 | 276 | 0 | 0 | 306 ms | 49 ms |
| Sitemaps | 11 | 11 | 0 | 0 | 15 ms | 87 ms |
| REST-API | 98 | 97 | 1 | 0 | 122 ms | 42 ms |
| Files | 12 | 0 | 12 | 0 | 92 ms | 19 ms |
| Other | 7 | 1 | 6 | 0 | 564 ms | 190 ms |

## Unexpected differences

None.

## Known differences

- **The compat run uses RustPress without allow_writes, so sign-in and wp-admin stay closed; XML-RPC and wp-cron.php do not exist. Sign-in and wp-admin are compared with their own scripts in dev/ (login-compare.sh, admin-compare.sh and others).**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … and 2 more
- **The route index lists only the routes RustPress serves itself (a subset).**: `/?rest_route=/` (bytes)
- **The reference copy of the real site does not have its uploaded files (they stay with the site owner); without the file the two sides answer differently.**: `/wp-content/uploads/2026/10/datei.pdf` (status), `/wp-content/uploads/2026/10/datei.pdf` (header:location), `/wp-content/uploads/2026/10/datei.pdf` (header:expires), `/wp-content/uploads/2026/10/datei.pdf` (header:link), `/wp-content/uploads/2026/10/film.mp4` (status), `/wp-content/uploads/2026/10/film.mp4` (header:location), `/wp-content/uploads/2026/10/film.mp4` (header:expires), `/wp-content/uploads/2026/10/film.mp4` (header:link), `/wp-content/uploads/2026/10/ton.mp3` (status), `/wp-content/uploads/2026/10/ton.mp3` (header:location), `/wp-content/uploads/2026/10/ton.mp3` (header:expires), `/wp-content/uploads/2026/10/ton.mp3` (header:link)
- **Without permalink rules WordPress does not know these files and redirects /robots.txt to /robots.txt/ (a quirk of plain permalinks); RustPress answers 404.**: `/robots.txt` (status), `/robots.txt` (header:location), `/robots.txt` (header:cache-control), `/robots.txt` (header:expires), `/robots.txt` (header:link), `/favicon.ico` (status), `/favicon.ico` (header:location), `/favicon.ico` (header:cache-control), `/favicon.ico` (header:expires), `/favicon.ico` (header:link)
- **RustPress deliberately sends Cache-Control: public, max-age=86400 for static files; the reference's Apache sends none.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-content/uploads/2026/10/datei.pdf` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-content/uploads/2026/10/film.mp4` (header:cache-control), `/wp-content/uploads/2026/10/ton.mp3` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/tabs/view.min.js?ver=cc1a34b1bee3c2e17bc4` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/accordion/view.min.js?ver=2bfb90e7aee6b5300a7c` (header:cache-control), `/wp-includes/blocks/navigation/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/cover/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/image/style.min.css?ver=7.1.3` (header:cache-control)

Listed, but not seen in this run:

- `^/wp-json/$` (bytes): The route index at /wp-json/ lists only the routes RustPress serves itself (a subset).
- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): The 404 page of a missing sitemap differs from WordPress only in whitespace in the head.
- `^/category/uncategorized/page/2/$` (body): Posts with exactly the same date (here gleich-a/gleich-b) have no defined order in MariaDB; it depends on the query plan and can differ between WordPress and RustPress.
- `^/(wp-config[.]php|[.]htaccess|wp-content/|wp-includes/)$` (*): Behaviour of the reference's Apache and PHP (empty index.php, 403 for directories and dotfiles, an executed wp-config.php); RustPress only serves assets.
- `^/(license[.]txt|readme[.]html)$` (header:cache-control): The reference's Apache sends no Cache-Control header for static files; RustPress deliberately sends public, max-age=86400.
- `^/wp-json/wp/v2/pages[?]parent=0&orderby=menu_order$` (*): Pages with the same menu_order have no defined order in MariaDB.
