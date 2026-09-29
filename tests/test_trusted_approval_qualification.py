import py_compile
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class TrustedApprovalQualificationTests(unittest.TestCase):
    def test_probe_is_valid_python_and_checks_public_audit(self):
        probe = ROOT / "scripts/probe_trusted_approval.py"
        with tempfile.TemporaryDirectory() as directory:
            py_compile.compile(probe, cfile=str(Path(directory) / "probe.pyc"), doraise=True)
        source = probe.read_text(encoding="utf-8")
        for required in (
            '"StartAgentTurn1"',
            '"SubmitDecision1"',
            '"ReadActivity1"',
            '"authentication_rejected"',
            'record.get("kind") == "effect"',
            'record.get("kind") == "execution"',
            'record.get("category") == "started"',
            '"authentication_challenge_expired"',
            '"authentication_challenge_unavailable"',
            '"challenge_observed": True',
            '"AccessDenied"',
        ):
            self.assertIn(required, source)

    def test_trusted_approval_image_excludes_the_bypass_package_and_rule(self):
        builder = (
            ROOT / "scripts/distribution/build_arm64_development_image.sh"
        ).read_text(encoding="utf-8")
        package = (
            ROOT / "distribution/packages/blossom-trusted-approval-qualification/PKGBUILD"
        ).read_text(encoding="utf-8")
        orchestrator = (
            ROOT
            / "distribution/packages/blossom-trusted-approval-qualification"
            / "blossom-trusted-approval-qualification-probe"
        ).read_text(encoding="utf-8")
        self.assertIn("trusted-approval", builder)
        self.assertIn("trusted-approval image contains blossom-qualification", builder)
        self.assertIn("trusted-approval image contains the qualification polkit bypass", builder)
        self.assertNotIn("49-blossom-model-effect-qualification.rules", package)
        self.assertIn("! pacman -Q blossom-qualification", orchestrator)
        self.assertIn("qualification-rule-present", orchestrator)
        self.assertIn("qualification-driver-present", orchestrator)

    def test_probe_covers_self_approval_and_missing_agent(self):
        orchestrator = (
            ROOT
            / "distribution/packages/blossom-trusted-approval-qualification"
            / "blossom-trusted-approval-qualification-probe"
        ).read_text(encoding="utf-8")
        for required in (
            "--case self_approval",
            "systemctl --user",
            "stop hyprpolkitagent.service",
            "--case missing_agent",
            "loginctl show-session",
            "invalid-environment-inactive-session",
            "session_active=yes",
            "effects=0 executor_starts=0 bypass=absent",
        ):
            self.assertIn(required, orchestrator)

    def test_vm_uses_the_proven_arm64_qualification_channel(self):
        service = (
            ROOT
            / "distribution/packages/blossom-trusted-approval-qualification"
            / "blossom-trusted-approval-qualification.service"
        ).read_text(encoding="utf-8")
        orchestrator = (
            ROOT
            / "distribution/packages/blossom-trusted-approval-qualification"
            / "blossom-trusted-approval-qualification-probe"
        ).read_text(encoding="utf-8")
        launcher = (
            ROOT / "scripts/distribution/qualify_arm64_trusted_approval_macos.sh"
        ).read_text(encoding="utf-8")
        channel = "org.blossomos.qualification"
        self.assertIn(f"/dev/virtio-ports/{channel}", service)
        self.assertIn(f"/dev/virtio-ports/{channel}", orchestrator)
        self.assertIn(f"name={channel}", launcher)

    def test_probe_does_not_block_graphical_target_or_pull_up_a_session(self):
        service = (
            ROOT
            / "distribution/packages/blossom-trusted-approval-qualification"
            / "blossom-trusted-approval-qualification.service"
        ).read_text(encoding="utf-8")
        orchestrator = (
            ROOT
            / "distribution/packages/blossom-trusted-approval-qualification"
            / "blossom-trusted-approval-qualification-probe"
        ).read_text(encoding="utf-8")
        self.assertIn("Type=simple", service)
        self.assertNotIn("After=graphical.target", service)
        self.assertIn("After=greetd.service NetworkManager-wait-online.service", service)
        self.assertNotIn("Wants=NetworkManager-wait-online.service blossom-shell-broker@1000.service", service)
        self.assertIn("BLOSSOM_ARM64_TRUSTED_APPROVAL_ACTIVATED", orchestrator)
        self.assertIn("systemd-cat", orchestrator)
        self.assertIn(">/dev/console", orchestrator)

    def test_probe_image_has_session_autologin_but_physical_image_does_not(self):
        arm_builder = (
            ROOT / "scripts/distribution/build_arm64_development_image.sh"
        ).read_text(encoding="utf-8")
        physical_builder = (
            ROOT / "scripts/distribution/build_physical_candidate.sh"
        ).read_text(encoding="utf-8")
        physical_greetd = (
            ROOT / "distribution/physical-rootfs/etc/greetd/config.toml"
        ).read_text(encoding="utf-8")
        self.assertIn("[initial_session]", arm_builder)
        self.assertIn('user = "blossom"', arm_builder)
        self.assertNotIn("[initial_session]", physical_builder)
        self.assertNotIn("[initial_session]", physical_greetd)

    def test_test_only_package_is_not_referenced_by_physical_builds(self):
        package_name = "blossom-trusted-approval-qualification"
        for relative in (
            "scripts/distribution/build_physical_candidate.sh",
            "scripts/distribution/verify_physical_candidate_image.sh",
            "distribution/manifest.json",
            "distribution/archiso/packages.x86_64",
        ):
            self.assertNotIn(
                package_name,
                (ROOT / relative).read_text(encoding="utf-8"),
                relative,
            )


if __name__ == "__main__":
    unittest.main()
