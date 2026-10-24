# Kompatibilitätsbericht: twentytwentyfive

Stand: 2026-10-24 23:42 UTC · Referenz: WordPress 7.1.3 · Theme: `twentytwentyfive`

**180 URLs verglichen: 149 identisch, 15 mit bekannten Abweichungen, 16 mit unerwarteten Abweichungen.**

| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |
|---|---:|---:|---:|---:|---:|---:|
| HTML-Seiten | 62 | 53 | 0 | 9 | 1141 ms | 69 ms |
| Feeds | 34 | 31 | 1 | 2 | 45 ms | 19 ms |
| Sitemaps | 9 | 9 | 0 | 0 | 14 ms | 22 ms |
| REST-API | 61 | 53 | 3 | 5 | 103 ms | 29 ms |
| Dateien | 7 | 0 | 7 | 0 | 52 ms | 4 ms |
| Sonstiges | 7 | 3 | 4 | 0 | 363 ms | 15 ms |

## Unerwartete Abweichungen

### `/` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-block-query-total-inline-css", "wp-block-post-comments-count-inline-css", "wp-block-read-more-inline-css", "wp-block-term-name-inline-css", "wp-block-term-count-inline-css", "wp-block-term-template-inline-css", "wp-block-archives-inline-css", "wp-block-categories-inline-css", "wp-block-tag-cloud-inline-css", "wp-block-latest-comments-inline-css", "wp-block-calendar-inline-css", "wp-block-loginout-inline-css", "wp-block-post-comments-link-inline-css", "wp-block-post-author-biography-inline-css", "wp-block-post-time-to-read-inline-css"], zu viel: []
- **body**

```text
24669 statt 35252 Bytes, erster Unterschied bei 8906:
RustPress: …ut-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<img …
WordPress: …ut-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><p><figure id="attachment_26" aria-describedby="caption-attachment-26" style="width: 300px" class="wp-c…
```

- **head**: fehlt: <style id=wp-block-query-total-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-count-inline-css>
- **head**: fehlt: <style id=wp-block-read-more-inline-css>
- **head**: fehlt: <style id=wp-block-term-name-inline-css>
- **head**: fehlt: <style id=wp-block-term-count-inline-css>
- **head**: fehlt: <style id=wp-block-term-template-inline-css>
- **head**: fehlt: <style id=wp-block-archives-inline-css>
- **head**: fehlt: <style id=wp-block-categories-inline-css>
- **head**: fehlt: <style id=wp-block-tag-cloud-inline-css>
- **head**: fehlt: <style id=wp-block-latest-comments-inline-css>
- **head**: fehlt: <style id=wp-block-calendar-inline-css>
- **head**: fehlt: <style id=wp-block-loginout-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-link-inline-css>
- **head**: fehlt: <style id=wp-block-post-author-biography-inline-css>
- **head**: fehlt: <style id=wp-block-post-time-to-read-inline-css>

### `/?s=` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-block-query-total-inline-css", "wp-block-post-comments-count-inline-css", "wp-block-read-more-inline-css", "wp-block-term-name-inline-css", "wp-block-term-count-inline-css", "wp-block-term-template-inline-css", "wp-block-archives-inline-css", "wp-block-categories-inline-css", "wp-block-tag-cloud-inline-css", "wp-block-latest-comments-inline-css", "wp-block-calendar-inline-css", "wp-block-loginout-inline-css", "wp-block-post-comments-link-inline-css", "wp-block-post-author-biography-inline-css", "wp-block-post-time-to-read-inline-css", "wp-block-cover-css"], zu viel: ["wp-block-cover-inline-css"]
- **body**

```text
25907 statt 36438 Bytes, erster Unterschied bei 6735:
RustPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><figure id="attachment_26" aria-describedby="caption-attachment-26" style="width: 300px" class="wp-ca…
```

