# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-25 04:00 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**782 URLs verglichen: 18 identisch, 26 mit bekannten Abweichungen, 738 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 318 | 7 | 0 | 311 | 737 ms | 252 ms |
| Feeds | 276 | 0 | 1 | 275 | 306 ms | 24 ms |
| Sitemaps | 12 | 10 | 0 | 2 | 17 ms | 23 ms |
| REST-API | 150 | 0 | 0 | 150 | 17 ms | 25 ms |
| Dateien | 19 | 0 | 19 | 0 | 60 ms | 6 ms |
| Sonstiges | 7 | 1 | 6 | 0 | 482 ms | 162 ms |

## Unerwartete Abweichungen

### `/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
87128 statt 89488 Bytes, erster Unterschied bei 3192:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/?s=` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85784 statt 88144 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Search Results for &#8220;&#8221; Feed" href="http://localhost:8080/index.php/search/feed/rss2/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Search Results for &#8220;&#8221; Feed" href="http://localhost:8080/search/feed/rss2/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/?s=test` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
90888 statt 93248 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Search Results for &#8220;test&#8221; Feed" href="http://localhost:8080/index.php/search/test/feed/rss2/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Search Results for &#8220;test&#8221; Feed" href="http://localhost:8080/search/test/feed/rss2/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/?s=gibt-es-garantiert-nicht-xyz` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
80631 statt 82951 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Search Results for &#8220;gibt-es-garantiert-nicht-xyz&#8221; Feed" href="http://localhost:8080/index.php/search/gibt-es-garantiert-nicht-xyz/feed/rss2/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Search Results for &#8220;gibt-es-garantiert-nicht-xyz&#8221; Feed" href="http://localhost:8080/search/gibt-es-garantiert-nicht-xyz/feed/rss2/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/page/9999/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/gibt-es-nicht/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/buecher/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/buecher/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
126656 statt 129006 Bytes, erster Unterschied bei 364:
RustPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Fe…
WordPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; C…
```


### `/buecher/feed/atom/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
126656 statt 129006 Bytes, erster Unterschied bei 364:
RustPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Fe…
WordPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; C…
```


### `/buecher/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/buecher/buch-1/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/kalender/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/kalender/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
126656 statt 129006 Bytes, erster Unterschied bei 364:
RustPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Fe…
WordPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; C…
```


### `/termine/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/termine/gibt-es-nicht/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/genre/krimi/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
126656 statt 129006 Bytes, erster Unterschied bei 364:
RustPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Fe…
WordPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; C…
```


### `/genre/roman/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/genre/roman/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/genre/gibt-es-nicht/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/secret/geheimnis/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/?secret=geheimnis` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
87196 statt 89556 Bytes, erster Unterschied bei 3192:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/?post_type=secret` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
87196 statt 89556 Bytes, erster Unterschied bei 3192:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/?post_type=book&s=buch` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
83230 statt 85550 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/?p=279` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
126656 statt 129006 Bytes, erster Unterschied bei 364:
RustPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Fe…
WordPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; C…
```


### `/feed/atom/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
126656 statt 129006 Bytes, erster Unterschied bei 364:
RustPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Fe…
WordPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; C…
```


### `/comments/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
126656 statt 129006 Bytes, erster Unterschied bei 364:
RustPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Fe…
WordPress: …e="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />⏎<link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; C…
```


### `/wp-sitemap-posts-post-9999.xml` (Sitemaps)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/types` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
6426 statt 6701 Bytes, erster Unterschied bei 303:
RustPress: …e,"_links":{"collection":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/types"}],"wp:items":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts"}],"c…
WordPress: …e,"_links":{"collection":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/types"}],"wp:items":[{"href":"http:\/\/localhost:8080\/index.php\/wp-jso…
```


### `/wp-json/wp/v2/types/post` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
472 statt 494 Bytes, erster Unterschied bei 295:
RustPress: …e,"_links":{"collection":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/types"}],"wp:items":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts"}],"c…
WordPress: …e,"_links":{"collection":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/types"}],"wp:items":[{"href":"http:\/\/localhost:8080\/index.php\/wp-jso…
```


### `/wp-json/wp/v2/types/book?context=embed` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
359 statt 381 Bytes, erster Unterschied bei 183:
RustPress: …e,"_links":{"collection":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/types"}],"wp:items":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book"}],"cu…
WordPress: …e,"_links":{"collection":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/types"}],"wp:items":[{"href":"http:\/\/localhost:8080\/index.php\/wp-jso…
```


### `/wp-json/wp/v2/types/book?context=edit` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/types/secret` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/types/nope` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/types?context=embed` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
5225 statt 5511 Bytes, erster Unterschied bei 208:
RustPress: …e,"_links":{"collection":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/types"}],"wp:items":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts"}],"c…
WordPress: …e,"_links":{"collection":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/types"}],"wp:items":[{"href":"http:\/\/localhost:8080\/index.php\/wp-jso…
```


### `/wp-json/wp/v2/taxonomies` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
2028 statt 2138 Bytes, erster Unterschied bei 214:
RustPress: …","_links":{"collection":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/taxonomies"}],"wp:items":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/catego…
WordPress: …","_links":{"collection":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/taxonomies"}],"wp:items":[{"href":"http:\/\/localhost:8080\/index.php\/w…
```


### `/wp-json/wp/v2/taxonomies?type=book` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
382 statt 404 Bytes, erster Unterschied bei 199:
RustPress: …","_links":{"collection":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/taxonomies"}],"wp:items":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/genre"…
WordPress: …","_links":{"collection":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/taxonomies"}],"wp:items":[{"href":"http:\/\/localhost:8080\/index.php\/w…
```


### `/wp-json/wp/v2/taxonomies?type=page` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/taxonomies/genre` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
372 statt 394 Bytes, erster Unterschied bei 190:
RustPress: …","_links":{"collection":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/taxonomies"}],"wp:items":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/genre"…
WordPress: …","_links":{"collection":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/taxonomies"}],"wp:items":[{"href":"http:\/\/localhost:8080\/index.php\/w…
```


### `/wp-json/wp/v2/taxonomies/category?context=embed` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
335 statt 357 Bytes, erster Unterschied bei 148:
RustPress: …","_links":{"collection":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/taxonomies"}],"wp:items":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/catego…
WordPress: …","_links":{"collection":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/taxonomies"}],"wp:items":[{"href":"http:\/\/localhost:8080\/index.php\/w…
```


### `/wp-json/wp/v2/taxonomies/post_format` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/taxonomies/nope` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/statuses` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
185 statt 196 Bytes, erster Unterschied bei 157:
RustPress: …lse,"_links":{"archives":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts"}]}}}…
WordPress: …lse,"_links":{"archives":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts"}]}}}…
```


### `/wp-json/wp/v2/statuses/publish` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
173 statt 184 Bytes, erster Unterschied bei 146:
RustPress: …lse,"_links":{"archives":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts"}]}}…
WordPress: …lse,"_links":{"archives":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts"}]}}…
```


### `/wp-json/wp/v2/statuses/draft` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/statuses/nope` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/statuses?context=embed` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
132 statt 143 Bytes, erster Unterschied bei 104:
RustPress: …sh","_links":{"archives":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts"}]}}}…
WordPress: …sh","_links":{"archives":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts"}]}}}…
```


### `/wp-json/wp/v2/search?type=term` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
3960 statt 4290 Bytes, erster Unterschied bei 170:
RustPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/tags\/5","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/localhost:808…
WordPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/tags\/5","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/lo…
```


### `/wp-json/wp/v2/search?type=term&search=roman` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
388 statt 421 Bytes, erster Unterschied bei 169:
RustPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/genre\/9","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/localhost:80…
WordPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/genre\/9","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/l…
```


### `/wp-json/wp/v2/search?type=post-format` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/search?type=bogus` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/search?subtype=bogus` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/search?search=buch` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1213 statt 1312 Bytes, erster Unterschied bei 191:
RustPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/localhost:8…
WordPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/…
```


### `/wp-json/wp/v2/search?search=buch&per_page=2&page=2` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/search?search=buch&per_page=2&page=1>; rel="prev"` statt `</index.php/wp-json/wp/v2/search?search=buch&per_page=2&page=1>; rel="prev"`
- **bytes**

```text
405 statt 438 Bytes, erster Unterschied bei 191:
RustPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book\/274","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/localhost:8…
WordPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/book\/274","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/…
```


### `/wp-json/wp/v2/search?exclude=276&search=buch` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
809 statt 875 Bytes, erster Unterschied bei 191:
RustPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book\/275","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/localhost:8…
WordPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/book\/275","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/…
```


### `/wp-json/wp/v2/search?subtype=page&per_page=3` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/search?subtype%5B0%5D=page&per_page=3&page=2>; rel="next"` statt `</index.php/wp-json/wp/v2/search?subtype%5B0%5D=page&per_page=3&page=2>; rel="next"`
- **bytes**

```text
1173 statt 1305 Bytes, erster Unterschied bei 62:
RustPress: …"id":32,"title":"Bildliste","url":"http:\/\/localhost:8080\/bildliste\/","type":"post","subtype":"page","_links":{"self":[{"embeddable":true,"href":"http:\/\/lo…
WordPress: …"id":32,"title":"Bildliste","url":"http:\/\/localhost:8080\/index.php\/bildliste\/","type":"post","subtype":"page","_links":{"self":[{"embeddable":true,"href":"…
```


### `/wp-json/wp/v2/search?page=999` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/search?page=998>; rel="prev"` statt `</index.php/wp-json/wp/v2/search?page=998>; rel="prev"`

### `/wp-json/wp/v2/search?context=edit` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/settings` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/block-types` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/themes` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/plugins` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/widgets` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/menus` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/templates` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/blocks` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/navigation` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
51960 statt 54567 Bytes, erster Unterschied bei 542:
RustPress: …ck-pages-list__item__link\" href=\"http:\/\/localhost:8080\/bildliste\/\">Bildliste<\/a><\/li><li class=\"wp-block-pages-list__item\"><a class=\"wp-block-pages-…
WordPress: …ck-pages-list__item__link\" href=\"http:\/\/localhost:8080\/index.php\/bildliste\/\">Bildliste<\/a><\/li><li class=\"wp-block-pages-list__item\"><a class=\"wp-b…
```


### `/wp-json/wp/v2/navigation/1` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/book` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
4360 statt 4591 Bytes, erster Unterschied bei 767:
RustPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/book?per_page=2&page=2` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/book?per_page=2&page=1>; rel="prev"` statt `</index.php/wp-json/wp/v2/book?per_page=2&page=1>; rel="prev"`
- **bytes**

```text
1454 statt 1531 Bytes, erster Unterschied bei 767:
RustPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book\/274","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/book\/274","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/book?genre=10` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
4360 statt 4591 Bytes, erster Unterschied bei 767:
RustPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/book?_embed` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
8680 statt 9142 Bytes, erster Unterschied bei 767:
RustPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/book/276` (REST-API)

- **bytes**

```text
1452 statt 1529 Bytes, erster Unterschied bei 766:
RustPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …krimi"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/book\/276","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/book/999` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/event` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
2064 statt 2163 Bytes, erster Unterschied bei 613:
RustPress: …entry"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/event\/278","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …entry"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/event\/278","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/event?orderby=menu_order` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
2064 statt 2163 Bytes, erster Unterschied bei 613:
RustPress: …entry"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/event\/278","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …entry"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/event\/278","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/event?parent=277` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1083 statt 1138 Bytes, erster Unterschied bei 613:
RustPress: …entry"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/event\/278","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …entry"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/event\/278","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/event/278?_embed` (REST-API)

- **bytes**

```text
1710 statt 1809 Bytes, erster Unterschied bei 612:
RustPress: …entry"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/event\/278","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …entry"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/event\/278","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/genre` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1891 statt 2034 Bytes, erster Unterschied bei 219:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/genre\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/genre\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/genre?post=276` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
687 statt 742 Bytes, erster Unterschied bei 219:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/genre\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/genre\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/genre/10?_embed` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1254 statt 1353 Bytes, erster Unterschied bei 218:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/genre\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/genre\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/genre?hide_empty=true` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1283 statt 1382 Bytes, erster Unterschied bei 219:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/genre\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/genre\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/secret` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/search?search=test` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/search?search=test&page=2>; rel="next"` statt `</index.php/wp-json/wp/v2/search?search=test&page=2>; rel="next"`
- **bytes**

```text
4081 statt 4411 Bytes, erster Unterschied bei 197:
RustPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/16","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/localhost:8…
WordPress: …"self":[{"embeddable":true,"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/16","targetHints":{"allow":["GET"]}}],"about":[{"href":"http:\/\/…
```


### `/wp-json/wp/v2/posts` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/posts?page=2>; rel="next"` statt `</index.php/wp-json/wp/v2/posts?page=2>; rel="next"`
- **bytes**

