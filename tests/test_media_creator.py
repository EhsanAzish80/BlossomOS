import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.distribution import media_creator as media


class MediaCreatorTests(unittest.TestCase):
    def release(self, directory: str):
        iso = Path(directory) / "blossom-os-test-x86_64.iso"
        sums = Path(directory) / "SHA256SUMS"
        iso.write_bytes(b"blossom-test-image")
        digest = hashlib.sha256(iso.read_bytes()).hexdigest()
        sums.write_text(f"{digest}  {iso.name}\n")
        return iso, sums

    def test_release_checksum_must_match(self):
        with tempfile.TemporaryDirectory() as directory:
            iso, sums = self.release(directory)
            self.assertEqual(media.verify_release(iso, sums), hashlib.sha256(iso.read_bytes()).hexdigest())
            iso.write_bytes(b"changed")
            with self.assertRaises(media.MediaError):
                media.verify_release(iso, sums)

    def test_inventory_only_never_writes(self):
        with tempfile.TemporaryDirectory() as directory:
            iso, sums = self.release(directory)
            device = media.Device("/dev/disk9", "TEST USB", 8_000_000_000, True, True)
            with patch.object(media, "inventory", return_value=[device]), patch.object(media, "write_image") as writer:
                media.run(iso, sums, None)
            writer.assert_not_called()

    def test_confirmation_and_reobservation_precede_write(self):
        with tempfile.TemporaryDirectory() as directory:
            iso, sums = self.release(directory)
            device = media.Device("/dev/disk9", "TEST USB", 8_000_000_000, True, True)
            challenge = f"ERASE {device.identity}"
            with patch.object(media, "inventory", side_effect=[[device], [device]]) as observed, patch.object(media, "write_image") as writer:
                media.run(iso, sums, device.path, read=lambda _prompt: challenge)
            self.assertEqual(observed.call_count, 2)
            writer.assert_called_once_with(iso, device)

    def test_identity_change_stops_before_write(self):
        with tempfile.TemporaryDirectory() as directory:
            iso, sums = self.release(directory)
            first = media.Device("/dev/disk9", "TEST USB", 8_000_000_000, True, True)
            changed = media.Device("/dev/disk9", "OTHER USB", 8_000_000_000, True, True)
            with patch.object(media, "inventory", side_effect=[[first], [changed]]), patch.object(media, "write_image") as writer:
                with self.assertRaises(media.MediaError):
                    media.run(iso, sums, first.path, read=lambda _prompt: f"ERASE {first.identity}")
            writer.assert_not_called()

    def test_disk_smaller_than_iso_stops_before_confirmation(self):
        with tempfile.TemporaryDirectory() as directory:
            iso, sums = self.release(directory)
            device = media.Device("/dev/disk9", "TINY USB", 1, True, True)
            with patch.object(media, "inventory", return_value=[device]), patch.object(media, "write_image") as writer:
                with self.assertRaises(media.MediaError):
                    media.run(iso, sums, device.path, read=lambda _prompt: self.fail("must not prompt"))
            writer.assert_not_called()

    def test_macos_writer_verifies_before_eject(self):
        with tempfile.TemporaryDirectory() as directory:
            iso, _ = self.release(directory)
            device = media.Device("/dev/disk9", "TEST USB", 8_000_000_000, True, True)
            calls = []
            with patch.object(media.sys, "platform", "darwin"), patch.object(
                media.subprocess, "run", side_effect=lambda command, **_kwargs: calls.append(command)
            ):
                media.write_image(iso, device)
            commands = [command[1] if command[0] == "sudo" else command[0] for command in calls]
            self.assertLess(commands.index("cmp"), commands.index("diskutil", 1))


if __name__ == "__main__":
    unittest.main()
