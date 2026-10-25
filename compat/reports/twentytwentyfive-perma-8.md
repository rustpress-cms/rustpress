# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 04:06 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**781 URLs verglichen: 445 identisch, 20 mit bekannten Abweichungen, 316 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 318 | 276 | 0 | 42 | 947 ms | 228 ms |
| Feeds | 275 | 0 | 1 | 274 | 36 ms | 39 ms |
| Sitemaps | 12 | 12 | 0 | 0 | 13 ms | 20 ms |
| REST-API | 150 | 149 | 1 | 0 | 37 ms | 22 ms |
| Dateien | 19 | 5 | 14 | 0 | 82 ms | 11 ms |
| Sonstiges | 7 | 3 | 4 | 0 | 585 ms | 178 ms |

## Unerwartete Abweichungen

### `/page/2/` (HTML-Seiten)

- **status**: RustPress 301 statt 200
- **header:location**: RustPress `/page/2` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/buecher/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/buecher/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/buecher/feed/atom/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/atom+xml; charset=UTF-8`
- **header:location**: RustPress `/buecher/feed/atom` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/kalender/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/kalender/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/genre/krimi/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/genre/krimi/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/feed/atom/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/atom+xml; charset=UTF-8`
- **header:location**: RustPress `/feed/atom` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/comments/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/comments/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/2026/10/24/page/2/` (HTML-Seiten)

- **status**: RustPress 301 statt 200
- **header:location**: RustPress `/2026/10/24/page/2` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/hello-world.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Hello world! Comments Feed" href="http://localhost:8080/admin/hello-world.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Hello world! Comments Feed" href="http://localhost:8080/admin/hello-world.html/feed/" />

### `/admin/hello-world.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/hello-world.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-1.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 1 Comments Feed" href="http://localhost:8080/admin/testbeitrag-1.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 1 Comments Feed" href="http://localhost:8080/admin/testbeitrag-1.html/feed/" />

### `/admin/testbeitrag-1.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-1.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-2.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 2 Comments Feed" href="http://localhost:8080/admin/testbeitrag-2.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 2 Comments Feed" href="http://localhost:8080/admin/testbeitrag-2.html/feed/" />

### `/admin/testbeitrag-2.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-2.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-3.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 3 Comments Feed" href="http://localhost:8080/admin/testbeitrag-3.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 3 Comments Feed" href="http://localhost:8080/admin/testbeitrag-3.html/feed/" />

### `/admin/testbeitrag-3.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-3.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-4.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 4 Comments Feed" href="http://localhost:8080/admin/testbeitrag-4.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 4 Comments Feed" href="http://localhost:8080/admin/testbeitrag-4.html/feed/" />

### `/admin/testbeitrag-4.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-4.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-5.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 5 Comments Feed" href="http://localhost:8080/admin/testbeitrag-5.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 5 Comments Feed" href="http://localhost:8080/admin/testbeitrag-5.html/feed/" />

### `/admin/testbeitrag-5.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-5.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-6.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 6 Comments Feed" href="http://localhost:8080/admin/testbeitrag-6.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 6 Comments Feed" href="http://localhost:8080/admin/testbeitrag-6.html/feed/" />

### `/admin/testbeitrag-6.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-6.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-7.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 7 Comments Feed" href="http://localhost:8080/admin/testbeitrag-7.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 7 Comments Feed" href="http://localhost:8080/admin/testbeitrag-7.html/feed/" />

### `/admin/testbeitrag-7.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-7.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-8.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 8 Comments Feed" href="http://localhost:8080/admin/testbeitrag-8.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 8 Comments Feed" href="http://localhost:8080/admin/testbeitrag-8.html/feed/" />

### `/admin/testbeitrag-8.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-8.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-9.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 9 Comments Feed" href="http://localhost:8080/admin/testbeitrag-9.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 9 Comments Feed" href="http://localhost:8080/admin/testbeitrag-9.html/feed/" />

