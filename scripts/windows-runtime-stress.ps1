param(
  [Parameter(Mandatory = $true)]
  [string]$ExePath,
  [int]$DurationSeconds = 30
)

$ErrorActionPreference = "Stop"
$Executable = (Resolve-Path -LiteralPath $ExePath).Path

Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class IterateRuntimeProbe {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);

  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsHungAppWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool MoveWindow(IntPtr hWnd, int x, int y, int width, int height, bool repaint);

  [DllImport("user32.dll")]
  public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);
}
'@

function Get-IterateWindowHandle {
  param([Parameter(Mandatory = $true)][int]$ProcessId)

  $script:MatchedRuntimeWindow = [IntPtr]::Zero
  [IterateRuntimeProbe]::EnumWindows({
    param([IntPtr]$Handle, [IntPtr]$LParam)
    [uint32]$WindowProcessId = 0
    [IterateRuntimeProbe]::GetWindowThreadProcessId($Handle, [ref]$WindowProcessId) | Out-Null
    if ($WindowProcessId -eq $ProcessId -and [IterateRuntimeProbe]::IsWindowVisible($Handle)) {
      $Title = [Text.StringBuilder]::new(256)
      [IterateRuntimeProbe]::GetWindowText($Handle, $Title, $Title.Capacity) | Out-Null
      if ($Title.ToString() -eq "iterate") {
        $script:MatchedRuntimeWindow = $Handle
        return $false
      }
    }
    return $true
  }, [IntPtr]::Zero) | Out-Null
  return $script:MatchedRuntimeWindow
}

function Get-DescendantNames {
  param([Parameter(Mandatory = $true)][int]$RootProcessId)

  $Processes = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name)
  $Pending = [Collections.Generic.Queue[uint32]]::new()
  $Pending.Enqueue([uint32]$RootProcessId)
  $Names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
  while ($Pending.Count -gt 0) {
    $ParentId = $Pending.Dequeue()
    foreach ($Child in $Processes | Where-Object ParentProcessId -eq $ParentId) {
      $Names.Add($Child.Name) | Out-Null
      $Pending.Enqueue([uint32]$Child.ProcessId)
    }
  }
  return @($Names)
}

$AppProcess = Start-Process -FilePath $Executable -PassThru
$ClosedNormally = $false
try {
  $StartupWatch = [Diagnostics.Stopwatch]::StartNew()
  $WindowHandle = [IntPtr]::Zero
  while ($StartupWatch.ElapsedMilliseconds -lt 5000) {
    if ($AppProcess.HasExited) {
      throw "iterate.exe exited before showing its window (exit code $($AppProcess.ExitCode))"
    }
    $WindowHandle = Get-IterateWindowHandle -ProcessId $AppProcess.Id
    if ($WindowHandle -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 25
  }
  if ($WindowHandle -eq [IntPtr]::Zero) {
    throw "iterate window was not visible within 5 seconds"
  }
  $StartupMilliseconds = $StartupWatch.ElapsedMilliseconds

  $StressWatch = [Diagnostics.Stopwatch]::StartNew()
  $Moves = 0
  $HungSamples = 0
  $ObservedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
  $NextProcessSample = 0
  while ($StressWatch.Elapsed.TotalSeconds -lt $DurationSeconds) {
    $Phase = $Moves % 8
    $X = 80 + (($Phase % 4) * 35)
    $Y = 70 + (([math]::Floor($Phase / 2) % 4) * 25)
    $Width = 600
    $Height = 650 + (($Phase % 3) * 55)
    if (-not [IterateRuntimeProbe]::MoveWindow($WindowHandle, $X, $Y, $Width, $Height, $true)) {
      throw "MoveWindow failed during stress iteration $Moves"
    }
    if ([IterateRuntimeProbe]::IsHungAppWindow($WindowHandle)) {
      $HungSamples++
    }
    $AppProcess.Refresh()
    if (-not $AppProcess.Responding) {
      $HungSamples++
    }
    if ($StressWatch.ElapsedMilliseconds -ge $NextProcessSample) {
      foreach ($Name in Get-DescendantNames -RootProcessId $AppProcess.Id) {
        $ObservedNames.Add($Name) | Out-Null
      }
      $NextProcessSample = $StressWatch.ElapsedMilliseconds + 1000
    }
    $Moves++
    Start-Sleep -Milliseconds 75
  }

  $Forbidden = @("tasklist.exe", "curl.exe", "cmd.exe", "conhost.exe")
  $ForbiddenObserved = @($Forbidden | Where-Object { $ObservedNames.Contains($_) })
  if ($HungSamples -ne 0) {
    throw "iterate reported $HungSamples hung/non-responding samples during the stress run"
  }
  if ($ForbiddenObserved.Count -ne 0) {
    throw "forbidden console child processes were observed: $($ForbiddenObserved -join ', ')"
  }

  $CloseWatch = [Diagnostics.Stopwatch]::StartNew()
  if (-not [IterateRuntimeProbe]::PostMessage($WindowHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
    throw "iterate did not accept the close message"
  }
  if (-not $AppProcess.WaitForExit(2000)) {
    throw "iterate did not exit within 2 seconds after one close request"
  }
  $ClosedNormally = $true

  Start-Sleep -Milliseconds 250
  $RemainingDescendants = @(Get-DescendantNames -RootProcessId $AppProcess.Id)
  if ($RemainingDescendants.Count -ne 0) {
    throw "iterate-owned processes remained after exit: $($RemainingDescendants -join ', ')"
  }

  Write-Host "[pass] Window visible in ${StartupMilliseconds}ms"
  Write-Host "[pass] $Moves move/resize operations over $([math]::Round($StressWatch.Elapsed.TotalSeconds, 1))s with 0 hung samples"
  Write-Host "[pass] No tasklist.exe, curl.exe, cmd.exe, or conhost.exe descendants observed"
  Write-Host "[pass] One close request exited iterate.exe in $($CloseWatch.ElapsedMilliseconds)ms with no owned processes remaining"
}
finally {
  if (-not $ClosedNormally -and -not $AppProcess.HasExited) {
    Stop-Process -Id $AppProcess.Id -Force -ErrorAction SilentlyContinue
  }
}
