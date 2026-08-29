param(
  [string]$PackageDir = "windows-package",
  [switch]$Launch
)

$ErrorActionPreference = "Stop"

function Pass($Message) {
  Write-Host "[pass] $Message"
  $script:PassCount += 1
}

function Fail($Message) {
  Write-Host "[fail] $Message"
  $script:FailCount += 1
}

function Warn($Message) {
  Write-Host "[warn] $Message"
  $script:WarnCount += 1
}

function Require-File($Path, $Label) {
  if (Test-Path -LiteralPath $Path -PathType Leaf) {
    Pass "$Label`: $Path"
  } else {
    Fail "$Label missing: $Path"
  }
}

function Get-PeMachine($Path) {
  $Bytes = [System.IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $Path))
  if ($Bytes.Length -lt 64 -or $Bytes[0] -ne 0x4D -or $Bytes[1] -ne 0x5A) {
    throw "$Path does not have a valid DOS MZ header"
  }

  $PeOffset = [System.BitConverter]::ToInt32($Bytes, 0x3C)
  if ($PeOffset -lt 0 -or $PeOffset + 6 -gt $Bytes.Length) {
    throw "$Path has an invalid PE header offset"
  }

  if ($Bytes[$PeOffset] -ne 0x50 -or $Bytes[$PeOffset + 1] -ne 0x45 -or
      $Bytes[$PeOffset + 2] -ne 0x00 -or $Bytes[$PeOffset + 3] -ne 0x00) {
    throw "$Path does not have a valid PE signature"
  }

  return [System.BitConverter]::ToUInt16($Bytes, $PeOffset + 4)
}

function Require-X64Pe($Path, $Label) {
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    return
  }

  try {
    $Machine = Get-PeMachine $Path
    if ($Machine -eq 0x8664) {
      Pass "$Label is x86-64 PE (machine=0x8664)"
    } else {
      Fail "$Label has wrong PE machine: 0x$('{0:X4}' -f $Machine), expected 0x8664"
    }
  } catch {
    Fail "$Label PE validation failed: $($_.Exception.Message)"
  }
}

$script:PassCount = 0
$script:WarnCount = 0
$script:FailCount = 0

Write-Host "iterate Windows package smoke"
Write-Host "package_dir=$PackageDir"
Write-Host "launch=$Launch"
Write-Host ""

$AppDir = Join-Path $PackageDir "app"

if (Test-Path -LiteralPath $PackageDir -PathType Container) {
  Pass "package directory exists"
} else {
  Fail "package directory missing: $PackageDir"
}

if (Test-Path -LiteralPath $AppDir -PathType Container) {
  Pass "app directory exists"
} else {
  Fail "app directory missing: $AppDir"
}

Require-File (Join-Path $AppDir "iterate.exe") "iterate executable"
Require-File (Join-Path $AppDir "mcp-server.exe") "mcp-server executable"
Require-File (Join-Path $AppDir "WebView2Loader.dll") "WebView2 loader"
Require-File (Join-Path $PackageDir "Install iterate.bat") "install helper"
Require-File (Join-Path $PackageDir "Start iterate.bat") "start helper"
Require-File (Join-Path $PackageDir "INSTALLATION.md") "installation guide"

$ActivationProbe = & (Join-Path $AppDir "iterate.exe") --activation-gate-status
if ($LASTEXITCODE -eq 0 -and $ActivationProbe -eq "activation_gate_required=false") {
  Pass "community activation gate is disabled"
} else {
  Fail "community activation gate probe failed: $ActivationProbe"
}

Require-X64Pe (Join-Path $AppDir "iterate.exe") "iterate executable"
Require-X64Pe (Join-Path $AppDir "mcp-server.exe") "mcp-server executable"
Require-X64Pe (Join-Path $AppDir "WebView2Loader.dll") "WebView2 loader"

if ($Launch) {
  $ExePath = Join-Path $AppDir "iterate.exe"
  if (Test-Path -LiteralPath $ExePath -PathType Leaf) {
    $Process = Start-Process -FilePath $ExePath -PassThru
    Start-Sleep -Seconds 5
    if ($Process.HasExited) {
      Fail "iterate.exe exited during smoke startup"
    } else {
      Pass "iterate.exe launched and stayed alive for 5 seconds"
      Stop-Process -Id $Process.Id -Force
    }
  } else {
    Fail "cannot launch missing iterate.exe"
  }
} else {
  Warn "launch smoke skipped; pass -Launch to start iterate.exe on Windows"
}

Write-Host ""
Write-Host "summary: pass=$PassCount warn=$WarnCount fail=$FailCount"

if ($FailCount -gt 0) {
  exit 1
}
