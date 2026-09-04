#!/usr/bin/env python3
"""Build the one closed Ollama CPU package tree from pre-fetched inputs."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
LOCK_PATH = ROOT / "system/model-runtime/registry/ollama-cpu-x86_64.lock.json"
REGISTRY_PATH = ROOT / "system/model-runtime/registry/ollama-cpu-x86_64.profile.json"
PACKAGE = ROOT / "system/model-runtime/packaging"
RUNTIME_ROOT = PurePosixPath("/usr/lib/blossom-os/providers/ollama")
MODEL_ROOT = PurePosixPath("/usr/lib/blossom-os/models/ollama")
PROFILE_PATH = PurePosixPath("/etc/blossom-os/model-profiles/active.json")
LOGICAL_MODEL = "qwen2.5:0.5b-instruct-q4_K_M"
MAX_ARCHIVE_BYTES = 2 * 1024 * 1024 * 1024
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
    if (lock["schema_version"], lock["profile"], lock["architecture"]) != (1, "ollama_cpu_v1", "x86_64"):
        fail("lock identity drift")
    runtime, model = lock["runtime"], lock["model"]
    if set(runtime) != {"project", "version", "archive", "url", "sha256", "bytes", "license", "members"}:
        fail("runtime lock schema drift")
    if set(model) != {"project", "tag", "manifest", "blobs"}:
        fail("model lock schema drift")
    if runtime["project"] != "ollama/ollama" or runtime["version"] != "v0.33.3":
        fail("runtime pin drift")
    if model["project"] != "library/qwen2.5" or model["tag"] != "0.5b-instruct-q4_K_M":
        fail("model pin drift")
    if runtime["version"] not in runtime["url"] or runtime["version"] not in runtime["license"]["url"]:
        fail("mutable runtime URL")
    if runtime["license"]["spdx"] != "MIT":
        fail("runtime license pin drift")
    seen_sources: set[str] = set()
    seen_installs: set[str] = set()
    for member in runtime["members"]:
        if set(member) != {"source", "installs", "sha256", "bytes"}:
            fail("runtime member schema drift")
        source = PurePosixPath(member["source"])
        if source.is_absolute() or ".." in source.parts or member["source"] in seen_sources:
            fail("unsafe or duplicate runtime source")
        seen_sources.add(member["source"])
        for installed in member["installs"]:
            target = PurePosixPath(installed)
            if target.is_absolute() or ".." in target.parts or installed in seen_installs:
                fail("unsafe or duplicate runtime install path")
            seen_installs.add(installed)
        validate_pin(member)
    if not {"ollama", "llama-server", "libggml-cpu-x64.so"} <= seen_installs:
        fail("runtime does not bind the fixed CPU executable set")
    validate_pin(runtime)
    validate_pin(runtime["license"])
    validate_pin(model["manifest"])
    if len(model["blobs"]) != 5:
        fail("model blob set drift")
    digests: set[str] = set()
    media_types: set[str] = set()
    for blob in model["blobs"]:
        if set(blob) != {"media_type", "url", "sha256", "bytes"}:
            fail("model blob schema drift")
        validate_pin(blob)
        if not blob["url"].endswith(f"sha256:{blob['sha256']}"):
            fail("mutable model blob URL")
        if blob["sha256"] in digests or blob["media_type"] in media_types:
            fail("duplicate model blob")
        digests.add(blob["sha256"])
        media_types.add(blob["media_type"])
    required = {
        "application/vnd.docker.container.image.v1+json",
        "application/vnd.ollama.image.model",
        "application/vnd.ollama.image.system",
        "application/vnd.ollama.image.template",
        "application/vnd.ollama.image.license",
    }
    if media_types != required:
        fail("model media-type set drift")
    return lock


def validate_pin(item: dict) -> None:
    digest, size = item.get("sha256"), item.get("bytes")
    if not isinstance(digest, str) or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        fail("invalid pinned digest")
    if not isinstance(size, int) or size <= 0:
        fail("invalid pinned size")


def artifact(path: PurePosixPath, sha256: str, size: int) -> dict:
    return {"path": str(path), "sha256": sha256, "bytes": size}


def artifact_set_digest(files: list[dict]) -> str:
    return digest_bytes(json.dumps(files, separators=(",", ":")).encode())


def render_provider_unit() -> bytes:
    template = (PACKAGE / "blossom-model-ollama.service.in").read_text(encoding="utf-8")
    replacements = {
        "@PROVIDER_BINARY@": str(RUNTIME_ROOT / "ollama"),
        "@PROVIDER_DIRECTORY@": str(RUNTIME_ROOT),
        "@MODEL_PATH@": str(MODEL_ROOT),
        "@MODEL_DIRECTORY@": str(MODEL_ROOT),
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
        "@PROFILE_ROOT@": str(PROFILE_PATH.parents[1]),
        "@PROVIDER_DIRECTORY@": str(RUNTIME_ROOT),
        "@MODEL_PATH@": str(MODEL_ROOT),
    }
    for token, value in replacements.items():
        if token not in template:
            fail(f"gateway template token drift: {token}")
        template = template.replace(token, value)
    if "@" in template.replace("@system-service", ""):
        fail("unresolved gateway template token")
    return template.encode()


def model_inventory(lock: dict) -> list[dict]:
    files = []
    manifest_path = MODEL_ROOT / "manifests/registry.ollama.ai/library/qwen2.5" / lock["model"]["tag"]
    manifest = lock["model"]["manifest"]
    files.append(artifact(manifest_path, manifest["sha256"], manifest["bytes"]))
    for blob in lock["model"]["blobs"]:
        files.append(artifact(MODEL_ROOT / "blobs" / f"sha256-{blob['sha256']}", blob["sha256"], blob["bytes"]))
    return sorted(files, key=lambda item: item["path"])


def registry_bytes(lock: dict) -> bytes:
    runtime_files = []
    for member in lock["runtime"]["members"]:
        for installed in member["installs"]:
            runtime_files.append(artifact(RUNTIME_ROOT / installed, member["sha256"], member["bytes"]))
    runtime_files.sort(key=lambda item: item["path"])
    model_files = model_inventory(lock)
    binary = next(item for item in runtime_files if item["path"] == str(RUNTIME_ROOT / "ollama"))
    manifest = {
        "profile_version": 5,
        "profile": "ollama_cpu_v1",
        "provider": "ollama",
        "logical_model": LOGICAL_MODEL,
        "gateway_protocol_version": 1,
        "model_protocol_version": 1,
        "binary": binary,
        "runtime_mount": str(RUNTIME_ROOT),
        "runtime_files": runtime_files,
        "runtime_set_sha256": artifact_set_digest(runtime_files),
        "model_mount": str(MODEL_ROOT),
        "model_files": model_files,
        "model_set_sha256": artifact_set_digest(model_files),
        "unit_sha256": digest_bytes(render_provider_unit()),
        "executable_arguments": [str(RUNTIME_ROOT / "ollama"), "serve"],
        "environment_names": [
            "HOME",
            "OLLAMA_HOST",
            "OLLAMA_LLM_LIBRARY",
            "OLLAMA_MAX_LOADED_MODELS",
            "OLLAMA_MAX_QUEUE",
            "OLLAMA_MODELS",
            "OLLAMA_NOPRUNE",
            "OLLAMA_NO_CLOUD",
            "OLLAMA_NUM_PARALLEL",
        ],
        "endpoint": "127.0.0.1:11434",
        "inference_path": "/api/chat",
        "filesystem": {
            "read_only_paths": [str(RUNTIME_ROOT), str(MODEL_ROOT)],
            "writable_paths": ["/var/lib/blossom/model-provider/ollama"],
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
            "provider_unit": "blossom-model-ollama.service",
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


def extract_exact(archive: Path, member: dict) -> bytes:
    result = subprocess.run(
        ["tar", "--extract", "--to-stdout", "--file", str(archive), member["source"]],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        fail(f"cannot read pinned archive member: {member['source']}")
    if len(result.stdout) != member["bytes"] or digest_bytes(result.stdout) != member["sha256"]:
        fail(f"pinned archive member digest mismatch: {member['source']}")
    return result.stdout


def verify_manifest(lock: dict, path: Path) -> None:
    raw = path.read_bytes()
    parsed = json.loads(raw)
    expected = {blob["media_type"]: (f"sha256:{blob['sha256']}", blob["bytes"]) for blob in lock["model"]["blobs"]}
    actual = {}
    config = parsed.get("config", {})
    actual[config.get("mediaType")] = (config.get("digest"), config.get("size"))
    for layer in parsed.get("layers", []):
        actual[layer.get("mediaType")] = (layer.get("digest"), layer.get("size"))
    if set(parsed) != {"schemaVersion", "mediaType", "config", "layers"} or parsed["schemaVersion"] != 2 or actual != expected:
        fail("registry manifest content drift")


def build(lock: dict, archive: Path, runtime_license: Path, manifest: Path, blob_directory: Path, gateway: Path, output: Path) -> None:
    paths = (archive, runtime_license, manifest, blob_directory, gateway, output)
    if any(not path.is_absolute() for path in paths) or output == Path("/") or len(output.parts) < 3:
        fail("package paths must be absolute and narrowly scoped")
    if output.exists() or output.is_symlink():
        fail("output path already exists")
    if digest_file(archive, MAX_ARCHIVE_BYTES) != (lock["runtime"]["sha256"], lock["runtime"]["bytes"]):
        fail("runtime archive does not match the immutable pin")
    if digest_file(runtime_license, 1024 * 1024) != (lock["runtime"]["license"]["sha256"], lock["runtime"]["license"]["bytes"]):
        fail("runtime license does not match the immutable pin")
    if digest_file(manifest, 1024 * 1024) != (lock["model"]["manifest"]["sha256"], lock["model"]["manifest"]["bytes"]):
        fail("model manifest does not match the immutable pin")
    if blob_directory.is_symlink() or not blob_directory.is_dir():
        fail("blob directory is not a regular directory")
    expected_names = {blob["sha256"] for blob in lock["model"]["blobs"]}
    if {path.name for path in blob_directory.iterdir()} != expected_names:
        fail("model blob directory does not exactly match the closed set")
    for blob in lock["model"]["blobs"]:
        if digest_file(blob_directory / blob["sha256"], blob["bytes"]) != (blob["sha256"], blob["bytes"]):
            fail("model blob does not match the immutable pin")
    verify_manifest(lock, manifest)
    digest_file(gateway, MAX_GATEWAY_BYTES)
    output.mkdir(mode=0o755)
    try:
        runtime_destination = output / str(RUNTIME_ROOT).lstrip("/")
        for member in lock["runtime"]["members"]:
            data = extract_exact(archive, member)
            for installed in member["installs"]:
                target = runtime_destination / installed
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                target.chmod(0o755 if installed in {"ollama", "llama-server"} else 0o644)
        store = output / str(MODEL_ROOT).lstrip("/")
        copy_exact(manifest, store / "manifests/registry.ollama.ai/library/qwen2.5" / lock["model"]["tag"], 0o644)
        for blob in lock["model"]["blobs"]:
            copy_exact(blob_directory / blob["sha256"], store / "blobs" / f"sha256-{blob['sha256']}", 0o644)
        license_root = output / "usr/share/licenses/blossom-model-runtime"
        copy_exact(runtime_license, license_root / "Ollama-LICENSE", 0o644)
        model_license = next(
            blob
            for blob in lock["model"]["blobs"]
            if blob["media_type"] == "application/vnd.ollama.image.license"
        )
        copy_exact(
            blob_directory / model_license["sha256"],
            license_root / "Qwen2.5-0.5B-Instruct-LICENSE",
            0o644,
        )
        copy_exact(gateway, output / "usr/lib/blossom-os/blossom-model-gateway", 0o755)
        for relative, source in {
            "usr/lib/systemd/system/blossom-model-netns.service": PACKAGE / "blossom-model-netns.service",
            "usr/lib/sysusers.d/blossom-model-runtime.conf": PACKAGE / "blossom-model-runtime.sysusers",
        }.items():
            copy_exact(source, output / relative, 0o644)
        gateway_unit = output / "usr/lib/systemd/system/blossom-model-gateway.service"
        gateway_unit.parent.mkdir(parents=True, exist_ok=True)
        gateway_unit.write_bytes(render_gateway_unit())
        gateway_unit.chmod(0o644)
        provider_unit = output / "usr/lib/systemd/system/blossom-model-ollama.service"
        provider_unit.write_bytes(render_provider_unit())
        provider_unit.chmod(0o644)
        profile = output / str(PROFILE_PATH).lstrip("/")
        profile.parent.mkdir(parents=True, exist_ok=True)
        profile.write_bytes(registry_bytes(lock))
        profile.chmod(0o644)
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
        for directory in sorted((path for path in output.rglob("*") if path.is_dir()), key=str):
            directory.chmod(0o755)
    except BaseException:
        shutil.rmtree(output, ignore_errors=True)
        raise


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify-lock", action="store_true")
    parser.add_argument("--emit-registry", action="store_true")
    parser.add_argument("--refresh-registry", action="store_true")
    parser.add_argument("--runtime-archive", type=Path)
    parser.add_argument("--runtime-license", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--blob-directory", type=Path)
    parser.add_argument("--gateway-binary", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    if sum((arguments.verify_lock, arguments.emit_registry, arguments.refresh_registry)) > 1:
        fail("select exactly one registry operation")
    lock = load_lock()
    package_arguments = [arguments.runtime_archive, arguments.runtime_license, arguments.manifest, arguments.blob_directory, arguments.gateway_binary, arguments.output]
    if arguments.verify_lock or arguments.emit_registry or arguments.refresh_registry:
        if any(package_arguments):
            fail("registry operation accepts no package paths")
        if arguments.verify_lock:
            if REGISTRY_PATH.read_bytes().removesuffix(b"\n") != registry_bytes(lock):
                fail("embedded registry drift")
        elif arguments.emit_registry:
            print(registry_bytes(lock).decode())
        else:
            REGISTRY_PATH.write_bytes(registry_bytes(lock) + b"\n")
        return
    if None in package_arguments:
        fail("all six closed package paths are required")
    build(lock, *package_arguments)


if __name__ == "__main__":
    main()