- **head**: fehlt: <style id=wp-block-query-total-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-count-inline-css>
- **head**: fehlt: <style id=wp-block-read-more-inline-css>
- **head**: fehlt: <style id=wp-block-term-name-inline-css>
- **head**: fehlt: <style id=wp-block-term-count-inline-css>
- **head**: fehlt: <style id=wp-block-term-template-inline-css>
- **head**: fehlt: <style id=wp-block-archives-inline-css>
- **head**: fehlt: <style id=wp-block-categories-inline-css>
- **head**: fehlt: <style id=wp-block-tag-cloud-inline-css>
- **head**: fehlt: <style id=wp-block-latest-comments-inline-css>
- **head**: fehlt: <style id=wp-block-calendar-inline-css>
- **head**: fehlt: <style id=wp-block-loginout-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-link-inline-css>
- **head**: fehlt: <style id=wp-block-post-author-biography-inline-css>
- **head**: fehlt: <style id=wp-block-post-time-to-read-inline-css>
- **head**: fehlt: <link id=wp-block-cover-css>
- **head**: zu viel: <style id=wp-block-cover-inline-css>

### `/feed/` (Feeds)

- **bytes**

```text
19120 statt 28914 Bytes, erster Unterschied bei 1271:
RustPress: …		<description><![CDATA[Ein Absatz mit klassischem Inhalt. [caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;] Eine …
WordPress: …		<description><![CDATA[Ein Absatz mit klassischem Inhalt. [unbekannt]Inhalt[/unbekannt] und [kein] Shortcode [] hier. bleibt wörtlich.]]></description>⏎						…
```


### `/feed/atom/` (Feeds)

- **bytes**

```text
21225 statt 31026 Bytes, erster Unterschied bei 1088:
RustPress: …ry type="html"><![CDATA[Ein Absatz mit klassischem Inhalt. [caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;] Eine …
WordPress: …ry type="html"><![CDATA[Ein Absatz mit klassischem Inhalt. [unbekannt]Inhalt[/unbekannt] und [kein] Shortcode [] hier. bleibt wörtlich.]]></summary>⏎⏎					<con…
```


### `/wp-json/wp/v2/posts` (REST-API)

- **bytes**

```text
29390 statt 41022 Bytes, erster Unterschied bei 436:
RustPress: …":{"rendered":"<p>Ein Absatz mit klassischem Inhalt.<\/p>\n<p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …":{"rendered":"<p>Ein Absatz mit klassischem Inhalt.<\/p>\n<figure id=\"attachment_26\" aria-describedby=\"caption-attachment-26\" style=\"width: 300px\" class=…
```


### `/2026/` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-block-query-total-inline-css", "wp-block-post-comments-count-inline-css", "wp-block-read-more-inline-css", "wp-block-term-name-inline-css", "wp-block-term-count-inline-css", "wp-block-term-template-inline-css", "wp-block-archives-inline-css", "wp-block-categories-inline-css", "wp-block-tag-cloud-inline-css", "wp-block-latest-comments-inline-css", "wp-block-calendar-inline-css", "wp-block-loginout-inline-css", "wp-block-post-comments-link-inline-css", "wp-block-post-author-biography-inline-css", "wp-block-post-time-to-read-inline-css"], zu viel: []
- **body**

```text
21908 statt 32459 Bytes, erster Unterschied bei 6165:
RustPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><figure id="attachment_26" aria-describedby="caption-attachment-26" style="width: 300px" class="wp-ca…
```

- **head**: fehlt: <style id=wp-block-query-total-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-count-inline-css>
- **head**: fehlt: <style id=wp-block-read-more-inline-css>
- **head**: fehlt: <style id=wp-block-term-name-inline-css>
- **head**: fehlt: <style id=wp-block-term-count-inline-css>
- **head**: fehlt: <style id=wp-block-term-template-inline-css>
- **head**: fehlt: <style id=wp-block-archives-inline-css>
- **head**: fehlt: <style id=wp-block-categories-inline-css>
- **head**: fehlt: <style id=wp-block-tag-cloud-inline-css>
- **head**: fehlt: <style id=wp-block-latest-comments-inline-css>
- **head**: fehlt: <style id=wp-block-calendar-inline-css>
- **head**: fehlt: <style id=wp-block-loginout-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-link-inline-css>
- **head**: fehlt: <style id=wp-block-post-author-biography-inline-css>
- **head**: fehlt: <style id=wp-block-post-time-to-read-inline-css>

