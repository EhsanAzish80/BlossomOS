import subprocess
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/package_llama_cpp_runtime.py"
REGISTRY = (
    ROOT
    / "system/model-runtime/registry/llama-cpp-cpu-x86_64.profile.json"
)


class ModelRuntimePackageTests(unittest.TestCase):
    def run_script(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *arguments],
            cwd=ROOT,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def test_embedded_registry_matches_the_closed_lock(self) -> None:
        verification = self.run_script("--verify-lock")
        self.assertEqual(verification.returncode, 0, verification.stdout)
        emitted = self.run_script("--emit-registry")
        self.assertEqual(emitted.returncode, 0, emitted.stdout)
        self.assertEqual(emitted.stdout.rstrip("\n").encode(), REGISTRY.read_bytes().rstrip(b"\n"))

    def test_builder_requires_the_complete_closed_input_set(self) -> None:
        incomplete = self.run_script("--runtime-archive", "/tmp/not-an-archive")
        self.assertNotEqual(incomplete.returncode, 0)
        self.assertIn("all five closed package paths are required", incomplete.stdout)


if __name__ == "__main__":
    unittest.main()
