import subprocess
import unittest
from pathlib import Path


class MacOSVMLauncherTests(unittest.TestCase):
    def test_launcher_is_bounded_and_non_destructive(self):
        repository = Path(__file__).resolve().parents[1]
        launcher = repository / "scripts/distribution/run_candidate_vm_macos.sh"
        text = launcher.read_text()

        subprocess.run(["bash", "-n", str(launcher)], check=True)
        self.assertIn("qemu-system-x86_64", text)
        self.assertIn("q35,accel=tcg", text)
        self.assertIn("media=cdrom,readonly=on", text)
        self.assertIn("if=virtio,format=qcow2", text)
        self.assertIn("edk2-x86_64-code.fd", text)
        self.assertNotIn("/dev/disk", text)
        self.assertNotIn("sudo", text)
        self.assertNotIn("rm ", text)


if __name__ == "__main__":
    unittest.main()
