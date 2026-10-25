# Stops a running dev server, builds and starts it in a hidden process and waits until it answers.
Get-Process rustpress-server -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 1
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
cargo build -q -p rustpress-server
if ($LASTEXITCODE -ne 0) { exit 1 }
Start-Process pwsh -ArgumentList '-NoProfile', '-File', "$repo\dev\serve-admin.ps1" -WindowStyle Hidden
for ($i = 0; $i -lt 60; $i++) {
    Start-Sleep 2
    $code = curl.exe -s -o NUL -w "%{http_code}" http://localhost:3001/wp-login.php
    if ($code -eq '200') { 'server up'; exit 0 }
}
'server did not start'; exit 1
