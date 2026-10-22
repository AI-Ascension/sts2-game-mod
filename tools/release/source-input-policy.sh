#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

# The exact reviewed inventory remains authoritative. This additional guard prevents a
# reviewed source path from accidentally admitting a build tree or host-owned artifact.
source_distribution_path_is_supported() {
    local relative=$1
    local artifact_path="/$relative"
    # Cargo's src/bin contains Rust entrypoint/module source, not generated binaries.
    # Remove only that one source segment; any other bin/ ancestor remains forbidden.
    if [[ "$relative" == *.rs ]]; then
        artifact_path=${artifact_path/\/src\/bin\//\/src\/}
    fi
    artifact_path=${artifact_path#/}
    case "$artifact_path" in
        .git/*|target/*|*/target/*|bin/*|*/bin/*|obj/*|*/obj/*|*.dll|*.dylib|*.so|*.pdb|*.save|*.sav|*.sts2profile|.env|.env.*|*/.env|*/.env.*)
            return 1
            ;;
    esac
    return 0
}
