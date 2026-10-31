$ErrorActionPreference='Stop'
$ProgressPreference='SilentlyContinue'
$bundle='\\Mac\Home\Documents\Codex\2026-10-30\new-chat\outputs\Horizon-Enhanced-Windows-x64-0.4.0'
$report='\\Mac\Home\Documents\Codex\2026-10-30\new-chat\work\windows-port\vm-test.json'
$root=Join-Path $env:TEMP ('Horizon Enhanced Smoke '+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$env:HORIZON_HOME=Join-Path $root 'game'
$env:HORIZON_LAUNCHER_HOME=Join-Path $root 'launcher'
$env:XDG_DATA_HOME=Join-Path $root 'data'
$env:XDG_CONFIG_HOME=Join-Path $root 'config'
$env:HORIZON_NO_AUDIO='1'
$env:HORIZON_LAUNCHER_NO_SELF_UPDATE='1'
$env:HORIZON_RESOURCES='<WINDOWS_USER_HOME>\AppData\Roaming\horizon-launcher\versions\nightly-2026-10-31-9d979ca\.local\horizon-client\26.3'
New-Item -ItemType Directory -Path (Join-Path $env:HORIZON_HOME 'saves') -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $env:HORIZON_HOME 'saves\keep.txt'),'keep this world')
[IO.File]::WriteAllText((Join-Path $env:HORIZON_HOME 'horizon-graphics.txt'),"pickedShaderPack:BSL_Horizon_Balanced_v10.1.8.zip`nresolutionScale:70`n")
[IO.File]::WriteAllText((Join-Path $env:HORIZON_HOME 'options.txt'),"renderDistance:2`nmaxFps:30`nenableVsync:false`n")
$launcher=Join-Path $bundle 'Launch-Horizon.ps1'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $launcher --launch-api
$apiExit=$LASTEXITCODE
$result=@{root=$root;api_exit=$apiExit;home_with_spaces=$true;resources=$env:HORIZON_RESOURCES;architecture=[Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()}
$result.profile_migrated=([IO.File]::ReadAllText((Join-Path $env:HORIZON_HOME 'horizon-graphics.txt')) -match 'BSL_Horizon_Balanced_r2_v10.1.8.zip')
$result.scale_preserved=([IO.File]::ReadAllText((Join-Path $env:HORIZON_HOME 'horizon-graphics.txt')) -match 'resolutionScale:70')
$result.world_sentinel_preserved=([IO.File]::ReadAllText((Join-Path $env:HORIZON_HOME 'saves\keep.txt')) -eq 'keep this world')
$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $report -Encoding UTF8
if ($apiExit -ne 0) { throw 'Launch API failed' }
# Run only a new temporary test world, never the current player world.
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $launcher --world WindowsSmoke --creative --test-terrain --seed 123 --render-distance 2 --offscreen --width 640 --height 360 --frames 120 --timeout-seconds 60 --control-socket (Join-Path $root 'control.json')
$result.game_exit=$LASTEXITCODE
$logs=Join-Path $env:HORIZON_HOME 'logs'
$runtime=Get-ChildItem -LiteralPath $logs -Filter '*.stderr.log' | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
if ($runtime) { $result.runtime_log=[IO.File]::ReadAllText($runtime.FullName); Copy-Item -LiteralPath $runtime.FullName -Destination '\\Mac\Home\Documents\Codex\2026-10-30\new-chat\work\windows-port\vm-runtime.log' }
$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $report -Encoding UTF8
$result | ConvertTo-Json -Depth 8
