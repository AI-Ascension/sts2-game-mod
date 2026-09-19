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
#
# `HKCU:` belongs to the caller. When the guest agent launches this script it runs in session 0 as
# a service account whose hive has no Steam key, so reading `HKCU:` alone reported `absent_client`
# for an installation that was present and running for the logged-in operator. Enumerate every
# loaded interactive user hive as well: the classification describes the interactive session, not
# the caller.
function Get-SteamKeyRoots {
    $roots = @('HKCU:')
    $hives = Get-ChildItem -Path 'Registry::HKEY_USERS' -ErrorAction SilentlyContinue
    foreach ($hive in @($hives)) {
        if ($hive.PSChildName -match '^S-1-5-21-') {
            $roots += "Registry::HKEY_USERS\$($hive.PSChildName)"
        }
    }
    return $roots
}

$steamCommand = Get-Command 'steam.exe' -ErrorAction SilentlyContinue
$clientPresent = $null -ne $steamCommand
$accountIndicated = $false
foreach ($root in Get-SteamKeyRoots) {
    $steamKey = Get-ItemProperty -Path "$root\Software\Valve\Steam" -ErrorAction SilentlyContinue
    if ($null -ne $steamKey -and -not [string]::IsNullOrWhiteSpace([string]$steamKey.SteamPath)) {
        $clientPresent = $true
    }
    # ActiveProcess is an unbound registry indicator, not proof of account, API, IPC, game, or
    # launcher usability for the observed process.
    $activeProcess = Get-ItemProperty -Path "$root\Software\Valve\Steam\ActiveProcess" -ErrorAction SilentlyContinue
    if ($null -ne $activeProcess -and $null -ne $activeProcess.ActiveUser -and
        [string]$activeProcess.ActiveUser -match '^[1-9][0-9]*$') {
        $accountIndicated = $true
    }
}
if (-not $clientPresent) { Emit-State 'absent_client' }

$steamProcesses = @(Get-Process -Name 'steam' -ErrorAction SilentlyContinue)
if ($steamProcesses.Count -eq 0) { Emit-State 'process_only_unready' }

if ($accountIndicated) { Emit-State 'process_present_account_indicated' }
Emit-State 'process_present_account_unverified'
