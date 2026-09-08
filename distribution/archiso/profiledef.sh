#!/usr/bin/env bash
# shellcheck disable=SC2034
iso_name="blossom-os"
iso_label="BLOSSOM_090"
iso_publisher="Blossom OS <https://blossom.3nsofts.com>"
iso_application="Blossom OS evidence installer"
iso_version="0.9.0-evidence"
install_dir="blossom"
buildmodes=('iso')
bootmodes=('uefi-x64.systemd-boot.esp' 'uefi-x64.systemd-boot.eltorito')
arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=(-comp zstd -Xcompression-level 15)
file_permissions=(
  ["/usr/local/bin/blossom-evidence-install"]="0:0:755"
)
