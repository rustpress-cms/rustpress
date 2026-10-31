# RustPress

[English](README.md) · **Deutsch**

Ein WordPress-Ersatz in Rust: Bestehende WordPress-Seiten laufen **ohne PHP** und **ohne Konvertierung der Datenbank**. RustPress liest die vorhandene MySQL/MariaDB (`wp_*`-Schema) und das vorhandene `wp-content/` und liefert dieselben Antworten wie WordPress. Mit `allow_writes` laufen auch Anmeldung, Kommentare und die Verwaltung (`wp-admin`).

Unterstützt werden **Block-Themes** (`theme.json` + HTML-Templates) und als Rust-Nachbau das klassische Theme **Twenty Seventeen**. Andere klassische PHP-Themes und PHP-Plugins laufen bewusst nicht; wichtige Plugins werden als native Module nachgebaut, die übernehmen, solange das Plugin aktiv ist (seine Dateien bleiben in `wp-content/plugins`): bisher **WP Date Remover** 1.1.3 und **SVG Support** 2.6.1 (Webseite, Einstellungen und bereinigte SVG-Uploads). Der aktuelle Stand und die Planung stehen in [PLAN.md](PLAN.md).

## Was funktioniert

- Seiten, Beiträge, Archive, Suche, 404 mit dem Block-Theme der Seite, verglichen mit WordPress Body für Body (inklusive Bildern: Beitragsbild, Logo, Bild-Blöcke, Galerien, `srcset` und Lazy-Loading)
- Canonical-Redirects (`/beitrag` → `/beitrag/`, `?p=5` → Permalink, …)
- Feeds (RSS2, Atom, Kommentar-Feeds), Sitemaps, `robots.txt`
- REST-API `wp/v2` (Beiträge, Seiten, Medien, Kategorien, Schlagwörter, Benutzer, Kommentare), mit `allow_writes` auch schreibend
- Verwaltung (`wp-admin`): Anmeldung, Beiträge und Seiten mit dem Block-Editor, Medien, Kommentare, Benutzer, Einstellungen, Datenschutz, Export
- Installer wie WordPress (`setup-config.php`, `install.php`), lädt auf Wunsch die WordPress-Dateien herunter
- Shared Hosting als CGI-Programm (statisch gelinktes Binary, Konfiguration aus `wp-config.php`)
- E-Mail per SMTP (`RUSTPRESS_SMTP_URL`)
- Statische Dateien aus `wp-content/` und `wp-includes/`
- Alle Sprachen, für die WordPress ein Sprachpaket hat (siehe [Sprachen](#sprachen))

## Starten

Jedes [Release](https://github.com/rustpress-cms/rustpress/releases) enthält fertige Builds: statische Linux-Binaries (x86_64, aarch64, armv7, armv6, i686), auch als `.deb`, `.rpm` und `.apk`, dazu macOS (Intel und Apple Silicon), Windows (x64, x86, ARM64) und FreeBSD. Das Container-Image ist `ghcr.io/rustpress-cms/rustpress` für amd64, arm64, arm/v7, arm/v6 und 386. Aus den Quellen:

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

`RUSTPRESS_DATABASE_URL`, `RUSTPRESS_TABLE_PREFIX`, `RUSTPRESS_WP_ROOT`, `RUSTPRESS_LISTEN`, `RUSTPRESS_RELOAD_SECS` überschreiben die Datei. Ohne `allow_writes = true` (`RUSTPRESS_ALLOW_WRITES=1`) läuft die Datenbankverbindung in einer `READ ONLY`-Session, und Anmeldung und Verwaltung bleiben geschlossen. TLS übernimmt ein vorgeschalteter Proxy.

## Mit Docker betreiben

Das `Dockerfile` baut ein kleines Alpine-Image mit einem statisch gelinkten `rustpress`; `compose.yaml` startet es zusammen mit MariaDB. Für eine bestehende WordPress-Seite zeigt `WORDPRESS_DIR` in `.env` auf den WordPress-Ordner (mit `wp-config.php`) und der Datenbank-Dump wird in den `db`-Container importiert. Eine neue Seite lässt sich mit dem eingebauten Installer im Browser einrichten oder mit WP-CLI:

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

Wie am Projekt gearbeitet wird (WordPress als Orakel, Referenz-Stack, Golden-Tests, native Plugin-Module), steht für Mitwirkende und Coding-Agents in [AGENTS.md](AGENTS.md) (englisch).

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

### Neuen WordPress-Versionen folgen

`WORDPRESS_VERSION` nennt die Version, der RustPress folgt; der Installer lädt sie herunter, und `dev/up.sh` hält das Referenz-WordPress auf ihr (automatische Updates aus). Ein täglicher Workflow eröffnet für jede neuere Version ein Issue mit dem geänderten PHP-Code, den RustPress-Dateien, die ihm folgen, und den Schritten für den Umstieg:

```sh
dev/wp-diff.sh 7.1.3 7.1.4    # derselbe Bericht lokal, dazu der vollständige PHP-Diff
dev/wp-bump.sh 7.1.4          # neue Version setzen und das Referenz-WordPress darauf bringen
```

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

## Lizenz

RustPress steht unter der GNU General Public License, Version 2 oder (nach deiner Wahl) jeder späteren Version (GPL-2.0-or-later), siehe [LICENSE](LICENSE).

RustPress ist ein Nachbau von WordPress in Rust. Texte, Vorlagen, Daten und der Nachbau von Twenty Seventeen stammen aus WordPress, © die WordPress-Mitwirkenden, ebenfalls unter GPLv2 oder später.

RustPress ist ein unabhängiges Projekt und steht in keiner Verbindung zur WordPress Foundation oder zu Automattic. WordPress ist eine eingetragene Marke der WordPress Foundation.
