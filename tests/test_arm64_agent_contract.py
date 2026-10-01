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
            "real two-turn result",
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
            "typed untrusted-data message",
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
            ROOT / "core/blossom-core/src/workspace_plan.rs",
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
            (
                ROOT / "core/blossom-core/src/workspace_plan.rs",
                "ValidatedWorkspacePlanEffect",
            ),
            (
                ROOT / "core/blossom-core/src/workspace_plan.rs",
                "ValidatedWorkspacePlanProposal",
            ),
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
        self.assertIn("WorkspacePlanProposalResolver::resolve", fuzz)

    def test_qualification_driver_uses_one_public_shell_path(self):
        driver = (ROOT / "scripts/qualify_agent_pipeline.py").read_text()
        core_package = (ROOT / "distribution/packages/blossom-core/PKGBUILD").read_text()
        qualification_package = (
            ROOT / "distribution/packages/blossom-qualification/PKGBUILD"
        ).read_text()
        builder = (ROOT / "scripts/distribution/build_physical_candidate.sh").read_text()
        self.assertNotIn('add_argument("--mode"', driver)
        self.assertIn('"transport": "production_gateway"', driver)
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
        self.assertIn("shell activity contains a sequence gap", driver)
        self.assertIn("--features production-private-inference", core_package)
        self.assertNotIn("qualify_agent_pipeline.py", core_package)
        self.assertIn("depends=('blossom-core'", qualification_package)
        self.assertIn("qualify_agent_pipeline.py", qualification_package)
        self.assertIn("qualification_fixture_provider.py", qualification_package)
        self.assertIn("fixtures/indirect-invalid.txt", qualification_package)
        self.assertIn("fixtures/indirect-valid.txt", qualification_package)
        qualification_build = 'makepkg --nodeps --noconfirm --dir "$build/source/distribution/packages/blossom-qualification"'
        qualification_copy = 'distribution/packages/blossom-qualification/blossom-qualification-*.pkg.tar.zst'
        self.assertIn(qualification_build, builder)
        self.assertIn(qualification_copy, builder)
        self.assertGreaterEqual(builder.count('if [[ "$mode" == vm-qualification ]]'), 3)

    def test_gateway_accepts_the_closed_profiles_available_on_this_architecture(self):
        gateway = (ROOT / "system/model-gateway/src/lib.rs").read_text()
        self.assertIn(".map(production_provider_profile)", gateway)
        self.assertIn(".flatten()", gateway)
        self.assertIn("if specifications.is_empty()", gateway)
        self.assertNotIn(
            "production_provider_profile(GatewayProfile::OllamaCpuV1)\n"
            "                .map_err",
            gateway,
        )

    def test_every_exported_shell_method_requires_a_production_handler(self):
        source = (ROOT / "system/shell-service/src/session_bus.rs").read_text()
        trait = source[source.index("pub trait ShellRequestHandler"):source.index("impl<E: Executor")]
        for handler in (
            "start",
            "start_agent",
            "decide",
            "approval_authentication",
            "reject_approval_authentication",
            "record_approval_authentication",
            "cancel",
            "activity",
            "battery",
            "network",
            "disconnect",
        ):
            declaration = trait[trait.index(f"fn {handler}("):]
            declaration = declaration[:declaration.index(";") + 1]
            self.assertNotIn("{", declaration, handler)
        self.assertNotIn("fn start_agent", trait.split(";")[-1])
        for method in ("QueryCommandBar1", "QueryCommandSuggestions1", "ActivateCommandRow1"):
            self.assertIn(f'#[zbus(name = "{method}")]', source)
        self.assertIn("decode_command_query(&input)", source)
        self.assertIn("decode_command_activation(&input)", source)
        self.assertIn("verified_application_launch(&current)", source)
        self.assertIn('"StartTransientUnit"', source)
        self.assertIn('(\"Type\", Value::new(\"exec\"))', source)
        self.assertIn('(\"ExitType\", Value::new(\"cgroup\"))', source)
        self.assertIn(
            '(\"CollectMode\", Value::new(\"inactive-or-failed\"))', source
        )
        self.assertIn('(\"Slice\", Value::new(\"app-graphical.slice\"))', source)
        self.assertIn('format!("app-blossom-{escaped_id}@{nonce}.service")', source)
        self.assertNotIn('(\"Restart\",', source)
        self.assertNotIn("std::process::Command", source)

    def test_desktop_accounts_are_deliberately_gateway_eligible(self):
        sysusers = (
            ROOT / "system/model-runtime/packaging/blossom-model-runtime.sysusers"
        ).read_text()
        provision = (
            ROOT / "scripts/distribution/provision_installed_identity.py"
        ).read_text()
        adr = (
            ROOT / "docs/decisions/0017-private-gateway-admission-and-cancellation.md"
        ).read_text()
        self.assertIn("m blossom blossom-ai", sysusers)
        self.assertIn('groups = "audio,video,blossom-ai"', provision)
        self.assertIn("every process running under that same desktop UID", adr)
        self.assertIn("unexpected-owner", adr)

    def test_desktop_users_never_receive_raw_input_group(self):
        provisioning_sources = (
            ROOT / "scripts/distribution/provision_installed_identity.py",
            ROOT / "scripts/distribution/build_arm64_development_image.sh",
            ROOT / "distribution/archiso/airootfs/usr/local/libexec/blossom-provision-live-user",
            ROOT / "scripts/alpine-setup.sh",
        )
        for path in provisioning_sources:
            source = path.read_text(encoding="utf-8")
            self.assertNotIn("audio,input", source, path)
            self.assertNotIn("addgroup blossom input", source, path)

    def test_adr_0034_closes_the_first_trusted_approval_boundary(self):
        adr = (
            ROOT / "docs/decisions/0034-trusted-approval-authentication.md"
        ).read_text(encoding="utf-8")
        for required in (
            "`auth_self`",
            "approval-binding digest",
            "Authentication is never cached",
            "sole source of preview data",
            "does not create trusted pixels",
            "lookalike overlay",
            "agent-substitution",
            "bypassed password authentication",
            "must never fall back",
        ):
            self.assertIn(required, adr)
        index = (ROOT / "docs/decisions/README.md").read_text(encoding="utf-8")
        self.assertIn("ADR-0034", index)

    def test_model_effect_policy_never_caches_authentication(self):
        policy = (
            ROOT / "system/shell/packaging/org.blossomos.shell.policy"
        ).read_text(encoding="utf-8")
        self.assertIn("org.blossomos.shell.approve-model-effect", policy)
        self.assertIn("<allow_active>auth_self</allow_active>", policy)
        self.assertIn(
            '<annotate key="org.freedesktop.policykit.owner">unix-user:blossom</annotate>',
            policy,
        )
        self.assertNotIn("auth_self_keep", policy)
        core_package = (
            ROOT / "distribution/packages/blossom-core/PKGBUILD"
        ).read_text(encoding="utf-8")
        self.assertIn("org.blossomos.shell.policy", core_package)

    def test_qualification_polkit_bypass_cannot_ship_in_production(self):
        qualification_package = (
            ROOT / "distribution/packages/blossom-qualification/PKGBUILD"
        ).read_text(encoding="utf-8")
        core_package = (
            ROOT / "distribution/packages/blossom-core/PKGBUILD"
        ).read_text(encoding="utf-8")
        physical_verifier = (
            ROOT / "scripts/distribution/verify_physical_candidate_image.sh"
        ).read_text(encoding="utf-8")
        physical_builder = (
            ROOT / "scripts/distribution/build_physical_candidate.sh"
        ).read_text(encoding="utf-8")
        rule_name = "49-blossom-model-effect-qualification.rules"
        qualification_rule = (
            ROOT / "distribution/packages/blossom-qualification" / rule_name
        ).read_text(encoding="utf-8")
        self.assertIn(rule_name, qualification_package)
        self.assertNotIn(rule_name, core_package)
        self.assertNotIn(rule_name, physical_verifier)
        self.assertIn(
            'action.id == "org.blossomos.shell.approve-model-effect"',
            qualification_rule,
        )
        self.assertIn('subject.user == "blossom"', qualification_rule)
        self.assertNotIn("subject.active", qualification_rule)
        self.assertNotIn("subject.local", qualification_rule)
        self.assertIn("physical root must never contain the qualification polkit bypass", physical_builder)
        self.assertIn("VM qualification root is missing its closed polkit bypass", physical_builder)
        driver = (ROOT / "scripts/qualify_agent_pipeline.py").read_text(encoding="utf-8")
        self.assertIn('"qualification_polkit_bypass"', driver)


if __name__ == "__main__":
    unittest.main()
