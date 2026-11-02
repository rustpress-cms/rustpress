# Compatibility report: twentytwentyfive

As of: 2026-10-25 04:49 UTC · Reference: WordPress 7.1.3 · Theme: `twentytwentyfive`

**803 URLs compared: 779 identical, 19 with known differences, 5 with unexpected differences.**

| Kind | URLs | identical | known | unexpected | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML pages | 329 | 328 | 0 | 1 | 714 ms | 240 ms |
| Feeds | 276 | 272 | 0 | 4 | 89 ms | 221 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 11 ms | 35 ms |
| REST-API | 160 | 159 | 1 | 0 | 15 ms | 26 ms |
| Files | 19 | 5 | 14 | 0 | 72 ms | 5 ms |
| Other | 7 | 3 | 4 | 0 | 471 ms | 45 ms |

## Unexpected differences

### `/buecher/feed/` (Feeds)

- **bytes**

```text
712 instead of 698 bytes, first difference at 293:
RustPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments for RustPress Test	</title>⏎	<atom:link href="http://localhost:8080/buecher/feed/" rel="self" type="…
WordPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments on: 	</title>⏎	<atom:link href="http://localhost:8080/buecher/feed/" rel="self" type="application/rs…
```


### `/buecher/feed/atom/` (Feeds)

- **bytes**

```text
598 instead of 490 bytes, first difference at 186:
RustPress: …g/syndication/thread/1.0"⏎	>⏎	<title type="text">⏎	Comments for RustPress Test	</title>⏎	<subtitle type="text"></subtitle>⏎⏎	<updated>2026-10-25T03:10:51Z</upda…
WordPress: …g/syndication/thread/1.0"⏎	>⏎	<title type="text">⏎	Comments on 	</title>⏎	<subtitle type="text"></subtitle>⏎⏎	<updated>2026-10-25T03:10:51Z</updated>⏎⏎	<link re…
```


### `/kalender/feed/` (Feeds)

- **bytes**

```text
713 instead of 699 bytes, first difference at 293:
RustPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments for RustPress Test	</title>⏎	<atom:link href="http://localhost:8080/kalender/feed/" rel="self" type=…
WordPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments on: 	</title>⏎	<atom:link href="http://localhost:8080/kalender/feed/" rel="self" type="application/r…
```


### `/genre/krimi/feed/` (Feeds)

- **bytes**

```text
716 instead of 702 bytes, first difference at 293:
RustPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments for RustPress Test	</title>⏎	<atom:link href="http://localhost:8080/genre/krimi/feed/" rel="self" ty…
WordPress: …1.0/modules/syndication/"⏎	⏎	>⏎<channel>⏎	<title>⏎	Comments on: 	</title>⏎	<atom:link href="http://localhost:8080/genre/krimi/feed/" rel="self" type="applicatio…
```


### `/secret/geheimnis/` (HTML pages)

- **status**: RustPress 301 instead of 404
- **header:location**: RustPress `/?post_type=secret&p=279` instead of (missing)
- **header:link**: RustPress (missing) instead of `</wp-json/>; rel="https://api.w.org/"`

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
