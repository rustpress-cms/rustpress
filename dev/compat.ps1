# Compares RustPress with the reference WordPress for one or more themes and writes a report per
# theme to compat/reports/<theme>.md. Each theme is activated on the reference first (WP-CLI),
# and the original theme is restored at the end. Needs the stack from dev/up.sh and dev/seed.sh.
#
#   dev/compat.ps1                                   # twentytwentyfive and twentytwentyfour
#   dev/compat.ps1 -Themes twentytwentyfour          # one theme
param(
    [string[]]$Themes = @('twentytwentyfive', 'twentytwentyfour'),
    [string]$Restore = 'twentytwentyfive'
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$wsl = '/mnt/' + $repo.Substring(0, 1).ToLower() + $repo.Substring(2).Replace('\', '/')

function Invoke-Wp {
    wsl.exe -d Debian -- bash "$wsl/dev/wp.sh" @args
    if ($LASTEXITCODE -ne 0) { throw "wp $args failed" }
}

$failed = $false
try {
    foreach ($theme in $Themes) {
        Write-Host "== $theme"
        Invoke-Wp theme activate $theme | Out-Null
        Push-Location $repo
        try {
            cargo run -q -p rustpress-compat -- --report "compat/reports/$theme.md"
            if ($LASTEXITCODE -ne 0) { $failed = $true }
        } finally {
            Pop-Location
        }
    }
} finally {
    Invoke-Wp theme activate $Restore | Out-Null
}
if ($failed) { exit 1 }
