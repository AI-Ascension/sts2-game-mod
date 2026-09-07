#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
exec "$script_dir/workshop_operator" verify-package "$@"
