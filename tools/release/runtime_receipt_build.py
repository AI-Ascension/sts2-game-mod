#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Git, host, toolchain, and payload-build operations for runtime receipts."""

from __future__ import annotations

import os
import pathlib
import shutil
import subprocess
from dataclasses import dataclass
from typing import Any

from runtime_receipt_common import (
    ENTRYPOINT,
    LOADER_MANIFEST,
    MANAGED_NAME,
    NATIVE_NAMES,
    absolute,
    digest,
    fail,
    input_path,
    parse_json,
    regular_directory,
    regular_file,
    safe_token,
    windows_path,
)


FIXTURE_PROPERTIES = (
    "EnableCombatDemoProbe",
    "EnableVideoMenuProbe",
    "EnableHandChoiceProbe",
    "EnableTerminalProbe",
    "EnableNativeCoopProbe",
)


@dataclass(frozen=True)
class DotnetTool:
    """Resolved executable and the argument path convention its SDK accepts."""

    path: pathlib.Path
    kind: str
    path_style: str


def run_capture(command: list[str], cwd: pathlib.Path, env: dict[str, str], label: str) -> str:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
    except OSError as exc:
        fail(f"{label} could not start: {exc}")
    if result.returncode != 0:
        fail(f"{label} failed")
    value = result.stdout.strip()
    if not value or "\n" in value:
        fail(f"{label} returned an invalid toolchain identity")
    safe_token(value, label, allow_space=True)
    return value


def run_logged(
    command: list[str],
    cwd: pathlib.Path,
    env: dict[str, str],
    log: pathlib.Path,
    label: str,
) -> None:
    try:
        with log.open("wb") as stream:
            result = subprocess.run(
                command,
                cwd=cwd,
                env=env,
                stdout=stream,
                stderr=subprocess.STDOUT,
                check=False,
            )
    except OSError as exc:
        fail(f"{label} could not start: {exc}")
    if result.returncode != 0:
        fail(f"{label} failed; inspect the task-private build log")


def repository(script_dir: pathlib.Path) -> pathlib.Path:
    try:
        value = subprocess.check_output(
            ["git", "-C", os.fspath(script_dir), "rev-parse", "--show-toplevel"],
            text=True,
            stderr=subprocess.PIPE,
        ).strip()
    except subprocess.CalledProcessError as exc:
        fail(f"cannot locate the Git checkout: {exc.stderr.strip()}")
    return absolute(value, "Git checkout")


def git_id(repo: pathlib.Path, expression: str, label: str) -> str:
    try:
        value = subprocess.check_output(
            ["git", "-C", os.fspath(repo), "rev-parse", "--verify", expression],
            text=True,
            stderr=subprocess.PIPE,
        ).strip().lower()
    except subprocess.CalledProcessError:
        fail(f"could not resolve {label}")
    if len(value) != 40 or any(character not in "0123456789abcdef" for character in value):
        fail(f"{label} is not a full Git object ID")
    return value


def checkout_identity(repo: pathlib.Path, requested: str, require_clean: bool) -> tuple[str, str]:
    head = git_id(repo, "HEAD", "checkout HEAD")
    requested_lower = requested.lower()
    if len(requested_lower) != 40 or any(character not in "0123456789abcdef" for character in requested_lower):
        fail("source commit must be a full Git object ID")
    if head != requested_lower:
        fail("source commit does not equal checkout HEAD")
    if require_clean:
        try:
            status = subprocess.check_output(
                ["git", "-C", os.fspath(repo), "status", "--porcelain=v1", "--untracked-files=all"],
                text=True,
                stderr=subprocess.PIPE,
            )
        except subprocess.CalledProcessError:
            fail("could not inspect checkout cleanliness")
        if status:
            fail("production receipt requires a clean checkout")
    return head, git_id(repo, f"{head}^{{tree}}", "checkout tree")


