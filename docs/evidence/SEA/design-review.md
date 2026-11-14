# SEA design review

**Design approved, with two Suggestions and no Must Fix findings.** Reviewed commit
`25345e8f99772193380689ccb89eb38827e4a589`, 14 November 2026, against the design review at
`7c7744bb5a63`.

The revised SEA design serves the human's request as the wave-field part of the wider water system.
The owner asked for cohesive water including puddles, rivers, waterfalls, streams and lakes, and the
planner task explicitly allowed splitting the module. SEA now supplies the CPU model and GPU ambient
waves; WATERS owns their readers and the common surface, query, interaction waves, foam and optics.
RIVERS and PUDDLES have explicit module briefs, consumers and build order in ARCHITECTURE. This is a
justified division of work, not a deletion of the original sea deliverable. It leaves water outside
the V1 exit and keeps measured performance separate from numeric admission gates.

This verdict approves SEA's design and acceptance criteria. It does not approve all of WATERS' new
mechanisms, design the later river and puddle modules, or qualify running water, visuals or performance.

**S5 — Suggestion, verified wording ambiguity: give the new rate-of-change comparison its units.**

SEA M2.2 in [DONE.json](../../plans/SEA/DONE.json), lines 134–137, adds rate of change to the cascade
comparison but describes the largest difference as a share of the band's RMS height. Displacement is
in metres; its time derivative is in metres per second. Consequently one dimensionless percentage of
height does not specify both error measures. [DESIGN.md](../../plans/SEA/DESIGN.md), lines 150–151,
makes that derivative an input to motion, query carry and reduced cadence, so its measurement should
remain interpretable by those consumers.

When recording the measured bound, name the derivative's units and its normalization separately, or
state the time interval over which derivative error is converted to displacement error. The existing
independent f64 comparison and long-session wrong-phase rejection remain useful and judgeable; this
is a clarification of the measurement, not a request for another mechanism or a blocking test campaign.

**S4 — Suggestion carried forward, verified evidence limitation: distinguish foam transfer from the complete hitch update.**

