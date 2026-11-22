# From Windows: dev/sql.ps1 <database> "<statement>"  (runs in the reference MariaDB)
param([string]$Database, [string]$Statement)
$b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Statement))
function WslPath($p) { '/mnt/' + $p.Substring(0, 1).ToLower() + $p.Substring(2).Replace('\', '/') }
wsl.exe -d Debian -- bash (WslPath (Join-Path $PSScriptRoot 'sql.sh')) $Database $b64