### `/2026/10/` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-block-query-total-inline-css", "wp-block-post-comments-count-inline-css", "wp-block-read-more-inline-css", "wp-block-term-name-inline-css", "wp-block-term-count-inline-css", "wp-block-term-template-inline-css", "wp-block-archives-inline-css", "wp-block-categories-inline-css", "wp-block-tag-cloud-inline-css", "wp-block-latest-comments-inline-css", "wp-block-calendar-inline-css", "wp-block-loginout-inline-css", "wp-block-post-comments-link-inline-css", "wp-block-post-author-biography-inline-css", "wp-block-post-time-to-read-inline-css"], zu viel: []
- **body**

```text
21932 statt 32508 Bytes, erster Unterschied bei 6174:
RustPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><figure id="attachment_26" aria-describedby="caption-attachment-26" style="width: 300px" class="wp-ca…
```

- **head**: fehlt: <style id=wp-block-query-total-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-count-inline-css>
- **head**: fehlt: <style id=wp-block-read-more-inline-css>
- **head**: fehlt: <style id=wp-block-term-name-inline-css>
- **head**: fehlt: <style id=wp-block-term-count-inline-css>
- **head**: fehlt: <style id=wp-block-term-template-inline-css>
- **head**: fehlt: <style id=wp-block-archives-inline-css>
- **head**: fehlt: <style id=wp-block-categories-inline-css>
- **head**: fehlt: <style id=wp-block-tag-cloud-inline-css>
- **head**: fehlt: <style id=wp-block-latest-comments-inline-css>
- **head**: fehlt: <style id=wp-block-calendar-inline-css>
- **head**: fehlt: <style id=wp-block-loginout-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-link-inline-css>
- **head**: fehlt: <style id=wp-block-post-author-biography-inline-css>
- **head**: fehlt: <style id=wp-block-post-time-to-read-inline-css>

### `/2026/10/24/` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-block-query-total-inline-css", "wp-block-post-comments-count-inline-css", "wp-block-read-more-inline-css", "wp-block-term-name-inline-css", "wp-block-term-count-inline-css", "wp-block-term-template-inline-css", "wp-block-archives-inline-css", "wp-block-categories-inline-css", "wp-block-tag-cloud-inline-css", "wp-block-latest-comments-inline-css", "wp-block-calendar-inline-css", "wp-block-loginout-inline-css", "wp-block-post-comments-link-inline-css", "wp-block-post-author-biography-inline-css", "wp-block-post-time-to-read-inline-css"], zu viel: []
- **body**

```text
21949 statt 32530 Bytes, erster Unterschied bei 6176:
RustPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><figure id="attachment_26" aria-describedby="caption-attachment-26" style="width: 300px" class="wp-ca…
```

- **head**: fehlt: <style id=wp-block-query-total-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-count-inline-css>
- **head**: fehlt: <style id=wp-block-read-more-inline-css>
- **head**: fehlt: <style id=wp-block-term-name-inline-css>
- **head**: fehlt: <style id=wp-block-term-count-inline-css>
- **head**: fehlt: <style id=wp-block-term-template-inline-css>
- **head**: fehlt: <style id=wp-block-archives-inline-css>
- **head**: fehlt: <style id=wp-block-categories-inline-css>
- **head**: fehlt: <style id=wp-block-tag-cloud-inline-css>
- **head**: fehlt: <style id=wp-block-latest-comments-inline-css>
- **head**: fehlt: <style id=wp-block-calendar-inline-css>
- **head**: fehlt: <style id=wp-block-loginout-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-link-inline-css>
- **head**: fehlt: <style id=wp-block-post-author-biography-inline-css>
- **head**: fehlt: <style id=wp-block-post-time-to-read-inline-css>

