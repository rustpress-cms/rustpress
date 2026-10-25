# From Windows: dev/sql.ps1 <database> "<statement>"  (runs in the reference MariaDB)
param([string]$Database, [string]$Statement)
$b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Statement))
wsl.exe -d Debian -- bash /mnt/c/work/experimental/rustpress/dev/sql.sh $Database $b64
