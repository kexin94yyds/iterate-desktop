param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerPath,
  [switch]$Install
)

$ErrorActionPreference = "Stop"
$Installer = (Resolve-Path -LiteralPath $InstallerPath).Path
if (-not $Installer.EndsWith("-setup.exe", [StringComparison]::OrdinalIgnoreCase)) {
  throw "Expected a Tauri NSIS *-setup.exe installer: $Installer"
}

$Bytes = [IO.File]::ReadAllBytes($Installer)
if ($Bytes.Length -lt 64 -or $Bytes[0] -ne 0x4D -or $Bytes[1] -ne 0x5A) {
  throw "Installer is not a valid Windows PE file: $Installer"
}
Write-Host "[pass] NSIS installer exists and has a PE header: $Installer"

if (-not $Install) {
  Write-Host "[warn] Install/start/uninstall smoke skipped; pass -Install to run it"
  exit 0
}

$InstallProcess = Start-Process -FilePath $Installer -ArgumentList "/S" -PassThru -Wait
if ($InstallProcess.ExitCode -ne 0) {
  throw "Silent installer failed with exit code $($InstallProcess.ExitCode)"
}

$UninstallEntry = Get-ChildItem "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall" |
  ForEach-Object { Get-ItemProperty $_.PSPath } |
  Where-Object { $_.DisplayName -eq "iterate" } |
  Select-Object -First 1
if (-not $UninstallEntry) {
  throw "iterate uninstall registration was not created for the current user"
}
Write-Host "[pass] Current-user uninstall registration exists"

$InstallLocation = ([string]$UninstallEntry.InstallLocation).Trim().Trim('"')
$InstalledExe = Join-Path $InstallLocation "iterate.exe"
if (-not (Test-Path -LiteralPath $InstalledExe -PathType Leaf)) {
  throw "Installed iterate.exe is missing: $InstalledExe"
}
Write-Host "[pass] Installed executable exists: $InstalledExe"

$ActivationProbe = & $InstalledExe --activation-gate-status
if ($LASTEXITCODE -ne 0 -or $ActivationProbe -ne "activation_gate_required=false") {
  throw "Installed community executable unexpectedly requires activation: $ActivationProbe"
}
Write-Host "[pass] Installed community executable reports activation_gate_required=false"

$Desktop = [Environment]::GetFolderPath("Desktop")
$DesktopShortcut = Join-Path $Desktop "iterate.lnk"
$StartMenu = [Environment]::GetFolderPath("StartMenu")
$StartMenuShortcut = Get-ChildItem -LiteralPath $StartMenu -Filter "iterate.lnk" -Recurse -ErrorAction SilentlyContinue |
  Select-Object -First 1
if (-not (Test-Path -LiteralPath $DesktopShortcut -PathType Leaf)) {
  throw "Desktop shortcut was not created: $DesktopShortcut"
}
if (-not $StartMenuShortcut) {
  throw "Start menu shortcut was not created under: $StartMenu"
}
$Shell = New-Object -ComObject WScript.Shell
$DesktopTarget = $Shell.CreateShortcut($DesktopShortcut).TargetPath
$StartMenuTarget = $Shell.CreateShortcut($StartMenuShortcut.FullName).TargetPath
if (-not [IO.Path]::GetFullPath($DesktopTarget).Equals([IO.Path]::GetFullPath($InstalledExe), [StringComparison]::OrdinalIgnoreCase)) {
  throw "Desktop shortcut target is incorrect: $DesktopTarget"
}
if (-not [IO.Path]::GetFullPath($StartMenuTarget).Equals([IO.Path]::GetFullPath($InstalledExe), [StringComparison]::OrdinalIgnoreCase)) {
  throw "Start menu shortcut target is incorrect: $StartMenuTarget"
}
Write-Host "[pass] Desktop and Start menu shortcuts target the installed iterate.exe"

Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class IterateWindowProbe {
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
  public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);
}
'@

