import json
import unittest

from scripts.distribution.physical_device_observer import (
    LSBLK,
    ObservationError,
    parse_archiso_search_uuid,
    parse_lsblk,
)


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
    def test_lsblk_explicitly_requests_tree_for_child_mountpoints(self):
        self.assertIn("--tree", LSBLK)
        self.assertIn("UUID", LSBLK[-1])
        self.assertIn("MOUNTPOINTS", LSBLK[-1])

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

    def test_archiso_live_media_mount_is_recognized(self):
        value = inventory()
        value["blockdevices"][0]["children"][0]["mountpoints"] = [None]
        value["blockdevices"][1]["children"] = [
            {"type": "part", "mountpoints": ["/run/archiso/bootmnt"]}
        ]
        self.assertEqual(parse_lsblk(json.dumps(value).encode())[0], "/dev/sdb")

    def test_archiso_search_uuid_uniquely_identifies_unmounted_boot_media(self):
        value = inventory()
        value["blockdevices"][0]["children"][0]["mountpoints"] = [None]
        value["blockdevices"][1]["children"] = [
            {
                "type": "part",
                "uuid": "2026-09-12-15-40-08-00",
                "mountpoints": [None],
            }
        ]
        cmdline = (
            b"initrd=\\blossom\\boot\\x86_64\\initramfs-linux.img "
            b"archisobasedir=blossom "
            b"archisosearchuuid=2026-09-12-15-40-08-00"
        )
        search_uuid = parse_archiso_search_uuid(cmdline)
        self.assertEqual(search_uuid, "2026-09-12-15-40-08-00")
        self.assertEqual(parse_lsblk(json.dumps(value).encode(), search_uuid)[0], "/dev/sdb")

    def test_archiso_search_uuid_must_be_unique_and_agree_with_mount_identity(self):
        duplicate = inventory()
        duplicate["blockdevices"][0]["children"][0]["mountpoints"] = [None]
        for node in duplicate["blockdevices"]:
            node["uuid"] = "same-uuid"
        conflicting = inventory()
        conflicting["blockdevices"][1]["uuid"] = "boot-uuid"
        for value, search_uuid in ((duplicate, "same-uuid"), (conflicting, "boot-uuid")):
            with self.subTest(value=value), self.assertRaises(ObservationError):
                parse_lsblk(json.dumps(value).encode(), search_uuid)

    def test_archiso_search_uuid_parser_is_bounded_and_closed(self):
        self.assertIsNone(parse_archiso_search_uuid(b"quiet archisobasedir=blossom"))
        invalid = (
            b"archisosearchuuid=one archisosearchuuid=two",
            b"archisosearchuuid=../../device",
            b"x" * (4 * 1024 + 1),
        )
        for cmdline in invalid:
            with self.subTest(cmdline=cmdline), self.assertRaises(ObservationError):
                parse_archiso_search_uuid(cmdline)

    def test_empty_card_reader_slot_is_not_observed_as_media(self):
        value = inventory()
        value["blockdevices"].append(
            {
                "path": "/dev/sdc",
                "type": "disk",
                "model": "SD Card Reader",
                "size": 0,
                "tran": "usb",
                "rm": True,
                "mountpoints": [None],
            }
        )
        _, devices = parse_lsblk(json.dumps(value).encode())
        self.assertEqual([device["path"] for device in devices], ["/dev/sda", "/dev/sdb"])

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
