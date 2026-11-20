#!/usr/bin/env bash
# usage: shot.sh <outfile.png> <app args...>  — launches the app under Xvfb (:99), grabs a frame, kills it
set -e
OUT=$1; shift
source "$(dirname "$0")/bench/xenv.sh"
"$(dirname "$0")/target/release/openehr-explorer-gpui" "$@" 2>/dev/null &
PID=$!
sleep 4
xdotool mousemove 1400 940
ffmpeg -y -loglevel error -draw_mouse 0 -f x11grab -video_size 1440x960 -i :99 -frames:v 1 "$OUT"
kill $PID 2>/dev/null || true
