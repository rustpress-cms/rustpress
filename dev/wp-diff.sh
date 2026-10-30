#!/usr/bin/env bash
# What changed between two WordPress releases, and where RustPress mirrors it. Writes a Markdown
# report (also the body of the issue that .github/workflows/wordpress.yml opens) and the full diff
# of the PHP code next to it.
#
#   dev/wp-diff.sh 7.1.3 7.1.4                    # report on stdout, diff in $WP_CACHE/7.1.3-7.1.4.diff
#   dev/wp-diff.sh 7.1.3 7.1.4 report.md          # report into a file, diff into report.diff
#
# The release packages are downloaded once into $WP_CACHE (default ~/.cache/rustpress-wp).
# Needs curl, unzip and git.
set -euo pipefail

old=${1:?old version}; new=${2:?new version}; out=${3:-}
for v in "$old" "$new"; do
  [[ $v =~ ^[0-9]+(\.[0-9]+){1,2}$ ]] || { echo "not a version: $v" >&2; exit 2; }
done
repo=$(cd "$(dirname "$0")/.." && pwd)
cache=${WP_CACHE:-$HOME/.cache/rustpress-wp}
mkdir -p "$cache"

fetch() { # version -> unpacked tree in $cache/<version>/wordpress
  local v=$1
  [[ -f $cache/$v/wordpress/wp-includes/version.php ]] && return
  curl -fsSL -o "$cache/wordpress-$v.zip" "https://downloads.wordpress.org/release/wordpress-$v.zip"
  rm -rf "$cache/$v" && mkdir -p "$cache/$v"
  unzip -q "$cache/wordpress-$v.zip" -d "$cache/$v"
}
fetch "$old"; fetch "$new"

if [[ -n $out ]]; then diff_file=${out%.md}.diff; else diff_file=$cache/$old-$new.diff; fi
# Git's PHP driver puts the enclosing function into each hunk header.
attributes=$(mktemp); trap 'rm -f "$attributes"' EXIT
echo '*.php diff=php' > "$attributes"
gitdiff() { (cd "$cache" && git -c core.attributesFile="$attributes" diff --no-index --no-color "$@" "$old/wordpress" "$new/wordpress") || true; }

changes=$(gitdiff --name-status | sed "s#$old/wordpress/##; s# $new/wordpress/# #; s#\t$new/wordpress/#\t#")
gitdiff -U3 > "$diff_file.all"
# Only PHP goes into the diff file: the assets are served from disk and need no porting.
awk '/^diff --git /{keep = ($0 ~ /\.php$/)} keep' "$diff_file.all" > "$diff_file"
rm -f "$diff_file.all"

php=$(grep -E '\.php$' <<<"$changes" || true)
json=$(grep -E '\.json$' <<<"$changes" || true)
assets=$(grep -vE '\.(php|json)$' <<<"$changes" | grep -v '^$' || true)
count() { [[ -z $1 ]] && echo 0 || wc -l <<<"$1" | tr -d ' '; }

# Per changed PHP file: the functions and methods whose bodies changed (hunk headers and definitions in changed lines).
functions=$(awk '
  /^diff --git / { file = $NF; sub(/^b\/[^\/]+\/wordpress\//, "", file); next }
  /^@@ / && match($0, /function &?[A-Za-z_][A-Za-z0-9_]*/) { print file "\t" substr($0, RSTART + 9, RLENGTH - 9); next }
  /^[+-][^+-]/ && match($0, /function &?[A-Za-z_][A-Za-z0-9_]*/) { print file "\t" substr($0, RSTART + 9, RLENGTH - 9) }
' "$diff_file" | tr -d '&' | sort -u)

# Where RustPress follows a PHP file: its class (class-wp-block-parser.php -> WP_Block_Parser), its block
# (blocks/categories.php -> "core/categories"), its file name, and the changed functions as whole words.
mirrors() { # php path, function names...
  local file=$1 base; shift
  base=$(basename "$file" .php)
  {
    if [[ $base == class-* ]]; then
      local class; class=$(sed 's/^class-//; s/-/_/g' <<<"$base" | sed -E 's/(^|_)([a-z])/\1\u\2/g; s/^Wp_/WP_/; s/_Rest_/_REST_/; s/_Ms_/_MS_/')
      grep -rlF "$class" crates --include=*.rs
    fi
    [[ $file == wp-includes/blocks/* ]] && grep -rlF "\"core/$base\"" crates --include=*.rs
    grep -rlF "$base.php" crates --include=*.rs
    for fn in "$@"; do
      [[ $fn == *_* && ${#fn} -gt 8 ]] && grep -rlw "$fn" crates --include=*.rs
    done
  } 2>/dev/null | sort -u | head -6 | sed 's/.*/`&`/' | paste -sd ' ' -
}

{
  echo "# WordPress $old → $new"
  echo
  echo "Release notes: https://wordpress.org/documentation/wordpress-version/version-${new//./-}/"
  echo
  echo "| | Files |"
  echo "|---|---|"
  echo "| PHP (logic RustPress may mirror) | $(count "$php") |"
  echo "| JSON (block and theme definitions read at runtime) | $(count "$json") |"
  echo "| Other (styles, scripts, images; served from disk) | $(count "$assets") |"
  echo
  echo "## Changed PHP code"
  echo
  echo "Files where RustPress names the class, block, file or a changed function are listed after the arrow; those need a look."
  echo "Without an arrow nothing was found (often code RustPress does not have, such as multisite or the Customizer)."
  echo
  while read -r line; do
    [[ -z $line ]] && continue
    file=${line#*$'\t'}
    [[ $file == wp-includes/version.php || $file == *.asset.php || $file == *-packages.php ]] && continue
    mapfile -t fns < <(awk -F'\t' -v f="$file" '$1 == f { print $2 }' <<<"$functions")
    hits=$(cd "$repo" && mirrors "$file" "${fns[@]}" || true)
    names=$( ((${#fns[@]})) && printf '`%s()` ' "${fns[@]}" || true)
    echo "- \`$file\`${names:+ (${names% })}${hits:+ → $hits}"
  done <<<"$php"
  echo
  echo "## Changed files"
  echo
  for group in php json assets; do
    list=${!group}
    [[ -z $list ]] && continue
    echo "<details><summary>$group ($(count "$list"))</summary>"
    echo
    sed 's/^\([A-Z]\)[0-9]*\t\(.*\)$/- \1 `\2`/; s/\t/ → /' <<<"$list" | head -300
    echo
    echo "</details>"
    echo
  done
  cat <<EOF
## Following the release

- [ ] Port the PHP changes above (full diff: \`$(basename "$diff_file")\`, \`dev/wp-diff.sh $old $new\`).
- [ ] \`dev/wp-bump.sh $new\` in WSL: sets \`WORDPRESS_VERSION\` and the READMEs and brings the reference WordPress to $new.
- [ ] Bake the data again from the reference and review the diff: \`dev/bake-editor.py\`, \`dev/bake-media.py\`, \`dev/bake-pages.py\`, \`dev/gen-*.py\`, \`dev/kses-golden.py\`, \`dev/kses-post-golden.py\`.
- [ ] \`cargo test --workspace\` with the reference stack (golden tests), \`dev/compat.ps1\` and the compare scripts of the touched areas.
- [ ] Pull request, then tag a release.
EOF
} > "${out:-/dev/stdout}"
