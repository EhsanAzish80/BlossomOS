import tempfile
import unittest
from io import StringIO
from pathlib import Path
from unittest.mock import patch

from scripts.distribution.installation_profile import (
    ProfileError, validate_password, validate_profile,
)
from scripts.distribution.provision_installed_identity import provision
from scripts.distribution.graphical_install_backend import _load_request


def profile():
    return validate_profile({
        "full_name": "Ehsan Azish", "username": "ehsan", "locale": "en_US.UTF-8",
        "timezone": "Europe/Istanbul", "keymap": "us", "administrator": True,
        "agent_mode": "off",
    })


class InstallerIdentityLoginTests(unittest.TestCase):
    def test_graphical_backend_accepts_only_the_closed_request_schema(self):
        value = {
            "schema": 1, "observation": "/run/user/1000/blossom-installer/observation.json",
            "confirmation": "ERASE /dev/sda digest", "profile": {},
            "password": "secret-not-logged", "password_confirmation": "secret-not-logged",
        }
        import json
        self.assertEqual(_load_request(StringIO(json.dumps(value))), value)
        with self.assertRaises(ValueError):
            _load_request(StringIO(json.dumps({**value, "debug": True})))

    def test_public_record_cannot_contain_a_password(self):
        record = profile().public_record()
        self.assertNotIn("password", record)
        self.assertEqual(record["agent_mode"], "off")

    def test_rejects_reserved_or_unsafe_identity(self):
        value = profile().public_record()
        value.pop("schema")
        for username in ("root", "Upper", "two words", "a" * 32):
            with self.subTest(username=username), self.assertRaises(ProfileError):
                validate_profile({**value, "username": username})

    def test_password_policy_and_confirmation(self):
        validate_password("a-long-passphrase-9", "a-long-passphrase-9", "ehsan")
        for password, confirmation in (("short1", "short1"), ("ehsan-password-9", "ehsan-password-9"), ("long-password-9", "different-password-9")):
            with self.assertRaises(ProfileError):
                validate_password(password, confirmation, "ehsan")

    @patch("scripts.distribution.provision_installed_identity.os.geteuid", return_value=0)
    def test_provision_uses_stdin_for_secret_and_persists_only_public_state(self, _geteuid):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "etc").mkdir()
            zone = root / "usr/share/zoneinfo/Europe/Istanbul"
            zone.parent.mkdir(parents=True)
            zone.write_bytes(b"tz")
            calls = []
            def run(command, **kwargs):
                calls.append((command, kwargs))
                class Result: pass
                return Result()
            provision(root, profile(), "a-long-passphrase-9", "a-long-passphrase-9", run)
            argv = " ".join(part for command, _ in calls for part in command)
            self.assertNotIn("a-long-passphrase-9", argv)
            password_call = next(item for item in calls if item[0][0] == "chpasswd")
            self.assertEqual(password_call[1]["input"], "ehsan:a-long-passphrase-9\n")
            self.assertTrue(any(item[0][:1] == ["arch-chroot"] for item in calls))
            owner = (root / "var/lib/blossom/installation/owner.json").read_text()
            self.assertNotIn("password", owner)
            self.assertNotIn("a-long-passphrase-9", owner)

    def test_graphical_sources_and_installed_greeter_are_explicit(self):
        repository = Path(__file__).resolve().parents[1]
        qml = (repository / "system/installer/qml/Main.qml").read_text()
        greetd = (repository / "distribution/physical-rootfs/etc/greetd/config.toml").read_text()
        session = (repository / "distribution/physical-rootfs/usr/share/wayland-sessions/blossom.desktop").read_text()
        for text in ("Language and keyboard", "Connect or continue offline", "Choose an installation disk", "Create your account", "Review before erasing"):
            self.assertIn(text, qml)
        self.assertIn('echoMode: TextInput.Password', qml)
        self.assertIn('password.text = ""', qml)
        self.assertIn('command = "Hyprland --config /etc/greetd/hyprland.conf"', greetd)
        self.assertIn("Exec=/usr/bin/start-hyprland", session)


if __name__ == "__main__":
    unittest.main()
