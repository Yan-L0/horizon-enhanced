$ErrorActionPreference='Continue'
$root='<WINDOWS_USER_HOME>\AppData\Local\Temp\Horizon Enhanced Smoke 0d489435191e45049a1b28eacbbc3a3b'
$dest='\\Mac\Home\Documents\Codex\2026-10-30\new-chat\work\windows-port'
$logs=Join-Path $root 'game\logs'
$err=Get-ChildItem -LiteralPath $logs -Filter 'client-*.stderr.log' | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
$out=Get-ChildItem -LiteralPath $logs -Filter 'client-*.log' | Where-Object { $_.Name -notlike '*.stderr.log' } | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
if ($err) { Copy-Item -LiteralPath $err.FullName -Destination (Join-Path $dest 'vm-runtime.log') }
if ($out) { Copy-Item -LiteralPath $out.FullName -Destination (Join-Path $dest 'vm-game.log') }
$ours=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'horizon-client.exe' -and $_.CommandLine.Contains($root) })
$ours | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
'Cleaned only isolated test processes: '+$ours.Count
