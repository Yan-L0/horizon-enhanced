$ErrorActionPreference='Continue'
$root='<WINDOWS_USER_HOME>\AppData\Local\Temp\Horizon Enhanced Smoke 0d489435191e45049a1b28eacbbc3a3b'
$logs=Join-Path $root 'game\logs'
Get-ChildItem -LiteralPath $logs -Filter 'client-*.log' | Sort-Object LastWriteTime -Descending | Select-Object -First 2 | ForEach-Object { $_.Name; Get-Content -LiteralPath $_.FullName -Tail 22 }
Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'horizon-client.exe' -and $_.CommandLine -like ('*'+$root+'*') } | Select-Object ProcessId,CommandLine
