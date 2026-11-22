# Prints the Windows path of the WordPress files of a reference: RUSTPRESS_WP_ROOT when it is set (for the main
# reference only), else the Podman volume in WSL.   dev/wp-root.ps1 [volume]
param([string]$Volume = 'rustpress-wp')
if ($Volume -eq 'rustpress-wp' -and $env:RUSTPRESS_WP_ROOT) { $env:RUSTPRESS_WP_ROOT; exit 0 }
$mount = "$(wsl.exe -d Debian -- podman volume inspect $Volume -f '{{.Mountpoint}}')".Trim()
if (-not $mount) { Write-Error "no volume $Volume in WSL (dev/up.sh creates it)"; exit 1 }
'\\wsl.localhost\Debian' + $mount.Replace('/', '\')
