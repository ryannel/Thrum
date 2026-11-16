# SIM-2D M1 review

Reviewed commit `2aa357818ab5` (builder's work ends at `5b37129b0271`). Reviewer: Claude Opus 5.5, high effort,
16 November 2026. First review of this milestone.

**Result: FAIL.** 4 items fail (M1.1, M1.2, M1.3, M1.8), 0 Could Not Judge, 0 Must Fix outside the items.
Both causes are small. The crowd itself is sound: the trace, the corrected-timeline fold, the digest, the refusal and
the wire route all do what the design says, and I reproduced the retained folds myself on the Mac.

## What I did

- Read the design (mechanisms 1–7, Cost, Proof), PLAN M1 and every changed line of the builder's commits:
  `61b84d7e3` (the main change: `thrum-core` profile, digest, refusal; `thrum-sim2d`; `crowd-2d`; `crowd-2d-run`;
  conformance and browser scripts), `e6169753f`, `1c88126cb`, `2eba83342`, `242bc29a7`, `d9311107a`, `9bbcab7c6`,
  and the evidence and log commits up to `5b37129b0`. No builder commit touches `examples/v1-game` or weakens an
  existing test.
- Built `crowd-2d-run` in release from this checkout and ran the 2,000-body trace straight and rewinding, and the
  5,000-body trace straight ([output](M1-review-1/mac-straight-and-rewinding.txt)). All three reproduce the retained
  folds; the rewinding run reports 30 rewinds, all 30 changing a digest.
- Read the target receipts (`docs/evidence/SIM-2D/M1/*.json`) and the git history of both cost records.
- Confirmed the main-branch check `exact-images` passed on `da0f278277c4`, the commit the script checked
  (`.git/orchestrate/main-checks/exact-images/runs/1794784938449-da0f278277c4.json`).
- Relied on the script's run of `./dev check` and every item's check on `da0f278277c4`. I re-ran none of the
  Chrome or simulator checks: nothing in them depends on timing, and my Mac runs matched the receipts.
- Edge conditions: the ORCHESTRATE target edge conditions (low sun, storm, distance, fast camera, full refresh;
  rotation and background on the simulator) are visual or app-lifecycle conditions and do not apply to a headless
  conformance executable. The conditions that do apply are the design's: all three sizes, rewinds, debug and release,
  the wire route and slot reuse. The receipts and the checks cover them, and I re-ran the release and rewinding paths.
- No item has a Reviewer Inspection, so there are no screenshots. The only file in the evidence folder is my run
  output.

## Findings, worst first

### 1. Must Fix (verified): M1.1, M1.2 and M1.3: the profile's retained vectors leave out the input conversion

**Items M1.1, M1.2, M1.3.** Their statements say the profile's retained vectors reproduce on each target. Design
mechanism 6 defines those vectors as "every operation of the profile number at the edges of the format, **and the
input conversion at the wire's rounding boundaries**", run in the same command on each target. The retained vectors
do not include the input conversion, so these statements are not yet true.

- `crates/thrum-core/src/profile.rs`, `conformance_vectors()`: arithmetic, comparisons, `to_i32` and `from_i32`,
  and `random`. There is no `Number::from_axis` call. `vectors.json` has 3,790 words, which is exactly 18 operands ×
  210 words plus 7 `from_i32` words plus 3 `random` words, so nothing else is in it.
- `scripts/sim2d_conformance.py run` compares only `vectors()` and the crowd trace on each target. In Chrome and
  the simulator, the input conversion is therefore exercised only by the trace's few axes (0.1, 0.5, −0.5, −0, 0.3,
  −0.7, ±1, −0.9) and their wire copies, and never at a rounding boundary.
- The exhaustive boundary comparison (`examples/crowd-2d-run/tests/input_route.rs`) runs on the Mac only, as
  M1.12 asks. It does not stand in for the per-target vectors. `from_axis` goes through `f64` multiply,
  `f64::round` and a saturating `as i16`, and a Wasm or iOS difference there is exactly what the per-target
  vectors exist to catch.

**What would fix it:** add the input conversion to the retained vectors: every wire integer's axis, both
`next_down`/`next_up` neighbours of each `(k + 0.5)/32767` boundary, ±1, ±0 and a few off-grid values. Then
regenerate `vectors.json` on purpose and re-run `run` on the three targets. This is one change, which clears all
three findings.

### 2. Must Fix (verified): M1.8: a Mac row that trips the quarter-slower tripwire is not flagged

