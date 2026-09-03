#!/usr/bin/env python3
"""Fail closed if the inactive Phase 4 model-runtime templates drift."""

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "system" / "model-runtime" / "packaging"

GATEWAY = "blossom-model-gateway"
PROVIDER = "blossom-model-provider"
ACCESS_GROUP = "blossom-ai"
NAMESPACE_UNIT = "blossom-model-netns.service"
GATEWAY_UNIT = "blossom-model-gateway.service"
GATEWAY_TEMPLATE = f"{GATEWAY_UNIT}.in"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def read(name: str) -> str:
    path = PACKAGE / name
    require(path.is_file() and not path.is_symlink(), f"missing regular package file: {name}")
    return path.read_text()


def check_identities() -> None:
    text = read("blossom-model-runtime.sysusers")
    require(f"g {ACCESS_GROUP}" in text, "missing inference access group")
    require(f"u {GATEWAY}" in text, "missing gateway system identity")
    require(f"u {PROVIDER}" in text, "missing provider system identity")
    require(f"m {GATEWAY} {ACCESS_GROUP}" in text, "gateway cannot assign socket group")
    require("/usr/bin/nologin" in text, "service identities must be non-login")
    require(not re.search(r"\s[0-9]{2,}\s", text), "numeric IDs must not be guessed")


COMMON = [
    "PrivateNetwork=yes",
    "NoNewPrivileges=yes",
    "CapabilityBoundingSet=\n",
    "AmbientCapabilities=\n",
    "PrivateTmp=yes",
    "PrivateDevices=yes",
    "ProtectSystem=strict",
        "ProtectHome=yes",
        "InaccessiblePaths=",
    "ProtectKernelTunables=yes",
    "ProtectKernelModules=yes",
    "ProtectKernelLogs=yes",
    "ProtectControlGroups=yes",
    "RestrictNamespaces=yes",
    "RestrictRealtime=yes",
    "RestrictSUIDSGID=yes",
    "LockPersonality=yes",
    "RemoveIPC=yes",
    "SystemCallArchitectures=native",
    "SystemCallFilter=@system-service",
    "LimitCORE=0",
    "MemorySwapMax=",
    "IPAddressDeny=any",
]


def check_unit(name: str, identity: str) -> str:
    text = read(name)
    for directive in COMMON:
        require(directive in text, f"{name}: missing hardening: {directive.strip()}")
    require(f"User={identity}" in text, f"{name}: user identity drift")
    require(f"Group={identity}" in text, f"{name}: group identity drift")
    require("[Install]" not in text, f"{name}: checkpoint must not be enableable")
    forbidden = ["sudo", "pkexec", "/bin/sh", "sh -c", "bash", "AF_UNIX AF_INET AF_INET6"]
    for value in forbidden:
        require(value not in text, f"{name}: forbidden surface: {value}")
    return text


def check_namespace_and_gateway() -> None:
    namespace = check_unit(NAMESPACE_UNIT, GATEWAY)
    require("Type=oneshot" in namespace, "namespace anchor type drift")
    require("ExecStart=/usr/bin/true" in namespace, "namespace anchor executable drift")
    require("RemainAfterExit=yes" in namespace, "namespace must remain active")

    gateway = check_unit(GATEWAY_TEMPLATE, GATEWAY)
    for value in [
        f"Requires={NAMESPACE_UNIT}",
        f"JoinsNamespaceOf={NAMESPACE_UNIT}",
        "ExecStart=/usr/lib/blossom-os/blossom-model-gateway",
        "SupplementaryGroups=blossom-ai",
        "DynamicUser=no",
        "RuntimeDirectory=blossom-model-gateway",
        "InaccessiblePaths=-/boot -/home -/media -/mnt -/opt -/root -/srv -/run/user",
        "TemporaryFileSystem=/etc:ro /usr/lib/blossom-os:ro",
        "BindReadOnlyPaths=/usr/lib/blossom-os/blossom-model-gateway @PROFILE_ROOT@ @PROVIDER_DIRECTORY@ @MODEL_PATH@ /etc/passwd /etc/group /proc/sys/kernel/random/boot_id",
        "ReadWritePaths=/run/blossom-model-gateway",
        "ProtectProc=invisible",
        "ProcSubset=all",
        "RestrictAddressFamilies=AF_UNIX AF_INET",
        "IPAddressAllow=localhost",
        "Restart=no",
    ]:
        require(value in gateway, f"gateway boundary drift: {value}")
    tokens = set(re.findall(r"@[A-Za-z0-9_-]+@?", gateway))
    require(
        tokens <= {"@PROFILE_ROOT@", "@PROVIDER_DIRECTORY@", "@MODEL_PATH@", "@system-service"},
        "gateway: unknown render token",
    )


def check_provider(name: str, kind: str) -> None:
    text = check_unit(name, PROVIDER)
    for value in [
        f"Requires={NAMESPACE_UNIT}",
        f"JoinsNamespaceOf={NAMESPACE_UNIT}",
        f"Before={GATEWAY_UNIT}",
        "DynamicUser=no",
        "RestrictAddressFamilies=AF_INET",
        "IPAddressAllow=localhost",
        "InaccessiblePaths=-/boot -/etc -/home -/media -/mnt -/opt -/root -/run/user -/srv",
        "TemporaryFileSystem=/usr/lib/blossom-os:ro",
        "BindReadOnlyPaths=@PROVIDER_DIRECTORY@ @MODEL_PATH@",
        f"ReadWritePaths=/var/lib/blossom/model-provider/{kind}",
        "ProtectProc=invisible",
        "ProcSubset=pid",
        "Restart=no",
    ]:
        require(value in text, f"{name}: provider boundary drift: {value}")
    allowed_tokens = {
        "@PROVIDER_BINARY@",
        "@PROVIDER_DIRECTORY@",
        "@MODEL_PATH@",
        "@MODEL_DIRECTORY@",
        "@TASKS_MAX@",
        "@MEMORY_MAX@",
        "@MEMORY_SWAP_MAX@",
        "@CPU_QUOTA@",
        "@FILE_SIZE_MAX@",
        "@OPEN_FILES_MAX@",
        "@system-service",
    }
    tokens = set(re.findall(r"@[A-Za-z0-9_-]+@?", text))
    require(tokens <= allowed_tokens, f"{name}: unknown render token")
    require("%i" not in text and "%I" not in text, f"{name}: generic instance forbidden")
    require("DeviceAllow=" not in text, f"{name}: CPU profile must expose no device")


def main() -> None:
    expected = {
        "README.md",
        "blossom-model-runtime.sysusers",
        NAMESPACE_UNIT,
        GATEWAY_TEMPLATE,
        "blossom-model-ollama.service.in",
        "blossom-model-llama-cpp.service.in",
    }
    require({path.name for path in PACKAGE.iterdir()} == expected, "unexpected package surface")
    check_identities()
    check_namespace_and_gateway()
    check_provider("blossom-model-ollama.service.in", "ollama")
    check_provider("blossom-model-llama-cpp.service.in", "llama-cpp")


if __name__ == "__main__":
    main()
