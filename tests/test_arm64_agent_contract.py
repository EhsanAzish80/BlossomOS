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

    def test_request_authority_modules_cannot_suppress_drop(self):
        authority_modules = (
            ROOT / "core/blossom-core/src/prepared_request.rs",
            ROOT / "core/blossom-core/src/approval.rs",
            ROOT / "core/blossom-core/src/request.rs",
            ROOT / "core/blossom-core/src/workspace_create.rs",
            ROOT / "core/blossom-core/src/shell_service.rs",
        )
        for path in authority_modules:
            source = path.read_text()
            self.assertNotIn("mem::forget", source, path)
            self.assertNotIn("ManuallyDrop", source, path)
            self.assertNotIn("Box::leak", source, path)

    def test_domain_request_types_never_deserialize_untrusted_input(self):
        declarations = (
            (ROOT / "core/blossom-core/src/file_read.rs", "FileIdentity"),
            (ROOT / "core/blossom-core/src/file_read.rs", "FileSelection"),
            (ROOT / "core/blossom-core/src/workspace_create.rs", "DirectoryIdentity"),
            (ROOT / "core/blossom-core/src/workspace_create.rs", "WorkspaceCreateSelection"),
            (ROOT / "core/blossom-core/src/service_status.rs", "ServiceSelection"),
            (ROOT / "core/blossom-core/src/request.rs", "ToolRequest"),
            (ROOT / "core/blossom-core/src/prepared_request.rs", "PreparedToolRequest"),
        )
        for path, type_name in declarations:
            source = path.read_text()
            declaration = source.index(f"{type_name} {{")
            derive_start = source.rfind("#[derive(", 0, declaration)
            derive_end = source.index(")]", derive_start)
            derive = source[derive_start:derive_end]
            self.assertNotIn("Deserialize", derive, f"{path}:{type_name}")

    def test_legacy_domain_parser_and_approval_store_are_gone(self):
        request = (ROOT / "core/blossom-core/src/request.rs").read_text()
        approval = (ROOT / "core/blossom-core/src/approval.rs").read_text()
        engine = (ROOT / "core/blossom-core/src/engine.rs").read_text()
        self.assertNotIn("fn parse_json", request)
        self.assertNotIn("struct ApprovalStore", approval)
        self.assertNotIn("with_file_content", engine)
        self.assertNotIn("with_workspace_create", engine)

    def test_fuzzer_exercises_wire_and_resolver_boundary(self):
        fuzz = (ROOT / "fuzz/fuzz_targets/untrusted_protocols.rs").read_text()
        self.assertIn("ToolRequestWire::parse_json", fuzz)
        self.assertIn("RequestResolver::resolve", fuzz)
        self.assertIn("capacity.reserve()", fuzz)

    def test_qualification_driver_uses_one_public_shell_path(self):
        driver = (ROOT / "scripts/qualify_agent_pipeline.py").read_text()
        package = (ROOT / "distribution/packages/blossom-core/PKGBUILD").read_text()
        self.assertNotIn('add_argument("--mode"', driver)
        self.assertIn('profile.get("logical_model") == "fixture-model:1"', driver)
        for method in ("StartAgentTurn1", "SubmitDecision1", "ReadActivity1"):
            self.assertIn(f'"{method}"', driver)
        for trace in (
            '"commit"',
            '"profile_digest"',
            '"receipt_digest"',
            '"architecture"',
            '"mode"',
        ):
            self.assertIn(trace, driver)
        self.assertIn("subprocess.TimeoutExpired", driver)
        self.assertIn("qualify-agent-pipeline", package)


if __name__ == "__main__":
    unittest.main()
