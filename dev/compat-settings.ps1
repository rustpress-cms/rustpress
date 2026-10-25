# Compares RustPress with the reference WordPress under several site settings and writes a report
# per case to compat/reports/twentytwentyfive-settings-<n>.md. The options are restored afterwards.
# Needs the stack from dev/up.sh and dev/seed.sh (page IDs: 2 = Sample Page, 20 = Ueber uns).
#
#   dev/compat-settings.ps1 -Only 1     # one case (1-based); run one case per call
param(
    [Parameter(Mandatory = $true)][int]$Only
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$wsl = '/mnt/' + $repo.Substring(0, 1).ToLower() + $repo.Substring(2).Replace('\', '/')

function Invoke-Wp {
    $out = wsl.exe -d Debian -- bash "$wsl/dev/wp.sh" @args
    if ($LASTEXITCODE -ne 0) { throw "wp $args failed" }
    $out
}

$cases = @(
    @{ name = 'static front page'; options = [ordered]@{ show_on_front = 'page'; page_on_front = '2'; page_for_posts = '20' } },
    @{ name = 'three posts per page'; options = [ordered]@{ posts_per_page = '3'; posts_per_rss = '3' } },
    @{ name = 'Berlin time, German formats'; options = [ordered]@{ timezone_string = 'Europe/Berlin'; date_format = 'd.m.Y'; time_format = 'H:i'; start_of_week = '1' } },
    @{ name = 'odd site title'; options = [ordered]@{ blogname = 'Test "Name" & <b>Co</b>'; blogdescription = 'Ein Slogan & mehr' } },
    @{ name = 'search engines discouraged'; options = [ordered]@{ blog_public = '0' } },
    @{ name = 'comment settings'; options = [ordered]@{ thread_comments = '0'; page_comments = '1'; comments_per_page = '2'; default_comments_page = 'oldest'; comment_order = 'desc'; show_avatars = '0' } },
    @{ name = 'other default category, no smilies'; options = [ordered]@{ default_category = '2'; use_smilies = '0' } },
    @{ name = 'UTC offset'; options = [ordered]@{ timezone_string = ''; gmt_offset = '5.5' } }
)

$case = $cases[$Only - 1]
Write-Host "== $Only  $($case.name)"
$saved = [ordered]@{}
foreach ($key in $case.options.Keys) { $saved[$key] = (Invoke-Wp option get $key) -join '' }
$failed = $false
try {
    foreach ($key in $case.options.Keys) { Invoke-Wp option update $key $case.options[$key] | Out-Null }
    Push-Location $repo
    try {
        cargo run -q -p rustpress-compat -- --report "compat/reports/twentytwentyfive-settings-$Only.md"
        if ($LASTEXITCODE -ne 0) { $failed = $true }
    } finally {
        Pop-Location
    }
} finally {
    foreach ($key in $saved.Keys) { Invoke-Wp option update $key $saved[$key] | Out-Null }
}
if ($failed) { exit 1 }
