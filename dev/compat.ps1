# Compares RustPress with the reference WordPress for one or more themes and writes a report per
# theme to compat/reports/<theme>.md. Each theme is activated on the reference first (WP-CLI),
# and the original theme is restored at the end. Needs the stack from dev/up.sh and dev/seed.sh.
#
#   dev/compat.ps1                                   # twentytwentyfive and twentytwentyfour
#   dev/compat.ps1 -Themes twentytwentyfour          # one theme
#   dev/compat.ps1 -Locale de_DE                     # the reference in German (language packs are installed)
#
# With -Locale the site language of the reference is switched for the run and set back to en_US
# afterwards; the reports are named <theme>-<locale>.md.
param(
    [string[]]$Themes = @('twentytwentyfive', 'twentytwentyfour'),
    [string]$Restore = 'twentytwentyfive',
    [string]$Locale = ''
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$wsl = '/mnt/' + $repo.Substring(0, 1).ToLower() + $repo.Substring(2).Replace('\', '/')

function Invoke-Wp {
    wsl.exe -d Debian -- bash "$wsl/dev/wp.sh" @args
    if ($LASTEXITCODE -ne 0) { throw "wp $args failed" }
}

$failed = $false
$suffix = ''
if ($Locale) {
    $suffix = "-$Locale"
    Invoke-Wp language core install $Locale | Out-Null
}
try {
    if ($Locale) { Invoke-Wp site switch-language $Locale | Out-Null }
    foreach ($theme in $Themes) {
        Write-Host "== $theme $Locale"
        Invoke-Wp theme activate $theme | Out-Null
        if ($Locale) { Invoke-Wp language theme install $theme $Locale | Out-Null }
        Push-Location $repo
        try {
            cargo run -q -p rustpress-compat -- --report "compat/reports/$theme$suffix.md"
            if ($LASTEXITCODE -ne 0) { $failed = $true }
        } finally {
            Pop-Location
        }
    }
} finally {
    Invoke-Wp theme activate $Restore | Out-Null
    if ($Locale) { Invoke-Wp site switch-language en_US | Out-Null }
}
if ($failed) { exit 1 }
