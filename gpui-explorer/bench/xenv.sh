# source me: starts a fresh Xvfb + openbox (GPUI's X11 refresh loop needs a WM to receive map/visibility events)
export XDG_RUNTIME_DIR=/tmp/xdg VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json DISPLAY=:99
mkdir -p $XDG_RUNTIME_DIR; chmod 700 $XDG_RUNTIME_DIR
if ! pgrep -x openbox >/dev/null; then
  pkill Xvfb 2>/dev/null || true; sleep 0.5
  Xvfb :99 -screen 0 1440x960x24 >/dev/null 2>&1 &
  sleep 1; openbox >/dev/null 2>&1 &
  sleep 1
fi