**Item M1.8.** The statement says "the record flags each tripwire the design names that a row trips". The design's
first tripwire is "a row about a quarter slower than the last comparable one at the same neighbour count". The
current record (`docs/evidence/SIM-2D/M1/costs-mac-metal.json`, measured at `371450716`) has a 5,000-body run 1 of
1.751 ms at 65,407 neighbours per tick. The previous retained record (`a3f4c2236`, at `bf07e76eb`) has 1.21 and
1.19 ms at the same 65,407 neighbours. That is 45–47% slower, and the record says `quarter_slower_tripwire: false`.
The builder found the same thing and wrote it in prose in `M1.md` ("44.7% slower than its previous retained
repeat"), but it is not in the record.

Cause: `measure()` in `scripts/sim2d_conformance.py` compares each row only with the same size's row in an
earlier repeat of the same invocation. Run 1 can never be flagged, and a change across commits, which is what the
tripwire is for, can never be flagged either. `costs()` checks only that the field is a boolean.

**What would fix it:** compare each row with the last comparable row of the previously retained record as well as
the earlier repeat, record which row it was compared with, and remeasure or recompute the current record. No
time should decide the result: the record only has to flag the row.

### 3. Suggestion (verified): M1.9's Chrome record has the same blind spot

The Chrome record is computed by the same `measure()`. No Chrome row trips against the previous record (1.37 →
1.41–1.495 ms at 5,000 bodies is at most 9%), so M1.9's statement holds for the record as it stands, and the item
passes. The fix for finding 2 fixes it here too.

### 4. Suggestion (verified): the sine is reachable from a rule package in a workspace build

`platform_sine_control` lives in `thrum_core::profile` behind the `conformance-control` feature, which
`crowd-2d-run` enables. Cargo unifies features within one build, so in `cargo build --workspace` or `./dev check`,
`crowd-2d` compiles against a `thrum-core` that exports the sine. A rule could call it and compile there, and fail
only when built alone. It is not a method of `Number`, and the reference crowd does not call it. M1.5 passes, but
M3's rule-package guard should refuse the symbol or the feature, or the control function should move out of
`thrum-core`. Separately, `Crowd::control` is a public injection hook inside the rule package.

### 5. Suggestion (verified): M1.7's check does not run the coverage test

`cargo test -p thrum-core --test world_digest` runs only the epoch-and-rewind test. The one-at-a-time coverage
mutations are in the library's unit tests (`crates/thrum-core/src/sim/registered_world/digest_tests.rs`), which run
in `./dev check` but not under the item's command. The item passes because both tests exist and both run in the
gate. Naming both in the check would make the item prove itself.

### 6. Suggestion (verified): M1.4's control uses a different comparison from M1.2

`control()` runs through the same `Runner`/`invoke`/`conformance` path as `run()`, but it compares the Mac fold
with the Chrome fold directly, not through `compare()` against the retained file. It is the same fold equality
that `compare()` starts with, so the control does show that the cross-target comparison can fail. Running the
Chrome control row through `compare()` and requiring it to fail would match the quality bar's wording exactly.

### 7. Suggestion (suspected, not measured separately): the profile check rescans every column each tick

`GameRuntime::step_inner` calls `world.validate_profile()` after edit validation, which reads every slot of every
profile column on every tick. It is needed for direct-write callbacks, which write tentatively. For edit-only
rules it repeats the per-value checks already made in `validate_batch_replace` and `EditScratch::validate`. It
is inside the recorded tick time, so it is a baseline, not a defect. The 10,000-body digest at about 1.2 ms
against the probe's 0.18 ms is worth a look too.

### 8. Suggestion (minor): the identity index is digested only through live slots

`RegisteredWorld::digest` encodes the identity index by looking up each live slot's stable identity. A stale entry
for a dead identity would not change the digest. Nothing in the product produces one, so this is a note for
`SESSION`'s baseline restore. (`run_trace`'s comment also says `generation > 0` while the code tests `> 1`.)

## Item results

| Item | Result | Reason |
|---|---|---|
| M1.1 | fail | Finding 1. The rest holds, see the answers below |
| M1.2 | fail | Finding 1. The Chrome receipt otherwise reproduces all three folds, the checkpoints and the wire route |
| M1.3 | fail | Finding 1. The simulator receipt otherwise reproduces all three folds, the checkpoints and the wire route |
| M1.4 | pass | Answers below. Control diverges at tick 1 (Mac `b99ebe39…`, Chrome `45158c2a…`) |
| M1.5 | pass | Answers below |
| M1.6 | pass | Answers below |
| M1.7 | pass | Answers below; finding 5 is a Suggestion |
| M1.8 | fail | Finding 2 |
| M1.9 | pass | The record is complete, the replay is recorded at each size, and no Chrome row trips a tripwire against any earlier record; finding 3 |
| M1.10 | pass | Answers below |
| M1.11 | pass | main-branch check exact-images passed on `da0f278277c4` |
| M1.12 | pass | Answers below |

## Answers to the Reviewer Questions

**M1.1.**
- *Straight retained run?* Yes. `retain()` invokes `conformance(..., straight=True, ...)`, so `frames(tick, true)`
  holds the corrections from tick 1, and it asserts `rewinds == 0`. My own straight runs at 2,000 and 5,000 give the
  retained folds.
- *One digest per tick, as last committed?* Yes. `run_trace` pushes one digest per forward tick, and replay
  overwrites `timeline[tick-1]`. The fold and the checkpoints are taken from that list afterwards.