### `/admin/testbeitrag-9.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-9.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-10.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 10 Comments Feed" href="http://localhost:8080/admin/testbeitrag-10.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 10 Comments Feed" href="http://localhost:8080/admin/testbeitrag-10.html/feed/" />

### `/admin/testbeitrag-10.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-10.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-11.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 11 Comments Feed" href="http://localhost:8080/admin/testbeitrag-11.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 11 Comments Feed" href="http://localhost:8080/admin/testbeitrag-11.html/feed/" />

### `/admin/testbeitrag-11.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-11.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/testbeitrag-12.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 12 Comments Feed" href="http://localhost:8080/admin/testbeitrag-12.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Testbeitrag 12 Comments Feed" href="http://localhost:8080/admin/testbeitrag-12.html/feed/" />

### `/admin/testbeitrag-12.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/testbeitrag-12.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/klassisch.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Klassischer Beitrag Comments Feed" href="http://localhost:8080/admin/klassisch.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Klassischer Beitrag Comments Feed" href="http://localhost:8080/admin/klassisch.html/feed/" />

### `/admin/klassisch.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/klassisch.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/geschuetzt.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Protected: Passwortgeschützt Comments Feed" href="http://localhost:8080/admin/geschuetzt.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Protected: Passwortgeschützt Comments Feed" href="http://localhost:8080/admin/geschuetzt.html/feed/" />

### `/admin/geschuetzt.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/geschuetzt.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/mehrseitig.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Mehrseitig Comments Feed" href="http://localhost:8080/admin/mehrseitig.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Mehrseitig Comments Feed" href="http://localhost:8080/admin/mehrseitig.html/feed/" />

### `/admin/mehrseitig.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/mehrseitig.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/gleich-a.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Gleich A Comments Feed" href="http://localhost:8080/admin/gleich-a.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Gleich A Comments Feed" href="http://localhost:8080/admin/gleich-a.html/feed/" />

### `/admin/gleich-a.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/gleich-a.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/gleich-b.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Gleich B Comments Feed" href="http://localhost:8080/admin/gleich-b.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Gleich B Comments Feed" href="http://localhost:8080/admin/gleich-b.html/feed/" />

### `/admin/gleich-b.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/gleich-b.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/bilder.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Bilder Comments Feed" href="http://localhost:8080/admin/bilder.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Bilder Comments Feed" href="http://localhost:8080/admin/bilder.html/feed/" />

### `/admin/bilder.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/bilder.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/klassische-bilder.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Klassische Bilder Comments Feed" href="http://localhost:8080/admin/klassische-bilder.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Klassische Bilder Comments Feed" href="http://localhost:8080/admin/klassische-bilder.html/feed/" />

### `/admin/klassische-bilder.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/klassische-bilder.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/logo.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Logo Comments Feed" href="http://localhost:8080/admin/logo.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Logo Comments Feed" href="http://localhost:8080/admin/logo.html/feed/" />

### `/admin/logo.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/logo.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/viele-bilder.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Viele Bilder Comments Feed" href="http://localhost:8080/admin/viele-bilder.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Viele Bilder Comments Feed" href="http://localhost:8080/admin/viele-bilder.html/feed/" />

### `/admin/viele-bilder.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/viele-bilder.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/cover-bilder.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Cover Bilder Comments Feed" href="http://localhost:8080/admin/cover-bilder.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Cover Bilder Comments Feed" href="http://localhost:8080/admin/cover-bilder.html/feed/" />

### `/admin/cover-bilder.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/cover-bilder.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/dynamische-bloecke.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Dynamische Bloecke Comments Feed" href="http://localhost:8080/admin/dynamische-bloecke.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Dynamische Bloecke Comments Feed" href="http://localhost:8080/admin/dynamische-bloecke.html/feed/" />

### `/admin/dynamische-bloecke.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/dynamische-bloecke.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/abfrage-bloecke.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Abfrage-Bloecke Comments Feed" href="http://localhost:8080/admin/abfrage-bloecke.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Abfrage-Bloecke Comments Feed" href="http://localhost:8080/admin/abfrage-bloecke.html/feed/" />

