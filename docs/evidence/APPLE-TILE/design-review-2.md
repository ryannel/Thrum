# APPLE-TILE (G3t) design review 2

9 November 2026. This is the second independent `/review-milestone` design review, of revision `224b58a1` ("Replan Apple
tile against the nine design findings"). It covers the [brief](../../design/modules/APPLE-TILE-BRIEF.md), the
[design](../../design/modules/APPLE-TILE.md), the [plan](../../design/modules/APPLE-TILE-PLAN.md), the
[done-when file](../../design/modules/APPLE-TILE-DONE.json) and the companion edits in the same commit (C07 in
CONTRACTS, RENDERING, PLATFORM-RUNTIME-MAC, BUILD-PLAN and maturity). Reviewer: Claude Opus 5.5. No milestone is judged,
so the done-when file is unchanged. Nothing was built or run on a GPU. I read the documents, the current
`thrum-render` sources and the locked wgpu-hal 30.0.1 sources, and ran `./dev check-docs` on the reviewed commit. It
passed: 577 Markdown files, 2483 links, 30 module records and the design hashes.

**Verdict: DESIGN CHANGES, 7 findings.** The re-plan answers all nine findings of the [first review](design-review.md).
The Chrome items can no longer pass on native-vs-Chrome parity, and the global perceptual tolerance is gone. But the
replacement, exact bytes everywhere, now rests on an unshown premise: that GPU and CPU f32 arithmetic agree bit for
bit. The locked Metal backend doesn't give that, and M8 tests exactly the cases where the two differ. M9 also shrank
to a refactor that changes no draw on the supported path.

## Capabilities derived from the brief

I wrote these from the brief before rereading the design. They match the first review's list.

1. The same cooked valley, with sky, Sun, stars, local lights, shadows and mirrors, survives moving views, a cut, day
   and night, dropped views, slot reuse, resize and device replacement.
2. One immutable generation registry, with prepared portable fallbacks. Every required resource and pipeline exists
   before readiness.
3. Attachment loads, stores and transient eligibility follow actual readers. Logical bytes are never reported as
   physical bandwidth.
4. Uploads can't race GPU reads. Dirty data survives skips and slot reuse. Compact transport keeps every value a
   supported shader reads, including supplied normals and history.
5. Light overflow and mirrored or degenerate rasterization keep established semantics. Cost rows expose CPU fallback
   and validation work.
6. G4 gets one prepared, generation-owned native boundary with no rival queue, raw drawable or mixed encoder.
7. Correctness, agent visual review and lifecycle evidence are mandatory. Numbers are baselines and tripwires, and a
   path is promoted only on a measured win.
8. The same cook and contribution seam, with no new schema. Dependencies are registered and unsafe code is justified.
9. Offscreen proof goes ahead while Mac M2 waits. Live and display qualification is recorded separately.
10. No proof can pass while less required work is drawn, and milestone dependencies resolve.

## Disposition of the first review's findings

| # | First-review finding | Status at `224b58a1` |
|---|---|---|
| 1 | Chrome-only items pass while both targets draw less | **Addressed.** D1.4, D3.4, D7.4 and D12.4 now carry asks or inspects for the wasm dependency tree, WGSL tracing of the 128-B row, a full-resolution comparison against a retained pre-change Chrome baseline, and light and visibility witnesses. D0.5 is honestly labelled smoke. |
| 2 | M9 mirrored ranges have no owner | **Addressed on ownership**: one V2 authority, raster policy kept, open meshes stay two-sided, slot 5060 is the fixture. The scope that's left raises finding 2 below. |
| 3 | Per-slot buffers on portable | **Addressed.** Portable keeps single queue-ordered buffers (design lines 142-150; D4.1, D4.2). D4.3 and D5.3 record the upload-byte deltas. This matches `synthetic.rs` today. |
| 4 | Underived perceptual tolerance | **Replaced** by a mechanism-specific exactness table and small-omission negative controls (design lines 288-304; D0.2). Finding 1 below asks whether exactness is reachable. |
| 5 | D8.1 closes on CPU fallback | **Addressed for mac-metal** (D8.1 text, design lines 183-187). Chrome still has this gap: finding 4 below. |
| 6 | Thermal and validation rows missing | **Addressed.** A sustained run of at least ten minutes per recipe, the thermal state and the validation/admission CPU cost are now required for promotion (design lines 306-314; D5.3, D12.1, D12.2). |
| 7 | G3t M2/M3 change the Mac M2 path unseen | **Addressed.** D2.5 and D3.5 require real-window runs and can't be waived. Mac M2 now needs `APPLE-TILE:M3`. The new items aren't fully judgeable (finding 3) and the schedule consequence goes unstated (finding 5). |
| 8 | Fallback allocations uncosted | **Addressed.** The resident fallback set is stated (design lines 103-117), and D1.2 and D1.3 reconcile resident totals. |
| 9 | D0.3 marked pass on plan prose | **Addressed.** D0.3 is open with empty evidence and must be rerun on the M0 revision. |

## Findings, worst first

### 1. The exactness contract and D8.1 assume CPU and GPU f32 agree bit for bit, and the locked Metal backend compiles with fast math (mechanism verified, consequence suspected)

**What I did.** I read where wgpu-hal compiles MSL, the CPU light oracle, the current light-list selection and the
present and display shaders.

**What I found.**

- **Fast math is on.** `wgpu-hal-30.0.1/src/metal/device.rs:272-278` builds `MTLCompileOptions::new()` and sets only
  the language version and `preserveInvariance`. Metal's default is fast math, which lets the compiler reassociate,
  contract into FMA and use approximate division. Nothing in Thrum overrides this.
- **The CPU oracle is IEEE.** Its score is `intensity / squared.max(0.0001)`, a sum of three squares in Rust f32
  (`crates/thrum-render/src/synthetic/cluster.rs:523-535`). Ranking uses `total_cmp` with index ties (lines 285-291).
- **The baseline for overflow cases is CPU-built.** Today the GPU builds lists only for 1 to 4 lights
  (`synthetic.rs:972`, `GPU_MAX_SOURCES`). So the pre-change baseline for the 16, 32 and 64-light cases uses
  CPU-built lists.

**Why it matters.**

- **M8.** D8.1 requires "score-boundary and tie cases" to match the CPU oracle exactly, executed on the GPU, with no
  fallback allowed. D8.2 requires exact HDR bytes against the pre-change baseline. Those are exactly the inputs where
  an approximate GPU divide or a contracted sum flips a 32nd-place choice. The design's "use the same f32
  operations/order" (line 181) can't be guaranteed from WGSL source under fast math. So M8 can probably only close
  if the boundary fixtures happen to avoid the disagreement. If they don't, M8 stalls under the design's own rule that
  "an unqualified case leaves D8.1 open and requires re-planning".
- **M3.** Exact submitted SDR bytes against the pre-change present oracle need three things to be reproduced
  bit-exactly in a different shader under fast math:
  - the tone arithmetic;
  - the hardware's Rgba8Unorm store rounding, which the old intermediate applied implicitly;
  - the `pow`-based decode in `present.wgsl:29-34`.
  The dither in `synthetic_display.wgsl:25-29,81` is also keyed on the scene-extent `position.xy`, so the fused
  shader must use source-pixel coordinates under letterbox. Nobody has shown that any of this is reachable.
- **M7.** Changing the row from a matrix to explicit affine rows may change the compiled operation order. The risk is
  lower, but it sits under the same exactness requirement.

The design has an honest escape: an exactness failure "is a design issue to resolve before building further". That
prevents a fake pass, but it means M3 or M8 could stop mid-module on a premise the plan could test now.

**Change.**
- State the shader math mode the contract assumes.
- For M8, say how GPU and CPU rankings agree by construction. For example, make the oracle model the GPU's
  arithmetic, or rank on a key both sides compute exactly. Alternatively, define and admit the boundary behaviour as
  a reviewed contract before D8.1 depends on it.
- Add a cheap feasibility probe before M3 and M8 are built: fused tone against the current pair on one full frame,
  and GPU against CPU on the boundary fixtures.
- State in advance what happens if exactness is unreachable, as a reviewed contract change rather than a widened
  metric.

### 2. M9 changes no draw on the supported path by the design's own rules, and its remaining work isn't commissioned (verified)

**What I did.** I traced how the supported route draws.

**What I found.**
- It issues one instanced cube draw over all physical rows (`synthetic.rs:691-701`). It picks the back-culled or
  cull-none pipeline (`synthetic.rs:1531-1532`) according to whether any active row needs two-sided
  (`upload.rs:285`).
- The design says to keep scene-wide cull-none "for the affected order group, including ordinary rows". It also says
  to coalesce adjacent runs "with identical admitted pipeline state even when determinant signs differ".
- On the supported path, that leaves exactly one run, with the same pipeline and the same draw as today. "Order
  group" is never defined. Read as the scene, which is the only grouping the legacy route has, M9 changes nothing on
  the supported path.
- What's left is moving the experimental G3c bucket classifier into a shared V2 authority. That is a refactor of a
  route that is "private until root reviews the wiring".

**Why it matters.**
- The brief asks only that mirrored and degenerate rasterization keep established semantics, and that G3t not
  create another culler. Without M9, G3t doesn't touch mirrors except through M7's reflection tests.
- So D9.1-D9.3 can close as a refactor with no supported-path change and an honestly reported "no win". That is scope
  nobody asked for, inside code another owner holds.

**Change.**
- Define "order group".
- Then do one of these:
  - name a supported case where a semantics-preserving split actually exists;
  - re-scope M9 explicitly as consolidating the classifier only, with no rendering change, and justify why G3t must
    modify the private bucket route;
  - drop M9 and leave the consolidation to the V2/G3c owner's plan.

### 3. D2.5 and D3.5 don't name the window observer or its grant, so "actual window observation evidence" can't be judged (verified)

**What I found.**
- Both items exclude the engine tap and offscreen sequences as proof of window presentation, which is right. But
  they don't say what does prove it.
- [T2-CAPTURE](../../design/modules/T2-CAPTURE.md) line 126 says OS window capture requires Screen Recording
  permission. Its windowed tap captures engine output, not the window.
- The profile requires native and compositor checks to "name their own attended reason". D2.5 and D3.5 name only "an
  unlocked active Mac display".

**Why it matters.** A reviewer can't tell whether logs plus an engine tap taken while a window exists are enough. A
builder could submit exactly that.

**Change.** Name the observer: for example, OS window capture of the game window with Screen Recording granted, or
the reviewer's own screenshot through computer use. Say what it compares: window content against the engine tap of
the same submission, and the letterbox geometry after a resize. List that grant in the ATTENDED SETUP text.

### 4. Chrome's GPU light path can close entirely on CPU fallback (verified from the item text)

**What I found.** D8.3 asks to "record GPU vs CPU fallback explicitly and compare to its own portable CPU oracle".
D12.4 asks for WebGPU dense-light readbacks "against the CPU oracle". Neither requires the WebGPU GPU builder to
execute the 32 and 64-light overflow cases.

**Why it matters.** M8's outcome is "GPU light lists preserve the established lighting choice", for both recipes. A
Chrome build that always uses the CPU path passes D8.3. This is the first review's finding 5, still open on the
portable target. D8.3 also never compares against the retained pre-change Chrome image baseline; only D12.4 does,
at the end of the module.

**Change.** Say whether GPU qualification on WebGPU is required. If CPU fallback on Chrome is acceptable, record it
as an explicit outcome that doesn't satisfy a GPU light-list claim. Add the pre-change Chrome baseline comparison to
D8.3.

### 5. Mac M2 now waits on Chrome and simulator capture plus three G3t milestones, and the build plan doesn't say so (verified)

**What I found.** The chain of dependencies:
- `PLATFORM-RUNTIME-MAC-DONE.json` M2 now needs `APPLE-TILE:M3`.
- G3t milestones are sequential, so M3 waits on M0, M1 and M2.
- G3t M1 needs `T2-CAPTURE:M1` (Chrome) and `T2-CAPTURE:M2` (iOS simulator).
- So the active G3b gate's Mac cadence can't close before Chrome capture, simulator capture and G3t M0-M3 are done.

Some of these prerequisites are attended: G3t D2.5 and D3.5 need an unlocked display.

The plan correctly says the ordering doesn't block G3t. BUILD-PLAN line 46 says only "final qualification reruns
after G3t M3". The ordering is the agents' call, but nobody reading the build plan would see the new critical path.

**Change.** State the transitive critical path in BUILD-PLAN and in the Mac plan. Alternatively, consider whether
M2.1 can qualify on the current path now, with only a post-M3 rerun gated by the dependency.

### 6. Apple per-slot duplicates are built at M4 before the pointer path that justifies them, and the plan never says they're removed if M5 stays unpromoted (verified)

**What I found.** The design says per-slot buffers are needed only for pointer writes (lines 145, 153). M4 adds them
on Apple while every write is still `write_buffer`. That doubles resident bytes and, by the design's own estimate,
roughly doubles steady dirty upload bytes. The pointer path only arrives in M5, and M5 may end unpromoted or
rejected (D5.3). The design doesn't say whether Apple then keeps the duplicates. If it does, Apple pays the cost the
re-plan removed from portable for the same reason, with no benefit.

**Change.** Make per-slot duplication part of the M5 candidate's promotion decision. Apple goes back to single
queue-ordered buffers whenever the pointer path isn't selected, and D5.3 records which.

### 7. Several item statements don't say "exact" where the contract table requires it (verified, low)

D5.2 says "match portable", D7.3 says "preserve route image and motion", and D8.3 says "visible lighting". The table
requires exact bytes for M4/M5, M7 and M8. D0.2 says the table governs later changes, so a careful reviewer would
apply it. But the item statements are what a verdict is recorded against. A visually matching run shouldn't be able
to claim them.

**Change.** Say "exact same-target scene-linear and submitted SDR bytes under the APPLE-TILE.md contract" in those
statements, as D1.3, D2.3, D4.3 and D6.2 already do.

## Done-when judgeability

Outside the findings above, I could judge each item, and most would catch the wrong substitutes the design names:
- D1.4 traces the wasm dependency tree.
- D7.4 rejects a Chrome path left on 192-byte rows.
- D8.1 forbids fallback.
- D9.2 has falsifiers for both culling directions.
- D10.2 rejects a no-op native segment.
- D12.3 refuses a functional window that is "owed".

The items to fix before building are:
- every exact-byte item that depends on cross-shader or CPU/GPU agreement (finding 1);
- D2.5 and D3.5 (finding 3);
- D8.3 and D12.4 on Chrome (finding 4);
- the three loosely worded statements (finding 7).

The M9 items are judgeable, but they can pass on a refactor that changes no draw (finding 2). Dependencies resolve
with no cycle:
- `T2-CAPTURE:M0` needs only `PLATFORM-RUNTIME-MAC:M1`, which is done.
- Mac M2 points forward at `APPLE-TILE:M3`.
- G4 still points only forward at G3t.

DESIGN CHANGES: 7 findings
