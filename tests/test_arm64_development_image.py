import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class Arm64DevelopmentImageTests(unittest.TestCase):
    def test_blossom_packages_are_native_on_both_release_architectures(self):
        for relative in (
            "distribution/packages/blossom-core/PKGBUILD",
            "distribution/packages/blossom-shell/PKGBUILD",
            "distribution/packages/blossom-model-runtime/PKGBUILD",
            "distribution/packages/blossom-qualification/PKGBUILD",
        ):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("arch=('x86_64' 'aarch64')", text)

    def test_arm64_package_set_contains_real_desktop_dependencies(self):
        packages = set(
            (ROOT / "distribution/arm64-dev/packages.aarch64")
            .read_text(encoding="utf-8")
            .splitlines()
        )
        for required in (
            "hyprland",
            "quickshell",
            "qt6-wayland",
            "networkmanager",
            "curl",
            "iputils",
            "linux-aarch64",
            "qemu-guest-agent",
            "spice-vdagent",
            "xdg-desktop-portal-hyprland",
        ):
            self.assertIn(required, packages)
        self.assertNotIn("intel-ucode", packages)
        self.assertNotIn("vulkan-intel", packages)

    def test_macos_builder_is_native_and_local(self):
        text = (
            ROOT / "scripts/distribution/build_arm64_development_image_macos.sh"
        ).read_text(encoding="utf-8")
        for required in (
            "--arch aarch64",
            "--vm-type vz",
            "--platform linux/arm64",
            "--disk 30",
            "ArchLinuxARM-aarch64-latest.tar.gz",
            "qemu-img check",
            "BLOSSOM_LLAMA_RUNTIME_ARCHIVE",
            "BLOSSOM_LLAMA_MODEL",
            "BLOSSOM_LLAMA_MODEL_LICENSE",
            "BLOSSOM_ARM64_MIN_FREE_GIB",
            "trap cleanup_build EXIT",
            "trap 'exit 130' INT",
            "trap 'exit 143' TERM",
            'rm -f "$raw" "$qcow" "$output/image/SHA256SUMS"',
            'rm -rf "$image_work"',
            "available_kib",
            "build_complete=true",
        ):
            self.assertIn(required, text)
        self.assertNotIn("github", text.lower())

    def test_guest_image_uses_uefi_and_virtio(self):
        text = (
            ROOT / "scripts/distribution/build_arm64_development_image.sh"
        ).read_text(encoding="utf-8")
        for required in (
            "systemd-bootaa64.efi",
            "BOOTAA64.EFI",
            "virtio_pci",
            "virtio_blk",
            "virtio_gpu",
            "NetworkManager.service",
            "10-blossom-dns.conf",
            "rc-manager=file",
            "systemd-resolved.service",
            "qemu-guest-agent.service",
            "blossom-arm64-qualification.service",
            "graphical.target.wants/blossom-arm64-qualification.service",
            "virtio_console",
            'gpgconf --kill all',
            'umount -R "$work/root"',
            "blossom-model-runtime-",
            "blossom-qualification-",
            "--verify-installed-root /",
            "blossom-model-llama-cpp.service",
            "blossom-model-gateway.service",
            "image-source-commit",
            "blossom-model-runtime-b[0-9]*-aarch64.pkg.tar.*",
        ):
            self.assertIn(required, text)
        self.assertNotIn("blossom-model-runtime-[0-9]*-aarch64", text)

    def test_launcher_uses_hvf_without_intel_emulation(self):
        text = (
            ROOT / "scripts/distribution/run_arm64_development_vm_macos.sh"
        ).read_text(encoding="utf-8")
        for required in (
            "qemu-system-aarch64",
            "accel=hvf",
            "-cpu host",
            "virtio-gpu-pci",
            "virtio-keyboard-pci",
            "virtio-tablet-pci",
            "BLOSSOM_ARM64_VM_RESOLUTION",
            "1280x800",
            "xres=$xres,yres=$yres",
            "edid=on",
            "zoom-to-fit=off",
        ):
            self.assertIn(required, text)
        self.assertNotIn("qemu-system-x86_64", text)

    def test_runtime_qualification_is_headless_and_machine_readable(self):
        launcher = (
            ROOT / "scripts/distribution/qualify_arm64_development_vm_macos.sh"
        ).read_text(encoding="utf-8")
        probe = (
            ROOT / "distribution/physical-rootfs/usr/local/bin/blossom-arm64-qualification-probe"
        ).read_text(encoding="utf-8")
        service = (
            ROOT / "distribution/physical-rootfs/usr/lib/systemd/system/blossom-arm64-qualification.service"
        ).read_text(encoding="utf-8")
        for required in (
            "virtserialport",
            "org.blossomos.qualification",
            "org.qemu.guest_agent.0",
            "-display none",
            "BLOSSOM_ARM64_QUALIFICATION_READY",
            "480",
            "BLOSSOM_ARM64_AGENT_GATE_READY",
        ):
            self.assertIn(required, launcher)
        for required in (
            "curl --fail",
            "getent ahosts archlinux.org",
            "Launch1 s installer",
            "blossom-installer.service",
            "Launch1 s browser",
            "/home/blossom/.config/mozilla/firefox/profiles.ini",
            "BLOSSOM_ARM64_QUALIFICATION_READY",
            "hyprctl monitors -j",
            "display=$display",
            "qualify-agent-pipeline",
            "--qualification-provider /run/blossom-no-fixture-receipt.json",
            "for attempt in {1..20}",
            'len(entries) == 100',
        ):
            self.assertIn(required, probe)
        self.assertIn(
            "ConditionPathExists=/dev/virtio-ports/org.blossomos.qualification",
            service,
        )


if __name__ == "__main__":
    unittest.main()
