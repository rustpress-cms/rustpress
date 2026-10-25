#!/usr/bin/env bash
# Runs SQL against a scratch database of the reference stack (from WSL): dev/sql.sh <database> <base64 of the statement>
sql=$(echo "$2" | base64 -d)
exec podman exec -i rustpress-db mariadb -uwordpress -pwordpress "$1" -N -B -e "$sql"
