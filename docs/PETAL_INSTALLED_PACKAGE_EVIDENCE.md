# Petal installed-package evidence

Status: passed locally on ARM64 on 2026-10-01 for the package boundary below.
This is not a clean-image or physical-device result.

## Scope

- Source branch: `design/petal-refresh`, based on the current `main` after PR
  #213.
- Package: locally built `blossom-shell 0.9.0-3` for `aarch64`.
- Installation: `pacman -U` in a disposable qcow2 overlay backed by the
  credential-free ARM64 development image.
- Guest resources: 4 vCPUs and 8 GiB RAM.
- Base image: retained and unmodified; the disposable overlay was deleted.
- No full image build and no GitHub-hosted image build ran.

## Observed results

- `ttf-ibm-plex 6.4.0-1` was installed through the real package dependency.
- `/usr/share/licenses/blossom-shell/IBM-Plex-OFL.txt` existed in the installed
  package and contained the reserved-name notice.
- The maximum exact-effect approval fixture ran in the real Wayland session
  with Qt accessibility enabled.
- AT-SPI recovered, exactly and without ellipsis:
  - the 60-character basename plus `.txt`;
  - the complete 4,096-byte content;
  - the complete original request;
  - the full destination and destination folder;
  - the byte length; and
  - the `Parsed directly from your request` proposal-source label.

The approval component did not import Petal and displayed the fixed
`Blossom approval · drawn by the system broker` header. Source checks reject
theme imports in the approval component and calculate WCAG contrast ratios for
the ordinary Petal text/background pairs.

## Local source gates

The shell QML, desktop platform, shell packaging, Phase 9 distribution, Phase
11 boundary, ARM64 development image, physical-candidate tests and
`git diff --check` passed before the installed-package run.

## Limitations

- This proves installed package layout and accessibility behavior, not the
  appearance of every compositor or display configuration.
- The fixed broker header is a recognizable cue, not trusted pixels. ADR-0034
  continues to record overlay phishing as a remaining risk.
- The command-bar interaction remains an accepted design contract and mockup;
  it is not implemented in this package.
