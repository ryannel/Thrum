# G3c M2: the occlusion experiment, and why it does not ship

7 November 2026. Apple M3 Max/Metal, Rust 1.99.1, wgpu 30.0.1. Branch `claude/g3c-qdepth2` at `5d847811`, five
commits on main `935ddd14`. This file records the branch for M3. Early
versions of the pyramid, the early and late phases and `occlusion.wgsl` already sit unwired on main. They have one
shared history buffer, no tail clear and no capture fix. The fixed versions below exist only on the branch, and M3
starts from it. The branch is local to the G3c worktree; it is not pushed. Branch commits:
- `f722ccb3` adds Occlusion;
- `b7eab41e` adds the equal-depth witness;
- `dc4f13e3` adds the blended-contribution witness;
- `7ca209c5` adds the field test's Occlusion count arm;
- `5d847811` fixes a test-module import.

`./dev check` passed at `5d847811` (`tmp/g3c-boundary/m2-qdepth4-devcheck.log`). The runs below were taken on a shared, contended
GPU; none of them is a timing claim.

**Decision:** Frustum ships for the field and Direct for the grid. Occlusion (two-phase Hi-Z with a split forward)
stays on the branch. Root ruled this in NOTES ("Root ruling on Q-Depth and the shipped path"), and the overseer
agreed. Shipping Hi-Z is M3's.

## What the branch builds
- `CullMode::Occlusion`, used for the field's opaque path only. With a contribution attached, it falls back to
  Frustum.
- Per-view visibility history with two sides, declared to the graph as separate history (committed and next).
- A pyramid built by min reduction under reverse-Z, equal to its CPU reference.
- Early and late visibility phases. The late phase recomputes the frustum test rather than reading a mask.
- A split forward pass: early forward, pyramid, then late forward with load.
- Early and late counts through the stats readback.

## Field flythrough
The release field test drives Direct, Frustum and Occlusion hosts through the same ticks. At `5d847811` (system load about
49), Frustum and Occlusion are both byte-equal to Direct at all 79 sampled ticks, and every culled frame was counted
(`79 frames compared; Frustum drew 21745 of 100513 at its fewest; 0 culled frames retired before their count`,
`tmp/g3c-boundary/m2-final-field.log`). This field has no
exactly coincident instances, so it cannot show the failure below. The branch's earlier field test had no count arm
for Occlusion and failed at tick 1. `./dev check` found this after the rebase, and `7ca209c5` fixed it.

## The failure: Direct equivalence at equal depth (unresolved)
`equal_depth_ties_follow_the_early_phase_when_a_lower_slot_enters_late` (`crates/thrum-render/tests/visibility.rs`).
A two-slot grid, with each slot drawing a different material:
- one slot stays in view for two frames;
- the other starts at x = +40, then takes the first slot's exact pose.

| Entering (late) slot | Early slot | Pixels differing from Direct |
| --- | --- | ---: |
| 0 | 1 | 3,216 |
| 1 | 0 | 0 |

The test prints its counts. At `5d847811` it printed `[3216, 0]`
(`tmp/g3c-boundary/m2-final-witness.log`: `pixels changed against Direct with slot 0, then slot 1, entering
late: [3216, 0]`), the same as before the rebase (`m2-witness-run.log`, at a pre-rebase
commit of the same test). 

**Cause.** Direct draws in slot order with a strict `Greater` test, so the lower slot wins a tie. Occlusion draws last
frame's visible set early, so a higher slot drawn early keeps the pixel.
Options costed in NOTES ("Q-Depth hold 1"):
- **GreaterEqual for the late pass, or a descending late order.** By reasoning, not by a run: either only flips
  which order fails.
- **A slot-ID target** written early, plus a depth copy the late fragment reads to settle ties. It changes the
  forward colour formats and the `SceneSurface.color_formats` contract, and adds a full-resolution R32 write and read
  each frame. Ruled out.
- **A depth-only prepass of the previous visible set**, the pyramid, the late test, one more compaction over the
  union into a slot-ordered list, and a colour draw into cleared depth. It costs a depth-only redraw of the previous
  set, one more per-slot dispatch, a pass and a depth target. This is the M3 candidate.
- **A slot-ordered depth bias.** It changes Direct's semantics. Rejected.

The test now asserts the hazard is present (`changed[0] > 0`) and that the opposite order matches
(`changed[1] == 0`). Once order is restored, it fails, asking for `[0, 0]`.

## Fixed on the branch: stale history words
The shader writes only the words of dispatched workgroups. A shrinking active prefix could leave stale high bits for
a later regrowth to replay. On publication, the next side's tail is now cleared past the dispatched words.
`history_words_never_outlive_a_shrink_capture_or_contribution` covers:
- capacity 600;
- a capture then a short frame;
- a regular short frame;
- regrowth with an early count below 200;
- a Skip frame (contribution) clearing everything.
Every frame also asserts that no committed bit lies at or above the active count. Mutant: removing the clear fails
the test at `visibility/tests.rs:493` (`tmp/g3c-boundary/m2-history-mutant.log`).

## Contribution fallback, with a visible blended draw
`occlusion_fallback_keeps_a_blended_contribution_on_top` (Metal) attaches a half-transparent triangle, drawn in
front of the cube at the end of the early forward pass, to a Direct host and an Occlusion host. Steps:
- the veiled Direct image differs from the plain one, so the veil is visible;
- the Occlusion capture from a cold history equals veiled Direct;
- after two display frames it still equals veiled Direct.
Mutant: the early and late phases ignore the contribution, with the pyramid still skipped. The cube is then listed
late and drawn over the blend, and the cold capture fails at `device.rs:617` with "cold capture" (`tmp/g3c-boundary/m2-veil-mutant.log`). The timing test
keeps the no-op fixture, as root suggested.

Both mutants ran on the branch before it was rebased from `45fb5871` onto `935ddd14`. Between those two commits,
`visibility.rs`, `visibility/` and `camera_culling.rs` are identical. `device.rs` only gains the late-map test,
and `camera_culling/stats.rs` gains the late-map fix, which neither mutant's path reaches.

## Limits of the witnesses
- The field test's Occlusion arm checks only `late <= drawn <= active`. Its byte equality would hold even if
  Occlusion hid nothing beyond Frustum.
- The equal-depth witness does not assert that the entering slot was drawn in the late phase.
- The veil test does not assert that the veil overlaps the cube; the veiled-versus-plain difference shows only that
  it is visible.
- The `tmp/g3c-boundary` logs are deleted at G3c closeout. Their decisive lines are quoted here.

## Open on the branch for M3
- The ordered-final-draw design, judged on both orderings, the full route and total cost against Frustum.
