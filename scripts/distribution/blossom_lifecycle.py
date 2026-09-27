#!/usr/bin/env python3
"""Closed Phase 9 install, first-run, update, and recovery state machine."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tempfile

PRODUCT = "blossom-os"
ARCH = "x86_64"
SCHEMA = 1
SLOTS = {"a", "b"}
MODEL_PROFILES = {"none", "llama_cpp_cpu_v1", "ollama_cpu_v1"}
STATE_FILES = {"install.json", "first-run.json", "update.json", "active-slot"}
HEX64 = re.compile(r"^[0-9a-f]{64}$")
MAX_STATE_BYTES = 64 * 1024
MAX_METADATA_BYTES = 64 * 1024
MAX_SIGNATURE_BYTES = 16 * 1024
MAX_EVIDENCE_PAYLOAD_BYTES = 1024 * 1024 * 1024


class LifecycleError(ValueError):
    pass


@dataclass(frozen=True)
class VerifiedUpdate:
    metadata_bytes: bytes
    payload: bytes


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def read_regular(path: Path, maximum: int, description: str) -> bytes:
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    except OSError as error:
        raise LifecycleError(f"{description} cannot be opened safely") from error
    try:
        metadata = os.fstat(descriptor)
        if not stat.S_ISREG(metadata.st_mode) or not 0 < metadata.st_size <= maximum:
            raise LifecycleError(f"{description} is outside the closed size bound")
        chunks: list[bytes] = []
        remaining = metadata.st_size
        while remaining:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                raise LifecycleError(f"{description} changed while being read")
            chunks.append(chunk)
            remaining -= len(chunk)
        if os.read(descriptor, 1):
            raise LifecycleError(f"{description} grew while being read")
        return b"".join(chunks)
    finally:
        os.close(descriptor)


def load_closed(path: Path, fields: set[str]) -> dict:
    return load_closed_bytes(read_regular(path, MAX_STATE_BYTES, "state input"), fields)


def load_closed_bytes(data: bytes, fields: set[str]) -> dict:
    try:
        value = json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise LifecycleError("invalid closed JSON") from error
    if not isinstance(value, dict) or set(value) != fields:
        raise LifecycleError("closed schema mismatch")
    return value


def atomic_write(path: Path, data: bytes, mode: int = 0o600) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        os.fchmod(fd, mode)
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def hardware_record(architecture: str, firmware: str, graphics: str,
                    memory_mib: int, virtio: bool) -> dict:
    if architecture != ARCH or firmware != "uefi":
        raise LifecycleError("unsupported installation target")
    if graphics not in {"virtio", "software"} or not isinstance(memory_mib, int):
        raise LifecycleError("unsupported hardware record")
    memory_class = "4g-plus" if memory_mib >= 4096 else "2g-plus" if memory_mib >= 2048 else "unsupported"
    if memory_class == "unsupported" or not virtio:
        raise LifecycleError("evidence target requirements not met")
    return {"architecture": ARCH, "firmware": "uefi", "graphics": graphics,
            "memory_class": memory_class, "virtio": True}


def install(root: Path, hardware: dict) -> None:
    if root.exists() and any(root.iterdir()):
        raise LifecycleError("installer requires an empty target")
    marker = {"architecture": ARCH, "hardware": hardware, "image_schema": SCHEMA,
              "product": PRODUCT, "slots": ["a", "b"]}
    (root / "slots/a").mkdir(parents=True)
    (root / "slots/b").mkdir(parents=True)
    (root / "user-data").mkdir(mode=0o700)
    atomic_write(root / "install.json", canonical(marker), 0o600)
    atomic_write(root / "active-slot", b"a\n", 0o600)
    atomic_write(root / "first-run.json", canonical({"completed": False, "schema": SCHEMA, "step": "locale"}))


def validate_install(root: Path) -> dict:
    marker = load_closed(root / "install.json", {"architecture", "hardware", "image_schema", "product", "slots"})
    if marker["product"] != PRODUCT or marker["architecture"] != ARCH or marker["image_schema"] != SCHEMA or marker["slots"] != ["a", "b"]:
        raise LifecycleError("installation marker mismatch")
    if set(p.name for p in root.iterdir() if p.is_file()) - STATE_FILES:
        raise LifecycleError("unknown installation state")
    return marker


def first_run(root: Path, locale: str, username: str, model: str) -> None:
    validate_install(root)
    if not re.fullmatch(r"[a-z]{2}_[A-Z]{2}\.UTF-8", locale):
        raise LifecycleError("invalid locale")
    if not re.fullmatch(r"[a-z_][a-z0-9_-]{0,30}", username) or model not in MODEL_PROFILES:
        raise LifecycleError("invalid first-run selection")
    state = {"completed": True, "locale": locale, "model_profile": model,
             "schema": SCHEMA, "username": username}
    atomic_write(root / "first-run.json", canonical(state))


UPDATE_FIELDS = {"architecture", "channel", "expires", "minimum_schema", "payload_bytes",
                 "payload_sha256", "product", "sequence", "target_slot", "version"}


def verify_update(metadata: Path, signature: Path, allowed_signers: Path, payload: Path,
                  now: int, current_sequence: int, active: str) -> VerifiedUpdate:
    metadata_bytes = read_regular(metadata, MAX_METADATA_BYTES, "metadata")
    payload_bytes = read_regular(payload, MAX_EVIDENCE_PAYLOAD_BYTES, "payload")
    signature_bytes = read_regular(signature, MAX_SIGNATURE_BYTES, "signature")
    update = load_closed_bytes(metadata_bytes, UPDATE_FIELDS)
    if update["product"] != PRODUCT or update["architecture"] != ARCH or update["channel"] != "evidence":
        raise LifecycleError("update identity mismatch")
    if update["minimum_schema"] != SCHEMA or update["target_slot"] not in SLOTS or update["target_slot"] == active:
        raise LifecycleError("update schema or slot mismatch")
    if not all(isinstance(update[k], int) for k in ("expires", "payload_bytes", "sequence")):
        raise LifecycleError("invalid update integer")
    if update["expires"] <= now or update["sequence"] <= current_sequence:
        raise LifecycleError("expired or downgraded update")
    if len(payload_bytes) != update["payload_bytes"] or hashlib.sha256(payload_bytes).hexdigest() != update["payload_sha256"] or not HEX64.fullmatch(update["payload_sha256"]):
        raise LifecycleError("payload mismatch")
    with tempfile.NamedTemporaryFile(mode="wb", prefix="blossom-update-signature-") as fixed_signature:
        fixed_signature.write(signature_bytes)
        fixed_signature.flush()
        result = subprocess.run(
            ["ssh-keygen", "-Y", "verify", "-f", str(allowed_signers),
             "-I", "blossom-release", "-n", "blossom-update", "-s", fixed_signature.name],
            input=metadata_bytes, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    if result.returncode:
        raise LifecycleError("update signature mismatch")
    return VerifiedUpdate(metadata_bytes=metadata_bytes, payload=payload_bytes)


def stage(root: Path, verified: VerifiedUpdate) -> None:
    validate_install(root)
    update = load_closed_bytes(verified.metadata_bytes, UPDATE_FIELDS)
    target = update["target_slot"]
    slot = root / "slots" / target
    atomic_write(slot / "payload", verified.payload, 0o644)
    atomic_write(slot / "payload.sha256", f'{update["payload_sha256"]}\n'.encode(), 0o644)
    state = {"pending": target, "previous": active_slot(root), "sequence": update["sequence"],
             "status": "pending", "version": update["version"]}
    atomic_write(root / "update.json", canonical(state))
    atomic_write(root / "active-slot", f"{target}\n".encode())


def active_slot(root: Path) -> str:
    try:
        slot = read_regular(root / "active-slot", 2, "active slot").decode("ascii").strip()
    except UnicodeDecodeError as error:
        raise LifecycleError("invalid active slot") from error
    if slot not in SLOTS:
        raise LifecycleError("invalid active slot")
    return slot


def boot_result(root: Path, healthy: bool) -> None:
    validate_install(root)
    state = load_closed(root / "update.json", {"pending", "previous", "sequence", "status", "version"})
    if state["status"] != "pending" or active_slot(root) != state["pending"]:
        raise LifecycleError("no matching pending boot")
    if healthy:
        state["status"] = "confirmed"
    else:
        atomic_write(root / "active-slot", f'{state["previous"]}\n'.encode())
        state["status"] = "rolled_back"
    atomic_write(root / "update.json", canonical(state))


def recover(root: Path, slot: str) -> None:
    validate_install(root)
    if slot not in SLOTS:
        raise LifecycleError("invalid recovery slot")
    digest_path = root / "slots" / slot / "payload.sha256"
    payload_path = root / "slots" / slot / "payload"
    if not digest_path.is_file() or not payload_path.is_file():
        raise LifecycleError("recovery slot is not populated")
    try:
        expected = read_regular(digest_path, 65, "recovery digest").decode("ascii").strip()
    except UnicodeDecodeError as error:
        raise LifecycleError("recovery digest is invalid") from error
    payload = read_regular(payload_path, MAX_EVIDENCE_PAYLOAD_BYTES, "recovery payload")
    if not HEX64.fullmatch(expected) or hashlib.sha256(payload).hexdigest() != expected:
        raise LifecycleError("recovery slot verification failed")
    atomic_write(root / "active-slot", f"{slot}\n".encode())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("inspect",))
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    validate_install(args.root)
    print(f"product={PRODUCT} architecture={ARCH} active_slot={active_slot(args.root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
