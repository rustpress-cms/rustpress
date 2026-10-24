# Plan: RustPress, ein WordPress-Dropin-Ersatz ohne PHP

## Context

Ziel: Bestehende WordPress-Seiten sollen ohne PHP und ohne Datenbank-Konvertierung laufen. Das Rust-Binary liest die vorhandene MySQL/MariaDB (originales `wp_*`-Schema) und das vorhandene `wp-content/` direkt.

Entschieden (mit dem User abgestimmt):
- **Themes/Plugins:** Es werden nur Block-Themes unterstützt (`theme.json` + HTML-Templates, kein PHP nötig). Wichtige Plugins werden später als native Rust-Module nachgebaut. Klassische PHP-Themes und beliebige PHP-Plugins laufen bewusst nicht.
- **Meilenstein 1:** read-only Frontend (Seiten, Beiträge, Archive, Feeds, REST-API lesend). Es gibt keinen Schreibzugriff auf die DB.
- **DB:** MySQL/MariaDB über `sqlx`.

Stand heute: Workspace mit einem Crate, `crates/rustpress-phpser` (`unserialize`/`serialize` in `src/lib.rs`, 8 Tests grün). Toolchain `stable-x86_64-pc-windows-msvc` ist eingerichtet. PHP und MySQL sind lokal nicht installiert.

## Ziel-Architektur (Workspace-Crates)

| Crate | Aufgabe |
|---|---|
| `rustpress-phpser` (existiert) | PHP-serialize, byte-genau. Wird um Referenzen (`R:`/`r:`) und `S:` erweitert. |
| `rustpress-db` | `sqlx`-MySQL-Pool, Tabellenpräfix aus der Config, typisierte Reads für `posts`, `postmeta`, `options`, `terms`, `term_taxonomy`, `term_relationships`, `users`, `usermeta`, `comments`. Die Verbindung wird nur lesend benutzt. |
| `rustpress-core` | Options-Cache (nutzt `phpser` für serialisierte Werte), Post-Abfrage (Teilmenge von `WP_Query`), Permalink-/Rewrite-Auflösung aus `permalink_structure` und `rewrite_rules`, Template-Hierarchie. |
| `rustpress-blocks` | Parser für Block-Kommentare (`<!-- wp:name {json} -->`) und Renderer für die Core-Blöcke, zuerst paragraph, heading, image, list, group, columns, post-title, post-content, query, navigation, template-part. |
| `rustpress-theme` | `theme.json`, `templates/*.html`, `parts/*.html`, Global-Styles-CSS-Generator, Auflösung des aktiven Themes aus den Options `template`/`stylesheet`. |
| `rustpress-server` | `axum`-Binary: Routing, statische Dateien (`wp-content/uploads`, Theme-Assets), Konfiguration (DB-URL, WP-Pfad), Logging. |
| `rustpress-compat` (nur Tests) | Golden-Tests: gleiche URL gegen ein echtes WordPress und gegen RustPress, normalisiertes HTML wird verglichen. |

## Schritte

