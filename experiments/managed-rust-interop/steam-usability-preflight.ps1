# SPDX-License-Identifier: MIT
# Read-only Steam usability classification for the disposable live launcher.
$ErrorActionPreference = 'Stop'

function Emit-State([string]$State) {
    @{ state = $State } | ConvertTo-Json -Compress
    exit 0
}

# These checks intentionally never start, stop, log into, or configure Steam. They emit only a
# fixed state token: no installation paths, account names, process IDs, or pipe details leave this
# script.
$steamKey = Get-ItemProperty -Path 'HKCU:\Software\Valve\Steam' -ErrorAction SilentlyContinue
$steamCommand = Get-Command 'steam.exe' -ErrorAction SilentlyContinue
$clientPresent = $null -ne $steamCommand -or
    ($null -ne $steamKey -and -not [string]::IsNullOrWhiteSpace([string]$steamKey.SteamPath))
if (-not $clientPresent) { Emit-State 'absent_client' }

$steamProcesses = @(Get-Process -Name 'steam' -ErrorAction SilentlyContinue)
if ($steamProcesses.Count -eq 0) { Emit-State 'process_only_unready' }

# ActiveProcess is an unbound registry indicator, not proof of account, API, IPC, game, or
# launcher usability for the observed process.
$activeProcess = Get-ItemProperty -Path 'HKCU:\Software\Valve\Steam\ActiveProcess' -ErrorAction SilentlyContinue
$accountIndicated = $null -ne $activeProcess -and $null -ne $activeProcess.ActiveUser -and
    [string]$activeProcess.ActiveUser -match '^[1-9][0-9]*$'
if ($accountIndicated) { Emit-State 'process_present_account_indicated' }
Emit-State 'process_present_account_unverified'
