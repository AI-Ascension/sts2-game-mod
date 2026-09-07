#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Bounded filesystem, path, JSON, and publication operations for receipts."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import secrets
import shutil
import stat
import subprocess
from typing import Any, NoReturn


RECEIPT_SCHEMA = "sts2-release-build-receipt-v1"
LOADER_MANIFEST = "experiments/managed-rust-interop/game-loader/mod_manifest.json"
MANAGED_NAME = "AIAscensionSTS2GameMod.dll"
ENTRYPOINT = "AIAscensionSTS2GameMod.json"
NATIVE_NAMES = {
    "windows-x86_64": (
        "x86_64-pc-windows-gnu",
        "ai_ascension_sts2_game_mod_native.dll",
        "AIAscensionSTS2GameModNative.dll",
    ),
    "linux-x86_64": (
        "x86_64-unknown-linux-gnu",
        "libai_ascension_sts2_game_mod_native.so",
        "libAIAscensionSTS2GameModNative.so",
    ),
}
MAX_FILE_BYTES = 268_435_456
MAX_TEXT_BYTES = 256


class ReceiptError(Exception):
    """A user-actionable receipt production failure."""


def fail(message: str) -> NoReturn:
    raise ReceiptError(message)


def duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            fail(f"JSON contains a duplicate property: {key}")
        value[key] = item
    return value


def absolute(value: str, label: str) -> pathlib.Path:
    """Return a normalized local path while rejecting existing symlink components."""
    if not value:
        fail(f"{label} must not be empty")
    path = pathlib.Path(os.path.abspath(value))
    current = pathlib.Path(path.anchor)
    for part in path.parts[1:] if path.anchor else path.parts:
        current /= part
        try:
            if current.is_symlink():
                fail(f"{label} path contains a symlink")
        except OSError as exc:
            fail(f"cannot inspect {label}: {exc}")
    return path


def windows_path(value: str) -> bool:
    return (
        len(value) >= 3
        and value[0].isalpha()
        and value[1] == ":"
        and value[2] in ("/", "\\")
    ) or value.startswith("\\\\")