### `/dynamische-bloecke/` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-block-archives-inline-css", "wp-block-categories-inline-css", "wp-block-tag-cloud-inline-css", "wp-block-latest-comments-inline-css", "wp-block-calendar-inline-css", "wp-block-loginout-inline-css", "wp-block-post-comments-count-inline-css", "wp-block-post-comments-link-inline-css", "wp-block-post-author-biography-inline-css", "wp-block-post-time-to-read-inline-css", "wp-block-read-more-inline-css"], zu viel: []
- **body**

```text
18392 statt 26389 Bytes, erster Unterschied bei 6251:
RustPress: …ut-constrained wp-block-post-content-is-layout-constrained"> [unknown_shortcode foo=&#8221;bar&#8221;] </div><div class="wp-block-group has-global-padding is-la…
WordPress: …ut-constrained wp-block-post-content-is-layout-constrained"><ul class="wp-block-archives-list wp-block-archives"><li><a href='http://localhost:8080/2026/10/'>Oc…
```

- **head**: fehlt: <style id=wp-block-archives-inline-css>
- **head**: fehlt: <style id=wp-block-categories-inline-css>
- **head**: fehlt: <style id=wp-block-tag-cloud-inline-css>
- **head**: fehlt: <style id=wp-block-latest-comments-inline-css>
- **head**: fehlt: <style id=wp-block-calendar-inline-css>
- **head**: fehlt: <style id=wp-block-loginout-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-count-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-link-inline-css>
- **head**: fehlt: <style id=wp-block-post-author-biography-inline-css>
- **head**: fehlt: <style id=wp-block-post-time-to-read-inline-css>
- **head**: fehlt: <style id=wp-block-read-more-inline-css>
- **head**

```text
<style id=block-style-variation-styles-inline-css> ist anders:
543 statt 543 Bytes, erster Unterschied bei 90:
RustPress: …on-styles-inline-css"> :root :where(.is-style-post-terms-1--2 a:where(:not(.wp-element-button))){border-radius: 20px;border-color: var(--wp--preset--color--acce…
WordPress: …on-styles-inline-css"> :root :where(.is-style-post-terms-1--3 a:where(:not(.wp-element-button))){border-radius: 20px;border-color: var(--wp--preset--color--acce…
```


### `/abfrage-bloecke/` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-block-query-total-inline-css", "wp-block-post-comments-count-inline-css", "wp-block-read-more-inline-css", "wp-block-term-name-inline-css", "wp-block-term-count-inline-css", "wp-block-term-template-inline-css"], zu viel: []
- **body**

```text
19596 statt 20985 Bytes, erster Unterschied bei 6322:
RustPress: …-block-query is-layout-flow wp-block-query-is-layout-flow"><ul class="wp-block-post-template is-layout-flow wp-block-post-template-is-layout-flow"><li class="wp…
WordPress: …-block-query is-layout-flow wp-block-query-is-layout-flow"><div class="wp-block-query-total">26 results found</div><ul class="wp-block-post-template is-layout-f…
```

- **head**: fehlt: <style id=wp-block-query-total-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-count-inline-css>
- **head**: fehlt: <style id=wp-block-read-more-inline-css>
- **head**: fehlt: <style id=wp-block-term-name-inline-css>
- **head**: fehlt: <style id=wp-block-term-count-inline-css>
- **head**: fehlt: <style id=wp-block-term-template-inline-css>

### `/klassischer-inhalt/` (HTML-Seiten)

- **body**

```text
19048 statt 20201 Bytes, erster Unterschied bei 6293:
RustPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><figure id="attachment_26" aria-describedby="caption-attachment-26" style="width: 300px" class="wp-ca…
```


### `/author/admin/` (HTML-Seiten)

