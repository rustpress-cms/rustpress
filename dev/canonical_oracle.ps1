# Regenerates crates/rustpress-core/tests/fixtures/canonical_expected.tsv from the
# real WordPress (see canonical_oracle.php.tpl). Needs the reference stack (dev/up.sh).
$fixtures = Join-Path (Split-Path $PSScriptRoot) 'crates\rustpress-core\tests\fixtures'
$urls = Get-Content (Join-Path $fixtures 'canonical_urls.txt') | Where-Object { $_ -ne '' }
$list = ($urls | ForEach-Object { "'" + $_.Replace('\', '\\').Replace("'", "\'") + "'" }) -join ",`n"
$php = (Get-Content (Join-Path $PSScriptRoot 'canonical_oracle.php.tpl') -Raw).Replace('@@URLS@@', $list)
$tmp = Join-Path $PSScriptRoot '_canonical_oracle.php'
Set-Content $tmp $php -Encoding utf8
function WslPath($p) { '/mnt/' + $p.Substring(0, 1).ToLower() + $p.Substring(2).Replace('\', '/') }
$out = wsl.exe -d Debian -- bash (WslPath (Join-Path $PSScriptRoot 'php.sh')) (WslPath $tmp)
Remove-Item $tmp
$rows = $out | Where-Object { $_ -match "`t" }
# Sitemaps, robots.txt and the favicon are answered by WordPress before (or instead of)
# redirect_canonical() in a real request, so ask the live site for those.
$rows = $rows | ForEach-Object {
    $url = ($_ -split "`t")[0]
    if ($url -match 'sitemap|robots|favicon') {
        $head = curl.exe -s -o NUL -D - "http://localhost:8080$url"
        $status = ($head | Select-Object -First 1) -replace '^HTTP/\S+\s+(\d+).*', '$1'
        $location = ($head | Where-Object { $_ -like 'Location:*' } | Select-Object -First 1)
        $target = if ($status -eq '301' -and $location) { ($location -replace '^Location:\s*http://localhost:8080', '').Trim() } else { '' }
        "$url`t$target"
    } else { $_ }
}
[IO.File]::WriteAllText((Join-Path $fixtures 'canonical_expected.tsv'), ($rows -join "`n") + "`n")
"wrote $($rows.Count) rows"


