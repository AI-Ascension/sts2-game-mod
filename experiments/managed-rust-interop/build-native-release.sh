#!/usr/bin/env bash

set -euo pipefail

usage() {
    printf 'usage: %s <windows-x86_64|linux-x86_64>\n' "$0" >&2
}

fail() {
    printf 'native release build: %s\n' "$1" >&2
    exit 1
}

if [[ $# -ne 1 ]]; then
    usage
    exit 2
fi

platform=$1
case "$platform" in
    windows-x86_64)
        target_triple=x86_64-pc-windows-gnu
        artifact_name=ai_ascension_sts2_game_mod_native.dll
        ;;
    linux-x86_64)
        target_triple=x86_64-unknown-linux-gnu
        artifact_name=libai_ascension_sts2_game_mod_native.so
        ;;
    *)
        usage
        exit 2
        ;;
esac

# Cargo environment flags replace the target-table flags. A release build must not
# silently accept an unreviewed ambient override, so require the caller to clear them.
if [[ -n ${RUSTFLAGS:-} || -n ${CARGO_ENCODED_RUSTFLAGS:-} ]]; then
    fail 'RUSTFLAGS and CARGO_ENCODED_RUSTFLAGS must be unset for a canonical release build'
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(cd -- "$script_dir/../.." && pwd -P)
native_manifest="$script_dir/native/Cargo.toml"

# Resolve CARGO_HOME before changing directory. Cargo registry paths can contain
# spaces; they are passed below through CARGO_ENCODED_RUSTFLAGS, not shell splitting.
invocation_root=$PWD
cargo_home_input=${CARGO_HOME:-${HOME:?}/.cargo}
if [[ "$cargo_home_input" = /* ]]; then
    cargo_home=$(cd -- "$cargo_home_input" && pwd -P) \
        || fail "CARGO_HOME does not name a directory: $cargo_home_input"
else
    cargo_home=$(cd -- "$invocation_root/$cargo_home_input" && pwd -P) \
        || fail "relative CARGO_HOME does not name a directory: $cargo_home_input"
fi
export CARGO_HOME="$cargo_home"

# Run from the checkout root so rustup selects the pinned rust-toolchain.toml even
# when this helper is called through an absolute path from another working directory.
cd -- "$repo_root"

rustflags=(
    "--remap-path-prefix=${repo_root}=/_/sts2-game-mod"
    "--remap-path-prefix=${cargo_home}=/_/cargo"
)
if [[ "$platform" == windows-x86_64 ]]; then
    # Setting CARGO_ENCODED_RUSTFLAGS overrides the target config, so carry the
    # required PE timestamp flag explicitly when adding the remap flags.
    rustflags+=("-C" 'link-arg=-Wl,--no-insert-timestamp')
fi

encoded_rustflags=''
for flag in "${rustflags[@]}"; do
    if [[ -n "$encoded_rustflags" ]]; then
        encoded_rustflags+=$'\x1f'
    fi
    encoded_rustflags+=$flag
done
export CARGO_ENCODED_RUSTFLAGS="$encoded_rustflags"

native_target_dir=$(cargo metadata --locked --offline --no-deps --format-version 1 \
    --manifest-path "$native_manifest" | jq -er '.target_directory | select(type == "string" and startswith("/"))')
cargo build --locked --offline --release --target "$target_triple" \
    --manifest-path "$native_manifest" >&2

artifact_path="$native_target_dir/$target_triple/release/$artifact_name"
if [[ ! -f "$artifact_path" || -L "$artifact_path" ]]; then
    fail "build did not produce a regular $platform artifact: $artifact_path"
fi

# A remap failure is a release failure. Check the exact source roots that must not
# escape into the producer artifact before returning its path to the caller.
for forbidden_path in "$repo_root" "$cargo_home"; do
    if LC_ALL=C grep -aFq -- "$forbidden_path" "$artifact_path"; then
        fail "artifact still contains an unmapped producer path: $forbidden_path"
    fi
done

printf '%s\n' "$artifact_path"
