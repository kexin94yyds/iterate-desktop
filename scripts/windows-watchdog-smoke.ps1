param(
  [Parameter(Mandatory = $true)]
  [string]$ExePath,
  [int]$TimeoutSeconds = 75
)

$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path -LiteralPath $ExePath).Path
$ExistingListener = Get-NetTCPConnection -LocalPort 8080 -State Listen -ErrorAction SilentlyContinue
if ($ExistingListener) {
  throw "Port 8080 is already in use; watchdog smoke requires an isolated port"
}

$Listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 8080)
$Listener.Start()
$AcceptTask = $Listener.AcceptTcpClientAsync()
$App = $null
try {
  $App = Start-Process -FilePath $Exe -PassThru
  $BlockUntil = (Get-Date).AddSeconds(8)
  while ((Get-Date) -lt $BlockUntil) {
    if ($AcceptTask.IsCompletedSuccessfully) {
      $Client = $AcceptTask.Result
      try {
        $Payload = [Text.Encoding]::ASCII.GetBytes("HTTP/1.1 503 Service Unavailable`r`nContent-Length: 0`r`nConnection: close`r`n`r`n")
        $Client.GetStream().Write($Payload, 0, $Payload.Length)
      }
      finally {
        $Client.Dispose()
      }
      $AcceptTask = $Listener.AcceptTcpClientAsync()
    }
    if ($App.HasExited) {
      throw "iterate.exe exited while Bridge port was unavailable"
    }
    Start-Sleep -Milliseconds 25
  }

  $Listener.Stop()
  $Deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  $Recovered = $false
  $Http = [Net.Http.HttpClient]::new()
  $Http.Timeout = [TimeSpan]::FromSeconds(2)
  try {
    while ((Get-Date) -lt $Deadline) {
      if ($App.HasExited) {
        throw "iterate.exe restarted or exited during Bridge recovery"
      }
      try {
        $Body = $Http.GetStringAsync("http://127.0.0.1:8080/api/version").GetAwaiter().GetResult()
        if ($Body -match "iterate") {
          $Recovered = $true
          break
        }
      }
      catch {
        # Expected while the Windows watchdog is waiting for three failures.
      }
      Start-Sleep -Milliseconds 250
    }
  }
  finally {
    $Http.Dispose()
  }

  if (-not $Recovered) {
    throw "Bridge did not recover within $TimeoutSeconds seconds"
  }
  if ($App.HasExited) {
    throw "iterate.exe did not preserve the original GUI process"
  }
  Write-Host "[pass] Bridge recovered without restarting iterate.exe (PID $($App.Id))"

  $CloseStarted = Get-Date
  $null = $App.CloseMainWindow()
  if (-not $App.WaitForExit(2000)) {
    throw "iterate.exe did not exit within 2 seconds after one close request"
  }
  $ExitMs = [math]::Round(((Get-Date) - $CloseStarted).TotalMilliseconds)
  Write-Host "[pass] iterate.exe exited after one close request in ${ExitMs}ms"
}
finally {
  try { $Listener.Stop() } catch {}
  if ($App -and -not $App.HasExited) {
    Stop-Process -Id $App.Id -Force -ErrorAction SilentlyContinue
  }
}
