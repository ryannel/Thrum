#!/bin/sh
# G5 asserted tripwires: run the scan/edit-to-visible/startup/hitch bench and the in-game upload bench,
# each checking its metrics against same-load history in docs/evidence/integration/tripwires/.
# Exits non-zero naming every regressed metric. Pass --record to append passing runs to the history.
# The benches need the GPU and minutes of machine time, so they stay out of ./dev check.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo"
if [ "${1:-}" = "--record" ]; then
  THRUM_TRIPWIRES_RECORD=1
  export THRUM_TRIPWIRES_RECORD
fi
work="$repo/tmp/g5-tripwires"
mkdir -p "$work"
status=0
for bench in "thrum-cook edit_to_visible" "v1-game cooked_edit"; do
  set -- $bench
  report="$work/$2.md"
  rm -f "$report"
  THRUM_G5_BENCH_OUT="$report" cargo bench --offline -q -p "$1" --bench "$2" >"$work/$2.log" 2>&1 || status=1
  sed -n '/^tripwires/,$p' "$report" 2>/dev/null || echo "$2: bench failed before reporting; see $work/$2.log"
done
exit "$status"
