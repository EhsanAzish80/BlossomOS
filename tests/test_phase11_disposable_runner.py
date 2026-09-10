import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.distribution.physical_install_guard import evaluate
from scripts.distribution.run_disposable_media_test import execute
from tests.test_phase11_write_harness import disposable_observation


class DisposableRunnerTests(unittest.TestCase):
    def test_cancel_has_no_claim_or_probe(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            initial = root / "initial.json"
            current = root / "current.json"
            value = disposable_observation()
            initial.write_text(json.dumps(value))
            current.write_text(json.dumps(value))
            confirmation = evaluate(value)["expected_confirmation"]
            with patch("scripts.distribution.run_disposable_media_test.probe") as backend:
                result = execute(root / "claim", initial, current, confirmation, True)
            self.assertEqual(result, {"schema": 1, "result": "cancelled_no_write", "probe": None})
            backend.assert_not_called()
            self.assertFalse((root / "claim").exists())

    def test_confirmed_unchanged_target_invokes_one_probe(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            initial = root / "initial.json"
            current = root / "current.json"
            value = disposable_observation()
            initial.write_text(json.dumps(value))
            current.write_text(json.dumps(value))
            confirmation = evaluate(value)["expected_confirmation"]
            probe_result = {"schema": 1, "result": "disposable_probe_completed_and_restored"}
            with patch(
                "scripts.distribution.run_disposable_media_test.probe",
                return_value=probe_result,
            ) as backend:
                result = execute(root / "claim", initial, current, confirmation, False)
            self.assertEqual(result["result"], "disposable_test_completed")
            self.assertEqual(result["probe"], probe_result)
            backend.assert_called_once()


if __name__ == "__main__":
    unittest.main()
