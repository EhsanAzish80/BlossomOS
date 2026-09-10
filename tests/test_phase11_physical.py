import json
from pathlib import Path
import tempfile
import unittest

from scripts.distribution.physical_preflight import PreflightError, classify, observe_host


def valid_observation():
    return {
        "architecture": "x86_64",
        "firmware": "uefi",
        "product_name": "MacBookPro11,1",
        "memory_mib": 8192,
        "drm_present": True,
        "internal_disk_present": True,
    }


class Phase11PhysicalPreflightTests(unittest.TestCase):
    def test_exact_target_is_only_eligible_for_qualification(self):
        result = classify(valid_observation())
        self.assertEqual(result["result"], "eligible_for_qualification")
        self.assertEqual(result["authority"], "read_only_preflight_only")
        self.assertTrue(all(result["checks"].values()))

    def test_wrong_target_or_required_property_is_ineligible(self):
        for field, value in (
            ("architecture", "aarch64"),
            ("firmware", "bios"),
            ("product_name", "MacBookPro12,1"),
            ("memory_mib", 2048),
            ("drm_present", False),
            ("internal_disk_present", False),
        ):
            observation = valid_observation()
            observation[field] = value
            with self.subTest(field=field):
                self.assertEqual(classify(observation)["result"], "ineligible")

    def test_schema_and_types_fail_closed(self):
        observation = valid_observation()
        observation["serial"] = "private"
        with self.assertRaises(PreflightError):
            classify(observation)
        observation = valid_observation()
        observation["drm_present"] = 1
        with self.assertRaises(PreflightError):
            classify(observation)

    def test_output_has_no_stable_identifiers_or_disk_path(self):
        encoded = json.dumps(classify(valid_observation())).lower()
        for forbidden in ("serial", "uuid", "mac_address", "ssid", "/dev/", "username"):
            self.assertNotIn(forbidden, encoded)

    def test_host_observation_reads_only_the_minimized_linux_surface(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "proc").mkdir()
            (root / "proc/meminfo").write_text("MemTotal:        8388608 kB\n")
            (root / "sys/firmware/efi").mkdir(parents=True)
            (root / "sys/class/dmi/id").mkdir(parents=True)
            (root / "sys/class/dmi/id/product_name").write_text("MacBookPro11,1\n")
            (root / "dev/dri").mkdir(parents=True)
            (root / "dev/dri/card1").touch()
            disk = root / "sys/block/sda"
            (disk / "device").mkdir(parents=True)
            (disk / "removable").write_text("0\n")
            observed = observe_host(root, architecture="x86_64")
            self.assertEqual(observed, valid_observation())
            self.assertEqual(classify(observed)["result"], "eligible_for_qualification")

    def test_host_observation_does_not_treat_removable_media_as_internal(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "proc").mkdir()
            (root / "proc/meminfo").write_text("MemTotal: 8388608 kB\n")
            disk = root / "sys/block/sdb"
            (disk / "device").mkdir(parents=True)
            (disk / "removable").write_text("1\n")
            self.assertFalse(observe_host(root, architecture="x86_64")["internal_disk_present"])


class Phase11RepositoryBoundaryTests(unittest.TestCase):
    def test_vm_installer_remains_exactly_virtio_bound(self):
        root = Path(__file__).resolve().parents[1]
        installer = (root / "distribution/archiso/airootfs/usr/local/bin/blossom-evidence-install").read_text()
        unit = (root / "distribution/evidence/blossom-evidence-install.service").read_text()
        self.assertIn("disk=/dev/vda", installer)
        self.assertIn("disk=virtio", installer)
        self.assertNotIn("physical", installer.lower())
        self.assertIn("WantedBy=multi-user.target", unit)


if __name__ == "__main__":
    unittest.main()
