@echo off
setlocal
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Launch-Horizon.ps1" %*
set "HORIZON_EXIT_CODE=%ERRORLEVEL%"
if not "%HORIZON_EXIT_CODE%"=="0" (
  echo.
  echo Horizon Enhanced could not start or exited with an error.
  echo See the launcher and client logs in your Horizon Enhanced data folder.
  pause
)
exit /b %HORIZON_EXIT_CODE%
