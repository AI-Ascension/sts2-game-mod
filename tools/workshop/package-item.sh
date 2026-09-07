#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

# Backwards-compatible Windows entry point. New platform packages must name their
# platform explicitly through package-platform-item.sh so a native payload can
# never be mistaken for another platform's Workshop content.
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
exec bash "$script_dir/package-platform-item.sh" windows-x86_64 "$@"
