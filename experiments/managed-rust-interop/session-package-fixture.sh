#!/usr/bin/env bash
set -euo pipefail
# Synthetic package producer; never reads a host assembly or invokes a compiler.
mkdir -p "$2"
for artifact in AIAscensionSTS2GameMod.dll AIAscensionSTS2GameModNative.dll AIAscensionSTS2GameMod.json; do
    printf 'synthetic %s\n' "$artifact" > "$2/$artifact"
done
