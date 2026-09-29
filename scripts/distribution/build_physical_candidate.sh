#!/usr/bin/env bash
set -euo pipefail

if (($# != 1)); then
  echo "usage: build_physical_candidate.sh OUTPUT_DIRECTORY" >&2
  exit 2
fi

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
mode=${BLOSSOM_CANDIDATE_MODE:-physical}
case "$mode" in
  physical|vm-qualification) ;;
  *) echo "BLOSSOM_CANDIDATE_MODE must be physical or vm-qualification" >&2; exit 2 ;;
esac
if [[ -e "$repo/distribution/DESKTOP_BUILD_LOCK" && "$mode" != vm-qualification ]]; then
  echo "physical candidate build blocked: Blossom Desktop Foundation runtime gate is active" >&2
  echo "only BLOSSOM_CANDIDATE_MODE=vm-qualification is allowed; do not bypass this lock" >&2
  exit 1
fi
output=$(realpath -m "$1")
rootfs_compressor='zstd -19 -T0'
if [[ "$mode" == vm-qualification ]]; then
  rootfs_compressor='zstd -3 -T0'
fi
case "$output" in
  "$repo"|"$repo"/*|/candidate) ;;
  *) echo "output must be the workspace or the isolated /candidate mount" >&2; exit 2 ;;
esac

build="$output/build"
rootfs="$build/rootfs"
profile="$build/profile"
packages="$build/packages"
iso="$output/iso"
rm -rf "$build" "$iso"
mkdir -p "$rootfs" "$profile" "$packages" "$iso"

useradd -m builder
mkdir -p "$build/source"
cp -a "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/apps" "$repo/core" \
  "$repo/system" "$repo/distribution" "$build/source/"
chown -R builder:builder "$build/source" "$packages"
runuser -u builder -- env HOME=/home/builder \
  cargo fetch --locked --manifest-path "$build/source/Cargo.toml"
runuser -u builder -- env HOME=/home/builder \
  makepkg --nodeps --noconfirm --dir "$build/source/distribution/packages/blossom-core"
runuser -u builder -- env HOME=/home/builder \
  makepkg --nodeps --noconfirm --dir "$build/source/distribution/packages/blossom-shell"
if [[ "$mode" == vm-qualification ]]; then
  runuser -u builder -- env HOME=/home/builder \
    makepkg --nodeps --noconfirm --dir "$build/source/distribution/packages/blossom-qualification"
fi
cp "$build/source"/distribution/packages/blossom-core/blossom-core-*.pkg.tar.zst "$packages/"
cp "$build/source"/distribution/packages/blossom-shell/blossom-shell-*.pkg.tar.zst "$packages/"
if [[ "$mode" == vm-qualification ]]; then
  cp "$build/source"/distribution/packages/blossom-qualification/blossom-qualification-*.pkg.tar.zst "$packages/"
fi

pacstrap -K -C "$repo/distribution/archiso/pacman.conf" "$rootfs" \
  adwaita-cursors base blueman bluez bluez-utils brightnessctl broadcom-wl-dkms bubblewrap dbus-broker dosfstools foot \
  dkms firefox gptfdisk greetd grim gvfs hyprland hyprpolkitagent intel-ucode iputils libnotify linux-firmware linux-lts linux-lts-headers mako mesa mousepad networkmanager \
  network-manager-applet nm-connection-editor noto-fonts noto-fonts-emoji openssh openssl pavucontrol pipewire pipewire-alsa \
  pipewire-pulse polkit python quickshell greetd-regreet qt6-base qt6-declarative qt6-wayland slurp sof-firmware sudo systemd systemd-ukify thunar \
  tumbler upower vulkan-intel wireplumber wpa_supplicant xdg-desktop-portal xdg-desktop-portal-gtk \
  xdg-desktop-portal-hyprland xorg-xwayland zstd
pacman --root "$rootfs" --config /etc/pacman.conf --noconfirm -U "$packages"/*.pkg.tar.zst
qualification_polkit_rule="$rootfs/usr/share/polkit-1/rules.d/49-blossom-model-effect-qualification.rules"
if [[ "$mode" == vm-qualification ]]; then
  [[ -f "$qualification_polkit_rule" ]] || {
    echo "VM qualification root is missing its closed polkit bypass" >&2
    exit 1
  }
elif [[ -e "$qualification_polkit_rule" ]]; then
  echo "physical root must never contain the qualification polkit bypass" >&2
  exit 1
fi

cp -a "$repo/distribution/physical-rootfs/." "$rootfs/"
chmod 0440 "$rootfs/etc/sudoers.d/10-blossom-wheel"
model_root=${BLOSSOM_LLAMA_CPP_RUNTIME_ROOT:-}
if [[ -z "$model_root" || ! -d "$model_root" || -L "$model_root" ]]; then
  echo "candidate requires a closed BLOSSOM_LLAMA_CPP_RUNTIME_ROOT package tree" >&2
  exit 1
fi
model_root=$(realpath "$model_root")
python "$repo/scripts/package_llama_cpp_runtime.py" \
  --verify-package-root "$model_root" \
  --gateway-binary "$rootfs/usr/lib/blossom-os/blossom-model-gateway"
cp -a "$model_root/." "$rootfs/"
install -d -m 0755 "$rootfs/etc/systemd/system/multi-user.target.wants"
ln -sf ../blossom-model-netns.service \
  "$rootfs/etc/systemd/system/multi-user.target.wants/blossom-model-netns.service"
ln -sf ../blossom-model-llama-cpp.service \
  "$rootfs/etc/systemd/system/multi-user.target.wants/blossom-model-llama-cpp.service"
ln -sf ../blossom-model-gateway.service \
  "$rootfs/etc/systemd/system/multi-user.target.wants/blossom-model-gateway.service"
install -d -m 0755 "$rootfs/opt/blossom/scripts/distribution" "$rootfs/etc/blossom-os"
install -m 0644 /dev/null "$rootfs/opt/blossom/scripts/__init__.py"
install -m 0644 /dev/null "$rootfs/opt/blossom/scripts/distribution/__init__.py"
# Release images contain only the runtime modules used by the installer and
# lifecycle service. Tests, workflows, build tools, and repository metadata
# are deliberately excluded.
runtime_modules=(
  blossom_lifecycle.py graphical_install_backend.py installation_profile.py
  physical_device_observer.py physical_install_context.py
  physical_install_backend.py physical_install_guard.py physical_install_harness.py
  physical_preflight.py provision_installed_identity.py
)
for module in "${runtime_modules[@]}"; do
  install -m 0644 "$repo/scripts/distribution/$module" \
    "$rootfs/opt/blossom/scripts/distribution/$module"
done
install -d -m 0755 "$rootfs/usr/share/blossom-os/default-home/.config/hypr"
install -m 0600 "$rootfs/usr/share/blossom-os/physical-home/hyprland.conf" \
  "$rootfs/usr/share/blossom-os/default-home/.config/hypr/hyprland.conf"
install -d -m 0755 "$rootfs/usr/share/blossom-os/default-home/.config/mako" \
  "$rootfs/usr/share/blossom-os/default-home/.config/xdg-desktop-portal"
install -m 0600 "$rootfs/usr/share/blossom-os/physical-home/mako.conf" \
  "$rootfs/usr/share/blossom-os/default-home/.config/mako/config"
install -m 0600 "$rootfs/usr/share/blossom-os/physical-home/hyprland-portals.conf" \
  "$rootfs/usr/share/blossom-os/default-home/.config/xdg-desktop-portal/hyprland-portals.conf"
install -d -m 0755 "$rootfs/etc/systemd/user/graphical-session.target.wants"
install -d -m 0755 "$rootfs/etc/systemd/system/graphical.target.wants"
ln -sf /usr/lib/systemd/user/blossom-shell-ui.service \
  "$rootfs/etc/systemd/user/graphical-session.target.wants/blossom-shell-ui.service"
ln -sf /usr/lib/systemd/system/NetworkManager.service \
  "$rootfs/etc/systemd/system/multi-user.target.wants/NetworkManager.service"
ln -sf /usr/lib/systemd/system/bluetooth.service \
  "$rootfs/etc/systemd/system/dbus-org.bluez.service"
ln -sf /usr/lib/systemd/system/blossom-privileged-helper.service \
  "$rootfs/etc/systemd/system/multi-user.target.wants/blossom-privileged-helper.service"
ln -sf /usr/lib/systemd/system/greetd.service \
  "$rootfs/etc/systemd/system/display-manager.service"
ln -sf /usr/lib/systemd/system/greetd.service \
  "$rootfs/etc/systemd/system/graphical.target.wants/greetd.service"
ln -sf /usr/lib/systemd/system/graphical.target "$rootfs/etc/systemd/system/default.target"
if [[ "$mode" == vm-qualification ]]; then
  # The disposable installed-image probe deliberately runs the repository's
  # closed distribution check and its Phase 9 regression suite.  Keep those
  # verifier inputs out of release images, but package the reviewed copies in
  # VM-qualification roots so the probe cannot pass against missing or host
  # files.
  install -d -m 0755 \
    "$rootfs/opt/blossom/.github/workflows" \
    "$rootfs/opt/blossom/scripts"
  cp -a "$repo/scripts/ci" "$rootfs/opt/blossom/scripts/"
  cp -a "$repo/tests" "$repo/distribution" "$rootfs/opt/blossom/"
  install -m 0644 "$repo/.github/workflows/phase9-vm-install-evidence.yml" \
    "$rootfs/opt/blossom/.github/workflows/phase9-vm-install-evidence.yml"
  install -Dm0755 "$repo/distribution/evidence/blossom-evidence-boot" \
    "$rootfs/usr/local/bin/blossom-evidence-boot"
  install -Dm0644 "$repo/distribution/evidence/blossom-evidence-boot.service" \
    "$rootfs/etc/systemd/system/blossom-evidence-boot.service"
  ln -sf ../blossom-evidence-boot.service \
    "$rootfs/etc/systemd/system/multi-user.target.wants/blossom-evidence-boot.service"
  install -Dm0644 "$repo/distribution/evidence/install-marker.json" \
    "$rootfs/etc/blossom-os/install.json"
fi
sed -i -E 's/^#?Storage=.*/Storage=persistent/' "$rootfs/etc/systemd/journald.conf"
install -d -m 2755 "$rootfs/var/log/journal"
# Published rootfs archives must never contain a build-machine identity.
install -d -m 0755 "$rootfs/etc"
: > "$rootfs/etc/machine-id"
chmod 0444 "$rootfs/etc/machine-id"
if [[ -s "$rootfs/etc/machine-id" ]]; then
  echo "root filesystem machine-id must be empty" >&2
  exit 1
