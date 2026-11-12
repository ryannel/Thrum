# M2 presented-time qualification

The reviewed `ab66bca5` commission grants six trace recordings total. The durable counter is
`tmp/mac-m2/presented-commission-20261111.json`; preserve it across output folders and restarts.
Four recordings are consumed: two failed configurations, qualified Deferred OFF and qualified Deferred ON.
Both matched ON sources are complete. No further recording is needed for the current evidence set.
The exhausted older `tmp/mac-m2/trace-commission.json` is separate history.

The exact M2.1 executable was reconstructed byte-for-byte from commit `10287802` after its `target/` copy
was missing. Use `tmp/mac-m2/recover-pinned-20261112/v1-game`, SHA-256
`aa1591522962933b55ec1f347cc538880f5eb14b7ea227bf4adcb7f00b585507`, retained outside disposable build output.
[Recovery record](binary-recovery.json) and [selected results](README.md) preserve the source/build identity.
Do not rebuild or substitute a changed executable for a matched arm.

The successful ON invocation was:

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py lock gpu --worktree . -- python3 scripts/qualify_mac_m2.py --binary tmp/mac-m2/recover-pinned-20261112/v1-game --presented-minimal --presented-deferred --presented-arm on-traced --output tmp/mac-m2/presented-deferred-20261111
```

Never overwrite an arm or repeat an unchanged failed recording hypothesis. The command pins the executable,
preset and cooked store. It records at most 16 seconds, allows at most 75 seconds for finalization, and bounds
each export to 30 seconds. The minimal configuration uses Blank plus Metal Application, Display and Points of
Interest, omits `--window`, and the fresh TOCs confirm Deferred mode. No security setting changes.

The Mac must remain unlocked with the built-in main panel awake. The collector checks this before consuming an
attempt and every 0.5 seconds during recording/exporting, stopping only its owned process group on refusal.
Queued requests and native movie review are also panel-guarded. A locked/asleep panel stops the sitting;
unattended evidence analysis can continue. Each recording/export batch and native playback review takes GPU.
Retain per-arm load, iostat and the lifetime CPU snapshot; other builders' CPU work can continue.

Reproduce witness/source/cost checks without another recording:

```sh
python3 scripts/mac_m2_presented.py --input tmp/mac-m2/presented-deferred-20261111/on-traced/presented.xml --arm tmp/mac-m2/presented-deferred-20261111/on-traced --output tmp/mac-m2/presented-deferred-20261111/on-traced/witness.json
python3 docs/evidence/render/mac-m2-presented-20261111/check-timing.py
python3 docs/evidence/render/mac-m2-presented-20261111/analyse-arms.py --root tmp/mac-m2/presented-deferred-20261111 --output tmp/mac-m2/reproduced-arm-costs.json
python3 -m unittest discover -s scripts -p 'test_mac_m2*.py' -v
```

Review complete native ON originals, the saved crops and host-time 1× movies. Keep submitted endpoints and
AppKit marker polling separate. Only the qualified capture-OFF witness supplies cadence; ON intervals are under
capture. Every marked submission joins or is reported unmatched. Every strict OFF over-period interval, both
whole OFF misses and all four ON misses are retained beside game gaps, yaw, neighboring host/GPU samples and
run-wide load. Movie PTS and handler callback times never substitute for OS-reported presented times.

[Historical checkpoint](HISTORY.md) retains the failed windows, panel sleep and resolved disk blocker. Keep
compressed originals, trace bundles, manifests, cooked store, executable pin and attempt counter through lead
review. Local bulk originals have no durable backup; PNG expansions are reconstructible after source verification.
