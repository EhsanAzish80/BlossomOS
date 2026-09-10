#!/usr/bin/env python3
"""Bounded write/read/restore probe for an already-authorized block device."""

from __future__ import annotations

import fcntl
import hashlib
import json
import os
import stat
import struct
from typing import Any

from scripts.distribution.physical_install_guard import DEVICE_PATH


class ProbeError(RuntimeError):
    """Raised when the disposable-media probe cannot complete and restore."""


BLKGETSIZE64 = 0x80081272
PROBE_OFFSET = 8 * 1024 * 1024
PROBE_BYTES = 4096
TARGET_FIELDS = {"challenge", "model", "path", "purpose", "size_bytes", "transport"}


def _pattern(target: dict[str, Any]) -> bytes:
    seed = hashlib.sha256(
        b"blossom-phase11-disposable-probe-v1\0"
        + json.dumps(target, sort_keys=True, separators=(",", ":")).encode()
    ).digest()
    return (seed * ((PROBE_BYTES + len(seed) - 1) // len(seed)))[:PROBE_BYTES]


def _read_exact(fd: int, offset: int) -> bytes:
    value = os.pread(fd, PROBE_BYTES, offset)
    if len(value) != PROBE_BYTES:
        raise ProbeError("bounded device read was incomplete")
    return value


def _write_exact(fd: int, value: bytes, offset: int) -> None:
    written = 0
    while written < len(value):
        count = os.pwrite(fd, value[written:], offset + written)
        if count <= 0:
            raise ProbeError("bounded device write was incomplete")
        written += count


def _probe_fd(fd: int, target: dict[str, Any]) -> dict[str, Any]:
    """Write one bounded probe and restore the original bytes before returning."""
    size = struct.unpack("Q", fcntl.ioctl(fd, BLKGETSIZE64, b"\0" * 8))[0]
    if size != target["size_bytes"] or PROBE_OFFSET + PROBE_BYTES > size:
        raise ProbeError("live device size does not match the authorized target")
    original = _read_exact(fd, PROBE_OFFSET)
    marker = _pattern(target)
    attempted_marker = False
    try:
        attempted_marker = True
        _write_exact(fd, marker, PROBE_OFFSET)
        os.fsync(fd)
        if _read_exact(fd, PROBE_OFFSET) != marker:
            raise ProbeError("bounded marker read-back failed")
    finally:
        if attempted_marker:
            try:
                _write_exact(fd, original, PROBE_OFFSET)
                os.fsync(fd)
                if _read_exact(fd, PROBE_OFFSET) != original:
                    raise ProbeError("recovery read-back failed")
            except Exception as error:
                raise ProbeError("bounded probe could not restore its recovery region") from error
    return {
        "schema": 1,
        "result": "disposable_probe_completed_and_restored",
        "offset_bytes": PROBE_OFFSET,
        "length_bytes": PROBE_BYTES,
        "target_digest": hashlib.sha256(
            json.dumps(target, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest(),
    }


def probe(target: dict[str, Any]) -> dict[str, Any]:
    """Open the exact block target exclusively and run the bounded probe."""
    if type(target) is not dict or set(target) != TARGET_FIELDS:
        raise ProbeError("authorized target schema drift")
    if target["purpose"] != "disposable_test" or target["transport"] != "usb":
        raise ProbeError("probe accepts only disposable USB authority")
    path = target["path"]
    if not isinstance(path, str) or not DEVICE_PATH.fullmatch(path):
        raise ProbeError("invalid authorized device path")
    flags = os.O_RDWR | os.O_EXCL | os.O_CLOEXEC
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        fd = os.open(path, flags)
    except OSError as error:
        raise ProbeError("authorized device could not be opened exclusively") from error
    try:
        if not stat.S_ISBLK(os.fstat(fd).st_mode):
            raise ProbeError("authorized path is not a block device")
        return _probe_fd(fd, target)
    finally:
        os.close(fd)
