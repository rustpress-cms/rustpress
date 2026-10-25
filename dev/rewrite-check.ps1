# Checks the rewrite rules RustPress generates (crates/rustpress-core/src/rewrite_gen.rs) against the
# ones WordPress stores, for several permalink structures. The structure of the reference is restored
# at the end. Needs the stack from dev/up.sh.
#
#   dev/rewrite-check.ps1                # all structures below
#   dev/rewrite-check.ps1 -Only 3,4      # some of them (1-based)
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
    @('/blog/%postname%/', 'ordner', ''),
    @('/%author%/%postname%.html', '', ''),
    @('/%year%/%monthnum%/%day%/%hour%/%minute%/%second%/', '', ''),
    @('/%post_id%/%postname%/', '', ''),
    @('', '', '')
)

# The rules the types are read from: the ones of the structure the reference has now.
$base = Join-Path $env:TEMP 'rewrite_base_rules.txt'
$structure = (Invoke-Wp option get permalink_structure | Out-String).Trim()
$rules = & "$PSScriptRoot/sql.ps1" wordpress "select option_value from wp_options where option_name='rewrite_rules'"
[System.IO.File]::WriteAllText($base, ($rules | Out-String).Trim().Replace('\\', '\'), [System.Text.UTF8Encoding]::new($false))
$frontOf = { param($s) $i = $s.IndexOf('%'); if ($i -lt 0) { '' } else { $s.Substring(0, $i) } }
$env:RUSTPRESS_BASE_RULES = $base
$env:RUSTPRESS_OLD_FRONT = & $frontOf $structure

$failed = $false
try {
    for ($i = 0; $i -lt $cases.Count; $i++) {
        $n = $i + 1
        if ($Only.Count -gt 0 -and $Only -notcontains $n) { continue }
        $s, $category, $tag = $cases[$i]
        Write-Host "== $n  $s  category=$category tag=$tag"
        Invoke-Wp option update category_base $category | Out-Null
        Invoke-Wp option update tag_base $tag | Out-Null
        Invoke-Wp rewrite structure $s --hard | Out-Null
        Invoke-Wp rewrite flush --hard | Out-Null
        Push-Location $repo
        try {
            $ErrorActionPreference = 'Continue'
            cargo test -q -p rustpress-core --test rewrite_gen_live 2>&1 | Select-Object -Last 12
            if ($LASTEXITCODE -ne 0) { $failed = $true }
            $ErrorActionPreference = 'Stop'
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
