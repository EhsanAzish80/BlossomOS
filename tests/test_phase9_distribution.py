import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from scripts.distribution.blossom_lifecycle import (
    LifecycleError, active_slot, boot_result, canonical, first_run,
    hardware_record, install, recover, stage, verify_update,
)


class Phase9LifecycleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)
        self.root = self.base / "system"
        self.hardware = hardware_record("x86_64", "uefi", "virtio", 4096, True)
        install(self.root, self.hardware)

    def tearDown(self):
        self.temporary.cleanup()

    def signed_update(self, sequence=1, target="b", expires=2_000_000_000):
        suffix = f"{sequence}-{target}-{expires}"
        private = self.base / f"release-key-{suffix}"
        public = self.base / f"allowed-signers-{suffix}"
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", private], check=True)
        public.write_text("blossom-release " + Path(f"{private}.pub").read_text())
        payload = self.base / f"payload-{suffix}"
        payload.write_bytes(b"verified-blossom-slot-v2")
        metadata = self.base / f"update-{suffix}.json"
        value = {"architecture": "x86_64", "channel": "evidence", "expires": expires,
                 "minimum_schema": 1, "payload_bytes": payload.stat().st_size,
                 "payload_sha256": hashlib.sha256(payload.read_bytes()).hexdigest(),
                 "product": "blossom-os", "sequence": sequence, "target_slot": target,
                 "version": "0.9.1"}
        metadata.write_bytes(canonical(value))
        subprocess.run(["ssh-keygen", "-Y", "sign", "-f", private, "-n", "blossom-update", metadata],
                       check=True, stdout=subprocess.DEVNULL)
        signature = Path(f"{metadata}.sig")
        return value, metadata, signature, public, payload

    def test_hardware_and_first_run_are_closed_and_content_minimized(self):
        first_run(self.root, "en_US.UTF-8", "ehsan", "none")
        state = json.loads((self.root / "first-run.json").read_text())
        self.assertTrue(state["completed"])
        self.assertEqual(set(self.hardware), {"architecture", "firmware", "graphics", "memory_class", "virtio"})
        for bad in (("arm64", "uefi", "virtio", 4096, True), ("x86_64", "bios", "virtio", 4096, True),
                    ("x86_64", "uefi", "virtio", 1024, True), ("x86_64", "uefi", "virtio", 4096, False)):
            with self.assertRaises(LifecycleError): hardware_record(*bad)
        with self.assertRaises(LifecycleError): first_run(self.root, "bad", "Root User", "remote-model")

    def test_signed_update_rolls_back_then_confirms_without_touching_user_data(self):
        user = self.root / "user-data/note"
        user.write_text("private")
        _, metadata, signature, public, payload = self.signed_update()
        update = verify_update(metadata, signature, public, payload, 1_900_000_000, 0, active_slot(self.root))
        stage(self.root, update, payload)
        self.assertEqual(active_slot(self.root), "b")
        boot_result(self.root, False)
        self.assertEqual(active_slot(self.root), "a")
        self.assertEqual(user.read_text(), "private")
        update = verify_update(metadata, signature, public, payload, 1_900_000_000, 0, active_slot(self.root))
        stage(self.root, update, payload)
        boot_result(self.root, True)
        self.assertEqual(active_slot(self.root), "b")
        self.assertEqual(user.read_text(), "private")

    def test_signature_expiry_downgrade_mutation_and_wrong_slot_fail_closed(self):
        value, metadata, signature, public, payload = self.signed_update()
        with self.assertRaises(LifecycleError): verify_update(metadata, signature, public, payload, 2_000_000_001, 0, "a")
        with self.assertRaises(LifecycleError): verify_update(metadata, signature, public, payload, 1, 1, "a")
        payload.write_bytes(b"changed")
        with self.assertRaises(LifecycleError): verify_update(metadata, signature, public, payload, 1, 0, "a")
        payload.write_bytes(b"verified-blossom-slot-v2")
        value["unknown"] = True
        metadata.write_bytes(canonical(value))
        with self.assertRaises(LifecycleError): verify_update(metadata, signature, public, payload, 1, 0, "a")
        _, metadata, signature, public, payload = self.signed_update(target="a")
        with self.assertRaises(LifecycleError): verify_update(metadata, signature, public, payload, 1, 0, "a")

    def test_recovery_requires_exact_marker_and_verified_populated_slot(self):
        with self.assertRaises(LifecycleError): recover(self.root, "b")
        _, metadata, signature, public, payload = self.signed_update()
        update = verify_update(metadata, signature, public, payload, 1, 0, "a")
        stage(self.root, update, payload)
        recover(self.root, "b")
        self.assertEqual(active_slot(self.root), "b")
        (self.root / "slots/b/payload").write_bytes(b"tampered")
        with self.assertRaises(LifecycleError): recover(self.root, "b")
        (self.root / "install.json").unlink()
        with self.assertRaises(LifecycleError): recover(self.root, "a")


class Phase9RepositoryTests(unittest.TestCase):
    def test_distribution_manifest_is_closed_and_safe(self):
        root = Path(__file__).resolve().parents[1]
        manifest = json.loads((root / "distribution/manifest.json").read_text())
        self.assertEqual(set(manifest), {"architecture", "boot", "channel", "image_schema", "packages", "product", "snapshot", "ssh_enabled", "telemetry"})
        self.assertEqual((manifest["architecture"], manifest["boot"], manifest["product"]), ("x86_64", "uefi", "blossom-os"))
        self.assertFalse(manifest["ssh_enabled"])
        self.assertFalse(manifest["telemetry"])
        text = "\n".join(p.read_text(errors="replace") for p in (root / "distribution").rglob("*.*"))
        for forbidden in ("PermitRootLogin yes", "PasswordAuthentication yes", "NOPASSWD", "curl |", "wget |", "sshd.service"):
            self.assertNotIn(forbidden, text)


if __name__ == "__main__":
    unittest.main()
