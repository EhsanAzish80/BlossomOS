import unittest
from pathlib import Path


class PhysicalCandidateTests(unittest.TestCase):
    def test_desktop_runtime_packages_and_launcher_are_present(self):
        repository = Path(__file__).resolve().parents[1]
        builder = (repository / "scripts/distribution/build_physical_candidate.sh").read_text()
        image_verifier = (repository / "scripts/distribution/verify_physical_candidate_image.sh").read_text()
        candidate_workflow = (repository / ".github/workflows/phase11-physical-candidate.yml").read_text()
        desktop_probe = (repository / "distribution/archiso/airootfs/usr/local/bin/blossom-desktop-probe").read_text()
        desktop_probe_unit = (repository / "distribution/archiso/airootfs/etc/systemd/system/blossom-desktop-probe.service").read_text()
        desktop_probe_link = repository / "distribution/archiso/airootfs/etc/systemd/system/multi-user.target.wants/blossom-desktop-probe.service"
        live_user = (repository / "distribution/archiso/airootfs/usr/local/libexec/blossom-provision-live-user").read_text()
        live_user_unit = (repository / "distribution/archiso/airootfs/etc/systemd/system/blossom-live-user.service").read_text()
        live_user_link = repository / "distribution/archiso/airootfs/etc/systemd/system/multi-user.target.wants/blossom-live-user.service"
        packages = (repository / "distribution/archiso/packages.x86_64").read_text()
        live_profile = (repository / "distribution/archiso/airootfs/home/blossom/.bash_profile").read_text()
        autologin = (repository / "distribution/archiso/airootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf").read_text()
        root_console = (repository / "distribution/archiso/airootfs/etc/systemd/system/getty@tty2.service.d/autologin.conf").read_text()
        live_hyprland = (repository / "distribution/archiso/airootfs/home/blossom/.config/hypr/hyprland.conf").read_text()
        installed_hyprland = (repository / "distribution/physical-rootfs/usr/share/blossom-os/physical-home/hyprland.conf").read_text()
        shell_package = (repository / "distribution/packages/blossom-shell/PKGBUILD").read_text()
        core_package = (repository / "distribution/packages/blossom-core/PKGBUILD").read_text()
        shell_unit = (repository / "system/shell/packaging/blossom-shell-ui.service").read_text()
        recovery_unit = (repository / "system/shell/packaging/blossom-shell-recovery.service").read_text()
        recovery_command = (repository / "system/shell/packaging/blossom-shell-recovery").read_text()
        session_command = (repository / "system/shell/runtime/blossom-start-session").read_text()
        shell_qml = (repository / "system/shell/qml/shell.qml").read_text()
        broker_header = (repository / "system/shell/client-plugin/blossombroker.h").read_text()
        broker_source = (repository / "system/shell/client-plugin/blossombroker.cpp").read_text()
        desktop_launcher = (repository / "system/desktop-launcher/main.cpp").read_text()
        desktop_unit = (repository / "system/desktop-launcher/blossom-desktop-launcher.service").read_text()
        installer_rule = (repository / "distribution/archiso/airootfs/etc/polkit-1/rules.d/49-blossom-live-installer.rules").read_text()
        greetd = (repository / "distribution/physical-rootfs/etc/greetd/config.toml").read_text()
        session = (repository / "distribution/physical-rootfs/usr/share/wayland-sessions/blossom.desktop").read_text()
        self.assertIn("noto-fonts", builder)
        self.assertIn("noto-fonts-emoji", builder)
        for package in ("adwaita-cursors", "foot", "iputils", "noto-fonts", "noto-fonts-emoji"):
            self.assertIn(f"\n{package}\n", f"\n{packages}")
        for package in ("blueman", "bluez", "bluez-utils", "firefox", "gvfs", "mousepad",
                        "networkmanager", "network-manager-applet", "nm-connection-editor",
                        "pavucontrol", "pipewire", "pipewire-alsa", "pipewire-pulse",
                        "thunar", "tumbler", "wireplumber"):
            self.assertIn(f"\n{package}\n", f"\n{packages}")
            self.assertIn(package, builder)
        for package in (
            "grim", "hyprpolkitagent", "libnotify", "mako", "qt6-wayland",
            "slurp", "xdg-desktop-portal", "xdg-desktop-portal-gtk",
            "xdg-desktop-portal-hyprland", "xorg-xwayland",
        ):
            self.assertIn(f"\n{package}\n", f"\n{packages}")
            self.assertIn(package, builder)
        self.assertIn('start-hyprland >"$session_log" 2>&1', live_profile)
        self.assertIn("Blossom OS could not start the desktop", live_profile)
        self.assertNotIn('useradd --root "$rootfs"', builder)
        self.assertNotIn("getty-autologin.conf", builder)
        self.assertIn("greetd", builder)
        self.assertIn("regreet", builder)
        self.assertIn('command = "Hyprland --config /etc/greetd/hyprland.conf"', greetd)
        self.assertIn("Exec=/usr/bin/start-hyprland", session)
        self.assertIn('graphical.target.wants/greetd.service', builder)
        graphical_wants = 'install -d -m 0755 "$rootfs/etc/systemd/system/graphical.target.wants"'
        greetd_link = '"$rootfs/etc/systemd/system/graphical.target.wants/greetd.service"'
        self.assertIn(graphical_wants, builder)
        self.assertLess(builder.index(graphical_wants), builder.index(greetd_link))
        self.assertIn(
            'cp -a "$repo/distribution/archiso/airootfs/." "$profile/airootfs/"',
            builder,
        )
        self.assertIn(
            '"$profile/airootfs/etc/systemd/system/multi-user.target.wants/NetworkManager.service"',
            builder,
        )
        self.assertIn(
            'chmod 0755 "$profile/airootfs/usr/local/bin/blossom-screenshot"',
            builder,
        )
        profile = (repository / "distribution/archiso/profiledef.sh").read_text()
        self.assertIn(
            '["/usr/local/bin/blossom-screenshot"]="0:0:755"',
            profile,
        )
        self.assertIn("--autologin blossom", autologin)
        self.assertIn("Requires=blossom-live-user.service", autologin)
        self.assertNotIn("useradd --create-home", autologin)
        self.assertIn("useradd --create-home", live_user)
        self.assertIn("chown -R blossom:blossom /home/blossom", live_user)
        self.assertIn('test "$(id -u blossom)" = 1000', live_user)
        self.assertIn("Before=getty@tty1.service blossom-desktop-probe.service", live_user_unit)
        self.assertTrue(live_user_link.is_symlink())
        self.assertEqual(live_user_link.readlink(), Path("../blossom-live-user.service"))
        self.assertNotIn("--autologin root", autologin)
        self.assertIn("--autologin root", root_console)
        for config in (live_hyprland, installed_hyprland):
            self.assertIn("XCURSOR_THEME,Adwaita", config)
            self.assertIn("background_color = rgb(0b111b)", config)
            self.assertIn("/usr/local/bin/blossom-start-session", config)
            self.assertNotIn("quickshell -p", config)
        self.assertIn('client-build/blossom-shell-ui', shell_package)
        self.assertIn('installer-build/blossom-installer', shell_package)
        self.assertIn('/usr/lib/qt6/qml/Blossom/Shell', shell_package)
        self.assertIn('blossom-shell-ui.service', shell_package)
        self.assertIn('blossom-shell-recovery.service', shell_package)
        self.assertIn('/usr/local/bin/blossom-shell-recovery', shell_package)
        self.assertIn('/usr/local/bin/blossom-start-session', shell_package)
        self.assertIn("--package blossom-shell-service", core_package)
        self.assertIn("--features production-dbus-service", core_package)
        self.assertIn("ExecStart=/usr/lib/blossom-os/blossom-shell-ui", shell_unit)
        self.assertIn("Restart=on-failure", shell_unit)
        self.assertIn("OnFailure=blossom-shell-recovery.service", shell_unit)
        self.assertIn("blossom-shell-recovery", recovery_unit)
        self.assertIn("restart-shell", recovery_command)
        self.assertIn("systemctl --user import-environment", session_command)
        self.assertIn("WAYLAND_DISPLAY", session_command)
        self.assertIn("systemctl --user start blossom-shell-ui.service", session_command)
        self.assertIn("systemctl --user start blossom-shell-service.service", session_command)
        self.assertIn("dbus-update-activation-environment --systemd", session_command)
        for service in (
            "hyprpolkitagent.service", "mako.service",
            "xdg-desktop-portal.service", "xdg-desktop-portal-hyprland.service",
        ):
            self.assertIn(service, session_command)
        self.assertNotIn("start graphical-session.target", session_command)
        self.assertIn('bsdtar -xpf "$package" -C "$profile/airootfs"', builder)
        self.assertIn('"$packages"/blossom-core-*.pkg.tar.zst', builder)
        self.assertIn('"$packages"/blossom-shell-*.pkg.tar.zst', builder)
        self.assertIn("Welcome to Blossom OS", shell_qml)
        self.assertIn("Install Blossom OS", shell_qml)
        self.assertIn("External disks are excluded", shell_qml)
        for surface in ("Applications", "Files", "Web Browser", "Text Editor", "Network Settings", "Audio Settings"):
            self.assertIn(surface, shell_qml)
        self.assertIn("liveEnvironment", broker_header)
        self.assertIn('/usr/lib/blossom-os/blossom-installer', desktop_launcher)
        for method in ("openFiles", "openBrowser", "openEditor", "openNetworkSettings", "openAudioSettings"):
            self.assertIn(method, broker_header)
        for method in ("restartSystem", "powerOff"):
            self.assertIn(method, broker_header)
        for method in ("openBluetoothSettings", "toggleAudioMute", "lowerVolume", "raiseVolume", "toggleDoNotDisturb", "logOut"):
            self.assertIn(method, broker_header)
        self.assertNotIn("QProcess", broker_source)
        self.assertIn('QStringLiteral("Launch1")', broker_source)
        for executable in ("/usr/bin/thunar", "/usr/bin/firefox", "/usr/bin/mousepad", "/usr/bin/nm-connection-editor", "/usr/bin/pavucontrol", "/usr/bin/blueman-manager", "/usr/bin/wpctl", "/usr/bin/makoctl"):
            self.assertIn(executable, desktop_launcher)
        self.assertIn('/usr/bin/hyprctl', desktop_launcher)
        self.assertIn('/usr/bin/systemctl', desktop_launcher)
        self.assertIn('/usr/bin/systemd-run', desktop_launcher)
        self.assertIn('QStringLiteral("--user")', desktop_launcher)
        self.assertIn('QStringLiteral("--collect")', desktop_launcher)
        self.assertIn("NoNewPrivileges=yes", desktop_unit)
        self.assertIn("ProtectHome=read-only", desktop_unit)
        self.assertIn("RestrictAddressFamilies=AF_UNIX", desktop_unit)
        for config in (live_hyprland, installed_hyprland):
            for binding in ("SUPER, Q, killactive", "SUPER, F, fullscreen", "SUPER, E, exec, thunar", "SUPER, 1, workspace, 1"):
                self.assertIn(binding, config)
        self.assertIn('subject.user == "blossom"', installer_rule)
        self.assertIn('action.lookup("program") == "/usr/local/libexec/blossom-graphical-install-backend"', installer_rule)
        self.assertIn("Blossom OS Recovery Console", builder)
        self.assertIn("vt.global_cursor_default=0", builder)
        self.assertIn("blossom.recovery=1", live_profile)
        self.assertIn("verify_physical_candidate_image.sh", candidate_workflow)
        for executable in (
            "blossom-shell-service",
            "blossom-shell-ui",
            "blossom-start-session",
            "blossom-screenshot",
            "xdg-desktop-portal-hyprland",
        ):
            self.assertIn(executable, image_verifier)
        self.assertIn('test -x "$root/$path"', image_verifier)
        self.assertIn("Boot exact candidate and require the graphical desktop", candidate_workflow)
        self.assertIn("blossom.desktop-probe=1", candidate_workflow)
        self.assertIn("systemd.getty_auto=no", candidate_workflow)
        self.assertIn("BLOSSOM_DESKTOP_READY security=active broker=active desktop=active layers=4", candidate_workflow)
        self.assertIn("hyprctl layers -j", desktop_probe)
        for namespace in ("blossom-background", "blossom-top-bar", "blossom-dock", "blossom-welcome"):
            self.assertIn(namespace, desktop_probe)
        self.assertIn("blossom-shell-ui.service", desktop_probe)
        self.assertIn("blossom-shell-service.service", desktop_probe)
        self.assertIn("blossom-desktop-shell.service", desktop_probe)
        self.assertIn("ExecCondition=/usr/bin/grep -qw blossom.desktop-probe=1 /proc/cmdline", desktop_probe_unit)
        self.assertTrue(desktop_probe_link.is_symlink())
        self.assertEqual(desktop_probe_link.readlink(), Path("../blossom-desktop-probe.service"))

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
        self.assertIn("--detach", builder)
        self.assertIn('logs --follow "$container"', builder)
        self.assertIn('wait "$container"', builder)
        self.assertIn("failed_or_interrupted", builder)
        self.assertIn("preserving diagnostics and Docker state", builder)

if __name__ == "__main__":
    unittest.main()
