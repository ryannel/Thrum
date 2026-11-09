# T2-CAPTURE design review 3 (narrow: second-review findings and Mac M2/M3 split)

9 November 2026. Independent `/review-milestone` design re-review of revision `b022fda7`, narrowed to two questions:
are the two findings of [design review 2](design-review-2.md) resolved, and did the fix break anything in T2-CAPTURE
or the Mac M2 plan? Reviewer: Claude Opus 5.5. No milestone is judged, so no done-when file is changed. Nothing was
built or run on a GPU.

What I read: the whole `b022fda7` diff (15 files: ORCHESTRATE, BUILD-PLAN, C12 in CONTRACTS, maturity ledger,
PLATFORM-RUNTIME, the Mac plan and JSON, the T2 design, plan and JSON, the APPLE-TILE design, plan and JSON,
APPLE-PRESENTATION-PLAN, and the Mac M2 evidence page). What I ran: `./dev check-docs` (passes: 579 Markdown files,
2505 local links, 30 module records, design hashes), a parse of the Mac, T2 and APPLE-TILE JSON files listing every
milestone's `state`, `needs`, `fails`, `replans` and each item's `check`/`lock` (all parse), a repository-wide grep
for stale "final qualification after G3t M3" or "M2 needs APPLE-TILE" wording (none left outside history and log
lines), and a look for the probe scratch and the sphere-exit claim cited by the new wording.

**Verdict: DESIGN CHANGES, 1 finding (minor).** Both second-review findings are resolved. T2 M0 and Mac M2 are ready
to build as written. The one finding is in the new Mac M3, which can't start until APPLE-TILE is recommissioned, so
it blocks neither T2 M0 nor Mac M2.

## Disposition of the second review's findings

| Finding | Disposition at `b022fda7` |
| --- | --- |
| F1 Mac M2 gated on parked `APPLE-TILE:M3` | **Resolved** (the first of the two options I proposed). Mac M2 `needs` is now only `["T2-CAPTURE:M0"]`. M2.4's trigger is "T2-CAPTURE M0 is done, without waiting for APPLE-TILE". The post-change rerun is a new Mac M3 with `needs: ["APPLE-TILE:M3"]`, so G3t's surface change still can't skip requalification. M3 inherits `replans: 4`, as the transition table says a split milestone should. APPLE-TILE M0 is `parked` in its JSON. The park, its restart rule (explicit lead recommission, fresh independent design approval) and the transitive chain (T2 Chrome M1, simulator M2, G3t M0-M3) are stated where consumers read them: ORCHESTRATE, BUILD-PLAN (G3b, G3t and G4 lines), C12, maturity (new `G3t_apple_tile` entry, the frame-runtime scope and the T2 entry), PLATFORM-RUNTIME, the Mac plan's `Blocked` line and the APPLE-TILE and APPLE-PRESENTATION plans. The two-attempt trace allowance is explicitly not renewed |
| F2 pan inspected against the Ground E image | **Resolved.** T2 M0.5 and Mac M2.1 now carry a worded list: coherent rock/ridge placement, authored sphere sweep including its exit from frame, yaw-consistent settled endpoints, no cut, snap or reversal, and repeats/holds/pauses against host time. The PNG is restricted to scene and framing identity, and the items say quality is never judged by matching it or by awaiting owner image approval. I checked the sphere exit is real behaviour, not invented: the historical pan evidence records "the sphere exited as authored" (`docs/evidence/render/mac-m2-pan/README.md:42`) |

The small inaccuracy noted under first-review F8 is also corrected: the T2 design now says the probe scratch lives in
the `platform-runtime-mac` worktree, and `tmp/platform-runtime-mac-m2/console-probe.txt` is there.

## Did the fix break anything?

Checked and found sound:

- **No cycle.** APPLE-TILE M2 needs `PLATFORM-RUNTIME-MAC:M1` (done); Mac M3 needs `APPLE-TILE:M3`. Nothing in G3t
  needs Mac M2 or M3. Mac M3 relies on in-module ordering for "after M2", which the done-when rules imply.
