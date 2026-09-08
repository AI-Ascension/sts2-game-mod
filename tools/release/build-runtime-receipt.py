#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Build one platform runtime payload and emit its immutable build receipt."""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import secrets
import shutil
import signal
import sys
import tempfile
from typing import Any, NoReturn

from runtime_receipt_build import (
    checkout_identity,
    fixture_inputs,
    host_references,
    input_path,
    production_inputs,
    repository,
    resolve_dotnet,
    source_loader,
)
from runtime_receipt_common import (
    ENTRYPOINT,
    MANAGED_NAME,
    NATIVE_NAMES,
    RECEIPT_SCHEMA,
    ReceiptError,
    absolute,
    claim_path,
    copy_bytes,
    copy_file,
    digest,
    ensure_absent,
    ensure_disjoint,
    fail,
    move_no_replace,
    regular_directory,
    release_claim,
    safe_token,
    write_private,
)


CURRENT_EVIDENCE: pathlib.Path | None = None


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="build-runtime-receipt.sh",
        description="build one platform payload and emit a provenance receipt",
    )
    parser.add_argument("--platform", choices=sorted(NATIVE_NAMES), required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--game-version", required=True)
    parser.add_argument("--package-version", required=True)
    parser.add_argument("--host-data-dir")
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--evidence-dir")
    parser.add_argument("--dotnet", default=os.environ.get("DOTNET_BIN", "dotnet"))
    parser.add_argument("--scope", choices=("production", "fixture"), default="production")
    parser.add_argument("--fixture-payload-dir")
    args = parser.parse_args(argv)
    safe_token(args.game_version, "game version")
    safe_token(args.package_version, "package version")
    if args.scope == "production" and args.fixture_payload_dir:
        fail("fixture payload requires --scope fixture")
    if args.scope == "fixture" and not args.fixture_payload_dir:
        fail("fixture scope requires --fixture-payload-dir")
    if args.scope == "production" and not args.host_data_dir:
        fail("production scope requires --host-data-dir")
    return args


def default_evidence(output: pathlib.Path) -> pathlib.Path:
    return output.parent / f".{output.name}.evidence-{os.getpid()}-{secrets.token_hex(8)}"


def validate_roots(
    output: pathlib.Path,
    evidence: pathlib.Path,
    repo: pathlib.Path,
    host: pathlib.Path | None,
) -> None:
    ensure_disjoint(output, "output directory", repo, "Git checkout")
    ensure_disjoint(evidence, "evidence directory", repo, "Git checkout")
    ensure_disjoint(output, "output directory", evidence, "evidence directory")
    if host is not None:
        ensure_disjoint(output, "output directory", host, "host data directory")
        ensure_disjoint(evidence, "evidence directory", host, "host data directory")


def prepare_attempt(
    output: pathlib.Path,
    evidence: pathlib.Path,
    repo: pathlib.Path,
    host: pathlib.Path | None,
) -> tuple[pathlib.Path, pathlib.Path, pathlib.Path, pathlib.Path, tuple[int, int]]:
    validate_roots(output, evidence, repo, host)
    ensure_absent(output, "output directory")
    ensure_absent(pathlib.Path(os.fspath(output) + ".claim"), "output claim")
    ensure_absent(evidence, "evidence directory")
    output.parent.mkdir(parents=True, exist_ok=True)
    regular_directory(output.parent, "output parent")
    # Recheck after creating missing parents, because a concurrent replacement
    # must never turn an output path into a repository or host child.
    validate_roots(output, evidence, repo, host)
    ensure_absent(output, "output directory")
    ensure_absent(pathlib.Path(os.fspath(output) + ".claim"), "output claim")
    evidence.mkdir(mode=0o700)
    os.chmod(evidence, 0o700)
    logs = evidence / "logs"
    logs.mkdir(mode=0o700)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="scratch-", dir=evidence))
    os.chmod(scratch, 0o700)
    ensure_disjoint(scratch, "scratch directory", repo, "Git checkout")
    if host is not None:
        ensure_disjoint(scratch, "scratch directory", host, "host data directory")
    ensure_disjoint(scratch, "scratch directory", output, "output directory")
    claim, claim_identity = claim_path(output)
    return evidence, logs, scratch, claim, claim_identity


def receipt_payload(
    stage_payload: pathlib.Path,
    native_name: str,
) -> list[dict[str, Any]]:
    records = []
    for name in (MANAGED_NAME, ENTRYPOINT, native_name):
        size, sha256 = digest(stage_payload / name)
        records.append({"path": name, "size_bytes": size, "sha256": sha256})
    return records


