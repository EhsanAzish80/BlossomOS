from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class Phase10ReleaseTests(unittest.TestCase):
    def test_sbom_is_spdx_and_reproducible(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first.json"
            second = Path(temporary) / "second.json"
            command = [
                "python3",
                "scripts/release/generate_sbom.py",
                "--revision",
                "test-revision",
                "--source-date-epoch",
                "1700000000",
            ]
            subprocess.run(command + ["--output", str(first)], cwd=ROOT, check=True)
            subprocess.run(command + ["--output", str(second)], cwd=ROOT, check=True)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            document = json.loads(first.read_text())
            self.assertEqual(document["spdxVersion"], "SPDX-2.3")
            self.assertGreater(len(document["packages"]), 1)
            self.assertTrue(document["relationships"])

    def test_release_manifest_is_canonical(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifact = root / "blossom-test"
            artifact.write_bytes(b"reviewed artifact\n")
            output = root / "output"
            subprocess.run(
                [
                    "python3",
                    "scripts/release/generate_manifest.py",
                    "--revision",
                    "abc123",
                    "--source-date-epoch",
                    "1700000000",
                    "--artifact",
                    str(artifact),
                    "--root-directory",
                    str(root),
                    "--output-directory",
                    str(output),
                ],
                cwd=ROOT,
                check=True,
            )
            manifest = json.loads((output / "release-manifest.json").read_text())
            self.assertEqual(manifest["revision"], "abc123")
            self.assertEqual(
                manifest["artifacts"][0]["sha256"],
                hashlib.sha256(artifact.read_bytes()).hexdigest(),
            )
            self.assertEqual(
                manifest["artifacts"][0]["path"],
                "blossom-test",
            )
            self.assertEqual(
                (output / "SHA256SUMS").read_text(),
                f"{manifest['artifacts'][0]['sha256']}  ../blossom-test\n",
            )
            subprocess.run(
                ["shasum", "-a", "256", "-c", "SHA256SUMS"],
                cwd=output,
                check=True,
            )

    def test_candidate_workflow_creates_both_attestation_types(self) -> None:
        workflow = (ROOT / ".github/workflows/phase10-beta-candidate.yml").read_text()
        self.assertIn("name: Generate signed build provenance", workflow)
        self.assertIn("name: Generate signed SBOM attestation", workflow)
        self.assertEqual(workflow.count("uses: actions/attest@"), 2)
        self.assertEqual(workflow.count("sbom-path:"), 1)


if __name__ == "__main__":
    unittest.main()
