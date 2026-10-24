#!/usr/bin/env bash
# Imports a mysqldump file into a fresh database of the reference MariaDB (dev/up.sh) and lets the
# `wordpress` user read it. Usage: dev/import-dump.sh <file.sql> <database>
# The database is dropped and created again; the dump never touches any other database.
set -euo pipefail
file=$1; db=$2
case "$db" in (*[!a-zA-Z0-9_]*|"") echo "invalid database name" >&2; exit 2;; esac
sql() { podman exec -i rustpress-db mariadb -uroot -prustpress "$@"; }
sql -e "DROP DATABASE IF EXISTS \`$db\`; CREATE DATABASE \`$db\` CHARACTER SET utf8mb4; GRANT SELECT ON \`$db\`.* TO 'wordpress'@'%';"
sql "$db" < "$file"
echo "imported $file into $db"