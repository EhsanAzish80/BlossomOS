import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

from scripts.distribution import physical_candidate_install as candidate


def host():
    return {
        "architecture": "x86_64",
        "firmware": "uefi",
        "product_name": "MacBookPro11,1",
        "memory_mib": 8192,
        "drm_present": True,
        "internal_disk_present": True,
    }


def devices():
    return (
        "/dev/sdb",
        [
            {
                "path": "/dev/sda",
                "model": "APPLE SSD SM0128F",
                "size_bytes": 121332826112,
                "transport": "sata",
                "removable": False,
                "mounted": False,
            },
            {
                "path": "/dev/sdb",
                "model": "BLOSSOM RECOVERY",
                "size_bytes": 64000000000,
                "transport": "usb",
                "removable": True,
                "mounted": True,
            },
        ],
    )


class PhysicalCandidateTests(unittest.TestCase):
    @patch("scripts.distribution.physical_candidate_install.os.geteuid", return_value=0)
    def test_exact_prompts_reobserve_and_delegate_once(self, _geteuid):
        with tempfile.TemporaryDirectory() as directory:
            answers = iter(["AC-READY", "RECOVERY-READY"])
            calls = []

            def read(prompt):
                if prompt.startswith("Type exactly:"):
                    return prompt.split("Type exactly: ", 1)[1].split("\n", 1)[0]
                return next(answers)

            def execute(*args):
                calls.append(args)
                return {"schema": 1, "result": "physical_install_completed"}

            with patch.object(candidate, "STATE_ROOT", Path(directory)):
                output = StringIO()
                with redirect_stdout(output):
                    result = candidate.run_interactive(
                        read=read,
                        host_observer=host,
                        device_observer=devices,
                        executor=execute,
                        challenge_factory=lambda size: bytes.fromhex("01" * size),
                    )
            self.assertEqual(result["result"], "physical_install_completed")
            self.assertEqual(len(calls), 1)
            self.assertEqual(calls[0][3], "ERASE /dev/sda d3e8f7fa8b5ae4af")
            self.assertIn("internal system disk is selected", output.getvalue())
            self.assertIn("external disks are never installation targets", output.getvalue())

    def test_desktop_runtime_packages_and_launcher_are_present(self):
        repository = Path(__file__).resolve().parents[1]
        builder = (repository / "scripts/distribution/build_physical_candidate.sh").read_text()
        packages = (repository / "distribution/archiso/packages.x86_64").read_text()
        profile = (repository / "distribution/physical-rootfs/home/blossom/.bash_profile").read_text()
        live_profile = (repository / "distribution/archiso/airootfs/home/blossom/.bash_profile").read_text()
        autologin = (repository / "distribution/archiso/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf").read_text()
        root_console = (repository / "distribution/archiso/airootfs/etc/systemd/system/getty@tty2.service.d/autologin.conf").read_text()
        live_hyprland = (repository / "distribution/archiso/airootfs/home/blossom/.config/hypr/hyprland.conf").read_text()
        installed_hyprland = (repository / "distribution/physical-rootfs/usr/share/blossom-os/physical-home/hyprland.conf").read_text()
        shell_package = (repository / "distribution/packages/blossom-shell/PKGBUILD").read_text()
        shell_unit = (repository / "system/shell/packaging/blossom-shell-ui.service").read_text()
        recovery_unit = (repository / "system/shell/packaging/blossom-shell-recovery.service").read_text()
        recovery_command = (repository / "system/shell/packaging/blossom-shell-recovery").read_text()
        shell_qml = (repository / "system/shell/qml/shell.qml").read_text()
        broker_header = (repository / "system/shell/client-plugin/blossombroker.h").read_text()
        broker_source = (repository / "system/shell/client-plugin/blossombroker.cpp").read_text()
        installer_rule = (repository / "distribution/archiso/airootfs/etc/polkit-1/rules.d/49-blossom-live-installer.rules").read_text()
        self.assertIn("noto-fonts", builder)
        self.assertIn("noto-fonts-emoji", builder)
        for package in ("adwaita-cursors", "foot", "noto-fonts", "noto-fonts-emoji"):
            self.assertIn(f"\n{package}\n", f"\n{packages}")
        self.assertIn("exec start-hyprland", profile)
        self.assertIn("exec start-hyprland", live_profile)
        self.assertNotIn("exec Hyprland", profile)
        self.assertIn(
            'cp -a "$repo/distribution/archiso/airootfs/." "$profile/airootfs/"',
            builder,
        )
        self.assertIn("--autologin blossom", autologin)
        self.assertIn("useradd --create-home", autologin)
        self.assertIn("chown -R blossom:blossom /home/blossom", autologin)
        self.assertNotIn("--autologin root", autologin)
        self.assertIn("--autologin root", root_console)
        for config in (live_hyprland, installed_hyprland):
            self.assertIn("XCURSOR_THEME,Adwaita", config)
            self.assertIn("background_color = rgb(0b111b)", config)
            self.assertIn("systemctl --user start blossom-shell-ui.service", config)
        self.assertIn('client-build/blossom-shell-ui', shell_package)
        self.assertIn('/usr/lib/qt6/qml/Blossom/Shell', shell_package)
        self.assertIn('blossom-shell-ui.service', shell_package)
        self.assertIn('blossom-shell-recovery.service', shell_package)
        self.assertIn('/usr/local/bin/blossom-shell-recovery', shell_package)
        self.assertIn("ExecStart=/usr/lib/blossom-os/blossom-shell-ui", shell_unit)
        self.assertIn("Restart=on-failure", shell_unit)
        self.assertIn("OnFailure=blossom-shell-recovery.service", shell_unit)
        self.assertIn("blossom-shell-recovery", recovery_unit)
        self.assertIn("restart-shell", recovery_command)
        self.assertIn("Welcome to Blossom OS", shell_qml)
        self.assertIn("Install Blossom OS", shell_qml)
        self.assertIn("External disks are excluded", shell_qml)
        self.assertIn("liveEnvironment", broker_header)
        self.assertIn('/usr/local/bin/blossom-physical-install', broker_source)
        self.assertIn('subject.user == "blossom"', installer_rule)
        self.assertIn('action.lookup("program") == "/usr/local/bin/blossom-physical-install"', installer_rule)
        self.assertIn("Blossom OS Recovery Console", builder)
        self.assertIn("vt.global_cursor_default=0", builder)
        self.assertIn("blossom.recovery=1", live_profile)

    def test_macos_builder_uses_an_isolated_x86_64_vm(self):
        repository = Path(__file__).resolve().parents[1]
        builder = (repository / "scripts/distribution/build_physical_candidate_macos.sh").read_text()
        self.assertIn("colima start", builder)
        self.assertIn("--arch x86_64", builder)
        self.assertIn("--platform linux/amd64", builder)
        self.assertIn("--dns 1.1.1.1", builder)
        self.assertIn('--env "BLOSSOM_CANDIDATE_MODE=$mode"', builder)
        self.assertIn("physical|vm-qualification", builder)
        self.assertIn("archlinux@sha256:", builder)
        self.assertIn("shasum -a 256 -c SHA256SUMS", builder)

    @patch("scripts.distribution.physical_candidate_install.os.geteuid", return_value=0)
    def test_prerequisite_cancellation_never_observes_devices(self, _geteuid):
        observed = []
        with self.assertRaises(candidate.CandidateError):
            candidate.run_interactive(
                read=lambda _prompt: "no",
                host_observer=host,
                device_observer=lambda: observed.append(True),
            )
        self.assertEqual(observed, [])

    @patch("scripts.distribution.physical_candidate_install.os.geteuid", return_value=1000)
    def test_requires_root(self, _geteuid):
        with self.assertRaises(candidate.CandidateError):
            candidate.run_interactive()


if __name__ == "__main__":
    unittest.main()
