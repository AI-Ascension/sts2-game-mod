#!/usr/bin/env bash
set -Eeuo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
source "$script_dir/session-launcher.sh"
fixture_root=$(mktemp -d)
trap 'rm -rf -- "$fixture_root"' EXIT
session_launcher_package_script="$script_dir/session-package-fixture.sh"
probe_status() { return 1; } # No listeners are contacted by this test.
fail() { printf 'session restoration test failed: %s\n' "$1" >&2; exit 1; }
for scenario in guardian_failed stop_failed inspection_failed inspection_hung still_running missing_backup linked_path success; do
    (
        session_launcher_repo_root="$fixture_root/$scenario"
        tasklist_cmd="$script_dir/session-launcher-fixture.sh"
        live_authorization_deadline=$((EPOCHSECONDS + 60))
        mkdir -p "$session_launcher_repo_root/host/mods"
        printf 'original managed\n' > "$session_launcher_repo_root/host/mods/AIAscensionSTS2GameMod.dll"
        printf 'original legacy\n' > "$session_launcher_repo_root/host/mods/AIAscensionSTS2Poc.dll"
        install_addon "$session_launcher_repo_root/host/data" "$session_launcher_repo_root/host" "$session_launcher_repo_root/host/mods"
        backup=$owned_backup_dir
        [[ ! -e "$owned_mods_dir/AIAscensionSTS2Poc.dll" ]] || fail 'legacy not retired'
        cp "$owned_mods_dir/AIAscensionSTS2GameMod.dll" "$session_launcher_repo_root/installed"
        game_started=1
        stop_bridge_guardian() { [[ "$scenario" != guardian_failed ]]; }
        game_pid=123
        game_start_ticks=456
        game_exe_windows=synthetic.exe
        windows_dotnet=/bin/true
        bridge_dll_windows=synthetic-bridge
        case $scenario in
            stop_failed) windows_dotnet=/bin/false ;;
            inspection_failed) tasklist_cmd=/bin/false ;;
            inspection_hung) export STS2_SESSION_TEST_INSPECTION_HANG=1; probe_timeout_seconds=1 ;;
            still_running) export STS2_SESSION_TEST_GAME_RUNNING=1 ;;
            missing_backup) rm "$backup/AIAscensionSTS2GameMod.dll" ;;
            linked_path)
                mv "$owned_mods_dir/AIAscensionSTS2GameMod.dll" "$session_launcher_repo_root/untouched"
                ln -s "$session_launcher_repo_root/untouched" "$owned_mods_dir/AIAscensionSTS2GameMod.dll"
                ;;
        esac
        cleanup_owned_processes >/dev/null 2>&1
        set -e
        if [[ "$scenario" == success ]]; then
            [[ $cleanup_failed == 0 && -z "$owned_backup_dir" ]] || fail 'successful cleanup not confirmed'
            cmp "$backup/AIAscensionSTS2GameMod.dll" "$session_launcher_repo_root/host/mods/AIAscensionSTS2GameMod.dll"
            cmp "$backup/AIAscensionSTS2Poc.dll" "$session_launcher_repo_root/host/mods/AIAscensionSTS2Poc.dll"
            [[ ! -e "$session_launcher_repo_root/host/mods/AIAscensionSTS2GameModNative.dll" ]] || fail 'new file not removed'
        else
            [[ $cleanup_failed == 1 && "$owned_backup_dir" == "$backup" ]] || fail 'failed cleanup lost recovery state'
            cmp "$session_launcher_repo_root/installed" "$owned_mods_dir/AIAscensionSTS2GameMod.dll"
            [[ ! -e "$owned_mods_dir/AIAscensionSTS2Poc.dll" ]] || fail 'unsafe partial restore'
        fi
    ) || exit 1
done
printf 'PASS: restoration requires confirmed stop, valid backup and unchanged paths\n'