0. **Plan im Repo ablegen**: Dieser Plan wird als `PLAN.md` im Repo-Root (`C:\work\experimental\rustpress\PLAN.md`) gespeichert. Das gilt dauerhaft: Jeder Plan wird zusätzlich im Repo aufgeschrieben und bei Änderungen dort mitgepflegt (Wunsch des Users). Außerdem wird das als Feedback-Memory festgehalten.
0. ✅ Erledigt: `PLAN.md` liegt im Repo.
1. ✅ **`phpser` abgeschlossen** (`crates/rustpress-phpser/src/lib.rs`): `R:`/`r:` (als `Value::Reference`/`ObjectRef` mit Slot-Nummer) und `S:` sind implementiert. `C:` und `E:` bleiben `Error::Unsupported`. Objekt-Properties haben jetzt `Key` statt `Vec<u8>`, weil Klassen mit `__serialize()` (z. B. `ArrayObject`) Integer-Keys nutzen. `crates/rustpress-phpser/tests/php_compat.rs` vergleicht die Ausgabe von echtem PHP 8.4 byte-genau (überspringt sich ohne `php`). Dabei wurde die Float-Schwelle korrigiert: Exponentialschreibweise außerhalb von 1e-4 ≤ |x| < 1e17.
2. ✅ **Workspace erweitert** (Crates sind angelegt, `[workspace.dependencies]` pflegt Versionen zentral; sqlx 0.9 mit `mysql`, `runtime-tokio`, `tls-rustls`; der Server ist ein Stub). Ursprünglicher Auftrag: Crates `rustpress-db`, `rustpress-core`, `rustpress-blocks`, `rustpress-theme`, `rustpress-server` anlegen, Dependencies zentral in `[workspace.dependencies]` pflegen (`tokio`, `axum`, `sqlx`, `serde`, `serde_json`, `tracing`).
3. ✅ **DB-Schicht umgesetzt** (`crates/rustpress-db`): `Db::connect` (Session `READ ONLY`, validiertes Tabellenpräfix), Options, Posts (`PostQuery` als `WP_Query`-Teilmenge mit Typ/Status/Name/Parent/Autor/Term/Suche/Datum/Sortierung/Paging und `count_posts`), Postmeta, Terms, Users, freigegebene Kommentare. Datumsspalten kommen als Strings (WP erlaubt `0000-00-00`). Tests in `tests/live_db.rs` laufen gegen die Referenz-DB und überspringen sich ohne DB. Entwicklungs-Stack: `dev/up.sh` (Podman: MariaDB `:3306`, WordPress `:8080`), `dev/seed.sh` (Testinhalte), `dev/wp.sh` (WP-CLI); der Web-Installer wurde per HTTP-POST ausgeführt (WP-CLI `core install` funktioniert mit WP 7.1.3 nicht). Noch offen in dieser Schicht: Usermeta, Termmeta, Kommentar-Meta, Multisite. Ursprünglicher Auftrag: Read-Modelle und Queries, getestet gegen eine echte WP-DB. Config über Env/TOML.
4. ✅ **Core umgesetzt** (`crates/rustpress-core`): `Options` (Autoload-Cache, `maybe_unserialize` über `phpser`), `Settings`, `RewriteRules` (die gespeicherten `rewrite_rules` werden als PCRE-artige Byte-Regexe in Reihenfolge angewendet, inkl. „verbose page rules“-Prüfung wie `WP::parse_request`), `Site::query(path, query_string)` mit den `is_*`-Flags und Posts der Hauptabfrage (Home inkl. Sticky Posts und statischer Startseite/Beitragsseite, Einzelbeitrag/Seite/Anhang, Kategorie/Tag/Taxonomie/Autor/Datum, Suche mit Relevanz-Sortierung, Paging, `handle_404`-Regeln) und `hierarchy::template_candidates` (Template-Kette bis `index`). Validiert durch `tests/golden.rs`: 67 URLs werden gegen das echte WordPress verglichen (HTTP-Status, `body_class`, Beitragstitel in Reihenfolge); der Test hat u. a. die Suchstrategie bei Stoppwörtern und die 200-Antwort bei zu hohen `page`-Nummern korrigiert.
   - **Offen / bewusst vereinfacht:** Canonical Redirects (`/team/` -> `/ueber-uns/team/`, `?p=ID` -> Permalink, ungültige Monate/Tage, Seite hinter `<!--nextpage-->`), Feeds (nur die Flags), eigene Post Types/Taxonomien (aus Plugins; Heuristik über Query-Var-Namen geplant), Autoren-Slug-Varianten, Kategorie-Pfad (nur letztes Segment, wie in WP), Status `private` (kein Login).
   - Sonstiges: Ursprünglicher Auftrag:  Options, Permalinks, Post-Abfrage, Template-Hierarchie (single, page, archive, home, 404, search).
