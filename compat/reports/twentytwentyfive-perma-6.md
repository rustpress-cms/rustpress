# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 04:58 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**803 URLs verglichen: 783 identisch, 19 mit bekannten Abweichungen, 1 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 329 | 328 | 0 | 1 | 738 ms | 239 ms |
| Feeds | 276 | 276 | 0 | 0 | 313 ms | 24 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 10 ms | 158 ms |
| REST-API | 160 | 159 | 1 | 0 | 16 ms | 36 ms |
| Dateien | 19 | 5 | 14 | 0 | 89 ms | 7 ms |
| Sonstiges | 7 | 3 | 4 | 0 | 307 ms | 159 ms |

## Unerwartete Abweichungen

### `/hello-world/?embed=true` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-embed-template-inline-css"], zu viel: ["wp-img-auto-sizes-contain-inline-css", "wp-block-site-title-inline-css", "wp-block-page-list-inline-css", "wp-block-navigation-css", "wp-block-group-inline-css", "wp-block-image-inline-css", "wp-block-heading-inline-css", "wp-block-paragraph-inline-css", "wp-block-search-inline-css", "wp-block-columns-inline-css", "wp-block-site-logo-inline-css", "wp-block-spacer-inline-css", "wp-block-navigation-link-inline-css", "wp-block-library-inline-css", "global-styles-inline-css", "core-block-supports-inline-css", "wp-block-template-skip-link-inline-css", "twentytwentyfive-style-inline-css", "wp-importmap", "@wordpress/interactivity-js-modulepreload"]
- **block-supports-css**

```text
1082 statt 0 Bytes, erster Unterschied bei 0:
RustPress: …⏎.wp-container-core-navigation-is-layout-6e943505{justify-content:flex-end;}.wp-container-core-group…
WordPress: ……
```

- **body**

```text
78737 statt 17722 Bytes, erster Unterschied bei 1:
RustPress: …<a class="skip-link screen-reader-text" id="wp-skip-link" href="#wp--skip-link--target">Skip to conte…
WordPress: …<!DOCTYPE html><html lang="en-US" class="no-js"><head><title>Page not found &#8211; RustPress Test</t…
```

- **head**: fehlt: <meta http-equiv="X-UA-Compatible" content="IE=edge">
- **head**: fehlt: <meta name='robots' content='noindex, follow, max-image-preview:large' />
- **head**: fehlt: <style id=wp-embed-template-inline-css>
- **head**: zu viel: <meta charset="UTF-8" />
- **head**: zu viel: <meta name="viewport" content="width=device-width, initial-scale=1" />
- **head**: zu viel: <meta name='robots' content='max-image-preview:large' />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: zu viel: <style id=wp-img-auto-sizes-contain-inline-css>
- **head**: zu viel: <style id=wp-block-site-title-inline-css>
- **head**: zu viel: <style id=wp-block-page-list-inline-css>
- **head**: zu viel: <link id=wp-block-navigation-css>
- **head**: zu viel: <style id=wp-block-group-inline-css>
- **head**: zu viel: <style id=wp-block-image-inline-css>
- **head**: zu viel: <style id=wp-block-heading-inline-css>
- **head**: zu viel: <style id=wp-block-paragraph-inline-css>
- **head**: zu viel: <style id=wp-block-search-inline-css>
- **head**: zu viel: <style id=wp-block-columns-inline-css>
- **head**: zu viel: <style id=wp-block-site-logo-inline-css>
- **head**: zu viel: <style id=wp-block-spacer-inline-css>
- **head**: zu viel: <style id=wp-block-navigation-link-inline-css>
- **head**: zu viel: <style id=wp-block-library-inline-css>
- **head**: zu viel: <style id=global-styles-inline-css>
- **head**: zu viel: <style id=core-block-supports-inline-css>
- **head**: zu viel: <style id=wp-block-template-skip-link-inline-css>
- **head**: zu viel: <style id=twentytwentyfive-style-inline-css>
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="EditURI" type="application/rsd+xml" title="RSD" href="http://localhost:8080/xmlrpc.php?rsd" />
- **head**: zu viel: <meta name="generator" content="WordPress 7.1.3" />
- **head**: zu viel: <script id=wp-importmap>
- **head**: zu viel: <link id=@wordpress/interactivity-js-modulepreload>
- **head**: zu viel: <style> <style class="wp-fonts-local"> @font-face{font-family:Manrop
- **footer**

```text
4135 statt 14 Bytes, erster Unterschied bei 1:
RustPress: …<script type="speculationrules"> {"prefetch":[{"source":"document","where":{"and":[{"href_matches":"/…
WordPress: …</body></html>…
```


## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-includes/js/jquery/jquery.min.js?ver=3.7.1` (header:cache-control), `/wp-includes/js/jquery/jquery-migrate.min.js?ver=3.4.1` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/images/smilies/mrgreen.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior-150x150.webp` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-migrate.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/wp-mediaelement.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/renderers/vimeo.min.js?ver=4.2.17` (header:cache-control) … und 2 weitere

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
