import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SHELL = (ROOT / "system/shell/qml/shell.qml").read_text(encoding="utf-8")


class DesktopFoundationLayoutTests(unittest.TestCase):
    def test_reserved_surfaces_fit_supported_logical_sizes(self):
        top = int(re.search(r"implicitHeight: (\d+)\n\s+exclusiveZone: 52", SHELL).group(1))
        dock_width = int(re.search(r"implicitWidth: (\d+)", SHELL).group(1))
        dock_height = int(re.search(r"implicitHeight: (\d+)\n\s+exclusiveZone: 92", SHELL).group(1))
        for width, height in ((1280, 720), (1920, 1080), (1440, 900)):
            with self.subTest(width=width, height=height):
                self.assertLessEqual(dock_width + 48, width)
                self.assertLess(top + dock_height + 180, height)

    def test_desktop_and_live_installer_are_distinct(self):
        self.assertIn('WlrLayershell.namespace: "blossom-background"', SHELL)
        self.assertIn('WlrLayershell.namespace: "blossom-top-bar"', SHELL)
        self.assertIn('WlrLayershell.namespace: "blossom-dock"', SHELL)
        self.assertIn("visible: BlossomBroker.liveEnvironment", SHELL)
        self.assertIn('text: "Continue to desktop"', SHELL)
        self.assertIn("The desktop works without an active model", SHELL)

    def test_window_recovery_is_discoverable(self):
        self.assertIn("Super+Tab switches windows", SHELL)
        self.assertIn("Super+Q closes the active window", SHELL)
        self.assertIn("BlossomBroker.desktopMessage", SHELL)

    def test_onboarding_is_service_owned_and_dismissible(self):
        self.assertIn("BlossomBroker.onboardingRequired", SHELL)
        self.assertIn("BlossomBroker.dismissOnboarding()", SHELL)
        self.assertNotIn("root.welcomeVisible = false", SHELL)


if __name__ == "__main__":
    unittest.main()
