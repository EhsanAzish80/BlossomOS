#!/usr/bin/env bash
set -euo pipefail

if (($# != 3)); then
  echo "usage: build_arm64_development_image.sh ROOTFS_TARBALL OUTPUT_RAW WORK_DIRECTORY" >&2
  exit 2
fi

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
rootfs_tarball=$(realpath "$1")
output_raw=$(realpath -m "$2")
work=$(realpath -m "$3")

if [[ $(uname -m) != aarch64 ]]; then
  echo "error: ARM64 development image must be assembled on an aarch64 Linux builder" >&2
  exit 2
fi
case "$output_raw" in
  /output/*) ;;
  *) echo "error: output image must be under /output" >&2; exit 2 ;;
esac
case "$work" in
  /output/*) ;;
  *) echo "error: work directory must be under /output" >&2; exit 2 ;;
esac

for command in bsdtar chroot losetup mkfs.ext4 mkfs.fat sgdisk; do
  command -v "$command" >/dev/null || { echo "error: missing builder command: $command" >&2; exit 2; }
done
for input in BLOSSOM_IMAGE_COMMIT BLOSSOM_LLAMA_RUNTIME_ARCHIVE BLOSSOM_LLAMA_MODEL BLOSSOM_LLAMA_MODEL_LICENSE; do
  [[ -n ${!input:-} ]] || { echo "error: missing image input: $input" >&2; exit 2; }
done
[[ $BLOSSOM_IMAGE_COMMIT =~ ^[0-9a-f]{40}$ ]] || {
  echo "error: invalid image source commit" >&2
  exit 2
}
for input in "$BLOSSOM_LLAMA_RUNTIME_ARCHIVE" "$BLOSSOM_LLAMA_MODEL" "$BLOSSOM_LLAMA_MODEL_LICENSE"; do
  [[ -f $input ]] || { echo "error: missing image input file: $input" >&2; exit 2; }
done

rm -rf "$work"
mkdir -p "$work/root" "$work/source" "$(dirname "$output_raw")"
rm -f "$output_raw"
truncate -s 20G "$output_raw"
sgdisk --clear \
  --new=1:2048:+512M --typecode=1:ef00 --change-name=1:BLOSSOM_EFI \
  --new=2:0:0 --typecode=2:8300 --change-name=2:BLOSSOM_ARM64 \
  "$output_raw"

sector_size=512
disk_bytes=$((20 * 1024 * 1024 * 1024))
efi_start=2048
efi_bytes=$((512 * 1024 * 1024))
root_start=$((efi_start + efi_bytes / sector_size))
root_bytes=$((disk_bytes - root_start * sector_size - 33 * sector_size))
efi_loop=$(losetup --find --show --offset $((efi_start * sector_size)) --sizelimit "$efi_bytes" "$output_raw")
root_loop=$(losetup --find --show --offset $((root_start * sector_size)) --sizelimit "$root_bytes" "$output_raw")
mounted=false
cleanup() {
  local status=$?
  set +e
  if [[ "$mounted" == true ]]; then
    umount "$work/root/var/cache/pacman/pkg" 2>/dev/null || true
    umount -R -l "$work/root/run" 2>/dev/null || true
    umount -R -l "$work/root/dev" 2>/dev/null || true
    umount -R -l "$work/root/proc" 2>/dev/null || true
    umount -R -l "$work/root/sys" 2>/dev/null || true
    umount -R -l "$work/root/boot" 2>/dev/null || true
    umount -R -l "$work/root" 2>/dev/null || true
  fi
  losetup -d "$efi_loop" 2>/dev/null || true
  losetup -d "$root_loop" 2>/dev/null || true
  exit "$status"
}
trap cleanup EXIT INT TERM

mkfs.fat -F 32 -n BLOSSOM_EFI "$efi_loop"
mkfs.ext4 -F -L BLOSSOM_ARM64 "$root_loop"
mount "$root_loop" "$work/root"
mkdir -p "$work/root/boot"
mount "$efi_loop" "$work/root/boot"
mounted=true

bsdtar -xpf "$rootfs_tarball" -C "$work/root"
# The upstream rootfs enables only the GeoIP endpoint by default.  Keep that
# preferred endpoint, but enable the official fallback servers already shipped
# in the signed rootfs mirror list so one slow redirect cannot abort a local
# image build.
mirrorlist="$work/root/etc/pacman.d/mirrorlist"
[[ -f $mirrorlist ]] || { echo "error: ARM64 rootfs has no pacman mirror list" >&2; exit 1; }
sed -E -i 's/^[[:space:]]*#[[:space:]]*(Server[[:space:]]*=)/\1/' "$mirrorlist"
(( $(grep -Ec '^[[:space:]]*Server[[:space:]]*=' "$mirrorlist") >= 2 )) || {
  echo "error: ARM64 rootfs does not provide a pacman mirror fallback" >&2
  exit 1
}
rm -f "$work/root/etc/resolv.conf"
cp -L /etc/resolv.conf "$work/root/etc/resolv.conf"
mount -t proc proc "$work/root/proc"
mount --rbind /sys "$work/root/sys"
mount --make-rslave "$work/root/sys"
mount --rbind /dev "$work/root/dev"
mount --make-rslave "$work/root/dev"
mount --rbind /run "$work/root/run"
mount --make-rslave "$work/root/run"
mkdir -p /output/cache/pacman/pkg "$work/root/var/cache/pacman/pkg"
mount --bind /output/cache/pacman/pkg "$work/root/var/cache/pacman/pkg"
mkdir -p "$work/root/opt/blossom-inputs"
mount --bind /output/cache "$work/root/opt/blossom-inputs"

chroot "$work/root" pacman-key --init
chroot "$work/root" pacman-key --populate archlinuxarm
mapfile -t packages < <(sed -E '/^[[:space:]]*(#|$)/d' "$repo/distribution/arm64-dev/packages.aarch64")
chroot "$work/root" pacman -Syu --noconfirm --needed "${packages[@]}"

tar -C "$repo" \
  --exclude=.git --exclude='.local-*' --exclude=.worktrees \
  -cf - Cargo.toml Cargo.lock apps core distribution/packages scripts system |
  tar -C "$work/source" -xf -
cp -a "$work/source/." "$work/root/opt/blossom-source/"

chroot "$work/root" useradd --create-home --shell /bin/bash builder
chroot "$work/root" chown -R builder:builder /opt/blossom-source
install -d "$work/root/opt/blossom-packages"
chroot "$work/root" chown builder:builder /opt/blossom-packages
chroot "$work/root" runuser -u builder -- env HOME=/home/builder \
  cargo fetch --locked --manifest-path /opt/blossom-source/Cargo.toml
chroot "$work/root" runuser -u builder -- env HOME=/home/builder PKGDEST=/opt/blossom-packages \
  makepkg --nodeps --noconfirm --dir /opt/blossom-source/distribution/packages/blossom-core
chroot "$work/root" runuser -u builder -- env HOME=/home/builder PKGDEST=/opt/blossom-packages \
  makepkg --nodeps --noconfirm --dir /opt/blossom-source/distribution/packages/blossom-shell
chroot "$work/root" runuser -u builder -- env HOME=/home/builder PKGDEST=/opt/blossom-packages \
  makepkg --nodeps --noconfirm --dir /opt/blossom-source/distribution/packages/blossom-qualification
chroot "$work/root" runuser -u builder -- env \
  HOME=/home/builder PKGDEST=/opt/blossom-packages \
  BLOSSOM_REPO_ROOT=/opt/blossom-source \
  BLOSSOM_LLAMA_RUNTIME_ARCHIVE="/opt/blossom-inputs/$(basename "$BLOSSOM_LLAMA_RUNTIME_ARCHIVE")" \
  BLOSSOM_LLAMA_MODEL="/opt/blossom-inputs/$(basename "$BLOSSOM_LLAMA_MODEL")" \
  BLOSSOM_LLAMA_MODEL_LICENSE="/opt/blossom-inputs/$(basename "$BLOSSOM_LLAMA_MODEL_LICENSE")" \
  BLOSSOM_MODEL_GATEWAY=/opt/blossom-source/distribution/packages/blossom-core/src/target/release/blossom-model-gateway \
  makepkg --nodeps --noconfirm --dir /opt/blossom-source/distribution/packages/blossom-model-runtime
chroot "$work/root" /bin/bash -lc '
  packages=(
    /opt/blossom-packages/blossom-core-[0-9]*-aarch64.pkg.tar.*
    /opt/blossom-packages/blossom-shell-[0-9]*-aarch64.pkg.tar.*
    /opt/blossom-packages/blossom-model-runtime-[0-9]*-aarch64.pkg.tar.*
    /opt/blossom-packages/blossom-qualification-[0-9]*-aarch64.pkg.tar.*
  )
  ((${#packages[@]} == 4)) || {
    echo "error: expected exactly four Blossom package artifacts" >&2
    exit 1
  }
  pacman --noconfirm -U "${packages[@]}"
'
chroot "$work/root" python3 /opt/blossom-source/scripts/package_llama_cpp_runtime.py \
  --architecture aarch64 \
  --verify-installed-root / \
  --gateway-binary /usr/lib/blossom-os/blossom-model-gateway

cp -a "$repo/distribution/physical-rootfs/." "$work/root/"
chmod 0440 "$work/root/etc/sudoers.d/10-blossom-wheel"
rm -rf "$work/root/home/alarm"
chroot "$work/root" userdel alarm 2>/dev/null || true
chroot "$work/root" userdel --remove builder
chroot "$work/root" groupadd --force blossom-ai
chroot "$work/root" useradd --uid 1000 --create-home --groups audio,input,video,wheel,blossom-ai --shell /bin/bash blossom
cp -a "$repo/distribution/archiso/airootfs/home/blossom/." "$work/root/home/blossom/"
chroot "$work/root" chown -R blossom:blossom /home/blossom
chmod 0700 "$work/root/home/blossom"

install -d -m 0755 \
  "$work/root/etc/systemd/system/multi-user.target.wants" \
  "$work/root/etc/systemd/system/graphical.target.wants" \
  "$work/root/etc/systemd/user/graphical-session.target.wants"
ln -sf /usr/lib/systemd/system/NetworkManager.service \
  "$work/root/etc/systemd/system/multi-user.target.wants/NetworkManager.service"
install -d -m 0755 "$work/root/etc/NetworkManager/conf.d"
cat >"$work/root/etc/NetworkManager/conf.d/10-blossom-dns.conf" <<'EOF'
[main]
dns=default
rc-manager=file
EOF
ln -sf /dev/null "$work/root/etc/systemd/system/systemd-resolved.service"
rm -f "$work/root/etc/resolv.conf"
printf 'nameserver 10.0.2.3\n' >"$work/root/etc/resolv.conf"
ln -sf /usr/lib/systemd/system/qemu-guest-agent.service \
  "$work/root/etc/systemd/system/multi-user.target.wants/qemu-guest-agent.service"
ln -sf /usr/lib/systemd/system/bluetooth.service \
  "$work/root/etc/systemd/system/dbus-org.bluez.service"
ln -sf /usr/lib/systemd/system/blossom-privileged-helper.service \
  "$work/root/etc/systemd/system/multi-user.target.wants/blossom-privileged-helper.service"
ln -sf /usr/lib/systemd/system/blossom-model-netns.service \
  "$work/root/etc/systemd/system/multi-user.target.wants/blossom-model-netns.service"
ln -sf /usr/lib/systemd/system/blossom-model-llama-cpp.service \
  "$work/root/etc/systemd/system/multi-user.target.wants/blossom-model-llama-cpp.service"
ln -sf /usr/lib/systemd/system/blossom-model-gateway.service \
  "$work/root/etc/systemd/system/multi-user.target.wants/blossom-model-gateway.service"
ln -sf /usr/lib/systemd/system/greetd.service \
  "$work/root/etc/systemd/system/graphical.target.wants/greetd.service"
ln -sf /usr/lib/systemd/system/blossom-arm64-qualification.service \
  "$work/root/etc/systemd/system/graphical.target.wants/blossom-arm64-qualification.service"
ln -sf /usr/lib/systemd/system/graphical.target "$work/root/etc/systemd/system/default.target"
ln -sf /usr/lib/systemd/user/blossom-shell-ui.service \
  "$work/root/etc/systemd/user/graphical-session.target.wants/blossom-shell-ui.service"

cat >"$work/root/etc/greetd/config.toml" <<'EOF'
[terminal]
vt = 1

[initial_session]
command = "start-hyprland"
user = "blossom"

[default_session]
command = "Hyprland --config /etc/greetd/hyprland.conf"
user = "greeter"
EOF

cat >"$work/root/etc/mkinitcpio.conf.d/blossom-arm64.conf" <<'EOF'
MODULES=(virtio_pci virtio_blk virtio_gpu virtio_input virtio_net virtio_console)
HOOKS=(base udev autodetect modconf kms keyboard keymap consolefont block filesystems fsck)
EOF
chroot "$work/root" mkinitcpio -P

root_uuid=$(blkid -s UUID -o value "$root_loop")
install -d -m 0755 "$work/root/boot/EFI/BOOT" "$work/root/boot/loader/entries"
cp "$work/root/usr/lib/systemd/boot/efi/systemd-bootaa64.efi" \
  "$work/root/boot/EFI/BOOT/BOOTAA64.EFI"
cat >"$work/root/boot/loader/loader.conf" <<'EOF'
default blossom-arm64.conf
timeout 1
console-mode keep
editor no
EOF
cat >"$work/root/boot/loader/entries/blossom-arm64.conf" <<EOF
title Blossom OS ARM64 Development
linux /Image
initrd /initramfs-linux.img
options root=UUID=$root_uuid rw quiet loglevel=3 console=tty1 console=ttyAMA0,115200
EOF
cat >"$work/root/etc/fstab" <<EOF
UUID=$root_uuid / ext4 defaults,noatime 0 1
LABEL=BLOSSOM_EFI /boot vfat umask=0077 0 2
EOF
printf 'blossom-arm64-dev\n' >"$work/root/etc/hostname"
printf 'Blossom OS ARM64 Development Image\n' >"$work/root/etc/issue"
printf '%s\n' "$BLOSSOM_IMAGE_COMMIT" >"$work/root/usr/share/blossom-os/image-source-commit"
rm -rf "$work/root/opt/blossom-source"
sync

# pacman-key may leave per-root GnuPG helpers alive briefly. Stop them before
# detaching the image and use one recursive unmount so an overlooked bind mount
# cannot invalidate an otherwise complete build.
chroot "$work/root" gpgconf --kill all 2>/dev/null || true

umount -R "$work/root" || umount -R -l "$work/root"
mounted=false
losetup -d "$efi_loop"
losetup -d "$root_loop"
trap - EXIT INT TERM
echo "ARM64 development disk assembled: $output_raw"
