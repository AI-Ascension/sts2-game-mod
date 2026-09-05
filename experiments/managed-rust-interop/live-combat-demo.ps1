# SPDX-License-Identifier: MIT
param(
    [Parameter(Mandatory=$true)][string]$HostDirectory,
    [Parameter(Mandatory=$true)][string]$UserDirectory,
    [Parameter(Mandatory=$true)][string]$LogPath,
    [Parameter(Mandatory=$true)][string]$StopFile,
    [string]$Seed = 'AIASCENSIONREPLAY1',
    [ValidateRange(1,65535)][int]$Port = 15626,
    [ValidateRange(-1,31)][int]$Display = -1,
    [ValidateRange(640,16384)][int]$Width = 1280,
    [ValidateRange(360,16384)][int]$Height = 720,
    [ValidateSet('windowed','fullscreen','borderless','maximized')][string]$WindowMode = 'windowed'
)
$ErrorActionPreference = 'Stop'
if ($LogPath.Contains('"') -or $LogPath.Contains("`r") -or $LogPath.Contains("`n")) { throw 'Invalid log path' }
Add-Type -AssemblyName System.Windows.Forms
if ($Display -ge [System.Windows.Forms.Screen]::AllScreens.Count) { throw 'Selected display is unavailable' }
if (-not (Test-Path "$HostDirectory\override.cfg")) { throw 'Isolated override is required' }
if (Get-Process SlayTheSpire2 -ErrorAction SilentlyContinue) { throw 'Another game is running' }
$token = [Console]::ReadLine()
if ($token -notmatch '^[A-Za-z0-9_-]{43,256}$') { throw 'Invalid session credential' }
$env:STS2_RUNTIME_TOKEN = $token
$env:STS2_RUNTIME_BIND_ADDRESS = '127.0.0.1'
$env:STS2_RUNTIME_PORT = "$Port"
$env:STS2_RUNTIME_SESSION = '1'
$env:STS2_LIVE_COMBAT = '1'
$env:STS2_LIVE_USER_DIR = $UserDirectory
$env:STS2_LIVE_SEED = $Seed
$env:STS2_LIVE_DISPLAY = "$Display"
$env:STS2_LIVE_WIDTH = "$Width"
$env:STS2_LIVE_HEIGHT = "$Height"
$env:STS2_LIVE_WINDOW_MODE = $WindowMode
$arguments = @('--resolution', "${Width}x${Height}", '--audio-driver', 'Dummy', '--log-file', ('"' + $LogPath + '"'))
if ($Display -ge 0) { $arguments += @('--screen', "$Display") }
switch ($WindowMode) {
    'fullscreen' { $arguments += '--fullscreen' }
    'maximized' { $arguments += '--maximized' }
    'borderless' { $arguments += @('--windowed', '--borderless') }
    default { $arguments += '--windowed' }
}
$owned = Start-Process -FilePath "$HostDirectory\SlayTheSpire2.exe" -WorkingDirectory $HostDirectory -ArgumentList $arguments -PassThru
try {
    Write-Output "OWNED_GAME_PID=$($owned.Id)"
    $deadline = [DateTime]::UtcNow.AddMinutes(15)
    while (-not $owned.WaitForExit(500)) {
        if ((Test-Path $StopFile) -or [DateTime]::UtcNow -gt $deadline) { break }
    }
} finally {
    if (-not $owned.HasExited) { $owned.Kill(); $owned.WaitForExit() }
    Write-Output 'OWNED_GAME_STOPPED=true'
    $owned.Dispose()
    Remove-Item Env:STS2_RUNTIME_TOKEN
}
