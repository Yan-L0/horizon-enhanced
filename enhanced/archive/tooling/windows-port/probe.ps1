$ErrorActionPreference='Stop'
whoami
[System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$env:APPDATA
$errors=$null;$tokens=$null
[System.Management.Automation.Language.Parser]::ParseFile('\\Mac\Home\Documents\Codex\2026-10-30\new-chat\work\windows-port\Launch-Horizon.ps1',[ref]$tokens,[ref]$errors) | Out-Null
$errors | Format-List
Get-ChildItem '<WINDOWS_USER_HOME>\AppData\Roaming\horizon-launcher\versions\nightly-2026-10-31-9d979ca\.local\horizon-client\26.3' | Select-Object -First 8 Name
