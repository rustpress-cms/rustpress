# Compatibility report: twentytwentyfour

As of: 2026-10-24 21:30 UTC · Reference: WordPress 7.1.3 · Theme: `twentytwentyfour`

**170 URLs compared: 155 identical, 15 with known differences, 0 with unexpected differences.**

| Kind | URLs | identical | known | unexpected | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML pages | 59 | 59 | 0 | 0 | 973 ms | 74 ms |
| Feeds | 31 | 30 | 1 | 0 | 55 ms | 35 ms |
| Sitemaps | 9 | 9 | 0 | 0 | 10 ms | 65 ms |
| REST-API | 57 | 54 | 3 | 0 | 88 ms | 31 ms |
| Files | 7 | 0 | 7 | 0 | 74 ms | 3 ms |
| Other | 7 | 3 | 4 | 0 | 398 ms | 49 ms |

## Unexpected differences

None.

## Known differences

- **The compat run uses RustPress without allow_writes, so sign-in and wp-admin stay closed; XML-RPC and wp-cron.php do not exist. Sign-in and wp-admin are compared with their own scripts in dev/ (login-compare.sh, admin-compare.sh and others).**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … and 2 more
- **The route index at /wp-json/ lists only the routes RustPress serves itself (a subset).**: `/wp-json/` (bytes)
- **The reference on port 8080 loses the port when canonicalizing (a WordPress quirk) and so does not redirect ?feed=…; RustPress redirects to /feed/ like WordPress does on a standard port.**: `/?feed=rss2` (status), `/?feed=rss2` (header:content-type), `/?feed=rss2` (header:location), `/?feed=rss2` (header:etag), `/?feed=rss2` (header:last-modified), `/?feed=rss2` (header:link)
- **These REST routes were not implemented yet (see PLAN.md).**: `/wp-json/wp/v2/types` (status), `/wp-json/wp/v2/types` (header:allow), `/wp-json/wp/v2/search?search=test` (status), `/wp-json/wp/v2/search?search=test` (header:link), `/wp-json/wp/v2/search?search=test` (header:x-wp-total), `/wp-json/wp/v2/search?search=test` (header:x-wp-totalpages), `/wp-json/wp/v2/search?search=test` (header:allow)
- **RustPress deliberately sends Cache-Control: public, max-age=86400 for static files; the reference's Apache sends none.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-content/themes/twentytwentyfour/assets/images/building-exterior.webp` (header:cache-control), `/wp-content/themes/twentytwentyfour/assets/images/tourist-and-building.webp` (header:cache-control), `/wp-content/themes/twentytwentyfour/assets/images/windows.webp` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/blocks/navigation/style-rtl.min.css?ver=7.1.3` (header:cache-control)

Listed, but not seen in this run:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): The 404 page of a missing sitemap differs from WordPress only in whitespace in the head.
- `^/category/uncategorized/page/2/$` (body): Posts with exactly the same date (here gleich-a/gleich-b) have no defined order in MariaDB; it depends on the query plan and can differ between WordPress and RustPress.
