import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class Arm64AgentContractTests(unittest.TestCase):
    def test_adr_0030_closes_the_cross_architecture_gate(self):
        adr = (ROOT / "docs/decisions/0030-cross-architecture-agent-qualification.md").read_text()
        for required in (
            "One immutable model record",
            "Separate `x86_64` and `aarch64` runtime records",
            "`cfg(target_arch)`",
            "grammar digest",
            "`files.write:create`",
            "twenty consecutive warm runs",
            "audit must show zero executor",
            "unsupported and untested",
            "`pacman -U`",
            "qualification client and does not prove independent human intent",
        ):
            self.assertIn(required, adr)

    def test_decision_index_lists_adr_0030(self):
        index = (ROOT / "docs/decisions/README.md").read_text()
        self.assertIn("ADR-0030", index)
        self.assertIn("architecture-specific llama.cpp runtimes", index)


if __name__ == "__main__":
    unittest.main()