function Get-IterateWindowHandle {
  param(
    [Parameter(Mandatory = $true)]
    [int]$ProcessId
  )

  $MatchedHandle = [IntPtr]::Zero
  [IterateWindowProbe]::EnumWindows({
    param([IntPtr]$Handle, [IntPtr]$LParam)

    [uint32]$WindowProcessId = 0
    [IterateWindowProbe]::GetWindowThreadProcessId($Handle, [ref]$WindowProcessId) | Out-Null
    if ($WindowProcessId -eq $ProcessId -and [IterateWindowProbe]::IsWindowVisible($Handle)) {
      $Title = [Text.StringBuilder]::new(256)
      [IterateWindowProbe]::GetWindowText($Handle, $Title, $Title.Capacity) | Out-Null
      if ($Title.ToString() -eq "iterate") {
        $script:MatchedIterateWindowHandle = $Handle
        return $false
      }
    }
    return $true
  }, [IntPtr]::Zero) | Out-Null

  if ($script:MatchedIterateWindowHandle) {
    $MatchedHandle = $script:MatchedIterateWindowHandle
    Remove-Variable MatchedIterateWindowHandle -Scope Script
  }
  return $MatchedHandle
}

function Start-ShortcutAndVerify {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ShortcutPath,
    [Parameter(Mandatory = $true)]
    [string]$Label
  )

  $ExistingPids = @(Get-Process iterate -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
  Start-Process -FilePath $ShortcutPath | Out-Null
  $Deadline = (Get-Date).AddSeconds(5)
  $AppProcess = $null
  while ((Get-Date) -lt $Deadline) {
    $AppProcess = Get-Process iterate -ErrorAction SilentlyContinue |
      Where-Object { $_.Id -notin $ExistingPids } |
      Select-Object -First 1
    if ($AppProcess) {
      $AppProcess.Refresh()
      $WindowHandle = Get-IterateWindowHandle -ProcessId $AppProcess.Id
      if ($WindowHandle -ne [IntPtr]::Zero) { break }
    }
    Start-Sleep -Milliseconds 100
  }

  if (-not $AppProcess -or $AppProcess.HasExited -or $WindowHandle -eq [IntPtr]::Zero) {
    throw "$Label did not launch a visible window titled iterate within 5 seconds"
  }

  $CloseWatch = [Diagnostics.Stopwatch]::StartNew()
  if (-not [IterateWindowProbe]::PostMessage($WindowHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
    throw "$Label did not accept a normal close request"
  }
  if (-not $AppProcess.WaitForExit(2000)) {
    Stop-Process -Id $AppProcess.Id -Force
    throw "$Label process did not exit within 2 seconds after one close request"
  }
  Write-Host "[pass] $Label launched iterate and closed normally in $($CloseWatch.ElapsedMilliseconds)ms"
}

Start-ShortcutAndVerify -ShortcutPath $DesktopShortcut -Label "Desktop shortcut"
Start-ShortcutAndVerify -ShortcutPath $StartMenuShortcut.FullName -Label "Start menu shortcut"

$OverlayProcess = Start-Process -FilePath $Installer -ArgumentList "/S" -PassThru -Wait
if ($OverlayProcess.ExitCode -ne 0) {
  throw "Silent overlay install failed with exit code $($OverlayProcess.ExitCode)"
}
if (-not (Test-Path -LiteralPath $InstalledExe -PathType Leaf)) {
  throw "Overlay install removed the installed executable"
}
Write-Host "[pass] Silent overlay install completed successfully"

$Uninstaller = $UninstallEntry.UninstallString.Trim('"')
if (-not (Test-Path -LiteralPath $Uninstaller -PathType Leaf)) {
  throw "Uninstaller is missing: $Uninstaller"
}
$UninstallProcess = Start-Process -FilePath $Uninstaller -ArgumentList "/S" -PassThru -Wait
if ($UninstallProcess.ExitCode -ne 0) {
  throw "Silent uninstaller failed with exit code $($UninstallProcess.ExitCode)"
}
Start-Sleep -Milliseconds 500
if ((Test-Path -LiteralPath $InstalledExe -PathType Leaf) -or (Test-Path -LiteralPath $DesktopShortcut -PathType Leaf) -or (Test-Path -LiteralPath $StartMenuShortcut.FullName -PathType Leaf)) {
  throw "Silent uninstall left the executable or shortcuts behind"
}
Write-Host "[pass] Silent uninstall removed the executable and shortcuts"
