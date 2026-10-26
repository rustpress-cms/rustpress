# Builds the release binary and runs it like dev/serve-admin.ps1 (port 3001, writes allowed) in a hidden process,
# for measuring. dev/restart-admin.ps1 goes back to the debug build.
Get-Process rustpress-server -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 1
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
cargo build --release -q -p rustpress-server
if ($LASTEXITCODE -ne 0) { exit 1 }
$root = '\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data'
$local = Join-Path (Split-Path -Parent $repo) 'wp-local'
if (Test-Path (Join-Path $local 'wp-includes')) { $root = $local }
$env:RUSTPRESS_DATABASE_URL = 'mysql://wordpress:wordpress@127.0.0.1:3306/wordpress'
$env:RUSTPRESS_WP_ROOT = $root
$env:RUSTPRESS_LISTEN = '127.0.0.1:3001'
$env:RUSTPRESS_SITE_URL = 'http://localhost:3001'
$env:RUSTPRESS_ALLOW_WRITES = '1'
$env:RUSTPRESS_RELOAD_SECS = '1'
$env:RUSTPRESS_ENVIRONMENT_TYPE = 'local'
Start-Process -WindowStyle Hidden -FilePath "$repo\target\release\rustpress-server.exe"
for ($i = 0; $i -lt 60; $i++) {
    Start-Sleep 1
    $code = curl.exe -s -o NUL -w "%{http_code}" http://localhost:3001/wp-login.php
    if ($code -eq '200') { 'release server up'; exit 0 }
}
'server did not start'; exit 1
