#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(CDPATH= cd -- "$script_dir/../.." && pwd -P)
export STS2_RELEASE_REPO_ROOT="$repo_root"
cd -- "$repo_root"
exec cargo run --quiet --locked --offline --manifest-path "$repo_root/Cargo.toml" \
    --package sts2-release-tool -- build-artifact-manifest "$@"
