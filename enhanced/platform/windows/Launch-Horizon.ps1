# Horizon Enhanced private Windows bootstrap. Windows PowerShell 5.1 compatible.
# No downloads, resource preparation, or changes to another launcher's data.
$ErrorActionPreference = 'Stop'
$GameArguments = @($args)
$BuildVersion = '0.4.0-windows.1'
$Utf8 = New-Object System.Text.UTF8Encoding($false)
$LauncherLog = $null

function Write-Utf8File([string]$Path, [string]$Text) {
    [System.IO.File]::WriteAllText($Path, $Text, $Utf8)
}

function Write-LaunchLog([string]$Text) {
    $Line = '[' + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss') + '] ' + $Text
    if ($LauncherLog) { [System.IO.File]::AppendAllText($LauncherLog, $Line + "`r`n", $Utf8) }
    Write-Host $Line
}

# Follow CommandLineToArgvW/CRT escaping, including empty arguments, embedded quotes,
# and a trailing backslash in a quoted path. Do not interpolate arguments into cmd.exe.
function ConvertTo-NativeArgument([AllowEmptyString()][string]$Value) {
    if ($Value.Length -gt 0 -and $Value -notmatch '[\s"]') { return $Value }
    $Result = New-Object System.Text.StringBuilder
    [void]$Result.Append('"')
    $Slashes = 0
    foreach ($Character in $Value.ToCharArray()) {
        if ($Character -eq '\') { $Slashes++; continue }
        if ($Character -eq '"') {
            [void]$Result.Append(('\' * ($Slashes * 2 + 1)))
        } elseif ($Slashes -gt 0) {
            [void]$Result.Append(('\' * $Slashes))
        }
        $Slashes = 0
        [void]$Result.Append($Character)
    }
    [void]$Result.Append(('\' * ($Slashes * 2)))
    [void]$Result.Append('"')
    return $Result.ToString()
}

function Find-PreparedResources {
    if (-not [string]::IsNullOrWhiteSpace($env:HORIZON_RESOURCES)) {
        $Override = [System.IO.Path]::GetFullPath($env:HORIZON_RESOURCES)
        if (-not (Test-Path -LiteralPath (Join-Path $Override 'items.json') -PathType Leaf)) {
            throw ('HORIZON_RESOURCES does not contain items.json: ' + $Override)
        }
        return $Override
    }
    $LauncherHome = if (-not [string]::IsNullOrWhiteSpace($env:HORIZON_LAUNCHER_HOME)) {
        [System.IO.Path]::GetFullPath($env:HORIZON_LAUNCHER_HOME)
    } else { Join-Path $env:APPDATA 'horizon-launcher' }
    $Versions = Join-Path $LauncherHome 'versions'
    if (Test-Path -LiteralPath $Versions -PathType Container) {
        $Candidates = @(Get-ChildItem -LiteralPath $Versions -Directory |
            Sort-Object LastWriteTimeUtc -Descending)
        foreach ($Version in $Candidates) {
            $Prepared = Join-Path $Version.FullName '.local\horizon-client\26.3'
            if (Test-Path -LiteralPath (Join-Path $Prepared 'items.json') -PathType Leaf) {
                return $Prepared
            }
        }
    }
    throw ('Prepared Minecraft 26.3 resources were not found under ' + $Versions +
        '. Open the official Horizon launcher and prepare Minecraft 26.3 first, or set HORIZON_RESOURCES to an existing prepared directory. This package does not include game assets.')
}

try {
    if ([string]::IsNullOrWhiteSpace($env:APPDATA)) { throw 'APPDATA is not defined.' }
    $BundleRoot = $PSScriptRoot
    $Client = Join-Path $BundleRoot 'horizon-client.exe'
    $Resources = Join-Path $BundleRoot 'resources'
    if (-not (Test-Path -LiteralPath $Client -PathType Leaf)) {
        throw ('Missing horizon-client.exe. Extract the whole ZIP before launching: ' + $BundleRoot)
    }
    $GameHome = if (-not [string]::IsNullOrWhiteSpace($env:HORIZON_HOME)) {
        [System.IO.Path]::GetFullPath($env:HORIZON_HOME)
    } else { Join-Path $env:APPDATA 'Horizon Enhanced' }
    [void][System.IO.Directory]::CreateDirectory($GameHome)
    $Logs = Join-Path $GameHome 'logs'
    [void][System.IO.Directory]::CreateDirectory($Logs)
    $LauncherLog = Join-Path $Logs 'launcher.log'
    Write-LaunchLog ('Horizon Enhanced ' + $BuildVersion + '; game data: ' + $GameHome)
    $Prepared = Find-PreparedResources
    Write-LaunchLog ('Prepared resources: ' + $Prepared)

    $PackDirectory = Join-Path $GameHome 'shaderpacks'
    [void][System.IO.Directory]::CreateDirectory($PackDirectory)
    foreach ($Name in @('BSL_v10.1.8.zip', 'BSL_Horizon_Quality_r2_v10.1.8.zip', 'BSL_Horizon_Balanced_r2_v10.1.8.zip')) {
        $Destination = Join-Path $PackDirectory $Name
        if (-not (Test-Path -LiteralPath $Destination)) {
            $Source = Join-Path $Resources $Name
            if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw ('Missing bundled shader pack: ' + $Source) }
            Copy-Item -LiteralPath $Source -Destination $Destination
        }
    }
    $Graphics = Join-Path $GameHome 'horizon-graphics.txt'
    if (-not (Test-Path -LiteralPath $Graphics)) {
        Write-Utf8File $Graphics "pickedShaderPack:BSL_Horizon_Quality_r2_v10.1.8.zip`nresolutionScale:75`n"
    }
    $Marker = Join-Path $GameHome 'horizon-enhanced-render-version.txt'
    if (-not (Test-Path -LiteralPath $Marker) -or ([System.IO.File]::ReadAllText($Marker).Trim() -ne $BuildVersion)) {
        $Choices = @{
            'pickedShaderPack:BSL_v10.1.8.zip' = 'pickedShaderPack:BSL_Horizon_Quality_r2_v10.1.8.zip'
            'pickedShaderPack:BSL_Horizon_Quality_v10.1.8.zip' = 'pickedShaderPack:BSL_Horizon_Quality_r2_v10.1.8.zip'
            'pickedShaderPack:BSL_Horizon_Balanced_v10.1.8.zip' = 'pickedShaderPack:BSL_Horizon_Balanced_r2_v10.1.8.zip'
        }
        $Lines = [System.IO.File]::ReadAllLines($Graphics)
        $Changed = $false
        $Updated = foreach ($Line in $Lines) {
            if ($Choices.ContainsKey($Line)) { $Changed = $true; $Choices[$Line] } else { $Line }
        }
        if ($Changed) { Write-Utf8File $Graphics (($Updated -join "`n") + "`n") }
        Write-Utf8File $Marker ($BuildVersion + "`n")
    }
    $Options = Join-Path $GameHome 'options.txt'
    if (-not (Test-Path -LiteralPath $Options)) {
        Write-Utf8File $Options "renderDistance:6`nmaxFps:60`nenableVsync:true`n"
    }
    $env:HORIZON_HOME = $GameHome
    $env:HORIZON_LAUNCHER_NO_SELF_UPDATE = '1'
    $LaunchArguments = @('--resources', $Prepared, '--game-dir', $GameHome, '--home', $GameHome, '--name', 'lol') + $GameArguments
    $NativeArguments = ($LaunchArguments | ForEach-Object { ConvertTo-NativeArgument ([string]$_) }) -join ' '
    $Stamp = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
    $StandardLog = Join-Path $Logs ('client-' + $Stamp + '.log')
    $ErrorLog = Join-Path $Logs ('client-' + $Stamp + '.stderr.log')
    Write-LaunchLog ('Starting renderer; client logs: ' + $StandardLog + ' and ' + $ErrorLog)
    $Process = Start-Process -FilePath $Client -ArgumentList $NativeArguments -WorkingDirectory $GameHome `
        -RedirectStandardOutput $StandardLog -RedirectStandardError $ErrorLog -NoNewWindow -Wait -PassThru
    Write-LaunchLog ('Renderer exited with code ' + $Process.ExitCode)
    if ($Process.ExitCode -ne 0) {
        Write-Host ('The renderer stopped. Check the logs above. On a real PC install a graphics driver supporting DirectX 12 or Vulkan.')
    }
    exit $Process.ExitCode
} catch {
    Write-LaunchLog ('Launch failed: ' + $_.Exception.Message)
    if ($LauncherLog) { Write-Host ('Launcher log: ' + $LauncherLog) }
    exit 1
}
