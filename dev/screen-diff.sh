#!/usr/bin/env bash
# Prints the first differences of one screen (see dev/admin-compare.sh): dev/screen-diff.sh "edit.php?post_type=book" [lines]
cd "$(dirname "$0")/.."
bash dev/admin-compare.sh "$1" > /dev/null 2>&1
safe=$(printf %s "$1" | tr '?&=/' '____')
diff "${TEMP:-/tmp}/ac_ref_$safe.html" "${TEMP:-/tmp}/ac_rp_$safe.html" | head -"${2:-20}" | cut -c1-300
