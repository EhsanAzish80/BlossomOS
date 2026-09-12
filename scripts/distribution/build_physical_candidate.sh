#!/usr/bin/env bash
set -euo pipefail

if (($# != 1)); then
  echo "usage: build_physical_candidate.sh OUTPUT_DIRECTORY" >&2
  exit 2
fi

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
output=$(realpath -m "$1")
mode=${BLOSSOM_CANDIDATE_MODE:-physical}
case "$mode" in
  physical|vm-qualification) ;;
  *) echo "BLOSSOM_CANDIDATE_MODE must be physical or vm-qualification" >&2; exit 2 ;;
esac
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
cp "$build/source"/distribution/packages/blossom-core/blossom-core-*.pkg.tar.zst "$packages/"
cp "$build/source"/distribution/packages/blossom-shell/blossom-shell-*.pkg.tar.zst "$packages/"

pacstrap -K -C "$repo/distribution/archiso/pacman.conf" "$rootfs" \
  base bluez bluez-utils brightnessctl bubblewrap dbus-broker dosfstools foot \
  gptfdisk hyprland intel-ucode iwd linux-firmware linux-lts mesa networkmanager \
  openssh openssl pipewire pipewire-alsa pipewire-pulse polkit python quickshell \
  qt6-base qt6-declarative sof-firmware sudo systemd upower vulkan-intel wireplumber zstd
pacman --root "$rootfs" --config /etc/pacman.conf --noconfirm -U "$packages"/*.pkg.tar.zst

cp -a "$repo/distribution/physical-rootfs/." "$rootfs/"
install -d -m 0755 "$rootfs/opt/blossom" "$rootfs/etc/blossom-os"
cp -a "$repo/scripts" "$repo/tests" "$repo/distribution" "$rootfs/opt/blossom/"
install -d -m 0755 "$rootfs/opt/blossom/.github/workflows"
install -m 0644 "$repo/.github/workflows/phase9-vm-install-evidence.yml" \
  "$rootfs/opt/blossom/.github/workflows/phase9-vm-install-evidence.yml"
useradd --root "$rootfs" -m -G audio,input,video,wheel -s /bin/bash blossom
passwd --root "$rootfs" --lock blossom
install -d -m 0700 "$rootfs/home/blossom/.config/hypr"
install -m 0600 "$rootfs/usr/share/blossom-os/physical-home/hyprland.conf" \
  "$rootfs/home/blossom/.config/hypr/hyprland.conf"
install -d -m 0755 "$rootfs/etc/systemd/system/getty@tty1.service.d"
install -m 0644 "$rootfs/usr/share/blossom-os/physical-home/getty-autologin.conf" \
  "$rootfs/etc/systemd/system/getty@tty1.service.d/autologin.conf"
chown -R 1000:1000 "$rootfs/home/blossom"
ln -sf /usr/lib/systemd/system/NetworkManager.service \
  "$rootfs/etc/systemd/system/multi-user.target.wants/NetworkManager.service"
ln -sf /usr/lib/systemd/system/bluetooth.service \
  "$rootfs/etc/systemd/system/dbus-org.bluez.service"
ln -sf /usr/lib/systemd/system/blossom-privileged-helper.service \
  "$rootfs/etc/systemd/system/multi-user.target.wants/blossom-privileged-helper.service"
if [[ "$mode" == vm-qualification ]]; then
  install -Dm0755 "$repo/distribution/evidence/blossom-evidence-boot" \
    "$rootfs/usr/local/bin/blossom-evidence-boot"
  install -Dm0644 "$repo/distribution/evidence/blossom-evidence-boot.service" \
    "$rootfs/etc/systemd/system/blossom-evidence-boot.service"
  ln -sf ../blossom-evidence-boot.service \
    "$rootfs/etc/systemd/system/multi-user.target.wants/blossom-evidence-boot.service"
  install -Dm0644 "$repo/distribution/evidence/install-marker.json" \
    "$rootfs/etc/blossom-os/install.json"
fi
sed -i 's/^#Storage=.*/Storage=volatile/' "$rootfs/etc/systemd/journald.conf"
tar --xattrs --numeric-owner -I "$rootfs_compressor" \
  -cf "$build/blossom-rootfs.tar.zst" -C "$rootfs" .

cp -a /usr/share/archiso/configs/releng/. "$profile/"
cp "$repo/distribution/archiso/profiledef.sh" "$profile/profiledef.sh"
cp "$repo/distribution/archiso/pacman.conf" "$profile/pacman.conf"
cp "$repo/distribution/archiso/packages.x86_64" "$profile/packages.x86_64"
sed -i -E 's/ archiso_pxe_(common|nbd|http|nfs)//g' \
  "$profile/airootfs/etc/mkinitcpio.conf.d/archiso.conf"
sed -i 's/iso_application=.*/iso_application="Blossom OS physical qualification candidate"/' \
  "$profile/profiledef.sh"
sed -i 's/iso_version=.*/iso_version="0.11.0-physical-candidate"/' "$profile/profiledef.sh"
rm -f "$profile/airootfs/etc/systemd/system/multi-user.target.wants/blossom-evidence-install.service"
rm -f "$profile/airootfs/etc/systemd/system/blossom-evidence-install.service"
install -Dm0755 "$repo/distribution/archiso/airootfs/usr/local/bin/blossom-physical-install" \
  "$profile/airootfs/usr/local/bin/blossom-physical-install"
install -Dm0755 "$repo/distribution/archiso/airootfs/usr/local/libexec/blossom-physical-install-backend" \
  "$profile/airootfs/usr/local/libexec/blossom-physical-install-backend"
install -d -m 0755 "$profile/airootfs/opt/blossom"
cp -a "$repo/scripts" "$profile/airootfs/opt/blossom/"
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
mkarchiso -v -w "$build/archiso-work" -o "$iso" "$profile"
(
  cd "$iso"
  sha256sum blossom-os-*.iso > SHA256SUMS
)
