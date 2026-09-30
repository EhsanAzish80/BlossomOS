#!/usr/bin/env python3
"""Fail closed if the broker, desktop UI, or recovery package boundary drifts."""

from pathlib import Path
import json
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
SHELL = ROOT / "system" / "shell"
PACKAGE = SHELL / "packaging"
LOCK = SHELL / "registry" / "arch-x86_64.lock.json"
EVIDENCE_LOCK = SHELL / "evidence" / "parent-compositors-arch-x86_64.lock.json"
BUS_NAME = "org.blossomos.Shell1"
BINARY = "/usr/lib/blossom-os/blossom-shell-service"
UNIT = "blossom-shell-broker@.service"
UI_UNIT = "blossom-shell-ui.service"
DESKTOP_UNIT = "blossom-desktop-shell.service"
RECOVERY_UNIT = "blossom-shell-recovery.service"
RECOVERY = "blossom-shell-recovery"
POLKIT_AGENT_UNIT = "blossom-polkit-agent.service"
POLICY = "org.blossomos.shell.policy"
MODEL_EFFECT_ACTION = "org.blossomos.shell.approve-model-effect"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def check_lock() -> None:
    require(LOCK.is_file() and not LOCK.is_symlink(), "missing regular shell lock")
    data = json.loads(LOCK.read_text())
    require(set(data) == {"schema_version", "observed_at", "architecture", "packages"}, "lock schema drift")
    require(data["schema_version"] == 1, "lock version drift")
    require(data["observed_at"] == "2026-09-04", "observation date drift")
    require(data["architecture"] == "x86_64", "unsupported architecture")
    expected = {"hyprland": ("0.56.2-2", "extra"), "quickshell": ("0.3.1-1", "extra"), "systemd": ("261.2-1", "core"), "dbus-broker": ("37-3", "core")}
    packages = data["packages"]
    require(len(packages) == len(expected), "package set must remain closed")
    require(len({item.get("name") for item in packages}) == len(packages), "duplicate package")
    for item in packages:
        require(set(item) == {"name", "version", "repository", "source"}, "package schema drift")
        name = item["name"]
        require(name in expected, f"unreviewed shell package: {name}")
        require((item["version"], item["repository"]) == expected[name], f"package pin drift: {name}")
        require(item["repository"] in {"core", "extra"}, f"non-stable repository: {name}")
        require(item["source"] == f"https://archlinux.org/packages/{item['repository']}/x86_64/{name}/", f"package source drift: {name}")


def check_evidence_lock() -> None:
    require(EVIDENCE_LOCK.is_file() and not EVIDENCE_LOCK.is_symlink(), "missing regular evidence lock")
    data = json.loads(EVIDENCE_LOCK.read_text())
    require(set(data) == {"schema_version", "purpose", "architecture", "packages"}, "evidence lock schema drift")
    require(data["schema_version"] == 1, "evidence lock version drift")
    require(data["purpose"] == "ci-parent-compositors-only", "evidence lock purpose drift")
    require(data["architecture"] == "x86_64", "unsupported evidence architecture")
    packages = data["packages"]
    require(len(packages) == 1, "evidence parent set drift")
    item = packages[0]
    require(set(item) == {"name", "version", "repository", "source"}, "evidence package schema drift")
    require((item["name"], item["version"], item["repository"]) == ("niri", "26.04-1", "extra"), "evidence parent pin drift")
    require(item["source"] == "https://archlinux.org/packages/extra/x86_64/niri/", "evidence source drift")


def check_unit() -> None:
    text = (PACKAGE / UNIT).read_text()
    required = ["BindsTo=user@%i.service", "After=user@%i.service", "RequiresMountsFor=/run/user/%i", "Type=simple", "ExecCondition=/usr/bin/test %i = 1000", "User=%i", "Group=%i", "SupplementaryGroups=blossom-ai", "Environment=HOME=/home/blossom", "Environment=XDG_RUNTIME_DIR=/run/user/%i", "Environment=DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/%i/bus", f"ExecStart={BINARY}", "Restart=no", "NoNewPrivileges=yes", "CapabilityBoundingSet=\n", "AmbientCapabilities=\n", "PrivateDevices=yes", "ProtectSystem=strict", "ProtectHome=tmpfs", "RestrictAddressFamilies=AF_UNIX AF_NETLINK", "MemoryDenyWriteExecute=yes", "IPAddressDeny=any"]
    for value in required:
        require(value in text, f"missing shell unit boundary: {value.strip()}")
    exposure = [line.strip() for line in text.splitlines()
                if line.strip().startswith(("BindPaths=", "BindReadOnlyPaths=", "ReadWritePaths=", "ReadOnlyPaths=", "ProtectHome="))]
    require(exposure == ["ProtectHome=tmpfs", "BindReadOnlyPaths=/run/user/%i/bus", "BindPaths=/home/blossom/Workspace"],
            "shell may expose only the session bus and bounded workspace through hidden homes")
    require("[Install]" not in text, "checkpoint must not be enableable")
    families = [line.strip() for line in text.splitlines()
                if line.strip().startswith("RestrictAddressFamilies=")]
    require(families == ["RestrictAddressFamilies=AF_UNIX AF_NETLINK"],
            "shell service address-family boundary drift")
    require("RestrictSUIDSGID=" not in text,
            "RestrictSUIDSGID blocks Bubblewrap's required openat2 syscall")
    for value in ["User=root", "User=blossom-model-gateway", "sudo", "pkexec", "/bin/sh", "sh -c", "bash", "systemctl", "RestrictNamespaces=", "PrivateUsers="]:
        require(value not in text, f"forbidden shell package surface: {value}")


