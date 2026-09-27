import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from scripts.distribution.physical_install_backend import (
    BackendError,
    LSBLK_TARGET,
    SLOT_SYNC_PATHS,
    STATE_DIRECTORIES,
    TARGET,
    command_plan,
    sync_slot_state,
    validate_target,
)


def target():
    return {**TARGET, "challenge": "0123456789abcdef0123456789abcdef"}


def inventory():
    return {
        "blockdevices": [
            {
                "path": "/dev/sda",
                "size": 121332826112,
                "model": "APPLE SSD SM0128F",
                "tran": "sata",
                "rm": False,
                "mountpoints": [None],
                "children": [],
            }
        ]
    }


class PhysicalInstallBackendTests(unittest.TestCase):
    def test_target_inventory_explicitly_preserves_one_disk_tree(self):
        self.assertIn("--tree", LSBLK_TARGET)
        self.assertEqual(LSBLK_TARGET[-1], "/dev/sda")
        self.assertIn("MOUNTPOINTS", LSBLK_TARGET[-2])

    def test_flat_lsblk_partition_roots_reproduce_the_physical_failure(self):
        flat = inventory()
        flat["blockdevices"].extend(
            [
                {"path": "/dev/sda1", "mountpoints": [None]},
                {"path": "/dev/sda2", "mountpoints": [None]},
            ]
        )
        with self.assertRaisesRegex(BackendError, "exactly one inventory root"):
            validate_target(target(), flat)

        tree = inventory()
        tree["blockdevices"][0]["children"] = [
            {"path": "/dev/sda1", "mountpoints": [None]},
            {"path": "/dev/sda2", "mountpoints": [None]},
        ]
        validate_target(target(), tree)

    def test_only_exact_frozen_target_is_accepted(self):
        validate_target(target(), inventory())
        for field, value in (
            ("path", "/dev/sdb"),
            ("model", "OTHER"),
            ("size_bytes", 121332826624),
            ("transport", "usb"),
            ("purpose", "disposable_test"),
        ):
            changed = target()
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(BackendError):
                validate_target(changed, inventory())

    def test_live_inventory_drift_or_mount_fails_closed(self):
        cases = []
        for field, value in (
            ("path", "/dev/sdb"),
            ("size", 121332826624),
            ("model", "OTHER"),
            ("tran", "usb"),
            ("rm", True),
        ):
            changed = inventory()
            changed["blockdevices"][0][field] = value
            cases.append(changed)
        mounted = inventory()
        mounted["blockdevices"][0]["children"] = [{"mountpoints": ["/mnt"]}]
        cases.append(mounted)
        invalid_children = inventory()
        invalid_children["blockdevices"][0]["children"] = "not-a-list"
        cases.append(invalid_children)
        for changed in cases:
            with self.subTest(changed=changed), self.assertRaises(BackendError):
                validate_target(target(), changed)

    def test_command_plan_creates_dual_root_and_never_constructs_children(self):
        plan = command_plan()
        rendered = "\n".join(" ".join(command) for command in plan)
        self.assertIn("sgdisk --zap-all /dev/sda", rendered)
        self.assertIn("BLOSSOM_EFI", rendered)
        self.assertIn("BLOSSOM_ROOT_A", rendered)
        self.assertIn("BLOSSOM_ROOT_B", rendered)
        self.assertIn("BLOSSOM_STATE", rendered)
        for forbidden in ("/dev/sda1", "/dev/sda2", "/dev/sdb", "/dev/vda", "sh -c", "bash -c"):
            self.assertNotIn(forbidden, rendered)
        self.assertTrue(all(type(command) is list for command in plan))

    def test_only_directories_are_persistent_bind_mounts(self):
        self.assertEqual(
            set(STATE_DIRECTORIES),
            {"home", "var/lib/blossom", "var/log", "etc/NetworkManager/system-connections", "var/lib/bluetooth"},
        )
        self.assertTrue({"etc/passwd", "etc/shadow", "etc/group", "etc/gshadow"}.issubset(SLOT_SYNC_PATHS))
        self.assertTrue({"etc/hostname", "etc/machine-id", "etc/locale.conf", "etc/vconsole.conf"}.issubset(SLOT_SYNC_PATHS))

    def test_slot_sync_preserves_regular_modes_and_timezone_symlink(self):
        with TemporaryDirectory() as temporary:
            base = Path(temporary)
            source = base / "a"
            destination = base / "b"
            source.mkdir()
            destination.mkdir()
            for relative in SLOT_SYNC_PATHS:
                path = source / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                if relative == "etc/localtime":
                    path.symlink_to("/usr/share/zoneinfo/Europe/Istanbul")
                else:
                    path.write_text(f"{relative}\n", encoding="utf-8")
                    path.chmod(0o400 if relative == "etc/shadow" else 0o644)
            sync_slot_state(source, destination)
            self.assertEqual((destination / "etc/shadow").stat().st_mode & 0o777, 0o400)
            self.assertTrue((destination / "etc/localtime").is_symlink())
            self.assertEqual(
                (destination / "etc/localtime").readlink(),
                Path("/usr/share/zoneinfo/Europe/Istanbul"),
            )

    def test_slot_sync_mirrors_missing_paths(self):
        with TemporaryDirectory() as temporary:
            base = Path(temporary)
            source = base / "a"
            destination = base / "b"
            source.mkdir()
            destination.mkdir()
            for relative in SLOT_SYNC_PATHS:
                source_path = source / relative
                source_path.parent.mkdir(parents=True, exist_ok=True)
                destination_path = destination / relative
                destination_path.parent.mkdir(parents=True, exist_ok=True)
                destination_path.write_text("stale\n", encoding="utf-8")
                if relative != "etc/vconsole.conf":
                    source_path.write_text("current\n", encoding="utf-8")
            sync_slot_state(source, destination)
            self.assertFalse((destination / "etc/vconsole.conf").exists())
            self.assertEqual((destination / "etc/passwd").read_text(), "current\n")

    def test_candidate_build_forces_an_empty_machine_identity(self):
        repository = Path(__file__).resolve().parents[1]
        builder = (repository / "scripts/distribution/build_physical_candidate.sh").read_text()
        self.assertIn(': > "$rootfs/etc/machine-id"', builder)
        self.assertIn('[[ -s "$rootfs/etc/machine-id" ]]', builder)
        self.assertNotIn("blossom-physical-install", builder)
        self.assertFalse(
            (repository / "distribution/archiso/airootfs/usr/local/bin/blossom-physical-install").exists()
        )
        self.assertFalse(
            (repository / "distribution/archiso/airootfs/usr/local/libexec/blossom-physical-install-backend").exists()
        )


if __name__ == "__main__":
    unittest.main()
