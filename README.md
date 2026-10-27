# RustPress

Ein WordPress-Ersatz in Rust: Bestehende WordPress-Seiten laufen **ohne PHP** und **ohne Konvertierung der Datenbank**. RustPress liest die vorhandene MySQL/MariaDB (`wp_*`-Schema, nur lesend) und das vorhandene `wp-content/` und liefert dieselben Antworten wie WordPress.

Unterstützt werden **Block-Themes** (`theme.json` + HTML-Templates) und als Rust-Nachbau das klassische Theme **Twenty Seventeen**. Andere klassische PHP-Themes und PHP-Plugins laufen bewusst nicht; wichtige Plugins sollen später als native Module nachgebaut werden. Der aktuelle Stand und die Planung stehen in [PLAN.md](PLAN.md).

## Was funktioniert (Meilenstein 1, lesend)

- Seiten, Beiträge, Archive, Suche, 404 mit dem Block-Theme der Seite, verglichen mit WordPress Body für Body (inklusive Bildern: Beitragsbild, Logo, Bild-Blöcke, Galerien, `srcset` und Lazy-Loading)
- Canonical-Redirects (`/beitrag` → `/beitrag/`, `?p=5` → Permalink, …)
- Feeds (RSS2, Atom, Kommentar-Feeds), Sitemaps, `robots.txt`
- REST-API `wp/v2` zum Lesen (Beiträge, Seiten, Medien, Kategorien, Schlagwörter, Benutzer, Kommentare)
- Statische Dateien aus `wp-content/` und `wp-includes/`
- Alle Sprachen, für die WordPress ein Sprachpaket hat (siehe [Sprachen](#sprachen))

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

## Mit Docker betreiben

Das `Dockerfile` baut ein kleines Alpine-Image mit einem statisch gelinkten `rustpress`; `compose.yaml` startet es zusammen mit MariaDB. Für eine bestehende WordPress-Seite zeigt `WORDPRESS_DIR` in `.env` auf den WordPress-Ordner (mit `wp-config.php`) und der Datenbank-Dump wird in den `db`-Container importiert. Eine neue Seite richtet WP-CLI ein, weil RustPress noch keinen eigenen Installer hat:

```sh
cp .env.example .env          # Passwort und Adresse eintragen
docker compose up -d          # legt ./wordpress mit den WordPress-Dateien an
docker compose run --rm wp config create --dbname=wordpress --dbuser=wordpress --dbpass=<DB_PASSWORD> --dbhost=db
docker compose run --rm wp core install --url=<SITE_URL> --title="Meine Seite" --admin_user=admin --admin_email=ich@example.com --prompt=admin_password
docker compose restart rustpress
```

RustPress lauscht auf Port 3000; TLS übernimmt ein vorgeschalteter Proxy.

## Sprachen

RustPress kennt keine Sprache im Code. Texte, Monats- und Wochentagsnamen, Anführungszeichen, Zahlenformat, Textrichtung, Pluralregeln und Wortzählung kommen aus denselben Sprachpaketen wie bei WordPress (`wp-content/languages/<locale>.mo`, `languages/themes/<theme>-<locale>.mo`). Eine weitere Sprache braucht deshalb nur ihr Sprachpaket (zum Beispiel mit `wp language core install <locale>` oder aus translate.wordpress.org); die Sprache der Seite ist die Option `WPLANG`. Rechts-nach-links-Sprachen erhalten `dir="rtl"`, die Body-Klasse `rtl` und die `-rtl`-Stylesheets. Das Konzept steht in [PLAN.md](PLAN.md#lokalisierungskonzept).

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
dev/compat.ps1                      # TT25, TT24 und TT23 nacheinander auf der Referenz aktivieren und vergleichen
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
