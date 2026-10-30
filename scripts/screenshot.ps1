# screenshot.sh から呼ぶ。headless Chrome を DevTools のプロトコル（CDP）で動かし、幅ごとにライト・ダークを撮る。
# 引数は設定の JSON のパス1つ（browser、url、dir、height、widths、wait）。dir の下に <幅>-light.png と <幅>-dark.png を書く。
#
# iframe に入れて --screenshot で撮る形をやめた理由（2026-10-16）: --virtual-time-budget の仮想時間は外のページにしか
# 効かず、別プロセスの iframe の中の WASM は撮る時点でまだ起動していない。起動後に描かれるもの（メニュー）が写らなかった。
# ここでは画面の幅を Emulation で決め、hydration の印（data-dioxus-id）が出るまで実時間で待ってから撮る。
# 1024px 未満（スマホと iPad。docs/adr/0007）はモバイルとして撮る。スクロールバーが本文に重なり（実機と同じ。
# ページは scrollbar-gutter: stable）、タッチの端末として hover が効かない。PC は実機どおりスクロールバーが幅を取る。
$ErrorActionPreference = 'Stop'
$cfg = Get-Content -Raw $args[0] | ConvertFrom-Json

$listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
$listener.Start(); $port = $listener.LocalEndpoint.Port; $listener.Stop()
$profileDir = Join-Path ([IO.Path]::GetTempPath()) ('rustruct-shot-' + [guid]::NewGuid())
$chrome = Start-Process $cfg.browser -PassThru -ArgumentList @(
  '--headless=new', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
  "--user-data-dir=$profileDir", "--remote-debugging-port=$port", 'about:blank')

$ws = $null
try {
  $page = $null
  foreach ($i in 1..50) {
    # Invoke-RestMethod は配列を1つの塊で流すので、括弧で包んで要素ごとに選ぶ
    try { $page = (Invoke-RestMethod "http://127.0.0.1:$port/json") | Where-Object { $_.type -eq 'page' } | Select-Object -First 1 } catch {}
    if ($page) { break }
    Start-Sleep -Milliseconds 200
  }
  if (-not $page) { throw 'Chrome の DevTools に届かない' }
  $ws = [Net.WebSockets.ClientWebSocket]::new()
  $ws.ConnectAsync([uri]$page.webSocketDebuggerUrl, [Threading.CancellationToken]::None).Wait()

  $script:id = 0
  function Send($method, $params = @{}) {
    $script:id++
    $bytes = [Text.Encoding]::UTF8.GetBytes((@{ id = $script:id; method = $method; params = $params } | ConvertTo-Json -Depth 10 -Compress))
    $ws.SendAsync([ArraySegment[byte]]::new($bytes), 'Text', $true, [Threading.CancellationToken]::None).Wait()
    while ($true) {
      $buf = [byte[]]::new(1048576); $sb = [Text.StringBuilder]::new()
      do {
        $r = $ws.ReceiveAsync([ArraySegment[byte]]::new($buf), [Threading.CancellationToken]::None).Result
        [void]$sb.Append([Text.Encoding]::UTF8.GetString($buf, 0, $r.Count))
      } while (-not $r.EndOfMessage)
      $o = $sb.ToString() | ConvertFrom-Json
      if ($o.id -eq $script:id) {
        if ($o.error) { throw "$method : $($o.error.message)" }
        return $o.result
      }
    }
  }
  function Eval($js) { (Send 'Runtime.evaluate' @{ expression = $js; returnByValue = $true }).result.value }

  foreach ($width in $cfg.widths) {
    $mobile = $width -lt 1024
    Send 'Emulation.setDeviceMetricsOverride' @{ width = $width; height = $cfg.height; deviceScaleFactor = 1; mobile = $mobile } | Out-Null
    Send 'Emulation.setTouchEmulationEnabled' @{ enabled = $mobile } | Out-Null
    foreach ($theme in 'light', 'dark') {
      Send 'Emulation.setEmulatedMedia' @{ features = @(@{ name = 'prefers-color-scheme'; value = $theme }) } | Out-Null
      # 前の撮影で選んだ色が残らないように、覚えた色を消してから開く
      Eval 'try{localStorage.clear()}catch(e){}' | Out-Null
      Send 'Page.navigate' @{ url = $cfg.url } | Out-Null
      $ready = $false
      foreach ($i in 1..150) {
        Start-Sleep -Milliseconds 200
        if (Eval "document.readyState === 'complete' && !!document.querySelector('[data-dioxus-id]')") { $ready = $true; break }
      }
      if (-not $ready) { [Console]::Error.WriteLine("$($cfg.url) が 30 秒で hydration しなかった（撮るのは続ける）") }
      Start-Sleep -Milliseconds $cfg.wait
      $png = (Send 'Page.captureScreenshot' @{ format = 'png' }).data
      $file = Join-Path $cfg.dir "$width-$theme.png"
      [IO.File]::WriteAllBytes($file, [Convert]::FromBase64String($png))
    }
  }
} finally {
  if ($ws) { $ws.Dispose() }
  # Chrome は子のプロセスを残すので、このプロファイルで立てたものをまとめて止める
  Stop-Process -Id $chrome.Id -Force -ErrorAction SilentlyContinue
  if ($IsWindows -or $env:OS -eq 'Windows_NT') {
    Get-CimInstance Win32_Process -Filter "Name='chrome.exe' OR Name='msedge.exe'" |
      Where-Object { $_.CommandLine -like "*$profileDir*" } |
      ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
  }
  Start-Sleep -Milliseconds 300
  Remove-Item -Recurse -Force $profileDir -ErrorAction SilentlyContinue
}
