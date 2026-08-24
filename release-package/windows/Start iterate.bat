@echo off
setlocal

set "SCRIPT_DIR=%~dp0"

if exist "%LOCALAPPDATA%\iterate\bin\iterate.exe" (
  start "" "%LOCALAPPDATA%\iterate\bin\iterate.exe"
  exit /b 0
)

if exist "%SCRIPT_DIR%app\iterate.exe" (
  start "" "%SCRIPT_DIR%app\iterate.exe"
  exit /b 0
)

echo [ERROR] iterate.exe not found.
echo Run "Install iterate.bat" first, or keep this file next to the "app" folder.
pause
exit /b 1
