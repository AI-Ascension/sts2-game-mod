#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
repo_root=$(CDPATH= cd -- "$script_dir/../../../.." && pwd -P)
dotnet_bin=${DOTNET_BIN:-}
if [[ -z "$dotnet_bin" && -n "${DOTNET_ROOT:-}" && -x "$DOTNET_ROOT/dotnet" ]]; then
    dotnet_bin="$DOTNET_ROOT/dotnet"
fi
if [[ -z "$dotnet_bin" ]]; then
    dotnet_bin=$(command -v dotnet || true)
elif [[ "$dotnet_bin" != */* ]]; then
    dotnet_bin=$(command -v "$dotnet_bin" || true)
fi
if [[ -z "$dotnet_bin" || ! -x "$dotnet_bin" ]]; then
    printf '%s\n' 'set DOTNET_BIN to the pinned .NET executable or expose dotnet on PATH' >&2
    exit 127
fi
temp_dir=$(mktemp -d -t sts2-ugc-create-item-XXXXXXXX)
trap 'rm -rf -- "$temp_dir"' EXIT

payload="$temp_dir/payload"
mkdir -p -- "$payload"
printf 'managed fixture\n' > "$payload/AIAscensionSTS2GameMod.dll"
printf '{"fixture":true}\n' > "$payload/AIAscensionSTS2GameMod.json"
printf 'native fixture\n' > "$payload/libAIAscensionSTS2GameModNative.so"
printf 'preview\n' > "$temp_dir/preview.jpg"

package="$temp_dir/package"
bash "$repo_root/tools/workshop/package-platform-item.sh" \
    linux-x86_64 "$payload" "$package" 2868840 0 0.107.1 0.4.1 \
    recovery-ugc-test "$temp_dir/preview.jpg" >/dev/null

cat > "$temp_dir/fake-steam-api.c" <<'FAKE_STEAM_API'
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

static uint64_t active_call;

static int mode_is(const char *expected)
{
    const char *mode = getenv("FAKE_STEAM_API_MODE");
    return mode != 0 && strcmp(mode, expected) == 0;
}

int SteamAPI_InitFlat(char *error_message)
{
    if (error_message != 0)
        error_message[0] = 0;
    return 0;
}

void SteamAPI_Shutdown(void)
{
}

void *SteamAPI_SteamApps_v008(void)
{
    return (void *)(uintptr_t)0x11;
}

void *SteamAPI_SteamUtils_v010(void)
{
    return (void *)(uintptr_t)0x22;
}

void *SteamAPI_SteamUser_v023(void)
{
    return (void *)(uintptr_t)0x33;
}

void *SteamAPI_SteamUGC_v020(void)
{
    return (void *)(uintptr_t)0x44;
}

uint32_t SteamAPI_ISteamUtils_GetAppID(void *self)
{
    (void)self;
    return 2868840;
}

uint8_t SteamAPI_ISteamUser_BLoggedOn(void *self)
{
    (void)self;
    return 1;
}

uint8_t SteamAPI_ISteamApps_BIsSubscribedApp(void *self, uint32_t app_id)
{
    (void)self;
    return app_id == 2868840;
}

uint64_t SteamAPI_ISteamUGC_CreateItem(void *self, uint32_t app_id, int file_type)
{
    (void)self;
    (void)app_id;
    (void)file_type;
    active_call = 77;
    return active_call;
}

uint8_t SteamAPI_ISteamUtils_IsAPICallCompleted(
    void *self,
    uint64_t call,
    uint8_t *failed)
{
    (void)self;
    if (failed != 0)
        *failed = mode_is("transport") ? 1 : 0;
    if (mode_is("timeout"))
        return 0;
    return call == active_call;
}

uint8_t SteamAPI_ISteamUtils_GetAPICallResult(
    void *self,
    uint64_t call,
    void *callback,
    int callback_size,
    int callback_id,
    uint8_t *failed)
{
    int32_t result_code = 1;
    uint64_t published_file_id = 1002003;
    uint8_t needs_legal_agreement = 0;
    (void)self;
    if (failed != 0)
        *failed = 0;
    if (call != active_call || callback == 0 || callback_size < 16 || callback_id != 3403)
        return 0;
    memcpy((char *)callback + 0, &result_code, sizeof(result_code));
    memcpy((char *)callback + 4, &published_file_id, sizeof(published_file_id));
    memcpy((char *)callback + 12, &needs_legal_agreement, sizeof(needs_legal_agreement));
    return 1;
}

void SteamAPI_RunCallbacks(void)
{
}
FAKE_STEAM_API
gcc -shared -fPIC -Werror -Wall -Wextra -o "$temp_dir/libfake-steam-api.so" \
    "$temp_dir/fake-steam-api.c"

build_dir="$temp_dir/build"
obj_dir="$temp_dir/obj"
mkdir -p -- "$build_dir" "$obj_dir"
"$dotnet_bin" build "$script_dir/UgcCreateItemHelper.csproj" \
    --nologo --configuration Release --property:BaseIntermediateOutputPath="$obj_dir/" \
    --property:OutputPath="$build_dir/" >/dev/null
helper="$build_dir/Sts2.UgcCreateItemHelper.dll"
[[ -f "$helper" ]]

library_sha=$(sha256sum "$temp_dir/libfake-steam-api.so" | awk '{print $1}')
if SteamAppId=2868840 SteamGameId=2868840 "$dotnet_bin" "$helper" \
    --library "$temp_dir/libfake-steam-api.so" --expected-sha256 "$library_sha" \
    --package-dir "$package" --platform windows-x86_64 --output "$temp_dir/windows-result.json" \
    --journal "$temp_dir/windows-journal.json" --timeout-seconds 5 >/dev/null 2>&1; then
    printf '%s\n' 'Windows CreateItem ABI was unexpectedly accepted' >&2
    exit 1
fi

result="$temp_dir/result.json"
journal="$temp_dir/journal.json"
SteamAppId=2868840 SteamGameId=2868840 "$dotnet_bin" "$helper" \
    --library "$temp_dir/libfake-steam-api.so" --expected-sha256 "$library_sha" \
    --package-dir "$package" --platform linux-x86_64 --output "$result" \
    --journal "$journal" --timeout-seconds 5 >/dev/null
python3 - "$result" "$journal" <<'PY'
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
journal = json.load(open(sys.argv[2], encoding="utf-8"))
assert result["outcome"] == "succeeded"
assert result["package_preflight_passed"] is True
assert result["loaded_library_matches"] is True
assert result["app_environment_matches"] is True
assert result["app_id_matches"] is True
assert result["logged_on"] is True
assert result["subscribed"] is True
assert result["create_item_call_issued"] is True
assert result["api_call_completed"] is True
assert result["result_code"] == 1
assert result["published_file_id"] == 1002003
assert journal["outcome"] == "succeeded"
assert journal["phase"] == "succeeded"
assert journal["published_file_id"] == 1002003
PY

transport_result="$temp_dir/transport-result.json"
transport_journal="$temp_dir/transport-journal.json"
if FAKE_STEAM_API_MODE=transport SteamAppId=2868840 SteamGameId=2868840 \
    "$dotnet_bin" "$helper" --library "$temp_dir/libfake-steam-api.so" \
    --expected-sha256 "$library_sha" --package-dir "$package" --platform linux-x86_64 \
    --output "$transport_result" --journal "$transport_journal" --timeout-seconds 5 \
    >/dev/null 2>&1; then
    printf '%s\n' 'failed API transport unexpectedly succeeded' >&2
    exit 1
fi
python3 - "$transport_result" "$transport_journal" <<'PY'
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
journal = json.load(open(sys.argv[2], encoding="utf-8"))
assert result["outcome"] == "unknown"
assert result["api_call_completed"] is True
assert result["api_call_failed"] is True
assert result["error_type"] == "ApiCallFailed"
assert journal["outcome"] == "unknown"
assert journal["phase"] == "unknown_api_call_failed"
PY

timeout_result="$temp_dir/timeout-result.json"
timeout_journal="$temp_dir/timeout-journal.json"
if FAKE_STEAM_API_MODE=timeout SteamAppId=2868840 SteamGameId=2868840 \
    "$dotnet_bin" "$helper" --library "$temp_dir/libfake-steam-api.so" \
    --expected-sha256 "$library_sha" --package-dir "$package" --platform linux-x86_64 \
    --output "$timeout_result" --journal "$timeout_journal" --timeout-seconds 1 \
    >/dev/null 2>&1; then
    printf '%s\n' 'API timeout unexpectedly succeeded' >&2
    exit 1
fi
python3 - "$timeout_result" "$timeout_journal" <<'PY'
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
journal = json.load(open(sys.argv[2], encoding="utf-8"))
assert result["outcome"] == "unknown"
assert result["api_call_completed"] is False
assert result["error_type"] == "CreateItemTimedOut"
assert journal["outcome"] == "unknown"
assert journal["phase"] == "unknown_timeout"
PY

if SteamAppId=2868840 SteamGameId=2868840 "$dotnet_bin" "$helper" \
    --library "$temp_dir/libfake-steam-api.so" --expected-sha256 "$library_sha" \
    --package-dir "$package" --platform linux-x86_64 --output "$result" \
    --journal "$journal" --timeout-seconds 5 >/dev/null 2>&1; then
    printf '%s\n' 'existing result and journal unexpectedly permitted a retry' >&2
    exit 1
fi

mismatch_result="$temp_dir/mismatch-result.json"
mismatch_journal="$temp_dir/mismatch-journal.json"
if SteamAppId=2868840 SteamGameId=2868840 "$dotnet_bin" "$helper" \
    --library "$temp_dir/libfake-steam-api.so" \
    --expected-sha256 0000000000000000000000000000000000000000000000000000000000000000 \
    --package-dir "$package" --platform linux-x86_64 --output "$mismatch_result" \
    --journal "$mismatch_journal" --timeout-seconds 5 >/dev/null 2>&1; then
    printf '%s\n' 'library hash mismatch unexpectedly succeeded' >&2
    exit 1
fi
python3 - "$mismatch_result" <<'PY'
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
assert result["outcome"] == "not_started"
assert result["package_preflight_passed"] is True
assert result["library_hash_matches"] is False
assert result["error_type"] == "LibraryHashMismatch"
assert not result["native_library_loaded"]
PY

missing_env_result="$temp_dir/missing-env-result.json"
missing_env_journal="$temp_dir/missing-env-journal.json"
if env -u SteamAppId -u SteamGameId "$dotnet_bin" "$helper" \
    --library "$temp_dir/libfake-steam-api.so" --expected-sha256 "$library_sha" \
    --package-dir "$package" --platform linux-x86_64 --output "$missing_env_result" \
    --journal "$missing_env_journal" --timeout-seconds 5 >/dev/null 2>&1; then
    printf '%s\n' 'missing Steam app environment unexpectedly succeeded' >&2
    exit 1
fi
python3 - "$missing_env_result" <<'PY'
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
assert result["outcome"] == "not_started"
assert result["library_hash_matches"] is True
assert result["app_environment_matches"] is False
assert result["error_type"] == "AppEnvironmentPreconditionFailed"
assert not result["native_library_loaded"]
PY

printf '%s\n' 'UGC helper build, native ABI dispatch, package preflight, success journal, retry barrier, hash mismatch, and app environment preconditions passed.'
