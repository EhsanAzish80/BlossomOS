import copy
import unittest

from scripts.distribution.physical_install_guard import GuardError, evaluate


def observation():
    return {
        "schema": 1,
        "purpose": "physical_install",
        "host_preflight_result": "eligible_for_qualification",
        "ac_power": True,
        "recovery_media_ready": True,
        "live_device": "/dev/sdb",
        "challenge": "0123456789abcdef0123456789abcdef",
        "devices": [
            {
                "path": "/dev/sda",
                "model": "APPLE SSD SM0128F",
                "size_bytes": 121332826112,
                "transport": "sata",
                "removable": False,
                "mounted": False,
            },
            {
                "path": "/dev/sdb",
                "model": "RECOVERY MEDIA",
                "size_bytes": 500107862016,
                "transport": "sata",
                "removable": True,
                "mounted": True,
            },
        ],
    }


class Phase11InstallGuardTests(unittest.TestCase):
    def test_exact_confirmation_is_bound_and_still_performs_no_write(self):
        pending = evaluate(observation())
        self.assertEqual(pending["result"], "confirmation_required")
        self.assertEqual(pending["authority"], "guard_decision_only")
        confirmed = evaluate(observation(), pending["expected_confirmation"])
        self.assertEqual(confirmed["result"], "guard_passed_no_write_performed")
        self.assertEqual(confirmed["target_digest"], pending["target_digest"])

    def test_confirmation_rejects_case_spacing_target_and_challenge_changes(self):
        pending = evaluate(observation())
        expected = pending["expected_confirmation"]
        for changed in (expected.lower(), expected + " ", expected.replace("sda", "sdb")):
            with self.subTest(changed=changed):
                self.assertEqual(evaluate(observation(), changed)["result"], "confirmation_required")
        mutated = observation()
        mutated["challenge"] = "fedcba9876543210fedcba9876543210"
        self.assertEqual(evaluate(mutated, expected)["result"], "confirmation_required")

    def test_live_removable_mounted_and_ambiguous_targets_fail_closed(self):
        cases = []
        live_target = observation()
        live_target["live_device"] = "/dev/sda"
        cases.append(live_target)
        removable = observation()
        removable["devices"][0]["removable"] = True
        cases.append(removable)
        mounted = observation()
        mounted["devices"][0]["mounted"] = True
        cases.append(mounted)
        ambiguous = observation()
        ambiguous["devices"].append(
            {
                "path": "/dev/nvme0n1",
                "model": "SECOND INTERNAL",
                "size_bytes": 256000000000,
                "transport": "nvme",
                "removable": False,
                "mounted": False,
            }
        )
        cases.append(ambiguous)
        for case in cases:
            with self.assertRaises(GuardError):
                evaluate(case)

    def test_power_recovery_preflight_and_schema_expansion_fail_closed(self):
        for field in ("ac_power", "recovery_media_ready"):
            value = observation()
            value[field] = False
            with self.assertRaises(GuardError):
                evaluate(value)
        value = observation()
        value["host_preflight_result"] = "ineligible"
        with self.assertRaises(GuardError):
            evaluate(value)
        value = observation()
        value["serial"] = "private"
        with self.assertRaises(GuardError):
            evaluate(value)

    def test_device_mutation_changes_digest_and_invalid_values_are_rejected(self):
        original = evaluate(observation())["target_digest"]
        changed = observation()
        changed["devices"][0]["size_bytes"] += 512
        self.assertNotEqual(evaluate(changed)["target_digest"], original)
        for field, value in (
            ("path", "/dev/disk/by-id/private"),
            ("model", "bad\nmodel"),
            ("size_bytes", 1),
            ("transport", "firewire"),
            ("mounted", 0),
        ):
            invalid = copy.deepcopy(observation())
            invalid["devices"][0][field] = value
            with self.subTest(field=field), self.assertRaises(GuardError):
                evaluate(invalid)

    def test_disposable_usb_purpose_is_bound_and_cannot_select_internal_disk(self):
        value = observation()
        value["purpose"] = "disposable_test"
        value["devices"][1]["mounted"] = False
        value["devices"][1]["transport"] = "usb"
        value["live_device"] = "/dev/nvme0n1"
        value["devices"].append({
            "path": "/dev/nvme0n1", "model": "LIVE SYSTEM", "size_bytes": 64000000000,
            "transport": "nvme", "removable": False, "mounted": True,
        })
        pending = evaluate(value)
        self.assertEqual(pending["target"]["path"], "/dev/sdb")
        self.assertEqual(pending["target"]["purpose"], "disposable_test")
        install = copy.deepcopy(value)
        install["purpose"] = "physical_install"
        self.assertEqual(evaluate(install, pending["expected_confirmation"])["result"], "confirmation_required")

    def test_physical_install_never_accepts_usb_target(self):
        value = observation()
        value["devices"][0]["transport"] = "usb"
        with self.assertRaises(GuardError):
            evaluate(value)


if __name__ == "__main__":
    unittest.main()
