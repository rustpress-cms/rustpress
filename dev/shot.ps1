# Screenshot of a wp-admin page of the dev server (:3001) or the reference (:8080, -Ref) with headless Edge, signed
# in as rpadmin (cookies are set through the DevTools protocol).
#   dev/shot.ps1 -Page options-general.php -Out C:\path\shot.png [-Ref] [-Width 1400] [-Height 1000] [-Full]
param([string]$Page, [string]$Out, [switch]$Ref, [int]$Width = 1400, [int]$Height = 1000, [switch]$Full, [int]$Wait = 2)
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
    function Receive-Until($wantedId, $wantedEvent, $timeoutSec) {
        $deadline = (Get-Date).AddSeconds($timeoutSec)
        $buffer = New-Object byte[] 65536
        while ((Get-Date) -lt $deadline) {
            $builder = New-Object Text.StringBuilder
            do {
                $task = $ws.ReceiveAsync([ArraySegment[byte]]::new($buffer), [Threading.CancellationToken]::None)
                if (-not $task.Wait([Math]::Max(100, [int](($deadline - (Get-Date)).TotalMilliseconds)))) { return $null }
                [void]$builder.Append([Text.Encoding]::UTF8.GetString($buffer, 0, $task.Result.Count))
            } until ($task.Result.EndOfMessage)
            $obj = $builder.ToString() | ConvertFrom-Json
            if ($wantedId -and $obj.id -eq $wantedId) { return $obj }
            if ($wantedEvent -and $obj.method -eq $wantedEvent) { return $obj }
        }
        return $null
    }
    [void](Send-Cdp 'Page.enable' @{}); [void](Receive-Until 1 $null 5)
    [void](Send-Cdp 'Network.enable' @{}); [void](Receive-Until 2 $null 5)
    foreach ($c in $cookies) {
        $i = Send-Cdp 'Network.setCookie' @{ name = $c.name; value = $c.value; domain = 'localhost'; path = $c.path; httpOnly = $true }
        [void](Receive-Until $i $null 5)
    }
    [void](Send-Cdp 'Emulation.setDeviceMetricsOverride' @{ width = $Width; height = $Height; deviceScaleFactor = 1; mobile = $false })
    [void](Receive-Until $script:id $null 5)
    [void](Send-Cdp 'Page.navigate' @{ url = "http://localhost:$port/wp-admin/$Page" })
    [void](Receive-Until $null 'Page.loadEventFired' 40)
    Start-Sleep -Seconds $Wait
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
