# Compatibility report: bluehost-blueprint

As of: 2026-11-09 09:10 UTC · Reference: WordPress 7.1.4 · Theme: `bluehost-blueprint`

**781 URLs compared: 758 identical, 21 with known differences, 2 with unexpected differences.**

| Kind | URLs | identical | known | unexpected | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML pages | 331 | 326 | 3 | 2 | 18 ms | 72 ms |
| Feeds | 232 | 230 | 2 | 0 | 7 ms | 22 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 1 ms | 26 ms |
| REST-API | 188 | 186 | 2 | 0 | 6 ms | 26 ms |
| Files | 9 | 0 | 9 | 0 | 0 ms | 0 ms |
| Other | 9 | 4 | 5 | 0 | 4 ms | 20 ms |

## Unexpected differences

### `/category/news/page/2/` (HTML pages)

- **head-ids**: different order
- **body**

```text
70921 instead of 70921 bytes, first difference at 56728:
RustPress: …ost-template-is-layout-flow"><li class="wp-block-post post-10 post type-post status-publish format-standard has-post-thumbnail hentry category-news category-rus…
WordPress: …ost-template-is-layout-flow"><li class="wp-block-post post-11 post type-post status-publish format-standard hentry category-news category-rust tag-alpha tag-bet…
```


### `/tag/beta/page/2/` (HTML pages)

- **head-ids**: different order
- **body**

```text
70863 instead of 70863 bytes, first difference at 56690:
RustPress: …ost-template-is-layout-flow"><li class="wp-block-post post-10 post type-post status-publish format-standard has-post-thumbnail hentry category-news category-rus…
WordPress: …ost-template-is-layout-flow"><li class="wp-block-post post-11 post type-post status-publish format-standard hentry category-news category-rust tag-alpha tag-bet…
```


## Known differences

- **Behaviour of the reference's Apache and PHP (empty index.php, 403 for directories and dotfiles, an executed wp-config.php); RustPress only serves assets.**: `/wp-content/` (status), `/wp-content/` (header:expires), `/wp-content/` (header:link), `/wp-includes/` (status), `/wp-includes/` (header:content-type), `/wp-includes/` (header:expires), `/wp-includes/` (header:link), `/.htaccess` (status), `/.htaccess` (header:content-type), `/.htaccess` (header:cache-control), `/.htaccess` (header:expires), `/.htaccess` (header:link) … and 4 more
- **Pages with the same menu_order have no defined order in MariaDB.**: `/wp-json/wp/v2/pages?parent=0&orderby=menu_order` (bytes)
- **RustPress deliberately sends Cache-Control: public, max-age=86400 for static files; the reference's Apache sends none.**: `/wp-content/` (header:cache-control), `/wp-includes/` (header:cache-control), `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=6a43cba92ed701f32cd9` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/blocks/navigation/style.min.css?ver=7.1.4` (header:cache-control), `/wp-includes/blocks/cover/style.min.css?ver=7.1.4` (header:cache-control), `/wp-content/themes/bluehost-blueprint/style.css?ver=1.0.2` (header:cache-control)
- **The compat run uses RustPress without allow_writes, so sign-in and wp-admin stay closed; XML-RPC and wp-cron.php do not exist. Sign-in and wp-admin are compared with their own scripts in dev/ (login-compare.sh, admin-compare.sh and others).**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … and 2 more
- **The reference's Apache sends no Cache-Control header for static files; RustPress deliberately sends public, max-age=86400.**: `/readme.html` (header:cache-control), `/license.txt` (header:cache-control)
- **The route index at /wp-json/ lists only the routes RustPress serves itself (a subset).**: `/wp-json/` (bytes)
- **The search results sort by title match and date only, and dev/seed_patterns.sh creates many pattern pages in the same second; their order is not defined in MariaDB and depends on the query plan.**: `/?s=test&feed=rss2` (bytes), `/search/test/feed/rss2/` (bytes)

Listed, but not seen in this run:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): The 404 page of a missing sitemap differs from WordPress only in whitespace in the head.
- `^/category/uncategorized/page/2/$` (body): Posts with exactly the same date (here gleich-a/gleich-b) have no defined order in MariaDB; it depends on the query plan and can differ between WordPress and RustPress.
- `^/[?]rest_route=/$` (bytes): The route index lists only the routes RustPress serves itself (a subset).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Without permalink rules WordPress does not know these files and redirects /robots.txt to /robots.txt/ (a quirk of plain permalinks); RustPress answers 404.
- `^/wp-content/uploads/` (*): The reference copy of the real site does not have its uploaded files (they stay with the site owner); without the file the two sides answer differently.
