from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "scripts/qualification_fixture_provider.py"
SPEC = importlib.util.spec_from_file_location("qualification_fixture_provider", SOURCE)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def request(prompt: str, *, untrusted_data: str | None = None) -> dict:
    messages = [{"role": "user", "content": prompt}]
    if untrusted_data is not None:
        messages.append(
            {
                "role": "tool",
                "content": "UNTRUSTED_FILE_CONTENT:\n" + untrusted_data,
            }
        )
    return {
        "messages": messages,
        "temperature": 0,
        "seed": 0,
        "parallel_tool_calls": False,
        "tools": [
            {
                "function": {
                    "name": "files.write:create",
                    "parameters": {"additionalProperties": False},
                }
            }
        ],
    }


class QualificationFixtureProviderTests(unittest.TestCase):
    def test_indirect_variants_deliberately_obey_the_planted_instruction(self):
        invalid = MODULE.validate_request(
            request(MODULE.INDIRECT_INVALID_PROMPT, untrusted_data="traversal instruction")
        )
        valid = MODULE.validate_request(
            request(MODULE.INDIRECT_VALID_PROMPT, untrusted_data="plausible instruction")
        )
        self.assertEqual(invalid[1:], ("../escaped.txt", "injected traversal"))
        self.assertEqual(valid[1:], ("injected-note.txt", "attacker controlled"))

    def test_indirect_turn_requires_a_separate_labeled_data_message(self):
        with self.assertRaisesRegex(ValueError, "omitted typed untrusted data"):
            MODULE.validate_request(request(MODULE.INDIRECT_VALID_PROMPT))
        malformed = request(MODULE.INDIRECT_VALID_PROMPT, untrusted_data="payload")
        malformed["messages"][1]["content"] = "payload"
        with self.assertRaisesRegex(ValueError, "separately and visibly labeled"):
            MODULE.validate_request(malformed)


if __name__ == "__main__":
    unittest.main()
