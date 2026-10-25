# Compares RustPress with the reference WordPress under several permalink structures and writes a
# report per structure to compat/reports/twentytwentyfive-perma-<n>.md. The structure of the
# reference is restored (and the rules flushed) at the end. Needs the stack from dev/up.sh.
#
#   dev/compat-permalinks.ps1                    # all structures below
#   dev/compat-permalinks.ps1 -Only 3,4          # some of them (1-based)
param(
    [int[]]$Only = @(),
    [string]$Restore = '/%postname%/'
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$wsl = '/mnt/' + $repo.Substring(0, 1).ToLower() + $repo.Substring(2).Replace('\', '/')

function Invoke-Wp {
    wsl.exe -d Debian -- bash "$wsl/dev/wp.sh" @args
    if ($LASTEXITCODE -ne 0) { throw "wp $args failed" }
}

# (structure, category_base, tag_base)
$cases = @(
    @('/%year%/%monthnum%/%day%/%postname%/', '', ''),
    @('/%year%/%monthnum%/%postname%/', 'themen', 'schlagwort'),
    @('/archives/%post_id%', '', ''),
    @('/%category%/%postname%/', '', ''),
    @('/%postname%', '', ''),
    @('/index.php/%postname%/', '', ''),
    @('/blog/%postname%/', '', ''),
    @('/%author%/%postname%.html', '', '')
)

$failed = $false
try {
    for ($i = 0; $i -lt $cases.Count; $i++) {
        $n = $i + 1
        if ($Only.Count -gt 0 -and $Only -notcontains $n) { continue }
        $structure, $category, $tag = $cases[$i]
        Write-Host "== $n  $structure  category=$category tag=$tag"
        Invoke-Wp option update category_base $category | Out-Null
        Invoke-Wp option update tag_base $tag | Out-Null
        Invoke-Wp rewrite structure $structure --hard | Out-Null
        Invoke-Wp rewrite flush --hard | Out-Null
        Push-Location $repo
        try {
            cargo run -q -p rustpress-compat -- --report "compat/reports/twentytwentyfive-perma-$n.md"
            if ($LASTEXITCODE -ne 0) { $failed = $true }
        } finally {
            Pop-Location
        }
    }
} finally {
    Invoke-Wp option update category_base '' | Out-Null
    Invoke-Wp option update tag_base '' | Out-Null
    Invoke-Wp rewrite structure $Restore --hard | Out-Null
    Invoke-Wp rewrite flush --hard | Out-Null
}
if ($failed) { exit 1 }
