# Compatibility report: twentytwentyfive

As of: 2026-10-25 04:22 UTC · Reference: WordPress 7.1.3 · Theme: `twentytwentyfive`

**802 URLs compared: 776 identical, 19 with known differences, 7 with unexpected differences.**

| Kind | URLs | identical | known | unexpected | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML pages | 329 | 324 | 0 | 5 | 697 ms | 236 ms |
| Feeds | 275 | 273 | 0 | 2 | 301 ms | 23 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 9 ms | 15 ms |
| REST-API | 160 | 159 | 1 | 0 | 13 ms | 26 ms |
| Files | 19 | 5 | 14 | 0 | 59 ms | 12 ms |
| Other | 7 | 3 | 4 | 0 | 427 ms | 19 ms |

## Unexpected differences

### `/page/2/` (HTML pages)

- **body**

```text
96818 instead of 96817 bytes, first difference at 92079:
RustPress: …flex" aria-label="Pagination"><a href="http://localhost:8080/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …flex" aria-label="Pagination"><a href="http://localhost:8080" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


### `/buecher/feed/atom/` (Feeds)

- **bytes**

```text
3558 instead of 3557 bytes, first difference at 399:
RustPress: …tp://localhost:8080" />⏎	<id>http://localhost:8080/feed/atom/</id>⏎	<link rel="self" type="application/atom+xml" href="http://localhost:8080/buecher/feed/atom/"…
WordPress: …tp://localhost:8080" />⏎	<id>http://localhost:8080/feed/atom</id>⏎	<link rel="self" type="application/atom+xml" href="http://localhost:8080/buecher/feed/atom/" …
```


### `/feed/atom/` (Feeds)

- **bytes**

```text
31028 instead of 31027 bytes, first difference at 383:
RustPress: …tp://localhost:8080" />⏎	<id>http://localhost:8080/feed/atom/</id>⏎	<link rel="self" type="application/atom+xml" href="http://localhost:8080/feed/atom/" />⏎⏎	<g…
WordPress: …tp://localhost:8080" />⏎	<id>http://localhost:8080/feed/atom</id>⏎	<link rel="self" type="application/atom+xml" href="http://localhost:8080/feed/atom/" />⏎⏎	<ge…
```


### `/2026/10/24/page/2/` (HTML pages)

- **body**

```text
84466 instead of 84465 bytes, first difference at 79719:
RustPress: …label="Pagination"><a href="http://localhost:8080/2026/10/24/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …label="Pagination"><a href="http://localhost:8080/2026/10/24" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


### `/tag/alpha/page/2/` (HTML pages)

- **body**

```text
87881 instead of 87880 bytes, first difference at 83289:
RustPress: …-label="Pagination"><a href="http://localhost:8080/tag/alpha/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …-label="Pagination"><a href="http://localhost:8080/tag/alpha" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


### `/tag/beta/page/2/` (HTML pages)

- **body**

```text
87876 instead of 87875 bytes, first difference at 83287:
RustPress: …a-label="Pagination"><a href="http://localhost:8080/tag/beta/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …a-label="Pagination"><a href="http://localhost:8080/tag/beta" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


### `/author/admin/page/2/` (HTML pages)

- **body**

```text
94792 instead of 94791 bytes, first difference at 89988:
RustPress: …bel="Pagination"><a href="http://localhost:8080/author/admin/" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow …
WordPress: …bel="Pagination"><a href="http://localhost:8080/author/admin" class="wp-block-query-pagination-previous"><span class='wp-block-query-pagination-previous-arrow i…
```


## Known differences

- **The compat run uses RustPress without allow_writes, so sign-in and wp-admin stay closed; XML-RPC and wp-cron.php do not exist. Sign-in and wp-admin are compared with their own scripts in dev/ (login-compare.sh, admin-compare.sh and others).**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … and 2 more
- **The route index at /wp-json/ lists only the routes RustPress serves itself (a subset).**: `/wp-json/` (bytes)
- **RustPress deliberately sends Cache-Control: public, max-age=86400 for static files; the reference's Apache sends none.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-includes/js/jquery/jquery.min.js?ver=3.7.1` (header:cache-control), `/wp-includes/js/jquery/jquery-migrate.min.js?ver=3.4.1` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/images/smilies/mrgreen.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior-150x150.webp` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-migrate.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/wp-mediaelement.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/renderers/vimeo.min.js?ver=4.2.17` (header:cache-control) … and 2 more

Listed, but not seen in this run:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): The 404 page of a missing sitemap differs from WordPress only in whitespace in the head.
- `^/category/uncategorized/page/2/$` (body): Posts with exactly the same date (here gleich-a/gleich-b) have no defined order in MariaDB; it depends on the query plan and can differ between WordPress and RustPress.
- `^/[?]rest_route=/$` (bytes): The route index lists only the routes RustPress serves itself (a subset).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Without permalink rules WordPress does not know these files and redirects /robots.txt to /robots.txt/ (a quirk of plain permalinks); RustPress answers 404.
- `^/wp-content/uploads/` (*): The reference copy of the real site does not have its uploaded files (they stay with the site owner); without the file the two sides answer differently.
