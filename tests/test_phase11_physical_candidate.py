import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.distribution import physical_candidate_install as candidate


def host():
    return {
        "architecture": "x86_64",
        "firmware": "uefi",
        "product_name": "MacBookPro11,1",
        "memory_mib": 8192,
        "drm_present": True,
        "internal_disk_present": True,
    }


def devices():
    return (
        "/dev/sdb",
        [
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
                "model": "BLOSSOM RECOVERY",
                "size_bytes": 64000000000,
                "transport": "usb",
                "removable": True,
                "mounted": True,
            },
        ],
    )


class PhysicalCandidateTests(unittest.TestCase):
    @patch("scripts.distribution.physical_candidate_install.os.geteuid", return_value=0)
    def test_exact_prompts_reobserve_and_delegate_once(self, _geteuid):
        with tempfile.TemporaryDirectory() as directory:
            answers = iter(["AC-READY", "RECOVERY-READY"])
            calls = []

            def read(prompt):
                if prompt.startswith("Type exactly:"):
                    return prompt.split("Type exactly: ", 1)[1].split("\n", 1)[0]
                return next(answers)

            def execute(*args):
                calls.append(args)
                return {"schema": 1, "result": "physical_install_completed"}

            with patch.object(candidate, "STATE_ROOT", Path(directory)):
                result = candidate.run_interactive(
                    read=read,
                    host_observer=host,
                    device_observer=devices,
                    executor=execute,
                    challenge_factory=lambda size: bytes.fromhex("01" * size),
                )
            self.assertEqual(result["result"], "physical_install_completed")
            self.assertEqual(len(calls), 1)
            self.assertEqual(calls[0][3], "ERASE /dev/sda d3e8f7fa8b5ae4af")

    @patch("scripts.distribution.physical_candidate_install.os.geteuid", return_value=0)
    def test_prerequisite_cancellation_never_observes_devices(self, _geteuid):
        observed = []
        with self.assertRaises(candidate.CandidateError):
            candidate.run_interactive(
                read=lambda _prompt: "no",
                host_observer=host,
                device_observer=lambda: observed.append(True),
            )
        self.assertEqual(observed, [])

    @patch("scripts.distribution.physical_candidate_install.os.geteuid", return_value=1000)
    def test_requires_root(self, _geteuid):
        with self.assertRaises(candidate.CandidateError):
            candidate.run_interactive()


if __name__ == "__main__":
    unittest.main()
