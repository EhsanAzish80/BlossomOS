import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
CHECK = ROOT / "scripts" / "verify_no_stale_blossom_processes.sh"


def run_check(proc_root: Path) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["BLOSSOM_PROC_ROOT"] = str(proc_root)
    return subprocess.run(
        [str(CHECK)],
        check=False,
        capture_output=True,
        text=True,
        env=environment,
    )


def add_process(proc_root: Path, pid: int, executable: str) -> None:
    process = proc_root / str(pid)
    process.mkdir()
    (process / "exe").symlink_to(executable)


class StaleProcessCheckTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.proc_root = Path(self.temporary.name)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_clean_processes_pass(self) -> None:
        add_process(
            self.proc_root, 10, "/usr/lib/blossom-os/blossom-shell-service"
        )
        add_process(self.proc_root, 11, "/usr/bin/firefox")

        result = run_check(self.proc_root)

        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "no stale Blossom executables\n")

    def test_deleted_blossom_binary_fails(self) -> None:
        add_process(
            self.proc_root,
            12,
            "/usr/lib/blossom-os/blossom-desktop-launcher (deleted)",
        )

        result = run_check(self.proc_root)

        self.assertEqual(result.returncode, 1)
        self.assertIn("pid=12", result.stderr)
        self.assertIn("(deleted)", result.stderr)

    def test_deleted_lock_quickshell_fails(self) -> None:
        add_process(self.proc_root, 13, "/usr/bin/quickshell (deleted)")

        result = run_check(self.proc_root)

        self.assertEqual(result.returncode, 1)
        self.assertIn("pid=13", result.stderr)

    def test_unrelated_deleted_binary_is_ignored(self) -> None:
        add_process(self.proc_root, 14, "/usr/bin/example (deleted)")

        result = run_check(self.proc_root)

        self.assertEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
