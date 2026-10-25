# Screenshot of a wp-admin page of the dev server (:3001) or the reference (:8080, -Ref) with headless Edge, signed
# in as rpadmin (cookies are set through the DevTools protocol).
#   dev/shot.ps1 -Page options-general.php -Out C:\path\shot.png [-Ref] [-Width 1400] [-Height 1000] [-Full]
param([string]$Page, [string]$Out, [switch]$Ref, [int]$Width = 1400, [int]$Height = 1000, [switch]$Full, [int]$Wait = 2, [switch]$Log, [string]$Eval = '')
$port = if ($Ref) { 8080 } else { 3001 }
$name = if ($Ref) { 'ref' } else { 'rp' }

# Sign in with curl to get the cookies.
$jar = Join-Path $env:TEMP "shot_jar_$name.txt"
Remove-Item $jar -ErrorAction SilentlyContinue
curl.exe -s -c $jar -b $jar "http://localhost:$port/wp-login.php" -o NUL
curl.exe -s -c $jar -b $jar -X POST "http://localhost:$port/wp-login.php" --data-urlencode log=rpadmin --data-urlencode pwd=rustpress-dev --data-urlencode testcookie=1 -o NUL
$cookies = foreach ($line in Get-Content $jar) {
    $l = $line -replace '^#HttpOnly_', ''
    if ($l.StartsWith('#') -or $l.Trim() -eq '') { continue }
    $f = $l -split "`t"
    if ($f.Count -ge 7) { @{ name = $f[5]; value = $f[6]; path = $f[2]; domain = 'localhost' } }
}

