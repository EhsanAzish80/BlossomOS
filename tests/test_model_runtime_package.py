import subprocess
import sys
import unittest
import importlib.util
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/package_llama_cpp_runtime.py"
REGISTRY_ROOT = ROOT / "system/model-runtime/registry"
RUNTIME_PKGBUILD = ROOT / "distribution/packages/blossom-model-runtime/PKGBUILD"


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

    def test_embedded_registries_match_split_records(self) -> None:
        for architecture in ("x86_64", "aarch64"):
            verification = self.run_script("--architecture", architecture, "--verify-lock")
            self.assertEqual(verification.returncode, 0, verification.stdout)
            emitted = self.run_script("--architecture", architecture, "--emit-registry")
            self.assertEqual(emitted.returncode, 0, emitted.stdout)
            registry = REGISTRY_ROOT / f"llama-cpp-cpu-{architecture}.profile.json"
            self.assertEqual(emitted.stdout.rstrip("\n").encode(), registry.read_bytes().rstrip(b"\n"))

    def test_receipts_bind_shared_model_and_architecture_runtime(self) -> None:
        specification = importlib.util.spec_from_file_location("llama_packager", SCRIPT)
        module = importlib.util.module_from_spec(specification)
        assert specification.loader is not None
        specification.loader.exec_module(module)
        receipts = {
            architecture: module.receipt(module.load_lock(architecture), "a" * 64)
            for architecture in ("x86_64", "aarch64")
        }
        self.assertEqual(
            receipts["x86_64"]["model_record_sha256"],
            receipts["aarch64"]["model_record_sha256"],
        )
        self.assertNotEqual(
            receipts["x86_64"]["runtime_record_sha256"],
            receipts["aarch64"]["runtime_record_sha256"],
        )
        self.assertEqual(receipts["x86_64"]["architecture"], "x86_64")
        self.assertEqual(receipts["aarch64"]["architecture"], "aarch64")

    def test_builder_requires_the_complete_closed_input_set(self) -> None:
        incomplete = self.run_script("--architecture", "aarch64", "--runtime-archive", "/tmp/not-an-archive")
        self.assertNotEqual(incomplete.returncode, 0)
        self.assertIn("all five closed package paths are required", incomplete.stdout)

    def test_runtime_package_binds_but_does_not_own_core_gateway(self) -> None:
        source = SCRIPT.read_text(encoding="utf-8")
        pkgbuild = RUNTIME_PKGBUILD.read_text(encoding="utf-8")
        self.assertNotIn(
            'copy_exact(gateway, output / "usr/lib/blossom-os/blossom-model-gateway"',
            source,
        )
        self.assertIn("'blossom-core'", pkgbuild)
        self.assertIn("runtime package must not own the blossom-core gateway", source)


if __name__ == "__main__":
    unittest.main()
