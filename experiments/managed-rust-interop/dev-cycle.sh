#!/usr/bin/env bash

set -Eeuo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(cd -- "$script_dir/../.." && pwd -P)
package_script="$script_dir/package-runtime-addon.sh"
authorization_script="$script_dir/live-authorization.sh"

game_dir_input=${STS2_GAME_DIR:-}
stage_dir=${STS2_RUNTIME_ADDON_STAGE_DIR:-"$repo_root/.sts2-dev/runtime-addon"}
backup_root=${STS2_RUNTIME_ADDON_BACKUP_DIR:-"$repo_root/.sts2-dev/backups"}
wait_seconds=${STS2_GAME_EXIT_TIMEOUT_SECONDS:-20}
stop_game=true
launch_game=true
backup_installed=true
dry_run=false
unlock_all_on_launch=false

usage() {
    printf '%s\n' \
        'Usage: dev-cycle.sh [options] [game-dir]' \
        '' \
        'Build, install, and restart the AI-Ascension STS2 runtime addon.' \
        'The game is stopped only after a successful build and is relaunched' \
        'from the same installation directory.' \
        '' \
        'Options:' \
        '  --game-dir PATH       STS2 install directory (or use STS2_GAME_DIR)' \
        '  --stage-dir PATH      ignored staging directory for packaged files' \
        '  --wait-seconds N      wait after taskkill (default: 20)' \
        '  --no-kill             leave the game running before installation' \
        '  --no-launch           do not relaunch after installation' \
        '  --no-backup           do not save replaced mod files before copying' \
        '  --unlock-all          pass the opt-in full-unlock flag on game launch' \
        '  --dry-run             show the cycle without building or changing files' \
        '  -h, --help            show this help' \
        '' \
        'Examples:' \
        '  ./experiments/managed-rust-interop/dev-cycle.sh' \
        '  ./experiments/managed-rust-interop/dev-cycle.sh --no-launch' \
        '  ./experiments/managed-rust-interop/dev-cycle.sh --game-dir "C:\\Games\\STS2"'
}

die() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

[[ -f "$authorization_script" ]] || die 'live authorization helper is missing'
source "$authorization_script"

take_value() {
    local option=$1
    if [[ $# -lt 2 || -z ${2:-} ]]; then
        die "$option requires a value"
    fi
    printf '%s' "$2"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --game-dir)
            game_dir_input=$(take_value "$1" "${2:-}")
            shift 2
            ;;
        --game-dir=*)
            game_dir_input=${1#*=}
            [[ -n "$game_dir_input" ]] || die '--game-dir requires a value'
            shift
            ;;
        --stage-dir)
            stage_dir=$(take_value "$1" "${2:-}")
            shift 2
            ;;
        --stage-dir=*)
            stage_dir=${1#*=}
            [[ -n "$stage_dir" ]] || die '--stage-dir requires a value'
            shift
            ;;
        --wait-seconds)
            wait_seconds=$(take_value "$1" "${2:-}")
            shift 2
            ;;
        --wait-seconds=*)
            wait_seconds=${1#*=}
            [[ -n "$wait_seconds" ]] || die '--wait-seconds requires a value'
            shift
            ;;
        --no-kill)
            stop_game=false
            shift
            ;;
        --no-launch)
            launch_game=false
            shift
            ;;
        --no-backup)
            backup_installed=false
            shift
            ;;
        --unlock-all|--ai-ascension-unlock-all)
            unlock_all_on_launch=true
            shift
            ;;
        --dry-run)
            dry_run=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            die "unknown option: $1"
            ;;
        *)
            if [[ -n "$game_dir_input" ]]; then
                die "unexpected argument: $1"
            fi
            game_dir_input=$1
            shift
            ;;
    esac
done

if [[ ! "$wait_seconds" =~ ^[0-9]+$ ]]; then
    die '--wait-seconds must be a non-negative integer'
fi

if [[ "$dry_run" != true ]]; then
    # Installation, stopping, and relaunching are live mutations. Require the
    # same explicit record as the single-instance runtime launcher before any
    # host path is resolved or any build/child action begins.
    validate_live_authorization
fi

