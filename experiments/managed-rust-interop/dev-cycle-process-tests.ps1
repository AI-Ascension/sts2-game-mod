# Synthetic process objects only: no real process is queried, started, or killed.
$ErrorActionPreference = 'Stop'
$helper = Join-Path $PSScriptRoot 'dev-cycle-process.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('sts2-process-test-' + [Guid]::NewGuid())
$null = New-Item -ItemType Directory -Path $root
$target = Join-Path $root 'host/SlayTheSpire2.exe'
$other = Join-Path $root 'other/SlayTheSpire2.exe'
$global:sts2TestProcesses = @()
$global:sts2TestInspectionFails = $false
function Get-Process {
    param([string]$ErrorAction)
    if ($global:sts2TestInspectionFails) { throw 'synthetic inspection denied' }
    return $global:sts2TestProcesses
}
function New-FakeProcess([string]$Path, [string]$Name = 'SlayTheSpire2') {
    $process = [PSCustomObject]@{
        ProcessName = $Name; Handle = 1; MainModule = [PSCustomObject]@{ FileName = $Path }
        HasExited = $false; Killed = $false; Disposed = $false
        KillFails = $false; ExitSucceeds = $true
    }
    $process | Add-Member ScriptMethod Kill {
        if ($this.KillFails) { throw 'synthetic kill denied' }
        $this.Killed = $true
    }
    $process | Add-Member ScriptMethod WaitForExit { param([int]$Timeout) return $this.ExitSucceeds }
    $process | Add-Member ScriptMethod Dispose { $this.Disposed = $true }
    return $process
}
function Invoke-Guard([string]$Mode, [long]$Deadline = ([DateTimeOffset]::UtcNow.ToUnixTimeSeconds() + 60)) {
    & $helper -ExecutablePath $target -StagePath (Join-Path $root 'stage') `
        -BackupPath (Join-Path $root 'backup') -Mode $Mode -WaitSeconds 0 -DeadlineEpoch $Deadline
}
function Assert-That([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
function Assert-Refused([scriptblock]$Action, [string]$Expected) {
    try { & $Action } catch {
        if ($_.ToString() -notlike "*$Expected*") { throw }
        return
    }
    throw "Expected refusal: $Expected"
}
try {
    $selected = New-FakeProcess $target
    $unrelated = New-FakeProcess $other
    $nonGame = New-FakeProcess '' 'Unrelated'
    $global:sts2TestProcesses = @($selected, $unrelated, $nonGame)
    Invoke-Guard Stop
    Assert-That $selected.Killed 'Selected installation was not stopped'
    Assert-That (-not $unrelated.Killed -and -not $nonGame.Killed) 'Unrelated process was stopped'
    $selected = New-FakeProcess $target
    $global:sts2TestProcesses = @($selected)
    Assert-Refused { Invoke-Guard AssertStopped } 'selected game installation is running'
    Assert-That (-not $selected.Killed) 'AssertStopped killed a process'
    Invoke-Guard Inspect
    Assert-That (-not $selected.Killed) 'Inspect killed a process'
    $global:sts2TestProcesses = @((New-FakeProcess $other))
    Invoke-Guard AssertStopped
    $global:sts2TestInspectionFails = $true
    Assert-Refused { Invoke-Guard Stop } 'synthetic inspection denied'
    $global:sts2TestInspectionFails = $false
    $selected = New-FakeProcess $target
    $global:sts2TestProcesses = @($selected, (New-FakeProcess ''))
    Assert-Refused { Invoke-Guard Stop } 'Cannot establish executable identity'
    Assert-That (-not $selected.Killed) 'Mutation preceded complete process inspection'
    $global:sts2TestProcesses = @($selected)
    Assert-Refused { Invoke-Guard Stop 1 } 'deadline passed'
    Assert-That (-not $selected.Killed) 'Expired authorization stopped a process'
    $selected.KillFails = $true
    Assert-Refused { Invoke-Guard Stop } 'synthetic kill denied'
    $selected.KillFails = $false
    $selected.ExitSucceeds = $false
    Assert-Refused { Invoke-Guard Stop } 'did not exit'
    $global:sts2TestProcesses = @()
    Invoke-Guard AssertStopped
    Assert-Refused {
        & $helper -ExecutablePath $target -StagePath ([IO.Path]::GetDirectoryName($target)) `
            -BackupPath (Join-Path $root 'backup') -Mode Inspect -DeadlineEpoch ([DateTimeOffset]::UtcNow.ToUnixTimeSeconds() + 60)
    } 'must not overlap'
    Write-Output 'PASS: selected-installation identity, no-kill, inspection failure, expiry, termination failure, and timeout'
} finally {
    Remove-Variable sts2TestProcesses, sts2TestInspectionFails -Scope Global
    Remove-Item -LiteralPath $root -Recurse -Force
}
