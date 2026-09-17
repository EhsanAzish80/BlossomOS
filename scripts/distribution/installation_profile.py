#!/usr/bin/env python3
"""Validate the non-secret portion of Blossom's graphical installer profile."""

from __future__ import annotations

import re
from dataclasses import dataclass


USERNAME = re.compile(r"[a-z][a-z0-9_-]{0,30}")
RESERVED = {"bin", "daemon", "dbus", "nobody", "root", "systemd", "wheel"}
LOCALE = re.compile(r"[A-Za-z]{2,3}_[A-Za-z]{2}(?:\.UTF-8)?")
TIMEZONE = re.compile(r"[A-Za-z0-9_+-]+(?:/[A-Za-z0-9_+.-]+)+")
KEYMAP = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,31}")


class ProfileError(ValueError):
    """Raised when installer identity data is unsafe or incomplete."""


@dataclass(frozen=True)
class InstallationProfile:
    full_name: str
    username: str
    locale: str
    timezone: str
    keymap: str
    administrator: bool = True
    agent_mode: str = "off"

    def public_record(self) -> dict[str, object]:
        """Return persistable settings. Passwords are intentionally impossible here."""
        return {
            "schema": 1,
            "full_name": self.full_name,
            "username": self.username,
            "locale": self.locale,
            "timezone": self.timezone,
            "keymap": self.keymap,
            "administrator": self.administrator,
            "agent_mode": self.agent_mode,
        }


def validate_profile(value: object) -> InstallationProfile:
    if type(value) is not dict or set(value) != {
        "full_name", "username", "locale", "timezone", "keymap",
        "administrator", "agent_mode",
    }:
        raise ProfileError("installation profile schema drift")
    full_name = value["full_name"]
    username = value["username"]
    if not isinstance(full_name, str) or not 1 <= len(full_name.strip()) <= 80:
        raise ProfileError("full name is required")
    if any(character in full_name for character in "\n\r:\0"):
        raise ProfileError("full name contains an unsafe character")
    if not isinstance(username, str) or not USERNAME.fullmatch(username):
        raise ProfileError("username must start with a letter and use lowercase characters")
    if username in RESERVED:
        raise ProfileError("username is reserved")
    locale = value["locale"]
    timezone = value["timezone"]
    keymap = value["keymap"]
    if not isinstance(locale, str) or not LOCALE.fullmatch(locale):
        raise ProfileError("unsupported locale syntax")
    if not isinstance(timezone, str) or not TIMEZONE.fullmatch(timezone) or ".." in timezone:
        raise ProfileError("unsupported time zone syntax")
    if not isinstance(keymap, str) or not KEYMAP.fullmatch(keymap):
        raise ProfileError("unsupported keyboard layout syntax")
    if type(value["administrator"]) is not bool:
        raise ProfileError("administrator must be a boolean")
    if value["agent_mode"] not in ("off", "on_demand"):
        raise ProfileError("agent mode must be off or on_demand")
    return InstallationProfile(
        full_name=full_name.strip(), username=username, locale=locale,
        timezone=timezone, keymap=keymap,
        administrator=value["administrator"], agent_mode=value["agent_mode"],
    )


def validate_password(password: str, confirmation: str, username: str) -> None:
    """Validate in memory only; callers must never serialize either argument."""
    if password != confirmation:
        raise ProfileError("passwords do not match")
    if len(password) < 12:
        raise ProfileError("password must contain at least 12 characters")
    if password.casefold() == username.casefold() or username.casefold() in password.casefold():
        raise ProfileError("password must not contain the username")
    if not any(character.isalpha() for character in password) or not any(
        character.isdigit() or not character.isalnum() for character in password
    ):
        raise ProfileError("password needs letters and a number or symbol")
