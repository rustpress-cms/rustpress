# Compatibility report: twentytwentyfive

As of: 2026-10-24 20:33 UTC · Reference: WordPress 7.1.3 · Theme: `twentytwentyfive`

**166 URLs compared: 151 identical, 13 with known differences, 2 with unexpected differences.**

| Kind | URLs | identical | known | unexpected | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML pages | 59 | 59 | 0 | 0 | 75 ms | 92 ms |
| Feeds | 31 | 30 | 1 | 0 | 10 ms | 26 ms |
| Sitemaps | 9 | 9 | 0 | 0 | 2 ms | 25 ms |
| REST-API | 52 | 49 | 3 | 0 | 28 ms | 36 ms |
| Files | 8 | 1 | 5 | 2 | 13 ms | 3 ms |
| Other | 7 | 3 | 4 | 0 | 22 ms | 25 ms |

## Unexpected differences

### `/wp-content/uploads/2026/10/wordpress-logo.png` (Files)

- **status**: RustPress 404 instead of 200
- **header:content-type**: RustPress `text/html; charset=UTF-8` instead of `image/png`
- **header:expires**: RustPress `Wed, 11 Jan 1984 05:00:00 GMT` instead of (missing)
- **header:etag**: RustPress (missing) instead of `"9b0-65e9903c02dd9"`
- **header:last-modified**: RustPress (missing) instead of `Sat, 24 Oct 2026 16:59:06 GMT`
- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` instead of (missing)

### `/wp-content/uploads/2026/10/building-exterior.webp` (Files)

- **status**: RustPress 404 instead of 200
- **header:content-type**: RustPress `text/html; charset=UTF-8` instead of `image/webp`
- **header:expires**: RustPress `Wed, 11 Jan 1984 05:00:00 GMT` instead of (missing)
- **header:etag**: RustPress (missing) instead of `"30c2c-65e9907461931"`
- **header:last-modified**: RustPress (missing) instead of `Sat, 24 Oct 2026 17:00:05 GMT`
- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` instead of (missing)

## Known differences

- **The compat run uses RustPress without allow_writes, so sign-in and wp-admin stay closed; XML-RPC and wp-cron.php do not exist. Sign-in and wp-admin are compared with their own scripts in dev/ (login-compare.sh, admin-compare.sh and others).**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … and 2 more
- **The route index at /wp-json/ lists only the routes RustPress serves itself (a subset).**: `/wp-json/` (bytes)
- **The reference on port 8080 loses the port when canonicalizing (a WordPress quirk) and so does not redirect ?feed=…; RustPress redirects to /feed/ like WordPress does on a standard port.**: `/?feed=rss2` (status), `/?feed=rss2` (header:content-type), `/?feed=rss2` (header:location), `/?feed=rss2` (header:etag), `/?feed=rss2` (header:last-modified), `/?feed=rss2` (header:link)
- **These REST routes were not implemented yet (see PLAN.md).**: `/wp-json/wp/v2/types` (status), `/wp-json/wp/v2/types` (header:allow), `/wp-json/wp/v2/search?search=test` (status), `/wp-json/wp/v2/search?search=test` (header:link), `/wp-json/wp/v2/search?search=test` (header:x-wp-total), `/wp-json/wp/v2/search?search=test` (header:x-wp-totalpages), `/wp-json/wp/v2/search?search=test` (header:allow)
- **RustPress deliberately sends Cache-Control: public, max-age=86400 for static files; the reference's Apache sends none.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/blocks/navigation/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/cover/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/gallery/style.min.css?ver=7.1.3` (header:cache-control)

Listed, but not seen in this run:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): The 404 page of a missing sitemap differs from WordPress only in whitespace in the head.
- `^/category/uncategorized/page/2/$` (body): Posts with exactly the same date (here gleich-a/gleich-b) have no defined order in MariaDB; it depends on the query plan and can differ between WordPress and RustPress.
