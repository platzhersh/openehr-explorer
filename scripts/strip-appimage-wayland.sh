#!/usr/bin/env bash
# Strip bundled libwayland-* from the Tauri AppImage and repack it (OEH-92).
#
# The bundled libwayland clashes with the newer system Mesa/EGL stack on recent
# distros (e.g. Ubuntu 26.04) and aborts with EGL_BAD_PARAMETER. libwayland is
# present on every Linux desktop, so the system copy is used instead.
#
# Usage: strip-appimage-wayland.sh [appimage-dir]
# If TAURI_SIGNING_PRIVATE_KEY is set, the repacked AppImage is re-signed so the
# updater signature (.AppImage.sig) stays valid.
set -euo pipefail

DIR="${1:-src-tauri/target/release/bundle/appimage}"
cd "$DIR"

shopt -s nullglob
candidates=(./*.AppImage)
if [[ ${#candidates[@]} -eq 0 ]]; then
  echo "No AppImage found in $DIR" >&2
  exit 1
fi
APPIMAGE=$(basename "${candidates[0]}")
ARCH_NAME=$(uname -m)

# Pinned appimagetool release, verified by checksum (no unpinned "continuous" build).
APPIMAGETOOL_VERSION=1.9.0
case "$ARCH_NAME" in
  x86_64) APPIMAGETOOL_SHA256=46fdd785094c7f6e545b61afcfb0f3d98d8eab243f644b4b17698c01d06083d1 ;;
  aarch64) APPIMAGETOOL_SHA256=04f45ea45b5aa07bb2b071aed9dbf7a5185d3953b11b47358c1311f11ea94a96 ;;
  *) echo "Unsupported architecture: $ARCH_NAME" >&2; exit 1 ;;
esac

rm -rf squashfs-root
"./$APPIMAGE" --appimage-extract >/dev/null

echo "Removing bundled Wayland libraries:"
find squashfs-root -name 'libwayland-*' -print -delete

curl -fsSL -o appimagetool \
  "https://github.com/AppImage/appimagetool/releases/download/${APPIMAGETOOL_VERSION}/appimagetool-${ARCH_NAME}.AppImage"
echo "${APPIMAGETOOL_SHA256}  appimagetool" | sha256sum -c -
chmod +x appimagetool

ARCH="$ARCH_NAME" ./appimagetool --appimage-extract-and-run squashfs-root "$APPIMAGE"
rm -rf squashfs-root appimagetool

if [[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  rm -f "$APPIMAGE.sig"
  (cd - >/dev/null && npx tauri signer sign "$DIR/$APPIMAGE")
fi
