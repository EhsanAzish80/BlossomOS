import tempfile, unittest
from pathlib import Path
from tests.test_phase11_install_guard import observation
from scripts.distribution.physical_install_guard import evaluate
from scripts.distribution.physical_write_harness import HarnessError, run_once

class HarnessTests(unittest.TestCase):
    def test_cancel_mutation_replay_and_failure_are_fail_closed(self):
        with tempfile.TemporaryDirectory() as d:
            state=Path(d)/"claim"; obs=observation(); confirmation=evaluate(obs)["expected_confirmation"]; calls=[]
            self.assertEqual(run_once(state,obs,obs,confirmation,True,calls.append),"cancelled_no_write"); self.assertFalse(state.exists())
            changed=observation(); changed["devices"][0]["size_bytes"]+=512
            with self.assertRaises(HarnessError): run_once(state,obs,changed,confirmation,False,calls.append)
            self.assertFalse(state.exists()); self.assertEqual(calls,[])
            self.assertEqual(run_once(state,obs,obs,confirmation,False,calls.append),"disposable_test_completed")
            with self.assertRaises(HarnessError): run_once(state,obs,obs,confirmation,False,calls.append)
            self.assertEqual(len(calls),1)
    def test_failure_consumes_attempt_and_cannot_retry(self):
        with tempfile.TemporaryDirectory() as d:
            state=Path(d)/"claim"; obs=observation(); confirmation=evaluate(obs)["expected_confirmation"]
            def fail(_): raise OSError("unplugged")
            with self.assertRaises(HarnessError): run_once(state,obs,obs,confirmation,False,fail)
            self.assertTrue(state.exists())
            with self.assertRaises(HarnessError): run_once(state,obs,obs,confirmation,False,lambda _: None)

if __name__ == "__main__": unittest.main()