```text
40557 statt 41558 Bytes, erster Unterschied bei 831:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/272","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/272","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/pages` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/pages?page=2>; rel="next"` statt `</index.php/wp-json/wp/v2/pages?page=2>; rel="next"`
- **bytes**

```text
18475 statt 19377 Bytes, erster Unterschied bei 286:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/bildliste\/","title":{"rendered":"Bildliste"},"content":{"rendered":"\n<div class=\"wp-block-query i…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/bildliste\/","title":{"rendered":"Bildliste"},"content":{"rendered":"\n<div class=\"wp-bl…
```


### `/wp-json/wp/v2/media` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
5977 statt 6076 Bytes, erster Unterschied bei 3188:
RustPress: …:199724,"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/media\/26","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …:199724,"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/media\/26","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/categories` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
2601 statt 2788 Bytes, erster Unterschied bei 222:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/categories\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/lo…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/categories\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"…
```


### `/wp-json/wp/v2/tags` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1761 statt 1893 Bytes, erster Unterschied bei 209:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/tags\/5","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localhos…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/tags\/5","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\…
```


### `/wp-json/wp/v2/users` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1542 statt 1586 Bytes, erster Unterschied bei 637:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/users\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/users\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/comments` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/comments?page=2>; rel="next"` statt `</index.php/wp-json/wp/v2/comments?page=2>; rel="next"`
- **bytes**

```text
12682 statt 13078 Bytes, erster Unterschied bei 958:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/18","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/18","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/posts/99999` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/categories?orderby=include` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
2601 statt 2788 Bytes, erster Unterschied bei 222:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/categories\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/lo…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/categories\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"…
```


### `/wp-json/wp/v2/categories?orderby=include_slugs` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
2601 statt 2788 Bytes, erster Unterschied bei 222:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/categories\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/lo…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/categories\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"…
```


### `/wp-json/wp/v2/posts?orderby=include` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/posts?per_page=0` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-json/wp/v2/posts?context=bogus` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/wp-sitemap-posts-page-1.xml` (Sitemaps)

- **bytes**

```text
30478 statt 32798 Bytes, erster Unterschied bei 299:
RustPress: …:17:18+00:00</lastmod></url><url><loc>http://localhost:8080/sample-page/</loc><lastmod>2026-10-24T14:08:17+00:00</lastmod></url><url><loc>http://localhost:8080/…
WordPress: …:17:18+00:00</lastmod></url><url><loc>http://localhost:8080/index.php/sample-page/</loc><lastmod>2026-10-24T14:08:17+00:00</lastmod></url><url><loc>http://local…
```


### `/2026/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/2026/10/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/2026/10/24/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/2026/13/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/2026/10/24/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/index.php/hello-world/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
86177 statt 88497 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fhello-world%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fhello-world%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/1" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fhello-world%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fhello-world%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/1" />

### `/index.php/hello-world/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-1/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
89528 statt 91848 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-1%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-1%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/5" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-1%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-1%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/5" />

### `/index.php/testbeitrag-1/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85662 statt 87982 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-2%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-2%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/6" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-2%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-2%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/6" />

### `/index.php/testbeitrag-2/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-3/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
86077 statt 88397 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-3%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-3%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/7" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-3%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-3%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/7" />

### `/index.php/testbeitrag-3/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-4/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85662 statt 87982 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-4%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-4%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/8" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-4%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-4%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/8" />

### `/index.php/testbeitrag-4/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-5/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
86077 statt 88397 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-5%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-5%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/9" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-5%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-5%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/9" />

### `/index.php/testbeitrag-5/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-6/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85663 statt 87983 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-6%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-6%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/10" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-6%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-6%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/10" />

### `/index.php/testbeitrag-6/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-7/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85350 statt 87670 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-7%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-7%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/11" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-7%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-7%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/11" />

### `/index.php/testbeitrag-7/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-8/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85350 statt 87670 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-8%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-8%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/12" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-8%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-8%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/12" />

### `/index.php/testbeitrag-8/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-9/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85352 statt 87672 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-9%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-9%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/13" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-9%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-9%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/13" />

### `/index.php/testbeitrag-9/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-10/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85357 statt 87677 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-10%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-10%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/14" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-10%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-10%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/14" />

### `/index.php/testbeitrag-10/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-11/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85359 statt 87679 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-11%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-11%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/15" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-11%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-11%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/15" />

### `/index.php/testbeitrag-11/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/testbeitrag-12/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85359 statt 87679 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-12%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-12%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/16" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-12%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftestbeitrag-12%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/16" />

### `/index.php/testbeitrag-12/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/klassisch/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
84044 statt 86364 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassisch%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassisch%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/17" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassisch%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassisch%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/17" />

### `/index.php/klassisch/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/geschuetzt/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
82509 statt 84829 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgeschuetzt%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgeschuetzt%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/19" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgeschuetzt%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgeschuetzt%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/19" />

### `/index.php/geschuetzt/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/mehrseitig/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
83932 statt 86252 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmehrseitig%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmehrseitig%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/22" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmehrseitig%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmehrseitig%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/22" />

### `/index.php/mehrseitig/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/gleich-a/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
83732 statt 86052 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgleich-a%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgleich-a%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/23" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgleich-a%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgleich-a%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/23" />

### `/index.php/gleich-a/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/gleich-b/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
83918 statt 86238 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgleich-b%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgleich-b%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/24" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgleich-b%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fgleich-b%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/24" />

### `/index.php/gleich-b/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/bilder/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
86289 statt 88609 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbilder%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbilder%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/27" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbilder%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbilder%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/27" />

### `/index.php/bilder/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/klassische-bilder/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
84339 statt 86659 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassische-bilder%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassische-bilder%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/28" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassische-bilder%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassische-bilder%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/28" />

### `/index.php/klassische-bilder/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/logo/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
84743 statt 87063 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Flogo%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Flogo%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/29" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Flogo%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Flogo%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/29" />

### `/index.php/logo/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/viele-bilder/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
86989 statt 89309 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fviele-bilder%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fviele-bilder%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/30" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fviele-bilder%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fviele-bilder%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/30" />

### `/index.php/viele-bilder/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/cover-bilder/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85647 statt 87967 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fcover-bilder%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fcover-bilder%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/31" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fcover-bilder%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fcover-bilder%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/31" />

### `/index.php/cover-bilder/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/dynamische-bloecke/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
92345 statt 94665 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fdynamische-bloecke%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fdynamische-bloecke%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/34" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fdynamische-bloecke%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fdynamische-bloecke%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/34" />

### `/index.php/dynamische-bloecke/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/abfrage-bloecke/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
86616 statt 88936 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fabfrage-bloecke%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fabfrage-bloecke%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/35" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fabfrage-bloecke%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fabfrage-bloecke%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/35" />

### `/index.php/abfrage-bloecke/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/klassischer-inhalt/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85970 statt 88290 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassischer-inhalt%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassischer-inhalt%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/36" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassischer-inhalt%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fklassischer-inhalt%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/36" />

### `/index.php/klassischer-inhalt/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/smilies/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
84210 statt 86530 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fsmilies%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fsmilies%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/265" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fsmilies%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fsmilies%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/265" />

### `/index.php/smilies/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/mit-teaser/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
84007 statt 86327 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmit-teaser%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmit-teaser%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/266" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmit-teaser%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmit-teaser%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/266" />

### `/index.php/mit-teaser/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/medien/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85182 statt 87502 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmedien%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmedien%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/271" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmedien%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fmedien%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/271" />

### `/index.php/medien/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/diskussion/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85637 statt 87957 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fdiskussion%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fdiskussion%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/posts/272" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fdiskussion%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fdiskussion%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/posts/272" />

### `/index.php/diskussion/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/sample-page/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76451 statt 78771 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Sample Page Comments Feed" href="http://localhost:8080/index.php/sample-page/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fsample-page%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fsample-page%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/2" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/sample-page/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Sample Page Comments Feed" href="http://localhost:8080/sample-page/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fsample-page%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fsample-page%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/2" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/sample-page/" />

### `/index.php/sample-page/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/ueber-uns/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75968 statt 78288 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fueber-uns%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fueber-uns%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/20" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/ueber-uns/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fueber-uns%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fueber-uns%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/20" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/ueber-uns/" />

### `/index.php/ueber-uns/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/ueber-uns/team/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75231 statt 77551 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fueber-uns%2Fteam%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fueber-uns%2Fteam%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/21" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/ueber-uns/team/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fueber-uns%2Fteam%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fueber-uns%2Fteam%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/21" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/ueber-uns/team/" />

### `/index.php/ueber-uns/team/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/bildliste/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77142 statt 79462 Bytes, erster Unterschied bei 3190:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/" aria-current="page">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navi…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/" aria-current="page">Bildliste</a></li><li class="wp-block-pages-list__item wp-…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbildliste%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbildliste%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/32" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/bildliste/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fbildliste%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fbildliste%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/32" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/bildliste/" />

### `/index.php/bildliste/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-banner-about-book/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76885 statt 79205 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item current-menu-item wp-block-naviga…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item current-menu-item wp-bl…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-about-book%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-about-book%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/37" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-banner-about-book/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-about-book%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-about-book%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/37" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-banner-about-book/" />

### `/index.php/pat-twentytwentyfive-banner-about-book/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-banner-cover-big-heading/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76135 statt 78455 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-cover-big-heading%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-cover-big-heading%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/38" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-banner-cover-big-heading/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-cover-big-heading%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-cover-big-heading%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/38" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-banner-cover-big-heading/" />

### `/index.php/pat-twentytwentyfive-banner-cover-big-heading/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-banner-intro-image/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76467 statt 78787 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-intro-image%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-intro-image%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/39" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-banner-intro-image/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-intro-image%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-intro-image%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/39" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-banner-intro-image/" />

### `/index.php/pat-twentytwentyfive-banner-intro-image/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-banner-intro/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75699 statt 78019 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-intro%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-intro%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/40" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-banner-intro/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-intro%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-intro%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/40" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-banner-intro/" />

### `/index.php/pat-twentytwentyfive-banner-intro/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-banner-poster/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77706 statt 80026 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-poster%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-poster%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/41" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-banner-poster/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-poster%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-poster%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/41" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-banner-poster/" />

### `/index.php/pat-twentytwentyfive-banner-poster/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-banner-description-images-grid/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76890 statt 79210 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-description-images-grid%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-banner-description-images-grid%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/42" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-banner-description-images-grid/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-description-images-grid%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-banner-description-images-grid%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/42" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-banner-description-images-grid/" />

### `/index.php/pat-twentytwentyfive-banner-description-images-grid/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-binding-format/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75255 statt 77575 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-binding-format%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-binding-format%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/43" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-binding-format/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-binding-format%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-binding-format%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/43" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-binding-format/" />

### `/index.php/pat-twentytwentyfive-binding-format/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-comments/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75195 statt 77515 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-comments%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-comments%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/44" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-comments/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-comments%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-comments%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/44" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-comments/" />

### `/index.php/pat-twentytwentyfive-comments/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-contact-centered-social-link/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
81231 statt 83551 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-contact-centered-social-link%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-contact-centered-social-link%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/45" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-contact-centered-social-link/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-contact-centered-social-link%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-contact-centered-social-link%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/45" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-contact-centered-social-link/" />

### `/index.php/pat-twentytwentyfive-contact-centered-social-link/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-contact-info-locations/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
79078 statt 81398 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-contact-info-locations%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-contact-info-locations%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/46" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-contact-info-locations/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-contact-info-locations%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-contact-info-locations%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/46" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-contact-info-locations/" />

### `/index.php/pat-twentytwentyfive-contact-info-locations/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-contact-location-and-link/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76553 statt 78873 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-contact-location-and-link%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-contact-location-and-link%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/47" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-contact-location-and-link/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-contact-location-and-link%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-contact-location-and-link%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/47" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-contact-location-and-link/" />

### `/index.php/pat-twentytwentyfive-contact-location-and-link/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-cta-book-links/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76782 statt 79102 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-book-links%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-book-links%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/48" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-cta-book-links/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-book-links%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-book-links%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/48" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-cta-book-links/" />

### `/index.php/pat-twentytwentyfive-cta-book-links/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-cta-book-locations/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
79534 statt 81854 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-book-locations%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-book-locations%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/49" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-cta-book-locations/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-book-locations%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-book-locations%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/49" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-cta-book-locations/" />

### `/index.php/pat-twentytwentyfive-cta-book-locations/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-cta-centered-heading/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76354 statt 78674 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-centered-heading%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-centered-heading%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/50" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-cta-centered-heading/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-centered-heading%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-centered-heading%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/50" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-cta-centered-heading/" />

### `/index.php/pat-twentytwentyfive-cta-centered-heading/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-cta-events-list/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
80011 statt 82331 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-events-list%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-events-list%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/51" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-cta-events-list/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-events-list%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-events-list%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/51" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-cta-events-list/" />

### `/index.php/pat-twentytwentyfive-cta-events-list/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-cta-grid-products-link/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
82118 statt 84438 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-grid-products-link%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-grid-products-link%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/52" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-cta-grid-products-link/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-grid-products-link%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-grid-products-link%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/52" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-cta-grid-products-link/" />

### `/index.php/pat-twentytwentyfive-cta-grid-products-link/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-cta-heading-search/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76197 statt 78517 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-heading-search%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-heading-search%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/53" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-cta-heading-search/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-heading-search%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-heading-search%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/53" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-cta-heading-search/" />

### `/index.php/pat-twentytwentyfive-cta-heading-search/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-cta-newsletter/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76541 statt 78861 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-newsletter%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-cta-newsletter%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/54" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-cta-newsletter/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-newsletter%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-cta-newsletter%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/54" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-cta-newsletter/" />

### `/index.php/pat-twentytwentyfive-cta-newsletter/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-event-3-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78659 statt 80979 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-event-3-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-event-3-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/55" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-event-3-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-event-3-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-event-3-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/55" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-event-3-col/" />

### `/index.php/pat-twentytwentyfive-event-3-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-event-rsvp/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78695 statt 81015 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-event-rsvp%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-event-rsvp%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/56" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-event-rsvp/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-event-rsvp%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-event-rsvp%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/56" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-event-rsvp/" />

### `/index.php/pat-twentytwentyfive-event-rsvp/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-event-schedule/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
82643 statt 84963 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-event-schedule%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-event-schedule%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/57" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-event-schedule/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-event-schedule%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-event-schedule%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/57" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-event-schedule/" />

### `/index.php/pat-twentytwentyfive-event-schedule/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-footer-centered/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75971 statt 78291 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer-centered%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer-centered%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/58" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-footer-centered/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer-centered%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer-centered%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/58" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-footer-centered/" />

### `/index.php/pat-twentytwentyfive-footer-centered/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-footer-columns/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
79316 statt 81636 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer-columns%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer-columns%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/59" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-footer-columns/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer-columns%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer-columns%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/59" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-footer-columns/" />

### `/index.php/pat-twentytwentyfive-footer-columns/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-footer-newsletter/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76471 statt 78791 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer-newsletter%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer-newsletter%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/60" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-footer-newsletter/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer-newsletter%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer-newsletter%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/60" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-footer-newsletter/" />

### `/index.php/pat-twentytwentyfive-footer-newsletter/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-footer/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
79074 statt 81394 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/61" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-footer/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/61" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-footer/" />

### `/index.php/pat-twentytwentyfive-footer/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-footer-social/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77167 statt 79487 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer-social%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-footer-social%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/62" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-footer-social/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer-social%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-footer-social%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/62" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-footer-social/" />

### `/index.php/pat-twentytwentyfive-footer-social/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-format-audio/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76502 statt 78822 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-format-audio%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-format-audio%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/63" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-format-audio/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-format-audio%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-format-audio%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/63" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-format-audio/" />

### `/index.php/pat-twentytwentyfive-format-audio/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-format-link/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76077 statt 78397 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-format-link%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-format-link%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/64" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-format-link/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-format-link%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-format-link%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/64" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-format-link/" />

### `/index.php/pat-twentytwentyfive-format-link/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-grid-videos/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76281 statt 78601 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-grid-videos%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-grid-videos%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/65" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-grid-videos/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-grid-videos%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-grid-videos%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/65" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-grid-videos/" />

### `/index.php/pat-twentytwentyfive-grid-videos/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-grid-with-categories/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78328 statt 80648 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-grid-with-categories%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-grid-with-categories%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/66" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-grid-with-categories/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-grid-with-categories%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-grid-with-categories%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/66" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-grid-with-categories/" />

### `/index.php/pat-twentytwentyfive-grid-with-categories/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-header-centered/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
145533 statt 150173 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-header-centered%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-header-centered%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/67" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-header-centered/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-header-centered%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-header-centered%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/67" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-header-centered/" />

### `/index.php/pat-twentytwentyfive-header-centered/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-header-columns/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
146055 statt 150695 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-header-columns%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-header-columns%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/68" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-header-columns/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-header-columns%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-header-columns%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/68" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-header-columns/" />

### `/index.php/pat-twentytwentyfive-header-columns/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-header-large-title/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
145845 statt 150485 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-header-large-title%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-header-large-title%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/69" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-header-large-title/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-header-large-title%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-header-large-title%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/69" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-header-large-title/" />

### `/index.php/pat-twentytwentyfive-header-large-title/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-header/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
145656 statt 150296 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-header%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-header%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/70" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-header/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-header%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-header%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/70" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-header/" />

### `/index.php/pat-twentytwentyfive-header/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-heading-and-paragraph-with-image/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76799 statt 79119 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-heading-and-paragraph-with-image%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-heading-and-paragraph-with-image%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/71" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-heading-and-paragraph-with-image/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-heading-and-paragraph-with-image%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-heading-and-paragraph-with-image%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/71" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-heading-and-paragraph-with-image/" />

### `/index.php/pat-twentytwentyfive-heading-and-paragraph-with-image/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hero-book/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76861 statt 79181 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hero-book%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hero-book%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/72" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hero-book/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hero-book%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hero-book%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/72" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hero-book/" />

### `/index.php/pat-twentytwentyfive-hero-book/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hero-full-width-image/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76748 statt 79068 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hero-full-width-image%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hero-full-width-image%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/73" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hero-full-width-image/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hero-full-width-image%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hero-full-width-image%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/73" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hero-full-width-image/" />

### `/index.php/pat-twentytwentyfive-hero-full-width-image/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hero-overlapped-book-cover-with-links/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78973 statt 81293 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hero-overlapped-book-cover-with-links%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hero-overlapped-book-cover-with-links%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/74" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hero-overlapped-book-cover-with-links/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hero-overlapped-book-cover-with-links%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hero-overlapped-book-cover-with-links%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/74" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hero-overlapped-book-cover-with-links/" />

### `/index.php/pat-twentytwentyfive-hero-overlapped-book-cover-with-links/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hero-podcast/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77313 statt 79633 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hero-podcast%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hero-podcast%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/75" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hero-podcast/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hero-podcast%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hero-podcast%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/75" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hero-podcast/" />

### `/index.php/pat-twentytwentyfive-hero-podcast/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hidden-404/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76795 statt 79115 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-404%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-404%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/76" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hidden-404/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-404%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-404%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/76" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hidden-404/" />

### `/index.php/pat-twentytwentyfive-hidden-404/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hidden-blog-heading/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75264 statt 77584 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-blog-heading%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-blog-heading%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/77" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hidden-blog-heading/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-blog-heading%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-blog-heading%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/77" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hidden-blog-heading/" />

### `/index.php/pat-twentytwentyfive-hidden-blog-heading/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hidden-search/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75759 statt 78079 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-search%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-search%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/78" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hidden-search/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-search%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-search%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/78" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hidden-search/" />

### `/index.php/pat-twentytwentyfive-hidden-search/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hidden-sidebar/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
79025 statt 81345 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-sidebar%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-sidebar%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/79" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hidden-sidebar/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-sidebar%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-sidebar%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/79" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hidden-sidebar/" />

### `/index.php/pat-twentytwentyfive-hidden-sidebar/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-hidden-written-by/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75645 statt 77965 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-written-by%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-hidden-written-by%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/80" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-hidden-written-by/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-written-by%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-hidden-written-by%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/80" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-hidden-written-by/" />

### `/index.php/pat-twentytwentyfive-hidden-written-by/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-logos/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75836 statt 78156 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-logos%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-logos%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/81" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-logos/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-logos%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-logos%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/81" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-logos/" />

### `/index.php/pat-twentytwentyfive-logos/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-media-instagram-grid/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77944 statt 80264 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-media-instagram-grid%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-media-instagram-grid%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/82" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-media-instagram-grid/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-media-instagram-grid%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-media-instagram-grid%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/82" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-media-instagram-grid/" />

### `/index.php/pat-twentytwentyfive-media-instagram-grid/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-more-posts/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
79835 statt 82155 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-more-posts%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-more-posts%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/83" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-more-posts/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-more-posts%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-more-posts%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/83" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-more-posts/" />

### `/index.php/pat-twentytwentyfive-more-posts/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-overlapped-images/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77166 statt 79486 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-overlapped-images%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-overlapped-images%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/84" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-overlapped-images/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-overlapped-images%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-overlapped-images%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/84" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-overlapped-images/" />

### `/index.php/pat-twentytwentyfive-overlapped-images/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-business-home/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
86565 statt 88885 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-business-home%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-business-home%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/85" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-business-home/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-business-home%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-business-home%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/85" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-business-home/" />

### `/index.php/pat-twentytwentyfive-page-business-home/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-coming-soon/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76953 statt 79273 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-coming-soon%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-coming-soon%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/86" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-coming-soon/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-coming-soon%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-coming-soon%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/86" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-coming-soon/" />

### `/index.php/pat-twentytwentyfive-page-coming-soon/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-cv-bio/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77824 statt 80144 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-cv-bio%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-cv-bio%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/87" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-cv-bio/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-cv-bio%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-cv-bio%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/87" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-cv-bio/" />

### `/index.php/pat-twentytwentyfive-page-cv-bio/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-landing-book/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
88996 statt 91316 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-landing-book%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-landing-book%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/88" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-landing-book/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-landing-book%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-landing-book%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/88" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-landing-book/" />

### `/index.php/pat-twentytwentyfive-page-landing-book/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-landing-event/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
89210 statt 91530 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-landing-event%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-landing-event%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/89" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-landing-event/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-landing-event%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-landing-event%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/89" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-landing-event/" />

### `/index.php/pat-twentytwentyfive-page-landing-event/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-landing-podcast/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
81972 statt 84292 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-landing-podcast%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-landing-podcast%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/90" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-landing-podcast/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-landing-podcast%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-landing-podcast%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/90" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-landing-podcast/" />

### `/index.php/pat-twentytwentyfive-page-landing-podcast/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77226 statt 79546 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/91" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/91" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image/" />

### `/index.php/pat-twentytwentyfive-page-link-in-bio-heading-paragraph-links-image/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-link-in-bio-wide-margins/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
82421 statt 84741 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-link-in-bio-wide-margins%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-link-in-bio-wide-margins%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/92" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-link-in-bio-wide-margins/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-link-in-bio-wide-margins%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-link-in-bio-wide-margins%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/92" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-link-in-bio-wide-margins/" />

### `/index.php/pat-twentytwentyfive-page-link-in-bio-wide-margins/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-link-in-bio-with-tight-margins/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77343 statt 79663 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-link-in-bio-with-tight-margins%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-link-in-bio-with-tight-margins%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/93" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-link-in-bio-with-tight-margins/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-link-in-bio-with-tight-margins%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-link-in-bio-with-tight-margins%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/93" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-link-in-bio-with-tight-margins/" />

### `/index.php/pat-twentytwentyfive-page-link-in-bio-with-tight-margins/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-portfolio-home/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85561 statt 87881 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-portfolio-home%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-portfolio-home%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/94" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-portfolio-home/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-portfolio-home%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-portfolio-home%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/94" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-portfolio-home/" />

### `/index.php/pat-twentytwentyfive-page-portfolio-home/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-page-shop-home/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
82321 statt 84641 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-shop-home%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-page-shop-home%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/95" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-page-shop-home/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-shop-home%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-page-shop-home%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/95" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-page-shop-home/" />

### `/index.php/pat-twentytwentyfive-page-shop-home/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-post-navigation/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76369 statt 78709 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-post-navigation%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-post-navigation%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/96" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-post-navigation/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-post-navigation%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-post-navigation%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/96" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-post-navigation/" />

### `/index.php/pat-twentytwentyfive-post-navigation/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-pricing-2-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78140 statt 80460 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-pricing-2-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-pricing-2-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/97" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-pricing-2-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-pricing-2-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-pricing-2-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/97" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-pricing-2-col/" />

### `/index.php/pat-twentytwentyfive-pricing-2-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-pricing-3-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
80959 statt 83279 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-pricing-3-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-pricing-3-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/98" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-pricing-3-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-pricing-3-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-pricing-3-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/98" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-pricing-3-col/" />

### `/index.php/pat-twentytwentyfive-pricing-3-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-services-3-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77309 statt 79629 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-services-3-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-services-3-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/99" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-services-3-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-services-3-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-services-3-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/99" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-services-3-col/" />

### `/index.php/pat-twentytwentyfive-services-3-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-services-subscriber-only-section/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77240 statt 79560 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-services-subscriber-only-section%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-services-subscriber-only-section%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/100" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-services-subscriber-only-section/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-services-subscriber-only-section%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-services-subscriber-only-section%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/100" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-services-subscriber-only-section/" />

### `/index.php/pat-twentytwentyfive-services-subscriber-only-section/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-services-team-photos/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77008 statt 79328 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-services-team-photos%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-services-team-photos%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/101" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-services-team-photos/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-services-team-photos%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-services-team-photos%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/101" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-services-team-photos/" />

### `/index.php/pat-twentytwentyfive-services-team-photos/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-404-vertical-header-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
152181 statt 156821 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-404-vertical-header-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-404-vertical-header-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/102" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-404-vertical-header-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-404-vertical-header-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-404-vertical-header-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/102" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-404-vertical-header-blog/" />

### `/index.php/pat-twentytwentyfive-template-404-vertical-header-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-archive-news-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
149650 statt 154300 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-archive-news-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-archive-news-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/103" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-archive-news-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-archive-news-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-archive-news-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/103" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-archive-news-blog/" />

### `/index.php/pat-twentytwentyfive-template-archive-news-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-archive-photo-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
150496 statt 155136 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-archive-photo-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-archive-photo-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/104" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-archive-photo-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-archive-photo-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-archive-photo-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/104" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-archive-photo-blog/" />

### `/index.php/pat-twentytwentyfive-template-archive-photo-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-archive-text-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
151464 statt 156124 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-archive-text-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-archive-text-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/105" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-archive-text-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-archive-text-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-archive-text-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/105" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-archive-text-blog/" />

### `/index.php/pat-twentytwentyfive-template-archive-text-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-archive-vertical-header-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
152158 statt 156818 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-archive-vertical-header-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-archive-vertical-header-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/106" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-archive-vertical-header-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-archive-vertical-header-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-archive-vertical-header-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/106" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-archive-vertical-header-blog/" />

### `/index.php/pat-twentytwentyfive-template-archive-vertical-header-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-home-news-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
158915 statt 163555 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-news-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-news-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/107" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-home-news-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-news-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-news-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/107" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-home-news-blog/" />

### `/index.php/pat-twentytwentyfive-template-home-news-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-home-photo-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
150879 statt 155519 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-photo-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-photo-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/108" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-home-photo-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-photo-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-photo-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/108" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-home-photo-blog/" />

### `/index.php/pat-twentytwentyfive-template-home-photo-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-home-posts-grid-news-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
162335 statt 166975 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-posts-grid-news-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-posts-grid-news-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/109" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-home-posts-grid-news-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-posts-grid-news-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-posts-grid-news-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/109" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-home-posts-grid-news-blog/" />

### `/index.php/pat-twentytwentyfive-template-home-posts-grid-news-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-home-text-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
151522 statt 156182 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-text-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-text-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/110" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-home-text-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-text-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-text-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/110" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-home-text-blog/" />

### `/index.php/pat-twentytwentyfive-template-home-text-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-home-vertical-header-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
152204 statt 156864 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-vertical-header-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-vertical-header-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/111" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-home-vertical-header-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-vertical-header-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-vertical-header-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/111" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-home-vertical-header-blog/" />

### `/index.php/pat-twentytwentyfive-template-home-vertical-header-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-home-with-sidebar-news-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
164641 statt 169281 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-with-sidebar-news-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-home-with-sidebar-news-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/112" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-home-with-sidebar-news-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-with-sidebar-news-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-home-with-sidebar-news-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/112" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-home-with-sidebar-news-blog/" />

### `/index.php/pat-twentytwentyfive-template-home-with-sidebar-news-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-page-photo-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
150217 statt 154857 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-page-photo-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-page-photo-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/113" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-page-photo-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-page-photo-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-page-photo-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/113" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-page-photo-blog/" />

### `/index.php/pat-twentytwentyfive-template-page-photo-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-page-vertical-header-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
155217 statt 159857 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-page-vertical-header-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-page-vertical-header-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/114" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-page-vertical-header-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-page-vertical-header-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-page-vertical-header-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/114" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-page-vertical-header-blog/" />

### `/index.php/pat-twentytwentyfive-template-page-vertical-header-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-query-loop-news-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77164 statt 79494 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop-news-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop-news-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/115" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-query-loop-news-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop-news-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop-news-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/115" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-query-loop-news-blog/" />

### `/index.php/pat-twentytwentyfive-template-query-loop-news-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-query-loop-photo-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75895 statt 78215 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop-photo-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop-photo-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/116" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-query-loop-photo-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop-photo-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop-photo-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/116" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-query-loop-photo-blog/" />

### `/index.php/pat-twentytwentyfive-template-query-loop-photo-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-query-loop/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76475 statt 78815 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/117" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-query-loop/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/117" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-query-loop/" />

### `/index.php/pat-twentytwentyfive-template-query-loop/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-query-loop-text-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76772 statt 79112 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop-text-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop-text-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/118" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-query-loop-text-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop-text-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop-text-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/118" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-query-loop-text-blog/" />

### `/index.php/pat-twentytwentyfive-template-query-loop-text-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-query-loop-vertical-header-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76707 statt 79047 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop-vertical-header-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-query-loop-vertical-header-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/119" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-query-loop-vertical-header-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop-vertical-header-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-query-loop-vertical-header-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/119" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-query-loop-vertical-header-blog/" />

### `/index.php/pat-twentytwentyfive-template-query-loop-vertical-header-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-search-news-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
150029 statt 154679 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-search-news-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-search-news-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/120" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-search-news-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-search-news-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-search-news-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/120" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-search-news-blog/" />

### `/index.php/pat-twentytwentyfive-template-search-news-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-search-photo-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
151143 statt 155783 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-search-photo-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-search-photo-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/121" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-search-photo-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-search-photo-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-search-photo-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/121" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-search-photo-blog/" />

### `/index.php/pat-twentytwentyfive-template-search-photo-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-search-text-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
152108 statt 156768 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-search-text-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-search-text-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/122" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-search-text-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-search-text-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-search-text-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/122" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-search-text-blog/" />

### `/index.php/pat-twentytwentyfive-template-search-text-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-search-vertical-header-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
152713 statt 157373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-search-vertical-header-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-search-vertical-header-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/123" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-search-vertical-header-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-search-vertical-header-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-search-vertical-header-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/123" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-search-vertical-header-blog/" />

### `/index.php/pat-twentytwentyfive-template-search-vertical-header-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-post-with-left-aligned-content/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
154236 statt 158896 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-post-with-left-aligned-content%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-post-with-left-aligned-content%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/124" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-post-with-left-aligned-content/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-post-with-left-aligned-content%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-post-with-left-aligned-content%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/124" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-post-with-left-aligned-content/" />

### `/index.php/pat-twentytwentyfive-post-with-left-aligned-content/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-single-news-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
156627 statt 161287 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-news-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-news-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/125" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-single-news-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-news-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-news-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/125" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-single-news-blog/" />

### `/index.php/pat-twentytwentyfive-template-single-news-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-single-offset/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
153250 statt 157910 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-offset%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-offset%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/126" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-single-offset/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-offset%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-offset%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/126" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-single-offset/" />

### `/index.php/pat-twentytwentyfive-template-single-offset/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-single-photo-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
153198 statt 157858 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-photo-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-photo-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/127" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-single-photo-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-photo-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-photo-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/127" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-single-photo-blog/" />

### `/index.php/pat-twentytwentyfive-template-single-photo-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-single-text-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
151787 statt 156447 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-text-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-text-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/128" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-single-text-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-text-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-text-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/128" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-single-text-blog/" />

### `/index.php/pat-twentytwentyfive-template-single-text-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-template-single-vertical-header-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
158201 statt 162861 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-vertical-header-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-template-single-vertical-header-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/129" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-template-single-vertical-header-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-vertical-header-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-template-single-vertical-header-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/129" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-template-single-vertical-header-blog/" />

### `/index.php/pat-twentytwentyfive-template-single-vertical-header-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-testimonials-2-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77702 statt 80022 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-testimonials-2-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-testimonials-2-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/130" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-testimonials-2-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-testimonials-2-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-testimonials-2-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/130" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-testimonials-2-col/" />

### `/index.php/pat-twentytwentyfive-testimonials-2-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-testimonials-6-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
82179 statt 84499 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-testimonials-6-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-testimonials-6-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/131" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-testimonials-6-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-testimonials-6-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-testimonials-6-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/131" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-testimonials-6-col/" />

### `/index.php/pat-twentytwentyfive-testimonials-6-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-testimonials-large/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
77068 statt 79388 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-testimonials-large%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-testimonials-large%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/132" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-testimonials-large/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-testimonials-large%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-testimonials-large%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/132" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-testimonials-large/" />

### `/index.php/pat-twentytwentyfive-testimonials-large/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-text-faqs/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78392 statt 80712 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-text-faqs%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-text-faqs%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/133" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-text-faqs/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-text-faqs%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-text-faqs%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/133" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-text-faqs/" />

### `/index.php/pat-twentytwentyfive-text-faqs/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfive-vertical-header/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
145697 statt 150337 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-vertical-header%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfive-vertical-header%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/134" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfive-vertical-header/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-vertical-header%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfive-vertical-header%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/134" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfive-vertical-header/" />

### `/index.php/pat-twentytwentyfive-vertical-header/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-banner-hero/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75048 statt 77368 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-banner-hero%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-banner-hero%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/135" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-banner-hero/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-banner-hero%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-banner-hero%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/135" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-banner-hero/" />

### `/index.php/pat-twentytwentyfour-banner-hero/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-banner-project-description/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75063 statt 77383 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-banner-project-description%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-banner-project-description%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/136" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-banner-project-description/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-banner-project-description%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-banner-project-description%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/136" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-banner-project-description/" />

### `/index.php/pat-twentytwentyfour-banner-project-description/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-cta-content-image-on-right/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75063 statt 77383 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-content-image-on-right%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-content-image-on-right%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/137" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-cta-content-image-on-right/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-content-image-on-right%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-content-image-on-right%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/137" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-cta-content-image-on-right/" />

### `/index.php/pat-twentytwentyfour-cta-content-image-on-right/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-cta-pricing/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75048 statt 77368 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-pricing%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-pricing%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/138" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-cta-pricing/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-pricing%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-pricing%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/138" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-cta-pricing/" />

### `/index.php/pat-twentytwentyfour-cta-pricing/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-cta-rsvp/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75045 statt 77365 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-rsvp%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-rsvp%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/139" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-cta-rsvp/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-rsvp%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-rsvp%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/139" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-cta-rsvp/" />

### `/index.php/pat-twentytwentyfour-cta-rsvp/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-cta-services-image-left/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-services-image-left%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-services-image-left%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/140" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-cta-services-image-left/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-services-image-left%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-services-image-left%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/140" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-cta-services-image-left/" />

### `/index.php/pat-twentytwentyfour-cta-services-image-left/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-cta-subscribe-centered/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-subscribe-centered%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-cta-subscribe-centered%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/141" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-cta-subscribe-centered/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-subscribe-centered%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-cta-subscribe-centered%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/141" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-cta-subscribe-centered/" />

### `/index.php/pat-twentytwentyfour-cta-subscribe-centered/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-footer-centered-logo-nav/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75061 statt 77381 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-footer-centered-logo-nav%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-footer-centered-logo-nav%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/142" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-footer-centered-logo-nav/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-footer-centered-logo-nav%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-footer-centered-logo-nav%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/142" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-footer-centered-logo-nav/" />

### `/index.php/pat-twentytwentyfour-footer-centered-logo-nav/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-footer-colophon-3-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75058 statt 77378 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-footer-colophon-3-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-footer-colophon-3-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/143" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-footer-colophon-3-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-footer-colophon-3-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-footer-colophon-3-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/143" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-footer-colophon-3-col/" />

### `/index.php/pat-twentytwentyfour-footer-colophon-3-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-footer/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75043 statt 77363 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-footer%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-footer%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/144" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-footer/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-footer%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-footer%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/144" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-footer/" />

### `/index.php/pat-twentytwentyfour-footer/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-gallery-full-screen-image/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75062 statt 77382 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-full-screen-image%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-full-screen-image%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/145" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-gallery-full-screen-image/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-full-screen-image%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-full-screen-image%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/145" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-gallery-full-screen-image/" />

### `/index.php/pat-twentytwentyfour-gallery-full-screen-image/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-gallery-offset-images-grid-2-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75069 statt 77389 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-offset-images-grid-2-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-offset-images-grid-2-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/146" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-gallery-offset-images-grid-2-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-offset-images-grid-2-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-offset-images-grid-2-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/146" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-gallery-offset-images-grid-2-col/" />

### `/index.php/pat-twentytwentyfour-gallery-offset-images-grid-2-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-gallery-offset-images-grid-3-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75069 statt 77389 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-offset-images-grid-3-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-offset-images-grid-3-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/147" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-gallery-offset-images-grid-3-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-offset-images-grid-3-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-offset-images-grid-3-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/147" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-gallery-offset-images-grid-3-col/" />

### `/index.php/pat-twentytwentyfour-gallery-offset-images-grid-3-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-gallery-offset-images-grid-4-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75069 statt 77389 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-offset-images-grid-4-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-offset-images-grid-4-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/148" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-gallery-offset-images-grid-4-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-offset-images-grid-4-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-offset-images-grid-4-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/148" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-gallery-offset-images-grid-4-col/" />

### `/index.php/pat-twentytwentyfour-gallery-offset-images-grid-4-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-gallery-project-layout/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-project-layout%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-gallery-project-layout%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/149" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-gallery-project-layout/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-project-layout%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-gallery-project-layout%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/149" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-gallery-project-layout/" />

### `/index.php/pat-twentytwentyfour-gallery-project-layout/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-404/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75047 statt 77367 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-404%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-404%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/150" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-404/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-404%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-404%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/150" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-404/" />

### `/index.php/pat-twentytwentyfour-hidden-404/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-comments/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75052 statt 77372 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-comments%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-comments%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/151" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-comments/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-comments%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-comments%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/151" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-comments/" />

### `/index.php/pat-twentytwentyfour-hidden-comments/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-no-results/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75054 statt 77374 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-no-results%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-no-results%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/152" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-no-results/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-no-results%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-no-results%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/152" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-no-results/" />

### `/index.php/pat-twentytwentyfour-hidden-no-results/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-portfolio-hero/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75058 statt 77378 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-portfolio-hero%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-portfolio-hero%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/153" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-portfolio-hero/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-portfolio-hero%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-portfolio-hero%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/153" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-portfolio-hero/" />

### `/index.php/pat-twentytwentyfour-hidden-portfolio-hero/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-post-meta/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75053 statt 77373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-post-meta%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-post-meta%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/154" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-post-meta/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-post-meta%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-post-meta%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/154" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-post-meta/" />

### `/index.php/pat-twentytwentyfour-hidden-post-meta/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-post-navigation/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-post-navigation%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-post-navigation%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/155" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-post-navigation/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-post-navigation%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-post-navigation%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/155" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-post-navigation/" />

### `/index.php/pat-twentytwentyfour-hidden-post-navigation/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-posts-heading/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75057 statt 77377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-posts-heading%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-posts-heading%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/156" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-posts-heading/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-posts-heading%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-posts-heading%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/156" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-posts-heading/" />

### `/index.php/pat-twentytwentyfour-hidden-posts-heading/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-search/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75050 statt 77370 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-search%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-search%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/157" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-search/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-search%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-search%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/157" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-search/" />

### `/index.php/pat-twentytwentyfour-hidden-search/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-hidden-sidebar/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75051 statt 77371 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-sidebar%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-hidden-sidebar%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/158" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-hidden-sidebar/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-sidebar%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-hidden-sidebar%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/158" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-hidden-sidebar/" />

### `/index.php/pat-twentytwentyfour-hidden-sidebar/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-page-about-business/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75056 statt 77376 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-about-business%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-about-business%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/159" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-page-about-business/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-about-business%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-about-business%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/159" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-page-about-business/" />

### `/index.php/pat-twentytwentyfour-page-about-business/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-page-home-blogging/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75055 statt 77375 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-home-blogging%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-home-blogging%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/160" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-page-home-blogging/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-home-blogging%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-home-blogging%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/160" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-page-home-blogging/" />

### `/index.php/pat-twentytwentyfour-page-home-blogging/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-page-home-business/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75055 statt 77375 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-home-business%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-home-business%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/161" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-page-home-business/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-home-business%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-home-business%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/161" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-page-home-business/" />

### `/index.php/pat-twentytwentyfour-page-home-business/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-page-home-gallery/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75054 statt 77374 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-home-gallery%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-home-gallery%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/162" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-page-home-gallery/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-home-gallery%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-home-gallery%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/162" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-page-home-gallery/" />

### `/index.php/pat-twentytwentyfour-page-home-gallery/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-page-home-portfolio/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75056 statt 77376 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-home-portfolio%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-home-portfolio%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/163" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-page-home-portfolio/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-home-portfolio%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-home-portfolio%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/163" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-page-home-portfolio/" />

### `/index.php/pat-twentytwentyfour-page-home-portfolio/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-page-newsletter-landing/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-newsletter-landing%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-newsletter-landing%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/164" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-page-newsletter-landing/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-newsletter-landing%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-newsletter-landing%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/164" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-page-newsletter-landing/" />

### `/index.php/pat-twentytwentyfour-page-newsletter-landing/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-page-portfolio-overview/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-portfolio-overview%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-portfolio-overview%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/165" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-page-portfolio-overview/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-portfolio-overview%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-portfolio-overview%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/165" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-page-portfolio-overview/" />

### `/index.php/pat-twentytwentyfour-page-portfolio-overview/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-page-rsvp-landing/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75054 statt 77374 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-rsvp-landing%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-page-rsvp-landing%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/166" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-page-rsvp-landing/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-rsvp-landing%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-page-rsvp-landing%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/166" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-page-rsvp-landing/" />

### `/index.php/pat-twentytwentyfour-page-rsvp-landing/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-posts-1-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75048 statt 77368 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-1-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-1-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/167" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-posts-1-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-1-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-1-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/167" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-posts-1-col/" />

### `/index.php/pat-twentytwentyfour-posts-1-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-posts-3-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75048 statt 77368 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-3-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-3-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/168" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-posts-3-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-3-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-3-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/168" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-posts-3-col/" />

### `/index.php/pat-twentytwentyfour-posts-3-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-posts-grid-2-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75053 statt 77373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-grid-2-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-grid-2-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/169" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-posts-grid-2-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-grid-2-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-grid-2-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/169" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-posts-grid-2-col/" />

### `/index.php/pat-twentytwentyfour-posts-grid-2-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-posts-images-only-3-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-images-only-3-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-images-only-3-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/170" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-posts-images-only-3-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-images-only-3-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-images-only-3-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/170" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-posts-images-only-3-col/" />

### `/index.php/pat-twentytwentyfour-posts-images-only-3-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-posts-images-only-offset-4-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75067 statt 77387 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-images-only-offset-4-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-images-only-offset-4-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/171" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-posts-images-only-offset-4-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-images-only-offset-4-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-images-only-offset-4-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/171" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-posts-images-only-offset-4-col/" />

### `/index.php/pat-twentytwentyfour-posts-images-only-offset-4-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-posts-list/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75047 statt 77367 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-list%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-posts-list%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/172" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-posts-list/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-list%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-posts-list%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/172" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-posts-list/" />

### `/index.php/pat-twentytwentyfour-posts-list/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-team-4-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75047 statt 77367 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-team-4-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-team-4-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/173" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-team-4-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-team-4-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-team-4-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/173" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-team-4-col/" />

### `/index.php/pat-twentytwentyfour-team-4-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-archive-blogging/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75062 statt 77382 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-archive-blogging%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-archive-blogging%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/174" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-archive-blogging/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-archive-blogging%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-archive-blogging%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/174" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-archive-blogging/" />

### `/index.php/pat-twentytwentyfour-template-archive-blogging/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-archive-portfolio/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75063 statt 77383 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-archive-portfolio%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-archive-portfolio%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/175" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-archive-portfolio/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-archive-portfolio%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-archive-portfolio%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/175" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-archive-portfolio/" />

### `/index.php/pat-twentytwentyfour-template-archive-portfolio/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-home-blogging/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-home-blogging%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-home-blogging%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/176" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-home-blogging/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-home-blogging%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-home-blogging%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/176" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-home-blogging/" />

### `/index.php/pat-twentytwentyfour-template-home-blogging/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-home-business/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-home-business%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-home-business%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/177" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-home-business/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-home-business%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-home-business%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/177" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-home-business/" />

### `/index.php/pat-twentytwentyfour-template-home-business/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-home-portfolio/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-home-portfolio%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-home-portfolio%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/178" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-home-portfolio/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-home-portfolio%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-home-portfolio%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/178" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-home-portfolio/" />

### `/index.php/pat-twentytwentyfour-template-home-portfolio/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-index-blogging/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-index-blogging%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-index-blogging%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/179" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-index-blogging/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-index-blogging%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-index-blogging%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/179" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-index-blogging/" />

### `/index.php/pat-twentytwentyfour-template-index-blogging/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-index-portfolio/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75061 statt 77381 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-index-portfolio%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-index-portfolio%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/180" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-index-portfolio/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-index-portfolio%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-index-portfolio%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/180" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-index-portfolio/" />

### `/index.php/pat-twentytwentyfour-template-index-portfolio/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-search-blogging/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75061 statt 77381 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-search-blogging%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-search-blogging%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/181" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-search-blogging/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-search-blogging%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-search-blogging%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/181" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-search-blogging/" />

### `/index.php/pat-twentytwentyfour-template-search-blogging/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-search-portfolio/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75062 statt 77382 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-search-portfolio%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-search-portfolio%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/182" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-search-portfolio/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-search-portfolio%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-search-portfolio%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/182" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-search-portfolio/" />

### `/index.php/pat-twentytwentyfour-template-search-portfolio/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-template-single-portfolio/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75062 statt 77382 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-single-portfolio%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-template-single-portfolio%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/183" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-template-single-portfolio/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-single-portfolio%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-template-single-portfolio%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/183" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-template-single-portfolio/" />

### `/index.php/pat-twentytwentyfour-template-single-portfolio/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-testimonial-centered/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75057 statt 77377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-testimonial-centered%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-testimonial-centered%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/184" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-testimonial-centered/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-testimonial-centered%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-testimonial-centered%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/184" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-testimonial-centered/" />

### `/index.php/pat-twentytwentyfour-testimonial-centered/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-text-alternating-images/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-alternating-images%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-alternating-images%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/185" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-text-alternating-images/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-alternating-images%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-alternating-images%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/185" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-text-alternating-images/" />

### `/index.php/pat-twentytwentyfour-text-alternating-images/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-text-centered-statement/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-centered-statement%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-centered-statement%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/186" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-text-centered-statement/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-centered-statement%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-centered-statement%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/186" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-text-centered-statement/" />

### `/index.php/pat-twentytwentyfour-text-centered-statement/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-text-centered-statement-small/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75066 statt 77386 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-centered-statement-small%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-centered-statement-small%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/187" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-text-centered-statement-small/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-centered-statement-small%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-centered-statement-small%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/187" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-text-centered-statement-small/" />

### `/index.php/pat-twentytwentyfour-text-centered-statement-small/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-text-faq/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75045 statt 77365 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-faq%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-faq%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/188" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-text-faq/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-faq%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-faq%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/188" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-text-faq/" />

### `/index.php/pat-twentytwentyfour-text-faq/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-text-feature-grid-3-col/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-feature-grid-3-col%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-feature-grid-3-col%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/189" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-text-feature-grid-3-col/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-feature-grid-3-col%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-feature-grid-3-col%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/189" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-text-feature-grid-3-col/" />

### `/index.php/pat-twentytwentyfour-text-feature-grid-3-col/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-text-project-details/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75057 statt 77377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-project-details%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-project-details%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/190" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-text-project-details/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-project-details%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-project-details%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/190" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-text-project-details/" />

### `/index.php/pat-twentytwentyfour-text-project-details/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentyfour-text-title-left-image-right/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75064 statt 77384 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-title-left-image-right%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentyfour-text-title-left-image-right%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/191" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentyfour-text-title-left-image-right/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-title-left-image-right%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentyfour-text-title-left-image-right%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/191" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentyfour-text-title-left-image-right/" />

### `/index.php/pat-twentytwentyfour-text-title-left-image-right/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentythree-cta/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75041 statt 77361 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-cta%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-cta%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/192" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentythree-cta/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-cta%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-cta%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/192" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentythree-cta/" />

### `/index.php/pat-twentytwentythree-cta/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentythree-footer-default/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75052 statt 77372 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-footer-default%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-footer-default%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/193" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentythree-footer-default/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-footer-default%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-footer-default%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/193" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentythree-footer-default/" />

### `/index.php/pat-twentytwentythree-footer-default/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentythree-hidden-404/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75048 statt 77368 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-hidden-404%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-hidden-404%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/194" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentythree-hidden-404/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-hidden-404%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-hidden-404%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/194" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentythree-hidden-404/" />

### `/index.php/pat-twentytwentythree-hidden-404/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentythree-hidden-comments/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75053 statt 77373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-hidden-comments%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-hidden-comments%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/195" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentythree-hidden-comments/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-hidden-comments%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-hidden-comments%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/195" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentythree-hidden-comments/" />

### `/index.php/pat-twentytwentythree-hidden-comments/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentythree-hidden-heading/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75052 statt 77372 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-hidden-heading%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-hidden-heading%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/196" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentythree-hidden-heading/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-hidden-heading%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-hidden-heading%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/196" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentythree-hidden-heading/" />

### `/index.php/pat-twentytwentythree-hidden-heading/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentythree-hidden-no-results-content/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75063 statt 77383 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-hidden-no-results-content%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-hidden-no-results-content%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/197" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentythree-hidden-no-results-content/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-hidden-no-results-content%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-hidden-no-results-content%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/197" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentythree-hidden-no-results-content/" />

### `/index.php/pat-twentytwentythree-hidden-no-results-content/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentythree-post-meta/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75047 statt 77367 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-post-meta%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentythree-post-meta%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/198" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentythree-post-meta/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-post-meta%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentythree-post-meta%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/198" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentythree-post-meta/" />

### `/index.php/pat-twentytwentythree-post-meta/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-default/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75050 statt 77370 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-default%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-default%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/199" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-default/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-default%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-default%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/199" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-default/" />

### `/index.php/pat-twentytwentytwo-footer-default/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-dark/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75047 statt 77367 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-dark%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-dark%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/200" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-dark/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-dark%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-dark%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/200" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-dark/" />

### `/index.php/pat-twentytwentytwo-footer-dark/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-logo/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75047 statt 77367 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-logo%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-logo%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/201" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-logo/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-logo%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-logo%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/201" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-logo/" />

### `/index.php/pat-twentytwentytwo-footer-logo/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-navigation/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75053 statt 77373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-navigation%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-navigation%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/202" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-navigation/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-navigation%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-navigation%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/202" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-navigation/" />

### `/index.php/pat-twentytwentytwo-footer-navigation/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-title-tagline-social/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75063 statt 77383 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-title-tagline-social%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-title-tagline-social%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/203" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-title-tagline-social/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-title-tagline-social%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-title-tagline-social%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/203" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-title-tagline-social/" />

### `/index.php/pat-twentytwentytwo-footer-title-tagline-social/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-social-copyright/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-social-copyright%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-social-copyright%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/204" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-social-copyright/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-social-copyright%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-social-copyright%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/204" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-social-copyright/" />

### `/index.php/pat-twentytwentytwo-footer-social-copyright/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-navigation-copyright/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75063 statt 77383 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-navigation-copyright%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-navigation-copyright%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/205" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-navigation-copyright/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-navigation-copyright%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-navigation-copyright%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/205" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-navigation-copyright/" />

### `/index.php/pat-twentytwentytwo-footer-navigation-copyright/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-about-title-logo/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-about-title-logo%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-about-title-logo%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/206" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-about-title-logo/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-about-title-logo%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-about-title-logo%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/206" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-about-title-logo/" />

### `/index.php/pat-twentytwentytwo-footer-about-title-logo/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-query-title-citation/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75063 statt 77383 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-query-title-citation%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-query-title-citation%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/207" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-query-title-citation/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-query-title-citation%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-query-title-citation%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/207" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-query-title-citation/" />

### `/index.php/pat-twentytwentytwo-footer-query-title-citation/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-query-images-title-citation/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75070 statt 77390 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-query-images-title-citation%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-query-images-title-citation%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/208" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-query-images-title-citation/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-query-images-title-citation%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-query-images-title-citation%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/208" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-query-images-title-citation/" />

### `/index.php/pat-twentytwentytwo-footer-query-images-title-citation/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-footer-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75047 statt 77367 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-footer-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/209" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-footer-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-footer-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/209" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-footer-blog/" />

### `/index.php/pat-twentytwentytwo-footer-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-subscribe/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75053 statt 77373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-subscribe%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-subscribe%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/210" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-subscribe/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-subscribe%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-subscribe%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/210" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-subscribe/" />

### `/index.php/pat-twentytwentytwo-general-subscribe/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-featured-posts/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75058 statt 77378 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-featured-posts%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-featured-posts%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/211" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-featured-posts/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-featured-posts%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-featured-posts%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/211" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-featured-posts/" />

### `/index.php/pat-twentytwentytwo-general-featured-posts/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-layered-images-with-duotone/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75071 statt 77391 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-layered-images-with-duotone%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-layered-images-with-duotone%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/212" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-layered-images-with-duotone/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-layered-images-with-duotone%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-layered-images-with-duotone%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/212" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-layered-images-with-duotone/" />

### `/index.php/pat-twentytwentytwo-general-layered-images-with-duotone/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-wide-image-intro-buttons/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75068 statt 77388 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-wide-image-intro-buttons%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-wide-image-intro-buttons%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/213" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-wide-image-intro-buttons/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-wide-image-intro-buttons%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-wide-image-intro-buttons%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/213" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-wide-image-intro-buttons/" />

### `/index.php/pat-twentytwentytwo-general-wide-image-intro-buttons/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-large-list-names/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75060 statt 77380 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-large-list-names%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-large-list-names%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/214" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-large-list-names/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-large-list-names%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-large-list-names%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/214" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-large-list-names/" />

### `/index.php/pat-twentytwentytwo-general-large-list-names/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-video-header-details/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75064 statt 77384 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-video-header-details%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-video-header-details%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/215" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-video-header-details/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-video-header-details%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-video-header-details%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/215" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-video-header-details/" />

### `/index.php/pat-twentytwentytwo-general-video-header-details/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-list-events/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75055 statt 77375 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-list-events%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-list-events%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/216" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-list-events/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-list-events%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-list-events%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/216" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-list-events/" />

### `/index.php/pat-twentytwentytwo-general-list-events/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-two-images-text/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-two-images-text%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-two-images-text%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/217" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-two-images-text/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-two-images-text%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-two-images-text%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/217" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-two-images-text/" />

### `/index.php/pat-twentytwentytwo-general-two-images-text/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-image-with-caption/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75062 statt 77382 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-image-with-caption%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-image-with-caption%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/218" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-image-with-caption/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-image-with-caption%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-image-with-caption%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/218" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-image-with-caption/" />

### `/index.php/pat-twentytwentytwo-general-image-with-caption/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-video-trailer/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75057 statt 77377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-video-trailer%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-video-trailer%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/219" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-video-trailer/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-video-trailer%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-video-trailer%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/219" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-video-trailer/" />

### `/index.php/pat-twentytwentytwo-general-video-trailer/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-pricing-table/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75057 statt 77377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-pricing-table%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-pricing-table%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/220" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-pricing-table/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-pricing-table%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-pricing-table%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/220" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-pricing-table/" />

### `/index.php/pat-twentytwentytwo-general-pricing-table/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-divider-light/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75057 statt 77377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-divider-light%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-divider-light%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/221" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-divider-light/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-divider-light%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-divider-light%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/221" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-divider-light/" />

### `/index.php/pat-twentytwentytwo-general-divider-light/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-general-divider-dark/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75056 statt 77376 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-divider-dark%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-general-divider-dark%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/222" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-general-divider-dark/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-divider-dark%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-general-divider-dark%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/222" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-general-divider-dark/" />

### `/index.php/pat-twentytwentytwo-general-divider-dark/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-default/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75050 statt 77370 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-default%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-default%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/223" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-default/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-default%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-default%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/223" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-default/" />

### `/index.php/pat-twentytwentytwo-header-default/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-large-dark/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75053 statt 77373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-large-dark%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-large-dark%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/224" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-large-dark/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-large-dark%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-large-dark%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/224" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-large-dark/" />

### `/index.php/pat-twentytwentytwo-header-large-dark/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-small-dark/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75053 statt 77373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-small-dark%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-small-dark%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/225" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-small-dark/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-small-dark%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-small-dark%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/225" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-small-dark/" />

### `/index.php/pat-twentytwentytwo-header-small-dark/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-image-background/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-image-background%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-image-background%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/226" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-image-background/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-image-background%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-image-background%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/226" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-image-background/" />

### `/index.php/pat-twentytwentytwo-header-image-background/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-image-background-overlay/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75067 statt 77387 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-image-background-overlay%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-image-background-overlay%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/227" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-image-background-overlay/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-image-background-overlay%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-image-background-overlay%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/227" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-image-background-overlay/" />

### `/index.php/pat-twentytwentytwo-header-image-background-overlay/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-with-tagline/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75055 statt 77375 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-with-tagline%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-with-tagline%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/228" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-with-tagline/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-with-tagline%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-with-tagline%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/228" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-with-tagline/" />

### `/index.php/pat-twentytwentytwo-header-with-tagline/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-text-only-green-background/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75069 statt 77389 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-text-only-green-background%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-text-only-green-background%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/229" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-text-only-green-background/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-text-only-green-background%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-text-only-green-background%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/229" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-text-only-green-background/" />

### `/index.php/pat-twentytwentytwo-header-text-only-green-background/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-text-only-salmon-background/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75070 statt 77390 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-text-only-salmon-background%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-text-only-salmon-background%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/230" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-text-only-salmon-background/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-text-only-salmon-background%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-text-only-salmon-background%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/230" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-text-only-salmon-background/" />

### `/index.php/pat-twentytwentytwo-header-text-only-salmon-background/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-title-and-button/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-title-and-button%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-title-and-button%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/231" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-title-and-button/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-title-and-button%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-title-and-button%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/231" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-title-and-button/" />

### `/index.php/pat-twentytwentytwo-header-title-and-button/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-text-only-with-tagline-black-background/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75082 statt 77402 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-text-only-with-tagline-black-background%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-text-only-with-tagline-black-background%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/232" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-text-only-with-tagline-black-background/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-text-only-with-tagline-black-background%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-text-only-with-tagline-black-background%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/232" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-text-only-with-tagline-black-background/" />

### `/index.php/pat-twentytwentytwo-header-text-only-with-tagline-black-background/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-logo-navigation-gray-background/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75074 statt 77394 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-logo-navigation-gray-background%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-logo-navigation-gray-background%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/233" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-logo-navigation-gray-background/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-logo-navigation-gray-background%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-logo-navigation-gray-background%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/233" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-logo-navigation-gray-background/" />

### `/index.php/pat-twentytwentytwo-header-logo-navigation-gray-background/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-logo-navigation-social-black-background/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75082 statt 77402 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-logo-navigation-social-black-background%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-logo-navigation-social-black-background%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/234" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-logo-navigation-social-black-background/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-logo-navigation-social-black-background%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-logo-navigation-social-black-background%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/234" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-logo-navigation-social-black-background/" />

### `/index.php/pat-twentytwentytwo-header-logo-navigation-social-black-background/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-title-navigation-social/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75066 statt 77386 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-title-navigation-social%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-title-navigation-social%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/235" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-title-navigation-social/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-title-navigation-social%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-title-navigation-social%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/235" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-title-navigation-social/" />

### `/index.php/pat-twentytwentytwo-header-title-navigation-social/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-logo-navigation-offset-tagline/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75073 statt 77393 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-logo-navigation-offset-tagline%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-logo-navigation-offset-tagline%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/236" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-logo-navigation-offset-tagline/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-logo-navigation-offset-tagline%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-logo-navigation-offset-tagline%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/236" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-logo-navigation-offset-tagline/" />

### `/index.php/pat-twentytwentytwo-header-logo-navigation-offset-tagline/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-stacked/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75050 statt 77370 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-stacked%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-stacked%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/237" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-stacked/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-stacked%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-stacked%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/237" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-stacked/" />

### `/index.php/pat-twentytwentytwo-header-stacked/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-centered-logo/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75056 statt 77376 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-centered-logo%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-centered-logo%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/238" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-centered-logo/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-centered-logo%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-centered-logo%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/238" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-centered-logo/" />

### `/index.php/pat-twentytwentytwo-header-centered-logo/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-centered-logo-black-background/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75073 statt 77393 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-centered-logo-black-background%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-centered-logo-black-background%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/239" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-centered-logo-black-background/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-centered-logo-black-background%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-centered-logo-black-background%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/239" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-centered-logo-black-background/" />

### `/index.php/pat-twentytwentytwo-header-centered-logo-black-background/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-header-centered-title-navigation-social/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75075 statt 77395 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-centered-title-navigation-social%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-header-centered-title-navigation-social%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/240" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-header-centered-title-navigation-social/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-centered-title-navigation-social%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-header-centered-title-navigation-social%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/240" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-header-centered-title-navigation-social/" />

### `/index.php/pat-twentytwentytwo-header-centered-title-navigation-social/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-hidden-404/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75046 statt 77366 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-hidden-404%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-hidden-404%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/241" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-hidden-404/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-hidden-404%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-hidden-404%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/241" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-hidden-404/" />

### `/index.php/pat-twentytwentytwo-hidden-404/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-hidden-bird/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75047 statt 77367 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-hidden-bird%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-hidden-bird%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/242" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-hidden-bird/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-hidden-bird%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-hidden-bird%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/242" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-hidden-bird/" />

### `/index.php/pat-twentytwentytwo-hidden-bird/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-hidden-heading-and-bird/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-hidden-heading-and-bird%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-hidden-heading-and-bird%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/243" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-hidden-heading-and-bird/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-hidden-heading-and-bird%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-hidden-heading-and-bird%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/243" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-hidden-heading-and-bird/" />

### `/index.php/pat-twentytwentytwo-hidden-heading-and-bird/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-about-media-left/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75057 statt 77377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-media-left%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-media-left%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/244" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-about-media-left/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-media-left%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-media-left%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/244" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-about-media-left/" />

### `/index.php/pat-twentytwentytwo-page-about-media-left/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-about-simple-dark/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75058 statt 77378 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-simple-dark%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-simple-dark%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/245" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-about-simple-dark/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-simple-dark%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-simple-dark%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/245" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-about-simple-dark/" />

### `/index.php/pat-twentytwentytwo-page-about-simple-dark/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-about-media-right/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75058 statt 77378 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-media-right%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-media-right%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/246" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-about-media-right/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-media-right%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-media-right%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/246" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-about-media-right/" />

### `/index.php/pat-twentytwentytwo-page-about-media-right/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-about-solid-color/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75058 statt 77378 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-solid-color%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-solid-color%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/247" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-about-solid-color/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-solid-color%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-solid-color%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/247" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-about-solid-color/" />

### `/index.php/pat-twentytwentytwo-page-about-solid-color/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-about-links/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75052 statt 77372 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-links%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-links%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/248" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-about-links/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-links%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-links%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/248" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-about-links/" />

### `/index.php/pat-twentytwentytwo-page-about-links/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-about-links-dark/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75057 statt 77377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-links-dark%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-links-dark%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/249" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-about-links-dark/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-links-dark%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-links-dark%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/249" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-about-links-dark/" />

### `/index.php/pat-twentytwentytwo-page-about-links-dark/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-about-large-image-and-buttons/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75070 statt 77390 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-large-image-and-buttons%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-about-large-image-and-buttons%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/250" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-about-large-image-and-buttons/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-large-image-and-buttons%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-about-large-image-and-buttons%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/250" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-about-large-image-and-buttons/" />

### `/index.php/pat-twentytwentytwo-page-about-large-image-and-buttons/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-layout-image-and-text/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75062 statt 77382 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-layout-image-and-text%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-layout-image-and-text%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/251" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-layout-image-and-text/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-layout-image-and-text%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-layout-image-and-text%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/251" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-layout-image-and-text/" />

### `/index.php/pat-twentytwentytwo-page-layout-image-and-text/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-layout-image-text-and-video/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75068 statt 77388 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-layout-image-text-and-video%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-layout-image-text-and-video%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/252" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-layout-image-text-and-video/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-layout-image-text-and-video%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-layout-image-text-and-video%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/252" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-layout-image-text-and-video/" />

### `/index.php/pat-twentytwentytwo-page-layout-image-text-and-video/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-layout-two-columns/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-layout-two-columns%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-layout-two-columns%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/253" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-layout-two-columns/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-layout-two-columns%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-layout-two-columns%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/253" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-layout-two-columns/" />

### `/index.php/pat-twentytwentytwo-page-layout-two-columns/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-sidebar-poster/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75055 statt 77375 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-sidebar-poster%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-sidebar-poster%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/254" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-sidebar-poster/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-sidebar-poster%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-sidebar-poster%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/254" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-sidebar-poster/" />

### `/index.php/pat-twentytwentytwo-page-sidebar-poster/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-sidebar-grid-posts/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-sidebar-grid-posts%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-sidebar-grid-posts%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/255" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-sidebar-grid-posts/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-sidebar-grid-posts%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-sidebar-grid-posts%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/255" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-sidebar-grid-posts/" />

### `/index.php/pat-twentytwentytwo-page-sidebar-grid-posts/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-sidebar-blog-posts/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75059 statt 77379 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-sidebar-blog-posts%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-sidebar-blog-posts%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/256" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-sidebar-blog-posts/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-sidebar-blog-posts%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-sidebar-blog-posts%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/256" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-sidebar-blog-posts/" />

### `/index.php/pat-twentytwentytwo-page-sidebar-blog-posts/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-page-sidebar-blog-posts-right/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75065 statt 77385 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-sidebar-blog-posts-right%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-page-sidebar-blog-posts-right%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/257" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-page-sidebar-blog-posts-right/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-sidebar-blog-posts-right%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-page-sidebar-blog-posts-right%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/257" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-page-sidebar-blog-posts-right/" />

### `/index.php/pat-twentytwentytwo-page-sidebar-blog-posts-right/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-query-default/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75049 statt 77369 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-default%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-default%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/258" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-query-default/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-default%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-default%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/258" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-query-default/" />

### `/index.php/pat-twentytwentytwo-query-default/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-query-simple-blog/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75053 statt 77373 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-simple-blog%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-simple-blog%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/259" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-query-simple-blog/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-simple-blog%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-simple-blog%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/259" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-query-simple-blog/" />

### `/index.php/pat-twentytwentytwo-query-simple-blog/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-query-grid/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75046 statt 77366 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-grid%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-grid%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/260" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-query-grid/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-grid%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-grid%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/260" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-query-grid/" />

### `/index.php/pat-twentytwentytwo-query-grid/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-query-text-grid/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75051 statt 77371 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-text-grid%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-text-grid%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/261" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-query-text-grid/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-text-grid%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-text-grid%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/261" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-query-text-grid/" />

### `/index.php/pat-twentytwentytwo-query-text-grid/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-query-image-grid/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75052 statt 77372 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-image-grid%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-image-grid%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/262" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-query-image-grid/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-image-grid%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-image-grid%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/262" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-query-image-grid/" />

### `/index.php/pat-twentytwentytwo-query-image-grid/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-query-large-titles/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75054 statt 77374 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-large-titles%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-large-titles%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/263" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-query-large-titles/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-large-titles%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-large-titles%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/263" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-query-large-titles/" />

### `/index.php/pat-twentytwentytwo-query-large-titles/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/pat-twentytwentytwo-query-irregular-grid/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
75056 statt 77376 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-irregular-grid%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fpat-twentytwentytwo-query-irregular-grid%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/pages/264" />
- **head**: fehlt: <link rel="canonical" href="http://localhost:8080/index.php/pat-twentytwentytwo-query-irregular-grid/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-irregular-grid%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Fpat-twentytwentytwo-query-irregular-grid%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/pages/264" />
- **head**: zu viel: <link rel="canonical" href="http://localhost:8080/pat-twentytwentytwo-query-irregular-grid/" />

### `/index.php/pat-twentytwentytwo-query-irregular-grid/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/buecher/buch-1/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
83600 statt 85920 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-1%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-1%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/book/274" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-1%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-1%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/book/274" />

### `/index.php/buecher/buch-1/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/buecher/buch-2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
83783 statt 86103 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-2%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-2%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/book/275" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-2%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-2%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/book/275" />

### `/index.php/buecher/buch-2/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/buecher/buch-3/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
83604 statt 85924 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-3%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-3%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/book/276" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-3%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Fbuecher%2Fbuch-3%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/book/276" />

### `/index.php/buecher/buch-3/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/termine/sommerfest/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
81188 statt 83508 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftermine%2Fsommerfest%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftermine%2Fsommerfest%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/event/277" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftermine%2Fsommerfest%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftermine%2Fsommerfest%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/event/277" />

### `/index.php/termine/sommerfest/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/termine/sommerfest/aufbau/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
81187 statt 83507 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftermine%2Fsommerfest%2Faufbau%2F" />
- **head**: fehlt: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/index.php/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftermine%2Fsommerfest%2Faufbau%2F&#038;format=xml" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/event/278" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (JSON)" type="application/json+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftermine%2Fsommerfest%2Faufbau%2F" />
- **head**: zu viel: <link rel="alternate" title="oEmbed (XML)" type="text/xml+oembed" href="http://localhost:8080/wp-json/oembed/1.0/embed?url=http%3A%2F%2Flocalhost%3A8080%2Findex.php%2Ftermine%2Fsommerfest%2Faufbau%2F&#038;format=xml" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/event/278" />

### `/index.php/termine/sommerfest/aufbau/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/category/uncategorized/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76041 statt 78361 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/categories/1" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/categories/1" />

### `/index.php/category/uncategorized/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76057 statt 78377 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/categories/1" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/categories/1" />

### `/index.php/category/news/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76229 statt 78549 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/categories/2" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/categories/2" />

### `/index.php/category/news/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78712 statt 81032 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/categories/2" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/categories/2" />

### `/index.php/category/tech/rust/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76191 statt 78511 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/categories/4" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/categories/4" />

### `/index.php/category/tech/rust/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78674 statt 80994 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/categories/4" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/categories/4" />

### `/index.php/tag/alpha/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85608 statt 87958 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/tags/5" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/tags/5" />

### `/index.php/tag/alpha/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
88270 statt 90630 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/tags/5" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/tags/5" />

### `/index.php/tag/beta/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
85604 statt 87954 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/tags/6" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/tags/6" />

### `/index.php/tag/beta/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
88265 statt 90625 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/tags/6" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/tags/6" />

### `/index.php/genre/krimi/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
78017 statt 80337 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/genre/10" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/genre/10" />

### `/index.php/genre/krimi/feed/` (Feeds)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`

### `/index.php/author/admin/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
86827 statt 89187 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/users/1" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/users/1" />

### `/index.php/author/admin/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
95360 statt 97740 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/users/1" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/users/1" />

### `/index.php/author/redakteur/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76429 statt 78749 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: fehlt: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/index.php/wp-json/wp/v2/users/2" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />
- **head**: zu viel: <link rel="alternate" title="JSON" type="application/json" href="http://localhost:8080/wp-json/wp/v2/users/2" />

### `/index.php/author/redakteur/page/2/` (HTML-Seiten)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **body**

```text
76417 statt 78737 Bytes, erster Unterschied bei 3172:
RustPress: …block-navigation-item__content" href="http://localhost:8080/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-item open-on-…
WordPress: …block-navigation-item__content" href="http://localhost:8080/index.php/bildliste/">Bildliste</a></li><li class="wp-block-pages-list__item wp-block-navigation-ite…
```

- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/index.php/feed/" />
- **head**: fehlt: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/index.php/comments/feed/" />
- **head**: fehlt: <link rel="https://api.w.org/" href="http://localhost:8080/index.php/wp-json/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Feed" href="http://localhost:8080/feed/" />
- **head**: zu viel: <link rel="alternate" type="application/rss+xml" title="RustPress Test &raquo; Comments Feed" href="http://localhost:8080/comments/feed/" />
- **head**: zu viel: <link rel="https://api.w.org/" href="http://localhost:8080/wp-json/" />

### `/wp-json/wp/v2/posts/272` (REST-API)

- **bytes**

```text
1851 statt 1961 Bytes, erster Unterschied bei 830:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/272","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/272","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/posts/271` (REST-API)

- **bytes**

```text
3199 statt 3298 Bytes, erster Unterschied bei 2291:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/271","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/271","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/posts/266` (REST-API)

- **bytes**

```text
1793 statt 1892 Bytes, erster Unterschied bei 885:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/266","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/266","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/posts/265` (REST-API)

- **bytes**

```text
2240 statt 2339 Bytes, erster Unterschied bei 1332:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/265","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/265","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/posts/36` (REST-API)

- **bytes**

```text
4168 statt 4267 Bytes, erster Unterschied bei 3266:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/36","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/36","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/35` (REST-API)

- **bytes**

```text
4637 statt 4736 Bytes, erster Unterschied bei 3735:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/35","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/35","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/34` (REST-API)

- **bytes**

```text
11340 statt 11439 Bytes, erster Unterschied bei 10438:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/34","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/34","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/31` (REST-API)

- **bytes**

```text
3600 statt 3699 Bytes, erster Unterschied bei 2698:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/31","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/31","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/30` (REST-API)

- **bytes**

```text
5114 statt 5213 Bytes, erster Unterschied bei 4212:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/30","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/30","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/29` (REST-API)

- **bytes**

```text
2613 statt 2712 Bytes, erster Unterschied bei 1711:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/29","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/29","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/28` (REST-API)

- **bytes**

```text
2244 statt 2343 Bytes, erster Unterschied bei 1342:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/28","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/28","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/27` (REST-API)

- **bytes**

```text
4359 statt 4458 Bytes, erster Unterschied bei 3457:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/27","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/27","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/22` (REST-API)

- **bytes**

```text
1802 statt 1901 Bytes, erster Unterschied bei 900:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/22","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/22","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/19` (REST-API)

- **bytes**

```text
1711 statt 1810 Bytes, erster Unterschied bei 809:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/19","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/19","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/17` (REST-API)

- **bytes**

```text
1899 statt 1998 Bytes, erster Unterschied bei 997:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/17","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/17","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/16` (REST-API)

- **bytes**

```text
2939 statt 3038 Bytes, erster Unterschied bei 2037:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/16","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/16","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/15` (REST-API)

- **bytes**

```text
2939 statt 3038 Bytes, erster Unterschied bei 2037:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/15","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/15","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/14` (REST-API)

- **bytes**

```text
2939 statt 3038 Bytes, erster Unterschied bei 2037:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/14","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/14","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/13` (REST-API)

- **bytes**

```text
2931 statt 3030 Bytes, erster Unterschied bei 2029:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/13","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/13","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/12` (REST-API)

- **bytes**

```text
2931 statt 3030 Bytes, erster Unterschied bei 2029:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/12","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/12","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/11` (REST-API)

- **bytes**

```text
2931 statt 3030 Bytes, erster Unterschied bei 2029:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/11","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/11","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/10` (REST-API)

- **bytes**

```text
3055 statt 3165 Bytes, erster Unterschied bei 2051:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/9` (REST-API)

- **bytes**

```text
3047 statt 3157 Bytes, erster Unterschied bei 2049:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/9","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/9","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/posts/8` (REST-API)

- **bytes**

```text
3047 statt 3157 Bytes, erster Unterschied bei 2049:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/8","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/8","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/posts/7` (REST-API)

- **bytes**

```text
3046 statt 3156 Bytes, erster Unterschied bei 2048:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/7","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/7","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/posts/6` (REST-API)

- **bytes**

```text
3047 statt 3157 Bytes, erster Unterschied bei 2049:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/6","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/6","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/posts/5` (REST-API)

- **bytes**

```text
3047 statt 3157 Bytes, erster Unterschied bei 2049:
RustPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/5","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …-beta"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/5","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/posts/1` (REST-API)

- **bytes**

```text
1885 statt 1984 Bytes, erster Unterschied bei 989:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/posts/24` (REST-API)

- **bytes**

```text
1690 statt 1789 Bytes, erster Unterschied bei 788:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/24","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/24","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/posts/23` (REST-API)

- **bytes**

```text
1690 statt 1789 Bytes, erster Unterschied bei 788:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/23","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/23","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/pages/32` (REST-API)

- **header:link**: RustPress `</bildliste/>; rel="alternate"; type=text/html` statt `</index.php/bildliste/>; rel="alternate"; type=text/html`
- **bytes**

```text
3505 statt 3593 Bytes, erster Unterschied bei 285:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/bildliste\/","title":{"rendered":"Bildliste"},"content":{"rendered":"\n<div class=\"wp-block-query i…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/bildliste\/","title":{"rendered":"Bildliste"},"content":{"rendered":"\n<div class=\"wp-bl…
```


### `/wp-json/wp/v2/pages/21` (REST-API)

- **header:link**: RustPress `</ueber-uns/team/>; rel="alternate"; type=text/html` statt `</index.php/ueber-uns/team/>; rel="alternate"; type=text/html`
- **bytes**

```text
1512 statt 1611 Bytes, erster Unterschied bei 286:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/ueber-uns\/team\/","title":{"rendered":"Team"},"content":{"rendered":"<p class=\"wp-block-paragraph\…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/ueber-uns\/team\/","title":{"rendered":"Team"},"content":{"rendered":"<p class=\"wp-block…
```


### `/wp-json/wp/v2/pages/20` (REST-API)

- **header:link**: RustPress `</ueber-uns/>; rel="alternate"; type=text/html` statt `</index.php/ueber-uns/>; rel="alternate"; type=text/html`
- **bytes**

```text
1559 statt 1658 Bytes, erster Unterschied bei 285:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/ueber-uns\/","title":{"rendered":"\u00dcber uns"},"content":{"rendered":"<p class=\"wp-block-paragra…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/ueber-uns\/","title":{"rendered":"\u00dcber uns"},"content":{"rendered":"<p class=\"wp-bl…
```


### `/wp-json/wp/v2/pages/2` (REST-API)

- **header:link**: RustPress `</sample-page/>; rel="alternate"; type=text/html` statt `</index.php/sample-page/>; rel="alternate"; type=text/html`
- **bytes**

```text
3044 statt 3132 Bytes, erster Unterschied bei 285:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/sample-page\/","title":{"rendered":"Sample Page"},"content":{"rendered":"\n<p class=\"wp-block-parag…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/sample-page\/","title":{"rendered":"Sample Page"},"content":{"rendered":"\n<p class=\"wp-…
```


### `/wp-json/wp/v2/pages/264` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-query-irregular-grid/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-query-irregular-grid/>; rel="alternate"; type=text/html`
- **bytes**

```text
1490 statt 1578 Bytes, erster Unterschied bei 348:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-query-irregular-grid\/","title":{"rendered":"Pattern twentytwentytwo query-irreg…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-query-irregular-grid\/","title":{"rendered":"Pattern twentytwentytwo …
```


### `/wp-json/wp/v2/pages/263` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-query-large-titles/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-query-large-titles/>; rel="alternate"; type=text/html`
- **bytes**

```text
1482 statt 1570 Bytes, erster Unterschied bei 344:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-query-large-titles\/","title":{"rendered":"Pattern twentytwentytwo query-large-t…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-query-large-titles\/","title":{"rendered":"Pattern twentytwentytwo qu…
```


### `/wp-json/wp/v2/pages/262` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-query-image-grid/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-query-image-grid/>; rel="alternate"; type=text/html`
- **bytes**

```text
1474 statt 1562 Bytes, erster Unterschied bei 340:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-query-image-grid\/","title":{"rendered":"Pattern twentytwentytwo query-image-gri…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-query-image-grid\/","title":{"rendered":"Pattern twentytwentytwo quer…
```


### `/wp-json/wp/v2/pages/261` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-query-text-grid/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-query-text-grid/>; rel="alternate"; type=text/html`
- **bytes**

```text
1470 statt 1558 Bytes, erster Unterschied bei 338:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-query-text-grid\/","title":{"rendered":"Pattern twentytwentytwo query-text-grid"…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-query-text-grid\/","title":{"rendered":"Pattern twentytwentytwo query…
```


### `/wp-json/wp/v2/pages/260` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-query-grid/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-query-grid/>; rel="alternate"; type=text/html`
- **bytes**

```text
1450 statt 1538 Bytes, erster Unterschied bei 328:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-query-grid\/","title":{"rendered":"Pattern twentytwentytwo query-grid"},"content…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-query-grid\/","title":{"rendered":"Pattern twentytwentytwo query-grid…
```


### `/wp-json/wp/v2/pages/259` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-query-simple-blog/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-query-simple-blog/>; rel="alternate"; type=text/html`
- **bytes**

```text
1478 statt 1566 Bytes, erster Unterschied bei 342:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-query-simple-blog\/","title":{"rendered":"Pattern twentytwentytwo query-simple-b…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-query-simple-blog\/","title":{"rendered":"Pattern twentytwentytwo que…
```


### `/wp-json/wp/v2/pages/258` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-query-default/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-query-default/>; rel="alternate"; type=text/html`
- **bytes**

```text
1462 statt 1550 Bytes, erster Unterschied bei 334:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-query-default\/","title":{"rendered":"Pattern twentytwentytwo query-default"},"c…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-query-default\/","title":{"rendered":"Pattern twentytwentytwo query-d…
```


### `/wp-json/wp/v2/pages/257` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-sidebar-blog-posts-right/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-sidebar-blog-posts-right/>; rel="alternate"; type=text/html`
- **bytes**

```text
1526 statt 1614 Bytes, erster Unterschied bei 366:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-sidebar-blog-posts-right\/","title":{"rendered":"Pattern twentytwentytwo pa…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-sidebar-blog-posts-right\/","title":{"rendered":"Pattern twentyt…
```


### `/wp-json/wp/v2/pages/256` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-sidebar-blog-posts/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-sidebar-blog-posts/>; rel="alternate"; type=text/html`
- **bytes**

```text
1502 statt 1590 Bytes, erster Unterschied bei 354:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-sidebar-blog-posts\/","title":{"rendered":"Pattern twentytwentytwo page-sid…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-sidebar-blog-posts\/","title":{"rendered":"Pattern twentytwentyt…
```


### `/wp-json/wp/v2/pages/255` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-sidebar-grid-posts/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-sidebar-grid-posts/>; rel="alternate"; type=text/html`
- **bytes**

```text
1502 statt 1590 Bytes, erster Unterschied bei 354:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-sidebar-grid-posts\/","title":{"rendered":"Pattern twentytwentytwo page-sid…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-sidebar-grid-posts\/","title":{"rendered":"Pattern twentytwentyt…
```


### `/wp-json/wp/v2/pages/254` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-sidebar-poster/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-sidebar-poster/>; rel="alternate"; type=text/html`
- **bytes**

```text
1486 statt 1574 Bytes, erster Unterschied bei 346:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-sidebar-poster\/","title":{"rendered":"Pattern twentytwentytwo page-sidebar…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-sidebar-poster\/","title":{"rendered":"Pattern twentytwentytwo p…
```


### `/wp-json/wp/v2/pages/253` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-layout-two-columns/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-layout-two-columns/>; rel="alternate"; type=text/html`
- **bytes**

```text
1502 statt 1590 Bytes, erster Unterschied bei 354:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-layout-two-columns\/","title":{"rendered":"Pattern twentytwentytwo page-lay…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-layout-two-columns\/","title":{"rendered":"Pattern twentytwentyt…
```


### `/wp-json/wp/v2/pages/252` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-layout-image-text-and-video/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-layout-image-text-and-video/>; rel="alternate"; type=text/html`
- **bytes**

```text
1538 statt 1626 Bytes, erster Unterschied bei 372:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-layout-image-text-and-video\/","title":{"rendered":"Pattern twentytwentytwo…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-layout-image-text-and-video\/","title":{"rendered":"Pattern twen…
```


### `/wp-json/wp/v2/pages/251` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-layout-image-and-text/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-layout-image-and-text/>; rel="alternate"; type=text/html`
- **bytes**

```text
1514 statt 1602 Bytes, erster Unterschied bei 360:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-layout-image-and-text\/","title":{"rendered":"Pattern twentytwentytwo page-…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-layout-image-and-text\/","title":{"rendered":"Pattern twentytwen…
```


### `/wp-json/wp/v2/pages/250` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-about-large-image-and-buttons/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-about-large-image-and-buttons/>; rel="alternate"; type=text/html`
- **bytes**

```text
1546 statt 1634 Bytes, erster Unterschied bei 376:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-about-large-image-and-buttons\/","title":{"rendered":"Pattern twentytwentyt…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-about-large-image-and-buttons\/","title":{"rendered":"Pattern tw…
```


### `/wp-json/wp/v2/pages/249` (REST-API)

- **header:link**: RustPress `</pat-twentytwentytwo-page-about-links-dark/>; rel="alternate"; type=text/html` statt `</index.php/pat-twentytwentytwo-page-about-links-dark/>; rel="alternate"; type=text/html`
- **bytes**

```text
1494 statt 1582 Bytes, erster Unterschied bei 350:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/pat-twentytwentytwo-page-about-links-dark\/","title":{"rendered":"Pattern twentytwentytwo page-about…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/pat-twentytwentytwo-page-about-links-dark\/","title":{"rendered":"Pattern twentytwentytwo…
```


### `/wp-json/wp/v2/media/26` (REST-API)

- **bytes**

```text
3708 statt 3763 Bytes, erster Unterschied bei 3187:
RustPress: …:199724,"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/media\/26","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …:199724,"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/media\/26","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/media/25` (REST-API)

- **bytes**

```text
2266 statt 2310 Bytes, erster Unterschied bei 1948:
RustPress: …e":2480,"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/media\/25","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localh…
WordPress: …e":2480,"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/media\/25","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http…
```


### `/wp-json/wp/v2/categories/2` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
618 statt 662 Bytes, erster Unterschied bei 221:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/categories\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/lo…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/categories\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"…
```


### `/wp-json/wp/v2/categories/4` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
716 statt 771 Bytes, erster Unterschied bei 227:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/categories\/4","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/lo…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/categories\/4","targetHints":{"allow":["GET"]}}],"collection":[{"href":"…
```


### `/wp-json/wp/v2/categories/3` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
617 statt 661 Bytes, erster Unterschied bei 220:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/categories\/3","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/lo…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/categories\/3","targetHints":{"allow":["GET"]}}],"collection":[{"href":"…
```


### `/wp-json/wp/v2/categories/1` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
645 statt 689 Bytes, erster Unterschied bei 248:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/categories\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/lo…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/categories\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"…
```


### `/wp-json/wp/v2/tags/5` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
587 statt 631 Bytes, erster Unterschied bei 208:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/tags\/5","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localhos…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/tags\/5","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\…
```


### `/wp-json/wp/v2/tags/6` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
584 statt 628 Bytes, erster Unterschied bei 205:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/tags\/6","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localhos…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/tags\/6","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\…
```


### `/wp-json/wp/v2/tags/7` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
586 statt 630 Bytes, erster Unterschied bei 207:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/tags\/7","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localhos…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/tags\/7","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\…
```


### `/wp-json/wp/v2/users/1` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
772 statt 794 Bytes, erster Unterschied bei 636:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/users\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/users\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/users/2` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
767 statt 789 Bytes, erster Unterschied bei 631:
RustPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/users\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/localho…
WordPress: …eta":[],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/users\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:…
```


### `/wp-json/wp/v2/comments/18` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1208 statt 1241 Bytes, erster Unterschied bei 957:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/18","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/18","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/17` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1208 statt 1241 Bytes, erster Unterschied bei 957:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/17","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/17","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/16` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1309 statt 1353 Bytes, erster Unterschied bei 958:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/16","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/16","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/15` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1311 statt 1355 Bytes, erster Unterschied bei 957:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/15","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/15","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/14` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1208 statt 1241 Bytes, erster Unterschied bei 957:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/14","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/14","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/13` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1306 statt 1350 Bytes, erster Unterschied bei 955:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/13","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/13","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/12` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1308 statt 1352 Bytes, erster Unterschied bei 954:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/12","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/12","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/11` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1205 statt 1238 Bytes, erster Unterschied bei 954:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/11","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/11","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/10` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1304 statt 1348 Bytes, erster Unterschied bei 954:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loc…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/10","targetHints":{"allow":["GET"]}}],"collection":[{"href":"h…
```


### `/wp-json/wp/v2/comments/9` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1304 statt 1348 Bytes, erster Unterschied bei 952:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/9","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loca…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/9","targetHints":{"allow":["GET"]}}],"collection":[{"href":"ht…
```


### `/wp-json/wp/v2/comments/8` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1202 statt 1235 Bytes, erster Unterschied bei 952:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/8","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loca…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/8","targetHints":{"allow":["GET"]}}],"collection":[{"href":"ht…
```


### `/wp-json/wp/v2/comments/7` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1301 statt 1345 Bytes, erster Unterschied bei 952:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/7","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loca…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/7","targetHints":{"allow":["GET"]}}],"collection":[{"href":"ht…
```


### `/wp-json/wp/v2/comments/6` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1304 statt 1348 Bytes, erster Unterschied bei 952:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/6","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loca…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/6","targetHints":{"allow":["GET"]}}],"collection":[{"href":"ht…
```


### `/wp-json/wp/v2/comments/5` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1202 statt 1235 Bytes, erster Unterschied bei 952:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/5","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loca…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/5","targetHints":{"allow":["GET"]}}],"collection":[{"href":"ht…
```


### `/wp-json/wp/v2/comments/4` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1059 statt 1092 Bytes, erster Unterschied bei 811:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/4","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loca…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/4","targetHints":{"allow":["GET"]}}],"collection":[{"href":"ht…
```


### `/wp-json/wp/v2/comments/2` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1041 statt 1074 Bytes, erster Unterschied bei 793:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loca…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/2","targetHints":{"allow":["GET"]}}],"collection":[{"href":"ht…
```


### `/wp-json/wp/v2/comments/1` (REST-API)

- **header:link**: RustPress `</wp-json/>; rel="https://api.w.org/"` statt `</index.php/wp-json/>; rel="https://api.w.org/"`
- **bytes**

```text
1301 statt 1334 Bytes, erster Unterschied bei 1053:
RustPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/comments\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/loca…
WordPress: …":null},"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/comments\/1","targetHints":{"allow":["GET"]}}],"collection":[{"href":"ht…
```


### `/wp-json/wp/v2/posts?_embed&per_page=10` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/posts?_embed&per_page=10&page=2>; rel="next"` statt `</index.php/wp-json/wp/v2/posts?_embed&per_page=10&page=2>; rel="next"`
- **bytes**

```text
70990 statt 73201 Bytes, erster Unterschied bei 831:
RustPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/wp-json\/wp\/v2\/posts\/272","targetHints":{"allow":["GET"]}}],"collection":[{"href":"http:\/\/local…
WordPress: …rized"],"_links":{"self":[{"href":"http:\/\/localhost:8080\/index.php\/wp-json\/wp\/v2\/posts\/272","targetHints":{"allow":["GET"]}}],"collection":[{"href":"htt…
```


### `/wp-json/wp/v2/pages?_embed&per_page=10` (REST-API)

- **header:link**: RustPress `</wp-json/wp/v2/pages?_embed&per_page=10&page=2>; rel="next"` statt `</index.php/wp-json/wp/v2/pages?_embed&per_page=10&page=2>; rel="next"`
- **bytes**

```text
29896 statt 31172 Bytes, erster Unterschied bei 286:
RustPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/bildliste\/","title":{"rendered":"Bildliste"},"content":{"rendered":"\n<div class=\"wp-block-query i…
WordPress: …s":"publish","type":"page","link":"http:\/\/localhost:8080\/index.php\/bildliste\/","title":{"rendered":"Bildliste"},"content":{"rendered":"\n<div class=\"wp-bl…
```


### `/?page_id=32` (HTML-Seiten)

- **header:location**: RustPress `/bildliste/` statt `/index.php/bildliste/`

### `/?page_id=21` (HTML-Seiten)

- **header:location**: RustPress `/ueber-uns/team/` statt `/index.php/ueber-uns/team/`

### `/?page_id=20` (HTML-Seiten)

- **header:location**: RustPress `/ueber-uns/` statt `/index.php/ueber-uns/`

## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.**: `/wp-sitemap-posts-post-9999.xml` (bytes)
- **Die Referenz auf Port 8080 verliert beim Kanonisieren den Port (WordPress-Eigenheit) und leitet ?feed=… deshalb nicht um; RustPress leitet wie WordPress auf /feed/ um.**: `/?feed=rss2` (status), `/?feed=rss2` (header:content-type), `/?feed=rss2` (header:location), `/?feed=rss2` (header:etag), `/?feed=rss2` (header:last-modified), `/?feed=rss2` (header:link)
- **Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.**: `/wp-content/uploads/2026/10/test.mp3` (header:link), `/wp-content/uploads/2026/10/test.mp3` (bytes), `/wp-content/uploads/2026/10/test.mp4` (header:link), `/wp-content/uploads/2026/10/test.mp4` (bytes), `/wp-content/uploads/2026/10/test.mp3?_=1` (header:link), `/wp-content/uploads/2026/10/test.mp3?_=1` (bytes), `/wp-content/uploads/2026/10/test.mp4?_=1` (header:link), `/wp-content/uploads/2026/10/test.mp4?_=1` (bytes), `/wp-content/uploads/2026/10/test.mp3?_=2` (header:link), `/wp-content/uploads/2026/10/test.mp3?_=2` (bytes)
- **Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.**: `/robots.txt` (header:link), `/favicon.ico` (header:link)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-includes/js/jquery/jquery.min.js?ver=3.7.1` (header:cache-control), `/wp-includes/js/jquery/jquery-migrate.min.js?ver=3.4.1` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-includes/images/smilies/mrgreen.png` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior-150x150.webp` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-and-player.min.js?ver=4.2.17` (header:cache-control), `/wp-includes/js/mediaelement/mediaelement-migrate.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/wp-mediaelement.min.js?ver=7.1.3` (header:cache-control), `/wp-includes/js/mediaelement/renderers/vimeo.min.js?ver=4.2.17` (header:cache-control) … und 2 weitere

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
