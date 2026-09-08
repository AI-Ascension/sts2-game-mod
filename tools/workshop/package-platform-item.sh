#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

# Keep the historical shell entry point stable while putting the filesystem
# transaction in the locked Rust release tool.
script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(CDPATH= cd -- "$script_dir/../.." && pwd -P)
export STS2_RELEASE_REPO_ROOT="$repo_root"
exec cargo +1.97.1 run --quiet --locked --offline --manifest-path "$repo_root/Cargo.toml" \
    --package sts2-release-tool -- package-platform-item "$@"