fi
rootfs_manifest="$build/blossom-rootfs.manifest.json"
python - "$rootfs" "$rootfs_manifest" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
output = Path(sys.argv[2])
required = [
    "etc/machine-id",
    "etc/passwd",
    "etc/shadow",
    "etc/group",
    "etc/gshadow",
    "boot/vmlinuz-linux-lts",
    "boot/initramfs-linux-lts.img",
    "boot/intel-ucode.img",
    "usr/bin/locale-gen",
]
for relative in required:
    path = root / relative
    if not (path.is_file() or path.is_symlink()):
        raise SystemExit(f"root filesystem prerequisite has invalid type: {relative}")
if (root / "etc/machine-id").stat().st_size != 0:
    raise SystemExit("root filesystem machine-id must be empty")
output.write_text(json.dumps({
    "schema": 1,
    "required_paths": required,
    "machine_id_bytes": 0,
}, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
PY
tar --xattrs --numeric-owner -I "$rootfs_compressor" \
  -cf "$build/blossom-rootfs.tar.zst" -C "$rootfs" .
(
  cd "$build"
  sha256sum blossom-rootfs.tar.zst blossom-rootfs.manifest.json \
    > blossom-rootfs.tar.zst.sha256
)

cp -a /usr/share/archiso/configs/releng/. "$profile/"
cp "$repo/distribution/archiso/profiledef.sh" "$profile/profiledef.sh"
cp "$repo/distribution/archiso/pacman.conf" "$profile/pacman.conf"
cp "$repo/distribution/archiso/packages.x86_64" "$profile/packages.x86_64"
cp -a "$repo/distribution/archiso/airootfs/." "$profile/airootfs/"
install -d -m 0755 "$profile/airootfs/etc/systemd/system/multi-user.target.wants"
ln -sf /usr/lib/systemd/system/NetworkManager.service \
  "$profile/airootfs/etc/systemd/system/multi-user.target.wants/NetworkManager.service"
# The live environment must contain the same reviewed Blossom packages as the
# installable rootfs. Package metadata is pacman-only and is not part of the
# live filesystem image.
for package in "$packages"/blossom-core-*.pkg.tar.zst "$packages"/blossom-shell-*.pkg.tar.zst; do
  bsdtar -xpf "$package" -C "$profile/airootfs" \
    --exclude .BUILDINFO --exclude .MTREE --exclude .PKGINFO
done
# Git tracks the helper source as data, while the package installs it as a
# command.  Reassert the live-root mode after overlay extraction so a later
# profile overlay or archive implementation cannot silently remove execute
# permission from the screenshot binding.
chmod 0755 "$profile/airootfs/usr/local/bin/blossom-screenshot"
uefi_entries=("$profile/efiboot/loader/entries/"*.conf)
if ((${#uefi_entries[@]} == 0)); then
  echo "ArchISO profile has no UEFI boot entries" >&2
  exit 1
fi
sed -i 's/^title .*/title Blossom OS Live/' "${uefi_entries[0]}"
if ((${#uefi_entries[@]} > 1)); then
  sed -i 's/^title .*/title Blossom OS Live (accessible speech)/' "${uefi_entries[1]}"
fi
recovery_entry="$profile/efiboot/loader/entries/90-blossom-recovery.conf"
cp "${uefi_entries[0]}" "$recovery_entry"
sed -i 's/^title .*/title Blossom OS Recovery Console/' "$recovery_entry"
sed -i '/^options /s/$/ blossom.recovery=1/' "$recovery_entry"
sed -i '/^options /s/$/ vt.global_cursor_default=0 quiet loglevel=3 rd.udev.log_level=3/' \
  "${uefi_entries[@]}"
# The recovery entry is intentionally verbose and must not inherit the quiet
# normal-boot presentation.
sed -i -E 's/ quiet( |$)/ /g; s/ loglevel=3( |$)/ loglevel=7 /g; s/ rd\.udev\.log_level=3( |$)/ rd.udev.log_level=7 /g' \
  "$recovery_entry"
sed -i -E 's/^timeout .*/timeout 5/' "$profile/efiboot/loader/loader.conf"
sed -i -E 's/ archiso_pxe_(common|nbd|http|nfs)//g' \
  "$profile/airootfs/etc/mkinitcpio.conf.d/archiso.conf"
sed -i 's/iso_application=.*/iso_application="Blossom OS physical qualification candidate"/' \
  "$profile/profiledef.sh"
sed -i 's/iso_version=.*/iso_version="0.11.0-physical-candidate"/' "$profile/profiledef.sh"
rm -f "$profile/airootfs/etc/systemd/system/multi-user.target.wants/blossom-evidence-install.service"
rm -f "$profile/airootfs/etc/systemd/system/blossom-evidence-install.service"
install -d -m 0755 "$profile/airootfs/opt/blossom/scripts/distribution"
install -m 0644 /dev/null "$profile/airootfs/opt/blossom/scripts/__init__.py"
install -m 0644 /dev/null "$profile/airootfs/opt/blossom/scripts/distribution/__init__.py"
for module in "${runtime_modules[@]}"; do
  install -m 0644 "$repo/scripts/distribution/$module" \
    "$profile/airootfs/opt/blossom/scripts/distribution/$module"
done
if [[ "$mode" == vm-qualification ]]; then
  sed -i 's/iso_application=.*/iso_application="Blossom OS generic VM qualification candidate"/' \
    "$profile/profiledef.sh"
  sed -i 's/iso_version=.*/iso_version="0.11.0-vm-qualification"/' "$profile/profiledef.sh"
  sed -i 's/-Xcompression-level 15/-Xcompression-level 3/' "$profile/profiledef.sh"
  sed -i '/^options /s/$/ console=ttyS0,115200n8 systemd.show_status=yes/' \
    "$profile/efiboot/loader/entries/"*.conf
  install -Dm0755 "$repo/distribution/archiso/airootfs/usr/local/bin/blossom-evidence-install" \
    "$profile/airootfs/usr/local/bin/blossom-evidence-install"
  install -Dm0644 "$repo/distribution/evidence/blossom-evidence-install.service" \
    "$profile/airootfs/etc/systemd/system/blossom-evidence-install.service"
  install -d -m 0755 "$profile/airootfs/etc/systemd/system/multi-user.target.wants"
  ln -sf ../blossom-evidence-install.service \
    "$profile/airootfs/etc/systemd/system/multi-user.target.wants/blossom-evidence-install.service"
fi
install -Dm0600 "$build/blossom-rootfs.tar.zst" \
  "$profile/airootfs/root/blossom-rootfs.tar.zst"
install -Dm0600 "$build/blossom-rootfs.manifest.json" \
  "$profile/airootfs/root/blossom-rootfs.manifest.json"
install -Dm0600 "$build/blossom-rootfs.tar.zst.sha256" \
  "$profile/airootfs/root/blossom-rootfs.tar.zst.sha256"
mkarchiso -v -w "$build/archiso-work" -o "$iso" "$profile"
(
  cd "$iso"
  sha256sum blossom-os-*.iso > SHA256SUMS
)
