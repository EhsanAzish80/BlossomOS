import unittest

from scripts.distribution.physical_install_backend import (
    BackendError,
    TARGET,
    command_plan,
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

    def test_command_plan_has_only_fixed_sda_partitioning(self):
        plan = command_plan()
        rendered = "\n".join(" ".join(command) for command in plan)
        self.assertIn("sgdisk --zap-all /dev/sda", rendered)
        self.assertIn("mkfs.fat -F 32 -n BLOSSOM_EFI /dev/sda1", rendered)
        self.assertIn("mkfs.ext4 -F -L BLOSSOM_SYSTEM /dev/sda2", rendered)
        for forbidden in ("/dev/sdb", "/dev/vda", "sh -c", "bash -c"):
            self.assertNotIn(forbidden, rendered)
        self.assertTrue(all(type(command) is list for command in plan))


if __name__ == "__main__":
    unittest.main()
