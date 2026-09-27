# Blossom OS ARM64 development image

This target is the fast local desktop-development companion to the reviewed
`x86_64` release media. It boots a native AArch64 guest under Apple's
Hypervisor Framework, so Apple-silicon Macs do not emulate an Intel CPU.

It is deliberately a generic UEFI virtual-machine image. It is not an Apple
silicon bare-metal installer and must never be written to a physical Mac disk.
The physical release path remains the separately qualified `x86_64` image.

The image starts from the official generic Arch Linux ARM root filesystem and
installs the same Blossom packages, shell, compositor, portal, and desktop
applications used by the release candidate. The source checkout is shared;
only the kernel, boot path, CPU architecture, and hardware-specific packages
differ.

Build and launch on an Apple-silicon Mac:

```sh
scripts/distribution/build_arm64_development_image_macos.sh
scripts/distribution/run_arm64_development_vm_macos.sh
```

The builder reuses `.local-arm64/cache` and writes its result under
`.local-arm64/image`. No GitHub Actions workflow is involved.

For the fastest visual review on macOS, import the generated QCOW2 into UTM as
an Apple Virtualization ARM64 VM with a VirtIO GPU. Keep UTM's Retina Mode off;
the image includes the SPICE display agent for dynamic resolution and clipboard
integration and the QEMU guest agent for local VM inspection. These integrations
take effect in the next image assembled from this recipe; they do not modify an
already-built QCOW2.