### `/admin/abfrage-bloecke.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/abfrage-bloecke.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/klassischer-inhalt.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Klassischer Inhalt Comments Feed" href="http://localhost:8080/admin/klassischer-inhalt.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Klassischer Inhalt Comments Feed" href="http://localhost:8080/admin/klassischer-inhalt.html/feed/" />

### `/admin/klassischer-inhalt.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/klassischer-inhalt.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/smilies.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Smilies Comments Feed" href="http://localhost:8080/admin/smilies.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Smilies Comments Feed" href="http://localhost:8080/admin/smilies.html/feed/" />

### `/admin/smilies.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/smilies.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/redakteur/mit-teaser.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Mit Teaser Comments Feed" href="http://localhost:8080/redakteur/mit-teaser.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Mit Teaser Comments Feed" href="http://localhost:8080/redakteur/mit-teaser.html/feed/" />

### `/redakteur/mit-teaser.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/redakteur/mit-teaser.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/medien.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Medien Comments Feed" href="http://localhost:8080/admin/medien.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Medien Comments Feed" href="http://localhost:8080/admin/medien.html/feed/" />

### `/admin/medien.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/medien.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/admin/diskussion.html` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Diskussion Comments Feed" href="http://localhost:8080/admin/diskussion.html/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Diskussion Comments Feed" href="http://localhost:8080/admin/diskussion.html/feed/" />

### `/admin/diskussion.html/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/admin/diskussion.html/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/sample-page` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Sample Page Comments Feed" href="http://localhost:8080/sample-page/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Sample Page Comments Feed" href="http://localhost:8080/sample-page/feed/" />

