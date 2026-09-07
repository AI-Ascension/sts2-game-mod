#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd -P)
temp_dir=$(mktemp -d -t sts2-workshop-lifecycle-XXXXXXXX)
trap 'rm -rf -- "$temp_dir"' EXIT

payload="$temp_dir/payload"
mkdir -p -- "$payload"
printf 'managed fixture\n' > "$payload/AIAscensionSTS2GameMod.dll"
printf '{"fixture":true}\n' > "$payload/AIAscensionSTS2GameMod.json"
printf 'native fixture\n' > "$payload/libAIAscensionSTS2GameModNative.so"
printf 'preview\n' > "$temp_dir/preview.jpg"

package="$temp_dir/package"
bash "$repo_root/tools/workshop/package-platform-item.sh" \
    linux-x86_64 "$payload" "$package" 2868840 1001001 0.107.1 0.4.1 \
    recovery-test "$temp_dir/preview.jpg" >/dev/null

operator="$script_dir/workshop_operator"

"$operator" verify-package \
    --package-dir "$package" --platform linux-x86_64 \
    --app-id 2868840 --item-id 1001001 >/dev/null

cat > "$temp_dir/fake-steamcmd" <<'FAKE_STEAMCMD'
#!/usr/bin/env bash
set -Eeuo pipefail
printf '%s\n' "$*" > "$FAKE_ARGS"
if [[ "${FAKE_FAIL:-0}" == 1 ]]; then
    exit 17
fi
if [[ "${FAKE_DOWNLOAD:-0}" == 1 ]]; then
    app_id=$4
    item_id=$5
    target="$FAKE_ROOT/steamapps/workshop/content/$app_id/$item_id"
    mkdir -p -- "$(dirname -- "$target")"
    cp -a -- "$SOURCE_PACKAGE" "$target"
fi
FAKE_STEAMCMD
chmod 755 "$temp_dir/fake-steamcmd"
touch "$temp_dir/fake-args"

plan_journal="$temp_dir/journals/plan.json"
STEAMCMD_USER=fixture "$operator" upload \
    --steamcmd "$temp_dir/fake-steamcmd" --package-dir "$package" \
    --platform linux-x86_64 --app-id 2868840 --item-id 1001001 \
    --vdf "$package.vdf" --journal "$plan_journal" >/dev/null
[[ ! -e "$plan_journal" ]]

unknown_journal="$temp_dir/journals/upload-unknown.json"
if STEAMCMD_USER=fixture FAKE_FAIL=1 FAKE_ARGS="$temp_dir/fail-args" \
    "$operator" upload \
    --steamcmd "$temp_dir/fake-steamcmd" --package-dir "$package" \
    --platform linux-x86_64 --app-id 2868840 --item-id 1001001 \
    --vdf "$package.vdf" --journal "$unknown_journal" \
    --execute-public-write >/dev/null; then
    printf '%s\n' 'failed SteamCMD operation unexpectedly succeeded' >&2
    exit 1
fi
python3 - "$unknown_journal" <<'PY'
import json
import sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
assert value["outcome"] == "unknown"
assert value["phase"] == "unknown_process_exit"
assert value["process_exit_code"] == 17
assert "fixture" not in json.dumps(value)
PY
if STEAMCMD_USER=fixture FAKE_FAIL=1 FAKE_ARGS="$temp_dir/fail-args" \
    "$operator" upload \
    --steamcmd "$temp_dir/fake-steamcmd" --package-dir "$package" \
    --platform linux-x86_64 --app-id 2868840 --item-id 1001001 \
    --vdf "$package.vdf" --journal "$unknown_journal" \
    --execute-public-write >/dev/null 2>&1; then
    printf '%s\n' 'journal collision unexpectedly permitted a retry' >&2
    exit 1
fi

success_journal="$temp_dir/journals/update-success.json"
STEAMCMD_USER=fixture FAKE_ARGS="$temp_dir/success-args" \
    "$operator" update \
    --steamcmd "$temp_dir/fake-steamcmd" --package-dir "$package" \
    --platform linux-x86_64 --app-id 2868840 --item-id 1001001 \
    --vdf "$package.vdf" --journal "$success_journal" \
    --execute-public-write >/dev/null
python3 - "$success_journal" <<'PY'
import json
import sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
assert value["outcome"] == "succeeded"
assert value["phase"] == "succeeded"
PY

download_root="$temp_dir/steamcmd-root"
mkdir -p -- "$download_root"
download_journal="$temp_dir/journals/download.json"
STEAMCMD_USER=fixture FAKE_DOWNLOAD=1 FAKE_ROOT="$download_root" \
SOURCE_PACKAGE="$package" FAKE_ARGS="$temp_dir/download-args" \
    "$operator" download \
    --steamcmd "$temp_dir/fake-steamcmd" --steamcmd-dir "$download_root" \
    --platform linux-x86_64 --app-id 2868840 --item-id 1001001 \
    --journal "$download_journal" --execute-local-download >/dev/null
python3 - "$download_journal" <<'PY'
import json
import sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
assert value["outcome"] == "succeeded"
assert value["phase"] == "succeeded"
PY

cat > "$temp_dir/fake-subscribe-runner" <<'FAKE_RUNNER'
#!/usr/bin/env bash
set -Eeuo pipefail
output=''
journal=''
while (($#)); do
    case "$1" in
        --output) output=$2; shift 2 ;;
        --journal) journal=$2; shift 2 ;;
        *) shift ;;
    esac
done
[[ "${SteamAppId:-}" == 2868840 && "${SteamGameId:-}" == 2868840 ]]
printf '{"outcome":"succeeded","source":"synthetic-runner"}\n' > "$output"
printf '{"outcome":"succeeded"}\n' > "$journal"
FAKE_RUNNER
chmod 755 "$temp_dir/fake-subscribe-runner"
printf 'synthetic library\n' > "$temp_dir/libsteam_api.so"
runner_sha=$(sha256sum "$temp_dir/fake-subscribe-runner" | awk '{print $1}')
library_sha=$(sha256sum "$temp_dir/libsteam_api.so" | awk '{print $1}')
subscribe_journal="$temp_dir/journals/subscribe.json"
STEAMCMD_USER=fixture "$operator" subscribe \
    --runner "$temp_dir/fake-subscribe-runner" --runner-sha256 "$runner_sha" \
    --library "$temp_dir/libsteam_api.so" --library-sha256 "$library_sha" \
    --app-id 2868840 --item-id 1001001 \
    --result "$temp_dir/subscribe-result.json" \
    --runner-journal "$temp_dir/journals/subscribe-native.json" \
    --journal "$subscribe_journal" --execute-ugc >/dev/null
python3 - "$subscribe_journal" <<'PY'
import json
import sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
assert value["outcome"] == "succeeded"
assert value["phase"] == "succeeded"
PY

ln -s -- "$package" "$temp_dir/package-link"
if "$operator" verify-package --package-dir "$temp_dir/package-link" \
    --platform linux-x86_64 --app-id 2868840 --item-id 1001001 >/dev/null 2>&1; then
    printf '%s\n' 'symlinked package root was accepted' >&2
    exit 1
fi

printf '%s\n' 'Workshop package verification, plan gating, unknown journals, update, download, subscription, and symlink refusal passed.'