$profile = Join-Path $env:TEMP "edge_profile_$name"
Remove-Item -Recurse -Force $profile -ErrorAction SilentlyContinue
$debugPort = 9300 + $port % 100
$edge = 'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe'
$proc = Start-Process -FilePath $edge -ArgumentList @('--headless=new', '--disable-gpu', '--hide-scrollbars', '--no-first-run', '--disable-extensions', "--window-size=$Width,$Height", "--user-data-dir=$profile", "--remote-debugging-port=$debugPort", 'about:blank') -PassThru -WindowStyle Hidden
try {
    $target = $null
    for ($i = 0; $i -lt 40 -and -not $target; $i++) {
        Start-Sleep -Milliseconds 500
        try { $targets = Invoke-RestMethod "http://127.0.0.1:$debugPort/json/list" } catch { continue }
        $target = $targets | Where-Object { $_.type -eq 'page' -and $_.url -eq 'about:blank' } | Select-Object -First 1
        if (-not $target) { $target = $targets | Where-Object { $_.type -eq 'page' } | Select-Object -First 1 }
    }
    $ws = New-Object System.Net.WebSockets.ClientWebSocket
    $ws.ConnectAsync([Uri]$target.webSocketDebuggerUrl, [Threading.CancellationToken]::None).Wait()
    $script:id = 0
    function Send-Cdp($method, $params) {
        $script:id++
        $msg = @{ id = $script:id; method = $method; params = $params } | ConvertTo-Json -Depth 8 -Compress
        $bytes = [Text.Encoding]::UTF8.GetBytes($msg)
        $ws.SendAsync([ArraySegment[byte]]::new($bytes), 'Text', $true, [Threading.CancellationToken]::None).Wait()
        return $script:id
    }
    $script:pending = $null
    $script:problems = New-Object System.Collections.Generic.List[string]
    $script:requests = New-Object System.Collections.Generic.List[string]
    $script:buffer = New-Object byte[] 262144
    function Next-Message($timeoutMs) {
        $builder = New-Object Text.StringBuilder
        do {
            if (-not $script:pending) { $script:pending = $ws.ReceiveAsync([ArraySegment[byte]]::new($script:buffer), [Threading.CancellationToken]::None) }
            if (-not $script:pending.Wait([Math]::Max(1, $timeoutMs))) { return $null }
            $task = $script:pending
            $script:pending = $null
            [void]$builder.Append([Text.Encoding]::UTF8.GetString($script:buffer, 0, $task.Result.Count))
        } until ($task.Result.EndOfMessage)
        return ($builder.ToString() | ConvertFrom-Json)
    }
    function Note-Problem($obj) {
        switch ($obj.method) {
            'Runtime.consoleAPICalled' {
                if ($obj.params.type -in 'error', 'warning') {
                    $text = "console.$($obj.params.type): " + (($obj.params.args | ForEach-Object { if ($_.value) { $_.value } else { $_.description } }) -join ' ')
                    $script:problems.Add($text.Substring(0, [Math]::Min(500, $text.Length)))
                }
            }
            'Runtime.exceptionThrown' { $script:problems.Add('exception: ' + $obj.params.exceptionDetails.text + ' ' + $obj.params.exceptionDetails.exception.description) }
            'Network.requestWillBeSent' { if ($obj.params.request.url -match 'wp-json|rest_route|admin-ajax') { $script:requests.Add("$($obj.params.request.method) $($obj.params.request.url)") } }
            'Network.responseReceived' { if ($obj.params.response.status -ge 400) { $script:problems.Add("HTTP $($obj.params.response.status) $($obj.params.response.url)") } }
            'Network.loadingFailed' { if ($obj.params.errorText -ne 'net::ERR_ABORTED') { $script:problems.Add("failed: $($obj.params.errorText) $($obj.params.requestId)") } }
        }
    }
    function Receive-Until($wantedId, $wantedEvent, $timeoutSec) {
        $deadline = (Get-Date).AddSeconds($timeoutSec)
        while ((Get-Date) -lt $deadline) {
            $obj = Next-Message ([int](($deadline - (Get-Date)).TotalMilliseconds))
            if (-not $obj) { return $null }
            Note-Problem $obj
            if ($wantedId -and $obj.id -eq $wantedId) { return $obj }
            if ($wantedEvent -and $obj.method -eq $wantedEvent) { return $obj }
        }
        return $null
    }
    [void](Send-Cdp 'Page.enable' @{}); [void](Receive-Until 1 $null 5)
    [void](Send-Cdp 'Network.enable' @{}); [void](Receive-Until 2 $null 5)
    [void](Send-Cdp 'Runtime.enable' @{}); [void](Receive-Until 3 $null 5)
    foreach ($c in $cookies) {
        $i = Send-Cdp 'Network.setCookie' @{ name = $c.name; value = $c.value; domain = 'localhost'; path = $c.path; httpOnly = $true }
        [void](Receive-Until $i $null 5)
    }
    [void](Send-Cdp 'Emulation.setDeviceMetricsOverride' @{ width = $Width; height = $Height; deviceScaleFactor = 1; mobile = $false })
    [void](Receive-Until $script:id $null 5)
    [void](Send-Cdp 'Page.navigate' @{ url = "http://localhost:$port/wp-admin/$Page" })
    [void](Receive-Until $null 'Page.loadEventFired' 40)
    # Let the page settle; console errors and failed requests are noted on the way.
    [void](Receive-Until $null 'never' $Wait)
    if ($Eval) {
        $i = Send-Cdp 'Runtime.evaluate' @{ expression = $Eval; awaitPromise = $true; returnByValue = $true }
        $r = Receive-Until $i $null 20
        $script:problems.Add('eval: ' + ($r.result.result | ConvertTo-Json -Depth 5 -Compress))
    }
    if ($Log) {
        $script:problems | Select-Object -Unique | ForEach-Object { Write-Host $_ }
        Write-Host '--- REST/Ajax requests:'
        $script:requests | ForEach-Object { Write-Host ($_ -replace 'http://localhost:\d+', '') }
    }
    $params = @{ format = 'png' }
    if ($Full) {
        $i = Send-Cdp 'Page.getLayoutMetrics' @{}
        $m = Receive-Until $i $null 5
        $h = [int]$m.result.cssContentSize.height
        $params = @{ format = 'png'; captureBeyondViewport = $true; clip = @{ x = 0; y = 0; width = $Width; height = [Math]::Min($h, 6000); scale = 1 } }
    }
    $i = Send-Cdp 'Page.captureScreenshot' $params
    $shot = Receive-Until $i $null 30
    [IO.File]::WriteAllBytes($Out, [Convert]::FromBase64String($shot.result.data))
    (Get-Item $Out).Length
} finally {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -like "*$profile*" } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
}