def source_loader(repo: pathlib.Path, commit: str) -> tuple[bytes, dict[str, Any]]:
    try:
        data = subprocess.check_output(
            ["git", "-C", os.fspath(repo), "show", f"{commit}:{LOADER_MANIFEST}"],
            stderr=subprocess.PIPE,
        )
    except subprocess.CalledProcessError:
        fail("selected source commit has no loader manifest")
    manifest = parse_json(data, "selected loader manifest")
    version = manifest.get("version")
    if not isinstance(version, str):
        fail("selected loader manifest version is missing")
    safe_token(version, "loader manifest version")
    return data, manifest


def host_references(host: pathlib.Path) -> tuple[list[dict[str, Any]], dict[str, tuple[int, str]]]:
    records: list[dict[str, Any]] = []
    values: dict[str, tuple[int, str]] = {}
    for name in ("sts2.dll", "GodotSharp.dll"):
        size, sha256 = digest(regular_file(host / name, f"host reference {name}"))
        values[name] = (size, sha256)
        records.append({"name": name, "size_bytes": size, "sha256": sha256})
    return records, values


def resolve_dotnet(value: str) -> DotnetTool:
    """Resolve a PATH alias or symlink and classify its SDK path convention."""
    if windows_path(value):
        value = input_path(value, "dotnet executable", allow_windows=True).as_posix()
    if "/" not in value and "\\" not in value:
        found = shutil.which(value)
        if not found:
            fail("selected dotnet executable is unavailable")
        candidate = pathlib.Path(found)
    else:
        candidate = pathlib.Path(os.path.abspath(value))
    try:
        resolved = candidate.resolve(strict=True)
    except (FileNotFoundError, OSError) as exc:
        fail(f"selected dotnet executable cannot be resolved: {exc}")
    regular_file(resolved, "dotnet executable", executable=True)
    if resolved.suffix.lower() == ".exe":
        kind, path_style = "windows-exe", "windows"
    else:
        kind, path_style = "linux-native", "posix"
    safe_token(resolved.name, "dotnet executable name")
    return DotnetTool(resolved, kind, path_style)


