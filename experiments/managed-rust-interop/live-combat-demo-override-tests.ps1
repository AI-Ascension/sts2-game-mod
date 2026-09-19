# SPDX-License-Identifier: MIT
#
# Source-only checks for the isolated-user-directory preflight. These run without a game and
# without a disposable host: they exercise the override.cfg resolution that decides whether the
# Windows launch contract agrees with itself (sts2-game-mod#173).

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot 'live-combat-demo-override.ps1')

function Fail([string]$Message) {
    throw "FAIL: $Message"
}

function Check([bool]$Condition, [string]$Message) {
    if (-not $Condition) { Fail $Message }
    Write-Output "PASS: $Message"
}

function New-HostDirectory([string]$OverrideText) {
    $dir = Join-Path ([System.IO.Path]::GetTempPath()) ("sts2-override-" + [guid]::NewGuid().ToString('n'))
    $null = New-Item -ItemType Directory -Force -Path $dir
    if (-not [string]::IsNullOrEmpty($OverrideText)) {
        Set-Content -LiteralPath (Join-Path $dir 'override.cfg') -Value $OverrideText -Encoding UTF8
    }
    return $dir
}

function Expect-Refusal([string]$Reason, [string]$HostDirectory, [string]$Declared) {
    try {
        $null = Get-IsolatedUserDirectory -HostDirectory $HostDirectory -DeclaredUserDirectory $Declared
    } catch {
        $message = $_.Exception.Message
        if (-not $message.StartsWith($Reason + ':', [StringComparison]::Ordinal)) {
            Fail "expected '$Reason' but got '$message'"
        }
        Write-Host "PASS: $Reason is refused with a named reason"
        return $message
    }
    Fail "expected '$Reason' but the preflight admitted the host"
}

$custom = @"
[application]
config/use_custom_user_dir=true
config/custom_user_dir_name="profile"
"@

try {
    $host_ = New-HostDirectory $custom
    $agreed = Join-Path $env:APPDATA 'profile'
    Check ((Get-IsolatedUserDirectory -HostDirectory $host_ -DeclaredUserDirectory $agreed) -eq
        [System.IO.Path]::GetFullPath($agreed)) "an agreeing declaration returns the resolved directory"
    Check ((Get-IsolatedUserDirectory -HostDirectory $host_ -DeclaredUserDirectory $agreed.ToUpperInvariant()) -eq
        [System.IO.Path]::GetFullPath($agreed)) "the comparison is case-insensitive"
    $trailing = "$agreed" + [System.IO.Path]::DirectorySeparatorChar
    Check ((Get-IsolatedUserDirectory -HostDirectory $host_ -DeclaredUserDirectory $trailing) -eq
        [System.IO.Path]::GetFullPath($agreed)) "the comparison tolerates a trailing separator"

    $mismatch = Expect-Refusal 'isolated_user_dir_mismatch' $host_ (Join-Path $env:APPDATA 'elsewhere')
    Check ($mismatch.Contains($agreed, [StringComparison]::Ordinal)) "a mismatch names the resolved directory"
    Check ($mismatch.Contains('elsewhere', [StringComparison]::Ordinal)) "a mismatch names the declared directory"

    $unset = Expect-Refusal 'isolated_user_dir_unset' $host_ ''
    Check ($unset.Contains($agreed, [StringComparison]::Ordinal)) "an unset declaration names the resolved directory"

    $overrideless = New-HostDirectory $null
    $null = Expect-Refusal 'overrides_absent' $overrideless (Join-Path $env:APPDATA 'profile')

    $notCustom = New-HostDirectory @"
[application]
config/use_custom_user_dir=false
config/custom_user_dir_name="profile"
"@
    $null = Expect-Refusal 'override_not_custom' $notCustom (Join-Path $env:APPDATA 'profile')

    $unnamed = New-HostDirectory @"
[application]
config/use_custom_user_dir=true
"@
    $null = Expect-Refusal 'override_unnamed' $unnamed (Join-Path $env:APPDATA 'profile')

    $traversal = New-HostDirectory @"
[application]
config/use_custom_user_dir=true
config/custom_user_dir_name="../escape"
"@
    $null = Expect-Refusal 'override_unusable_name' $traversal (Join-Path $env:APPDATA 'profile')

    Write-Output 'isolated user directory preflight checks passed'
} finally {
    Get-ChildItem ([System.IO.Path]::GetTempPath()) -Directory -Filter 'sts2-override-*' -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue }
}
