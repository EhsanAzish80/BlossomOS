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

    def test_adr_0031_closes_model_proposed_write_arguments(self):
        adr = (ROOT / "docs/decisions/0031-model-proposed-workspace-create.md").read_text()
        for required in (
            "`[a-z0-9][a-z0-9._-]{0,63}`",
            "at most 4096 bytes",
            "cannot construct `WorkspaceCreateSelection`",
            "mode `0600`",
            "returns `ask`",
            "origin `model_proposed`",
            "zero executor starts",
            "explicitly labeled untrusted data",
        ):
            self.assertIn(required, adr)

    def test_adr_0032_owns_prepared_requests_through_approval(self):
        adr = (ROOT / "docs/decisions/0032-prepared-request-ownership.md").read_text()
        for required in (
            "Only `ToolRequestWire` deserializes",
            "It is not `Clone`, `Serialize` or `Deserialize`",
            "preview digest",
            "Neither client nor engine resubmits",
            "per peer and globally",
            "Admission reserves capacity before",
            "Capacity exhaustion never evicts",
            "approval capacity exhausted",
            "same descriptor",
            "`/proc/self/fd`",
            "fuzz harness",
        ):
            self.assertIn(required, adr)


if __name__ == "__main__":
    unittest.main()