- **head-ids**: fehlt: ["wp-block-query-total-inline-css", "wp-block-post-comments-count-inline-css", "wp-block-read-more-inline-css", "wp-block-term-name-inline-css", "wp-block-term-count-inline-css", "wp-block-term-template-inline-css", "wp-block-archives-inline-css", "wp-block-categories-inline-css", "wp-block-tag-cloud-inline-css", "wp-block-latest-comments-inline-css", "wp-block-calendar-inline-css", "wp-block-loginout-inline-css", "wp-block-post-comments-link-inline-css", "wp-block-post-author-biography-inline-css", "wp-block-post-time-to-read-inline-css"], zu viel: []
- **body**

```text
21951 statt 32492 Bytes, erster Unterschied bei 6168:
RustPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …yout-constrained"><p>Ein Absatz mit klassischem Inhalt.</p><figure id="attachment_26" aria-describedby="caption-attachment-26" style="width: 300px" class="wp-ca…
```

- **head**: fehlt: <style id=wp-block-query-total-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-count-inline-css>
- **head**: fehlt: <style id=wp-block-read-more-inline-css>
- **head**: fehlt: <style id=wp-block-term-name-inline-css>
- **head**: fehlt: <style id=wp-block-term-count-inline-css>
- **head**: fehlt: <style id=wp-block-term-template-inline-css>
- **head**: fehlt: <style id=wp-block-archives-inline-css>
- **head**: fehlt: <style id=wp-block-categories-inline-css>
- **head**: fehlt: <style id=wp-block-tag-cloud-inline-css>
- **head**: fehlt: <style id=wp-block-latest-comments-inline-css>
- **head**: fehlt: <style id=wp-block-calendar-inline-css>
- **head**: fehlt: <style id=wp-block-loginout-inline-css>
- **head**: fehlt: <style id=wp-block-post-comments-link-inline-css>
- **head**: fehlt: <style id=wp-block-post-author-biography-inline-css>
- **head**: fehlt: <style id=wp-block-post-time-to-read-inline-css>

### `/wp-json/wp/v2/posts/36` (REST-API)

- **bytes**

```text
3166 statt 4146 Bytes, erster Unterschied bei 435:
RustPress: …":{"rendered":"<p>Ein Absatz mit klassischem Inhalt.<\/p>\n<p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …":{"rendered":"<p>Ein Absatz mit klassischem Inhalt.<\/p>\n<figure id=\"attachment_26\" aria-describedby=\"caption-attachment-26\" style=\"width: 300px\" class=…
```


### `/wp-json/wp/v2/posts/35` (REST-API)

- **bytes**

```text
3043 statt 4592 Bytes, erster Unterschied bei 455:
RustPress: …-block-query is-layout-flow wp-block-query-is-layout-flow\">\n\n<ul class=\"wp-block-post-template is-layout-flow wp-block-post-template-is-layout-flow\"><li cl…
WordPress: …-block-query is-layout-flow wp-block-query-is-layout-flow\"><div class=\"wp-block-query-total\">26 results found<\/div>\n\n<ul class=\"wp-block-post-template is…
```


### `/wp-json/wp/v2/posts/34` (REST-API)

- **bytes**

```text
1804 statt 10916 Bytes, erster Unterschied bei 390:
RustPress: …e":{"rendered":"Dynamische Bloecke"},"content":{"rendered":"\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n[unknown_shortcode foo=&#8221;bar&#822…
WordPress: …e":{"rendered":"Dynamische Bloecke"},"content":{"rendered":"<ul class=\"wp-block-archives-list wp-block-archives\">\t<li><a href='http:\/\/localhost:8080\/2026\…
```


### `/wp-json/wp/v2/posts?_embed&per_page=10` (REST-API)

- **bytes**

```text
43331 statt 55017 Bytes, erster Unterschied bei 436:
RustPress: …":{"rendered":"<p>Ein Absatz mit klassischem Inhalt.<\/p>\n<p>[caption id=&#8221;attachment_26&#8243; align=&#8221;alignright&#8221; width=&#8221;300&#8243;]<im…
WordPress: …":{"rendered":"<p>Ein Absatz mit klassischem Inhalt.<\/p>\n<figure id=\"attachment_26\" aria-describedby=\"caption-attachment-26\" style=\"width: 300px\" class=…
```