### `/sample-page/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/sample-page/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/ueber-uns/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/ueber-uns/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/ueber-uns/team/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/ueber-uns/team/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/bildliste/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/bildliste/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-banner-about-book/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-banner-about-book/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-banner-cover-big-heading/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-banner-cover-big-heading/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-banner-intro-image/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-banner-intro-image/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-banner-intro/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-banner-intro/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-banner-poster/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-banner-poster/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-banner-description-images-grid/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-banner-description-images-grid/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-binding-format/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-binding-format/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-comments/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-comments/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-contact-centered-social-link/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-contact-centered-social-link/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-contact-info-locations/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-contact-info-locations/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-contact-location-and-link/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-contact-location-and-link/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-cta-book-links/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-cta-book-links/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-cta-book-locations/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-cta-book-locations/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-cta-centered-heading/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-cta-centered-heading/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-cta-events-list/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-cta-events-list/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-cta-grid-products-link/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-cta-grid-products-link/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-cta-heading-search/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-cta-heading-search/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-cta-newsletter/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-cta-newsletter/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-event-3-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-event-3-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-event-rsvp/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-event-rsvp/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-event-schedule/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-event-schedule/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-footer-centered/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-footer-centered/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-footer-columns/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-footer-columns/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-footer-newsletter/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-footer-newsletter/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-footer/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-footer/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-footer-social/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-footer-social/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-format-audio/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-format-audio/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-format-link/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-format-link/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-grid-videos/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-grid-videos/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-grid-with-categories/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-grid-with-categories/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-header-centered/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-header-centered/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-header-columns/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-header-columns/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-header-large-title/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-header-large-title/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-header/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-header/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-heading-and-paragraph-with-image/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-heading-and-paragraph-with-image/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hero-book/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hero-book/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hero-full-width-image/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hero-full-width-image/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hero-overlapped-book-cover-with-links/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hero-overlapped-book-cover-with-links/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hero-podcast/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hero-podcast/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hidden-404/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hidden-404/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hidden-blog-heading/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hidden-blog-heading/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hidden-search/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hidden-search/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hidden-sidebar/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hidden-sidebar/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-hidden-written-by/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-hidden-written-by/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-logos/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-logos/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-media-instagram-grid/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-media-instagram-grid/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-more-posts/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-more-posts/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-overlapped-images/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-overlapped-images/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-business-home/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-business-home/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-coming-soon/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-coming-soon/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-cv-bio/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-cv-bio/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-landing-book/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-landing-book/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-landing-event/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-landing-event/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-landing-podcast/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-landing-podcast/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-link-in-bio-wide-margins/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-link-in-bio-wide-margins/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-link-in-bio-with-tight-margins/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-link-in-bio-with-tight-margins/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-portfolio-home/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-portfolio-home/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-page-shop-home/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-page-shop-home/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-post-navigation/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-post-navigation/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-pricing-2-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-pricing-2-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-pricing-3-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-pricing-3-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-services-3-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-services-3-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-services-subscriber-only-section/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-services-subscriber-only-section/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-services-team-photos/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-services-team-photos/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-404-vertical-header-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-404-vertical-header-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-archive-news-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-archive-news-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-archive-photo-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-archive-photo-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-archive-text-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-archive-text-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-archive-vertical-header-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-archive-vertical-header-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-home-news-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-home-news-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-home-photo-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-home-photo-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-home-posts-grid-news-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-home-posts-grid-news-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-home-text-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-home-text-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-home-vertical-header-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-home-vertical-header-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-home-with-sidebar-news-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-home-with-sidebar-news-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-page-photo-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-page-photo-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-page-vertical-header-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-page-vertical-header-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-query-loop-news-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-query-loop-news-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-query-loop-photo-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-query-loop-photo-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-query-loop/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-query-loop/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-query-loop-text-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-query-loop-text-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-query-loop-vertical-header-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-query-loop-vertical-header-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-search-news-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-search-news-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-search-photo-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-search-photo-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-search-text-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-search-text-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-search-vertical-header-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-search-vertical-header-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-post-with-left-aligned-content/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-post-with-left-aligned-content/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-single-news-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-single-news-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-single-offset/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-single-offset/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-single-photo-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-single-photo-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-single-text-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-single-text-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-template-single-vertical-header-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-template-single-vertical-header-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-testimonials-2-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-testimonials-2-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-testimonials-6-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-testimonials-6-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-testimonials-large/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-testimonials-large/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-text-faqs/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-text-faqs/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfive-vertical-header/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfive-vertical-header/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-banner-hero/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-banner-hero/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-banner-project-description/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-banner-project-description/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-cta-content-image-on-right/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-cta-content-image-on-right/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-cta-pricing/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-cta-pricing/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-cta-rsvp/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-cta-rsvp/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-cta-services-image-left/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-cta-services-image-left/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-cta-subscribe-centered/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-cta-subscribe-centered/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-footer-centered-logo-nav/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-footer-centered-logo-nav/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-footer-colophon-3-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-footer-colophon-3-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-footer/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-footer/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-gallery-full-screen-image/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-gallery-full-screen-image/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-gallery-offset-images-grid-2-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-gallery-offset-images-grid-2-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-gallery-offset-images-grid-3-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-gallery-offset-images-grid-3-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-gallery-offset-images-grid-4-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-gallery-offset-images-grid-4-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-gallery-project-layout/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-gallery-project-layout/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-404/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-404/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-comments/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-comments/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-no-results/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-no-results/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-portfolio-hero/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-portfolio-hero/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-post-meta/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-post-meta/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-post-navigation/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-post-navigation/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-posts-heading/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-posts-heading/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-search/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-search/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-hidden-sidebar/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-hidden-sidebar/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-page-about-business/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-page-about-business/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-page-home-blogging/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-page-home-blogging/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-page-home-business/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-page-home-business/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-page-home-gallery/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-page-home-gallery/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-page-home-portfolio/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-page-home-portfolio/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-page-newsletter-landing/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-page-newsletter-landing/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-page-portfolio-overview/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-page-portfolio-overview/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-page-rsvp-landing/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-page-rsvp-landing/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-posts-1-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-posts-1-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-posts-3-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-posts-3-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-posts-grid-2-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-posts-grid-2-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-posts-images-only-3-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-posts-images-only-3-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-posts-images-only-offset-4-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-posts-images-only-offset-4-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-posts-list/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-posts-list/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-team-4-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-team-4-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-archive-blogging/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-archive-blogging/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-archive-portfolio/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-archive-portfolio/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-home-blogging/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-home-blogging/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-home-business/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-home-business/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-home-portfolio/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-home-portfolio/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-index-blogging/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-index-blogging/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-index-portfolio/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-index-portfolio/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-search-blogging/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-search-blogging/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-search-portfolio/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-search-portfolio/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-template-single-portfolio/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-template-single-portfolio/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-testimonial-centered/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-testimonial-centered/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-text-alternating-images/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-text-alternating-images/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-text-centered-statement/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-text-centered-statement/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-text-centered-statement-small/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-text-centered-statement-small/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-text-faq/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-text-faq/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-text-feature-grid-3-col/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-text-feature-grid-3-col/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-text-project-details/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-text-project-details/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentyfour-text-title-left-image-right/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentyfour-text-title-left-image-right/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentythree-cta/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentythree-cta/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentythree-footer-default/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentythree-footer-default/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentythree-hidden-404/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentythree-hidden-404/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentythree-hidden-comments/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentythree-hidden-comments/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentythree-hidden-heading/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentythree-hidden-heading/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentythree-hidden-no-results-content/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentythree-hidden-no-results-content/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentythree-post-meta/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentythree-post-meta/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-default/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-default/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-dark/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-dark/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-logo/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-logo/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-navigation/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-navigation/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-title-tagline-social/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-title-tagline-social/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-social-copyright/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-social-copyright/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-navigation-copyright/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-navigation-copyright/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-about-title-logo/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-about-title-logo/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-query-title-citation/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-query-title-citation/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-query-images-title-citation/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-query-images-title-citation/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-footer-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-footer-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-subscribe/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-subscribe/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-featured-posts/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-featured-posts/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-layered-images-with-duotone/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-layered-images-with-duotone/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-wide-image-intro-buttons/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-wide-image-intro-buttons/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-large-list-names/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-large-list-names/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-video-header-details/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-video-header-details/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-list-events/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-list-events/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-two-images-text/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-two-images-text/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-image-with-caption/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-image-with-caption/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-video-trailer/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-video-trailer/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-pricing-table/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-pricing-table/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-divider-light/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-divider-light/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-general-divider-dark/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-general-divider-dark/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-default/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-default/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-large-dark/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-large-dark/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-small-dark/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-small-dark/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-image-background/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-image-background/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-image-background-overlay/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-image-background-overlay/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-with-tagline/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-with-tagline/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-text-only-green-background/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-text-only-green-background/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-text-only-salmon-background/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-text-only-salmon-background/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-title-and-button/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-title-and-button/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-text-only-with-tagline-black-background/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-text-only-with-tagline-black-background/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-logo-navigation-gray-background/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-logo-navigation-gray-background/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-logo-navigation-social-black-background/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-logo-navigation-social-black-background/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-title-navigation-social/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-title-navigation-social/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-logo-navigation-offset-tagline/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-logo-navigation-offset-tagline/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-stacked/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-stacked/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-centered-logo/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-centered-logo/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-centered-logo-black-background/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-centered-logo-black-background/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-header-centered-title-navigation-social/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-header-centered-title-navigation-social/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-hidden-404/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-hidden-404/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-hidden-bird/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-hidden-bird/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-hidden-heading-and-bird/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-hidden-heading-and-bird/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-about-media-left/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-about-media-left/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-about-simple-dark/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-about-simple-dark/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-about-media-right/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-about-media-right/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-about-solid-color/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-about-solid-color/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-about-links/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-about-links/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-about-links-dark/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-about-links-dark/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-about-large-image-and-buttons/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-about-large-image-and-buttons/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-layout-image-and-text/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-layout-image-and-text/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-layout-image-text-and-video/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-layout-image-text-and-video/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-layout-two-columns/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-layout-two-columns/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-sidebar-poster/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-sidebar-poster/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-sidebar-grid-posts/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-sidebar-grid-posts/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-sidebar-blog-posts/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-sidebar-blog-posts/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-page-sidebar-blog-posts-right/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-page-sidebar-blog-posts-right/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-query-default/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-query-default/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-query-simple-blog/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-query-simple-blog/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-query-grid/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-query-grid/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-query-text-grid/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-query-text-grid/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-query-image-grid/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-query-image-grid/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-query-large-titles/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-query-large-titles/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/pat-twentytwentytwo-query-irregular-grid/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/pat-twentytwentytwo-query-irregular-grid/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/buecher/buch-1` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Buch 1 Comments Feed" href="http://localhost:8080/buecher/buch-1/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Buch 1 Comments Feed" href="http://localhost:8080/buecher/buch-1/feed/" />

