import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SHELL = (ROOT / "system/shell/qml/shell.qml").read_text(encoding="utf-8")


class DesktopFoundationLayoutTests(unittest.TestCase):
    def test_qml_object_blocks_are_not_semicolon_terminated(self):
        self.assertNotIn("};", SHELL)

    def test_reserved_surfaces_fit_supported_logical_sizes(self):
        top = int(re.search(r"implicitHeight: (\d+)\n\s+exclusiveZone: 52", SHELL).group(1))
        dock_width = int(re.search(r"implicitWidth: (\d+)", SHELL).group(1))
        dock_height = int(re.search(r"implicitHeight: (\d+)\n\s+exclusiveZone: 92", SHELL).group(1))
        for width, height in ((1280, 720), (1920, 1080), (1440, 900)):
            with self.subTest(width=width, height=height):
                self.assertLessEqual(dock_width + 48, width)
                self.assertLess(top + dock_height + 180, height)

    def test_primary_output_does_not_duplicate_desktop_surfaces(self):
        self.assertIn("readonly property var desktopScreens", SHELL)
        self.assertIn("? [Quickshell.screens[0]] : []", SHELL)
        self.assertEqual(SHELL.count("model: root.desktopScreens"), 5)

    def test_desktop_and_live_installer_are_distinct(self):
        self.assertIn('WlrLayershell.namespace: "blossom-background"', SHELL)
        self.assertIn("exclusionMode: ExclusionMode.Ignore", SHELL)
        self.assertIn('WlrLayershell.namespace: "blossom-top-bar"', SHELL)
        self.assertIn('WlrLayershell.namespace: "blossom-dock"', SHELL)
        self.assertIn("visible: BlossomBroker.liveEnvironment", SHELL)
        self.assertIn('text: "Continue to desktop"', SHELL)
        self.assertIn('placeholderText: "What should Blossom do?"', SHELL)
        self.assertIn('background: Rectangle {\n                                    color: "transparent"', SHELL)
        self.assertIn("BlossomBroker.requestAgentTurn(agentPrompt.text)", SHELL)
        self.assertIn('text: BlossomBroker.state === "requesting" ? "Request sent', SHELL)
        self.assertIn("ApprovalPanel {}", SHELL)
        self.assertIn('BlossomBroker.failureReason + " Nothing was applied."', SHELL)
        self.assertIn('"Request failed closed · " + BlossomBroker.failureReason', SHELL)

    def test_window_recovery_is_discoverable(self):
        self.assertIn("Super+Tab switches windows", SHELL)
        self.assertIn("Super+Q closes the active window", SHELL)
        self.assertIn("BlossomBroker.desktopMessage", SHELL)
        self.assertIn('sequence: "Escape"', SHELL)

    def test_destructive_session_confirmations_are_centered(self):
        self.assertIn('WlrLayershell.namespace: "blossom-system-confirmation"', SHELL)
        self.assertIn("anchors.centerIn: parent", SHELL)
        self.assertIn('text: "Save your work before continuing."', SHELL)

    def test_top_bar_exposes_normal_system_controls(self):
        for label in ("Network ", "Sound", "Bluetooth", "Quick settings", "Wi-Fi & Ethernet", "Do not disturb", "Log Out"):
            self.assertIn(label, SHELL)
        for action in ("openNetworkSettings", "openAudioSettings", "openBluetoothSettings", "toggleAudioMute", "lowerVolume", "raiseVolume", "toggleDoNotDisturb", "logOut"):
            self.assertIn(f"BlossomBroker.{action}", SHELL)

    def test_onboarding_is_service_owned_and_dismissible(self):
        self.assertIn("BlossomBroker.onboardingRequired", SHELL)
        self.assertIn("BlossomBroker.dismissOnboarding()", SHELL)
        self.assertNotIn("root.welcomeVisible = false", SHELL)
        self.assertIn("!root.installerOpened", SHELL)
        self.assertIn('BlossomBroker.desktopMessage === "Opened installer."', SHELL)
        self.assertIn('BlossomBroker.desktopMessage.startsWith("Could not")', SHELL)


if __name__ == "__main__":
    unittest.main()
