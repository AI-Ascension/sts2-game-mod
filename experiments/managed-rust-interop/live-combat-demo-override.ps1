# SPDX-License-Identifier: MIT
#
# Resolves the isolated user directory a disposable host is configured to use, before anything is
# launched.
#
# The mod refuses to initialise unless the game's resolved user directory equals the directory the
# launcher declared in STS2_LIVE_USER_DIR. It can only report that after the game has started, and
# the resolved directory is decided here -- by override.cfg plus the launch-scoped APPDATA root --
# not by the launcher argument. When the two disagree the operator learned "live demo requires its
# isolated user directory" and nothing about which directory the game examined (sts2-game-mod#173).
#
# This file is dot-sourced by live-combat-demo.ps1 and by its tests. It launches nothing.

Set-StrictMode -Version Latest

function Get-NormalizedUserDirectory {
    <#
      Absolute-form normalization, except that trailing separators are removed on every platform.
      [System.IO.Path]::GetFullPath keeps a trailing separator on Unix and removes it on Windows,
      which would otherwise make the same declaration agree on one platform and not another.
    #>
    param([Parameter(Mandatory = $true)][string]$Directory)

    $full = [System.IO.Path]::GetFullPath($Directory)
    $root = [System.IO.Path]::GetPathRoot($full)
    if ($full.Length -gt $root.Length) {
        $separators = [char[]]@(
            [System.IO.Path]::DirectorySeparatorChar,
            [System.IO.Path]::AltDirectorySeparatorChar)
        return $full.TrimEnd($separators)
    }
    return $full
}

function Get-IsolatedUserDirectory {
    <#
      Returns the absolute directory the host's override.cfg makes the game resolve, or throws with
      a stable reason token. The reason tokens are the same ones the mod reports:
      overrides_absent, override_not_custom, override_unnamed, override_unusable_name,
      isolated_user_dir_unset, isolated_user_dir_mismatch.
    #>
    param(
        [Parameter(Mandatory = $true)][string]$HostDirectory,
        [string]$DeclaredUserDirectory = ''
    )

    $overridePath = Join-Path $HostDirectory 'override.cfg'
    if (-not (Test-Path -LiteralPath $overridePath)) {
        throw "overrides_absent: no override.cfg in '$HostDirectory', so the game would use the shared user directory"
    }

    $text = Get-Content -LiteralPath $overridePath -Raw
    if ($text -notmatch '(?m)^\s*config/use_custom_user_dir\s*=\s*true\s*$') {
        throw "override_not_custom: override.cfg does not set config/use_custom_user_dir=true, so the game would use the shared user directory"
    }

    $named = [regex]::Match($text, '(?m)^\s*config/custom_user_dir_name\s*=\s*"([^"]+)"\s*$')
    if (-not $named.Success) {
        throw "override_unnamed: override.cfg does not name config/custom_user_dir_name, so the isolated user directory is undetermined"
    }

    $name = $named.Groups[1].Value
    if ([string]::IsNullOrWhiteSpace($name) -or $name -match '[\\/]' -or $name -match '[\x00-\x1f]') {
        throw "override_unusable_name: override.cfg names a config/custom_user_dir_name that is not a single safe directory name"
    }

    $resolved = Get-NormalizedUserDirectory (Join-Path -Path $env:APPDATA -ChildPath $name)

    if ([string]::IsNullOrWhiteSpace($DeclaredUserDirectory)) {
        throw "isolated_user_dir_unset: the game resolves its user directory to '$resolved' but no -UserDirectory was declared. Pass that directory as -UserDirectory."
    }

    $declared = Get-NormalizedUserDirectory $DeclaredUserDirectory
    if (-not [string]::Equals($resolved, $declared, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "isolated_user_dir_mismatch: the game resolves its user directory to '$resolved' but -UserDirectory declared '$declared'. config/custom_user_dir_name in override.cfg and the launch-scoped APPDATA root decide the resolved path."
    }

    return $resolved
}