def main(argv: list[str]) -> int:
    global CURRENT_EVIDENCE
    args = parse_args(argv)
    script_dir = pathlib.Path(__file__).resolve().parent
    repo = repository(script_dir)
    require_clean = args.scope == "production"
    source_commit, source_tree = checkout_identity(repo, args.source_commit, require_clean)
    loader_bytes, loader_manifest = source_loader(repo, source_commit)
    if loader_manifest["version"] != args.package_version:
        fail("package version does not match selected source loader manifest version")

    _, _, native_package_name = NATIVE_NAMES[args.platform]
    host: pathlib.Path | None = None
    host_records: list[dict[str, Any]] = []
    host_before: dict[str, tuple[int, str]] = {}
    if args.scope == "production":
        host = regular_directory(
            input_path(args.host_data_dir, "host data directory", allow_windows=True),
            "host data directory",
        )
        host_records, host_before = host_references(host)
    fixture: pathlib.Path | None = None
    if args.scope == "fixture":
        fixture = regular_directory(
            input_path(args.fixture_payload_dir, "fixture payload", allow_windows=True),
            "fixture payload",
        )

    output = absolute(args.output_dir, "output directory")
    evidence = absolute(args.evidence_dir, "evidence directory") if args.evidence_dir else default_evidence(output)
    CURRENT_EVIDENCE = evidence
    prepared_evidence, logs, scratch, claim, claim_identity = prepare_attempt(
        output, evidence, repo, host
    )
    CURRENT_EVIDENCE = prepared_evidence
    stage = scratch / "receipt-root"
    stage_payload = stage / "payload"
    committed = False
    try:
        stage_payload.mkdir(parents=True)
        os.chmod(stage, 0o700)
        os.chmod(stage_payload, 0o700)
        if args.scope == "fixture":
            expected_files, toolchain = fixture_inputs(fixture, native_package_name, args.package_version)
            fixture_flags = ["fixture_payload"]
        else:
            expected_files, toolchain, built_host_records, built_host = production_inputs(
                repo,
                script_dir,
                host,
                resolve_dotnet(args.dotnet),
                args.platform,
                source_commit,
                args.package_version,
                scratch,
                logs,
            )
            if built_host != host_before or built_host_records != host_records:
                fail("host reference changed during the build")
            fixture_flags = []

        copy_file(expected_files[MANAGED_NAME], stage_payload / MANAGED_NAME)
        copy_bytes(loader_bytes, stage_payload / ENTRYPOINT)
        copy_file(expected_files[native_package_name], stage_payload / native_package_name)
        payload_records = receipt_payload(stage_payload, native_package_name)

        final_commit, final_tree = checkout_identity(repo, source_commit, require_clean)
        if final_commit != source_commit or final_tree != source_tree:
            fail("source checkout changed during the build")
        final_loader_bytes, final_loader_manifest = source_loader(repo, source_commit)
        if final_loader_bytes != loader_bytes or final_loader_manifest["version"] != args.package_version:
            fail("selected loader manifest changed during the build")
        if host is not None:
            final_host = regular_directory(
                input_path(args.host_data_dir, "host data directory", allow_windows=True),
                "host data directory",
            )
            _, final_host_values = host_references(final_host)
            if final_host_values != host_before:
                fail("host reference changed before receipt publication")

        if os.environ.get("STS2_RELEASE_TEST_INTERRUPT_BEFORE_PUBLISH") == "1":
            os.kill(os.getpid(), signal.SIGTERM)

        receipt = {
            "schema_version": RECEIPT_SCHEMA,
            "scope": args.scope,
            "platform": args.platform,
            "source_commit": source_commit,
            "source_tree": source_tree,
            "game_version": args.game_version,
            "package_version": args.package_version,
            "loader_manifest_version": final_loader_manifest["version"],
            "toolchain": toolchain,
            "host_references": host_records,
            "fixture_flags": fixture_flags,
            "payload": payload_records,
        }
        receipt_data = (json.dumps(receipt, indent=2, ensure_ascii=True) + "\n").encode("utf-8")
        write_private(stage / "build-receipt.json", receipt_data)
        if {entry.name for entry in stage.iterdir()} != {"build-receipt.json", "payload"}:
            fail("receipt staging root contains an unexpected entry")
        move_no_replace(stage, output)
        committed = True
        print(f"runtime receipt: {output}")
        print(f"source commit: {source_commit}")
        print(f"private evidence: {evidence}")
        return 0
    finally:
        if committed:
            shutil.rmtree(scratch, ignore_errors=True)
        # A failed attempt retains evidence and its scratch tree for diagnosis.
        # The private root is outside the published receipt and is never copied.
        release_claim(claim, claim_identity)


if __name__ == "__main__":
    def interrupt(_signum: int, _frame: Any) -> NoReturn:
        raise KeyboardInterrupt

    for signal_name in ("SIGINT", "SIGTERM", "SIGHUP"):
        if hasattr(signal, signal_name):
            signal.signal(getattr(signal, signal_name), interrupt)
    try:
        raise SystemExit(main(sys.argv[1:]))
    except ReceiptError as exc:
        suffix = f"; private evidence retained at {CURRENT_EVIDENCE}" if CURRENT_EVIDENCE else ""
        print(f"build-runtime-receipt.sh: {exc}{suffix}", file=sys.stderr)
        raise SystemExit(1)
    except KeyboardInterrupt:
        suffix = f"; private evidence retained at {CURRENT_EVIDENCE}" if CURRENT_EVIDENCE else ""
        print(f"build-runtime-receipt.sh: interrupted; no receipt was published{suffix}", file=sys.stderr)
        raise SystemExit(130)
