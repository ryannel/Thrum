# Building in Thrum

Engine rules for `/build-milestone` in this repository, on top of the generic skill. The profile is
[ORCHESTRATE.md](../../../../ORCHESTRATE.md).

## Pick the work

- **Extend running paths before hardening inactive ones.** Code with no consumer by the end of a slice does not land,
  and parked tracks stay parked.
- **Aim for showcase quality, not parity.** Prefer named techniques, paper reproductions and specialisations agents
  can sustain. Claimed wins need measurements. Record sources and evidence in
  [the technique register](../../../../docs/design/AI-PERFORMANCE.md#technique-register).
- **Native and browser.** Native leads and uses every hardware feature. The browser runs the portable recipe at
  showcase quality and never limits native.
- **Crates.** Before building a general subsystem, look for a proven crate that meets the targets (Wasm, mobile,
  determinism where needed). Build your own where a specialised path is the point and can show its win.

## Match rigour to the state

| What the code touches | On violation |
| --- | --- |
| Authoritative world state, saves, wire input, security boundaries | Refuse with a typed error, leave state unchanged, test the failure path |
| Presentation, VFX, audio, cosmetic budgets, frame resources | Degrade: clamp or drop, count it, log it; `debug_assert!` in development. Never stall a tick or fail a frame |
| Authored content | Validate with tolerances; importers repair what they safely can (weld, drop degenerates) and warn |

Don't add exact multi-precision arithmetic, certificates, logical byte-credit accounting or diagnostic-ordering
contracts unless an observed failure needs them and a simpler bound failed.

## Shape the code

The lint policy and `./dev check` enforce suppression discipline, API reachability, test placement and size
backstops. When designing types, APIs, errors, decoders or test support, read [rust code](rust-code.md).
- **One path per behaviour.** Express variants with generics, typestate or a builder.
- **Engine crates hold no game schema.**
- **Keep internals `pub(crate)` behind a small facade,** with one error enum per public boundary.
- **When a size note or limit fires,** split by responsibility, never by line count.

## Hot paths

For per-tick, per-frame and per-message code:
- Reuse buffers and arenas, and move values.
- Store iterated data as columns, and validate once.
- Use a fast non-cryptographic hasher for internal maps.
- Account cost before choosing a representation
  ([the cost account](../../../../docs/design/AI-PERFORMANCE.md#the-cost-account-in-each-owning-design)).
- Select accelerated paths from the capability registry and keep a portable fallback that the same tests exercise
  ([hardware-native decision](../../../../docs/design/decisions/HARDWARE-NATIVE-ENGINE.md)).

## Verify

- **Tests** cover behaviour and plausible failure; authority work covers rejected, stale and duplicate input.
- **Timing** lives in `benches/` with stored baselines. Allocation claims use the counting-allocator assertion.
- **Parsers:** every parser of untrusted bytes gets a fuzz target.
- **Device, frame-time and visual claims** come only from runs on named targets:
  - this M3 Max;
  - the iPhone simulator (function only, never speed);
  - Chrome.

  Label Mac stand-ins "proxy". Device timing runs alone on the GPU: wrap timing and capture
  runs in `agent_loop.py lock gpu -- <command>`, which holds a machine-wide lock.
- **Regressions.** Investigate visible hitches, about 25% regression from a comparable baseline, or unexpected scaling
  ([performance](performance.md)).
- **Visible output:** capture fixed cameras and a frame sequence from the exact build on the named target, offscreen.
  Convert captures to PNG for review, since capture writes `.ppm`. Look at them yourself against the inspection list.
- **Status** goes in [the maturity ledger](../../../../docs/design/maturity.json), separately for implementation,
  correctness, visuals and device. Evaluator passes record `agent_reviewed`.
- **Evidence** is at most one page per milestone, linked from the done-when items. Bulk output follows
  [the storage rules](../../../../docs/ASSET-LIFECYCLE.md).

## Build outputs and worktrees

- **Run `cargo clean` in your worktree** when work stops. A daily sweep deletes stale `target` directories.
- **Remove the worktree once the commit has landed,** that is, once `git merge-base --is-ancestor <commit>
  origin/main` passes after a fetch.
- **In fan-out,** each stream keeps one target directory across its milestones.