- *Registered runtime and history?* Yes. The crowd steps through `GameRuntime::new_published` with one registered
  system, uses `Edit::spawn`/`Edit::despawn` with a `StructuralPermit`, and rewinds through
  `RuntimeHistory::rewind`, `amend_future` and `replay_next`. There is no private loop.
- *Wire route through `thrum-network`?* Yes. `wire_route` calls `input_batch::encode` and `decode` on the whole
  four-frame batch, then `decode_frame`.
- *Witnesses above zero?* Yes. `compare()` asserts that reused, hits, rewinds and changed rewinds are above zero
  for every row, and that changed axes are above zero on the wire row (21,903). The wire route runs only the
  smallest size, as the design says.

**M1.4.** The control replaces `chase.x` with `sin(chase.x)` in the enemy steering rule, and nothing else changes.
It goes through the same runner and the same `conformance` export as M1.2. The comparison is a direct fold
inequality, not `compare()` (finding 6).

**M1.5.**
- *Does `Number` expose anything outside mechanism 1's list?* No. It exposes `from_i32`, `to_i32` and `from_axis`
  (the input conversion); sqrt, abs, floor, ceil, trunc and round; comparison-based min, max and clamp; `+ - * /`
  and negation; and derived `PartialEq` and `PartialOrd`, which are partial comparison only. There is no sine,
  power, fused multiply-add, sign-bit read, `to_bits`, `total_cmp` or `From<f32>`, and the inside is private.
  `StateEncode::encode` writes the bits into a digest with NaN normalised, so the only sign it can reveal is a
  zero's, which IEEE fixes. The feature-gated sine is finding 4.
- *Do the vectors cover the edges?* Yes. The operands include ±0, ±min subnormal, max subnormal, min normal,
  ±0.5, 1, 1+ulp, 2−ulp, 2, 2²⁴, ±max and ±∞, and a NaN with a payload. They produce subnormal results (min normal
  − max subnormal, subnormal × 0.5), ties (2²⁴ + 1, min subnormal × 0.5, `round(±0.5)`, `from_i32(±16,777,217)`),
  both zeros, overflow (max × 2, max + max) and NaN operands, all against every operand. Square root and division
  cannot produce exact ties.

**M1.6.** Yes. `profile_refusal.rs` registers a `Number` component with `register_profile`, and a registered
`Simulate` rule pushes `Edit::replace` with the candidate's value. `runtime.step` refuses it with
`NonFinite { schema: 123, slot }`, once for `1/0` and once for `0/0`. The tick, the resource and the digest are
unchanged, and the next valid tick commits.

**M1.7.**
- *Does it change each kind of covered value one at a time?* Yes, across two tests.
  - `digest_tests.rs` mutates, one at a time, the live flag, generation, stable identity, structural tick,
    free-list order, component presence, component value, change tick, identity index and row directory, and
    compares tick 0 with tick 1.
  - `world_digest.rs` changes the game resource, compares worlds in epochs 1 and 999 at every tick, and compares a
    rewound-and-replayed world with a straight one.
- *Is any value hashed through `std::hash` or at the machine's width?* No. The mixer is FNV-1a over explicit
  little-endian bytes, with published vectors. `usize` lengths and rows are widened to `u64`.

**M1.8.**
- *Release builds of the claimed source, through the engine runtime, with load?* Yes. Both runs are release builds
  of `crowd-2d-run`, timed through `RuntimeHistory::step` on the real crowd, with `getloadavg` recorded for each
  run. The record's `source_sha256` is checked against the current sources.
- *Replay figure for `SESSION`?* Yes: `rewind_eight_ms` and `replay_eight_ms` at each size, one sample each.
- *Does the command decide on completeness, never on time?* Yes. `costs()` asserts only presence, types and source
  identity. The item fails on finding 2.

**M1.10.** Yes. The test reads `examples/crowd-2d/retained/crowd.txt` through `include_str!`, which is the same
file the conformance command reads, and it has no regeneration path (only the explicit `retain` action writes the
file). It compares 240 slice ticks and requires reuse, hits and changed rewinds. No builder commit touched the V1
game, and `./dev check` (`cargo test --workspace`) passed on `da0f278277c4`.

**M1.12.**
- *Does the wire side use the public codec?* Yes. The test goes through `input_batch::encode`/`decode` on whole
  batches, not a copy of the arithmetic.
- *Does it cover every case the item names?* Yes. It covers all 65,535 wire values, both neighbours of every
  boundary, ±1 and ±0, and checks that converting a decoded axis gives the same bits.
- *Do changed axes steer players?* Yes. The slice asserts `changed_axes > 0`, and the controls set player
  velocity.
- *Does it run in `./dev check`?* Yes, as part of the workspace tests.
- *Can a rule turn an axis into a profile number another way?* Not through the type: `Number` has no conversion from
  a raw `f32`. Only raw-float arithmetic followed by an integer cast could do it, and M3's lint is planned to refuse
  that.

## Proposed items

None. Both Must Fix findings break the items' own statements, so the existing items already catch them.
