param(
    [Parameter(Mandatory = $true)][string]$ExecutablePath,
    [Parameter(Mandatory = $true)][string]$StagePath,
    [Parameter(Mandatory = $true)][string]$BackupPath,
    [ValidateSet('Inspect', 'AssertStopped', 'Stop')][string]$Mode = 'AssertStopped',
    [ValidateRange(0, 600)][int]$WaitSeconds = 20,
    [Parameter(Mandatory = $true)][long]$DeadlineEpoch
)

$ErrorActionPreference = 'Stop'

function Assert-Deadline {
    if ([DateTimeOffset]::UtcNow.ToUnixTimeSeconds() -ge $DeadlineEpoch) {
        throw 'LIVE_AUTHORIZATION deadline passed'
    }
}

function Assert-NoReparseAncestor([string]$Path) {
    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw 'Reparse points are not accepted in installation or staging paths'
            }
        }
        $parent = [IO.Path]::GetDirectoryName($current)
        if ($parent -eq $current) { break }
        $current = $parent
    }
}

function Assert-DisjointPaths([string]$Left, [string]$Right) {
    $separator = [IO.Path]::DirectorySeparatorChar
    $a = [IO.Path]::GetFullPath($Left).TrimEnd($separator)
    $b = [IO.Path]::GetFullPath($Right).TrimEnd($separator)
    $comparison = [StringComparison]::OrdinalIgnoreCase
    if ($a.Equals($b, $comparison) -or $a.StartsWith($b + $separator, $comparison) -or
            $b.StartsWith($a + $separator, $comparison)) {
        throw 'Game installation, staging, and backup paths must not overlap'
    }
}

Assert-Deadline
$target = [IO.Path]::GetFullPath($ExecutablePath)
Assert-DisjointPaths $StagePath ([IO.Path]::GetDirectoryName($target))
Assert-DisjointPaths $BackupPath ([IO.Path]::GetDirectoryName($target))
Assert-DisjointPaths $StagePath $BackupPath
Assert-NoReparseAncestor $target
Assert-NoReparseAncestor ([IO.Path]::Combine([IO.Path]::GetDirectoryName($target), 'mods'))
Assert-NoReparseAncestor $StagePath
Assert-NoReparseAncestor $BackupPath
foreach ($artifact in @('AIAscensionSTS2GameMod.dll', 'AIAscensionSTS2GameModNative.dll', 'AIAscensionSTS2GameMod.json')) {
    Assert-NoReparseAncestor ([IO.Path]::Combine([IO.Path]::GetDirectoryName($target), 'mods', $artifact))
    Assert-NoReparseAncestor ([IO.Path]::Combine($StagePath, $artifact))
}

# Inspect every same-name process before taking any action. A process whose
# executable cannot be inspected is not evidence that the installation is idle.
# Retain Process objects (and their opened handles), not unverified reusable PIDs.
$selected = @()
try {
    foreach ($process in @(Get-Process -ErrorAction Stop)) {
        if ($process.ProcessName -ine 'SlayTheSpire2') { continue }
        $null = $process.Handle
        $path = $process.MainModule.FileName
        if ([string]::IsNullOrWhiteSpace($path)) {
            throw 'Cannot establish executable identity of a running game process'
        }
        if ([string]::Equals([IO.Path]::GetFullPath($path), $target,
                [StringComparison]::OrdinalIgnoreCase)) {
            $selected += $process
        } else {
            $process.Dispose()
        }
    }
    if ($Mode -eq 'Inspect') { return }
    if ($Mode -eq 'AssertStopped' -and $selected.Count -gt 0) {
        throw 'The selected game installation is running; refusing to replace its addon'
    }
    foreach ($process in $selected) {
        Assert-Deadline
        # Kill only this inspected process, not all matching image names or an
        # uninspected descendant tree. Never ignore a termination failure.
        if (-not $process.HasExited) { $process.Kill() }
    }
    $end = [DateTimeOffset]::UtcNow.AddSeconds($WaitSeconds)
    foreach ($process in $selected) {
        Assert-Deadline
        $remaining = [Math]::Max(0, ($end - [DateTimeOffset]::UtcNow).TotalMilliseconds)
        $authorized = [Math]::Max(0, ($DeadlineEpoch - [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()) * 1000)
        if (-not $process.WaitForExit([int][Math]::Min($remaining, $authorized))) {
            throw 'Selected game process did not exit within the authorized timeout'
        }
    }
} finally {
    foreach ($process in $selected) { $process.Dispose() }
}