def convert_windows_path(value: str, label: str) -> str:
    if not windows_path(value):
        return value
    converter = shutil.which("wslpath")
    if not converter:
        fail(f"{label} is a Windows path; wslpath is required in this environment")
    try:
        result = subprocess.run(
            [converter, "-u", "--", value],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
    except OSError as exc:
        fail(f"could not convert {label} with wslpath: {exc}")
    converted = result.stdout.strip()
    if result.returncode != 0 or not converted or "\n" in converted:
        fail(f"could not convert {label} with wslpath")
    return converted


def input_path(value: str, label: str, allow_windows: bool = False) -> pathlib.Path:
    if windows_path(value):
        if not allow_windows:
            fail(f"{label} must be a local WSL/POSIX path")
        value = convert_windows_path(value, label)
    return absolute(value, label)


def regular_file(path: pathlib.Path, label: str, executable: bool = False) -> pathlib.Path:
    try:
        mode = path.lstat().st_mode
    except FileNotFoundError:
        fail(f"{label} does not exist")
    except OSError as exc:
        fail(f"cannot inspect {label}: {exc}")
    if not stat.S_ISREG(mode) or path.is_symlink():
        fail(f"{label} must be a regular non-symlink file")
    if executable and not os.access(path, os.X_OK):
        fail(f"{label} is not executable")
    return path


def regular_directory(path: pathlib.Path, label: str) -> pathlib.Path:
    try:
        mode = path.lstat().st_mode
    except FileNotFoundError:
        fail(f"{label} does not exist")
    except OSError as exc:
        fail(f"cannot inspect {label}: {exc}")
    if not stat.S_ISDIR(mode) or path.is_symlink():
        fail(f"{label} must be a regular non-symlink directory")
    return path


def existing(path: pathlib.Path) -> bool:
    try:
        path.lstat()
        return True
    except FileNotFoundError:
        return False


def ensure_absent(path: pathlib.Path, label: str) -> None:
    if existing(path):
        fail(f"{label} already exists; reconcile it before retrying")


def paths_overlap(first: pathlib.Path, second: pathlib.Path) -> bool:
    return first == second or first.is_relative_to(second) or second.is_relative_to(first)


def ensure_disjoint(
    path: pathlib.Path,
    label: str,
    protected: pathlib.Path,
    protected_label: str,
) -> None:
    if paths_overlap(path, protected):
        fail(f"{label} overlaps {protected_label}; choose an external path")


def safe_token(value: str, label: str, allow_space: bool = False) -> None:
    if not value or len(value) > MAX_TEXT_BYTES or ".." in value:
        fail(f"{label} contains an unsafe token")
    allowed = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"
    if allow_space:
        allowed += " ()"
    if any(character not in allowed for character in value):
        fail(f"{label} contains unsupported characters")


def digest(path: pathlib.Path) -> tuple[int, str]:
    regular_file(path, "artifact")
    hasher = hashlib.sha256()
    size = 0
    try:
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                hasher.update(chunk)
                size += len(chunk)
    except OSError as exc:
        fail(f"cannot read artifact: {exc}")
    if size <= 0 or size > MAX_FILE_BYTES:
        fail(f"artifact size is outside the release bound: {path.name}")
    return size, hasher.hexdigest()


def parse_json(data: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=duplicate_keys)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail(f"{label} is not valid UTF-8 JSON: {exc}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    return value


def copy_bytes(data: bytes, destination: pathlib.Path) -> None:
    try:
        with destination.open("xb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
    except OSError as exc:
        fail(f"cannot write staged payload: {exc}")


def copy_file(source: pathlib.Path, destination: pathlib.Path) -> None:
    size, sha256 = digest(source)
    try:
        with source.open("rb") as source_stream, destination.open("xb") as destination_stream:
            while True:
                chunk = source_stream.read(1024 * 1024)
                if not chunk:
                    break
                destination_stream.write(chunk)
            destination_stream.flush()
            os.fsync(destination_stream.fileno())
    except OSError as exc:
        fail(f"cannot copy staged payload: {exc}")
    actual_size, actual_sha256 = digest(destination)
    if (actual_size, actual_sha256) != (size, sha256):
        fail(f"staged payload changed during copy: {source.name}")


def write_private(path: pathlib.Path, data: bytes, mode: int = 0o644) -> None:
    try:
        fd = os.open(os.fspath(path), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(path, mode)
    except FileExistsError:
        fail(f"output entry already exists: {path.name}")
    except OSError as exc:
        fail(f"cannot write output entry: {exc}")


def claim_path(path: pathlib.Path) -> tuple[pathlib.Path, tuple[int, int]]:
    claim = pathlib.Path(os.fspath(path) + ".claim")
    ensure_absent(claim, "output claim")
    try:
        fd = os.open(os.fspath(claim), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        identity = os.fstat(fd)
        with os.fdopen(fd, "w", encoding="ascii") as stream:
            stream.write(f"pid={os.getpid()}\n")
            stream.flush()
            os.fsync(stream.fileno())
    except FileExistsError:
        fail("output is already claimed")
    except OSError as exc:
        fail(f"could not claim output: {exc}")
    return claim, (identity.st_dev, identity.st_ino)


def release_claim(claim: pathlib.Path, identity: tuple[int, int]) -> None:
    try:
        current = claim.lstat()
        if (current.st_dev, current.st_ino) == identity:
            claim.unlink()
    except FileNotFoundError:
        pass
    except OSError:
        pass


def directory_identity(path: pathlib.Path, label: str) -> tuple[int, int]:
    try:
        value = path.lstat()
    except OSError as exc:
        fail(f"cannot inspect {label}: {exc}")
    if not stat.S_ISDIR(value.st_mode) or stat.S_ISLNK(value.st_mode):
        fail(f"{label} must be a regular non-symlink directory")
    return value.st_dev, value.st_ino


def move_no_replace(stage: pathlib.Path, output: pathlib.Path) -> None:
    staged_identity = directory_identity(stage, "staged receipt root")
    try:
        result = subprocess.run(
            ["mv", "-nT", os.fspath(stage), os.fspath(output)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
    except OSError as exc:
        fail(f"could not publish receipt output atomically: {exc}")
    # GNU mv may return success when -n declines a destination race.  Require
    # both source consumption and device/inode identity before committing.
    if result.returncode != 0 or existing(stage):
        fail("could not publish receipt output atomically")
    published_identity = directory_identity(output, "published receipt output")
    if published_identity != staged_identity:
        fail("published receipt output did not retain staged directory identity")
