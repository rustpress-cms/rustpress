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
[IO.File]::WriteAllText((Join-Path $fixtures 'canonical_expected.tsv'), ($rows -join "`n") + "`n")
"wrote $($rows.Count) rows"