def check_ui_unit() -> None:
    text = (PACKAGE / UI_UNIT).read_text()
    for value in [
        "PartOf=graphical-session.target",
        "After=graphical-session.target",
        f"OnFailure={RECOVERY_UNIT}",
        "StartLimitIntervalSec=30",
        "StartLimitBurst=3",
        "Type=exec",
        "ExecStart=/usr/lib/blossom-os/blossom-shell-ui",
        "Restart=on-failure",
        "Environment=QT_QPA_PLATFORM=wayland",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ProtectHome=read-only",
        "RestrictSUIDSGID=yes",
        "WantedBy=graphical-session.target",
    ]:
        require(value in text, f"missing shell UI boundary: {value}")
    for value in ["User=root", "sudo", "pkexec", "/bin/sh", "sh -c", "bash -c"]:
        require(value not in text, f"forbidden shell UI surface: {value}")


def check_desktop_unit() -> None:
    text = (PACKAGE / DESKTOP_UNIT).read_text()
    for value in [
        "PartOf=graphical-session.target",
        "After=graphical-session.target",
        "RuntimeDirectory=quickshell",
        "RuntimeDirectoryMode=0700",
        f"OnFailure={RECOVERY_UNIT}",
        "ExecStart=/usr/bin/quickshell -p /usr/share/blossom-os/shell",
        "Restart=on-failure",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ProtectHome=read-only",
        "RestrictSUIDSGID=yes",
    ]:
        require(value in text, f"missing desktop shell boundary: {value}")
    for value in ["User=root", "sudo", "pkexec", "/bin/sh", "sh -c", "bash -c"]:
        require(value not in text, f"forbidden desktop shell surface: {value}")


def check_recovery() -> None:
    unit = (PACKAGE / RECOVERY_UNIT).read_text()
    for value in [
        f"ExecStart=/usr/bin/foot --title=Blossom Shell Recovery /usr/local/bin/{RECOVERY}",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ProtectHome=read-only",
        "RestrictSUIDSGID=yes",
    ]:
        require(value in unit, f"missing shell recovery boundary: {value}")
    require("[Install]" not in unit, "recovery unit must not be independently enableable")
    script = (PACKAGE / RECOVERY).read_text()
    for value in [
        "set -euo pipefail",
        "blossom-desktop-shell.service",
        "blossom-shell-ui.service",
        "systemctl --user reset-failed",
        "systemctl --user restart",
        "exec bash --noprofile --norc",
    ]:
        require(value in script, f"missing bounded recovery behavior: {value}")
    for value in ["sudo", "pkexec", "eval ", "curl ", "wget "]:
        require(value not in script, f"forbidden recovery authority: {value}")


def check_polkit_agent_unit() -> None:
    text = (PACKAGE / POLKIT_AGENT_UNIT).read_text()
    for value in [
        "PartOf=graphical-session.target",
        "After=graphical-session.target",
        "ConditionEnvironment=WAYLAND_DISPLAY",
        "ExecStart=/usr/bin/lxqt-policykit-agent",
        "Slice=session.slice",
        "Restart=on-failure",
        "RestartSec=1sec",
        "WantedBy=graphical-session.target",
    ]:
        require(value in text, f"missing PolicyKit agent lifecycle rule: {value}")
    for value in ["User=root", "sudo", "pkexec", "/bin/sh", "sh -c", "bash -c"]:
        require(value not in text, f"forbidden PolicyKit agent authority: {value}")


def check_polkit_helper_preset() -> None:
    preset = (PACKAGE / "50-blossom-core.preset").read_text()
    require(preset == "enable polkit-agent-helper.socket\n",
            "PolicyKit helper socket preset must remain exact and enabled")
    core_package = (ROOT / "distribution/packages/blossom-core/PKGBUILD").read_text()
    require("$pkgdir/usr/lib/systemd/system-preset/50-blossom-core.preset" in core_package,
            "blossom-core must ship the PolicyKit helper socket preset")
    shell_package = (ROOT / "distribution/packages/blossom-shell/PKGBUILD").read_text()
    require("'lxqt-policykit'" in shell_package,
            "blossom-shell must depend on the reviewed graphical PolicyKit agent")
    require("hyprpolkitagent" not in shell_package,
            "the crashing Hyprtoolkit PolicyKit agent must not return")


def check_policy() -> None:
    root = ET.parse(PACKAGE / POLICY).getroot()
    actions = root.findall("action")
    require(len(actions) == 1, "shell policy action set must remain closed")
    action = actions[0]
    require(action.get("id") == MODEL_EFFECT_ACTION, "shell policy action drift")
    defaults = action.find("defaults")
    require(defaults is not None, "shell policy defaults missing")
    require(defaults.findtext("allow_any") == "no", "shell policy must deny remote callers")
    require(defaults.findtext("allow_inactive") == "no", "shell policy must deny inactive callers")
    require(defaults.findtext("allow_active") == "auth_self", "each active approval must authenticate")
    require(
        "auth_self_keep" not in (PACKAGE / POLICY).read_text(),
        "shell approval authentication must never be cached",
    )


def main() -> None:
    expected = {
        "README.md", UNIT, UI_UNIT, DESKTOP_UNIT, RECOVERY_UNIT, RECOVERY,
        POLKIT_AGENT_UNIT, "50-blossom-core.preset", POLICY,
    }
    require({path.name for path in PACKAGE.iterdir()} == expected, "unexpected shell package surface")
    check_lock()
    check_evidence_lock()
    check_unit()
    check_ui_unit()
    check_desktop_unit()
    check_recovery()
    check_polkit_agent_unit()
    check_polkit_helper_preset()
    check_policy()


if __name__ == "__main__":
    main()
