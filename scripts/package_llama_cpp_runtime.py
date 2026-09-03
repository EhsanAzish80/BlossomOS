#!/usr/bin/env python3
"""Build the one closed llama.cpp CPU package tree from pre-fetched inputs."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import tarfile

ROOT = Path(__file__).resolve().parents[1]
LOCK_PATH = ROOT / "system/model-runtime/registry/llama-cpp-cpu-x86_64.lock.json"
REGISTRY_PATH = ROOT / "system/model-runtime/registry/llama-cpp-cpu-x86_64.profile.json"
PACKAGE = ROOT / "system/model-runtime/packaging"
RUNTIME_ROOT = PurePosixPath("/usr/lib/blossom-os/providers/llama-cpp")
MODEL_PATH = PurePosixPath(
    "/usr/lib/blossom-os/models/llama-cpp/qwen2.5-0.5b-instruct-q4_k_m.gguf"
)
PROFILE_PATH = PurePosixPath(
    "/etc/blossom-os/model-profiles/llama-cpp-cpu-x86_64.json"
)
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_GATEWAY_BYTES = 64 * 1024 * 1024


def fail(message: str) -> None:
    raise SystemExit(message)


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest_file(path: Path, maximum: int) -> tuple[str, int]:
    if path.is_symlink() or not path.is_file():
        fail(f"input is not a regular non-symlink file: {path}")
    size = path.stat().st_size
    if size <= 0 or size > maximum:
        fail(f"input size is outside the closed bound: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest(), size


def load_lock() -> dict:
    lock = json.loads(LOCK_PATH.read_text(encoding="utf-8"))
    if set(lock) != {"schema_version", "profile", "architecture", "runtime", "model"}:
        fail("lock schema drift")
    if (lock["schema_version"], lock["profile"], lock["architecture"]) != (
        1,
        "llama_cpp_cpu_v1",
        "x86_64",
    ):
        fail("lock identity drift")
    runtime = lock["runtime"]
    model = lock["model"]
    if set(runtime) != {"project", "version", "archive", "url", "sha256", "bytes", "license", "members"}:
        fail("runtime lock schema drift")
    if set(model) != {"project", "revision", "file", "url", "sha256", "bytes", "license"}:
        fail("model lock schema drift")
    if runtime["project"] != "ggml-org/llama.cpp" or runtime["version"] != "b10775":
        fail("runtime pin drift")
    if model["project"] != "Qwen/Qwen2.5-0.5B-Instruct-GGUF":
        fail("model pin drift")
    if set(runtime["license"]) != {"source", "sha256", "bytes", "spdx"}:
        fail("runtime license schema drift")
    if set(model["license"]) != {"file", "url", "sha256", "bytes", "spdx"}:
        fail("model license schema drift")
    if runtime["license"]["spdx"] != "MIT" or model["license"]["spdx"] != "Apache-2.0":
        fail("license pin drift")
    if runtime["version"] not in runtime["url"] or model["revision"] not in model["url"]:
        fail("mutable upstream URL")
    if model["revision"] not in model["license"]["url"]:
        fail("mutable model license URL")
    pinned_objects = [runtime, runtime["license"], model, model["license"]]
    if any(
        len(item["sha256"]) != 64
        or any(character not in "0123456789abcdef" for character in item["sha256"])
        or not isinstance(item["bytes"], int)
        or item["bytes"] <= 0
        for item in pinned_objects
    ):
        fail("invalid pinned digest or size")
    seen_sources: set[str] = set()
    seen_installs: set[str] = set()
    for member in runtime["members"]:
        if set(member) != {"source", "installs", "sha256", "bytes"}:
            fail("runtime member schema drift")
        if (
            len(member["sha256"]) != 64
            or any(character not in "0123456789abcdef" for character in member["sha256"])
            or not isinstance(member["bytes"], int)
            or member["bytes"] <= 0
        ):
            fail("invalid runtime member digest or size")
        source = PurePosixPath(member["source"])
        if source.name != member["source"] or member["source"] in seen_sources:
            fail("unsafe or duplicate runtime source")
        seen_sources.add(member["source"])
        for installed in member["installs"]:
            target = PurePosixPath(installed)
            if target.name != installed or installed in seen_installs:
                fail("unsafe or duplicate runtime install path")
            seen_installs.add(installed)
    if "llama-server" not in seen_installs:
        fail("runtime does not bind llama-server")
    return lock


def artifact(path: PurePosixPath, sha256: str, size: int) -> dict:
    return {"path": str(path), "sha256": sha256, "bytes": size}


def artifact_set_digest(files: list[dict]) -> str:
    return digest_bytes(json.dumps(files, separators=(",", ":")).encode())


def render_provider_unit() -> bytes:
    template = (PACKAGE / "blossom-model-llama-cpp.service.in").read_text(encoding="utf-8")
    replacements = {
        "@PROVIDER_BINARY@": str(RUNTIME_ROOT / "llama-server"),
        "@PROVIDER_DIRECTORY@": str(RUNTIME_ROOT),
        "@MODEL_PATH@": str(MODEL_PATH),
        "@TASKS_MAX@": "64",
        "@MEMORY_MAX@": "4G",
        "@MEMORY_SWAP_MAX@": "0",
        "@CPU_QUOTA@": "200%",
        "@FILE_SIZE_MAX@": "1M",
        "@OPEN_FILES_MAX@": "256",
    }
    for token, value in replacements.items():
        if token not in template:
            fail(f"provider template token drift: {token}")
        template = template.replace(token, value)
    if "@" in template.replace("@system-service", ""):
        fail("unresolved provider template token")
    return template.encode()


def render_gateway_unit() -> bytes:
    template = (PACKAGE / "blossom-model-gateway.service.in").read_text(encoding="utf-8")
    replacements = {
        "@PROFILE_PATH@": str(PROFILE_PATH),
        "@PROVIDER_DIRECTORY@": str(RUNTIME_ROOT),
        "@MODEL_PATH@": str(MODEL_PATH),
    }
    for token, value in replacements.items():
        if token not in template:
            fail(f"gateway template token drift: {token}")
        template = template.replace(token, value)
    if "@" in template.replace("@system-service", ""):
        fail("unresolved gateway template token")
    return template.encode()


def registry_bytes(lock: dict) -> bytes:
    runtime_files = []
    for member in lock["runtime"]["members"]:
        for installed in member["installs"]:
            runtime_files.append(
                artifact(RUNTIME_ROOT / installed, member["sha256"], member["bytes"])
            )
    runtime_files.sort(key=lambda item: item["path"])
    model_file = artifact(MODEL_PATH, lock["model"]["sha256"], lock["model"]["bytes"])
    binary = next(item for item in runtime_files if item["path"] == str(RUNTIME_ROOT / "llama-server"))
    unit = render_provider_unit()
    manifest = {
        "profile_version": 5,
        "profile": "llama_cpp_cpu_v1",
        "provider": "llama_cpp",
        "logical_model": "qwen2.5-0.5b-instruct:q4_k_m",
        "gateway_protocol_version": 1,
        "model_protocol_version": 1,
        "binary": binary,
        "runtime_mount": str(RUNTIME_ROOT),
        "runtime_files": runtime_files,
        "runtime_set_sha256": artifact_set_digest(runtime_files),
        "model_mount": str(MODEL_PATH),
        "model_files": [model_file],
        "model_set_sha256": artifact_set_digest([model_file]),
        "unit_sha256": digest_bytes(unit),
        "executable_arguments": [
            str(RUNTIME_ROOT / "llama-server"),
            "--model",
            str(MODEL_PATH),
            "--no-webui",
        ],
        "environment_names": ["HOME"],
        "endpoint": "127.0.0.1:8080",
        "inference_path": "/v1/chat/completions",
        "filesystem": {
            "read_only_paths": [str(RUNTIME_ROOT), str(MODEL_PATH)],
            "writable_paths": ["/var/lib/blossom/model-provider/llama-cpp"],
            "devices": [],
        },
        "resources": {
            "memory_max_bytes": 4294967296,
            "memory_swap_max_bytes": 0,
            "cpu_quota_percent": 200,
            "tasks_max": 64,
            "open_files_max": 256,
            "file_size_max_bytes": 1048576,
            "output_max_bytes": 131072,
            "request_deadline_ms": 120000,
        },
        "identity": {
            "gateway_user": "blossom-model-gateway",
            "gateway_group": "blossom-model-gateway",
            "provider_user": "blossom-model-provider",
            "provider_group": "blossom-model-provider",
            "access_group": "blossom-ai",
            "gateway_unit": "blossom-model-gateway.service",
            "provider_unit": "blossom-model-llama-cpp.service",
            "namespace_unit": "blossom-model-netns.service",
        },
    }
    return json.dumps(manifest, separators=(",", ":")).encode()


def copy_exact(source: Path, destination: Path, mode: int) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with source.open("rb") as input_file, destination.open("xb") as output_file:
        shutil.copyfileobj(input_file, output_file, 1024 * 1024)
        output_file.flush()
        os.fsync(output_file.fileno())
    destination.chmod(mode)


def build(
    lock: dict,
    archive: Path,
    model: Path,
    model_license: Path,
    gateway: Path,
    output: Path,
) -> None:
    paths = (archive, model, model_license, gateway, output)
    if any(not path.is_absolute() for path in paths):
        fail("package paths must be absolute")
    if output == Path("/") or len(output.parts) < 3:
        fail("output path is too broad")
    if output.exists() or output.is_symlink():
        fail("output path already exists")
    archive_digest, archive_size = digest_file(archive, MAX_ARCHIVE_BYTES)
    if (archive_digest, archive_size) != (lock["runtime"]["sha256"], lock["runtime"]["bytes"]):
        fail("runtime archive does not match the immutable pin")
    model_digest, model_size = digest_file(model, lock["model"]["bytes"])
    if (model_digest, model_size) != (lock["model"]["sha256"], lock["model"]["bytes"]):
        fail("model does not match the immutable pin")
    license_digest, license_size = digest_file(model_license, 1024 * 1024)
    if (license_digest, license_size) != (
        lock["model"]["license"]["sha256"],
        lock["model"]["license"]["bytes"],
    ):
        fail("model license does not match the immutable pin")
    digest_file(gateway, MAX_GATEWAY_BYTES)
    output.mkdir(mode=0o755)
    try:
        runtime_destination = output / str(RUNTIME_ROOT).lstrip("/")
        runtime_destination.mkdir(parents=True, mode=0o755)
        with tarfile.open(archive, "r:gz") as bundle:
            by_name = {member.name: member for member in bundle.getmembers()}
            prefix = f"llama-{lock['runtime']['version']}/"
            for pinned in lock["runtime"]["members"]:
                member = by_name.get(prefix + pinned["source"])
                if member is None or not member.isfile() or member.size != pinned["bytes"]:
                    fail(f"missing or unsafe pinned archive member: {pinned['source']}")
                extracted = bundle.extractfile(member)
                if extracted is None:
                    fail(f"cannot read pinned archive member: {pinned['source']}")
                data = extracted.read(pinned["bytes"] + 1)
                if len(data) != pinned["bytes"] or digest_bytes(data) != pinned["sha256"]:
                    fail(f"pinned archive member digest mismatch: {pinned['source']}")
                for installed in pinned["installs"]:
                    target = runtime_destination / installed
                    target.write_bytes(data)
                    target.chmod(0o755 if installed == "llama-server" else 0o644)
            license_pin = lock["runtime"]["license"]
            license_member = by_name.get(prefix + license_pin["source"])
            if license_member is None or not license_member.isfile():
                fail("missing or unsafe runtime license")
            runtime_license = bundle.extractfile(license_member)
            if runtime_license is None:
                fail("cannot read runtime license")
            runtime_license_bytes = runtime_license.read(license_pin["bytes"] + 1)
            if (
                len(runtime_license_bytes) != license_pin["bytes"]
                or digest_bytes(runtime_license_bytes) != license_pin["sha256"]
            ):
                fail("runtime license digest mismatch")
        copy_exact(model, output / str(MODEL_PATH).lstrip("/"), 0o644)
        license_root = output / "usr/share/licenses/blossom-model-runtime"
        license_root.mkdir(parents=True, exist_ok=True)
        llama_license = license_root / "llama.cpp-LICENSE"
        llama_license.write_bytes(runtime_license_bytes)
        llama_license.chmod(0o644)
        copy_exact(model_license, license_root / "Qwen2.5-0.5B-Instruct-LICENSE", 0o644)
        copy_exact(gateway, output / "usr/lib/blossom-os/blossom-model-gateway", 0o755)
        fixed_files = {
            "usr/lib/systemd/system/blossom-model-netns.service": PACKAGE / "blossom-model-netns.service",
            "usr/lib/sysusers.d/blossom-model-runtime.conf": PACKAGE / "blossom-model-runtime.sysusers",
        }
        for relative, source in fixed_files.items():
            copy_exact(source, output / relative, 0o644)
        gateway_unit_path = output / "usr/lib/systemd/system/blossom-model-gateway.service"
        gateway_unit_path.parent.mkdir(parents=True, exist_ok=True)
        gateway_unit_path.write_bytes(render_gateway_unit())
        gateway_unit_path.chmod(0o644)
        unit_path = output / "usr/lib/systemd/system/blossom-model-llama-cpp.service"
        unit_path.parent.mkdir(parents=True, exist_ok=True)
        unit_path.write_bytes(render_provider_unit())
        unit_path.chmod(0o644)
        profile_path = output / str(PROFILE_PATH).lstrip("/")
        profile_path.parent.mkdir(parents=True, exist_ok=True)
        profile_path.write_bytes(registry_bytes(lock))
        profile_path.chmod(0o644)
        receipt = {
            "schema_version": 1,
            "profile_sha256": digest_bytes(registry_bytes(lock)),
            "gateway_sha256": digest_file(gateway, MAX_GATEWAY_BYTES)[0],
            "source_lock_sha256": digest_file(LOCK_PATH, 1024 * 1024)[0],
            "services_enabled": False,
        }
        receipt_path = output / "usr/share/blossom-os/model-runtime-package.json"
        receipt_path.parent.mkdir(parents=True, exist_ok=True)
        receipt_path.write_bytes(json.dumps(receipt, separators=(",", ":")).encode())
        receipt_path.chmod(0o644)
        for directory in sorted(
            (path for path in output.rglob("*") if path.is_dir()),
            key=lambda path: str(path),
        ):
            directory.chmod(0o755)
        output.chmod(0o755)
    except BaseException:
        shutil.rmtree(output, ignore_errors=True)
        raise


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify-lock", action="store_true")
    parser.add_argument("--emit-registry", action="store_true")
    parser.add_argument("--runtime-archive", type=Path)
    parser.add_argument("--model", type=Path)
    parser.add_argument("--model-license", type=Path)
    parser.add_argument("--gateway-binary", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    lock = load_lock()
    if arguments.verify_lock:
        if any([arguments.runtime_archive, arguments.model, arguments.model_license, arguments.gateway_binary, arguments.output]):
            fail("verification mode accepts no package paths")
        if REGISTRY_PATH.read_bytes().removesuffix(b"\n") != registry_bytes(lock):
            fail("embedded registry drift")
        return
    if arguments.emit_registry:
        if any([arguments.runtime_archive, arguments.model, arguments.model_license, arguments.gateway_binary, arguments.output]):
            fail("registry emission accepts no package paths")
        print(registry_bytes(lock).decode())
        return
    if None in (
        arguments.runtime_archive,
        arguments.model,
        arguments.model_license,
        arguments.gateway_binary,
        arguments.output,
    ):
        fail("all five closed package paths are required")
    build(
        lock,
        arguments.runtime_archive,
        arguments.model,
        arguments.model_license,
        arguments.gateway_binary,
        arguments.output,
    )


if __name__ == "__main__":
    main()
