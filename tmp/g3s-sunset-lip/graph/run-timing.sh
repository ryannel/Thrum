#!/bin/bash
set -euo pipefail
p=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$p/../../.." && pwd)
store=${THRUM_HORIZON_STORE:-$root/tmp/g3s-sunset-lip/runs/check-20261107-202046/store}
output=${THRUM_HORIZON_ROWS:-$p/timing-verified}
mkdir "$output"
for round in taps1 cells1 cells2 taps2; do
    uptime > "$output/$round.load"
    if [[ "$round" == taps* ]]; then
        export THRUM_PROBE_HORIZON_TAPS=1
    else
        unset THRUM_PROBE_HORIZON_TAPS
    fi
    THRUM_SCENE=valley THRUM_ASSET_STORE="$store" THRUM_CAPTURE_SCENE="$output/no-capture-$round" \
      THRUM_PROBE_HORIZON_TIMING="$output/$round.jsonl" "$p/timing-game" > "$output/$round.log" 2>&1
    uptime >> "$output/$round.load"
done
