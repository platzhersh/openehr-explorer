# AUR packaging (`open-ehr-explorer-bin`)

This directory is the source of truth for the AUR package `PKGBUILD`. It is
mirrored to the AUR git remote (`ssh://aur@aur.archlinux.org/open-ehr-explorer-bin.git`)
by hand for now — there's no CI job for this yet (unlike the apt repo), since
AUR pushes require a maintainer's personal SSH key registered on an Arch
Linux account, which can't be delegated to a CI secret.

## One-time setup (maintainer)

1. Create an account at https://aur.archlinux.org and add an SSH public key
   under Account Settings.
2. Clone the (initially empty) AUR repo:
   ```bash
   git clone ssh://aur@aur.archlinux.org/open-ehr-explorer-bin.git aur-open-ehr-explorer-bin
   ```

## Releasing a new version

1. In this directory, bump `pkgver` (and reset `pkgrel=1`) in `PKGBUILD` to
   match the new release tag, and update `sha256sums` to the new `.deb`'s
   digest:
   ```bash
   curl --proto '=https' -fsSL -o /tmp/pkg.deb \
     "https://github.com/platzhersh/openehr-explorer/releases/download/v<NEW_VERSION>/openEHR.Explorer_<NEW_VERSION>_amd64.deb"
   sha256sum /tmp/pkg.deb
   ```
2. Regenerate `.SRCINFO` (requires `makepkg`, i.e. run this step on Arch or
   in an `archlinux` container):
   ```bash
   makepkg --printsrcinfo > .SRCINFO
   ```
3. Copy both files into the cloned AUR repo, commit, and push:
   ```bash
   cp PKGBUILD .SRCINFO ../aur-open-ehr-explorer-bin/
   cd ../aur-open-ehr-explorer-bin
   git add PKGBUILD .SRCINFO
   git commit -m "chore: update to <NEW_VERSION>"
   git push
   ```

## Why `-bin` and why it downloads the `.deb`

- AUR convention requires the `-bin` suffix for packages that install
  pre-compiled binaries rather than building from source in AUR's clean
  chroot.
- Building this app from source needs npm/cargo to fetch dependencies over
  the network mid-build, which AUR's build environment disallows — so this
  package instead downloads the already-built `.deb` from the GitHub
  release and repackages its contents (`bsdtar` unpacks the outer `ar`
  archive, then the inner `data.tar.*` is extracted straight into
  `$pkgdir`).
- `depends` mirrors the `.deb`'s real `Depends:` field
  (`libwebkit2gtk-4.1-0`, `libgtk-3-0` → `webkit2gtk-4.1`, `gtk3` on Arch),
  verified against the actual v0.5.0 build's `control` file.

See OEH-12 in Linear for the full packaging plan (apt repo + AUR).
