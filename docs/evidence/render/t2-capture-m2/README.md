# T2-CAPTURE M2 prerequisite blocker

**BLOCKED, 9 November 2026.** M2.1 and M2.2 remain open. The iPhone 18 Pro / iOS 27.2 simulator runs
ordinary diagnostic foreground frames while the Mac console is locked. Its Metal adapter lacks
`DownlevelFlags::INDIRECT_EXECUTION`; the existing cooked-scene contribution requires it. G5's
`thrum-asset-gpu` owner must provide and qualify a capability-selected cooked drawing path on this target.
No T2 capture, cooked motion, pending-capture lifecycle, device-speed or physical-display proof is claimed.

The [minimal probe](probe.py) temporarily adds the existing `attach_cooked_scene` call to the ordinary
simulator host and records actual adapter/downlevel properties. It adds no capture or alternative renderer.
It restores source in `finally`, terminates the app and deletes its fresh simulator. Run from this worktree:

```sh
RUSTC_WRAPPER= RUSTC="$(rustup which --toolchain 1.99.1 rustc)" rustup run 1.99.1 cargo run --locked --offline -p thrum-cook -- cook examples/v1-game/assets/showcase tmp/t2-m2-cooked
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.3.1/skills/orchestration/build/scripts/agent_loop.py lock gpu -- python3 docs/evidence/render/t2-capture-m2/probe.py --output tmp/t2-m2-probe --store tmp/t2-m2-cooked
```

The output directory must be new. Required simulator/tool failures raise errors; no unavailable-target skip is
accepted. This is a prerequisite diagnostic, not the future M2 capture command. The output's `blocked` field
reports whether the expected missing-feature/cooked-failure combination was observed.

Evidence: [report](prerequisite-report.json.gz), [exact source patch](probe.patch),
[diagnostic state](diagnostic-state.txt), [cooked failure](cooked-state.txt), and the
[initial failure](initial-failure.json). The initial adaptation reported the explicit error:
`Device::create_buffer`, `cooked meshlet draws`, `INDIRECT_EXECUTION` required but unsupported. The minimal
probe also records cooked validation failure; asynchronous error callbacks may report a downstream invalid
bind group first. Its actual adapter capability list independently shows the missing flag. The final minimal run records 239 diagnostic presents, world tick 327, one pending GPU frame at peak, then the cooked fault. The production
contribution allocates `STORAGE | INDIRECT` argument buffers in `crates/thrum-asset-gpu/src/draw.rs` and calls
`multi_draw_indexed_indirect`. A fresh simulator avoids the shared booted device's wedged app-container service.

The report pins base commit, temporary source/patch, binary and cooked-product hashes, host, simulator identity,
lock state before/after and commands. No Screen Recording permission was requested; absence of that grant is
not qualified here because execution fails before image evidence. No attended exception is invented. The
unverified capture prototype was discarded; production code is unchanged. This is function-only evidence.

[M1 closeout](m1-closeout.json) verifies its reviewed selected Git tree, selected PNG hashes and compressed
metadata before deleting 16,008,456,538 bytes of owned scratch. Full M1 local sources/bundle are retired after
its passed review; selected originals, time records and views remain in Git. M2 retains this compact failure
record and reproducer; scratch/build products are reconstructible. Profile Check and integration results
are recorded in the plan log.
