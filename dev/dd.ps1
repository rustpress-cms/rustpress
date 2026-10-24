# Dumps RustPress and the reference WordPress of the real site (dev/up-soli.sh) for one path and
# prints the first differences line by line.  Usage: dev/dd.ps1 -Path /   (-Main: the block-theme stack on :8080)
param([string]$Path = '/', [int]$Shown = 3, [string]$Reference = '127.0.0.1:8081', [switch]$Main)
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
$root = '\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp-soli\_data'
if ($Main) { cargo run -q -p rustpress-compat -- --dump $Path 2>&1 | Select-String 'RustPress \d+|error' } else {
cargo run -q -p rustpress-compat -- --reference $Reference --host localhost:8081 --db mysql://wordpress:wordpress@127.0.0.1:3306/soli_ref --wp-root $root --dump $Path 2>&1 | Select-String 'RustPress \d+|error' }
$a = [IO.File]::ReadAllLines("$repo\compat\dump\ours.txt"); $b = [IO.File]::ReadAllLines("$repo\compat\dump\theirs.txt")
"ours $($a.Count) lines, theirs $($b.Count)"
$i = 0; $j = 0; $count = 0
while ($i -lt $a.Count -and $j -lt $b.Count -and $count -lt $Shown) {
  if ($a[$i] -ceq $b[$j]) { $i++; $j++; continue }
  "--- ours line $($i+1) / theirs line $($j+1)"
  $x1 = $a[$i]; $y1 = $b[$j]; $n = [Math]::Min($x1.Length, $y1.Length); $c = 0
  while ($c -lt $n -and $x1[$c] -ceq $y1[$c]) { $c++ }
  if ($c -lt $n -or $x1.Length -ne $y1.Length) {
    $s = [Math]::Max(0, $c - 70)
    '  @' + $c + ' O: …' + $x1.Substring($s, [Math]::Min(220, $x1.Length - $s))
    '  @' + $c + ' T: …' + $y1.Substring($s, [Math]::Min(220, $y1.Length - $s))
  }
  for ($k = 0; $k -lt 3; $k++) { if ($i + $k -lt $a.Count) { '  O: ' + $a[$i + $k].Substring(0, [Math]::Min(260, $a[$i + $k].Length)) } }
  for ($k = 0; $k -lt 3; $k++) { if ($j + $k -lt $b.Count) { '  T: ' + $b[$j + $k].Substring(0, [Math]::Min(260, $b[$j + $k].Length)) } }
  $count++
  $found = $false
  for ($d = 1; $d -lt 60 -and -not $found; $d++) {
    for ($e = 0; $e -le $d; $e++) {
      $x = $i + $e; $y = $j + ($d - $e)
      if ($x -lt $a.Count -and $y -lt $b.Count -and $a[$x] -ceq $b[$y] -and $a[$x].Length -gt 8) { $i = $x; $j = $y; $found = $true; break }
    }
  }
  if (-not $found) { break }
}
