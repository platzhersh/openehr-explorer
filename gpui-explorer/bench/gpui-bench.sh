#!/usr/bin/env bash
# Benchmarks the GPUI build under Xvfb + lavapipe (software Vulkan). Output: JSON lines on stdout.
BIN="$(cd "$(dirname "$0")/.." && pwd)/target/release/openehr-explorer-gpui"
source "$(dirname "$0")/xenv.sh"
echo "# binary_bytes=$(stat -c %s "$BIN")"
echo "# startup (5 cold launches, process spawn -> first render)"
for i in 1 2 3 4 5; do
  T0=$(date +%s%3N)
  "$BIN" --demo --view dashboard 2> /tmp/gpui-start.log &
  PID=$!
  sleep 2.5
  PSS=$(grep Pss: /proc/$PID/smaps_rollup | awk '{print $2}')
  kill $PID 2>/dev/null; wait $PID 2>/dev/null
  EP=$(grep -o 'first_render_epoch_ms=[0-9]*' /tmp/gpui-start.log | cut -d= -f2)
  echo "startup run=$i spawn_to_first_render_ms=$((EP-T0)) idle_pss_kb_after_2.5s=$PSS"
done
echo "# aql scenarios"
for N in ${@:-3 1000 10000 50000 1000000}; do
  "$BIN" --demo --view aql --run --rows $N --bench-scroll 2> /tmp/gpui-run.log > /tmp/gpui-run.out
  echo "rows=$N $(grep -o 'run_to_render_ms=[0-9.]*' /tmp/gpui-run.log) $(cat /tmp/gpui-run.out)"
done
