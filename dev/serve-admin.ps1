# Starts RustPress on port 3001 against the reference database with writing allowed, serving the
# WordPress files of the reference (dev only: the reference has throw-away users and a scratch database).
#   dev/serve-admin.ps1            # builds and runs in the foreground
$root = '\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data'
$env:RUSTPRESS_DATABASE_URL = 'mysql://wordpress:wordpress@127.0.0.1:3306/wordpress'
$env:RUSTPRESS_WP_ROOT = $root
$env:RUSTPRESS_LISTEN = '127.0.0.1:3001'
$env:RUSTPRESS_ALLOW_WRITES = '1'
$env:RUSTPRESS_RELOAD_SECS = '1'
Set-Location (Split-Path -Parent $PSScriptRoot)
cargo run -q -p rustpress-server