def dotnet_path(tool: DotnetTool, path: pathlib.Path, label: str) -> str:
    """Convert only the paths passed to a Windows SDK; Linux SDKs stay POSIX."""
    if tool.path_style == "posix":
        return os.fspath(path)
    converter = shutil.which("wslpath")
    if not converter:
        fail(f"Windows dotnet.exe requires wslpath to convert {label}")
    try:
        result = subprocess.run(
            [converter, "-w", "--", os.fspath(path)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
    except OSError as exc:
        fail(f"could not convert {label} for Windows dotnet.exe: {exc}")
    converted = result.stdout.strip()
    if result.returncode != 0 or not converted or "\n" in converted:
        fail(f"could not convert {label} for Windows dotnet.exe")
    return converted


def validate_fixture_payload(payload: pathlib.Path, native_name: str, package_version: str) -> None:
    expected = {MANAGED_NAME, ENTRYPOINT, native_name}
    try:
        entries = list(payload.iterdir())
    except OSError as exc:
        fail(f"cannot inspect fixture payload: {exc}")
    if {entry.name for entry in entries} != expected:
        fail("fixture payload must contain exactly three runtime files")
    for name in expected:
        regular_file(payload / name, "fixture payload")
        digest(payload / name)
    loader = parse_json((payload / ENTRYPOINT).read_bytes(), "fixture loader manifest")
    if loader.get("version") != package_version:
        fail("fixture loader manifest version does not match package version")


def fixture_inputs(
    payload: pathlib.Path,
    native_name: str,
    package_version: str,
) -> tuple[dict[str, pathlib.Path], dict[str, str]]:
    validate_fixture_payload(payload, native_name, package_version)
    return (
        {
            MANAGED_NAME: payload / MANAGED_NAME,
            ENTRYPOINT: payload / ENTRYPOINT,
            native_name: payload / native_name,
        },
        {
            "dotnet_executable": "fixture",
            "dotnet_path_style": "fixture",
            "dotnet_version": "fixture",
            "rustc_version": "fixture",
            "cargo_version": "fixture",
        },
    )


def production_inputs(
    repo: pathlib.Path,
    script_dir: pathlib.Path,
    host: pathlib.Path,
    tool: DotnetTool,
    platform: str,
    source_commit: str,
    package_version: str,
    scratch: pathlib.Path,
    logs: pathlib.Path,
) -> tuple[dict[str, pathlib.Path], dict[str, str], list[dict[str, Any]], dict[str, tuple[int, str]]]:
    native_triple, native_built_name, native_package_name = NATIVE_NAMES[platform]
    env = os.environ.copy()
    env.pop("RUSTFLAGS", None)
    env.pop("CARGO_ENCODED_RUSTFLAGS", None)
    env["DOTNET_CLI_TELEMETRY_OPTOUT"] = "1"
    env["DOTNET_SKIP_FIRST_TIME_EXPERIENCE"] = "1"
    dotnet_home = scratch / "dotnet-home"
    dotnet_home.mkdir()
    env["DOTNET_CLI_HOME"] = dotnet_path(tool, dotnet_home, "DOTNET_CLI_HOME")
    rustc_version = run_capture(["rustc", "--version"], repo, env, "rustc version")
    cargo_version = run_capture(["cargo", "--version"], repo, env, "cargo version")
    dotnet_version = run_capture([os.fspath(tool.path), "--version"], repo, env, "dotnet version")
    native_target = scratch / "native-target"
    native_target.mkdir()
    env["CARGO_TARGET_DIR"] = os.fspath(native_target)
    native_helper = regular_file(
        script_dir.parent.parent / "experiments/managed-rust-interop/build-native-release.sh",
        "native build helper",
        executable=True,
    )
    run_logged([os.fspath(native_helper), platform], repo, env, logs / "native-build.log", "native build")
    native_output = native_target / native_triple / "release" / native_built_name
    regular_file(native_output, "native build output")

    managed_output = scratch / "managed-output"
    managed_intermediate = scratch / "managed-intermediate"
    managed_output.mkdir()
    managed_intermediate.mkdir()
    project = repo / "experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj"
    separator = "\\" if tool.path_style == "windows" else "/"
    output_path = dotnet_path(tool, managed_output, "managed output") + separator
    intermediate_path = dotnet_path(tool, managed_intermediate, "managed intermediate") + separator
    managed_args = [
        os.fspath(tool.path),
        "restore",
        dotnet_path(tool, project, "managed project"),
        f"-p:STS2GameDataDir={dotnet_path(tool, host, 'host data directory')}",
        f"-p:SourceRevisionId={source_commit}",
        f"-p:Version={package_version}",
        f"-p:PackageVersion={package_version}",
        f"-p:OutputPath={output_path}",
        f"-p:BaseIntermediateOutputPath={intermediate_path}",
    ]
    for name in FIXTURE_PROPERTIES:
        managed_args.append(f"-p:{name}=false")
    run_logged(managed_args, repo, env, logs / "managed-restore.log", "managed restore")
    managed_args[1] = "build"
    managed_args.extend(["--configuration", "Release", "--no-restore"])
    run_logged(managed_args, repo, env, logs / "managed-build.log", "managed build")
    managed_output_file = managed_output / MANAGED_NAME
    regular_file(managed_output_file, "managed build output")
    regular_directory(host, "host data directory")
    records, before = host_references(host)
    return (
        {
            MANAGED_NAME: managed_output_file,
            native_package_name: native_output,
        },
        {
            "dotnet_executable": tool.path.name,
            "dotnet_path_style": tool.path_style,
            "dotnet_version": dotnet_version,
            "rustc_version": rustc_version,
            "cargo_version": cargo_version,
        },
        records,
        before,
    )