## Bekannte Abweichungen

- **Anmeldung, Admin-Oberfläche, XML-RPC und Cron gibt es nicht: RustPress arbeitet nur lesend und ohne PHP.**: `/wp-login.php` (status), `/wp-login.php` (header:link), `/wp-admin/` (status), `/wp-admin/` (header:location), `/wp-admin/` (header:link), `/xmlrpc.php` (status), `/xmlrpc.php` (header:content-type), `/xmlrpc.php` (header:cache-control), `/xmlrpc.php` (header:expires), `/xmlrpc.php` (header:link), `/xmlrpc.php` (header:allow), `/wp-cron.php` (status) … und 2 weitere
- **Der Routenindex von /wp-json/ listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).**: `/wp-json/` (bytes)
- **Die Referenz auf Port 8080 verliert beim Kanonisieren den Port (WordPress-Eigenheit) und leitet ?feed=… deshalb nicht um; RustPress leitet wie WordPress auf /feed/ um.**: `/?feed=rss2` (status), `/?feed=rss2` (header:content-type), `/?feed=rss2` (header:location), `/?feed=rss2` (header:etag), `/?feed=rss2` (header:last-modified), `/?feed=rss2` (header:link)
- **Diese REST-Routen sind noch nicht umgesetzt (siehe PLAN.md).**: `/wp-json/wp/v2/types` (status), `/wp-json/wp/v2/types` (header:allow), `/wp-json/wp/v2/search?search=test` (status), `/wp-json/wp/v2/search?search=test` (header:link), `/wp-json/wp/v2/search?search=test` (header:x-wp-total), `/wp-json/wp/v2/search?search=test` (header:x-wp-totalpages), `/wp-json/wp/v2/search?search=test` (header:allow)
- **RustPress sendet für statische Dateien bewusst Cache-Control: public, max-age=86400; der Apache der Referenz sendet nichts.**: `/wp-includes/js/dist/script-modules/interactivity/index.min.js?ver=efaa5193bbad9c60ffd1` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior.webp` (header:cache-control), `/wp-content/uploads/2026/10/building-exterior-150x150.webp` (header:cache-control), `/wp-content/uploads/2026/10/wordpress-logo.png` (header:cache-control), `/wp-includes/js/dist/script-modules/block-library/navigation/view.min.js?ver=1bf28ded04f9f188bdcb` (header:cache-control), `/wp-includes/blocks/navigation/style.min.css?ver=7.1.3` (header:cache-control), `/wp-includes/blocks/cover/style.min.css?ver=7.1.3` (header:cache-control)

Eingetragen, aber in diesem Lauf nicht aufgetreten:

- `^/wp-sitemap-posts-post-9999[.]xml$` (bytes): Die 404-Seite eines fehlenden Sitemaps unterscheidet sich von WordPress nur durch Whitespace im Head.
- `^/category/uncategorized/page/2/$` (body): Beiträge mit exakt gleichem Datum (hier gleich-a/gleich-b) haben in MariaDB keine definierte Reihenfolge; sie hängt vom Plan der Abfrage ab und kann sich zwischen WordPress und RustPress unterscheiden.
- `^/[?]rest_route=/$` (bytes): Der Routenindex listet nur die Routen, die RustPress selbst bedient (eine Teilmenge).
- `^/(robots[.]txt|favicon[.]ico)$` (*): Ohne Permalink-Regeln kennt WordPress diese Dateien nicht und leitet /robots.txt auf /robots.txt/ um (Eigenheit einfacher Permalinks); RustPress liefert dafür 404.
- `^/wp-content/uploads/` (*): Die Referenz der echten Seite hat die hochgeladenen Dateien nicht (sie liegen beim Betreiber); beide Seiten antworten ohne Datei unterschiedlich.
