#!/usr/bin/env bash
# Benchmarks the GPUI build under Xvfb + lavapipe (software Vulkan). Output: JSON lines on stdout.
# Exits non-zero (before printing a result) when a run fails or produces incomplete measurements.
set -u
BIN="$(cd "$(dirname "$0")/.." && pwd)/target/release/openehr-explorer-gpui"
source "$(dirname "$0")/xenv.sh"
fail() { echo "bench error: $*" >&2; exit 1; }
[[ -x "$BIN" ]] || fail "binary not found: $BIN (run cargo build --release)"
echo "# binary_bytes=$(stat -c %s "$BIN")"
echo "# startup (5 cold launches, process spawn -> first render)"
for i in 1 2 3 4 5; do
  T0=$(date +%s%3N)
  "$BIN" --demo --view dashboard > /dev/null 2> /tmp/gpui-start.log &
  PID=$!
  sleep 2.5
  kill -0 "$PID" 2>/dev/null || fail "run $i: the app exited before the measurement"
  PSS=$(awk '/Pss:/ {print $2}' "/proc/$PID/smaps_rollup")
  kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null
  EP=$(grep -o 'first_render_epoch_ms=[0-9]*' /tmp/gpui-start.log | cut -d= -f2)
  [[ -n "$EP" && -n "$PSS" ]] || fail "run $i: no first_render marker / PSS (see /tmp/gpui-start.log)"
  echo "startup run=$i spawn_to_first_render_ms=$((EP-T0)) idle_pss_kb_after_2.5s=$PSS"
done
echo "# aql scenarios"
for N in ${@:-3 1000 10000 50000 1000000}; do
  "$BIN" --demo --view aql --run --rows "$N" --bench-scroll 2> /tmp/gpui-run.log > /tmp/gpui-run.out || fail "rows=$N: the app exited with an error (see /tmp/gpui-run.log)"
  RENDER=$(grep -o 'run_to_render_ms=[0-9.]*' /tmp/gpui-run.log)
  [[ -n "$RENDER" ]] || fail "rows=$N: no run_to_render_ms marker"
  grep -q '"frames"' /tmp/gpui-run.out || fail "rows=$N: no benchmark JSON on stdout"
  echo "rows=$N $RENDER $(cat /tmp/gpui-run.out)"
done
