import tempfile
import unittest
from pathlib import Path

from scripts.distribution.physical_install_guard import evaluate
from scripts.distribution.physical_install_harness import InstallHarnessError, run_once
from tests.test_phase11_install_guard import observation


class PhysicalInstallHarnessTests(unittest.TestCase):
    def test_exact_confirmation_revalidation_and_once_only_claim(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory) / "claim"
            initial = observation()
            confirmation = evaluate(initial)["expected_confirmation"]
            calls = []
            self.assertEqual(
                run_once(state, initial, initial, confirmation, False, calls.append),
                "physical_install_completed",
            )
            self.assertEqual([call["path"] for call in calls], ["/dev/sda"])
            with self.assertRaisesRegex(InstallHarnessError, "already consumed"):
                run_once(state, initial, initial, confirmation, False, calls.append)

    def test_cancel_change_failure_and_wrong_purpose_are_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory) / "claim"
            initial = observation()
            confirmation = evaluate(initial)["expected_confirmation"]
            calls = []
            self.assertEqual(
                run_once(state, initial, initial, confirmation, True, calls.append),
                "cancelled_no_write",
            )
            changed = observation()
            changed["devices"][0]["size_bytes"] += 512
            with self.assertRaisesRegex(InstallHarnessError, "target changed"):
                run_once(state, initial, changed, confirmation, False, calls.append)
            disposable = observation()
            disposable["purpose"] = "disposable_test"
            disposable["live_device"] = "/dev/sda"
            disposable["devices"][0]["mounted"] = True
            disposable["devices"][1]["transport"] = "usb"
            disposable["devices"][1]["mounted"] = False
            disposable_confirmation = evaluate(disposable)["expected_confirmation"]
            with self.assertRaisesRegex(InstallHarnessError, "rejects disposable-test"):
                run_once(
                    state,
                    disposable,
                    disposable,
                    disposable_confirmation,
                    False,
                    calls.append,
                )
            self.assertFalse(state.exists())

            def fail(_target):
                raise OSError("installation failed")

            with self.assertRaisesRegex(InstallHarnessError, "failed after once-only claim"):
                run_once(state, initial, initial, confirmation, False, fail)
            self.assertTrue(state.exists())


if __name__ == "__main__":
    unittest.main()
