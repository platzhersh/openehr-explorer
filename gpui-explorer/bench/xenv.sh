# source me: makes sure an X server and a window manager are running on display :99 (GPUI's X11 refresh loop needs a WM
# to receive map/visibility events). Reuses what is already there and never stops processes it did not start.
export XDG_RUNTIME_DIR=/tmp/xdg VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json DISPLAY=:99
mkdir -p $XDG_RUNTIME_DIR; chmod 700 $XDG_RUNTIME_DIR
x_ok() { xdpyinfo -display :99 >/dev/null 2>&1; }
wm_ok() { xprop -display :99 -root _NET_SUPPORTING_WM_CHECK 2>/dev/null | grep -q 'window id'; }
if ! x_ok; then
  Xvfb :99 -screen 0 1440x960x24 >/dev/null 2>&1 &
  for _ in $(seq 1 50); do x_ok && break; sleep 0.1; done
fi
if ! wm_ok; then
  openbox >/dev/null 2>&1 &
  for _ in $(seq 1 50); do wm_ok && break; sleep 0.1; done
fi
