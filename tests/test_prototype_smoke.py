"""Minimal characterization tests for the preserved rule-based prototype."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "ai-core" / "blossom-ai.py"


def load_prototype_module():
    spec = importlib.util.spec_from_file_location("blossom_prototype", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("Unable to load the preserved Blossom prototype")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PrototypeSmokeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_prototype_module()

    def setUp(self) -> None:
        self.assistant = self.module.BlossomAI(model_path="/nonexistent/test-model")

    def test_default_response_is_rule_based_help(self) -> None:
        response = self.assistant.chat("hello")
        self.assertIn("I'm here to help", response)
        self.assertEqual(len(self.assistant.history), 2)

    def test_disk_space_command_is_only_suggested(self) -> None:
        response = self.assistant.chat("how do I check disk space?")
        self.assertIn("df -h", response)
        self.assertIn("Suggested command", response)

    def test_model_loader_remains_a_stub(self) -> None:
        self.assertIsNone(self.assistant.load_model())


if __name__ == "__main__":
    unittest.main()