### `/buecher/buch-1/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/buecher/buch-1/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/buecher/buch-2` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Buch 2 Comments Feed" href="http://localhost:8080/buecher/buch-2/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Buch 2 Comments Feed" href="http://localhost:8080/buecher/buch-2/feed/" />

### `/buecher/buch-2/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/buecher/buch-2/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/buecher/buch-3` (HTML-Seiten)

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Buch 3 Comments Feed" href="http://localhost:8080/buecher/buch-3/feed" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Buch 3 Comments Feed" href="http://localhost:8080/buecher/buch-3/feed/" />

### `/buecher/buch-3/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/buecher/buch-3/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/termine/sommerfest/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/termine/sommerfest/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/termine/sommerfest/aufbau/feed/` (Feeds)

- **status**: RustPress 301 statt 200
- **header:content-type**: RustPress (fehlt) statt `application/rss+xml; charset=UTF-8`
- **header:location**: RustPress `/termine/sommerfest/aufbau/feed` statt (fehlt)
- **header:etag**: RustPress (fehlt) statt `"e25dbc0b70e8141ef347bf6998f3a25b"`
- **header:last-modified**: RustPress (fehlt) statt `Sun, 25 Oct 2026 03:10:51 GMT`
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/category/uncategorized/page/2/` (HTML-Seiten)

- **status**: RustPress 301 statt 200
- **header:location**: RustPress `/category/uncategorized/page/2` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/category/news/page/2/` (HTML-Seiten)

