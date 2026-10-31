Horizon Enhanced 0.4.0 - Windows x64 bootstrap

Extract the entire ZIP, then double-click Launch-Horizon.cmd.
No Python or Rust installation is required to launch the compiled package.

Expected package layout:
  Launch-Horizon.cmd
  Launch-Horizon.ps1
  horizon-client.exe
  horizon-ctl.exe
  resources\BSL_v10.1.8.zip
  resources\BSL_Horizon_Quality_r2_v10.1.8.zip
  resources\BSL_Horizon_Balanced_r2_v10.1.8.zip

Prepare Minecraft 26.3 once using the official Horizon launcher on Windows.
This bootstrap searches newest prepared version directories beneath:
  %APPDATA%\horizon-launcher\versions\*\.local\horizon-client\26.3
The candidate must contain items.json. If you use a custom launcher home,
set HORIZON_LAUNCHER_HOME, or point HORIZON_RESOURCES directly at the prepared
26.3 directory. No game assets are downloaded or supplied by this package.

Enhanced uses its own data directory:
  %APPDATA%\Horizon Enhanced
It does not copy or change original Minecraft/official Horizon worlds.
Set HORIZON_HOME before launching to choose another Enhanced data directory.
Only missing shader packs and missing default preference files are created.
Known old BSL profile choices migrate once to the matching r2 choice; custom
packs, existing options, render scale and saved worlds remain untouched.
Upgrader is not loaded.

Logs:
  %APPDATA%\Horizon Enhanced\logs\launcher.log
  %APPDATA%\Horizon Enhanced\logs\client-DATE-TIME.log
  %APPDATA%\Horizon Enhanced\logs\client-DATE-TIME.stderr.log
The location follows HORIZON_HOME if overridden.

Arguments after Launch-Horizon.cmd are forwarded to the game, for example:
  Launch-Horizon.cmd --control-socket "%TEMP%\horizon-enhanced-control.json"
  horizon-ctl.exe --socket "%TEMP%\horizon-enhanced-control.json" state
Do not connect agent tools to a different game's socket by accident.

This Windows build needs testing with your actual graphics driver. A VM using
Microsoft Basic Render Driver/software rendering cannot represent real PC FPS.
The renderer changes retain their previous macOS test results, but those do not
prove Windows GPU correctness or performance.
