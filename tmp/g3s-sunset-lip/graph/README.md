# Sunset horizon in the V1 render graph

WIP against main at 6726a626. The game graph consumes the live caster
raster and computes a max pyramid. The flat leaf is deliberately
under review. The far-cascade fade is unchanged. This branch is a
diagnostic and switches atmosphere shadow scenes with sufficient
sampled texture slots, including Chrome. Main still runs taps.

The native release V1 game used the same cooked west-spur store
(`32ed89be…`) for baseline and candidate. The terrain generator and
hashes are on `codex/g3s-horizon-metal-wip`. In the main checkout,
the existing store is
`tmp/g3s-sunset-lip/runs/check-20261107-202046/store`. Regenerate with
`thrum-cook cook` and the saved west-spur source if needed.

## Rerun

Build `v1-game` release at the baseline and candidate commits. Launch
both with `THRUM_SCENE=valley`, the same `THRUM_ASSET_STORE` and a new
directory in `THRUM_CAPTURE_SCENE`. The real capture route writes 97
held sunset frames. `THRUM_CAPTURE_LIVE_TICKS=36` adds 36 advancing
world ticks at 17.7 h. These captures are offscreen.

For paired timings, run `build-timing.sh` from this directory. It adds
temporary probe code to the existing capture entry, builds and restores
the source. Set `THRUM_HORIZON_STORE` to the exact cooked store, then
run `run-timing.sh` with access to Metal. It interleaves two legacy
and two candidate rounds in a single binary. Every row verifies a
cooked submission and a valid GPU timestamp batch. Use
`python3 timing-summary.py timing-verified` for p50/p95/p99. Set
`THRUM_HORIZON_ROWS` to a new directory to retain a repeat.

## Next decisions

The full Metal workspace check failed on image equality after discarded
cooked uploads. The exact workspace-built executable reproduced it
in isolation. Both individually built versions passed. Keep the
workspace-feature check open and compare pixels, shader paths and
resource lifetimes in an equivalent baseline build. Upload frame counts
and byte counts matched in the failure.

Independent review found a 5% lit uphill plane becomes fully shadowed
at 3.2° Sun because an 80 m sample is used at its cell entry of 64 m.
Keep conservative parent bounds, separate the leaf estimate and prove
the corrected shape in the real raster. Also bound near-cardinal
reciprocals. Restrict the experiment to an explicit route before
promotion and prove Chrome, camera snaps and moving casters there.
