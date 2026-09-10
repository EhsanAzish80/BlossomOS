import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.distribution.disposable_media_probe import (
    PROBE_BYTES,
    PROBE_OFFSET,
    ProbeError,
    _probe_fd,
    probe,
)


def target(size=64 * 1024**3):
    return {
        "challenge": "0" * 32,
        "model": "DISPOSABLE",
        "path": "/dev/sdz",
        "purpose": "disposable_test",
        "size_bytes": size,
        "transport": "usb",
    }


class DisposableProbeTests(unittest.TestCase):
    def test_bounded_probe_reads_writes_verifies_and_restores(self):
        original = bytes((index % 251 for index in range(PROBE_BYTES)))
        with tempfile.TemporaryFile() as device:
            device.seek(PROBE_OFFSET)
            device.write(original)
            device.flush()
            with patch(
                "scripts.distribution.disposable_media_probe.fcntl.ioctl",
                return_value=(64 * 1024**3).to_bytes(8, "little"),
            ):
                result = _probe_fd(device.fileno(), target())
            device.seek(PROBE_OFFSET)
            self.assertEqual(device.read(PROBE_BYTES), original)
        self.assertEqual(result["result"], "disposable_probe_completed_and_restored")
        self.assertEqual(result["length_bytes"], 4096)
        self.assertNotIn("original", result)

    def test_size_change_fails_before_write(self):
        with tempfile.TemporaryFile() as device, patch(
            "scripts.distribution.disposable_media_probe.fcntl.ioctl",
            return_value=(63 * 1024**3).to_bytes(8, "little"),
        ):
            with self.assertRaisesRegex(ProbeError, "size does not match"):
                _probe_fd(device.fileno(), target())

    def test_failed_marker_verification_still_restores_original_bytes(self):
        original = b"o" * PROBE_BYTES
        with tempfile.TemporaryFile() as device:
            device.seek(PROBE_OFFSET)
            device.write(original)
            device.flush()
            with (
                patch(
                    "scripts.distribution.disposable_media_probe.fcntl.ioctl",
                    return_value=(64 * 1024**3).to_bytes(8, "little"),
                ),
                patch(
                    "scripts.distribution.disposable_media_probe._read_exact",
                    side_effect=[original, b"x" * PROBE_BYTES, original],
                ),
                self.assertRaisesRegex(ProbeError, "read-back failed"),
            ):
                _probe_fd(device.fileno(), target())
            device.seek(PROBE_OFFSET)
            self.assertEqual(device.read(PROBE_BYTES), original)

    def test_public_probe_rejects_regular_files_and_wrong_authority(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "not-a-device"
            path.write_bytes(b"\0" * PROBE_BYTES)
            value = target()
            value["path"] = str(path)
            with self.assertRaises(ProbeError):
                probe(value)
        for field, value in (("purpose", "physical_install"), ("transport", "sata")):
            changed = target()
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(ProbeError):
                probe(changed)


if __name__ == "__main__":
    unittest.main()
