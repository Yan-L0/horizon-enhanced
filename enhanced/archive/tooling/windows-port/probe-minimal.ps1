$errors=$null;$tokens=$null
[System.Management.Automation.Language.Parser]::ParseInput('Write-Host ok',[ref]$tokens,[ref]$errors) | Out-Null
'PARSER OK'
(Get-Item '<WINDOWS_USER_HOME>\AppData\Roaming\horizon-launcher\versions\nightly-2026-10-31-9d979ca\.local\horizon-client\26.3\items.json').Length