- **T2 is untouched except M0.5's `inspect` and M0 `replans` 1 -> 2.** M1 and M2 keep `replans: 1`. Every T2 item
  still has its proof; M0.1, M0.2, M1.2 and M2.2 keep `./dev check` under `gpu`. The T2 plan still says it has no
  dependency on G3t, G4 or Mac M2, and now adds that APPLE-TILE's dependency on T2 M0 does not authorise a restart.
- **Mac M2's four items keep every control** from the first two reviews: capture-OFF cadence arm, ON arms labelled
  under capture, numeric Swap Time with a stated join error, known-pattern tap correspondence, pending-readback
  surface loss, capture-OFF allocation check, `./dev check` under `gpu` on M2.3, and the bounded M2.4 stop rule.
  M2.3's ask now says "retain the current direct-output behavior" instead of "G3t direct-output behavior", which is
  correct now that M2 runs on the pre-G3t path.
- **M0/M1 history is unchanged.** The only edit to done Mac M1 is `1×` re-serialised as `×` in M1.2's `inspect`,
  which is the same string after parsing. The loop compares parsed definitions, so this doesn't invalidate M1.
- **APPLE-TILE edits stay within recording the park.** M0 `state` -> `parked`; D2.5, D3.5 and D12.3 now point at Mac
  M3 instead of M2, with M2/M3/M12 `replans` incremented to 2; the plan explicitly claims no resolution of the
  seven findings. That module is out of this review's scope, and I make no judgement of its design.

## Findings

### F1. Mac M3.2 bundles three proofs that M2 deliberately split, and drops M2.3's real-Metal `check` (verified from documents; minor, blocks nothing now)

**What I found.** `PLATFORM-RUNTIME-MAC-DONE.json` M3 has two items. M3.2's `inspect` reruns all three M2.2 cadence
arms; its `ask` also revalidates every M2.3 windowed-tap proof (known-pattern correspondence, lease, pending-readback
loss, capture-OFF direct path) and reruns the M2.4 trace/export command. On the current path these are three items,
split after first-review F1, F3 and F7 so that a failure names one thing. M2.3 also carries
`"check": "./dev check"` with `"lock": "gpu"`; M3.2 has no `check` at all, so on the changed path the loop has nothing
to rerun before believing a tap claim. The parse output shows this: M2.3 `check` present with `gpu`, M3.1 and M3.2
none.

**Why it matters.** G3t M2/M3 replace the surface graph and move tone encoding into present, which is exactly where
the tap's output correspondence and capture-OFF direct path could regress. A single M3.2 verdict of `fail` would not
say whether the tap, the tooling or the cadence failed, and a tap regression would rest on the reviewer reading an
`ask` rather than on a rerunnable check. It doesn't let a fake through (the item still forbids earlier traces and
functional-window-only runs), so this is about precision and the gate, not correctness.

**Should be:** mirror M2. Split M3.2 into a tap revalidation item with `./dev check` under `gpu` plus the live-window
scenario, a trace tooling rerun item, and the cadence item that needs both. Since M3 can't start until APPLE-TILE is
recommissioned, this can be folded into that re-plan rather than done now; the owner may decide it isn't worth a
change at all.

## Done-when judgeability (changed items only)

| Item | Could a reviewer judge it? | Would it catch a convincing fake? |
| --- | --- | --- |
| T2 M0.5 | Yes, now from a worded list | Yes. Fixed-step route and endpoint redraw excluded; omissions make it inconclusive |
| Mac M2.1 | Yes, now from a worded list | Yes. Same protocol; historical `0d53d3aa` pass kept as history only |
| Mac M2.2, M2.3, M2.4 | Yes, and now reachable after T2 M0 | Yes, unchanged controls |
| Mac M3.1 | Yes | Yes. Requires fresh post-G3t identity; a pre-change run or fixed-step movie cannot pass |
| Mac M3.2 | Yes, but one verdict covers three proofs (F1) | Yes for fakes; weaker gate than M2.3 because no `check` |

## Owner's aim

Met. T2 M0 can be built now, and Mac M2, including the attended windowed-tap and trace-tooling work, follows it
without waiting on the parked APPLE-TILE. The remaining V1 Mac obligation after G3t is stated honestly as blocked on
the parked design, with a named restart rule.

DESIGN CHANGES: 1 findings
