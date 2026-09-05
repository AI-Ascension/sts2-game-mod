#!/usr/bin/env bash
set -euo pipefail
[[ $* == *'--message-format=json'* ]] || exit 1
printf '{"reason":"compiler-artifact","target":{"name":"sts2-gateway-runtime","kind":["bin"]},"executable":"%s"}\n' "$STS2_PROVIDER_FIXTURE_EXECUTABLE"
printf '{"reason":"build-finished","success":true}\n'