5. ✅ **Blocks umgesetzt** (`crates/rustpress-blocks`): Port von `WP_Block_Parser`, Block-Registry (liest `block.json` aus `wp-includes/blocks`, inkl. `style`-Handles; Blöcke ohne `style`-Eintrag, aber mit `style.min.css`, bekommen `wp-block-<name>` wie in WP), Render-Pipeline (`render_block_data`-/`render_block`-Filter nachgebaut: Block-Supports für Farbe/Typografie/Rahmen/Abstand/Maße/Schatten/Anker, Layout-Klassen mit `wp-container-…`-Hash, Elements, Block-Style-Variationen, Interactivity-Direktiven), `WP_Style_Engine`, `WP_HTML_Tag_Processor`-Semantik (`TagEditor`) und die dynamischen Blöcke (Site-Title/Tagline, Template-Part, Pattern, Post-Title/Date/Terms/Author/Content, Navigation inkl. Page-List und Submenüs, Query-Loop mit Pagination, Suche, Kommentare inkl. Formular, Post-Navigation). Content-Filter: `wptexturize`, `wpautop`, `capital_P_dangit`, Passwort-Formular, `<!--more-->`/`nextpage`. Verglichen mit echtem WP in `rustpress-theme/tests/golden_pages.rs`: 10 URLs liefern identisches Body-Markup (Whitespace-normalisiert). Bekannte Lücken: `post-featured-image`/`site-logo` (noch keine Bildunterstützung), `wp_filter_content_tags` (Lazy-Loading), `make_clickable`/Smilies in Kommentaren, Shortcodes.
6. ✅ **Theme umgesetzt** (`crates/rustpress-theme`): `theme.json` in den Ebenen Core/Parent/Child/User (`wp_global_styles`) mit Preset-Auflösung und Block-Style-Variationen (`styles/blocks/*.json`), Templates/Parts aus Dateien mit Überschreibung durch `wp_template`/`wp_template_part` aus der DB, Patterns (mini PHP-Evaluator `phpt.rs` für die `<?php`-Fragmente in Pattern-Dateien), Global-Styles-Generator (Preset-Variablen, Fluid-Fonts, Root-/Layout-Regeln, Element- und Block-Styles; `golden_styles.rs`: 146/146 Regeln identisch) und `render_document` (`document.rs`): `<title>` (`wp_get_document_title`), `body_class`, Styles in WP-Reihenfolge (Block-CSS inline bis 40 KB wie `wp_maybe_inline_styles`, sonst `<link>`; Emoji-Styles, `wp-block-library`, Global Styles, Variationen, `core-block-supports`, Skip-Link, Theme-CSS), `@font-face`, Feed-/REST-/Canonical-Links, Script-Module mit Importmap aus `script-modules-packages.php`. `tests/golden_documents.rs` vergleicht Titel, Body-Klassen und die Style-/Script-IDs inkl. Reihenfolge für 6 URLs mit WP: identisch. Nicht unterstützt (PHP): `functions.php` des Themes (z. B. `register_block_style`, Block-Bindings). Offen: Inhalt der Head-Styles wird nur über die IDs verglichen, nicht byte-genau.
7. **Server**: Routing von Request zu Template zu HTML, dazu Statik, Feeds (RSS/Atom), `sitemap.xml` und die lesende REST-API (`/wp-json/wp/v2/posts|pages|categories|tags|media`).
8. **Compat-Tests**: Referenz-WordPress per Docker aufsetzen, Seiten vergleichen, Abweichungen dokumentieren.

Reihenfolge bis zum ersten sichtbaren Ergebnis: Schritte 1 bis 4, danach Schritt 5 bis 7 für eine einzelne Beitragsseite, dann in die Breite.

## Wiederverwendung
- `rustpress_phpser::{unserialize, serialize, Value, Key, Error}` für `option_value`, `meta_value` und `rewrite_rules`.
- Workspace-Einstellungen in `Cargo.toml` (`edition = 2024`, Lizenz GPL-2.0-or-later) werden von allen neuen Crates über `*.workspace = true` geerbt.

## Voraussetzungen, die noch zu installieren sind
- ✅ **PHP 8.4** (`winget install PHP.PHP.8.4`) für die Vergleichstests von `phpser` ist installiert.
- **Container:** Statt Docker Desktop wird **rootless Podman 5.4.2 in der vorhandenen Debian-WSL2** genutzt (`wsl -d Debian -- podman ...`). Damit laufen eine MariaDB als Test-DB und ein Referenz-WordPress. Docker ist dort installiert, aber der Daemon läuft nicht, und `sudo` verlangt ein Passwort. In der WSL liegen außerdem fremde Container (`frappe_docker-*`), die nicht angefasst werden.
- Eine Beispiel-WordPress-Instanz mit einem Block-Theme (z. B. Twenty Twenty-Five) und Testinhalten.

## Verification
- `cargo test --workspace` und `cargo clippy --workspace -- -D warnings` müssen grün sein.
- `phpser`: Proptest-Round-Trip plus Gegenprüfung mit `php -r` auf echten `wp_options`-Werten.
- Mit Test-DB: `cargo run -p rustpress-server`, dann `curl http://localhost:PORT/`, `/hello-world/` und `/wp-json/wp/v2/posts`. Das HTML wird mit der Ausgabe des Referenz-WordPress verglichen.
- Compat-Suite (`rustpress-compat`) meldet pro URL die normalisierte Abweichung. Ziel für Meilenstein 1: Startseite, Einzelbeitrag, Seite, Kategorie-Archiv und 404 sind identisch bis auf bekannte Ausnahmen.

## Offene Risiken
- Byte-genaue HTML-Gleichheit ist unrealistisch, weil WordPress-Filter (`wptexturize`, `wpautop`, Emoji-Skripte) viel verändern. Deshalb wird nach der Normalisierung verglichen und die Filter werden gezielt nachgebaut.
- Plugins, die Inhalte per Shortcode oder Filter erzeugen, fehlen in Meilenstein 1 und müssen einzeln als Rust-Module ergänzt werden.
