# RustPress

Ein WordPress-Ersatz in Rust: Bestehende WordPress-Seiten laufen **ohne PHP** und **ohne Konvertierung der Datenbank**. RustPress liest die vorhandene MySQL/MariaDB (`wp_*`-Schema, nur lesend) und das vorhandene `wp-content/` und liefert dieselben Antworten wie WordPress.

Unterstützt werden **Block-Themes** (`theme.json` + HTML-Templates). Klassische PHP-Themes und PHP-Plugins laufen bewusst nicht; wichtige Plugins sollen später als native Module nachgebaut werden. Der aktuelle Stand und die Planung stehen in [PLAN.md](PLAN.md).

## Was funktioniert (Meilenstein 1, lesend)

- Seiten, Beiträge, Archive, Suche, 404 mit dem Block-Theme der Seite, verglichen mit WordPress Body für Body (inklusive Bildern: Beitragsbild, Logo, Bild-Blöcke, Galerien, `srcset` und Lazy-Loading)
- Canonical-Redirects (`/beitrag` → `/beitrag/`, `?p=5` → Permalink, …)
- Feeds (RSS2, Atom, Kommentar-Feeds), Sitemaps, `robots.txt`
- REST-API `wp/v2` zum Lesen (Beiträge, Seiten, Medien, Kategorien, Schlagwörter, Benutzer, Kommentare)
- Statische Dateien aus `wp-content/` und `wp-includes/`

## Starten

```sh
cargo run --release -p rustpress-server
```

Konfiguration über eine TOML-Datei (`RUSTPRESS_CONFIG=pfad.toml`) und/oder Umgebungsvariablen, die Variablen gewinnen:

```toml
database_url = "mysql://wordpress:geheim@127.0.0.1:3306/wordpress"
table_prefix = "wp_"
wp_root      = "/var/www/html"   # enthält wp-content/ und wp-includes/
listen       = "127.0.0.1:3000"
reload_secs  = 60                # Optionen und Theme werden so oft neu gelesen
```

`RUSTPRESS_DATABASE_URL`, `RUSTPRESS_TABLE_PREFIX`, `RUSTPRESS_WP_ROOT`, `RUSTPRESS_LISTEN`, `RUSTPRESS_RELOAD_SECS` überschreiben die Datei. Die Datenbankverbindung läuft in einer `READ ONLY`-Session; Schreibzugriffe (Kommentare, Login, Admin) gibt es nicht. TLS übernimmt ein vorgeschalteter Proxy.

## Entwickeln und testen

Die Tests vergleichen RustPress mit einem echten WordPress. Dafür gibt es einen Referenz-Stack (rootless Podman in WSL2; MariaDB auf `:3306`, WordPress 7 mit Twenty Twenty-Five auf `:8080`):

```sh
dev/up.sh     # Container starten
dev/seed.sh   # Testinhalte anlegen
cargo test --workspace
```

Ohne den Stack überspringen sich die Golden-Tests. Unter Windows muss Cargo in PowerShell laufen (Git-Bash's `link.exe` verdeckt den MSVC-Linker).

### Kompatibilitätslauf

`rustpress-compat` vergleicht RustPress mit dem Referenz-WordPress über viele URLs auf einmal (feste Proben, alles, was die Sitemaps und die REST-API der Referenz auflisten, die Dateien der Startseite) und schreibt einen Markdown-Bericht. Verglichen werden Status, Header und je nach Art Body (HTML-Dokumente: Titel, Body-Klassen, jedes Element des Heads, Body, Footer, zuletzt Byte für Byte), Feeds, Sitemaps und REST-Antworten.

```sh
cargo run -p rustpress-compat -- --report compat/reports/twentytwentyfive.md
dev/compat.ps1                      # mehrere Themes nacheinander auf der Referenz aktivieren
cargo run -p rustpress-compat -- --dump /ueber-uns/   # beide Antworten nach compat/dump/ schreiben
```

Abweichungen, die verstanden und bewusst akzeptiert sind, stehen mit Begründung in `compat/known.toml`; nur andere Abweichungen lassen den Lauf fehlschlagen. Die Berichte liegen unter `compat/reports/`.

## Crates

| Crate | Aufgabe |
|---|---|
| `rustpress-phpser` | PHP-`serialize`/`unserialize`, byte-genau |
| `rustpress-db` | Lesezugriffe auf das WordPress-Schema (sqlx) |
| `rustpress-core` | Optionen, Rewrite-Regeln, Hauptabfrage, Template-Hierarchie, Canonical, Sitemaps |
| `rustpress-blocks` | Block-Parser und -Renderer, Block-Supports, Style-Engine |
| `rustpress-theme` | `theme.json`, Templates, Global Styles, Dokument und Feeds |
| `rustpress-rest` | REST-API (lesend) |
| `rustpress-server` | axum-Server |
| `rustpress-compat` | Vergleichslauf gegen ein echtes WordPress mit Bericht |

Lizenz: GPL-2.0-or-later.