to_wsl_path() {
    local input=$1
    if [[ "$input" == /* ]]; then
        printf '%s' "$input"
        return 0
    fi

    command -v wslpath >/dev/null 2>&1 \
        || die "wslpath is required to convert Windows path '$input'"
    wslpath -u -- "$input"
}

to_windows_path() {
    command -v wslpath >/dev/null 2>&1 \
        || die 'wslpath is required to launch the Windows game from WSL'
    wslpath -w -- "$1"
}

find_windows_tool() {
    local tool=$1
    local candidate

    if command -v "$tool" >/dev/null 2>&1; then
        command -v "$tool"
        return 0
    fi

    case "$tool" in
        tasklist.exe|taskkill.exe|cmd.exe)
            for candidate in \
                "/mnt/c/Windows/System32/$tool" \
                "/mnt/c/Windows/Sysnative/$tool"; do
                if [[ -x "$candidate" ]]; then
                    printf '%s' "$candidate"
                    return 0
                fi
            done
            ;;
        powershell.exe)
            for candidate in \
                '/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe' \
                '/mnt/c/Windows/Sysnative/WindowsPowerShell/v1.0/powershell.exe'; do
                if [[ -x "$candidate" ]]; then
                    printf '%s' "$candidate"
                    return 0
                fi
            done
            ;;
    esac

    return 1
}

if [[ -z "$game_dir_input" ]]; then
    for candidate in \
        '/mnt/c/Program Files (x86)/Steam/steamapps/common/Slay the Spire 2' \
        '/mnt/c/Program Files/Steam/steamapps/common/Slay the Spire 2'; do
        if [[ -d "$candidate" ]]; then
            game_dir_input=$candidate
            break
        fi
    done
fi

[[ -n "$game_dir_input" ]] \
    || die 'game directory is missing; pass --game-dir or set STS2_GAME_DIR'

game_dir=$(to_wsl_path "$game_dir_input")
[[ -d "$game_dir" ]] \
    || die "game directory does not exist: $game_dir_input"
game_dir=$(cd -- "$game_dir" && pwd -P)

game_data_dir="$game_dir/data_sts2_windows_x86_64"
game_exe="$game_dir/SlayTheSpire2.exe"
mods_dir="$game_dir/mods"

[[ -f "$game_exe" ]] \
    || die "SlayTheSpire2.exe was not found under: $game_dir"
[[ -f "$game_data_dir/sts2.dll" ]] \
    || die "sts2.dll was not found under: $game_data_dir"
[[ -f "$game_data_dir/GodotSharp.dll" ]] \
    || die "GodotSharp.dll was not found under: $game_data_dir"
[[ -f "$package_script" ]] \
    || die "packaging script was not found: $package_script"

managed_artifact='AIAscensionSTS2GameMod.dll'
native_artifact='AIAscensionSTS2GameModNative.dll'
manifest_artifact='AIAscensionSTS2GameMod.json'
artifacts=("$managed_artifact" "$native_artifact" "$manifest_artifact")

printf '%s\n' 'STS2 runtime addon cycle'
printf '  game:  %s\n' "$game_dir"
printf '  stage: %s\n' "$stage_dir"
printf '  stop:  %s\n' "$stop_game"
printf '  start: %s\n' "$launch_game"
printf '  unlock on launch: %s\n' "$unlock_all_on_launch"

if [[ "$dry_run" == true ]]; then
    printf '%s\n' 'dry-run: would build and stage the three addon artifacts.'
    if [[ "$stop_game" == true ]]; then
        printf '%s\n' 'dry-run: would stop SlayTheSpire2.exe after the build succeeds.'
    fi
    printf '%s\n' 'dry-run: would copy the staged DLLs and manifest into the game mods directory.'
    if [[ "$launch_game" == true ]]; then
        printf '%s\n' 'dry-run: would relaunch SlayTheSpire2.exe.'
        if [[ "$unlock_all_on_launch" == true ]]; then
            printf '%s\n' 'dry-run: would pass --ai-ascension-unlock-all to the game.'
        fi
    fi
    exit 0
fi

printf '%s\n' 'Building and staging addon artifacts...'
bash "$package_script" "$game_data_dir" "$stage_dir"

for artifact in "${artifacts[@]}"; do
    [[ -s "$stage_dir/$artifact" ]] \
        || die "expected staged artifact is missing or empty: $stage_dir/$artifact"
done

tasklist_cmd=''
taskkill_cmd=''
powershell_cmd=''
if [[ "$stop_game" == true || "$launch_game" == true ]]; then
    tasklist_cmd=$(find_windows_tool tasklist.exe) \
        || die 'tasklist.exe is unavailable; run this script from WSL on Windows'
fi
if [[ "$stop_game" == true ]]; then
    taskkill_cmd=$(find_windows_tool taskkill.exe) \
        || die 'taskkill.exe is unavailable; run this script from WSL on Windows'
fi
if [[ "$launch_game" == true ]]; then
    powershell_cmd=$(find_windows_tool powershell.exe) \
        || die 'Windows PowerShell is unavailable; cannot relaunch the game'
fi

game_is_running() {
    "$tasklist_cmd" /FI 'IMAGENAME eq SlayTheSpire2.exe' /NH 2>/dev/null \
        | tr -d '\r' \
        | awk '$1 == "SlayTheSpire2.exe" { found = 1 } END { exit(found ? 0 : 1) }'
}

if [[ "$stop_game" == true ]]; then
    if game_is_running; then
        printf '%s\n' 'Stopping SlayTheSpire2.exe...'
        "$taskkill_cmd" /IM SlayTheSpire2.exe /T /F >/dev/null 2>&1 || true

        for ((second = 0; second < wait_seconds; second++)); do
            if ! game_is_running; then
                break
            fi
            sleep 1
        done

        if game_is_running; then
            die "SlayTheSpire2.exe did not exit within ${wait_seconds}s"
        fi
        printf '%s\n' 'Game stopped.'
    else
        printf '%s\n' 'Game is not running.'
    fi
elif [[ "$launch_game" == true ]] && game_is_running; then
    printf '%s\n' 'warning: --no-kill was requested while the game is running; installation may be locked.' >&2
fi

mkdir -p "$mods_dir"

backup_dir=''
existing_files=()
if [[ "$backup_installed" == true ]]; then
    backup_dir="$backup_root/$(date -u +%Y%m%dT%H%M%SZ)"
    mkdir -p "$backup_dir"

    for artifact in "${artifacts[@]}"; do
        if [[ -f "$mods_dir/$artifact" ]]; then
            existing_files+=("$artifact")
            cp -p -- "$mods_dir/$artifact" "$backup_dir/$artifact"
        fi
    done
fi

restore_previous_install() {
    local artifact
    if [[ "$backup_installed" != true ]]; then
        return 0
    fi

    for artifact in "${artifacts[@]}"; do
        if [[ -f "$backup_dir/$artifact" ]]; then
            cp -f -- "$backup_dir/$artifact" "$mods_dir/$artifact"
        else
            for existing in "${existing_files[@]}"; do
                [[ "$existing" == "$artifact" ]] && continue 2
            done
            rm -f -- "$mods_dir/$artifact"
        fi
    done
}

install_failed=false
for artifact in "${artifacts[@]}"; do
    if ! cp -f -- "$stage_dir/$artifact" "$mods_dir/$artifact"; then
        install_failed=true
        break
    fi
done
if [[ "$install_failed" == true ]]; then
    restore_previous_install
    die 'copying the staged addon into the game mods directory failed; the previous files were restored when backed up'
fi

for artifact in "${artifacts[@]}"; do
    if ! cmp -s -- "$stage_dir/$artifact" "$mods_dir/$artifact"; then
        restore_previous_install
        die "installed artifact verification failed: $artifact"
    fi
done

printf 'Installed %d addon artifacts into %s\n' "${#artifacts[@]}" "$mods_dir"
if [[ "$backup_installed" == true && ${#existing_files[@]} -gt 0 ]]; then
    printf 'Previous files backed up to %s\n' "$backup_dir"
fi

if [[ "$launch_game" == true ]]; then
    game_exe_windows=$(to_windows_path "$game_exe")
    game_dir_windows=$(to_windows_path "$game_dir")
    escaped_game_exe=${game_exe_windows//\'/\'\'}
    escaped_game_dir=${game_dir_windows//\'/\'\'}
    launch_arguments='--headless --audio-driver Dummy'
    if [[ "$unlock_all_on_launch" == true ]]; then
        launch_arguments+=" --ai-ascension-unlock-all"
    fi

    printf '%s\n' 'Relaunching SlayTheSpire2.exe...'
    "$powershell_cmd" -NoProfile -NonInteractive -Command \
        "\$startInfo = New-Object System.Diagnostics.ProcessStartInfo; \
\$startInfo.FileName = '$escaped_game_exe'; \
\$startInfo.WorkingDirectory = '$escaped_game_dir'; \
\$startInfo.UseShellExecute = \$false; \
\$startInfo.CreateNoWindow = \$true; \
if ('$launch_arguments' -ne '') { \$startInfo.Arguments = '$launch_arguments' }; \
[void][System.Diagnostics.Process]::Start(\$startInfo)"
    printf '%s\n' 'Game launch requested.'
else
    printf '%s\n' 'Game was not relaunched (--no-launch).'
fi
