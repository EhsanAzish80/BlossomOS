import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/check_shell_broker_runtime.py"
SPEC = importlib.util.spec_from_file_location("check_shell_broker_runtime", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class ShellBrokerRuntimeTests(unittest.TestCase):
    def write_proc(self, uid_map: str, uid: int = 1000) -> tuple[Path, int]:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        pid = 42
        process = root / str(pid)
        process.mkdir()
        (process / "status").write_text(f"Name:\tbroker\nUid:\t{uid}\t{uid}\t{uid}\t{uid}\n")
        (process / "uid_map").write_text(uid_map)
        return root, pid

    def test_accepts_host_identity_map(self):
        root, pid = self.write_proc("         0          0 4294967295\n")
        MODULE.verify_uid_map(root, pid, 1000)

    def test_rejects_single_uid_namespace(self):
        root, pid = self.write_proc("         0       1000          1\n")
        with self.assertRaises(MODULE.BrokerRuntimeError):
            MODULE.verify_uid_map(root, pid, 1000)

    def test_rejects_wrong_runtime_uid(self):
        root, pid = self.write_proc("         0          0 4294967295\n", uid=0)
        with self.assertRaises(MODULE.BrokerRuntimeError):
            MODULE.verify_uid_map(root, pid, 1000)


if __name__ == "__main__":
    unittest.main()
