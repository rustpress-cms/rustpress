# Starts RustPress on port 3002 against the copy of a real site that dev/up-soli.sh serves on port 8081, with the
# WordPress files of that reference, for dev/seventeen-compare.py.
#   dev/serve-soli.ps1          # expects a built target\golden\debug\rustpress-server.exe
$repo = Split-Path -Parent $PSScriptRoot
Get-Process rustpress-server -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*golden*' } | Stop-Process -Force -ErrorAction SilentlyContinue
$env:RUSTPRESS_DATABASE_URL = 'mysql://wordpress:wordpress@127.0.0.1:3306/soli_ref'
$env:RUSTPRESS_WP_ROOT = '\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp-soli\_data'
$env:RUSTPRESS_LISTEN = '127.0.0.1:3002'
$env:RUSTPRESS_SITE_URL = 'http://localhost:3002'
$env:RUSTPRESS_RELOAD_SECS = '1'
# Signing in stores a session; this is a scratch copy.
$env:RUSTPRESS_ALLOW_WRITES = '1'
$env:RUSTPRESS_MAX_UPLOAD_SIZE = '2097152'
Start-Process "$repo\target\golden\debug\rustpress-server.exe" -WindowStyle Hidden
for ($i = 0; $i -lt 60; $i++) {
    Start-Sleep 2
    if ((curl.exe -s -o NUL -w "%{http_code}" http://localhost:3002/) -eq '200') { 'server up'; exit 0 }
}
'server did not start'; exit 1
