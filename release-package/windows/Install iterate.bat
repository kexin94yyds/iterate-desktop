@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
set "PACKAGE_DIR=%SCRIPT_DIR%app"
set "INSTALL_DIR=%LOCALAPPDATA%\iterate"
set "BIN_DIR=%INSTALL_DIR%\bin"
set "SHORTCUT_PATH=%USERPROFILE%\Desktop\iterate.lnk"
set "WEBVIEW2_GUID={F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

if not exist "%PACKAGE_DIR%\iterate.exe" (
  echo [ERROR] Missing "%PACKAGE_DIR%\iterate.exe"
  exit /b 1
)

if not exist "%PACKAGE_DIR%\mcp-server.exe" (
  echo [ERROR] Missing "%PACKAGE_DIR%\mcp-server.exe"
  exit /b 1
)

if not exist "%PACKAGE_DIR%\WebView2Loader.dll" (
  echo [ERROR] Missing "%PACKAGE_DIR%\WebView2Loader.dll"
  exit /b 1
)

call :ensure_webview2_runtime
if errorlevel 1 exit /b 1

echo Installing iterate to "%INSTALL_DIR%"

if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"

copy /Y "%PACKAGE_DIR%\iterate.exe" "%BIN_DIR%\iterate.exe" >nul
copy /Y "%PACKAGE_DIR%\mcp-server.exe" "%BIN_DIR%\mcp-server.exe" >nul
copy /Y "%PACKAGE_DIR%\WebView2Loader.dll" "%BIN_DIR%\WebView2Loader.dll" >nul

if exist "%SCRIPT_DIR%INSTALLATION.md" (
  copy /Y "%SCRIPT_DIR%INSTALLATION.md" "%INSTALL_DIR%\INSTALLATION.md" >nul
)

if exist "%SCRIPT_DIR%INSTALL_PROMPT.md" (
  copy /Y "%SCRIPT_DIR%INSTALL_PROMPT.md" "%INSTALL_DIR%\INSTALL_PROMPT.md" >nul
)

if exist "%SCRIPT_DIR%SYSTEM_PROMPT.md" (
  copy /Y "%SCRIPT_DIR%SYSTEM_PROMPT.md" "%INSTALL_DIR%\SYSTEM_PROMPT.md" >nul
)

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ws = New-Object -ComObject WScript.Shell; " ^
  "$shortcut = $ws.CreateShortcut('%SHORTCUT_PATH%'); " ^
  "$shortcut.TargetPath = '%BIN_DIR%\iterate.exe'; " ^
  "$shortcut.WorkingDirectory = '%BIN_DIR%'; " ^
  "$shortcut.IconLocation = '%BIN_DIR%\iterate.exe,0'; " ^
  "$shortcut.Save()" >nul 2>nul

echo.
echo [OK] iterate installed.
echo [OK] Desktop shortcut: "%SHORTCUT_PATH%"
echo.
echo Next:
echo 1. Double-click the desktop shortcut or run "Start iterate.bat"
echo 2. Open INSTALLATION.md to connect your client
echo 3. Add SYSTEM_PROMPT.md to your client's system prompt or rules
echo 4. Restart Windsurf / Cursor / Codex after setup
echo.
pause
exit /b 0

:ensure_webview2_runtime
reg query "HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\%WEBVIEW2_GUID%" /v pv >nul 2>nul && goto :webview2_ready
reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\%WEBVIEW2_GUID%" /v pv >nul 2>nul && goto :webview2_ready
reg query "HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\%WEBVIEW2_GUID%" /v pv >nul 2>nul && goto :webview2_ready

echo Microsoft WebView2 Runtime is missing. Installing it now...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ErrorActionPreference = 'Stop'; " ^
  "$bootstrapper = Join-Path $env:TEMP 'MicrosoftEdgeWebView2Setup.exe'; " ^
  "Invoke-WebRequest 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $bootstrapper; " ^
  "Start-Process -FilePath $bootstrapper -ArgumentList '/silent','/install' -Wait"
if errorlevel 1 (
  echo [ERROR] Failed to install Microsoft WebView2 Runtime.
  echo [ERROR] Install it manually from https://developer.microsoft.com/microsoft-edge/webview2/
  exit /b 1
)

reg query "HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\%WEBVIEW2_GUID%" /v pv >nul 2>nul && goto :webview2_ready
reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\%WEBVIEW2_GUID%" /v pv >nul 2>nul && goto :webview2_ready
reg query "HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\%WEBVIEW2_GUID%" /v pv >nul 2>nul && goto :webview2_ready

echo [ERROR] WebView2 Runtime still not detected after installation.
exit /b 1

:webview2_ready
exit /b 0
