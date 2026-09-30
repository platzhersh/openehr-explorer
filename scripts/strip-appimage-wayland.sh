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

APPIMAGE=$(ls ./*.AppImage | head -n1)
APPIMAGE=$(basename "$APPIMAGE")
ARCH_NAME=$(uname -m)

rm -rf squashfs-root
"./$APPIMAGE" --appimage-extract >/dev/null

echo "Removing bundled Wayland libraries:"
find squashfs-root -name 'libwayland-*' -print -delete

curl -fsSL -o appimagetool \
  "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH_NAME}.AppImage"
chmod +x appimagetool

ARCH="$ARCH_NAME" ./appimagetool --appimage-extract-and-run squashfs-root "$APPIMAGE"
rm -rf squashfs-root appimagetool

if [ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  rm -f "$APPIMAGE.sig"
  (cd - >/dev/null && npx tauri signer sign "$DIR/$APPIMAGE")
fi
