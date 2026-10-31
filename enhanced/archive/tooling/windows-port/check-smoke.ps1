$ErrorActionPreference='Continue'
$root='<WINDOWS_USER_HOME>\AppData\Local\Temp\Horizon Enhanced Smoke 0d489435191e45049a1b28eacbbc3a3b'
Get-ChildItem -LiteralPath (Join-Path $root 'game\logs') | Select-Object Name,Length
Get-ChildItem -LiteralPath (Join-Path $root 'game\logs') -Filter '*.stderr.log' | Sort-Object LastWriteTime -Descending | Select-Object -First 1 | Get-Content -Tail 18
& '\\Mac\Home\Documents\Codex\2026-10-30\new-chat\outputs\Horizon-Enhanced-Windows-x64-0.4.0\horizon-ctl.exe' --socket (Join-Path $root 'control.json') --timeout 5 state
