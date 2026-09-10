import tempfile
import unittest
from pathlib import Path

from scripts.distribution.physical_install_guard import evaluate
from scripts.distribution.physical_write_harness import HarnessError, run_once
from tests.test_phase11_install_guard import observation


def disposable_observation():
    value = observation()
    value["purpose"] = "disposable_test"
    value["live_device"] = "/dev/sda"
    value["devices"][0]["mounted"] = True
    value["devices"][1]["mounted"] = False
    value["devices"][1]["transport"] = "usb"
    return value


class HarnessTests(unittest.TestCase):
    def test_cancel_mutation_replay_and_failure_are_fail_closed(self):
        with tempfile.TemporaryDirectory() as d:
            state = Path(d) / "claim"
            obs = disposable_observation()
            confirmation = evaluate(obs)["expected_confirmation"]
            calls = []
            self.assertEqual(
                run_once(state, obs, obs, confirmation, True, calls.append),
                "cancelled_no_write",
            )
            self.assertFalse(state.exists())
            changed = disposable_observation()
            changed["devices"][1]["size_bytes"] += 512
            with self.assertRaises(HarnessError):
                run_once(state, obs, changed, confirmation, False, calls.append)
            self.assertFalse(state.exists())
            self.assertEqual(calls, [])
            self.assertEqual(
                run_once(state, obs, obs, confirmation, False, calls.append),
                "disposable_test_completed",
            )
            with self.assertRaises(HarnessError):
                run_once(state, obs, obs, confirmation, False, calls.append)
            self.assertEqual(len(calls), 1)

    def test_failure_consumes_attempt_and_cannot_retry(self):
        with tempfile.TemporaryDirectory() as d:
            state = Path(d) / "claim"
            obs = disposable_observation()
            confirmation = evaluate(obs)["expected_confirmation"]

            def fail(_):
                raise OSError("unplugged")

            with self.assertRaises(HarnessError):
                run_once(state, obs, obs, confirmation, False, fail)
            self.assertTrue(state.exists())
            with self.assertRaises(HarnessError):
                run_once(state, obs, obs, confirmation, False, lambda _: None)

    def test_physical_install_authority_never_reaches_disposable_backend(self):
        with tempfile.TemporaryDirectory() as directory:
            obs = observation()
            confirmation = evaluate(obs)["expected_confirmation"]
            calls = []
            with self.assertRaisesRegex(HarnessError, "rejects physical-install authority"):
                run_once(
                    Path(directory) / "claim",
                    obs,
                    obs,
                    confirmation,
                    False,
                    calls.append,
                )
            self.assertEqual(calls, [])


if __name__ == "__main__":
    unittest.main()