The previous review's S4 now belongs to WATERS M7.13, formerly SEA M7.11.
[WATERS' design](../../plans/WATERS/DESIGN.md), lines 533–545, preserves the transfer chain and the
full-cover loss rule. Part D of [revision_proof.py](design-proof/revision_proof.py) still exercises
the source-free linear transfer chain, without bounded births, saturation, drift or spreading.
Its long-step equality is evidence for that chain, not the entire composed foam update.

Keep this distinction in the eventual implementation evidence: WATERS M7.13 already requires births
and transport to agree with the reference, and M7.1 requires bounded cover. The composed comparison
should state its tolerance and include active births and cover near one. The migration has not removed
that obligation, and this remains a Suggestion rather than a new blocker for SEA.

**Previous review findings**

The latest review approved the design and had no unresolved Must Fix. I checked that the five earlier
fixes it recorded survive the revision, including where their ownership moved.

| Earlier finding | Result in this revision |
|---|---|
| **MF1 — numeric performance gates override the approved brief** | Remains resolved. SEA's cost account, lines 227–242, labels estimates and says no timing figure decides an item. M2.7 records per-frame pass costs and requires causes for overruns. The paired whole-frame records and tripwire investigations moved to WATERS M2.7, M6.12, M7.12, M8.10, M10.11 and M11.5; numeric timing admission did not return. |
| **MF2 — body rectangles do not define the region receiving their reflections** | The correction moved to WATERS' optics: source rectangles and receiving rectangles remain distinct, with the reflected and refracted volumes and bounded fallback. WATERS M8.4 requires whole reflected bodies; M8.7 retains the independent CPU-ray coverage question, including a reflection outside its source rectangle. Wider scene gathering is an intentional inland-water addition, not a return to silhouette-only receiving coverage. |
| **MF3 — the gathered colour is not the physical radiance used in the optics equation** | The inverse of exposure and camera aerial transport, storage scale, and declared approximations remain in WATERS' optics. M8.8 retains the independent expected-colour question, sixteenfold exposure separation and distant panel that reject both raw-copy and exposure-only substitutes. |
| **MF4 — alternating banks cannot supply the promised consecutive endpoints after skipped ticks** | SEA no longer promises ambient endpoints: M2.4 requires recomputation at each frame's display time. WATERS retains timestamped last-committed endpoints for wake and foam, interpolation over their actual span, bounded catch-up and counted dropped ticks. M6.13 and M7.13 explicitly cover those endpoints and lifecycle. No missing tick is invented. |
| **MF5 — M7.1 forbids the stock transfers it requires** | WATERS M7.1 preserves recipient growth, falling total film plus yield-weighted air inventory, bounded cover and eventual disappearance. Its independent closed-form question still rejects decay of each stock in isolation. S4 above remains the narrower evidence issue. |

The earlier S1 catch-up/source-path requirements remain in WATERS M6.13. S2's normalization to the
requested use tick remains in WATERS M5.1–M5.2, now with separate source times for the frame-evaluated
cascades and tick-stepped wake. The bounded cascade prediction error is explicitly disclosed. S3's
visual inspection scope was not replaced by numerical field checks: the moved milestones retain
submitted imagery and motion inspections on Mac and Chrome.

**Changed-item audit**

I read all 18 current SEA items, compared the changed criteria with the reviewed base, and read every
removed milestone's statements and quality bar against its destination in WATERS. M1's planner-written
criteria are unchanged; its completed status is a separate milestone result, not a finding of this review.

| Changed criterion | Assessment |
|---|---|
| M2 quality bar and dependencies | The bar now asks for the same waves at the same time however frames fall, rather than identical tick banks. `EFFECT-GRAPH:M2` remains the dependency. Its prepared resources, named compute passes and per-frame clock suffice for numerical GPU proofs without a drawn surface. |
| M2.2 | Adds rate of change, an inter-tick frame time, early and 24-hour comparisons, and rejection of f32-second phase. These distinguish the new behavior from the old tick-only field. S5 clarifies the recorded error units. |
| M2.3 | Removing “bank” matches the single freshly evaluated field. Mip slope and Jacobian moments remain checked against the base level. |
| M2.4 | Covers different alphas within one tick, pause, hitch, discard, capture at another tick, backward tick, timeline change and device replacement against the CPU model. An old field reused at every alpha fails. No future WATERS surface is needed to read back these results. |
| M2.5 | Wind transitions must follow the model at each frame time, so an upload that simply steps powers at presentation boundaries cannot rely on the old tick-only criterion. |
| M2.7 | Costs are now per frame, consistent with the new scheduling. The deleted skipped-pass report is unnecessary for these every-frame passes; reduced cadence belongs to WATERS' later knobs. |
| M2.9, added | Whole-cycle cascade frequencies, the half-cycle rounding bound and loop equality are directly testable in the CPU model. Long waves retain exact deep-water frequencies. This explicitly commissions the change to M1's existing cascade evaluator. |

The removed milestones have these destinations in [WATERS' criteria](../../plans/WATERS/DONE.json):

| Removed SEA milestone | Destination and retained obligation |
|---|---|
| M3 | WATERS M2: first native sea, shared light, frame contribution, smooth motion and paired cost. |
| M4 | WATERS M3: first Chrome sea, numerical parity, lifecycle and visible motion. Its lifecycle item now uses display-time fields too. |
| M5 | WATERS M5: asynchronous queries, refusal and generations, one craft and eight floats, speed range, Chrome and game-owned integration. |
| M6 | WATERS M6: physical wake, theoretical wavelength and angle, other-body response, turn/stop behavior, reference agreement and lifecycle; extended to inland interaction waves. |
| M7 | WATERS M7: transported four-stock foam, births, persistence, cooked pattern, Chrome, visuals and lifecycle; extended to shore and current. |
| M8 | WATERS M8: reflection/refraction, radiance conversion, shared cloud shade, Chrome and cost; extended to beds and shore reflections. |
| M9 | WATERS M10: controls, full preset/hour/height/vista range, TAAU, resets, overlays and knobs. |
| M10 | WATERS M11: MetalFX, governor, paired variants, attended cadence, combined scene, public documentation, maturity and provenance. The future APPLE-PRESENTATION M7/M8 readiness check remains explicit. |

WATERS M2 depends on SEA M2, so the first drawn consumer cannot precede the field it requires.
ARCHITECTURE's “Water as one system” and “RIVERS and PUDDLES” sections assign the remaining owner-requested
water types to modules, with shared interfaces and concrete completion cues. They are outstanding planned
work, not capabilities delivered by SEA M2. The scope split is disclosed in SEA's plan as an assumption
the owner can overturn.

**Mechanism and interface checks**

The ambient field can be recomputed from the model and time without the old GPU endpoint lifecycle.
I checked the actual [contribution clock](../../../crates/thrum-render/src/contribution/state.rs) and
[contribution interface](../../../crates/thrum-render/src/contribution.rs): the clock supplies tick,
tick duration, alpha, timeline and capture identity; named compute passes can run each frame. Captures
must not advance committed effect state, but evaluating a function into disposable output does not do
that. SEA declares these passes without state production. WATERS declares them within its contribution,
before its stateful passes and readers, rather than creating another executor.

I also read the existing CPU cascade and component evaluators. They currently use the deep-water
frequency and do not return cascade time derivatives. The revised PLAN explicitly assigns both changes
to SEA M2; it does not silently assume M1 already built them. M2.9 preserves the long-wave relation while
changing only cascade frequencies. The design and plan's “M1 and M2 are unchanged” wording refers to the
later scope split, not to all changes since the reviewed commit; the M2 implementation paragraph gives
the actual work.

The wrapped phase follows directly from integer arithmetic: a bin with integer cycle count `n`, multiplied
by the loop fraction modulo 2^32, advances by exactly `n` turns per loop. Quantizing the loop fraction has
less than one unit of error, so its phase error before float conversion is bounded by `2πn / 2^32`.
The separate non-looping long waves prevent the entire ambient field acquiring that temporal loop.

I read and reran [first_principles_proof.py](design-proof/first_principles_proof.py) unchanged:

```sh
PYTHONDONTWRITEBYTECODE=1 uv run --no-project --no-cache --with numpy python docs/evidence/SEA/design-proof/first_principles_proof.py
```

[The rerun output](design-review/first-principles-rerun.txt) agrees with the design: across its five
sample wavelengths, f32-second phase reaches 0.2556 rad error after a day; integer-loop phase reaches
7.29 × 10⁻⁶ rad; the largest sampled frequency change is 0.1163%. Part G gives 0.5346% amplitude error
for blending a 0.4 m wave's tick endpoints, and explicitly quantifies the later query's prediction error.
This is a rerun of planner evidence, not a GPU qualification or a proof over every admitted profile.
Part G's zero error for direct evaluation is an analytic identity, not a measured implementation result.
The reduced sampling cost is a plausible cost account, still awaiting the paired WATERS M2 measurement.

**Scope and validation**

Read: all four planner tasks in RECORD.json, whole and oldest first; the approved project brief; the
historical SEA brief; the previous review; the complete current SEA DESIGN, PLAN and DONE; the removed
milestone statements; the relevant WATERS mechanisms, all its milestone statements and quality bars,
and the proof fields carrying the earlier fixes. I also read the module split and later-module briefs,
the port decision, C02/C04/C06, the effect clock/interface, the CPU wave/cascade code, and the project's
review guidance and calibration examples. There is no plans glossary. Reading and judging were not
delegated. This was not a blind technique-selection study or a complete review of WATERS.

No GPU scene, browser, image sequence or physical display was inspected. Those remain the responsibilities
of the named implementation milestones. No Broadsides source was modified or newly qualified.

`./dev check-docs` passed; [the output](design-review/check-docs-current.log) is retained. Only this review
and its evidence folder were changed, left uncommitted for the review script. DONE.json is untouched.

DESIGN OK
