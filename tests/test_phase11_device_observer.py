import json
import unittest

from scripts.distribution.physical_device_observer import ObservationError, parse_lsblk


def inventory():
    return {
        "blockdevices": [
            {
                "path": "/dev/sda",
                "type": "disk",
                "model": "APPLE SSD SM0128F ",
                "size": 121332826112,
                "tran": "sata",
                "rm": False,
                "mountpoints": [None],
                "children": [{"type": "part", "mountpoints": ["/"]}],
            },
            {
                "path": "/dev/sdb",
                "type": "disk",
                "model": "DISPOSABLE",
                "size": 64000000000,
                "tran": "usb",
                "rm": True,
                "mountpoints": [None],
            },
        ]
    }


class Phase11DeviceObserverTests(unittest.TestCase):
    def test_normalizes_closed_inventory_and_finds_live_root_disk(self):
        live, devices = parse_lsblk(json.dumps(inventory()).encode())
        self.assertEqual(live, "/dev/sda")
        self.assertEqual(devices[0]["model"], "APPLE SSD SM0128F")
        self.assertTrue(devices[0]["mounted"])
        self.assertEqual(devices[1]["transport"], "usb")

    def test_live_media_cdrom_mount_is_recognized(self):
        value = inventory()
        value["blockdevices"][0]["children"][0]["mountpoints"] = [None]
        value["blockdevices"][1]["children"] = [{"type": "part", "mountpoints": ["/cdrom"]}]
        self.assertEqual(parse_lsblk(json.dumps(value).encode())[0], "/dev/sdb")

    def test_ambiguous_missing_and_unsupported_inventories_fail_closed(self):
        cases = []
        no_live = inventory()
        no_live["blockdevices"][0]["children"][0]["mountpoints"] = [None]
        cases.append(no_live)
        ambiguous = inventory()
        ambiguous["blockdevices"][1]["mountpoints"] = ["/cdrom"]
        cases.append(ambiguous)
        unsupported = inventory()
        unsupported["blockdevices"][1]["tran"] = "firewire"
        cases.append(unsupported)
        invalid_children = inventory()
        invalid_children["blockdevices"][0]["children"] = None
        cases.append(invalid_children)
        for value in cases:
            with self.subTest(value=value), self.assertRaises(ObservationError):
                parse_lsblk(json.dumps(value).encode())

    def test_output_and_device_count_are_bounded(self):
        with self.assertRaises(ObservationError):
            parse_lsblk(b" " * (64 * 1024 + 1))
        value = inventory()
        value["blockdevices"] = [value["blockdevices"][0]] * 9
        with self.assertRaises(ObservationError):
            parse_lsblk(json.dumps(value).encode())


if __name__ == "__main__":
    unittest.main()