- **status**: RustPress 301 statt 200
- **header:location**: RustPress `/category/news/page/2` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/category/tech/rust/page/2/` (HTML-Seiten)

- **status**: RustPress 301 statt 200
- **header:location**: RustPress `/category/tech/rust/page/2` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/tag/alpha/page/2/` (HTML-Seiten)

- **status**: RustPress 301 statt 200
- **header:location**: RustPress `/tag/alpha/page/2` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/tag/beta/page/2/` (HTML-Seiten)

- **status**: RustPress 301 statt 200
- **header:location**: RustPress `/tag/beta/page/2` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

### `/author/admin/page/2/` (HTML-Seiten)

- **status**: RustPress 301 statt 200
- **header:location**: RustPress `/author/admin/page/2` statt (fehlt)
- **header:link**: RustPress (fehlt) statt `</wp-json/>; rel="https://api.w.org/"`

## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **Die Referenz auf Port 8080 verliert beim Kanonisieren den Port (WordPress-Eigenheit) und leitet ?feed=… deshalb nicht um; RustPress leitet wie WordPress auf /feed/ um.**: `/?feed=rss2` (status), `/?feed=rss2` (header:content-type), `/?feed=rss2` (header:location), `/?feed=rss2` (header:etag), `/?feed=rss2` (header:last-modified), `/?feed=rss2` (header:link)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-includes/js/jquery/jquery.min.js?ver=3.7.1` (header:cache-control), `/wp-includes/js/jquery/jquery-migrate.min.js?ver=3.4.1` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/images/smilies/mrgreen.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior-150x150.webp` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-migrate.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/wp-mediaelement.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/renderers/vimeo.min.js?ver=4.2.17` (header:cache-control) … und 2 weitere

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
